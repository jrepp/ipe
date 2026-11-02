# Hosted integration programs

The integration endpoint hosts operator-installed JavaScript programs. An agent can author the program and submit its source through the API.
The program selects permitted reads, follows external references, and maps source observations into records.
Shared Rust code manages connections, secrets, grants, scheduling, retries, execution limits, and local storage.

This is a local MVP. It stores records for inspection and reasoning. It does not publish facts to enforcement nodes or activate policies.

```mermaid
flowchart LR
    Agent[Agent-authored JavaScript] --> Host[Integration endpoint]
    Host --> Broker[Native connections and read grants]
    Broker --> Provider[External system]
    Provider --> Broker
    Broker --> Code[Sandboxed extraction program]
    Code --> Contract[Requirements]
    Code --> Facts[Expiring status facts]
    Code --> Outcomes[External execution history]
    Host --> Metrics[Collector measurements]
    Contract --> Review[Agent JSON and human monitor]
    Facts --> Review
    Outcomes --> Review
    Metrics --> Review
```

## Run the GitHub example

Build the host:

```sh
cargo build --locked -p ipe-integration --bins
```

Run one manual extraction and one scheduled extraction:

```sh
python3 tools/integration-host/demo.py --prove-schedule
```

The host obtains the GitHub token from the local Git credential helper. The demo needs no token environment variable.
The example pins the IPE docs workflow and limits reads to eight repositories.
It installs [the JavaScript program](../testing/integration/hosted/github-actions.js) through the endpoint. The CLI does not perform extraction itself.

Outputs in `target/integration-host-demo` include:

- `snapshot.html`: a standalone human review view with embedded evidence.
- `snapshot.json`: the same records and history for agents.
- `host-config.json`: named native connection and operation configuration, without credentials.
- `open-monitor.html`: private local launcher for the live monitor. Do not share this file.
- `operator.env`: the private local operator credential. Do not share this file.
- `state/`: private installation source, current records, and bounded run history.

Add `--serve` to keep the endpoint active at `http://127.0.0.1:8767`.
Add `--open` to open the live monitor automatically. You can also open the private `open-monitor.html` file locally.
The launcher submits the operator token in a POST body and redirects to the monitor.
The host sets an HttpOnly, SameSite cookie. Monitor API requests require a custom header and the configured local host address.
The monitor preserves open evidence panels and keyboard focus during refresh.
The monitor checks the browser session automatically. It refreshes the installed program, run history, and records every three seconds.
On connection, the monitor upgrades older v1-only sessions to cover both API versions. Refresh the page after a host upgrade.
Keep `open-monitor.html` private; it contains the operator credential. Share `snapshot.html` instead.
Manual token entry remains available for API operators. The GitHub helper credential is separate from the local operator credential.
Use Ctrl-C to stop the host. Without `--serve`, the demo stops it after capture.
Do not combine `--serve` with the two-second proof schedule for normal use. The default schedule is 300 seconds.

### Refresh the local build

The binary embeds the monitor HTML, styles, JavaScript, and guest SDK at build time.
A browser refresh cannot load source changes from an older running binary.

1. Run `cargo build --locked -p ipe-integration --bins`.
2. Wait for any active capture to finish, then stop the host.
3. Back up the private state directory.
4. Restart `target/debug/ipe-integration <config-path> --host` with the existing operator credential in its environment.
5. Open the existing private `open-monitor.html` launcher.

Reuse the existing configuration and state directory to preserve installations and history.
Do not rerun the demo setup to restart an existing installation; setup installs the example program and writes new operator credentials.
The installed collector source is separate from the embedded monitor. Review Program before changing an installation through the API.

## Monitor views

The monitor has two primary views. **Runtime** opens by default.

- **Runtime** shows installed scope, schedule, collector version, latest capture, and capture duration.
  Use **Runs** for reads, errors, outputs, and external outcomes. Use **Program** for source, input, and grants.
  Use **Changes** to compare retained captures and review catalogs.
- **Integration coverage** shows ontology mappings from one collector run.
  Inspect discovered rules, mechanisms, principals, source evidence, candidate execution links, and missing mappings.
  Use **Operations** for named requirements and evidence checks. Use **Records** for record search, freshness, and collection coverage.

Missing mappings do not prove that an external control is absent. Coverage counts describe retained data, not verified enforcement.
Before connection, the monitor shows connection instructions. Manual operator token entry is available in a disclosure.

The host shows installed code. It cannot show unsaved agent drafts or local files that have not been installed.
During extraction, the monitor shows available read activity. Final duration and output counts appear after completion.
Runs also show bounded program progress reports: phase, task type, queued work, completed work, and gaps.
These reports describe collector work. Native read evidence separately records host-observed activity.
The monitor prevents duplicate start requests while a capture runs. The host enforces the same limit.

### Collector execution and model view

Select a run under **Runs** to open its D3 graph. The graph uses that run's records, not the current catalog.
Choose one operation and one observed execution. The diagram shows that operation, its selected execution, and supporting facts.
Named requirements appear beside the diagram. Requirement fulfillment remains unknown, even when a provider reports execution success.
Exact execution identities connect provider outcomes to facts. Dashed links show candidate connections between operations and provider outcomes.
Select a node to inspect its details. Collection reads and source records remain available in a separate evidence disclosure.
Unmapped observations remain available when no operation link was captured. Missing links are shown explicitly; the view does not infer them from names.
The view marks failed reads, missing receipt links, unresolved model references, and reads without emitted records.
A read without an emitted record can be a valid traversal step. These gaps do not establish a policy violation.
Candidate references remain uncertain. An observed fact does not establish requirement fulfillment or authorization.

New runs include native request steps and emission steps. An emission means the host validated a record during collection.
Running emissions remain staged. Failed runs retain emission traces but do not publish partial records or renew facts.
Successful runs publish their captured records. Older runs show unavailable emission steps instead of inferred timing.
Each trace retains at most 256 emissions and 128 KiB. The focused diagram limits facts per page and reports incomplete traces. The supporting projection retains a bounded node set.
Fact details retain observation time and expiry; the view does not replay a historical policy evaluation.

The host serves a pinned D3 build locally. Exported snapshots include the graph assets and need no CDN connection.

Choose a retained capture as the source comparison baseline. The preview limits large source comparisons and identifies omitted lines.
Full retained source remains available below the preview. The Program view shows the current installed source.
Without an accepted catalog, current declarations are not confirmed changes to an accepted contract.
Catalog acceptance does not install source, activate policy, or renew facts.

Snapshots remain read-only. Source comparison requires retained manifests in the optional `revision_details` snapshot field.
Existing snapshots without those manifests still show program, records, runs, and catalog review data.

## Versioned record types

### Governance extraction and execution overlay

Select a policy or contract under **Integration coverage** to open its concept map.
The map groups governing rules, mechanisms and people, operation scope, and execution evidence.
Select a concept to open its mapping details. Adjacent cards do not establish a relationship.
Use **Show gaps and unknowns** to find candidate references, missing mappings, expired evidence, and unknown applicability.
The detail panel states the source, relationship basis, and next check. Source receipts and emission steps remain in disclosures.
The map preserves concept selection during refresh and resets selection when you change the governing rule or capture.

Select one named requirement to inspect its retained configuration fields and source record.
Opaque or truncated specifications remain source text. The view does not infer missing values.
The requirement panel marks the missing binding between a governance requirement and an evidence predicate or check.
Facts with matching execution identities do not establish this binding or requirement fulfillment.
Capture scope, collector settings, execution summaries, and complete descriptions remain in collapsed disclosures.

Source details show retained paths, revisions, native reads, and exact record emission steps when available.
The source location identifies a retained record. It does not establish a verified field-level transformation.
Operation subjects, candidate execution links, and facts with exact execution identities remain separate.

Current collector read grants, output grants, schedule, and fact lifetime appear under collector runtime configuration.
Saved snapshots label these settings as historical. They do not establish the current installation.
These settings control collection. They do not establish external policy configuration or activate IPE enforcement.
Use Changes to compare the current installation with a retained capture.


The collector uses permitted APIs to extract external governance. The host retains native read receipts for each emitted description.
The Integration coverage view compares those descriptions with external execution evidence from the same capture.
Collector execution is extraction activity; an external execution is an observed attempt in the governed system.

Model v2 adds `entity.kind: "governance"` with a typed `record`. Existing entity kinds continue to use model v1.
Installations must grant the `governance` output kind. SDK 0.7 provides `ipe.governanceModel(record, {scope, claim})` and receipt-bound `ipe.observation`.

| Governance record type | Captured meaning |
| --- | --- |
| `policy` | Governing source, declared authority, subject identities, and named requirements |
| `contract` | Identified parties, governing subject identities, and named terms |
| `control` | Mechanism description and references to governing policies or contracts |
| `handoff` | Policy reference, sender and recipient, trigger, required action, completion, and continuation conditions |
| `principal` | Authority-scoped human, service, group, or role identity |
| `role_assignment` | Sourced membership, responsibility, approval authority, or delegation with validity bounds |
| `contact_method` | Sourced route, owner, channel, availability, purpose, and optional validity limit |

Each record includes a source revision. Resolved references require the exact typed target and source revision in the same capture.
Candidate and unknown references can remain unresolved. Resolved identity does not verify authority or effective enforcement.
Policy subjects can reference existing operation identities. An operation's matching provider outcome remains a candidate link.
Facts link to outcomes by exact execution identity. A matching name or shared receipt does not establish a semantic link.

Role assignments and contact methods expire at the earlier of the native fact lifetime and any source validity limit.
They remain separate from accepted review declarations. Restoring a catalog or failing a capture does not renew them.
Contact methods display as source-provided text; the monitor does not send messages or infer missing addresses.
The current capture does not verify contact reachability, approval authority, handoff completion, or requirement fulfillment.

The [synthetic fixture](../testing/integration/hosted/governance-handoff.json) covers an automated release gate and a human approval handoff.
Native tests exercise API extraction, grants, receipt provenance, exact reference validation, expiry, atomic failure, and restart.
The GitHub collector now reads environment protection and repository ruleset APIs through separately granted native operations.
It links literal workflow environment names to operation identities; dynamic expressions and unmatched environment names remain gaps.
Required reviewers produce account or team identities, scoped approval-role descriptions, handoffs, and source-provided profile routes.
A GitHub `User` account does not establish a human identity, so its principal kind remains `unknown`. Bot and team types remain distinct.
The collector does not infer email addresses, team membership, policy owners, or governing contracts from these responses.

Grant `list_environments`, `list_rulesets`, and `get_ruleset`, plus the `governance` output kind, to enable these reads.
The demo installation includes these grants and retains API version `2022-11-28`.
The collector samples at most two environments, two reviewers per environment, six environment rules, and six ruleset details.
Each ruleset description retains at most 32 rules. Long specifications are bounded and report a truncation gap.
Denied reads, missing grants, unmatched literal environments, and partial samples preserve explicit gaps; they do not establish policy absence.
Rulesets retain provider scope instead of an inferred workflow link. Neither ruleset configuration nor execution success proves effective enforcement.

See GitHub's [environment API](https://docs.github.com/en/rest/deployments/environments?apiVersion=2022-11-28) and [ruleset API](https://docs.github.com/en/rest/repos/rules?apiVersion=2022-11-28).
Native synthetic API tests execute the actual collector and validate successful capture, denied reads, missing grants, source links, and expiry.
These tests do not establish a particular repository's live configuration or access permissions.
Current API configuration does not establish which policy revision applied to an earlier execution; temporal applicability remains unknown.
Complete governance removal coverage, handoff event records, escalation rules, and live directory resolution remain future work.

### Existing v1 records

| Record | Meaning | Agent use | Human view |
| --- | --- | --- | --- |
| `definition` | Versioned workflow or action interface | Resolve source revisions and declared inputs | Review declared interfaces |
| `evidence` | Source excerpt or lexical clue | Support an inference without treating it as a rule | Inspect raw source separately |
| `constraint` | Operation description with applicability and named requirements; the wire name remains v1 | Identify prerequisites and uncertainty | Review actor, requirements, dependencies, and declared enforcer |
| `invocation` | A call site and its input bindings | Distinguish repeated calls and defaults | Follow calls to captured definitions |
| `fact` | Provider-reported status with finite expiry | Supply evidence for a later authorized evaluation | Inspect current and expired observations |
| `association` | Expiring link between subjects under explicit context | Connect evidence across named sources | Review exact and candidate links |
| `outcome` | Execution result with structured attempt identity | Compare actual attempts | Inspect status, duration, and step results |

Each emitted observation contains `model`, `data`, and `receipts`.
Existing entity kinds use `model.schema_version: 1`; governance uses version 2. The host validates each entity and derives its key and revision digest.
`data` retains raw provider material for inspection. It does not implicitly add executable constraints.
Use authenticated `GET /v1/host/record-schema` to obtain JSON Schema generated from the Rust model types.
Native validation also checks grants, receipt ownership, bounded values, duplicate identities, and resolved definition links.

Operation identity (`ControlId` in stored v1 records) excludes source revision. Record identity and revision digests preserve different source versions and mapping versions.
Invocation identity includes its parent definition and call-site pointer. Execution identity includes provider, resource, run, attempt, job, and step.
Facts with an execution reference must use that execution's canonical subject identity.
A display-name correlation remains a candidate. It cannot become a resolved declaration link.

Stored records without a v1 model remain labeled as legacy untyped evidence.
The outcome export keeps these under `legacy_untyped`; they do not count as new typed executions.
Old programs must emit v1 observations before they can run successfully on the updated host.
New installations cannot request the obsolete `contract` output kind.

A declared permission is not proof of effective permission. An action input declaration is not proof of runtime validation.
The GitHub program leaves expression evaluation, environment protection, and actor authorization unresolved.
A job name can suggest a declaration match. It does not establish a unique match for matrix jobs or dynamic names.

Each record includes its adapter digest, run identity, native observation time, and successful read receipts.
Native source evidence includes operation parameters, source digest, observation time, and truncation status.
Receipts establish which reads supported a record. They do not prove that the program interpreted those reads correctly.

The host updates current requirements and facts only after a successful extraction. A failed extraction does not remove requirements or refresh facts.
Facts retain their original expiry after failure. Missing evidence must not become an allow decision.
Current records are upserts by kind and key unless the host verifies a complete declaration scope.
A YAML read operation can define `snapshot_source` with `namespace`, `resource_parameter`, `path_parameter`, and `revision_parameter`.
These parameter names must refer to native operation parameters. Only a full YAML document can establish this authority.

The GitHub example emits one job constraint per continuation step to bound modeling work.
The program can return `collections` in its completion output. Each claim contains `schema_version: 1`, `scope`, `kind: "constraint"`, and a native `receipt`.
The host verifies the receipt, successful read status, source identity, and absence of truncation.
It replaces constraints only within that source definition and scope. A verified empty collection can remove prior constraints.
Missing claims, partial reads, and failed runs preserve prior constraints. A failed run does not publish a partial catalog.
Complete declarations do not establish complete effective policy coverage. Permissions, environment protection, and actor authorization can remain unknown.

Native read evidence reports observed, unavailable, or pending status, including a sanitized failure code.
Link pagination, provider truncation flags, record limits, and provider counts identify incomplete collections.
A failed mapper retains native read evidence for diagnosis. Fact expiry starts at the earliest supporting native observation time.
The coverage API lists the latest reads, verified declaration scopes, and retained constraints that were not observed in the latest run.

Outcomes are separate from collector telemetry. A durable archive retains each execution independently of the eight collector logs.
Repeated polling updates its observation time without adding an execution or an unchanged payload to its change history.
The archive retains up to 256 execution identities and 16 changed payloads per identity, within 4 MiB.
It reports omitted change counts. When identity or byte capacity is exhausted, the capture fails without replacing existing records.
Use `GET /v1/integrations/:id/outcomes` to export the archive, including native evidence.
The GitHub key includes repository, workflow run, attempt, and job identity where applicable.
This permits tracking real executions without counting each poll as another execution.

See [contextual evidence assessments](INTEGRATION_ASSESSMENTS.md) for cross-source dependencies, ticket approvals, and compliance evidence.

## Review catalogs and operation explanations

Each successful capture creates an immutable review catalog when its content or collection context changes.
The manifest records typed declarations, native source evidence, coverage, SDK version, installed JavaScript, input, grants, and native operation digest.
Repeated observations of the same content reuse its revision identity. Their latest timestamps remain in current observations.

The host retains up to eight revisions within 4 MiB. It can evict an unaccepted revision.
Accepted revisions remain available for restoration. If accepted revisions fill the archive, new captures fail atomically with `revision_capacity`.
Export retained manifests before clearing or replacing the local state directory. This MVP has no revision deletion endpoint.

Acceptance selects the catalog used for local review and operation explanations. It does not activate enforcement or replace collector code.
Restoration selects a previously accepted catalog. Current facts and external outcomes stay separate, with their original timestamps and expiry.
New captures update the candidate catalog without replacing the accepted selection.

The diff uses logical identities. It distinguishes added or removed declarations, requirement changes, source changes, mapping changes, changed evidence, and evidence refreshes.
Conditions, dependencies, input defaults, and call bindings remain part of the requirement comparison.
A source revision change alone does not become a changed requirement. The diff also reports collection context changes.
Opaque provider expressions remain opaque; this is not a proof of equivalent provider behavior.

The monitor and `/explanations` use the same native projection. Each operation includes its condition, actor context, prerequisites, source evidence, unknowns, and candidate outcomes.
Fact requirements compare the exact subject and predicate against fresh facts. Missing, expired, or conflicting facts produce an unknown assessment.
Requested permissions, environment protection, and execution actor authorization remain unknown without supporting observations.
The projection returns only follow-up operations granted to the current installation. It does not run them automatically.
Historical outcomes match the source revision and declared control candidates. Candidate links never establish exact execution authorization.

The monitor provides operation selection, readable requirement summaries, coverage details, and accept/restore controls.
It retains raw records for inspection. Static snapshots disable actions that change the host.

To repeat the live review proof:

```sh
python3 tools/integration-host/demo.py --out target/integration-review-proof --port 8768 --prove-review
```

This proof captures GitHub twice with different collector source revisions. It accepts both catalogs and restores the first.
It verifies unchanged observed records and expiry after restoration. Deterministic tests cover changed requirements and external source revisions.

## Performance evidence

The host measures collector elapsed time, HTTP attempts, retries, bytes, read failures, and operation latency.
Operation latency includes retry waits. These measurements describe extraction performance.

GitHub supplies external status, conclusion, and timestamps. Job and step duration use `completed_at - started_at`.
Workflow created-to-start time uses `run_started_at - created_at`. This interval is not a measured scheduler queue duration.
The workflow API supplies no authoritative completion timestamp in this example. Workflow execution duration remains unknown.
`updated_at` is not substituted for completion time.

The sample includes up to three workflow runs and six jobs per attempt at the selected commit.
It does not establish a population success rate, latency percentile, or service-level objective.
Large action bundles, incomplete traversal, and rejected reads remain explicit extraction gaps.
They are not counted as external control failures.

## Endpoint and SDK

Start an existing configuration with `ipe-integration <config-path> --host`.
The endpoint binds only to a loopback address. API routes require a bearer operator token of at least 32 bytes.

| Method and route | Behavior |
| --- | --- |
| `GET /v1/host/record-schema` | Return the versioned model JSON Schema |
| `GET /v1/host/capabilities` | Return named operations, parameter constraints, record kinds, and limits |
| `PUT /v1/integrations/:id` | Validate and durably install JavaScript source, input, operation grants, schedule, TTL, and record grants |
| `GET /v1/integrations/:id` | Return installed JavaScript, input, operation grants, schedule, and TTL |
| `GET /v1/integrations` | List installations, schedules, latest status, and expired fact counts |
| `POST /v1/integrations/:id/runs` | Start a manual extraction; reject overlap |
| `GET /v1/integrations/:id/runs` | Return retained collector runs and independent archived external outcomes |
| `GET /v1/integrations/:id/outcomes` | Export execution identities, changed outcomes, first/last observation times, and evidence |
| `GET /v1/integrations/:id/coverage` | Return native read status, verified declaration scopes, and retained constraint provenance |
| `GET /v1/integrations/:id/revisions` | List immutable catalogs and diff current observations against the accepted catalog |
| `GET /v1/integrations/:id/revisions/:revision` | Export a retained manifest |
| `POST /v1/integrations/:id/revisions/:revision/accept` | Select a catalog for local review |
| `POST /v1/integrations/:id/revisions/:revision/restore` | Select a previously accepted catalog |
| `GET /v1/integrations/:id/catalog` | Return selected declarations, current facts, and expiring associations |
| `POST /v1/integrations/:id/assessments` | Assess configured evidence rules against caller-selected context; no mutations or provider reads |
| `GET /v2/integrations/:id/operations` | Read selected operation descriptions and exact mapped record revisions |
| `POST /v2/integrations/:id/assessments` | Check pinned evidence rules; keep evidence matches separate from requirement fulfillment |
| `GET /v1/integrations/:id/explanations` | Explain operation requirements, evidence, unknowns, outcomes, and permitted next observations |
| `GET /v1/integrations/:id/records` | Return current requirements and facts, including expiry flags |

The installation body contains `source`, `operations`, `input`, `interval_seconds`, `enabled`, `fact_ttl_seconds`, and `output_kinds`.
See [v2 domain APIs and rule migration](INTEGRATION_ASSESSMENTS.md#versioned-domain-apis) before using the new assessment route.
See [demo.py](../tools/integration-host/demo.py) for complete configuration and API requests.

SDK 0.3 adds typed `ipe.observation` output. SDK 0.2 introduced `ipe.call`, `ipe.exchange`, and `ipe.timestamp`.
`ipe.call` names an approved operation. Native parameter constraints authorize the read.
`ipe.exchange` returns requests, continuation state, and records with successful receipt references.
`ipe.timestamp` parses an explicit timestamp. Guest code has no ambient clock, network, filesystem, or secrets.
Existing ServiceNow adapters retain their operation restrictions and reject hosted observation output.

Native connection configuration supplies fixed origins and host-owned credential sources.
For local GitHub access, configure:

```json
{
  "id": "github",
  "origin": "https://api.github.com/",
  "git_credential_url": "https://github.com"
}
```

The host invokes `git credential fill` with the credential URL on standard input.
Git selects helpers from the host's Git configuration, including repository configuration in its working directory.
The helper's password becomes the connection's bearer token. This mode requires a helper that returns an API-compatible token.
The operator explicitly maps the credential lookup URL to the API origin. Installed programs cannot change either address.
Paths in the lookup URL follow Git's `credential.useHttpPath` setting.

The host disables terminal prompts and asks supported helpers to avoid interactive login.
Lookup has a five-second deadline and a 16 KiB output limit. Errors contain fixed codes, without helper output or diagnostics.
Missing credentials fail startup. The host does not fall back to anonymous access or invoke `approve` or `reject`.
Credentials remain in native memory. They do not enter installation source, guest responses, durable records, or capabilities.
The host rejects response content that contains a configured credential.

Credentials are resolved at startup and cached until shutdown. Restart the host after a token expires or changes.
Automatic refresh during a running host remains unimplemented.
Existing `credential_env` configurations still work. A connection cannot specify both credential sources.
Neither source is permitted for plain HTTP connections.
See [Git's credential interface](https://git-scm.com/docs/git-credential) for helper selection and the credential protocol.

Operation configuration supplies path templates, parameter constraints, query templates, decoders, and JSON field projections.
The shared broker supports JSON, bounded YAML, and text. GitHub traversal and interpretation remain in the installed program.

## Limits and failure behavior

- Four installations, two concurrent workers, and one active run per installation.
- At most 128 steps, 96 HTTP attempts, 8 MiB received, and 180 seconds per run.
- At most three attempts per read. Retry transport failures, HTTP 429, and server errors within the budget.
- Reject redirects. Accept numeric `Retry-After` values up to two seconds; report other values as budget errors.
- Reject duplicate JSON keys, invalid YAML structures, forged receipts, and operations outside native grants.
- Retain at most 256 current records, 256 records per run, and eight runs per installation.
- Limit the durable store to 16 MiB. Reject excess capacity without replacing current records.
- Store source and records in a private directory with one writer and atomic file replacement.
- Sync the state file before replacement. Unix also syncs the directory entry; Windows does not provide this directory sync through the standard file API.
- Restore interrupted runs as interrupted. Do not replay missed schedule intervals.

This host has one operator scope. It is not a multi-tenant service or a long-term metrics database.
OAuth refresh, distributed scheduling, archival export, policy activation, and enforcement fact publication require further work.
