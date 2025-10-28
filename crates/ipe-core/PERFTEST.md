# Predicate Execution Performance Tests

This document describes the comprehensive performance testing suite for predicate execution in IPE Core.

## Overview

The performance test suite measures predicate evaluation performance at high scale on a single CPU for short bursts (10 seconds) and collects comprehensive statistical data. It tests both the interpreter and JIT-compiled execution paths under different workload characteristics.

**🎨 New in v2: Interactive D3.js Visualization!**

Run `just perftest-all` to automatically generate an interactive HTML dashboard with:
- Real-time filtering and comparison
- Multiple chart types (bar, box plot, pie, heatmap)
- JIT vs Interpreter speedup analysis
- Hover tooltips with detailed statistics
- Data export capabilities

**Performance Features:**
- Sample rate tracking (samples/sec)
- JIT cache hit rate statistics
- Mixed workload scenarios (60% simple, 30% medium, 10% complex)
- Bytecode stress tests (deep nesting, many operations)
- Jump-heavy tests (branch prediction stress)
- Cache hit rate comparison across different optimization strategies

## Test Configurations

### Workload Types

1. **Uniform Random Predicates**
   - Tests with 100 different randomly generated predicates
   - Evaluates JIT optimization capabilities with varied predicates
   - Measures overhead of compiling and executing diverse policies
   - Best for testing worst-case scenarios and optimizer effectiveness

2. **Cache-Heavy Predicates**
   - Tests with only 10 unique predicates repeated many times
   - Evaluates cache efficiency and JIT code reuse
   - Measures best-case performance with high locality
   - Best for testing optimized execution paths

### Complexity Levels

- **Simple**: 1-2 comparisons per predicate
- **Medium**: 3-5 comparisons per predicate
- **Complex**: 6-10 comparisons per predicate
- **Very Complex**: 10-20 comparisons per predicate (available for custom tests)

## Statistical Metrics

### Execution Statistics

Each test collects and reports:

- **Min/Max Latency**: Fastest and slowest execution times
- **Mean Latency**: Average execution time
- **Mode**: Most common execution time (bucketed by microsecond)
- **Standard Deviation**: Measure of latency variance
- **Percentiles**:
  - p50 (Median): 50% of executions complete within this time
  - p95: 95% of executions complete within this time
  - p99: 99% of executions complete within this time
- **Throughput**: Operations per second
- **Sample Rate**: Samples collected per second (same as throughput)
- **Total Samples**: Number of evaluations performed during test

### JIT Cache Statistics (JIT tests only)

JIT tests additionally report:

- **Unique Policies**: Number of distinct policies compiled
- **Total Compilations**: Total number of compilation operations
- **Cache Hits**: Number of times compiled code was reused
- **Cache Misses**: Number of times compilation was required
- **Cache Hit Rate**: Percentage of executions using cached compiled code

## Running the Tests

### Prerequisites

```bash
# Install just (Rust-native task runner)
cargo install just

# Ensure you have serde dependencies (already in Cargo.toml)
```

### Quick Start (Recommended)

```bash
# From project root - runs all tests and generates interactive visualization
just perftest-all

# Open the visualization in your browser
open crates/ipe-core/perftest-results.html

# Quick cache test (~20 seconds)
just perftest-quick

# JIT vs Interpreter comparison
just perftest-compare
```

### Interpreter Tests

Run all interpreter performance tests:

```bash
# Without JIT feature (interpreter only)
cargo test --release --test perftest_predicate_execution -- --ignored --nocapture --test-threads=1
```

### JIT Tests

Run all JIT performance tests:

```bash
# With JIT feature enabled
cargo test --release --test perftest_predicate_execution --features jit -- --ignored --nocapture --test-threads=1
```

### Individual Tests

Run specific test configurations:

```bash
# Interpreter - Simple uniform random predicates
cargo test --release --test perftest_predicate_execution perftest_interpreter_uniform_random_simple -- --ignored --nocapture

# Interpreter - Medium complexity uniform random predicates
cargo test --release --test perftest_predicate_execution perftest_interpreter_uniform_random_medium -- --ignored --nocapture

# Interpreter - Complex uniform random predicates
cargo test --release --test perftest_predicate_execution perftest_interpreter_uniform_random_complex -- --ignored --nocapture

# Interpreter - Cache-heavy workload
cargo test --release --test perftest_predicate_execution perftest_interpreter_cache_heavy -- --ignored --nocapture

# JIT - Simple uniform random predicates
cargo test --release --test perftest_predicate_execution --features jit perftest_jit_uniform_random_simple -- --ignored --nocapture

# JIT - Medium complexity uniform random predicates
cargo test --release --test perftest_predicate_execution --features jit perftest_jit_uniform_random_medium -- --ignored --nocapture

# JIT - Complex uniform random predicates
cargo test --release --test perftest_predicate_execution --features jit perftest_jit_uniform_random_complex -- --ignored --nocapture

# JIT - Cache-heavy workload
cargo test --release --test perftest_predicate_execution --features jit perftest_jit_cache_heavy -- --ignored --nocapture

# JIT vs Interpreter comparison (cache-heavy workload)
cargo test --release --test perftest_predicate_execution --features jit perftest_jit_vs_interpreter_comparison -- --ignored --nocapture
```

## Important Notes

### Single-Threaded Execution

Always use `--test-threads=1` to ensure:
- Single CPU execution for consistent results
- No interference between concurrent tests
- Accurate performance measurements

### Release Mode Required

Always run with `--release` flag:
- Debug builds have 10-100x overhead
- Performance metrics in debug mode are meaningless
- Optimizer configurations significantly impact results

### Test Duration

Each test runs for approximately:
- 1 second warm-up phase
- 10 seconds measurement phase
- Additional compilation time for JIT tests

Total runtime for all tests: ~2-3 minutes

## Example Output

### Interpreter Test Output

```
================================================================================
Performance Test: Interpreter - Uniform Random (Simple)
================================================================================
Total samples:    1245678
Test duration:    10.01s
Throughput:       124443 ops/sec
Sample rate:      124443 samples/sec

Latency Statistics:
  Min:                 4.125 µs
  Max:               125.834 µs
  Mean:                8.034 µs
  Mode:                7.500 µs
  Std Dev:             2.145 µs

Percentiles:
  p50 (median):        7.750 µs
  p95:                11.250 µs
  p99:                15.125 µs
================================================================================
```

### JIT Test Output

```
================================================================================
Performance Test: JIT - Cache Heavy (10 predicates)
================================================================================
Total samples:    3421567
Test duration:    10.00s
Throughput:       342157 ops/sec
Sample rate:      342157 samples/sec

Latency Statistics:
  Min:                 1.250 µs
  Max:                45.125 µs
  Mean:                2.923 µs
  Mode:                2.750 µs
  Std Dev:             0.845 µs

Percentiles:
  p50 (median):        2.875 µs
  p95:                 3.750 µs
  p99:                 4.625 µs
================================================================================

JIT Cache Statistics:
  Unique policies:    10
  Total compilations: 10
  Cache hits:         3421557
  Cache misses:       10
  Cache hit rate:     99.9997%
================================================================================
```

## Performance Targets

Based on the existing benchmark comments in `evaluation.rs`:

### Interpreter
- **Single policy p99**: < 50µs

### JIT (when available)
- **Single policy p99**: < 10µs
- **JIT speedup**: 3-10x over interpreter (workload dependent)

### Batch Evaluation
- **1000 policies p99**: < 500µs

## Customizing Tests

To create custom tests or modify existing ones:

1. Use `PredicateGenerator` to create predicates with specific characteristics
2. Adjust `PredicateComplexity` for different workload sizes
3. Modify test duration in the `run_*_test()` calls
4. Adjust the number of unique predicates in `generate_cache_heavy()`

Example:

```rust
#[test]
#[ignore]
fn perftest_custom() {
    let mut gen = PredicateGenerator::new(12345);

    // Generate 50 very complex predicates
    let policies: Vec<_> = (0..50)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::VeryComplex))
        .collect();

    let contexts = create_test_contexts(200, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Custom Test",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(30), // Longer test duration
    );

    stats.print("Custom Test Results");
}
```

## Interpreting Results

### Latency Analysis

- **Low p99 latency** (< 50µs): Excellent for real-time decision making
- **Small stddev**: Consistent, predictable performance
- **Large stddev**: Investigate variability sources (GC, cache misses, etc.)

### Throughput Analysis

- **High throughput**: System can handle high request rates
- **Interpreter baseline**: 50k-200k ops/sec typical
- **JIT improvement**: 3-10x over interpreter

### When to Use JIT

JIT provides benefits when:
- Predicates are reused frequently (cache-heavy workload)
- Low latency is critical (p99 < 10µs required)
- CPU budget available for compilation overhead

Interpreter is sufficient when:
- Predicates change frequently
- p99 < 50µs is acceptable
- Memory/binary size is constrained

## Troubleshooting

### Tests Running Slowly

- Ensure `--release` flag is used
- Check CPU governor settings (should be "performance")
- Ensure no other CPU-intensive processes are running
- Use `--test-threads=1` for consistency

### Compilation Errors

- JIT tests require `--features jit` flag
- Ensure all dependencies are up to date: `cargo update`
- Check Rust version: JIT requires recent Rust toolchain

### Inconsistent Results

- Run multiple times and compare
- Check for thermal throttling on laptop CPUs
- Ensure consistent power management settings
- Disable CPU frequency scaling if possible

## Contributing

When adding new performance tests:

1. Follow the naming convention: `perftest_<executor>_<workload>_<complexity>`
2. Use `#[ignore]` attribute to avoid running in CI
3. Document the test purpose and expected results
4. Include both interpreter and JIT variants when applicable
5. Use consistent test durations (10 seconds) for comparability
