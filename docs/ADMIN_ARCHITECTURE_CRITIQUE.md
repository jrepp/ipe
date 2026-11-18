# Admin Architecture Design Critique

**Review outcome:** The earlier architecture explained reliable transport more clearly than publication authority. The revised [architecture review](ADMIN_ARCHITECTURE_REVIEW.md) addresses the findings below. This review changes documentation only; the described controls still need implementation.

## High severity findings

### The central policy store and external contracts were missing

The main diagram reduced integration to fact production and omitted integration's publication of externally enforced contract constraints and evaluations. That was a product-level omission, not a diagram styling issue.

**Resolution:** The central policy service now explicitly contains executable policy records, descriptive external contracts, and contextual external evaluation records. Integration owns source-scoped publication of the latter two. External systems remain authoritative for their own behavior; the central store retains our representation and history. Volatile fact publication remains a distinct path.

### Origin was being confused with execution authority

“Local versus external policy” is insufficient. An externally sourced rule translated into IPE is executable locally; a discovered external contract is knowledge about another system. Today, `PolicyDocument.external` already belongs to the executable family, and the policy publication API activates revisions immediately.

**Resolution:** New descriptive resource types and permissions must not reuse `manage_policies` or the executable publication endpoint. Existing executable imports retain their behavior and source-validity deadlines. Translation into a local guard requires explicit executable authority, tests, and declared source-validity dependence. [Implemented publication contract](RUNTIME_DATA.md#policy-management-and-candidate-testing-api)

### A cache refresh hint had become a messaging subsystem

The earlier design made durable outboxes, replay cursors, producer epochs, and pinned snapshot/delta handoff appear mandatory for configuration refresh. That complexity had no demonstrated requirement when authoritative latest-state polling could repair a missed hint.

**Resolution:** The configuration default is durable assignments, periodic authorized pulls, and best-effort hints after commit. Retained delivery remains a separate option for ordered fact deltas under [RFC 014](../rfcs/014-fact-publication-and-synchronization.md), or other explicitly loss-sensitive events. Complete snapshots are a bounded initial fact-delivery option.

### Central assignment and endpoint writes could compete

The existing endpoint API permits immediate policy activation. Adding a central assignment system without restricting that path would create two writers claiming authority over the same managed scope.

**Resolution:** The proposed managed profile uses central assignments as activation authority and rejects ordinary direct writes for the scope. Standalone mode remains separate. A future override would need an explicit state and audit contract. Central identities, assignment generations, and endpoint-local revisions are mapped, never treated as interchangeable.

## Medium severity findings

### AT Protocol had no unique required role

Stable IDs, typed schemas, digests, derived views, and fetch-plus-notify are general design techniques. Describing them as AT adaptations did not supply actual interoperability or a distinctive trust guarantee. AT's differentiated identity and repository federation features were not established requirements.

**Resolution:** AT remains comparative research with no selected component or wire protocol. The main architecture no longer follows an AT box analogy. Reconsider adoption only for a concrete cross-organization publication and disclosure requirement. [AT protocol structure](https://atproto.com/specs/atp#protocol-structure)

### xDS and TUF appeared more selected than they were

Borrowing acknowledgement terminology is not xDS adoption. A content digest and signature are not the full update-security guarantees of TUF. The document did state that these were references, but their prominent placement still left the stack ambiguous.

**Resolution:** xDS is an alternative delivery protocol, and TUF an artifact-security option contingent on a threat model and implementation choice. Neither is presumed. The initial configuration path is ordinary authenticated HTTP with optional SSE hints. [xDS acknowledgement semantics](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol#ack-and-nack-semantics-summary), [TUF specification](https://theupdateframework.github.io/specification/latest/)

### Identity binding was underspecified

An opaque ID is not authentication, and a self-reported new process incarnation cannot supersede an existing endpoint binding. Provider resource identities and workload identities also serve different purposes.

**Resolution:** Operator-approved enrollment binds workload identity, scope, role, and the current incarnation. Reports are accepted only from that binding. Addresses and labels remain mutable metadata. No new decentralized identity mechanism is justified by these requirements.

### Observability enforcement needed an explicit scope

The review initially challenged telemetry compliance as product expansion. That criticism was qualified because admin-layer OTel scraping, traces, and observability enforcement are explicit user requirements.

**Resolution:** Retain those requirements, while selecting admission and rollout consequences through an explicit telemetry profile. Keep collector topology deployment-specific, distinguish telemetry compliance from runtime health, and avoid a synchronous telemetry dependency in ordinary evaluation. Durable audit follows a separately selected guarantee.

## Architectural acceptance proof

The decisive first proof is an integration publishing an external contract into the central policy store without changing local enforcement, alongside an explicitly assigned executable policy reaching an endpoint and becoming active. Then show that a contextual external evaluation can be inspected without becoming an unrestricted permission or a new executable rule.

Exercise missed hints, expired observations, partial discovery, conflicting publisher attempts, direct-write rejection in managed mode, and stale process reports. A dashboard or notification stream alone does not demonstrate these boundaries.

## Remaining decisions

Disconnected operation and revocation deadlines, external-result reuse, audit durability and retention, deployment availability, and artifact trust under compromised distribution infrastructure require explicit requirements. They should not be hidden inside a protocol choice.

A final review of the revised architecture found no remaining high-severity contradiction with these corrections. That is a design assessment, not evidence that the proposed controls have been implemented or tested in runtime code.
