#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=CR-BLACK-0001; change_request_id=CR-BLACK-0002

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BUDGET_FILE="$ROOT/data/team_ops/budget_envelopes.csv"
RELEASE_LOG="$ROOT/planning/reports/RELEASE_GATE_LOG.csv"
RUN_REGISTRY="$ROOT/data/team_ops/run_registry.csv"

python3 - "$BUDGET_FILE" "$RELEASE_LOG" "$RUN_REGISTRY" <<'PY'
import csv
import sys

budget_file, release_log, run_registry = sys.argv[1:4]

required_budget_fields = [
    "run_id", "workflow_id", "subsystem", "per_run_cap_usd", "daily_cap_usd", "monthly_cap_usd", "fallback_mode", "owner_role"
]

with open(run_registry, "r", encoding="utf-8", newline="") as f:
    run_rows = [r for r in csv.DictReader(f) if r.get("run_id")]
if not run_rows:
    print("FAIL[CR-BLACK-0001] no runs found in run_registry", file=sys.stderr)
    sys.exit(1)

active_run_id = run_rows[-1]["run_id"]

with open(budget_file, "r", encoding="utf-8", newline="") as f:
    budget_rows = [r for r in csv.DictReader(f)]
if not budget_rows:
    print("FAIL[CR-BLACK-0001] budget envelope file has no data rows", file=sys.stderr)
    sys.exit(1)

budget_for_run = [r for r in budget_rows if r.get("run_id") == active_run_id]
if not budget_for_run:
    print(f"FAIL[CR-BLACK-0001] missing budget envelope row for run_id={active_run_id}", file=sys.stderr)
    sys.exit(1)

for row in budget_for_run:
    for field in required_budget_fields:
        if not row.get(field, "").strip():
            print(f"FAIL[CR-BLACK-0001] missing required budget field '{field}' for run_id={active_run_id}", file=sys.stderr)
            sys.exit(1)
    for cap in ("per_run_cap_usd", "daily_cap_usd", "monthly_cap_usd"):
        try:
            if float(row[cap]) <= 0:
                raise ValueError
        except ValueError:
            print(f"FAIL[CR-BLACK-0001] non-positive cap '{cap}' for run_id={active_run_id}", file=sys.stderr)
            sys.exit(1)

with open(release_log, "r", encoding="utf-8", newline="") as f:
    gate_rows = [r for r in csv.DictReader(f)]
if not gate_rows:
    print("FAIL[CR-BLACK-0002] release gate log has no rows", file=sys.stderr)
    sys.exit(1)

rows_for_run = [r for r in gate_rows if r.get("release_id") == active_run_id]
if not rows_for_run:
    print(f"FAIL[CR-BLACK-0002] no release gate row for run_id={active_run_id}", file=sys.stderr)
    sys.exit(1)

mandatory = ["security_gate", "budget_gate", "evidence_gate", "role_gate", "change_gate"]
for row in rows_for_run:
    for gate in mandatory:
        value = (row.get(gate) or "").strip().lower()
        if value not in {"green", "yellow", "red"}:
            print(f"FAIL[CR-BLACK-0002] invalid gate value {gate}={value!r}", file=sys.stderr)
            sys.exit(1)
        if value == "red":
            print(f"FAIL[CR-BLACK-0002] publish blocked: {gate} is red for run_id={active_run_id}", file=sys.stderr)
            sys.exit(1)

print(f"PASS[CR-BLACK-0001/0002] budget and release gates valid for run_id={active_run_id}")
PY
