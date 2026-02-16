#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-032

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
BASE_REF="${1:-HEAD}"
REPORT="$ROOT/teams/_validation/validation_report.json"
mkdir -p "$ROOT/teams/_validation"

python3 - "$ROOT" "$BASE_REF" "$REPORT" <<'PY'
import json
import subprocess
import sys
from datetime import datetime, timezone

root, base_ref, report = sys.argv[1], sys.argv[2], sys.argv[3]
checks = [
    {
        "id": "RQ-029",
        "name": "check_pipeline_order",
        "rule_ref": "teams/shared/OPERATING_DOCTRINE.md",
        "cmd": [f"{root}/teams/_validation/check_pipeline_order.sh"],
    },
    {
        "id": "RQ-030",
        "name": "check_append_only",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_VALIDATION_2026-02-10.md#RQ-030",
        "cmd": [f"{root}/teams/_validation/check_append_only.sh", base_ref],
    },
    {
        "id": "RQ-031",
        "name": "check_qa_edit_authority",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_VALIDATION_2026-02-10.md#RQ-031",
        "cmd": [f"{root}/teams/_validation/check_qa_edit_authority.sh", base_ref],
    },
    {
        "id": "RQ-034",
        "name": "check_request_id_policy",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_VALIDATION_2026-02-10.md#RQ-034",
        "cmd": [f"{root}/teams/_validation/check_request_id_policy.sh", base_ref],
    },
    {
        "id": "RQ-MGR-001",
        "name": "check_request_id_global_uniqueness",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_MICROMANAGER_2026-02-10.md#RQ-MGR-001",
        "cmd": [f"{root}/teams/_validation/check_request_id_global_uniqueness.sh"],
    },
    {
        "id": "RQ-MGR-003",
        "name": "check_handoff_run_state_sync",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_MICROMANAGER_2026-02-10.md#RQ-MGR-003",
        "cmd": [f"{root}/teams/_validation/check_handoff_run_state_sync.sh", base_ref],
    },
    {
        "id": "RQ-MGR-004",
        "name": "check_stage_output_format",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_MICROMANAGER_2026-02-10.md#RQ-MGR-004",
        "cmd": [f"{root}/teams/_validation/check_stage_output_format.sh"],
    },
    {
        "id": "RQ-MGR-005",
        "name": "generate_cycle_health_summary",
        "rule_ref": "planning/reports/TEAM_LEAD_REQUEST_QUEUE_MICROMANAGER_2026-02-10.md#RQ-MGR-005",
        "cmd": [f"{root}/teams/_validation/generate_cycle_health_summary.sh"],
    },
    {
        "id": "CR-BLACK-0001/0002",
        "name": "check_budget_and_release_gates",
        "rule_ref": "planning/RELEASE_GATES_POLICY.md",
        "cmd": [f"{root}/teams/_validation/check_budget_and_release_gates.sh"],
    },
    {
        "id": "CR-WHITE-0001..0012/0015 + CR-BLACK-0003..0007",
        "name": "check_review_artifact_contract",
        "rule_ref": "planning/REVIEW_METADATA_CONTRACT_v1.md",
        "cmd": [f"{root}/teams/_validation/check_review_artifact_contract.sh"],
    },
    {
        "id": "CR-BLACK-0005..0015|CR-WHITE-0004..0012/0014/0015/0016/0017/0022/0023/0024/0025/0026|CR-GREEN-0019/0020/0024|CR-RED-0007/0008|CR-GREY-0008/0009/0010/0011",
        "name": "check_extended_contracts",
        "rule_ref": "planning/QA_EXECUTION_MATRIX_2026-02-11.md",
        "cmd": [f"{root}/teams/_validation/check_extended_contracts.sh"],
    },
]

results = []
failed = False
for c in checks:
    proc = subprocess.run(c["cmd"], capture_output=True, text=True)
    diag = (proc.stdout + proc.stderr).strip().splitlines()
    results.append(
        {
            "check_id": c["id"],
            "check_name": c["name"],
            "rule_ref": c["rule_ref"],
            "status": "pass" if proc.returncode == 0 else "fail",
            "exit_code": proc.returncode,
            "diagnostic": diag[-1] if diag else "",
        }
    )
    if proc.returncode != 0:
        failed = True

payload = {
    "generated_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "base_ref": base_ref,
    "overall_status": "fail" if failed else "pass",
    "checks": results,
}
with open(report, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2)
print(f"Wrote {report}")
if failed:
    sys.exit(32)
PY
