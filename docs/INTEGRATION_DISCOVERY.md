# Standalone integration discovery

The independent Rust `ipe-integration` workload hosts a WASM metadata adapter and
an authenticated native HTTPS broker. Rust CLI and MCP clients expose bounded
ServiceNow discovery. TypeScript provides the synthetic provider facade for local
testing. This slice does not publish enforcement facts or claim to recover complete
ServiceNow business rules.

## Local deployment proof

Prerequisites: Rust/Cargo, Node >=22.18, and OpenSSL. No Python, real ServiceNow
credential, or enterprise installation is needed for this integration slice.

```sh
cargo build --locked -p ipe-integration --bins
cargo test --locked -p ipe-integration
cargo test --locked -p ipe-integration --test discovery_acceptance -- --ignored
cargo run --locked -p ipe-integration --example integration-demo -- --report target/integration-demo-report.json
```

The Rust acceptance test launches both Rust binaries and the TypeScript HTTPS
facade. It checks authenticated reads, hostile provider responses, strict input,
masked identities, deadline cancellation, CLI, and MCP. Unit tests additionally
exercise the WASM sandbox and client transport boundaries. Synthetic fixtures are
not live ServiceNow validation.

For a standalone acceptance report or interactive deployment:

```sh
cargo run --locked -p ipe-integration --example integration-demo -- --acceptance --report target/integration-discovery-report.json
cargo run --locked -p ipe-integration --example integration-demo -- --serve
```

The launcher reports the local workload address, opaque connection reference and
an operator-only temporary environment file. Source that file in a second shell;
use the printed address and reference with the CLI. Ctrl-C stops the deployment
and removes temporary credentials and certificates. `--binary` and `--toolkit`
can override the default executable paths next to the built example.

```sh
target/debug/integration-toolkit --url http://127.0.0.1:PORT capabilities
target/debug/integration-toolkit --url http://127.0.0.1:PORT check CONNECTION_REF
target/debug/integration-toolkit --url http://127.0.0.1:PORT discover CONNECTION_REF
```

The client reads `IPE_INTEGRATION_TOKEN`; bearer tokens are not command arguments.
JSON results distinguish observed metadata from unknown or unavailable coverage.

## MCP access

Configure a stdio MCP client to launch:

```text
/absolute/path/to/ipe/target/debug/integration-toolkit --url http://127.0.0.1:PORT mcp
```

Supply the workload token through the client's protected process environment.
The server negotiates MCP `2025-11-25` and exposes:

- `integration_capabilities`
- `integration_check_connection` with `connection_ref`
- `integration_discover` with `connection_ref`

Complete `initialize` and `notifications/initialized` before tool calls. Messages
are newline-delimited JSON-RPC; stdout contains protocol messages only. The same
workload authorization applies to CLI and MCP. Streamable HTTP, durable tasks,
dynamic package deployment and per-user elevation remain future capabilities.

## Operator configuration

```sh
cargo run --locked -p ipe-integration --bin ipe-integration -- /private/path/integration.json
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

This is private operator configuration, not policy data or agent output. Use
independent random secrets for the workload token and stable identity key, and a
separately issued ServiceNow bearer credential. Keep them out of source control
and command arguments. Retain the identity key and opaque connection reference
across restarts; rotating the identity key changes derived references.

The configured origin must be HTTPS. Optional `ca_cert_path` supplies a private CA;
verification remains enabled. Redirects and ambient proxies are disabled. Internal
origins are a trusted operator choice, not an agent-supplied URL. The control API
binds only to loopback HTTP; remote operation requires separately configured
authenticated TLS termination. This is not a production ingress or multi-tenant
credential-management system.

An empty resource list opts into bounded metadata inventory. The broker reads
fixed metadata routes and returns opaque resource/field references and recognized
type/role information. It does not expose table names, raw labels, reference links,
source record IDs, or source error text. Inventory is limited by credentials and
request budgets. Operators can instead configure explicit resource/table and
field mappings to restrict access further.

A successful connection check proves only that a metadata read succeeded. It does
not establish remediation-record access or discover complete workflow constraints.
Workflow semantics remain explicitly unknown.

## Hosted WASM adapters

The default ServiceNow field-role classifier executes inside the workload's Wasmi
interpreter. An operator can set `adapter_path` to a binary WASM module implementing
the [metadata ABI](../crates/ipe-integration/adapters/README.md). Loading a different
module and restarting the workload changes its integration interpretation without
changing native transport code. Capability and discovery output identify the
module by digest, not its physical path.

The initial ABI accepts a validated field identifier, recognized type and required
flag and returns a finite semantic-role code. Native Rust owns transport,
credentials, opaque identity, response shaping and output validation. Guests have
no imports, WASI, filesystem, environment or network access. Each discovery run
gets a fresh instance with bounded memory, stack and shared fuel. Guest traps or
invalid output become unknown coverage; they cannot publish facts or enforce policy.

This proves configurable sandboxed interpretation, not the complete future adapter
package lifecycle. Artifact signing, agent build/stage/deploy, hot upgrades,
shared-domain constraint projection, synchronization and durable forensic auditing
remain separate milestones.

See the [integration architecture and MVP](../rfcs/010-remediation-adapters-and-external-policy-discovery.md)
for the roadmap and focused design contracts.
