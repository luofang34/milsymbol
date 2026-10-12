#!/usr/bin/env bash
# Usage: tools/ci/release-auth.sh
# Releases authenticate only through Trusted Publishing: no workflow reads a
# registry token from secrets or switches authentication on a variable.
set -euo pipefail
if grep -nE 'secrets\.CARGO_REGISTRY_TOKEN|vars\.CRATES_IO' .github/workflows/*.yml; then
  echo "release authentication must use Trusted Publishing only" >&2
  exit 1
fi
