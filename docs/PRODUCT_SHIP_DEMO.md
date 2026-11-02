# Product ship qualification simulation

This example models a central product ship policy and nine linked qualification policies.
It supplies nine headless integration instances, one shared SDK program, and three interactive UX studies.
All evidence is sample data. The example does not approve a real release or activate runtime enforcement.

## Policy network

The central policy requires every listed child policy. Each child requires its listed controls.
Each control identifies its evidence source. Evidence must identify the same release, artifact digest, and environment.
These relationships use explicit identifiers. Matching titles do not create links.

| Child policy | Example controls |
| --- | --- |
| Accessibility | Keyboard navigation; screen reader labels |
| Security | Security review; no blocking dependency findings |
| Open-source legal | License approval; third-party notices |
| Build provenance | Source revision; release artifact digest |
| Test quality | Required suites; coverage threshold |
| Privacy | Privacy review; data inventory |
| Change control | Recorded approval; change record names the artifact |
| Reliability | Operational readiness review; recovery plan |
| Operations | Release monitoring; support ownership |

These are example enterprise requirements, not a complete compliance standard.
The example uses a fixed evaluation time for repeatable results.
Missing, expired, future, or mismatched evidence cannot establish that a control passed.
An explicit failed control produces a simulated deny decision. Otherwise, missing evidence produces an indeterminate decision.
Only complete matching evidence permits a simulated allow decision.

The model records parent and child links for this example.
Native governance records describe policies and controls, but do not implement recursive policy enforcement.
The separate demo evaluator interprets the example links. Its result is not a native authorization decision.

## Author and run

Edit [the model](../testing/integration/hosted/product-ship/product-ship.json) to author the central policy, children, controls, and sample evidence.
Keep policy and control identifiers unique. Include every child identifier in the central policy's `children` list.
Set each child's `parent_id` to the central policy identifier.

The [SDK collector](../testing/integration/hosted/product-ship/collector.js) requests data through a named broker operation.
It emits source-backed governance descriptions and expiring facts with read receipt references.
The native host retains its grant, receipt, schema, and output checks.
Each instance selects one child policy through `input.policy_id` and emits only its controls and evidence.
The replay runs the actual embedded SDK with synthetic read responses. It needs no external account or credential.
The output lists instance configuration under `integrations` and separate test cases under `scenarios`.

Generate the mock data:

```sh
node testing/integration/hosted/product-ship/runner.cjs --output tools/product-ship-mocks/data.json
```

Start the local mock server:

```sh
python3 tools/product-ship-mocks/serve.py
```

Open [the three UX studies](http://127.0.0.1:8768/).
The server exposes only the mock assets and the shared demo evaluator.
It does not expose repository files or the integration host's private credentials.

## Three UX studies

| Study | Main question | Navigation |
| --- | --- | --- |
| Release readiness | What prevents this release from shipping? | Expand the combined policy checklist, select a control, and explore its evidence. |
| Policy network | How do the central policy and child policies connect? | Select a connected policy and follow its requirements. |
| Evaluation console | Why would the release gate allow, deny, or wait? | Run a simulated check and inspect its decision trace. |

The readiness view now combines policies and controls in one checklist.
The evidence workspace uses most of the screen width. Select **Expand evidence** to hide the checklist temporarily.
Use **Connections** to inspect the policy, control, check, observation, source, and release context.
Select an object or connection to read its fields and matching reason.
Dashed object borders mark missing evidence or an artifact mismatch.
Use **Evidence record** for source fields. Expand the application details or full record for further review.
The map describes fixture relationships and a simulated check. It does not claim a recorded production enforcement event.

All three use the same generated data and evaluator.
Switch scenarios to compare ready, blocked, missing evidence, expired evidence, and the wrong artifact.
Scenario changes affect the browser simulation only.

## Systems, source records, and responsible people

The readiness view includes a **Systems & people** preview for each selected control.
It links the control to required ServiceNow records, role assignments, people, and their email or Slack profiles.
The default evidence view also summarizes the available source records and assigned owners.

Record identity uses the instance, table, and `sys_id`. The record number is a display label, not a join key.
The view compares the required state with the observed source state. It preserves source revision, read receipt, observation time, and expiry.
The table names and state values are fixture definitions. They do not specify a universal ServiceNow schema.

Assignments identify a person by an explicit source identifier. A matching display name does not resolve a person.
Owner, reviewer, and approver are distinct roles. A listed approver does not prove that an approval occurred.
Missing, unresolved, or expired profile data remains visible. Contact profiles do not authorize messaging or approval.

The [provenance collector](../testing/integration/hosted/product-ship/provenance-collector.js) uses the SDK and the named `read_control_context` operation.
It emits source evidence and governance descriptions for people, roles, assignments, and contact methods.
The [headless provenance runner](../testing/integration/hosted/product-ship/provenance-runner.cjs) derives the preview data from those emissions.
Regenerate `data.json` with the existing runner command to include this context.

This preview does not connect to real ServiceNow or Slack accounts. All people and records are synthetic.
Source record context and contact resolution remain separate from the existing qualification evaluator.
A production binding must specify which record states and role actions a control requires before they can affect its result.

## Limits

The synthetic integrations do not query accessibility tools, vulnerability scanners, legal systems, or approval systems.
A native host test checks collection with a loopback provider. It does not establish live provider support.
The host permits four installed programs per state directory. The nine-instance test uses isolated hosts and preserves this limit.
The native fact TTL bounds retained records. The demo separately checks the source observation and expiry timestamps stored in each record.
Policy authoring changes this fixture. It does not publish an enforcement policy or create a production deployment gate.
Production use still needs reviewed evidence bindings, authority rules, policy revision pins, and runtime activation.
