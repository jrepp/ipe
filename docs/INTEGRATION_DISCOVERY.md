# Standalone integration discovery

The initial slice runs an independent `ipe-integration` workload with CLI and MCP
access. It checks an operator-configured ServiceNow credential and discovers bounded
metadata through a native HTTPS broker. It does not execute agent-authored adapters,
publish enforcement facts, or claim to recover complete ServiceNow business rules.

## Local deployment proof

Prerequisites: Rust/Cargo, Python 3.10 or newer, and OpenSSL. No real ServiceNow
credential or enterprise installation is needed for the stand-in demonstration.

```sh
cargo build --locked -p ipe-integration
python3 scripts/accept-integration-discovery.py
python3 scripts/demo-integration-discovery.py
```

The demo launches the real workload and a lightweight HTTPS ServiceNow-style
stand-in, exercises API/CLI/MCP, writes `target/integration-demo-report.json`, and
stops the processes. Acceptance adds negative cases and writes its own report.
Synthetic fixtures are not live ServiceNow validation.

For an interactive local deployment:

```sh
python3 scripts/demo-integration-discovery.py --serve
```

The launcher reports the local workload address, opaque connection reference and
an operator-only temporary environment file. Source that file in a second shell;
use the printed address and reference with the CLI. Ctrl-C stops the deployment
and removes temporary credentials and certificates.

```sh
python3 scripts/integration-toolkit.py --url http://127.0.0.1:PORT capabilities
python3 scripts/integration-toolkit.py --url http://127.0.0.1:PORT check CONNECTION_REF
python3 scripts/integration-toolkit.py --url http://127.0.0.1:PORT discover CONNECTION_REF
```

`PORT` and `CONNECTION_REF` are placeholders for the launcher output. The client
reads `IPE_INTEGRATION_TOKEN`; it does not take bearer tokens as command arguments.
JSON results distinguish observed metadata from unknown or unavailable coverage.

## MCP access

Configure a stdio MCP client to launch:

```text
python3 /absolute/path/to/ipe/scripts/integration-toolkit.py --url http://127.0.0.1:PORT mcp
```

Supply the workload token through the client's protected process environment.
The server negotiates MCP `2025-11-25` and exposes only:

- `integration_capabilities`
- `integration_check_connection` with `connection_ref`
- `integration_discover` with `connection_ref`

The client must complete `initialize` and `notifications/initialized` before tool
calls. Messages are newline-delimited JSON-RPC; stdout contains protocol messages
only. This slice has no Streamable HTTP MCP endpoint, task/resumption interface,
dynamic package deployment, or per-user elevation service. The same workload
authorization applies to CLI and MCP.

## Operator configuration

The workload accepts one private JSON configuration path:

```sh
cargo run --locked -p ipe-integration -- /private/path/integration.json
```

A discovery-first configuration has no preselected business tables:

```json
{
  "listen": "127.0.0.1:9010",
  "api_token_env": "IPE_INTEGRATION_TOKEN",
  "connection_ref": "conn_random_opaque_identity",
  "origin": "https://your-authorized-instance.example",
  "credential_env": "SERVICENOW_TOKEN",
  "identity_key_env": "IPE_INTEGRATION_IDENTITY_KEY",
  "resources": []
}
```

This is operator configuration, not policy data or agent output. Use independent
random secrets for the workload token and stable identity key, and a separately
issued ServiceNow bearer credential. Keep them out of source control and command
arguments. Retain the identity key and opaque connection reference across restarts
to preserve discovered resource identities. Rotation of the identity key changes
those derived references and needs coordinated migration.

The configured origin must be HTTPS. An optional `ca_cert_path` supplies a private
CA for a fixture or authorized installation; verification remains enabled. Redirects
and ambient proxies are disabled. Internal origins are a trusted operator choice,
not an agent-supplied URL. The control API binds only to loopback HTTP; remote
operation requires separately configured authenticated TLS termination. This local
slice is not a production ingress or multi-tenant credential-management system.

An empty resource list explicitly opts into bounded metadata inventory. The broker
reads fixed metadata routes and returns opaque resource/field references and safe
recognized type/role information. It does not expose table names, raw labels,
reference links, source record IDs, or source error text. Inventory is limited by
the credential and request budgets; it is not an exhaustive catalog. Where the
operator wants narrower access, configure explicit resource/table and field mappings
instead; see the acceptance fixture configuration.

A successful connection check means the metadata read succeeded. It does not prove
permission to read remediation records, inspect all constraints, or perform an
operation. Workflow semantics remain explicitly unknown in this slice. Discovery
results guide the next selection of authorized records and constraint sources.

## What the first slice proves

The local deployment proves standalone workload execution, authenticated broker
reads, bounded schema discovery, opaque references, sanitized errors, and equivalent
CLI/MCP access. The reusable [stand-in](../testing/integration/README.md) provides
synthetic success, denied access, malformed responses and other failure modes.

Later milestones include: sandboxed adapter authoring and deployment, shared-domain
constraint projections, fact synchronization into enforcement, upgrades, and
forensic reconstruction. They are not implied by this discovery demonstration.
