use app_core::image_generator::generate_image;
use app_core::pipeline::PipelineDefinition;
use app_core::subsystems::marketing_data_analysis::{
    build_executive_dashboard_snapshot, AnalyticsRunStore, MockAnalyticsRequestV1,
    SnapshotBuildOptions,
};
use app_core::tools::base_tool::BaseTool;
use app_core::tools::css_analyzer::CssAnalyzerTool;
use app_core::tools::html_bundler::HtmlBundlerTool;
use app_core::tools::screenshot_tool::ScreenshotTool;
use app_core::tools::tool_definition::{
    ParameterDefinition, ToolComplexity, ToolDefinition, ToolUIMetadata,
};
use app_core::tools::tool_registry::ToolRegistry;

use serde_json::{json, Value};
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

fn validate_governed_pipeline_contract(definition: &PipelineDefinition) -> Result<(), String> {
    let Some(refs) = definition.governance_refs.as_ref() else {
        return Err(
            "governed pipeline requires definition.governance_refs with budget/release/provenance references"
                .to_string(),
        );
    };

    if refs.budget_envelope_ref.trim().is_empty() {
        return Err(
            "governed pipeline requires non-empty governance_refs.budget_envelope_ref".to_string(),
        );
    }
    if refs.release_gate_log_ref.trim().is_empty() {
        return Err(
            "governed pipeline requires non-empty governance_refs.release_gate_log_ref".to_string(),
        );
    }

    let has_change_request = refs
        .change_request_ids
        .iter()
        .any(|value| !value.trim().is_empty());
    let has_decision = refs
        .decision_ids
        .iter()
        .any(|value| !value.trim().is_empty());
    if !has_change_request && !has_decision {
        return Err(
            "governed pipeline requires at least one non-empty change_request_id or decision_id"
                .to_string(),
        );
    }

    Ok(())
}

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
    let mut tools = registry.get_available_tool_definitions();
    tools.push(ToolDefinition {
        name: "analytics::mock_pipeline".to_string(),
        description:
            "Deterministic mock analytics pipeline with persistence, drift checks, and narratives."
                .to_string(),
        ui_metadata: ToolUIMetadata {
            category: "Analytics".to_string(),
            display_name: "Mock Analytics Pipeline".to_string(),
            icon: Some("analytics".to_string()),
            complexity: ToolComplexity::Intermediate,
            estimated_time_seconds: 4,
            tags: vec![
                "analytics".to_string(),
                "mock-data".to_string(),
                "trend-analysis".to_string(),
            ],
        },
        parameters: vec![
            ParameterDefinition {
                name: "start_date".to_string(),
                r#type: "string".to_string(),
                description: "Start date in YYYY-MM-DD.".to_string(),
                optional: false,
            },
            ParameterDefinition {
                name: "end_date".to_string(),
                r#type: "string".to_string(),
                description: "End date in YYYY-MM-DD.".to_string(),
                optional: false,
            },
            ParameterDefinition {
                name: "profile_id".to_string(),
                r#type: "string".to_string(),
                description: "Profile identifier for longitudinal history grouping.".to_string(),
                optional: false,
            },
            ParameterDefinition {
                name: "seed".to_string(),
                r#type: "integer".to_string(),
                description: "Optional deterministic seed override.".to_string(),
                optional: true,
            },
        ],
        input_examples: vec![json!({
            "start_date": "2026-02-01",
            "end_date": "2026-02-07",
            "profile_id": "marketing_default",
            "include_narratives": true
        })],
        output_schema: Some(json!({
            "type": "object",
            "required": ["schema_version", "metadata", "report", "validation", "quality_controls", "historical_analysis"]
        })),
    });
    Ok(tools)
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
    validate_governed_pipeline_contract(&definition)?;

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

/// # NDOC
/// component: `tauri_commands::start_mock_analytics_job`
/// purpose: Start async deterministic mock analytics run and return job id for polling.
#[tauri::command]
fn start_mock_analytics_job(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    request: MockAnalyticsRequestV1,
) -> Result<JobHandle, String> {
    state.start_mock_analytics_job(&app_handle, request)
}

/// # NDOC
/// component: `tauri_commands::get_mock_analytics_run_history`
/// purpose: Retrieve persisted analytics runs for operator trend inspection.
#[tauri::command]
fn get_mock_analytics_run_history(
    profile_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let store = AnalyticsRunStore::default();
    let max = limit.unwrap_or(25).min(200);
    let maybe_profile = profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let runs = store
        .list_recent(maybe_profile, max)
        .map_err(|err| format!("{}: {}", err.code, err.message))?;
    serde_json::to_value(runs).map_err(|err| format!("failed to serialize run history: {err}"))
}

/// # NDOC
/// component: `tauri_commands::get_analysis_workflows`
/// purpose: Provide analysis workflow catalog for registry UX and discoverability.
#[tauri::command]
fn get_analysis_workflows() -> Result<Value, String> {
    Ok(json!([
        {
            "workflow_id": "wf.analytics.mock_pipeline.v1",
            "title": "Mock Analytics Pipeline",
            "entrypoint": "start_mock_analytics_job",
            "history_entrypoint": "get_mock_analytics_run_history",
            "stages": [
                "validating_input",
                "generating_data",
                "assembling_report",
                "validating_invariants",
                "historical_analysis",
                "persisting_artifact",
                "completed"
            ],
            "discoverability_tags": ["analytics", "trend", "drift", "anomaly", "operator"],
            "governance_ready": true
        }
    ]))
}

/// # NDOC
/// component: `tauri_commands::get_executive_dashboard_snapshot`
/// purpose: Return multi-chart executive snapshot assembled from persisted analytics runs.
#[tauri::command]
fn get_executive_dashboard_snapshot(
    profile_id: String,
    limit: Option<usize>,
    compare_window_runs: Option<u8>,
    target_roas: Option<f64>,
    monthly_revenue_target: Option<f64>,
) -> Result<Value, String> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err("profile_id cannot be empty".to_string());
    }
    let store = AnalyticsRunStore::default();
    let runs = store
        .list_recent(Some(profile_id), limit.unwrap_or(24).min(64))
        .map_err(|err| format!("{}: {}", err.code, err.message))?;
    let options = SnapshotBuildOptions {
        compare_window_runs: compare_window_runs.unwrap_or(1).max(1) as usize,
        target_roas,
        monthly_revenue_target,
    };
    let snapshot =
        build_executive_dashboard_snapshot(profile_id, &runs, options).ok_or_else(|| {
            format!(
                "No persisted analytics runs found for profile '{}'. Generate a run first.",
                profile_id
            )
        })?;
    serde_json::to_value(snapshot)
        .map_err(|err| format!("failed to serialize dashboard snapshot: {err}"))
}

/// # NDOC
/// component: `tauri_commands::get_dashboard_chart_definitions`
/// purpose: Return stable chart catalog metadata for frontend rendering surfaces.
#[tauri::command]
fn get_dashboard_chart_definitions() -> Result<Value, String> {
    Ok(json!([
        {"id":"kpi_strip","title":"North Star KPIs","kind":"cards"},
        {"id":"scale_efficiency","title":"Spend vs Revenue and ROAS","kind":"line"},
        {"id":"funnel","title":"Funnel Leakage","kind":"funnel"},
        {"id":"storefront_behavior","title":"Wix Storefront Behavior","kind":"matrix"},
        {"id":"campaign_portfolio","title":"Campaign Portfolio","kind":"table"},
        {"id":"forecast_pacing","title":"Forecast and Pacing","kind":"forecast"},
        {"id":"data_quality_scorecard","title":"Data Quality Scorecard","kind":"scorecard"},
        {"id":"publish_export_gate","title":"Publish and Export Gate","kind":"gate"},
        {"id":"decision_feed","title":"Governance Decision Feed","kind":"cards"},
        {"id":"trust_risk","title":"Trust and Risk","kind":"signals"}
    ]))
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
            start_mock_analytics_job,
            get_mock_analytics_run_history,
            get_analysis_workflows,
            get_executive_dashboard_snapshot,
            get_dashboard_chart_definitions,
            generate_image_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
