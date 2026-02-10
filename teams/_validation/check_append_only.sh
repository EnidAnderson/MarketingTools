#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-030

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
GLOB_A="$ROOT/pipeline/*.md"
GLOB_B="$ROOT/data/team_ops/*.csv"

if ! git -C "$ROOT" rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "FAIL[RQ-030] invalid base ref: $BASE_REF" >&2
  exit 30
fi

# Reject deleted tracked files in append-only scopes.
while IFS= read -r line; do
  status="${line%%$'\t'*}"
  path="${line#*$'\t'}"
  if [[ "$status" == "D" ]]; then
    echo "FAIL[RQ-030] append-only violation: deleted file $path" >&2
    exit 30
  fi
done < <(git -C "$ROOT" diff --name-status "$BASE_REF" -- $GLOB_A $GLOB_B)

# Reject any removal hunk in append-only scopes.
if git -C "$ROOT" diff --unified=0 --no-color "$BASE_REF" -- $GLOB_A $GLOB_B \
  | rg -n '^-([^ -]|$)' >/tmp/rq030_removals.log 2>&1; then
  echo "FAIL[RQ-030] append-only violation: row mutation/delete detected in append-only scope" >&2
  echo "DETAIL[RQ-030] offending lines:" >&2
  sed -n '1,20p' /tmp/rq030_removals.log >&2
  exit 30
fi

echo "PASS[RQ-030] append-only integrity check passed for pipeline/*.md and data/team_ops/*.csv"

