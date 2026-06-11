---
description: Review and merge the release-plz release PR with explicit user confirmation
allowed-tools: Bash, Read, Grep, Edit, AskUserQuestion
---

# Release

Releases are driven by **release-plz** (`release-plz.toml` +
`.github/workflows/release-plz.yml`). On every push to main it maintains a
`chore: release vX.Y.Z` PR with the version bump and CHANGELOG update derived
from conventional commits. **Merging that PR ships the release** — release-plz
pushes the `vX.Y.Z` tag, which triggers `release.yml` (static binaries, GitHub
release, crates.io, Docker) and `flakehub-publish-tagged.yml`.

This command walks that flow safely. Never merge the release PR without
explicit user confirmation — it is an outward-facing, hard-to-reverse action.

## Phase 1: Locate the release PR

```bash
gh pr list --repo utensils/nxv --search "chore: release in:title" --state open
```

If none exists, check whether main has unreleased conventional commits since
the last tag (`git log $(git describe --tags --abbrev=0)..origin/main --oneline`).
If there are releasable commits but no PR, check the latest `Release-plz`
workflow run for errors (`gh run list --workflow release-plz.yml`).

## Phase 2: Review what will ship

1. Show the PR's version bump (Cargo.toml diff) and CHANGELOG addition.
2. Verify the version follows the project convention: `feat:` commits bump
   the 0.x minor (`features_always_increment_minor = true`), fixes bump patch.
3. If the version or changelog is wrong, edit the release PR directly
   (it is a normal editable PR) — release-plz preserves manual edits unless
   new commits land on main.

## Phase 3: Pre-flight

1. All CI checks on the release PR must be green. If checks show "Expected /
   Waiting", the App token is misconfigured — see `release-plz.yml` comments.
2. Confirm no other PR is about to land that should be in this release.

## Phase 4: Confirm and merge

Present a summary (version, commit list, changelog) and require the user to
confirm with the explicit version number (AskUserQuestion). Then:

```bash
gh pr merge <PR> --repo utensils/nxv --squash
```

## Phase 5: Monitor

1. The `Release-plz` workflow on main pushes the tag.
2. The `Release` workflow builds binaries and publishes — monitor with
   `gh run list --workflow release.yml` / `gh run watch`.
3. Verify the GitHub release has all four binaries + SHA256SUMS.txt,
   crates.io has the new version, and ghcr.io has the version tag.

## Manual escape hatches

- Tagging failed after the release PR merged: re-run the `Release-plz`
  workflow on main (it is idempotent — existing tags are the gate).
- The old fully-manual flow (bump Cargo.toml, edit CHANGELOG, tag by hand)
  still works in emergencies but must match release-plz's expectations
  (tag `vX.Y.Z`, CHANGELOG section `## [X.Y.Z] - YYYY-MM-DD`).
