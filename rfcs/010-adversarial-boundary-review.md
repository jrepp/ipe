# RFC 010 adversarial review and open gates

**Reviewed:** 2026-10-18
**Design:** [Integration workloads and constraint discovery](010-remediation-adapters-and-external-policy-discovery.md)

This record tracks failure cases and evidence required to close them. RFC 010 maps
design ownership to the area RFCs; this review does not duplicate their contracts. All gates remain open
for implementation and verification. The initial review inspected code and
documentation; the follow-up below adds synthetic execution evidence. Neither
review establishes production security or performance.

## Initial slice evidence

The discovery implementation has [Rust acceptance and deployment
checks](../acceptance/integration-discovery/README.md) against a TypeScript HTTPS
test facade, plus native and CLI/MCP process tests. The
[hosted WASM tests](../crates/ipe-integration/src/wasm.rs) exercise fuel exhaustion,
memory and stack bounds, imports/start rejection, invalid exports and outputs,
and fresh-run isolation. The local deployment demo exercises actual API, CLI and
MCP and supports interactive startup/cleanup. These support the initial
single-scope discovery boundary and metadata sandbox; they do not close the
broader package deployment, synchronization, audit or performance gates below.
Live ServiceNow and enforcement-load validation remain open. Repeatable commands,
not historical test counts, define the current verification surface.

## Implementation review remediation

A synthetic hostile-provider probe reproduced a lost truncation diagnostic:
33 dictionary rows plus an unsupported field type returned 32 fields with only
`metadata_gap`, overwriting `record_limit`. Resource enumeration had the same
failure mode. Discovery now accumulates all distinct coverage codes independently
for resources and fields and retains truncation in its scalar summary. The
[Rust acceptance suite](../crates/ipe-integration/examples/support/mod.rs) combines
limits, invalid/duplicate identifiers, unsupported types, fuel exhaustion and
invalid guest output; it also verifies that issues do not carry into the next run.
This closes that diagnostic defect, not E4's broader snapshot/absence gate.

The security check wrapper previously redirected ThreadSanitizer findings to files,
printed them only when console output matched a sanitizer marker, and deleted them
on exit. It could also accept abnormal exits based on functional test counts.
The [wrapper](../.github/scripts/check-sanitizer.sh) now retains per-run logs,
exit status and file reports, prints file reports independently, and fails on any
abnormal command exit or detected finding. [CI](../.github/workflows/security.yml)
uploads the reports even on failure; orchestration regressions exercise hidden
reports and misleading success counts. This repairs the verification path; an
unexplained sanitizer failure remains a landing blocker until its report has been
investigated. Passing ordinary tests cannot waive it.

## Enforcement and synchronization

| ID | Failure case and grounding | Closure evidence |
| --- | --- | --- |
| E1 | Background integration still delays evaluation. [Server](../crates/ipe-server/src/runtime.rs) polls in-process; [FactStore](../crates/ipe-core/src/runtime_facts.rs) clones state under the writer mutex and persists synchronously | Two-workload deployment; bounded admitted ingestion; p99 and availability within agreed budgets under integration saturation and failure |
| E2 | Catalog growth increases runtime work. Active documents participate in decisions and [revision preparation](../crates/ipe-core/src/policy_document.rs) reaches compilation from evaluation | External descriptions cannot enter decision combination; catalog growth leaves evaluator state unchanged; measure executable revision preparation independently |
| E3 | Delay or replay renews stale evidence; lost notifications strand consumers; old producers overwrite new state | Crash/reorder/replay tests cover original deadlines, durable applied positions, producer fencing, snapshot activation and reconciliation. Current APIs need these extensions |
| E4 | Partial pages or inaccessible records become authoritative absence | Fixtures distinguish full snapshots, deltas, denied access, unknown records and supported negative coverage; partial generations never activate |

## Confidentiality and forensic access

| ID | Failure case and grounding | Closure evidence |
| --- | --- | --- |
| S1 | Legacy provenance leaks topology. `ExternalPolicy.locator` appears in evaluation provenance | Versioned outward contract and migration inventory; inspect legacy documents, nested values, descriptions, logs, errors, metrics and traces for seeded secrets/topology |
| S2 | Masking hides an applicable rule, or reveals protected existence | Identical enforcement for callers with different inspection access; no distinguishable protected entries/counts/errors; missing enforcement evidence remains indeterminate |
| S3 | Inspection elevation becomes permanent execution authority | Separate inspect/resolve/connect/execute grants; denied self-approval; scope/audience enforcement; expiry, revocation and cache invalidation tests |
| S4 | Opaque references cannot reconstruct history, or audit storage becomes a disclosure path | Authorized replay with retained evidence and versioned mappings; denied topology resolution without separate grant; integrity/deletion detection and chain-of-custody checks |
| S5 | Durable auditing silently loses records or stalls indefinitely | Bounded spool and recovery tests; explicit fail-closed behavior when the selected profile requires recording; retention limits stated |

## Extension and domain boundaries

| ID | Failure case | Closure evidence |
| --- | --- | --- |
| X1 | Every new provider needs native changes or reimplements platform mechanics | A second provider uses existing recipes with descriptor/mapping changes only; agent writes no authentication, retry, checkpoint or release-control machinery |
| X2 | A universal schema changes provider meaning | ServiceNow, SCM and SAP/BPM fixtures preserve actor context, ordering, exceptions, units and opaque residuals through shared policy descriptions |
| X3 | Valid JSON is mistaken for true evidence; a sandboxed guest leaks through requests | Independent semantic fixtures; host receipts and identities; restricted guest inputs; resource-level broker grants; denied query/log exfiltration attempts |
| X4 | Retrieved scripts or a validation endpoint acquire execution authority | Imported code remains evidence; operation effects and authorization are explicit; no runtime callbacks or automatic policy promotion |
| X5 | Tests bypass production behavior or a stage publishes live data | [RFC 011](011-agent-integration-toolkit.md) mode-parity reports, platform-owned conformance tests, isolated sinks/checkpoints, and denied production publication from probes/shadows |
| X6 | An upgrade races the current writer or rollback rewinds evidence | Plan revision checks, artifact/configuration pinning, publisher fencing, checkpoint compatibility, and crash/recovery tests with continued stream ordering |
| X7 | Agent disconnect loses work or a monitor leaks protected details | Durable job lookup, resumable bounded signals, equivalent API/MCP authorization, agent handoff, and expired-grant stream tests |

## Review maintenance

Update the RFC when a contract changes and link the affected gate to its acceptance
artifact when implemented. A design mitigation alone does not close a gate. Before
provider implementation, verify the deployed provider version and permissions;
before a performance claim, establish the baseline and numerical budget.

## Area ownership

| Gates | Owning design |
| --- | --- |
| E1, E3 | [014 Synchronization](014-fact-publication-and-synchronization.md) |
| E2, X2 | [013 Domain projections](013-domain-constraints-and-projections.md) |
| E4, X1, X3, X4 | [012 Runtime and discovery](012-integration-runtime-and-discovery.md) |
| S1, S2, S3 | [015 Access and identities](015-policy-access-and-identities.md) |
| S4, S5 | [016 Signals and audit](016-integration-signals-and-audit.md) |
| X5, X6, X7 | [011 Agent lifecycle](011-agent-integration-toolkit.md), with 014 for ordering and 016 for signals |
