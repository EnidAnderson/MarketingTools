#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-MGR-006

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
RUN_LOG="$ROOT/data/team_ops/run_registry.csv"
MODE_LOG_REL="teams/shared/run_mode_registry.csv"
MODE_LOG="$ROOT/$MODE_LOG_REL"
HANDOFF_LOG="$ROOT/data/team_ops/handoff_log.csv"

if ! git -C "$ROOT" rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "FAIL[RQ-MGR-006] invalid base ref: $BASE_REF" >&2
  exit 46
fi

python3 - "$ROOT" "$BASE_REF" "$RUN_LOG" "$MODE_LOG" "$HANDOFF_LOG" "$MODE_LOG_REL" <<'PY'
import csv
import re
import subprocess
import sys
from pathlib import Path

root, base_ref, run_log, mode_log, handoff_log, mode_log_rel = sys.argv[1:7]


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


def parse_added_rows(abs_path: str, rel_path: str):
    lines = Path(abs_path).read_text(encoding="utf-8").splitlines()
    if not lines:
        return []
    header = next(csv.reader([lines[0]]))
    out = []
    for ln in added_line_numbers(rel_path):
        if ln <= 1 or ln > len(lines):
            continue
        row = next(csv.reader([lines[ln - 1]]))
        out.append((ln, dict(zip(header, row))))
    return out


run_added = parse_added_rows(run_log, "data/team_ops/run_registry.csv")
handoff_added = parse_added_rows(handoff_log, "data/team_ops/handoff_log.csv")
mode_added = parse_added_rows(mode_log, mode_log_rel)

if not Path(mode_log).exists():
    print(f"FAIL[RQ-MGR-006] missing {mode_log_rel}", file=sys.stderr)
    sys.exit(46)

with open(mode_log, "r", encoding="utf-8", newline="") as f:
    mode_rows = [r for r in csv.DictReader(f)]

latest_mode = {}
for row in mode_rows:
    run_id = (row.get("run_id") or "").strip()
    if not run_id:
        continue
    ts = (row.get("declared_utc") or "").strip()
    prev = latest_mode.get(run_id)
    if prev is None or ts >= (prev.get("declared_utc") or ""):
        latest_mode[run_id] = row

valid_modes = {"full", "lite"}
errors = []

for idx, row in mode_added:
    mode = (row.get("pipeline_mode") or "").strip().lower()
    run_id = (row.get("run_id") or "").strip()
    if not run_id:
        errors.append(f"run_mode_registry line {idx} missing run_id")
    if mode not in valid_modes:
        errors.append(
            f"run_mode_registry line {idx} has invalid pipeline_mode='{row.get('pipeline_mode', '')}'"
        )

impacted_runs = set()
for _, row in run_added:
    run_id = (row.get("run_id") or "").strip()
    if run_id:
        impacted_runs.add(run_id)
for _, row in handoff_added:
    run_id = (row.get("run_id") or "").strip()
    if run_id:
        impacted_runs.add(run_id)

if not impacted_runs:
    print("PASS[RQ-MGR-006] no new run/handoff rows added")
    sys.exit(0)

for run_id in sorted(impacted_runs):
    mode_row = latest_mode.get(run_id)
    if mode_row is None:
        errors.append(f"run_id={run_id} has no pipeline mode declaration in run_mode_registry")
        continue
    mode = (mode_row.get("pipeline_mode") or "").strip().lower()
    if mode not in valid_modes:
        errors.append(f"run_id={run_id} has invalid declared pipeline_mode='{mode}'")

if errors:
    print("FAIL[RQ-MGR-006] run pipeline mode registry violations detected", file=sys.stderr)
    for item in errors:
        print(f"DETAIL[RQ-MGR-006] {item}", file=sys.stderr)
    sys.exit(46)

print(
    f"PASS[RQ-MGR-006] validated pipeline mode declarations for {len(impacted_runs)} impacted run(s)"
)
PY
