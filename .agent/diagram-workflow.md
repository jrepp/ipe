# Architecture diagram workflow

Use this workflow to create and review durable, animated architecture diagrams.
The [Enterprise Intent Contracts example](../design/enterprise-intent-contracts/README.md) contains editable assets and local run instructions.

## Define the comparison

1. Check the branch, local changes, and applicable agent instructions.
2. Read the [code boundaries](repository.md), [runtime guide](../docs/RUNTIME_DATA.md), and [vocabulary](../docs/VOCABULARY.md).
3. Read the [design skill](../.agents/skills/ipe-design/SKILL.md) and its feedback.
4. State the baseline assumptions, requested product name, and intended audience.
5. Separate implemented behavior from proposed connections. Record findings in [docs-cms](../docs-cms/README.md).

For this comparison, use **Enterprise Intent Contracts** in the visible product interface.
This name does not rename repository packages or establish a new contract schema.
The baseline is an assumed capable agent stack, not a survey of every agent product.

Present this study to management in this order:

1. Modeling: P1 describes policies and controls. P2 supplies sourced evidence so the agent can plan and identify gaps.
2. Enforcement: P3 evaluates requests and applies decisions at identified action boundaries.

Show all five projects on one page. Put the P2 evidence flow between modeling and enforcement.
Follow enforcement with P4 evidence operations and P5 product delivery.
Use short labels and visually distinct paths. Keep source details in a disclosure.
Export one self-contained HTML file with embedded scripts, styles, fonts, and dependency licenses.
Verify the exported file from an isolated directory with network access disabled.

Show how missing policy information leads to source-system failures or a blocked step.
Compare that path with a model that identifies the requirement, evidence gap, and authorized human handoff before action.
Knowledge of a policy does not grant permission, prove approval, or remove every possible failure.

## Draw the boundaries

Keep the baseline controls visible in both views. Add policy modeling, fact integration, and enforcement to the second view.
Distinguish requests, credentials, source observations, published facts, evaluation requests, and decisions.
Show the token broker on the credential path. Do not imply that every action passes through it.
Connect each component or identify it as context. Put a model provider outside the sandbox unless deployment evidence says otherwise.

Show source, scope, revision, and expiry where they affect a decision.
Keep policy activation separate from authorized fact publication.
An engine returns a result. An enforcement point applies it before the protected operation.
Preserve native permissions and local controls after an allow result.
Do not equate external approval, evidence matches, or past success with current authorization.

## Build and refine

Store editable HTML, CSS, JavaScript, dependency licenses, and local run instructions under `design/<name>/`.
Prefer a standalone preview that does not depend on a conversation host or a personal skill path.
Pin dependencies. Keep generated previews reproducible. Do not require production credentials or external accounts.
Use D3 for the diagram and finite request traces. Provide replay, readable states, and reduced motion.
Keep the initial diagram useful without animation. Preserve text labels when colors change or motion stops.
Reuse the static site's [stylesheet](../docs/site.css) and [product overview](../docs/index.html) styling for this study.
Use its local fonts, typography, colors, spacing, and project-route identity. Keep diagram-specific styles local to the artifact.

When the user requests Sol refinement, delegate a bounded review to the available Sol model.
Pass the source paths, product name, source guides, semantic concerns, and required checks.
Assign file ownership before parallel edits. Have the primary agent review the resulting diagram independently.
Do not delegate solely because this guide includes that example.

## Review and validate

1. Inspect the rendered diagram at desktop and narrow widths in light and dark themes.
2. Check text bounds, connectors, component boundaries, keyboard focus, and control access.
3. Trace a permitted request, a rejected request, and a request with expired evidence.
4. Confirm that a blocked trace cannot reach the protected action.
5. Change the scenario during a trace. Check that old animation work stops.
6. Check reduced motion and replay. Confirm that completed traces leave no moving marks.
7. Record check commands and results in the artifact README or linked review memo.
8. Run `make docs-cms-check` after structured-document changes and `make repo-check` after repository-map changes.

Treat simulated outcomes as interface evidence only. They do not prove runtime integration or enforcement coverage.
Report mocked geometry, browser checks, source review, and implementation tests separately.
Record unresolved findings without marking a design accepted. Link the final assets and workflow in the handoff.
