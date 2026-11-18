# Mergify merge queue

The repository configuration is in [.mergify.yml](../.mergify.yml).
It follows the queue pattern in [Prism](https://github.com/jrepp/prism-data-layer/blob/main/.mergify.yml), with IPE checks and explicit concurrency.
The configuration must reach the default branch before Mergify uses it.
Local validation is not proof that automatic merging is active.

## Queue policy

- Target main with non-draft pull requests.
- Require at least one approval and no requested changes.
- Hold pull requests labeled do-not-merge or work-in-progress.
- Require all checks below before queue entry and on the combined batch before merge.
- Run up to five speculative checks, with up to five PRs per batch and a five-minute maximum batch wait.
- Squash each PR using its title and body.

The configuration does not approve owner or dependency PRs automatically.
A passing title pattern does not establish dependency-update severity.
Review conditions apply to the original PRs; required CI runs on Mergify's temporary batch PRs too.

| Required GitHub Actions check | Coverage |
| --- | --- |
| CI gate | Format, Clippy, all test matrix variants, default features, release builds, Rust docs, and minimal versions. |
| Security gate | Dependency audit, deny, Miri, sanitizers, and supply-chain checks. |
| Coverage gate | Overall and per-crate coverage jobs. |
| Performance gate | Benchmark and memory-profiling jobs. |
| Runtime acceptance | Storage, server boundaries, HTTP facts, policy documents, and imports. |
| Integration acceptance | Integration tests, adapters, host views, and external-client acceptance. |
| Build and check docs site | Static site build, tests, and portable archive. |
| Validate design documentation | Docuchango metadata, links, and traceability. |
| Workflow validation | Actions syntax, Mergify schema, and merge-gate regression tests. |

The aggregate gates require success from every prerequisite. Failure, cancellation, skipping, and empty results cannot produce a successful gate.
Canceled workflows stop their gates so obsolete runs do not block newer commits in the same concurrency group. An active workflow still runs its gate after a prerequisite fails. Mergify requires a successful gate, so a canceled or skipped gate cannot authorize a merge.
Integration acceptance runs on all PRs, including documentation-only and workflow-only changes.
Other required workflows already run without PR path filters. These choices increase CI work but avoid absent required checks.
Existing checks and their failure policies remain intact. Do not remove a gate to bypass an existing failure.

Mergify normally creates draft PRs for speculative batches. These use pull_request workflows, not GitHub's separate native merge_group queue.
Do not skip required checks based on draft state, bot identity, or a Mergify branch name.
See [batch CI](https://docs.mergify.com/merge-queue/batches/ci/) and [queue configuration](https://docs.mergify.com/configuration/file-format/).

## Validation

```sh
mergify config validate
actionlint
uv run --no-project --with PyYAML==6.0.3 --with jsonschema==4.25.1 python .github/scripts/check-mergify.py
uv run --no-project --with PyYAML==6.0.3 python -m unittest discover -s .github/scripts/tests -p test_merge_queue.py -v
```

The workflow validator checks the published Mergify schema. A download failure fails validation.
The regression tests verify that required check names exist, PR triggers are unfiltered, and aggregate gates reject unsuccessful prerequisites.
Simulation is read-only:

```sh
mergify config simulate https://github.com/jrepp/ipe/pull/58
```

Use a current open PR if that example has closed. Do not pass credentials as command arguments.

## Activation and verification

1. Review and merge the configuration and workflow changes together.
2. Confirm the Mergify GitHub app has access to jrepp/ipe.
3. Enable Merge Queue and Merge Protections for IPE in Mergify. Automatic queueing uses auto_merge_conditions.
4. Protect main with the named checks and review requirement if branch protection is available.
5. Observe a reviewed PR enter the queue and a temporary batch PR run all required checks.
6. Confirm a failed batch prevents merging and that a successful batch uses squash merges.

[Queue rules](https://docs.mergify.com/merge-queue/rules/) explain the required Mergify products.
GitHub's allow_auto_merge setting is not the switch for this Mergify queue.
The Mergify configuration alone does not prevent a user from bypassing it through an unprotected branch.

As checked on 2026-11-17, main has no GitHub branch protection or repository rulesets.
The Mergify CLI can read an empty queue using the existing GitHub credential fallback.
Dedicated Mergify login is absent. GitHub app-installation inspection returns 403 for the current token.
A read-only simulation reports no workflow automation rules, as expected for a queue-only configuration.
It does not confirm queue eligibility or product activation.
Queue and Merge Protections product activation, and a successful live batch, still need confirmation.
No existing PR was approved, queued, or merged during local setup.
