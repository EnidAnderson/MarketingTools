// rustBotNetwork/app_core/src/tools/echo_tool.rs

use async_trait::async_trait;
use serde_json::{json, Value};
use std::error::Error;

use super::tool_definition::{
    ParameterDefinition, Tool, ToolComplexity, ToolDefinition, ToolUIMetadata,
};

pub struct EchoTool;

impl EchoTool {
    pub fn new() -> Self {
        EchoTool
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String {
        "echo_tool".to_string()
    }

    fn description(&self) -> String {
        "An example tool that echoes back the input it receives.".to_string()
    }

    fn metadata(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: self.description(),
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

    async fn execute(&self, args: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let message = args["message"].as_str().unwrap_or_default();
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
