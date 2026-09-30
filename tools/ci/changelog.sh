#!/usr/bin/env bash
# Usage: tools/ci/changelog.sh [--release]
# The changelog has an entry for the crate version. With --release the entry
# must carry a date, not "Unreleased".
set -euo pipefail
version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "milsymbol") | .version')
entry=$(grep -E "^## ${version//./\\.} — " CHANGELOG.md || true)
if [ -z "$entry" ]; then
  echo "CHANGELOG.md has no entry for version $version" >&2
  exit 1
fi
if [ "${1:-}" = "--release" ] && echo "$entry" | grep -q "Unreleased"; then
  echo "CHANGELOG.md entry for $version is still marked Unreleased" >&2
  exit 1
fi
echo "$entry"
