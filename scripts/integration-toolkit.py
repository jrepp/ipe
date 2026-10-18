#!/usr/bin/env python3
"""CLI and MCP stdio access to the standalone IPE Integration Workload.

The operator supplies IPE_INTEGRATION_TOKEN through the environment. This client
never accepts provider credentials or arbitrary provider destinations.
"""
from __future__ import annotations

import argparse
import http.client
import ipaddress
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

MAX_MESSAGE = 65536
MAX_RESPONSE = 1048576
PROTOCOL_VERSION = "2025-11-25"
REF_PATTERN = re.compile(r"^[A-Za-z0-9_-]{1,128}$")


class ToolkitError(Exception):
    def __init__(self, code):
        self.code = code
        super().__init__(code)


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result


def decode_json(data):
    return json.loads(data, object_pairs_hook=no_duplicates,
                      parse_constant=lambda _value: (_ for _ in ()).throw(ValueError("invalid number")))


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


class Client:
    def __init__(self, url, token_env):
        try:
            parsed = urllib.parse.urlsplit(url)
            _port = parsed.port
            if (parsed.scheme not in ("http", "https") or not parsed.hostname
                    or parsed.username or parsed.password or parsed.query or parsed.fragment
                    or parsed.path not in ("", "/")):
                raise ValueError()
            if parsed.scheme == "http" and not ipaddress.ip_address(parsed.hostname).is_loopback:
                raise ValueError()
        except ValueError:
            raise ToolkitError("invalid_workload_endpoint") from None
        token = os.environ.get(token_env, "")
        if not token or not token.isascii() or any(ord(c) <= 32 or ord(c) >= 127 for c in token):
            raise ToolkitError("missing_or_invalid_workload_credential")
        self.url = url.rstrip("/")
        self.token = token
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def call(self, operation, connection_ref=None):
        if operation == "capabilities":
            path, payload = "/v1/capabilities", None
        elif operation in ("check", "discover"):
            if not isinstance(connection_ref, str) or not REF_PATTERN.fullmatch(connection_ref):
                raise ToolkitError("invalid_connection_ref")
            path = "/v1/connections/check" if operation == "check" else "/v1/discovery"
            payload = json.dumps({"connection_ref": connection_ref}).encode()
        else:
            raise ToolkitError("unknown_operation")
        request = urllib.request.Request(self.url + path, data=payload, headers={
            "Authorization": "Bearer " + self.token,
            "Content-Type": "application/json", "Accept": "application/json",
        })
        try:
            with self.opener.open(request, timeout=35) as response:
                body = response.read(MAX_RESPONSE + 1)
                if len(body) > MAX_RESPONSE:
                    raise ToolkitError("workload_response_too_large")
                result = decode_json(body)
                if not isinstance(result, dict):
                    raise ToolkitError("invalid_workload_response")
                return result
        except urllib.error.HTTPError as error:
            # Do not echo bodies, URLs, headers or server reason text on errors.
            code = {401: "unauthenticated", 403: "forbidden", 404: "not_found",
                    409: "conflict", 422: "invalid_request", 429: "rate_limited"}.get(
                        error.code, "workload_request_failed")
            error.close()
            raise ToolkitError(code) from None
        except (urllib.error.URLError, TimeoutError, OSError, http.client.HTTPException):
            raise ToolkitError("workload_unavailable") from None
        except (ValueError, UnicodeError, RecursionError):
            raise ToolkitError("invalid_workload_response") from None


def tool_schema(name, description, has_ref):
    schema = {"type": "object", "properties": {}, "additionalProperties": False}
    if has_ref:
        schema["properties"]["connection_ref"] = {
            "type": "string", "pattern": REF_PATTERN.pattern,
            "description": "Opaque connection reference returned by integration_capabilities",
        }
        schema["required"] = ["connection_ref"]
    return {"name": name, "description": description, "inputSchema": schema,
            "annotations": {"readOnlyHint": True, "openWorldHint": has_ref}}


TOOLS = [
    tool_schema("integration_capabilities", "List authorized connections and the implemented discovery capabilities. No provider credentials are returned.", False),
    tool_schema("integration_check_connection", "Check configured ServiceNow metadata access through the native broker. This consumes a bounded upstream read; metadata access is not proof of business-record access.", True),
    tool_schema("integration_discover", "Discover bounded permitted ServiceNow metadata and explicit coverage gaps. Does not publish facts or execute discovered scripts.", True),
]
OPERATIONS = {"integration_capabilities": "capabilities",
              "integration_check_connection": "check", "integration_discover": "discover"}


def emit(value):
    sys.stdout.write(json.dumps(value, separators=(",", ":"), allow_nan=False) + "\n")
    sys.stdout.flush()


def rpc_error(request_id, code, message):
    emit({"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}})


def mcp(client):
    initialized = False
    ready = False
    while True:
        raw = sys.stdin.buffer.readline(MAX_MESSAGE + 1)
        if not raw:
            return 0
        if len(raw) > MAX_MESSAGE:
            rpc_error(None, -32700, "Message exceeds limit")
            return 1
        try:
            message = decode_json(raw)
        except (ValueError, UnicodeError, RecursionError):
            rpc_error(None, -32700, "Invalid JSON")
            continue
        if not isinstance(message, dict):
            rpc_error(None, -32600, "Invalid request")
            continue
        request_id = message.get("id")
        has_id = "id" in message
        if (message.get("jsonrpc") != "2.0" or not isinstance(message.get("method"), str)
                or (has_id and (isinstance(request_id, bool) or not isinstance(request_id, (str, int))))):
            rpc_error(None, -32600, "Invalid request")
            continue
        method = message["method"]
        params = message.get("params", {})
        if not isinstance(params, dict):
            if has_id:
                rpc_error(request_id, -32602, "Invalid params")
            continue
        if not has_id:
            if method == "notifications/initialized" and initialized:
                ready = True
            # No unsolicited messages, subscriptions, or cancellation claims.
            continue
        if method == "initialize":
            if initialized or not isinstance(params.get("protocolVersion"), str):
                rpc_error(request_id, -32602, "Invalid initialization")
                continue
            initialized = True
            result = {"protocolVersion": PROTOCOL_VERSION, "capabilities": {"tools": {}},
                      "serverInfo": {"name": "ipe-integration-toolkit", "version": "0.1.0"},
                      "instructions": "Initial discovery slice only. Credentials are operator-configured. Discovery is bounded and may be partial; no adapters or policy facts are deployed by these tools."}
        elif method == "ping":
            result = {}
        elif not ready:
            rpc_error(request_id, -32000, "Initialization required")
            continue
        elif method == "tools/list":
            result = {"tools": TOOLS}
        elif method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments", {})
            if not isinstance(name, str) or name not in OPERATIONS:
                rpc_error(request_id, -32602, "Unknown tool")
                continue
            operation = OPERATIONS[name]
            expected = set() if operation == "capabilities" else {"connection_ref"}
            if (not isinstance(arguments, dict) or set(arguments) != expected
                    or (expected and (not isinstance(arguments.get("connection_ref"), str)
                                      or not REF_PATTERN.fullmatch(arguments["connection_ref"])))):
                rpc_error(request_id, -32602, "Invalid tool arguments")
                continue
            try:
                data = client.call(operation, arguments.get("connection_ref"))
                result = {"content": [{"type": "text", "text": json.dumps(data)}],
                          "structuredContent": data, "isError": False}
            except ToolkitError as error:
                result = {"content": [{"type": "text", "text": json.dumps({"error": {"code": error.code}})}], "isError": True}
        else:
            rpc_error(request_id, -32601, "Method not found")
            continue
        emit({"jsonrpc": "2.0", "id": request_id, "result": result})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default=os.environ.get("IPE_INTEGRATION_URL", "http://127.0.0.1:9010"), help="Operator-configured Integration Workload origin; HTTP requires a numeric loopback address")
    parser.add_argument("--token-env", default="IPE_INTEGRATION_TOKEN", help="Environment variable containing the workload bearer token")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("capabilities")
    for name in ("check", "discover"):
        commands.add_parser(name).add_argument("connection_ref")
    commands.add_parser("mcp", help="Serve MCP over stdio (JSON lines)")
    args = parser.parse_args()
    try:
        client = Client(args.url, args.token_env)
        if args.command == "mcp":
            return mcp(client)
        emit(client.call(args.command, getattr(args, "connection_ref", None)))
        return 0
    except ToolkitError as error:
        output = {"error": {"code": error.code}}
        if args.command == "mcp":
            sys.stderr.write(json.dumps(output) + "\n")
        else:
            emit(output)
        return 1
    except (KeyboardInterrupt, BrokenPipeError):
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
