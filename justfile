# Justfile for IPE - Rust-native task runner
# Install: cargo install just
# Run: just <command>

# Show available commands
default:
    @just --list

# Run all perftest (interpreter + JIT) and generate visualization
perftest-all: perftest-run perftest-visualize
    @echo "✅ Complete! Open crates/ipe-core/perftest-results.html in your browser"

# Run interpreter perftests with JSON output
perftest-interpreter:
    @echo "⚡ Running interpreter perftests..."
    cd crates/ipe-core && cargo test --release --test perftest_predicate_execution \
      -- --ignored --nocapture --test-threads=1 interpreter

# Run JIT perftests with JSON output
perftest-jit:
    @echo "⚡ Running JIT perftests..."
    cd crates/ipe-core && cargo test --release --test perftest_predicate_execution --features jit \
      -- --ignored --nocapture --test-threads=1 jit

# Run all perftests and save JSON output
perftest-run:
    @echo "⚡ Running all perftests with JSON output..."
    cd crates/ipe-core && cargo run --release --bin perftest_runner --features jit

# Generate D3.js visualization from results
perftest-visualize:
    @echo "📊 Generating visualization..."
    @echo "✅ Visualization ready: crates/ipe-core/perftest-results.html"

# Run specific perftest by name
perftest TEST:
    cd crates/ipe-core && cargo test --release --test perftest_predicate_execution --features jit \
      -- --ignored --nocapture --test-threads=1 {{TEST}}

# Build perftests without running
perftest-build:
    cd crates/ipe-core && cargo test --release --test perftest_predicate_execution --no-run
    cd crates/ipe-core && cargo test --release --test perftest_predicate_execution --features jit --no-run

# Quick cache test (fastest, ~20s)
perftest-quick:
    @just perftest perftest_jit_cache_heavy

# Compare JIT vs interpreter
perftest-compare:
    @just perftest perftest_jit_vs_interpreter_comparison

# Clean perftest results
perftest-clean:
    rm -f crates/ipe-core/perftest-results.json
    rm -f crates/ipe-core/perftest-results.html
    rm -rf crates/ipe-core/perftest_results_*

# Build all
build:
    cargo build --workspace --all-features

# Build release
build-release:
    cargo build --workspace --all-features --release

# Run all tests
test:
    cargo test --workspace --all-features

# Run clippy
lint:
    cargo clippy --workspace --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run all checks
check: fmt-check lint test

# Development cycle
dev: fmt lint test

# Generate documentation
docs:
    cargo doc --workspace --all-features --no-deps --document-private-items --open

# Clean build artifacts
clean:
    cargo clean
