# IPE documentation

Start with the [documentation map](../docs-cms/wiki/pages/documentation-map.md) for the current guides, design records, and contributor workflows.
Use the [project map](../docs-cms/PROJECTS.md) for the five product lanes and their proposed handoffs.

## Run and use IPE

- [Workloads and artifacts](workloads.md): current binaries, modes, and packaging limits.
- [Runtime data](runtime-data.md): enforcement API, fact storage, policies, and credentials.
- [Integration discovery](integration-discovery.md) and [integration host](integration-host.md): source access, collectors, and local records.
- [Integration SDK](integration-sdk.md): guest programming interface and package cache.

The integration host does not yet publish facts to the enforcement runtime. See [PRD 003](../docs-cms/prd/prd-003-evidence-integration.md) for the proposed path.

## Design and develop

- [Design records](../docs-cms/README.md): draft requirements and RFCs, decisions, and research.
- [Repository map](repository-guide.md): workspace components, ownership boundaries, and placeholders.
- [Vocabulary](vocabulary-guide.md): project terms and their meanings.
- [Contributing](../CONTRIBUTING.md) and [validation guide](../.agent/validation.md): setup and checks.
- [Release guide](release-guide.md) and [merge queue guide](merge-queue.md): delivery procedures and limits.

## Publish and explore

The [documentation site](https://jrepp.github.io/ipe/) is a static publication. See the [site guide](README.md) for preview and deployment.
Run `make docs-cms-check` for structured records and wiki pages. Run `make docs-check` for the site.

Older plans, test reports, and examples are in the [historical archive](archive/index.md). Their original paths remain available.
