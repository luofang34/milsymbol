#!/usr/bin/env bash
# Hand-written .rs files stay under 500 lines. src/generated/ is exempt:
# it is data emitted by tools/codegen (see UPSTREAM.md).
set -euo pipefail
fail=0
while IFS= read -r f; do
  case "$f" in src/generated/*) continue ;; esac
  n=$(wc -l < "$f")
  if [ "$n" -gt 500 ]; then
    echo "$f: $n lines (limit 500)" >&2
    fail=1
  fi
done < <(git ls-files '*.rs')
lib=$(wc -l < src/lib.rs)
if [ "$lib" -gt 100 ]; then echo "src/lib.rs: $lib lines (limit 100)" >&2; fail=1; fi
exit $fail
