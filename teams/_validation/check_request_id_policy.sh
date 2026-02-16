#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0003; change_request_id=CR-RED-0012

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
QUEUE="$ROOT/data/team_ops/change_request_queue.csv"

if ! git -C "$ROOT" rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "FAIL[RQ-034] invalid base ref: $BASE_REF" >&2
  exit 34
fi

python3 - "$ROOT" "$BASE_REF" "$QUEUE" <<'PY'
import csv
import re
import subprocess
import sys
from pathlib import Path

root, base_ref, queue_path = sys.argv[1], sys.argv[2], sys.argv[3]
queue_rel = "data/team_ops/change_request_queue.csv"
canonical_pattern = re.compile(r"^CR-(BLUE|RED|GREEN|BLACK|WHITE|GREY)-([0-9]{4})$")
legacy_pattern = re.compile(r"^CR-([0-9]{4})-(BLUE|RED|GREEN|BLACK|WHITE|GREY)$")
proc = subprocess.run(
    ["git", "-C", root, "diff", "--unified=0", "--no-color", base_ref, "--", queue_rel],
    capture_output=True,
    text=True,
    check=False,
)
if proc.returncode not in (0, 1):
    print(f"FAIL[RQ-034] git diff failed with code {proc.returncode}", file=sys.stderr)
    sys.exit(34)
diff_text = proc.stdout

added_line_numbers = []
new_line_no = None
hunk_re = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
for line in diff_text.splitlines():
    m = hunk_re.match(line)
    if m:
        new_line_no = int(m.group(1))
        continue
    if new_line_no is None:
        continue
    if line.startswith("+++"):
        continue
    if line.startswith("+"):
        added_line_numbers.append(new_line_no)
        new_line_no += 1
        continue
    if line.startswith("-") and not line.startswith("---"):
        continue
    new_line_no += 1

if not added_line_numbers:
    print("PASS[RQ-034] no new change-request rows added")
    sys.exit(0)

lines = Path(queue_path).read_text(encoding="utf-8").splitlines()
if len(lines) < 1:
    print("FAIL[RQ-034] change request queue is empty", file=sys.stderr)
    sys.exit(34)

header = next(csv.reader([lines[0]]))
try:
    req_idx = header.index("request_id")
    src_idx = header.index("source_team")
except ValueError as exc:
    print(f"FAIL[RQ-034] missing required header: {exc}", file=sys.stderr)
    sys.exit(34)

errors = []
added_request_ids = []

for line_no in added_line_numbers:
    if line_no <= 1:
        continue
    if line_no > len(lines):
        errors.append(f"line {line_no}: not present in current queue file")
        continue
    raw = lines[line_no - 1]
    try:
        row = next(csv.reader([raw]))
    except Exception as exc:
        errors.append(f"line {line_no}: invalid csv row ({exc})")
        continue
    if len(row) <= max(req_idx, src_idx):
        errors.append(f"line {line_no}: missing request_id/source_team columns")
        continue

    request_id = row[req_idx].strip()
    source_team = row[src_idx].strip().lower()
    canonical_match = canonical_pattern.match(request_id)
    legacy_match = legacy_pattern.match(request_id)
    if not canonical_match and not legacy_match:
        errors.append(
            f"line {line_no}: request_id '{request_id}' must match CR-<TEAM>-NNNN (TEAM in BLUE|RED|GREEN|BLACK|WHITE|GREY)"
        )
        continue
    # Transitional support: legacy CR-NNNN-TEAM ids remain valid while migration
    # is in-progress; new canonical ids should use CR-TEAM-NNNN.
    if canonical_match:
        id_team = canonical_match.group(1).lower()
    else:
        id_team = legacy_match.group(2).lower()
    if id_team != source_team:
        errors.append(
            f"line {line_no}: request_id team '{id_team}' does not match source_team '{source_team}'"
        )
        continue
    added_request_ids.append(request_id)

all_ids = []
with open(queue_path, "r", encoding="utf-8", newline="") as f:
    reader = csv.DictReader(f)
    for row in reader:
        rid = (row.get("request_id") or "").strip()
        if rid:
            all_ids.append(rid)

for rid in added_request_ids:
    if all_ids.count(rid) > 1:
        dup_rows = []
        with open(queue_path, "r", encoding="utf-8", newline="") as f:
            for row in csv.DictReader(f):
                if (row.get("request_id") or "").strip() == rid:
                    dup_rows.append(row)
        latest = dup_rows[-1]
        latest_status = (latest.get("status") or "").strip().lower()
        latest_supersedes = (latest.get("supersedes_request_id") or "").strip()
        if latest_status == "open":
            errors.append(
                f"request_id '{rid}' is duplicated and latest row is still open; duplicate lifecycle must be explicitly closed"
            )
        elif not latest_supersedes:
            errors.append(
                f"request_id '{rid}' is duplicated and latest row is missing supersedes_request_id lineage"
            )

if errors:
    print("FAIL[RQ-034] change request ID policy violations detected", file=sys.stderr)
    for err in errors:
        print(f"DETAIL[RQ-034] {err}", file=sys.stderr)
    sys.exit(34)

print("PASS[RQ-034] team-coded and unique request IDs validated for added queue rows")
PY
