#!/usr/bin/env bash
# Differential check of a suite, or one shard of it: renders the cases
# through milsymbol.js once, checks the committed fixture against those
# records (x64 Node only), renders the same cases with the Rust port and
# reports every mismatch.
# Usage: tools/oracle/check-suite.sh <suite> [shard/shards] [maxReports]
#
# ORACLE_NODE selects the Node binary (default: node). Its architecture is
# passed to the Rust side as the reference platform, because V8's Math.sin
# and Math.cos differ in the last bit between x64 and arm64 builds.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
# A directory per run, so concurrent runs never share record files.
out="$here/out/$$"
suite="$1"
spec="${2:-1/1}"
shard="${spec%/*}"
shards="${spec#*/}"
node_bin="${ORACLE_NODE:-node}"
arch="$("$node_bin" -p process.arch)"
name="$suite.$shard-of-$shards"
status=0
"$node_bin" "$here/check.mjs" "$suite" "$shard" "$shards" "$out" || status=$?
cases="$out/$name.cases.jsonl"
(cd "$root" && ORACLE_ARCH="$arch" cargo run -q --release --example dump) < "$cases" > "$out/$name.rust.jsonl"
(cd "$root" && cargo xtask compare "$cases" "$out/$name.oracle.jsonl" "$out/$name.rust.jsonl" "${3:-10}") || status=$?
# Record files reach gigabytes for the base suite; keep them only on request.
if [ -z "${KEEP_RECORDS:-}" ]; then
  rm -rf "$out"
else
  echo "records kept in $out" >&2
fi
exit $status
