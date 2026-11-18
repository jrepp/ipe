---
title: Shared foundations and permissive Rust package survey
author: IPE maintainers
created: 2026-11-17
tags: [architecture, dependencies, research]
id: memo-001
project_id: ipe
doc_uuid: cac1bb4e-0b21-4459-909d-e357169920f6
---

# Recommendation

Reuse established packages before writing general-purpose infrastructure.
Keep the current stack where it meets the contract. Evaluate a new package only against a named gap and observable acceptance criteria.
Do not add all shortlisted crates, upgrade to every latest release, or create a general shared framework from this survey.
This memo responds to the project owner's shared-needs and reuse requests on 2026-11-17.

The [project map](../PROJECTS.md) defines P1–P5. The [repository map](../../docs/REPOSITORY.md) identifies existing packages.
The [integration audit](../../docs/INTEGRATION_STACK.md) retains its implementation decisions; this memo expands the survey across lanes.
No dependencies or runtime behavior change in this research slice.

# Security follow-up, 2026-11-17

The CI security pass found evidence that changes this survey's initial recommendations.
[RUSTSEC-2026-0543](https://github.com/RustSec/advisory-db/blob/main/crates/chrono/RUSTSEC-2026-0543.md) marks every Chrono version unmaintained.
Chrono is excluded from new adoption. IPE now uses Jiff for UTC timestamps.
The latest official MCP SDK still requires Chrono. The project owner selected a reviewed local SDK patch using a supported time library.
The [patch record](../../vendor/rmcp/IPE-PATCH.md) preserves the upstream source, license, archive checksum, exact delta, and removal condition.
The initial registry snapshot below is retained as historical evidence; release activity alone did not establish acceptable maintenance.

The CI remediation also updates the MCP SDK to 3.11.0, libz-sys to 1.1.33, rustix to 1.1.6, and removes pin-utils through compatible parent updates.
Wasmi 2.0.0 was subsequently reported yanked; the selected patch release is 2.0.1 with the same feature set.
The resulting workspace passed `cargo audit --deny warnings` and `cargo deny --locked --all-features check`.
The all-feature suite passed 363 tests, with two external acceptance tests ignored. Cross-platform CI remains a separate result.

# Selection method and evidence limits

The shortlist considers ecosystem adoption, release activity, project stewardship, current-stack fit, and license metadata.
“Leading” means a credible established candidate for this use, not a claim of exhaustive market ranking.
Download counts include automated and transitive downloads. They do not measure quality or unique users.
Release dates and repository pushes are maintenance signals, not evidence of review quality or a support commitment.
Apalis has less registry adoption than the major infrastructure crates; it is a specialist candidate for evaluation, not an equal-confidence default.

Registry facts below were retrieved from the official crates.io API on 2026-11-17.
Each package link selects the observed release. The version is the registry's maximum stable release, not an adoption recommendation.
Locked versions come from the initial survey checkout's Cargo.lock, before the security follow-up. Presence in the lockfile can be transitive and does not prove application use.
Repository status was checked for 27 principal incumbents and candidates through GitHub's repository API; details appear below.
API documentation supplies the capability evidence linked in the matrix and tradeoff sections.
Candidate dependency graphs, feature combinations, native code licenses, and security advisories have not all been audited.

# Shared needs and package choices

Each row defines shared capability, not a proposed new IPE crate. Owner means the responsible project lane, not an assigned person.
Licenses and observed versions for every named candidate appear in the registry table.

| Shared need | Consumers; owner | Preferred reuse and alternatives | IPE contract and next decision |
| --- | --- | --- | --- |
| Async runtime, worker ownership, shutdown | P2, P3, P4; P3 | Keep tokio and tokio-util. Use existing JoinSet and CancellationToken; TaskTracker is an option for tasks needing tracked shutdown. | Preserve bounded drain and publication fences. Do not build another executor or task framework. |
| HTTP APIs, clients, transport security | P2, P3, P4; P2 | Keep axum, tower, tower-http, reqwest, and rustls. | Scope, disclosure, redirects, byte limits, and response contracts remain explicit. Separate Axum migration from feature work. |
| Concurrency, rate limits, retries | P2, P3; P2 | Evaluate Tower layers first; governor for keyed quotas; backon for approved retry loops. | Preserve immediate rejection versus queued readiness. Define idempotency and total deadlines before enabling retries. |
| Data contracts, schemas, safe decoding | P1–P4; P1 | Keep serde, serde_json, and rmcp's schemars support. Evaluate jsonschema only for actual JSON Schema inputs. | Keep ipe-json duplicate rejection and bounded input. Generic schemas do not define policy semantics; disable unapproved remote schema retrieval. |
| Object identities, revisions, provenance | P1–P4; P1 | uuid for opaque IDs; existing hmac, sha2, and subtle for scoped deterministic references. | Name identity domains and revision rules once. UUIDs do not replace scoped HMAC identifiers. Never write custom cryptographic primitives. |
| Secrets and credentials | P2–P4; P2 | Keep secrecy and zeroize. Use configured secret references and an external secret system when required. | Prevent serialization and logging of secret values. These crates do not provide a secret vault, rotation service, or identity provider. |
| Login, federation, token validation | P1–P4; P4 | openidconnect for OIDC; oauth2 for OAuth flows; jsonwebtoken for a defined JWT verification boundary. | Choose the actual protocol first. Configure issuer, audience, algorithms, key rotation, clock tolerance, and disclosure. Do not write token parsers. |
| Grants, access policies, exceptions | P1, P3, P4; P4 | Compare cedar-policy first for declarative authorization; casbin for RBAC/ABAC models and adapters. | Test semantic fit before extending our policy language. Keep issuer authority, explicit exception eligibility, expiry, and revocation contracts. |
| Timestamps, deadlines, calendar windows | P1–P4; P4 | Use std::time for deadlines and jiff for UTC records. Evaluate Jiff calendar rules and time against specific needs. Chrono is excluded after the security review. | Use monotonic deadlines for elapsed work. Define DST ambiguity, clock jumps, evidence expiry, and exception expiry separately. |
| Durable facts and policy revisions | P1–P3; P3 | Keep rocksdb for existing fact storage. Compare redb only if a pure-Rust store solves a measured deployment problem. | Require migration, crash recovery, snapshot consistency, fencing, and atomic writes. A new database is not a routine cleanup. |
| Queryable audit and operations records | P2–P4; P4 | Compare rusqlite for local relational records with sqlx for a selected async SQL backend. Reuse RocksDB where it already fits. | Decide query, retention, transaction, and deployment needs before choosing storage. Tracing logs are not the durable audit record. |
| Scheduling and durable execution | P2, P4; P2 | Keep Tokio timers for current local periodic collection. Compare apalis for durable jobs; async-nats/JetStream for a required broker-backed delivery topology. | These solve different problems. Define restart, acknowledgement, duplicate work, fencing, and expiry first; do not write a queue broker. |
| Evidence topology and impact traversal | P1, P2, P4; P4 | Evaluate petgraph for graph data structures, cycle detection, and traversal. | IPE owns node roles, provenance, scope filtering, and bounded traversal. Do not build generic graph algorithms or infer enforcement from a graph edge. |
| Configuration, CLI, and errors | P2, P3, P5; P3 | Keep clap, Serde, thiserror, and anyhow. Compare figment with config only when layered configuration is required. | Preserve strict parsing, precedence, secret references, and sanitized errors. Do not add a layering engine for one configuration file. |
| Adapter runtimes and SDK protocols | P2, P4; P2 | Keep wasmi 2.0.1, rquickjs, and the patched official rmcp. Compare wasmtime only for a demonstrated execution or component-model need. | Keep host capabilities, limits, and grants in native code. A runtime or protocol SDK does not authorize operations. |
| Telemetry and operational diagnostics | P2–P5; P3 | Keep tracing/tracing-subscriber and existing prometheus. Add opentelemetry plus a compatible tracing-opentelemetry/exporter set when needed. Compare prometheus-client or metrics before changing metric APIs. | Agree on correlation IDs, redaction, cardinality, and export budgets. Do not install multiple instrumentation stacks without a consumer. |
| Shared fixtures, property tests, and performance | P1–P5; P5 | Keep tempfile and criterion. Add proptest for invariants, insta for stable redacted snapshots, wiremock for ordinary HTTP fixtures; evaluate cargo-nextest for execution. | Keep independent protocol clients and malformed-wire tests. Snapshot approval cannot replace behavioral assertions. |
| Package compatibility and supply-chain checks | P1–P5; P5 | Keep Cargo/Cargo.lock, cargo-deny, and cargo-audit. Use cargo_metadata for Rust graph tooling and semver for Rust version requirements if needed. | Existing npm tooling owns integration-package resolution. Rust semver is not an npm resolver; do not build a second package manager. |
| Site publishing and shared product contracts | P5 with P1–P4; P5 | Keep static HTML/JS, the existing site build, Docuchango, and GitHub Pages. Rust schema types can supply published contracts where required. | No Cargo web framework is needed for a static site. Keep frontend rendering separate from domain authority and trace claims to source. |

The existing crate set already supplies most infrastructure primitives. The remaining work is largely contract selection and integration.
See [Tower](https://docs.rs/tower/0.5.3/tower/) for middleware and [Tokio task tracking](https://docs.rs/tokio-util/0.7.20/tokio_util/task/struct.TaskTracker.html) for shutdown mechanisms.
See [petgraph](https://docs.rs/petgraph/0.8.3/petgraph/) for reusable graph representations and algorithms.

# Decisions that require comparison

## Authorization must earn custom code

[Cedar](https://docs.rs/cedar-policy/4.15.0/cedar_policy/) provides an authorization language, evaluator, and schema support.
[Casbin](https://docs.rs/casbin/2.20.0/casbin/) provides policy models, enforcement interfaces, and adapters.
Compare these against actual IPE scenarios before adding another general permission evaluator.
The comparison must cover deny precedence, incomplete evidence, expiry, scoped objects, exception rules, and diagnostic output.
Neither library alone establishes collector provenance or proves that an application applied a decision.
Retain current language-2 behavior until an explicit compatibility design and migration proof justify a change.

[OpenID Connect](https://docs.rs/openidconnect/4.1.0/openidconnect/) handles an identity protocol; it is not the same responsibility as policy evaluation.
[jsonwebtoken](https://docs.rs/jsonwebtoken/12.0.0/jsonwebtoken/) exposes verification primitives and configurable crypto providers.
Do not select a JWT crate merely because an access-grant record exists; first decide whether that record is a signed token.
Use ed25519-dalek only if a reviewed protocol needs direct Ed25519 signatures; prefer the protocol SDK's supported verification path.

## Time and durable work are separate contracts

[Jiff](https://docs.rs/jiff/1.0.0/jiff/) supports time-zone-aware date/time operations. Its reviewed release is recent; evaluate migration cost before adopting it.
IPE now uses Jiff for UTC records after the Chrono advisory. A date library cannot choose our clock-skew or offline-revocation policy.

The existing host uses Tokio intervals with skipped missed ticks. Restart does not replay missed runs or refresh evidence.
[Apalis](https://docs.rs/apalis/1.2.0/apalis/) offers job processing and selectable storage backends.
[JetStream](https://docs.rs/async-nats/0.53.0/async_nats/jetstream/index.html) provides broker-backed persistence and acknowledgement mechanisms.
Do not equate broker delivery guarantees with exactly-once external effects. Expired observations must remain expired after retries or recovery.
Select one approach only after the deployment and recovery contracts require it.

## Storage, configuration, and schemas

[redb](https://docs.rs/redb/4.6.0/redb/) offers a pure-Rust transactional embedded store; it is an alternative to evaluate, not a reason to replace working RocksDB.
For relational records, compare rusqlite's local SQLite API with sqlx's asynchronous database access against the chosen topology.
Native libraries and optional database drivers require their own dependency and license review.

[Figment](https://docs.rs/figment/0.10.19/figment/) and [config](https://docs.rs/config/0.15.27/config/) both support configuration composition.
Prefer the existing strict typed loader until file/environment/profile precedence is a real requirement.
[jsonschema](https://docs.rs/jsonschema/0.61.2/jsonschema/) supports external references; configure bounded offline resolution for untrusted inputs.
It does not replace recursive duplicate-key rejection or IPE's narrow domain field checks.

# License and compatibility findings

The registry table preserves exact declared license expressions. MIT, Apache-2.0, BSD, and ISC options satisfy this shortlist's permissive-license filter.
For Jiff, select the MIT option rather than requiring an Unlicense allowance.
Wasmi and Wiremock publish the older MIT/Apache-2.0 spelling; verify the selected release's license files during adoption.
Wasmtime declares Apache-2.0 WITH LLVM-exception. Keep that exact expression in any review; do not shorten it to plain Apache-2.0.
Its observed release requires Rust 1.97, above ipe-integration's declared minimum of 1.96. Do not adopt it without resolving that compatibility gap.

A permissive top-level crate does not clear all transitive code, native libraries, certificate data, or enabled features.
The existing [deny.toml](../../deny.toml) also allows MPL-2.0 for project crates; MPL-2.0 is not classified as permissive in this survey.
The webpki-root-certs data exception is version-scoped and retains its separate notice requirement.
Do not expand license exceptions merely to make a candidate pass.
See [cargo-deny license configuration](https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html) for graph checks and scoped exceptions.

On 2026-11-17, `cargo deny --locked check licenses` passed for the current workspace graph.
It reported one unused Unicode-DFS-2016 allowance. No allowlist change was needed.
This run does not cover dependencies that are only candidates here, nor does it establish advisory status.
The table below records the initial survey. The security follow-up and current Cargo.lock take precedence for adoption.

# Adoption order and proof

| Order | Project owner | Concrete next outcome | Required proof |
| --- | --- | --- | --- |
| 1 | P2, P3 | Map existing HTTP limits, shutdown, and schema handling to the current crates before adding helpers. | Keep response codes, bounded work, cancellation fences, and strict decoding tests. |
| 2 | P4 with P1/P3 | Compare Cedar/Casbin and identity SDKs against temporary authority and disclosure scenarios. | Run allow, deny, unknown, expired, revoked, wrong-scope, and malformed-token cases. Record semantic gaps before custom code. |
| 3 | P2 with P3 | Specify evidence publication recovery; compare existing storage/timers with durable jobs or broker delivery if needed. | Crash/restart, replay, stale delivery, duplicate acknowledgement, fencing, and expired evidence tests. |
| 4 | P4 | Select calendar and topology primitives for the system map and time controls. | DST ambiguity, clock changes, bounded cycles, cross-scope filtering, and missing provenance cases. |
| 5 | P5 with P1–P4 | Add only the selected observability and testing tools. | Sanitized output, bounded cardinality, reproducible tests, and licensed artifacts in CI. |

Before adopting a candidate, record the exact release and enabled features in the owning design.
Check Rust versions, target platforms, dependency duplication, license files, native components, and current advisories on that resolved graph.
Run focused compatibility tests and the affected acceptance suite. Use measured evidence for performance claims.
Prefer a thin adapter over a general wrapper framework. Reuse existing project types before creating a common crate.
If reuse fails, record the failed requirement and the smallest custom mechanism needed to satisfy it.

# Registry snapshot

“Recent downloads” is the crates.io API's reported recent_downloads field, rounded to millions; it is not a popularity rank.
“Not declared” means the registry supplied no rust_version value, not that any toolchain will work.
“—” in the lock column means the package was absent from this checkout's Cargo.lock.

| Package and observed release | Locked version(s) | Declared license | Minimum Rust | Release date | Recent downloads |
| --- | --- | --- | --- | --- | --- |
| [tokio 1.54.1](https://crates.io/crates/tokio/1.54.1) | 1.53.3 | MIT | 1.85 | 2026-10-21 | 315.06m |
| [tokio-util 0.7.20](https://crates.io/crates/tokio-util/0.7.20) | 0.7.19 | MIT | 1.85 | 2026-10-09 | 244.62m |
| [axum 0.8.9](https://crates.io/crates/axum/0.8.9) | 0.7.9 | MIT | 1.80 | 2026-04-14 | 168.73m |
| [tower 0.5.3](https://crates.io/crates/tower/0.5.3) | 0.4.13, 0.5.2 | MIT | 1.64.0 | 2026-01-12 | 248.24m |
| [tower-http 0.7.1](https://crates.io/crates/tower-http/0.7.1) | 0.5.2, 0.6.11 | MIT | 1.65 | 2026-08-31 | 227.35m |
| [reqwest 0.13.5](https://crates.io/crates/reqwest/0.13.5) | 0.13.5 | MIT OR Apache-2.0 | 1.85.0 | 2026-09-08 | 271.80m |
| [rustls 0.23.46](https://crates.io/crates/rustls/0.23.46) | 0.23.46 | Apache-2.0 OR ISC OR MIT | 1.71 | 2026-10-14 | 291.01m |
| [serde 1.0.229](https://crates.io/crates/serde/1.0.229) | 1.0.228 | MIT OR Apache-2.0 | 1.56 | 2026-07-18 | 441.10m |
| [serde_json 1.0.154](https://crates.io/crates/serde_json/1.0.154) | 1.0.145 | MIT OR Apache-2.0 | 1.71 | 2026-10-11 | 443.62m |
| [schemars 1.2.3](https://crates.io/crates/schemars/1.2.3) | 1.2.3 | MIT | 1.74 | 2026-10-12 | 256.85m |
| [jsonschema 0.61.2](https://crates.io/crates/jsonschema/0.61.2) | — | MIT | 1.85.0 | 2026-11-12 | 42.89m |
| [uuid 1.29.3](https://crates.io/crates/uuid/1.29.3) | 1.28.0 | Apache-2.0 OR MIT | 1.89.0 | 2026-11-17 | 265.67m |
| [secrecy 0.10.3](https://crates.io/crates/secrecy/0.10.3) | 0.10.3 | Apache-2.0 OR MIT | 1.60 | 2024-10-09 | 60.17m |
| [zeroize 1.9.2](https://crates.io/crates/zeroize/1.9.2) | 1.9.1 | Apache-2.0 OR MIT | 1.85 | 2026-11-17 | 266.86m |
| [hmac 0.13.0](https://crates.io/crates/hmac/0.13.0) | 0.12.1 | MIT OR Apache-2.0 | 1.85 | 2026-03-29 | 239.65m |
| [sha2 0.11.1](https://crates.io/crates/sha2/0.11.1) | 0.10.9 | MIT OR Apache-2.0 | 1.88 | 2026-10-12 | 395.06m |
| [subtle 2.6.1](https://crates.io/crates/subtle/2.6.1) | 2.6.1 | BSD-3-Clause | Not declared | 2024-06-24 | 242.59m |
| [jsonwebtoken 12.0.0](https://crates.io/crates/jsonwebtoken/12.0.0) | — | MIT | 1.88.0 | 2026-11-13 | 76.07m |
| [oauth2 5.0.0](https://crates.io/crates/oauth2/5.0.0) | — | MIT OR Apache-2.0 | 1.65 | 2025-01-21 | 19.86m |
| [openidconnect 4.1.0](https://crates.io/crates/openidconnect/4.1.0) | — | MIT | 1.88 | 2026-10-31 | 6.00m |
| [time 0.3.55](https://crates.io/crates/time/0.3.55) | — | MIT OR Apache-2.0 | 1.88.0 | 2026-08-01 | 252.65m |
| [chrono 0.4.45](https://crates.io/crates/chrono/0.4.45) | 0.4.42 | MIT OR Apache-2.0 | 1.62.0 | 2026-06-04 | 246.41m |
| [jiff 1.0.0](https://crates.io/crates/jiff/1.0.0) | — | Unlicense OR MIT | 1.85 | 2026-11-15 | 122.50m |
| [rocksdb 0.25.0](https://crates.io/crates/rocksdb/0.25.0) | 0.22.0 | Apache-2.0 | 1.88.0 | 2026-08-16 | 8.18m |
| [redb 4.6.0](https://crates.io/crates/redb/4.6.0) | — | MIT OR Apache-2.0 | 1.90 | 2026-11-13 | 7.06m |
| [rusqlite 0.43.0](https://crates.io/crates/rusqlite/0.43.0) | — | MIT | 1.89.0 | 2026-11-15 | 63.52m |
| [sqlx 0.9.0](https://crates.io/crates/sqlx/0.9.0) | — | MIT OR Apache-2.0 | 1.94.0 | 2026-05-21 | 59.46m |
| [tracing 0.1.44](https://crates.io/crates/tracing/0.1.44) | 0.1.41 | MIT | 1.65.0 | 2025-12-18 | 280.43m |
| [tracing-subscriber 0.3.23](https://crates.io/crates/tracing-subscriber/0.3.23) | 0.3.20 | MIT | 1.65.0 | 2026-03-13 | 199.73m |
| [opentelemetry 0.33.1](https://crates.io/crates/opentelemetry/0.33.1) | — | Apache-2.0 | 1.75.0 | 2026-10-09 | 91.54m |
| [tracing-opentelemetry 0.34.0](https://crates.io/crates/tracing-opentelemetry/0.34.0) | — | MIT | 1.75.0 | 2026-09-23 | 66.07m |
| [prometheus-client 0.26.0](https://crates.io/crates/prometheus-client/0.26.0) | — | Apache-2.0 OR MIT | Not declared | 2026-11-11 | 12.35m |
| [metrics 0.24.6](https://crates.io/crates/metrics/0.24.6) | — | MIT | 1.71.1 | 2026-05-13 | 30.21m |
| [clap 4.6.8](https://crates.io/crates/clap/4.6.8) | 4.6.7 | MIT OR Apache-2.0 | 1.85 | 2026-11-16 | 294.05m |
| [figment 0.10.19](https://crates.io/crates/figment/0.10.19) | — | MIT OR Apache-2.0 | Not declared | 2024-05-17 | 13.86m |
| [config 0.15.27](https://crates.io/crates/config/0.15.27) | — | MIT OR Apache-2.0 | 1.88.0 | 2026-09-30 | 23.52m |
| [wasmi 2.0.0](https://crates.io/crates/wasmi/2.0.0) | 2.0.0 | MIT/Apache-2.0 | 1.86 | 2026-09-01 | 6.74m |
| [wasmtime 50.0.2](https://crates.io/crates/wasmtime/50.0.2) | — | Apache-2.0 WITH LLVM-exception | 1.97.0 | 2026-11-16 | 13.22m |
| [rquickjs 0.14.0](https://crates.io/crates/rquickjs/0.14.0) | 0.14.0 | MIT | 1.87 | 2026-09-18 | 4.05m |
| [rmcp 3.11.0](https://crates.io/crates/rmcp/3.11.0) | 3.5.1 | Apache-2.0 | 1.88 | 2026-11-17 | 29.91m |
| [proptest 1.12.0](https://crates.io/crates/proptest/1.12.0) | — | MIT OR Apache-2.0 | 1.88 | 2026-10-10 | 79.12m |
| [insta 1.49.0](https://crates.io/crates/insta/1.49.0) | — | Apache-2.0 | 1.66.0 | 2026-10-03 | 40.89m |
| [wiremock 0.6.5](https://crates.io/crates/wiremock/0.6.5) | — | MIT/Apache-2.0 | Not declared | 2025-08-24 | 36.69m |
| [tempfile 3.27.0](https://crates.io/crates/tempfile/3.27.0) | 3.23.0 | MIT OR Apache-2.0 | 1.63 | 2026-03-11 | 261.21m |
| [cargo-deny 0.21.0](https://crates.io/crates/cargo-deny/0.21.0) | — | MIT OR Apache-2.0 | 1.89.0 | 2026-11-16 | 2.52m |
| [cargo-audit 0.23.0](https://crates.io/crates/cargo-audit/0.23.0) | — | Apache-2.0 OR MIT | 1.88 | 2026-11-10 | 4.51m |
| [cargo-nextest 0.9.152](https://crates.io/crates/cargo-nextest/0.9.152) | — | Apache-2.0 OR MIT | 1.91 | 2026-11-13 | 1.90m |
| [criterion 0.8.2](https://crates.io/crates/criterion/0.8.2) | 0.5.1 | Apache-2.0 OR MIT | 1.86 | 2026-02-04 | 82.13m |
| [cedar-policy 4.15.0](https://crates.io/crates/cedar-policy/4.15.0) | — | Apache-2.0 | 1.89 | 2026-11-16 | 4.36m |
| [casbin 2.20.0](https://crates.io/crates/casbin/2.20.0) | — | Apache-2.0 | Not declared | 2026-02-04 | 1.00m |
| [governor 0.10.4](https://crates.io/crates/governor/0.10.4) | — | MIT | Not declared | 2025-12-16 | 24.00m |
| [backon 1.6.0](https://crates.io/crates/backon/1.6.0) | — | Apache-2.0 | 1.85 | 2025-10-18 | 44.72m |
| [apalis 1.2.0](https://crates.io/crates/apalis/1.2.0) | — | MIT OR Apache-2.0 | Not declared | 2026-11-14 | 0.48m |
| [prometheus 0.14.0](https://crates.io/crates/prometheus/0.14.0) | 0.14.0 | Apache-2.0 | 1.81 | 2025-03-27 | 31.27m |
| [petgraph 0.8.3](https://crates.io/crates/petgraph/0.8.3) | — | MIT OR Apache-2.0 | 1.64 | 2025-09-30 | 149.67m |
| [thiserror 2.0.21](https://crates.io/crates/thiserror/2.0.21) | 1.0.69, 2.0.21 | MIT OR Apache-2.0 | 1.77 | 2026-09-23 | 542.25m |
| [anyhow 1.0.104](https://crates.io/crates/anyhow/1.0.104) | 1.0.104 | MIT OR Apache-2.0 | 1.68 | 2026-07-18 | 298.62m |
| [semver 1.0.28](https://crates.io/crates/semver/1.0.28) | 1.0.28 | MIT OR Apache-2.0 | 1.68 | 2026-04-04 | 294.10m |
| [cargo_metadata 0.23.1](https://crates.io/crates/cargo_metadata/0.23.1) | — | MIT | 1.86.0 | 2025-11-12 | 73.97m |
| [async-nats 0.53.0](https://crates.io/crates/async-nats/0.53.0) | — | Apache-2.0 | 1.89.0 | 2026-11-17 | 6.96m |
| [ed25519-dalek 3.0.0](https://crates.io/crates/ed25519-dalek/3.0.0) | — | BSD-3-Clause | 1.85 | 2026-07-06 | 87.90m |

# Repository maintenance snapshot

These repositories reported neither archived nor disabled at review time. Push dates include non-release work and automation.
This is a bounded maintenance check; maintainer responsiveness, issue quality, and support arrangements were not independently assessed.
Packages absent from this table received registry release checks only.

| Package | Upstream repository | Last push observed |
| --- | --- | --- |
| tokio | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | 2026-11-17 |
| axum | [tokio-rs/axum](https://github.com/tokio-rs/axum) | 2026-11-17 |
| rustls | [rustls/rustls](https://github.com/rustls/rustls) | 2026-11-16 |
| serde | [serde-rs/serde](https://github.com/serde-rs/serde) | 2026-11-16 |
| jsonschema | [Stranger6667/jsonschema](https://github.com/Stranger6667/jsonschema) | 2026-11-17 |
| jsonwebtoken | [Keats/jsonwebtoken](https://github.com/Keats/jsonwebtoken) | 2026-11-13 |
| oauth2 | [ramosbugs/oauth2-rs](https://github.com/ramosbugs/oauth2-rs) | 2026-02-22 |
| openidconnect | [ramosbugs/openidconnect-rs](https://github.com/ramosbugs/openidconnect-rs) | 2026-10-31 |
| jiff | [BurntSushi/jiff](https://github.com/BurntSushi/jiff) | 2026-11-17 |
| rocksdb | [rust-rocksdb/rust-rocksdb](https://github.com/rust-rocksdb/rust-rocksdb) | 2026-11-17 |
| redb | [cberner/redb](https://github.com/cberner/redb) | 2026-11-17 |
| rusqlite | [rusqlite/rusqlite](https://github.com/rusqlite/rusqlite) | 2026-11-15 |
| sqlx | [launchbadge/sqlx](https://github.com/launchbadge/sqlx) | 2026-11-17 |
| opentelemetry | [open-telemetry/opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust) | 2026-11-17 |
| figment | [SergioBenitez/Figment](https://github.com/SergioBenitez/Figment) | 2026-10-13 |
| config | [rust-cli/config-rs](https://github.com/rust-cli/config-rs) | 2026-11-16 |
| wasmi | [wasmi-labs/wasmi](https://github.com/wasmi-labs/wasmi) | 2026-11-14 |
| rquickjs | [DelSkayn/rquickjs](https://github.com/DelSkayn/rquickjs) | 2026-11-17 |
| rmcp | [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) | 2026-11-17 |
| proptest | [proptest-rs/proptest](https://github.com/proptest-rs/proptest) | 2026-10-10 |
| cedar-policy | [cedar-policy/cedar](https://github.com/cedar-policy/cedar) | 2026-11-17 |
| casbin | [casbin/casbin-rs](https://github.com/casbin/casbin-rs) | 2026-11-15 |
| governor | [boinkor-net/governor](https://github.com/boinkor-net/governor) | 2026-08-24 |
| backon | [Xuanwo/backon](https://github.com/Xuanwo/backon) | 2026-11-12 |
| apalis | [apalis-dev/apalis](https://github.com/apalis-dev/apalis) | 2026-11-14 |
| petgraph | [petgraph/petgraph](https://github.com/petgraph/petgraph) | 2026-11-15 |
| async-nats | [nats-io/nats.rs](https://github.com/nats-io/nats.rs) | 2026-11-17 |

[Shared platform research spike](memo-004-shared-platform-spike.md) adds adoption order, source findings, and current dependency blockers to this survey.
