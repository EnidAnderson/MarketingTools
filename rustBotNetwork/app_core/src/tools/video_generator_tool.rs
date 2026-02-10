use super::base_tool::BaseTool;
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

// Define a trait for running commands, making it mockable
pub trait CommandRunner {
    fn run_command(&self, command_name: &str, args: &[&str]) -> Result<Output, std::io::Error>;
    fn check_command_available(&self, command_name: &str) -> bool;
}

// Concrete implementation that uses std::process::Command
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run_command(&self, command_name: &str, args: &[&str]) -> Result<Output, std::io::Error> {
        Command::new(command_name).args(args).output()
    }

    fn check_command_available(&self, command_name: &str) -> bool {
        Command::new(command_name)
            .arg("-version")
            .output()
            .map_or(false, |output| output.status.success())
    }
}

pub struct VideoGeneratorTool {
    command_runner: Box<dyn CommandRunner + Send + Sync>,
}

impl VideoGeneratorTool {
    pub fn new() -> Self {
        VideoGeneratorTool {
            command_runner: Box::new(RealCommandRunner),
        }
    }

    // Constructor for dependency injection in tests
    #[cfg(test)]
    pub fn new_with_runner(runner: Box<dyn CommandRunner + Send + Sync>) -> Self {
        VideoGeneratorTool {
            command_runner: runner,
        }
    }
}

#[async_trait]
impl BaseTool for VideoGeneratorTool {
    fn name(&self) -> &'static str {
        "VideoGeneratorTool"
    }

    fn description(&self) -> &'static str {
        "Generates video content based on provided specifications using ffmpeg and ImageMagick."
    }

    fn is_available(&self) -> bool {
        self.command_runner.check_command_available("ffmpeg")
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        if !self.is_available() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "ffmpeg is not installed or not in PATH. Cannot generate video."
            }));
        }

        let video_script = input["video_script"].as_str().unwrap_or("");
        let video_style = input["video_style"].as_str().unwrap_or("default");
        let video_duration_seconds = input["video_duration_seconds"].as_u64().unwrap_or(5);
        let video_assets_description = input["video_assets_description"].as_str().unwrap_or("");
        let campaign_dir_str = input["campaign_dir"]
            .as_str()
            .ok_or("campaign_dir is required")?;
        let campaign_dir = PathBuf::from(campaign_dir_str);

        fs::create_dir_all(&campaign_dir)?;

        let output_video_filename = format!(
            "generated_video_{}s_{}.mp4",
            video_duration_seconds,
            video_style.replace(' ', "_")
        );
        let output_video_path = campaign_dir.join(output_video_filename);

        println!("--- Simulating Video Generation ---");
        println!("Script: {:.100}...", video_script);
        println!("Style: {}", video_style);
        println!("Duration: {}s", video_duration_seconds);
        println!("Assets: {:.100}...", video_assets_description);

        let has_imagemagick = self.command_runner.check_command_available("magick");
        let mut ffmpeg_cmd_args: Vec<String> = Vec::new();

        let temp_text_image = if has_imagemagick {
            let temp_path = campaign_dir.join("temp_text.png");
            let temp_path_str = temp_path
                .to_str()
                .ok_or("Invalid temp image path")?
                .to_string();

            let imagemagick_output = self.command_runner.run_command(
                "magick",
                &[
                    "convert",
                    "-size",
                    "1280x720",
                    "xc:white",
                    "-font",
                    "Arial",
                    "-pointsize",
                    "48",
                    "-fill",
                    "black",
                    "-gravity",
                    "Center",
                    "-annotate",
                    "+0+0",
                    &format!(
                        "Simulated Video: {}
Duration: {}s",
                        video_style, video_duration_seconds
                    ),
                    &temp_path_str,
                ],
            )?;

            if !imagemagick_output.status.success() {
                eprintln!(
                    "ImageMagick error: {}",
                    String::from_utf8_lossy(&imagemagick_output.stderr)
                );
                return Err(Box::from(format!(
                    "ImageMagick failed: {}",
                    String::from_utf8_lossy(&imagemagick_output.stderr)
                )));
            }
            Some(temp_path)
        } else {
            println!("Warning: ImageMagick 'magick' command not found. Generating blank video.");
            None
        };

        if let Some(ref temp_img_path) = temp_text_image {
            ffmpeg_cmd_args.push("-y".to_string());
            ffmpeg_cmd_args.push("-loop".to_string());
            ffmpeg_cmd_args.push("1".to_string());
            ffmpeg_cmd_args.push("-i".to_string());
            ffmpeg_cmd_args.push(
                temp_img_path
                    .to_str()
                    .ok_or("Invalid temp image path")?
                    .to_string(),
            );
            ffmpeg_cmd_args.push("-c:v".to_string());
            ffmpeg_cmd_args.push("libx264".to_string());
            ffmpeg_cmd_args.push("-t".to_string());
            ffmpeg_cmd_args.push(video_duration_seconds.to_string());
            ffmpeg_cmd_args.push("-pix_fmt".to_string());
            ffmpeg_cmd_args.push("yuv420p".to_string());
            ffmpeg_cmd_args.push("-vf".to_string());
            ffmpeg_cmd_args.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string());
            ffmpeg_cmd_args.push(
                output_video_path
                    .to_str()
                    .ok_or("Invalid output video path")?
                    .to_string(),
            );
        } else {
            ffmpeg_cmd_args.push("-y".to_string());
            ffmpeg_cmd_args.push("-f".to_string());
            ffmpeg_cmd_args.push("lavfi".to_string());
            ffmpeg_cmd_args.push("-i".to_string());
            ffmpeg_cmd_args.push("color=c=black:s=1280x720:d=1".to_string());
            ffmpeg_cmd_args.push("-vf".to_string());
            ffmpeg_cmd_args.push(format!("drawtext=fontfile=Arial:text='Simulated Video':x=(w-text_w)/2:y=(h-text_h)/2:fontsize=48:fontcolor=white"));
            ffmpeg_cmd_args.push("-t".to_string());
            ffmpeg_cmd_args.push(video_duration_seconds.to_string());
            ffmpeg_cmd_args.push(
                output_video_path
                    .to_str()
                    .ok_or("Invalid output video path")?
                    .to_string(),
            );
        };

        let ffmpeg_args_str: Vec<&str> = ffmpeg_cmd_args.iter().map(|s| s.as_str()).collect();

        let ffmpeg_output = self
            .command_runner
            .run_command("ffmpeg", &ffmpeg_args_str)?;

        if !ffmpeg_output.status.success() {
            let error_msg = format!(
                "ffmpeg error: {}",
                String::from_utf8_lossy(&ffmpeg_output.stderr)
            );
            eprintln!("{}", error_msg);
            return Ok(serde_json::json!({
                "status": "error",
                "message": error_msg
            }));
        }

        if let Some(temp_img_path) = temp_text_image {
            fs::remove_file(temp_img_path)?;
        }

        println!("Simulated video saved to {}", output_video_path.display());
        Ok(serde_json::json!({
            "status": "success",
            "path": output_video_path.to_str().ok_or("Invalid output video path")?
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt; // Import ExitStatusExt
    use std::path::Path;
    use std::process::ExitStatus; // Import ExitStatus
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir; // Import Write for fs::File::create().write_all()

    // Mock implementation for CommandRunner
    struct MockCommandRunner {
        // Store expected outputs for commands
        mock_outputs: Arc<Mutex<Vec<(String, Vec<String>, Result<Output, std::io::Error>)>>>,
        mock_availability: Arc<Mutex<HashMap<String, bool>>>,
    }

    impl MockCommandRunner {
        fn new() -> Self {
            MockCommandRunner {
                mock_outputs: Arc::new(Mutex::new(Vec::new())),
                mock_availability: Arc::new(Mutex::new(HashMap::new())),
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

        // Set mock availability for a command
        fn set_command_available(&self, command: &str, available: bool) {
            self.mock_availability
                .lock()
                .unwrap()
                .insert(command.to_string(), available);
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
                    status: ExitStatus::from_raw(1), // Fixed
                    stdout: Vec::new(),
                    stderr: b"Mock command failed: No mock output defined.".to_vec(),
                })
            }
        }

        fn check_command_available(&self, command_name: &str) -> bool {
            *self
                .mock_availability
                .lock()
                .unwrap()
                .get(command_name)
                .unwrap_or(&false)
        }
    }

    #[test]
    fn test_is_available() {
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", true);
        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));
        assert!(tool.is_available());

        let mock_runner_fail = MockCommandRunner::new();
        mock_runner_fail.set_command_available("ffmpeg", false);
        let tool_fail = VideoGeneratorTool::new_with_runner(Box::new(mock_runner_fail));
        assert!(!tool_fail.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_video_generation_success_with_imagemagick() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", true);
        mock_runner.set_command_available("magick", true);

        let temp_dir = tempdir().unwrap();
        let campaign_path_str = temp_dir.path().to_str().unwrap();
        let temp_text_image_path = PathBuf::from(campaign_path_str).join("temp_text.png");
        let output_video_path =
            PathBuf::from(campaign_path_str).join("generated_video_1s_simple.mp4");

        // Mock ImageMagick command
        let imagemagick_args = vec![
            "convert".to_string(),
            "-size".to_string(),
            "1280x720".to_string(),
            "xc:white".to_string(),
            "-font".to_string(),
            "Arial".to_string(),
            "-pointsize".to_string(),
            "48".to_string(),
            "-fill".to_string(),
            "black".to_string(),
            "-gravity".to_string(),
            "Center".to_string(),
            "-annotate".to_string(),
            "+0+0".to_string(),
            "Simulated Video: simple\nDuration: 1s".to_string(),
            temp_text_image_path.to_str().unwrap().to_string(),
        ];
        mock_runner.add_mock_output(
            "magick",
            imagemagick_args,
            Ok(Output {
                status: std::process::ExitStatus::from_raw(0), // Success
                stdout: b"mock magick output".to_vec(),
                stderr: Vec::new(),
            }),
        );
        // Simulate creation of the temp image file by imagemagick
        let mut temp_file = fs::File::create(&temp_text_image_path).unwrap();
        temp_file.write_all(b"dummy image content").unwrap();

        // Mock FFmpeg command
        let ffmpeg_args = vec![
            "-y".to_string(),
            "-loop".to_string(),
            "1".to_string(),
            "-i".to_string(),
            temp_text_image_path.to_str().unwrap().to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-t".to_string(),
            "1".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-vf".to_string(),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string(),
            output_video_path.to_str().unwrap().to_string(),
        ];
        mock_runner.add_mock_output(
            "ffmpeg",
            ffmpeg_args,
            Ok(Output {
                status: std::process::ExitStatus::from_raw(0), // Success
                stdout: b"mock ffmpeg output".to_vec(),
                stderr: Vec::new(),
            }),
        );

        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = serde_json::json!({
            "video_script": "A test script.",
            "video_style": "simple",
            "video_duration_seconds": 1,
            "video_assets_description": "A test description.",
            "campaign_dir": campaign_path_str
        });

        let result = tool.run(input).await.unwrap(); // Added .await

        assert_eq!(result["status"], "success");
        let video_path = result["path"].as_str().unwrap();
        assert!(Path::new(video_path).exists());
        assert!(video_path.ends_with(".mp4"));
        assert!(
            !temp_text_image_path.exists(),
            "Temporary image file should be removed."
        );
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_video_generation_success_without_imagemagick() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", true);
        mock_runner.set_command_available("magick", false); // ImageMagick not available

        let temp_dir = tempdir().unwrap();
        let campaign_path_str = temp_dir.path().to_str().unwrap();
        let output_video_path =
            PathBuf::from(campaign_path_str).join("generated_video_1s_simple.mp4");

        // Mock FFmpeg command for the fallback scenario (without imagemagick)
        let ffmpeg_args = vec![
            "-y".to_string(), "-f".to_string(), "lavfi".to_string(), "-i".to_string(), "color=c=black:s=1280x720:d=1".to_string(),
            "-vf".to_string(), "drawtext=fontfile=Arial:text='Simulated Video':x=(w-text_w)/2:y=(h-text_h)/2:fontsize=48:fontcolor=white".to_string(),
            "-t".to_string(), "1".to_string(),
            output_video_path.to_str().unwrap().to_string(),
        ];
        mock_runner.add_mock_output(
            "ffmpeg",
            ffmpeg_args,
            Ok(Output {
                status: std::process::ExitStatus::from_raw(0), // Success
                stdout: b"mock ffmpeg output".to_vec(),
                stderr: Vec::new(),
            }),
        );

        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = serde_json::json!({
            "video_script": "A test script.",
            "video_style": "simple",
            "video_duration_seconds": 1,
            "video_assets_description": "A test description.",
            "campaign_dir": campaign_path_str
        });

        let result = tool.run(input).await.unwrap(); // Added .await

        assert_eq!(result["status"], "success");
        let video_path = result["path"].as_str().unwrap();
        assert!(Path::new(video_path).exists());
        assert!(video_path.ends_with(".mp4"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_ffmpeg_not_available() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", false); // ffmpeg not available
        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = serde_json::json!({
            "video_script": "script",
            "video_style": "style",
            "video_duration_seconds": 1,
            "video_assets_description": "assets",
            "campaign_dir": "/tmp"
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("ffmpeg is not installed or not in PATH. Cannot generate video."));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_magick_failure() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", true);
        mock_runner.set_command_available("magick", true);

        let temp_dir = tempdir().unwrap();
        let campaign_path_str = temp_dir.path().to_str().unwrap();
        let temp_text_image_path = PathBuf::from(campaign_path_str).join("temp_text.png");

        // Mock ImageMagick command to FAIL
        let imagemagick_args = vec![
            "convert".to_string(),
            "-size".to_string(),
            "1280x720".to_string(),
            "xc:white".to_string(),
            "-font".to_string(),
            "Arial".to_string(),
            "-pointsize".to_string(),
            "48".to_string(),
            "-fill".to_string(),
            "black".to_string(),
            "-gravity".to_string(),
            "Center".to_string(),
            "-annotate".to_string(),
            "+0+0".to_string(),
            "Simulated Video: simple\nDuration: 1s".to_string(),
            temp_text_image_path.to_str().unwrap().to_string(),
        ];
        mock_runner.add_mock_output(
            "magick",
            imagemagick_args,
            Ok(Output {
                status: std::process::ExitStatus::from_raw(1), // Failure
                stdout: b"".to_vec(),
                stderr: b"mock magick failure output".to_vec(),
            }),
        );
        // No need to create dummy temp file as magick fails

        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = serde_json::json!({
            "video_script": "A test script.",
            "video_style": "simple",
            "video_duration_seconds": 1,
            "video_assets_description": "A test description.",
            "campaign_dir": campaign_path_str
        });

        let result = tool.run(input).await.unwrap_err(); // Added .await // Expecting an error
        assert!(result
            .to_string()
            .contains("ImageMagick failed: mock magick failure output"));
        assert!(
            !temp_text_image_path.exists(),
            "Temporary image file should not be created on magick failure."
        );
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_ffmpeg_failure() {
        // Added async
        let mock_runner = MockCommandRunner::new();
        mock_runner.set_command_available("ffmpeg", true);
        mock_runner.set_command_available("magick", false); // No imagemagick for simpler ffmpeg path

        let temp_dir = tempdir().unwrap();
        let campaign_path_str = temp_dir.path().to_str().unwrap();
        let output_video_path =
            PathBuf::from(campaign_path_str).join("generated_video_1s_simple.mp4");

        // Mock FFmpeg command to FAIL
        let ffmpeg_args = vec![
            "-y".to_string(), "-f".to_string(), "lavfi".to_string(), "-i".to_string(), "color=c=black:s=1280x720:d=1".to_string(),
            "-vf".to_string(), "drawtext=fontfile=Arial:text='Simulated Video':x=(w-text_w)/2:y=(h-text_h)/2:fontsize=48:fontcolor=white".to_string(),
            "-t".to_string(), "1".to_string(),
            output_video_path.to_str().unwrap().to_string(),
        ];
        mock_runner.add_mock_output(
            "ffmpeg",
            ffmpeg_args,
            Ok(Output {
                status: std::process::ExitStatus::from_raw(1), // Failure
                stdout: b"".to_vec(),
                stderr: b"mock ffmpeg failure output".to_vec(),
            }),
        );

        let tool = VideoGeneratorTool::new_with_runner(Box::new(mock_runner));

        let input = serde_json::json!({
            "video_script": "A test script.",
            "video_style": "simple",
            "video_duration_seconds": 1,
            "video_assets_description": "A test description.",
            "campaign_dir": campaign_path_str
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("ffmpeg error: mock ffmpeg failure output"));
        assert!(
            !output_video_path.exists(),
            "Output video file should not be created on ffmpeg failure."
        );
    }
}
