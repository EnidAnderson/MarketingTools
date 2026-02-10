use crate::contracts::{ToolContract, ToolError, ToolResult, TypedTool};
use crate::tools::base_tool::BaseTool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoAnalyzerInput {
    pub text: String,
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoAnalyzerOutput {
    pub word_count: usize,
    pub sentence_count: usize,
    pub avg_words_per_sentence: f64,
    pub keyword_density_pct: HashMap<String, f64>,
}

pub struct SeoAnalyzerContract;

impl ToolContract for SeoAnalyzerContract {
    const NAME: &'static str = "seo_analyzer";
    const VERSION: &'static str = "2.0.0";
    type Input = SeoAnalyzerInput;
    type Output = SeoAnalyzerOutput;
}

#[derive(Serialize, Deserialize)]
pub struct SEOAnalyzerTool {
    name: &'static str,
    description: &'static str,
}

impl SEOAnalyzerTool {
    pub fn new() -> Self {
        SEOAnalyzerTool {
            name: "seo_analyzer",
            description: "Analyzes text for SEO metrics like keyword density and readability.",
        }
    }
}

#[async_trait]
impl TypedTool<SeoAnalyzerContract> for SEOAnalyzerTool {
    async fn run_typed(&self, input: SeoAnalyzerInput) -> ToolResult<SeoAnalyzerOutput> {
        if input.text.trim().is_empty() {
            return Err(ToolError::validation("'text' cannot be empty"));
        }

        let word_count = count_words(&input.text);
        let sentence_count = count_sentences(&input.text).max(1);
        let avg_words_per_sentence = word_count as f64 / sentence_count as f64;

        let keyword_density_pct = input
            .keywords
            .unwrap_or_default()
            .into_iter()
            .map(|keyword| {
                let count = count_keyword_occurrences(&input.text, &keyword);
                let density = if word_count == 0 {
                    0.0
                } else {
                    (count as f64 / word_count as f64) * 100.0
                };
                (keyword, density)
            })
            .collect::<HashMap<_, _>>();

        Ok(SeoAnalyzerOutput {
            word_count,
            sentence_count,
            avg_words_per_sentence,
            keyword_density_pct,
        })
    }
}

#[async_trait]
impl BaseTool for SEOAnalyzerTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let typed_input: SeoAnalyzerInput = serde_json::from_value(input).map_err(|e| {
            Box::new(ToolError::validation(e.to_string()))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

        let output = self
            .run_typed(typed_input)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        serde_json::to_value(output).map_err(|e| {
            Box::new(ToolError::internal(e.to_string())) as Box<dyn std::error::Error + Send + Sync>
        })
    }
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn count_sentences(text: &str) -> usize {
    text.matches(['.', '!', '?']).count()
}

fn count_keyword_occurrences(text: &str, keyword: &str) -> usize {
    let needle = keyword.trim().to_lowercase();
    if needle.is_empty() {
        return 0;
    }

    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w == &needle)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn run_typed_returns_metrics() {
        let tool = SEOAnalyzerTool::new();
        let output = tool
            .run_typed(SeoAnalyzerInput {
                text: "Healthy raw food supports dog digestion. Healthy choices matter."
                    .to_string(),
                keywords: Some(vec!["Healthy".to_string(), "dog".to_string()]),
            })
            .await
            .expect("should succeed");

        assert!(output.word_count > 0);
        assert!(output.keyword_density_pct.contains_key("Healthy"));
    }

    #[tokio::test]
    async fn legacy_run_accepts_json_contract() {
        let tool = SEOAnalyzerTool::new();
        let output = tool
            .run(json!({
                "text": "Nature's Diet supports vibrant pet health.",
                "keywords": ["Nature's", "health"]
            }))
            .await
            .expect("should serialize output");

        assert!(output.get("word_count").is_some());
    }
}
