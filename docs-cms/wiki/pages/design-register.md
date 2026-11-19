---
title: Design register
project_id: ipe
doc_uuid: 6b0c8f90-2c39-47de-8f44-b3fc726e165e
created: 2026-11-18
tags: [design, planning]
---

# Design register

This register lists design needs and unresolved questions across IPE. It links to the records that own their details.
All listed work needs review. A draft design, sample, or passing documentation check is not evidence of a shipped feature.
No individual owners, schedule, or approval are assigned here.

**Status terms:** Open means a decision or design is needed. Draft means a proposal exists and needs review. Foundation exists means part of the behavior exists; the linked outcome is incomplete.

## Domain and product needs

| Need | Status | Next review or evidence | Source |
| --- | --- | --- | --- |
| Shared terminology and legacy field mappings | Foundation exists | Preserve v1 wire names; verify new mappings and provider semantics. | [Ontology audit](../../memos/memo-008-ontology-audit.md#findings-and-actions) O01, O02, O08, O09 |
| Integration-to-runtime fact publication | Draft | Define source authority, scope, expiry, replay, and receiver recovery. Prove one observation reaches evaluation. | [PRD 003](../../prd/prd-003-evidence-integration.md), [RFC 014](../../../rfcs/014-fact-publication-and-synchronization.md) |
| Integration and enforcement point identities | Open | Define logical role identities, deployment relationships, and evaluator versus action gate. | [Ontology audit](../../memos/memo-008-ontology-audit.md#findings-and-actions) O06, [RFC 018](../../rfcs/rfc-018-evidence-system-map.md#system-roles) |
| Prototype requirements, checks, and controls | Open | Decide what each sample checklist item represents before changing labels or data. | [Ontology audit](../../memos/memo-008-ontology-audit.md#findings-and-actions) O07 |
| Evidence suppliers and consumers | Draft | Show sourced relationships, active revisions, uncertainty, and restricted views. | [PRD 001](../../prd/prd-001-evidence-system-map.md), [RFC 018](../../rfcs/rfc-018-evidence-system-map.md) |
| Time-based facts and controls | Draft | Agree on calendars, time zones, grace periods, and blocking or advisory effects. | [PRD 001](../../prd/prd-001-evidence-system-map.md#scope-and-open-decisions) |
| Scoped exceptions and access | Open | Decide eligibility, approvers, precedence, actions, objects, expiry, and revocation. | [PRD 001](../../prd/prd-001-evidence-system-map.md#scope-and-open-decisions), [RFC 015](../../../rfcs/015-policy-access-and-identities.md) |
| Real-time propagation and offline behavior | Open | Define effective versus acknowledged state, latency, and offline authority rules. | [PRD 001](../../prd/prd-001-evidence-system-map.md#scope-and-open-decisions) |
| Policy modeling and activation | Foundation exists | Review source-to-model completeness, candidate review, and separate activation authority. | [PRD 002](../../prd/prd-002-policy-modeling.md) |
| Action gates and reported outcomes | Foundation exists | Identify a gate and distinguish its applied action from an evaluation result. | [PRD 004](../../prd/prd-004-policy-enforcement.md) |
| Enforcement inspection and agent tools | Draft | Review principal identity, bounded transport, trace disclosure, and retention. | [RFC 017](../../../rfcs/017-enforcement-mcp-surface.md) |

## Delivery and maintenance needs

| Need | Status | Next review or evidence | Source |
| --- | --- | --- | --- |
| Release and deployment | Open | Define supported artifacts, configuration, upgrades, rollback, and current deployment profiles. | [Workload review](../../memos/memo-005-workload-structure.md#prioritized-work) |
| Coverage and legacy test harnesses | Open | Measure the reported gaps and repair misleading checks without reducing standards. | [RFC 019](../../rfcs/rfc-019-cohesive-monorepo.md#remaining-review-work) |
| Documentation and site | Foundation exists | Keep the validated site, guide paths, design records, and historical archive coherent. | [PRD 005](../../prd/prd-005-product-site-and-delivery.md), [documentation conventions](documentation-conventions.md) |

## Maintain this register

Update a row when its source record or implementation evidence changes. Link to the owning PRD, RFC, memo, or guide instead of copying its details.
Use a new PRD or RFC for requirements or proposals and a Proposed ADR for an unapproved decision. Record human approval before marking a decision Accepted.
Run `make docs-cms-check` and `make docs-check` after changes. Review the [documentation map](documentation-map.md) and [domain model](../../../docs/domain-model.md) when terminology changes.
