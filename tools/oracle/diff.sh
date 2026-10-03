#!/usr/bin/env bash
# Differential check of one whole corpus suite against milsymbol.js; see
# check-suite.sh, which CI runs shard by shard.
# Usage: tools/oracle/diff.sh <suite> [maxReports]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
"${ORACLE_NODE:-node}" --test "$here/json-lines.test.mjs" "$here/render.test.mjs"
exec "$here/check-suite.sh" "$1" 1/1 "${2:-10}"
