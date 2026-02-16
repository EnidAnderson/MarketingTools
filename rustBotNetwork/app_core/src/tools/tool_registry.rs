use super::tool_definition::{Tool, ToolDefinition}; // Import the new Tool trait and ToolDefinition
use super::echo_tool::EchoTool; // Added EchoTool import
use serde_json::Value; // Keep serde_json::Value for tool execution arguments/results
use std::collections::HashMap;
use async_trait::async_trait; // Added for Tool trait
use std::error::Error; // Added for Tool trait

/// Manages a registry of tools, allowing them to be registered, retrieved, and their descriptions queried.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>, // Use Box<dyn Tool>
}

impl ToolRegistry {
    /// Creates a new, empty `ToolRegistry`.
    pub fn new() -> Self {
        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };
        registry.register_tool(Box::new(EchoTool::new())); // Register EchoTool
        registry
    }

    /// Registers a tool with the registry.
    /// The tool is moved into a `Box` to allow for dynamic dispatch.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) { // Accept Box<dyn Tool>
        self.tools.insert(tool.name(), tool);
    }

    /// Returns an instantiated tool if available.
    /// Returns `None` if not found.
    pub fn get_tool_instance(&self, tool_name: &str) -> Option<&dyn Tool> { // Return Option<&dyn Tool>
        self.tools.get(tool_name).map(|tool| tool.as_ref())
    }

    /// Returns a list of `ToolDefinition` for all registered tools.
    pub fn get_available_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|tool| tool.metadata()) // Directly use tool.metadata()
            .collect()
    }
}
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}