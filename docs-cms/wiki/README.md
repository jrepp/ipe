# IPE documentation

Start with the [documentation map](pages/documentation-map.md). It links current workload guides, design records, and contributor workflows.
The numbered records remain the source for requirements, proposals, decisions, and findings.
Implementation guides describe current behavior; source and tests settle differences.

Add a focused page under `pages/` using [the template](templates/page.md).
Use lowercase kebab-case filenames and a fresh UUID. Run `make docs-cms-check` from the repository root.
The wiki is a Docuchango subproject; the parent config also validates the numbered design records.
