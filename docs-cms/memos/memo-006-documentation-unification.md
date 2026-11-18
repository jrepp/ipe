---
title: Documentation unification and compatibility
author: IPE maintainers
created: 2026-11-18
tags: [documentation, migration]
id: memo-006
project_id: ipe
doc_uuid: 36ac5a51-7cc4-4780-961b-63b342299cdd
---

# Context

On 2026-11-18, the project owner requested completion of Docuchango cleanup and unified project documentation.
[ADR 001](../adr/adr-001-hybrid-documentation.md) records the earlier hybrid layout and path-preservation choice.
This cleanup changes canonical guide names in response to the later request. It preserves the previous paths as compatibility pages.
The earlier decision remains a historical record. This memo does not mark a new architectural decision Accepted.

# Result

The [documentation map](../wiki/pages/documentation-map.md) connects workload guides, design records, operations, and the published site.
The Docuchango wiki subproject validates its focused pages alongside the existing numbered records.
Twenty-five uppercase top-level guide paths now link to lowercase canonical guides and corresponding sections.
Conventional `README.md` and `AGENTS.md` names remain. Legacy RFC paths and structured document identifiers remain unchanged.

The compatibility pages keep old URLs and section headings. They display links to the new sections; they do not redirect the browser automatically.
External bookmarks cannot be fully inventoried. Keep the old paths until a later reviewed retention decision.
The checked-in [compatibility test](../../scripts/tests/test_doc_compatibility.py) checks the guide pairs and their packaged section links.

# Limits

Docuchango validates configured numbered records and wiki pages, not every operational guide or historic RFC.
The site and repository checks have separately documented scopes. No Docusaurus build, runtime behavior, or live provider is established by these documentation checks.
Do not move implementation claims into design records. Keep each current guide as the canonical source for its operational behavior.
