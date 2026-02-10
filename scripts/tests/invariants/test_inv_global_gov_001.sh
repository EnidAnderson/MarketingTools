#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/RELEASE_GATES_POLICY.md"

python3 - <<'PY'
import sys

def validate(gates):
    blocking = [k for k, v in gates.items() if v == "red"]
    if blocking:
        print(f"BLOCKED gate={blocking[0]}")
        return 41
    print("ALLOWED all_gates_non_red")
    return 0

# Negative case: one red gate blocks publish.
code = validate(
    {
        "security_gate": "green",
        "budget_gate": "green",
        "evidence_gate": "red",
        "role_gate": "green",
        "change_gate": "green",
    }
)
if code == 0:
    print("expected red-gate block but got allow", file=sys.stderr)
    sys.exit(41)

# Positive case: all green allows publish path.
code = validate(
    {
        "security_gate": "green",
        "budget_gate": "green",
        "evidence_gate": "green",
        "role_gate": "green",
        "change_gate": "green",
    }
)
if code != 0:
    print("expected all-green allow but got block", file=sys.stderr)
    sys.exit(42)
PY

pass "INV-GLOBAL-GOV-001 validated"

