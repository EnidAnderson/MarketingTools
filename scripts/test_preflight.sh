#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

"$ROOT/scripts/check_disk_space.sh"
"$ROOT/scripts/governance_preflight.sh"
"$ROOT/scripts/cargo_audit.sh"
"$ROOT/scripts/cargo_dupe_audit.sh"

echo "PASS[test_preflight]"
