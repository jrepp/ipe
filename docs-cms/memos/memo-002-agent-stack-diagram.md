---
title: Enterprise Intent Contracts agent stack diagram review
author: IPE maintainers
created: 2026-11-17
tags: [design, enforcement, visualization]
id: memo-002
project_id: ipe
doc_uuid: 2ec59aa8-ed70-4cb5-a1d4-b587f06ae0f4
---

# Scope

The project owner requested an animated D3 comparison of an agent stack before and after adding a policy engine.
The owner selected **Enterprise Intent Contracts** as the visible product name, then requested Sol refinement and durable repository artifacts.
These instructions came from the project conversation on 2026-11-17. They do not establish approval of the resulting architecture.

The owner then clarified the audience and order: explain P1–P3 to management, with modeling before enforcement.
The owner requested the static site's styling for both comparisons.
The owner then requested less text, more visual contrast, all five projects on one page, and a self-contained HTML export.

The [artifact directory](../../design/enterprise-intent-contracts/README.md) owns the editable diagram, preview instructions, and verification commands.
The [diagram workflow](../../.agent/diagram-workflow.md) records the repeatable creation and review steps.

# Source boundaries

| Diagram concept | Source | Limit |
| --- | --- | --- |
| Policy revisions, scoped facts, immutable snapshots, finite expiry | [Runtime data](../../docs/RUNTIME_DATA.md) | A diagram trace does not exercise the runtime. |
| Modeling and candidate review | [Policy modeling PRD](../prd/prd-002-policy-modeling.md) | Draft requirements do not prove delivery. |
| Authorized collection and fact publication | [Evidence integration PRD](../prd/prd-003-evidence-integration.md) | The complete integration-to-enforcement flow remains conceptual. |
| Evaluation and enforcement points | [Policy enforcement PRD](../prd/prd-004-policy-enforcement.md) | A returned decision does not establish enforcement at every boundary. |
| Integration facts versus runtime facts | [Vocabulary](../../docs/VOCABULARY.md) | Local hosted integration records are not automatically enforcement inputs. |
| Existing workload limits | [Code boundaries](../../.agent/repository.md) | Integration discovery and the local host do not publish enforcement facts. |

The baseline assumes a sandbox, agent harness, network controls, identity, model access, memory, approval checks, and audit logs.
It also includes a token broker, integration gateway, and native access checks. This is an illustrative baseline, not a market survey.
The second view retains those controls. Enterprise policy decisions supplement them.

# Management presentation

The [project map](../PROJECTS.md) defines the three roles. They are product outcomes, not assigned teams or delivery dates.

| Presentation order | Projects | Management question |
| --- | --- | --- |
| Modeling comparison | P1 policy modeling and P2 evidence integration | Can the agent identify requirements, evidence gaps, and necessary human handoffs before attempting work? |
| Enforcement comparison | P3 policy enforcement, with P1 policies and P2 facts | Which component checks the action, and which component prevents it when the requirements do not hold? |

The modeling baseline has no supplied policy model. A source system reports a failure, and the agent cannot bridge a required approval step.
The modeled path exposes the policy, control, source evidence, and authorized handoff. The approval remains pending until an authorized person decides.
This comparison illustrates earlier knowledge of a dependency, not new authority or guaranteed completion.

Use the [static site stylesheet](../../docs/site.css) and [overview](../../docs/index.html) as the visual source.
Reuse typography, colors, project-route identity, and spacing. Do not introduce a separate visual brand for each comparison.

The revised page uses short labels and different route shapes to make the change visible.
P2, P4, and P5 use compact project flows. Technical source notes remain available in a disclosure.
The [self-contained export](../../design/enterprise-intent-contracts/enterprise-intent-contracts.html) embeds local fonts, D3, styles, and scripts for offline sharing.
The editable source and export command remain in the artifact directory. The export does not publish the site or activate a policy.

# Initial review

The first artifact lived under `output/enterprise-intent-contracts/` and depended on a conversation preview wrapper.
Its three sample outcomes passed a DOM test with mocked SVG geometry. That test did not verify browser layout.
The subsequent Safari review showed readable desktop labels, but exposed these semantic problems:

| Finding | Required refinement |
| --- | --- |
| The token broker sat in the serial action path. | Show credential acquisition separately from the action request. |
| Identity and approval appeared without connections. | Connect them to the components that consume their context. |
| Model access appeared inside the sandbox. | Distinguish external model access from sandboxed execution. |
| The second view replaced some local-rule labels. | Keep existing controls visible in both views. |
| Decision lines did not explain evaluation requests. | Distinguish requests to the engine from returned decisions. |

# Evidence and approval limits

The refinement uses sample data. A successful browser trace demonstrates interface behavior only.
The source guides establish the documented runtime and integration boundaries; they do not prove the proposed connections are implemented.
No production integration, runtime enforcement, latency, or capacity claim follows from this artifact.
The user requested refinement. The resulting design still requires user review.

Verification results belong with the final artifact. Keep unmet checks explicit when updating that record.
