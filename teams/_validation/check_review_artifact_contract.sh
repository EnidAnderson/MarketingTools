#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=CR-WHITE-0001; change_request_id=CR-WHITE-0002; change_request_id=CR-WHITE-0003; change_request_id=CR-WHITE-0004; change_request_id=CR-WHITE-0005; change_request_id=CR-WHITE-0006; change_request_id=CR-WHITE-0007; change_request_id=CR-WHITE-0008; change_request_id=CR-WHITE-0009; change_request_id=CR-WHITE-0010; change_request_id=CR-WHITE-0011; change_request_id=CR-WHITE-0012; change_request_id=CR-WHITE-0015; change_request_id=CR-BLACK-0003; change_request_id=CR-BLACK-0004; change_request_id=CR-BLACK-0005; change_request_id=CR-BLACK-0006; change_request_id=CR-BLACK-0007

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
ARTIFACT_DIR="$ROOT/data/team_ops/review_artifacts"

python3 - "$ARTIFACT_DIR" <<'PY'
import json
import os
import re
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
    "source_class",
    "analytics_path",
    "lifecycle_stage",
    "publication_lane",
    "signal_class",
    "caveat_sentence",
    "action_scope",
    "glossary_terms",
    "continuity_templates",
    "causal_checks",
    "metric_checks",
    "trust_delta",
    "fallback_checks",
    "connector_authenticity",
    "social_rollout",
    "high_impact_action",
]

mode_values = {"explore", "draft", "approved"}
confidence_values = {"low", "medium", "high"}
claim_classes = {"educational_qualified", "promotional_qualified"}
source_classes = {"observed", "scraped_first_party", "simulated", "connector_derived", "mixed"}
analytics_paths = {"typed_rust", "rust_typed", "edge_script_assisted", "script_adapter"}
lifecycle_values = {"pre_launch", "in_flight", "post_campaign"}
publication_lanes = {"educational", "promotional", "owned_channel", "external_publication"}
prohibited_risk_masking = (
    "same guarantees as rust path",
    "equivalent ad-hoc script path",
)
deterministic_causal_verbs = re.compile(r"\b(caused|proved|definitely drove|guaranteed impact)\b", re.IGNORECASE)
metric_absolutism = re.compile(r"\b(always|never|guaranteed|best possible|definitive)\b", re.IGNORECASE)

def fail(code: str, path: str, message: str) -> None:
    print(f"FAIL[{code}] {path} {message}", file=sys.stderr)
    sys.exit(1)

for path in sorted(files):
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    for field in required_top:
        if field not in data:
            fail("CR-WHITE-0002", path, f"missing field: {field}")

    if data["mode_label"] not in mode_values:
        fail("CR-WHITE-0002", path, "invalid mode_label")
    if data["confidence_label"] not in confidence_values:
        fail("CR-WHITE-0002", path, "invalid confidence_label")
    if data["bounded_claim_class"] not in claim_classes:
        fail("CR-BLACK-0004", path, "invalid bounded_claim_class")
    if data["source_class"] not in source_classes:
        fail("CR-WHITE-0004", path, "invalid source_class")
    if data["analytics_path"] not in analytics_paths:
        fail("CR-WHITE-0004", path, "invalid analytics_path")
    if data["lifecycle_stage"] not in lifecycle_values:
        fail("CR-WHITE-0007", path, "invalid lifecycle_stage")
    if data["publication_lane"] not in publication_lanes:
        fail("CR-WHITE-0007", path, "invalid publication_lane")

    ws = data["ws_fail_state_checks"]
    for key in ("WS-FS-001", "WS-FS-002", "WS-FS-003"):
        if key not in ws or type(ws[key]) is not bool:
            fail("CR-WHITE-0003", path, f"invalid fail-state check key={key}")

    scan = data["prohibited_implication_scan"]
    hits = scan.get("hits")
    if type(hits) is not int or hits < 0:
        fail("CR-BLACK-0004", path, "invalid prohibited_implication_scan.hits")

    if not isinstance(data["evidence_caveat_map"], list) or len(data["evidence_caveat_map"]) == 0:
        fail("CR-BLACK-0003", path, "missing evidence_caveat_map entries")

    if not isinstance(data["editor_log"], list) or len(data["editor_log"]) == 0:
        fail("CR-0019", path, "missing editor_log entries")

    if len(str(data["source_input_hash"])) < 8:
        fail("CR-0019", path, "source_input_hash too short")

    # CR-WHITE-0004/0005: terminology + mixed-source downgrade.
    classes = [data.get("source_class"), data.get("signal_class")]
    for cls in classes:
        if cls not in source_classes:
            fail("CR-WHITE-0004", path, f"unknown source class value '{cls}'")
    if len(set(classes)) > 1:
        caveat = str(data.get("caveat_sentence", "")).lower()
        if "uncertainty" not in caveat and "advisory" not in caveat:
            fail("CR-WHITE-0005", path, "mixed-source caveat sentence must carry uncertainty/advisory text")
        if data["confidence_label"] == "high":
            fail("CR-WHITE-0005", path, "mixed-source artifact cannot be high confidence")
    risk_mask_text = json.dumps(data, ensure_ascii=False).lower()
    for phrase in prohibited_risk_masking:
        if phrase in risk_mask_text:
            fail("CR-WHITE-0004", path, f"disallowed wording detected: '{phrase}'")

    # CR-WHITE-0006: canonical glossary IDs.
    glossary = data.get("glossary_terms")
    if not isinstance(glossary, list):
        fail("CR-WHITE-0006", path, "glossary_terms must be list")
    for key in ("WG-001", "WG-002", "WG-003", "WG-004", "WG-005", "WG-006", "WG-007"):
        if key not in glossary:
            fail("CR-WHITE-0006", path, f"missing glossary term: {key}")

    # CR-WHITE-0007: lifecycle confidence policy.
    if data["lifecycle_stage"] == "pre_launch" and data["mode_label"] == "approved":
        fail("CR-WHITE-0007", path, "pre_launch artifacts cannot be approved")
    if data["lifecycle_stage"] == "post_campaign" and "simulated" in set(classes) and data["mode_label"] == "approved":
        fail("CR-WHITE-0007", path, "post_campaign simulated artifacts cannot be approved")

    # CR-WHITE-0008: continuity templates CT-001..CT-004 token presence.
    ct = data.get("continuity_templates")
    if not isinstance(ct, dict):
        fail("CR-WHITE-0008", path, "continuity_templates must be object")
    for key in ("CT-001", "CT-002", "CT-003", "CT-004"):
        if ct.get(key) is not True:
            fail("CR-WHITE-0008", path, f"{key} must be true")

    # CR-WHITE-0009: causal overstatement guard.
    causal = data.get("causal_checks")
    if not isinstance(causal, dict):
        fail("CR-WHITE-0009", path, "causal_checks must be object")
    for key in ("CAUS-001", "CAUS-002", "CAUS-003", "CAUS-004"):
        if causal.get(key) is not True:
            fail("CR-WHITE-0009", path, f"{key} must be true")
    if deterministic_causal_verbs.search(json.dumps(data.get("evidence_caveat_map", []), ensure_ascii=False)):
        fail("CR-WHITE-0009", path, "deterministic causal verb found in evidence/caveat map")

    # CR-WHITE-0010: metric anti-overstatement.
    metrics = data.get("metric_checks")
    if not isinstance(metrics, dict):
        fail("CR-WHITE-0010", path, "metric_checks must be object")
    for key in ("MET-001", "MET-002", "MET-003"):
        if metrics.get(key) is not True:
            fail("CR-WHITE-0010", path, f"{key} must be true")
    if metric_absolutism.search(json.dumps(data.get("evidence_caveat_map", []), ensure_ascii=False)):
        fail("CR-WHITE-0010", path, "absolutist metric wording found in evidence/caveat map")

    # CR-WHITE-0011: trust-delta prompt.
    trust = data.get("trust_delta")
    if not isinstance(trust, dict):
        fail("CR-WHITE-0011", path, "trust_delta must be object")
    if not str(trust.get("action_delta", "")).strip():
        fail("CR-WHITE-0011", path, "missing trust_delta.action_delta")
    if not str(trust.get("uncertainty_delta", "")).strip():
        fail("CR-WHITE-0011", path, "missing trust_delta.uncertainty_delta")
    if trust.get("from") != trust.get("to") and not str(trust.get("reason", "")).strip():
        fail("CR-WHITE-0011", path, "confidence transition must include reason")

    # CR-WHITE-0012: integration language fields.
    for key in ("signal_class", "confidence_label", "caveat_sentence", "action_scope"):
        if not str(data.get(key, "")).strip():
            fail("CR-WHITE-0012", path, f"missing {key}")
    if data.get("signal_class") not in source_classes:
        fail("CR-WHITE-0012", path, "invalid signal_class")

    # CR-WHITE-0015: fallback-state templates + health caveat.
    fb = data.get("fallback_checks")
    if not isinstance(fb, dict):
        fail("CR-WHITE-0015", path, "fallback_checks must be object")
    for key in ("FB-001", "FB-002", "FB-003", "FB-004", "FB-005"):
        if fb.get(key) is not True:
            fail("CR-WHITE-0015", path, f"missing {key}")

    # CR-BLACK-0005/0006/0007: high-impact, authenticity, rollout gates.
    impact_policy = data.get("high_impact_action")
    if not isinstance(impact_policy, dict):
        fail("CR-BLACK-0005", path, "high_impact_action must be object")
    for key in ("threshold_spend_usd", "threshold_reach", "is_high_impact", "blocked_when_hard_fail"):
        if key not in impact_policy:
            fail("CR-BLACK-0005", path, f"missing high_impact_action.{key}")
    auth = data.get("connector_authenticity")
    if not isinstance(auth, dict):
        fail("CR-BLACK-0006", path, "connector_authenticity must be object")
    for key in ("source_identity_verified", "freshness_window_ok", "replay_check_pass"):
        if type(auth.get(key)) is not bool:
            fail("CR-BLACK-0006", path, f"connector_authenticity.{key} must be boolean")
    if data.get("mode_label") == "approved" and not all(auth.get(k) for k in ("source_identity_verified", "freshness_window_ok", "replay_check_pass")):
        fail("CR-BLACK-0006", path, "approved artifact requires passing authenticity triplet")

    rollout = data.get("social_rollout")
    if not isinstance(rollout, dict):
        fail("CR-BLACK-0007", path, "social_rollout must be object")
    if int(rollout.get("tier1_stable_cycles", 0)) < 2:
        fail("CR-BLACK-0007", path, "tier1_stable_cycles must be >= 2")
    if int(rollout.get("unresolved_provenance_incidents", 0)) != 0:
        fail("CR-BLACK-0007", path, "unresolved_provenance_incidents must be 0")
    if rollout.get("continuity_note_present") is not True:
        fail("CR-BLACK-0007", path, "continuity_note_present must be true")

    if data["mode_label"] == "approved":
        lexical_fail_count = int(data.get("lexical_hard_fail_count", 0))
        if lexical_fail_count != 0:
            fail("CR-WHITE-0001", path, "approved artifact has lexical hard fails")
        if hits != 0:
            fail("CR-BLACK-0004", path, "approved artifact has prohibited implication hits")
        if impact_policy.get("is_high_impact") and (not all(auth.get(k) for k in ("source_identity_verified", "freshness_window_ok", "replay_check_pass"))):
            fail("CR-BLACK-0005", path, "high_impact_action must be blocked when contamination/authenticity hard failures exist")

        try:
            datetime.fromisoformat(data["approval_timestamp_utc"].replace("Z", "+00:00"))
        except ValueError:
            fail("CR-0019", path, "invalid approval_timestamp_utc")

print(
    "PASS[CR-WHITE-0001/0002/0003/0004/0005/0006/0007/0008/0009/0010/0011/0012/0015|"
    "CR-BLACK-0003/0004/0005/0006/0007] "
    f"validated {len(files)} review artifact(s)"
)
PY
