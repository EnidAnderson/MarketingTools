use crate::contracts::ToolErrorEnvelope;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub r#type: String, // `type` is a Rust keyword, so we use r#type
    pub description: String,
    pub optional: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolComplexity {
    Simple,
    Intermediate,
    Advanced,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolMaturity {
    Stable,
    Experimental,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolUIMetadata {
    pub category: String,
    pub display_name: String,
    pub icon: Option<String>,
    pub complexity: ToolComplexity,
    pub estimated_time_seconds: u32,
    pub tags: Vec<String>,
}

impl Default for ToolUIMetadata {
    fn default() -> Self {
        Self {
            category: "Uncategorized".to_string(),
            display_name: "".to_string(),
            icon: None,
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 30,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub maturity: ToolMaturity,
    pub human_workflow: String,
    pub output_artifact_kind: String,
    pub requires_review: bool,
    pub default_input_template: Value,
    pub ui_metadata: ToolUIMetadata,
    pub parameters: Vec<ParameterDefinition>,
    pub input_examples: Vec<Value>,
    pub output_schema: Option<Value>,
}

#[async_trait]
pub trait ToolRuntime: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    fn is_available(&self) -> bool;
    async fn execute(&self, args: Value) -> Result<Value, ToolErrorEnvelope>;
}
