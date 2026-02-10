#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/RAPID_REVIEW_CELL/logs/CLAIM_REGISTER.csv"
require_file "$ROOT/planning/RAPID_REVIEW_CELL/logs/EVIDENCE_SUPPORT.csv"

python3 - <<'PY'
import csv
import io
import sys

claims_csv = io.StringIO(
    """claim_id,channel
claim_001,external
claim_002,external
claim_003,internal
"""
)
evidence_csv = io.StringIO(
    """claim_id,support_status,caveat_text
claim_001,supported,
claim_003,unsupported,internal only
"""
)

claims = list(csv.DictReader(claims_csv))
evidence = {r["claim_id"]: r for r in csv.DictReader(evidence_csv)}

offending = []
for claim in claims:
    if claim["channel"] != "external":
        continue
    ev = evidence.get(claim["claim_id"])
    if not ev:
        offending.append(claim["claim_id"])
        continue
    if ev["support_status"] in {"supported", "caveated"}:
        continue
    if ev["support_status"] == "unsupported" and ev.get("caveat_text", "").strip():
        continue
    offending.append(claim["claim_id"])

if not offending:
    print("expected missing linkage failure", file=sys.stderr)
    sys.exit(81)
print("OFFENDING_CLAIM_IDS", ",".join(offending))

# Positive case with complete mapping.
evidence_csv = io.StringIO(
    """claim_id,support_status,caveat_text
claim_001,supported,
claim_002,caveated,requires context
"""
)
evidence = {r["claim_id"]: r for r in csv.DictReader(evidence_csv)}
offending = []
for claim in claims:
    if claim["channel"] != "external":
        continue
    ev = evidence.get(claim["claim_id"])
    if not ev:
        offending.append(claim["claim_id"])
        continue
    if ev["support_status"] in {"supported", "caveated"}:
        continue
    if ev["support_status"] == "unsupported" and ev.get("caveat_text", "").strip():
        continue
    offending.append(claim["claim_id"])
if offending:
    print(f"expected pass, got offending={offending}", file=sys.stderr)
    sys.exit(82)
PY

pass "INV-GLOBAL-EVD-001 validated"

