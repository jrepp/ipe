# Policy workspace mock

This interactive design mock uses sample data. It does not connect to the IPE runtime or external systems.
Policy assignments, node reports, and source observations illustrate proposed UX behavior.

## Run locally

From the repository root, start a static server:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory design/admin-ux-review/dist
```

Open <http://127.0.0.1:8765/#workspace/policies> for the policy workspace.
Stop the server with Ctrl-C when finished.

No build step or package installation is required. The `dist` directory contains the editable HTML, CSS, JavaScript, fonts, and images.

## Review scope

The workspace includes policy search, filters, pagination, saved views, selection review, and CSV export.
Related screens show VPCs, clusters, node reports, integrations, and external policies.
The root page also retains the four design directions used in the original review.

`scale.js` generates 4,800 sample policies and the related deployment data.
`external-policies.js` supplies external policy samples. `app.js` supplies shared screens and interactions.
These counts do not establish runtime capacity or performance.

Saved views and design feedback use browser local storage. Other interactions can reset when the page reloads.
Do not enter credentials or production data.

## Source snapshot

The initial snapshot uses published Sites version 7, source commit `d353faab6c136e38b06c79f55e7419959ddc91d9`.
Only whitespace and line endings in the font license were normalized.
The [hosted review](https://ipe-admin-design-review.artfulrain13.chatgpt.site/#workspace/policies) can change independently.
This directory omits the separate Sites repository metadata and hosting identity. Changes here do not publish the hosted review.
