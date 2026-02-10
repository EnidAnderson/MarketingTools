#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-tracked}"
if [[ "$MODE" != "tracked" && "$MODE" != "staged" ]]; then
  echo "Usage: $0 [tracked|staged]" >&2
  exit 2
fi

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

SECRET_PATTERN='(AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35}|gh[pousr]_[A-Za-z0-9]{36,255}|xox[baprs]-[A-Za-z0-9-]+|-----BEGIN (RSA|OPENSSH|EC|DSA|PRIVATE) KEY-----|([Aa][Pp][Ii][_-]?[Kk][Ee][Yy]|[Ss][Ee][Cc][Rr][Ee][Tt]|[Tt][Oo][Kk][Ee][Nn]|[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd])[[:space:]]*[:=][[:space:]]*["'"'"'`]?[A-Za-z0-9_./+\-=]{20,})'
ALLOW_PATTERN='(YOUR_[A-Z0-9_]+|EXAMPLE|PLACEHOLDER|DUMMY|CHANGEME|REPLACE_ME|NOT_A_REAL_KEY|<token>|<secret>)'

run_scan() {
  local scope="$1"
  local raw=""

  if [[ "$scope" == "tracked" ]]; then
    raw="$(git grep -nI -E "$SECRET_PATTERN" -- . 2>/dev/null || true)"
  else
    if git diff --cached --quiet; then
      echo "No staged changes to scan."
      return 0
    fi
    raw="$(git grep -nI --cached -E "$SECRET_PATTERN" -- . 2>/dev/null || true)"
  fi

  if [[ -z "$raw" ]]; then
    return 0
  fi

  local filtered
  filtered="$(printf '%s\n' "$raw" | rg -v -i "$ALLOW_PATTERN" || true)"

  if [[ -n "$filtered" ]]; then
    echo "Secret scan failed in $scope scope. Potential leaks:" >&2
    printf '%s\n' "$filtered" >&2
    return 1
  fi

  return 0
}

run_scan "$MODE"
echo "Secret scan passed ($MODE)."
