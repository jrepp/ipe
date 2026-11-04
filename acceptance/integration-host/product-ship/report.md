# Product ship qualification study

Verified on 2026-11-02.

## Scope

The example contains one central policy, nine child policies, and 18 required controls.
Nine headless integration instances use one SDK collector with separate policy inputs.
Each instance emits its selected policy, controls, and evidence. The central declaration retains all nine child requirements.
The generated review data combines evidence from those collector runs.

Five scenarios exercise ready evidence, a failed control, missing evidence, expired evidence, and a mismatched artifact.
The shared demo evaluator returns allow, deny, indeterminate, indeterminate, and indeterminate, respectively.
It uses the model's fixed observation clock. The UI states that time and labels decisions as simulations.

## UX evidence

| Study | Evidence | Purpose |
| --- | --- | --- |
| Release gate board | [Screenshot](screenshots/board.png) | Compare all required policies, then inspect one control. |
| Policy network | [Screenshot](screenshots/network.png) | Navigate central and child policies through explicit links. |
| Enforcement console | [Screenshot](screenshots/console.png) | Inspect the simulated decision and each supporting check. |

All three preserve the scenario, policy, and control when moving between views.
Missing evidence displays no record. Expired and mismatched evidence retain the values that caused the unknown result.
The readiness total includes all 18 controls, independent of the selected policy.

Browser checks found no script errors and no page overflow at a 390-pixel viewport.
Axe reported zero violations on the overview and all three studies.
Screenshots were inspected at a 1440-pixel viewport. Automated checks do not establish accessibility certification.

## Validation

- Integration crate tests: 138 passed; two opt-in acceptance tests remained ignored.
- JavaScript checks: 84 passed, including eight model/collector tests and four new interaction tests.
- Integration crate Clippy passed with all targets, all features, and warnings denied.
- Rust formatting and whitespace checks passed.
- Documentation checks passed.

The native fixture executes the collector through the Rust host with a loopback provider.
It checks the complete hierarchy and each of the nine policy-specific installations.
It preserves unknown status, wrong-artifact data, source identity, read receipts, bounded traces, and fact TTLs.
Missing output grants reject publication. A denied provider read produces no fabricated evidence.
The fixture uses isolated hosts for the nine instances and preserves the four-installation limit.

The mock server returned 404 for repository metadata and private token paths.
It serves an explicit asset list, not the repository directory.

## Limits

No external provider, deployment, or enforcement policy was changed.
The browser evaluator is a simulation; native recursive policy enforcement is not implemented by this study.
The Node replay uses the embedded SDK for deterministic checks. It is not a security sandbox.
The existing integration monitor and its private state remain separate from these mockups.
See [the authoring guide](../../../docs/PRODUCT_SHIP_DEMO.md) for model files and launch commands.


## Combined checklist and evidence workspace

The readiness view now uses one compact frame for the policy chain and its controls.
The main workspace shows the selected control's evidence map and an object or connection inspector.
Users can expand the evidence area, switch to source fields, and open policy application details or the full record.
Expected and reported artifact identities remain separate. Missing evidence does not create a source record.
Evidence: [Checklist and evidence workspace](screenshots/checklist-evidence.png).

The focused suite passed 15 tests: eight model/collector tests and seven UI tests.
New checks cover checklist expansion, selection and focus, full-width evidence, relationship inspection, missing records, and open details.
Browser checks found zero Axe violations on desktop and mobile, no script errors, and no overflow at 390 pixels.
This pass changes mock UI and documentation only. Native behavior remains unchanged; Rust tests were not rerun.


## System provenance and responsible people

The preview now links each of the 18 controls to synthetic ServiceNow record context and responsible roles.
Canonical identity includes the instance, table, and `sys_id`. The view shows required and observed record states separately.
Owner, reviewer, and approver assignments refer to people by explicit source IDs. Names do not resolve missing or duplicate identities.
Email and Slack profiles retain source, observation time, and expiry. Unavailable or stale contacts remain visible without active contact links.
The compact summary opens a larger source-object inspector through **Systems & people**.
Evidence: [Source record](screenshots/system-provenance.png) and [role contact profiles](screenshots/roles-and-profiles.png).

The SDK program performs a named broker read and emits evidence plus typed governance descriptions.
The generated preview uses those emitted observations. It keeps broker receipts separate from source-provided receipt strings.
The native host test verifies output grants, receipt chains, temporal bounds, unresolved references, and denied reads without publication.
Node tests also cover truncated reads, duplicate people, modified source receipts, and preserved canonical profile IDs.

Integration crate tests passed: 139 tests, with two opt-in acceptance tests ignored.
All 97 JavaScript tests passed. These include 12 UI tests and five provenance extraction tests.
Integration crate all-feature Clippy, formatting, documentation, and whitespace checks passed.
Desktop and mobile browser checks found zero Axe violations, no script errors, and no page overflow at 390 pixels.

All records and people are synthetic. No ServiceNow or Slack account was connected and no message was sent.
Record context and role assignments do not change the qualification evaluator or establish that an approval occurred.


## Agent evaluation and illustrative efficiency

The readiness view now shows distinct agents, queried controls, and qualified controls for the selected scenario.
Checklist rows show how many agents queried and qualified each control.
The selected-control ledger separates queries from qualification attempts and links each cited receipt to its evidence record.
The ready fixture contains six agents, 18 queried controls, 15 qualified controls, and three query-only controls.
A repeated query increases call count, not the number of unique agents or controls.
Evidence: [Agent evaluation](screenshots/agent-evaluation.png) and [comparison details](screenshots/agent-comparison.png).

Each scenario comparison includes model preparation once. It reports input/output tokens, elapsed time, and summed work separately.
The raw-source baseline uses fixture totals for equivalent work. All activity and usage values are illustrative, not measured agent savings.
Missing or zero baselines suppress savings claims. Additional usage remains visible as more tokens or slower work.
Activity is hidden when policy, evidence, or event scope no longer matches the selected release and scenario.
Policy manifests remain separate snapshots when the caller changes the current model.

All 105 JavaScript tests passed. These include four activity model tests and 16 mock UI tests.
Documentation and whitespace checks passed. No Rust runtime or SDK behavior changed, so Rust tests were not rerun.
Desktop and mobile checks found zero Axe violations, no script errors, and no page overflow at 390 pixels.
No live agent usage was collected and no agent service was called for this fixture.

## Policy chain usage report

The default preview replaces the metric strip with a closed chain report. Control activity remains in the evidence workspace.
The report separates how agents use the chain, its ship gate, findings, intended optimization, and resource cost.
Run history counts five simulated scenario runs. Page views do not add uses. Mismatched runs do not enter the count.
Findings open the affected control. Policy outcomes remain separate from agent qualification coverage.
Context use means cumulative input tokens. The report separates output tokens, elapsed time, and summed work.
All comparisons remain illustrative. No dollar cost, model training, avoided work, or live enforcement is measured.
Evidence: [Chain usage report](screenshots/policy-usage-report.png).

All 107 JavaScript tests passed. The final UI run passed all 18 tests.
Desktop and mobile accessibility scans found zero violations. The mobile page fits a 390-pixel viewport.
No browser script errors were reported. Documentation and whitespace checks passed.
No runtime or SDK behavior changed; Rust tests were not rerun.

## Simple English copy pass

The preview text follows the local Simple English skill in pragmatic mode.
The rewrite covers the usage report, agent activity, source details, evidence explanations, view introductions, and load errors.
The copy review checks sentence length, active voice, consistent terms, and sample-data labels.
Source values, identifiers, and policy behavior remain unchanged.
Evidence: [Revised preview text](screenshots/simple-english-preview.png).

All 18 UI tests passed. Documentation and whitespace checks passed.
Desktop and mobile accessibility scans found zero violations. The page fits a 390-pixel viewport and reports no browser script errors.
Rust tests were not rerun for this text change.

## Policy report placement and design skill

The usage report opens in the detail workspace beside the policy checklist.
The central policy includes a report link and the count of simulated runs.
Control evidence remains the default view. Report findings open the affected control, and view changes preserve the selected control.
The report uses direct labels for activity, decisions, findings, comparisons, and resource use.
Evidence: [Policy usage workspace](screenshots/policy-usage-workspace.png).

The project-local IPE design skill records user feedback, plain-language rules, and checks for policy interfaces.
The agent guide links to the skill. Its feedback record marks this layout as a revision for user review.
The skill validator passed. All 19 UI tests passed, including report navigation and control selection.
Desktop and mobile accessibility scans found zero violations. The page fits a 390-pixel viewport and reports no script errors.
Documentation and whitespace checks passed. Rust tests were not rerun for this UI and guidance change.

## Contemporary Plex overview

The overview replaces the sales banner with run totals, completion rate, allow rate, and elapsed time saved.
Daily charts show release outcomes, evaluation health, and mean evaluation time. The period selector updates all history metrics.
The daily table gives exact values. Current release evidence and scenario controls remain separate from history.
The preview reuses IBM Plex Sans and the Contemporary Plex style from the earlier workspace study.
Fonts and their license are local. The server permits only named font assets.
Evidence: [Desktop overview](screenshots/overview-plex.png) and [mobile overview](screenshots/overview-plex-mobile.png).

Seven generated daily summaries contain 168 sample evaluations: 165 completed and three errors.
The sample outcomes include 102 allowed, 28 denied, and 35 unknown results.
Completion and known-result rates use separate denominators. Errors do not count as denied or unknown outcomes.
Cost comparisons cover the same completed runs and include preparation once per modeled run.
All history values are sample data. They do not measure production health or actual savings.

All 111 JavaScript tests passed. They cover period totals, scenario links, invalid history, missing data, zero baselines, and additional cost.
Desktop and mobile overview scans found zero accessibility violations. The policy report scan also found zero violations after shared style changes.
The mobile overview fits a 390-pixel viewport. No browser script errors were reported.
Font files returned HTTP 200. Unlisted repository paths and traversal requests returned HTTP 404.
The design skill validator, documentation checks, and whitespace checks passed. Rust tests were not rerun for this preview change.

## Header cleanup

The shared header removes the secondary product label and uses a compact sample-data badge.
Each page has one short title that matches its navigation label. Repeated introductory text no longer precedes the content.
Evidence: [Overview headers](screenshots/overview-headers.png).
All 21 UI tests passed. Desktop and mobile accessibility scans found zero violations.
The overview fits a 390-pixel viewport. Documentation and whitespace checks passed.
Rust tests were not rerun for this text and layout change.
