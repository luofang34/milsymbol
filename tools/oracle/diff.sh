#!/usr/bin/env bash
# Differential check of one corpus suite: renders the cases through
# milsymbol.js and the Rust port and reports mismatches.
# Usage: tools/oracle/diff.sh <suite> [maxReports]
#
# ORACLE_NODE selects the Node binary (default: node). Its architecture is
# passed to the Rust side as the reference platform, because V8's Math.sin
# and Math.cos differ in the last bit between x64 and arm64 builds.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
out="$here/out"
mkdir -p "$out"
suite="$1"
node_bin="${ORACLE_NODE:-node}"
"$node_bin" --test "$here/json-lines.test.mjs" "$here/render.test.mjs"
arch="$("$node_bin" -p process.arch)"
"$node_bin" "$here/cases.mjs" "$suite" > "$out/$suite.cases.jsonl"
"$node_bin" "$here/render.mjs" < "$out/$suite.cases.jsonl" > "$out/$suite.oracle.jsonl"
(cd "$root" && ORACLE_ARCH="$arch" cargo run -q --release --example dump) < "$out/$suite.cases.jsonl" > "$out/$suite.rust.jsonl"
status=0
(cd "$root" && cargo xtask compare "$out/$suite.cases.jsonl" "$out/$suite.oracle.jsonl" "$out/$suite.rust.jsonl" "${2:-10}") || status=$?
# Record files reach gigabytes for the base suite; keep them only on request.
if [ -z "${KEEP_RECORDS:-}" ]; then
  rm -f "$out/$suite.oracle.jsonl" "$out/$suite.rust.jsonl"
fi
exit $status
