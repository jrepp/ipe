# Repository details

Read this before changing runtime, integration, or shared boundary behavior.

### Current implementation and older material

- The implemented runtime uses language-2 policies, scoped facts, immutable snapshots, and expiring evidence.
- The integration workload supports bounded ServiceNow metadata discovery through Rust CLI and MCP clients. It does not yet publish enforcement facts.
- Operator-configured JavaScript/TypeScript adapters use a sandbox and native broker for permitted reads. They do not publish facts or perform writes.
- The local `github-contract` CLI captures GitHub workflow and action evidence. It does not execute actions or publish policies.
- Read each RFC's status and the implementation limits in the current guides. A proposal is not an implemented feature.
- The legacy parser, bytecode interpreter, and JIT were removed. Several documents still describe them.
- [QUICKSTART.md](../QUICKSTART.md), [.agent/core-prompt.md](../.agent/core-prompt.md), and some workflow examples contain historical architecture and commands.
- Use this guide for repository orientation. Check older instructions against current source, manifests, tests, and CI before using them.
- UI mocks, synthetic provider tests, and archived benchmarks do not establish production readiness or capacity.

## Code map

| Location | Responsibility |
| --- | --- |
| [ipe-core](../crates/ipe-core/src/) | Runtime facts, policy documents, language-2 evaluation, approvals, and relationships |
| [runtime_facts.rs](../crates/ipe-core/src/runtime_facts.rs) | Fact storage, revisions, expiry, and snapshots |
| [runtime_policy.rs](../crates/ipe-core/src/runtime_policy.rs) | Policy language, validation, evaluation, and budgets |
| [policy_document.rs](../crates/ipe-core/src/policy_document.rs) | Immutable policy revisions, policy data, and activation |
| [ipe-server](../crates/ipe-server/src/) | Scoped HTTP API, authentication, bootstrap configuration, and pull connectors |
| [ipe-integration](../crates/ipe-integration/src/) | Integration runtime, HTTPS broker, WASM adapter host, and CLI/MCP toolkit |
| [Adapters](../crates/ipe-integration/adapters/README.md) | WASM metadata adapter contract and source |
| [script.rs](../crates/ipe-integration/src/script.rs) and [script_service.rs](../crates/ipe-integration/src/script_service.rs) | JavaScript/TypeScript sandbox and native operation broker |
| [SDK](../crates/ipe-integration/sdk/) and [package cache](../tools/integration-packages/) | Embedded guest SDK and build-side npm artifact cache; see the [SDK guide](../docs/INTEGRATION_SDK.md) |
| [github_contract.rs](../crates/ipe-integration/src/github_contract.rs) | Bounded GitHub capture, sandboxed declaration projection, and local review reports |
| [ipe-json](../crates/ipe-json/src/lib.rs) | Strict JSON decoding shared across trust boundaries |
| [testing/integration](../testing/integration/README.md) | TypeScript provider stand-in and independent protocol client |
| [acceptance](../acceptance/) and [scripts](../scripts/) | External acceptance fixtures, reports, and documentation tooling |

`ipe-parser`, `ipe-control`, `ipe-web`, `ipe-wasm`, and `ipe-ffi` are placeholders. Check their source before assigning implementation work to them.

## Runtime and integration boundaries

- Preserve tenant/environment isolation and independent grants for fact writes, evaluation, inspection, and policy management.
- Treat caller input as caller-attested data. It does not establish end-user identity or authoritative external facts.
- Evaluate one immutable snapshot. Preserve finite fact TTLs and the evidence-deadline check before an allow decision.
- Preserve decision precedence: deny, then indeterminate, then allow. Missing or expired evidence must not become an allow.
- Keep fact publication separate from policy activation. An external policy description does not authorize local enforcement changes.
- Preserve strict JSON decoding, byte limits, deadlines, and bounded work. Duplicate keys must remain invalid at every nesting level.
- Integration discovery must preserve unknown, unavailable, and incomplete source information. A failed read does not establish policy removal.
- Keep script operation grants and budget checks in Rust. SDK validation does not authorize requests or replace native checks.
- Preserve sandbox isolation. Guests must not receive credentials or direct network/filesystem access. Package caching does not authorize activation.
- Keep credentials in protected configuration or environment handoffs. Do not expose them in arguments, URLs, reports, logs, or errors.
- Use synthetic providers for local checks. State separately whether a live external system was tested.

## Rust and dependencies

Use `Result` and `Option` for expected failures. Avoid production panics and unchecked `unwrap()` calls.
Justify new `unsafe` code and document its safety conditions.

Keep dependency changes scoped. Preserve `Cargo.lock` unless the task requires an update.
Do not use a broad dependency update to fix an unrelated build failure.
