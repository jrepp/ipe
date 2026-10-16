# IPE Documentation Index

**Welcome to the Idempotent Predicate Engine documentation!**

## Local Documentation Site

Run `make docs-site` to export three real Rust parser trees and fresh interpreter
microbenchmarks, then `make docs-serve` and open <http://127.0.0.1:8000/docs/>.
The build requires Rust and [uv](https://docs.astral.sh/uv/). Python is pinned to
**3.15.0** in `.python-version` (the stable release published October 9, 2026).
Both local Make targets and CI read that file; the builder rejects other versions.
The first build downloads Cargo dependencies. If an older uv cannot install the
pin, update uv or use `uvx --from uv uv python install 3.15.0` once.

- [AST explorer](ast.html): rendered, expandable trees with source and node details.
- [Performance dashboard](performance.html): measured latency percentiles, throughput,
  outliers, downloadable results, and requirement coverage.

Generated data lives in `docs/data/site-data.json` and records the run time, platform,
Rust version, and Git revision. Run `make docs-site` again after code changes.
The dashboard measures hand-built bytecode through the real interpreter; it excludes
parsing, compilation, context construction, and JIT. These are local microbenchmarks,
not production performance validation. The existing Criterion evaluation suite has
placeholder paths and is not the dashboard's data source.

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

Fresh measurements are generated in CI and kept in the artifact, not committed.
Shared-runner timings are reported as evidence and never used as pass/fail
performance thresholds. JIT, end-to-end policy compilation, and production load
remain outside the measurement scope. Dependency updates are proposed monthly
by Dependabot; update `.python-version` after checking a new stable Python release
and rerun `make docs-check`.

This index provides a guided path through IPE's documentation based on your needs.

## Getting Started

**New to IPE?** Start here:

1. **[README](../README.md)** - Project overview, badges, and quick links
2. **[QUICKSTART](../QUICKSTART.md)** - Get IPE running in 5 minutes
3. **[END_TO_END_EXAMPLE](../END_TO_END_EXAMPLE.md)** - Complete walkthrough of predicate lifecycle

## Understanding IPE

**Learn the architecture and design:**

- **[SUMMARY](../SUMMARY.md)** - Executive summary and vision (5 min read)
- **[ARCHITECTURE](ARCHITECTURE.md)** - System architecture with diagrams (15 min read)
- **[ROADMAP](../ROADMAP.md)** - Development phases and milestones

## Technical Reference

**Deep dives into IPE internals:**

- **[AST](AST.md)** - Abstract syntax tree specification
- **[BYTECODE](BYTECODE.md)** - Bytecode instruction set and execution model
- **[RFC Collection](../rfcs/INDEX.md)** - Design proposals and specifications

## RFCs (Request for Comments)

**Design documents for major features:**

See **[rfcs/INDEX.md](../rfcs/INDEX.md)** for the complete RFC collection, including:

- **RFC-001:** Sidecar service architecture
- **RFC-002:** SSE/JSON protocol specification
- **RFC-003:** Predicate tree storage system

## Crate Documentation

**Per-crate technical docs:**

- **[ipe-core](../crates/ipe-core/)** - Core engine and interpreter
- **[ipe-parser](../crates/ipe-parser/README.md)** - Predicate language parser
- **[ipe-control](../crates/ipe-control/)** - Control plane (gRPC)
- **[ipe-web](../crates/ipe-web/)** - Web application
- **[ipe-wasm](../crates/ipe-wasm/)** - WebAssembly bindings
- **[ipe-ffi](../crates/ipe-ffi/)** - C FFI and language bindings

## Development

**Contributing and building:**

- **[CONTRIBUTING](../CONTRIBUTING.md)** - Contribution guidelines
- **[MVP_PLAN](../MVP_PLAN.md)** - MVP scope and checklist
- **[Archive](archive/)** - Historical development docs

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
├── QUICKSTART.md          # 5-minute getting started guide
├── SUMMARY.md             # Executive summary and vision
├── END_TO_END_EXAMPLE.md  # Complete usage walkthrough
├── ROADMAP.md             # Development roadmap
├── MVP_PLAN.md            # MVP scope and progress
│
├── docs/                  # Technical documentation
│   ├── INDEX.md          # This file
│   ├── ARCHITECTURE.md   # System architecture
│   ├── AST.md           # AST specification
│   ├── BYTECODE.md      # Bytecode reference
│   └── archive/         # Historical/dev docs
│
├── rfcs/                 # Design proposals
│   ├── INDEX.md         # RFC navigation
│   ├── 000-overview.md  # RFC process
│   ├── 001-*.md         # Individual RFCs
│   └── ...
│
└── crates/              # Per-crate documentation
    └── */README.md      # Crate-specific docs
```

---

**Last updated:** 2025-10-27
