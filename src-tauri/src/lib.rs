use app_core::image_generator::generate_image;
use app_core::pipeline::PipelineDefinition;
use app_core::tools::base_tool::BaseTool;
use app_core::tools::css_analyzer::CssAnalyzerTool;
use app_core::tools::html_bundler::HtmlBundlerTool;
use app_core::tools::screenshot_tool::ScreenshotTool;
use app_core::tools::tool_definition::ToolDefinition;
use app_core::tools::tool_registry::ToolRegistry;
use serde_json::Value;
use tauri::AppHandle;
use tauri::State;
use tauri_plugin_dialog::init as init_dialog_plugin;
use tauri_plugin_fs::init as init_fs_plugin;

mod governance;
mod runtime;
use governance::{
    validate_budget_envelope, validate_release_gates, BudgetEnvelope, GovernanceValidationResult,
    ReleaseGateInput,
};
use runtime::{JobHandle, JobManager, JobSnapshot};
use std::time::Duration;

#[tauri::command]
async fn screenshot(url: String) -> Result<Value, String> {
    let tool = ScreenshotTool::new();
    let input = serde_json::json!({"url": url, "output_path": "output.png"});
    match tool.run(input).await {
        Ok(output) => Ok(output),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn analyze_css(path: String) -> Result<Value, String> {
    let tool = CssAnalyzerTool::new();
    let input = serde_json::json!({"path": path});
    match tool.run(input).await {
        Ok(output) => Ok(output),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn bundle_html(path: String) -> Result<Value, String> {
    let tool = HtmlBundlerTool::new();
    let input = serde_json::json!({"path": path});
    match tool.run(input).await {
        Ok(output) => Ok(output),
        Err(e) => Err(e.to_string()),
    }
}

/// # NDOC
/// component: `tauri_commands::get_tools`
/// purpose: Return backend tool definitions for dynamic frontend rendering.
/// invariants:
///   - Tool definitions originate from backend registry (single source of truth).
#[tauri::command]
fn get_tools() -> Result<Vec<ToolDefinition>, String> {
    let registry = ToolRegistry::new();
    Ok(registry.get_available_tool_definitions())
}

/// # NDOC
/// component: `tauri_commands::run_tool`
/// purpose: Compatibility command that runs a tool and waits for completion.
/// invariants:
///   - Internally uses job manager rather than direct tool execution.
#[tauri::command]
async fn run_tool(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    tool_name: String,
    input: Value,
) -> Result<Value, String> {
    let handle = state.start_tool_job(&app_handle, tool_name, input)?;
    let snapshot = state
        .wait_for_terminal_state(&handle.job_id, Duration::from_secs(120))
        .await?;

    match snapshot.status {
        runtime::JobStatus::Succeeded => snapshot
            .output
            .ok_or_else(|| "Completed job missing output payload.".to_string()),
        runtime::JobStatus::Failed => {
            let message = snapshot
                .error
                .as_ref()
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Job failed");
            Err(message.to_string())
        }
        runtime::JobStatus::Canceled => Err("Job canceled".to_string()),
        _ => Err("Job did not reach terminal state.".to_string()),
    }
}

/// # NDOC
/// component: `tauri_commands::start_tool_job`
/// purpose: Start asynchronous tool execution and return a job handle.
#[tauri::command]
fn start_tool_job(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    tool_name: String,
    input: Value,
) -> Result<JobHandle, String> {
    state.start_tool_job(&app_handle, tool_name, input)
}

/// # NDOC
/// component: `tauri_commands::start_tool_job_governed`
/// purpose: Start tool execution only when release gates and budget envelope pass validation.
/// invariants:
///   - Existing `start_tool_job` behavior remains unchanged.
#[tauri::command]
fn start_tool_job_governed(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    tool_name: String,
    input: Value,
    budget: BudgetEnvelope,
    gates: ReleaseGateInput,
) -> Result<JobHandle, String> {
    let budget_validation = validate_budget_envelope(&budget);
    if !budget_validation.ok {
        return Err(format!(
            "budget envelope validation failed: {}",
            budget_validation.errors.join("; ")
        ));
    }

    let gate_validation = validate_release_gates(&gates);
    if !gate_validation.ok {
        return Err(format!(
            "release gate validation failed: {}",
            gate_validation.errors.join("; ")
        ));
    }

    state.start_tool_job(&app_handle, tool_name, input)
}

/// # NDOC
/// component: `tauri_commands::start_pipeline_job`
/// purpose: Start asynchronous pipeline execution and return a job handle.
/// invariants:
///   - Pipeline lifecycle is managed by the same JobManager used for tool jobs.
#[tauri::command]
fn start_pipeline_job(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    definition: PipelineDefinition,
) -> Result<JobHandle, String> {
    state.start_pipeline_job(&app_handle, definition)
}

/// # NDOC
/// component: `tauri_commands::start_pipeline_job_governed`
/// purpose: Start pipeline execution only when release gates and budget envelope pass validation.
#[tauri::command]
fn start_pipeline_job_governed(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    definition: PipelineDefinition,
    budget: BudgetEnvelope,
    gates: ReleaseGateInput,
) -> Result<JobHandle, String> {
    let budget_validation = validate_budget_envelope(&budget);
    if !budget_validation.ok {
        return Err(format!(
            "budget envelope validation failed: {}",
            budget_validation.errors.join("; ")
        ));
    }

    let gate_validation = validate_release_gates(&gates);
    if !gate_validation.ok {
        return Err(format!(
            "release gate validation failed: {}",
            gate_validation.errors.join("; ")
        ));
    }

    state.start_pipeline_job(&app_handle, definition)
}

/// # NDOC
/// component: `tauri_commands::validate_governance_inputs`
/// purpose: Expose governance validation to frontend/operators without starting execution.
#[tauri::command]
fn validate_governance_inputs(
    budget: BudgetEnvelope,
    gates: ReleaseGateInput,
) -> Result<GovernanceValidationResult, String> {
    let mut errors = Vec::new();
    let budget_validation = validate_budget_envelope(&budget);
    if !budget_validation.ok {
        errors.extend(budget_validation.errors);
    }
    let gate_validation = validate_release_gates(&gates);
    if !gate_validation.ok {
        errors.extend(gate_validation.errors);
    }

    Ok(GovernanceValidationResult {
        ok: errors.is_empty(),
        errors,
    })
}

/// # NDOC
/// component: `tauri_commands::run_pipeline`
/// purpose: Compatibility command that starts and waits for a pipeline run.
/// invariants:
///   - Uses `start_pipeline_job` + terminal wait for consistent behavior.
#[tauri::command]
async fn run_pipeline(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    definition: PipelineDefinition,
) -> Result<Value, String> {
    let handle = state.start_pipeline_job(&app_handle, definition)?;
    let snapshot = state
        .wait_for_terminal_state(&handle.job_id, Duration::from_secs(180))
        .await?;

    match snapshot.status {
        runtime::JobStatus::Succeeded => snapshot
            .output
            .ok_or_else(|| "Completed pipeline job missing output payload.".to_string()),
        runtime::JobStatus::Failed => {
            let message = snapshot
                .error
                .as_ref()
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Pipeline job failed");
            Err(message.to_string())
        }
        runtime::JobStatus::Canceled => Err("Pipeline job canceled".to_string()),
        _ => Err("Pipeline job did not reach terminal state.".to_string()),
    }
}

/// # NDOC
/// component: `tauri_commands::get_tool_job`
/// purpose: Poll current job snapshot by id.
#[tauri::command]
fn get_tool_job(state: State<'_, JobManager>, job_id: String) -> Result<JobSnapshot, String> {
    state
        .get_job(&job_id)
        .ok_or_else(|| format!("Job '{}' not found.", job_id))
}

/// # NDOC
/// component: `tauri_commands::cancel_tool_job`
/// purpose: Request cancellation for a queued/running job.
#[tauri::command]
fn cancel_tool_job(state: State<'_, JobManager>, job_id: String) -> Result<(), String> {
    state.cancel_job(&job_id)
}

#[tauri::command]
async fn generate_image_command(prompt: String, campaign_dir: String) -> Result<String, String> {
    match generate_image(&prompt, &campaign_dir).await {
        Ok(path) => Ok(path.to_string_lossy().into_owned()),
        Err(e) => Err(e),
    }
}

/// # NDOC
/// component: `tauri_app::run`
/// purpose: Tauri app bootstrap and command registration.
/// invariants:
///   - `JobManager` must be managed in app state before command invocation.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(JobManager::new())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(init_dialog_plugin())?;
            app.handle().plugin(init_fs_plugin())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            screenshot,
            analyze_css,
            bundle_html,
            get_tools,
            run_tool,
            start_tool_job,
            start_tool_job_governed,
            start_pipeline_job,
            start_pipeline_job_governed,
            run_pipeline,
            validate_governance_inputs,
            get_tool_job,
            cancel_tool_job,
            generate_image_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
