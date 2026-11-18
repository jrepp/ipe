DOCUCHANGO ?= uvx docuchango==1.20.0

.PHONY: release test-verbose test-default fmt-check perf watch dev pre-commit ci repo-check help build test bench lint fmt check clean install-tools coverage audit fuzz docs docs-site docs-serve docs-check docs-cms-check runtime-acceptance policy-acceptance

# One exact version shared with actions/setup-python in docs CI.
DOCS_PYTHON = uv run --no-project --python $(shell cat .python-version) python

# Default target
help:
	@echo "IPE development commands (see QUICKSTART.md)"
	@echo "build release test test-default lint fmt check bench perf"
	@echo "runtime-acceptance policy-acceptance repo-check"
	@echo "docs-site docs-serve docs-check docs-cms-check"
	@echo "coverage audit clean install-tools"

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
	cargo build --locked --workspace --all-features

# Build release
release:
	cargo build --locked --workspace --release --all-features
	mkdir -p target/release/licenses
	cp licenses/*.txt target/release/licenses/
	@ls -lh target/release/ipe-* 2>/dev/null || true

# Run all tests
test:
	cargo test --locked --workspace --all-features

# Run tests with output
test-verbose:
	cargo test --locked --workspace --all-features -- --nocapture

# Run benchmarks
bench:
	cargo bench --locked --workspace --all-features

# Run clippy linter
lint:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Run all checks
check: repo-check fmt-check lint test-default test

# Generate code coverage
coverage:
	cargo llvm-cov --all-features --workspace --html
	@command -v open >/dev/null 2>&1 && open target/llvm-cov/html/index.html || true

# Run security audits
audit:
	cargo audit
	cargo deny check

# No maintained fuzz target exists. Do not create a new workspace as a side effect.
fuzz:
	@echo "No maintained fuzz target exists. See docs-cms/rfcs/rfc-019-cohesive-monorepo.md." >&2
	@exit 1

# Clean build artifacts
clean:
	cargo clean

# Install development tools
install-tools:
	rustup component add rustfmt
	rustup component add clippy
	rustup component add llvm-tools-preview
	cargo install cargo-llvm-cov
	cargo install cargo-audit
	cargo install cargo-deny
	cargo install cargo-fuzz
	cargo install cargo-criterion
	cargo install cargo-geiger

# Measure the current runtime workload; Criterion writes target/criterion.
perf:
	cargo bench --locked -p ipe-core --features approvals --bench v2_evaluation

test-default:
	cargo test --locked --workspace

repo-check:
	$(DOCS_PYTHON) scripts/check-repository.py
	$(DOCS_PYTHON) -m unittest discover -s scripts/repository-tests -v

# Documentation
docs-cms-check:
	$(DOCUCHANGO) validate --dry-run --verbose --skip-build

docs-site:
	$(DOCS_PYTHON) scripts/build-docs.py

docs-check: docs-site
	$(DOCS_PYTHON) -m unittest discover -s scripts/tests -v
	$(DOCS_PYTHON) scripts/check-docs-http.py
	npm test --prefix docs

docs-serve: docs-site
	$(DOCS_PYTHON) -m http.server 8000 --bind 127.0.0.1 --directory target/docs-site

docs:
	cargo doc --locked --workspace --all-features --no-deps --document-private-items
	@command -v open >/dev/null 2>&1 && open target/doc/ipe_core/index.html || true

# Watch mode (requires cargo-watch)
watch:
	@command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
	cargo watch -x 'test --locked --workspace --all-features'

# Quick development cycle
dev: fmt lint test

# Pre-commit hook
pre-commit: fmt-check lint test

# Local Rust checks; acceptance, documentation, and CI platform checks are separate.
ci: fmt-check lint test coverage audit
