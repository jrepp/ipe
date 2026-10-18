# Integration stack and crate reuse audit

Reviewed 2026-10-18; updated after MCP, CLI, shared JSON, secret-wrapper and enforcement worker lifecycle migrations.
This records the implemented stack and the next dependency choices. Rows marked
implemented describe branch code; remaining recommendations
are not installed dependencies or completed migrations.

## Stack and ownership

| Layer | Implemented today | Direction |
| --- | --- | --- |
| Enforcement | Existing `ipe-core` policy/fact semantics and `ipe-server` | Preserve local evaluation; no MCP or adapter execution on the evaluation path |
| Integration workload | Rust `ipe-integration`, Tokio, Axum, reqwest with rustls | Keep native connection/authentication, bounded reads, identity generation and safe projection |
| Agent protocol | Rust `integration-toolkit` using `rmcp` 3.5.1 with typed tools | SDK owns lifecycle/dispatch; a bounded strict transport guard owns frame and disclosure limits |
| HTTP MCP | Not implemented | Mount `rmcp` Streamable HTTP as a Tower service in the Integration Workload's Axum router |
| CLI | `clap` derives in workload, toolkit and demo | Keep sanitized parse errors and MCP channel isolation |
| Sandbox | Wasmi interpreter, binary WASM, bundled WAT compiled at build time | Keep the bounded metadata ABI; expand only with explicit capability contracts |
| Serialization | Shared `ipe-json` over Serde/serde_json; schemars-generated MCP schemas | Keep recursive duplicate-key rejection and sanitized errors at untrusted JSON boundaries |
| Identity and cryptography | `hmac`, `sha2`, `subtle`; `secrecy` in integration and enforcement | Keep native scope/domain separation and explicit secret exposure |
| Local proof | Rust/Tokio async child lifecycle and `tempfile`; TypeScript HTTPS provider facade | Bounded readiness/cleanup; provider semantics stay in TypeScript |

The native workload is the security boundary around provider connections. WASM
receives permitted field metadata, never credentials, origin URLs or identity
keys. The test facade is an external synthetic provider, not a guest runtime.
See [discovery operations](INTEGRATION_DISCOVERY.md) and the
[implemented WASM ABI](../crates/ipe-integration/adapters/README.md).

The recommended MCP SDK is the [official `rmcp`](https://github.com/modelcontextprotocol/rust-sdk).
Its [StreamableHttpService](https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/tower/index.html)
implements Tower service integration; it also supports Tokio stdio and a reqwest
client transport. One typed tool implementation should serve both transports.
Authorization and execution limits belong at the operation boundary, so stdio
cannot bypass controls enforced only by HTTP middleware.

Current lockfile: Axum 0.7.9, Tokio 1.48.0, Hyper 1.7.0, reqwest 0.13.5,
rustls 0.23.46 and Wasmi 2.0.0. Tower resolves both 0.4.13 and 0.5.2;
tower-http resolves both 0.5.2 and 0.6.11. These are observed resolutions, not a
recommendation to pin old versions indefinitely. The SDK migration uses published `rmcp` 3.5.1 (Apache-2.0, declared Rust minimum
1.88), with server/macros/stdio features and a development-only SDK client. It
does not add an HTTP transport or change Axum. The dependency-tree review found
no new HTTP-stack version introduced by this migration; cargo-audit reported no
vulnerabilities or warnings for the resulting lockfile. Recheck these findings
when updating dependencies. Do not copy a git-main dependency into production or assume upgrading
Axum is behavior-neutral: the existing enforcement routes use 0.7 path syntax.
Keep an Axum migration separate from adopting MCP stdio unless the chosen SDK
actually requires it.

## Findings and decisions

The audit covered the integration workload, CLI, WASM host, Rust test harness,
adjacent enforcement HTTP ingestion/lifecycle code, strict JSON parsing, and fact
persistence. It is a targeted source review, not a repository-wide security audit
or certification of crate maintenance. “Prefer” means a concrete migration
candidate with primary documentation, subject to version and compatibility checks.

| Priority | Current contribution and evidence | Crate reuse decision | Behavior that must survive |
| --- | --- | --- | --- |
| Implemented | [Toolkit](../crates/ipe-integration/src/bin/integration-toolkit.rs): custom protocol state machine removed | `rmcp` handles protocol and typed tools; strict framing guard remains | Same tool authority, bounded input/output, sanitized failures, stdout protocol-only, no secret exposure; validate SDK behavior for duplicate keys and malformed frames |
| Implemented | Toolkit `run`, [workload main](../crates/ipe-integration/src/main.rs), [demo](../crates/ipe-integration/examples/integration-demo.rs) | [`clap` Parser/Subcommand](https://docs.rs/clap/latest/clap/_derive/index.html) replaces manual argument loops | Existing command names, token environment references, private config path; MCP startup failures remain on stderr without echoing sensitive input |
| Implemented | Toolkit input schemas previously handwritten separately from validation | [`schemars`](https://docs.rs/schemars/latest/schemars/) derives schemas from SDK parameter types | Generated schema and deserialization agree; deny unknown fields; do not describe private operator configuration as agent inputs |
| Implemented | Integration `Runtime`, toolkit `Client`, and enforcement server `Credential`/`Connector` use secret wrappers | [`secrecy`](https://docs.rs/secrecy/latest/secrecy/) with `zeroize` | Explicit exposure at validation, authorization and authorized-request boundaries; no debug/serialization leakage. Wrapping does not erase environment variables or copies made by HTTP libraries |
| Implemented | Integration signal drain; [enforcement server `run`](../crates/ipe-server/src/runtime.rs) owns cleanup/connector workers and starts them only after binding succeeds | Tokio `JoinSet` and [`CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html), with bounded async drain and join waits | Cancellation fences connector publication and expired-fact cleanup; pending network responses cannot publish after that boundary. The fence is outside the evaluation hot path. Async deadlines cannot preempt synchronous storage; cancellation does not provide durable jobs |
| P2 | [Integration `deadline`](../crates/ipe-integration/src/lib.rs), per-operation semaphore | Evaluate Tower/tower-http timeout, concurrency and load-shed layers; retain small wrappers where semantics differ | Existing 504 `run_deadline`, immediate 429 for concurrent discovery, permit release on cancellation. Tower readiness can queue rather than reject; timeout layers can return empty bodies. No blanket timeout on long-lived MCP streams |
| Implemented | [Shared `ipe-json`](../crates/ipe-json/src/lib.rs) replaces the toolkit and enforcement server visitors | One recursive Serde visitor exposes typed and `Value` decoding with opaque errors; [`serde_with` duplicate prevention](https://docs.rs/serde_with/latest/serde_with/rust/maps_duplicate_key_is_error/index.html) remains an option for isolated typed map fields | Reject nested and escaped-equivalent duplicate keys, trailing data, invalid JSON and excess depth. Callers retain byte limits and domain validation. A field-level map helper cannot replace recursive `serde_json::Value` validation |
| Implemented | [Harness](../crates/ipe-integration/examples/support/mod.rs): blocking readiness reader and custom polling removed | Tokio child handles, async wait/read, timeout, explicit bounded kill/reap and kill-on-drop fallback | Reap both children, bounded startup/output, private files, Ctrl-C cleanup, environment isolation. Keep synthetic provider behavior in TypeScript |

The [tower-http timeout documentation](https://docs.rs/tower-http/latest/tower_http/timeout/index.html)
distinguishes service timeouts from body-transfer timeouts. A middleware swap must
be checked against the selected version and IPE's observable response contract.
It does not replace WASM fuel or interrupt synchronous guest execution.

## Keep IPE-owned behavior

- **Connection and disclosure policy.** reqwest already provides HTTP/TLS, URL
  parsing and redirect/proxy settings. Fixed native recipes, allowlisted operations,
  response byte budgets and safe field projection are our integration contract.
  Share those policies only where scopes match; a generic HTTP SDK does not enforce them.
- **Identity semantics.** HMAC/SHA-256 and constant-time comparison already come from
  crates. The stable masked reference format, connection/domain separation and
  rotation behavior remain ours. Random UUIDs are not a substitute for deterministic
  scope-separated references.
- **WASM capabilities.** Wasmi supplies execution and metering. Import denial,
  memory/fuel limits, fresh instances, ABI validation and finite output projection
  remain host policy. Adding WASI to simplify guest authoring would change the boundary.
- **Fact and policy semantics.** [FactStore](../crates/ipe-core/src/runtime_facts.rs)
  already uses RocksDB for durable writes. Revision fencing, source ownership,
  tombstones, expiry and atomic publication are not duplicated database machinery.
  Neither a generic cache nor a JSON Schema validator replaces them. The narrow
  `FieldSchema` is a domain type checker, not an incomplete general JSON Schema engine.
- **Provider facades.** Keep the lightweight TypeScript stand-in for ServiceNow
  envelopes, faults and hostile fields. Native Rust socket fixtures are useful for
  malformed HTTP/stdio boundaries; replacing all of them with a mock server can
  make truncated-wire cases impossible to exercise.

Avoid adding retry frameworks now: automatic retry is a policy decision tied to
idempotency, request budgets and freshness. The existing bounded discovery path
intentionally does not promise retries. Likewise, the `ipe-control` gRPC module is
only a stub, not an existing custom RPC implementation to replace.

## Migration order and proof

1. **Implemented:** `rmcp` 3.5.1 protocol machinery and typed tools. Live acceptance
   uses an SDK client; independent wire tests cover malformed/fragmented frames,
   typed-argument rejection, sanitized errors and outstanding-request limits.
   SDK conformance alone does not prove IPE authorization or masking.
2. **Implemented:** `clap`, generated MCP schemas, and `secrecy` in integration
   and enforcement. Keep public command and error contracts stable; record any
   intentional change. Reject source URLs, arbitrary operations and extra
   parameters before contacting the provider.
3. **Implemented:** integration signal drain and async harness cleanup, plus
   enforcement `CancellationToken`/`JoinSet` worker ownership. The enforcement
   publication fence coordinates connector commits and cleanup with cancellation;
   evaluation does not acquire this lock. An in-progress synchronous storage call
   can delay the fence and cannot be preempted by an async timeout. Evaluate
   middleware replacement against the existing 12-second discovery deadline and
   concurrent-run acceptance cases.
4. **Implemented:** shared `ipe-json` decoding in the toolkit and enforcement
   server. Tests cover nested objects/arrays, escaped-equivalent duplicate keys,
   trailing input, invalid JSON, depth limits, typed decoding and error redaction.
   Byte budgets remain the caller's responsibility; this migration does not change
   integration provider/configuration parsing semantics.

For each dependency: use a published release and lockfile; inspect license,
Rust/toolchain compatibility, advisory results and enabled features; run
`cargo tree -d` and explain duplicate HTTP stack versions. Enable only needed SDK
transports/capabilities. Keep build-time WAT and test-only tooling out of production
runtime features. These are adoption checks, not a claim that every candidate has
already passed a supply-chain review.

The existing [acceptance suite](../acceptance/integration-discovery/README.md) is
the regression baseline. The SDK migration preserves CLI commands and tool operations; its documented
protocol differences include concurrent response ordering, strict SDK initialization,
and typed parameter failures that may use `isError` results. See the discovery
guide for frame, outstanding-request and shutdown bounds.

CLI/secret migration dependencies are published `clap` 4.6.7 (minimal parsing/help
features) and `secrecy` 0.10.3. Cargo audit reports no vulnerabilities or warnings
for this lockfile. The workload drains for at most 13 seconds on shutdown; the
harness uses explicit async cleanup, and toolkit runtime teardown is bounded for
platform stdin reads that cannot be canceled. No new task framework was needed
inside the integration workload. Enforcement uses `tokio-util` cancellation and
Tokio task ownership; these changes do not complete the durable integration
lifecycle roadmap.
