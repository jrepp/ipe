# Hosted integration proof

[proof.json](proof.json) records a live, read-only GitHub capture through the hosted JavaScript program.
It includes one manual extraction and one scheduled extraction against the same pinned workflow.
The proof separates native collector metrics from provider-reported external durations.

Reproduce from the repository root:

```sh
cargo build --locked -p ipe-integration --bins
python3 tools/integration-host/demo.py --out target/integration-host-proof-new --prove-schedule
```

Use `IPE_GITHUB_TOKEN` through a protected environment handoff if GitHub requires authentication or anonymous rate limits prevent capture.
The demo installs code through HTTP, waits for both triggers, writes JSON and HTML snapshots, checks credential exclusion, and stops the host.
The observed outcomes can change when GitHub removes historical runs. A missing outcome is not a successful outcome.

Run deterministic failure checks without GitHub:

```sh
cargo test --locked -p ipe-integration host --lib
node --test tools/integration-host/view.test.cjs
```

The view test uses `docs/node_modules/jsdom`. Install it with `npm ci --ignore-scripts --prefix docs` when needed.
Tests cover operation grants, forged receipts, retries, denied reads, redirects, response limits, restart, scheduling, expiry, and failed extraction retention.
The view test checks measurement separation, unknown durations, expired facts, and provider text escaping.

See [the host guide](../../docs/INTEGRATION_HOST.md) for API details and implemented limits.
