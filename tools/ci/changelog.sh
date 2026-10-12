#!/usr/bin/env bash
# Usage: tools/ci/changelog.sh [--release] [--notes]
# The changelog has an entry with text for the crate version. With --release
# the entry must carry a date, not "Unreleased". With --notes the whole entry
# is printed, as the GitHub release's notes; otherwise its heading.
set -euo pipefail
release=false notes=false
for arg in "$@"; do
  case "$arg" in
    --release) release=true ;;
    --notes) notes=true ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done
version=$(tools/ci/crate.sh version)
section=$(awk -v heading="## $version — " '
  index($0, heading) == 1 { found = 1; print; next }
  found && /^## / { exit }
  found { print }
' CHANGELOG.md)
entry=$(printf '%s\n' "$section" | head -n 1)
if [ -z "$entry" ]; then
  echo "CHANGELOG.md has no entry for version $version" >&2
  exit 1
fi
if ! printf '%s\n' "$section" | tail -n +2 | grep -q '[^[:space:]]'; then
  echo "CHANGELOG.md entry for $version has no text" >&2
  exit 1
fi
if $release && echo "$entry" | grep -q "Unreleased"; then
  echo "CHANGELOG.md entry for $version is still marked Unreleased" >&2
  exit 1
fi
if $notes; then
  printf '%s\n' "$section"
else
  echo "$entry"
fi
