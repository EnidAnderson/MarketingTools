#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/invariants/global/INV-GLOBAL-GOV-002.md"

python3 - <<'PY'
import sys

required_roles = {"technical_owner", "business_owner"}

def validate_signoff(signoffs):
    missing = sorted(required_roles - set(signoffs))
    if missing:
        return False, f"missing_required_role={missing[0]}"
    return True, "ok"

# Negative: fail without two-role signoff.
ok, msg = validate_signoff(["technical_owner"])
if ok:
    print("expected two-person signoff failure", file=sys.stderr)
    sys.exit(101)
print(msg)

# Positive: technical + business signoff passes.
ok, msg = validate_signoff(["technical_owner", "business_owner"])
if not ok:
    print(f"expected pass, got {msg}", file=sys.stderr)
    sys.exit(102)
PY

pass "INV-GLOBAL-GOV-002 validated"

