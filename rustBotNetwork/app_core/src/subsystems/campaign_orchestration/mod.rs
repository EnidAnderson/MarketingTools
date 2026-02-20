use crate::subsystems::agent_graph::{
    validate_agent_graph_definition_v1, AgentGraphDefinitionV1, AgentGraphEdgeConditionV1,
    AgentGraphEdgeV1, AgentGraphNodeKindV1, AgentGraphNodeV1,
};
use crate::subsystems::text_intelligence::TextWorkflowKindV1;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Domain contracts for campaign execution plans and run state.
/// invariants:
///   - Campaign runs are immutable after completion.
///   - Every run references a pipeline definition version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignRunDescriptor {
    pub campaign_id: String,
    pub pipeline_name: String,
    pub pipeline_version: String,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Placeholder orchestration trait for future campaign runtimes.
pub trait CampaignOrchestrator: Send + Sync {
    fn orchestrator_name(&self) -> &'static str;
}

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Graph template descriptor for high-complexity campaign workflows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignGraphTemplateV1 {
    pub template_id: String,
    pub title: String,
    pub workflow_kind: TextWorkflowKindV1,
    pub graph: AgentGraphDefinitionV1,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Build the three priority workflow templates tied to a shared campaign spine.
pub fn prioritized_text_graph_templates_v1(
    campaign_spine_id: &str,
) -> Result<Vec<CampaignGraphTemplateV1>, String> {
    if campaign_spine_id.trim().is_empty() {
        return Err("campaign_spine_id cannot be empty".to_string());
    }

    let templates = vec![
        message_house_template(campaign_spine_id),
        email_landing_sequence_template(campaign_spine_id),
        ad_variant_pack_template(campaign_spine_id),
    ];

    for template in &templates {
        validate_agent_graph_definition_v1(&template.graph)
            .map_err(|err| format!("template '{}' invalid: {err}", template.template_id))?;
    }

    Ok(templates)
}

fn message_house_template(campaign_spine_id: &str) -> CampaignGraphTemplateV1 {
    CampaignGraphTemplateV1 {
        template_id: "tpl.message_house.v1".to_string(),
        title: "Persona + Positioning + Message House".to_string(),
        workflow_kind: TextWorkflowKindV1::PersonaPositioningMessageHouse,
        graph: AgentGraphDefinitionV1 {
            graph_id: "wf.message_house.v1".to_string(),
            version: "1".to_string(),
            entry_node_id: "planner".to_string(),
            nodes: vec![
                node(
                    "planner",
                    AgentGraphNodeKindV1::Planner,
                    "Decompose inputs into persona and positioning workstreams",
                ),
                node(
                    "generator",
                    AgentGraphNodeKindV1::Generator,
                    "Draft persona cards and message house",
                ),
                node(
                    "critic",
                    AgentGraphNodeKindV1::Critic,
                    "Critique evidence-linking and claims risk",
                ),
                node(
                    "refiner",
                    AgentGraphNodeKindV1::Refiner,
                    "Refine weak proof points and tone constraints",
                ),
                node(
                    "review_gate",
                    AgentGraphNodeKindV1::ReviewGate,
                    "Gate output on critical safety constraints",
                ),
            ],
            edges: vec![
                edge("planner", "generator", AgentGraphEdgeConditionV1::Always),
                edge("generator", "critic", AgentGraphEdgeConditionV1::Always),
                edge("critic", "refiner", AgentGraphEdgeConditionV1::OnFailure),
                edge(
                    "critic",
                    "review_gate",
                    AgentGraphEdgeConditionV1::ScoreAtLeast {
                        metric: "instruction_coverage".to_string(),
                        threshold: 0.7,
                    },
                ),
                edge("refiner", "review_gate", AgentGraphEdgeConditionV1::Always),
            ],
            metadata: graph_metadata(campaign_spine_id, "message_house"),
        },
    }
}

fn email_landing_sequence_template(campaign_spine_id: &str) -> CampaignGraphTemplateV1 {
    CampaignGraphTemplateV1 {
        template_id: "tpl.email_landing_sequence.v1".to_string(),
        title: "Email + Landing Sequence".to_string(),
        workflow_kind: TextWorkflowKindV1::EmailLandingSequence,
        graph: AgentGraphDefinitionV1 {
            graph_id: "wf.email_landing_sequence.v1".to_string(),
            version: "1".to_string(),
            entry_node_id: "planner".to_string(),
            nodes: vec![
                node(
                    "planner",
                    AgentGraphNodeKindV1::Planner,
                    "Plan sequence arcs and conversion milestones",
                ),
                node(
                    "email_generator",
                    AgentGraphNodeKindV1::Generator,
                    "Generate email sequence drafts",
                ),
                node(
                    "landing_generator",
                    AgentGraphNodeKindV1::Generator,
                    "Generate landing page draft",
                ),
                node(
                    "merge",
                    AgentGraphNodeKindV1::Merge,
                    "Merge drafts into one consistent narrative spine",
                ),
                node(
                    "critic",
                    AgentGraphNodeKindV1::Critic,
                    "Critique consistency, claims safety, and CTA strength",
                ),
                node(
                    "refiner",
                    AgentGraphNodeKindV1::Refiner,
                    "Refine weak sections flagged by critic",
                ),
                node(
                    "review_gate",
                    AgentGraphNodeKindV1::ReviewGate,
                    "Gate final sequence",
                ),
            ],
            edges: vec![
                edge(
                    "planner",
                    "email_generator",
                    AgentGraphEdgeConditionV1::Always,
                ),
                edge(
                    "planner",
                    "landing_generator",
                    AgentGraphEdgeConditionV1::Always,
                ),
                edge(
                    "email_generator",
                    "merge",
                    AgentGraphEdgeConditionV1::Always,
                ),
                edge(
                    "landing_generator",
                    "merge",
                    AgentGraphEdgeConditionV1::Always,
                ),
                edge("merge", "critic", AgentGraphEdgeConditionV1::Always),
                edge("critic", "refiner", AgentGraphEdgeConditionV1::OnFailure),
                edge(
                    "critic",
                    "review_gate",
                    AgentGraphEdgeConditionV1::ScoreAtLeast {
                        metric: "audience_alignment".to_string(),
                        threshold: 0.68,
                    },
                ),
                edge("refiner", "review_gate", AgentGraphEdgeConditionV1::Always),
            ],
            metadata: graph_metadata(campaign_spine_id, "email_landing_sequence"),
        },
    }
}

fn ad_variant_pack_template(campaign_spine_id: &str) -> CampaignGraphTemplateV1 {
    CampaignGraphTemplateV1 {
        template_id: "tpl.ad_variant_pack.v1".to_string(),
        title: "Ad Variant Pack + Experiment Plan".to_string(),
        workflow_kind: TextWorkflowKindV1::AdVariantPackExperimentPlan,
        graph: AgentGraphDefinitionV1 {
            graph_id: "wf.ad_variant_pack.v1".to_string(),
            version: "1".to_string(),
            entry_node_id: "planner".to_string(),
            nodes: vec![
                node(
                    "planner",
                    AgentGraphNodeKindV1::Planner,
                    "Define angle taxonomy and test hypotheses",
                ),
                node(
                    "generator",
                    AgentGraphNodeKindV1::Generator,
                    "Generate structured ad variants",
                ),
                node(
                    "critic",
                    AgentGraphNodeKindV1::Critic,
                    "Critique novelty, compliance, and conversion heuristics",
                ),
                node(
                    "refiner",
                    AgentGraphNodeKindV1::Refiner,
                    "Refine low-performing variants",
                ),
                node(
                    "review_gate",
                    AgentGraphNodeKindV1::ReviewGate,
                    "Gate output and experiment matrix",
                ),
            ],
            edges: vec![
                edge("planner", "generator", AgentGraphEdgeConditionV1::Always),
                edge("generator", "critic", AgentGraphEdgeConditionV1::Always),
                edge("critic", "refiner", AgentGraphEdgeConditionV1::OnFailure),
                edge(
                    "critic",
                    "review_gate",
                    AgentGraphEdgeConditionV1::ScoreAtLeast {
                        metric: "novelty".to_string(),
                        threshold: 0.55,
                    },
                ),
                edge("refiner", "review_gate", AgentGraphEdgeConditionV1::Always),
            ],
            metadata: graph_metadata(campaign_spine_id, "ad_variant_pack"),
        },
    }
}

fn node(node_id: &str, kind: AgentGraphNodeKindV1, description: &str) -> AgentGraphNodeV1 {
    AgentGraphNodeV1 {
        node_id: node_id.to_string(),
        kind,
        description: description.to_string(),
        params: serde_json::Value::Null,
    }
}

fn edge(from: &str, to: &str, condition: AgentGraphEdgeConditionV1) -> AgentGraphEdgeV1 {
    AgentGraphEdgeV1 {
        from_node_id: from.to_string(),
        to_node_id: to.to_string(),
        condition,
    }
}

fn graph_metadata(campaign_spine_id: &str, workflow_name: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "campaign_spine_id".to_string(),
        campaign_spine_id.to_string(),
    );
    metadata.insert("workflow_name".to_string(), workflow_name.to_string());
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prioritized_templates_validate_and_share_spine() {
        let spine = "spine.alpha";
        let templates = prioritized_text_graph_templates_v1(spine).expect("templates");
        assert_eq!(templates.len(), 3);
        for template in templates {
            assert_eq!(
                template.graph.metadata.get("campaign_spine_id"),
                Some(&spine.to_string())
            );
            assert!(template.graph.nodes.len() >= 5);
        }
    }

    #[test]
    fn prioritized_templates_require_non_empty_spine() {
        let err = prioritized_text_graph_templates_v1(" ").expect_err("must fail");
        assert!(err.contains("campaign_spine_id"));
    }
}
