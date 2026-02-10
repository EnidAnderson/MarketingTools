use super::base_tool::BaseTool;
use serde_json::Value;
use std::error::Error;
// Removed tokio::runtime::Runtime as it's no longer needed for block_on
use async_trait::async_trait;
use playwright::api::Playwright as Pw; // Alias Playwright to Pw
use std::fs;
// Removed unused: use std::path::PathBuf;

// Define a simplified trait for Playwright operations, making it mockable
#[async_trait]
pub trait PlaywrightRunner: Send + Sync {
    async fn take_screenshot_from_url(
        &self,
        url: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>; // Added Send + Sync
}

// Concrete implementation that uses the actual Playwright library
pub struct RealPlaywrightRunner;

#[async_trait]
impl PlaywrightRunner for RealPlaywrightRunner {
    async fn take_screenshot_from_url(
        &self,
        url: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // Added Send + Sync
        let pw = Pw::initialize().await.map_err(|e| {
            eprintln!("Playwright initialization failed: {:?}", e);
            Box::<dyn Error + Send + Sync>::from(e) // Ensure error is Send + Sync
        })?;
        let chromium = pw.chromium();
        let browser = chromium.launcher().launch().await?;
        let context = browser.context_builder().build().await?;
        let page = context.new_page().await?;
        page.goto_builder(url).goto().await?;

        let screenshot_bytes = page
            .screenshot_builder()
            .full_page(true)
            .screenshot()
            .await?;

        browser.close().await?;
        Ok(screenshot_bytes)
    }
}

pub struct ScreenshotTool {
    playwright_runner: Box<dyn PlaywrightRunner>,
}

impl ScreenshotTool {
    pub fn new() -> Self {
        ScreenshotTool {
            playwright_runner: Box::new(RealPlaywrightRunner),
        }
    }

    // Constructor for dependency injection in tests
    #[cfg(test)]
    pub fn new_with_runner(runner: Box<dyn PlaywrightRunner>) -> Self {
        ScreenshotTool {
            playwright_runner: runner,
        }
    }
}

#[async_trait]
impl BaseTool for ScreenshotTool {
    fn name(&self) -> &'static str {
        "ScreenshotTool"
    }

    fn description(&self) -> &'static str {
        "Takes a screenshot of a given URL and saves it to a specified path."
    }

    fn is_available(&self) -> bool {
        // For now, we assume playwright is installed and ready.
        // A more robust check could verify browser installation.
        true
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let url = input["url"].as_str().ok_or("URL is required")?;
        let output_path = input["output_path"]
            .as_str()
            .ok_or("output_path is required")?;

        // Removed rt.block_on and just await directly
        let screenshot_bytes = self.playwright_runner.take_screenshot_from_url(url).await?;

        fs::write(output_path, screenshot_bytes)?;

        Ok(serde_json::json!({
            "status": "success",
            "path": output_path,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir; // Still needed for tempdir().path().join()

    // Mock implementation for PlaywrightRunner
    struct MockPlaywrightRunner {
        screenshot_result: Result<Vec<u8>, Box<dyn Error + Send + Sync>>, // Added Send + Sync
    }

    impl MockPlaywrightRunner {
        fn new(result: Result<Vec<u8>, Box<dyn Error + Send + Sync>>) -> Self {
            // Added Send + Sync
            MockPlaywrightRunner {
                screenshot_result: result,
            }
        }
    }

    #[async_trait]
    impl PlaywrightRunner for MockPlaywrightRunner {
        async fn take_screenshot_from_url(
            &self,
            _url: &str,
        ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
            // Added Send + Sync
            // We need to clone the error if it's an Err, as Box<dyn Error> is not Clone
            match &self.screenshot_result {
                Ok(bytes) => Ok(bytes.clone()),
                Err(e) => Err(format!("{}", e).into()), // Convert error to a new Box<dyn Error + Send + Sync>
            }
        }
    }

    // Base64 encoded 1x1 transparent PNG - as dummy screenshot data
    const DUMMY_PNG_BYTES: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[tokio::test] // Changed to tokio::test
    async fn test_take_screenshot_success() {
        // Added async
        let mock_runner = MockPlaywrightRunner::new(Ok(DUMMY_PNG_BYTES.to_vec()));
        let tool = ScreenshotTool::new_with_runner(Box::new(mock_runner));

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("screenshot.png");
        let output_path_str = output_path.to_str().unwrap();

        let input = json!({
            "url": "https://www.example.com",
            "output_path": output_path_str
        });

        let result = tool.run(input).await.unwrap(); // Added .await

        assert_eq!(result["status"], "success");
        assert_eq!(result["path"], output_path_str);
        assert!(fs::metadata(&output_path).is_ok()); // Borrow output_path
        assert_eq!(fs::read(&output_path).unwrap(), DUMMY_PNG_BYTES); // Borrow output_path
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_url() {
        // Added async
        let mock_runner = MockPlaywrightRunner::new(Ok(DUMMY_PNG_BYTES.to_vec()));
        let tool = ScreenshotTool::new_with_runner(Box::new(mock_runner));

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("screenshot.png");
        let output_path_str = output_path.to_str().unwrap();

        let input = json!({
            // "url": "https://www.example.com", // Missing URL
            "output_path": output_path_str
        });

        let result = tool.run(input).await.unwrap_err(); // Added .await
        assert!(result.to_string().contains("URL is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_output_path() {
        // Added async
        let mock_runner = MockPlaywrightRunner::new(Ok(DUMMY_PNG_BYTES.to_vec()));
        let tool = ScreenshotTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "url": "https://www.example.com"
            // "output_path": output_path_str // Missing output_path
        });

        let result = tool.run(input).await.unwrap_err(); // Added .await
        assert!(result.to_string().contains("output_path is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_playwright_failure() {
        // Added async
        let mock_runner = MockPlaywrightRunner::new(Err(Box::<dyn Error + Send + Sync>::from(
            "Playwright internal error",
        ))); // Added Send + Sync
        let tool = ScreenshotTool::new_with_runner(Box::new(mock_runner));

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("screenshot.png");
        let output_path_str = output_path.to_str().unwrap();

        let input = json!({
            "url": "https://www.example.com",
            "output_path": output_path_str
        });

        let result = tool.run(input).await.unwrap_err(); // Added .await
        assert!(result.to_string().contains("Playwright internal error"));
        assert!(!output_path.exists()); // No file should be written on Playwright failure
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_file_write_failure() {
        // Added async
        let mock_runner = MockPlaywrightRunner::new(Ok(DUMMY_PNG_BYTES.to_vec()));
        let tool = ScreenshotTool::new_with_runner(Box::new(mock_runner));

        // Use an invalid path to force fs::write to fail
        let output_path_str = "/non/existent/path/screenshot.png";

        let input = json!({
            "url": "https://www.example.com",
            "output_path": output_path_str
        });

        let result = tool.run(input).await.unwrap_err(); // Added .await
        assert!(result.to_string().contains("No such file or directory"));
    }
}
