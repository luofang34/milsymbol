#!/usr/bin/env bash
# Hand-written .rs files stay under 500 lines and lib.rs under 100.
# src/generated/ is exempt: it is generated data (see UPSTREAM.md).
set -euo pipefail
fail=0
while IFS= read -r f; do
  case "$f" in src/generated/*) continue ;; esac
  n=$(wc -l < "$f")
  if [ "$n" -gt 500 ]; then
    echo "$f: $n lines (limit 500)" >&2
    fail=1
  fi
done < <(git ls-files --cached --others --exclude-standard '*.rs')
lib=$(wc -l < src/lib.rs)
if [ "$lib" -gt 100 ]; then echo "src/lib.rs: $lib lines (limit 100)" >&2; fail=1; fi
if git ls-files --cached --others --exclude-standard | grep -E '(^|/)mod\.rs$|(^|/)(utils|helpers|common)\.rs$'; then
  echo "use foo.rs + foo/ and domain module names (no mod.rs, utils, helpers, common)" >&2
  fail=1
fi
exit $fail
