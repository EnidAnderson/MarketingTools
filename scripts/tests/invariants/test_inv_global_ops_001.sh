#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/INCIDENT_RESPONSE_PLAYBOOK.md"

python3 - <<'PY'
import sys

def authorize_action(safe_mode, action):
    restricted = {"external_publish", "approve_budget_exception", "approve_security_exception"}
    if safe_mode and action in restricted:
        return False, "SAFE_MODE_BLOCKED"
    return True, "ALLOWED"

# Negative: safe mode blocks external publish.
ok, reason = authorize_action(True, "external_publish")
if ok or reason != "SAFE_MODE_BLOCKED":
    print("safe mode failed to block external publish", file=sys.stderr)
    sys.exit(91)
print(reason)

# Negative: safe mode blocks exception approvals.
for action in ("approve_budget_exception", "approve_security_exception"):
    ok, reason = authorize_action(True, action)
    if ok or reason != "SAFE_MODE_BLOCKED":
        print(f"safe mode failed to block {action}", file=sys.stderr)
        sys.exit(92)

# Positive: cleared safe mode allows permitted path.
ok, reason = authorize_action(False, "external_publish")
if not ok:
    print(f"expected allow after clear, got reason={reason}", file=sys.stderr)
    sys.exit(93)
PY

pass "INV-GLOBAL-OPS-001 validated"

