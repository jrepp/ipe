#!/usr/bin/env python3
"""Independent black-box checks using an actual Integration Workload and HTTPS fixture."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import http.client
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
from urllib.parse import parse_qs, urlparse


import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "testing" / "integration"))
from servicenow_standin import Fixture, SECRET, TOPOLOGY, RAW_ID

def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Acceptance:
    def __init__(self, binary, report_path):
        self.binary = binary.resolve()
        self.report_path = report_path
        self.temp = tempfile.TemporaryDirectory(prefix="ipe-integration-acceptance-")
        self.directory = Path(self.temp.name)
        self.fixture = Fixture(self.directory)
        self.port = free_port()
        self.token = secrets.token_urlsafe(36)
        self.other_token = secrets.token_urlsafe(36)
        self.denied_token = secrets.token_urlsafe(36)
        self.process = None
        self.log = None
        self.env = os.environ.copy()
        self.report = {"suite": "integration-discovery", "checks": [], "failures": []}
        self.responses = []

    @staticmethod
    def require(value, message):
        if not value:
            raise AssertionError(message)

    def check(self, name, operation):
        try:
            operation()
            self.report["checks"].append({"name": name, "status": "passed"})
        except Exception as error:
            # Never include provider errors, bodies, config, or authorization values.
            self.report["failures"].append({"name": name, "error": type(error).__name__, "message": str(error) if isinstance(error, AssertionError) else "harness operation failed"})

    def request(self, method, path, payload=None, token=True):
        conn = http.client.HTTPConnection("127.0.0.1", self.port, timeout=20)
        headers = {"Content-Type": "application/json"}
        if token is True:
            token = self.token
        if token:
            headers["Authorization"] = "Bearer " + token
        try:
            conn.request(method, path, None if payload is None else json.dumps(payload), headers)
            response = conn.getresponse()
            raw = response.read().decode()
            self.responses.append(raw)
            self.redacted(raw)
            return response.status, json.loads(raw) if raw else None
        finally:
            conn.close()

    def redacted(self, raw):
        for marker in [SECRET, TOPOLOGY, RAW_ID, self.token, self.other_token, self.denied_token, self.fixture.url, str(self.fixture.cert), self.env.get("IPE_INTEGRATION_IDENTITY_KEY", "identity-key-not-configured")]:
            self.require(marker not in raw, "response disclosed a secret, source identity, or topology marker")

    def start(self):
        self.log = (self.directory / "workload.log").open("a")
        self.process = subprocess.Popen([str(self.binary), str(self.config_path)], env=self.env, stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            self.require(self.process.poll() is None, "integration workload exited before readiness")
            try:
                with socket.create_connection(("127.0.0.1", self.port), timeout=0.2):
                    return
            except OSError:
                time.sleep(0.05)
        raise AssertionError("integration workload did not become ready")

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if self.log:
            self.log.close()

    def close(self):
        self.stop()
        self.fixture.stop()
        self.temp.cleanup()

    def toolkit(self, *args, stdin=None):
        script = Path(__file__).resolve().parent / "integration-toolkit.py"
        # Agent clients receive only their own credential, never operator secrets.
        env = {"PATH": os.defpath, "IPE_INTEGRATION_TOKEN": self.token}
        result = subprocess.run([sys.executable, str(script), "--url", f"http://127.0.0.1:{self.port}", *args], input=stdin, text=True, capture_output=True, env=env, timeout=30)
        self.redacted(result.stdout + result.stderr)
        self.require(result.returncode == 0, "toolkit command failed")
        return result.stdout

    def test_cli_mcp(self):
        for command in [("capabilities",), ("check", self.connection_ref), ("discover", self.connection_ref)]:
            self.require(isinstance(json.loads(self.toolkit(*command)), dict), "CLI did not return JSON object")
        calls = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "acceptance", "version": "1"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
        ]
        for identifier, name, arguments in [(3, "integration_capabilities", {}), (4, "integration_check_connection", {"connection_ref": self.connection_ref}), (5, "integration_discover", {"connection_ref": self.connection_ref})]:
            calls.append({"jsonrpc": "2.0", "id": identifier, "method": "tools/call", "params": {"name": name, "arguments": arguments}})
        output = self.toolkit("mcp", stdin="".join(json.dumps(call) + "\n" for call in calls))
        responses = [json.loads(line) for line in output.splitlines() if line.strip()]
        self.require({response.get("id") for response in responses} == {1, 2, 3, 4, 5}, "MCP did not answer every request")
        self.require(all("result" in response and not response["result"].get("isError") for response in responses), "MCP returned an error")
        names = {tool["name"] for response in responses if response.get("id") == 2 for tool in response["result"]["tools"]}
        self.require(names == {"integration_capabilities", "integration_check_connection", "integration_discover"}, "unexpected MCP capability surface")

    def configure(self):
        self.connection_ref = "conn_acceptance"
        self.config_path = self.directory / "config.json"
        self.config = {
            "listen": f"127.0.0.1:{self.port}",
            "api_token_env": "IPE_INTEGRATION_TOKEN",
            "connection_ref": self.connection_ref,
            "origin": self.fixture.url,
            "credential_env": "SERVICENOW_TOKEN",
            "ca_cert_path": str(self.fixture.cert),
            "resources": [{"resource_ref": "resource_remediation", "table": "sn_vul_vulnerable_item", "fields": [{"field_ref": "field_state", "name": "state"}]}],
        }
        self.config_path.write_text(json.dumps(self.config))
        self.config_path.chmod(0o600)
        self.env.update({"IPE_INTEGRATION_TOKEN": self.token, "SERVICENOW_TOKEN": SECRET})

    def configure_inventory(self):
        self.configure()
        self.config["resources"] = []
        self.config["identity_key_env"] = "IPE_INTEGRATION_IDENTITY_KEY"
        self.env["IPE_INTEGRATION_IDENTITY_KEY"] = secrets.token_urlsafe(48)
        self.config_path.write_text(json.dumps(self.config))

    def test_inventory_success(self):
        self.fixture.mode = "success"
        code, result = self.discover()
        self.require(code == 200 and result["coverage"] == "bounded", "initial inventory failed")
        self.require(result["constraints"]["status"] == "unknown", "inventory inferred workflow semantics")
        self.require(len(result["resources"]) == 1, "initial inventory did not discover stand-in table")
        resource = result["resources"][0]
        self.require(resource["resource_ref"].startswith("resource_") and resource["status"] == "observed", "inventory resource identity unavailable")
        self.require(len(resource["fields"]) == 1 and resource["fields"][0]["role"] == "lifecycle_state", "inventory did not discover safe field role")
        for marker in ["sn_vul_vulnerable_item", "sys_dictionary", "sys_db_object", self.env["IPE_INTEGRATION_IDENTITY_KEY"]]:
            self.require(marker not in json.dumps(result), "inventory leaked source topology or identity key")
        self.success = result

    def test_run_deadline(self):
        self.stop()
        original = self.config["resources"][0]
        self.config["resources"] = [{"resource_ref": f"resource_{index}", "table": original["table"], "fields": [{"field_ref": f"field_{index}", "name": "state"}]} for index in range(3)]
        self.config_path.write_text(json.dumps(self.config))
        self.start()
        self.fixture.delay_seconds = 3
        before = len(self.fixture.requests)
        started = time.monotonic()
        try:
            with ThreadPoolExecutor(max_workers=1) as executor:
                future = executor.submit(self.discover)
                while len(self.fixture.requests) == before and time.monotonic() - started < 3:
                    time.sleep(0.01)
                code, _ = self.discover()
                self.require(code == 429, "concurrent discovery run was not rejected")
                code, result = future.result(timeout=16)
                elapsed = time.monotonic() - started
                self.require(code == 504 and result.get("code") == "run_deadline", "global run deadline did not cancel cumulative provider waits")
                self.require(10 <= elapsed < 16, "global deadline did not bound elapsed run time")
        finally:
            self.fixture.delay_seconds = 0
        code, _ = self.discover()
        self.require(code == 200, "deadline cancellation leaked concurrency permit")

    def test_inventory(self):
        self.stop()
        self.configure_inventory()
        self.start()
        self.test_inventory_success()
        self.test_restart()
        self.test_cli_mcp()
        self.fixture.mode = "denied"
        code, result = self.discover()
        self.require(code == 200 and result.get("code") == "access_denied" and result["constraints"]["status"] == "unknown", "unseeded access denial was modeled as absence")
        self.fixture.mode = "success"

    def discover(self, **extra):
        return self.request("POST", "/v1/discovery", {"connection_ref": self.connection_ref, **extra})

    def test_success(self):
        self.fixture.mode = "success"
        code, capabilities = self.request("GET", "/v1/capabilities")
        self.require(code == 200 and capabilities["publication"] is False, "capabilities must explicitly disable publication")
        self.require(capabilities["connection_refs"] == [self.connection_ref], "capabilities omit configured opaque reference")
        code, check = self.request("POST", "/v1/connections/check", {"connection_ref": self.connection_ref})
        self.require(code == 200 and check["status"] == "available", "authorized connection check did not observe metadata")
        code, result = self.discover()
        self.require(code == 200 and result["coverage"] == "bounded", "discovery must report bounded coverage")
        self.require(result["constraints"]["status"] == "unknown", "metadata discovery must not infer complete workflow semantics")
        resource = result["resources"][0]
        self.require(resource["resource_ref"] == "resource_remediation" and resource["status"] == "observed", "configured resource was not observed")
        field = resource["fields"][0]
        self.require(field["field_ref"] == "field_state" and field["status"] == "observed" and field["required"] is True, "required-field metadata was not discovered")
        for marker in ["sn_vul_vulnerable_item", "sys_dictionary", "sys_db_object"]:
            self.require(marker not in json.dumps(result), "public discovery exposed configured physical table names")
        self.success = result

    def test_authorization(self):
        before = len(self.fixture.requests)
        for token in [False, self.other_token]:
            for method, path, payload in [("GET", "/v1/capabilities", None), ("POST", "/v1/connections/check", {"connection_ref": self.connection_ref}), ("POST", "/v1/discovery", {"connection_ref": self.connection_ref})]:
                code, _ = self.request(method, path, payload, token=token)
                self.require(code in (401, 403), "unauthorized caller reached integration operation")
        for ref in ["conn_foreign", "https://internal.example", "../conn_acceptance"]:
            code, _ = self.request("POST", "/v1/discovery", {"connection_ref": ref})
            self.require(code in (400, 403, 404), "unconfigured connection reference accepted")
        self.require(len(self.fixture.requests) == before, "denied operations contacted the provider")

    def test_request_restrictions(self):
        before = len(self.fixture.requests)
        for extra in [{"url": "https://internal.example"}, {"table": "sys_user"}, {"sysparm_query": "nameISNOTEMPTY"}, {"credential": SECRET}, {"publish": True}]:
            code, _ = self.discover(**extra)
            self.require(400 <= code < 500, "caller-controlled provider or publication parameter accepted")
        for path in ["/v1/publish", "/v1/evaluate", "/v1/runtime/facts", "/v1/connections"]:
            code, _ = self.request("POST", path, {})
            self.require(code in (404, 405), "unexpected write/enforcement capability is exposed")
        self.require(len(self.fixture.requests) == before, "rejected request contacted provider")

    def test_upstream_bounds(self):
        self.require(bool(self.fixture.requests), "no provider requests observed")
        for path, headers in self.fixture.requests:
            parsed = urlparse(path)
            query = parse_qs(parsed.query)
            self.require(parsed.path in ("/api/now/table/sys_db_object", "/api/now/table/sys_dictionary"), "provider call escaped metadata allowlist")
            self.require(int(query["sysparm_limit"][0]) <= 33, "provider rows not bounded")
            self.require("sysparm_fields" in query and "sysparm_query" in query, "provider projection and query must be explicit")
            self.require(query["sysparm_query"][0].startswith("name=sn_vul_vulnerable_item"), "provider query escaped configured table")
            self.require(headers.get("authorization", headers.get("Authorization")) == "Bearer " + SECRET, "native broker omitted configured provider credential")

    def test_fault(self, mode):
        self.fixture.mode = mode
        before = len(self.fixture.requests)
        code, result = self.discover()
        self.require(code == 200, "provider failure must return a sanitized discovery result")
        self.require(result["constraints"]["status"] == "unknown", "failure was interpreted as absence of constraints")
        resource = result["resources"][0]
        self.require(resource["status"] == "unknown", "unavailable or incomplete evidence treated as observed")
        self.require(resource.get("code") not in (None, "ok"), "unknown evidence lacks machine-readable reason")
        self.require(len(self.fixture.requests) - before <= 2, "provider failure caused unbounded retries")
        if mode == "redirect":
            self.require(all("forbidden-redirect" not in path for path, _ in self.fixture.requests), "broker followed provider redirect")
        self.fixture.mode = "success"

    def test_provider_credential(self):
        self.fixture.expected_token = "rotated-fixture-credential"
        try:
            code, result = self.discover()
            self.require(code == 200 and result["resources"][0]["status"] == "unknown", "invalid provider credential produced observed evidence")
            self.require(result["resources"][0]["code"] != "ok", "provider authentication failure lacks reason")
        finally:
            self.fixture.expected_token = SECRET

    def test_restart(self):
        self.fixture.mode = "success"
        self.stop()
        self.start()
        code, result = self.discover()
        self.require(code == 200 and result == self.success, "same configured identity and evidence changed across restart")

    def test_log_redaction(self):
        if self.log:
            self.log.flush()
        self.redacted((self.directory / "workload.log").read_text())

    def run(self):
        try:
            self.require(self.binary.is_file(), "build the ipe-integration binary before running acceptance")
            self.configure()
            self.start()
            self.check("opaque discovery and honest bounded coverage", self.test_success)
            self.check("authentication and connection isolation", self.test_authorization)
            self.check("strict input and no publication/enforcement surface", self.test_request_restrictions)
            for mode in ["denied", "unauthorized", "failure", "unsupported", "empty", "wrong_shape", "malformed", "oversize", "redirect", "truncated"]:
                self.check("provider " + mode + " remains unknown and redacted", lambda mode=mode: self.test_fault(mode))
            self.fixture.mode = "success"
            self.check("CLI and MCP exercise live deployment", self.test_cli_mcp)
            self.check("provider credential authentication", self.test_provider_credential)
            self.check("bounded authenticated metadata reads", self.test_upstream_bounds)
            self.check("stable opaque identities after restart", self.test_restart)
            self.check("run deadline and concurrency permit release", self.test_run_deadline)
            self.check("unseeded credential-first discovery and stable restart", self.test_inventory)
            self.check("workload log redaction", self.test_log_redaction)
        except Exception as error:
            self.report["failures"].append({"name": "harness", "error": type(error).__name__, "message": str(error) if isinstance(error, AssertionError) else "harness operation failed"})
        finally:
            self.close()
            self.report["passed"] = not self.report["failures"]
            self.report_path.parent.mkdir(parents=True, exist_ok=True)
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
        print(json.dumps(self.report, indent=2))
        return 0 if self.report["passed"] else 1


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target/debug/ipe-integration")
    parser.add_argument("--report", type=Path, default=root / "target/integration-discovery-report.json")
    args = parser.parse_args()
    return Acceptance(args.binary, args.report).run()


if __name__ == "__main__":
    sys.exit(main())
