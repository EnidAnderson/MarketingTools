#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=CR-WHITE-0001; change_request_id=CR-WHITE-0002; change_request_id=CR-WHITE-0003; change_request_id=CR-BLACK-0003; change_request_id=CR-BLACK-0004

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
ARTIFACT_DIR="$ROOT/data/team_ops/review_artifacts"

python3 - "$ARTIFACT_DIR" <<'PY'
import json
import os
import sys
from datetime import datetime

artifact_dir = sys.argv[1]

if not os.path.isdir(artifact_dir):
    print(f"FAIL[CR-WHITE-0002] missing artifact directory: {artifact_dir}", file=sys.stderr)
    sys.exit(1)

files = [
    os.path.join(artifact_dir, f)
    for f in os.listdir(artifact_dir)
    if f.endswith('.json')
]
if not files:
    print("FAIL[CR-WHITE-0002] no review artifact JSON files found", file=sys.stderr)
    sys.exit(1)

required_top = [
    "run_id",
    "artifact_id",
    "mode_label",
    "confidence_label",
    "prompt_version",
    "model_version",
    "source_input_hash",
    "editor_log",
    "approval_timestamp_utc",
    "evidence_caveat_map",
    "provenance_bundle",
    "white_lexicon_version",
    "bounded_claim_class",
    "prohibited_implication_scan",
    "ws_fail_state_checks",
]

mode_values = {"explore", "draft", "approved"}
confidence_values = {"low", "medium", "high"}
claim_classes = {"educational_qualified", "promotional_qualified"}

for path in sorted(files):
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    for field in required_top:
        if field not in data:
            print(f"FAIL[CR-WHITE-0002] {path} missing field: {field}", file=sys.stderr)
            sys.exit(1)

    if data["mode_label"] not in mode_values:
        print(f"FAIL[CR-WHITE-0002] {path} invalid mode_label", file=sys.stderr)
        sys.exit(1)
    if data["confidence_label"] not in confidence_values:
        print(f"FAIL[CR-WHITE-0002] {path} invalid confidence_label", file=sys.stderr)
        sys.exit(1)
    if data["bounded_claim_class"] not in claim_classes:
        print(f"FAIL[CR-BLACK-0004] {path} invalid bounded_claim_class", file=sys.stderr)
        sys.exit(1)

    ws = data["ws_fail_state_checks"]
    for key in ("WS-FS-001", "WS-FS-002", "WS-FS-003"):
        if key not in ws or type(ws[key]) is not bool:
            print(f"FAIL[CR-WHITE-0003] {path} invalid fail-state check key={key}", file=sys.stderr)
            sys.exit(1)

    scan = data["prohibited_implication_scan"]
    hits = scan.get("hits")
    if type(hits) is not int or hits < 0:
        print(f"FAIL[CR-BLACK-0004] {path} invalid prohibited_implication_scan.hits", file=sys.stderr)
        sys.exit(1)

    if not isinstance(data["evidence_caveat_map"], list) or len(data["evidence_caveat_map"]) == 0:
        print(f"FAIL[CR-BLACK-0003] {path} missing evidence_caveat_map entries", file=sys.stderr)
        sys.exit(1)

    if not isinstance(data["editor_log"], list) or len(data["editor_log"]) == 0:
        print(f"FAIL[CR-0019] {path} missing editor_log entries", file=sys.stderr)
        sys.exit(1)

    if len(str(data["source_input_hash"])) < 8:
        print(f"FAIL[CR-0019] {path} source_input_hash too short", file=sys.stderr)
        sys.exit(1)

    if data["mode_label"] == "approved":
        lexical_fail_count = int(data.get("lexical_hard_fail_count", 0))
        if lexical_fail_count != 0:
            print(f"FAIL[CR-WHITE-0001] {path} approved artifact has lexical hard fails", file=sys.stderr)
            sys.exit(1)
        if hits != 0:
            print(f"FAIL[CR-BLACK-0004] {path} approved artifact has prohibited implication hits", file=sys.stderr)
            sys.exit(1)

        try:
            datetime.fromisoformat(data["approval_timestamp_utc"].replace("Z", "+00:00"))
        except ValueError:
            print(f"FAIL[CR-0019] {path} invalid approval_timestamp_utc", file=sys.stderr)
            sys.exit(1)

print(f"PASS[CR-WHITE-0001/0002/0003|CR-BLACK-0003/0004] validated {len(files)} review artifact(s)")
PY
