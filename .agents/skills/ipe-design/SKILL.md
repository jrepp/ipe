---
name: ipe-design
description: Design and review IPE policy, control, evidence, and integration interfaces. Apply project feedback on plain language, navigation, source context, and agent usage reports. Use for IPE UX changes, not unrelated sites or runtime implementation.
---

# IPE design

Use this skill for IPE interface design and preview revisions. Read [feedback](references/feedback.md) before changing the information structure.
Treat the current user request as the task. Past feedback supplies defaults, not permission to expand the task.

## Language

Use direct labels that name the content: **Release decision**, **Findings**, **Agent activity**, **Resource use**, and **Run history**.
Avoid headings such as “What it permits,” “What it can reduce,” and “How it works.” Name the actual subject instead.
Use verbs for actions: **View evidence**, **Open source record**, and **Show checklist**.
A simpler word must preserve the meaning. Explain a necessary domain term once instead of rotating through synonyms.

Use short sentences, active voice, and one fact per sentence. Keep instructions within 20 words and descriptions within 25 words.
Keep facts, identifiers, commands, source values, and quoted errors intact. Rewrite interface prose without rewriting source evidence.
For a full copy pass, use `~/d/SimpleEnglish/skills/simple-english/SKILL.md` when available. These local rules remain usable without that checkout.
Follow the repository writing guide. Do not claim formal STE compliance from an automated scan.

## Information structure

Start with the user's object and task. In the ship preview, the chain owns policies, policies own controls, and controls link to evidence.
Keep the policy chain compact and readable as a checklist. Give the selected control and its evidence more space.
Attach a report to the object it describes. Place chain usage with the central policy, not as a large report above the checklist.
Use the detail workspace for the selected control or policy report. Keep navigation and selection clear when switching views.
Show source records, people, and connections through focused detail views. Use a graph when it helps users trace evidence links.
Keep IDs and raw records available in details. Do not make them the main explanation.

For overview screens and the modernized Plex style, read [Contemporary Plex](references/modern-plex.md).

## Meaning and measurement

Keep policy decisions, agent checks, source states, and enforcement events distinct.
An owner or approver role does not prove approval. A read receipt does not prove control effectiveness.
Show the source system, record ID, required state, observed state, and responsible people where they explain a check.
Keep stale, missing, ambiguous, and mismatched evidence visible. Do not fill gaps with inferred identities or passing states.

A chain report explains use, release decisions, findings, and resource use.
Count recorded runs separately from agents, calls, and unique controls. Page views do not count as policy uses.
Separate input tokens, output tokens, elapsed time, and total work time. State whether preparation is included.
Label sample data and unknown costs. Compare equivalent work before claiming savings. Findings do not imply model training.

## Review before handoff

- Check that each heading names its content and each action names its result.
- Check that the selected policy or control explains the detail view.
- Open the report from the policy. Return to the selected control without losing context.
- Inspect the desktop and narrow layouts for overflow, hidden content, focus, and readable text.
- Check one passing result, one failed result, and one unknown result after changes to report navigation.
- Record new feedback in the reference. Distinguish user approval from an implementation that still needs review.

Run checks that match the change. Use existing UI tests for navigation and state. Do not add tests that only freeze prose.
