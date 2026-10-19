# RFC 013 Domain constraints and policy projections

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

Own observations, external constraints, outcomes, and their shared policy descriptions.
This catalog is separate from executable enforcement policies. Runtime transport is
[RFC 014](014-fact-publication-and-synchronization.md); disclosure is [RFC 015](015-policy-access-and-identities.md).

## Domain contract

Keep three semantic records distinct:

| Record | Meaning | Projection |
| --- | --- | --- |
| Observation | Evidence about a subject at a time | Selected expiring facts |
| Constraint | Requirement governing an operation | Shared policy description and planning prerequisites |
| External outcome | Result reported by an external enforcer | Observed outcome; optionally an expiring fact |

Use shared entity and operation identities. A constraint records its subject,
operation, actor context, applicability, obligations, evidence requirements,
dependencies, exceptions, enforcer, source authority, validity, and coverage.
All external identities and provenance references in released projections are opaque.
Distinguish the discovery principal from the intended execution actor.

Start with common forms for required fields, evidence, approvals, transitions,
temporal obligations, and dependencies. Preserve identity namespaces, units,
cardinality, event ordering, and schema versions. Keep provider expressions in a
known dialect or retain a protected opaque reference when normalization would lose
meaning. Do not flatten sequential obligations into booleans or impose a universal
precedence rule. Shared vocabulary extensions require versioned compatibility.

Coverage is `complete`, `partial`, or `opaque` for a declared operation and scope;
completeness needs source support and validation evidence, not just a guest flag.
Freshness, applicability, and disclosure are independent properties. Failed pages,
permission denials, inaccessible rules, and unknown records are not empty sets.
Pagination completion alone does not prove a consistent upstream snapshot. Negative
evidence needs an explicit, supported coverage claim.

Pin source observations, mapping version, domain schema, and projection version.
Produce shared policy descriptions and workflow prerequisites from those records.
Descriptions and executable policies use different types and publication routes.
Only explicit authorized promotion of supported semantics can produce a local IPE
guard, with pinned evidence schemas and translation tests. Unsupported conditions
remain represented in protected knowledge even when no local predicate is possible.

A planner receives the constraints it may inspect, evidence gaps, dependencies,
permitted next steps, and uncertainty. Suggested steps are not grants or commands
from source text. The external backend still decides whether its operation succeeds;
HTTP success alone is not proof of the desired business outcome.

## First integration and portability

For security remediation, preserve the chain:

**Finding → affected subject → tracked remediation requirement → required evidence
and approvals → external outcome.**

A finding, CVE, asset, and backend ticket are different identities with potentially
many-to-many relationships. Preserve authority for each field, source severity
scheme, deadlines, ownership, and lifecycle. Missing links mean unknown or unlinked.
Distinguish verified remediation, risk acceptance, false positive, suppression,
duplicate closure, and reopening. A clean scan does not close a backend obligation;
a closed ticket does not prove a vulnerability is gone. An
[OSV advisory query](https://google.github.io/osv.dev/api/) is not an asset scan.

The first fixture should follow a finding to its ServiceNow requirement, discover
resolution prerequisites, gather fix/review/CI evidence for the correct commit,
observe deployment and verification for the affected asset, and read the backend's
actual resolution outcome. Action execution is modeled but not implemented here.

| Discovery track | Grounding and question |
| --- | --- |
| Findings backend | Select the actual provider; cover pagination, duplicates, exceptions, reopening, permission gaps, and links to remediation records |
| ServiceNow | Identify application, tables, release, custom rules and principal. [State management](https://www.servicenow.com/docs/r/zurich/platform-administration/state-management/state-model.html) and [Change Management APIs](https://www.servicenow.com/docs/r/api-reference/rest-apis/change-management-api.html) are starting points, not proof that remediation uses Change Management. [Scripted transitions](https://www.servicenow.com/docs/r/xanadu/it-service-management/change-management/t_ConfigStateModelTransit.html) may remain opaque |
| SCM and CI | [GitHub rules](https://docs.github.com/en/rest/repos/rules) and [check runs](https://docs.github.com/en/rest/checks/runs) exercise applicability, bypass actors, exact commit identity, and distinct status/conclusion semantics. Select the CI provider |
| SAP | Select one application and release; distinguish schema metadata, authorization, business rules, and instance state. Do not infer complete constraints from API schemas |
| BPM | Test definitions versus running instances and external workers. Camunda [decision dependencies](https://docs.camunda.io/docs/components/modeler/dmn/decision-requirements-graph/) and [authorization](https://docs.camunda.io/docs/components/concepts/access-control/authorizations/) provide concrete counterexamples to treating model read access as action permission |

Provider documents identify discovery surfaces, not deployed tenant behavior. Pin
actual versions and permissions in fixtures. Imported scripts remain source evidence;
neither adapter host nor IPE executes them as external enforcement logic.

## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | Typed discovery inventory and explicit visibility/coverage gaps | No unsupported claim of complete policy discovery |
| M2 adapter iteration | Minimal observation/constraint/outcome schemas and example projections | Fixture preserves actor, subject, authority, provenance and opaque residuals |
| M3 first live integration | Selected ServiceNow requirement and evidence projections; one independently authored IPE guard | Constraint import cannot change executable policy; expected local decisions use only synchronized facts |
| M4 operational proof | Versioned mapping compatibility and forensic references | Upgrade preserves identities and explains semantic differences |
| Later | SCM plus SAP/BPM counterexamples and richer temporal forms | Shared vocabulary expands without flattening provider semantics |

## Review questions

- Which actual discovered requirement has enough accessible semantics for the first projection?
- Which information must remain opaque rather than normalized?
- Which exact fact fields does the first local policy need?
