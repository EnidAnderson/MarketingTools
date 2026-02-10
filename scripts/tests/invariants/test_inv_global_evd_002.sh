#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/RAPID_REVIEW_CELL/SUMMARY_TEMPLATE.md"

python3 - <<'PY'
import sys

def validate(summary):
    unsupported = set(summary["unsupported_claim_ids"])
    safe = set(summary["safe_to_say_claim_ids"])
    conflict = sorted(unsupported & safe)
    if conflict:
        return False, f"claim_id={conflict[0]} section=Safe to say"
    return True, "ok"

# Negative: unsupported claim in safe section must fail.
bad = {
    "unsupported_claim_ids": ["claim_777"],
    "safe_to_say_claim_ids": ["claim_100", "claim_777"],
}
ok, msg = validate(bad)
if ok:
    print("expected unsupported claim placement failure", file=sys.stderr)
    sys.exit(111)
print(msg)

# Positive: unsupported claims only in do-not-claim section.
good = {
    "unsupported_claim_ids": ["claim_777"],
    "safe_to_say_claim_ids": ["claim_100", "claim_200"],
}
ok, msg = validate(good)
if not ok:
    print(f"expected pass, got {msg}", file=sys.stderr)
    sys.exit(112)
PY

pass "INV-GLOBAL-EVD-002 validated"

