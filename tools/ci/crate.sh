#!/usr/bin/env bash
# Usage: tools/ci/crate.sh name|version
# The package at the workspace root: the crate this repository publishes.
set -euo pipefail
field=${1:-}
case "$field" in
  name | version) ;;
  *) echo "usage: tools/ci/crate.sh name|version" >&2; exit 2 ;;
esac
cargo metadata --no-deps --format-version 1 | jq -er --arg field "$field" \
  '.workspace_root as $root | .packages[] | select(.manifest_path == ($root + "/Cargo.toml")) | .[$field]'
