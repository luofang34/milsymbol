#!/usr/bin/env bash
# Every workflow that branch pushes or pull requests start cancels the run a newer push
# supersedes, so merging a stack of pull requests does not test each superseded state to the
# end. Tag-started workflows, the release, always run to the end.
set -euo pipefail
cd "$(dirname "$0")/../.."
missing=0
for workflow in .github/workflows/*.yml; do
  if grep -qE '^  pull_request:|^    branches:' "$workflow" &&
    ! grep -qE '^  cancel-in-progress: true$' "$workflow"; then
    echo "$workflow: runs on branch pushes or pull requests without cancel-in-progress concurrency" >&2
    missing=1
  fi
done
exit "$missing"
