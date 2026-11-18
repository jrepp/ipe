---
id: adr-001
title: Adopt hybrid documentation
status: Accepted
deciders: Project owner
created: 2026-11-17
tags: [design, documentation]
project_id: ipe
doc_uuid: fa549864-4713-47c7-8c9d-7d0be377eef8
---

# Context

The project has runtime guides, integration guides, design RFCs, agent guides, and historical material.
Their locations are already linked from source, tests, and the documentation site.
New feature requests need stable requirements and links to design and verification.

# Decision

The project owner requested Docuchango adoption with support for existing documentation on 2026-11-17.
This decision records that documentation choice only. It does not approve the feature designs.

Use [docs-cms](../README.md) for new PRDs, RFCs, ADRs, and memos.
Keep existing documentation at its current path. Link to it instead of copying it.
The [project config](../docs-project.yaml) defines four document types and enables requirement traceability for PRDs and RFCs.
The bundled [schema](../docs-project.schema.json) defines the configuration format.

- PRDs own requested outcomes, requirement IDs, and acceptance criteria.
- RFCs own proposed mechanisms, design coverage, verification plans, and open decisions.
- ADRs record decisions and their approval source.
- Memos record findings and their evidence limits.
- Existing guides own implemented behavior, procedures, and public API examples.
- Source and tests establish implemented behavior when documentation differs.

Use Draft for new PRDs and RFCs. Use Proposed for an ADR without explicit approval.
A completed design or passing documentation check does not establish implementation.

Continue the main RFC number sequence across both directories. The first new RFC is rfc-018.
The separate historical docs/rfcs sequence remains path-qualified and unchanged.
Do not renumber existing documents or rewrite their status to fit the new schema.

# Validation and migration

Run `make docs-cms-check` for structured documents. Keep `make docs-check` for the existing documentation site.
The new check validates metadata, names, links, and requirement references in the configured folders.
It does not certify all existing documentation or feature behavior.

Docuchango supports generic document types, custom naming rules, and subprojects.
Use those options if a later migration brings existing folders into validation.
First inventory their existing link and metadata errors. Do not weaken checks to hide those errors.
Do not add frontmatter to every existing guide as part of this adoption.

# Consequences

Existing links and the documentation site remain usable. New work gets reviewable requirements and design coverage.
Two documentation styles remain. The [design index](../README.md) and [existing index](../../docs/INDEX.md) connect them.
Authors must update implementation guides when a proposed feature ships.
Revisit the structure if navigation or duplicate requirements become difficult to maintain.
