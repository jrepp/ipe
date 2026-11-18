# IPE Documentation Index

**Welcome to the Idempotent Predicate Engine documentation!**

## Releases

- [Release guide](RELEASES.md): generated notes, labels, draft review, and publication.
- [Changelog](../CHANGELOG.md): GitHub Releases and archived history.

## Product and project organization

- [Mission and product principles](../MISSION.md): current positioning and implemented boundaries.
- [Project map](../docs-cms/PROJECTS.md): subway routes, dependencies, delivery states, and five project PRDs.
- [Merge queue](MERGE_QUEUE.md): speculative batching, required checks, and activation limits.

## Design documentation

The [design index](../docs-cms/README.md) contains structured requirements, proposals, and decisions.
Existing guides and RFCs retain their paths. See [ADR 001](../docs-cms/adr/adr-001-hybrid-documentation.md) for the hybrid structure.

- [PRD 001](../docs-cms/prd/prd-001-evidence-system-map.md): system map, evidence suppliers and consumers, temporal controls, exceptions, and access grants.
- [RFC 018](../docs-cms/rfcs/rfc-018-evidence-system-map.md): proposed design and verification coverage for those requests.

Both feature documents are Draft. Run `make docs-cms-check` to validate structured documents.

[Repository map](REPOSITORY.md) and [cleanup design](../docs-cms/rfcs/rfc-019-cohesive-monorepo.md) describe component ownership, contributor commands, and remaining audit work.

## Local Documentation Site

Run `make docs-serve`, then open <http://127.0.0.1:8000/docs/>.
The target builds and serves `target/docs-site`, not the checkout.
The build uses the Python version in `.python-version` through uv and copies the static site and archived measurement data.
It does not compile Rust or generate new runtime measurements.

```sh
npm ci --prefix docs --ignore-scripts
make docs-check
make docs-cms-check
```

The site checks cover the project map, Markdown viewer, archived explorers, artifact links, and HTTP paths under `/ipe/`.
Docuchango checks structured design records and requirement traceability.
The overview map is static SVG with text links. The Markdown viewer has raw-source fallback when its rendering libraries are unavailable.

The [site and publishing guide](README.md) describes the archive, source revision, and deployment process.
The [Docs site workflow](../.github/workflows/docs-site.yml) builds review artifacts on PRs.
Publication from main requires successful site, design-document, and workflow validation jobs.
PR builds do not deploy the production site.
The public site is [jrepp.github.io/ipe](https://jrepp.github.io/ipe/).

### Archived references

- [AST explorer](ast.html): parsed trees from the removed legacy engine.
- [Contract summary](contract.html): v2 runtime inputs, decisions, application responsibilities, and limits.
- [Performance dashboard](performance.html): archived interpreter measurements, not current runtime capacity.

The frozen `docs/data/site-data.json` records the original run time, platform, Rust version, and source revision.
Rebuilding the site does not refresh those measurements. Current production performance is not characterized.

This index provides a guided path through IPE's documentation based on your needs.

## Getting Started

**New to IPE?** Start here:

- **[Runtime data workflow](RUNTIME_DATA.md)** — implemented authenticated push/pull, TTL, language-2 policies, deployment limits, and external HTTP acceptance tests.
- **[Integration discovery](INTEGRATION_DISCOVERY.md)** — standalone Integration Workload, CLI/MCP access, credential-first metadata discovery, and a local HTTPS stand-in deployment.
- **[Integration stack and crate reuse](INTEGRATION_STACK.md)** — implemented dependencies, recommended MCP stack, bespoke-code audit and migration priorities.

1. **[README](../README.md)** - Project overview, badges, and quick links
2. **[QUICKSTART](../QUICKSTART.md)** - Get IPE running in 5 minutes
3. **[END_TO_END_EXAMPLE](../END_TO_END_EXAMPLE.md)** - Legacy engine lifecycle walkthrough (historical)

## Understanding IPE

**Learn the architecture and design:**

- **[SUMMARY](../SUMMARY.md)** - Executive summary and vision (historical)
- **[ARCHITECTURE](ARCHITECTURE.md)** - v2 runtime architecture with diagrams
- **[Admin control plane architecture review](ADMIN_ARCHITECTURE_REVIEW.md)** - Central policy resource lifecycles, publication authority, endpoint registry, push-accelerated pull, observability, protocol tradeoffs, and [design critique](ADMIN_ARCHITECTURE_CRITIQUE.md).
- **[ROADMAP](../ROADMAP.md)** - Development phases and milestones

## Technical Reference

**Deep dives into IPE internals:**

- **[Project vocabulary](VOCABULARY.md)** - Preferred terms, domain boundaries, and open naming decisions.
- **[Vocabulary review](VOCABULARY_REVIEW.md)** - Naming recommendations, source evidence, and the repository update plan.
- **[Integration domain model review](INTEGRATION_DOMAIN_MODEL.md)** - Operations, requirements, controls, evidence, and proposed assessment semantics.
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
- **[RFC-010](../rfcs/010-remediation-adapters-and-external-policy-discovery.md):** Integration architecture, ServiceNow discovery-first MVP, and roadmap across focused area RFCs (draft discovery)
- **[RFC-011](../rfcs/011-agent-integration-toolkit.md):** Agent lifecycle; [012](../rfcs/012-integration-runtime-and-discovery.md) runtime/discovery; [013](../rfcs/013-domain-constraints-and-projections.md) domain modeling; [014](../rfcs/014-fact-publication-and-synchronization.md) synchronization; [015](../rfcs/015-policy-access-and-identities.md) access; [016](../rfcs/016-integration-signals-and-audit.md) signals/audit. Each owns its contract and milestone roadmap (draft discovery).

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

- **[Workstream register](../.agent/workstreams.md)** - Current work, committed checkpoints, merged branches, and next actions.
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

### Proposed fact retrieval contract

[RFC 0003: Fact Retrieval API Contract](rfcs/0003-fact-retrieval-api.md) proposes
bounded current-state collection listing. It remains a draft with open decisions;
no collection-listing route is implemented by this document.

- [Hosted integration programs](INTEGRATION_HOST.md): installed JavaScript, external requirements, status facts, and performance history.
- [Contextual evidence assessments](INTEGRATION_ASSESSMENTS.md): explicit links across collectors, execution context, trust rules, and evidence traces.

- [Integration model review](INTEGRATION_MODEL_REVIEW.md): evidence-backed recommendations for collection, contract identity, reasoning, and updates.

- [Product ship qualification simulation](PRODUCT_SHIP_DEMO.md): linked policy model, headless SDK collectors, and three UX studies.

- [Enterprise scale planning](ENTERPRISE_SCALE_PLANNING.md): quarterly volume, full agent jobs, and assumed savings ranges.
