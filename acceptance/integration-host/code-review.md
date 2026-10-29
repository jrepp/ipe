# Integration host review

Reviewed on 2026-10-29. Scope: PR #55 at `75911f0`, plus the dependency and sanitizer changes in this review. This is an implementation self-review. It is not an independent maintainer approval.

## Boundary review

| Boundary | Source checked | Result and test evidence |
| --- | --- | --- |
| Credential isolation | `host/credentials.rs`, `host/broker.rs`, `host/mod.rs` | The native host resolves credentials. Git receives the context on stdin. Helper time and output are bounded. Guest code receives projected responses. Redirects are disabled. Raw and JSON-escaped credential echoes are rejected. Tests cover helper failure, ambiguous output, origin escape, rejected redirects, echoed credentials, and API authentication. |
| Scoped deletion | `host/coverage.rs`, capture completion in `host/mod.rs` | Removal needs a successful, complete native read with an authorized snapshot source. Members must match its source revision and receipt. Reconciliation limits removal to the document, kind, and scope. Missing coverage causes upserts only. Tests cover empty complete collections, partial captures, unavailable reads, forged receipts, and preservation after failure. |
| Revision restoration | `host/revisions.rs`, revision API in `host/mod.rs` | Restore selects a retained, previously accepted review catalog. It does not replace collector code, renew facts, or activate policy. Accepted revisions remain pinned. Capacity errors preserve prior state. Tests cover acceptance, restoration, restart, fact expiry, source changes, and changed call bindings. |
| Outcome identity | `host/model.rs`, `host/outcomes.rs`, `host/explanations.rs` | Host-derived keys distinguish workflow attempts, jobs, and steps. Repeated polls update an existing execution. Changed payloads have bounded history. Collector eviction does not remove the outcome archive. Exact source revisions constrain candidate links. Tests cover repeated polls, reruns, legacy records, archive capacity, restart, and conflicting or expired facts. |

No blocking defect was found in these four boundaries. This finding applies to the local, single-operator review host. It does not establish that agent-authored mappings are correct or that a captured contract permits an action. Native receipts prove a read; they do not prove its interpretation.

## Changes from this review

- Update only `libz-sys` from 1.1.22 to 1.1.31 in the lockfile. This resolves RUSTSEC-2026-0418.
- Build sanitizer tests with Rust and native optimization level 1. Keep native and Rust instrumentation, parallel tests, report detection, and the 100 ms guest deadline. LLVM [recommends optimization level 1 or higher](https://clang.llvm.org/docs/ThreadSanitizer.html) for reasonable performance with ThreadSanitizer and gives the [same guidance for AddressSanitizer](https://clang.llvm.org/docs/AddressSanitizer.html#usage). The prior hosted run had guest timeouts and no sanitizer findings. The first follow-up run passed Cargo Audit but exposed the same guest timeouts in AddressSanitizer. Apply optimization to all sanitizer profiles, including native build-script settings. The final remote run must pass before this is treated as a verified CI fix.

## Limits and follow-up

- State uses a synced temporary file and atomic rename. On Unix, a directory sync error after rename can leave the commit result uncertain: disk can contain the new state while memory rolls back. Recovery from injected filesystem failures is not proved. If a state write fails, stop the host and inspect retained state before resuming. Add explicit uncertain-commit handling before relying on this local store for production durability.
- The review catalog holds at most eight revisions. Accepted revisions remain pinned, so capacity can stop later captures. This is an explicit local MVP limit.
- Expressions that the collector cannot resolve remain unknown. Name-based control links remain candidates. This host does not publish enforcement facts or activate policy.
- GitHub has no independent maintainer approval recorded at review time.

## Validation

- `cargo audit --deny warnings`: passed with the updated lockfile.
- Sanitizer script regression tests: all seven passed.
- Full local ThreadSanitizer command and report checks: passed, including all 77 integration library tests. Some child-process reports warn that symbols can be missing; no sanitizer finding was reported.
- Workspace tests with default and all features: passed.
- Workspace Clippy with all targets and features, warnings denied: passed.
- Rust formatting and diff checks: passed.
- GitHub validation must pass on the final commit. See the PR checks and validation note for the remote result.
