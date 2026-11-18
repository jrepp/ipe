# Start development

Read the [repository map](docs/repository-guide.md) before choosing a component.
IPE has an experimental runtime and separate integration tools. Product designs do not establish connected runtime behavior.

## Prerequisites

Use stable Rust with rustfmt and Clippy. Integration requires Rust 1.96 or newer.
Install a C/C++ toolchain, CMake, and libclang for RocksDB builds.
Install uv for the pinned documentation Python environment.
Integration fixtures require Node 22.18 or newer and OpenSSL. Site tests require Node 22.12 or newer.

## Build and check

Run these commands from the repository root:

```sh
make build
make test-default
make test
make lint
make repo-check
```

`make check` combines repository checks, Rust formatting, lint, and both feature configurations.
`just` commands delegate to Make. The retired JIT tasks have no replacement.
`make perf` measures the current runtime with Criterion. It writes results under `target/criterion`.
It does not establish a production latency or capacity target.

## Exercise service boundaries

```sh
make runtime-acceptance
make policy-acceptance
cargo build --locked -p ipe-integration --bins
cargo test --locked -p ipe-integration --test discovery_acceptance -- --ignored
```

Read the [runtime acceptance guide](acceptance/runtime-data/README.md) and [integration acceptance guide](acceptance/integration-discovery/README.md) for prerequisites and cleanup.
These checks use temporary credentials and synthetic sources. Do not supply production credentials.
The runtime server requires configured credentials and a loopback listener; see [runtime data](docs/runtime-data.md).

## Preview documentation

```sh
npm ci --prefix docs --ignore-scripts
make docs-check
make docs-cms-check
make docs-serve
```

Open <http://127.0.0.1:8000/docs/>. Stop the preview with Ctrl-C.
See [site publishing](docs/README.md) for GitHub Pages automation.
See [validation](.agent/validation.md) for component-specific checks.
