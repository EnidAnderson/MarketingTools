use super::base_tool::BaseTool;
use super::code_verifier::CodeVerifierTool;
use super::competitive_analysis::CompetitiveAnalysisTool;
use super::css_analyzer::CssAnalyzerTool; // New Import
use super::data_validator::DataValidatorTool;
use super::data_viz::DataVizTool;
use super::html_bundler::HtmlBundlerTool; // New Import
use super::product_crawler::ProductCrawlerTool;
use super::seo_analyzer::SEOAnalyzerTool;
use super::tool_definition::{ParameterDefinition, ToolComplexity, ToolDefinition, ToolUIMetadata}; // Import ToolDefinition
use serde_json::json;
use std::collections::HashMap;

/// Manages a registry of tools, allowing them to be registered, retrieved, and their descriptions queried.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn BaseTool>>,
}

impl ToolRegistry {
    /// Creates a new, empty `ToolRegistry`.
    pub fn new() -> Self {
        let mut registry = ToolRegistry {
            tools: HashMap::new(),
        };
        registry.register_tool(Box::new(SEOAnalyzerTool::new()));
        registry.register_tool(Box::new(CompetitiveAnalysisTool::new()));
        registry.register_tool(Box::new(DataValidatorTool::new()));
        registry.register_tool(Box::new(DataVizTool::new()));
        registry.register_tool(Box::new(CodeVerifierTool::new()));
        registry.register_tool(Box::new(CssAnalyzerTool::new())); // New Tool
        registry.register_tool(Box::new(HtmlBundlerTool::new())); // New Tool
        registry.register_tool(Box::new(ProductCrawlerTool::new()));
        registry
    }

    /// Registers a tool with the registry.
    /// The tool is moved into a `Box` to allow for dynamic dispatch.
    pub fn register_tool(&mut self, tool: Box<dyn BaseTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Returns an instantiated tool if available and its `is_available()` method returns True.
    /// Returns `None` if not found or not available.
    pub fn get_tool_instance(&self, tool_name: &str) -> Option<&dyn BaseTool> {
        self.tools
            .get(tool_name)
            .filter(|tool| tool.is_available())
            .map(|tool| tool.as_ref())
    }

    /// Returns a list of `ToolDefinition` for all available tools.
    pub fn get_available_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .filter(|tool| tool.is_available())
            .map(|tool| {
                let mut parameters = Vec::new();
                match tool.name() {
                    "seo_analyzer" => {
                        parameters.push(ParameterDefinition {
                            name: "text".to_string(),
                            r#type: "string".to_string(),
                            description: "The text content to analyze for SEO.".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "keywords".to_string(),
                            r#type: "array".to_string(),
                            description: "An array of keywords to analyze density for.".to_string(),
                            optional: true,
                        });
                    },
                    "competitive_analysis" => {
                        parameters.push(ParameterDefinition {
                            name: "topic".to_string(),
                            r#type: "string".to_string(),
                            description: "The topic for competitive analysis.".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "max_sources".to_string(),
                            r#type: "number".to_string(),
                            description:
                                "Maximum number of sources to analyze (defaults to 8, clamped to 3-20)."
                                    .to_string(),
                            optional: true,
                        });
                    },
                    "data_validator" => {
                        parameters.push(ParameterDefinition {
                            name: "data".to_string(),
                            r#type: "json".to_string(),
                            description: "The data to validate (JSON string).".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "schema".to_string(),
                            r#type: "json".to_string(),
                            description: "The JSON schema to validate against (JSON string).".to_string(),
                            optional: false,
                        });
                    },
                    "data_viz" => {
                        parameters.push(ParameterDefinition {
                            name: "data".to_string(),
                            r#type: "json".to_string(),
                            description: "The data for visualization (JSON string).".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "chart_type".to_string(),
                            r#type: "string".to_string(),
                            description: "The type of chart to generate (e.g., 'bar', 'line', 'pie').".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "output_path".to_string(),
                            r#type: "string".to_string(),
                            description: "The path to save the generated visualization image.".to_string(),
                            optional: false,
                        });
                    },
                    "code_verifier" => {
                        parameters.push(ParameterDefinition {
                            name: "code".to_string(),
                            r#type: "string".to_string(),
                            description: "The code to verify.".to_string(),
                            optional: false,
                        });
                        parameters.push(ParameterDefinition {
                            name: "language".to_string(),
                            r#type: "string".to_string(),
                            description: "The programming language of the code (e.g., 'python', 'javascript').".to_string(),
                            optional: false,
                        });
                    },
                    "css_analyzer" => {
                        parameters.push(ParameterDefinition {
                            name: "url".to_string(),
                            r#type: "string".to_string(),
                            description: "The URL of the webpage to analyze CSS from.".to_string(),
                            optional: false,
                        });
                    },
                    "html_bundler" => {
                        parameters.push(ParameterDefinition {
                            name: "path".to_string(),
                            r#type: "string".to_string(),
                            description: "The file path to the HTML file to bundle.".to_string(),
                            optional: false,
                        });
                    }
                    "product_crawler" => {
                        parameters.push(ParameterDefinition {
                            name: "base_url".to_string(),
                            r#type: "string".to_string(),
                            description: "Base URL to crawl for products.".to_string(),
                            optional: true,
                        });
                        parameters.push(ParameterDefinition {
                            name: "max_products".to_string(),
                            r#type: "number".to_string(),
                            description: "Maximum number of product links to return (1-100)."
                                .to_string(),
                            optional: true,
                        });
                    }
                    _ => {
                        // Default parameters or no specific parameters
                        parameters.push(ParameterDefinition {
                            name: "input".to_string(),
                            r#type: "string".to_string(),
                            description: "Generic input for the tool.".to_string(),
                            optional: false,
                        });
                    }
                }

                ToolDefinition {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    ui_metadata: ui_metadata_for(tool.name(), tool.description()),
                    parameters,
                    input_examples: input_examples_for(tool.name()),
                    output_schema: output_schema_for(tool.name()),
                }
            })
            .collect()
    }
}
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn ui_metadata_for(name: &str, description: &str) -> ToolUIMetadata {
    match name {
        "seo_analyzer" => ToolUIMetadata {
            category: "Research".to_string(),
            display_name: "SEO Analyzer".to_string(),
            icon: Some("search".to_string()),
            complexity: ToolComplexity::Simple,
            estimated_time_seconds: 10,
            tags: vec![
                "seo".to_string(),
                "copy".to_string(),
                "analysis".to_string(),
            ],
        },
        "product_crawler" => ToolUIMetadata {
            category: "Research".to_string(),
            display_name: "Product Crawler".to_string(),
            icon: Some("scan".to_string()),
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 40,
            tags: vec![
                "crawler".to_string(),
                "competitor".to_string(),
                "products".to_string(),
            ],
        },
        "competitive_analysis" => ToolUIMetadata {
            category: "Research".to_string(),
            display_name: "Competitive Analysis".to_string(),
            icon: Some("bar-chart-3".to_string()),
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 25,
            tags: vec![
                "market".to_string(),
                "competitor".to_string(),
                "signals".to_string(),
            ],
        },
        "css_analyzer" => ToolUIMetadata {
            category: "Engineering".to_string(),
            display_name: "CSS Analyzer".to_string(),
            icon: Some("code".to_string()),
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 15,
            tags: vec!["frontend".to_string(), "analysis".to_string()],
        },
        "html_bundler" => ToolUIMetadata {
            category: "Engineering".to_string(),
            display_name: "HTML Bundler".to_string(),
            icon: Some("package".to_string()),
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 15,
            tags: vec!["frontend".to_string(), "bundling".to_string()],
        },
        _ => ToolUIMetadata {
            display_name: description.to_string(),
            ..ToolUIMetadata::default()
        },
    }
}

fn input_examples_for(name: &str) -> Vec<serde_json::Value> {
    match name {
        "seo_analyzer" => vec![json!({
            "text": "Nature's Diet supports vibrant digestion with clean ingredients.",
            "keywords": ["Nature's Diet", "digestion", "clean ingredients"]
        })],
        "competitive_analysis" => vec![
            json!({
                "topic": "freeze dried raw dog food for sensitive stomachs",
                "max_sources": 8
            }),
            json!({
                "topic": "best affordable raw cat food subscription"
            })
        ],
        "product_crawler" => vec![json!({
            "base_url": "https://naturesdietpet.com",
            "max_products": 12
        })],
        "css_analyzer" => vec![json!({
            "url": "https://naturesdietpet.com"
        })],
        "html_bundler" => vec![json!({
            "path": "/path/to/input.html"
        })],
        _ => vec![],
    }
}

fn output_schema_for(name: &str) -> Option<serde_json::Value> {
    match name {
        "seo_analyzer" => Some(json!({
            "type": "object",
            "properties": {
                "word_count": {"type": "number"},
                "sentence_count": {"type": "number"},
                "avg_words_per_sentence": {"type": "number"},
                "keyword_density_pct": {"type": "object"}
            }
        })),
        "product_crawler" => Some(json!({
            "type": "object",
            "properties": {
                "base_url": {"type": "string"},
                "products": {"type": "array"},
                "competitive_insights": {"type": "array"},
                "suggested_ad_angles": {"type": "array"}
            }
        })),
        "competitive_analysis" => Some(json!({
            "type": "object",
            "properties": {
                "topic": {"type": "string"},
                "source_count": {"type": "number"},
                "sources": {"type": "array"},
                "keyword_frequency": {"type": "object"},
                "recurring_phrases": {"type": "array"},
                "signals": {"type": "array"},
                "inferred_notes": {"type": "array"},
                "signal_report_markdown": {"type": "string"}
            },
            "required": [
                "topic",
                "source_count",
                "sources",
                "keyword_frequency",
                "recurring_phrases",
                "signals",
                "inferred_notes",
                "signal_report_markdown"
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::error::Error; // Import Error trait for Box<dyn Error> // Import async_trait

    // Mock BaseTool implementation for testing purposes
    struct MockToolAvailable {
        name_val: &'static str,
        desc_val: &'static str,
        available: bool,
    }

    #[async_trait] // Added this
    impl BaseTool for MockToolAvailable {
        fn name(&self) -> &'static str {
            self.name_val
        }
        fn description(&self) -> &'static str {
            self.desc_val
        }
        fn is_available(&self) -> bool {
            self.available
        }
        async fn run(&self, _input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
            // Added async
            Ok(serde_json::json!({"status": "mocked_success"}))
        }
    }

    struct MockToolUnavailable {
        name_val: &'static str,
        desc_val: &'static str,
        available: bool,
    }

    #[async_trait] // Added this
    impl BaseTool for MockToolUnavailable {
        fn name(&self) -> &'static str {
            self.name_val
        }
        fn description(&self) -> &'static str {
            self.desc_val
        }
        fn is_available(&self) -> bool {
            self.available
        }
        async fn run(&self, _input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
            // Added async
            Ok(serde_json::json!({"status": "mocked_failure_unavailable"}))
        }
    }

    #[test]
    fn test_register_tool() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockToolAvailable {
            name_val: "TestTool",
            desc_val: "A test tool.",
            available: true,
        });
        registry.register_tool(tool);
        assert!(registry.tools.contains_key("TestTool"));
    }

    #[test]
    fn test_get_tool_instance_available() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockToolAvailable {
            name_val: "AvailableTool",
            desc_val: "An available tool.",
            available: true,
        });
        registry.register_tool(tool);

        let retrieved = registry.get_tool_instance("AvailableTool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "AvailableTool");
    }

    #[test]
    fn test_get_tool_instance_unavailable() {
        let mut registry = ToolRegistry::new();
        let tool = Box::new(MockToolUnavailable {
            name_val: "UnavailableTool",
            desc_val: "An unavailable tool.",
            available: false,
        });
        registry.register_tool(tool);

        let retrieved = registry.get_tool_instance("UnavailableTool");
        assert!(retrieved.is_none()); // Should be None because is_available is false
    }

    #[test]
    fn test_get_tool_instance_not_found() {
        let registry = ToolRegistry::new();
        let retrieved = registry.get_tool_instance("NonExistentTool");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_available_tool_descriptions() {
        let mut registry = ToolRegistry::new();
        let tool1 = Box::new(MockToolAvailable {
            name_val: "Tool1",
            desc_val: "Description 1.",
            available: true,
        });
        let tool2 = Box::new(MockToolUnavailable {
            name_val: "Tool2",
            desc_val: "Description 2.",
            available: false,
        });
        let tool3 = Box::new(MockToolAvailable {
            name_val: "Tool3",
            desc_val: "Description 3.",
            available: true,
        });

        registry.register_tool(tool1);
        registry.register_tool(tool2);
        registry.register_tool(tool3);

        let descriptions = registry.get_available_tool_definitions(); // Changed name here
        assert!(descriptions.len() >= 2); // Built-in tools may also be present

        let names: Vec<String> = descriptions.iter().map(|d| d.name.clone()).collect(); // Updated to access name field directly
        assert!(names.contains(&"Tool1".to_string()));
        assert!(names.contains(&"Tool3".to_string()));
        assert!(!names.contains(&"Tool2".to_string()));

        // Verify content for one tool
        let tool1_desc = descriptions.iter().find(|&d| d.name == "Tool1").unwrap(); // Updated to access name field directly
        assert_eq!(tool1_desc.description, "Description 1."); // Updated to access description field directly
                                                              // assert_eq!(tool1_desc["input_schema"]["type"], "object"); // This line was removed, as it's no longer relevant
    }
}
