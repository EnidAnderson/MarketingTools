#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-029

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
HANDOFF_LOG="$ROOT/data/team_ops/handoff_log.csv"
DOCTRINE_REF="teams/shared/OPERATING_DOCTRINE.md"
EXPECTED_ORDER=("blue" "red" "green" "black" "white" "grey" "qa_fixer")

if [[ ! -f "$HANDOFF_LOG" ]]; then
  echo "FAIL[RQ-029] missing handoff log: $HANDOFF_LOG" >&2
  exit 29
fi

python3 - "$HANDOFF_LOG" "$DOCTRINE_REF" <<'PY'
import csv
import sys

handoff_log = sys.argv[1]
doctrine_ref = sys.argv[2]
expected = ["blue", "red", "green", "black", "white", "grey", "qa_fixer"]

rows = []
with open(handoff_log, "r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        if not row.get("run_id") or not row.get("from_team") or not row.get("to_team"):
            continue
        rows.append(row)

if not rows:
    print(f"FAIL[RQ-029] no handoff rows found; doctrine={doctrine_ref}", file=sys.stderr)
    sys.exit(29)

by_run = {}
for row in rows:
    by_run.setdefault(row["run_id"], []).append(row)

for run_id, run_rows in by_run.items():
    run_rows.sort(key=lambda r: r.get("timestamp_utc", ""))
    stage_idx = 0
    for row in run_rows:
        from_team = row["from_team"].strip()
        to_team = row["to_team"].strip()
        # Allow duplicate/superseding rows for the same handoff pair in append-only logs.
        exp_from = expected[stage_idx]
        exp_to = expected[stage_idx + 1] if stage_idx + 1 < len(expected) else None
        if from_team == exp_from and to_team == exp_to:
            stage_idx += 1
            if stage_idx >= len(expected) - 1:
                break
            continue

        # Allow repeated prior successful pair as a superseding append (no stage advance).
        if stage_idx > 0:
            prev_from = expected[stage_idx - 1]
            prev_to = expected[stage_idx]
            if from_team == prev_from and to_team == prev_to:
                continue

        expected_pair = f"{exp_from}->{exp_to}" if exp_to is not None else exp_from
        observed_pair = f"{from_team}->{to_team}"
        if observed_pair != expected_pair:
            print(
                f"FAIL[RQ-029] run_id={run_id} out-of-order handoff; "
                f"expected={expected_pair}; observed={observed_pair}; doctrine={doctrine_ref}",
                file=sys.stderr,
            )
            sys.exit(29)

print(f"PASS[RQ-029] pipeline order valid; doctrine={doctrine_ref}")
PY
