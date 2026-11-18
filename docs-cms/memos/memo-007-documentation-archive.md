---
title: Documentation simplification and historical archive
author: IPE maintainers
created: 2026-11-18
tags: [documentation, research]
id: memo-007
project_id: ipe
doc_uuid: 9c065a40-f0c6-493c-975b-5f78d1d85f02
---

# Findings

The project owner asked to simplify documentation and identify material suitable for archive on 2026-11-18.
The previous documentation index repeated navigation, historical plans, RFC listings, and contributor commands.
Some older documents describe the removed interpreter and JIT. Several test reports describe dated branch-specific runs.
They are useful historical records but do not establish current behavior, coverage, or performance.

# Changes

The [documentation index](../../docs/documentation-index.md) now links to current tasks and the [archive](../../docs/archive/index.md).
The archive groups the earlier engine plans, test reports, and archived site examples.
Historical files remain at their existing paths so links and internal relative references continue to work.
The previously published uppercase index retains its old headings and links to the new index.
The RFC index now points to the existing original RFC instead of a nonexistent archive path.

# Limits

This classification does not claim that every historical document has been found or reviewed.
The older RFC series retains its existing status and paths. Review an individual RFC before marking it superseded or moving it.
The archived test results are observations from their stated runs, not evidence for the present revision.
