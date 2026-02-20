use super::prioritized_text_graph_templates_v1;
use crate::subsystems::agent_graph::{
    deterministic_topological_order_v1, validate_agent_graph_definition_v1, AgentGraphNodeKindV1,
};
use crate::subsystems::provider_platform::model_routing::{
    estimate_cost_v1, route_model_v1, GenerationCapabilityV1, ModelRouteV1,
    RoutingBudgetEnvelopeV1, RoutingRequestV1,
};
use crate::subsystems::text_intelligence::{
    new_text_artifact_v1, CampaignSpineV1, CritiqueFindingV1, CritiqueSeverityV1,
    TextQualityScorecardV1, TextSectionV1, TextWorkflowArtifactV1, TextWorkflowKindV1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TEXT_WORKFLOW_RUN_SCHEMA_VERSION_V1: &str = "text_workflow_run.v1";
const DEFAULT_VARIANT_COUNT: u8 = 12;
const MAX_VARIANT_COUNT: u8 = 30;

/// # NDOC
/// component: `subsystems::campaign_orchestration::runtime`
/// purpose: Input contract for deterministic text-workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextWorkflowRunRequestV1 {
    pub template_id: String,
    pub campaign_spine: CampaignSpineV1,
    #[serde(default)]
    pub variant_count: Option<u8>,
    #[serde(default)]
    pub budget: RoutingBudgetEnvelopeV1,
    #[serde(default)]
    pub paid_calls_allowed: bool,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration::runtime`
/// purpose: Per-node deterministic execution trace for auditability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowNodeExecutionTraceV1 {
    pub node_id: String,
    pub node_kind: AgentGraphNodeKindV1,
    pub route: ModelRouteV1,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_cost_usd: f64,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration::runtime`
/// purpose: Full run artifact for text workflow execution and review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextWorkflowRunResultV1 {
    pub schema_version: String,
    pub template_id: String,
    pub graph_id: String,
    pub workflow_kind: TextWorkflowKindV1,
    pub campaign_spine_id: String,
    pub execution_order: Vec<String>,
    pub traces: Vec<WorkflowNodeExecutionTraceV1>,
    pub total_estimated_input_tokens: u32,
    pub total_estimated_output_tokens: u32,
    pub total_estimated_cost_usd: f64,
    pub artifact: TextWorkflowArtifactV1,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration::runtime`
/// purpose: Execute one prioritized text workflow as deterministic, replayable mock orchestration.
/// invariants:
///   - Graph is validated before execution.
///   - Execution order is deterministic topological order.
///   - Token and cost totals are bounded by request budget envelope.
pub fn run_prioritized_text_workflow_v1(
    request: TextWorkflowRunRequestV1,
) -> Result<TextWorkflowRunResultV1, String> {
    if request.template_id.trim().is_empty() {
        return Err("template_id cannot be empty".to_string());
    }
    let variant_count = request.variant_count.unwrap_or(DEFAULT_VARIANT_COUNT);
    if !(1..=MAX_VARIANT_COUNT).contains(&variant_count) {
        return Err(format!(
            "variant_count must be in 1..={MAX_VARIANT_COUNT}, received {}",
            variant_count
        ));
    }

    let templates = prioritized_text_graph_templates_v1(&request.campaign_spine.campaign_spine_id)?;
    let template = templates
        .into_iter()
        .find(|candidate| candidate.template_id == request.template_id)
        .ok_or_else(|| {
            format!(
                "unknown template_id '{}' for campaign spine '{}': expected one of tpl.message_house.v1, tpl.email_landing_sequence.v1, tpl.ad_variant_pack.v1",
                request.template_id, request.campaign_spine.campaign_spine_id
            )
        })?;

    validate_agent_graph_definition_v1(&template.graph)
        .map_err(|err| format!("workflow graph invalid: {err}"))?;
    let order = deterministic_topological_order_v1(&template.graph)
        .map_err(|err| format!("topological order failed: {err}"))?;
    let node_by_id = template
        .graph
        .nodes
        .iter()
        .map(|node| (node.node_id.clone(), node.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut traces = Vec::with_capacity(order.len());
    let mut total_input_tokens: u32 = 0;
    let mut total_output_tokens: u32 = 0;
    let mut total_cost_usd = 0.0_f64;

    for node_id in &order {
        let node = node_by_id
            .get(node_id)
            .ok_or_else(|| format!("missing node '{}' in graph map", node_id))?;
        let (estimated_input_tokens, estimated_output_tokens) =
            estimate_node_tokens(&node.kind, &template.workflow_kind, variant_count);

        total_input_tokens = total_input_tokens
            .checked_add(estimated_input_tokens)
            .ok_or_else(|| "input token total overflow".to_string())?;
        total_output_tokens = total_output_tokens
            .checked_add(estimated_output_tokens)
            .ok_or_else(|| "output token total overflow".to_string())?;
        if total_input_tokens > request.budget.max_total_input_tokens {
            return Err(format!(
                "input token budget exceeded: {} > {}",
                total_input_tokens, request.budget.max_total_input_tokens
            ));
        }
        if total_output_tokens > request.budget.max_total_output_tokens {
            return Err(format!(
                "output token budget exceeded: {} > {}",
                total_output_tokens, request.budget.max_total_output_tokens
            ));
        }

        let route_request = RoutingRequestV1 {
            capability: GenerationCapabilityV1::Text,
            complexity_score: complexity_score_for_kind(&node.kind),
            quality_priority: quality_priority_for_kind(&node.kind),
            latency_priority: 5,
            expected_input_tokens: estimated_input_tokens,
            expected_output_tokens: estimated_output_tokens,
            paid_calls_allowed: request.paid_calls_allowed,
            allow_openai: true,
            allow_google: true,
            allow_mock_fallback: true,
        };

        let route = route_model_v1(&route_request, &request.budget)
            .map_err(|err| format!("model routing failed for node '{}': {}", node.node_id, err))?;
        let node_cost = estimate_cost_v1(&route, estimated_input_tokens, estimated_output_tokens);
        total_cost_usd += node_cost;

        traces.push(WorkflowNodeExecutionTraceV1 {
            node_id: node.node_id.clone(),
            node_kind: node.kind.clone(),
            route,
            estimated_input_tokens,
            estimated_output_tokens,
            estimated_cost_usd: node_cost,
        });
    }

    if total_cost_usd > request.budget.max_cost_per_run_usd {
        return Err(format!(
            "estimated run cost ${:.4} exceeds max_cost_per_run_usd ${:.4}",
            total_cost_usd, request.budget.max_cost_per_run_usd
        ));
    }

    let sections = build_sections(
        &template.workflow_kind,
        &request.campaign_spine,
        variant_count,
    );
    let findings = build_findings(
        &template.workflow_kind,
        &request.campaign_spine,
        variant_count,
        sections.len(),
    );
    let quality = build_scorecard(
        &template.workflow_kind,
        &request.campaign_spine,
        variant_count,
        sections.len(),
    );

    let artifact = new_text_artifact_v1(
        template.workflow_kind.clone(),
        request.campaign_spine.campaign_spine_id.clone(),
        sections,
        findings,
        quality,
    )
    .map_err(|err| format!("failed to build text workflow artifact: {err}"))?;

    Ok(TextWorkflowRunResultV1 {
        schema_version: TEXT_WORKFLOW_RUN_SCHEMA_VERSION_V1.to_string(),
        template_id: template.template_id,
        graph_id: template.graph.graph_id,
        workflow_kind: template.workflow_kind,
        campaign_spine_id: request.campaign_spine.campaign_spine_id,
        execution_order: order,
        traces,
        total_estimated_input_tokens: total_input_tokens,
        total_estimated_output_tokens: total_output_tokens,
        total_estimated_cost_usd: total_cost_usd,
        artifact,
    })
}

fn complexity_score_for_kind(kind: &AgentGraphNodeKindV1) -> u8 {
    match kind {
        AgentGraphNodeKindV1::Planner => 8,
        AgentGraphNodeKindV1::Generator => 7,
        AgentGraphNodeKindV1::ToolCall => 5,
        AgentGraphNodeKindV1::Critic => 7,
        AgentGraphNodeKindV1::Refiner => 6,
        AgentGraphNodeKindV1::ReviewGate => 5,
        AgentGraphNodeKindV1::Merge => 6,
    }
}

fn quality_priority_for_kind(kind: &AgentGraphNodeKindV1) -> u8 {
    match kind {
        AgentGraphNodeKindV1::Planner => 8,
        AgentGraphNodeKindV1::Generator => 7,
        AgentGraphNodeKindV1::ToolCall => 5,
        AgentGraphNodeKindV1::Critic => 8,
        AgentGraphNodeKindV1::Refiner => 7,
        AgentGraphNodeKindV1::ReviewGate => 6,
        AgentGraphNodeKindV1::Merge => 6,
    }
}

fn estimate_node_tokens(
    kind: &AgentGraphNodeKindV1,
    workflow_kind: &TextWorkflowKindV1,
    variant_count: u8,
) -> (u32, u32) {
    match kind {
        AgentGraphNodeKindV1::Planner => (900, 260),
        AgentGraphNodeKindV1::Generator => {
            if matches!(
                workflow_kind,
                TextWorkflowKindV1::AdVariantPackExperimentPlan
            ) {
                let variants = u32::from(variant_count.max(1));
                (1300 + (variants * 90), 500 + (variants * 110))
            } else {
                (1300, 680)
            }
        }
        AgentGraphNodeKindV1::ToolCall => (600, 220),
        AgentGraphNodeKindV1::Critic => (1100, 420),
        AgentGraphNodeKindV1::Refiner => (1000, 460),
        AgentGraphNodeKindV1::ReviewGate => (700, 180),
        AgentGraphNodeKindV1::Merge => (900, 320),
    }
}

fn build_sections(
    workflow_kind: &TextWorkflowKindV1,
    spine: &CampaignSpineV1,
    variant_count: u8,
) -> Vec<TextSectionV1> {
    match workflow_kind {
        TextWorkflowKindV1::PersonaPositioningMessageHouse => vec![
            TextSectionV1 {
                section_id: "positioning_statement".to_string(),
                section_title: "Positioning Statement".to_string(),
                content: spine.positioning_statement.clone(),
            },
            TextSectionV1 {
                section_id: "message_house".to_string(),
                section_title: "Message House".to_string(),
                content: format!(
                    "Big idea: {} | pillars: {}",
                    spine.message_house.big_idea,
                    spine
                        .message_house
                        .pillars
                        .iter()
                        .map(|pillar| pillar.title.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
            TextSectionV1 {
                section_id: "audience_segments".to_string(),
                section_title: "Audience Segments".to_string(),
                content: spine.audience_segments.join("; "),
            },
        ],
        TextWorkflowKindV1::EmailLandingSequence => vec![
            TextSectionV1 {
                section_id: "email_1".to_string(),
                section_title: "Email 1: Problem and Hook".to_string(),
                content: format!(
                    "Subject: A simpler routine for {}. Body: {}",
                    spine.product_name, spine.offer_summary
                ),
            },
            TextSectionV1 {
                section_id: "email_2".to_string(),
                section_title: "Email 2: Proof and Objection Handling".to_string(),
                content: format!(
                    "Proof points: {}",
                    spine
                        .message_house
                        .proof_points
                        .iter()
                        .map(|proof| proof.claim_text.clone())
                        .collect::<Vec<_>>()
                        .join(" | ")
                ),
            },
            TextSectionV1 {
                section_id: "landing_page".to_string(),
                section_title: "Landing Page Structure".to_string(),
                content: format!(
                    "Hero for {} with CTA aligned to offer: {}",
                    spine.product_name, spine.offer_summary
                ),
            },
        ],
        TextWorkflowKindV1::AdVariantPackExperimentPlan => (1..=variant_count)
            .map(|idx| TextSectionV1 {
                section_id: format!("variant_{idx}"),
                section_title: format!("Ad Variant #{idx}"),
                content: format!(
                    "Hook {} for {} audience: {} | CTA: Shop now",
                    idx,
                    spine.product_name,
                    spine
                        .audience_segments
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "general pet owners".to_string())
                ),
            })
            .collect(),
        TextWorkflowKindV1::IntegratedLaunchCampaignKit => vec![
            TextSectionV1 {
                section_id: "launch_spine".to_string(),
                section_title: "Launch Campaign Spine".to_string(),
                content: format!(
                    "Positioning: {} | Offer: {}",
                    spine.positioning_statement, spine.offer_summary
                ),
            },
            TextSectionV1 {
                section_id: "channel_matrix".to_string(),
                section_title: "Channel Matrix".to_string(),
                content: "Email, landing, ads, and social plans share one message spine"
                    .to_string(),
            },
        ],
    }
}

fn build_findings(
    workflow_kind: &TextWorkflowKindV1,
    spine: &CampaignSpineV1,
    variant_count: u8,
    section_count: usize,
) -> Vec<CritiqueFindingV1> {
    let mut findings = Vec::new();

    if spine.evidence_refs.is_empty() {
        findings.push(CritiqueFindingV1 {
            code: "unsupported_high_risk_claim".to_string(),
            severity: CritiqueSeverityV1::Critical,
            message: "no evidence_refs provided for claims-sensitive output".to_string(),
            section_id: None,
            evidence_ref_ids: Vec::new(),
        });
    }

    if spine.message_house.proof_points.is_empty() {
        findings.push(CritiqueFindingV1 {
            code: "missing_required_section".to_string(),
            severity: CritiqueSeverityV1::Critical,
            message: "message_house.proof_points cannot be empty".to_string(),
            section_id: Some("message_house".to_string()),
            evidence_ref_ids: Vec::new(),
        });
    }

    let required_sections = required_sections(workflow_kind, variant_count);
    if section_count < required_sections {
        findings.push(CritiqueFindingV1 {
            code: "missing_required_section".to_string(),
            severity: CritiqueSeverityV1::Critical,
            message: format!(
                "workflow requires at least {required_sections} sections but produced {section_count}"
            ),
            section_id: None,
            evidence_ref_ids: Vec::new(),
        });
    }

    if matches!(
        workflow_kind,
        TextWorkflowKindV1::AdVariantPackExperimentPlan
    ) && variant_count < 10
    {
        findings.push(CritiqueFindingV1 {
            code: "generic_copy".to_string(),
            severity: CritiqueSeverityV1::Medium,
            message: "variant_count below recommended threshold (10)".to_string(),
            section_id: None,
            evidence_ref_ids: Vec::new(),
        });
    }

    findings
}

fn build_scorecard(
    workflow_kind: &TextWorkflowKindV1,
    spine: &CampaignSpineV1,
    variant_count: u8,
    section_count: usize,
) -> TextQualityScorecardV1 {
    let required = required_sections(workflow_kind, variant_count) as f64;
    let instruction_coverage = clamp01((section_count as f64 / required).min(1.0));
    let audience_alignment = clamp01(0.55 + ((spine.audience_segments.len() as f64) * 0.08));
    let claims_risk = if spine.evidence_refs.is_empty() {
        0.92
    } else {
        clamp01(0.22 + (spine.message_house.proof_points.len() as f64 * 0.03))
    };
    let brand_voice_consistency = if spine.message_house.tone_guide.is_empty() {
        0.52
    } else {
        0.76
    };
    let novelty = if matches!(
        workflow_kind,
        TextWorkflowKindV1::AdVariantPackExperimentPlan
    ) {
        clamp01((variant_count as f64) / 20.0)
    } else {
        0.63
    };
    let revision_gain = if spine.evidence_refs.is_empty() {
        0.3
    } else {
        0.64
    };

    TextQualityScorecardV1 {
        instruction_coverage,
        audience_alignment,
        claims_risk,
        brand_voice_consistency,
        novelty,
        revision_gain,
    }
}

fn required_sections(workflow_kind: &TextWorkflowKindV1, variant_count: u8) -> usize {
    match workflow_kind {
        TextWorkflowKindV1::PersonaPositioningMessageHouse => 3,
        TextWorkflowKindV1::EmailLandingSequence => 3,
        TextWorkflowKindV1::AdVariantPackExperimentPlan => usize::from(variant_count),
        TextWorkflowKindV1::IntegratedLaunchCampaignKit => 2,
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::provider_platform::model_routing::GenerationProviderV1;
    use crate::subsystems::text_intelligence::{
        CampaignSpineV1, EvidenceRefV1, MessageHouseV1, MessagePillarV1, ProofPointV1,
    };

    fn sample_spine(with_evidence: bool) -> CampaignSpineV1 {
        CampaignSpineV1 {
            campaign_spine_id: "spine.test.v1".to_string(),
            product_name: "Nature's Diet Raw Mix".to_string(),
            offer_summary: "Save 20% on first order".to_string(),
            audience_segments: vec![
                "new puppy owners".to_string(),
                "sensitive stomach".to_string(),
            ],
            positioning_statement: "Raw-first nutrition with practical prep".to_string(),
            message_house: MessageHouseV1 {
                big_idea: "Fresh confidence in every bowl".to_string(),
                pillars: vec![MessagePillarV1 {
                    pillar_id: "p1".to_string(),
                    title: "Digestive comfort".to_string(),
                    supporting_points: vec!["gentle proteins".to_string()],
                }],
                proof_points: vec![ProofPointV1 {
                    claim_id: "claim1".to_string(),
                    claim_text: "high digestibility blend".to_string(),
                    evidence_ref_ids: vec!["ev1".to_string()],
                }],
                do_not_say: vec!["cure".to_string()],
                tone_guide: vec!["clear".to_string(), "grounded".to_string()],
            },
            evidence_refs: if with_evidence {
                vec![EvidenceRefV1 {
                    evidence_id: "ev1".to_string(),
                    source_ref: "internal.digestibility.v1".to_string(),
                    excerpt: "digestibility improved 11% vs baseline".to_string(),
                }]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn deterministic_run_produces_unblocked_artifact_with_evidence() {
        let request = TextWorkflowRunRequestV1 {
            template_id: "tpl.email_landing_sequence.v1".to_string(),
            campaign_spine: sample_spine(true),
            variant_count: None,
            budget: RoutingBudgetEnvelopeV1::default(),
            paid_calls_allowed: false,
        };

        let result = run_prioritized_text_workflow_v1(request).expect("workflow");
        assert_eq!(result.schema_version, "text_workflow_run.v1");
        assert!(!result.execution_order.is_empty());
        assert_eq!(result.execution_order.len(), result.traces.len());
        assert!(!result.artifact.gate_decision.blocked);
        assert!(result
            .traces
            .iter()
            .all(|trace| trace.route.provider == GenerationProviderV1::LocalMock));
    }

    #[test]
    fn missing_evidence_blocks_weighted_gate() {
        let request = TextWorkflowRunRequestV1 {
            template_id: "tpl.message_house.v1".to_string(),
            campaign_spine: sample_spine(false),
            variant_count: None,
            budget: RoutingBudgetEnvelopeV1::default(),
            paid_calls_allowed: false,
        };

        let result = run_prioritized_text_workflow_v1(request).expect("workflow");
        assert!(result.artifact.gate_decision.blocked);
        assert!(result
            .artifact
            .gate_decision
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("unsupported_high_risk_claim")));
    }

    #[test]
    fn variant_count_bounds_are_enforced() {
        let request = TextWorkflowRunRequestV1 {
            template_id: "tpl.ad_variant_pack.v1".to_string(),
            campaign_spine: sample_spine(true),
            variant_count: Some(31),
            budget: RoutingBudgetEnvelopeV1::default(),
            paid_calls_allowed: false,
        };

        let err = run_prioritized_text_workflow_v1(request).expect_err("must reject");
        assert!(err.contains("variant_count"));
    }
}
