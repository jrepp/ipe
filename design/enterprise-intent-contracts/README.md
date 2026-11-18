# Enterprise Intent Contracts

This management review shows two changes to a sample release workflow. Policy modeling helps an agent find approval requirements before it acts. Evidence integration supplies facts for enforcement. An enforcement point uses the evaluation result to control a tool call, credential request, or external action.

The comparison uses assumed controls and sample data. It covers P1 through P5 on one page, with proposed connections clearly marked. The [review memo](../../docs-cms/memos/memo-002-agent-stack-diagram.md) links the implemented capabilities and the proposed connections.

## Share one file

Open or send [enterprise-intent-contracts.html](enterprise-intent-contracts.html). It embeds D3, styles, fonts, and license notices.
It works offline without a server, the repository, or companion files. Rebuild it after source changes:

```sh
npm run export --prefix design/enterprise-intent-contracts
```

## Enforcement detail

The second [self-contained page](policy-enforcement.html) shows a JWT endpoint claim, service RAR, policy checks, immutable policy revisions, and a fact store.
The [editable source](enforcement-detail.html) uses the same site styles. `npm run export` rebuilds both standalone pages.

The JWT adapter and endpoint claim are proposed. RAR means Resource–Action–Request in this project.
The service matches the endpoint claim to trusted configuration before it sends a request.
The page uses current documented runtime constraints as its NFR basis. No separate curated NFR document was found.
The [source review](../../docs-cms/memos/memo-003-enforcement-detail.md) identifies each source and separates old performance targets from current constraints.

## Open the editable source

Run this command from the repository root.

```sh
python3 -m http.server 8760 --bind 127.0.0.1
```

Open [the comparison](http://127.0.0.1:8760/design/enterprise-intent-contracts/). The page uses the light theme, IBM Plex fonts, and project colors from the repository's static site. It uses the checked-in [D3 7.9.0 bundle](vendor/d3.v7.9.0.min.js) under its [license](vendor/D3-LICENSE). It needs no external connection.

Use **Play comparison** under P1 to compare a source rejection with a planned handoff to a reviewer. Both cases start with missing approval. The proposed path stops for a human decision.

Select an approval state and use **Play comparison** under P3 to compare the enforcement paths. The trace uses a current approval, a rejection, expired evidence, or missing evidence. A blocked request in the proposed design stops at the tool boundary. An allowed request must also pass the credential, action, and release service checks. The animated fact inputs illustrate the state available to the policy engine before the deployment request.

## Edit and verify

The [page](index.html), [diagram styles](style.css), [planning trace](modeling.js), and [enforcement trace](diagram.js) are editable source. See the [diagram workflow](../../.agent/diagram-workflow.md) for review steps.

Run these commands from the repository root.

```sh
npm ci --ignore-scripts --prefix design/enterprise-intent-contracts
npm exec --prefix design/enterprise-intent-contracts -- playwright install chromium
npm test --prefix design/enterprise-intent-contracts
```

The browser tests check the planning and enforcement outcomes, keyboard activation, reduced motion, cancellation, replay, and label fit from 320 to 1280 pixels. They also check that the site keeps its light palette. A trace exercises the interface with sample data.

## Presentation images

Use the [full project overview](previews/00-projects.png), or [modeling first](previews/01-modeling.png), then [enforcement](previews/02-enforcement.png).
The [enforcement detail image](previews/03-enforcement-detail.png) shows the second one-pager.
These images preserve the initial sample state. Use the live page to trace each path.
Regenerate the export and presentation images after editing the diagram:

```sh
npm run export --prefix design/enterprise-intent-contracts
npm run previews --prefix design/enterprise-intent-contracts
```

## Review results

On 2026-11-17, all ten Chromium tests passed, including isolated offline tests for both exports. Desktop Safari review and narrow Chromium review checked the rendered layout.
The detail tests cover allow, deny, expired evidence, invalid JWT, endpoint mismatch, and an unavailable engine.
The editable preview loads `docs/site.css` and its local fonts. The export embeds them. It retains the site's light palette under either operating-system color preference.
`make docs-cms-check` and `make repo-check` passed. These checks validate design records and repository links, not runtime enforcement.

The management story covers P1 modeling, P2 evidence integration, P3 enforcement, P4 evidence operations, and P5 delivery. The complete connection remains proposed.
No runtime code or live external service was tested for this diagram change.

The [initial conversation fragment](reference/initial-fragment.html) is a historical snapshot. It expects the original host's styles and loads D3 from a CDN. Open the current comparison for review.
