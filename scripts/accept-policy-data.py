#!/usr/bin/env python3
"""Exercise external policy import and runtime/policy separation over real HTTP."""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import pathlib
import ssl
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("runtime_acceptance", ROOT / "scripts/accept-runtime-data.py")
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


class PolicyAcceptance(runtime.RuntimeDataAcceptance):
    def write_config(self):
        self.suite = json.loads((ROOT / "acceptance/policy-data/pr-landing.json").read_text())
        self.base = "/v1/scopes/acceptance/test"
        self.config = {
            "database_path": str(self.database), "bind": f"127.0.0.1:{self.server_port}",
            "input_schema": self.suite["input_schema"],
            "collections": [{"scope": runtime.scope(), "name": name, "config": config}
                            for name, config in self.suite["collections"].items()],
            "policies": [], "connectors": [],
            "credentials": [
                {"token_env": "IPE_ACCEPT_WRITER", "scope": runtime.scope(), "source_id": "forge-runtime", "write_collections": ["pull_requests"]},
                {"token_env": "IPE_ACCEPT_EVAL", "scope": runtime.scope(), "evaluate": True},
                {"token_env": "IPE_ACCEPT_INSPECT", "scope": runtime.scope(), "inspect": True, "evaluate": True, "manage_policies": True},
            ],
        }
        self.config_path.write_text(json.dumps(self.config))

    def req(self, method, path, body=None, token=None, expected=200):
        status, result = self.request(method, self.base + path, body, token or self.inspect_token)
        self.require(status == expected, f"{method} {path}: HTTP {status}, wanted {expected}: {result}")
        return result

    def decision(self):
        return self.req("POST", "/evaluate", {"input": self.suite["cases"][0]["input"]}, self.eval_token)

    def publish(self, document, expected_revision=None, token=None, expected=200):
        return self.req("POST", "/policies/pr-landing/revisions", {"document": document, "expected_revision": expected_revision}, token, expected)

    def import_external(self, minimum, revision, observed_at):
        # This fixture acts as an agent: read authoritative source bytes, map supported
        # rules, attach exact source pointers, and publish through the management API.
        upstream = {"review": {"minimum": minimum}, "required_checks": ["test", "lint"]}
        self.upstream_state.set(upstream)
        context = ssl.create_default_context(cafile=str(self.ca_path))
        with urllib.request.urlopen(self.upstream_url, context=context, timeout=5) as response:
            raw = response.read()
        source = json.loads(raw)
        document = copy.deepcopy(self.suite["documents"][0])
        document["data"]["value"] = source
        document["external"].update(locator=self.upstream_url, source_revision=revision,
                                    source_digest=hashlib.sha256(raw).hexdigest(),
                                    observed_at=observed_at, valid_until=observed_at + 300)
        return document

    def test_import_and_runtime_separation(self):
        now = runtime.unix_now()
        self.document = self.import_external(2, "r1", now - 2)
        self.require(self.decision()["decision"] == "deny", "empty catalog allowed")
        for token in [self.writer_token, self.eval_token]:
            self.publish(self.document, token=token, expected=403)
        self.first = self.publish(self.document)
        self.require(self.first["revision"] == 1, "first revision was not 1")
        self.require(self.decision()["evaluation_status"] == "indeterminate", "missing facts allowed")
        value = self.suite["cases"][0]["facts"][0]["value"]
        self.req("POST", "/facts/pull_requests/commit", {"mutations": [runtime.put("commit-abc", value, 1, None, now, 120)]}, self.writer_token)
        result = self.decision()
        self.require(result["decision"] == "allow", f"fresh evidence did not allow: {result}")
        self.runtime_revision = result["revision"]
        self.require(result["policy_documents"][0]["document_digest"] == self.first["document_digest"], "decision lost policy provenance")
        status, _ = self.request("GET", "/v1/scopes/other/test/policies/pr-landing/revisions/1", token=self.inspect_token)
        self.require(status == 403, "cross-scope revision exposed")

    def test_refresh_and_dry_run(self):
        now = runtime.unix_now()
        self.refreshed = self.import_external(3, "r2", now - 1)
        body = {"document": self.refreshed, "input": self.suite["cases"][0]["input"]}
        self.req("POST", "/policies/test", body, self.eval_token, expected=403)
        result = self.req("POST", "/policies/test", body)
        self.require(result["simulation"] and result["decision"] == "deny", "candidate did not evaluate stricter policy")
        self.require(self.decision()["decision"] == "allow", "dry run mutated policy")
        self.publish(self.refreshed, None, expected=409)
        self.second = self.publish(self.refreshed, 1)
        result = self.decision()
        self.require(result["decision"] == "deny", "updated external policy did not deny")
        self.require(result["revision"] == self.runtime_revision, "policy publication mutated runtime revision")
        self.require(result["policy_documents"][0]["revision"] == 2, "evaluation did not use new revision")
        prior = self.req("GET", "/policies/pr-landing/revisions/1")
        self.require(prior == self.first, "historical revision changed")

    def test_rejected_refresh_and_unsupported(self):
        now = runtime.unix_now()
        expired = copy.deepcopy(self.refreshed)
        expired["external"].update(observed_at=now-1, valid_until=now)
        self.publish(expired, 2, expected=422)
        invalid = copy.deepcopy(self.refreshed)
        invalid["sections"][0]["rule"]["source"] = "bad syntax"
        self.publish(invalid, 2, expected=422)
        self.require(self.req("GET", "/catalog")["policy_documents"][0]["revision"] == 2, "invalid refresh replaced policy")
        unknown = copy.deepcopy(self.document)
        unknown["sections"].append({"id": "unknown", "title": "Unknown rule", "source_pointer": "/new_rule", "rule": {"kind": "unsupported", "reason": "No model for this external rule"}})
        result = self.req("POST", "/policies/test", {"document": unknown, "input": self.suite["cases"][0]["input"]})
        self.require(result["evaluation_status"] == "indeterminate" and result["decision"] == "deny", "unsupported rule allowed")
        expired["data"]["value"]["review"]["minimum"] = 2
        result = self.req("POST", "/policies/test", {"document": expired, "input": self.suite["cases"][0]["input"]})
        self.require(result["evaluation_status"] == "indeterminate", "expired observation allowed")

    def test_policy_restart(self):
        self.stop_server()
        self.start_server()
        self.wait_for_ready()
        self.require(self.req("GET", "/policies/pr-landing/revisions/1") == self.first, "old revision lost on restart")
        self.require(self.req("GET", "/policies/pr-landing/revisions/2") == self.second, "active revision lost on restart")
        self.require(self.decision()["decision"] == "deny", "restart changed policy behavior")

    def run(self):
        self.report["suite"] = "policy-data-acceptance"
        try:
            self.prepare_tls_upstream()
            self.write_config()
            self.start_server()
            self.wait_for_ready()
            # Later groups rely on successfully publishing the first revisions.
            for name, operation in [
                ("HTTPS source import, grants, scope, and runtime evidence", self.test_import_and_runtime_separation),
                ("refresh, immutable history, conflicts, and candidate testing", self.test_refresh_and_dry_run),
                ("invalid refreshes, unsupported rules, and stale imports", self.test_rejected_refresh_and_unsupported),
                ("policy and runtime restart persistence", self.test_policy_restart),
            ]:
                operation()
                self.report["checks"].append({"name": name, "status": "passed"})
        except Exception as error:
            self.failure("policy workflow", str(error))
        finally:
            self.stop_server()
            self.stop_upstream()
            self.report.update(finished_at=runtime.utc_now(), passed=not self.report["failures"])
            self.report_path.parent.mkdir(parents=True, exist_ok=True)
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
            self.temp.cleanup()
        print(json.dumps(self.report, indent=2))
        return 0 if self.report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(PolicyAcceptance(ROOT / "target/debug/ipe-server", ROOT / "target/policy-data-acceptance.json").run())
