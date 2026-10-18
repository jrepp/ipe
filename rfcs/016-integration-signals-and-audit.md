# RFC 016 Integration signals and forensic audit

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

Own operational signals, delivery visibility, protected decision evidence and audit
retention. Sampled telemetry is not durable audit. [RFC 015](015-policy-access-and-identities.md)
owns masking and access policy; [RFC 011](011-agent-integration-toolkit.md) exposes
permitted query and monitoring operations.

## Operational signals

The native runtime emits correlated signals for broker calls, collection, sandbox
execution, normalization, masking, durable publication, and consumer application.
Use opaque run, candidate, installation and stream IDs. Trace events distinguish
accepted job, started run, collected source, validated output, published revision,
and observed consumer application. Do not conflate those stages into "success".

Expose three independent views:

- **Execution health:** duration, retries, throttling, budget usage, errors and stalls.
- **Evidence health:** age, coverage, schema drift, unsupported constraints, unknown
  records, expiry headroom and differences from the prior observation.
- **Delivery health:** queued/published/applied revisions, lag, rejected batches,
  resynchronization state, and last consumer acknowledgment.

Absent consumer feedback means application is unknown, not successful. A healthy
worker can still have stale or incomplete evidence. Host counters are authoritative
for platform actions; adapter diagnostics are labeled assertions. Evidence-health
signals must not become enforcement facts without their own authorized projection.

Provide bounded queries and resumable watches with filters for stage, run, time and
severity. Explain common failures with stable codes and permitted recovery actions.
Watch gaps are explicit and recoverable by query. Quotas, sampling and cardinality
limits protect the runtime. Durable audit is distinct from sampled telemetry.
Optional registered diagnostic events have schemas and approved fields; arbitrary
stdout, payload dumps, endpoint labels and baggage do not bypass masking. Restricted
details require Need-to-Know Policy Access and do not appear in default agent context.

## Forensic audit

Record decision ID, executable policy/schema revisions, local snapshot revision,
evaluation time, opaque evidence references and validity, result, and permitted
reason codes. Protected provenance links evidence to source observations and adapter
versions. Preserve authorized input and evidence snapshots in encrypted restricted
storage when exact reconstruction is required; never archive connection secrets.
Topology resolution is a separate permission from forensic inspection.

Use append-only retention, authenticated integrity manifests, deletion detection,
and access auditing. Record chain of custody for protected exports. Audit elevation
requests, approvals, grants, use, expiry, and revocation. Ordinary audit views remain
masked. Revisions and digests alone cannot provide replay after evidence or necessary
execution artifacts have been deleted; state retention and replay limits explicitly.

The Enforcement Workload uses a bounded durable audit spool with asynchronous export,
independent of integration collection jobs. A profile requiring durable audit fails
closed when it cannot record. Lossless audit and unlimited availability during an
unbounded audit-storage outage cannot both be promised.

## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | Bounded structured discovery report and deployment demo report | Connection, source access and unknown coverage are distinguishable; no sensitive error text |
| M2 adapter iteration | Run IDs, reports and host counters | Agent diagnoses a fixture failure without raw payload dumps |
| M3 first live integration | Execution/evidence/delivery health and consumer acknowledgment | Published and applied are separate states; absent acknowledgment remains unknown |
| M4 operational proof | Minimal protected decision record/evidence archive and bounded audit spool | Authorized reconstruction without topology disclosure; replay limitations and outage behavior tested |
| Later | OTLP export, durable watches, enterprise retention and richer investigation tooling | Sampling never weakens the audit guarantee |

## Review questions

- What minimum evidence must be retained to reconstruct the chosen policy decision?
- Which audit profile and retention duration apply to the first live environment?
- How can the first agent query delivery state without exposing topology?
