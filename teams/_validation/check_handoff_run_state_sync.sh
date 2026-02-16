#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-MGR-003

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
HANDOFF_LOG="$ROOT/data/team_ops/handoff_log.csv"
RUN_LOG="$ROOT/data/team_ops/run_registry.csv"

if ! git -C "$ROOT" rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "FAIL[RQ-MGR-003] invalid base ref: $BASE_REF" >&2
  exit 43
fi

python3 - "$ROOT" "$BASE_REF" "$HANDOFF_LOG" "$RUN_LOG" <<'PY'
import csv
import re
import subprocess
import sys
from pathlib import Path

root, base_ref, handoff_log, run_log = sys.argv[1:5]

def added_line_numbers(rel_path: str):
    proc = subprocess.run(
        ["git", "-C", root, "diff", "--unified=0", "--no-color", base_ref, "--", rel_path],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode not in (0, 1):
        raise RuntimeError(f"git diff failed for {rel_path} with code {proc.returncode}")
    lines = []
    new_line_no = None
    hunk_re = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
    for line in proc.stdout.splitlines():
        m = hunk_re.match(line)
        if m:
            new_line_no = int(m.group(1))
            continue
        if new_line_no is None or line.startswith("+++"):
            continue
        if line.startswith("+"):
            lines.append(new_line_no)
            new_line_no += 1
            continue
        if line.startswith("-") and not line.startswith("---"):
            continue
        new_line_no += 1
    return lines

handoff_added = added_line_numbers("data/team_ops/handoff_log.csv")
run_added = added_line_numbers("data/team_ops/run_registry.csv")

if not handoff_added:
    print("PASS[RQ-MGR-003] no new handoff rows added")
    sys.exit(0)

handoff_lines = Path(handoff_log).read_text(encoding="utf-8").splitlines()
run_lines = Path(run_log).read_text(encoding="utf-8").splitlines()

handoff_header = next(csv.reader([handoff_lines[0]]))
run_header = next(csv.reader([run_lines[0]]))
handoff_rows = []
run_rows = []
for ln in handoff_added:
    if ln <= 1 or ln > len(handoff_lines):
        continue
    row = next(csv.reader([handoff_lines[ln - 1]]))
    handoff_rows.append((ln, dict(zip(handoff_header, row))))
for ln in run_added:
    if ln <= 1 or ln > len(run_lines):
        continue
    row = next(csv.reader([run_lines[ln - 1]]))
    run_rows.append((ln, dict(zip(run_header, row))))

run_by_id = {}
for ln, row in run_rows:
    run_id = (row.get("run_id") or "").strip()
    if run_id:
        run_by_id.setdefault(run_id, []).append((ln, row))

errors = []
for ln, h in handoff_rows:
    run_id = (h.get("run_id") or "").strip()
    if not run_id:
        continue
    candidates = run_by_id.get(run_id, [])
    if not candidates:
        errors.append(
            f"handoff line {ln} run_id={run_id} has no appended run_registry row in same cycle"
        )
        continue

    blocking_flags = (h.get("blocking_flags") or "").strip()
    has_block = bool(blocking_flags and blocking_flags != "[]")
    if has_block:
        if not any((r.get("status") or "").strip().lower() == "blocked" for _, r in candidates):
            errors.append(
                f"handoff line {ln} run_id={run_id} has blocking_flags but no appended blocked run state"
            )

if errors:
    print("FAIL[RQ-MGR-003] handoff/run-state synchronization violations detected", file=sys.stderr)
    for item in errors:
        print(f"DETAIL[RQ-MGR-003] {item}", file=sys.stderr)
    sys.exit(43)

print(
    f"PASS[RQ-MGR-003] validated {len(handoff_rows)} added handoff row(s) with {len(run_rows)} added run-state row(s)"
)
PY

