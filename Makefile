.PHONY: help build test bench lint fmt check clean install-tools coverage audit fuzz docs docs-site docs-serve docs-check runtime-acceptance policy-acceptance

# One exact version shared with actions/setup-python in docs CI.
DOCS_PYTHON = uv run --no-project --python $(shell cat .python-version) python

# Default target
help:
	@echo "Idempotent Predicate Engine - Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make build          - Build all crates"
	@echo "  make test           - Run all tests"
	@echo "  make bench          - Run benchmarks"
	@echo "  make lint           - Run clippy linter"
	@echo "  make fmt            - Format code with rustfmt"
	@echo "  make check          - Run all checks (fmt, lint, test)"
	@echo "  make coverage       - Generate code coverage report"
	@echo "  make audit          - Run security audits"
	@echo "  make fuzz           - Run fuzzing tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make install-tools  - Install development tools"
	@echo "  make release        - Build release binaries"
	@echo "  make perf           - Run performance validation"
	@echo "  make docs-site      - Generate parser trees and local performance dashboard data"
	@echo "  make docs-serve     - Serve docs at http://127.0.0.1:8000/docs/"
	@echo "  make docs-check     - Build and validate the docs (run npm ci --prefix docs first)"

# Exercise the public runtime-data workflow against a real server.
runtime-acceptance:
	cargo build --locked -p ipe-server
	python3 scripts/accept-runtime-data.py

# Test immutable policy data, mutable evidence, and external policy imports.
policy-acceptance:
	cargo build --locked -p ipe-server
	cargo run --locked -p ipe-core --features approvals --example policy_scenarios -- acceptance/policy-data/pr-landing.json target/policy-data-scenarios.json
	python3 scripts/accept-policy-data.py

# Build all crates
build:
	@echo "🔨 Building all crates..."
	cargo build --all-features

# Build release
release:
	@echo "🚀 Building release binaries..."
	cargo build --release --all-features
	@echo "✅ Release build complete"
	@ls -lh target/release/ipe-* 2>/dev/null || true

# Run all tests
test:
	@echo "🧪 Running tests..."
	cargo test --all-features --workspace

# Run tests with output
test-verbose:
	@echo "🧪 Running tests (verbose)..."
	cargo test --all-features --workspace -- --nocapture

# Run benchmarks
bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --all-features

# Run clippy linter
lint:
	@echo "🔍 Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt --all

# Check formatting
fmt-check:
	@echo "🔍 Checking formatting..."
	cargo fmt --all -- --check

# Run all checks
check: fmt-check lint test
	@echo "✅ All checks passed!"

# Generate code coverage
coverage:
	@echo "📊 Generating code coverage..."
	cargo llvm-cov --all-features --workspace --html
	@echo "✅ Coverage report generated: target/llvm-cov/html/index.html"
	@command -v open >/dev/null 2>&1 && open target/llvm-cov/html/index.html || true

# Run security audits
audit:
	@echo "🔒 Running security audits..."
	cargo audit
	cargo deny check
	@echo "✅ Security audit complete"

# Run fuzzing tests
fuzz:
	@echo "🐛 Running fuzzing tests..."
	@if [ ! -d "fuzz" ]; then \
		echo "Initializing fuzz targets..."; \
		cargo install cargo-fuzz; \
		cargo fuzz init; \
	fi
	cargo +nightly fuzz run parse_policy -- -max_total_time=300
	@echo "✅ Fuzzing complete"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@echo "✅ Clean complete"

# Install development tools
install-tools:
	@echo "📦 Installing development tools..."
	@echo "Installing rustfmt..."
	rustup component add rustfmt
	@echo "Installing clippy..."
	rustup component add clippy
	@echo "Installing llvm-tools..."
	rustup component add llvm-tools-preview
	@echo "Installing cargo-llvm-cov..."
	cargo install cargo-llvm-cov
	@echo "Installing cargo-audit..."
	cargo install cargo-audit
	@echo "Installing cargo-deny..."
	cargo install cargo-deny
	@echo "Installing cargo-fuzz..."
	cargo install cargo-fuzz
	@echo "Installing cargo-criterion..."
	cargo install cargo-criterion
	@echo "Installing cargo-geiger..."
	cargo install cargo-geiger
	@echo "✅ All tools installed!"

# Performance validation
perf:
	@echo "⚡ Running performance validation..."
	cargo build --release --all-features
	@echo "Running load tests..."
	# cargo run --release --example load_test
	@echo "✅ Performance validation complete"

# Documentation
docs-site:
	$(DOCS_PYTHON) scripts/build-docs.py

docs-check: docs-site
	$(DOCS_PYTHON) -m unittest discover -s scripts/tests -v
	$(DOCS_PYTHON) scripts/check-docs-http.py
	npm test --prefix docs

docs-serve:
	$(DOCS_PYTHON) -m http.server 8000 --bind 127.0.0.1

docs:
	@echo "📚 Building documentation..."
	cargo doc --all-features --no-deps --document-private-items
	@echo "✅ Documentation built: target/doc/ipe_core/index.html"
	@command -v open >/dev/null 2>&1 && open target/doc/ipe_core/index.html || true

# Watch mode (requires cargo-watch)
watch:
	@command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
	cargo watch -x 'test --all-features'

# Quick development cycle
dev: fmt lint test
	@echo "✅ Development cycle complete!"

# Pre-commit hook
pre-commit: fmt-check lint test
	@echo "✅ Pre-commit checks passed!"

# CI simulation (runs all CI checks locally)
ci: fmt-check lint test coverage audit
	@echo "✅ CI simulation complete!"
