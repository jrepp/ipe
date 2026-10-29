# Agent guide

IPE is a Rust workspace for policy enforcement and external-system integration.

## Working rules

- Check the branch and local changes. Preserve unrelated work.
- Read the relevant guide, source, and tests before editing. Distinguish implemented behavior from proposals.
- State assumptions that affect scope or behavior. Resolve routine questions from repository evidence; ask when a missing decision blocks progress.
- Define a verifiable outcome. Reproduce bugs and test changed behavior, including relevant failure paths.
- Make the smallest complete change. Reuse existing code. Avoid speculative features, configuration, abstractions, and unrelated cleanup.
- Preserve security checks and bounded work. Keep credentials out of arguments, URLs, logs, and reports.
- Update affected documentation and describe user-visible changes in the PR. GitHub generates the changelog; follow the [release guide](docs/RELEASES.md).
- Run the applicable checks. Report results, limitations, and checks that could not run. Do not weaken checks.
- Use ASD-STE100 for UX text, documentation, and reports. Use active voice, familiar words, and consistent terms; see [writing guidance](.agent/writing.md).

## Read when needed

Open the guides for your task. Follow their links for additional detail.

| Task | Start here |
| --- | --- |
| Product scope | [README](README.md), [mission](MISSION.md), [roadmap](ROADMAP.md) |
| Runtime, integration, or shared boundaries | [Code map and invariants](.agent/repository.md) |
| Runtime or HTTP APIs | [Runtime data](docs/RUNTIME_DATA.md) |
| Integration, CLI, MCP, or discovery | [Discovery](docs/INTEGRATION_DISCOVERY.md), [stack and reuse](docs/INTEGRATION_STACK.md) |
| Script adapters, SDK, or package cache | [SDK and package cache](docs/INTEGRATION_SDK.md), [code map and invariants](.agent/repository.md) |
| Architecture or policy semantics | [RFC index](rfcs/INDEX.md), [constraints and projections](rfcs/013-domain-constraints-and-projections.md) |
| Tests, builds, performance, commits, or PRs | [Validation and review](.agent/validation.md) |
| Deployment | [Deployment guide](deploy/README.md) |
| Documentation or UX | [Documentation index](docs/INDEX.md), [writing and terms](.agent/writing.md) |

Legacy parser and JIT instructions remain in older documents, including `.agent/core-prompt.md`. Verify historical guidance against current source and CI.
