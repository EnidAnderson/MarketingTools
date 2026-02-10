use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub r#type: String, // `type` is a Rust keyword, so we use r#type
    pub description: String,
    pub optional: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolComplexity {
    Simple,
    Intermediate,
    Advanced,
}

#[derive(Debug, Serialize, Clone)]
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

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub ui_metadata: ToolUIMetadata,
    pub parameters: Vec<ParameterDefinition>,
    pub input_examples: Vec<Value>,
    pub output_schema: Option<Value>,
}
