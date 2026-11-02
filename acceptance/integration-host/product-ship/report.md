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
