---
title: Domain ontology and mapping audit
author: IPE maintainers
created: 2026-11-18
tags: [domain, ontology, research]
id: memo-008
project_id: ipe
doc_uuid: 4d4af44f-b8e6-4bd9-8a20-9b291bd38a08
---

# Outcome

IPE has a substantial [vocabulary](../../docs/vocabulary-guide.md) and a [domain model review](../../docs/integration-domain-model.md).
It does not yet have one fully implemented ontology across its workloads, prototypes, and proposed system map.
The [domain model and mappings](../../docs/domain-model.md) now connects preferred terms to representative code and current API fields.
It marks unimplemented roles and semantic decisions. It does not approve schema renames or claim exhaustive consistency.

The project owner requested an ontology audit across documentation and code on 2026-11-18.
This bounded review sampled the vocabulary, RFC 013, RFC 018, PRD 001, runtime and host guides, Rust domain and evaluation types, API handlers, monitor projections, and product mock data.
It did not inspect every historical record, source adapter, generated export, localization, or provider field.

# Findings and actions

| ID | Evidence | Assessment and action |
| --- | --- | --- |
| O01 | [Vocabulary](../../docs/vocabulary-guide.md), [domain projection](../../crates/ipe-integration/src/host/domain.rs), [host model](../../crates/ipe-integration/src/host/model.rs) | Operation and requirement mappings exist, but v1 `constraint`, `control`, and `obligation` still appear in storage and API fields. The new mapping preserves those names and distinguishes them from governance controls and duties. Do not silently rename wire fields. |
| O02 | [Governance model](../../crates/ipe-integration/src/host/governance.rs), [installation validation](../../crates/ipe-integration/src/host/mod.rs), [older vocabulary review](../../docs/vocabulary-review.md) | Nested v2 governance contracts exist; the obsolete top-level `contract` output kind remains rejected. The older review now labels its observation as historical. Provider-reported terms do not establish an agreement. |
| O03 | [Model review](../../docs/integration-model-review.md#contract-construction), [DefinitionId and Source](../../crates/ipe-integration/src/host/model.rs) | The review combined logical identity with commit and digest. It now separates namespace/resource/path from captured source revision and content digest. No stored identity changed. |
| O04 | [README](../../README.md), [policy evaluator](../../crates/ipe-core/src/runtime_policy.rs), [HTTP handler](../../crates/ipe-server/src/runtime.rs) | The README described indeterminate as an aggregate decision. It now distinguishes the binary HTTP decision from `evaluation_status`; per-policy outcomes still include `not_applicable` and `indeterminate`. |
| O05 | [Integration facts](../../crates/ipe-integration/src/host/model.rs), [runtime facts](../../crates/ipe-core/src/runtime_facts.rs), [RFC 014](../../rfcs/014-fact-publication-and-synchronization.md) | These are different records with different identity and authority boundaries. A reviewed publication mapping is missing. Preserve source expiry and scope; do not treat similarly named fields as a completed mapping. |
| O06 | [PRD 001](../prd/prd-001-evidence-system-map.md), [RFC 018](../rfcs/rfc-018-evidence-system-map.md), [writing guidance](../../.agent/writing.md) | Integration/enforcement points are logical roles; node terms describe deployment roles. The evaluator and the action gate remain distinct. Their identity, co-location, and mapping to actual deployments need a design decision. |
| O07 | [Sample product qualification data](../../tools/product-ship-mocks/data.json), [governance control](../../crates/ipe-integration/src/host/governance.rs) | The prototype's `controls` list includes conditions such as review completion, rather than identified mechanisms. Its shape is not the governance control schema. Review product labels and data migration before normalizing the prototype. |
| O08 | [V2 assessments](../../crates/ipe-integration/src/host/assessments.rs), [monitor](../../tools/integration-host/trace-view.js) | V2 distinguishes evidence-check status from requirement fulfillment. V1 uses `satisfied`/`unsatisfied` for the check; retain versioned fields and avoid presenting them as fulfillment or authorization. |
| O09 | [Integration SDK](../../crates/ipe-integration/sdk/index.d.ts), [host model](../../crates/ipe-integration/src/host/model.rs) | `ipe.observation` names an emitted envelope while prose also uses source observation for a read. Preserve the API name; qualify the object in prose. Review adapter-specific documentation before changing it. |
| O10 | [RFC 018 coverage](../rfcs/rfc-018-evidence-system-map.md#requirements-coverage), [PRD 001](../prd/prd-001-evidence-system-map.md) | Coverage in a draft means the design addresses an acceptance criterion, not implementation evidence. Exception precedence, control modes, calendars, grants, propagation, and offline behavior remain unresolved. |

# Shared mapping and change process

The current [domain model and mappings](../../docs/domain-model.md) identifies records, source roles, and the boundaries between observed and proposed relationships.
Before changing a concept, identify its owning component, issuer or authority, scope, identity, revision, time basis, and relationship evidence.
Keep runtime, integration, provider, and UI fields exact in examples. Use the preferred term for explanatory prose.
Review backwards compatibility and migration before changing a public schema or SDK identifier.
Add positive and negative cross-boundary tests when a mapping becomes executable.

The first connected publication design must test duplicate and reordered delivery, scope isolation, expiry, and failure recovery.
A policy decision must remain separate from an observed action result. A receipt, assessment, or external run cannot prove a control operated.
Do not introduce a graph database or an ontology language solely to express this map.

# Open decisions and limits

Product and runtime maintainers need to define integration-point and enforcement-point identities and their relation to deployments.
The policy authority needs to resolve exception eligibility, precedence, approval, revocation, and offline behavior before those records carry authority.
The UX and domain owners need to decide whether the sample checklist items are requirements, checks, or separately identified controls.
Provider-specific mappings need source evidence and live validation; a synthetic fixture alone does not establish provider semantics.
No owners have been assigned by this audit. These are roles needed for decisions, not named teams or approved decisions.
