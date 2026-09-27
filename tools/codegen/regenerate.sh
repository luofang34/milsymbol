#!/usr/bin/env bash
# Regenerates src/generated/*.rs from the pinned upstream milsymbol.js.
# Dev-only (needs Node); the crate itself never runs JavaScript.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
"$here/../fetch-upstream.sh"
node "$here/extract.mjs"
node "$here/extract-misc.mjs"
node "$here/emit.mjs"
