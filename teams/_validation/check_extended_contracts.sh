#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0016; change_request_id=CR-WHITE-0017

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
ARTIFACT_DIR="$ROOT/data/team_ops/review_artifacts"
POLICY_FILE="$ROOT/planning/HIGH_IMPACT_ACTION_POLICY.md"
MATRIX_FILE="$ROOT/planning/QA_EXECUTION_MATRIX_2026-02-11.md"

python3 - "$ARTIFACT_DIR" "$POLICY_FILE" "$MATRIX_FILE" <<'PY'
import json
import pathlib
import re
import sys

artifact_dir = pathlib.Path(sys.argv[1])
policy_file = pathlib.Path(sys.argv[2])
matrix_file = pathlib.Path(sys.argv[3])

if not artifact_dir.exists():
    print(f"FAIL[CR-WHITE-0004] missing artifact dir: {artifact_dir}", file=sys.stderr)
    sys.exit(1)
if not policy_file.exists():
    print(f"FAIL[CR-BLACK-0005] missing policy file: {policy_file}", file=sys.stderr)
    sys.exit(1)
if not matrix_file.exists():
    print(f"FAIL[CR-WHITE-0014] missing matrix file: {matrix_file}", file=sys.stderr)
    sys.exit(1)

files = sorted(artifact_dir.glob("*.json"))
if not files:
    print("FAIL[CR-WHITE-0004] no review artifact JSON files found", file=sys.stderr)
    sys.exit(1)

required_fields = [
    "source_class", "analytics_path", "publication_lane", "lifecycle_stage",
    "signal_class", "caveat_sentence", "action_scope", "glossary_terms",
    "continuity_templates", "causal_checks", "metric_checks", "fallback_checks",
    "connector_authenticity", "social_rollout", "high_impact_action", "kpi_narratives",
]

allowed_source = {"observed", "scraped_first_party", "simulated", "connector_derived", "mixed"}
allowed_path = {"rust_typed", "script_adapter"}
allowed_lane = {"owned_channel", "external_publication"}
allowed_stage = {"pre_launch", "in_flight", "post_campaign"}
allowed_scope = {"action_blocked", "action_limited", "action_review_only", "approved"}
required_terms = {"WG-001", "WG-002", "WG-003", "WG-004", "WG-005", "WG-006", "WG-007"}
causal_verbs = re.compile(r"\b(caused|proved|guaranteed|definitely drove)\b", re.IGNORECASE)

for path in files:
    data = json.loads(path.read_text(encoding="utf-8"))

    for field in required_fields:
        if field not in data:
            print(f"FAIL[CR-WHITE-0004] {path} missing field: {field}", file=sys.stderr)
            sys.exit(1)

    if data["source_class"] not in allowed_source:
        print(f"FAIL[CR-WHITE-0004] {path} invalid source_class", file=sys.stderr)
        sys.exit(1)
    if data["analytics_path"] not in allowed_path:
        print(f"FAIL[CR-WHITE-0004] {path} invalid analytics_path", file=sys.stderr)
        sys.exit(1)
    if data["publication_lane"] not in allowed_lane:
        print(f"FAIL[CR-WHITE-0007] {path} invalid publication_lane", file=sys.stderr)
        sys.exit(1)
    if data["lifecycle_stage"] not in allowed_stage:
        print(f"FAIL[CR-WHITE-0007] {path} invalid lifecycle_stage", file=sys.stderr)
        sys.exit(1)
    if data["action_scope"] not in allowed_scope:
        print(f"FAIL[CR-WHITE-0012] {path} invalid action_scope", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0005: mixed-source downgrade
    if data["source_class"] == "mixed" and data.get("confidence_label") == "high":
        print(f"FAIL[CR-WHITE-0005] {path} mixed source cannot be high confidence", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0006 glossary enforcement
    present_terms = set(data.get("glossary_terms", []))
    if not required_terms.issubset(present_terms):
        print(f"FAIL[CR-WHITE-0006] {path} missing WG glossary terms", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0008 continuity templates
    for key in ["CT-001", "CT-002", "CT-003", "CT-004"]:
        if data["continuity_templates"].get(key) is not True:
            print(f"FAIL[CR-WHITE-0008] {path} missing/false continuity template {key}", file=sys.stderr)
            sys.exit(1)

    # CR-WHITE-0009 causal checks
    for key in ["CAUS-001", "CAUS-002", "CAUS-003", "CAUS-004"]:
        if data["causal_checks"].get(key) is not True:
            print(f"FAIL[CR-WHITE-0009] {path} missing/false causal check {key}", file=sys.stderr)
            sys.exit(1)

    # CR-WHITE-0010 metric checks
    for key in ["MET-001", "MET-002", "MET-003"]:
        if data["metric_checks"].get(key) is not True:
            print(f"FAIL[CR-WHITE-0010] {path} missing/false metric check {key}", file=sys.stderr)
            sys.exit(1)

    # CR-WHITE-0011 trust-delta requirement
    td = data.get("trust_delta", {})
    if not td.get("action_delta") or not td.get("uncertainty_delta"):
        print(f"FAIL[CR-WHITE-0011] {path} missing trust_delta action/uncertainty deltas", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0012 integration-language fields
    if data["signal_class"] not in {"observed", "scraped_first_party", "simulated", "connector_derived"}:
        print(f"FAIL[CR-WHITE-0012] {path} invalid signal_class", file=sys.stderr)
        sys.exit(1)
    if not data.get("caveat_sentence"):
        print(f"FAIL[CR-WHITE-0012] {path} missing caveat_sentence", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0015 fallback validators
    for key in ["FB-001", "FB-002", "FB-003", "FB-004", "FB-005"]:
        if data["fallback_checks"].get(key) is not True:
            print(f"FAIL[CR-WHITE-0015] {path} missing/false fallback check {key}", file=sys.stderr)
            sys.exit(1)

    # CR-BLACK-0006 authenticity triplet
    auth = data["connector_authenticity"]
    for key in ["source_identity_verified", "freshness_window_ok", "replay_check_pass"]:
        if auth.get(key) is not True:
            print(f"FAIL[CR-BLACK-0006] {path} authenticity triplet failed: {key}", file=sys.stderr)
            sys.exit(1)

    # CR-BLACK-0007 rollout gate
    rollout = data["social_rollout"]
    if rollout.get("tier1_stable_cycles", 0) < 2:
        print(f"FAIL[CR-BLACK-0007] {path} tier1_stable_cycles < 2", file=sys.stderr)
        sys.exit(1)
    if rollout.get("unresolved_provenance_incidents", 1) != 0:
        print(f"FAIL[CR-BLACK-0007] {path} unresolved provenance incidents present", file=sys.stderr)
        sys.exit(1)
    if rollout.get("continuity_note_present") is not True:
        print(f"FAIL[CR-BLACK-0007] {path} continuity note missing", file=sys.stderr)
        sys.exit(1)

    # CR-BLACK-0005 high-impact policy application
    hip = data["high_impact_action"]
    for key in ["threshold_spend_usd", "threshold_reach", "is_high_impact", "blocked_when_hard_fail"]:
        if key not in hip:
            print(f"FAIL[CR-BLACK-0005] {path} missing high_impact_action.{key}", file=sys.stderr)
            sys.exit(1)
    if hip["is_high_impact"] and hip["blocked_when_hard_fail"] is not True:
        print(f"FAIL[CR-BLACK-0005] {path} high-impact action not blocked on hard fail", file=sys.stderr)
        sys.exit(1)

    # CR-WHITE-0016 source-class label per KPI narrative section.
    narratives = data.get("kpi_narratives")
    if not isinstance(narratives, list) or not narratives:
        print(f"FAIL[CR-WHITE-0016] {path} missing/empty kpi_narratives", file=sys.stderr)
        sys.exit(1)
    for narrative in narratives:
        if narrative.get("source_class") not in {"observed", "scraped_first_party", "simulated", "connector_derived"}:
            print(f"FAIL[CR-WHITE-0016] {path} invalid kpi_narrative source_class", file=sys.stderr)
            sys.exit(1)
        if not str(narrative.get("section_id", "")).strip():
            print(f"FAIL[CR-WHITE-0016] {path} missing kpi_narrative section_id", file=sys.stderr)
            sys.exit(1)

    # CR-WHITE-0017 causal phrase class guard fields.
    for narrative in narratives:
        text = str(narrative.get("text", ""))
        guard = narrative.get("causal_guard", {})
        if causal_verbs.search(text):
            for key in ("method", "uncertainty", "counterfactual_note"):
                if not str(guard.get(key, "")).strip():
                    print(f"FAIL[CR-WHITE-0017] {path} causal narrative missing causal_guard.{key}", file=sys.stderr)
                    sys.exit(1)

print(f"PASS[CR-BLACK-0005/0006/0007|CR-WHITE-0004/0005/0006/0007/0008/0009/0010/0011/0012/0014/0015/0016/0017] validated {len(files)} artifact(s)")
PY
