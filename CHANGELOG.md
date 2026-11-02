# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Integration monitor groups inspection into Runtime and Integration coverage. Detail navigation follows the selected journey; disconnected sessions show connection instructions.
- GitHub collector extracts bounded environment protections and repository rulesets through granted API reads, including reviewer principals, approval handoffs, and provider profile contacts. Missing grants, denied reads, unresolved environments, and truncated samples remain explicit gaps.
- SDK 0.7 and native model v2 support source-backed governance descriptions: policies, contracts, controls, handoffs, principals, role assignments, and contact methods. Existing v1 records remain supported.
- Governance inspection separates API extraction evidence from external execution links, surfaces missing or candidate references, and shows human handoff responsibilities and sourced contacts. Role assignments and contacts have bounded expiry; discovery does not issue grants or send messages.
- Integration domain design now centers external governance and specifies human handoff policies, principal roles, and sourced contact methods. The first description schema is implemented; handoff events remain proposed.
- Operation-focused D3 view shows one operation, observed execution, and supporting facts. It marks candidate and missing links, retains bounded emission steps, and works in offline snapshots.
- Integration monitor uses operation and named-requirement terms, separates evidence checks from fulfillment, and pins live checks through the v2 domain APIs.
- SDK 0.6 adds operation and named-requirement types with v1 compatibility aliases and stable identity encoding.
- Versioned integration domain APIs expose selected operation descriptions and bind evidence checks to exact mapped record revisions. Evidence matches remain separate from requirement fulfillment and authorization.
- Integration domain model review separates operation declarations, normative duties, control mechanisms, and evidence assessments; remaining lifecycle extensions are marked as proposed.
- Project vocabulary baseline and research-backed review, linked from the documentation site, with domain boundaries, naming rules, and unresolved term drift.
- Immutable integration review catalogs with semantic diffs, explicit acceptance, and restoration without fact renewal.
- Shared human and agent operation explanations with prerequisite evidence, unknowns, granted next observations, and candidate outcome links.
- Hosted integration coverage verifies complete declaration scopes before removing absent constraints. Failed reads preserve requirements and native diagnostics.
- Versioned integration models separate definitions, evidence, constraints, invocations, facts, and outcomes. The host validates schemas and derives control, revision, call-site, and execution identities.
- External execution archive retains outcomes independently of collector log eviction, with bounded change history and an authenticated export API.
- Private local monitor launcher establishes a browser session without token copying. The monitor shows installed JavaScript, input, grants, schedule, and live records.
- Integration host resolves bearer tokens through local Git credential helpers with bounded, noninteractive lookup. The hosted GitHub demo no longer needs a token environment variable.
- Local integration endpoint hosts installed JavaScript with shared connections, native secret handling, schedules, retries, and durable bounded history.
- Hosted GitHub example captures requirements, expiring status facts, and distinct external outcomes with job and step durations. Includes agent JSON and a human monitor.
- Local GitHub workflow and action contract capture with pinned source evidence, sandboxed SDK projections, input bindings, and a standalone review report.
- Typed sandbox SDK helpers for brokered requests, response handling, JSON state and projections; build-side npm artifact caching with pinned SHA-512 integrity and offline replay.
- Integration service JavaScript/TypeScript step adapters with bounded QuickJS execution, startup TypeScript transpilation, operator-allowlisted HTTPS reads, and CLI/MCP access.
- Core agent prompt and workflow guides for consistent development practices
- Requirements specification document (REQUIREMENTS.md) with 100+ requirements
- Interactive bytecode statistics info card in live demo
- Comprehensive CI/CD pipeline with security audits

### Changed
- Integration coverage explains a rule, its linked operations, and reported executions before showing record types. Connection evidence and missing requirement checks stay explicit.
- Integration coverage uses a selectable policy ontology map with one detail panel and a gap filter. Candidate references, expired evidence, and unknown applicability stay explicit.
- Governance review focuses on one requirement, presents retained configuration fields, and marks missing requirement-to-fact bindings. Saved collector settings are labeled as historical snapshots.
- Governance review now explains source-to-ontology mappings, candidate execution links, exact fact emission steps, and missing evidence. Current collector settings remain separate from external policy configuration.
- Bytecode storage uses bincode-next with the existing version 1 encoding.
- Bytecode encode and decode methods return `bytecode::SerializationError` instead of `bincode::Error`.
- Cranelift uses version 0.123, which removes the unmaintained `mach` dependency.
- Updated GitHub Actions workflows to use wrapper scripts for better error handling
- Improved sanitizer checks to handle multiline output correctly

### Fixed
- Ontology navigation uses consecutive heading levels and a named group that assistive tools can identify.
- Monitor sessions now cover v1 and v2 APIs. Refreshing the monitor upgrades existing v1-only cookies without relaxing origin or authentication checks.
- Documentation previews include new top-level Markdown guides before staging. Site checks verify Markdown viewer targets in the packaged artifact.
- Hosted declaration projection avoids repeated internal JSON copies while keeping SDK and native validation at the input and output boundaries.
- Updated `libz-sys` to 1.1.31 for RUSTSEC-2026-0418.
- Sanitizer checks use optimized Rust and native builds with the existing instrumentation to preserve bounded guest execution on hosted runners.
- Hosted GitHub extraction models job constraints in separate steps to stay within the unchanged guest deadline.
- Integration host state writes no longer attempt to open a directory as a file on Windows. State files remain synced before replacement.
- Monitor refresh preserves open evidence panels, source inspection, and keyboard focus.
- Updated TLS certificate verification and root data for RUSTSEC-2026-0382 and RUSTSEC-2026-0383.
- Dependency advisories: removed unused gRPC, LRU, and memory-map dependencies and updated affected locked versions.
- Security checks cover all features and retain audit reports after failures.
- Cargo deny failures due to missing license entries
- Supply chain security checks for virtual manifests
- Benchmark action parsing issues with colored output

## [0.1.0] - 2025-01-26

### Added
- Initial project structure with Cargo workspace
- Core predicate engine (ipe-core) with bytecode interpreter
- Language parser (ipe-parser) with AST generation
- JIT compilation support via Cranelift
- Lock-free predicate data store with atomic updates
- Multi-tier execution (interpreter → baseline JIT → optimized JIT)
- Control plane with gRPC API (ipe-control)
- WebAssembly bindings (ipe-wasm)
- C FFI bindings (ipe-ffi)
- Web interface with interactive demo (ipe-web)
- Comprehensive test suite (248 tests, 93.67% coverage)
- Performance benchmarks with criterion
- Security audits (cargo-deny, cargo-audit, cargo-geiger)
- CI/CD with GitHub Actions
- Documentation (Architecture, AST, Bytecode specs)
- RFC system for design proposals

### Performance
- Interpreter: p99 < 50µs per evaluation
- Baseline JIT: p99 < 20µs per evaluation
- Optimized JIT: p99 < 10µs per evaluation
- Throughput: 100K+ evaluations/second per core

### Security
- Memory-safe Rust implementation
- No unsafe code in workspace crates
- Pass all sanitizers (address, leak, thread)
- Supply chain audits passing

[Unreleased]: https://github.com/jrepp/ipe/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jrepp/ipe/releases/tag/v0.1.0
