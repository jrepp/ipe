# IPE documentation site

The [public site](https://jrepp.github.io/ipe/) explains the policy runtime and the local integration host.
The [project map](../docs-cms/PROJECTS.md) connects five project routes to their draft requirements.
The [documentation index](INDEX.md) links to implementation and operating guides.

## Preview the site

Run these commands from the repository root. The build uses the Python version in [the version file](../.python-version) through uv.

```sh
npm ci --prefix docs --ignore-scripts
make docs-check
make docs-serve
```

Open [the local overview](http://127.0.0.1:8000/docs/). The server reads the portable site from `target/docs-site/`.
The HTTP check serves the same files under `/ipe/` to test the GitHub Pages project path.
The interaction tests check the document viewer, the archived policy explorer, and the measurement dashboard.

## GitHub Pages publication

The repository uses **GitHub Actions** as its Pages source.
The [Docs site workflow](../.github/workflows/docs-site.yml) builds a portable site and checks its data, links, and interactions.
On a pull request, it saves a review artifact named `docs-site-<commit SHA>`.
The same run packages the checked files for Pages.

A push to `main` starts automatic publication. Maintainers can also run the workflow manually on `main`.
Publication requires the site checks, design document validation, and [workflow validation](../.github/workflows/workflow-validation.yml) to pass.
The publish job deploys that run's Pages artifact and records its URL in the `github-pages` environment.
Use the [workflow runs](https://github.com/jrepp/ipe/actions/workflows/docs-site.yml) to find the deployed revision and result.

## Archived examples

The policy explorer and performance dashboard use a fixed export from the removed interpreter.
The build copies that export into the archive. Read the [runtime data guide](RUNTIME_DATA.md) for the current evaluation service.
