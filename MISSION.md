# Mission

Make policy decisions explicit, evidence-backed, and inspectable where applications and agents act.
Keep the source, scope, validity, and authority behind each decision visible to permitted users.

## Product message

**Evidence-backed policy decisions.**

Connect external evidence to explicit policy checks. See the rule, the source, and the reason for each decision.
IPE serves platform teams and developers who govern application and agent actions across external systems.

Today, IPE has a Rust evaluation runtime and a separate local integration host.
The runtime evaluates scoped policy revisions against expiring facts. The host collects source observations and explains external requirements.
The host does not yet publish enforcement facts. The complete connected product remains planned.

See the [project map](docs-cms/PROJECTS.md) for project routes, requirements, dependencies, and the first connected milestone.

## Core principles

1. Preserve evidence provenance and original validity. Receipt time and cached content do not renew a source observation.
2. Keep unknown, missing, stale, and restricted information distinct. Missing required evidence cannot become an allow decision.
3. Separate modeling, integration, evaluation, and action enforcement. Co-location does not merge these responsibilities.
4. Separate review, publication, and activation. Discovering an external rule does not authorize local enforcement changes.
5. Bound authority by principal, action, object, scope, and time. An exception is separate from an access grant.
6. Evaluate one local snapshot with bounded work. Do not call external source systems during evaluation.
7. Explain the user's policy, control, evidence, or action first. Keep raw records available in details.
8. State what is implemented, proposed, tested, and measured. A demonstration is not a production-readiness claim.

## Implemented foundations

The [runtime](docs/runtime-data.md) supports scoped fact writes, HTTPS pull ingestion, mandatory expiry, and language-2 policy evaluation.
It captures immutable snapshots and rechecks evidence deadlines before returning allow.
Policy documents have immutable revisions, source provenance, candidate evaluation, and activation operations.

The [integration host](docs/integration-host.md) executes installed JavaScript through bounded, permitted native reads.
It retains observations, governance descriptions, evidence checks, and run history for human and agent inspection.
Its record acceptance does not activate enforcement policies or prove external requirement fulfillment.

The [documentation site](docs/README.md) and product previews explain these boundaries with source links and labeled sample data.
Legacy parser, bytecode, interpreter, and JIT components were removed. Archived measurements do not describe the current runtime.

## Product projects

| Project | Outcome |
| --- | --- |
| [P1 — Policy modeling](docs-cms/prd/prd-002-policy-modeling.md) | Turn sourced requirements into reviewed policy revisions. |
| [P2 — Evidence integration](docs-cms/prd/prd-003-evidence-integration.md) | Supply fresh, attributable facts to the decisions that need them. |
| [P3 — Policy enforcement](docs-cms/prd/prd-004-policy-enforcement.md) | Evaluate policy and show where each decision takes effect. |
| [P4 — Evidence operations](docs-cms/prd/prd-001-evidence-system-map.md) | Trace evidence, time conditions, exceptions, and temporary access. |
| [P5 — Product site and delivery](docs-cms/prd/prd-005-product-site-and-delivery.md) | Explain the product and publish checked static documentation. |

These are proposed product boundaries with existing technical foundations. They are not staffed teams or deployed service guarantees.
Production hardening, durable audit, replication bounds, live publication, and application-gate proof remain necessary work.
Measure performance and cost against defined workloads before making numerical product claims.

## Success evidence

The first connected milestone traces one governed action through its sourced policy, fresh evidence, decision, and observed enforcement outcome.
It must also demonstrate that missing or expired evidence cannot permit the blocked action.
See the [project map](docs-cms/PROJECTS.md#first-connected-milestone) and each PRD for acceptance criteria and open decisions.
