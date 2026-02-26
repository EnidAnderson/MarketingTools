use super::base_tool::BaseTool;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

// Internal asynchronous helper function
async fn _bundle_html_impl(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    println!("Bundling HTML from {}...", path);

    let html_content = fs::read_to_string(path)?;

    // In a real scenario, this would parse HTML, find linked resources (CSS, JS, images),
    // fetch them, and embed them directly into the HTML.
    // For this mock, we'll just indicate a successful "bundling".
    let bundled_content = format!(
        "<!-- Bundled HTML (mock) from: {} -->\n<style>/* Inlined CSS */</style>\n<script>// Inlined JS</script>\n{}",
        path, html_content
    );

    println!(
        "Finished HTML bundling for {}. Total size: {} characters.",
        path,
        bundled_content.len()
    );
    Ok(bundled_content)
}

pub struct HtmlBundlerTool;

impl HtmlBundlerTool {
    pub fn new() -> Self {
        HtmlBundlerTool
    }
}

#[async_trait]
impl BaseTool for HtmlBundlerTool {
    fn name(&self) -> &'static str {
        "html_bundler"
    }

    fn description(&self) -> &'static str {
        "Bundles HTML and its linked resources into a single file."
    }

    fn is_available(&self) -> bool {
        true // This is a conceptual tool, always available.
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let path = input["path"]
            .as_str()
            .ok_or("Path is required for html_bundler")?;

        if !Path::new(path).exists() {
            return Err(Box::from(format!("Input HTML file not found: {}", path)));
        }

        match _bundle_html_impl(path).await {
            Ok(bundled_html) => Ok(serde_json::json!({
                "status": "success",
                "path": path,
                "bundled_html": bundled_html,
                "length": bundled_html.len()
            })),
            Err(e) => Err(e), // Error is already boxed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_html_bundler_tool_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.html");
        fs::write(&file_path, "<html><body>Hello</body></html>").unwrap();

        let tool = HtmlBundlerTool::new();
        let input = json!({"path": file_path.to_str().unwrap()});

        let result = tool.run(input).await.unwrap();

        assert_eq!(result["status"], "success");
        assert!(result["bundled_html"]
            .as_str()
            .unwrap()
            .contains("<html><body>Hello</body></html>"));
        assert!(result["length"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_html_bundler_tool_missing_path() {
        let tool = HtmlBundlerTool::new();
        let input = json!({});

        let err = tool.run(input).await.unwrap_err();
        assert!(err.to_string().contains("Path is required"));
    }

    #[tokio::test]
    async fn test_html_bundler_tool_file_not_found() {
        let tool = HtmlBundlerTool::new();
        let input = json!({"path": "/non/existent/file.html"});

        let err = tool.run(input).await.unwrap_err();
        assert!(err.to_string().contains("Input HTML file not found"));
    }
}
