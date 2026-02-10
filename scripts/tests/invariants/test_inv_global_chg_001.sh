#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/ADR_TRIGGER_RULES.md"
require_file "$ROOT/planning/adrs/ADR_TEMPLATE.md"

python3 - <<'PY'
import sys

def adr_required(change):
    return change.get("architecture_impact", False)

def validate(change):
    if adr_required(change) and not change.get("adr_ref"):
        return False, "missing ADR path/id for architecture-impacting change"
    return True, "ok"

# Negative: architecture-triggered change without ADR fails.
ok, msg = validate({"architecture_impact": True, "adr_ref": ""})
if ok:
    print("expected ADR requirement failure", file=sys.stderr)
    sys.exit(151)
print(msg)

# Positive: architecture-triggered change with ADR passes.
ok, msg = validate({"architecture_impact": True, "adr_ref": "planning/adrs/ADR-0001.md"})
if not ok:
    print(f"expected pass, got {msg}", file=sys.stderr)
    sys.exit(152)
PY

pass "INV-GLOBAL-CHG-001 validated"

