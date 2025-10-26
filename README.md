# Intent Policy Engine (IPE)

**A high-performance, AI-native policy engine built in Rust**

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Overview

Intent Policy Engine (IPE) is a declarative policy language and evaluation engine designed for DevOps/SecOps workflows. It combines human readability with extreme performance through:

- **Natural language intent** + structured logic
- **Bytecode compilation** with runtime JIT optimization
- **<50μs p99 latency** for 10k+ policies
- **Zero-downtime updates** via atomic policy swapping
- **AI-native** semantic layer for queries and generation

## Key Features

### 🚀 Performance
- Bytecode interpretation: ~50μs per policy
- JIT compilation (Cranelift): ~10μs per policy (5-10x faster)
- Adaptive tiering: automatic optimization of hot policies
- Zero-copy evaluation with arena allocation
- Memory-mapped policy storage

### 📝 Developer Experience
- Natural language intent as first-class documentation
- Visual policy editor with real-time validation
- SQL/Go-like syntax (no YAML indentation hell)
- Git-friendly diffs
- Comprehensive error messages

### 🤖 AI Integration
- Bidirectional translation (natural language ↔ policy code)
- Semantic queries over policy corpus
- Conflict detection and resolution
- Policy effectiveness analytics

### 🔧 Embeddable
- Native libs (C FFI)
- Python bindings (PyO3)
- Node.js bindings (napi-rs)
- WebAssembly (browser + server)
- <2MB binary footprint

## Quick Start

### Example Policy

```rust
policy RequireApproval:
  "Production deployments need 2+ approvals from senior engineers"
  
  triggers when
    resource.type == "Deployment"
    and environment in ["production", "staging"]
    and resource.risk_level >= "medium"
  
  requires
    approvals.count >= 2
    where approver.role == "senior-engineer"
    and approver.department != requester.department
```

### Rust Usage

```rust
use ipe_core::{PolicyEngine, EvaluationContext};

// Load compiled policies
let engine = PolicyEngine::from_file("policies.ipe")?;

// Create evaluation context
let ctx = EvaluationContext {
    resource: /* ... */,
    action: /* ... */,
    request: /* ... */,
};

// Evaluate (with automatic JIT optimization)
let decision = engine.evaluate(&ctx)?;

if decision.kind == DecisionKind::Allow {
    // Proceed with action
}
```

### JIT Compilation Demo

```bash
cd ipe-rfc
cargo run --example jit_demo --features jit --release
```

Expected output:
```
Phase 1: Interpreter Mode (Cold Start)
  Average: 47.32μs

Phase 2: Triggering JIT Compilation
  Waiting for JIT compilation...

Phase 3: JIT Mode (Hot Path)
  Average: 8.15μs
  Speedup: 5.8x
```

## Architecture

```
┌──────────────┐
│ Source (.ipe)│  Natural language + structured logic
└──────┬───────┘
       │ Compile
┌──────▼────────┐
│ Bytecode      │  Compact representation (~200 bytes/policy)
└──────┬────────┘
       │ Evaluate
┌──────▼────────┐
│ Interpreter   │  ~50μs per policy
└──────┬────────┘
       │ Profile (100+ evals)
┌──────▼────────┐
│ JIT (Cranelift│  ~10μs per policy (5-10x faster)
└───────────────┘
```

### Tiered Execution

| Tier | When | Compile Time | Eval Latency |
|------|------|--------------|--------------|
| Interpreter | Default | 0 (pre-compiled) | ~50μs |
| Baseline JIT | >100 evals | ~500μs | ~10μs |
| Optimized JIT | >10k evals | ~5ms | ~5μs |

## Project Structure

```
ipe-rfc/
├── RFC.md                    # Complete technical specification
├── Cargo.toml                # Workspace configuration
├── crates/
│   ├── ipe-core/             # Core engine + JIT
│   ├── ipe-parser/           # Language parser
│   ├── ipe-control/          # gRPC control plane
│   ├── ipe-wasm/             # WebAssembly bindings
│   ├── ipe-ffi/              # C FFI
│   └── ipe-web/              # Web application
└── examples/
    └── jit_demo.rs           # JIT compilation demo
```

## Building

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies (Ubuntu/Debian)
sudo apt-get install build-essential pkg-config

# Install dependencies (macOS)
brew install pkg-config
```

### Build

```bash
# Debug build
cargo build

# Release build with JIT
cargo build --release --features jit

# WebAssembly
cargo build --target wasm32-unknown-unknown --features jit

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench
```

## Roadmap

- **Phase 1-2 (Months 1-4):** Core engine + bytecode compilation
- **Phase 3 (Months 5-6):** JIT compilation with Cranelift
- **Phase 4 (Months 7-8):** gRPC control plane + atomic updates
- **Phase 5 (Months 9-10):** WASM + language bindings
- **Phase 6 (Months 11-12):** Web application
- **Phase 7 (Months 13-14):** AI integration
- **Phase 8 (Ongoing):** Production hardening

See [RFC.md](RFC.md) for detailed roadmap and milestones.

## Performance Targets

| Metric | Target | Achieved |
|--------|--------|----------|
| Single policy eval (interpreter) | <50μs | TBD |
| Single policy eval (JIT) | <10μs | TBD |
| 10k policies (indexed) | <100μs | TBD |
| Policy compilation | <10ms | TBD |
| Atomic policy swap | <5μs | TBD |
| Binary size | <2MB | TBD |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- Inspired by Cedar, Rego/OPA, and other policy languages
- Powered by Cranelift JIT compiler
- Built with Rust for safety and performance

---

**Status:** RFC / Prototype Phase  
**Version:** 0.1.0  
**Contact:** [Your contact info]
