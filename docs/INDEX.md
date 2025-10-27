# IPE Documentation Index

**Welcome to the Intent Policy Engine documentation!**

This index provides a guided path through IPE's documentation based on your needs.

## Getting Started

**New to IPE?** Start here:

1. **[README](../README.md)** - Project overview, badges, and quick links
2. **[QUICKSTART](../QUICKSTART.md)** - Get IPE running in 5 minutes
3. **[END_TO_END_EXAMPLE](../END_TO_END_EXAMPLE.md)** - Complete walkthrough of policy lifecycle

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
- **RFC-003:** Policy tree storage system

## Crate Documentation

**Per-crate technical docs:**

- **[ipe-core](../crates/ipe-core/)** - Core engine and interpreter
- **[ipe-parser](../crates/ipe-parser/README.md)** - Policy language parser
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
