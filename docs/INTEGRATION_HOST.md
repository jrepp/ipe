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
The monitor checks the browser session automatically. It refreshes the installed program, run history, and records every three seconds.
Keep `open-monitor.html` private; it contains the operator credential. Share `snapshot.html` instead.
Manual token entry remains available for API operators. The GitHub helper credential is separate from the local operator credential.
Use Ctrl-C to stop the host. Without `--serve`, the demo stops it after capture.
Do not combine `--serve` with the two-second proof schedule for normal use. The default schedule is 300 seconds.

## Three record types

| Record | Meaning | Agent use | Human view |
| --- | --- | --- | --- |
| `contract` | Requirements, declared permissions, dependencies, interfaces, and source clues | Plan required operations and identify unknown prerequisites | Review requirements and exact source evidence |
| `fact` | External status observed at a stated time, with finite expiry | Supply evidence for a later authorized evaluation | Inspect current and expired observations |
| `outcome` | A provider-reported execution result, keyed by execution and attempt | Compare previous attempts and diagnose failures | Inspect status, duration, and step results |

A declared permission is not proof of effective permission. An action input declaration is not proof of runtime validation.
The GitHub program leaves expression evaluation, environment protection, and actor authorization unresolved.
A job name can suggest a declaration match. It does not establish a unique match for matrix jobs or dynamic names.

Each record includes its adapter digest, run identity, native observation time, and successful read receipts.
Native source evidence includes operation parameters, source digest, observation time, and truncation status.
Receipts establish which reads supported a record. They do not prove that the program interpreted those reads correctly.

The host updates current requirements and facts only after a successful extraction. A failed extraction does not remove requirements or refresh facts.
Facts retain their original expiry after failure. Missing evidence must not become an allow decision.
Current records are upserts by kind and key. This MVP does not infer deletion from an absent observation.

Outcomes are separate from collector telemetry. Repeated polling retains one latest outcome per key within retained history.
The GitHub key includes repository, workflow run, attempt, and job identity where applicable.
This permits tracking real executions without counting each poll as another execution.

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
| `GET /v1/host/capabilities` | Return named operations, parameter constraints, record kinds, and limits |
| `PUT /v1/integrations/:id` | Validate and durably install JavaScript source, input, operation grants, schedule, TTL, and record grants |
| `GET /v1/integrations/:id` | Return installed JavaScript, input, operation grants, schedule, and TTL |
| `GET /v1/integrations` | List installations, schedules, latest status, and expired fact counts |
| `POST /v1/integrations/:id/runs` | Start a manual extraction; reject overlap |
| `GET /v1/integrations/:id/runs` | Return retained collector runs and distinct external outcomes |
| `GET /v1/integrations/:id/records` | Return current requirements and facts, including expiry flags |

The installation body contains `source`, `operations`, `input`, `interval_seconds`, `enabled`, `fact_ttl_seconds`, and `output_kinds`.
See [demo.py](../tools/integration-host/demo.py) for complete configuration and API requests.

SDK 0.2 adds `ipe.call`, `ipe.exchange`, and `ipe.timestamp`.
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
- Restore interrupted runs as interrupted. Do not replay missed schedule intervals.

This host has one operator scope. It is not a multi-tenant service or a long-term metrics database.
OAuth refresh, distributed scheduling, archival export, policy activation, and enforcement fact publication require further work.
