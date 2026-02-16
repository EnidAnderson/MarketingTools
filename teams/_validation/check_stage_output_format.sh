#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-MGR-004

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
TEAM_REGISTRY="$ROOT/data/team_ops/team_registry.csv"

python3 - "$TEAM_REGISTRY" <<'PY'
import csv
import sys
from pathlib import Path

team_registry = Path(sys.argv[1])
if not team_registry.exists():
    print(f"FAIL[RQ-MGR-004] missing team registry: {team_registry}", file=sys.stderr)
    sys.exit(44)

required_sections = [
    "1. Summary (<= 300 words).",
    "2. Numbered findings.",
    "3. Open questions (if any).",
    "4. Explicit non-goals.",
]

errors = []
checked = 0
with team_registry.open("r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        team_id = (row.get("team_id") or "").strip()
        output_file = (row.get("output_file") or "").strip()
        if not team_id or not output_file:
            continue
        path = Path("/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam") / output_file
        if not path.exists():
            errors.append(f"{team_id}: missing output file {output_file}")
            continue
        text = path.read_text(encoding="utf-8")
        checked += 1
        for section in required_sections:
            if section not in text:
                errors.append(f"{team_id}: {output_file} missing section '{section}'")

if errors:
    print("FAIL[RQ-MGR-004] required output-format sections missing", file=sys.stderr)
    for item in errors:
        print(f"DETAIL[RQ-MGR-004] {item}", file=sys.stderr)
    sys.exit(44)

print(f"PASS[RQ-MGR-004] validated required sections for {checked} stage output file(s)")
PY

