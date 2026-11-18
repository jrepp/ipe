# IPE design documentation

Use this index for requirements, proposals, decisions, and findings.
Use the [existing documentation index](../docs/INDEX.md) for implemented behavior, operations, and historical references.
The [hybrid documentation decision](adr/adr-001-hybrid-documentation.md) defines the boundary.

## Project organization

Start with the [project map](PROJECTS.md) for principles, subway routes, dependencies, delivery states, and product messaging.

## Current documents

| Document | State | Purpose |
| --- | --- | --- |
| [ADR 001: Hybrid documentation](adr/adr-001-hybrid-documentation.md) | Accepted | Preserve existing paths and add structured design memory. |
| [PRD 001: Evidence system map and temporary authority](prd/prd-001-evidence-system-map.md) | Draft | Requested outcomes, acceptance criteria, and open product decisions. |
| [RFC 018: Evidence system map and temporary authority](rfcs/rfc-018-evidence-system-map.md) | Draft | System roles, evidence links, temporal controls, exceptions, grants, and planned verification. |
| [PRD 002: Policy modeling](prd/prd-002-policy-modeling.md) | Draft | Sourced models, candidate review, and controlled activation. |
| [PRD 003: Evidence integration](prd/prd-003-evidence-integration.md) | Draft | Permitted collection, lineage, freshness, and publication. |
| [PRD 004: Policy enforcement](prd/prd-004-policy-enforcement.md) | Draft | Snapshot evaluation, application gates, and active-revision reports. |
| [PRD 005: Product site and delivery](prd/prd-005-product-site-and-delivery.md) | Draft | Product message, project navigation, and checked Pages publication. |

No memos have been added yet. Draft designs do not establish implemented behavior.

## Where documents belong

| Content | Location and rule |
| --- | --- |
| Implemented behavior and procedures | Keep in [docs](../docs/INDEX.md), [deployment](../deploy/README.md), and acceptance guides. |
| Existing proposals | Keep [main RFCs](../rfcs/INDEX.md) and [historical retrieval proposal](../docs/rfcs/0003-fact-retrieval-api.md) at their current paths. |
| New requirements | Use prd/ and the PRD template. Define stable requirement and acceptance IDs. |
| New designs | Use rfcs/ and the RFC template. Reference the owning PRD with implements. |
| Decisions | Use adr/ and the ADR template. Record who approved the decision. |
| Findings | Use memos/ and the memo template. Record sources and limits. |
| Agent workflow | Keep [AGENTS.md](../AGENTS.md) and [.agent guides](../.agent/repository.md). |

Do not copy existing guides into this directory or retroactively change proposal status.
Continue main RFC numbering across both locations; rfc-018 follows the existing RFC 017.
Check both indexes before allocating the next number. Preserve the separate historical docs/rfcs numbering.

[Repository cleanup design](rfcs/rfc-019-cohesive-monorepo.md) describe component ownership, contributor commands, and remaining audit work.

[Shared-foundations research](memos/memo-001-shared-foundations.md) compares permissive Rust packages for common needs across the five lanes.

## Authoring and checks

Copy the appropriate file from templates/. Set its title, ID, project_id, creation date, and a fresh UUID v4.
Keep the UUID stable. Use Draft for PRDs and RFCs, Proposed for unapproved ADRs, and no status for memos.
Derive later modification dates from Git instead of adding updated frontmatter.
Use [project vocabulary](../docs/VOCABULARY.md) and [writing guidance](../.agent/writing.md).

PRDs own requirements. RFCs reference them and record Covered, Deferred, or Unresolved design coverage.
Keep unresolved decisions visible. A coverage table is not proof of working code.

```sh
make docs-cms-check
# Apply reviewed, automatic repairs when needed:
uvx docuchango==1.20.0 validate --skip-build
```

The check uses Docuchango 1.20.0 and [docs-project.yaml](docs-project.yaml).
It validates configured ADR, RFC, memo, and PRD folders, including links and PRD/RFC traceability.
The root project map uses generic title, project, and UUID metadata and link validation without a numbered record ID.
It does not scan every existing guide. Templates remain authoring support files. The project map is an unnumbered overview, not an ADR or PRD.
Keep existing `make docs-check` site validation. No Docusaurus migration is required.

The [configuration schema](docs-project.schema.json) supports mixed document types and subprojects for later adoption.
Expand validation only after reviewing the target folder. Preserve historical content and report existing errors separately.
