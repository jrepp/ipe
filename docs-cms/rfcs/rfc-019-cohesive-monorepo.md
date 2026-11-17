---
id: rfc-019
title: Cohesive monorepo and repository checks
status: Draft
author: IPE maintainers
created: 2026-11-17
tags: [design, repository]
project_id: ipe
doc_uuid: 12431ad6-87a6-467f-964a-792a31720d85
---

# Scope and outcome

Support the five routes in the [project map](../PROJECTS.md) through one repository and one contributor command surface.
This design responds to the repository cleanup request on 2026-11-17. It is not a new product project.
The first slice changes navigation, commands, placeholder status, and artifact verification. It does not change runtime authorization.
P5 owns contributor and publication workflow. P1–P4 retain ownership of their code and technical guides.

# Requirements

These local requirements define the cleanup; no separate PRD is needed for this maintenance slice.

| ID | Requirement | Basis |
| --- | --- | --- |
| FR-1 | Map each Rust workspace member to its product project and implementation state. | Contributors need a source location and an honest status. |
| FR-2 | Define contributor commands in Make and delegate Just aliases to them. | Duplicate command behavior has drifted. |
| FR-3 | Fail checks when required artifacts are absent; do not claim results for work that did not run. | Existing performance and release commands give misleading results. |
| FR-4 | Preserve stable source paths, security boundaries, and unrelated work during cleanup. | Organization changes must not change authority or erase history. |

## AC-1: Complete workspace map

Requirement: FR-1

The repository check fails for an unmapped, duplicate, missing, or invalid-state Rust member.
The map distinguishes implemented code from placeholders and links to product scope.

## AC-2: Consistent contributor commands

Requirement: FR-2

Each Just alias names an existing Make target. Setup links resolve to existing local targets.
Make uses locked workspace build and test commands. Default and all-feature tests remain distinct.

## AC-3: Observable work

Requirement: FR-3

Performance invokes the current runtime benchmark. Unsupported fuzzing returns failure with an explanation.
The release check measures the server binary and fails when it is missing or empty.
Placeholder crates contain no empty success tests. This does not claim complete coverage elsewhere.

## AC-4: Bounded changes

Requirement: FR-4

Existing workspace package paths and public modules remain intact. Runtime implementation and trust boundaries remain unchanged.
Historical files retain their paths with explicit status. Unrelated staged work stays unchanged.

# Design and verification coverage

| Source requirement | Acceptance criterion | Disposition | Mechanism | Verification |
| --- | --- | --- | --- | --- |
| FR-1 | AC-1 | Covered | A checked table in the repository map, compared with Cargo metadata in TOML. | Repository inventory failure cases. |
| FR-2 | AC-2 | Covered | Make owns recipes; Just delegates. Entry links have a bounded check. | Alias and missing-link failure cases; Make dry runs. |
| FR-3 | AC-3 | Covered | Real benchmark, explicit unsupported target, and required artifact check. | Missing/empty artifact tests; placeholder crate tests. |
| FR-4 | AC-4 | Covered | Keep paths and modules; edit only documented surfaces. | Diff review and staged-file comparison. |

Covered describes this design. Check results belong in the change report; they do not establish whole-repository correctness.

# Audit baseline and first slice

Findings come from source inspection on branch docs/credential-enclave, 2026-11-17, before this cleanup.

| Finding | Evidence | Action |
| --- | --- | --- |
| Just calls removed JIT features and test binaries. | Previous justfile referenced perftest_predicate_execution and feature jit. | Replace duplicated recipes with Make aliases. |
| Performance reports completion without running a workload. | Previous Make perf target contained only a build and a commented load test. | Run the v2_evaluation benchmark. |
| Release measurement accepts absent output. | CI measured nonexistent ipe-cli and converted failure into zero bytes. | Check the actual ipe-server artifact and reject missing output. |
| Placeholder tests inflate the passing test count. | Five placeholder libraries each had an empty stub_test. | Remove those tests, retain packages, and label status. |
| Entry guidance describes removed code. | QUICKSTART, STRUCTURE, and core-prompt referenced legacy engine paths. | Use current setup and one repository map; retire the competing agent prompt. |
| Node lockfile ignore rules omit two maintained packages. | .gitignore exempted only docs/package-lock.json. | Exempt the integration fixture and package-cache lockfiles. |

# Remaining review work

These are open findings, not approved scope reductions or completed repairs.

| Area | Owner | Evidence and required next outcome |
| --- | --- | --- |
| Runtime deployment | P3 | Root Dockerfile uses legacy TOML arguments and socket health checks. Define a current configuration and secret contract, then test startup and failure paths before recommending it. |
| Coverage evidence | P3, P5 | check-coverage.sh substitutes passing-test counts for low measured coverage. The per-crate job skips three removed critical files, then claims full critical coverage. Audit all coverage gates and replace unsupported exceptions with measured evidence without reducing thresholds. |
| Older documents | P1–P5 | Historical indexes and architecture guides retain old claims and links. Review by owning project before removal or archive migration. |
| Browser harnesses | P4, P5 | Older Playwright/Docker harnesses coexist with the current site suite. Identify consumers before retiring files. |
| Placeholder packages | P1, P3, P4 | Retain package compatibility now. Remove packages or implement them only through an explicit product decision and dependency review. |

# Review method

For each later slice, record the conflicting claim, source evidence, owning project, and expected observable outcome.
Read consumers before moving files. Update the canonical guide instead of adding another competing overview.
Use focused failure tests for automation changes. Report tests that were not run and live-service limits.
Do not classify unfinished product design as dead code merely because it is not implemented.
