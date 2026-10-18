# RFC 014 Fact publication and synchronization

**Status:** Draft discovery
**Created:** 2026-10-18
**Parent and MVP:** [RFC 010](010-remediation-adapters-and-external-policy-discovery.md)

## Scope

Own the versioned change contract between the Integration Workload and Enforcement
Workload. This API carries selected facts, never guest code or external policy
catalogs. [RFC 016](016-integration-signals-and-audit.md) owns delivery diagnostics.

## Publication contract

The Integration Workload publishes sanitized fact projections to a durable feed.
It pushes revision notifications; background synchronization in the Enforcement
Workload pulls authorized changes from a configured endpoint. Periodic reconciliation
recovers missed notifications. Notifications cannot nominate arbitrary URLs and
are never themselves evidence. Optional direct push uses the same envelope and
acceptance rules. The feed carries facts, not the external policy catalog.

Use one ordered stream per authorized scoped collection. An envelope identifies
protocol/schema version, stream, producer epoch, sequence, predecessor, payload
digest, and completeness scope. Mutations carry opaque keys, typed values or
tombstones, observation time, and original validity deadline. Upstream revisions
remain separate from publication sequences. Epoch replacement requires authenticated
coordination and fences retired producers.

Delivery is at least once. Apply identical retries idempotently; reject conflicting
payloads for an accepted revision. Persist the applied position atomically with
local data before acknowledging application. Distinguish durable receipt from
local visibility. Detect gaps and replay; when history is unavailable, resynchronize
from a scoped snapshot with a pinned high-water revision. Validate a complete
inactive generation before activation, then apply later deltas. Partial downloads
never replace current facts. No cross-collection atomicity is implied.

Keep queues, batches, retained history, keys, and bytes bounded, with tenant fairness
and backpressure. Coalesce only where declared snapshot semantics permit it; never
silently discard ordered deltas or terminal outcomes. Advance upstream collection
checkpoints after durable integration publication. A run manifest records independently
committed outputs; a common run ID does not make catalog and fact writes atomic.

Transit and retries cannot renew validity. A new verified observation may have a
new deadline even if its content is unchanged; cached bytes alone cannot establish
that observation. On ingestion, expiry is bounded by the original source deadline,
local TTL ceiling, and observation-age limit. Evaluation may impose tighter freshness.
Define clock tolerance and reject untrustworthy timestamps. Delayed expired data
cannot become usable merely because it was received recently.

## Enforcement isolation

Evaluation reads the last accepted local state without a refresh callback or network
wait. Bound materialized keys, values and write rate. Current FactStore cloning,
writer coordination and synchronous persistence require contention measurements;
separate workloads alone do not prove enforcement latency isolation.


## Area roadmap

| Phase | Deliverable | Proof |
| --- | --- | --- |
| M1 onboarding and discovery | No publication path in the initial discovery slice | Probe cannot mutate enforcement facts |
| M2 adapter iteration | Envelope fixtures and simulated receiver | Original deadlines, duplicate revisions and partial snapshots behave correctly |
| M3 first live integration | One scoped stream, push notification, authenticated pull, reconciliation and durable applied cursor | Change reaches enforcement; missed notification recovers; expired data never renews in transit |
| M4 operational proof | Restart, replay, producer cutover and bounded-load tests | One publisher; no accepted revision lost; enforced latency budget met |
| Later | Multiple streams, partition rollout and retention tuning | No implicit cross-collection atomicity or unfair tenant starvation |

## Review questions

- What exact durable transaction binds applied position and local fact state?
- What are clock tolerance, replay retention and snapshot limits?
- What latency and ingestion budgets define the measured proof?
