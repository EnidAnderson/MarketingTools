use serde_json::Value;
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Executes a Python tool via the `python_tool_dispatcher.py` script.
///
/// This function constructs and executes a command to run a Python script,
/// passing the tool name and JSON-serialized parameters. It captures the
/// standard output and standard error, returning the stdout if successful,
/// or an error message if the Python script fails or returns an error status.
///
/// # Arguments
/// * `tool_name` - The name of the Python tool to execute (e.g., "email_sender_tool").
/// * `params` - A `serde_json::Value` containing the parameters for the tool.
///
/// # Returns
/// A `Result` which is `Ok(String)` containing the JSON output from the Python
/// script on success, or `Err(String)` containing an error message on failure.
pub fn execute_python_tool(tool_name: String, params: Value) -> Result<String, String> {
    let current_dir =
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let script_path = current_dir.join("src").join("python_tool_dispatcher.py");

    let params_str = serde_json::to_string(&params)
        .map_err(|e| format!("Failed to serialize parameters to JSON: {}", e))?;

    let output = Command::new("python3") // Use "python" or "python3" depending on system setup
        .arg(&script_path)
        .arg(&tool_name)
        .arg(&params_str)
        .output()
        .map_err(|e| format!("Failed to execute python_tool_dispatcher.py: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "Python script execution failed for tool '{}'.
Stdout: {}
Stderr: {}",
            tool_name, stdout, stderr
        ));
    }

    // Attempt to parse the stdout as JSON to check for "status: error" from the dispatcher
    match serde_json::from_str::<Value>(&stdout) {
        Ok(json_output) => {
            if json_output["status"] == "error" {
                return Err(format!(
                    "Python dispatcher reported an error for tool '{}': {}",
                    tool_name,
                    json_output["message"].as_str().unwrap_or("Unknown error")
                ));
            }
            Ok(stdout)
        }
        Err(_) => {
            // If stdout is not valid JSON, it might be an unexpected output or a print from a tool
            // For now, treat it as a successful but non-JSON output
            // In a real scenario, you might want stricter validation here
            Ok(stdout)
        }
    }
}
