# Performance Testing Quick Start

Get started with IPE predicate execution performance testing in 3 minutes.

## One-Time Setup

```bash
# Install just (Rust-native task runner)
cargo install just
```

## Run Tests with Visualization

```bash
# From project root
just perftest-all

# Opens after ~3 minutes with 18 test results
open crates/ipe-core/perftest-results.html
```

## What You Get

### 🎨 Interactive D3.js Dashboard

- **6 visualization types:**
  1. Latency comparison bar chart
  2. Throughput comparison
  3. Latency distribution box plots
  4. JIT cache hit rate pie chart
  5. Performance heatmap
  6. JIT vs Interpreter speedup

- **Interactive features:**
  - Filter by executor (interpreter/JIT)
  - Filter by workload type
  - Switch metrics (p50/p95/p99/throughput)
  - Hover tooltips with details
  - Export data to JSON

### 📊 Test Coverage

**18 total tests:**
- 7 interpreter tests
- 11 JIT tests

**Workload types:**
- Uniform random (simple/medium/complex)
- Cache-heavy (best JIT case)
- Mixed workload (realistic)
- Bytecode stress (worst case)
- Jump-heavy (branch prediction)

## Quick Commands

```bash
# Fastest check (20 seconds)
just perftest-quick

# Compare JIT vs Interpreter
just perftest-compare

# Full suite with visualization (3 minutes)
just perftest-all

# Clean old results
just perftest-clean

# See all commands
just --list
```

## Using Cargo Aliases

From anywhere in the project:

```bash
cargo perftest          # Interpreter tests
cargo perftest-jit      # JIT tests
cargo perftest-cache    # Cache-heavy only
cargo perftest-compare  # JIT vs Interpreter
```

## Understanding Results

### Key Metrics

- **p99 latency**: 99% of operations complete within this time
- **Throughput**: Operations per second
- **Sample rate**: Measurements per second
- **Cache hit rate**: % of JIT code reuse (JIT only)

### Performance Targets

- **Interpreter p99**: < 50µs
- **JIT p99**: < 10µs
- **JIT speedup**: 3-10x
- **Cache hit rate**: > 99% (cache-heavy workloads)

## File Locations

- **Test code**: `crates/ipe-core/tests/perftest_predicate_execution.rs`
- **Runner binary**: `crates/ipe-core/src/bin/perftest_runner.rs`
- **Visualization**: `crates/ipe-core/perftest-visualization.html`
- **Results JSON**: `crates/ipe-core/perftest-results.json` (generated)
- **Results HTML**: `crates/ipe-core/perftest-results.html` (generated)

## Documentation

- **This file**: Quick start
- **README_PERFTEST.md**: Detailed usage guide
- **PERFTEST.md**: Complete documentation

## Troubleshooting

### "just: command not found"
```bash
cargo install just
```

### Tests take too long
```bash
# Run quick subset instead of full suite
just perftest-quick
```

### Visualization doesn't show
```bash
# Make sure JSON file exists
ls crates/ipe-core/perftest-results.json

# Regenerate visualization
cd crates/ipe-core
cargo run --release --bin perftest_runner --features jit
```

### Inconsistent results
- Close other applications
- Run in release mode (already default)
- Run multiple times and compare
- Check CPU throttling on laptops

## Next Steps

1. ✅ Run `just perftest-all`
2. ✅ Open `perftest-results.html` in browser
3. ✅ Explore the interactive charts
4. ✅ Try filtering and comparing
5. ✅ Export data for analysis

## Advanced Usage

### Compare Multiple Runs

```bash
# Run 1
just perftest-all
mv crates/ipe-core/perftest-results.json results-baseline.json

# Make code changes...

# Run 2
just perftest-all
mv crates/ipe-core/perftest-results.json results-optimized.json

# Compare JSON files
diff results-baseline.json results-optimized.json
```

### Custom Test Duration

Edit test source to change `Duration::from_secs(10)` to desired duration.

### CI Integration

```bash
# Build only (fast)
just perftest-build

# Run subset
just perftest-quick
```

## Tips

- Always use release mode (default in commands)
- Single-threaded execution (automatic)
- Run on idle system for accuracy
- JIT benefits most visible in cache-heavy tests
- Interpreter excels with diverse predicates

## Have Fun! 🎉

The visualization is interactive - play with filters, hover over charts, and explore the performance characteristics of the predicate execution engine!
