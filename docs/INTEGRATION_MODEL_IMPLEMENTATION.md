# Integration model implementation

Objective: complete the six numbered recommendations from the model review.
This checklist tracks completion evidence. An unchecked item is not implemented or not fully verified.

- [x] 1. Versioned schemas separate evidence, constraints, facts, and outcomes; malformed records are rejected.
- [x] 2. Controls, revisions, invocations, and executions have separate identities and tested joins.
- [x] 3. Collection coverage is explicit; failed and incomplete reads cannot imply deletion.
- [ ] 4. Semantic diffs distinguish requirement changes, source changes, and evidence refreshes; prior accepted revisions can be restored.
- [ ] 5. Human and agent projections explain operation prerequisites, evidence, unknowns, outcomes, and permitted next observations.
- [x] 6. External outcomes persist independently of collector logs; repeated observations do not inflate samples.

Item 6 implementation: `host/outcomes.rs`, authenticated `/outcomes` export, and independent archive in durable installation state.
Tests cover polling, changed outcomes, collector eviction, source replacement, restart, explicit change retention, and atomic capacity failures.
Full live proof of the complete six-part model remains pending.

Browser review findings are recorded in `acceptance/integration-host/browser-review.md`.
The first UI fix preserves open evidence and keyboard focus during refresh.

Items 1–2 implementation: `host/model.rs` defines the native v1 schema and identities.
The authenticated `/v1/host/record-schema` API exposes JSON Schema generated from those Rust types.
SDK 0.3 provides typed observations; the hosted GitHub program emits separate definitions, evidence, constraints, calls, facts, and outcomes.
The native host derives keys and revision digests. It rejects malformed models, inferred facts, duplicate identities, and unresolved definitions claimed as resolved.
Deterministic tests cover repeated action calls, caller/default bindings, matrix display names, dependencies, rerun attempts, and source revision changes.
The live proof in `acceptance/integration-host/typed-model-proof.json` captured nine definitions, ten evidence records, two constraints, eight resolved calls, three facts, and three outcomes.
Historical records without v1 models remain explicitly untyped. Legacy outcomes are exported separately and do not inflate typed execution counts.

Item 3 implementation: native YAML snapshot authority and `host/coverage.rs` verify declaration scope claims before reconciliation.
The authenticated `/coverage` API exposes native read status and retained constraint provenance.
Tests cover complete and empty scopes, absent claims, denied and paginated reads, mismatched receipts and sources, and mapper failure across restart.
The live proof in `acceptance/integration-host/coverage-proof.json` captured one complete workflow scope with two constraints in manual and scheduled runs.
Native read gaps remain explicit. Complete declarations do not establish complete effective external policy coverage.
