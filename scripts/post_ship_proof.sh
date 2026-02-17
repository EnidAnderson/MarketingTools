#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

echo "POST_SHIP_PROOF_BEGIN"
echo "local_head=$(git rev-parse HEAD)"
echo "latest_commit=$(git log -n 1 --oneline)"
echo "remote=$(git remote -v | tr '\n' ';' | sed 's/;*$//')"
echo "branch=$(git branch --show-current)"
echo "status_porcelain=$(git status --porcelain | tr '\n' ';' | sed 's/;*$//')"

if remote_line="$(git ls-remote --heads origin main | head -n 1)"; then
  echo "remote_main=${remote_line}"
else
  echo "remote_main=UNREACHABLE"
  echo "POST_SHIP_PROOF_WARN commit created locally but not pushed"
  exit 1
fi

echo "POST_SHIP_PROOF_END"
