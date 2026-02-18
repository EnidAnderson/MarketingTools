// rustBotNetwork/app_core/src/tools/echo_tool.rs

use crate::contracts::{ToolErrorCategory, ToolErrorEnvelope, ToolErrorSource};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::tool_definition::{
    ParameterDefinition, ToolComplexity, ToolDefinition, ToolMaturity, ToolRuntime, ToolUIMetadata,
};

pub struct EchoTool;

impl EchoTool {
    pub fn new() -> Self {
        EchoTool
    }
}

#[async_trait]
impl ToolRuntime for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "echo_tool".to_string(),
            description: "Echoes input text; used as a deterministic execution healthcheck."
                .to_string(),
            maturity: ToolMaturity::Stable,
            human_workflow:
                "Review echoed_message; if it matches intent, proceed to the next campaign step."
                    .to_string(),
            output_artifact_kind: "utility.echo_result.v1".to_string(),
            requires_review: false,
            default_input_template: json!({
                "message": "Nature's Diet workflow smoke test",
                "uppercase": false
            }),
            ui_metadata: ToolUIMetadata {
                category: "Utility".to_string(),
                display_name: "Echo Tool".to_string(),
                icon: Some("message".to_string()),
                complexity: ToolComplexity::Simple,
                estimated_time_seconds: 1,
                tags: vec!["utility".to_string(), "test".to_string()],
            },
            parameters: vec![
                ParameterDefinition {
                    name: "message".to_string(),
                    r#type: "string".to_string(),
                    description: "The message to echo back.".to_string(),
                    optional: false,
                },
                ParameterDefinition {
                    name: "uppercase".to_string(),
                    r#type: "boolean".to_string(),
                    description: "If true, the echoed message will be in uppercase.".to_string(),
                    optional: true,
                },
            ],
            input_examples: vec![
                json!({"message": "hello world"}),
                json!({"message": "test message", "uppercase": true}),
            ],
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "echoed_message": {"type": "string"},
                    "original_message": {"type": "string"},
                },
                "required": ["echoed_message", "original_message"]
            })),
        }
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolErrorEnvelope> {
        let Some(message) = args.get("message").and_then(Value::as_str) else {
            return Err(ToolErrorEnvelope::new(
                "missing_required_field",
                ToolErrorCategory::Validation,
                ToolErrorSource::Tool,
                "echo_tool requires a non-empty string field 'message'",
                false,
            )
            .with_field_paths(vec!["/message".to_string()]));
        };
        if message.trim().is_empty() {
            return Err(ToolErrorEnvelope::new(
                "invalid_argument",
                ToolErrorCategory::Validation,
                ToolErrorSource::Tool,
                "echo_tool requires 'message' to be non-empty",
                false,
            )
            .with_field_paths(vec!["/message".to_string()]));
        }

        let uppercase = args["uppercase"].as_bool().unwrap_or(false);

        let echoed_message = if uppercase {
            message.to_uppercase()
        } else {
            message.to_string()
        };

        Ok(json!({
            "echoed_message": echoed_message,
            "original_message": message,
        }))
    }
}
