---
id: prd-005
title: Product site and verified delivery
status: Draft
author: IPE maintainers
created: 2026-11-17
target_release: Unscheduled
tags: [product, project]
project_id: ipe
doc_uuid: 26e6b2e8-5e8f-4227-8365-7f8abce99e35
---

# Project

Project P5 in the [project map](../PROJECTS.md).
Explain IPE's product, show the project routes, and publish checked static documentation through GitHub Pages.
The user explicitly requested Astra for static-site work and automatic GitHub Pages publishing on 2026-11-17.

# Users and problem

A prospective user needs to understand the problem IPE solves, current capabilities, and what is planned.
A contributor needs a route from product intent to requirements, source, and verification.
A maintainer needs repeatable publication that does not present failed or unreviewed work as a current release.

# Existing foundation

The repository has a static HTML documentation site, a Markdown viewer, a portable archive, and GitHub Pages workflow.
The [design index](../README.md) adds structured PRDs, RFCs, and decisions.
The [merge queue guide](../../docs/merge-queue.md) describes the proposed rollout of validated merge automation.
A local build does not establish that the current changes are deployed.

# Route and scope

Explain → Validate → Publish.
P5 owns product messaging, project navigation, static documentation packaging, and publication status.
Each product project owns its detailed requirement statements. Site text links to those records rather than redefining them.

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-1 | Present a clear product message backed by implemented behavior, with planned capabilities identified. | Must have |
| FR-2 | Show a subway map of project routes, shared milestones, dependencies, and PRD links. | Must have |
| FR-3 | Automatically publish a validated static artifact from main through GitHub Pages, with a review artifact for PRs. | Must have |
| NFR-1 | Keep the site usable under the repository path and accessible without depending solely on the visual map. | Must have |

## AC-1: Honest product message

Requirement: FR-1

Given the landing page, a reader can identify who IPE serves, what it does today, and the next product outcomes.
Claims of performance, savings, compliance, and production readiness require evidence or remain explicitly unclaimed.
The separately implemented runtime and integration host are not presented as a completed integrated enforcement product.

## AC-2: Project navigation

Requirement: FR-2

Given a project route, its station labels show the user outcome, delivery state, and shared dependencies.
Each route links to its owning PRD. Selecting a station or equivalent list link reaches useful design context.
Crossings represent explicit handoffs, not inferred implementation completion or dates.

## AC-3: Checked publication

Requirement: FR-3

Given a PR, the build produces a reviewable archive without publishing production changes.
Given an eligible main push, successful required documentation and workflow checks precede deployment of that artifact.
A failed check prevents publication and leaves the prior site available.
The run records the source revision and deployment result. A local-only result is not labeled published.

## AC-4: Portable and accessible navigation

Requirement: NFR-1

Given the packaged site served under /ipe/, project links, Markdown documents, assets, and diagrams resolve correctly.
Keyboard users and narrow screens can reach all projects through a text/list alternative.
A missing data or rendering dependency produces a clear fallback rather than a false project status.

# Boundaries and next slice

Use GitHub Pages, not a second hosting service or an application backend.
Keep source-system credentials and private runtime state out of published artifacts.
Publishing user-authored or unrelated staged work requires its own review; the site project does not imply approval of all local files.
The first slice updates messaging and the project map, validates the archive, and confirms the Pages workflow configuration.
A public deployment follows from reviewed source on main. Record remaining permissions or failed checks as explicit rollout limits.

# Verification

Use the existing site interaction, packaging, and HTTP checks, plus project-route and fallback checks where behavior changes.
Verify Pages configuration and a completed workflow/deployment when available.
Treat static-site tests, merge-queue activation, and live publishing as different evidence.
