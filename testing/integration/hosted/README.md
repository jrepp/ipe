# Hosted GitHub Actions extraction

`github-actions.js` is an operator-installed example program. It owns workflow discovery, action traversal, requirement interpretation, and external outcome mapping.
The Rust host owns connections, credentials, operation grants, retries, schedules, execution budgets, fact expiry, and durable history.
The example reads GitHub definitions and a bounded sample of workflow attempts and jobs. It does not execute GitHub Actions.

The program emits versioned model records:

- `definition`: immutable workflow and action interfaces.
- `evidence`: source excerpts and lexical clues.
- `constraint`: operation requirements, actor context, applicability, and obligations.
- `invocation`: distinct call sites, resolved definitions, and caller/default input bindings.
- `fact`: an expiring observation of an external run or job status.
- `outcome`: provider-reported execution results and durations, keyed by run, attempt, and job.

Workflow `updated_at` is not treated as a completion time. Job durations use `started_at` and `completed_at` only.
Collector latency and retries come from the host, not this program.

The local demo uses the host's Git credential helper for GitHub authentication. The installed program contains no credential lookup or token handling.
