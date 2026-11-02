# Integration host usability review

## Current pass: 2026-11-02

The current monitor has two primary views: Runtime and Integration coverage.
The dated sections below retain evidence from earlier layouts and SDK versions.
They are not verification of the current build.

The local server still served the November 1 binary after the monitor source changed on November 2.
The binary embeds the UI. Rebuilding and restarting with the existing configuration restored the current views.
The private state backup remains in `target/integration-host-local`. Existing installations, credentials, and history were preserved.
The host guide now explains this restart procedure and distinguishes it from demo setup.

The browser audit found a skipped heading level and an unsupported label on the ontology container.
The ontology title now uses heading level 3; its lanes use level 4. The named container now has a group role.

### Current verification

- Default and all-feature workspace tests each passed 346 tests, with two opt-in tests ignored.
- The discovery acceptance test passed when explicitly enabled.
- All-feature workspace Clippy passed with warnings denied. Formatting and the TypeScript authoring check passed.
- All 64 host JavaScript tests passed, including the final accessibility markup changes.
- `make docs-check` passed after the documentation updates.
- The rebuilt host serves the current source. The installed collector source also matches the repository file.
- All seven detail views fit a 390-pixel viewport. Navigation moves focus to the selected heading.
- The ontology gap filter and selected requirement survive refresh.
- Axe reported zero violations across all seven views after the fix. The browser reported no application script errors.
- Manual GitHub capture 641 succeeded in 11,080 ms, with 41 requests, nine failed reads, and 58 stored records.
- The monitor disabled extraction while the run was active, then restored the action after completion.

Evidence: [desktop coverage](screenshots/2026-11-02-coverage.png) and [mobile coverage](screenshots/2026-11-02-mobile-coverage.png).

### Current limits

Axe could not determine contrast for some SVG graph labels in Runs. This remains a manual accessibility review item.
The successful capture includes failed reads and collection gaps. It does not establish complete external policy coverage.
Requirement fulfillment and policy applicability remain unknown. No policy activation or deployment occurred.
The local server remains available at `http://127.0.0.1:8767/` for review.
See the [workstream register](../../../.agent/workstreams.md) for committed work and next actions.

## Earlier pass: 2026-10-29

Date: 2026-10-29. URL: http://127.0.0.1:8767. Browser session: `ipe-usability`.
Scope: authored program, installed state, captured records, runs, and changes.
The review used agent-browser with the existing GitHub installation and real retained data.

## Initial findings

Five medium-severity UX issues; no critical or high-severity issue observed.
These are visible information gaps. Repro video: N/A for each finding.

### ISSUE-001: Program scope is hidden

Open the monitor. The installed program shows operation names but hides repository, workflow, revision, and input behind a source disclosure.
The operator cannot identify what is being collected from the summary.
Evidence: [Initial overview](screenshots/before-overview.png).

### ISSUE-002: No navigation between the main tasks

Open the monitor. Authored code, run history, contract requirements, changes, and raw records share one long page.
There is no direct navigation or clear starting point for each task.
Evidence: [Initial overview](screenshots/before-overview.png).

### ISSUE-003: Runs lack time and evidence drill-downs

Scroll to Collector runs. The rows show IDs, status, duration, and retries.
They do not show start time, the collector version, read activity, or captured outputs.
Evidence: [Run history](screenshots/before-runs.png).

### ISSUE-004: Revision changes lack a useful baseline and code comparison

Scroll to Revision review. No accepted catalog exists, so the list presents all current records as changes.
Long revision hashes and raw change records do not explain collector source or configuration differences.
Evidence: [Revision review](screenshots/before-changes.png).

### ISSUE-005: Captured objects have no inventory controls

Scroll to Definitions, evidence, constraints, and facts.
The operator must scan one list without type counts, search, or a distinction between typed and legacy records.
Evidence: [Record list](screenshots/before-records.png).

## Verification after changes

All five findings have UI changes and browser checks.

| Finding | Change | Evidence |
| --- | --- | --- |
| ISSUE-001 | Overview shows resource, workflow, revision, schedule, and installed scope. Program exposes source, input, and grants. | [Overview](screenshots/final-overview.png), [Program](screenshots/after-program.png) |
| ISSUE-002 | Five named views separate tasks. Navigation resets scroll and refresh preserves open panels and focus. | [Navigation](screenshots/final-overview.png) |
| ISSUE-003 | Select a capture to inspect its time, version, reads, errors, and outputs. Active runs show pending final metrics. | [Active run](screenshots/run-in-progress.png), [Completed run](screenshots/run-complete.png), [Run history](screenshots/final-runs.png) |
| ISSUE-004 | Select a retained capture to compare source and configuration. Catalog review explains missing accepted baselines. | [Live comparison](screenshots/final-changes.png), [Synthetic changed source](screenshots/synthetic-source-comparison.png) |
| ISSUE-005 | Records have type counts, text search, expiry labels, and links to their capture run. | [Mobile records](screenshots/mobile-records.png), [Facts](screenshots/mobile-facts.png) |

### Verification

- Started the rebuilt integration host with the existing installation, state, and host-managed credentials.
- Manual GitHub capture 241 completed successfully. The inventory contained 54 records: 32 typed and 22 legacy records.
- Exercised record filters, search, empty results, program inspection, capture details, and retained-capture comparison with agent-browser.
- The live installed program matched its retained capture. No source installation or catalog acceptance was needed for this review.
- A separate read-only synthetic snapshot verified changed source, changed input, and escaped HTML-like source text.
- At 390 pixels, all five views had no horizontal page overflow. [Mobile overview](screenshots/final-mobile-overview.png).
- Axe reported zero violations in Overview, Program, Records, Runs, and Changes. No browser script errors were reported.
- Nine JavaScript tests passed. They cover view state, comparisons, escaping, duplicate starts, failures, and stale refresh responses.
- All 38 host tests passed, including session authentication, persistence, scheduler concurrency, and capture failure handling.
- The integration binary built successfully. `git diff --check` passed. Full workspace checks were not run for this monitor-only change.

### Limits and next decisions

The monitor shows installed code, not unsaved drafts or local agent files. Author identity and draft history need a separate authoring model.
Program comparison uses retained captures. It does not provide a complete edit log between captures.
Existing snapshots without retained manifests explain why source comparison is unavailable.
Large source comparisons have bounded previews and links to full source.

Video recording could not run because `ffmpeg` was unavailable. Screenshots provide the review evidence.
The local host remains running for user review. No manual changelog entry was added, per the requested release automation direction.


## SDK follow-up: installed collector review

The installed program contained 204 lines of JavaScript and predated the repository's projection and incremental emission fixes.
The SDK follow-up adds a shared bounded work plan, receipt-bound model envelopes, projection, batching, duration, and execution-key helpers.
Hosted context now exposes granted operation metadata. Read results expose native pagination and snapshot source metadata.
The host validates and retains structured progress reports. The monitor labels these as program reports, separate from native read evidence.

The migrated collector includes workflow attempt in task identity. It preserves explicit interpretation, sampling, and coverage decisions.
The SDK version is 0.4.0. Its source remains part of the collector digest.

Live GitHub capture 244 succeeded with 39 reads, 35 emitted records, 45 steps, and 44 progress reports.
The capture completed in 10,639 milliseconds. This single observation is not a performance benchmark.
Evidence: [SDK progress in Runs](screenshots/sdk-progress.png).
The local installation now uses the migrated collector. Existing grants, schedule, TTL, and history were preserved.
A private state backup remains under the local target directory.

Verification included deterministic collector replay, root-read denial, truncation, SDK bounds, receipt preservation, native progress validation, and restart persistence.
Workspace tests passed with default and all features. Workspace all-feature Clippy passed with warnings denied.
The discovery acceptance test and TypeScript authoring example passed.
Agent-browser confirmed the progress view, zero Axe violations, no script errors, and no mobile page overflow.

General collection builders, generated input schemas, and a separately bundled GitHub helper library remain follow-up work.
The work plan does not replace native authorization, retries, scheduling, deadlines, or atomic capture validation.


## Contextual dependency and cross-source follow-up

The host now supports expiring association records and host-configured evidence rules.
Rules pin the consumer control and source revision, source installations, authorities, predicates, expected values, and required context.
The same native assessment engine joins CI, ticket, and compliance observations. It contains no GitHub-specific matching logic.

Live GitHub capture 251 succeeded with 39 reads, 37 records, and 46 progress reports across 47 steps.
Two association records link declared controls to observed job executions under the exact parent run, attempt, and source revision.
The publish dependency now exposes the build-job success and its native provenance.
Its assessment remains unknown because the declaration-to-job association is a display-name candidate.
Evidence: [Context assessment](screenshots/context-assessment.png).

The cross-source fixture runs three collectors through the actual host and native broker against a synthetic provider.
It proves exact ticket approval and compliance evidence joins, without connecting real ticketing or compliance accounts.
Tests cover wrong attempts, changed artifacts, expiry, future observations, authority, ambiguous targets, conflicts, and bounded matches.
A literal historical fact cannot bypass contextual dependency matching. Accepted catalogs do not freeze association freshness.
The API rejects unauthenticated requests, duplicate JSON keys, and oversized bodies.

The monitor adds an Associations filter and a context assessment form under Records.
Operation selection includes source revision. Results show the rule, producer, association, observed value, and expiry.
Agent-browser verified the live form, zero Axe violations, no script errors, and no page overflow at 390 pixels.

Workspace default and all-feature tests passed: 322 tests each, with two opt-in tests ignored.
The discovery acceptance test passed when explicitly enabled.
After final guard and display fixes, all 48 host tests and 16 JavaScript tests passed.
All-feature workspace Clippy, formatting, and the TypeScript authoring check passed.

The local host remains running with the migrated collector and one explicit GitHub dependency evidence rule.
Private state and host-configuration backups remain under the local target directory.
The local host still supports four installations. Thousands-of-collector deployment capacity is not claimed.
See [platform contract and limits](../../../docs/INTEGRATION_ASSESSMENTS.md) and the linked synthetic fixture for the reusable design.
