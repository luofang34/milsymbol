# Releasing

Releases are published to crates.io by `.github/workflows/release.yml` when a
`v*` tag is pushed. Nothing is published otherwise.

## Before a release

1. Update `version` in `Cargo.toml` and the `CHANGELOG.md` entry.
2. Make sure CI on `main` is green (it includes `cargo publish --dry-run`).
   The release job refuses a tag whose commit is not on `main` or has no
   successful CI run; after CI passes, re-run the failed release job.

## Repository protections

- `main`: no force pushes or deletion; the CI jobs are required status
  checks (repository admins may still push directly).
- Tags `v*`: only repository admins may create them; they cannot be moved
  or deleted.
- Environment `crates-io` (used by the release job and holding its
  secrets): deployments only from `v*` tags. No reviewer is required.

## First release (API token)

crates.io only allows Trusted Publishing for crates that already exist, so
the first release uses an API token:

1. crates.io → Account Settings → API Tokens: create a token with the
   `publish-new` and `publish-update` scopes, restricted to the `milsymbol`
   crate.
2. GitHub → Settings → Secrets and variables → Actions → **Repository
   secrets**: add `CARGO_REGISTRY_TOKEN`. (Or add it as an environment secret
   of the `crates-io` environment, which the release jobs use.)
3. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.

## Switching to Trusted Publishing

After the first release:

1. crates.io → `milsymbol` → Settings → Trusted Publishing → add a GitHub
   publisher: owner `luofang34`, repository `milsymbol`, workflow
   `release.yml`, environment `crates-io`.
2. GitHub → Settings → Secrets and variables → Actions → **Variables**: add
   `CRATES_IO_TRUSTED_PUBLISHING` = `true`. The release job then obtains a
   short-lived token through `rust-lang/crates-io-auth-action` (OIDC).
3. After a successful Trusted-Publishing release, delete the
   `CARGO_REGISTRY_TOKEN` secret and revoke the token on crates.io.
4. Optionally enable "require Trusted Publishing" in the crate settings so
   API tokens can no longer publish it.

The Trusted Publishing exchange can only be exercised by a real release run.
