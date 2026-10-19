# RFC 0003: Fact Retrieval API Contract

**Status:** Draft — open maintainer decisions in §11; merging this document
records a proposal and does not authorize implementation.
**Created:** 2026-10-18

## 1. Summary and grounding

Propose one bounded, authorized collection-read route for the v2 runtime:

```text
GET /v1/scopes/{tenant}/{environment}/facts/{collection}
```

The current API inspects individual keys. A collection listing would help debug
fact state and reconcile local caches. It would not provide historical reads,
a durable change feed, or a consistent snapshot across multiple HTTP requests.

Implementation references (symbols, rather than shifting line numbers):

- [Runtime HTTP server](../../crates/ipe-server/src/runtime.rs): `inspect_fact`,
  `authorize`, `ApiError`, `Clock::now`, and route registration.
- [Fact storage](../../crates/ipe-core/src/runtime_facts.rs): `KeyState`,
  `FactSnapshot`, `FactSnapshot::get`, `take_revision`, `replace_snapshot`,
  and `cleanup_expired`.
- [Runtime data contract](../RUNTIME_DATA.md): credential grants, TTLs,
  connector generations, and local revision semantics.
- [Runtime acceptance suite](../../acceptance/runtime-data/README.md).

The store retains the latest state per key. Each changed key gets its own revision,
including each entry or tombstone in a pull-generation replacement. A request can
clone an atomic `FactSnapshot`; later requests clone newer state. No old payloads
are retained for reconstruction.

## 2. Goals and boundaries

Goals: bounded reads within one authorized scope and collection, optional field
projection, metadata-only inspection for writers, and strict TTL enforcement.

Excluded: cross-scope queries, joins, policy evaluation, writes or TTL renewal,
historical reads, portable consistency tokens, and reliable incremental feed
synchronization. That last capability requires a separate retained-history or
snapshot-session design; see §5.

## 3. Query parameters

All parameters are optional except the cursor pairing described below. Reject
unknown or repeated parameter names, invalid encodings, and malformed values.

| Parameter | Type/default | Proposed meaning |
| --- | --- | --- |
| `prefix` | UTF-8 string / absent | Match key byte prefixes; at most 1024 encoded bytes. |
| `limit` | integer / 100 | Entries per page, range 1–1000. |
| `cursor` | unsigned revision / absent | Include only keys whose current last-change revision is greater than this value; requires `max_revision`. |
| `max_revision` | unsigned revision / first request's snapshot revision | Inclusive upper bound on current key revisions. Must be at least `cursor` and no greater than the request's snapshot revision. |
| `fields` | CSV / absent | Allowlisted top-level fields of record-schema values. Unknown or duplicate names, empty components, and non-record schemas are errors. A valid optional field absent from a record is omitted. |
| `include_inactive` | `true` or `false` / false | Include expired and deleted keys as metadata only. |

An absent cursor starts at revision zero. Order matching entries by
`(revision, key)` ascending, then apply entry and encoded-response byte limits.
Revisions are unique per changed key in the current store. If that invariant
changes, a revision-only cursor must be replaced before shipping the change.

The client must keep scope, collection, prefix, projection, and inactive selection
unchanged while traversing a window. Revision values are local database metadata;
a cursor is invalid after database replacement/restore and must be discarded.
The current store has no incarnation token to detect every such event; this is
an explicit limitation and implementation decision (§11).

## 4. Wire contracts

Proposed success page:

```json
{
  "records": [
    {
      "key": "deployment-42",
      "status": "live",
      "revision": 7,
      "observed_at": 1792173600,
      "expires_at": 1792173660,
      "source_revision": 12,
      "record": {
        "value": {"active": true, "priority": 7},
        "observed_at": 1792173600,
        "expires_at": 1792173660,
        "source_revision": 12,
        "revision": 7
      }
    }
  ],
  "revision": 50,
  "max_revision": 42,
  "truncated": true,
  "next_cursor": 7
}
```

`revision` identifies the actual snapshot used for this request. `max_revision`
is the enumeration bound, preserved across pages; it can be older than `revision`.
On the first unbounded request both are equal. This separates current observation
from the requested window.

`records[].record` uses `FactRecord` serialization, with projection applied to
`value`. It is present only for a live key and an `inspect` credential. Writers
without `inspect` receive no `record` member even for live keys. Flat metadata
reflects `KeyState`; deleted keys have `expires_at: null`.

```json
{
  "records": [
    {"key":"stale-node","status":"expired","revision":5,"observed_at":1792173500,"expires_at":1792173560,"source_revision":11},
    {"key":"removed-node","status":"deleted","revision":9,"observed_at":1792173590,"expires_at":null,"source_revision":13}
  ],
  "revision": 42,
  "max_revision": 42,
  "truncated": false,
  "next_cursor": null
}
```

`truncated` is true exactly when more matching entries remain in this request's
snapshot after the page. In that case `next_cursor` is the last emitted revision;
otherwise it is null. An empty terminal page is valid, including when later
mutations move all remaining keys outside the window. Never emit a nonterminal
empty page that cannot advance the cursor.

## 5. Consistency and reconciliation

Within a request, clone one snapshot and take one trusted clock reading. Evaluate
liveness with the same expiry rule as `FactSnapshot::get`: equality with
`expires_at` is expired. Never return expired values, including when payload
cleanup has not yet run. Cleanup can erase payloads without advancing revisions.

Across requests, `max_revision` only filters **current** key states. It does not
pin an old version. For example, page one captures bound 7 with a key at revision
5; before page two that key is deleted at revision 9. Page two excludes the key
because its current revision exceeds 7. It cannot return either the old live
value or the out-of-window tombstone. An overwrite behaves the same way.

Consequently the pass does not enumerate exactly the keys present on page one.
Under concurrent changes, keys can disappear from the window before observation.
A bounded pass does not repeat an emitted key, but it can miss a page-one key.
If no writes occur and liveness filters remain unchanged, pagination enumerates
the matching current state. Passive TTL expiry can still remove a matching key
from default listings between pages.

A subsequent window can expose the later current revision, if retained, but this
is not a durable feed. In particular, passive expiry takes no revision, so a
revision-only delta cannot notify a mirror of all expirations. Clients must enforce
cached `expires_at` locally and perform reconciliation. Absence from any partial
or concurrent pass is not authoritative deletion evidence. Inspecting tombstones
requires `include_inactive=true`; no historical event ordering is promised.

Applications requiring a complete stable enumeration or durable replication must
wait for a retained snapshot/session or feed contract, rather than treating this
endpoint as that contract. Snapshot sessions would also need explicit memory,
expiry, authorization, and revocation budgets.

## 6. Conditional requests

A revision-only ETag is insufficient: TTL expiry changes page membership without
a revision increment. Prefix, cursor, projection, inactive selection and the
caller's permitted view also affect the representation. A weak validator does
not make different membership semantically equivalent.

Propose a strong opaque validator computed from the complete deterministic,
authorized response representation after liveness filtering, such as a digest of
its encoded bytes. Reauthorize, validate query parameters, check the clock, build
the current permitted representation and compare before returning `304`.
Changing the response requires a different validator. This includes passive
expiry and payload cleanup when they change visible data. A `304` has no body.
It carries the same ETag and cache policy as the equivalent `200`.

Use `Cache-Control: private, no-cache` so a stored representation must be
revalidated before reuse and is not shared across credentials. Clients still
expire locally cached values by `expires_at` between reads. A `304` saves response
bandwidth, not the scan, authorization or representation-construction cost.

Accept standard `If-None-Match` lists and wildcard syntax with weak comparison for
GET, including the server's own returned validator. Reject malformed syntax as
`400 invalid_parameter`. No conditional behavior is added to single-key
inspection in this proposal. Final cache policy remains a maintainer decision.

## 7. Authorization and errors

Authorize before returning data or `304`, using the existing `inspect_fact` rule:

- `inspect`: values and metadata in the credential's scope.
- Writer of the named collection without `inspect`: metadata only.
- `evaluate` or `manage_policies` alone: no listing authority.
- Pull collections: inspect credentials only; bootstrap already rejects writer
  grants for connector-owned collections.

A credential for another scope fails with `403 scope_forbidden`; missing or
invalid bearer credentials fail with `401 unauthenticated`. No permission
expansion is implied by cursor or cache reuse. `fields` never grants value access.

Use the existing `ApiError` envelope `{"error":"code","message":"..."}`:

| Condition | Status/code |
| --- | --- |
| No collection inspection/write grant | 403 `operation_forbidden` |
| Unknown collection, after authorization | 404 `unknown_collection` |
| Invalid query or malformed conditional header | 400 `invalid_parameter` (new) |
| Untrusted clock | 503 `clock_untrusted` |
| Unavailable/corrupt storage | 503 `storage_unavailable`, where applicable |

No JSON request body is defined. Query strictness parallels the server's typed
body and strict JSON conventions; GET parsing does not invoke JSON decoding.
Write-only capacity and oversized-body errors are not listing response contracts.

## 8. Bounds and implementation surface

Propose 100 entries by default, at most 1000 entries and 4 MiB of encoded response
per page, including metadata and the response envelope. Values are bounded by the
store, but JSON escaping and wrapping also consume the response budget. If one
entry cannot fit, return an explicit `413 response_limit` error rather than an
empty truncated page or silently skipping the entry. Confirm this code before
implementation (§11).

The store currently caps retained keys and total state. Byte limits can require
more than ten pages even with at most 10,000 keys and `limit=1000`. Do not promise
a fixed page count. Bound serialization work and avoid constructing an unbounded
response before applying the cap. Snapshot cloning still holds the store mutex;
conditional GET does not remove that cost.

Add a read-only enumeration method on `FactSnapshot`, reusing collection/schema
lookup and expiry semantics. No persistence format change is needed for the
limited current-state listing. Measure scan, clone, sort and serialization cost
under the maximum supported state before making latency claims. Existing
loopback/proxy deployment rules remain applicable.

## 9. Versioning

Keep the route under `/v1`. Additive response fields are permitted; clients must
handle unknown status values conservatively. Unknown query names remain errors
so typos cannot silently alter selection. Changing ordering, cursor meaning,
authorization, or conditional semantics requires an explicit versioning decision.
The accepted contract must be covered by executable acceptance tests before
implementation is declared complete.

## 10. Proposed acceptance scenarios

Use real HTTP against the runtime binary in the runtime-data acceptance style:

1. Scope isolation and grant enforcement, including writer metadata redaction
   and rejected evaluate-only requests.
2. Authorized unknown collection and strict query errors, including repeated
   names, invalid bounds, overflow, projection and conditional-header syntax.
3. A quiescent multi-page scan is complete, ordered and disjoint.
4. Concurrent overwrite/delete of an unvisited key above the bound excludes it
   from the old window; no old value is reconstructed. New keys above the bound
   are likewise excluded. A later window can return their retained current state.
5. TTL expiry between pages never returns the expired value; inactive listings
   return metadata only. Cached values expire locally without a new revision.
6. Tombstones retain conditional-write revisions; absence from a partial pass
   is never interpreted as a deletion receipt.
7. Projection rejects unknown fields and non-record schemas, omits absent
   optional fields, and never reveals values to writers without inspect.
8. Replay the returned ETag, weak/list/wildcard forms, and check `304` only for
   the current permitted representation. Expiry changing membership changes the
   validator without a commit. Revoked credentials cannot obtain `304`.
9. Entry and byte caps produce advancing cursors; an individually oversized
   encoded entry fails explicitly. Terminal empty windows do not loop.
10. Pull-generation replacement assigns per-key revisions and preserves the
    previous generation after a failed poll; inspect-only access remains valid.

## 11. Open maintainer decisions

These are recommendations, not approvals. Resolve them before implementation:

1. Accept a limited current-state listing, or require retained snapshot sessions
   for stable enumeration? Recommendation: ship the limited semantics only if
   they meet the caller's reconciliation need; do not promise feed synchronization.
2. Single `include_inactive` flag for expired and deleted entries? Recommend yes.
3. Default/hard entry limits 100/1000? Recommend yes, subject to measured costs.
4. Writer-visible timestamps and source revisions exceed today's minimal
   single-key metadata. Recommend an explicit authorization review before expansion.
5. Representation-based ETag and private revalidation, or omit conditional GET
   initially? Recommend correct representation validation if caching is included.
6. Single-key conditional GET parity? Recommend defer to a separate change.
7. Omit policy-style freshness filters? Recommend yes; enforce hard TTLs only.
8. Introduce `400 invalid_parameter` and `413 response_limit`? Recommend explicit
   codes with acceptance coverage rather than overloading fact-validation errors.
9. Accept 4 MiB encoded pages and local revision cursors without incarnation
   detection, or add a database identity? Recommend requiring clients to restart
   on database replacement and evaluating an identity token before production use.
10. Confirm inspect-only visibility for connector-owned collections? Recommend
    yes, following the existing inspection rule.
