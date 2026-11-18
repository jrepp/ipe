---
title: Policy enforcement detail and NFR source review
author: IPE maintainers
created: 2026-11-17
tags: [design, enforcement, security]
id: memo-003
project_id: ipe
doc_uuid: cc5f5254-f8d3-477b-9c10-4adb1f1566e5
---

# Scope

The project owner requested a second self-contained management page with fact storage, immutable policies, and policy checks.
The request adds a JWT carrying a policy endpoint claim and a service that submits an RAR to that endpoint.
The owner also requested the project's curated non-functional requirements (NFRs).

The [editable page](../../design/enterprise-intent-contracts/enforcement-detail.html) and [offline export](../../design/enterprise-intent-contracts/policy-enforcement.html) use sample data.
The JWT adapter and endpoint binding are proposed connections. This memo does not approve or implement that protocol.

# Source review

The repository uses RAR for **Resource–Action–Request**. See the [RAR source](../../crates/ipe-core/src/rar.rs).
This page does not use OAuth Rich Authorization Requests as its wire protocol.
The current [runtime guide](../../docs/RUNTIME_DATA.md) describes caller-attested JSON input and scoped bearer credentials, not JWT authentication.

Searches found current runtime constraints, project requirements, and older targets in [REQUIREMENTS.md](../../REQUIREMENTS.md).
No separate curated NFR document was identified in the repository or accessible Pages search.
The author asked the owner for its location. Until identified, the page attributes its constraints to the sources below.
An empty connected-source search does not prove that no other document exists.

| NFR | Source | Evidence and limit |
| --- | --- | --- |
| Scope isolation | [Runtime credentials](../../docs/RUNTIME_DATA.md#bootstrap-and-credentials), [PRD 001 NFR-1](../prd/prd-001-evidence-system-map.md) | Credentials bind exact tenant/environment scope and independent grants. A token cannot choose another tenant. |
| Stable evaluation reads | [Policy API](../../docs/RUNTIME_DATA.md#document-and-section-representation), [PRD 004 AC-1](../prd/prd-004-policy-enforcement.md) | Capture active immutable policy revisions once and one fact snapshot. The stores do not provide a joint transaction. |
| Fresh evidence | [Expiration and limits](../../docs/RUNTIME_DATA.md#expiration-consistency-and-limits) | Required facts have finite expiry. Recheck the earliest evidence deadline before allow. Missing or expired evidence yields indeterminate. |
| Bounded work | [Runtime source](../../crates/ipe-core/src/runtime_policy.rs), [HTTP source](../../crates/ipe-server/src/runtime.rs) | Default evaluation budget is 10,000 steps and 1,000 collection-item visits. HTTP JSON body limit is 1 MiB. These are limits, not throughput claims. |
| Bounded storage | [Fact-store source](../../crates/ipe-core/src/runtime_facts.rs) | Current state limits include 10,000 keys/collections and 16 MiB encoded state. This is not an unbounded historical fact archive. |
| No provider I/O during evaluation | [Runtime guide](../../docs/RUNTIME_DATA.md#expiration-consistency-and-limits), [PRD 004 FR-1](../prd/prd-004-policy-enforcement.md) | Ingestion refreshes facts separately. An evaluation does not fetch missing provider data. |
| Block on uncertainty | [Runtime aggregation](../../docs/RUNTIME_DATA.md#supported-shapes-and-queries), [PRD 004 AC-2](../prd/prd-004-policy-enforcement.md) | Deny overrides indeterminate, which overrides allow. An empty or inapplicable catalog denies. The proposed service also blocks when no decision arrives. |
| Traceable decisions | [Runtime policy catalog](../../docs/RUNTIME_DATA.md), [PRD 004 FR-3](../prd/prd-004-policy-enforcement.md) | Preserve policy revisions/digests and fact revision context. A returned decision does not prove the external action occurred. |

The old sub-50-microsecond interpreter target and 100,000 evaluations/second/core target do not establish current service performance.
[RFC 009](../../rfcs/009-runtime-facts-and-policy-language.md) requires representative measurements before release budgets are set.
The page does not claim a production latency, availability, replication, or throughput guarantee.

# Proposed JWT and endpoint binding

`policy_endpoint` is an illustrative custom claim, not a registered JWT claim or implemented runtime field.
The example uses an HTTPS evaluation endpoint whose path binds the runtime scope.
It does not identify a caller-selected subset of policies or trigger a download of mutable policy content.

1. The service validates signature, configured algorithm, issuer, audience, subject, token type, and lifetime.
2. It matches the endpoint claim to a configured trusted route and the authenticated tenant/environment scope.
3. It builds the RAR from the validated identity and requested resource, action, and context.
4. It submits the request using separate service authentication and an evaluation grant.
5. The engine checks scope and input, captures active policies and facts, then evaluates all applicable checks.
6. The service applies the matching decision to that request. Deny, indeterminate, mismatch, or unavailable evaluation prevents the action.

Trust and key locations come from configuration. A token-supplied endpoint cannot authorize an arbitrary network request.
The service remains responsible for native permissions and action-specific checks.
Request-to-action binding, replay controls, and decision validity still need a concrete application-gate contract under PRD 004.
This diagram does not establish atomic check-and-act behavior or eliminate changes after evaluation.

JWT guidance comes from [RFC 8725](https://www.rfc-editor.org/rfc/rfc8725.html) and claim semantics from [RFC 7519](https://www.rfc-editor.org/rfc/rfc7519.html).
The proposed adapter also follows the trust boundary in [RFC 009](../../rfcs/009-runtime-facts-and-policy-language.md#authenticated-push-api).

# Verification boundary

Browser tests validate the sample paths, layout, replay, and offline packaging.
They do not test cryptographic validation, real endpoint routing, or live policy enforcement.
The artifact README records the commands and completed checks.
The exported page retains a readable NFR source map and an illustrative RAR input in its expanded notes.
The example is not a new runtime schema. A deployment must configure and validate its input schema and service-to-principal trust boundary.
