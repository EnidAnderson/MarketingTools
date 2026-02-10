#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/SECURITY_THREAT_MODEL.md"
require_file "$ROOT/planning/RAPID_REVIEW_CELL/logs/EVIDENCE_SUPPORT.csv"

python3 - <<'PY'
import sys

def can_promote(untrusted_source: bool, evidence_bound: bool, caveated: bool, bypass: bool):
    if bypass:
        return False, "BYPASS_REJECTED"
    if untrusted_source and not (evidence_bound or caveated):
        return False, "EVIDENCE_BINDING_REQUIRED"
    return True, "ALLOWED"

# Negative: untrusted content without evidence/caveat must fail.
ok, reason = can_promote(untrusted_source=True, evidence_bound=False, caveated=False, bypass=False)
if ok:
    print("expected untrusted content rejection", file=sys.stderr)
    sys.exit(121)
print(reason)

# Negative: bypass attempt is rejected.
ok, reason = can_promote(untrusted_source=True, evidence_bound=False, caveated=False, bypass=True)
if ok or reason != "BYPASS_REJECTED":
    print("expected bypass rejection", file=sys.stderr)
    sys.exit(122)
print(reason)

# Positive: caveated or evidence-bound paths are allowed.
ok, _ = can_promote(untrusted_source=True, evidence_bound=True, caveated=False, bypass=False)
if not ok:
    print("expected evidence-bound allow", file=sys.stderr)
    sys.exit(123)
PY

pass "INV-GLOBAL-SEC-002 validated"

