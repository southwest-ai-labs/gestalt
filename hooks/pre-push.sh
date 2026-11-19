#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────
# Gestalt Pre-Push Hook — Inspect Commits Being Pushed
# ──────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

z40="0000000000000000000000000000000000000000"

while read -r local_ref local_sha remote_ref remote_sha; do
  # If local branch is being deleted on remote, skip
  if [ "$local_sha" = "$z40" ] || [[ "$local_sha" =~ ^0+$ ]]; then
    continue
  fi

  echo "🔍 Pre-push validation for $local_ref ($local_sha)..."

  if [ "$remote_sha" = "$z40" ] || [[ "$remote_sha" =~ ^0+$ ]]; then
    # New branch on remote
    if ! git diff --check "$local_sha"; then
      echo "❌ Conflict markers or whitespace error detected in $local_sha!"
      exit 1
    fi
  else
    # Existing branch on remote: check commits between remote and local
    if ! git diff --check "$remote_sha..$local_sha"; then
      echo "❌ Conflict markers or whitespace error detected in $remote_sha..$local_sha!"
      exit 1
    fi
  fi
done

exit 0
