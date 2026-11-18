---
title: Workload structure and documentation migration review
author: IPE maintainers
created: 2026-11-18
tags: [architecture, documentation, repository]
id: memo-005
project_id: ipe
doc_uuid: 4d3a6fce-78ca-47d9-b278-665987e646e8
---

# Overview

IPE has useful runtime and integration foundations, but the repository has not yet established a complete product distribution and maintenance model.
The project owner requested a review of prototype growth, workload shape, and documentation conventions on 2026-11-18.
This memo records observations and recommendations, not an approved architecture decision or a production-readiness claim.
The [workload guide](../../docs/workloads.md) is the current artifact inventory; the [project map](../PROJECTS.md) describes product outcomes.

The recommended next step is to make each existing workload operable, testable, and releasable at its present boundary.
Then implement one evidence-to-enforcement path with explicit ownership and recovery.
Do not reorganize every crate or introduce new services merely to match the five product lanes.

# Documentation assessment

Docuchango's pinned 1.20.0 bootstrap recommends numbered, lowercase kebab-case names for structured ADRs, RFCs, PRDs, and memos.
The configured [project](../docs-project.yaml) validates those four types under `docs-cms` and checks PRD/RFC traceability.
The site has its own packaging and checks. The validator does not scan every existing guide.
The [hybrid ADR](../adr/adr-001-hybrid-documentation.md) explicitly preserves legacy paths and leaves implemented behavior in existing guides.
The project has therefore adopted Docuchango for structured design records, not migrated all documentation into its model.

Uppercase names such as `runtime-data.md` are legacy guide paths; they are not evidence of a Docuchango validation failure.
They make naming inconsistent and obscure the path to a coherent documentation tree.
A 2026-11-18 filename inventory found 30 all-uppercase Markdown basenames across `docs`, `rfcs`, and `docs-cms`.
That count excludes mixed-case names and root files; it is a naming inventory, not a content-quality assessment.
The project map is a configured special overview, not a numbered PRD or RFC.

The site viewer defaults to `docs/documentation-index.md` and rewrites relative Markdown links.
The site builder includes top-level guides, structured records, and Git-tracked Markdown.
Repository checks name selected entry paths directly. Old links can exist outside this repository and in earlier releases.
A bulk rename would therefore need explicit compatibility and cannot be proven safe by a local link scan alone.
See [viewer](../../docs/doc.js), [site builder](../../scripts/build-docs.py), and [repository check](../../scripts/check-repository.py).

## Recommended convention and migration

1. Name new ordinary guides with lowercase kebab-case. Keep conventional entry files such as `README.md` and required `AGENTS.md` where their tools expect them.
2. Keep numbered Docuchango records in their existing folders with stable IDs, UUIDs, and status rules. Continue the existing RFC sequence.
3. Select one guide at a time for migration based on readership and ownership. Inventory inbound repository links, site links, bookmarks, and published URLs first.
4. Move the canonical content to a lowercase path. Keep the old Markdown path as a clear link to the new guide; preserve important heading anchors or supply tested site redirects when needed.
5. Update source, tests, site navigation, and docs-build inputs together. Check the old and new paths under the `/ipe/` Pages prefix and in GitHub source views.
6. Pilot Docuchango generic-document validation on a small owned guide folder with a reviewed config and metadata. Record its coverage and existing errors before expanding it.
7. Remove compatibility paths only after an explicit retention decision informed by published-link use. Do not assume external links can be inventoried completely.

This guide is the first lowercase workload entry, not a rename of an existing public guide.
The bundled schema supports generic document types, naming standards, and subprojects; a config entry alone does not establish that a migration is complete.
Test nested guide packaging explicitly before introducing folders: the current site builder directly globs only top-level `docs/*.md`, then adds tracked Markdown.
The current `docs-cms` security setting disallows external paths. Evaluate a dedicated config or supported subproject layout before scanning other roots.
Do not force all legacy documents into a new schema or use automatic repair without reviewing changes.

# Current workload and delivery assessment

The [workload guide](../../docs/workloads.md) identifies the binaries, their modes, shared libraries, and supporting artifacts.
It links to implemented contracts rather than repeating API procedures.
The following findings identify concrete gaps, not defects inferred from file names alone.

| Finding | Evidence | Consequence |
| --- | --- | --- |
| Integration and enforcement are separate processes with separate stores; integration does not publish enforcement facts. | [Integration host](../../docs/integration-host.md), [runtime data](../../docs/runtime-data.md), [PRD 003](../prd/prd-003-evidence-integration.md) | Product diagrams and releases must not present a connected evidence path as implemented. |
| Hosted integration and discovery share a package and executable, but have different configuration and API modes. | [Integration entry point](../../crates/ipe-integration/src/main.rs) | Document and test each mode independently before deciding whether to split packages. |
| The integration host combines API routes, scheduling, storage, and execution coordination in its host module. | [Host module](../../crates/ipe-integration/src/host/mod.rs) | This is a review and change-isolation risk as features grow, not proof that the module is currently incorrect. |
| Several workspace packages are placeholders. Mock interfaces and architecture pages are separate review artifacts. | [Repository map](../../docs/repository-guide.md) | A package name, interface image, or build target must not be presented as a completed product. |
| Release-mode CI checks build outputs, but the release-notes workflow does not attach assets. | [CI](../../.github/workflows/ci.yml), [release guide](../../docs/release-guide.md) | The project lacks a defined, automated distribution contract for each supported artifact. |
| Docker and Helm examples use an older profile. | [Deployment guide](../../deploy/README.md) | They are not evidence of a deployable current runtime or an integration deployment. |
| Prototype interfaces, synthetic provider runs, and archived measurements have different evidence value. | [Integration discovery](../../docs/integration-discovery.md), [site guide](../../docs/README.md) | Maintain explicit labels and keep them out of production capability and performance claims. |
| Current coverage and older harnesses need independent review. | [RFC 019 remaining review](../rfcs/rfc-019-cohesive-monorepo.md) | A green workflow alone does not establish a measured coverage percentage or complete browser validation. |

# Recommended development structure

The following is a proposed logical ownership map within the existing monorepo. It does not require immediate directory moves, new crates, or new services.
An owner is a role to assign, not a claim that a team already exists.

| Area | Responsibility and boundary | Evidence required for sustained work |
| --- | --- | --- |
| Enforcement workload | `ipe-server` and runtime evaluation, scoped facts, policy revisions, and authorization. | Versioned configuration and API, upgrade/recovery tests, deployment instructions, and per-release artifact checks. |
| Integration workload | Brokered source access, sandbox execution, scheduling, local records, and publication. | Per-source contract fixtures, grant and budget tests, recovery and expiry tests, and a supported configuration for each mode. |
| Shared contracts | Narrow schemas and semantics for identities, observation, fact publication, and diagnostics across workloads. | Version compatibility, malformed inputs, tenant scope, replay, expiry and independent producer/consumer tests. Keep domain authority outside generic serialization helpers. |
| Operator and agent tools | Toolkit/MCP, contract capture, inspection, and future administrative interfaces. | Document which service each tool accesses, its grants and lifecycle, and supported CLI/protocol compatibility. A tool cannot grant its own authority. |
| Design and documentation | Product map, structured records, operational guides, mocks, and static site. | A named canonical source, status, review owner, traceable requirements, link compatibility, and checked publication per change. |
| Delivery and operations | Builds, artifact distribution, configuration, upgrades, telemetry, and incident recovery. | Repeatable packages, target matrix, provenance and checksums, deployment smoke tests, rollback and security/maintenance ownership. |

Use the existing [repository map](../../docs/repository-guide.md) for exact code ownership and the [platform research](memo-004-shared-platform-spike.md) for dependency reuse.
Assign accountable maintainers and review responsibilities before treating these logical areas as teams or creating new package boundaries.
Split a module or crate when a real change shows a stable interface, separate test boundary, or dependency benefit.
Keep integration source credentials and guest grants separate from enforcement policy activation.

# Prioritized work

1. **Establish a release baseline.** Define supported target artifacts, version compatibility, checksums, configuration, and the person responsible for release. Exercise install, startup, failure, upgrade, and rollback for each selected workload.
2. **Repair deployment guidance.** Design and test a current runtime configuration and secret handoff before calling the Docker or Helm profile supported. Decide separately whether the integration host needs a deployable profile.
3. **Connect one evidence path.** Implement the reviewed P2→P3 contract, including identity, scope, original expiry, acknowledgement, duplicate handling, bounded work, and recovery. Validate with real workload processes and a synthetic source first.
4. **Make changes local.** Review the host module's route, store, scheduler, and execution responsibilities when the next behavior change touches them. Extract only boundaries supported by tests and current ownership.
5. **Treat design artifacts as products of review.** Record source, sample-data status, export and accessibility checks, and owner. Separate prototypes from supported interfaces in navigation and release claims.
6. **Migrate documentation in slices.** Pilot one high-value guide and generic validation; preserve old paths and validate Pages and source links. Record unresolved public-link limitations.
7. **Resolve known assurance gaps.** Review coverage and legacy browser/deployment harness claims without weakening checks. Record measured evidence separately from design coverage.

These steps are ordered by dependency and risk, not by calendar. The connected-path design and release policy need review before implementation.
Reconsider a crate split if independent release cadence or a concrete dependency boundary emerges; retain the shared package if it adds coordination without isolation.

# Verification and limits

This review used the workspace manifests, workload entry points, current guides, release workflow, Docuchango bootstrap, configuration schema, and site tooling.
It is a bounded structural assessment; it is not a full security audit, user study, live-provider validation, performance test, or proof of production readiness.
The proposed responsibilities and migration steps are design recommendations. A documentation check proves metadata and link integrity within its configured scope, not runtime behavior.
