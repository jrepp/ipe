---
title: Shared platform research spike and Rust adoption sequence
author: IPE maintainers
created: 2026-11-17
tags: [architecture, dependencies, platform, research]
id: memo-004
project_id: ipe
doc_uuid: a2069f88-7bea-449c-bf58-27a66a876df9
---

# Recommendation

Build on the existing Tokio, Axum, Serde, Rustls, RocksDB, Jiff, tracing, and sandbox stack.
The first shared platform work should connect evidence publication to enforcement, with explicit recovery, expiry, and scope contracts.
Do not introduce a platform framework, second policy language, broker, or database before its contract requires one.

This spike extends the [61-package survey](memo-001-shared-foundations.md) with source inspection, refreshed registry facts, dependency blockers, and ordered experiments.
It responds to the project owner's research request on 2026-11-17.
These are recommendations, not accepted architecture decisions or implemented product capabilities.
No workspace dependency or runtime code changes in this spike.

# Scope and evidence

The inspected baseline is commit `d258db5cdb28cf36ca6f095c88cfe8e1a771151d`.
Registry and upstream documentation were checked on 2026-11-18 UTC, still November 17 in the project timezone.
The [project map](../PROJECTS.md) defines P1 modeling, P2 integration, P3 enforcement, P4 evidence operations, and P5 delivery.

The review refreshed 30 package records and inspected direct dependencies for ten shortlisted packages.
The selection considers existing-stack fit, documented behavior, registry adoption, release history, and permissive license declarations.
It is not an exhaustive popularity ranking. Downloads include automation and transitive use; they do not establish quality.
The prior survey covers supporting CLI, secrets, hashing, configuration, schema, test, and sandbox packages.

## What the source establishes

| Observation | Evidence | Consequence |
| --- | --- | --- |
| Runtime facts already use RocksDB with synchronous writes. Each commit stores the complete serialized state under one key before memory publication. | [FactStore and persist_and_publish](../../crates/ipe-core/src/runtime_facts.rs) | Measure whole-state write cost before changing engines. RocksDB alone does not make the application use incremental storage. |
| The integration host persists a bounded JSON store through a synced temporary file and rename. Unix also syncs the directory. | [Host persist and scheduling](../../crates/ipe-integration/src/host/mod.rs) | A future audit/query store has a real migration boundary. Do not describe this as a transactional SQL store or durable job queue. |
| Host restart schedules future collection; missed intervals are not replayed. Workers and broker calls have explicit bounds. | [Host implementation](../../crates/ipe-integration/src/host/mod.rs), [integration guide](../../docs/integration-host.md) | Durable delivery must define acknowledgement and replay independently of collection scheduling. |
| Enforcement has cancellation, publication fencing, and a clock that accounts for elapsed time. | [Server runtime](../../crates/ipe-server/src/runtime.rs) | Reuse these boundaries. A generic timeout or calendar library cannot replace them. |
| The control crate is a placeholder. The integration host does not publish enforcement facts. | [Control crate](../../crates/ipe-control/src/lib.rs), [code map](../../.agent/repository.md) | Neither gRPC nor a distributed control plane is an implemented platform to standardize today. |
| Cargo.lock contains Tower 0.4 and 0.5, and tower-http 0.5 and 0.6. Axum is 0.7.9. | [Lockfile](../../Cargo.lock), [workspace manifest](../../Cargo.toml) | Plan HTTP compatibility as a separate migration. New middleware must match the service versions it wraps. |

# Shared needs and build-on choices

“Keep” means reuse the current implementation. “Evaluate” requires a bounded experiment before adoption. “Defer” means no dependency addition now.
Lane ownership below identifies a responsibility; it does not assign a person or team.

| Need and consumers | Recommendation | IPE-owned contract and acceptance evidence |
| --- | --- | --- |
| Task execution and shutdown; P2/P3/P4 | Keep tokio and tokio-util; use existing JoinSet, Semaphore, and CancellationToken. | Owned tasks, bounded admission, drain, and no publication after shutdown. Do not create an executor or generic worker framework. |
| HTTP and service limits; P1–P4 | Keep axum, tower, tower-http, reqwest, rustls. Evaluate governor only for keyed quotas and backon only for approved retries. | Immediate overload rejection, total deadline, body limits, redacted errors, allowed destinations, idempotency. Retrying must not renew evidence validity. |
| Typed contracts and strict input; P1–P4 | Keep serde, serde_json, ipe-json, and existing schema generation. | Separate source observations, publication envelopes, policy revisions, decisions, and applied outcomes. Preserve duplicate-key rejection and version compatibility. |
| Local state and snapshots; P1/P2/P3 | Keep rocksdb. Evaluate redb only for a measured native-build or operational problem. Retain arc-swap where already used. | Atomic publication, revision fencing, crash recovery, finite validity, and immutable evaluation snapshots. Do not promise lock-free behavior from a manifest entry. |
| Queryable audit and work records; P2/P4 | Evaluate rusqlite for one-process local storage; sqlx with PostgreSQL for a selected shared-service topology. | Ordered append, scope-filtered queries, retention, transactional outbox, backup and migration. Neither tracing nor object storage supplies this contract. |
| Durable publication; P2→P3 | Start with existing bounded HTTP and receiver snapshot semantics. Evaluate async-nats JetStream when durable fan-out or independent consumers become necessary. | Stable delivery ID, original observed/expiry times, acknowledged commit, replay, tombstones, source authority and receiver high-water marks. |
| Recoverable background jobs; P2/P4 | Keep current Tokio timers. Evaluate apalis with one selected backend if restartable jobs are required. | Lease/fencing rules, attempt budget, expiry on retry, cancellation, and crash recovery. Do not add both a job system and broker without distinct needs. |
| Identity and temporary access; P1/P3/P4 | Use an established identity provider. Evaluate openidconnect after its dependency blocker is resolved; jsonwebtoken only for an approved token profile. | Issuer, audience, algorithms, key rotation, object/action scope, revocation and expiry. JWT validation is not an identity provider or complete OIDC flow. |
| Authorization; P1/P3/P4 | Keep language-2 behavior. Evaluate cedar-policy for a bounded access-policy case; retain Casbin as the earlier survey's alternative. | Deny then indeterminate then allow; incomplete evidence, hard/soft controls, exception eligibility and diagnostics. No language replacement without semantic and migration proof. |
| Time conditions; P1–P4 | Keep Jiff for records and std::time::Instant for elapsed deadlines. | Clock rollback, exact expiry boundary, DST gaps/overlaps, timezone data, and offline authority. Current minimal Jiff features do not establish calendar-control support. |
| Evidence map and impact queries; P1/P2/P4 | Evaluate petgraph for graph storage and algorithms. | Distinct modeling/integration/enforcement identities, typed edges, scope filtering, bounded traversal, and explicit incomplete results. A graph edge does not prove enforcement. |
| Evidence payloads and exports; P2/P4/P5 | Evaluate object_store after its dependency blocker; compare OpenDAL when broader storage protocols are needed. | Immutable digest references, scoped metadata, conditional writes, authorized downloads, retention and deletion. A generic storage SDK must remain behind native grants. |
| Derived lookup caches; P1/P3/P4 | Defer moka until profiling identifies a repeatable expensive lookup. | Scope and policy revision in keys; recheck validity and revocation on use. Cache eviction cannot define evidence expiry or authorization. |
| Service protocols and HA; P2/P3/P4 | Defer tonic/prost until a gRPC interoperability need exists. Defer openraft until replicated authority is an approved requirement. | Message limits, schema evolution, authentication and recovery. Prefer managed storage/broker replication before operating application consensus. |
| Telemetry; P2–P5 | Keep tracing and prometheus. Evaluate opentelemetry with a matching tracing bridge and exporter when cross-service traces have a consumer. | Correlate observation, publication, decision and applied outcome. Bound label cardinality and export queues; redact credentials and sensitive evidence. |
| Verification and publishing; all lanes | Keep existing acceptance fixtures, Cargo audit/deny, Docuchango, static site and Pages. Evaluate proptest for cross-boundary invariants. | Failure-path proof and honest design status. A static documentation site does not need a Rust web framework. |

## Authorization: a real semantic mismatch

[Cedar's documented algorithm](https://docs.cedarpolicy.com/auth/authorization.html) uses default deny, forbid precedence, and skip-on-error behavior.
An errored policy does not participate in the result; diagnostics report the error.
A separate matching permit can therefore yield allow despite an errored policy.

IPE instead preserves deny, then indeterminate, then allow, with missing or expired evidence unable to establish permission.
This makes Cedar a comparison candidate, not a drop-in replacement.
A prototype must test error diagnostics and incomplete inputs before mapping its result into IPE.
Reject adoption if the mapping loses required explanations or requires a second general evaluator around Cedar.
Do not invent a custom JWT, cryptographic, or OAuth implementation to avoid this policy-language comparison.

## Publication: delivery does not establish authority

[JetStream](https://docs.rs/async-nats/0.53.0/async_nats/jetstream/) provides persisted messaging and acknowledgement mechanisms.
Its delivery semantics do not make an external side effect atomic with a local database commit.
IPE still needs idempotent receiver publication and a defined acknowledgement boundary.

Keep the first connected milestone on bounded HTTP unless restart and fan-out requirements justify a broker.
For durable source-side publication, commit the record and outbox entry in one selected storage transaction.
For a broker prototype, acknowledge after receiver commit, then kill the consumer before acknowledgement and verify safe replay.
Replay must preserve original validity and cannot revive an expired fact.
Use the existing [publication RFC](../../rfcs/014-fact-publication-and-synchronization.md) for domain semantics.

[Apalis](https://docs.rs/apalis/1.2.0/apalis/) is a job-processing ecosystem with backend choices.
It is a candidate when the missing requirement is recoverable work, rather than delivery of one fact revision to many consumers.
Its smaller registry adoption and backend-specific behavior warrant a narrower trial than the existing Tokio stack.

## Storage and caches: different responsibilities

[rusqlite](https://docs.rs/rusqlite/0.43.0/rusqlite/) is a local SQLite wrapper; [SQLx](https://docs.rs/sqlx/0.9.1/sqlx/) provides asynchronous SQL access.
Choose by deployment and transaction requirements, rather than adding both as interchangeable backends.
Keep blocking SQLite work off asynchronous request workers and bound queued database work.

[object_store](https://docs.rs/object_store/0.14.2/object_store/) offers cloud/local object APIs and conditional operations.
[OpenDAL](https://docs.rs/crate/opendal/0.59.5) offers many selectable storage services and layers.
Compare only the required backend and operations; enabling every backend increases the review surface.
Neither package replaces an authorization check before download or an audit index.

[Moka](https://docs.rs/moka/0.12.16/moka/) explicitly documents best-effort capacity bounding.
Do not use it as the sole hard memory limit for hostile workloads.
Keep admission and payload limits outside a candidate cache, and recheck evidence deadlines at decision time.

[OpenRaft](https://docs.rs/openraft/0.10.0/openraft/) documents unstable APIs and possible on-disk changes before 1.0.
No source review here establishes a requirement for an IPE-operated consensus group.
[Tonic](https://docs.rs/tonic/0.14.9/tonic/) is suitable for an eventual gRPC boundary, with Tokio/HTTP integration and configured message limits.
A placeholder named “control plane” is insufficient reason to adopt either package now.

# Dependency findings that change the shortlist

The [Chrono advisory](https://github.com/RustSec/advisory-db/blob/main/crates/chrono/RUSTSEC-2026-0543.md) marks all versions unmaintained.
The project's security policy rejects that dependency; this review does not propose an exception.
The approved MCP patch is specific to that SDK, not blanket approval to maintain more forks.

| Candidate | Observed package metadata | Adoption consequence |
| --- | --- | --- |
| openidconnect 4.1.0 | [Direct dependencies](https://crates.io/api/v1/crates/openidconnect/4.1.0/dependencies) require chrono `^0.4`, non-optional. | Block adoption at this version. Wait for supported upstream time handling or review a separate alternative. Do not substitute bare JWT verification for OIDC. |
| object_store 0.14.2 | [Direct dependencies](https://crates.io/api/v1/crates/object_store/0.14.2/dependencies) require chrono `^0.4.34`, non-optional. | Block adoption at this version, even with cloud defaults disabled. OpenDAL still needs its own selected-backend graph review. |
| async-nats 0.53.0 | [Features](https://crates.io/api/v1/crates/async-nats/0.53.0) make Chrono optional; JetStream selects time instead. | Test `default-features = false` with `jetstream` and one crypto provider. Do not enable Chrono or unrelated services. |
| tracing-opentelemetry 0.34.0 | [Direct dependencies](https://crates.io/api/v1/crates/tracing-opentelemetry/0.34.0/dependencies) require OpenTelemetry `^0.33.0` and tracing-subscriber `^0.3.22`. | The current locked subscriber is 0.3.20. Plan a narrow compatible update, SDK/exporter selection, and bounded shutdown test. |
| tonic 0.14.9 | [Direct dependencies](https://crates.io/api/v1/crates/tonic/0.14.9/dependencies) include optional Axum `^0.8` and Tower `^0.5`. | Review enabled transport/router features against current Axum 0.7 and Tower duplicates; avoid accidental stack expansion. |

Top-level permissive licenses do not clear transitive crates, native libraries, certificate data, or server software.
Preserve the exact license expression, including Moka's conjunction and object_store's older slash syntax.
Use Jiff's MIT option. An absent rust-version is unknown, not proof of compatibility.
The only explicitly declared crate MSRV inspected is ipe-integration's Rust 1.96; this does not prove workspace-wide MSRV support.
None of the declared candidate MSRVs in the refreshed table exceeds 1.96, but selected graphs still need compilation at that toolchain.

# Ordered adoption experiments

These experiments are proposed work, not completed implementation. Named people and delivery dates remain unassigned.
Each experiment must use a separate locked manifest, explicit features, and the existing security/license policy before workspace adoption.

| Order and owner lane | Experiment and requirement | Pass condition; evidence that stops adoption |
| --- | --- | --- |
| 1 — P2/P3 | Connect one observation to one fact collection using existing HTTP; [PRD 003 AC-2/AC-3](../prd/prd-003-evidence-integration.md). | Crash before/after commit, duplicate and reordered delivery, stale evidence, tombstones, and cross-scope input preserve receiver state and expiry. Add a broker only if recovery/fan-out needs cannot be met simply. |
| 2 — P4 | Prototype petgraph on a scoped evidence map; [PRD 001 AC-1–AC-3 and AC-9](../prd/prd-001-evidence-system-map.md). | Model distinct roles, cycles, missing sources and restricted links; traversal stops at explicit node/edge budgets. Keep a simple adjacency map if generic algorithms provide no useful reduction. |
| 3 — P2/P4 | Compare current persistence with rusqlite for audit queries and a transactional outbox. | Crash recovery, concurrent writes, retention, backup/restore and migration pass. Record state size, write amplification and p95 query/write latency against identical workloads. Retain current storage if no measured need exists. |
| 4 — P3/P4 | Evaluate an identity-provider/token profile and Cedar semantic corpus; [PRD 001 AC-6–AC-8](../prd/prd-001-evidence-system-map.md). | Test wrong issuer/audience, key rotation, expiry, revocation, missing attributes, deny precedence and scoped exceptions. Block unresolved dependencies or permissive result mappings. Product authority must resolve exception and offline rules first. |
| 5 — P2/P3 | Trial minimal async-nats JetStream only if durable delivery is selected. | Broker outage, consumer restart, duplicate delivery and full queues preserve bounded work and original expiry. Reject if operating a broker adds cost without required capability. |
| 6 — P2/P4/P5 | Trial one object backend and one compatible OpenTelemetry exporter when demanded by a consumer. | Verify conditional write conflict, denied download, deletion/retention, redacted spans, full exporter queue and bounded shutdown. Block unsupported dependency graphs or new authority paths. |

Use proptest for generated scope, revision, replay, and time-boundary sequences when the first production boundary is implemented.
Keep malformed-wire fixtures and concrete acceptance cases; property tests complement them.
No arbitrary latency target is set here. Define target workload, storage size, concurrency, hardware, and acceptable bounds before performance approval.

# Verification and limits

This is design research. Source inspection establishes current code structure, not production capacity or complete end-to-end enforcement.
The table below records registry declarations, not resolved feature graphs or approval to upgrade.
Full candidate builds, provider interoperability, fault-injection experiments, and production benchmarks remain open.
Existing workspace test results from the merge work do not validate these candidate packages.

## Completed dependency-resolution probes

Three isolated Cargo manifests resolved successfully outside the workspace.
Each passed `cargo audit --deny warnings` and `cargo deny --config <IPE deny.toml> check licenses`.
The advisory database contained 1,527 records. These checks did not change IPE's lockfile or license configuration.
License checks reported unused policy allowances and an unmatched certificate-data exception; no license rejection occurred.

| Probe | Exact dependency selection | Resolved dependencies, excluding probe root | Result |
| --- | --- | --- | --- |
| Graph | petgraph `=0.8.3`, default features | 14 | Advisory and license checks passed. |
| Delivery | async-nats `=0.53.0`, defaults disabled, `jetstream` and `ring` | 119 | Advisory and license checks passed; Chrono was absent. |
| Local audit | rusqlite `=0.43.0`, defaults disabled, `bundled` | 11 | Advisory and license checks passed, including the selected SQLite binding graph. |

These are package-resolution and supply-chain results, not compilation, behavioral tests, or deployment approval.
Adding authentication features, a SQL driver, another crypto provider, or a different target requires a new graph check.
Probe manifests and lockfiles remain in `/tmp/ipe-platform-probes` in the research environment; they are not workspace members.

# Refreshed package evidence

Each version link names the exact observed stable release. The registry snapshot includes yanked state, declared MSRV and release date.
All listed releases reported `yanked: false` at retrieval. “Locked” can include transitive dependencies and does not prove direct use.

| Package and observed version | Locked | Declared license | Declared Rust | Release date (UTC) | Recent registry downloads |
| --- | --- | --- | --- | --- | --- |
| [tokio 1.54.1](https://crates.io/crates/tokio/1.54.1) | 1.53.3 | MIT | 1.85 | 2026-10-21 | 313,277,503 |
| [tokio-util 0.7.20](https://crates.io/crates/tokio-util/0.7.20) | 0.7.19 | MIT | 1.85 | 2026-10-09 | 243,473,047 |
| [axum 0.8.9](https://crates.io/crates/axum/0.8.9) | 0.7.9 | MIT | 1.80 | 2026-04-14 | 167,980,041 |
| [tower 0.5.3](https://crates.io/crates/tower/0.5.3) | 0.4.13, 0.5.2 | MIT | 1.64.0 | 2026-01-12 | 247,071,950 |
| [tower-http 0.7.1](https://crates.io/crates/tower-http/0.7.1) | 0.5.2, 0.6.11 | MIT | 1.65 | 2026-08-31 | 226,427,896 |
| [reqwest 0.13.5](https://crates.io/crates/reqwest/0.13.5) | 0.13.5 | MIT OR Apache-2.0 | 1.85.0 | 2026-09-08 | 270,547,320 |
| [rustls 0.23.46](https://crates.io/crates/rustls/0.23.46) | 0.23.46 | Apache-2.0 OR ISC OR MIT | 1.71 | 2026-10-14 | 289,567,285 |
| [jiff 1.0.1](https://crates.io/crates/jiff/1.0.1) | 1.0.0 | Unlicense OR MIT | 1.85 | 2026-11-18 | 122,253,845 |
| [rocksdb 0.25.0](https://crates.io/crates/rocksdb/0.25.0) | 0.22.0 | Apache-2.0 | 1.88.0 | 2026-08-16 | 8,123,878 |
| [redb 4.6.0](https://crates.io/crates/redb/4.6.0) | — | MIT OR Apache-2.0 | 1.90 | 2026-11-13 | 7,008,788 |
| [rusqlite 0.43.0](https://crates.io/crates/rusqlite/0.43.0) | — | MIT | 1.89.0 | 2026-11-15 | 63,349,623 |
| [sqlx 0.9.1](https://crates.io/crates/sqlx/0.9.1) | — | MIT OR Apache-2.0 | 1.94.0 | 2026-11-18 | 59,201,109 |
| [async-nats 0.53.0](https://crates.io/crates/async-nats/0.53.0) | — | Apache-2.0 | 1.89.0 | 2026-11-17 | 6,916,398 |
| [apalis 1.2.0](https://crates.io/crates/apalis/1.2.0) | — | MIT OR Apache-2.0 | Not declared | 2026-11-14 | 478,566 |
| [cedar-policy 4.15.0](https://crates.io/crates/cedar-policy/4.15.0) | — | Apache-2.0 | 1.89 | 2026-11-16 | 4,343,416 |
| [openidconnect 4.1.0](https://crates.io/crates/openidconnect/4.1.0) | — | MIT | 1.88 | 2026-10-31 | 5,970,041 |
| [jsonwebtoken 12.0.0](https://crates.io/crates/jsonwebtoken/12.0.0) | — | MIT | 1.88.0 | 2026-11-13 | 75,793,066 |
| [petgraph 0.8.3](https://crates.io/crates/petgraph/0.8.3) | — | MIT OR Apache-2.0 | 1.64 | 2025-09-30 | 148,880,545 |
| [object_store 0.14.2](https://crates.io/crates/object_store/0.14.2) | — | MIT/Apache-2.0 | 1.85 | 2026-09-15 | 31,747,340 |
| [opendal 0.59.5](https://crates.io/crates/opendal/0.59.5) | — | Apache-2.0 | 1.91 | 2026-10-18 | 7,188,982 |
| [moka 0.12.16](https://crates.io/crates/moka/0.12.16) | — | (MIT OR Apache-2.0) AND Apache-2.0 | 1.71.1 | 2026-08-09 | 51,081,682 |
| [openraft 0.10.0](https://crates.io/crates/openraft/0.10.0) | — | MIT OR Apache-2.0 | Not declared | 2026-11-10 | 1,036,593 |
| [tonic 0.14.9](https://crates.io/crates/tonic/0.14.9) | — | MIT | 1.88 | 2026-10-29 | 115,815,018 |
| [prost 0.14.6](https://crates.io/crates/prost/0.14.6) | — | Apache-2.0 | 1.85 | 2026-11-12 | 177,485,217 |
| [opentelemetry 0.33.1](https://crates.io/crates/opentelemetry/0.33.1) | — | Apache-2.0 | 1.75.0 | 2026-10-09 | 91,245,426 |
| [tracing-opentelemetry 0.34.0](https://crates.io/crates/tracing-opentelemetry/0.34.0) | — | MIT | 1.75.0 | 2026-09-23 | 65,884,729 |
| [proptest 1.12.0](https://crates.io/crates/proptest/1.12.0) | — | MIT OR Apache-2.0 | 1.88 | 2026-10-10 | 78,844,341 |
| [governor 0.10.4](https://crates.io/crates/governor/0.10.4) | — | MIT | Not declared | 2025-12-16 | 23,907,212 |
| [backon 1.6.0](https://crates.io/crates/backon/1.6.0) | — | Apache-2.0 | 1.85 | 2025-10-18 | 44,587,100 |
| [arc-swap 1.9.5](https://crates.io/crates/arc-swap/1.9.5) | 1.7.1 | MIT OR Apache-2.0 | Not declared | 2026-11-09 | 119,395,694 |

## Additional upstream maintenance checks

These six repositories reported neither archived nor disabled. Push dates alone do not establish maintenance quality or support.
The earlier survey contains the other principal repository checks.

| Repository | Last push observed (UTC) |
| --- | --- |
| [apache/arrow-rs-object-store](https://github.com/apache/arrow-rs-object-store) | 2026-11-01T12:56:53Z |
| [apache/opendal](https://github.com/apache/opendal) | 2026-11-16T03:42:00Z |
| [moka-rs/moka](https://github.com/moka-rs/moka) | 2026-08-09T23:17:14Z |
| [databendlabs/openraft](https://github.com/databendlabs/openraft) | 2026-11-18T03:13:56Z |
| [grpc/grpc-rust](https://github.com/grpc/grpc-rust) | 2026-11-17T23:44:37Z |
| [tokio-rs/prost](https://github.com/tokio-rs/prost) | 2026-11-12T18:02:15Z |
