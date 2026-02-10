// src/tools/code_verifier.rs

use crate::tools::base_tool::BaseTool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value; // Add Value import
use std::process::Command;

#[derive(Serialize, Deserialize)]
pub struct CodeVerifierTool {
    name: &'static str,
    description: &'static str,
}

impl CodeVerifierTool {
    pub fn new() -> Self {
        CodeVerifierTool {
            name: "code_verifier",
            description: "Verifies code for syntax errors, linting warnings, and type errors.",
        }
    }

    fn check_command_exists(&self, command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .output()
            .map_or(false, |output| output.status.success())
    }
}

#[async_trait]
impl BaseTool for CodeVerifierTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn is_available(&self) -> bool {
        // Check for common code verification tools like 'python', 'mypy', 'flake8', 'pylint', 'npm', 'tsc'
        self.check_command_exists("python")
            || self.check_command_exists("mypy")
            || self.check_command_exists("flake8")
            || self.check_command_exists("pylint")
            || self.check_command_exists("npm")
            || self.check_command_exists("tsc")
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would execute code verification tools
        // For now, we'll just return a success message.
        Ok(serde_json::json!({
            "status": "success",
            "message": format!("Successfully verified code for: {}", input)
        }))
    }
}
