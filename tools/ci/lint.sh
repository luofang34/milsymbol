#!/usr/bin/env bash
# Usage: tools/ci/lint.sh [toolchain[:lib]]...
# Runs clippy with warnings denied on each toolchain (default: stable). A
# toolchain with the `:lib` suffix lints only the library, for versions whose
# dev-dependencies do not build. The name `default` uses the active toolchain.
set -euo pipefail
toolchains=("$@")
[ ${#toolchains[@]} -gt 0 ] || toolchains=(stable)
for spec in "${toolchains[@]}"; do
  tc=${spec%%:*}
  echo "== clippy on $tc"
  plus="+$tc"
  [ "$tc" != default ] || plus=""
  if [ "$spec" != "${spec%:lib}" ]; then
    for features in "--all-features" "--no-default-features" "--no-default-features --features compact-paths"; do
      cargo $plus clippy -p milsymbol --lib $features -- -D warnings
    done
  else
    cargo $plus clippy --workspace --all-targets -- -D warnings
    cargo $plus clippy -p milsymbol --all-targets --all-features -- -D warnings
    cargo $plus clippy -p milsymbol --all-targets --no-default-features -- -D warnings
    cargo $plus clippy -p milsymbol --all-targets --no-default-features --features compact-paths -- -D warnings
    (cd fuzz && cargo $plus clippy -- -D warnings)
  fi
done
