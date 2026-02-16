#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-MGR-001

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
QUEUE="$ROOT/data/team_ops/change_request_queue.csv"

python3 - "$QUEUE" <<'PY'
import csv
import sys
from collections import defaultdict

queue = sys.argv[1]
rows = []
with open(queue, "r", encoding="utf-8", newline="") as f:
    reader = csv.DictReader(f)
    for i, row in enumerate(reader, start=2):
        row["_line"] = i
        rows.append(row)

if not rows:
    print(f"FAIL[RQ-MGR-001] empty queue: {queue}", file=sys.stderr)
    sys.exit(41)

by_id = defaultdict(list)
for row in rows:
    rid = (row.get("request_id") or "").strip()
    if rid:
        by_id[rid].append(row)

violations = []
duplicate_count = 0
resolved_count = 0

for rid, group in by_id.items():
    if len(group) == 1:
        continue
    duplicate_count += 1
    group = sorted(group, key=lambda r: r["_line"])
    latest = group[-1]
    latest_status = (latest.get("status") or "").strip().lower()
    latest_sup = (latest.get("supersedes_request_id") or "").strip()

    # Duplicate IDs are considered resolved when lifecycle has a closed latest row
    # and that row provides explicit supersedes lineage.
    if latest_status == "open":
        violations.append(
            f"request_id={rid} line={latest['_line']} latest duplicate row is open; expected closed lifecycle"
        )
        continue
    if not latest_sup:
        violations.append(
            f"request_id={rid} line={latest['_line']} missing supersedes_request_id on latest duplicate row"
        )
        continue
    resolved_count += 1

if violations:
    print("FAIL[RQ-MGR-001] unresolved duplicate request_id values detected", file=sys.stderr)
    for item in violations:
        print(f"DETAIL[RQ-MGR-001] {item}", file=sys.stderr)
    sys.exit(41)

print(
    f"PASS[RQ-MGR-001] duplicate_ids={duplicate_count}; resolved_via_supersedes={resolved_count}; queue={queue}"
)
PY
