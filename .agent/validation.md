# Validation and review

Choose checks for the affected behavior. Run commands from the repository root unless stated otherwise.

## Build and validation

Rust uses edition 2021. Normal development uses the stable toolchain. Native builds can require a C/C++ toolchain, libclang, and CMake for RocksDB.

Runtime acceptance requires Python 3.10 or newer and OpenSSL. Integration acceptance requires Node 22.18 or newer and OpenSSL. The docs builder uses the exact Python version in [.python-version](../.python-version), normally through `uv`.

| Change | Commands |
| --- | --- |
| Format Rust | `cargo fmt --all -- --check` |
| Lint Rust | `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` |
| Test default features | `cargo test --locked --workspace` |
| Test all features | `cargo test --locked --workspace --all-features` |
| Runtime storage and policies | `cargo test --locked -p ipe-core --features approvals` |
| Server boundaries | `cargo test --locked -p ipe-server` |
| Shared JSON and integration | `cargo test --locked -p ipe-json -p ipe-integration` |
| Runtime HTTP acceptance | `make runtime-acceptance` |
| Policy documents and imports | `make policy-acceptance` |
| Integration acceptance | `cargo build --locked -p ipe-integration --bins`, then `cargo test --locked -p ipe-integration --test discovery_acceptance -- --ignored` |
| TypeScript provider | In `testing/integration`: `npm ci`, `npm run check`, and `npm test` |
| Integration package cache | `npm ci --ignore-scripts --prefix tools/integration-packages`, then `npm test --prefix tools/integration-packages` |
| Hosted integrations | `cargo test --locked -p ipe-integration host --lib`; `node --test tools/integration-host/*.test.cjs`; see [live proof](../docs/INTEGRATION_HOST.md) |
| GitHub contract capture | `cargo test --locked -p ipe-integration github_contract --lib`; see the [live proof and view checks](../docs/GITHUB_CONTRACT_SPIKE.md#proof-and-next-questions) |
| Runtime performance | `cargo bench --locked -p ipe-core --features approvals --bench v2_evaluation` |

The `approvals` feature enables the runtime modules in `ipe-core`. Default-feature tests alone do not exercise them. Test both feature configurations when changing feature gates or dependencies.

For script adapter or SDK changes, run the shared JSON/integration tests and integration acceptance checks above. After `npm ci --prefix testing/integration`, check the typed adapter example:

```sh
testing/integration/node_modules/.bin/tsc --noEmit --strict --target ES2022 --module NodeNext --moduleResolution NodeNext --types node testing/integration/adapters/read-step.ts --typeRoots testing/integration/node_modules/@types
```

For Rust changes, run formatting, lint, and the affected tests. Before handoff, run workspace tests when shared behavior or dependencies change. Run acceptance checks when API or trust-boundary behavior changes. For documentation-only changes, check links and examples; Rust tests are not normally needed.

Measure performance changes against the same workload and environment. Do not reuse historical latency or coverage claims as new results.

For documentation-site changes:

```sh
npm ci --prefix docs --ignore-scripts
make docs-check
```

See the [Makefile](../Makefile) and the [runtime](../.github/workflows/runtime-data.yml), [integration](../.github/workflows/integration.yml), and [general CI](../.github/workflows/ci.yml) workflows for exact automation. Some `justfile` and benchmark targets reference removed legacy code.

Acceptance guides explain prerequisites, cleanup, and report paths: [runtime](../acceptance/runtime-data/README.md) and [integration](../acceptance/integration-discovery/README.md). Keep generated credentials temporary. Stop processes started for a test or preview.

## Commits and review

Use Conventional Commits, such as `fix(runtime): reject expired evidence`. Keep messages technical. Omit emoji, tool branding, and attribution tags.

[Commit validation](../scripts/check-commit-msg) and [pre-commit hooks](../.pre-commit-config.yaml) define the local checks. Do not weaken checks to make a change pass. Report existing failures separately.

A PR description should state the problem, the resulting behavior, and the verification performed. Include compatibility changes and material limitations. Distinguish proposed designs from implemented behavior.
