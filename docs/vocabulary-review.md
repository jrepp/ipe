# Vocabulary review and repository update plan

**Status:** Initial review with superseded naming recommendations. Canonical terms now appear in the project vocabulary and domain model review.
**Updated:** 2026-10-29

This review defines naming decisions for IPE's runtime and integration model.
Use the [project vocabulary](vocabulary-guide.md) for the current baseline and [scholarly grounding](#scholarly-grounding) for research references.
See the [architecture](architecture-guide.md) for component boundaries and the [documentation index](documentation-index.md) for related guides.

This review uses the current workspace, including uncommitted host assessment and monitor changes.
It checks the [vocabulary baseline](vocabulary-guide.md) against Rust types, SDK types, provider mappings, guides, and monitor labels.
Recommendations below prepare the repository update. They do not approve a schema migration or establish provider behavior.

## Recommended decisions

The later [integration domain model review](integration-domain-model.md) takes precedence for operation, control, requirement, and obligation semantics.
The monitor now uses operation and named-requirement terms from that review. Earlier recommendations below are historical, not a checklist to apply unchanged.

### Keep the constraint, obligation, and requirement hierarchy

The [integration domain model review](integration-domain-model.md) supersedes this recommendation as a target ontology.
The hierarchy below describes the current representation. Its `Obligation` wrapper does not always represent an actor's duty.
Retain current wire identifiers pending an explicit schema migration.

| Concept | Question it answers | Current representation |
| --- | --- | --- |
| Constraint | Which operation does this record govern, under what conditions, and through which enforcer? | `Entity::Constraint` |
| Obligation | Which named item within that constraint needs evidence? | `Obligation`, identified by `id` within the constraint |
| Requirement | What does that obligation require? | `Requirement`, a tagged value inside the obligation |
| Assessment | What does the available evidence establish about that requirement? | An assessment attached to the obligation in explanation and assessment responses |

Evidence: [Rust model](../crates/ipe-integration/src/host/model.rs), [SDK types](../crates/ipe-integration/sdk/index.d.ts), and [explanation projection](../crates/ipe-integration/src/host/explanations.rs).

Use **Constraints** for the monitor's constraint record filter. Use **Obligations** for the named items inside a selected constraint.
Use **Requirement and evidence** for an item's typed requirement and its supporting evidence.
The response field `requirements` currently contains obligation descriptions, IDs, requirements, and assessments. Document this mapping; retain the field for compatibility.

Do not treat a constraint as proof of effective enforcement. The current collector can construct one from declarations and unresolved provider semantics.
Keep claim status, applicability, and assessment visible when explaining a constraint.

### Reserve “contract” for an explicitly modeled concept

The absence of a current schema type does not justify removing a domain concept.
Reserve “contract” for a defined agreement or specification with explicit parties or roles, terms, scope, and lifecycle.
Decide whether IPE needs service agreements, interface specifications, or both before introducing a contract entity.
The replacements below correct ambiguous references to existing objects. They do not prohibit a future contract model.

At the time of this review, the model had no governance `contract` entity. The current v2 model supports a nested governance contract. The installation endpoint still rejects the legacy top-level `contract` output grant.
The assessment request selects a control and source revision. It does not select an object named a contract.
Use the object actually referenced:

| Existing use | Preferred wording |
| --- | --- |
| Selected contract and revision, in the assessment diagram | Selected constraint and source revision |
| Accepted contract, in catalog review text | Accepted review catalog |
| Contract revision, when referring to a selected constraint's source | Source revision |
| Contract interpretation, when referring to source mapping | Declaration mapping |
| Shared record contract, as a schema section title | Shared record schema |

Retain “API contract” when it means interface behavior. Retain the `github-contract` binary name and exact historical identifiers.
Do not replace every occurrence with “constraint.” The GitHub collector's internal `contract` branch emits either definitions or evidence records.
Rename that internal branch only after separating its two meanings in the helper and callers.

Evidence: [host routes and installation checks](../crates/ipe-integration/src/host/mod.rs), [assessment request](../crates/ipe-integration/src/host/assessments.rs), and [GitHub mapping](../testing/integration/hosted/github-actions.js).

### Separate observations, records, and evidence

Use **source observation** for data obtained from a source at a stated time.
Use **integration record** for a typed domain record stored by the host.
Use **record envelope** for the SDK's `model`, `data`, and `receipts` structure.
Keep **evidence** as the role of material supporting a claim. The `evidence` record kind is one specific representation of that material.

The SDK's `Observation` type and `observations` fields can carry constraints, definitions, and outcomes.
Retain these identifiers in the first update. Define them as record envelopes in SDK prose and comments.
This avoids describing an inferred constraint as a direct provider observation.

### Name identity and revision separately

| Preferred term | Current meaning |
| --- | --- |
| Logical definition identity | Namespace, resource, and path |
| Source reference | Logical definition identity, source revision, and pointer |
| Logical control identity | Namespace, resource, operation, and local identifier |
| Record key | Host storage identity; source-backed kinds include their source revision |
| Record revision digest | Digest of the typed model and adapter digest |
| Program digest | Digest covering source language, original source, emitted JavaScript, and embedded SDK |
| Review catalog revision | Identity of a retained catalog and its collection context |

Evidence: [`logical_key`, `key`, and `revision`](../crates/ipe-integration/src/host/model.rs) and [script digest construction](../crates/ipe-integration/src/script.rs).

Replace “collector version” with **Program digest** when displaying `adapter_digest`.
Explain that SDK changes can change this digest without edits to the program source.
Do not call it an installation digest: input, grants, and schedule are separate installation settings.
Correct the model review's definition-identity table, which includes commit and digest in the logical identity.

### Keep component names tied to their roles

| Term | Recommended scope |
| --- | --- |
| Integration host | The local service that installs programs, brokers reads, schedules runs, and stores records |
| Integration program | Authored code executed by the host |
| Collector | An integration program acting in a collection role; use this term when discussing collection behavior |
| Installation | A configured program with input, operation grants, schedule, and fact lifetime |
| Collector run | One execution of that installed program |
| Metadata adapter | A WASM module implementing the metadata classification ABI |
| Script adapter | A JavaScript or TypeScript implementation of the step ABI |
| Pull connector | Runtime ingestion that reads a configured source and replaces a fact collection generation |

These roles can overlap. “Adapter” does not mean a host, installation, or pull connector.
Evidence: [adapter ABI](../crates/ipe-integration/adapters/README.md), [host installation](../crates/ipe-integration/src/host/mod.rs), and [runtime pull connector](runtime-data.md#pull-connector).

## Check the decisions against examples

### GitHub job dependency: implemented mapping

The [GitHub collector](../testing/integration/hosted/github-actions.js) constructs a constraint for a declared job.
That constraint contains an obligation with an ID such as `dependency_0`.
Its requirement references another logical control through `kind: "dependency"`.
An assessment needs evidence for the relevant execution attempt. A prior successful job does not satisfy that requirement by itself.

This example distinguishes an operation description, a named requirement, and its specification. V1 retains the earlier storage names.

### Ticket approval: synthetic evidence, not ServiceNow enforcement

The [cross-source fixture](../testing/integration/hosted/cross-source-assessment.json) contains an `approval` obligation under a release constraint.
Its requirement is opaque. A host evidence rule selects ticket approval evidence through an association for the requested artifact and environment.
The [assessment tests](../crates/ipe-integration/src/host/assessment_tests.rs) distinguish missing context, expired links, candidate links, and conflicting evidence.

This validates the naming against a second evidence source. It does not establish ServiceNow transition semantics.
The metadata adapter explicitly does not discover complete workflow constraints.
For a future ServiceNow transition, identify the actual operation, applicable requirements, responsible actors, and supporting evidence before extending the model.
Keep the provider portability check open until that example exists.

## Disposition of the initial drift register

| Item | Recommended disposition | Update target |
| --- | --- | --- |
| V01: policy | Qualify enforcement and external policy at component boundaries. Keep “policy” in clear runtime contexts. | Baseline, writing guide, README, mission, architecture |
| V02: constraint/obligation/requirement | Superseded: use operation, named requirement, and requirement specification in domain UX. Retain v1 wire names. | Monitor, host guide, assessment guide, SDK comments |
| V03: contract | Reserve for an explicitly defined agreement or specification. Correct ambiguous references to existing objects. | Assessment guide, monitor, SDK guide; later, GitHub helper internals |
| V04: observation | Distinguish source observations from integration records and record envelopes. | Host guide, SDK guide and comments |
| V05: fact | Use integration fact and runtime fact across component boundaries. | Host, runtime, and architecture guides |
| V06: outcome | Use external outcome and evaluation result. Do not alter status values. | Monitor, host and runtime guides |
| V07: definition identity | Separate logical identity, source reference, and digest. | Model review identity table, vocabulary baseline |
| V08: program/adapter/collector | Apply the role definitions above. Qualify adapter types. | SDK, discovery, host, and adapter guides |
| V09: coverage | Keep verified declaration coverage distinct from effective policy coverage. Label RFC coverage values as proposed. | Host guide and RFC cross-references |

Two additional findings need entries in the baseline:

- **V10: version/digest.** The monitor's source-only description of collector version omits the SDK contribution to `adapter_digest`.
- **V11: deployment roles.** The writing guide mixes design terms with current runtime terms. Label assigned revision and node roles as design vocabulary where appropriate.

The [writing guide](../.agent/writing.md) defines active revision as a node report.
The [runtime guide](runtime-data.md) also describes local policy activation. Define the owning component before using “active revision.”
Review historical `predicate` examples in the mission separately from current language-2 examples. A vocabulary replacement cannot make old syntax valid.

## Repository update sequence

### 1. Normalize definitions and current documentation

Update the baseline with the decisions above and mark unresolved provider validation explicitly.
Keep the writing guide's domain definitions in the baseline to avoid maintaining two competing glossaries.
Update the host, assessment, SDK, and model review guides with the exact replacements above.
Preserve literal routes, schema fields, CLI names, and historical examples.

Check local links and anchors, then run `git diff --check`.
Review each changed statement against the source cited here. This stage needs no Rust behavior change.

### 2. Normalize monitor labels and SDK explanations

The initial **Constraints** label recommendation is superseded. The filter now uses **Operations** for stored `constraint` records.
Named requirements belong to the operation. They are not separate operation records or proof of a normative duty.
Use **External outcomes**, **Collector runs**, and **Program digest** consistently.
Use **Review catalog** for retained catalog selections. Avoid “capture” as an interchangeable name for both a run and a catalog.
Explain plain meanings in nearby text, such as “An invocation is a call site with input bindings.”

Update text assertions in the existing monitor tests without changing record selectors or JSON fixtures.
Run `node --test tools/integration-host/*.test.cjs`.
Confirm that filters, run inspection, evidence display, and catalog acceptance still refer to the same objects.

### 3. Review internal names and compatibility separately

Inventory consumers before renaming `Observation`, `observations`, `requirements`, `adapter_digest`, or persisted identifiers.
The first update retains these names and documents their meaning. Do not add duplicate fields merely to introduce preferred terms.
Rename the GitHub helper's internal `contract` branch only with mapping tests that preserve emitted record kinds and identities.
Run the hosted Rust and JavaScript checks from the [validation guide](../.agent/validation.md) for that change.

A later schema rename needs a version decision, migration rules, and tests for existing state and clients.
There is no evidence in this review that a schema rename is necessary to complete the first normalization pass.

## Review limits

This review inspected source and existing tests; it did not execute the runtime or test a live provider.
The repository update remains pending. Historical documents and all provider-specific vocabulary have not received a full audit.
The recommendations establish naming boundaries, not a universal provider ontology.

## Scholarly grounding

These references inform the design review. Their terminology is not automatically equivalent to IPE's current types.
The summaries use accessible abstracts and source excerpts; they are not full-paper evaluations.

| Reference | Contribution | Question for IPE |
| --- | --- | --- |
| Weigand, Johannesson, and Guizzardi. [A core ontology of organizational policies](https://research.tilburguniversity.edu/en/publications/a-core-ontology-of-organizational-policies/). Software and Systems Modeling, 2026. DOI: `10.1007/s10270-025-01356-4`. | Separates policies from policy documents and examines organizational authority, delegation, and coordination. | What distinguishes a governing policy from its document and executable representation? |
| Griffo, Almeida, Guizzardi, and Nardi. [From an Ontology of Service Contracts to Contract Modeling in Enterprise Architecture](https://bia.unibz.it/esploro/outputs/conferenceProceeding/From-an-Ontology-of-Service-Contracts/991005926351201241). IEEE EDOC, 2017, pp. 40–49. [Author copy](https://www.inf.ufes.br/~gguizzardi/UFO-L.pdf). | Models service contracts through relations between parties and distinguishes legal positions such as rights and permissions. | Which parties and commitments would make an IPE record a contract, rather than a declaration or catalog? |
| Singh. [An ontology for commitments in multiagent systems: Toward a unification of normative concepts](https://www.csc2.ncsu.edu/faculty/mpsingh/papers/mas/ai%2Blaw-final.pdf). Artificial Intelligence and Law 7, 1999, pp. 97–113. | Models social commitments, their operations, and their context in multiagent systems. | Who owes an obligation to whom, and how does that obligation change over time? |
| Damianou, Dulay, Lupu, and Sloman. [The Ponder Policy Specification Language](https://link.springer.com/chapter/10.1007/3-540-44569-2_2). POLICY 2001, pp. 18–38. DOI: `10.1007/3-540-44569-2_2`. | Separates policy specification from access-control mechanisms and supports event-triggered obligation policies. | Which requirements describe permission, and which require an action after a trigger? |
| Schneider. [Enforceable Security Policies](https://www.cs.cornell.edu/fbs/publications/EnfSecPols.pdf). ACM Transactions on Information and System Security 3(1), 2000, pp. 30–50. | Characterizes policies enforceable by execution monitoring under a specified mechanism model. | What can a particular IPE guard prevent, and what can it only observe or report? |

Read the policy, service-contract, and enforcement papers first. Use the commitment paper for agent responsibilities and Ponder for executable policy design.
The linked contract PDF timed out during direct retrieval. Its indexed excerpts and institutional publication record were accessible.
Ponder's publisher abstract was accessible; its full chapter was not reviewed.

### Consequences for the proposed ontology

The following are IPE design recommendations drawn from these sources, not claims of a shared standard:

- Separate normative intent, its representation, the operating mechanism, and evidence of operation.
- Do not require every policy to pass through a contract before enforcement. Model those relationships explicitly where they exist.
- Do not assume policies are global, contracts always have exactly two parties, or controls are always deterministic software.
- Separate policy identity from immutable policy revisions. “Mutable policy” and “immutable policy” otherwise describe different levels ambiguously.
- Keep an actor distinct from the principal used to authenticate or authorize its actions.
- Keep assertions distinct from supporting evidence. A signed assertion establishes provenance under trust assumptions; it does not by itself prove compliance.
- Keep an obligation to act distinct from a condition to satisfy. The current `Obligation` wrapper does not establish all normative obligation semantics.
- Keep a control description distinct from an operating control. Current `ControlId` identifies a declared operation-level concept, not verified enforcement code.

The [W3C ODRL Information Model](https://www.w3.org/TR/odrl-model/) provides a useful standards comparison, separate from these papers.
It defines permissions, prohibitions, duties, parties, and constraints; it also treats Agreement as a Policy subtype.
That is a concrete alternative to a mandatory three-layer hierarchy. IPE must state its chosen meanings and mappings rather than imply universal agreement.

Before schema changes, test the ontology against questions about identity, responsibility, applicability, authority, enforcement, evidence, and expiry.
Include cases where a declaration exists without an operating control, and where missing evidence prevents an assessment.
