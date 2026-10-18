# Integration discovery vertical-slice acceptance

Run the real Rust Integration Workload and Rust CLI/MCP against the local
TypeScript HTTPS ServiceNow facade:

```sh
cargo build --locked -p ipe-integration --bins
cargo test --locked -p ipe-integration
cargo test --locked -p ipe-integration --test discovery_acceptance -- --ignored
cargo run --locked -p ipe-integration --example integration-demo -- --acceptance --report target/integration-discovery-report.json
```

Rust owns acceptance and process orchestration. Prerequisites are Rust, Node
>=22.18 and OpenSSL. No Python is needed. Tests tear down temporary processes,
certificates and configuration. The example accepts `--binary`, `--toolkit` and
`--report` to select executables and a JSON report destination.

Checks cover credential-first discovery, hosted WASM classification, CLI/MCP,
authentication, wrong connection references, strict request parameters, absent
publication/enforcement routes, bounded metadata queries, provider authentication,
stable opaque identities across restart, and output/log redaction. Both configured
candidates and inventory without known tables are exercised. A delayed facade
verifies cumulative run deadlines, concurrent-run rejection and permit release.
Provider faults cover denial, unauthorized access, 500, 404, empty results, invalid
envelopes, malformed JSON, oversized bodies, redirects and full row-limit responses.
Unavailable evidence must remain unknown.

Native unit tests reject WASM imports, start functions, memory growth, unbounded
execution and invalid semantic-role output; they verify configurable replacement
modules. Rust toolkit tests cover hostile/truncated responses, redirected credential
exfiltration, endpoint validation and MCP protocol boundaries. The TypeScript
facade's own tests and type checking are documented in
[testing/integration](../../testing/integration/README.md).

```sh
cargo run --locked -p ipe-integration --example integration-demo -- --serve
```

The interactive demo prints its local API address, opaque reference and private
operator token-file path. Source that file before invoking
`target/debug/integration-toolkit --url <printed address> capabilities`. Ctrl-C
stops the deployment and removes generated files.

These tests establish a local single-connection boundary with synthetic metadata.
They do not establish multi-tenant authorization, production credential storage,
OAuth onboarding, live ServiceNow ACL compatibility, durable discovery history,
complete workflow semantics, an agent package deployment service or enforcement
publication.
