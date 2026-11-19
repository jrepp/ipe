---
title: Documentation conventions
project_id: ipe
doc_uuid: f17a566d-cb9b-4b60-a359-547e252bf3fa
created: 2026-11-18
tags: [documentation, wiki]
---

# Documentation conventions

Use the [documentation map](documentation-map.md) to find existing material before adding a new document.
Keep implemented behavior in operational guides. Use the configured design records for proposals, decisions, requirements, and findings.
Link to the canonical guide instead of copying its procedures.

Use lowercase kebab-case for new guide and wiki filenames. Preserve conventional `README.md` and `AGENTS.md` entry names.
Number ADRs, RFCs, PRDs, and memos with their existing type sequence and retain their IDs and UUIDs.
Continue the RFC sequence across the structured and legacy RFC directories.
Use Draft for new PRDs and RFCs and Proposed for decisions without explicit approval. Memos have no status.

The older uppercase guide paths are compatibility pages. Update canonical lowercase guides, not those pages.
Keep compatibility sections while published links may use them. Their links lead to the matching canonical section.
Historic RFC paths and IDs remain unchanged. Do not infer acceptance from a successful validation check.

Run `make docs-cms-check` for structured records and wiki pages. Run `make docs-check` for the site and `make repo-check` when entry paths change.
See the [writing guidance](../../../.agent/writing.md) and [design index](../../README.md) for project-specific rules.
