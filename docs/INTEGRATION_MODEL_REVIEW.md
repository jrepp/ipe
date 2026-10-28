# Integration model review and recommendations

Status: recommendations, not implemented features unless stated otherwise.

## Recommendation

Make the next milestone a reviewable contract revision for one operation: publish the IPE docs site.
Trace that operation from source declarations through dependencies, current evidence, and one external execution.
Show what changed between two revisions and explain each unresolved requirement.

Keep the existing observation, constraint, and outcome separation from [RFC 013](../rfcs/013-domain-constraints-and-projections.md).
Improve identity, evidence, and update rules before expanding the collector to more providers.
Do not introduce a universal policy language or execute imported action code in this milestone.

## Evidence reviewed

The review uses the hosted program, native host, live GitHub captures, and the current local monitor.
The [live proof](../acceptance/integration-host/proof.json) and [Git helper proof](../acceptance/integration-host/git-credential-proof.json) contain reproducible summaries.
The [review inventory](../acceptance/integration-host/model-review.json) records the counts used below.

| Observed state | Interpretation |
| --- | --- |
| Nine definitions: one workflow and eight action definitions | Different action revisions remain distinct definitions |
| Ten implementation records | Source excerpts and lexical clues; not proven requirements or runtime effects |
| 76 requirement entries | Two job summaries plus action input declarations; not 76 independent enforcement controls |
| Eight references and 26 expressions | Source evidence exists, but the hosted model has no resolved invocation graph |
| Three current status facts | One workflow attempt and two jobs |
| Three distinct outcomes | Repeated polls do not create new external executions |
| Nine byte-limit failures, five source-sample gaps, two truncation gaps | Successful extraction still has partial coverage |
| Two job durations: 26 seconds and 11 seconds | Provider-reported execution duration; workflow duration remains unknown |

The native host checks envelopes, grants, receipts, and budgets. It does not validate the meaning of each record's arbitrary `data` object.
The program emits job dependencies as either a string or an array, matching source syntax.
Both job outcomes have one name-based declaration candidate, but `declared_job_id` remains null.
An inherited projection gap still says run results were not captured, although the hosted path captures a sample.
These are concrete consistency issues to resolve before agents rely on a normalized model.

The current monitor exposes installed code, grants, input, schedule, history, facts, and source evidence.
The private local launcher establishes browser access. The shareable snapshot contains evidence and installed program data, without the operator credential.
There is no candidate approval gate, semantic diff, or automatic desired-state comparison yet.

## Priorities and acceptance checks

| Priority | Recommendation | Why it matters | Proof of completion |
| --- | --- | --- | --- |
| P0 | Version the record schema and separate evidence from constraints | An excerpt must not look like an enforceable requirement | Reject malformed records; render declarations, clues, constraints, facts, and outcomes with distinct labels |
| P0 | Give logical controls, revisions, invocations, and executions separate identities | Agents need stable references across edits and repeated calls | Two calls to one action remain distinct; a source edit creates a revision of the same logical control |
| P0 | Record coverage per collection scope and source operation | Missing reads must not imply removed requirements | Denied or truncated reads preserve existing requirements and mark the affected scope unknown |
| P0 | Add reviewable revisions and semantic diffs | Changes must be explainable and reversible | Compare two captures, identify changed requirements, and restore the previous accepted revision |
| P1 | Expose an operation-centered explanation | A planner needs prerequisites, evidence gaps, and permitted next observations | Explain publish readiness without reading raw JSON or treating unknowns as success |
| P1 | Separate collection plans for immutable definitions and mutable state | Repeated full traversal wastes work and obscures freshness | A status refresh does not reread unchanged pinned definitions; cache use does not renew fact validity |
| P1 | Retain external outcomes independently of collector runs | Eight polls are not a useful historical retention policy | An outcome survives collector-log eviction; repeated observations do not inflate metrics |
| P2 | Extend collection to protection rules, artifacts, and deployments | Source declarations alone do not explain all external gates or effects | Link one actual gate and one deployment outcome to the selected operation, with explicit inaccessible scopes |

Implement P0 as one vertical slice. Preserve the current source-backed JSON throughout migration.
Treat the remaining priorities as follow-up increments, not one large rewrite.

## Fact collection

Keep collection infrastructure in Rust: connections, credential resolution, retries, rate budgets, response limits, and durable checkpoints.
Keep provider interpretation and traversal in the installed program.
Give the program typed response metadata without exposing secrets or permitting arbitrary network access.

Each source observation should record:

- Source operation, permitted parameters, receipt, response digest, and API version.
- Provider revision or ETag, when available.
- Request start, response completion, and provider event time as separate fields.
- Discovery principal reference and visibility scope, without credential material.
- Pagination state, truncation, access failures, and snapshot consistency limits.

The current broker detects truncation when a returned array exceeds its configured limit.
It does not prove that the provider returned every page or that an upstream tree was complete.
Preserve provider pagination and truncation metadata before making any negative assertion.
A missing record is not evidence of deletion until its exact collection scope has supported completeness evidence.

Split the collection plan into three parts:

1. Read immutable definitions by repository, commit, and path. Reuse verified content by digest.
2. Observe mutable state, such as active attempts and protection settings, under explicit freshness requirements.
3. Observe terminal outcomes and retain their history independently of current facts.

Use conditional requests where supported. A validated unchanged response can establish a new observation; replaying cached bytes cannot.
Use bounded rescheduling for long rate-limit delays instead of holding a worker or treating throttling as a policy failure.
GitHub recommends conditional requests and webhook-driven updates where possible. Add webhooks only after reconciliation works reliably. [GitHub API guidance](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api)

For large action bundles, retain the explicit gap first.
A later content-addressed evidence store could hash bounded streams outside the guest and expose selected excerpts.
Do not raise sandbox limits merely to fit bundled JavaScript. Do not infer source-to-bundle equivalence from repository proximity.

## Contract construction

Use a small typed graph, with provider-specific payloads where normalization would lose meaning.

| Entity | Identity and purpose |
| --- | --- |
| Definition | Repository, commit, path, and content digest; immutable source |
| Logical control | Stable provider namespace, resource, operation, and declared identifier |
| Control revision | Logical control plus source and mapping revisions |
| Invocation | Parent definition and call-site pointer, with supplied and defaulted inputs |
| Execution | Provider run, attempt, job, and step identities |
| Evidence | Source observations and field-level references that support a claim |

Link entities with typed relationships: `depends_on`, `invokes`, `requires_evidence`, `enforced_by`, and `observed_execution`.
Record a relationship's evidence and resolution status. A suggested name match is a candidate edge, not a resolved edge.

Distinguish four kinds of statements:

- **Declaration:** the source requests a permission or declares an input.
- **Constraint:** an external enforcer requires something for a particular operation and actor context.
- **Observation:** the provider reported a value at a stated time.
- **Inference:** a mapping or analysis suggests a relationship or effect that still needs support.

Source excerpts belong in evidence records. They can support a claim, but are not themselves constraints.
Use explicit claim status such as `declared`, `provider_reported`, `inferred`, or `unknown`.
Avoid a single numeric confidence score that hides these differences.

For each constraint, define subject, operation, actor context, applicability, obligation, evidence requirements, dependencies, enforcer, and scope.
Keep original expressions and their dialect. Normalize only supported forms, with tests for defaults and provider-specific behavior.
For example, a job dependency is not merely a Boolean prerequisite under every condition; GitHub permits conditional handling of dependent jobs. [Workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)

Keep requested token permissions separate from effective permissions.
Keep discovery identity separate from the intended execution actor.
The next protection-rule collector must distinguish active, evaluate-only, and disabled rules where its endpoint exposes them.
GitHub's branch-rule endpoint returns active applicable rules and excludes evaluate-only and disabled rules. That endpoint alone cannot establish their absence. [Repository rules API](https://docs.github.com/en/rest/repos/rules)

## Reasoning and human consumption

Start review from an operation, such as “Publish docs for commit X,” rather than a list of captured files.
Show five linked sections:

1. Requirements and applicability.
2. Current supporting evidence, with freshness and source.
3. Unknown or unmet prerequisites.
4. Previous execution outcomes and performance.
5. Changes from the previous reviewed revision.

Return the same structure to agents through a versioned JSON projection.
Each requirement should have an assessment: `satisfied`, `unsatisfied`, `unknown`, or `not_applicable`.
Include the assessment time, actor context, evidence references, and explanation.
Absence, expiry, and access denial must lead to an explicit unknown state where they prevent assessment.

A useful answer for the current capture is:

> GitHub reported a successful publish job for the selected workflow attempt. Its execution duration was 11 seconds.
> The declaration link is a name-based candidate. Environment protection and effective actor permissions remain unobserved.
> This outcome does not establish permission for a future deployment.

Suggested next observations should identify their expected value and required host capability.
They are planning hints, not grants. Source text must not become executable instructions for an agent.
An alternative trajectory must satisfy its own applicable constraints; a failed control does not authorize bypass.

Prefer progressive detail in the human view: operation summary, requirement explanation, supporting evidence, then raw source.
Show extraction status and coverage separately. A completed collector run must not imply complete contract knowledge.

## Updates and reviewable state

Create an immutable review manifest for each candidate revision.
Pin source observations, installed code digest, operation-grant digest, input, domain schema version, and projection version.
The current digest covers code and SDK, but does not by itself pin every collection or interpretation setting.
Retain the source needed to replay an earlier revision, not only the digest of a replaced program.

Compare semantic changes separately from evidence refreshes:

- Requirement added, changed, removed, or no longer observable.
- Source revision changed with no supported semantic difference.
- Evidence refreshed, expired, or contradicted.
- Execution outcome added or updated for the same attempt.
- Mapping or schema changed while source evidence stayed fixed.

Use `candidate → reviewed → accepted` for the catalog if review gates are introduced.
This acceptance does not activate executable enforcement policy.
Fact publication and policy activation remain separate operations, as specified by [RFC 014](../rfcs/014-fact-publication-and-synchronization.md).

Require explicit scoped deletion evidence or a supported tombstone before removing a requirement.
The current upsert-only store preserves requirements safely after failed reads, but can accumulate obsolete revisions until its capacity limit.
Add reconciliation tests before enabling deletion. Never replace a complete prior scope with a partial new scope.

Freshness must start from the supporting observation, not from delayed publication or replay.
Keep source event time, observation time, fact validity deadline, and host commit time distinct.
Historical outcomes remain useful after a current status fact expires.

## Performance history

Keep collector metrics, external execution metrics, and assessment of a control's effectiveness separate.
A successful job is an execution result, not proof that a protection control prevented unauthorized work.

Use attempt-level identities for workflow and job history. Keep step ordering and reported timestamps.
GitHub exposes job and step status, conclusion, start time, and completion time; use those fields without inventing missing lifecycle times. [Workflow jobs API](https://docs.github.com/en/rest/actions/workflow-jobs)

Retain outcome changes as observations under one execution identity.
Store definitions for duration units, eligible populations, retry treatment, and incomplete executions before computing aggregates.
Show sample count and observation window with every rate or percentile.
Do not combine unrelated control revisions or treat every poll as another sample.

The immediate improvement is durable terminal-outcome retention with explicit bounds and export.
A time-series backend is not required for the first proof.
Later, compare cohorts by control revision, execution context, and provider-reported conclusion.

## Recommended next experiment

Use two pinned revisions of the docs workflow and synthetic runtime responses.
Change one dependency, one applicability condition, and one action input binding.
Include a matrix job, a repeated action call, a rerun, an inaccessible protection rule, and an incomplete page.

The review must answer:

- What changed, and which source supports that change?
- Which operation and actor context does each requirement govern?
- Which prerequisites are satisfied, unsatisfied, unknown, or not applicable?
- Which outcomes belong to the same execution attempt?
- What evidence is missing, and which permitted read could supply it?
- Can we replay the old mapping and restore the previous reviewed revision?

Use deterministic fixtures for these claims. Keep the live GitHub run as a separate integration proof.
After this works, test the same vocabulary against one ServiceNow state transition.
That counterexample should determine which concepts generalize and which must remain provider-specific.
