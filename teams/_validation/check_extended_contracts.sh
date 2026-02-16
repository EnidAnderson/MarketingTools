#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0016; change_request_id=CR-WHITE-0017; change_request_id=CR-BLACK-0008; change_request_id=CR-BLACK-0009; change_request_id=CR-BLACK-0010; change_request_id=CR-BLACK-0011; change_request_id=CR-BLACK-0012; change_request_id=CR-BLACK-0013; change_request_id=CR-BLACK-0014; change_request_id=CR-BLACK-0015; change_request_id=CR-GREEN-0019; change_request_id=CR-GREEN-0020; change_request_id=CR-GREEN-0024; change_request_id=CR-GREEN-0028; change_request_id=CR-GREEN-0029; change_request_id=CR-RED-0007; change_request_id=CR-RED-0008; change_request_id=CR-WHITE-0022; change_request_id=CR-WHITE-0023; change_request_id=CR-WHITE-0024; change_request_id=CR-WHITE-0025; change_request_id=CR-WHITE-0026; change_request_id=CR-WHITE-0027; change_request_id=CR-GREY-0008; change_request_id=CR-GREY-0009; change_request_id=CR-GREY-0010; change_request_id=CR-GREY-0011

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
    "freshness_sla", "schema_drift", "identity_resolution", "source_separation",
    "attribution_integrity", "publish_gate", "runtime_guardrails", "retry_budget",
    "what_changed_why", "review_annotations", "report_annotation_templates",
    "denominator_policy", "attribution_window_declaration", "confidence_scope_alignment",
    "identity_alerts", "freshness_alerts", "failsafe_rollback", "narrative_classifier",
    "escalation_handoff", "escalation_telemetry",
]

allowed_source = {"observed", "scraped_first_party", "simulated", "connector_derived", "mixed"}
allowed_path = {"rust_typed", "script_adapter"}
allowed_lane = {"owned_channel", "external_publication"}
allowed_stage = {"pre_launch", "in_flight", "post_campaign"}
allowed_scope = {"action_blocked", "action_limited", "action_review_only", "approved"}
required_terms = {"WG-001", "WG-002", "WG-003", "WG-004", "WG-005", "WG-006", "WG-007"}
causal_verbs = re.compile(r"\b(caused|proved|guaranteed|definitely drove)\b", re.IGNORECASE)
external_approval = re.compile(r"\b(approved for external|ready for publication|externally approved)\b", re.IGNORECASE)
reason_taxonomy = {"schema_update", "attribution_reconciliation", "freshness_repair", "identity_resolution", "copy_hardening"}
constrained_external_claim_phrases = (
    "proven uplift",
    "confirmed roi",
    "winning campaign",
    "publish-ready result",
    "validated externally",
    "market-facing claim",
    "breakout performance",
    "confirmed winner",
    "ready to scale",
    "campaign proved",
    "public-ready",
)

def fail(code: str, path: pathlib.Path, msg: str) -> None:
    print(f"FAIL[{code}] {path} {msg}", file=sys.stderr)
    sys.exit(1)

for path in files:
    data = json.loads(path.read_text(encoding="utf-8"))

    for field in required_fields:
        if field not in data:
            fail("CR-WHITE-0004", path, f"missing field: {field}")

    if data["source_class"] not in allowed_source:
        fail("CR-WHITE-0004", path, "invalid source_class")
    if data["analytics_path"] not in allowed_path:
        fail("CR-WHITE-0004", path, "invalid analytics_path")
    if data["publication_lane"] not in allowed_lane:
        fail("CR-WHITE-0007", path, "invalid publication_lane")
    if data["lifecycle_stage"] not in allowed_stage:
        fail("CR-WHITE-0007", path, "invalid lifecycle_stage")
    if data["action_scope"] not in allowed_scope:
        fail("CR-WHITE-0012", path, "invalid action_scope")

    # CR-WHITE-0005 mixed-source downgrade.
    if data["source_class"] == "mixed" and data.get("confidence_label") == "high":
        fail("CR-WHITE-0005", path, "mixed source cannot be high confidence")

    # CR-WHITE-0006 glossary enforcement.
    present_terms = set(data.get("glossary_terms", []))
    if not required_terms.issubset(present_terms):
        fail("CR-WHITE-0006", path, "missing WG glossary terms")

    # CR-WHITE-0008 continuity templates.
    for key in ["CT-001", "CT-002", "CT-003", "CT-004"]:
        if data["continuity_templates"].get(key) is not True:
            fail("CR-WHITE-0008", path, f"missing/false continuity template {key}")

    # CR-WHITE-0009 causal checks.
    for key in ["CAUS-001", "CAUS-002", "CAUS-003", "CAUS-004"]:
        if data["causal_checks"].get(key) is not True:
            fail("CR-WHITE-0009", path, f"missing/false causal check {key}")

    # CR-WHITE-0010 metric checks.
    for key in ["MET-001", "MET-002", "MET-003"]:
        if data["metric_checks"].get(key) is not True:
            fail("CR-WHITE-0010", path, f"missing/false metric check {key}")

    # CR-WHITE-0011 trust-delta requirement.
    td = data.get("trust_delta", {})
    if not td.get("action_delta") or not td.get("uncertainty_delta"):
        fail("CR-WHITE-0011", path, "missing trust_delta action/uncertainty deltas")

    # CR-WHITE-0012 integration-language fields.
    if data["signal_class"] not in {"observed", "scraped_first_party", "simulated", "connector_derived"}:
        fail("CR-WHITE-0012", path, "invalid signal_class")
    if not data.get("caveat_sentence"):
        fail("CR-WHITE-0012", path, "missing caveat_sentence")

    # CR-WHITE-0015 fallback validators.
    for key in ["FB-001", "FB-002", "FB-003", "FB-004", "FB-005"]:
        if data["fallback_checks"].get(key) is not True:
            fail("CR-WHITE-0015", path, f"missing/false fallback check {key}")

    # CR-WHITE-0016 source-class labels in KPI narratives.
    narratives = data.get("kpi_narratives")
    if not isinstance(narratives, list) or not narratives:
        fail("CR-WHITE-0016", path, "missing/empty kpi_narratives")
    for n in narratives:
        if n.get("source_class") not in {"observed", "scraped_first_party", "simulated", "connector_derived"}:
            fail("CR-WHITE-0016", path, "invalid kpi_narrative source_class")
        if not str(n.get("section_id", "")).strip():
            fail("CR-WHITE-0016", path, "missing kpi_narrative section_id")

    # CR-WHITE-0017 causal phrase guard.
    for n in narratives:
        text = str(n.get("text", ""))
        guard = n.get("causal_guard", {})
        if causal_verbs.search(text):
            for key in ("method", "uncertainty", "counterfactual_note"):
                if not str(guard.get(key, "")).strip():
                    fail("CR-WHITE-0017", path, f"causal narrative missing causal_guard.{key}")

    # CR-WHITE-0022/CR-GREEN-0019 what_changed_why taxonomy.
    wcw = data.get("what_changed_why", {})
    for key in ("summary", "reason_code", "fields_changed"):
        if not wcw.get(key):
            fail("CR-WHITE-0022", path, f"what_changed_why missing {key}")
    if wcw.get("reason_code") not in reason_taxonomy:
        fail("CR-WHITE-0022", path, "invalid reason_code taxonomy")

    # CR-WHITE-0023 lifecycle template caveats.
    lt = data.get("report_annotation_templates", {})
    for key in ("pre_launch", "in_flight", "post_campaign"):
        txt = str(lt.get(key, "")).lower()
        if "uncertainty" not in txt or "freshness" not in txt:
            fail("CR-WHITE-0023", path, f"lifecycle template '{key}' missing uncertainty/freshness caveat")

    # CR-WHITE-0024 annotation semantics AN-001..AN-005.
    ann = data.get("review_annotations", {})
    for key in ("AN-001", "AN-002", "AN-003", "AN-004", "AN-005"):
        if ann.get(key) is not True:
            fail("CR-WHITE-0024", path, f"missing annotation semantic flag {key}")

    # CR-WHITE-0025 KPI annotation scope.
    kp = data.get("kpi_annotation_scope", {})
    for key in ("KP-ANN-001", "KP-ANN-002", "KP-ANN-003"):
        if kp.get(key) is not True:
            fail("CR-WHITE-0025", path, f"missing KPI annotation scope flag {key}")
    if kp.get("PH-CAUS-001") is not True:
        fail("CR-WHITE-0025", path, "missing PH-CAUS-001 threshold-aware causal phrase guard")

    # CR-WHITE-0026 constrained-state classifier.
    classifier = data.get("narrative_classifier", {})
    if classifier.get("NS-STATE-001") is not True:
        fail("CR-WHITE-0026", path, "missing NS-STATE-001 classifier")
    if data["action_scope"] in {"action_blocked", "action_limited", "action_review_only"}:
        for n in narratives:
            if external_approval.search(str(n.get("text", ""))):
                fail("CR-WHITE-0026", path, "constrained state narrative includes external-approval phrase")

    # CR-WHITE-0027 escalation handoff language contract (ESC-HL-001..006).
    esc = data.get("escalation_handoff", {})
    esc_flags = esc.get("language_contract_flags", {})
    for key in ("ESC-HL-001", "ESC-HL-002", "ESC-HL-003", "ESC-HL-004", "ESC-HL-005", "ESC-HL-006"):
        if esc_flags.get(key) is not True:
            fail("CR-WHITE-0027", path, f"missing language contract flag {key}")
    required_lines = esc.get("required_lines", {})
    for key in (
        "headline",
        "threshold_declaration",
        "state_and_scope",
        "evidence_and_confidence",
        "external_claim_prohibition",
        "owner_next_action",
    ):
        if not str(required_lines.get(key, "")).strip():
            fail("CR-WHITE-0027", path, f"missing required escalation handoff line: {key}")
    handoff_text = str(esc.get("handoff_text", "")).lower()
    if data["action_scope"] in {"action_blocked", "action_limited", "action_review_only"}:
        for phrase in constrained_external_claim_phrases:
            if phrase in handoff_text:
                fail("CR-WHITE-0027", path, f"constrained-state handoff contains prohibited phrase: {phrase}")
        if "internal-only" not in handoff_text:
            fail("CR-WHITE-0027", path, "constrained-state handoff must include internal-only caveat")

    # CR-BLACK-0006 authenticity triplet.
    auth = data["connector_authenticity"]
    for key in ["source_identity_verified", "freshness_window_ok", "replay_check_pass"]:
        if auth.get(key) is not True:
            fail("CR-BLACK-0006", path, f"authenticity triplet failed: {key}")

    # CR-BLACK-0007 rollout gate.
    rollout = data["social_rollout"]
    if rollout.get("tier1_stable_cycles", 0) < 2:
        fail("CR-BLACK-0007", path, "tier1_stable_cycles < 2")
    if rollout.get("unresolved_provenance_incidents", 1) != 0:
        fail("CR-BLACK-0007", path, "unresolved provenance incidents present")
    if rollout.get("continuity_note_present") is not True:
        fail("CR-BLACK-0007", path, "continuity note missing")

    # CR-BLACK-0005 high-impact policy application.
    hip = data["high_impact_action"]
    for key in ["threshold_spend_usd", "threshold_reach", "is_high_impact", "blocked_when_hard_fail"]:
        if key not in hip:
            fail("CR-BLACK-0005", path, f"missing high_impact_action.{key}")
    if hip["is_high_impact"] and hip["blocked_when_hard_fail"] is not True:
        fail("CR-BLACK-0005", path, "high-impact action not blocked on hard fail")

    # CR-BLACK-0008 freshness SLA by source class.
    sla = data.get("freshness_sla", {})
    expected_limits = {
        "observed": 60,
        "connector_derived": 120,
        "scraped_first_party": 360,
        "simulated": 1440,
    }
    for cls, limit in expected_limits.items():
        if int(sla.get(cls, -1)) <= 0:
            fail("CR-BLACK-0008", path, f"missing freshness SLA for {cls}")
        if int(sla.get(cls, 0)) > limit:
            fail("CR-BLACK-0008", path, f"freshness SLA too lax for {cls}")

    # CR-BLACK-0009 schema drift fail-closed + quarantine.
    drift = data.get("schema_drift", {})
    if drift.get("fail_closed") is not True or drift.get("quarantine_on_mismatch") is not True:
        fail("CR-BLACK-0009", path, "schema drift policy must be fail-closed with quarantine")

    # CR-BLACK-0010 identity-confidence gate.
    ident = data.get("identity_resolution", {})
    if float(ident.get("confidence_floor", 0.0)) < 0.8:
        fail("CR-BLACK-0010", path, "identity confidence floor too low")
    if float(ident.get("duplicate_rate_cap", 1.0)) > 0.05:
        fail("CR-BLACK-0010", path, "duplicate_rate_cap too high")

    # CR-BLACK-0011 source separation and caveat map completeness.
    sep = data.get("source_separation", {})
    if sep.get("strict_partitioning") is not True:
        fail("CR-BLACK-0011", path, "strict source partitioning required")
    for n in narratives:
        sid = n.get("section_id")
        mapped = any(m.get("claim_id") == sid for m in data.get("evidence_caveat_map", []))
        if not mapped:
            fail("CR-BLACK-0011", path, f"missing caveat map entry for narrative section {sid}")

    # CR-BLACK-0012 attribution integrity.
    attrib = data.get("attribution_integrity", {})
    if int(attrib.get("window_delta_days_cap", 999)) > 2:
        fail("CR-BLACK-0012", path, "attribution window delta cap too high")
    if attrib.get("assumption_presence_required") is not True:
        fail("CR-BLACK-0012", path, "assumption presence must be required")
    if attrib.get("confidence_bound_causal_language") is not True:
        fail("CR-BLACK-0012", path, "confidence-bound causal language must be enforced")

    # CR-BLACK-0013 publish-gate completeness.
    pg = data.get("publish_gate", {})
    for key in ("provenance_complete", "freshness_within_sla", "caveat_complete", "prohibited_implication_scan_pass"):
        if pg.get(key) is not True:
            fail("CR-BLACK-0013", path, f"publish gate missing pass condition: {key}")

    # CR-BLACK-0014 runtime/cost guardrails.
    rg = data.get("runtime_guardrails", {})
    warn_pct = float(rg.get("warn_pct", 0))
    block_pct = float(rg.get("block_pct", 0))
    if warn_pct != 80.0 or block_pct != 100.0:
        fail("CR-BLACK-0014", path, "runtime guardrail thresholds must be 80 warn / 100 block")

    # CR-BLACK-0015 retry budget non-bypass.
    rb = data.get("retry_budget", {})
    if int(rb.get("max_retries", 99)) > 2:
        fail("CR-BLACK-0015", path, "max_retries exceeds 2")
    if rb.get("fallback_scope") != "action_review_only":
        fail("CR-BLACK-0015", path, "fallback scope must be action_review_only")
    if rb.get("non_bypass_enforced") is not True:
        fail("CR-BLACK-0015", path, "retry non-bypass must be enforced")

    # CR-GREEN-0020 constrained-state comm caveats.
    if data["action_scope"] in {"action_blocked", "action_limited", "action_review_only"}:
        if "caveat" not in str(data.get("caveat_sentence", "")).lower():
            fail("CR-GREEN-0020", path, "constrained state must include caveat wording")

    # CR-GREEN-0024 annotation scaffold/state-bound template usage.
    scaffold = data.get("annotation_scaffold", {})
    for key in ("missing_data", "delayed_conversion", "partial_ingestion", "state_bound_template"):
        if not str(scaffold.get(key, "")).strip():
            fail("CR-GREEN-0024", path, f"annotation scaffold missing {key}")

    # CR-GREEN-0028 high-impact escalation prompts + state/action coherence.
    esc = data.get("escalation_handoff", {})
    for key in (
        "threshold_type",
        "breach_magnitude",
        "persistence_windows",
        "confidence_trend",
        "source_classes",
        "allowed_scope",
        "state_action_scope_coherent",
        "external_claim_status",
        "required_prompts",
    ):
        if key not in esc:
            fail("CR-GREEN-0028", path, f"escalation_handoff missing {key}")
    if str(esc.get("threshold_type", "")).lower() not in {"spend", "reach", "both"}:
        fail("CR-GREEN-0028", path, "threshold_type must be spend|reach|both")
    if int(esc.get("persistence_windows", 0)) < 1:
        fail("CR-GREEN-0028", path, "persistence_windows must be >= 1")
    if str(esc.get("confidence_trend", "")).lower() not in {"up", "flat", "down"}:
        fail("CR-GREEN-0028", path, "confidence_trend must be up|flat|down")
    if esc.get("state_action_scope_coherent") is not True:
        fail("CR-GREEN-0028", path, "state/action-scope coherence must be true")
    req_prompts = esc.get("required_prompts", {})
    for key in (
        "thresholds_breached_prompt",
        "what_changed_confidence_prompt",
        "source_classes_prompt",
        "action_scope_prompt",
        "external_claims_not_allowed_prompt",
    ):
        if not str(req_prompts.get(key, "")).strip():
            fail("CR-GREEN-0028", path, f"required escalation prompt missing {key}")
    if data["action_scope"] in {"action_blocked", "action_limited", "action_review_only"}:
        if esc.get("allowed_scope") != data["action_scope"]:
            fail("CR-GREEN-0028", path, "allowed_scope must match constrained action_scope")
        if str(esc.get("external_claim_status", "")).lower() != "prohibited":
            fail("CR-GREEN-0028", path, "external_claim_status must be prohibited in constrained states")

    # CR-GREEN-0029 escalation telemetry quality checks.
    telem = data.get("escalation_telemetry", {})
    for key in (
        "missing_prompt_rate",
        "unresolved_threshold_persistence_windows",
        "confidence_delta_ack_coverage",
    ):
        if key not in telem:
            fail("CR-GREEN-0029", path, f"escalation_telemetry missing {key}")
    if float(telem.get("missing_prompt_rate", 1.0)) > 0.05:
        fail("CR-GREEN-0029", path, "missing_prompt_rate exceeds 5%")
    if int(telem.get("unresolved_threshold_persistence_windows", 0)) < 0:
        fail("CR-GREEN-0029", path, "unresolved_threshold_persistence_windows must be >= 0")
    if float(telem.get("confidence_delta_ack_coverage", 0.0)) < 0.95:
        fail("CR-GREEN-0029", path, "confidence_delta_ack_coverage must be >= 95%")

    # CR-RED-0007 declaration alignment.
    if not str(data.get("denominator_policy", "")).strip():
        fail("CR-RED-0007", path, "missing denominator_policy")
    if not str(data.get("attribution_window_declaration", "")).strip():
        fail("CR-RED-0007", path, "missing attribution_window_declaration")
    csa = data.get("confidence_scope_alignment", {})
    if csa.get("aligned") is not True:
        fail("CR-RED-0007", path, "confidence label scope is not aligned")

    # CR-RED-0008 alert checks.
    ia = data.get("identity_alerts", {})
    fa = data.get("freshness_alerts", {})
    if ia.get("degradation_pre_publish_block") is not True:
        fail("CR-RED-0008", path, "identity degradation must block pre-publish")
    if fa.get("skew_threshold_breach_block") is not True:
        fail("CR-RED-0008", path, "freshness skew threshold breach must block pre-publish")

    # CR-GREY-0008 fail-closed + advisory-only routing.
    if drift.get("fail_closed") is not True:
        fail("CR-GREY-0008", path, "fail-closed drift requirement missing")
    if data.get("routing_on_failure", {}).get("advisory_only") is not True:
        fail("CR-GREY-0008", path, "failure routing must be advisory_only")

    # CR-GREY-0009 attribution/confidence downgrade controls.
    downgrade = data.get("confidence_downgrade_controls", {})
    if downgrade.get("evidence_sufficiency_required") is not True:
        fail("CR-GREY-0009", path, "evidence sufficiency required for confidence level")
    if downgrade.get("identity_quality_required") is not True:
        fail("CR-GREY-0009", path, "identity quality required for confidence level")

    # CR-GREY-0010 decision-grade surfaces with source separation + lineage.
    lineage = data.get("lineage", {})
    for key in ("inputs", "spec_ref", "run_metadata", "decision_ref"):
        if not lineage.get(key):
            fail("CR-GREY-0010", path, f"lineage missing {key}")

    # CR-GREY-0011 fail-safe rollback continuity.
    rollback = data.get("failsafe_rollback", {})
    if rollback.get("last_known_good_enabled") is not True:
        fail("CR-GREY-0011", path, "last-known-good continuity must be enabled")
    if not str(rollback.get("rollback_reason_code", "")).strip():
        fail("CR-GREY-0011", path, "rollback_reason_code is required")

print(
    "PASS[CR-BLACK-0005/0006/0007/0008/0009/0010/0011/0012/0013/0014/0015|"
    "CR-WHITE-0004/0005/0006/0007/0008/0009/0010/0011/0012/0014/0015/0016/0017/0022/0023/0024/0025/0026/0027|"
    "CR-GREEN-0019/0020/0024/0028/0029|CR-RED-0007/0008|CR-GREY-0008/0009/0010/0011] "
    f"validated {len(files)} artifact(s)"
)
PY
