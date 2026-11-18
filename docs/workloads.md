# IPE workloads and artifacts

This guide describes the buildable artifacts and their current boundaries.
It does not claim that a release package or a connected production deployment exists.
For product direction, see the [project map](../docs-cms/PROJECTS.md).
For proposed changes to these boundaries, see the [workload and documentation review](../docs-cms/memos/memo-005-workload-structure.md).

## Running workloads

| Artifact | Role | Inputs and outputs | Current boundary |
| --- | --- | --- | --- |
| `ipe-server` | Enforcement workload | Loads scoped configuration and credentials; accepts fact commits, pulls configured snapshots, and evaluates policies through HTTP. | The listener is loopback-only. It persists facts in RocksDB. It does not authenticate end users through JWT or provide the proposed central control plane. See [runtime data](RUNTIME_DATA.md). |
| `ipe-integration` | Integration workload, discovery mode | Loads operator configuration; uses a native broker and sandboxed adapter to read permitted source data. | The current discovery slice is bounded and focused on ServiceNow metadata. It does not publish enforcement facts. See [discovery](INTEGRATION_DISCOVERY.md). |
| `ipe-integration --host` | Integration workload, hosted mode | Uses the same executable with a distinct configuration and API to install and schedule sandboxed programs. Stores observations, requirements, and outcomes locally. | This is a loopback, single-operator local MVP. It does not publish enforcement facts or activate policies. See [integration host](INTEGRATION_HOST.md). |

These are two executables with two integration modes, not three independently packaged services.
The integration host and enforcement server each persist their own data.
[PRD 003](../docs-cms/prd/prd-003-evidence-integration.md) and [RFC 014](../rfcs/014-fact-publication-and-synchronization.md) describe the missing publication path.
A fact in the integration host is not automatically available to an enforcement policy.

## Supporting artifacts

| Artifact | Purpose and status |
| --- | --- |
| `integration-toolkit` | CLI and stdio MCP client-facing interface for the integration workload. It is built from the `ipe-integration` package. See [discovery](INTEGRATION_DISCOVERY.md). |
| `github-contract` | Standalone GitHub source-capture and report tool. It does not execute workflows or publish policies. See [GitHub contract capture](GITHUB_CONTRACT_SPIKE.md). |
| `ipe-core` and `ipe-json` | Shared Rust libraries for runtime behavior and strict JSON decoding. They are workspace components, not separate deployed workloads. |
| Integration SDK | Guest-facing API embedded in the integration binary, with TypeScript declarations for authors. It is not a separate running service. See [SDK and package cache](INTEGRATION_SDK.md). |
| Static documentation site | A buildable site and Pages workflow. See [site and publishing](README.md). Product mocks and design exports are review artifacts, not the live workloads. |
| `ipe-control`, `ipe-parser`, `ipe-wasm`, `ipe-ffi`, `ipe-web` | Placeholder packages. Their names or library output types do not establish usable control-plane, parser, binding, or web products. See [repository map](REPOSITORY.md). |

## Build and release evidence

The workspace CI builds release-mode targets and checks the server executable is present and nonempty.
The release-notes workflow creates a draft; it does not build, attach, or publish binary assets.
The [release guide](RELEASES.md) requires a person to attach validated assets and publish the release.
The Docker and Helm examples describe an older server profile and need migration before use with the current runtime.
See the [deployment guide](../deploy/README.md) for that limitation.

The current guides and acceptance tests define configuration and validation for each mode.
They do not establish a supported distribution matrix, upgrade policy, production deployment, or end-to-end source-to-enforcement flow.
