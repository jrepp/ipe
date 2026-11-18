---
id: prd-004
title: Policy evaluation and enforcement
status: Draft
author: IPE maintainers
created: 2026-11-17
target_release: Unscheduled
tags: [product, project]
project_id: ipe
doc_uuid: 21b2b0d1-50a3-4e8d-976e-ce042b796889
---

# Project

Project P3 in the [project map](../PROJECTS.md).
Evaluate explicit policies against fresh scoped facts, then show where applications apply each decision.
This project separates the decision service from the action gate that acts on its result.

# Users and problem

An application developer needs an unambiguous decision for one action and context.
An operator needs the policy revision, fact snapshot, and enforcement point behind that decision.
A returned allow result alone does not prove the external action happened.

# Existing foundation

The [runtime guide](../../docs/runtime-data.md) describes language-2 evaluation, immutable snapshots, scope isolation, finite TTLs, and deadline checks.
The runtime returns policy decisions; a complete fleet of application enforcement points is not established by the current preview.
[Enforcement MCP](../../rfcs/017-enforcement-mcp-surface.md) remains a design proposal.

# Route and scope

Bind scope → Evaluate → Apply decision → Record outcome.
P1 supplies reviewed policy revisions. P2 supplies fresh runtime facts. P4 shows operating points, time, and temporary authority.
P3 owns decision correctness and the application-gate contract; P4 owns the operator-facing cross-system inspection.

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-1 | Evaluate one scoped policy and fact snapshot with bounded work and no provider request during evaluation. | Must have |
| FR-2 | Apply decisions at an identified action gate and distinguish decision results from observed enforcement outcomes. | Must have |
| FR-3 | Report assigned and active policy revisions with sufficient evidence context to explain the decision. | Must have |

## AC-1: Stable and fresh decision

Requirement: FR-1

Given concurrent policy and fact updates, an evaluation uses the revisions captured for that evaluation.
Missing or expired required evidence cannot yield allow. Expiry reached before completion prevents an allow response.
Provider outage does not cause the evaluator to make a synchronous source request.

## AC-2: Action gate behavior

Requirement: FR-2

Given an action gate and a matching decision, deny or indeterminate prevents the governed action under the current blocking contract.
An allow applies only to the evaluated identity, action, object, and context within its usable validity.
The gate reports its applied decision separately from a provider-reported action outcome.
An absent report remains unknown rather than an inferred success.

## AC-3: Revision visibility

Requirement: FR-3

Given a new assignment and an enforcement point still using an older revision, the operator sees both revisions and last report time.
A replayed status cannot make an old point appear current.
Authorized inspection identifies the captured policy and evidence basis without disclosing protected source details.

# Boundaries and dependencies

The first application gate, decision-to-action validity contract, replay handling, and fleet reporting protocol need review.
A decision is not a transferable capability by default.
Do not weaken current deny and indeterminate precedence to introduce advisory behavior.
P4's temporal control and exception design must specify its interaction with this contract before runtime implementation.
No replication, production latency, complete audit durability, or universal provider enforcement guarantee is claimed.

# Verification and next slice

Retain runtime acceptance and snapshot/expiry tests. Add a concrete action-gate fixture that rejects mismatched or expired decisions.
Test stale assignment reports, missing acknowledgments, and the distinction between returned decisions and performed actions.
The first proposed slice connects one runtime scope to one real application gate with an inspectable outcome trail.
