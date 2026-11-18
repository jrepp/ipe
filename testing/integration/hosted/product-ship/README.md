# Product ship qualification demo

This demo models one release candidate and nine linked subpolicies. The controls cover accessibility, security, open-source legal review, build provenance, test quality, privacy, change control, reliability, and operations.

The central policy's `children` list defines the required policy tree. The evaluator checks evidence against release, artifact, environment, source, observation time, and expiry. Duplicate evidence, missing policies, expired evidence, and identity mismatches produce `unknown`. Any `not_met` required control produces `deny`; otherwise unknown evidence produces `indeterminate`. The evaluator uses the model's fixed ISO 8601 `as_of` time.

The policy tree is descriptive input for this simulation. It does not establish native hierarchy enforcement or a production release gate. The controls are proposed examples. They do not assert legal or compliance certification.

The collector runs the embedded SDK source against a deterministic broker replay. The Node vm is a test harness, not a security sandbox. Hosted execution relies on the native host's sandbox and grant checks. The collector emits receipt-bound governance descriptions and expiring evidence observations. A denied read emits an unknown result and no evidence.

The qualification collector separates response handling from projection and constructs at most four observations per step. The provenance collector constructs at most eight. The collector keeps compact provider data between steps, instead of copying all expanded observations. Native byte limits and the 100 ms step deadline remain in force.

```sh
node --test testing/integration/hosted/product-ship/product-ship.test.cjs
node testing/integration/hosted/product-ship/runner.cjs --output tools/product-ship-mocks/data.json
```

Omit `--output` to print JSON to standard output. The output has the shared model fields at the top level, with nine logical source-domain instances under `integrations`, each with its collector program, input, operation grant, and emitted evidence count. The `scenarios` array and `assessments` map show the five qualification outcomes over the evidence aggregated from those nine captures. The integrations are synthetic runs of one simulated broker, not live external systems.
