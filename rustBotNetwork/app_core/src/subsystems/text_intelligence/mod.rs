use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const TEXT_ARTIFACT_SCHEMA_VERSION_V1: &str = "text_workflow_artifact.v1";
const CRITICAL_FINDING_CODES: [&str; 4] = [
    "unsupported_high_risk_claim",
    "policy_violation",
    "missing_required_section",
    "internal_inconsistency",
];

/// # NDOC
/// component: `subsystems::text_intelligence`
/// purpose: Supported high-complexity text workflow families.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextWorkflowKindV1 {
    IntegratedLaunchCampaignKit,
    PersonaPositioningMessageHouse,
    AdVariantPackExperimentPlan,
    EmailLandingSequence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CritiqueSeverityV1 {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRefV1 {
    pub evidence_id: String,
    pub source_ref: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofPointV1 {
    pub claim_id: String,
    pub claim_text: String,
    #[serde(default)]
    pub evidence_ref_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagePillarV1 {
    pub pillar_id: String,
    pub title: String,
    pub supporting_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageHouseV1 {
    pub big_idea: String,
    pub pillars: Vec<MessagePillarV1>,
    pub proof_points: Vec<ProofPointV1>,
    #[serde(default)]
    pub do_not_say: Vec<String>,
    #[serde(default)]
    pub tone_guide: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CampaignSpineV1 {
    pub campaign_spine_id: String,
    pub product_name: String,
    pub offer_summary: String,
    pub audience_segments: Vec<String>,
    pub positioning_statement: String,
    pub message_house: MessageHouseV1,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRefV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextSectionV1 {
    pub section_id: String,
    pub section_title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CritiqueFindingV1 {
    pub code: String,
    pub severity: CritiqueSeverityV1,
    pub message: String,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub evidence_ref_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextQualityScorecardV1 {
    pub instruction_coverage: f64,
    pub audience_alignment: f64,
    pub claims_risk: f64,
    pub brand_voice_consistency: f64,
    pub novelty: f64,
    pub revision_gain: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextGateDecisionV1 {
    pub blocked: bool,
    pub blocking_reasons: Vec<String>,
    pub warning_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextWorkflowArtifactV1 {
    pub schema_version: String,
    pub workflow_kind: TextWorkflowKindV1,
    pub campaign_spine_id: String,
    pub sections: Vec<TextSectionV1>,
    pub critique_findings: Vec<CritiqueFindingV1>,
    pub quality: TextQualityScorecardV1,
    pub gate_decision: TextGateDecisionV1,
}

/// # NDOC
/// component: `subsystems::text_intelligence`
/// purpose: Validate scorecard bounds prior to gate evaluation.
pub fn validate_scorecard_v1(scorecard: &TextQualityScorecardV1) -> Result<(), String> {
    let fields = [
        ("instruction_coverage", scorecard.instruction_coverage),
        ("audience_alignment", scorecard.audience_alignment),
        ("claims_risk", scorecard.claims_risk),
        ("brand_voice_consistency", scorecard.brand_voice_consistency),
        ("novelty", scorecard.novelty),
        ("revision_gain", scorecard.revision_gain),
    ];
    for (name, value) in fields {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!("{name} must be finite and in [0.0, 1.0]"));
        }
    }
    Ok(())
}

/// # NDOC
/// component: `subsystems::text_intelligence`
/// purpose: Weighted gate policy that blocks on critical risk only.
/// invariants:
///   - Critical red findings always block.
///   - Non-critical weaknesses surface as warnings, not blockers.
pub fn evaluate_weighted_gate_v1(
    findings: &[CritiqueFindingV1],
    scorecard: &TextQualityScorecardV1,
) -> Result<TextGateDecisionV1, String> {
    validate_scorecard_v1(scorecard)?;
    let critical_codes = BTreeSet::from(CRITICAL_FINDING_CODES.map(|v| v.to_string()));
    let mut blocking_reasons = Vec::new();
    let mut warning_reasons = Vec::new();

    for finding in findings {
        let is_critical = finding.severity == CritiqueSeverityV1::Critical
            || critical_codes.contains(&finding.code);
        if is_critical {
            blocking_reasons.push(format!("{}: {}", finding.code, finding.message));
        } else {
            warning_reasons.push(format!("{}: {}", finding.code, finding.message));
        }
    }

    if scorecard.claims_risk >= 0.8 {
        blocking_reasons.push("claims_risk score is above critical threshold (>= 0.8)".to_string());
    }
    if scorecard.brand_voice_consistency < 0.55 {
        warning_reasons
            .push("brand_voice_consistency below recommended threshold (0.55)".to_string());
    }
    if scorecard.novelty < 0.45 {
        warning_reasons.push("novelty below recommended threshold (0.45)".to_string());
    }

    Ok(TextGateDecisionV1 {
        blocked: !blocking_reasons.is_empty(),
        blocking_reasons,
        warning_reasons,
    })
}

pub fn new_text_artifact_v1(
    workflow_kind: TextWorkflowKindV1,
    campaign_spine_id: impl Into<String>,
    sections: Vec<TextSectionV1>,
    critique_findings: Vec<CritiqueFindingV1>,
    quality: TextQualityScorecardV1,
) -> Result<TextWorkflowArtifactV1, String> {
    let gate_decision = evaluate_weighted_gate_v1(&critique_findings, &quality)?;
    Ok(TextWorkflowArtifactV1 {
        schema_version: TEXT_ARTIFACT_SCHEMA_VERSION_V1.to_string(),
        workflow_kind,
        campaign_spine_id: campaign_spine_id.into(),
        sections,
        critique_findings,
        quality,
        gate_decision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_gate_blocks_on_critical_findings() {
        let findings = vec![CritiqueFindingV1 {
            code: "policy_violation".to_string(),
            severity: CritiqueSeverityV1::High,
            message: "unsafe regulated claim".to_string(),
            section_id: Some("hero".to_string()),
            evidence_ref_ids: Vec::new(),
        }];
        let score = TextQualityScorecardV1 {
            instruction_coverage: 0.8,
            audience_alignment: 0.8,
            claims_risk: 0.4,
            brand_voice_consistency: 0.8,
            novelty: 0.6,
            revision_gain: 0.7,
        };
        let decision = evaluate_weighted_gate_v1(&findings, &score).expect("decision");
        assert!(decision.blocked);
        assert_eq!(decision.blocking_reasons.len(), 1);
    }

    #[test]
    fn weighted_gate_warns_for_non_critical_only() {
        let findings = vec![CritiqueFindingV1 {
            code: "generic_copy".to_string(),
            severity: CritiqueSeverityV1::Medium,
            message: "copy is generic".to_string(),
            section_id: Some("headline".to_string()),
            evidence_ref_ids: Vec::new(),
        }];
        let score = TextQualityScorecardV1 {
            instruction_coverage: 0.8,
            audience_alignment: 0.8,
            claims_risk: 0.3,
            brand_voice_consistency: 0.5,
            novelty: 0.4,
            revision_gain: 0.6,
        };
        let decision = evaluate_weighted_gate_v1(&findings, &score).expect("decision");
        assert!(!decision.blocked);
        assert!(!decision.warning_reasons.is_empty());
    }

    #[test]
    fn scorecard_out_of_range_is_rejected() {
        let score = TextQualityScorecardV1 {
            instruction_coverage: 1.1,
            audience_alignment: 0.8,
            claims_risk: 0.3,
            brand_voice_consistency: 0.8,
            novelty: 0.8,
            revision_gain: 0.8,
        };
        let err = evaluate_weighted_gate_v1(&[], &score).expect_err("must fail bounds");
        assert!(err.contains("instruction_coverage"));
    }

    #[test]
    fn claims_risk_threshold_blocks_even_without_findings() {
        let score = TextQualityScorecardV1 {
            instruction_coverage: 0.9,
            audience_alignment: 0.9,
            claims_risk: 0.85,
            brand_voice_consistency: 0.9,
            novelty: 0.9,
            revision_gain: 0.9,
        };
        let decision = evaluate_weighted_gate_v1(&[], &score).expect("decision");
        assert!(decision.blocked);
        assert_eq!(decision.blocking_reasons.len(), 1);
    }
}
