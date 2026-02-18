use super::base_tool::BaseTool;
use super::competitive_analysis::CompetitiveAnalysisTool;
use super::echo_tool::EchoTool;
use super::seo_analyzer::SEOAnalyzerTool;
use super::tool_definition::{
    ParameterDefinition, ToolComplexity, ToolDefinition, ToolMaturity, ToolRuntime, ToolUIMetadata,
};
use crate::contracts::{ToolError, ToolErrorEnvelope, ToolErrorSource};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// # NDOC
/// component: `tools::tool_registry`
/// purpose: Runtime config controlling tool visibility and gating.
#[derive(Debug, Clone, Copy)]
pub struct ToolRegistryConfig {
    pub include_experimental: bool,
}

impl Default for ToolRegistryConfig {
    fn default() -> Self {
        let include_experimental = std::env::var("TOOL_REGISTRY_INCLUDE_EXPERIMENTAL")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);
        Self {
            include_experimental,
        }
    }
}

/// # NDOC
/// component: `tools::tool_registry`
/// purpose: Deterministic runtime registry for tool discovery and execution.
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn ToolRuntime>>,
    config: ToolRegistryConfig,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::with_config(ToolRegistryConfig::default())
    }

    pub fn with_config(config: ToolRegistryConfig) -> Self {
        let mut registry = ToolRegistry {
            tools: BTreeMap::new(),
            config,
        };
        registry.register_tool(Arc::new(EchoTool::new()));
        registry.register_tool(Arc::new(BaseToolAdapter::new(
            SEOAnalyzerTool::new(),
            ToolDefinition {
                name: "seo_analyzer".to_string(),
                description:
                    "Analyzes marketing copy for readability and keyword coverage using deterministic metrics."
                        .to_string(),
                maturity: ToolMaturity::Stable,
                human_workflow:
                    "Review keyword_density_pct and avg_words_per_sentence, then revise copy before publishing."
                        .to_string(),
                output_artifact_kind: "analysis.seo_metrics.v1".to_string(),
                requires_review: true,
                default_input_template: json!({
                    "text": "Nature's Diet dog food helps active dogs recover quickly and stay healthy.",
                    "keywords": ["Nature's Diet", "dog food", "healthy"]
                }),
                ui_metadata: ToolUIMetadata {
                    category: "Content".to_string(),
                    display_name: "SEO Analyzer".to_string(),
                    icon: Some("search".to_string()),
                    complexity: ToolComplexity::Simple,
                    estimated_time_seconds: 2,
                    tags: vec!["seo".to_string(), "copy".to_string()],
                },
                parameters: vec![
                    ParameterDefinition {
                        name: "text".to_string(),
                        r#type: "string".to_string(),
                        description: "Marketing copy to analyze.".to_string(),
                        optional: false,
                    },
                    ParameterDefinition {
                        name: "keywords".to_string(),
                        r#type: "array<string>".to_string(),
                        description: "Optional keywords to track density for.".to_string(),
                        optional: true,
                    },
                ],
                input_examples: vec![json!({
                    "text": "Nature's Diet provides premium pet nutrition.",
                    "keywords": ["Nature's Diet", "pet nutrition"]
                })],
                output_schema: Some(json!({
                    "type": "object",
                    "required": ["word_count", "sentence_count", "avg_words_per_sentence", "keyword_density_pct"]
                })),
            },
        )));
        registry.register_tool(Arc::new(BaseToolAdapter::new(
            CompetitiveAnalysisTool::new(),
            ToolDefinition {
                name: "competitive_analysis".to_string(),
                description: "Pulls competitive signal data from live web search and summarizes recurring messaging."
                    .to_string(),
                maturity: ToolMaturity::Experimental,
                human_workflow:
                    "Verify cited source URLs, then decide campaign differentiation and update messaging brief."
                        .to_string(),
                output_artifact_kind: "analysis.competitive_signals.v1".to_string(),
                requires_review: true,
                default_input_template: json!({"topic":"premium pet food market trends","max_sources":8}),
                ui_metadata: ToolUIMetadata {
                    category: "Market Research".to_string(),
                    display_name: "Competitive Analysis".to_string(),
                    icon: Some("insights".to_string()),
                    complexity: ToolComplexity::Advanced,
                    estimated_time_seconds: 20,
                    tags: vec!["research".to_string(), "competitors".to_string()],
                },
                parameters: vec![
                    ParameterDefinition {
                        name: "topic".to_string(),
                        r#type: "string".to_string(),
                        description: "Topic to analyze.".to_string(),
                        optional: false,
                    },
                    ParameterDefinition {
                        name: "max_sources".to_string(),
                        r#type: "integer".to_string(),
                        description: "Optional source cap (3..20).".to_string(),
                        optional: true,
                    },
                ],
                input_examples: vec![json!({"topic":"raw dog food competitors","max_sources":8})],
                output_schema: Some(json!({
                    "type":"object",
                    "required":["topic","source_count","sources","signals","signal_report_markdown"]
                })),
            },
        )));
        registry
    }

    pub fn register_tool(&mut self, tool: Arc<dyn ToolRuntime>) {
        let definition = tool.definition();
        self.tools.insert(definition.name.clone(), tool);
    }

    pub fn get_tool_instance(&self, tool_name: &str) -> Option<Arc<dyn ToolRuntime>> {
        let tool = self.tools.get(tool_name)?;
        let definition = tool.definition();
        if !self.is_visible(&definition.maturity) || !tool.is_available() {
            return None;
        }
        Some(Arc::clone(tool))
    }

    pub fn get_available_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| tool.definition())
            .filter(|def| self.is_visible(&def.maturity))
            .collect()
    }

    pub async fn execute_tool(
        &self,
        tool_name: &str,
        input: Value,
    ) -> Result<Value, ToolErrorEnvelope> {
        let Some(tool) = self.get_tool_instance(tool_name) else {
            return Err(ToolErrorEnvelope::new(
                "tool_unavailable",
                crate::contracts::ToolErrorCategory::Validation,
                ToolErrorSource::Runtime,
                format!("Tool '{}' not found or unavailable.", tool_name),
                false,
            ));
        };

        let task = tokio::spawn(async move { tool.execute(input).await });
        match task.await {
            Ok(result) => result,
            Err(join_err) => Err(ToolErrorEnvelope::new(
                "internal_error",
                crate::contracts::ToolErrorCategory::Internal,
                ToolErrorSource::Runtime,
                format!("tool execution panicked: {}", join_err),
                false,
            )),
        }
    }

    fn is_visible(&self, maturity: &ToolMaturity) -> bool {
        match maturity {
            ToolMaturity::Stable => true,
            ToolMaturity::Experimental => self.config.include_experimental,
            ToolMaturity::Disabled => false,
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

struct BaseToolAdapter<T>
where
    T: BaseTool + Send + Sync + 'static,
{
    tool: T,
    definition: ToolDefinition,
}

impl<T> BaseToolAdapter<T>
where
    T: BaseTool + Send + Sync + 'static,
{
    fn new(tool: T, definition: ToolDefinition) -> Self {
        Self { tool, definition }
    }
}

#[async_trait]
impl<T> ToolRuntime for BaseToolAdapter<T>
where
    T: BaseTool + Send + Sync + 'static,
{
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn is_available(&self) -> bool {
        self.tool.is_available()
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolErrorEnvelope> {
        self.tool.run(args).await.map_err(convert_tool_error)
    }
}

fn convert_tool_error(err: Box<dyn std::error::Error + Send + Sync>) -> ToolErrorEnvelope {
    if let Some(tool_error) = err.downcast_ref::<ToolError>() {
        let mut converted = ToolErrorEnvelope::from(tool_error.clone());
        converted.source = ToolErrorSource::Tool;
        return converted;
    }
    ToolErrorEnvelope::new(
        "tool_execution_error",
        crate::contracts::ToolErrorCategory::Internal,
        ToolErrorSource::Tool,
        err.to_string(),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_stable_tools_have_runnable_defaults() {
        let registry = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: false,
        });
        let definitions = registry.get_available_tool_definitions();
        assert!(!definitions.is_empty());

        for def in definitions {
            if def.maturity != ToolMaturity::Stable {
                continue;
            }
            assert!(
                def.default_input_template.is_object(),
                "stable tool '{}' must expose object default template",
                def.name
            );
            let has_fields = def
                .default_input_template
                .as_object()
                .map(|obj| !obj.is_empty())
                .unwrap_or(false);
            assert!(
                has_fields,
                "stable tool '{}' default must be non-empty",
                def.name
            );
            assert!(
                !def.human_workflow.trim().is_empty(),
                "stable tool '{}' must include human_workflow",
                def.name
            );
        }
    }

    #[test]
    fn test_tool_registry_maturity_filtering() {
        let stable_only = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: false,
        });
        let stable_names = stable_only
            .get_available_tool_definitions()
            .into_iter()
            .map(|d| d.name)
            .collect::<Vec<_>>();
        assert!(stable_names.contains(&"echo_tool".to_string()));
        assert!(!stable_names.contains(&"competitive_analysis".to_string()));

        let with_experimental = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: true,
        });
        let all_names = with_experimental
            .get_available_tool_definitions()
            .into_iter()
            .map(|d| d.name)
            .collect::<Vec<_>>();
        assert!(all_names.contains(&"competitive_analysis".to_string()));
    }

    #[tokio::test]
    async fn test_tool_e2e_echo_tool_produces_actionable_artifact() {
        let registry = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: false,
        });
        let output = registry
            .execute_tool(
                "echo_tool",
                json!({"message":"hello operations","uppercase":true}),
            )
            .await
            .expect("echo_tool should execute");
        assert_eq!(output["echoed_message"], "HELLO OPERATIONS");
        assert_eq!(output["original_message"], "hello operations");
    }

    #[tokio::test]
    async fn test_tool_e2e_seo_analyzer_produces_actionable_artifact() {
        let registry = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: false,
        });
        let output = registry
            .execute_tool(
                "seo_analyzer",
                json!({
                    "text":"Nature's Diet supports healthy digestion and active dogs.",
                    "keywords":["Nature's Diet","healthy","dogs"]
                }),
            )
            .await
            .expect("seo_analyzer should execute");
        assert!(output["word_count"].as_u64().unwrap_or(0) > 0);
        assert!(output["avg_words_per_sentence"].is_number());
        let keyword_density = output
            .get("keyword_density_pct")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert!(!keyword_density.is_empty());
    }

    #[tokio::test]
    async fn test_stable_defaults_execute_non_empty_artifact() {
        let registry = ToolRegistry::with_config(ToolRegistryConfig {
            include_experimental: false,
        });
        for def in registry.get_available_tool_definitions() {
            if def.maturity != ToolMaturity::Stable {
                continue;
            }
            let output = registry
                .execute_tool(&def.name, def.default_input_template.clone())
                .await
                .expect("stable default template should execute");
            let non_empty = output
                .as_object()
                .map(|obj| !obj.is_empty())
                .unwrap_or(!output.is_null());
            assert!(
                non_empty,
                "stable tool '{}' returned empty output",
                def.name
            );
        }
    }
}
