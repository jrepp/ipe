# RFC 0003: Fact Retrieval API Contract

## Status
- **Status**: Draft — awaiting maintainer decisions on the open questions (§10)
- **Author**: Generated design pass (glm-5.3-flash), reviewed by session + maintainer
- **Created**: 2026-10-18

## Summary

A read-side contract for the v2 runtime's fact collections: one listing route with
revision-cursor pagination, projection, conditional GET via snapshot-revision ETags,
strict error/authorization reuse, and explicit TTL/consistency semantics. Grounded in
the existing implementation (file:line citations throughout); no as-of time travel.

Status: proposal for review; no code is changed by this document. Grounded in
the shipping v2 surface only (`runtime.rs`, `runtime_facts.rs`,
`strict_json.rs`); legacy engine/store/parser modules are out of scope.

## 1. Goals and non-goals

Goals

- G1 Bounded, authorized reads of fact state within one scope+collection, for
  debugging, synchronization, and mirroring (the gap: today only single-key
  `inspect_fact`, `runtime.rs:300-324`).
- G2 Reuse existing patterns: scoped bearer auth (`runtime.rs:179-214`),
  `ApiError` codes (`runtime.rs:146-172`), strict JSON (`strict_json.rs:47-66`),
  `/v1/scopes/:tenant/:environment/...` route shape (`runtime.rs:822-832`),
  response envelopes `{"records":...}` (`runtime.rs:298`) and
  `{"status":...,"revision":...}` (`runtime.rs:315-318`).
- G3 Cheap re-poll for sync clients (conditional GET).
- G4 Preserve the mandatory-TTL posture: no reader may obtain a fact value the
  resolver would refuse (`FactSnapshot::get`, `runtime_facts.rs:753-783`).

Non-goals

- N1 Not a general query engine: no cross-collection joins, no predicate
  pushdown beyond key prefix, no sorting beyond the defined iteration order.
  Policies query facts; this API inspects state.
- N2 No cross-scope reads, ever. Scope comes from the path and the credential
  match (`runtime.rs:179-214`); snapshots are scope-keyed
  (`runtime_facts.rs:726-736`).
- N3 No change-feed / event log. The store keeps only current state per key
  (`KeyState`, `runtime_facts.rs:216-225`); historical payloads are not
  archived (docs/RUNTIME_DATA.md, policy-catalog section). A feed would require
  new durable storage. The revision-window iterator below delivers the sync
  use case without one.
- N4 No historical as-of-revision reads. `FactSnapshot` clones only the current
  `PersistedState` (`runtime_facts.rs:640-646`); state at an older revision
  cannot be reconstructed. `max_revision` below is a paging bound, not time
  travel (see §5).
- N5 No writes, no TTL refresh, no `min_revision` consistency token (docs
  already state it is not implemented; revision stays "local database
  metadata, not a portable consistency token", docs/RUNTIME_DATA.md).

## 2. Endpoint design

One new route, consistent with existing style:

    GET /v1/scopes/{tenant}/{environment}/facts/{collection}

Query parameters (all optional; unknown names rejected — §6):

| Param | Type | Default | Meaning |
| --- | --- | --- | --- |
| `prefix` | string | — | Only keys with this byte prefix (keys ≤1024 bytes, `runtime_facts.rs:19,847-851`). |
| `limit` | integer | 100 | Max entries per page; hard max 1000 (§8). |
| `cursor` | integer | — | Revision cursor: return only keys whose latest change revision > `cursor`. Must be paired with `max_revision`. |
| `max_revision` | integer | snapshot revision at page 1 | Upper bound on key change revision; pins the page set across pages. Required whenever `cursor` is present. |
| `fields` | CSV | — | Projection: allowlist of top-level fields of `record.value`; valid only for `record`-schema collections; unknown name → error. |
| `include_inactive` | bool | false | Also return `expired` and `deleted` entries. Values are never returned for non-live entries (§5). |

Pagination: **revision-cursor, not offset**. Rationale:

- Every put, delete, and pull-generation change takes one strictly monotone
  revision under the state mutex (`take_revision`, `runtime_facts.rs:946-950`;
  delete branch `runtime_facts.rs:451-465`), and `FactSnapshot` is an atomic
  clone (`runtime_facts.rs:640-646`). Therefore the set {keys with change
  revision in `(cursor, max_revision]}` is disjoint across pages and stable
  under concurrent commits: later commits land above the bound.
- Offset pagination over the key-ordered `BTreeMap<FactAddress, KeyState>`
  (`runtime_facts.rs:216-225`) is O(skip) per page and unstable: concurrent
  commits shift offsets, duplicating or skipping keys between pages.
- State is bounded (≤10,000 keys, `runtime_facts.rs:22`), so each page rescans
  the collection's key range in memory; a secondary index is unnecessary.

Page semantics: entries ordered by `(revision, key)` ascending; a page returns
up to `limit` entries from the window; `next_cursor` is the last entry's
revision and is non-null iff more matching entries remain (deterministic — the
scan is bounded); a key changed twice inside the window appears once, with its
latest state. This is a key-enumeration iterator by last-change revision, not
an event log (N3).

Conditional requests: every 200 response carries `ETag: W/"<revision>"` where
`revision` is the snapshot revision (`FactSnapshot::revision` =
`next_revision`, `runtime_facts.rs:693-696`). `If-None-Match: "<revision>"`
matching the current snapshot revision returns `304` with the same `ETag` and
no body. The validator is weak because the body is also a function of `now`
(TTL expiry is time-based and does not advance the revision — §5); clients
must keep honoring per-record `expires_at` locally across `304`s.

As-of-revision reads: rejected (N4). `max_revision` bounds which key changes
are considered; liveness/expiry is always evaluated at read time against
current state. Example: key put at r=5, deleted at r=9; a request with
`max_revision=7` will not surface the r=9 tombstone, so the key appears live
in the window even though it is deleted now. Documented, intended: the
window is for stable iteration, not history.

`include_expired` vs `include_inactive`: one flag covering both `expired` and
`deleted`, named `include_inactive`. Justification against the mandatory-TTL
posture: non-live entries are returned metadata-only (no `record.value`),
matching the resolver, which refuses the value after expiry
(`runtime_facts.rs:763-765`) and `cleanup_expired`, which nulls payloads
within ≤30 s while keeping revision/watermarks (`runtime_facts.rs:648-666`;
spawn loop `runtime.rs:798-806`). Metadata (revision, observed_at,
expires_at, source_revision) lets sync clients distinguish expiry from
deletion without resurrecting evidence.

## 3. Wire contracts

Success page (200):

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
  "revision": 42,
  "truncated": false,
  "next_cursor": null
}
```

- `records[].record` is the exact `FactRecord` JSON
  (`runtime_facts.rs:171-177`, `Serialize`), present iff `status == "live"`.
- Flat `revision`/`observed_at`/`expires_at`/`source_revision` mirror the
  stored `KeyState` (`runtime_facts.rs:216-225`) and are present on every
  entry; `expires_at` is `null` for `deleted` (delete stores
  `expires_at: None`, `runtime_facts.rs:459`).
- `revision` (top level) is the snapshot revision = paging bound used;
  clients echo it back as `max_revision`.
- `truncated` is `true` iff the page stopped early (limit or byte budget,
  §8); `next_cursor` is non-null iff more entries remain in the window.

Empty page (200):

```json
{"records": [], "revision": 42, "truncated": false, "next_cursor": null}
```

Truncated page (200):

```json
{
  "records": [ /* up to `limit` entries, shape above */ ],
  "revision": 42,
  "truncated": true,
  "next_cursor": 87
}
```

Inactive entries (only with `include_inactive=true`; no `record` member):

```json
{"key": "stale-node", "status": "expired", "revision": 5,
 "observed_at": 1792173500, "expires_at": 1792173560, "source_revision": 11}
{"key": "removed-node", "status": "deleted", "revision": 9,
 "observed_at": 1792173590, "expires_at": null, "source_revision": 13}
```

Projection (`fields=active`): `record.value` contains only allowlisted
fields; the rest of the entry is unchanged. Unknown field name, non-record
schema, or a schema-optional field that is absent is simply omitted (only
unknown *requested names* error, §6).

Conditional GET: request `GET .../facts/approvals` with
`If-None-Match: "42"`; if the current snapshot revision is still `42` → `304`
(empty body, same `ETag`). Any commit advancing the store revision → `200`
with a new `ETag`. Note: passive expiry does NOT change the revision (§5), so
a `304` can hide entries that expired since the cached page; clients filter
cached entries by `expires_at` themselves.

Not modified / error bodies use the existing envelope
`{"error": code, "message": ...}` (`runtime.rs:146-150`).

## 4. Semantic guarantees

- Snapshot isolation per request: each page handler captures one
  `store.snapshot(scope)`; the clone under the mutex is atomic, so one
  response is internally consistent (`runtime_facts.rs:640-646`). Within one
  page, evaluation shares one `Clock::now()` reading (pattern:
  `runtime.rs:326-360`).
- Cross-page stability: the client pins `max_revision` to page 1's top-level
  `revision`; entries only come from `(cursor, max_revision]`. Commits between
  pages take revisions above the bound (`take_revision` is strictly monotone,
  `runtime_facts.rs:946-950`) and are invisible to the pinned pass; the next
  pass starting at `cursor = max_revision` (or last `next_cursor`) delivers
  them. No duplicates, no misses inside a pass.
- Replay of commits between pages: a key mutated twice in the window appears
  once with its latest state; a key mutated after the bound appears in the
  next pass. This is exactly the mirror-advance rule.
- TTL/expiry visibility: liveness is evaluated at read time with the caller's
  `now` — `now >= expires_at` → expired, equality is expiry
  (`runtime_facts.rs:763-765`; exclusive boundary by construction
  `runtime_facts.rs:916-943`). A key emitted as `live` on page 1 is never
  re-emitted later in the same pass (its revision is ≤ the advanced cursor);
  keys not yet emitted may surface as `expired` (with `include_inactive=true`)
  or be absent (default). Clients caching pages must apply `expires_at`
  locally; revision does NOT advance on passive expiry (`cleanup_expired`
  nulls values without taking revisions, `runtime_facts.rs:648-666`).
- Deletion visibility: deletes store value-less tombstones that keep their
  revision and `source_revision` for replay ordering
  (`runtime_facts.rs:207-215,451-465`). Default listings show deletions only
  as absence; `include_inactive=true` surfaces `status:"deleted"` entries.
  Absence plus a revision gap is not proof of prior existence — mirrors that
  need deletion notices should enable `include_inactive`.
- Clock/high-water: reads use `Clock::now()`, a monotone high-water over wall
  time; a >5 s wall discontinuity yields 503 `clock_untrusted` and the read
  fails closed (`runtime.rs:106-143`). Because `now` is monotone, expiry
  decisions cannot flip backwards between pages of one pass.

## 5. Error contract

All errors use `ApiError` → `{"error":code,"message":...}`
(`runtime.rs:146-150`). Reuse `From<FactError>` mappings unchanged
(`runtime.rs:152-172`).

| Condition | Status | Code | Source pattern |
| --- | --- | --- | --- |
| Missing/invalid Bearer token | 401 | `unauthenticated` | `runtime.rs:202-213` |
| Valid token, different scope | 403 | `scope_forbidden` | `runtime.rs:202-213` |
| No `inspect`, not writer of this collection | 403 | `operation_forbidden` | `runtime.rs:216-218`, check modeled on `runtime.rs:307` |
| Collection not configured in scope | 404 | `unknown_collection` | `runtime_facts.rs:711-721` via `runtime.rs:158` |
| Malformed query param, unknown param name, `cursor` without `max_revision`, `limit` out of range, `prefix` >1024 bytes, bad `fields` name, non-boolean `include_inactive`, malformed `If-None-Match` | 400 | `invalid_parameter` (NEW) | 400 is the truthful status for bad request metadata; body-level input keeps 422 `invalid_json`/`invalid_fact` (`runtime.rs:175-177,165-171`) |
| Clock discontinuity during read | 503 | `clock_untrusted` | `runtime.rs:120-143` |
| Storage/corrupt state | 503 | `storage_unavailable` | `runtime.rs:160-164` (reads touch the in-memory Arc only; kept for parity) |
| Bounded-state capacity exhausted | 507 | `state_limit` | `runtime.rs:165-166` (not reachable on reads; listed for completeness) |
| Oversized body | 413 | (axum default) | `DefaultBodyLimit`, `runtime.rs:833` — GET carries no body; do not accept bodies on GET |

Strict-JSON posture: request bodies are absent for GET; unknown **query
parameters** are rejected with 400 `invalid_parameter`, the query-string
analog of `deny_unknown_fields` (`runtime.rs:49,53,222-241`) and of the
duplicate-key rejection in `strict_json.rs:47-66`.

## 6. Authorization

Exactly the `inspect_fact` rule, applied per collection
(`runtime.rs:307`):

- `inspect` grant → full entries including `record.value` (mirrors
  `runtime.rs:315` and `catalog`, `runtime.rs:401-407`).
- Writer of this collection without `inspect` → listing with all metadata
  (`status`, `revision`, `observed_at`, `expires_at`, `source_revision`) but
  no `record` member. This extends "a writer without `inspect` can read a
  key's revision/status for conditional updates, but cannot read stored
  values" (docs/RUNTIME_DATA.md, credentials section) from one key to many;
  the writer only sees the collection it already writes.
- `evaluate`-only or `manage_policies`-only credentials → 403
  `operation_forbidden` (no new grant is introduced).
- Pull-mode (connector-owned) collections: no push-writer credential can exist
  for them (`runtime.rs:704,725`), so they are readable only by `inspect`
  credentials — same posture as `inspect_fact` (no ingestion-mode check,
  `runtime.rs:300-324`) and `connector_status` (`runtime.rs:601-616`).
  Confirm explicitly (OQ-10).
- Scope isolation is inherited from `authorize` (`runtime.rs:179-214`) plus
  the scope-keyed snapshot (`runtime_facts.rs:726-736`); there is no route
  shape that can name two scopes.

## 7. Limits and abuse

- `limit` default 100, hard max 1000; the full key space is ≤10,000 keys and
  16 MiB (`runtime_facts.rs:22-23`), so a complete mirror is at most
  10 pages of 1000.
- Response byte budget: stop appending entries (and set `truncated=true` with
  `next_cursor`) at 4 MiB — 4×`MAX_BODY`, the same multiplier already accepted
  for bootstrap configuration (`runtime.rs:32,792`). Individual values are
  ≤1 MiB (`runtime_facts.rs:20`), so the budget bounds worst-case pages.
- No in-process rate limiting exists for any route; the listener is loopback
  only, and remote access is delegated to a local TLS-terminating proxy
  (`runtime.rs:624-628`; docs/RUNTIME_DATA.md deployment note). Recommend:
  rate limiting and request-size quotas stay a proxy concern; conditional GET
  (`304`) is the intended cheap re-poll so sync loops do not refetch bodies.
- Write amplification: none — no persistence or clock writes; the only
  shared-mutex work is one state clone per request, the same cost class as
  `evaluate`'s snapshot (`runtime.rs:340`).

## 8. Versioning

- Routes stay under `/v1` (`runtime.rs:822-832`). Evolution rule:
  **additive-only** within `/v1` — new optional query params, new response
  fields, new `status` values must be tolerated by clients; this is stated as
  a client obligation in the docs.
- Unknown query parameters are rejected (400), mirroring
  `deny_unknown_fields`; silently ignoring them would let clients depend on
  typos and make later reuse a breaking change.
- Breaking changes (removing/renaming params or fields, changing entry
  ordering or `304` semantics) require a new route version, not header
  negotiation. `ETag` values are opaque quoted strings; clients must not
  parse them.
- The contract is also pinned by the external acceptance suite
  (acceptance/runtime-data/README.md): new scenarios in §10 join that suite
  so wire changes fail CI before shipping.

## 9. Implementation surface (core gap, design note)

`FactSnapshot` currently exposes only `revision`, `snapshot_metadata`,
`collection_config`, `schema`, `collections`, `key_revision`, `get`
(`runtime_facts.rs:693-783`). Listing needs one new read-only core method,
e.g. `entries(collection, now, window: (u64,u64), include_inactive) ->
Vec<Entry>`: a `BTreeMap` range scan over the collection's addresses with
status derived from the same expiry logic as `get` (`runtime_facts.rs:753-783`).
No persistence, schema, or `FactError` changes; the store's header comment
already anticipates an indexed layout behind unchanged semantics
(`runtime_facts.rs:3-7`).

## 10. Acceptance-test scenarios

Style: black-box HTTP against the real binary using `check(name, operation)`
with `require(...)` (scripts/accept-runtime-data.py:129-141), joined to the
runtime-data suite (acceptance/runtime-data/README.md).

1. Multi-tenant isolation — Given the same collection name in scopes A and B,
   when A's credential lists A, then only A's keys appear; the same token on
   scope B is 403 `scope_forbidden`.
2. Grant enforcement — Given a writer credential without `inspect`, when it
   lists, then entries carry metadata but no `record`; an `evaluate`-only
   credential gets 403 `operation_forbidden`.
3. Unknown collection — Given an unconfigured collection name, when any
   authorized credential lists it, then 404 `unknown_collection`.
4. Strict parameters — Given an unknown query parameter, `limit=1001`, or
   `cursor` without `max_revision`, then each is 400 `invalid_parameter`.
5. Cursor stability under concurrent commits — Given 250 seeded keys and
   `limit=100` with `max_revision` pinned from page 1, when a batch commit
   lands between pages, then pages are disjoint, the union is exactly the
   pinned window, new keys are absent until a follow-up pass with
   `cursor=max_revision` returns exactly those keys.
6. TTL expiry mid-pagination — Given a short-TTL key expiring between pages,
   when page 2 is fetched with `include_inactive=true`, then the key appears
   once as `status:"expired"` with no value; without the flag it is absent;
   no response ever contains its value after expiry.
7. Deletion visibility — Given a committed key that is then deleted, when
   listing with `include_inactive=true`, then the entry is
   `status:"deleted"` with retained revision, and a subsequent put naming
   that tombstone `expected_revision` succeeds.
8. Projection — Given a record-schema collection, when listing with
   `fields=active`, then every `record.value` contains only `active`; an
   unknown field name or non-record schema returns 400.
9. Conditional GET — Given `ETag: W/"r"`, when the request repeats with
   `If-None-Match: "r"` and no commit occurred, then 304; after any commit,
   200 with a new `ETag`; when a listed fact expires with no commit, still
   304 and the client drops the cached entry by its `expires_at`.
10. Pull collection inspection — Given a connector-owned collection after a
    failed poll, when an `inspect` credential lists it, then the previous
    accepted generation's entries and revisions are returned unchanged, and
    no writer credential can be granted on that collection (bootstrap
    rejects it).

## 11. Open questions for the maintainer

1. `include_inactive` as a single flag for expired+tombstone entries —
   Recommendation: yes; two flags doubles surface for one audit need.
2. Default `limit` 100 / hard max 1000 — Recommendation: accept; confirm
   against expected production key counts (store caps at 10,000).
3. Writer-without-`inspect` listings include metadata beyond status+revision
   (observed_at/expires_at/source_revision), slightly richer than
   `inspect_fact`'s status+revision (`runtime.rs:317-318`) — Recommendation:
   allow (writers only see collections they already write); confirm.
4. `304` semantics rely on client-side `expires_at` filtering rather than a
   time-quantized validator — Recommendation: accept documented obligation;
   a quantized ETag would still not be exact and complicates caching.
5. Add `If-None-Match` to single-key `inspect_fact` for parity —
   Recommendation: yes, additive, but can ship after the list endpoint.
6. No `max_age`/freshness filter on listing (policies express `fresh within`;
   retrieval only inspects state) — Recommendation: keep out of v1.
7. New 400 `invalid_parameter` code vs overloading 422 `invalid_fact` for
   query errors — Recommendation: introduce the new code; the error table
   stays truthful.
8. Byte budget 4 MiB per page (4×`MAX_BODY`, `runtime.rs:792`) —
   Recommendation: accept; revisit if value sizes grow.
9. Document `revision` as cursor-grade but still "local database metadata,
   not a portable consistency token" (docs/RUNTIME_DATA.md) —
   Recommendation: keep both statements; the cursor works within one store
   lifetime, which is all this API serves.
10. Pull collections visible to `inspect`-only credentials — Recommendation:
    confirm explicitly in docs (current behavior already implies it,
    `runtime.rs:300-324,601-616`).

## Executive summary

1. One new route: `GET /v1/scopes/{t}/{e}/facts/{collection}` for bounded, authorized fact reads.
2. Revision-cursor pagination pinned by `max_revision`; stable and disjoint under concurrent commits.
3. Pages are key-enumeration windows by last-change revision — not a change-feed, not history.
4. No as-of-revision reads: the store keeps only current state; documented, not hidden.
5. Entries mirror existing shapes: `FactRecord` JSON plus flat `KeyState` metadata and `status`.
6. `status` is `live|expired|deleted`; values are never returned for non-live entries.
7. Authorization reuses `inspect_fact`'s rule; writers see metadata, `inspect` sees values.
8. Errors reuse `ApiError` codes; one new code, 400 `invalid_parameter`, for query-string faults.
9. Conditional GET via weak `ETag` on snapshot revision; clients honor `expires_at` locally.
10. Limits: 1000 keys/page hard max, 4 MiB page budget, loopback+TLS-proxy deployment unchanged.

REPORT: /tmp/ipe-pr/fact-retrieval-api-contract.md
ENDPOINTS: GET /v1/scopes/{tenant}/{environment}/facts/{collection}
OPENQUESTIONS: 10
