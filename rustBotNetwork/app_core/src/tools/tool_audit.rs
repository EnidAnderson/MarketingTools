use super::tool_definition::ToolMaturity;
use super::tool_registry::{ToolRegistry, ToolRegistryConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// # NDOC
/// component: `tools::tool_audit`
/// purpose: Versioned operator-facing audit snapshot for tool usability readiness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolAuditReportV1 {
    pub schema_version: String,
    pub entries: Vec<ToolAuditEntryV1>,
}

/// # NDOC
/// component: `tools::tool_audit`
/// purpose: One scored tool audit row for release gating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolAuditEntryV1 {
    pub tool_name: String,
    pub module_name: String,
    pub in_registry: bool,
    pub registered_maturity: Option<ToolMaturity>,
    pub determinism_score: u8,
    pub actionability_score: u8,
    pub failure_quality_score: u8,
    pub recommended_maturity: ToolMaturity,
    pub actionable_next_step: String,
}

const TOOL_AUDIT_SCHEMA_VERSION: &str = "tool_audit_report.v1";

const TOOL_MODULES: &[&str] = &[
    "code_verifier",
    "competitive_analysis",
    "css_analyzer",
    "data_validator",
    "data_viz",
    "echo_tool",
    "email_sender_tool",
    "event_calendar_tool",
    "facebook_adapter",
    "generation_budget_manager",
    "gif_generator_tool",
    "google_ads_adapter",
    "html_bundler",
    "human_feedback_tool",
    "image_generation",
    "image_manipulation_tool",
    "marketing_platform_manager",
    "memory_retrieval",
    "product_crawler",
    "screenshot_tool",
    "seo_analyzer",
    "video_generator_tool",
];

/// # NDOC
/// component: `tools::tool_audit`
/// purpose: Build static + registry-aware usability audit across all tool modules.
/// invariants:
///   - Every module listed in `tools/mod.rs` (excluding framework modules) is represented.
///   - Stable recommendation requires scores >= 4 across determinism/actionability/failure quality.
pub fn build_tool_audit_report_v1() -> ToolAuditReportV1 {
    let registry = ToolRegistry::with_config(ToolRegistryConfig {
        include_experimental: true,
    });
    let mut registered = registry
        .get_available_tool_definitions()
        .into_iter()
        .map(|d| (d.name, d.maturity))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut entries = Vec::new();
    for module_name in TOOL_MODULES {
        let (
            determinism_score,
            actionability_score,
            failure_quality_score,
            recommended_maturity,
            actionable_next_step,
        ) = baseline_scores(module_name);
        let registry_name = to_registry_name(module_name);
        let registered_maturity = registered.remove(&registry_name);
        entries.push(ToolAuditEntryV1 {
            tool_name: registry_name,
            module_name: (*module_name).to_string(),
            in_registry: registered_maturity.is_some(),
            registered_maturity,
            determinism_score,
            actionability_score,
            failure_quality_score,
            recommended_maturity,
            actionable_next_step,
        });
    }

    ToolAuditReportV1 {
        schema_version: TOOL_AUDIT_SCHEMA_VERSION.to_string(),
        entries,
    }
}

fn to_registry_name(module_name: &str) -> String {
    match module_name {
        "echo_tool" => "echo_tool".to_string(),
        "seo_analyzer" => "seo_analyzer".to_string(),
        "competitive_analysis" => "competitive_analysis".to_string(),
        other => other.to_string(),
    }
}

fn baseline_scores(module_name: &str) -> (u8, u8, u8, ToolMaturity, String) {
    match module_name {
        "echo_tool" => (
            5,
            4,
            5,
            ToolMaturity::Stable,
            "Use as registry healthcheck and CI smoke step.".to_string(),
        ),
        "seo_analyzer" => (
            5,
            4,
            4,
            ToolMaturity::Stable,
            "Add to copy-review workflow with pre-publish checks.".to_string(),
        ),
        "competitive_analysis" => (
            2,
            4,
            3,
            ToolMaturity::Experimental,
            "Add deterministic fixture mode and connector error taxonomy hardening.".to_string(),
        ),
        "css_analyzer" | "html_bundler" | "screenshot_tool" | "code_verifier" | "data_validator" => (
            4,
            3,
            3,
            ToolMaturity::Experimental,
            "Define output artifact contract and register with default template.".to_string(),
        ),
        _ => (
            2,
            2,
            2,
            ToolMaturity::Disabled,
            "Implement runtime contract, typed errors, and actionable output summary before exposure."
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_audit_report_covers_all_tool_modules() {
        let report = build_tool_audit_report_v1();
        let modules = report
            .entries
            .iter()
            .map(|e| e.module_name.as_str())
            .collect::<BTreeSet<_>>();
        let expected = TOOL_MODULES.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(modules, expected);
    }

    #[test]
    fn test_tool_audit_stable_recommendations_meet_thresholds() {
        let report = build_tool_audit_report_v1();
        for entry in &report.entries {
            if entry.recommended_maturity == ToolMaturity::Stable {
                assert!(
                    entry.determinism_score >= 4,
                    "stable tool '{}' determinism below threshold",
                    entry.tool_name
                );
                assert!(
                    entry.actionability_score >= 4,
                    "stable tool '{}' actionability below threshold",
                    entry.tool_name
                );
                assert!(
                    entry.failure_quality_score >= 4,
                    "stable tool '{}' failure quality below threshold",
                    entry.tool_name
                );
            }
        }
    }
}
