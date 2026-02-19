#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

BASELINE_FILE="$ROOT/scripts/tests/cargo_duplicate_baseline_app_core.txt"

if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "FAIL[cargo_dupe_audit] missing baseline: $BASELINE_FILE" >&2
  exit 1
fi

tmp_current="$(mktemp)"
tmp_new="$(mktemp)"
tmp_removed="$(mktemp)"
cleanup() {
  rm -f "$tmp_current" "$tmp_new" "$tmp_removed"
}
trap cleanup EXIT

cargo tree -d -p app_core --prefix none \
  | sed -nE 's/^([A-Za-z0-9_-]+) v[0-9].*/\1/p' \
  | sort -u > "$tmp_current"

comm -13 "$BASELINE_FILE" "$tmp_current" > "$tmp_new"
comm -23 "$BASELINE_FILE" "$tmp_current" > "$tmp_removed"

if [[ -s "$tmp_new" ]]; then
  echo "FAIL[cargo_dupe_audit] new duplicate dependency families detected:"
  cat "$tmp_new"
  echo "Update baseline only after security review: $BASELINE_FILE"
  exit 1
fi

if [[ -s "$tmp_removed" ]]; then
  echo "WARN[cargo_dupe_audit] duplicate families removed from current graph:"
  cat "$tmp_removed"
fi

echo "PASS[cargo_dupe_audit] baseline matches current duplicate dependency families"
