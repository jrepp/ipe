#!/usr/bin/env python3
"""Local HTTPS ServiceNow metadata stand-in. Synthetic only; never contacts ServiceNow."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import ssl
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

SECRET = "provider-secret-DO-NOT-DISCLOSE"
TOPOLOGY = "internal-remediation.corp.example"
RAW_ID = "1234567890abcdef1234567890abcdef"


class Fixture:
    def __init__(self, directory, expected_token=SECRET):
        self.mode = "success"
        self.delay_seconds = 0
        self.expected_token = expected_token
        self.requests = []
        config = directory / "tls.cnf"
        config.write_text("[req]\ndistinguished_name=dn\nx509_extensions=ext\n[dn]\n[ext]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n")
        self.cert = directory / "ca.crt"
        ca_key = directory / "ca.key"
        key = directory / "server.key"
        csr = directory / "server.csr"
        server_cert = directory / "server.crt"
        extensions = directory / "server.ext"
        extensions.write_text("subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n")
        commands = [
            ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-keyout", str(ca_key), "-out", str(self.cert), "-subj", "/CN=Fixture CA", "-config", str(config)],
            ["openssl", "req", "-newkey", "rsa:2048", "-nodes", "-keyout", str(key), "-out", str(csr), "-subj", "/CN=localhost"],
            ["openssl", "x509", "-req", "-in", str(csr), "-CA", str(self.cert), "-CAkey", str(ca_key), "-CAcreateserial", "-days", "1", "-extfile", str(extensions), "-out", str(server_cert)],
        ]
        for command in commands:
            subprocess.run(command, check=True, capture_output=True)
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                fixture.requests.append((self.path, dict(self.headers)))
                time.sleep(fixture.delay_seconds)
                body = fixture.payload(self.path)
                status = {"denied": 403, "unauthorized": 401, "failure": 500, "unsupported": 404, "redirect": 302}.get(fixture.mode, 200)
                if self.headers.get("Authorization") != "Bearer " + fixture.expected_token:
                    status = 401
                    body = b'{"error":{"message":"authentication required"}}'
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                if fixture.mode == "redirect":
                    self.send_header("Location", fixture.url + "/forbidden-redirect")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                try:
                    self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError, ssl.SSLError):
                    pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.load_cert_chain(server_cert, key)
        self.server.socket = tls.wrap_socket(self.server.socket, server_side=True)
        self.url = f"https://127.0.0.1:{self.server.server_port}"
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def payload(self, path):
        if self.mode == "malformed":
            return b'{"result": ['
        if self.mode == "oversize":
            return b"x" * (2 * 1024 * 1024)
        if self.mode not in ("success", "empty", "truncated", "wrong_shape"):
            return json.dumps({"error": {"message": SECRET, "detail": TOPOLOGY}}).encode()
        if self.mode == "wrong_shape":
            return json.dumps({"result": {"secret": SECRET}}).encode()
        if self.mode == "empty":
            return b'{"result": []}'
        table = urlparse(path).path.rsplit("/", 1)[-1]
        rows = {
            "sys_db_object": [{"sys_id": RAW_ID, "name": "sn_vul_vulnerable_item", "label": f"{SECRET} at https://{TOPOLOGY}", "super_class": {"value": RAW_ID, "link": f"https://{TOPOLOGY}/api/now/table/task/{RAW_ID}"}}],
            "sys_dictionary": [{"sys_id": RAW_ID, "name": "sn_vul_vulnerable_item", "element": "state", "internal_type": "choice", "mandatory": "true", "read_only": "false", "column_label": SECRET, "reference": TOPOLOGY}],
            "sys_choice": [{"sys_id": RAW_ID, "name": "sn_vul_vulnerable_item", "element": "state", "value": "1", "label": SECRET, "inactive": "false"}],
        }.get(table, [])
        if self.mode == "truncated":
            limit = int(parse_qs(urlparse(path).query).get("sysparm_limit", ["100"])[0])
            rows = (rows or [{"sys_id": RAW_ID}]) * limit
        return json.dumps({"result": rows}).encode()

    def stop(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, help="Keep generated TLS fixture files here")
    parser.add_argument("--mode", default="success", choices=["success", "empty", "truncated", "wrong_shape", "denied", "unauthorized", "failure", "unsupported", "redirect", "malformed", "oversize"])
    args = parser.parse_args()
    temporary = tempfile.TemporaryDirectory(prefix="ipe-servicenow-standin-") if args.directory is None else None
    directory = args.directory or Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    fixture = Fixture(directory)
    fixture.mode = args.mode
    print(json.dumps({"url": fixture.url, "ca_certificate": str(fixture.cert), "mode": args.mode}), flush=True)
    try:
        threading.Event().wait()
    except KeyboardInterrupt:
        pass
    finally:
        fixture.stop()
        if temporary:
            temporary.cleanup()


if __name__ == "__main__":
    main()
