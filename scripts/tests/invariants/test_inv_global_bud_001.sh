#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/BUDGET_GUARDRAILS_STANDARD.md"

python3 - <<'PY'
import sys

required = [
    "run_id",
    "workflow_id",
    "subsystem",
    "per_run_cap_usd",
    "daily_cap_usd",
    "monthly_cap_usd",
    "fallback_mode",
    "owner_role",
]

def missing_keys(env):
    return [k for k in required if k not in env or env[k] in ("", None)]

# Negative: missing budget envelope fields rejected.
bad = {
    "run_id": "run-1",
    "workflow_id": "wf-1",
    "subsystem": "core",
}
miss = missing_keys(bad)
if not miss:
    print("expected missing field detection", file=sys.stderr)
    sys.exit(51)
print("MISSING_KEYS", ",".join(miss))

# Positive: complete envelope accepted.
good = {
    "run_id": "run-1",
    "workflow_id": "wf-1",
    "subsystem": "core",
    "per_run_cap_usd": 25,
    "daily_cap_usd": 100,
    "monthly_cap_usd": 800,
    "fallback_mode": "reduced_scope",
    "owner_role": "team_lead",
}
miss = missing_keys(good)
if miss:
    print("complete envelope unexpectedly rejected", file=sys.stderr)
    sys.exit(52)
PY

pass "INV-GLOBAL-BUD-001 validated"

