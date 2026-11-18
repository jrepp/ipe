---
id: prd-003
title: Evidence integration and publication
status: Draft
author: IPE maintainers
created: 2026-11-17
target_release: Unscheduled
tags: [product, project]
project_id: ipe
doc_uuid: e8c8fd91-8361-49c6-bed1-9f721439de2a
---

# Project

Project P2 in the [project map](../PROJECTS.md).
Collect permitted source observations and deliver fresh, attributable facts to the evaluations that need them.
This is a proposed project boundary based on the user's evidence-supplier and system-map requests.

# Users and problem

An integration operator needs to know which source was read, what was collected, and which controls depend on that evidence.
An evaluator needs scoped facts with original validity, not a live provider call on its decision path.
A successful collector run does not establish that a requirement is fulfilled.

# Existing foundation

The [integration host](../../docs/INTEGRATION_HOST.md) has brokered reads, sandboxed programs, schedules, records, and review catalogs.
The [runtime](../../docs/RUNTIME_DATA.md) accepts authenticated push and pull fact updates.
The host does not currently publish enforcement facts. [RFC 014](../../rfcs/014-fact-publication-and-synchronization.md) owns that proposed delivery contract.
[Assessments](../../docs/INTEGRATION_ASSESSMENTS.md) distinguish evidence checks from requirement fulfillment.

# Route and scope

Connect → Observe → Assess evidence → Publish facts.
P2 owns collection and the integration-to-runtime publication boundary, using RFC 014.
P1 uses observations for modeling. P3 consumes published facts. P4 shows supplier and consumer relationships.
No collector can approve its own grants, policy activation, or disclosure rules.

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-1 | Collect only permitted source data with bounded work and protected credentials. | Must have |
| FR-2 | Preserve evidence lineage, coverage, uncertainty, and original validity through modeling and publication. | Must have |
| FR-3 | Deliver authorized fact changes with ordering and recovery visible to operators. | Must have |

## AC-1: Bounded collection

Requirement: FR-1

Given an installed collector with limited grants, an allowed read produces a native receipt and a source observation.
A denied read or exhausted budget produces a bounded failure without exposing credentials or expanding source access.
A failed run does not imply that a policy or source object was removed.

## AC-2: Lineage and expiry survive delivery

Requirement: FR-2

Given an observation used by two requirements, both retain the source context and validity deadline.
A duplicate delivery, cached read, or delayed receipt cannot renew expiry.
Missing, uncertain, or mismatched evidence remains distinct from a matching evidence check and from requirement fulfillment.

## AC-3: Receiver recovery

Requirement: FR-3

Given a receiver that misses updates, reconnection reconciles authorized changes in revision order and reports applied progress.
An old update cannot overwrite a newer revision. Partial failure cannot become a complete source snapshot.
The operator can distinguish collected, published, and applied states.
The first live proof must show a real collector observation reaching a scoped runtime evaluation through the authorized path.

# Boundaries and dependencies

P2 does not activate enforcement policy or execute provider remediation actions.
The publication schema, initial source, receiver identity, reconciliation bounds, and trust configuration need review.
Use RFC 013 for source semantics, RFC 014 for transport, RFC 015 for disclosure, and RFC 016 for audit.
Do not define competing transport protocols in project navigation documents.

# Verification and next slice

Replay native collector fixtures, then test expired data, duplicate delivery, out-of-order revisions, missed notifications, and partial snapshots.
The first proposed slice connects one permitted source to one runtime collection with retained delivery evidence.
Provider fixtures prove protocol handling; they do not prove live account grants or production capacity.
