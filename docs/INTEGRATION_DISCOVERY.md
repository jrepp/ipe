# Standalone integration discovery

See the [stack and crate reuse audit](INTEGRATION_STACK.md) for implemented
dependencies, the completed `rmcp` migration and remaining reuse priorities.

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
cargo test --locked -p ipe-json -p ipe-integration
cargo test --locked -p ipe-integration --test discovery_acceptance -- --ignored
cargo run --locked -p ipe-integration --example integration-demo -- --report target/integration-demo-report.json
```

The Rust acceptance test launches both Rust binaries and the TypeScript HTTPS
facade. It checks authenticated reads, hostile provider responses, strict input,
masked identities, deadline cancellation, CLI, and MCP. Unit tests additionally
exercise the WASM sandbox and client transport boundaries. Synthetic fixtures are
not live ServiceNow validation.

The independent external-client replay starts a fresh deployment, exercises CLI,
HTTP and handwritten MCP requests, saves a sanitized report, and tears down the
processes:

```sh
cargo run --locked -p ipe-integration --example integration-demo -- --external-agent --report target/external-agent-validation.json
```

CI runs this replay and retains deployment reports as artifacts. Changes to the
shared `ipe-json` boundary also trigger integration checks. A replay exercises the
client developed during the independent agent pass; it does not launch a new
reasoning agent or establish OS sandbox isolation. Live ServiceNow validation
remains separate.

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
The server uses the official `rmcp` SDK for protocol negotiation and typed tools,
including compatibility with MCP `2025-11-25`, and exposes:

- `integration_capabilities`
- `integration_check_connection` with `connection_ref`
- `integration_discover` with `connection_ref`

Complete `initialize`, wait for its response, then send `notifications/initialized`
before tool calls. Keep stdin open until outstanding responses arrive. Request
IDs correlate responses; concurrent replies need not preserve request order. Messages
are newline-delimited JSON-RPC; stdout contains protocol messages only. A narrow
transport guard retains bounded frames, recursive duplicate-key rejection and
sanitized SDK errors. The SDK owns lifecycle, dispatch and cancellation. The same
workload authorization applies to CLI and MCP. Streamable HTTP, durable tasks,
dynamic package deployment and per-user elevation remain future capabilities.

The stdio guard accepts frames up to 64 KiB and at most eight outstanding request
IDs. Exceeding that count or reusing an outstanding ID closes the session; clients
must reconnect. Serialized replies are capped at 8 MiB (including both structured
and text content), with a five-second write deadline. The SDK may drain in-flight
handlers for up to five seconds on input closure. Invalid typed tool parameters
can return `isError` tool results rather than JSON-RPC parameter errors; diagnostics
remain sanitized. Pre-initialization requests follow SDK lifecycle rules.

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

Catalog results retain every distinct host-generated coverage issue in
`coverage_codes`; each resource separately reports `field_coverage_codes`. Empty
arrays mean no issue was observed within the bounded read, not complete coverage.
The summary `code` and `field_code` retain `record_limit` whenever truncation was
observed; other failures remain in the arrays, including malformed/duplicate
metadata, metadata gaps, and guest traps or invalid output. A later failure cannot
hide an earlier limit. Clients must inspect both resource and field coverage;
an observed resource with incomplete fields is not a complete schema.

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

## CLI, secret ownership and shutdown

The workload, toolkit and deployment example use `clap` for argument parsing.
Existing commands and flags remain available; `--help` lists them. Parse failures
return exit code 2 and the fixed `invalid_command_arguments` diagnostic on stderr,
without echoing supplied values. Toolkit `mcp --help` also uses stderr, keeping
stdout reserved for protocol traffic. `--url` overrides `IPE_INTEGRATION_URL`;
help does not display that environment value. The deployment example rejects
combining `--acceptance` and `--serve` because acceptance shuts down the workload
as part of its lifecycle checks.

The integration workload stores its API token, provider credential and identity
key in `secrecy::SecretString`; the toolkit uses the same wrapper for its bearer
token. Access is explicit at native validation, authentication, HMAC and HTTP
boundaries. Wrappers protect against accidental debug output and zeroize their
owned storage on drop. They do not erase process environment values or copies
made by HTTP/TLS libraries. The enforcement server also wraps API and connector
credentials in `SecretString`. These wrappers do not add a credential vault.

Ctrl-C, or SIGTERM on Unix, stops workload intake and lets active requests drain
for up to 13 seconds, covering the existing 12-second request deadline plus cleanup.
An overrun produces a sanitized `shutdown_deadline` failure. CLI entrypoints return
normally rather than bypassing destructors with `process::exit`. The toolkit bounds
Tokio runtime teardown after protocol completion because a platform stdin read
may not be cancelable.

The local deployment harness uses Tokio process handles, bounded async readiness,
and explicit cleanup that kills and reaps children before returning. `kill_on_drop`
is a fallback for interrupted paths. Acceptance verifies successful SIGTERM drain
while discovery is in progress; the interactive demo closes its children and
removes temporary files after Ctrl-C. This is local process lifecycle management,
not durable integration jobs or a deployment controller.

The enforcement server binds its listener before starting tracked connector and
cleanup workers. `CancellationToken` and Tokio `JoinSet` coordinate shutdown;
a publication fence prevents canceled connector responses from replacing snapshots
or renewing freshness. HTTP drain allows 13 seconds, followed by a two-second
worker join and a two-second abort/join fallback. These are cooperative async
limits: an already-running synchronous storage or OS call cannot be preempted.
The publication fence is outside policy evaluation.

Toolkit and enforcement input boundaries share `ipe-json`, a small Serde visitor
that rejects duplicate decoded keys recursively before typed deserialization.
It retains serde_json's depth limit and returns opaque errors. Each caller still
owns byte limits, request schemas and authorization.
