#!/usr/bin/env bash
# Claude Code PreToolUse hook: before `but push`, `but pr` or `git push`, run
# the CI native job locally and block the push when it fails.
command=$(jq -r '.tool_input.command // ""')
case "$command" in
  *"but push"*|*"but pr new"*|*"git push"*) ;;
  *) exit 0 ;;
esac
cd "$CLAUDE_PROJECT_DIR" || exit 0
if ! output=$(tools/ci/check.sh 2>&1); then
  echo "Local CI failed; fix before pushing:" >&2
  echo "$output" | tail -40 >&2
  exit 2
fi
