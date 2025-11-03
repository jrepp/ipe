# Core Agent Prompt: Idempotent Predicate Engine (IPE)

## Project Overview

You are working on the **Idempotent Predicate Engine (IPE)**, a high-performance, AI-native predicate evaluation engine built in Rust.

IPE compiles human-readable predicates into optimized bytecode with multi-tier execution (interpreter → baseline JIT → optimized JIT) and lock-free concurrent evaluation. The engine is designed for access control, workflow validation, and dynamic configuration that changes frequently without requiring service redeployment.

### Key Project Documents

- **README**: `README.md` - Project overview and quick start
- **Requirements**: `REQUIREMENTS.md` - Comprehensive requirements specification
- **Architecture**: `docs/ARCHITECTURE.md` - System architecture with diagrams
- **Bytecode**: `docs/BYTECODE.md` - Instruction set and execution model
- **AST**: `docs/AST.md` - Abstract syntax tree specification
- **RFC Index**: `rfcs/INDEX.md` - Design proposals and specifications

## Core Principles

### 0. Professional Communication Standards

**CRITICAL**: All commits, PRs, and documentation must be professional, concise, and tool-agnostic.

**Absolutely Prohibited**:
- ❌ Agent/tool branding (e.g., "Generated with Claude Code", "Co-Authored-By: Claude")
- ❌ Marketing links or promotional content
- ❌ AI assistant attribution or references
- ❌ Emoji in PRs, commits, or code
- ❌ Verbose explanations or marketing language
- ❌ Long-winded PR descriptions (target: <20 lines)

**Required**:
- ✅ Concise, technical, verification-focused PRs
- ✅ Actual test results and numbers (not descriptions of benefits)
- ✅ Clear problem/solution/verification structure
- ✅ Focus on what was tested and verified
- ✅ Tool-agnostic documentation
- ✅ Human-centric communication

**PR Length Guideline**: Most PRs should be <20 lines. Include problem statement, solution approach, and concrete verification data (test counts, benchmark results, coverage percentages). Skip verbose explanations, benefits sections, and decorative formatting.

This is a professional open-source project. All contributions should read as if written by a human developer focused on technical substance and verification, not marketing or explanation.

### 1. Rust Best Practices

Follow Rust idioms and best practices:

1. **Memory Safety First**: No unsafe code in workspace crates without explicit justification
2. **Zero-Cost Abstractions**: Optimize for performance without sacrificing safety
3. **Explicit Error Handling**: Use `Result<T, E>` and `Option<T>`, never `unwrap()` in production code
4. **Ownership and Borrowing**: Leverage Rust's ownership system for safe concurrency
5. **Type Safety**: Use strong typing to catch errors at compile time

### 2. Performance-Critical Code

IPE is a performance-critical system with specific targets:

- **Latency**: p99 < 10µs (JIT), p99 < 50µs (interpreter)
- **Throughput**: 100K+ evaluations/second per core
- **Memory**: <1KB overhead per predicate
- **Binary Size**: <50MB for embedded deployment

Always:
- Profile before optimizing
- Benchmark changes against baseline
- Document performance implications
- Avoid premature optimization

### 3. Lock-Free Concurrency

The system uses Arc-based immutable snapshots for lock-free reads:

- **No Locks**: Reader threads never block on locks
- **Atomic Updates**: Predicate sets swap atomically
- **Immutable Data**: Snapshots are read-only
- **Zero-Copy**: Minimize allocations during evaluation

### 4. Test-Driven Development

Maintain high code quality through comprehensive testing:

- **Coverage Target**: 90%+ test coverage (current: 93.67%)
- **Test Types**: Unit tests, integration tests, property-based tests, fuzz tests, benchmarks
- **CI Validation**: All tests must pass before merge
- **No Regressions**: Performance benchmarks prevent slowdowns

## Project Architecture Context

### Core Components

1. **Parser** (`ipe-parser`)
   - Lexer and parser for predicate source
   - AST generation
   - Error reporting with source locations

2. **Core Engine** (`ipe-core`)
   - AST → Bytecode compiler
   - Stack-based bytecode interpreter
   - Optional JIT compilation via Cranelift
   - Lock-free predicate data store
   - Evaluation engine with RAR context

3. **Control Plane** (`ipe-control`)
   - gRPC API for predicate management
   - Atomic predicate updates
   - Metrics and observability

4. **WASM Bindings** (`ipe-wasm`)
   - WebAssembly compilation
   - Browser-side evaluation
   - Feature flags and A/B testing

5. **FFI Bindings** (`ipe-ffi`)
   - C-compatible API
   - Cross-language integration

6. **Web Interface** (`ipe-web`)
   - Interactive demo
   - Documentation site
   - Performance visualizations

### Multi-Tier Execution

IPE uses adaptive tiering for performance:

1. **Interpreter** (baseline) - All predicates start here
2. **Baseline JIT** (100+ evals) - Fast compilation
3. **Optimized JIT** (10K+ evals) - Full optimization with Cranelift

### Technology Stack

- **Language**: Rust 2021 edition (stable)
- **JIT Compiler**: Cranelift
- **Build System**: Cargo with workspaces
- **Testing**: cargo test, criterion benchmarks, cargo fuzz
- **CI/CD**: GitHub Actions
- **Documentation**: Markdown, Mermaid diagrams

## Commit Message Standards

This project uses **Conventional Commits** with specific requirements.

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Critical Requirements

**MUST OMIT**:
- ❌ NO agent/tool branding or marketing links
- ❌ NO co-authorship tags or attribution unless explicitly requested by user
- ❌ NO emoji (except in commit footer if explicitly requested)
- ❌ NO fluff or marketing language

**MUST INCLUDE**:
- ✅ Clear, concise descriptions
- ✅ Technical details in body
- ✅ Reference issues when applicable

### Types

**Release triggers** (semantic versioning):
- `feat:` - New feature (MINOR bump: 0.1.0 → 0.2.0)
- `fix:` - Bug fix (PATCH bump: 0.1.0 → 0.1.1)
- `perf:` - Performance improvement (PATCH bump)

**Non-release**:
- `docs:` - Documentation only
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring without behavior change
- `test:` - Test additions or modifications
- `build:` - Build system or dependency changes
- `ci:` - CI/CD configuration changes
- `chore:` - Maintenance tasks

**Breaking changes**:
- Add `BREAKING CHANGE:` in footer OR `!` after type
- Triggers MAJOR bump (0.1.0 → 1.0.0)

### Examples

```bash
# Feature commit
git commit -m "feat(jit): add baseline JIT tier with Cranelift

Implements fast compilation path for hot predicates.
Triggers after 100 evaluations of same predicate.
Reduces p99 latency from 50µs to 20µs."

# Bug fix
git commit -m "fix(interpreter): resolve stack overflow in nested expressions

Increases max stack depth and adds overflow detection.
Fixes issue with deeply nested logical expressions."

# Performance improvement
git commit -m "perf(evaluation): optimize field lookup with perfect hashing

Replaces HashMap with compile-time perfect hash for field access.
Improves evaluation throughput by 15%."

# Documentation
git commit -m "docs: add RFC-004 for WASM feature flags

Proposes client-side feature flag evaluation using WASM.
Includes performance analysis and security considerations."

# Breaking change
git commit -m "feat(api)!: change predicate compilation API to return Result

BREAKING CHANGE: compile() now returns Result<Bytecode, CompileError>
instead of panicking on error.

Migration: Wrap compile() calls in match or ? operator."
```

## Rust Development Standards

### Workspace Structure

The project uses Cargo workspaces:

```
ipe/
├── Cargo.toml           # Workspace root
├── crates/
│   ├── ipe-core/        # Core engine
│   ├── ipe-parser/      # Language parser
│   ├── ipe-control/     # Control plane
│   ├── ipe-wasm/        # WASM bindings
│   ├── ipe-ffi/         # C FFI
│   └── ipe-web/         # Web interface
└── examples/
    └── jit_demo.rs      # Usage examples
```

### Common Commands

```bash
# Build all crates
cargo build --all-features

# Run tests
cargo test --all-features --workspace

# Run tests with output
cargo test --all-features --workspace -- --nocapture

# Run benchmarks
cargo bench --all-features

# Check code (no build)
cargo check --all-features

# Format code
cargo fmt --all

# Lint code
cargo clippy --all-features --all-targets -- -D warnings

# Generate documentation
cargo doc --all-features --no-deps --open

# Run examples
cargo run --example jit_demo --features jit

# Run with sanitizers
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test

# Fuzz testing
cargo +nightly fuzz run fuzz_target_1
```

### Code Quality Requirements

Before every commit:

```bash
# Format check
cargo fmt --all -- --check

# Clippy without warnings
cargo clippy --all-features --all-targets -- -D warnings

# All tests pass
cargo test --all-features --workspace

# Benchmarks don't regress
cargo bench --all-features

# Documentation builds
cargo doc --all-features --no-deps
```

### Dependencies

Add dependencies using Cargo:

```bash
# Add runtime dependency
cargo add <crate-name> -p <package-name>

# Add dev dependency
cargo add --dev <crate-name> -p <package-name>

# Add build dependency
cargo add --build <crate-name> -p <package-name>

# Update dependencies
cargo update
```

## Security and Safety Standards

### Memory Safety

- **No Unsafe Code**: Workspace crates should not contain `unsafe` blocks
- **Exceptions**: If `unsafe` is required, document why and add safety comments
- **Dependencies**: Audit dependencies with unsafe code using `cargo-geiger`
- **Sanitizers**: All tests must pass address, leak, and thread sanitizers

### Supply Chain Security

```bash
# Check for security vulnerabilities
cargo audit

# Check for license compliance
cargo deny check licenses

# Check for unsafe code
cargo geiger --all-features

# Validate all checks
.github/scripts/check-supply-chain.sh
```

### Input Validation

- **Parse-time validation**: Reject invalid predicates during parsing
- **Bytecode validation**: Verify bytecode integrity before execution
- **Context validation**: Validate evaluation context structure
- **Resource limits**: Enforce max bytecode size, stack depth, evaluation time

## Key Functional Requirements Reference

### Language and Compilation (REQ-LANG, REQ-COMP)
- Human-readable DSL with boolean logic (REQ-LANG-001 to 007)
- Parse → AST → Bytecode pipeline (REQ-COMP-001 to 007)
- Stack-based bytecode (~200 bytes/predicate)
- Comprehensive error messages with source locations

### Evaluation and Execution (REQ-EVAL, REQ-EXEC)
- RAR (Resource-Action-Request) context evaluation (REQ-EVAL-001)
- Binary Allow/Deny decisions (REQ-EVAL-002)
- Deterministic and idempotent (REQ-EVAL-005, REQ-EVAL-006)
- Multi-tier execution: Interpreter → JIT (REQ-EXEC-001 to 006)

### Storage and Concurrency (REQ-STOR, REQ-SCALE)
- Lock-free concurrent reads (REQ-STOR-002)
- Atomic updates (REQ-STOR-003)
- Immutable snapshots (REQ-STOR-004)
- 1000+ concurrent readers (REQ-SCALE-003)

### Performance Targets (REQ-PERF)
- Interpreter: p99 < 50µs (REQ-PERF-001)
- Baseline JIT: p99 < 20µs (REQ-PERF-002)
- Optimized JIT: p99 < 10µs (REQ-PERF-003)
- Throughput: 100K+ evals/sec/core (REQ-PERF-006)

### Security (REQ-SEC)
- Memory-safe Rust (REQ-SEC-001)
- No unsafe code in workspace (REQ-SEC-002)
- Pass sanitizers (REQ-SEC-003)
- Supply chain audits (REQ-SEC-004 to 007)

## Common Tasks

### Starting New Feature

1. **Check requirements** - Review REQUIREMENTS.md for relevant specs
2. **Read architecture docs** - Understand system design in docs/
3. **Create RFC if major** - Significant changes need RFC in rfcs/
4. **Write tests first** - TDD approach for new functionality
5. **Implement feature** - Follow Rust best practices
6. **Benchmark if performance-critical** - Use criterion for measurements
7. **Update documentation** - Keep docs in sync with code
8. **Commit with conventional format** - Follow commit standards

### Fixing Bugs

1. **Reproduce the bug** - Create failing test case
2. **Root cause analysis** - Understand why it happens
3. **Implement fix** - Minimal changes to fix issue
4. **Verify fix** - Test passes, no regressions
5. **Add regression test** - Prevent future occurrences
6. **Document if needed** - Update docs if behavior clarified
7. **Commit with fix: prefix** - Clear commit message

### Optimizing Performance

1. **Establish baseline** - Run benchmarks to get current metrics
2. **Profile the code** - Use `cargo flamegraph` or `perf`
3. **Identify bottlenecks** - Focus on hot paths
4. **Implement optimization** - Make targeted changes
5. **Benchmark again** - Verify improvement
6. **Document trade-offs** - Note any complexity added
7. **Commit with perf: prefix** - Include metrics in message

### Adding Documentation

1. **Choose document type**:
   - Architecture → `docs/ARCHITECTURE.md`
   - Design proposal → `rfcs/NNN-title.md`
   - API docs → Rust doc comments `///`
   - Examples → `examples/` directory
2. **Write clear explanations** - Target audience matters
3. **Include diagrams** - Use Mermaid for visualization
4. **Add code examples** - Show practical usage
5. **Review for accuracy** - Verify against implementation
6. **Commit with docs: prefix** - Documentation-only changes

### Updating CHANGELOG.md

The CHANGELOG follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.

**When to update**:
- All user-facing changes (features, fixes, breaking changes)
- API changes (additions, modifications, removals)
- Performance improvements
- Security fixes
- Deprecations

**How to update**:

1. **Add to Unreleased section** at the top under appropriate category:
   - `Added` - New features
   - `Changed` - Changes to existing functionality
   - `Deprecated` - Soon-to-be removed features
   - `Removed` - Removed features
   - `Fixed` - Bug fixes
   - `Security` - Security fixes
   - `Performance` - Performance improvements

2. **Format**: Brief description linking to PR/issue if available
   ```markdown
   ### Added
   - New bytecode instruction for pattern matching (#42)
   - JIT compilation tier for hot predicates
   ```

3. **Commit with changelog** in the same commit as your changes

**What NOT to include**:
- Internal refactoring (unless it affects users)
- Documentation updates (unless they document new features)
- Test additions (unless they're examples for users)
- CI/CD changes (unless they affect contribution process)

## CI/CD Pipeline

All PRs must pass CI checks:

### Required Checks

1. **Build**: `cargo build --all-features`
2. **Tests**: `cargo test --all-features --workspace`
3. **Format**: `cargo fmt --all -- --check`
4. **Clippy**: `cargo clippy --all-features --all-targets -- -D warnings`
5. **Benchmarks**: `cargo bench --all-features` (run but don't fail)
6. **Sanitizers**: Address, leak, thread sanitizers
7. **Security**: cargo-deny, cargo-audit, cargo-geiger
8. **Coverage**: Upload to codecov (target: 90%+)

### Workflow Files

- `.github/workflows/ci.yml` - Main CI pipeline
- `.github/workflows/bench.yml` - Benchmark runs
- `.github/workflows/security.yml` - Security audits
- `.github/scripts/` - Helper scripts

## Git Workflow Best Practices

### Branch Naming

```
feat/<feature-name>        # New features
fix/<bug-description>      # Bug fixes
perf/<optimization-name>   # Performance improvements
refactor/<component-name>  # Refactoring
docs/<doc-name>            # Documentation
ci/<change-description>    # CI/CD changes
```

### Pull Request Standards

**CRITICAL**: PRs must be concise, technical, and verification-focused.

**Title**: Clear, follows conventional commit format (e.g., "fix(parser): resolve nested expression bug")

**Body format** (concise, no emoji, no verbose explanations):

```markdown
## Problem
[1-2 sentences: what bug/requirement/issue this addresses]

## Solution
[2-3 sentences: technical approach taken]

## Verification
- Tests: [test command output or "all 248 tests pass"]
- Benchmarks: [criterion results if applicable, or "no regression"]
- Clippy: [clean or "0 warnings"]
- Coverage: [percentage, e.g., "93.7% maintained"]
- Manual: [specific manual test if needed]

## Breaking Changes
[Only if applicable, 1-2 sentences on migration]
```

**What NOT to include**:
- ❌ Emoji or decorative elements
- ❌ Verbose explanations or marketing language
- ❌ Repetitive "benefits" or "features" sections
- ❌ Long file-by-file change descriptions
- ❌ Excessive formatting (tables, diagrams unless essential)

**Focus on**:
- ✅ Test results and verification data
- ✅ Actual command output (test count, benchmark numbers)
- ✅ Technical facts (what changed, what was verified)
- ✅ Brevity (target: <20 lines for most PRs)

### Small, Focused PRs

- **Target**: < 400 lines changed
- **One concern per PR**: Don't mix features, fixes, and refactoring
- **Easier to review**: Small PRs get merged faster
- **Easier to revert**: If issues arise

## Quick Command Reference

```bash
# Development cycle
cargo fmt --all                                      # Format code
cargo clippy --all-features --all-targets -- -D warnings  # Lint
cargo test --all-features --workspace                # Test
cargo bench --all-features                           # Benchmark
cargo build --release --all-features                 # Release build

# Security checks
cargo audit                                          # Vulnerability check
cargo deny check                                     # License/security
cargo geiger --all-features                          # Unsafe code audit

# CI simulation (run before pushing)
.github/scripts/check-sanitizer.sh                   # Sanitizer tests
.github/scripts/check-supply-chain.sh                # Supply chain check
make check                                           # Full checks

# Documentation
cargo doc --all-features --no-deps --open            # Generate docs
cargo readme > README.md                             # Generate README

# Utilities
make clean                                           # Clean build artifacts
make coverage                                        # Generate coverage report
make fuzz                                            # Run fuzzing
```

## Performance Optimization Guidelines

### When to Optimize

1. **Profile first** - Don't guess, measure
2. **Focus on hot paths** - Optimize what matters
3. **Benchmark continuously** - Prevent regressions
4. **Document trade-offs** - Speed vs. complexity

### Common Optimizations

**Hot Loops**:
- Use iterators (zero-cost abstractions)
- Avoid allocations (pre-allocate or use stack)
- Consider SIMD for data-parallel ops
- Profile-guided optimization (PGO)

**Memory Management**:
- Use `&[T]` instead of `Vec<T>` when possible
- Avoid cloning (use references)
- Pool allocations for repeated use
- Consider `SmallVec` for small collections

**Concurrency**:
- Use lock-free structures (Arc, atomics)
- Minimize shared mutable state
- Batch operations to reduce contention
- Profile with `perf` to find cache misses

## Testing Strategy

### Unit Tests

- Test individual functions and methods
- Mock dependencies with traits
- Use property-based testing with `proptest`
- Aim for 100% coverage of public API

### Integration Tests

- Test full compilation pipeline
- Test multi-tier execution
- Test concurrent evaluation scenarios
- Verify error handling end-to-end

### Benchmark Tests

- Use `criterion` for statistical rigor
- Compare against baselines
- Test with realistic workloads
- Track metrics over time

### Fuzz Tests

- Fuzz parser with random inputs
- Fuzz compiler with malformed AST
- Fuzz interpreter with random bytecode
- Continuous fuzzing with OSS-Fuzz

## Critical Rules

1. **NO AGENT/TOOL BRANDING** - Never include tool names, attribution, marketing links, or AI references in commits, PRs, or code
2. **CONCISE PRS** - PRs must be brief (<20 lines), verification-focused, no emoji, no verbose explanations
3. **NO UNSAFE CODE** - Without explicit justification and safety comments
4. **ALL TESTS MUST PASS** - Before committing
5. **NO CLIPPY WARNINGS** - Fix or suppress with `#[allow]` and comment
6. **FORMAT BEFORE COMMIT** - Run `cargo fmt --all`
7. **CONVENTIONAL COMMITS** - Follow format strictly, no fluff
8. **VERIFICATION DATA** - PRs must include actual test results, benchmark numbers, coverage percentages
9. **BENCHMARK PERFORMANCE-CRITICAL CODE** - Verify no regressions
10. **DOCUMENT PUBLIC API** - All public items need doc comments
11. **UPDATE REQUIREMENTS.MD** - Keep requirements in sync
12. **UPDATE CHANGELOG.MD** - Add entries to Unreleased section for all user-facing changes
13. **SECURITY AUDITS** - Run supply chain checks regularly
14. **HUMAN-CENTRIC** - All contributions should read as if written by a human developer

## Success Criteria

Your work is successful when:

1. ✅ All CI checks pass (build, test, clippy, format)
2. ✅ Code coverage maintained or improved (target: 90%+)
3. ✅ No performance regressions (benchmark results)
4. ✅ Security audits pass (cargo-audit, cargo-deny, cargo-geiger)
5. ✅ Documentation updated (docs/, REQUIREMENTS.md, CHANGELOG.md, code comments)
6. ✅ CHANGELOG.md updated for user-facing changes
7. ✅ Commits follow conventional format
8. ✅ PR includes why/what/how/testing
9. ✅ Changes align with requirements and architecture
10. ✅ No agent/tool branding in any commits or documentation

---

**For detailed technical specifications**, refer to:
- Architecture: `docs/ARCHITECTURE.md`
- Requirements: `REQUIREMENTS.md`
- Bytecode Spec: `docs/BYTECODE.md`
- AST Spec: `docs/AST.md`
- RFC Index: `rfcs/INDEX.md`
