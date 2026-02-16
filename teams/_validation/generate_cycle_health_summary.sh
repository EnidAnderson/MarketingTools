#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-MGR-005

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
QUEUE="$ROOT/data/team_ops/change_request_queue.csv"
HANDOFF="$ROOT/data/team_ops/handoff_log.csv"
RUNS="$ROOT/data/team_ops/run_registry.csv"
OUT="$ROOT/teams/_validation/cycle_health_summary.json"

python3 - "$QUEUE" "$HANDOFF" "$RUNS" "$OUT" <<'PY'
import csv
import json
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone

queue_path, handoff_path, runs_path, out_path = sys.argv[1:5]

def read_csv(path):
    with open(path, "r", encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f))

queue_rows = read_csv(queue_path)
handoff_rows = read_csv(handoff_path)
run_rows = read_csv(runs_path)

latest_request = {}
for row in queue_rows:
    rid = (row.get("request_id") or "").strip()
    if rid:
        latest_request[rid] = row

open_requests = [r for r in latest_request.values() if (r.get("status") or "").strip().lower() == "open"]
duplicate_ids = Counter((r.get("request_id") or "").strip() for r in queue_rows if (r.get("request_id") or "").strip())
duplicate_count = sum(1 for _, c in duplicate_ids.items() if c > 1)

latest_run = {}
for row in run_rows:
    run_id = (row.get("run_id") or "").strip()
    if run_id:
        latest_run[run_id] = row

stage_map = defaultdict(lambda: defaultdict(int))
block_reasons = defaultdict(list)
for row in handoff_rows:
    run_id = (row.get("run_id") or "").strip()
    if not run_id:
        continue
    pair = f"{(row.get('from_team') or '').strip()}->{(row.get('to_team') or '').strip()}"
    stage_map[run_id][pair] += 1
    flags = (row.get("blocking_flags") or "").strip()
    if flags and flags != "[]":
        block_reasons[run_id].append(flags)

payload = {
    "schema_version": "1",
    "generated_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "duplicate_request_id_count": duplicate_count,
    "unresolved_request_count": len(open_requests),
    "unresolved_request_ids": sorted((r.get("request_id") or "").strip() for r in open_requests),
    "runs": [],
}

for run_id, run_state in sorted(latest_run.items()):
    payload["runs"].append(
        {
            "run_id": run_id,
            "status": (run_state.get("status") or "").strip(),
            "current_phase": (run_state.get("current_phase") or "").strip(),
            "stage_completion_map": dict(sorted(stage_map[run_id].items())),
            "block_reasons": block_reasons.get(run_id, []),
        }
    )

with open(out_path, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2, sort_keys=True)

print(f"PASS[RQ-MGR-005] wrote cycle summary: {out_path}")
PY

