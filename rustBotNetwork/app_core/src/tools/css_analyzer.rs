use super::base_tool::BaseTool;
use async_trait::async_trait;
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use url::Url;

// Internal synchronous helper function to parse HTML and extract CSS info
fn _parse_css_from_html(
    html_content: &str,
    base_url: &Url,
) -> Result<(Vec<String>, Vec<String>), Box<dyn Error + Send + Sync>> {
    let document = Html::parse_document(html_content);
    let mut inline_css = Vec::new();
    let mut external_css_urls = Vec::new();

    // Extract inline <style> tags
    let style_selector = Selector::parse("style").unwrap();
    for style_tag in document.select(&style_selector) {
        inline_css.push(style_tag.inner_html());
    }

    // Collect external stylesheet URLs
    let link_selector = Selector::parse("link[rel='stylesheet']").unwrap();
    for link_tag in document.select(&link_selector) {
        if let Some(href) = link_tag.value().attr("href") {
            let css_url = base_url.join(href)?;
            external_css_urls.push(css_url.to_string());
        }
    }
    Ok((inline_css, external_css_urls))
}

// Internal asynchronous helper function
async fn _get_all_css_impl(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    println!("Analyzing CSS for {}...", url);

    let client = reqwest::Client::new();
    let response_text = client.get(url).send().await?.text().await?;
    let base_url = Url::parse(url)?;

    let (mut combined_css, external_css_urls) = _parse_css_from_html(&response_text, &base_url)?;

    // Now fetch external CSS asynchronously
    for css_url_str in external_css_urls {
        let css_response = client.get(&css_url_str).send().await?.text().await?;
        combined_css.push(css_response);
    }

    // Clean up and combine
    let final_css = combined_css.join("\n");
    let comment_regex = Regex::new(r"/\*.*?\*/").unwrap();
    let whitespace_regex = Regex::new(r"\s{2,}").unwrap();
    let final_css = comment_regex.replace_all(&final_css, "").to_string();
    let final_css = whitespace_regex.replace_all(&final_css, " ").to_string();
    let final_css = final_css
        .replace("; ", ";")
        .replace(" {", "{")
        .replace(" }", "}")
        .replace(";}", "}");

    println!(
        "Finished CSS analysis for {}. Total size: {} characters.",
        url,
        final_css.len()
    );
    Ok(final_css)
}

pub struct CssAnalyzerTool;

impl CssAnalyzerTool {
    pub fn new() -> Self {
        CssAnalyzerTool
    }
}

#[async_trait]
impl BaseTool for CssAnalyzerTool {
    fn name(&self) -> &'static str {
        "css_analyzer"
    }

    fn description(&self) -> &'static str {
        "Analyzes CSS from a given URL, extracting inline and external stylesheets."
    }

    fn is_available(&self) -> bool {
        true // This is a conceptual tool, always available.
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let url = input["url"]
            .as_str()
            .ok_or("URL is required for css_analyzer")?;

        match _get_all_css_impl(url).await {
            Ok(css_content) => Ok(serde_json::json!({
                "status": "success",
                "url": url,
                "css_content": css_content,
                "length": css_content.len()
            })),
            Err(e) => Err(e), // Error is already boxed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_css_analyzer_tool_success() {
        // Mock a simple HTTP server or use a known small, reliable page for a real integration test
        // For a unit test, we might want to mock `reqwest::Client`
        // For now, let's use a real URL for a quick test if connectivity is available
        // Note: This makes the test an integration test rather than a pure unit test.
        let tool = CssAnalyzerTool::new();
        let input = json!({"url": "https://example.com"});

        let result = tool.run(input).await.unwrap();

        assert_eq!(result["status"], "success");
        assert!(result["css_content"].as_str().unwrap().contains("body")); // Example.com has some basic CSS
        assert!(result["length"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_css_analyzer_tool_missing_url() {
        let tool = CssAnalyzerTool::new();
        let input = json!({});

        let err = tool.run(input).await.unwrap_err();
        assert!(err.to_string().contains("URL is required"));
    }

    #[tokio::test]
    async fn test_css_analyzer_tool_invalid_url() {
        let tool = CssAnalyzerTool::new();
        let input = json!({"url": "not-a-valid-url"});

        let err = tool.run(input).await.unwrap_err();
        assert!(err.to_string().contains("relative URL without a base"));
    }
}
