---
id: prd-001
title: Evidence system map and temporary authority
status: Draft
author: IPE maintainers
target_release: Unscheduled
created: 2026-11-17
tags: [design, documentation]
project_id: ipe
doc_uuid: d65227a1-6aa1-4795-a6ac-04daac04940e
---

# Project

Project P4 — Evidence operations in the [project map](../PROJECTS.md).
This PRD owns the map, temporal control, exception, and temporary-access requests.
Supporting project PRDs do not replace these acceptance criteria.

# Problem and source

Operators need to see which integrations supply evidence and where controls use it.
They also need time-based facts, time-based controls, and temporary exceptions and access.
The project owner requested these features in the design discussion on 2026-11-17.
This document records those requests. Detailed semantics remain proposed until reviewed.

The current preview shows policy dependencies and sample evidence. It does not establish a live system map.
See the [current runtime guide](../../docs/RUNTIME_DATA.md), [integration host guide](../../docs/INTEGRATION_HOST.md), and [design](../rfcs/rfc-018-evidence-system-map.md).

# Users and outcomes

An operator can trace a requirement from source evidence to the point where an action is allowed or blocked.
A policy author can distinguish a model from an active policy and an observed enforcement outcome.
An authorized requester can obtain bounded access or an exception and inspect its remaining validity.

# Requirements

| ID | Requirement | Basis |
| --- | --- | --- |
| FR-1 | Show modeling, integration, and enforcement as distinct entities in a system map. | User request |
| FR-2 | Show integration points supplying each evidence item and the controls that depend on it. | User request |
| FR-3 | Show enforcement points consuming evidence and any enforcement events that supply new evidence. | User request; event direction is a proposed clarification |
| FR-4 | Show time-based facts and their observation, validity, and expiry. | User request |
| FR-5 | Represent time-based controls and distinguish hard controls from soft controls. | User request; blocking and advisory labels are proposed |
| FR-6 | Support time-limited policy exceptions with explicit scope and authority. | User request; lifecycle below is proposed |
| FR-7 | Support time-limited access to evidence and governed objects. | User request; lifecycle below is proposed |
| FR-8 | Show when exceptions and grants take effect, expire, or are revoked at each affected enforcement point. | Proposed interpretation of real-time behavior |
| NFR-1 | Preserve scope isolation, evidence freshness, and restricted information across maps, details, and exports. | Existing repository invariants and RFC 015 |

# Acceptance criteria

These are proposed acceptance contracts for the requested features. They are not reports of passing tests.

## AC-1: Distinct system roles

Requirement: FR-1

Given a collector, a model revision, an evaluator, and an action gate, the map identifies their separate roles and relationships.
Co-location does not merge their identities. A policy decision is distinguishable from an action actually blocked or allowed.

## AC-2: Evidence suppliers

Requirement: FR-2

Given one source observation used by two controls, selecting either control reveals the supplying integration and source provenance.
A missing or failed read remains missing or failed. It does not appear as a successful observation.

## AC-3: Evidence consumers

Requirement: FR-3

Given two enforcement points using different policy revisions, selecting evidence shows each known consumer and its active revision.
A reported enforcement event links back to its producing point. Unknown consumers remain unknown.

## AC-4: Fact validity

Requirement: FR-4

Given evidence with a known expiry, the view shows its observation and expiry times against a stated clock.
At expiry equality, the evidence cannot satisfy a freshness-dependent check.
A later collection receipt does not renew the original observation without a valid new observation.

## AC-5: Control time and effect

Requirement: FR-5

Given blocking and advisory controls, the view states each mode, time condition, and failed or unknown result.
A blocking failure prevents the governed action. An advisory failure remains visible without independently blocking that action.
A schedule, deadline, or grace period is separate from fact expiry. Unknown mode or time semantics cannot silently become advisory.

## AC-6: Scoped exception

Requirement: FR-6

Given an authorized exception for one control, object, and interval, only matching evaluations can use it during that interval.
Before its start, at its expiry, or after effective revocation, it cannot waive the control.
The original result and exception basis remain visible. An exception does not renew evidence or grant access.
An unauthorized request or conflicting update leaves effective authority unchanged.

## AC-7: Scoped access

Requirement: FR-7

Given an approved access grant, only its principal, audience, actions, fields, and objects are available during its interval.
Each protected request checks current authority. Another principal, object, or tenant remains unauthorized.
Grant expiry or revocation prevents subsequent disclosure. An access grant does not waive a policy requirement.
Already disclosed information cannot be recalled.

## AC-8: Distributed lifecycle

Requirement: FR-8

Given an approved change, show the requested state separately from each enforcement point's acknowledged state.
An offline point cannot appear current. Duplicate delivery cannot extend validity or reverse a newer revocation.
Show revocation as pending where it is not yet confirmed. Activation and revocation latency targets remain open.

## AC-9: Restricted and incomplete views

Requirement: NFR-1

Given a caller who cannot see a source or object, the graph, counts, search, and exports do not disclose its protected existence or fields.
Where permitted, distinguish restricted details from missing evidence. A hidden policy still applies.
Missing required evidence cannot become an allow result through masking, an expired grant, or an unrelated exception.

# Scope and open decisions

The first deliverable is this design baseline. No feature implementation or production rollout is authorized by its Draft status.
Mockups may use clearly labeled sample data. Production publication, identity integration, and enforcement require separate implementation proof.

| Question | Owner | Impact |
| --- | --- | --- |
| Does real-time mean request-time checks, prompt propagation, or both? What latency is acceptable? | Project owner | Blocks numerical acceptance for AC-8 |
| Which controls are eligible for exceptions, and who may approve them? | Policy authority | Blocks live AC-6 |
| Can hard controls permit exceptions, and what is the precedence for overlapping exceptions? | Policy authority | Blocks live AC-5 and AC-6 |
| Which time zones, calendars, deadlines, and grace rules are required first? | Project owner | Blocks full temporal scope for AC-5 |
| Which access actions and object classes are in the first release? | Project owner and access authority | Blocks live AC-7 |
| What may an offline enforcement point do while authority cannot be refreshed? | Security and runtime maintainers | Blocks live AC-8 |

No latency, capacity, release date, or approval policy is inferred from the request.
