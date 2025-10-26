# IPE Quick Start Guide

**⚡ Get productive in 5 minutes**

---

## Prerequisites

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
make install-tools
```

---

## Essential Commands

### Daily Development

```bash
# Build
make build

# Format, lint, and test (run before committing!)
make check

# Run tests
make test

# Generate coverage report
make coverage
```

### Performance Testing

```bash
# Run benchmarks
make bench

# Load test
cargo run --release --example load_test -- --evals 100000

# Stress test
cargo run --release --example stress_test -- --policies 10000
```

### Quality & Security

```bash
# Run security audit
make audit

# Run fuzzing (10 minutes)
make fuzz

# Simulate CI locally
make ci
```

---

## Project Structure

```
ipe/
├── crates/
│   └── ipe-core/           # 👈 Start here
│       ├── src/
│       │   ├── lib.rs
│       │   ├── parser/     # Week 1-2: Implement parser
│       │   ├── compiler/   # Week 3-4: Implement compiler
│       │   ├── interpreter/# Week 5-6: Implement interpreter
│       │   └── engine/     # Week 5-6: Implement engine
│       ├── tests/          # Integration tests
│       └── benches/        # Performance benchmarks
│
├── examples/               # Load/stress tests
└── .github/workflows/      # CI/CD (auto-runs)
```

---

## Current Phase: Parser & AST (Weeks 1-2)

### Week 1 Tasks

1. **Implement Lexer** (`crates/ipe-core/src/parser/lexer.rs`)
   ```bash
   # Create the file and start coding
   touch crates/ipe-core/src/parser/lexer.rs
   ```

2. **Implement Parser** (`crates/ipe-core/src/parser/parser.rs`)
   ```bash
   # Create the file
   touch crates/ipe-core/src/parser/parser.rs
   ```

3. **Write Tests** (`crates/ipe-core/tests/parser_tests.rs`)
   ```bash
   # Create test file
   touch crates/ipe-core/tests/parser_tests.rs
   ```

### Success Criteria
- [ ] Parse all RFC examples
- [ ] <10ms for 1000-line file
- [ ] >90% test coverage

---

## Performance Targets

| Metric | Target | How to Test |
|--------|--------|-------------|
| Single policy eval | <50μs p99 | `make bench` |
| Throughput | >20k ops/sec | `make perf` |
| Code coverage | >90% | `make coverage` |

---

## Git Workflow

```bash
# Create feature branch
git checkout -b phase1/parser-week1

# Make changes, then run checks
make check

# Commit
git add .
git commit -m "feat: implement lexer"

# Push and create PR
git push origin phase1/parser-week1
```

---

## Troubleshooting

### Build fails
```bash
# Clean and rebuild
make clean && make build
```

### Tests fail
```bash
# Run with output
make test-verbose
```

### Coverage not generating
```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Try again
make coverage
```

---

## Key Files to Read

1. **[MVP_PLAN.md](MVP_PLAN.md)** - Weekly deliverables
2. **[RFC.md](RFC.md)** - Technical specification
3. **[ROADMAP.md](ROADMAP.md)** - Long-term plan

---

## CI/CD

All checks run automatically on push:
- ✅ Format check
- ✅ Clippy lint
- ✅ Tests (Linux, macOS, Windows)
- ✅ Security audit
- ✅ Code coverage

**PR Requirements:**
- All CI checks pass
- Code coverage >90%
- No clippy warnings

---

## Help & Resources

### Getting Unstuck
- Read [MVP_PLAN.md](MVP_PLAN.md) for context
- Check [RFC.md](RFC.md) for specifications
- Run `make help` for available commands

### Learning
- **Parser:** [nom documentation](https://docs.rs/nom/)
- **Performance:** [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- **Testing:** [cargo-llvm-cov docs](https://docs.rs/cargo-llvm-cov/)

---

## Quick Reference

```bash
make build          # Build all crates
make test           # Run tests
make bench          # Run benchmarks
make lint           # Run clippy
make fmt            # Format code
make check          # Format + lint + test
make coverage       # Generate coverage report
make audit          # Security audit
make ci             # Simulate CI locally
make help           # Show all commands
```

---

## Status Dashboard

**Phase:** 1 (Parser & AST)
**Week:** 1
**Target Completion:** 2 weeks

**Build:** ✅ Passing
**Tests:** ✅ 0 failing (stubs)
**Coverage:** ⏸️ TBD (after implementation)
**Security:** ✅ 0 vulnerabilities

---

**Last Updated:** 2025-10-26
**Next Review:** End of Week 1
