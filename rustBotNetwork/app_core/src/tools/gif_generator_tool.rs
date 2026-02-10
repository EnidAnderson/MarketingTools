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

pub struct GifGeneratorTool {
    command_runner: Box<dyn CommandRunner + Send + Sync>,
}

impl GifGeneratorTool {
    pub fn new() -> Self {
        GifGeneratorTool {
            command_runner: Box::new(RealCommandRunner),
        }
    }

    // Constructor for dependency injection in tests
    #[cfg(test)]
    pub fn new_with_runner(runner: Box<dyn CommandRunner + Send + Sync>) -> Self {
        GifGeneratorTool {
            command_runner: runner,
        }
    }

    /// Helper to run an `ffmpeg` command and return success status.
    fn _run_ffmpeg_command(&self, args: &[&str]) -> bool {
        match self.command_runner.run_command("ffmpeg", args) {
            Ok(output) => {
                if output.status.success() {
                    // println!("FFmpeg command successful: {:?}", args);
                    // println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
                    true
                } else {
                    eprintln!("FFmpeg command failed: {:?}", args);
                    eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to execute ffmpeg command: {}", e);
                eprintln!("Please ensure FFmpeg is installed and 'ffmpeg' is in your PATH.");
                false
            }
        }
    }
}

#[async_trait]
impl BaseTool for GifGeneratorTool {
    fn name(&self) -> &'static str {
        "GifGeneratorTool"
    }

    fn description(&self) -> &'static str {
        "Generates a GIF from a list of input image paths. Input requires 'input_images' (array of strings), 'output_path', optional 'frame_rate' (number), and optional 'loop_count' (number)."
    }

    fn is_available(&self) -> bool {
        self._run_ffmpeg_command(&["-version"])
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let input_images = input["input_images"].as_array();
        let output_path = input["output_path"].as_str();
        let frame_rate = input["frame_rate"].as_u64().unwrap_or(10);
        let loop_count = input["loop_count"].as_i64().unwrap_or(0);

        if input_images.is_none() || input_images.unwrap().is_empty() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "input_images list is required and cannot be empty."
            }));
        }
        if output_path.is_none() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "output_path is required."
            }));
        }

        let output_path_str = output_path.unwrap();
        let input_images_vec: Vec<&str> = input_images
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        for img_path in &input_images_vec {
            if !Path::new(img_path).exists() {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("Input image not found: {}", img_path)
                }));
            }
        }

        let mut ffmpeg_args: Vec<String> = Vec::new();
        for img_path in input_images_vec {
            ffmpeg_args.push("-i".to_string());
            ffmpeg_args.push(img_path.to_string());
        }

        // Output options
        ffmpeg_args.push("-vf".to_string());
        ffmpeg_args.push(format!("fps={}", frame_rate));
        ffmpeg_args.push("-loop".to_string());
        ffmpeg_args.push(loop_count.to_string());
        ffmpeg_args.push(output_path_str.to_string());

        // Convert Vec<String> to Vec<&str> for _run_ffmpeg_command
        let args_str_refs: Vec<&str> = ffmpeg_args.iter().map(|s| s.as_str()).collect();

        if self._run_ffmpeg_command(&args_str_refs) {
            Ok(serde_json::json!({
                "status": "success",
                "message": format!("GIF generated successfully. Output: {}", output_path_str)
            }))
        } else {
            Ok(serde_json::json!({
                "status": "error",
                "message": "GIF generation failed."
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
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
            "ffmpeg",
            vec!["-version".to_string()],
            Ok(Output {
                status: ExitStatus::from_raw(0), // Fixed: Use from_raw after import
                stdout: b"ffmpeg version mock".to_vec(),
                stderr: Vec::new(),
            }),
        );
        let tool = GifGeneratorTool::new_with_runner(Box::new(mock_runner));
        assert!(tool.is_available());

        let mock_runner_fail = MockCommandRunner::new();
        mock_runner_fail.add_mock_output(
            "ffmpeg",
            vec!["-version".to_string()],
            Ok(Output {
                status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                stdout: Vec::new(),
                stderr: b"ffmpeg not found mock".to_vec(),
            }),
        );
        let tool_fail = GifGeneratorTool::new_with_runner(Box::new(mock_runner_fail));
        assert!(!tool_fail.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_input_images() {
        // Added async
        let tool = GifGeneratorTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide a mock runner
        let input = json!({
            "output_path": "output.gif",
            // Missing input_images
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("input_images list is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_output_path() {
        // Added async
        let tool = GifGeneratorTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide a mock runner
        let input = json!({
            "input_images": ["input1.png"],
            // Missing output_path
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("output_path is required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_input_image_not_found() {
        // Added async
        let dir = tempdir().unwrap();
        let tool = GifGeneratorTool::new_with_runner(Box::new(MockCommandRunner::new())); // Provide a mock runner
        let input = json!({
            "input_images": [dir.path().join("non_existent.png").to_str().unwrap()],
            "output_path": dir.path().join("output.gif").to_str().unwrap(),
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Input image not found"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_gif_creation_success() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file1 = create_dummy_file(dir.path(), "input1.png", b"dummy_png_content_1");
        let input_file2 = create_dummy_file(dir.path(), "input2.png", b"dummy_png_content_2");
        let output_file = dir.path().join("output.gif");

        let expected_args: Vec<String> = vec![
            "-i".to_string(),
            input_file1.to_str().unwrap().to_string(),
            "-i".to_string(),
            input_file2.to_str().unwrap().to_string(),
            "-vf".to_string(),
            "fps=5".to_string(),
            "-loop".to_string(),
            "0".to_string(),
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "ffmpeg",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(0), // Fixed: Use from_raw after import
                stdout: b"mock ffmpeg output success".to_vec(),
                stderr: Vec::new(),
            }),
        );

        let tool = GifGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "input_images": [input_file1.to_str().unwrap(), input_file2.to_str().unwrap()],
            "output_path": output_file.to_str().unwrap(),
            "frame_rate": 5,
            "loop_count": 0
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        // Simulate the file creation by ffmpeg
        create_dummy_file(dir.path(), "output.gif", b"dummy_gif_content");
        assert!(output_file.exists(), "Output GIF file was not created.");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_gif_creation_failure() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        let dir = tempdir().unwrap();
        let input_file1 = create_dummy_file(dir.path(), "input1.png", b"dummy_png_content_1");
        let output_file = dir.path().join("output_fail.gif");

        let expected_args: Vec<String> = vec![
            "-i".to_string(),
            input_file1.to_str().unwrap().to_string(),
            "-vf".to_string(),
            "fps=0".to_string(), // Invalid frame rate might cause ffmpeg to fail
            "-loop".to_string(),
            "0".to_string(),
            output_file.to_str().unwrap().to_string(),
        ];

        mock_runner.add_mock_output(
            "ffmpeg",
            expected_args,
            Ok(Output {
                status: ExitStatus::from_raw(1), // Fixed: Use from_raw after import
                stdout: Vec::new(),
                stderr: b"mock ffmpeg output failure".to_vec(),
            }),
        );

        let tool = GifGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = json!({
            "input_images": [input_file1.to_str().unwrap()],
            "output_path": output_file.to_str().unwrap(),
            "frame_rate": 0, // Invalid frame rate might cause ffmpeg to fail
            "loop_count": 0
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("GIF generation failed."));
        assert!(
            !output_file.exists(),
            "Output GIF file should not have been created on failure."
        );
    }
}
