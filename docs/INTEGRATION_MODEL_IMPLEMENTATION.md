# Integration model implementation

Objective: complete the six numbered recommendations from the model review.
This checklist tracks completion evidence. An unchecked item is not implemented or not fully verified.

- [ ] 1. Versioned schemas separate evidence, constraints, facts, and outcomes; malformed records are rejected.
- [ ] 2. Controls, revisions, invocations, and executions have separate identities and tested joins.
- [ ] 3. Collection coverage is explicit; failed and incomplete reads cannot imply deletion.
- [ ] 4. Semantic diffs distinguish requirement changes, source changes, and evidence refreshes; prior accepted revisions can be restored.
- [ ] 5. Human and agent projections explain operation prerequisites, evidence, unknowns, outcomes, and permitted next observations.
- [x] 6. External outcomes persist independently of collector logs; repeated observations do not inflate samples.

Item 6 implementation: `host/outcomes.rs`, authenticated `/outcomes` export, and independent archive in durable installation state.
Tests cover polling, changed outcomes, collector eviction, source replacement, restart, explicit change retention, and atomic capacity failures.
Full live proof of the complete six-part model remains pending.

Browser review findings are recorded in `acceptance/integration-host/browser-review.md`.
The first UI fix preserves open evidence and keyboard focus during refresh.
