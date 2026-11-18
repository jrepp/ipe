# Integration model implementation

Objective: complete the six numbered recommendations from the model review.
This checklist tracks completion evidence. An unchecked item is not implemented or not fully verified.

- [x] 1. Versioned schemas separate evidence, constraints, facts, and outcomes; malformed records are rejected.
- [x] 2. Controls, revisions, invocations, and executions have separate identities and tested joins.
- [x] 3. Collection coverage is explicit; failed and incomplete reads cannot imply deletion.
- [x] 4. Semantic diffs distinguish requirement changes, source changes, and evidence refreshes; prior accepted revisions can be restored.
- [x] 5. Human and agent projections explain operation prerequisites, evidence, unknowns, outcomes, and permitted next observations.
- [x] 6. External outcomes persist independently of collector logs; repeated observations do not inflate samples.

Item 6 implementation: `host/outcomes.rs`, authenticated `/outcomes` export, and independent archive in durable installation state.
Tests cover polling, changed outcomes, collector eviction, source replacement, restart, explicit change retention, and atomic capacity failures.
The combined live proof and completion audit below cover all six items.

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

Items 4–5 implementation: `host/revisions.rs` retains immutable review catalogs, diffs logical identities, and selects accepted catalogs.
`host/explanations.rs` provides the shared human and agent projection. The monitor renders the native API result.
Tests cover changed conditions, dependencies, call bindings, source revisions, mapping revisions, and evidence refreshes.
Acceptance and restore survive restart. A retained mapping replays against the synthetic provider.
Restore preserves the active collector, observed facts, and fact deadlines. Capacity failure leaves accepted catalogs intact.
Explanation tests cover missing, expired, conflicting, matching, and unmet fact requirements; denied observation grants; and cross-revision outcome exclusion.

## Completion audit

| Item | Authoritative evidence |
| --- | --- |
| 1 | Native schema validation and generated schema tests; typed GitHub proof |
| 2 | Authored graph tests for repeated calls, defaults, rerun attempts, dependencies, and matrix-name ambiguity |
| 3 | Native coverage tests and live `coverage-proof.json`; denied and paginated reads preserve constraints |
| 4 | Revision tests, host accept/restore/replay/restart test, and live `review-proof.json` |
| 5 | Explanation tests, view tests, live API proof, and agent-browser checks on port 8767 |
| 6 | Outcome archive tests for polling, changes, log eviction, restart, migration, export, and overflow |

Validation passed: workspace tests with default and all features, workspace Clippy with warnings denied, Rust formatting, discovery acceptance, and monitor view tests.
The final live review proof is `acceptance/integration-host/review-proof.json`.
Browser evidence is in `target/integration-review-v1/browser/`.
The final browser checks found no console errors or axe violations. The loaded page fits a 390-pixel viewport.

Limits remain explicit: opaque expressions are not evaluated, candidate execution links are not exact joins, and external authorization gaps remain unknown.
The host does not publish facts to enforcement nodes or activate policies. Follow-up provider coverage and immutable-read caching remain outside these six items.
