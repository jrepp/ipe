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

# Run 100MB logarithmic distribution test (interpreter) - WARNING: Slow!
perftest-100mb-interpreter:
    @echo "⚠️  Running 100MB logarithmic distribution test (interpreter)..."
    @echo "⚠️  This may take several minutes to generate policies and run!"
    @just perftest perftest_interpreter_logarithmic_100mb

# Run 100MB logarithmic distribution test (JIT) - WARNING: Slow!
perftest-100mb-jit:
    @echo "⚠️  Running 100MB logarithmic distribution test (JIT)..."
    @echo "⚠️  This may take several minutes to generate policies and run!"
    @just perftest perftest_jit_logarithmic_100mb

# Run both 100MB logarithmic distribution tests
perftest-100mb: perftest-100mb-interpreter perftest-100mb-jit

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

# Run Criterion benchmarks
bench:
    cargo bench

# Export benchmark results to GitHub Pages
bench-export:
    cd crates/ipe-core && cargo run --release --bin bench_export

# Run benchmarks and export results
bench-all: bench bench-export
    @echo "✅ Benchmarks complete! Results exported to docs/"

# Run continuous benchmarks (1-second intervals) for time-series charts
bench-continuous DURATION="60":
    @echo "🚀 Running continuous benchmarks for {{DURATION}} seconds..."
    cd crates/ipe-core && cargo run --release --bin bench_continuous --features jit {{DURATION}}
    @echo "✅ Continuous benchmark complete! View at http://localhost:8080/benchmarks.html"

# Start Docker nginx server for docs
docs-serve:
    @echo "🐳 Starting nginx server with Docker..."
    cd docs && docker-compose up -d docs
    @echo "✅ Server running at http://localhost:8080"
    @echo "📊 Performance: http://localhost:8080/performance.html"
    @echo "📈 Benchmarks: http://localhost:8080/benchmarks.html"

# Stop Docker nginx server
docs-stop:
    @echo "🛑 Stopping nginx server..."
    cd docs && docker-compose down

# View Docker logs
docs-logs:
    cd docs && docker-compose logs -f docs

# Test GitHub Pages with Playwright (uses Docker nginx)
test-pages:
    @echo "🧪 Testing GitHub Pages with Playwright..."
    cd docs && docker-compose up -d docs
    @sleep 2
    cd docs && npx playwright test
    cd docs && docker-compose down

# Test GitHub Pages with headed browser (uses Docker nginx)
test-pages-headed:
    @echo "🧪 Starting Docker nginx server..."
    cd docs && docker-compose up -d docs
    @sleep 2
    @echo "🌐 Opening browser at http://localhost:8080"
    @open http://localhost:8080
    @echo "✅ Server running. Press Ctrl+C when done, then run 'just docs-stop'"

# Run quick validation tests
test-pages-quick:
    @echo "🧪 Running quick validation tests..."
    cd docs && node test-pages.js

# Full test with Docker (runs tests in container)
test-pages-docker:
    @echo "🐳 Running full test suite with Docker..."
    cd docs && docker-compose --profile test up --abort-on-container-exit
    cd docs && docker-compose down
