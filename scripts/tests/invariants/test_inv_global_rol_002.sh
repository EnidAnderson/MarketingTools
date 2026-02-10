#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/ROLE_ESCALATION_PROTOCOL.md"

python3 - <<'PY'
import sys

SLA_HOURS = 24

def evaluate_conflict(hours_open: int, resolved: bool):
    if resolved and hours_open <= SLA_HOURS:
        return "allow_progress", ""
    if not resolved and hours_open > SLA_HOURS:
        return "blocked", "ESCALATION_REF:ROLE_ESCALATION_PROTOCOL"
    return "in_review", ""

# Negative: unresolved conflict beyond SLA blocks.
state, ref = evaluate_conflict(hours_open=25, resolved=False)
if state != "blocked":
    print(f"expected blocked, got {state}", file=sys.stderr)
    sys.exit(131)
if "ESCALATION_REF" not in ref:
    print("missing escalation reference for SLA breach", file=sys.stderr)
    sys.exit(132)
print(ref)

# Positive: resolved within SLA allows progression.
state, _ = evaluate_conflict(hours_open=3, resolved=True)
if state != "allow_progress":
    print(f"expected allow_progress, got {state}", file=sys.stderr)
    sys.exit(133)
PY

pass "INV-GLOBAL-ROL-002 validated"

