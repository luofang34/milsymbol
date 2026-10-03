#!/usr/bin/env bash
# Differential check of every corpus suite against milsymbol.js, split into
# work units that run in parallel, one per CPU. With job/jobs, runs only
# every jobs-th unit starting at job, so CI can spread the units over
# several machines.
# Usage: tools/oracle/check-all.sh [job/jobs]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
spec="${1:-1/1}"
job="${spec%/*}"
jobs="${spec#*/}"
units=()
add() { for ((k = 1; k <= $2; k++)); do units+=("$1 $k/$2"); done; }
# Shard counts follow the suites' oracle time; base and modifiers dominate.
add base 8
add modifiers 4
add direction 2
add layout 2
add fuzz 1
add options 1
add config 1
add invalid 1
add known 1
mine=()
for i in "${!units[@]}"; do
  if (( i % jobs == job - 1 )); then mine+=("${units[$i]}"); fi
done
(cd "$root" && cargo build -q --release --example dump && cargo build -q --release -p xtask)
if [ "$job" = 1 ]; then
  "${ORACLE_NODE:-node}" --test "$here/json-lines.test.mjs" "$here/render.test.mjs" >/dev/null
fi
logs="$(mktemp -d)"
cpus="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)"
printf '%s\n' "${mine[@]}" | xargs -P "$cpus" -I{} bash -c '
  unit="{}"; name="${unit// /_}"; log="$1/${name//\//-}.log"
  if "$2/check-suite.sh" $unit > "$log" 2>&1; then echo "ok   $unit"; else echo "FAIL $unit"; fi
' _ "$logs" "$here" | sort -k2 | tee "$logs/summary"
for f in "$logs"/*.log; do
  echo "== $(basename "$f" .log)"
  grep -vE '^\s*$' "$f" || true
done
! grep -q '^FAIL' "$logs/summary"
