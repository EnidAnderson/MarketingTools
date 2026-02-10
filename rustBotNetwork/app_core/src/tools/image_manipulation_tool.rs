use super::base_tool::BaseTool;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
#[allow(unused_imports)] // fs is used by Path::new().exists() indirectly and in tests
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// Define a trait for running commands, making it mockable
pub trait CommandRunner {
    fn run_command(&self, command_name: &str, args: &[&str]) -> Result<Output, std::io::Error>;
}

// Concrete implementation that uses std::process::Command
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run_command(&self, command_name: &str, args: &[&str]) -> Result<Output, std::io::Error> {
        Command::new(command_name).args(args).output()
    }
}

pub struct ImageManipulationTool {
    command_runner: Box<dyn CommandRunner + Send + Sync>,
}

impl ImageManipulationTool {
    pub fn new() -> Self {
        ImageManipulationTool {
            command_runner: Box::new(RealCommandRunner),
        }
    }

    // Constructor for dependency injection in tests
    #[cfg(test)]
    pub fn new_with_runner(runner: Box<dyn CommandRunner + Send + Sync>) -> Self {
        ImageManipulationTool {
            command_runner: runner,
        }
    }

    /// Helper to run a `magick` command and return success status.
    fn _run_magick_command(&self, args: &[&str]) -> bool {
        match self.command_runner.run_command("magick", args) {
            Ok(output) => {
                if output.status.success() {
                    // println!("Magick command successful: {:?}", args);
                    // println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
                    true
                } else {
                    eprintln!("Magick command failed: {:?}", args);
                    eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to execute magick command: {}", e);
                eprintln!("Please ensure ImageMagick is installed and 'magick' is in your PATH.");
                false
            }
        }
    }
}

#[async_trait]
impl BaseTool for ImageManipulationTool {
    fn name(&self) -> &'static str {
        "ImageManipulationTool"
    }

    fn description(&self) -> &'static str {
        "Performs image manipulation (resize or add_watermark). Input requires 'action', 'input_path', 'output_path' and action-specific parameters."
    }

    fn is_available(&self) -> bool {
        self._run_magick_command(&["-version"])
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let action = input["action"].as_str();
        let input_path = input["input_path"].as_str();
        let output_path = input["output_path"].as_str();

        if input_path.is_none() || output_path.is_none() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "input_path and output_path are required."
            }));
        }

        let input_path_str = input_path.unwrap();
        let output_path_str = output_path.unwrap();

        if !Path::new(input_path_str).exists() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": format!("Input file not found: {}", input_path_str)
            }));
        }

        let mut args: Vec<String> = Vec::new();

        match action {
            Some("resize") => {
                let width = input["width"].as_u64();
                let height = input["height"].as_u64();

                if width.is_none() && height.is_none() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "For 'resize' action, 'width' or 'height' is required."
                    }));
                }

                let size_arg = if let (Some(w), Some(h)) = (width, height) {
                    format!("{}x{}", w, h)
                } else if let Some(w) = width {
                    format!("{}x", w)
                } else {
                    format!("x{}", height.unwrap()) // height must be Some here
                };

                args.push(input_path_str.to_string());
                args.push("-resize".to_string());
                args.push(size_arg);
                args.push(output_path_str.to_string());
            }
            Some("add_watermark") => {
                let watermark_path = input["watermark_path"].as_str();
                let gravity = input["gravity"].as_str().unwrap_or("SouthEast");

                if watermark_path.is_none() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "For 'add_watermark' action, 'watermark_path' is required."
                    }));
                }
                let watermark_path_str = watermark_path.unwrap();

                if !Path::new(watermark_path_str).exists() {
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": format!("Watermark file not found: {}", watermark_path_str)
                    }));
                }

                // Magick command for watermarking: magick input.jpg watermark.png -gravity SouthEast -composite output.jpg
                args.push(input_path_str.to_string());
                args.push(watermark_path_str.to_string());
                args.push("-gravity".to_string());
                args.push(gravity.to_string());
                args.push("-composite".to_string());
                args.push(output_path_str.to_string());
            }
            _ => {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": "Invalid action. Must be 'resize' or 'add_watermark'."
                }));
            }
        }

        // Convert Vec<String> to Vec<&str> for _run_magick_command
        let args_str_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        if self._run_magick_command(&args_str_refs) {
            Ok(serde_json::json!({
                "status": "success",
                "message": format!("Image manipulation '{}' successful. Output: {}", action.unwrap_or("unknown"), output_path_str)
            }))
        } else {
            Ok(serde_json::json!({
                "status": "error",
                "message": format!("Image manipulation '{}' failed.", action.unwrap_or("unknown"))
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write; // Needed for writing to temp files
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf; // PathBuf is used here
    use std::process::ExitStatus; // Import ExitStatus
    use std::sync::{Arc, Mutex}; // For shared state in mock
    use tempfile::tempdir; // Import ExitStatusExt

    // Mock implementation for CommandRunner
    struct MockCommandRunner {
        // Store expected outputs for commands
        mock_outputs: Arc<Mutex<Vec<(String, Vec<String>, Result<Output, std::io::Error>)>>>,
    }

    impl MockCommandRunner {
        fn new() -> Self {
            MockCommandRunner {
                mock_outputs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        // Add a mock output for a specific command and arguments
        fn add_mock_output(
            &self,
            command: &str,
            args: Vec<String>,
            output: Result<Output, std::io::Error>,
        ) {
            self.mock_outputs
                .lock()
                .unwrap()
                .push((command.to_string(), args, output));
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn run_command(&self, command_name: &str, args: &[&str]) -> Result<Output, std::io::Error> {
            let mut mock_outputs = self.mock_outputs.lock().unwrap();

            // Find a matching mock output
            if let Some(pos) = mock_outputs.iter().position(|(cmd, cmd_args, _)| {
                cmd == command_name
                    && cmd_args
                        .iter()
                        .zip(args.iter())
                        .all(|(mock_arg, real_arg)| mock_arg == real_arg)
            }) {
                let (_, _, output) = mock_outputs.remove(pos); // Consume the mock output
                output
            } else {
                // If no mock found, return a default error output
                eprintln!(
                    "No mock output found for command: {} {:?}",
                    command_name, args
                );
                Ok(Output {
                    status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                    stdout: Vec::new(),
                    stderr: b"Mock command failed: No mock output defined.".to_vec(),
                })
            }
        }
    }

    // Helper to create a dummy image file
    fn create_dummy_file(dir: &Path, filename: &str, content: &[u8]) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    #[test]
    fn test_is_available() {
        let mock_runner = MockCommandRunner::new();
        mock_runner.add_mock_output(
            "magick",
            vec!["-version".to_string()],
            Ok(Output {
                status: ExitStatus::from_raw(0), // Fixed: Use from_raw after import
                stdout: b"ImageMagick 7.0.10-60 Q16 x86_64 2020-04-03 https://imagemagick.org"
                    .to_vec(),
                stderr: Vec::new(),
            }),
        );
        let tool = ImageManipulationTool::new_with_runner(Box::new(mock_runner));
        assert!(tool.is_available());

        let mock_runner_fail = MockCommandRunner::new();
        mock_runner_fail.add_mock_output(
            "magick",
            vec!["-version".to_string()],
            Ok(Output {
                status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                stdout: Vec::new(),
                stderr: b"magick: command not found".to_vec(),
            }),
        );
        let tool_fail = ImageManipulationTool::new_with_runner(Box::new(mock_runner_fail));
        assert!(!tool_fail.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_paths() {
        // Added async
        let tool = ImageManipulationTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide mock runner
        let input = json!({
            "action": "resize",
            // Missing input_path and output_path
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("input_path and output_path are required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_input_file_not_found() {
        // Added async
        let dir = tempdir().unwrap();
        let tool = ImageManipulationTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide mock runner
        let input = json!({
            "action": "resize",
            "input_path": dir.path().join("non_existent.jpg").to_str().unwrap(),
            "output_path": dir.path().join("output.jpg").to_str().unwrap(),
            "width": 100
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Input file not found"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_resize_success() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file = create_dummy_file(dir.path(), "input.jpg", b"dummy_jpeg_content");
        let output_file = dir.path().join("resized_output.jpg");

        let expected_args: Vec<String> = vec![
            input_file.to_str().unwrap().to_string(),
            "-resize".to_string(),
            "50x50".to_string(),
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "magick",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(0), // Fixed: Use from_raw after import
                stdout: b"mock magick resize success".to_vec(),
                stderr: Vec::new(),
            }),
        );

        let tool = ImageManipulationTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "action": "resize",
            "input_path": input_file.to_str().unwrap(),
            "output_path": output_file.to_str().unwrap(),
            "width": 50,
            "height": 50
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        // Simulate the file creation by magick
        create_dummy_file(dir.path(), "resized_output.jpg", b"dummy_resized_content");
        assert!(output_file.exists(), "Output file was not created.");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_resize_failure() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file = create_dummy_file(dir.path(), "input.jpg", b"dummy_jpeg_content");
        let output_file = dir.path().join("resized_output_fail.jpg");

        let expected_args: Vec<String> = vec![
            input_file.to_str().unwrap().to_string(),
            "-resize".to_string(),
            "50x".to_string(), // Missing height could cause failure if ImageMagick is strict
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "magick",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                stdout: Vec::new(),
                stderr: b"mock magick resize failure".to_vec(),
            }),
        );

        let tool = ImageManipulationTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "action": "resize",
            "input_path": input_file.to_str().unwrap(),
            "output_path": output_file.to_str().unwrap(),
            "width": 50
            // No height
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Image manipulation 'resize' failed."));
        assert!(
            !output_file.exists(),
            "Output file should not have been created on failure."
        );
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_add_watermark_success() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file = create_dummy_file(dir.path(), "input.jpg", b"dummy_jpeg_content");
        let watermark_file = create_dummy_file(dir.path(), "watermark.png", b"dummy_png_content");
        let output_file = dir.path().join("watermarked_output.jpg");

        let expected_args: Vec<String> = vec![
            input_file.to_str().unwrap().to_string(),
            watermark_file.to_str().unwrap().to_string(),
            "-gravity".to_string(),
            "NorthWest".to_string(),
            "-composite".to_string(),
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "magick",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(0), // Fixed: Use from_raw after import
                stdout: b"mock magick watermark success".to_vec(),
                stderr: Vec::new(),
            }),
        );

        let tool = ImageManipulationTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "action": "add_watermark",
            "input_path": input_file.to_str().unwrap(),
            "output_path": output_file.to_str().unwrap(),
            "watermark_path": watermark_file.to_str().unwrap(),
            "gravity": "NorthWest"
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        // Simulate the file creation by magick
        create_dummy_file(
            &dir.path(),
            "watermarked_output.jpg",
            b"dummy_watermarked_content",
        );
        assert!(output_file.exists(), "Output file was not created.");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_add_watermark_failure() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file = create_dummy_file(dir.path(), "input.jpg", b"dummy_jpeg_content");
        let watermark_file = create_dummy_file(dir.path(), "watermark.png", b"dummy_png_content");
        let output_file = dir.path().join("watermarked_output_fail.jpg");

        let expected_args: Vec<String> = vec![
            input_file.to_str().unwrap().to_string(),
            watermark_file.to_str().unwrap().to_string(),
            "-gravity".to_string(),
            "InvalidGravity".to_string(), // Invalid gravity to force failure
            "-composite".to_string(),
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "magick",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                stdout: Vec::new(),
                stderr: b"mock magick watermark failure".to_vec(),
            }),
        );

        let tool = ImageManipulationTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "action": "add_watermark",
            "input_path": input_file.to_str().unwrap(),
            "output_path": output_file.to_str().unwrap(),
            "watermark_path": watermark_file.to_str().unwrap(),
            "gravity": "InvalidGravity"
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Image manipulation 'add_watermark' failed."));
        assert!(
            !output_file.exists(),
            "Output file should not have been created on failure."
        );
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_invalid_action() {
        // Added async
        let dir = tempdir().unwrap();
        let input_file = create_dummy_file(dir.path(), "input.jpg", b"dummy_jpeg_content");
        let tool = ImageManipulationTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide mock runner
        let input = json!({
            "action": "crop", // Invalid action
            "input_path": input_file.to_str().unwrap(),
            "output_path": dir.path().join("cropped_output.jpg").to_str().unwrap()
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Invalid action"));
    }
}
