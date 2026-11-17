# Releases and generated changelog

[GitHub Releases](https://github.com/jrepp/ipe/releases) is the changelog source.
GitHub generates notes from merged pull requests, contributors, and comparison links.
The repository does not maintain a second generated copy of release notes.
[Previous manual entries](CHANGELOG_LEGACY.md) remain archived without changes.

## Prepare a pull request

Write a title that describes the resulting behavior.
Include compatibility changes, migration steps, and validation in the PR body.
Generated notes link to the PR; they do not copy its entire body.
Do not edit `CHANGELOG.md` or the archive for each change.

Apply a label when useful:

| Label | Release section |
| --- | --- |
| `bug` | Fixes |
| `enhancement` | Features |
| `dependencies` | Dependencies |
| `documentation` | Documentation |
| Other labels or no labels | Other changes |

[Release configuration](../.github/release.yml) defines the categories.
No authors or labels are excluded. The final category catches unclassified PRs.
Review category placement when a PR has multiple labels.

## Create and publish a release

1. Merge the release changes and check CI on the intended commit.
2. Select the version and update package versions when needed.
3. Create and push a tag such as `v1.2.3` or `v1.2.3-rc.1` on that commit.
4. Open the draft created by the **Release notes** workflow.
5. Check the comparison range, generated notes, and linked PRs.
6. Add migration details and attach any validated release assets.
7. Publish the release when it is ready.

The workflow creates a draft with GitHub's generated notes.
A tag suffix such as `-rc.1` marks the draft as a prerelease.
The workflow does not bump versions, build assets, create tags, or publish releases.
It does not change the existing test and release-build workflows.

For an existing tag, run **Release notes** from the Actions page and enter the tag.
The workflow must first exist on the default branch for manual dispatch.
A tag without this workflow can use manual dispatch from the default branch.

Creation stops if the tag is missing or a release already exists for it.
API errors also stop creation. A rerun does not overwrite reviewed notes.
Use the Releases page to regenerate or edit an existing draft.
For the first release, check the range carefully; there may be no previous release tag.

The equivalent local command requires GitHub CLI authentication and repository write access:

```sh
GH_REPO=jrepp/ipe bash scripts/draft-release.sh v1.2.3
```

## Validate changes

```sh
python3 -m unittest discover -s scripts/tests -p 'test_draft_release.py' -v
actionlint .github/workflows/release.yml
```

The tests replace GitHub CLI with a local stub. They never create releases.
They check draft-only creation, existing-tag verification, prerelease handling, invalid tags, and API failure propagation.
The PR workflow runs these tests without release-write permissions.
Only tag pushes and manual dispatch can run the job with `contents: write`.

See GitHub's [generated release notes documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes)
and [release creation command](https://cli.github.com/manual/gh_release_create).
