# IPE Documentation Index

**Welcome to the Idempotent Predicate Engine documentation!**

## Local Documentation Site

Run `make docs-site` to export the generated docs data, then `make docs-serve`
and open <http://127.0.0.1:8000/docs/>.

> **Legacy data.** The site's parser trees and interpreter microbenchmarks are
> generated from the removed legacy engine and are kept as an archive. They do
> not measure the v2 runtime, whose performance under production load is not
> characterized.

The build requires Rust and [uv](https://docs.astral.sh/uv/). Python is pinned to
**3.15.0** in `.python-version` (the stable release published October 9, 2026).
Both local Make targets and CI read that file; the builder rejects other versions.
The first build downloads Cargo dependencies. If an older uv cannot install the
pin, update uv or use `uvx --from uv uv python install 3.15.0` once.

- [AST explorer](ast.html): rendered, expandable trees with source and node details (legacy-parser output, archived).
- [Contract summary](contract.html): inputs, decisions, application responsibilities,
  and current implementation limits in plain language for the v2 runtime.
- [Performance dashboard](performance.html): archived legacy-interpreter latency
  percentiles, throughput, outliers, downloadable results, and requirement coverage.

Generated data lives in `docs/data/site-data.json` and records the run time, platform,
Rust version, and Git revision. Run `make docs-site` again after code changes.
The dashboard measured hand-built bytecode through the legacy interpreter; it excluded
parsing, compilation, context construction, and JIT. These are local microbenchmarks
of the removed engine, not production performance validation.

### Checks and pull request artifacts

```sh
npm ci --prefix docs --ignore-scripts
make docs-check
```

The checks validate parser structure, percentile consistency, portable artifact
links, expanded tree rendering, node selection, example switching, dashboard
filters, and missing-data errors. JavaScript checks use jsdom; they do not verify
browser layout. The new views need no CDN libraries. Existing architecture
diagrams on the overview still use Mermaid from its CDN.

The **Docs site / Build and check docs site** workflow runs on pull requests and
main-branch pushes. It uses the Python pin, locked Cargo/npm dependencies, and
immutable GitHub Action revisions. It uploads `target/docs-site/` as a 14-day
artifact named for the commit. Download and extract the artifact, serve its root
with `python -m http.server --bind 127.0.0.1`, then visit `/docs/`.
This preserves the relative paths to Markdown and source references.

The HTTP test serves the packaged site under `/ipe/`, the GitHub Pages project path.
It checks the root redirect and each local HTML link.
It also checks that the server returns the generated parser and benchmark data.
The same test runs through `make docs-check` and the PR workflow.

Fresh measurements are generated in CI and kept in the artifact, not committed.
Shared-runner timings are reported as evidence and never used as pass/fail
performance thresholds. JIT, end-to-end policy compilation, and production load
remain outside the measurement scope. Dependency updates are proposed monthly
by Dependabot; update `.python-version` after checking a new stable Python release
and rerun `make docs-check`.

### Automated publishing

The same workflow publishes the checked artifact to GitHub Pages after a successful
push to `main`. Pull requests build and validate the Pages archive without deploying.
The live site address is `https://jrepp.github.io/ipe/`; its root forwards to `docs/`.
Publication begins once the workflow change is merged into `main`.

Repository Pages settings use **GitHub Actions** as the publishing source. The
`github-pages` environment permits deployment from `main` only. The deploy job has
the Pages and identity-token permissions it needs; the PR build stays read-only.
Python comes from `.python-version` in both local builds and CI. Deployment runs
are serialized, and a newer push does not interrupt a running main-branch build
or deployment.

To republish, run **Docs site → Run workflow** with `main` selected. To roll back,
revert the unwanted change on `main`; the next successful workflow republishes the
site. If a check fails, the deploy job does not run and the last published site
remains available. The workflow's deployment environment links to the live site;
the artifact also remains downloadable for review.

This index provides a guided path through IPE's documentation based on your needs.

## Getting Started

**New to IPE?** Start here:

- **[Runtime data workflow](RUNTIME_DATA.md)** — implemented authenticated push/pull, TTL, language-2 policies, deployment limits, and external HTTP acceptance tests.

1. **[README](../README.md)** - Project overview, badges, and quick links
2. **[QUICKSTART](../QUICKSTART.md)** - Get IPE running in 5 minutes
3. **[END_TO_END_EXAMPLE](../END_TO_END_EXAMPLE.md)** - Legacy engine lifecycle walkthrough (historical)

## Understanding IPE

**Learn the architecture and design:**

- **[SUMMARY](../SUMMARY.md)** - Executive summary and vision (historical)
- **[ARCHITECTURE](ARCHITECTURE.md)** - v2 runtime architecture with diagrams
- **[ROADMAP](../ROADMAP.md)** - Development phases and milestones

## Technical Reference

**Deep dives into IPE internals:**

- **[RUNTIME_DATA](RUNTIME_DATA.md)** - The implemented v2 runtime: API, limits, and workflows
- **[AST](AST.md)** - Legacy parser AST specification (historical)
- **[BYTECODE](BYTECODE.md)** - Legacy bytecode instruction set (historical)
- **[RFC Collection](../rfcs/INDEX.md)** - Design proposals and specifications

## RFCs (Request for Comments)

**Design documents for major features:**

See **[rfcs/INDEX.md](../rfcs/INDEX.md)** for the complete RFC collection, including:

- **RFC-001:** Sidecar service architecture
- **RFC-002:** SSE/JSON protocol specification
- **RFC-003:** Predicate tree storage system
- **[RFC-009](../rfcs/009-runtime-facts-and-policy-language.md):** Runtime facts, authenticated push/pull ingestion, mandatory TTL, and typed policy language (draft)

## Crate Documentation

**Per-crate technical docs:**

- **[ipe-core](../crates/ipe-core/)** - Runtime facts, policy documents, language-2 evaluation
- **[ipe-server](../crates/ipe-server/)** - Axum HTTP runtime service
- **[ipe-parser](../crates/ipe-parser/README.md)** - Placeholder crate
- **[ipe-control](../crates/ipe-control/)** - Control plane placeholder (gRPC)
- **[ipe-web](../crates/ipe-web/)** - Web application placeholder
- **[ipe-wasm](../crates/ipe-wasm/)** - WebAssembly placeholder
- **[ipe-ffi](../crates/ipe-ffi/)** - C FFI placeholder

## Development

**Contributing and building:**

- **[CONTRIBUTING](../CONTRIBUTING.md)** - Contribution guidelines
- **[MVP_PLAN](../MVP_PLAN.md)** - MVP scope and checklist (historical)

## API Documentation

**Generated API docs:**

```bash
# Generate and open Rust API docs
cargo doc --all-features --no-deps --open
```

## Quick Reference

**Common tasks:**

| Task | Command |
|------|---------|
| Build project | `cargo build --release` |
| Run tests | `cargo test --all-features` |
| Check formatting | `cargo fmt --all -- --check` |
| Run linter | `cargo clippy --all-targets` |
| Generate coverage | `cargo llvm-cov --all-features --html` |
| Build docs | `cargo doc --all-features --open` |

## Documentation Standards

All documentation follows these principles:

1. **Start with "why"** - Explain motivation before solution
2. **Show, don't tell** - Use examples and diagrams
3. **Progressive disclosure** - Summary → Detail → Reference
4. **Keep it current** - Update docs with code changes
5. **Be concise** - Respect reader's time

## Getting Help

- **GitHub Issues:** https://github.com/jrepp/ipe/issues
- **Discussions:** https://github.com/jrepp/ipe/discussions
- **Email:** [Your contact]

## Documentation Structure

```
ipe/
├── README.md              # Project overview and entry point
├── QUICKSTART.md          # Getting started guide
├── SUMMARY.md             # Executive summary (historical)
├── END_TO_END_EXAMPLE.md  # Legacy engine walkthrough (historical)
├── ROADMAP.md             # Development roadmap
├── MVP_PLAN.md            # MVP scope (historical)
│
├── docs/                  # Technical documentation
│   ├── INDEX.md          # This file
│   ├── RUNTIME_DATA.md   # v2 runtime workflow and API (current)
│   ├── ARCHITECTURE.md   # v2 architecture
│   ├── AST.md            # Legacy AST spec (historical)
│   └── BYTECODE.md       # Legacy bytecode reference (historical)
│
├── rfcs/                 # Design proposals
│   ├── INDEX.md         # RFC navigation
│   ├── 001-*.md         # Individual RFCs
│   └── ...
│
└── crates/              # Per-crate code
    └── */               # ipe-core, ipe-server, placeholders
```

---

**Last updated:** 2025-10-27
