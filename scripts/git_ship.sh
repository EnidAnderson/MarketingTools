#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

usage() {
  echo "Usage: $0 -m \"commit message\" [-- <git-push-args...>]" >&2
  exit 2
}

MESSAGE=""
PUSH_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -m|--message)
      shift
      [[ $# -gt 0 ]] || usage
      MESSAGE="$1"
      ;;
    --)
      shift
      PUSH_ARGS=("$@")
      break
      ;;
    *)
      usage
      ;;
  esac
  shift || true
done

[[ -n "$MESSAGE" ]] || usage

# Preflight push target before committing, so commit cannot proceed without
# an available push path.
if [[ ${#PUSH_ARGS[@]} -gt 0 ]]; then
  git push --dry-run "${PUSH_ARGS[@]}" >/dev/null
else
  if ! git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' >/dev/null 2>&1; then
    echo "No upstream branch configured." >&2
    echo "Use: $0 -m \"message\" -- <remote> <branch>" >&2
    exit 1
  fi
  git push --dry-run >/dev/null
fi

# Require explicit staging to prevent accidental large commits.
if git diff --cached --quiet; then
  echo "No staged changes. Stage files first, then run git_ship.sh." >&2
  exit 1
fi

"$ROOT/scripts/secret_scan.sh" staged

export SAFE_SHIP=1
git commit -m "$MESSAGE"
unset SAFE_SHIP

"$ROOT/scripts/secret_scan.sh" tracked

if [[ ${#PUSH_ARGS[@]} -gt 0 ]]; then
  git push "${PUSH_ARGS[@]}"
else
  git push
fi

echo "Ship complete: committed and pushed with secret checks."
