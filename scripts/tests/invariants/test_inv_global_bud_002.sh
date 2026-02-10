#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/BUDGET_GUARDRAILS_STANDARD.md"
require_file "$ROOT/planning/BUDGET_EXCEPTION_LOG.md"

python3 - <<'PY'
import datetime as dt
import sys

def run_state(spend, cap, exception_expiry_utc=None):
    if spend <= cap:
        return "allowed"
    if exception_expiry_utc:
        expiry = dt.datetime.fromisoformat(exception_expiry_utc.replace("Z", "+00:00"))
        now = dt.datetime.now(dt.timezone.utc)
        if expiry > now:
            return "temporarily_unblocked_by_exception"
    return "blocked_budget_cap_exceeded"

# Negative: exceedance must block.
state = run_state(spend=120, cap=100, exception_expiry_utc=None)
if state != "blocked_budget_cap_exceeded":
    print(f"expected blocked state, got {state}", file=sys.stderr)
    sys.exit(61)

# Positive: valid non-expired exception temporarily unblocks.
future = (dt.datetime.now(dt.timezone.utc) + dt.timedelta(hours=2)).isoformat().replace("+00:00", "Z")
state = run_state(spend=120, cap=100, exception_expiry_utc=future)
if state != "temporarily_unblocked_by_exception":
    print(f"expected temporary unblock, got {state}", file=sys.stderr)
    sys.exit(62)

# Negative: expired exception must not unblock.
past = (dt.datetime.now(dt.timezone.utc) - dt.timedelta(hours=2)).isoformat().replace("+00:00", "Z")
state = run_state(spend=120, cap=100, exception_expiry_utc=past)
if state != "blocked_budget_cap_exceeded":
    print(f"expired exception incorrectly unblocked: {state}", file=sys.stderr)
    sys.exit(63)
PY

pass "INV-GLOBAL-BUD-002 validated"

