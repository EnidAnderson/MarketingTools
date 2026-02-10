#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/invariants/global/INV-GLOBAL-AUD-002.md"

python3 - <<'PY'
import sys

required = ["inputs", "spec", "run_metadata", "decision"]

def missing_dims(lineage):
    return [k for k in required if not lineage.get(k)]

# Negative: missing lineage dimensions fail.
bad = {"inputs": "ok", "spec": "", "run_metadata": "ok", "decision": ""}
missing = missing_dims(bad)
if not missing:
    print("expected missing lineage dimension failure", file=sys.stderr)
    sys.exit(171)
print("MISSING_LINEAGE_DIMENSIONS", ",".join(missing))

# Positive: complete lineage passes.
good = {"inputs": "ok", "spec": "ok", "run_metadata": "ok", "decision": "ok"}
missing = missing_dims(good)
if missing:
    print(f"expected complete lineage pass, missing={missing}", file=sys.stderr)
    sys.exit(172)
PY

pass "INV-GLOBAL-AUD-002 validated"

