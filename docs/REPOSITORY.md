# Repository map

The [project map](../docs-cms/PROJECTS.md) owns product scope.
This guide maps that scope to existing code. The [code guide](../.agent/repository.md) owns implementation invariants.
The [cleanup design](../docs-cms/rfcs/rfc-019-cohesive-monorepo.md) records decisions, findings, and remaining work.

## Rust workspace

Every workspace member appears once below. Project identifiers refer to the product map.
Shared code supports several projects; it does not imply a separate deployed service.
Placeholder packages retain their paths and public module names. They contain no implemented product behavior.

| Component | State | Project | Responsibility |
| --- | --- | --- | --- |
| `crates/ipe-core` | implemented | P1, P3 | Policy documents, facts, evaluation, approvals, and relationships |
| `crates/ipe-server` | implemented | P3 | Authenticated runtime HTTP service and pull connector |
| `crates/ipe-integration` | implemented | P2, P4 | Discovery, adapters, hosted records, and local review APIs |
| `crates/ipe-json` | implemented | P2, P3 | Strict JSON decoding at shared boundaries |
| `crates/ipe-parser` | placeholder | P1 | Reserved parser package; the old parser was removed |
| `crates/ipe-control` | placeholder | P1, P3 | Reserved control-plane package |
| `crates/ipe-wasm` | placeholder | P3 | Reserved bindings package; distinct from the integration adapter host |
| `crates/ipe-ffi` | placeholder | P3 | Reserved native bindings package |
| `crates/ipe-web` | placeholder | P4 | Reserved web package; product previews live under tools |

## Other components

| Location | Project | Contract |
| --- | --- | --- |
| [Integration fixtures](../testing/integration/README.md) | P2 | Synthetic source protocols and independent clients |
| [Package cache](../tools/integration-packages/) | P2 | Build-side artifacts for the integration SDK |
| [Product previews](../tools/product-ship-mocks/) | P1, P4 | Sample-data interfaces, not live enforcement |
| [Acceptance](../acceptance/) | P2, P3 | External service tests and fixtures |
| [Site](README.md) | P5 | Static publication, archived measurements, and document viewer |
| [Design records](../docs-cms/README.md) | P1–P5 | Requirements, proposals, decisions, and findings |
| [Main RFCs](../rfcs/INDEX.md) | P1–P5 | Existing proposals at stable paths |
| [Automation](../.github/workflows/) | P5 | Build, test, review, and publication checks |

See the [shared-foundations survey](../docs-cms/memos/memo-001-shared-foundations.md) before adding general-purpose infrastructure or shared packages.

## Commands and documentation

[Makefile](../Makefile) owns contributor commands. [Just](../justfile) delegates to Make.
Use [QUICKSTART](../QUICKSTART.md) for setup and [validation](../.agent/validation.md) for change-specific checks.
Keep runtime guides in docs/, historical RFCs in rfcs/, and new design records in docs-cms/.
Do not move stable paths for appearance alone. Record a concrete navigation or ownership benefit before a move.

`make repo-check` checks workspace mapping, supported entry links, Node lockfiles, and command aliases.
Its entry-link scope is README, QUICKSTART, CONTRIBUTING, this guide, and the agent code guide.
It checks local file targets, not heading anchors, remote sites, every historical document, or source-code quality.
Docuchango and the site suite provide separate design and publication checks.

## Historical and unresolved surfaces

[ROADMAP](../ROADMAP.md), [RFC](../RFC.md), and old parser measurements describe earlier work.
[Deployment](../deploy/README.md) contains legacy Helm/TOML examples. The root Dockerfile still needs migration to current runtime configuration.
[Testing](../testing/README.md) distinguishes current provider fixtures from historical policy-tree examples.
Some old architecture documents, coverage scripts, and browser harnesses need a separate evidence-based review.
The cleanup design lists these gaps; this map does not establish their correctness.
