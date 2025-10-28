# Quick Start - Performance Testing

This guide shows you the fastest ways to run performance tests with interactive D3.js visualizations.

## Prerequisites

```bash
# Install just (Rust-native task runner)
cargo install just

# Verify installation
just --version
```

## Quick Commands

### Using `just` (Recommended - Rust Standard)

From the project root:

```bash
# Run ALL perftests and generate interactive visualization (18 tests, ~3 min)
just perftest-all

# Quick cache test (fastest, ~20s)
just perftest-quick

# Compare JIT vs interpreter
just perftest-compare

# Run specific test
just perftest perftest_interpreter_cache_heavy

# Build only (no run)
just perftest-build

# Clean results
just perftest-clean
```

## Cargo Aliases

From anywhere in the project:

```bash
# Run interpreter perftests
cargo perftest

# Run JIT perftests
cargo perftest-jit

# Run all perftests
cargo perftest-all

# Quick test specific workloads
cargo perftest-simple       # Simple predicates only
cargo perftest-cache        # Cache-heavy workloads only
cargo perftest-mixed        # Mixed workload tests only
cargo perftest-stress       # Bytecode stress tests only

# Run comparison tests (JIT vs interpreter)
cargo perftest-compare

# Build perftests
cargo perftest-build
```

## Interactive Visualization

After running tests, open the generated HTML file in your browser:

```bash
# Generate visualization (done automatically by just perftest-all)
open crates/ipe-core/perftest-results.html
```

### Visualization Features

The D3.js dashboard includes:

1. **📊 Latency Comparison** - Bar chart of test latencies
2. **⚡ Throughput Comparison** - Operations per second comparison
3. **📈 Latency Distribution** - Box plots showing variance
4. **🎯 JIT Cache Hit Rate** - Pie chart of cache efficiency
5. **🔬 Performance Heatmap** - Color-coded latency matrix
6. **⚖️ JIT vs Interpreter Speedup** - Direct speedup comparison

**Interactive Features:**
- Hover tooltips with detailed stats
- Filter by executor (interpreter/JIT)
- Filter by workload type
- Select different metrics (p50/p95/p99/throughput)
- Export data as JSON
- Responsive design

## Direct Cargo Commands

From `crates/ipe-core`:

```bash
# Interpreter tests
cargo test --release --test perftest_predicate_execution \
  -- --ignored --nocapture --test-threads=1 interpreter

# JIT tests
cargo test --release --test perftest_predicate_execution --features jit \
  -- --ignored --nocapture --test-threads=1 jit

# Specific test
cargo test --release --test perftest_predicate_execution --features jit \
  -- --ignored --nocapture perftest_jit_cache_heavy
```

## Understanding Output

### Execution Statistics
- **Throughput**: Operations per second
- **Sample rate**: Samples collected per second
- **p50/p95/p99**: Latency percentiles in microseconds

### JIT Cache Statistics (JIT tests only)
- **Cache hit rate**: Percentage of executions using cached compiled code
- **Unique policies**: Number of distinct policies compiled
- **Total compilations**: Times JIT compiler was invoked

### Test Types

1. **Uniform Random**: 100 diverse predicates (tests worst-case JIT)
2. **Cache Heavy**: 10 repeated predicates (tests best-case JIT)
3. **Mixed Workload**: 60% simple, 30% medium, 10% complex (realistic)
4. **Bytecode Stress**: Deep nesting, many operations (stress test)
5. **Jump Heavy**: Many conditional branches (branch prediction)

## Quick Examples

```bash
# Fast: Run just cache-heavy test to see JIT benefits (~20s)
just perftest-quick

# Compare: See JIT vs interpreter performance (~20s)
just perftest-compare

# Full: Run complete test suite with interactive visualization (~3 min)
just perftest-all

# Then open the visualization
open crates/ipe-core/perftest-results.html

# Clean old results
just perftest-clean
```

## Visualization Workflow

```bash
# 1. Run tests and generate data
just perftest-all

# 2. View results in browser
open crates/ipe-core/perftest-results.html

# 3. Interact with the dashboard:
#    - Filter by executor/workload
#    - Switch between metrics
#    - Hover for detailed stats
#    - Export data for analysis

# 4. Compare runs by saving results
mv crates/ipe-core/perftest-results.json run1.json
just perftest-all
mv crates/ipe-core/perftest-results.json run2.json
# Then compare run1.json vs run2.json
```

## Performance Targets

- **Interpreter p99**: < 50µs per policy evaluation
- **JIT p99**: < 10µs per policy evaluation
- **JIT speedup**: 3-10x over interpreter (workload dependent)
- **Cache hit rate**: > 99% for cache-heavy workloads

## Customizing Tests

See [PERFTEST.md](./PERFTEST.md) for:
- Detailed test descriptions
- Customization examples
- Troubleshooting guide
- Full command reference

## Tips

1. **Always use `--release`**: Debug builds are 10-100x slower
2. **Use `--test-threads=1`**: Ensures single-CPU execution for consistency
3. **Run on idle system**: Close other apps for accurate results
4. **Check thermal throttling**: Long tests on laptops may throttle CPU
5. **Multiple runs**: Run 2-3 times and compare for consistency
