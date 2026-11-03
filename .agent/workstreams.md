# Workstream register

Verified on 2026-11-02 from local worktrees and GitHub pull requests.
This register separates current work from merged branches and proposed features.
Update the status and next action when a workstream changes.

## Active work

| Workstream | Location | State | Next action |
| --- | --- | --- | --- |
| Integration host usability | `feat/integration-host-usability`, main checkout | Host usability and coverage simplification are committed. This pass adds shared native coverage explanations and snapshot support. Local server rebuilt with existing state. | Review the new Runtime and Integration coverage views, then prepare the branch for integration. |
| Admin control plane architecture | `docs/admin-architecture-review`, sibling `ipe-admin-review` worktree | Proposal and exports committed in `7bc4be5`. No control plane implementation is claimed. | Review the architecture against current main before opening a PR. |
| Admin UX prototype | Separate repository at `ipe-admin-review/design/admin-ux-review` | Clean at `d353faa` on its own `main`; no Git remote is configured. | Decide how to retain and review this prototype with the architecture work. |
| Dependency maintenance | GitHub PRs [57](https://github.com/jrepp/ipe/pull/57) and [58](https://github.com/jrepp/ipe/pull/58) | Open Dependabot updates for setup-python and setup-node. | Review CI results and the existing pinned versions before merging. |

The nested prototype is already committed in its own repository. The parent worktree shows `design/` as untracked.
It is not an uncommitted source change or a configured submodule. Do not add an unusable Git link without a source repository.

## Product ship qualification study

The usability branch also contains a synthetic product ship policy network with nine child policies.
SDK programs collect sample controls and evidence. Three interactive studies compare readiness, network navigation, and simulated enforcement.
See [the demo guide](../docs/PRODUCT_SHIP_DEMO.md) for authoring and launch steps.
The study does not activate policies or enforce a real release gate. The selected direction combines a policy/control checklist with a larger evidence workspace.
A synthetic provenance pass adds ServiceNow source records, assigned roles, and email or Slack profile context.
An agent evaluation preview adds per-step activity and illustrative token/time comparisons. Actual provider usage is not measured. Review it before integration into the monitor.

## Integration host scope

The usability branch contains these related changes:

- Runtime and Integration coverage navigation, source comparisons, record filters, and run inspection.
- Coverage simplification: requirement, operation, and execution explanations first; record connections in supporting details.
- Shared native coverage explanations now serve the monitor and agent clients for one retained run. Snapshot exports retain the same responses.
- Design recommendations updated from the ontology article review. Exact governance requirement-to-check bindings remain the next proposed system change.
- SDK helpers, bounded progress and emission traces, v2 domain APIs, and contextual evidence assessments.
- GitHub environment and ruleset capture, governance descriptions, and source-backed handoff descriptions.
- Vocabulary and model documentation, acceptance evidence, and local documentation packaging.

The [host guide](../docs/INTEGRATION_HOST.md) describes implemented behavior and local restart steps.
The [usability report](../acceptance/integration-host/usability/report.md) records checks and limits for each pass.
The [domain review](../docs/INTEGRATION_DOMAIN_MODEL.md) identifies proposed lifecycle extensions.

Requirement fulfillment, handoff events, enforcement fact publication, and policy activation remain incomplete or proposed.
Ticket and compliance evidence joins use synthetic providers. They do not establish live account integration.
These are future scope, not separate implementation workstreams currently running in this checkout.

## Merged work

| Workstream | Evidence | Local state |
| --- | --- | --- |
| Policy workspace mock | [PR 54](https://github.com/jrepp/ipe/pull/54), merged | `ipe-policy-workspace-pr` is clean. |
| GitHub release changelog automation | [PR 56](https://github.com/jrepp/ipe/pull/56), merged | `ipe-release-changelog` is clean. |
| Hosted contract capture | [PR 55](https://github.com/jrepp/ipe/pull/55), merged | Baseline for the usability branch. |
| Script SDK and merge repair | [PR 51](https://github.com/jrepp/ipe/pull/51), merged | `ipe-pr-51` is clean. |
| Agent guidance and CI duration | [PR 53](https://github.com/jrepp/ipe/pull/53) and [PR 52](https://github.com/jrepp/ipe/pull/52), merged | Retained local branches do not mean active work. |

The usability branch started at `2eaa394`. Fetched main is `733f6d7`, which also contains PRs 54 and 56.
The local commits have not been pushed. No PR, merge, or deployment was performed during this pass.

The policy chain preview now includes a collapsible usage report. Live usage collection and measured optimization remain future work.
