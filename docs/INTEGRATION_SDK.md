# Integration sandbox SDK and package cache

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

SDK 0.3 supports [hosted integration programs](INTEGRATION_HOST.md). The endpoint installs JavaScript source and runs it manually or on a schedule.
Programs emit typed definitions, evidence, constraints, invocations, expiring facts, and external outcomes into bounded local storage.
See [the typed observation example](../testing/integration/hosted/typed-observation.ts). The host derives logical keys and revision digests. Enforcement publication remains separate.

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
