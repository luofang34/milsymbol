# Releasing

Releases are published to crates.io by `.github/workflows/release.yml` when a
`v*` tag is pushed. Authentication uses crates.io Trusted Publishing (OIDC)
only; no registry token secret or feature variable is required. Manual runs
only verify authentication and never publish.

## Before a release

1. Update `version` in `Cargo.toml` and the `CHANGELOG.md` entry, replacing
   `Unreleased` in its heading with the release date. The release job runs
   `tools/ci/changelog.sh --release` and refuses an undated entry.
2. Run `tools/ci/lint.sh stable 1.85:lib` and `cargo semver-checks` locally.
   A breaking change needs a new minor version while the crate is 0.x.
3. Make sure CI on `main` is green (it includes `cargo publish --dry-run`).
   The release job refuses a tag whose commit is not on `main` or has no
   successful CI run; after CI passes, re-run the failed release job.

## Repository protections

- `main`: no force pushes or deletion; the CI jobs are required status
  checks (repository admins may still push directly).
- Tags `v*`: only repository admins may create them (ruleset "release
  tags: admins create"), and nobody can move or delete them (ruleset
  "release tags: immutable"). To remove a mistaken tag that was never
  released, disable the immutable ruleset, delete the tag, and re-enable it.
- Environment `crates-io`: deployments only from `v*` tags. No reviewer is
  required.

## Trusted Publishing

The crates.io publisher must match owner `luofang34`, repository `milsymbol`,
workflow `release.yml` and environment `crates-io`. The authentication action
exchanges GitHub's OIDC identity for a short-lived token and revokes it when
the job ends.

To test authentication without creating a tag or uploading a version, run
the **Release** workflow manually on a ref containing `workflow_dispatch`.
The **Verify Trusted Publishing (no upload)** job checks the real token
exchange; its post-action step must also confirm token revocation. The
publish job is skipped.

Use an existing release tag containing that manual entry point. To test a
workflow change on `main`, temporarily allow the `main` branch in the
`crates-io` environment's deployment rules, dispatch with
`gh workflow run release.yml --ref main`, and remove that branch exception
when the run finishes. A successful authentication test does not exercise
the crate upload endpoint.
