# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
- Bytecode storage uses bincode-next with the existing version 1 encoding.
- Bytecode encode and decode methods return `bytecode::SerializationError` instead of `bincode::Error`.
- Cranelift uses version 0.123, which removes the unmaintained `mach` dependency.
- Updated GitHub Actions workflows to use wrapper scripts for better error handling
- Improved sanitizer checks to handle multiline output correctly

### Fixed
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
