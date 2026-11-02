# Project vocabulary

**Status:** Canonical integration terms adopted in SDK 0.7, v2 domain APIs, governance model v2, and monitor UX. Stored v1 names remain compatible.
**Updated:** 2026-10-29

See the [vocabulary review and update plan](VOCABULARY_REVIEW.md) for evidence, recommended decisions, and exact update targets.
The integration domain and monitor apply the decisions below. Historical documents and stored v1 identifiers still need separate review.

Use this reference for new documentation, UX text, and design reviews.
It defines preferred terms and their boundaries. It does not rename API fields or change runtime behavior.
The review covers the current runtime, integration host, and RFC 013 domain model.
It is not a complete audit of historical documents or provider-specific terms.

## Review rules

- Use one preferred term for each concept. Use a qualifier when two domains share a word.
- Keep code identifiers, JSON fields, routes, and provider terms exact when quoting them.
- Describe current behavior from source and tests. Label proposed behavior as proposed.
- Treat the open decisions below as unresolved. Do not introduce another synonym while they remain open.
- Add a vocabulary entry when a change introduces a domain concept or changes its meaning.

For each new concept, record its definition, domain, identity, relationships, and nearest distinct concept.
Include a source or proposal link and an example that shows the distinction.
The change reviewer must check whether an existing term already has that meaning.
If a rename affects a public schema, record the compatibility plan before changing identifiers.

## Enforcement and integration boundaries

| Preferred term | Meaning and boundary | Source |
| --- | --- | --- |
| Enforcement policy | A rule that IPE evaluates against input and runtime facts. Use “policy” alone only in a clear enforcement context. | [Runtime guide](RUNTIME_DATA.md) |
| Policy document | A document with policy data, provenance, and modeled or unsupported sections. It is distinct from one rule. | [`PolicyDocument`](../crates/ipe-core/src/policy_document.rs) |
| Policy revision | An immutable published policy document with schema pins and a scoped revision number. | [`PolicyRevision`](../crates/ipe-core/src/policy_document.rs) |
| Policy activation | Selection of a policy revision for runtime evaluation. Integration catalog acceptance does not perform this action. | [Policy API](RUNTIME_DATA.md#policy-management-and-candidate-testing-api) |
| External policy | A requirement enforced by an external system. An IPE description of it does not grant permission in that system. | [RFC 013](../rfcs/013-domain-constraints-and-projections.md) |
| Policy | Rules established by an authority for a stated scope. Distinguish the rule from its document and enforcement mechanism. | [Domain boundaries](INTEGRATION_DOMAIN_MODEL.md#proposed-conceptual-boundaries) |
| Contract | Commitments or interaction terms involving identified parties or roles. A schema or review catalog alone does not establish an agreement. | [Domain boundaries](INTEGRATION_DOMAIN_MODEL.md#proposed-conceptual-boundaries) |
| Control | An identified mechanism that prevents, detects, or responds to a governed condition. A label or successful operation does not prove its effectiveness. | [Domain boundaries](INTEGRATION_DOMAIN_MODEL.md#proposed-conceptual-boundaries) |
| Operation | An action offered or performed by an external system. Its description includes applicability, actor context, named requirements, and a declared enforcer. Stored v1 `constraint` records describe operations. | [`Entity::Constraint`](../crates/ipe-integration/src/host/model.rs) |
| Obligation | A duty of an identified party or role to perform an action under stated conditions. The legacy `Obligation` wrapper does not establish this meaning. Use “named requirement” for that wrapper in UX. | [`Obligation`](../crates/ipe-integration/src/host/model.rs) |
| Requirement | A named condition or duty attributed to a source or explicit interpretation. Its specification can describe a fact check, permission, environment reference, dependency, or opaque condition. | [`Requirement`](../crates/ipe-integration/src/host/model.rs) |
| Evidence check | A configured comparison of evidence with an expected value. Results are `matched`, `not_matched`, or `unknown`; a match does not prove requirement fulfillment. | [Assessment guide](INTEGRATION_ASSESSMENTS.md) |
| Requirement fulfillment | Whether the external requirement holds in the stated context. Current v2 assessments leave this unknown, separately from their evidence-check result. | [Assessment guide](INTEGRATION_ASSESSMENTS.md) |
| Handoff policy | A policy governing routing or transfer of work, responsibility, or a required decision, including completion and continuation conditions. Routing does not itself confer authority. | [Human handoff design](INTEGRATION_DOMAIN_MODEL.md#governing-rules-and-human-handoffs) |
| Handoff instance | An observed occurrence governed by a handoff policy revision. Notification, acknowledgment, decision, and completion are distinct events. | [Human handoff design](INTEGRATION_DOMAIN_MODEL.md#governing-rules-and-human-handoffs) |
| Principal | A provider- or authority-scoped identity. A human can have several principals; a service principal need not represent a human. | [Identity and contact design](INTEGRATION_DOMAIN_MODEL.md#principals-roles-and-contact-methods) |
| Role assignment | A sourced relationship assigning a principal a responsibility or authority within a scope and validity interval. Membership alone does not establish approval authority. | [Identity and contact design](INTEGRATION_DOMAIN_MODEL.md#principals-roles-and-contact-methods) |
| Contact method | A sourced route for a person, principal, group, or role, such as an email address, team channel, or ticket queue. It does not confer authority. | [Identity and contact design](INTEGRATION_DOMAIN_MODEL.md#principals-roles-and-contact-methods) |
| Evaluation result | The result of IPE policy evaluation. Preserve `allow`, `deny`, `indeterminate`, and `not_applicable` where the interface uses them. | [`PolicyOutcome`](../crates/ipe-core/src/runtime_policy.rs) |
| Review catalog | A retained integration capture with declarations, source evidence, coverage, code, input, and grants. It is separate from the runtime policy catalog. | [Host review catalogs](INTEGRATION_HOST.md#review-catalogs-and-operation-explanations) |
| Catalog acceptance | Selection of a review catalog for local review and explanations. It does not activate an enforcement policy. | [Host review catalogs](INTEGRATION_HOST.md#review-catalogs-and-operation-explanations) |

The [integration domain model review](INTEGRATION_DOMAIN_MODEL.md) defines these boundaries and their research basis.
Use **requirement fulfillment** for whether an external requirement holds. The current v2 assessment keeps it `unknown`.
Use **control** for an identified enforcement or monitoring mechanism, not for the operation it governs.
A **declared enforcer** is a source label; it does not establish an identified or effective control.
A **policy** is a rule established by an authority for a scope. A **contract** specifies commitments or interaction terms involving identified parties.
The collector does not create separate policy, contract, or control records merely by finding a workflow.

Stored v1 `Constraint`, `Obligation`, and `ControlId` names remain compatibility identifiers.
In domain UX, use **operation**, **named requirement**, and **operation identity** for those existing objects.
This projection does not rename stored fields or imply a schema migration.

## Evidence and identity

| Preferred term | Meaning and boundary | Source |
| --- | --- | --- |
| Source observation | Data read from an external system at a stated time. A read can support several domain records. | [Host record guide](INTEGRATION_HOST.md#versioned-record-types) |
| Evidence | Material that supports a claim. The host's `evidence` record is a specific source description, excerpt, or lexical clue. | [`EvidenceMethod` and `Entity`](../crates/ipe-integration/src/host/model.rs) |
| Claim status | How a model presents its claim: `declared`, `provider_reported`, `inferred`, or `unknown`. This is not an assessment result. | [`Claim`](../crates/ipe-integration/src/host/model.rs) |
| Declaration | A statement in source, such as a requested permission or input default. It does not prove effective permission or runtime validation. | [Host record guide](INTEGRATION_HOST.md#versioned-record-types) |
| Integration fact | A hosted subject/predicate/value record with an authority and finite expiry. It is not automatically an enforcement input. | [`Entity::Fact`](../crates/ipe-integration/src/host/model.rs) |
| Runtime fact | A value stored in a scoped runtime collection, with observation time, expiry, source revision, and local revision. | [`FactRecord`](../crates/ipe-core/src/runtime_facts.rs) |
| Association | An expiring subject-to-target link with a predicate, context, authority, and resolution status. A candidate link is not resolved evidence. | [Assessment guide](INTEGRATION_ASSESSMENTS.md) |
| Definition | A captured source interface, such as a workflow or action definition. Its source revision is separate from its logical identity. | [`DefinitionId` and `Source`](../crates/ipe-integration/src/host/model.rs) |
| Operation identity | A stable identity formed from namespace, resource, operation, and local identifier. A source edit does not create another operation. V1 calls this `ControlId`; SDK 0.6 also exposes `OperationId`. | [`ControlId`](../crates/ipe-integration/src/host/model.rs) |
| Invocation | A source call site with input bindings and a possible resolved callee. Two calls to one definition are distinct invocations. | [`Entity::Invocation`](../crates/ipe-integration/src/host/model.rs) |
| External execution | A provider run or attempt, optionally narrowed to a job or step. Repeated observations do not create another execution. | [`ExecutionId`](../crates/ipe-integration/src/host/model.rs) |
| External outcome | A reported execution result, including status and any conclusion or duration. It does not prove future authorization. | [`Entity::Outcome`](../crates/ipe-integration/src/host/model.rs) |
| Collector run | One host execution of an installed collection program. Its success does not prove that an external operation succeeded. | [`Run`](../crates/ipe-integration/src/host/mod.rs) |
| Coverage | The supported extent of a collection within a stated scope. Complete declarations do not establish complete effective policy coverage. | [Host coverage rules](INTEGRATION_HOST.md#versioned-record-types) |
| Projection | A mapping from source or domain records into another representation. Name its destination, such as an explanation or fact projection. | [RFC 013](../rfcs/013-domain-constraints-and-projections.md) |

Always qualify “revision,” “scope,” “status,” and “snapshot” when their owner is unclear.
For example, a runtime scope is a tenant/environment pair; a host collection scope identifies collection coverage.
A source revision, policy revision, and review catalog revision have different identities and lifecycles.
An evaluation snapshot and a captured provider document also have different consistency guarantees.

## Relationship model

These relationships describe the current boundaries. They do not define a new storage schema.

```text
collector run -> source observations -> typed integration records
definition + source revision -> source reference
operation identity + source reference -> operation description
operation description -> named requirements -> requirement specifications
invocation -> parent call site + callee + input bindings
external execution -> external outcome
named requirement + exact mapped revision + evidence rule + context -> evidence check
requirement fulfillment remains distinct from the evidence check
policy revision + input + runtime fact snapshot -> evaluation result
```

Review catalogs retain collection context and declarations. Current facts keep their own expiry.
Restoring a review catalog does not renew facts.
Fact publication and policy activation are separate authorized operations.
The local integration host does not publish enforcement facts.

Example: a workflow declares a requested permission. An operation description records that declaration as a named requirement.
A collector run reads it. A later external execution produces a provider-reported outcome.
A successful outcome does not prove that the requested permission is currently effective.
Requirement fulfillment can therefore remain unknown even when the earlier execution succeeded.

## Drift register and open decisions

These findings need targeted follow-up. The baseline does not approve schema renames.

| ID | Evidence of drift | Rule for new text | Remaining decision or work |
| --- | --- | --- | --- |
| V01 | [Writing guide](../.agent/writing.md) uses “policy”; [RFC 013](../rfcs/013-domain-constraints-and-projections.md) separates descriptions and executable policies. | Use “enforcement policy” and “external policy” across the boundary. | Review product pages for unqualified uses. |
| V02 | [Host guide](INTEGRATION_HOST.md) calls constraints “requirements”; the model nests `Requirement` inside `Obligation`. | Use operation, named requirement, and requirement specification in domain UX. Preserve exact v1 names in schema references. | Adopted in SDK 0.6, v2 APIs, and monitor. Live non-workflow provider validation remains open. |
| V03 | [Assessment guide](INTEGRATION_ASSESSMENTS.md) uses “contract”; the host rejects new `contract` output kinds. | Name the actual object: constraint, definition, or review catalog. | Define whether “contract” remains an umbrella term. Inventory existing routes and fields before any rename. |
| V04 | [Host guide](INTEGRATION_HOST.md) calls all emitted envelopes “observations,” including constraints and outcomes. | Use “source observation” for a read and “integration record” for a typed stored record. | Review SDK prose without changing wire names. |
| V05 | Both runtime and host store “facts”; their schemas and publication paths differ. | Use “runtime fact” or “integration fact” across components. | Keep this distinction in future publication designs and tests. |
| V06 | Runtime `PolicyOutcome` and host `Entity::Outcome` use the same noun for different results. | Use “evaluation result” and “external outcome.” | Review UI labels and reports; retain current identifiers. |
| V07 | [Model review](INTEGRATION_MODEL_REVIEW.md#contract-construction) includes commit and digest in definition identity; `DefinitionId` uses namespace, resource, and path. | Separate logical definition identity from source revision and content digest. | Correct the earlier recommendation when its review document is next updated. |
| V08 | [Host guide](INTEGRATION_HOST.md) uses program, adapter, collector, and installation near the same code. | Use “program” for code, “collector” for its read role, and “installation” for configured hosted code. | Audit “adapter” across WASM discovery, script steps, and hosted programs before choosing a broader definition. |
| V09 | [RFC 013](../rfcs/013-domain-constraints-and-projections.md) proposes coverage values; the host guide describes verified declaration scopes and native read status. | Name the coverage mechanism and scope. | Do not present proposed coverage values as the current host schema. |

“Unknown” also occurs in claim status, link resolution, execution status, and assessments.
Keep those fields distinct. A provider-reported claim can still support an unknown assessment.
Do not replace `indeterminate` with `unknown` in the runtime API.

## Next review and completion checks

1. Validate V02 with a live non-workflow provider. Validate contract-party mappings under V03 before claiming a discovered agreement.
2. Review the SDK, host monitor, and current guides against this baseline.
3. Review runtime and integration boundary terms in the README, mission, and architecture guides.
4. Mark historical uses separately. Preserve exact examples of old schemas.

The first normalization pass is complete when each drift item has a decision and affected references have been updated.
Any retained alias must name its preferred term and compatibility reason.
Any schema change must include migration guidance and tests for the affected behavior.
Do not use a global word replacement: several shared words have valid, distinct meanings.
