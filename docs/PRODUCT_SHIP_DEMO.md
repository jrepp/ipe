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
| Release readiness | What prevents this release from shipping? | Open a policy card, then inspect its controls and evidence. |
| Policy network | How do the central policy and child policies connect? | Select a connected policy and follow its requirements. |
| Evaluation console | Why would the release gate allow, deny, or wait? | Run a simulated check and inspect its decision trace. |

All three use the same generated data and evaluator.
Switch scenarios to compare ready, blocked, missing evidence, expired evidence, and the wrong artifact.
Scenario changes affect the browser simulation only.

## Limits

The synthetic integrations do not query accessibility tools, vulnerability scanners, legal systems, or approval systems.
A native host test checks collection with a loopback provider. It does not establish live provider support.
The host permits four installed programs per state directory. The nine-instance test uses isolated hosts and preserves this limit.
The native fact TTL bounds retained records. The demo separately checks the source observation and expiry timestamps stored in each record.
Policy authoring changes this fixture. It does not publish an enforcement policy or create a production deployment gate.
Production use still needs reviewed evidence bindings, authority rules, policy revision pins, and runtime activation.
