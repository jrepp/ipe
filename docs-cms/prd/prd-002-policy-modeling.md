---
id: prd-002
title: Policy modeling and review
status: Draft
author: IPE maintainers
created: 2026-11-17
target_release: Unscheduled
tags: [product, project]
project_id: ipe
doc_uuid: 361da0c8-8497-4c6f-995b-9ab3d24494a9
---

# Project

Project P1 in the [project map](../PROJECTS.md).
Turn sourced requirements into explicit, reviewed policy models that operators can inspect before activation.
The user requested project-level organization on 2026-11-17. This PRD proposes the project boundary and first outcomes.

# Users and problem

A policy author must preserve the source meaning while identifying what IPE can evaluate.
An operator must know which model and policy revision will govern an action.
A discovered source rule is neither a reviewed IPE model nor permission to activate one.

# Existing foundation

[Policy documents](../../docs/RUNTIME_DATA.md) support immutable revisions, provenance, candidate testing, and activation.
[Integration models](../../docs/INTEGRATION_DOMAIN_MODEL.md) describe external policies, controls, and requirements.
The [vocabulary](../../docs/VOCABULARY.md) separates catalog acceptance from policy activation.
A complete source-to-reviewed-model workspace remains planned.

# Route and scope

Source → Model → Review → Publish policy revision.
This project owns modeling provenance, review, unsupported sections, and activation intent.
P2 supplies source observations; P3 consumes approved policy revisions; P4 exposes the links in its system map.
Runtime transport and source collection remain with their owning projects.

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-1 | Preserve source identity, source revision, model revision, and unsupported requirements in a candidate policy. | Must have |
| FR-2 | Let an authorized operator test a candidate and inspect its decision basis without changing active enforcement. | Must have |
| FR-3 | Separate review approval, publication, and activation, with explicit authority and revision checks. | Must have |

## AC-1: Incomplete model remains explicit

Requirement: FR-1

Given a source with an unsupported rule, modeling retains that rule and the source revision.
The view labels the gap. It cannot present the candidate as fully modeled or silently omit the rule from an applicable evaluation.
Changing the mapper preserves the distinction between source and model revisions.

## AC-2: Candidate review has no activation side effect

Requirement: FR-2

Given an active policy and a candidate, testing the candidate returns results and evidence references.
The active policy revision remains unchanged even when the candidate succeeds or the test fails.
A source label or successful read cannot stand in for approval.

## AC-3: Controlled activation

Requirement: FR-3

Given a reviewed candidate, an authorized publication and activation can select its exact policy revision.
A stale expected revision or missing management permission leaves active policy unchanged.
An integration catalog acceptance cannot activate the candidate.

# Boundaries and dependencies

Do not generate executable meaning for opaque external rules without an explicit mapping and review basis.
No autonomous policy approval, provider mutation, or complete semantic translation is promised.
P2 must provide durable source references. P3 must report the active revision it evaluates.
The initial provider, policy author workflow, and review roles need product decisions before implementation.

# Verification and next slice

Use existing candidate and activation acceptance tests as the baseline. Add source/model revision and unsupported-rule review cases.
The first proposed slice is one sourced policy, candidate comparison, and explicit publication/activation history.
Record test results separately; this PRD does not claim those additional cases exist or pass.
