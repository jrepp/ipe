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

## Design documentation

Use [docs-cms](docs-cms/README.md) for new requirements, proposals, decisions, and findings.
Read relevant structured documents and existing guides before changing architecture, schemas, or process.
Keep implemented behavior in the existing guides. Link to those guides instead of copying them.
Use Draft for new PRDs and RFCs, Proposed for unapproved ADRs, and no status for memos.
Do not mark a decision Accepted without explicit human approval. Record the approval source.
Continue main RFC numbering across docs-cms/rfcs and rfcs; preserve existing paths and IDs.
After structured-document changes, run `make docs-cms-check`. Report design coverage separately from implementation evidence.

## Read when needed

Open the guides for your task. Follow their links for additional detail.

| Task | Start here |
| --- | --- |
| Product scope | [README](README.md), [mission](MISSION.md), [roadmap](ROADMAP.md) |
| Runtime, integration, or shared boundaries | [Code map and invariants](.agent/repository.md) |
| Runtime or HTTP APIs | [Runtime data](docs/RUNTIME_DATA.md) |
| Integration, CLI, MCP, or discovery | [Discovery](docs/INTEGRATION_DISCOVERY.md), [stack and reuse](docs/INTEGRATION_STACK.md) |
| Script adapters, SDK, or package cache | [SDK and package cache](docs/INTEGRATION_SDK.md), [code map and invariants](.agent/repository.md) |
| Requirements, new designs, or decisions | [Design documentation](docs-cms/README.md), [project config](docs-cms/docs-project.yaml) |
| Architecture or policy semantics | [RFC index](rfcs/INDEX.md), [constraints and projections](rfcs/013-domain-constraints-and-projections.md) |
| Tests, builds, performance, commits, or PRs | [Validation and review](.agent/validation.md) |
| Deployment | [Deployment guide](deploy/README.md) |
| Policy and integration UX | [IPE design skill](.agents/skills/ipe-design/SKILL.md) |
| Documentation or UX | [Documentation index](docs/INDEX.md), [writing and terms](.agent/writing.md) |

Use [the repository map](docs/REPOSITORY.md) for component ownership. Legacy parser and JIT documents are historical; verify their claims against current source and CI.
