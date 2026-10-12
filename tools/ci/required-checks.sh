#!/usr/bin/env bash
# Usage: tools/ci/required-checks.sh [run-id]
# Every status check that branch protection requires on the target branch is
# a job of this CI run (default: $GITHUB_RUN_ID), so a renamed or removed job
# fails here instead of leaving every pull request waiting for a check that
# nothing reports. Jobs that are not required are listed, not refused.
# Needs `gh` with a token that can read Actions (actions: read).
set -euo pipefail
run=${1:-${GITHUB_RUN_ID:?pass a workflow run ID}}
repo=${GITHUB_REPOSITORY:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}
branch=${GITHUB_BASE_REF:-${GITHUB_REF_NAME:-main}}
required=$(gh api "repos/$repo/branches/$branch" \
  --jq '.protection.required_status_checks.contexts // [] | .[]')
jobs=$(gh api --paginate "repos/$repo/actions/runs/$run/jobs" --jq '.jobs[].name')
if [ -z "$required" ]; then
  echo "$repo $branch requires no status checks"
  exit 0
fi
missing=$(comm -23 <(sort -u <<<"$required") <(sort -u <<<"$jobs"))
extra=$(comm -13 <(sort -u <<<"$required") <(sort -u <<<"$jobs"))
echo "required by $branch and run by this workflow:"
comm -12 <(sort -u <<<"$required") <(sort -u <<<"$jobs") | sed 's/^/  /'
if [ -n "$extra" ]; then
  echo "run but not required:"
  sed 's/^/  /' <<<"$extra"
fi
if [ -n "$missing" ]; then
  echo "required by $branch protection but not a job of run $run:" >&2
  sed 's/^/  /' <<<"$missing" >&2
  echo "rename the job back, or update the required checks of $branch" >&2
  exit 1
fi
