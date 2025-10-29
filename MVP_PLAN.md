# Idempotent Predicate Engine: MVP Implementation Plan

## Executive Summary

This document outlines the **Minimum Viable Product (MVP)** plan for the Idempotent Predicate Engine, a security-critical predicate evaluation engine built in Rust. Our approach prioritizes:

1. **Security First**: Comprehensive testing, fuzzing, and code coverage
2. **Performance Validation**: Early benchmarking to validate <50μs targets
3. **Production Readiness**: Linting, CI/CD, and quality gates from day one
4. **Incremental Delivery**: Working software at each phase

## MVP Scope

### What's IN the MVP

**Core Functionality:**
- ✅ Full predicate parsing (nom-based parser)
- ✅ Type-checked AST with semantic validation
- ✅ Bytecode compiler with optimizations
- ✅ Bytecode interpreter (50μs target)
- ✅ Basic predicate indexing (resource type)
- ✅ RAR (Resource-Action-Request) evaluation model
- ✅ Comprehensive error handling

**Testing & Quality:**
- ✅ Unit tests (>90% code coverage)
- ✅ Integration tests
- ✅ Fuzzing infrastructure
- ✅ Benchmark suite with regression detection
- ✅ Load tests (1M+ evals/sec target)

**Developer Experience:**
- ✅ Clippy linting (strict mode)
- ✅ rustfmt formatting
- ✅ GitHub Actions CI/CD
- ✅ Automated security audits
- ✅ Documentation (rustdoc)

**Example Predicates:**
- ✅ 20+ real-world predicate examples
- ✅ Test corpus for validation

### What's OUT of MVP (Post-MVP Phases)

- ⏭️ JIT compilation (Phase 2)
- ⏭️ gRPC control plane (Phase 3)
- ⏭️ Web application (Phase 4)
- ⏭️ Language bindings (Phase 5)
- ⏭️ AI integration (Phase 6)

## MVP Timeline: 8-10 Weeks

### Week 1-2: Foundation & Parser
**Goal:** Parse predicate source into typed AST

**Deliverables:**
- [ ] Lexer with full token support
- [ ] nom-based parser for predicate syntax
- [ ] AST node definitions
- [ ] Type system implementation
- [ ] Parser tests (100+ cases)
- [ ] Error recovery and reporting

**Success Criteria:**
- Parse all RFC examples correctly
- Helpful error messages with line/column info
- <10ms parsing for 1000-line predicate file

### Week 3-4: Bytecode Compiler
**Goal:** Compile AST to optimized bytecode

**Deliverables:**
- [ ] Bytecode instruction set (extend existing)
- [ ] AST → Bytecode code generation
- [ ] Constant pooling
- [ ] Basic optimizations (constant folding, dead code elimination)
- [ ] Bytecode serialization/deserialization
- [ ] Compiler tests (>85% coverage)

**Success Criteria:**
- Compile 1000 predicates in <5 seconds
- Bytecode size ~200 bytes per predicate
- Deterministic compilation (same input = same output)

### Week 5-6: Interpreter & Engine
**Goal:** Evaluate bytecode with <50μs latency

**Deliverables:**
- [ ] Stack-based bytecode interpreter
- [ ] RAR context evaluation
- [ ] Predicate indexing by resource type
- [ ] Decision resolution (allow/deny)
- [ ] Memory arena for zero-copy evaluation
- [ ] Engine tests (>90% coverage)

**Success Criteria:**
- Single predicate evaluation: <50μs p99
- 1000 predicates with indexing: <500μs p99
- Zero heap allocations during evaluation
- Thread-safe, concurrent evaluation

### Week 7-8: Testing & Benchmarks
**Goal:** Comprehensive test coverage and performance validation

**Deliverables:**
- [ ] Fuzzing harness (cargo-fuzz)
- [ ] Property-based tests (proptest)
- [ ] Criterion benchmarks
- [ ] Load test framework
- [ ] Stress tests (1M+ predicates)
- [ ] Memory leak detection (valgrind)

**Success Criteria:**
- Code coverage: >90% overall, 100% for critical paths
- Zero memory leaks
- Zero panics under fuzzing (100M+ iterations)
- Performance targets validated

### Week 9-10: CI/CD & Documentation
**Goal:** Production-ready infrastructure

**Deliverables:**
- [ ] GitHub Actions workflows
- [ ] Automated security audits
- [ ] Release automation
- [ ] Comprehensive rustdoc
- [ ] User guide with examples
- [ ] Performance dashboard

**Success Criteria:**
- CI completes in <10 minutes
- All quality gates automated
- Documentation published
- Binary artifacts built for Linux/macOS/Windows

## MVP Success Metrics

### Performance (Must-Have)
| Metric | Target | Test Method |
|--------|--------|-------------|
| Single predicate eval (cold) | <50μs p99 | Criterion bench |
| 1000 predicates (indexed) | <500μs p99 | Custom load test |
| Predicate compilation | <10ms per predicate | Criterion bench |
| Memory per predicate | <300 bytes | Memory profiler |
| Throughput (single-thread) | >20k ops/sec | Load test |

### Quality (Must-Have)
| Metric | Target | Test Method |
|--------|--------|-------------|
| Code coverage | >90% | llvm-cov |
| Critical path coverage | 100% | llvm-cov + manual |
| Fuzzing stability | 100M+ iterations | cargo-fuzz |
| Security audit | 0 high/critical | cargo-audit |
| Clippy warnings | 0 | CI |
| Memory leaks | 0 | valgrind/miri |

### Reliability (Must-Have)
- No panics in safe code
- Graceful error handling (no unwrap in production paths)
- Deterministic behavior (no timing dependencies)
- Thread-safe concurrent evaluation

## Technical Architecture

### Module Structure

```
ipe/
├── crates/
│   ├── ipe-core/           # Core engine (MVP focus)
│   │   ├── src/
│   │   │   ├── lib.rs      # Public API
│   │   │   ├── parser/     # Lexer + Parser
│   │   │   │   ├── mod.rs
│   │   │   │   ├── lexer.rs
│   │   │   │   ├── parser.rs
│   │   │   │   └── error.rs
│   │   │   ├── ast/        # AST + Type System
│   │   │   │   ├── mod.rs
│   │   │   │   ├── nodes.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── visitor.rs
│   │   │   ├── compiler/   # Bytecode Compiler
│   │   │   │   ├── mod.rs
│   │   │   │   ├── codegen.rs
│   │   │   │   ├── optimizer.rs
│   │   │   │   └── constant_pool.rs
│   │   │   ├── bytecode/   # Bytecode Definition
│   │   │   │   ├── mod.rs
│   │   │   │   ├── instruction.rs
│   │   │   │   └── serialize.rs
│   │   │   ├── interpreter/# Bytecode Interpreter
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vm.rs
│   │   │   │   └── stack.rs
│   │   │   ├── engine/     # Predicate Engine
│   │   │   │   ├── mod.rs
│   │   │   │   ├── predicate_db.rs
│   │   │   │   ├── index.rs
│   │   │   │   └── decision.rs
│   │   │   ├── rar/        # RAR Model
│   │   │   │   ├── mod.rs
│   │   │   │   ├── context.rs
│   │   │   │   └── value.rs
│   │   │   └── error.rs    # Error Types
│   │   ├── tests/          # Integration Tests
│   │   │   ├── parser_tests.rs
│   │   │   ├── compiler_tests.rs
│   │   │   ├── interpreter_tests.rs
│   │   │   └── end_to_end_tests.rs
│   │   ├── benches/        # Benchmarks
│   │   │   ├── parsing.rs
│   │   │   ├── compilation.rs
│   │   │   └── evaluation.rs
│   │   └── fuzz/           # Fuzz Targets
│   │       └── fuzz_targets/
│   │           ├── parse_predicate.rs
│   │           └── evaluate_predicate.rs
│   │
│   └── ipe-cli/            # CLI Tool (for testing)
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
│
├── examples/               # Example Predicates
│   ├── deployment.ipe
│   ├── security.ipe
│   └── compliance.ipe
│
├── .github/
│   └── workflows/
│       ├── ci.yml          # Main CI
│       ├── security.yml    # Security Audit
│       ├── coverage.yml    # Code Coverage
│       └── bench.yml       # Performance Regression
│
├── Cargo.toml              # Workspace Config
├── rustfmt.toml            # Formatting Config
├── clippy.toml             # Linting Config
├── deny.toml               # Dependency Policy
└── codecov.yml             # Coverage Config
```

### Data Flow

```
┌──────────────┐
│ Predicate Source│  (.ipe files)
└──────┬───────┘
       │ parse
       ▼
┌──────────────┐
│ AST + Types  │  (in-memory, type-checked)
└──────┬───────┘
       │ compile
       ▼
┌──────────────┐
│ Bytecode     │  (serializable, ~200 bytes/predicate)
└──────┬───────┘
       │ load into engine
       ▼
┌──────────────┐
│ PredicateDB  │  (indexed by resource type)
└──────┬───────┘
       │ evaluate (with RAR context)
       ▼
┌──────────────┐
│ Decision     │  (Allow/Deny + metadata)
└──────────────┘
```

## Test Strategy

### 1. Unit Tests (>90% coverage target)

**Per-module testing:**
- **Parser:** Token stream validation, syntax error recovery
- **Compiler:** Bytecode correctness, optimization verification
- **Interpreter:** Instruction execution, stack operations
- **Engine:** Predicate matching, decision resolution

**Critical path testing (100% coverage):**
- Predicate evaluation core loop
- Memory management (arena allocation)
- Error handling paths

### 2. Integration Tests

**End-to-end scenarios:**
- Parse → Compile → Evaluate full predicates
- Multiple predicates with conflicts
- Edge cases (empty predicates, large predicates)
- Concurrent evaluation

### 3. Property-Based Tests (proptest)

**Invariants to verify:**
- Parsing is deterministic: `parse(str) == parse(str)`
- Compilation is deterministic: `compile(ast) == compile(ast)`
- Evaluation is pure: `eval(ctx) == eval(ctx)`
- No panics on any valid input

**Generators:**
- Random valid ASTs
- Random bytecode sequences
- Random RAR contexts

### 4. Fuzzing (cargo-fuzz)

**Fuzz targets:**
- **Parse:** Random strings → Parser (should never panic)
- **Compile:** Random ASTs → Compiler (should never panic)
- **Evaluate:** Random bytecode + contexts → Interpreter (should never panic)

**Fuzzing goals:**
- 100M+ iterations without panics
- Memory sanitizer enabled
- Address sanitizer enabled

### 5. Load Testing

**Scenarios:**
- **Throughput test:** 1M evaluations, measure ops/sec
- **Latency test:** P50/P99/P99.9 latencies under load
- **Stress test:** 100k predicates loaded, evaluate randomly
- **Concurrent test:** 8 threads evaluating simultaneously

**Tools:**
- Custom load test harness
- Criterion for microbenchmarks
- Flamegraph for profiling

### 6. Security Testing

**Static analysis:**
- `cargo-audit`: Known vulnerability scanning
- `cargo-deny`: License and dependency policy
- `clippy`: Security-focused lints

**Dynamic analysis:**
- `valgrind`: Memory leak detection
- `miri`: Undefined behavior detection
- Address sanitizer (ASAN)
- Thread sanitizer (TSAN)

## Development Workflow

### Local Development

```bash
# Format code
cargo fmt

# Lint code (strict mode)
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features

# Run with coverage
cargo llvm-cov --all-features --html

# Run benchmarks
cargo bench

# Fuzz (10 minutes)
cargo +nightly fuzz run parse_predicate -- -max_total_time=600
```

### Pre-Commit Checks

```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

echo "Running pre-commit checks..."

# Format
cargo fmt --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --all-features --quiet

# Audit
cargo audit

echo "✅ All checks passed!"
```

### CI/CD Pipeline

**On every push:**
1. Format check (`cargo fmt --check`)
2. Clippy lint (`cargo clippy`)
3. Tests (`cargo test --all-features`)
4. Build all targets (Linux, macOS, Windows)

**On PR:**
- All of the above, plus:
- Code coverage check (must be >90%)
- Benchmark regression check
- Security audit

**On main branch:**
- All of the above, plus:
- Extended fuzzing (1 hour)
- Memory leak detection
- Publish docs to GitHub Pages

**On release tag:**
- Build release binaries
- Run full test suite
- Publish to crates.io
- Create GitHub release

## Risk Management

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance targets not met | High | Early benchmarking (week 1), profiling, optimization sprints |
| Parser complexity | Medium | Use proven nom library, extensive testing, fuzzing |
| Memory safety bugs | High | Rust safety guarantees, miri, sanitizers, no unsafe in MVP |
| Concurrency issues | Medium | Immutable data structures, Arc for sharing, TSAN |

### Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Scope creep | High | Strict MVP scope, defer JIT to Phase 2 |
| Underestimated complexity | Medium | Buffer time (10 weeks for 8 weeks of work) |
| Dependency issues | Low | Minimal dependencies, cargo-deny policy |

### Quality Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Insufficient test coverage | High | Coverage gates in CI, 100% critical path coverage |
| Missed edge cases | Medium | Fuzzing, property-based tests, manual review |
| Performance regression | Medium | Continuous benchmarking, regression detection |

## Post-MVP: Maturity Roadmap

After MVP (weeks 11+), we'll incrementally add:

### Phase 2: JIT Compilation (4-6 weeks)
- Cranelift integration
- Adaptive tiering
- Performance validation (5-10x improvement)

### Phase 3: Control Plane (4-6 weeks)
- gRPC service
- Atomic updates
- Observability

### Phase 4: Embeddability (4-6 weeks)
- C FFI
- Python bindings
- Node.js bindings
- WASM compilation

### Phase 5: Production Hardening (Ongoing)
- Security audit
- Formal verification (optional)
- Production deployments
- Community support

## Success Criteria for MVP

The MVP is complete when:

✅ **Functional:**
- [ ] All RFC examples parse, compile, and evaluate correctly
- [ ] Performance targets met (< 50μs single predicate eval)
- [ ] All planned tests passing

✅ **Quality:**
- [ ] Code coverage >90%
- [ ] Zero clippy warnings
- [ ] Zero security vulnerabilities
- [ ] Fuzzing: 100M+ iterations without panic

✅ **Operational:**
- [ ] CI/CD fully automated
- [ ] Documentation complete
- [ ] Release process tested

✅ **Validated:**
- [ ] 20+ real-world predicates tested
- [ ] Performance benchmarks published
- [ ] Load tests demonstrate 20k+ ops/sec

## Next Steps

1. **Review & Approve** this MVP plan
2. **Set up repository** with CI/CD scaffolding
3. **Begin Week 1** development (parser)
4. **Daily standups** to track progress
5. **Weekly demos** of working software

---

**Document Status:** Ready for Review
**Version:** 1.0
**Last Updated:** 2025-10-26
