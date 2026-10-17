#!/usr/bin/env python3
"""Black-box acceptance checks for runtime facts and the pull connector.

The script launches the public ipe-server binary, talks only over HTTP, and
serves a local HTTPS snapshot endpoint with a generated CA and localhost
certificate. It writes a JSON report even when an acceptance assertion fails.
"""

from __future__ import annotations

import argparse
import http.client
import json
import os
import pathlib
import secrets
import shutil
import signal
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any


TENANT = "acceptance"
ENVIRONMENT = "test"


class SnapshotState:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.body: bytes = b""
        self.status = 200

    def set(self, payload: Any, status: int = 200, raw: bool = False) -> None:
        body = payload if raw else json.dumps(payload, separators=(",", ":")).encode()
        with self.lock:
            self.body = body
            self.status = status

    def get(self) -> tuple[int, bytes]:
        with self.lock:
            return self.status, self.body


class SnapshotHandler(BaseHTTPRequestHandler):
    state: SnapshotState

    def do_GET(self) -> None:  # noqa: N802
        status, body = self.state.get()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, _format: str, *_args: object) -> None:
        return


class AcceptanceFailure(RuntimeError):
    pass


class RuntimeDataAcceptance:
    def __init__(self, binary: pathlib.Path, report: pathlib.Path) -> None:
        self.binary = binary.resolve()
        self.report_path = report.resolve()
        self.report: dict[str, Any] = {
            "suite": "runtime-data-acceptance",
            "started_at": utc_now(),
            "binary": str(self.binary),
            "checks": [],
            "failures": [],
        }
        self.temp = tempfile.TemporaryDirectory(prefix="ipe-runtime-data-")
        self.directory = pathlib.Path(self.temp.name)
        self.process: subprocess.Popen[str] | None = None
        self.server_port = free_port()
        self.metrics_port = free_port()
        while self.metrics_port == self.server_port:
            self.metrics_port = free_port()
        self.writer_token = secrets.token_urlsafe(48)
        self.eval_token = secrets.token_urlsafe(48)
        self.inspect_token = secrets.token_urlsafe(48)
        self.denied_token = secrets.token_urlsafe(48)
        self.upstream_state = SnapshotState()
        self.upstream: ThreadingHTTPServer | None = None
        self.upstream_thread: threading.Thread | None = None
        self.database = self.directory / "runtime-db"
        self.config_path = self.directory / "config.json"
        self.ca_path = self.directory / "ca.crt"
        self.config: dict[str, Any] = {}
        self.log_path = self.directory / "server.log"

    def run(self) -> int:
        try:
            self.require(self.binary.is_file(), f"server binary does not exist: {self.binary}")
            self.prepare_tls_upstream()
            self.write_config()
            self.start_server()
            self.wait_for_ready()
            self.check("authentication and scope", self.test_authentication_and_scope)
            self.check("typed values and policy shapes", self.test_typed_values)
            self.check("strict rejection and atomic commits", self.test_rejections_and_atomicity)
            self.check("missing and stale evidence", self.test_missing_and_stale)
            self.check("source replay and revision conflicts", self.test_replay_and_conflicts)
            self.check("pull snapshots", self.test_pull_snapshots)
            self.check("restart persistence", self.test_restart_persistence)
        except Exception as error:  # always make the failure inspectable
            self.failure("harness", str(error))
        finally:
            self.stop_server()
            self.stop_upstream()
            self.report["finished_at"] = utc_now()
            self.report["passed"] = not self.report["failures"]
            self.report_path.parent.mkdir(parents=True, exist_ok=True)
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
            self.temp.cleanup()
        print(json.dumps(self.report, indent=2))
        return 0 if self.report["passed"] else 1

    def check(self, name: str, operation: Any) -> None:
        try:
            operation()
            self.report["checks"].append({"name": name, "status": "passed"})
        except Exception as error:
            self.failure(name, str(error))

    def failure(self, name: str, message: str) -> None:
        self.report["failures"].append({"name": name, "message": message})

    @staticmethod
    def require(condition: bool, message: str) -> None:
        if not condition:
            raise AcceptanceFailure(message)

    def prepare_tls_upstream(self) -> None:
        openssl = shutil.which("openssl")
        self.require(openssl is not None, "openssl is required to create the local HTTPS fixture")
        ca_key = self.directory / "ca.key"
        server_key = self.directory / "server.key"
        server_csr = self.directory / "server.csr"
        server_cert = self.directory / "server.crt"
        ext = self.directory / "server.ext"
        ca_config = self.directory / "ca.cnf"
        ca_config.write_text("[req]\ndistinguished_name=dn\n[dn]\n[v3_ca]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\nsubjectKeyIdentifier=hash\n")
        commands = [
            [openssl, "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-keyout", str(ca_key), "-out", str(self.ca_path), "-subj", "/CN=IPE Acceptance CA", "-config", str(ca_config), "-extensions", "v3_ca"],
            [openssl, "req", "-newkey", "rsa:2048", "-nodes", "-keyout", str(server_key), "-out", str(server_csr), "-subj", "/CN=localhost"],
        ]
        ext.write_text("subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\n")
        commands.append([openssl, "x509", "-req", "-in", str(server_csr), "-CA", str(self.ca_path), "-CAkey", str(ca_key), "-CAcreateserial", "-days", "1", "-extfile", str(ext), "-out", str(server_cert)])
        for command in commands:
            completed = subprocess.run(command, capture_output=True, text=True)
            self.require(completed.returncode == 0, f"openssl failed: {completed.stderr.strip()}")

        SnapshotHandler.state = self.upstream_state
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), SnapshotHandler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(server_cert, server_key)
        self.upstream.socket = context.wrap_socket(self.upstream.socket, server_side=True)
        self.upstream_thread = threading.Thread(target=self.upstream.serve_forever, daemon=True)
        self.upstream_thread.start()
        self.upstream_url = f"https://127.0.0.1:{self.upstream.server_port}/snapshot"

    def write_config(self) -> None:
        self.upstream_state.set(snapshot(1, unix_now(), {"pullcase": valid_fact(), "pull-old": valid_fact()}))
        schemas = schemas_for_acceptance()
        self.config = {
            "database_path": str(self.database),
            "bind": f"127.0.0.1:{self.server_port}",
            "input_schema": input_schema(),
            "credentials": [
                {"token_env": "IPE_ACCEPT_WRITER", "scope": scope(), "source_id": "approval-service", "write_collections": ["approvals", "scalar_bool", "scalar_string", "scalar_int", "scalar_strings"], "evaluate": False, "inspect": False},
                {"token_env": "IPE_ACCEPT_EVAL", "scope": scope(), "evaluate": True, "inspect": False},
                {"token_env": "IPE_ACCEPT_INSPECT", "scope": scope(), "inspect": True, "evaluate": False},
                {"token_env": "IPE_ACCEPT_DENIED", "scope": scope(), "evaluate": False, "inspect": False},
            ],
            "collections": [
                {"scope": scope(), "name": "approvals", "config": {"schema": schemas, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "approval-service", "mode": "push"}},
                {"scope": scope(), "name": "scalar_bool", "config": {"schema": {"type": "bool"}, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "approval-service", "mode": "push"}},
                {"scope": scope(), "name": "scalar_string", "config": {"schema": {"type": "string"}, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "approval-service", "mode": "push"}},
                {"scope": scope(), "name": "scalar_int", "config": {"schema": {"type": "int"}, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "approval-service", "mode": "push"}},
                {"scope": scope(), "name": "scalar_strings", "config": {"schema": {"type": "array", "items": {"type": "string"}, "max_items": 10}, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "approval-service", "mode": "push"}},
                {"scope": scope(), "name": "directory", "config": {"schema": schemas, "max_ttl_seconds": 120, "max_observation_age_seconds": 120, "source_id": "directory-service", "mode": "pull"}},
            ],
            "policies": [
                {"scope": scope(), "source": approval_policy()},
                {"scope": scope(), "source": pull_policy()},
            ],
            "connectors": [
                {"scope": scope(), "collection": "directory", "source_id": "directory-service", "url": self.upstream_url, "ca_certificate": str(self.ca_path), "poll_interval_seconds": 1, "timeout_seconds": 3, "max_bytes": 1048576}
            ],
        }
        self.config_path.write_text(json.dumps(self.config, indent=2) + "\n")

    def start_server(self) -> None:
        env = os.environ.copy()
        env.update({
            "IPE_RUNTIME_CONFIG": str(self.config_path),
            "IPE_ACCEPT_WRITER": self.writer_token,
            "IPE_ACCEPT_EVAL": self.eval_token,
            "IPE_ACCEPT_INSPECT": self.inspect_token,
            "IPE_ACCEPT_DENIED": self.denied_token,
            "IPE_SERVER_HOST": "127.0.0.1",
            "IPE_SERVER_PORT": str(self.server_port),
            "IPE_SERVER_METRICS_PORT": str(self.metrics_port),
        })
        self.log_file = self.log_path.open("w")
        self.process = subprocess.Popen([str(self.binary)], env=env, stdout=self.log_file, stderr=subprocess.STDOUT, text=True)

    def stop_server(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if hasattr(self, "log_file"):
            self.log_file.close()
        self.process = None

    def stop_upstream(self) -> None:
        if self.upstream is not None:
            self.upstream.shutdown()
            self.upstream.server_close()
            self.upstream = None
        if self.upstream_thread is not None:
            self.upstream_thread.join(timeout=2)

    def wait_for_ready(self) -> None:
        deadline = time.monotonic() + 20
        last_error = ""
        while time.monotonic() < deadline:
            if self.process and self.process.poll() is not None:
                raise AcceptanceFailure(f"server exited {self.process.returncode}: {self.log_tail()}")
            try:
                status, _ = self.request("GET", "/ready", token=self.eval_token)
                if status == 200:
                    return
            except OSError as error:
                last_error = str(error)
            time.sleep(0.1)
        raise AcceptanceFailure(f"server did not become ready: {last_error}; {self.log_tail()}")

    def log_tail(self) -> str:
        try:
            return "\n".join(self.log_path.read_text().splitlines()[-30:])
        except OSError:
            return "server log unavailable"

    def request(self, method: str, path: str, body: Any = None, token: str | None = None, raw: bytes | None = None) -> tuple[int, Any]:
        connection = http.client.HTTPConnection("127.0.0.1", self.server_port, timeout=5)
        headers = {}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        if body is not None or raw is not None:
            headers["Content-Type"] = "application/json"
        payload = raw if raw is not None else (json.dumps(body, separators=(",", ":")).encode() if body is not None else None)
        connection.request(method, path, body=payload, headers=headers)
        response = connection.getresponse()
        data = response.read()
        status = response.status
        connection.close()
        try:
            decoded: Any = json.loads(data) if data else None
        except json.JSONDecodeError:
            decoded = data.decode("utf-8", errors="replace")
        return status, decoded

    def commit(self, mutations: list[dict[str, Any]], token: str | None = None, collection: str = "approvals") -> tuple[int, Any]:
        return self.request("POST", f"/v1/scopes/{TENANT}/{ENVIRONMENT}/facts/{collection}/commit", {"mutations": mutations}, token or self.writer_token)

    def write_case(self, key: str, fact_value: dict[str, Any], scalar_values: dict[str, Any], observed_at: int, ttl_seconds: int = 60) -> tuple[int, Any]:
        collection_values = {"approvals": fact_value, **scalar_values}
        for collection, value in collection_values.items():
            status, result = self.commit([put(key, value, 1, None, observed_at, ttl_seconds)], collection=collection)
            if status != 200:
                return status, result
        return 200, {"key": key}

    def evaluate(self, key: str, collection: str = "approvals") -> tuple[int, Any]:
        return self.request("POST", f"/v1/scopes/{TENANT}/{ENVIRONMENT}/evaluate", {"input": input_value(key)}, self.eval_token)

    def fact(self, collection: str, key: str, token: str | None = None) -> tuple[int, Any]:
        return self.request("GET", f"/v1/scopes/{TENANT}/{ENVIRONMENT}/facts/{collection}/{key}", token=token or self.inspect_token)

    def test_authentication_and_scope(self) -> None:
        status, _ = self.request("GET", "/v1/scopes/acceptance/test/facts/approvals/demo")
        self.require(status == 401, f"unauthenticated fact read returned HTTP {status}, expected 401")
        status, _ = self.fact("approvals", "demo", self.denied_token)
        self.require(status == 403, f"unauthorized fact read returned HTTP {status}, expected 403")
        status, _ = self.request("GET", "/v1/scopes/another/test/facts/approvals/demo", token=self.inspect_token)
        self.require(status == 403, f"cross-scope fact read returned HTTP {status}, expected 403")

    def test_typed_values(self) -> None:
        observed = unix_now()
        scalar_values = {"scalar_bool": True, "scalar_string": "alice", "scalar_int": 7,
                         "scalar_strings": ["critical", "release"]}
        status, result = self.write_case("demo", valid_fact(), scalar_values, observed)
        self.require(status == 200, f"valid typed record commit failed: {result}")
        for collection, value in scalar_values.items():
            status, scalar_record = self.fact(collection, "demo")
            self.require(status == 200 and scalar_record.get("record", {}).get("value") == value,
                         f"{collection} did not preserve its typed value: {scalar_record}")
        status, result = self.evaluate("demo")
        self.require(status == 200 and result.get("decision") == "allow", f"typed policy did not allow nested/list aggregate fact: {result}")
        status, result = self.fact("approvals", "demo")
        self.require(status == 200 and result.get("status") == "found", f"fact inspection failed: {result}")
        record = result.get("record", {})
        self.require(record.get("value", {}).get("active") is True, "bool field did not round-trip")
        self.require(record.get("value", {}).get("priority") == 7, "int64 field did not round-trip")
        self.require(record.get("value", {}).get("labels") == ["critical", "release"], "string list did not round-trip")
        self.require(record.get("value", {}).get("metadata", {}).get("level") == "high", "nested record did not round-trip")
        self.require(len(record.get("value", {}).get("approvers", [])) == 2, "record list did not round-trip")

        negative_cases = [
            ("low-priority", valid_fact(), {**scalar_values, "scalar_int": 4}),
            ("wrong-owner", valid_fact(), {**scalar_values, "scalar_string": "mallory"}),
            ("false-flag", valid_fact(), {**scalar_values, "scalar_bool": False}),
            ("missing-membership", valid_fact(), {**scalar_values, "scalar_strings": ["release"]}),
            ("duplicate-principals", {**valid_fact(), "approvers": [
                {"principal_id": "same", "role": "senior", "team": "security"},
                {"principal_id": "same", "role": "senior", "team": "platform"}]}, scalar_values),
            ("empty-approvers", {**valid_fact(), "approvers": []}, scalar_values),
        ]
        for key, fact_value, direct_values in negative_cases:
            status, details = self.write_case(key, fact_value, direct_values, unix_now())
            self.require(status == 200, f"negative case {key} setup failed: {details}")
            status, decision = self.evaluate(key)
            self.require(status == 200 and decision.get("evaluation_status") == "deny",
                         f"valid unsatisfied case {key} did not deny: {decision}")

    def test_rejections_and_atomicity(self) -> None:
        bad_facts = [
            {**valid_fact(), "active": None},
            {**valid_fact(), "priority": 1.5},
            {**valid_fact(), "labels": ["good", 7]},
            {**valid_fact(), "unknown": "field"},
        ]
        for index, value in enumerate(bad_facts):
            status, _ = self.commit([put(f"bad-{index}", value, 1, None, unix_now(), 20)])
            self.require(status == 422, f"invalid value case {index} returned HTTP {status}, expected 422")

        duplicate = b'{"mutations":[{"operation":"put","key":"dup","key":"other","value":{"active":true},"expected_revision":null,"source_revision":1,"observed_at":1,"ttl_seconds":10}]}'
        status, _ = self.request("POST", f"/v1/scopes/{TENANT}/{ENVIRONMENT}/facts/approvals/commit", raw=duplicate, token=self.writer_token)
        self.require(status == 422, f"duplicate JSON key returned HTTP {status}, expected 422")

        status, result = self.commit([
            put("atomic-good", valid_fact(), 1, None, unix_now(), 20),
            put("atomic-bad", {**valid_fact(), "priority": None}, 1, None, unix_now(), 20),
        ])
        self.require(status == 422, f"mixed valid/invalid batch returned HTTP {status}, expected 422: {result}")
        status, result = self.fact("approvals", "atomic-good")
        self.require(status == 200 and result.get("status") != "found", f"failed batch partially committed: {result}")

    def test_missing_and_stale(self) -> None:
        status, result = self.evaluate("not-present")
        self.require(status == 200 and result.get("evaluation_status") == "indeterminate", f"missing evidence did not become indeterminate: {result}")

        scalar_values = {"scalar_bool": True, "scalar_string": "alice", "scalar_int": 7,
                         "scalar_strings": ["critical", "release"]}
        status, result = self.write_case("expired", valid_fact(), scalar_values, unix_now(), ttl_seconds=1)
        self.require(status == 200, f"expiry setup write failed: {result}")
        deadline = time.monotonic() + 3
        expired_record: Any = None
        while time.monotonic() < deadline:
            status, expired_record = self.fact("approvals", "expired")
            if status == 200 and "expired" in str(expired_record.get("status", "")):
                break
            time.sleep(0.1)
        self.require(status == 200 and "expired" in str(expired_record.get("status", "")),
                     f"fact resolver did not expose expiry: {expired_record}")
        deadline = time.monotonic() + 3
        expired_result: Any = None
        while time.monotonic() < deadline:
            status, expired_result = self.evaluate("expired")
            if status == 200 and expired_result.get("evaluation_status") == "indeterminate":
                break
            time.sleep(0.1)
        self.require(status == 200 and expired_result.get("evaluation_status") == "indeterminate", f"expired evidence did not become indeterminate: {expired_result}")

        observed = unix_now()
        status, result = self.write_case("freshness", valid_fact(), scalar_values, observed)
        self.require(status == 200, f"freshness setup write failed: {result}")
        status, result = self.evaluate("freshness")
        self.require(status == 200 and result.get("decision") == "allow", f"fresh fact did not allow: {result}")
        deadline = time.monotonic() + 12
        stale_result: Any = None
        while time.monotonic() < deadline:
            time.sleep(0.15)
            status, stale_result = self.evaluate("freshness")
            if status == 200 and stale_result.get("evaluation_status") == "indeterminate":
                break
        self.require(status == 200 and stale_result.get("evaluation_status") == "indeterminate", f"stale evidence did not become indeterminate: {stale_result}")
        reason_text = json.dumps(stale_result)
        self.require("StaleFact" in reason_text or "stale" in reason_text.lower(),
                     f"freshness boundary failed for an unexpected reason: {stale_result}")

    def test_replay_and_conflicts(self) -> None:
        observed = unix_now()
        status, result = self.commit([put("replay", valid_fact(), 1, None, observed, 8)])
        self.require(status == 200, f"replay setup failed: {result}")
        record = result.get("records", [{}])[0] or {}
        original_expiry = record.get("expires_at")
        time.sleep(1.2)
        status, retry = self.commit([put("replay", valid_fact(), 1, None, observed, 8)])
        self.require(status == 200, f"identical retry failed: {retry}")
        retried = retry.get("records", [{}])[0] or {}
        self.require(retried.get("expires_at") == original_expiry, "identical retry extended the fact expiry")

        status, conflict = self.commit([put("replay", valid_fact(), 2, 99999, unix_now(), 8)])
        self.require(status == 409, f"wrong expected revision returned HTTP {status}: {conflict}")
        status, replay = self.commit([put("replay", valid_fact(), 1, None, observed, 8)])
        self.require(status == 200, f"same exact retry should remain idempotent, got {status}: {replay}")

    def test_pull_snapshots(self) -> None:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            status, result = self.fact("directory", "pull-old")
            if status == 200 and result.get("status") == "found":
                break
            time.sleep(0.15)
        self.require(status == 200 and result.get("status") == "found", f"initial pull snapshot was not published: {result}")
        status, decision = self.evaluate("pullcase")
        self.require(status == 200 and decision.get("decision") == "allow", f"pull-backed policy did not allow from its complete snapshot: {decision}")
        original_record = result.get("record", {})
        original_expiry = original_record.get("expires_at")
        time.sleep(1.2)
        status, retried = self.fact("directory", "pull-old")
        self.require(status == 200 and retried.get("record", {}).get("expires_at") == original_expiry,
                     f"same pull source revision renewed its TTL: {retried}")

        self.upstream_state.set(b"{bad json", raw=True)
        time.sleep(1.5)
        status, result = self.fact("directory", "pull-old")
        self.require(status == 200 and result.get("status") == "found", f"malformed pull discarded prior generation: {result}")

        self.upstream_state.set(snapshot(2, unix_now(), {"pullcase": valid_fact(), "pull-new": valid_fact()}))
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            status, current = self.fact("directory", "pull-new")
            if status == 200 and current.get("status") == "found":
                break
            time.sleep(0.15)
        self.require(status == 200 and current.get("status") == "found", f"fresh pull generation was not published: {current}")
        status, old = self.fact("directory", "pull-old")
        self.require(status == 200 and old.get("status") != "found", f"snapshot replacement kept removed key: {old}")

    def test_restart_persistence(self) -> None:
        observed = unix_now()
        scalar_values = {"scalar_bool": True, "scalar_string": "alice", "scalar_int": 7,
                         "scalar_strings": ["critical", "release"]}
        status, setup = self.write_case("restart-check", valid_fact(), scalar_values, observed)
        self.require(status == 200, f"restart check setup failed: {setup}")
        status, before = self.evaluate("restart-check")
        self.require(status == 200 and before.get("decision") == "allow", f"fresh fact did not allow before restart: {before}")
        self.stop_server()
        self.start_server()
        self.wait_for_ready()
        status, result = self.fact("approvals", "restart-check")
        self.require(status == 200 and result.get("status") == "found", f"fact did not persist after restart: {result}")
        status, pulled = self.fact("directory", "pullcase")
        self.require(status == 200 and pulled.get("status") == "found", f"pull generation did not persist after restart: {pulled}")
        status, decision = self.evaluate("restart-check")
        self.require(status == 200 and decision.get("decision") == "allow", f"fresh fact did not allow after restart: {decision}")


def scope() -> dict[str, str]:
    return {"tenant": TENANT, "environment": ENVIRONMENT}


def input_schema() -> dict[str, Any]:
    string = {"type": "string"}
    return {"type": "record", "fields": {
        "resource": {"schema": {"type": "record", "fields": {
            "id": {"schema": string, "optional": False},
            "environment": {"schema": string, "optional": False},
        }}, "optional": False},
        "action": {"schema": {"type": "record", "fields": {
            "operation": {"schema": string, "optional": False},
        }}, "optional": False},
    }}


def schemas_for_acceptance() -> dict[str, Any]:
    string = {"type": "string"}
    return {"type": "record", "fields": {
        "active": {"schema": {"type": "bool"}, "optional": False},
        "owner": {"schema": string, "optional": False},
        "priority": {"schema": {"type": "int"}, "optional": False},
        "labels": {"schema": {"type": "array", "items": string, "max_items": 10}, "optional": False},
        "metadata": {"schema": {"type": "record", "fields": {
            "level": {"schema": string, "optional": False},
            "zone": {"schema": string, "optional": True},
        }}, "optional": False},
        "approvers": {"schema": {"type": "array", "max_items": 20, "items": {"type": "record", "fields": {
            "principal_id": {"schema": string, "optional": False},
            "role": {"schema": string, "optional": False},
            "team": {"schema": string, "optional": False},
        }}}, "optional": False},
    }}


def approval_policy() -> str:
    return '''language 2
policy RuntimeFactShapes: "Checks typed fact values and bounded aggregates"
  triggers when
    input.resource.environment == "test"
    and input.action.operation == "Deploy"
    and input.resource.id != "pullcase"
  facts
    approval = data.approvals[input.resource.id] fresh within 10s
    active = data.scalar_bool[input.resource.id] fresh within 10s
    owner = data.scalar_string[input.resource.id] fresh within 10s
    priority = data.scalar_int[input.resource.id] fresh within 10s
    labels = data.scalar_strings[input.resource.id] fresh within 10s
  requires
    active.value == true
    and owner.value == "alice"
    and priority.value >= 5
    and "critical" in labels.value
    and approval.value.metadata.level == "high"
    and count_distinct(a.principal_id for a in approval.value.approvers where a.role == "senior") >= 2
'''


def pull_policy() -> str:
    return '''language 2
policy PulledRuntimeFact: "Checks the published HTTPS collection"
  triggers when
    input.resource.environment == "test"
    and input.action.operation == "Deploy"
    and input.resource.id == "pullcase"
  facts
    directory = data.directory[input.resource.id] fresh within 30s
  requires
    directory.value.active == true
'''


def valid_fact() -> dict[str, Any]:
    return {
        "active": True,
        "owner": "alice",
        "priority": 7,
        "labels": ["critical", "release"],
        "metadata": {"level": "high"},
        "approvers": [
            {"principal_id": "alice", "role": "senior", "team": "security"},
            {"principal_id": "bob", "role": "senior", "team": "platform"},
        ],
    }


def input_value(key: str) -> dict[str, Any]:
    return {"resource": {"id": key, "environment": "test"}, "action": {"operation": "Deploy"}}


def put(key: str, value: Any, source_revision: int, expected_revision: int | None, observed_at: int, ttl_seconds: int) -> dict[str, Any]:
    return {"operation": "put", "key": key, "value": value, "expected_revision": expected_revision,
            "source_revision": source_revision, "observed_at": observed_at, "ttl_seconds": ttl_seconds}


def snapshot(source_revision: int, observed_at: int, items: dict[str, Any]) -> dict[str, Any]:
    return {"source_revision": source_revision, "observed_at": observed_at, "ttl_seconds": 60,
            "items": [{"key": key, "value": value} for key, value in items.items()]}


def free_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def unix_now() -> int:
    return int(time.time())


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path, default=pathlib.Path("target/debug/ipe-server"), help="ipe-server executable")
    parser.add_argument("--report", type=pathlib.Path, default=pathlib.Path("target/runtime-data-acceptance.json"), help="JSON report destination")
    args = parser.parse_args()
    return RuntimeDataAcceptance(args.binary, args.report).run()


if __name__ == "__main__":
    sys.exit(main())
