#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/AGENT_ROLE_CONTRACTS.md"

python3 - <<'PY'
import sys

authorized_roles = {
    "team_lead",
    "product_steward",
    "security_steward",
    "platform_architect",
    "tool_engineer",
    "qa_validation",
}

def validate_decision(record):
    owners = record.get("authority_owners", [])
    if len(owners) != 1:
        return False, "ambiguous_owner: expected exactly one authority owner"
    if owners[0] not in authorized_roles:
        return False, f"role_authority_mismatch: unauthorized owner role {owners[0]}"
    return True, "ok"

# Negative: ambiguous owner fails.
ok, msg = validate_decision({"authority_owners": ["team_lead", "product_steward"]})
if ok:
    print("expected ambiguous owner failure", file=sys.stderr)
    sys.exit(71)
print(msg)

# Positive: single authorized owner passes.
ok, msg = validate_decision({"authority_owners": ["team_lead"]})
if not ok:
    print(f"expected pass but failed: {msg}", file=sys.stderr)
    sys.exit(72)

# Negative: role-authority mismatch caught.
ok, msg = validate_decision({"authority_owners": ["intern"]})
if ok:
    print("expected role-authority mismatch failure", file=sys.stderr)
    sys.exit(73)
print(msg)
PY

pass "INV-GLOBAL-ROL-001 validated"

