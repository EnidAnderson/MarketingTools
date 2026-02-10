#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-031

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
EDITOR_TEAM="${EDITOR_TEAM:-qa_fixer}"

if ! git -C "$ROOT" rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "FAIL[RQ-031] invalid base ref: $BASE_REF" >&2
  exit 31
fi

is_executable_asset() {
  local p="$1"
  [[ "$p" == *.rs || "$p" == *.py || "$p" == *.sh || "$p" == *.yml || "$p" == *.yaml || "$p" == *.json || "$p" == *.toml || "$p" == *.sql || "$p" == *.hook ]]
}

while IFS= read -r path; do
  [[ -n "$path" ]] || continue
  case "$path" in
    teams/*|pipeline/*|data/team_ops/*|scripts/*|.github/workflows/*|src/*|src-tauri/*|rustBotNetwork/*)
      ;;
    *)
      continue
      ;;
  esac
  if ! is_executable_asset "$path"; then
    continue
  fi
  if [[ "$path" == "teams/_validation/validation_report.json" ]]; then
    continue
  fi

  if [[ "$EDITOR_TEAM" != "qa_fixer" ]]; then
    echo "FAIL[RQ-031] unauthorized editor team '$EDITOR_TEAM' modified executable asset '$path'" >&2
    exit 31
  fi

  abs="$ROOT/$path"
  if [[ ! -f "$abs" ]]; then
    continue
  fi
  if ! rg -q 'decision_id|change_request_id' "$abs"; then
    echo "FAIL[RQ-031] missing provenance reference in '$path' (decision_id or change_request_id required)" >&2
    exit 31
  fi
done < <(git -C "$ROOT" diff --name-only "$BASE_REF")

echo "PASS[RQ-031] QA edit authority + provenance checks passed"
