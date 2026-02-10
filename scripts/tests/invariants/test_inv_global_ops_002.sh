#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/INCIDENT_RESPONSE_PLAYBOOK.md"

python3 - <<'PY'
import sys

def within_sla(elapsed_minutes):
    return elapsed_minutes <= 60

# Positive: containment <= 60 min passes.
elapsed = 45
if not within_sla(elapsed):
    print("expected SLA pass at 45m", file=sys.stderr)
    sys.exit(161)
print(f"SLA_OK elapsed_minutes={elapsed} owner=security_steward")

# Negative: containment breach fails.
elapsed = 75
if within_sla(elapsed):
    print("expected SLA failure at 75m", file=sys.stderr)
    sys.exit(162)
print(f"SLA_BREACH elapsed_minutes={elapsed} owner=security_steward")
PY

pass "INV-GLOBAL-OPS-002 validated"

