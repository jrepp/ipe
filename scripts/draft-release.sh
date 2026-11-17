#!/usr/bin/env bash
# GitHub Releases is the changelog source. Never publish or create a tag here.
set -euo pipefail

tag="${1:-}"
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$ ]]; then
    echo 'Expected vMAJOR.MINOR.PATCH with an optional prerelease suffix.' >&2
    exit 2
fi
: "${GH_REPO:?Set GH_REPO to the owner/repository}"

args=(release create "$tag" --repo "$GH_REPO" --verify-tag --draft --generate-notes --title "$tag")
if [[ "$tag" == *-* ]]; then
    args+=(--prerelease)
fi
# Existing releases and API failures stop the command. Do not overwrite reviewed notes.
gh "${args[@]}"
