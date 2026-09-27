#!/usr/bin/env bash
# Clone the pinned upstream milsymbol.js into tools/oracle/upstream (gitignored).
# Dev-only: the Rust crate never needs this checkout.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
pin="$here/oracle/pin.json"
repo=$(node -e "console.log(require('$pin').upstream)")
commit=$(node -e "console.log(require('$pin').commit)")
dest="$here/oracle/upstream"
if [ -d "$dest/.git" ]; then
  git -C "$dest" fetch -q --tags origin
else
  git clone -q "$repo" "$dest"
fi
git -C "$dest" checkout -q "$commit"
actual=$(git -C "$dest" rev-parse HEAD)
if [ "$actual" != "$commit" ]; then
  echo "upstream checkout is $actual, expected $commit" >&2
  exit 1
fi
echo "upstream milsymbol at $actual"
