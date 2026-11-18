# Integration sandbox SDK and package cache

See the [integration domain model review](INTEGRATION_DOMAIN_MODEL.md) for proposed semantic boundaries and migration questions.
The API described below is the implemented SDK contract, including the first compatible domain refactor.

The workload provides the global `ipe` SDK before evaluating each JavaScript or
TypeScript step. Its implementation is embedded in the Rust binary, frozen, and
included in the adapter digest. There are no SDK imports to resolve at runtime.
TypeScript declarations live in
[`sdk/index.d.ts`](../crates/ipe-integration/sdk/index.d.ts); the
[typed example](../testing/integration/adapters/read-step.ts) demonstrates the
complete read/resume cycle. The SDK version is advertised in capabilities and in
the guest context, alongside permitted operation names and parameter names.

```javascript
function step(context, responses, state) {
  if (state === null) {
    return ipe.requests([ipe.request("catalog", "metadata")], { phase: "read" });
  }
  const result = ipe.response(responses, "catalog");
  if (result.code !== undefined) {
    return ipe.complete({ status: "unknown", code: result.code });
  }
  const rows = ipe.records(responses, "catalog");
  return ipe.complete({ rows: rows.map(row => ipe.pick(row, ["name"])), receipt: result.receipt });
}
```

| Helper | Purpose |
| --- | --- |
| `observation(model, data, receipts)` | Emit a v1 typed model with separate raw data and native receipt references. |
| `call(id, operation, params = {})` | Construct a hosted operation request with bounded strings; native grants remain authoritative. |
| `exchange(items, state, observations)` | Return requests and up to 16 typed observations with receipt references. |
| `timestamp(value)` | Parse an explicit ISO timestamp to milliseconds; return null for invalid input. |
| `request(id, operation, params = {})` | Construct a named broker request; reject unsupported parameter syntax. |
| `requests(items, state = null)` | Return up to eight requests with explicit continuation state; reject duplicate IDs. |
| `response(responses, id)` | Find exactly one response, including its host error code or receipt. |
| `records(responses, id)` | Require a successful row response; return a JSON copy. |
| `pick(record, fields)` | Project own properties without prototype assignment. |
| `state(value, initial = null)` | Copy persisted state or initialize a new run. |
| `complete(outputs, state = null)` | Finish with a validated JSON result. |

SDK validation rejects cyclic/non-JSON values, nesting beyond 32 levels, and
results exceeding 64 KiB in UTF-8. It is an authoring aid, not an authorization
boundary: Rust independently validates requests, grants, budgets and output.
No helper performs network I/O, waits, accesses secrets, or publishes facts.
State is per-run continuation data, not a durable checkpoint. Each step has a new
context; use state rather than mutable globals. Existing raw `json-step-v1`
adapters remain supported.

## Hosted programs

SDK 0.5 supports [hosted integration programs](INTEGRATION_HOST.md). The endpoint installs JavaScript source and runs it manually or on a schedule.
Programs emit typed definitions, evidence, constraints, invocations, expiring facts, and external outcomes into bounded local storage.
See [the typed observation example](../testing/integration/hosted/typed-observation.ts). The host derives logical keys and revision digests. Enforcement publication remains separate.

## Hosted authoring helpers

SDK 0.6 adds `OperationId`, `RequirementSpecification`, `NamedRequirement`, and typed v2 operation and assessment API contracts.
`ControlId` and `Requirement` remain compatibility aliases. V1 `Obligation` remains a named assessment item, not proof of an actor's duty.
`ipe.operationKey` uses the existing `control:` encoding; `ipe.controlKey` remains available with identical output.
These additions do not introduce new emitted record kinds or network access from guest code.
See the [domain type example](../testing/integration/hosted/domain-types.ts) and [v2 assessment guide](INTEGRATION_ASSESSMENTS.md#versioned-domain-apis).

SDK source changes alter program and mapped record digests. Review assessment rule pins after an SDK update.

SDK 0.7 adds source-backed governance descriptions through `ipe.governanceModel(record, {scope, claim})`.
Use `ipe.observation` with successful native read receipts to emit the resulting model v2 envelope.
Policies, contracts, controls, handoffs, principals, role assignments, and contact methods have separate typed fields.
Existing `Model` and `Observation` types remain v1; `EmittedModel` and `EmittedObservation` include governance.
See the [governance schema and limits](INTEGRATION_HOST.md#governance-extraction-and-execution-overlay) and [typed example](../testing/integration/hosted/governance-types.ts).
The builder does not resolve principals, verify authority, or contact people. Native validation checks grants, receipts, references, and bounds.
Existing stored records remain readable. Accepting a catalog and updating installed code remain separate operations.

SDK 0.4 adds a bounded work plan and model helpers. Existing `step` programs remain supported.
The SDK version and source contribute to the collector digest. Upgrading the SDK changes that digest, even without program edits.

| Helper | Purpose |
| --- | --- |
| `tasks(state, options)` | Resume a JSON work plan with a bounded queue, deduplication, request IDs, response matching, and gap accounting. |
| `observed(receipts, scope, claim).emit(entity, data)` | Build model envelopes with explicit scope and claim, using the supplied receipts. |
| `readResult(responses, id)` | Return an error or a read result with `observations(scope, claim)` bound to its receipt. |
| `project(record, fields, excluded)` | Select fields and retain other fields under `unmodeled`. Excluded fields are handled by the caller. |
| `emitBatch(items, mapper, maximum)` | Map a bounded batch, then remove its items. A mapper failure leaves the input queue intact. |
| `elapsed(start, end)` | Return milliseconds, or null for invalid timestamps or reversed intervals. |
| `controlKey(control)` | Build the shared control identity used in dependency associations. |
| `operationKey(operation)` | Build the same stored operation identity without implying an operating control. |
| `executionKey(execution)` | Build the shared execution subject format. Native model validation still checks the identity. |

The work plan defaults to 64 queued tasks, 80 dispatched tasks, depth four, and 40 retained gaps.
Programs can lower these limits. Additional gaps increment `droppedGaps`; they do not disappear from the reported count.
Provide stable task keys that include execution attempts and other identity fields relevant to the collector.
Use `next()` once, then `request(task, operation, params)` and return an exchange.
On the next step, call `receive(responses)` before dispatching another task. Store only `plan.state` in continuation state.
The work plan is per-run state, not a durable checkpoint or an independent scheduler.

`emitBatch` limits item count, not CPU cost or encoded bytes. Start with one expensive model per step.
`project` keeps local references to avoid repeated JSON serialization. The exchange boundary validates the complete result.
Do not store helper objects or functions in continuation state.

The hosted context includes permitted operation parameter metadata and native limits.
It does not include connection URLs, credentials, or ungranted operation metadata.
A program can use this metadata to explain or narrow its read plan. Only native validation authorizes requests.

Pass `plan.progress(phase, task)` as the fourth argument to `exchange` to report work progress.
The host bounds and stores these reports with the native step number and timestamp.
The monitor displays the latest reports separately from native read evidence. Program reports do not prove external work occurred.

Read results include native pagination status and optional snapshot source identity.
Receipt-bound builders do not prove completeness. Collection claims still require explicit scopes and native coverage verification.
A failed or truncated read must not establish that a prior requirement was removed.

The [GitHub collector](../testing/integration/hosted/github-actions.js) uses these helpers.
Provider reference parsing, expression semantics, and contract interpretation remain authored JavaScript.
Input-schema generation, a separate GitHub helper bundle, and general collection builders remain future work.

SDK 0.5 adds the `association` entity and canonical control keys for [contextual evidence assessments](INTEGRATION_ASSESSMENTS.md).
Request the `association` output grant when installing a collector that emits these records.

## GitHub contract spike

The [GitHub contract spike](GITHUB_CONTRACT_SPIKE.md) captures a workflow and its referenced actions at resolved commits.
The local `github-contract` binary keeps GitHub reads in Rust and projects parsed declarations through the existing sandbox SDK.
It writes agent JSON and a standalone human review report. This path does not change the ServiceNow broker or publish enforcement facts.

## Backend integration capabilities to grow next

| Capability | Ownership and next work |
| --- | --- |
| HTTP/JSON | Keep named operations and typed parameters. The local host supports operator-defined REST paths and response projections. No arbitrary guest `fetch`. |
| Authentication | Native connection recipes for OAuth refresh, API keys and signing; expose opaque connection references only. |
| Pagination and retries | Native bounded iteration, Retry-After handling and checkpoints. Guest code may interpret permitted cursor values but cannot follow arbitrary URLs. |
| Validation and mapping | Bundle approved pure-JS schema and mapping libraries; retain native output validation and source receipts. |
| Time and encoding | Explicit host-provided observation time for replay; bounded UTF-8/base64 and digest helpers where needed. Avoid ambient clocks/randomness. |
| Diagnostics | Bounded structured events with host redaction; no raw console or credential-bearing HTTP logs. |
| Writes | Separate capability, side-effect declaration, idempotency and audit design before adding POST/PATCH operations. |

## Internal npm artifact cache

Use npm's maintained libraries rather than implementing registry resolution and
content storage: [pacote](https://github.com/npm/pacote) fetches registry artifacts,
[cacache](https://github.com/npm/cacache) stores and verifies content-addressed
bytes, and [ssri](https://github.com/npm/ssri) verifies the approved SHA-512 pin.
The initial implementation is in
[`tools/integration-packages`](../tools/integration-packages/cache.mjs).

The cache is a trusted, separate build-side tool. It never runs inside QuickJS.
Its input is an operator-reviewed flat artifact lock, not a `package.json` or
standard npm dependency-tree lock. Example shape:

```json
{
  "version": 1,
  "packages": [
    { "name": "approved-package", "version": "1.2.3", "integrity": "sha512-REPLACE_WITH_APPROVED_DIGEST" }
  ]
}
```

Each entry needs a full SHA-512 SRI digest and an exact stable version. Ranges,
tags, git/file/URL specifications, unknown lock fields and duplicate entries are
rejected. The tool caps the lock at 64 KiB, its package list at 64 entries, and each
artifact at 8 MiB. It downloads sequentially, never extracts archives or runs
lifecycle scripts, and verifies the supplied pin before committing an artifact.
Do not derive an approval pin automatically from the same unreviewed download.

```sh
npm ci --ignore-scripts --prefix tools/integration-packages
node tools/integration-packages/cli.mjs \
  --lock /private/approved-artifacts.json --cache /private/ipe-npm-cache \
  --registry https://npm.internal.example/
node tools/integration-packages/cli.mjs \
  --lock /private/approved-artifacts.json --cache /private/ipe-npm-cache --offline
```

An optional `IPE_NPM_TOKEN` is scoped to the configured HTTPS registry. It is not
passed to a guest or printed in reports. HTTP is accepted only for numeric
loopback development registries, without authentication. Offline mode reads only
the local verified artifact store and fails on missing/corrupt entries; it does
not fall back to the network. Reports contain package identities, pins and sizes.

The operator controls the registry and build process egress. Registry metadata
may reference tarballs on other hosts; this tool is not an SSRF-isolated download
service. Disk quotas, retention, tenant access control and credential provisioning
belong to the surrounding build infrastructure. No registry has been deployed.

For a shared internal npm endpoint, [Verdaccio](https://verdaccio.org/) is the
compatible private/proxy registry option. Configure the tool to use that endpoint;
keep the local integrity-verified artifact store for reproducible offline builds.

Caching is the first dependency-management stage. Transitive dependency resolution,
approval of the complete dependency graph, safe extraction, module bundling and
native-addon/Node API rejection still need implementation before cached packages
can be used by sandbox adapters. Registry credentials and installation tools must
remain outside the integration workload. Current guests still have no imports.
