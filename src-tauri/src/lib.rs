use app_core::image_generator::generate_image;
use app_core::pipeline::PipelineDefinition;
use app_core::subsystems::campaign_orchestration::{
    prioritized_text_graph_templates_v1, runtime::TextWorkflowRunRequestV1,
};
use app_core::subsystems::marketing_data_analysis::{
    analytics_connector_config_from_env, build_executive_dashboard_snapshot,
    evaluate_analytics_connectors_preflight, load_attestation_key_registry_from_env_or_file,
    resolve_attestation_policy_v1, verify_connector_attestation_with_registry_v1,
    AnalyticsConnectorConfigV1, AnalyticsRunStore, DashboardExportAuditRecordV1,
    DashboardExportAuditStore, MockAnalyticsRequestV1, PersistedAnalyticsRunV1,
    SimulatedAnalyticsConnectorV2, SnapshotBuildOptions,
};
use app_core::tools::base_tool::BaseTool;
use app_core::tools::css_analyzer::CssAnalyzerTool;
use app_core::tools::html_bundler::HtmlBundlerTool;
use app_core::tools::screenshot_tool::ScreenshotTool;
use app_core::tools::tool_audit::{build_tool_audit_report_v1, ToolAuditReportV1};
use app_core::tools::tool_definition::{
    ParameterDefinition, ToolComplexity, ToolDefinition, ToolMaturity, ToolUIMetadata,
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        maturity: ToolMaturity::Stable,
        human_workflow: "Review decision_feed and publish_export_gate, then approve only if all blockers are cleared."
            .to_string(),
        output_artifact_kind: "analytics.mock_artifact.v1".to_string(),
        requires_review: true,
        default_input_template: json!({
            "start_date": "2026-02-01",
            "end_date": "2026-02-07",
            "profile_id": "marketing_default",
            "include_narratives": true,
            "budget_envelope": {
                "max_retrieval_units": 20000,
                "max_analysis_units": 10000,
                "max_llm_tokens_in": 15000,
                "max_llm_tokens_out": 8000,
                "max_total_cost_micros": 50000000,
                "policy": "fail_closed",
                "provenance_ref": "ui_example.v1"
            }
        }),
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
            ParameterDefinition {
                name: "budget_envelope".to_string(),
                r#type: "object".to_string(),
                description: "Required budget envelope for retrieval/analysis/LLM/cost caps."
                    .to_string(),
                optional: false,
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
    tools.push(ToolDefinition {
        name: "text::workflow_pipeline".to_string(),
        description:
            "Deterministic graph-based text workflow run (message house, email+landing, ad variants) with weighted gate decisions."
                .to_string(),
        maturity: ToolMaturity::Stable,
        human_workflow:
            "Run workflow, inspect gate decision and critical findings, then publish only when blockers are zero."
                .to_string(),
        output_artifact_kind: "text_workflow_run.v1".to_string(),
        requires_review: true,
        default_input_template: json!({
            "template_id": "tpl.email_landing_sequence.v1",
            "variant_count": 12,
            "paid_calls_allowed": false,
            "budget": {
                "remaining_daily_budget_usd": 10.0,
                "max_cost_per_run_usd": 2.0,
                "max_total_input_tokens": 24000,
                "max_total_output_tokens": 8000,
                "hard_daily_cap_usd": 10.0
            },
            "campaign_spine": {
                "campaign_spine_id": "spine.default.v1",
                "product_name": "Nature's Diet Raw Mix",
                "offer_summary": "Save 20% on first order",
                "audience_segments": ["new puppy owners", "sensitive stomach"],
                "positioning_statement": "Raw-first nutrition with practical prep",
                "message_house": {
                    "big_idea": "Fresh confidence in every bowl",
                    "pillars": [{"pillar_id":"p1","title":"Digestive comfort","supporting_points":["gentle proteins"]}],
                    "proof_points": [{"claim_id":"claim1","claim_text":"high digestibility blend","evidence_ref_ids":["ev1"]}],
                    "do_not_say": ["cure claims"],
                    "tone_guide": ["clear", "grounded"]
                },
                "evidence_refs": [{"evidence_id":"ev1","source_ref":"internal.digestibility.v1","excerpt":"digestibility improved 11% vs baseline"}]
            }
        }),
        ui_metadata: ToolUIMetadata {
            category: "Content".to_string(),
            display_name: "Text Workflow Pipeline".to_string(),
            icon: Some("automation".to_string()),
            complexity: ToolComplexity::Advanced,
            estimated_time_seconds: 3,
            tags: vec![
                "agent-graph".to_string(),
                "text-generation".to_string(),
                "weighted-gate".to_string(),
            ],
        },
        parameters: vec![
            ParameterDefinition {
                name: "template_id".to_string(),
                r#type: "string".to_string(),
                description: "Template id (tpl.message_house.v1 | tpl.email_landing_sequence.v1 | tpl.ad_variant_pack.v1).".to_string(),
                optional: false,
            },
            ParameterDefinition {
                name: "campaign_spine".to_string(),
                r#type: "object".to_string(),
                description: "Shared campaign spine used for all workflow sections.".to_string(),
                optional: false,
            },
            ParameterDefinition {
                name: "variant_count".to_string(),
                r#type: "integer".to_string(),
                description: "Variant count for ad-variant workflow (1..=30).".to_string(),
                optional: true,
            },
            ParameterDefinition {
                name: "paid_calls_allowed".to_string(),
                r#type: "boolean".to_string(),
                description: "When false, all node routes use local zero-cost mock provider.".to_string(),
                optional: false,
            },
        ],
        input_examples: vec![json!({
            "template_id": "tpl.email_landing_sequence.v1",
            "paid_calls_allowed": false
        })],
        output_schema: Some(json!({
            "type": "object",
            "required": ["schema_version", "template_id", "execution_order", "traces", "artifact"]
        })),
    });
    Ok(tools)
}

/// # NDOC
/// component: `tauri_commands::get_tool_audit_report`
/// purpose: Return full tool usability audit for operator and release-gate review.
#[tauri::command]
fn get_tool_audit_report() -> Result<ToolAuditReportV1, String> {
    Ok(build_tool_audit_report_v1())
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
/// component: `tauri_commands::validate_analytics_connectors_preflight`
/// purpose: Validate analytics connector config and credential readiness without starting a job.
#[tauri::command]
async fn validate_analytics_connectors_preflight(
    config: Option<AnalyticsConnectorConfigV1>,
) -> Result<Value, String> {
    let connector = SimulatedAnalyticsConnectorV2::new();
    let effective_config = match config {
        Some(cfg) => cfg,
        None => analytics_connector_config_from_env().map_err(|err| {
            format!(
                "{}: {} (field_paths={:?})",
                err.code, err.message, err.field_paths
            )
        })?,
    };
    let preflight = evaluate_analytics_connectors_preflight(&connector, &effective_config).await;
    serde_json::to_value(preflight)
        .map_err(|err| format!("failed to serialize analytics connector preflight: {err}"))
}

/// # NDOC
/// component: `tauri_commands::start_mock_text_workflow_job`
/// purpose: Start async deterministic text workflow run and return job id for polling.
#[tauri::command]
fn start_mock_text_workflow_job(
    app_handle: AppHandle,
    state: State<'_, JobManager>,
    request: TextWorkflowRunRequestV1,
) -> Result<JobHandle, String> {
    state.start_mock_text_workflow_job(&app_handle, request)
}

/// # NDOC
/// component: `tauri_commands::get_text_workflow_templates`
/// purpose: Expose prioritized text workflow templates for operator/template selection UI.
#[tauri::command]
fn get_text_workflow_templates(campaign_spine_id: String) -> Result<Value, String> {
    let spine = campaign_spine_id.trim();
    if spine.is_empty() {
        return Err("campaign_spine_id cannot be empty".to_string());
    }
    let templates = prioritized_text_graph_templates_v1(spine)
        .map_err(|err| format!("failed to build text workflow templates: {err}"))?;
    serde_json::to_value(templates)
        .map_err(|err| format!("failed to serialize text workflow templates: {err}"))
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
            "preflight_entrypoint": "validate_analytics_connectors_preflight",
            "stages": [
                "validating_input",
                "preflight_connectors",
                "generating_data",
                "assembling_report",
                "validating_invariants",
                "historical_analysis",
                "persisting_artifact",
                "completed"
            ],
            "discoverability_tags": ["analytics", "trend", "drift", "anomaly", "operator"],
            "governance_ready": true
        },
        {
            "workflow_id": "wf.text.workflow_pipeline.v1",
            "title": "Text Workflow Pipeline",
            "entrypoint": "start_mock_text_workflow_job",
            "templates_entrypoint": "get_text_workflow_templates",
            "stages": [
                "validating_graph",
                "planning_routes",
                "generating_artifact",
                "evaluating_gate",
                "completed"
            ],
            "discoverability_tags": ["text", "agent-graph", "campaign-spine", "weighted-gate"],
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
    enforce_publish_export_gate: Option<bool>,
) -> Result<Value, String> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err("profile_id cannot be empty".to_string());
    }
    let options = SnapshotBuildOptions {
        compare_window_runs: compare_window_runs.unwrap_or(1).max(1) as usize,
        target_roas,
        monthly_revenue_target,
    };
    let (snapshot, latest_run) = build_dashboard_snapshot_checked(
        profile_id,
        limit.unwrap_or(24).min(64),
        options,
        enforce_publish_export_gate.unwrap_or(true),
    )?;
    let _ = latest_run;
    serde_json::to_value(snapshot)
        .map_err(|err| format!("failed to serialize dashboard snapshot: {err}"))
}

fn build_dashboard_snapshot_checked(
    profile_id: &str,
    limit: usize,
    options: SnapshotBuildOptions,
    enforce_publish_export_gate: bool,
) -> Result<
    (
        app_core::subsystems::marketing_data_analysis::ExecutiveDashboardSnapshotV1,
        PersistedAnalyticsRunV1,
    ),
    String,
> {
    let store = AnalyticsRunStore::default();
    let runs = store
        .list_recent(Some(profile_id), limit)
        .map_err(|err| format!("{}: {}", err.code, err.message))?;
    let snapshot =
        build_executive_dashboard_snapshot(profile_id, &runs, options).ok_or_else(|| {
            format!(
                "No persisted analytics runs found for profile '{}'. Generate a run first.",
                profile_id
            )
        })?;
    let latest_run = runs.first().cloned().ok_or_else(|| {
        format!(
            "No persisted analytics runs found for profile '{}'. Generate a run first.",
            profile_id
        )
    })?;
    if enforce_publish_export_gate {
        enforce_dashboard_attestation_policy(profile_id, &latest_run)?;
        if !snapshot.publish_export_gate.export_ready {
            return Err(serde_json::json!({
                "code": "analytics_dashboard_gate_blocked",
                "message": "Dashboard export/publish gate is blocked for this profile.",
                "source": "executive_dashboard",
                "details": {
                    "profile_id": profile_id,
                    "gate_status": snapshot.publish_export_gate.gate_status,
                    "blocking_reasons": snapshot.publish_export_gate.blocking_reasons,
                }
            })
            .to_string());
        }
    }
    Ok((snapshot, latest_run))
}

fn enforce_dashboard_attestation_policy(
    profile_id: &str,
    latest_run: &PersistedAnalyticsRunV1,
) -> Result<(), String> {
    let policy = resolve_attestation_policy_v1(profile_id).map_err(|err| {
        serde_json::json!({
            "code": "analytics_attestation_policy_invalid",
            "message": err.message,
            "source": "attestation_policy",
            "details": { "profile_id": profile_id, "field_paths": err.field_paths }
        })
        .to_string()
    })?;
    if !policy.require_signed_attestations {
        return Ok(());
    }

    let attestation = &latest_run.metadata.connector_attestation;
    let signature_present = attestation
        .fingerprint_signature
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !signature_present {
        return Err(serde_json::json!({
            "code": "analytics_dashboard_signature_required",
            "message": "Signed attestation is required by policy for this profile.",
            "source": "executive_dashboard",
            "details": {
                "profile_id": profile_id,
                "run_id": latest_run.metadata.run_id,
            }
        })
        .to_string());
    }

    let registry = load_attestation_key_registry_from_env_or_file()
        .map_err(|err| {
            serde_json::json!({
                "code": "analytics_attestation_registry_invalid",
                "message": err.message,
                "source": "attestation_registry",
                "details": { "profile_id": profile_id }
            })
            .to_string()
        })?
        .ok_or_else(|| {
            serde_json::json!({
                "code": "analytics_attestation_registry_missing",
                "message": "Signed attestation policy requires ATTESTATION_KEY_REGISTRY_JSON or ATTESTATION_KEY_REGISTRY_PATH.",
                "source": "attestation_registry",
                "details": { "profile_id": profile_id }
            })
            .to_string()
        })?;

    verify_connector_attestation_with_registry_v1(
        &latest_run.metadata.run_id,
        &latest_run.metadata.run_id,
        attestation,
        &registry,
    )
    .map_err(|err| {
        serde_json::json!({
            "code": "analytics_attestation_signature_invalid",
            "message": err.message,
            "source": "attestation_signature",
            "details": {
                "profile_id": profile_id,
                "run_id": latest_run.metadata.run_id,
                "validation_code": err.code
            }
        })
        .to_string()
    })
}

/// # NDOC
/// component: `tauri_commands::export_executive_dashboard_governed`
/// purpose: Export executive dashboard payload only when release gates and dashboard gates pass, and append audit record.
#[tauri::command]
fn export_executive_dashboard_governed(
    profile_id: String,
    gates: ReleaseGateInput,
    export_format: Option<String>,
    target_ref: Option<String>,
    limit: Option<usize>,
    compare_window_runs: Option<u8>,
    target_roas: Option<f64>,
    monthly_revenue_target: Option<f64>,
) -> Result<Value, String> {
    let gate_validation = validate_release_gates(&gates);
    if !gate_validation.ok {
        return Err(serde_json::json!({
            "code": "analytics_dashboard_release_gates_blocked",
            "message": "Release gate validation failed for governed export.",
            "source": "governance",
            "details": { "errors": gate_validation.errors }
        })
        .to_string());
    }

    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err("profile_id cannot be empty".to_string());
    }
    let options = SnapshotBuildOptions {
        compare_window_runs: compare_window_runs.unwrap_or(1).max(1) as usize,
        target_roas,
        monthly_revenue_target,
    };
    let (snapshot, latest_run) =
        build_dashboard_snapshot_checked(profile_id, limit.unwrap_or(24).min(64), options, true)?;

    let policy = resolve_attestation_policy_v1(profile_id).map_err(|err| {
        serde_json::json!({
            "code": "analytics_attestation_policy_invalid",
            "message": err.message,
            "source": "attestation_policy",
            "details": { "profile_id": profile_id, "field_paths": err.field_paths }
        })
        .to_string()
    })?;
    let signature_present = latest_run
        .metadata
        .connector_attestation
        .fingerprint_signature
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let attestation_verified = if policy.require_signed_attestations {
        true
    } else {
        signature_present
    };
    let export_id = format!("exp-{}-{}", now_millis(), latest_run.metadata.run_id);
    let audit_record = DashboardExportAuditRecordV1 {
        schema_version: String::new(),
        export_id: export_id.clone(),
        profile_id: profile_id.to_string(),
        run_id: latest_run.metadata.run_id.clone(),
        exported_at_utc: String::new(),
        export_format: export_format
            .unwrap_or_else(|| "json".to_string())
            .trim()
            .to_string(),
        target_ref: target_ref
            .unwrap_or_else(|| "operator_download".to_string())
            .trim()
            .to_string(),
        gate_status: snapshot.publish_export_gate.gate_status.clone(),
        publish_ready: snapshot.publish_export_gate.publish_ready,
        export_ready: snapshot.publish_export_gate.export_ready,
        blocking_reasons: snapshot.publish_export_gate.blocking_reasons.clone(),
        warning_reasons: snapshot.publish_export_gate.warning_reasons.clone(),
        attestation_policy_required: policy.require_signed_attestations,
        attestation_verified,
        attestation_key_id: latest_run
            .metadata
            .connector_attestation
            .fingerprint_key_id
            .clone(),
        checked_by: gates.checked_by,
        release_id: gates.release_id,
    };
    let audit_store = DashboardExportAuditStore::default();
    let persisted = audit_store
        .append_record(audit_record)
        .map_err(|err| format!("{}: {}", err.code, err.message))?;

    Ok(json!({
        "export_id": export_id,
        "snapshot": snapshot,
        "audit_record": persisted
    }))
}

fn now_millis() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i128)
        .unwrap_or(0)
}

/// # NDOC
/// component: `tauri_commands::get_dashboard_export_audit_history`
/// purpose: Return governed dashboard export audit history for operator review.
#[tauri::command]
fn get_dashboard_export_audit_history(
    profile_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let store = DashboardExportAuditStore::default();
    let maybe_profile = profile_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let rows = store
        .list_recent(maybe_profile, limit.unwrap_or(50).min(500))
        .map_err(|err| format!("{}: {}", err.code, err.message))?;
    serde_json::to_value(rows)
        .map_err(|err| format!("failed to serialize export audit history: {err}"))
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
            get_tool_audit_report,
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
            validate_analytics_connectors_preflight,
            start_mock_text_workflow_job,
            get_mock_analytics_run_history,
            get_analysis_workflows,
            get_text_workflow_templates,
            get_executive_dashboard_snapshot,
            export_executive_dashboard_governed,
            get_dashboard_export_audit_history,
            get_dashboard_chart_definitions,
            generate_image_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::subsystems::marketing_data_analysis::contracts::{
        AnalyticsRunMetadataV1, AnalyticsValidationReportV1, MockAnalyticsArtifactV1,
        MockAnalyticsRequestV1, MOCK_ANALYTICS_SCHEMA_VERSION_V1,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_mutex() -> &'static Mutex<()> {
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    fn with_temp_env<F>(pairs: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        let previous = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        f();

        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }

    fn build_run(profile_id: &str) -> PersistedAnalyticsRunV1 {
        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: MockAnalyticsRequestV1 {
                start_date: "2026-02-01".to_string(),
                end_date: "2026-02-07".to_string(),
                campaign_filter: None,
                ad_group_filter: None,
                seed: Some(7),
                profile_id: profile_id.to_string(),
                include_narratives: true,
                source_window_observations: Vec::new(),
                budget_envelope:
                    app_core::subsystems::marketing_data_analysis::contracts::BudgetEnvelopeV1::default(
                    ),
            },
            metadata: AnalyticsRunMetadataV1 {
                run_id: "run-1".to_string(),
                connector_id: "mock".to_string(),
                profile_id: profile_id.to_string(),
                seed: 7,
                schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
                date_span_days: 7,
                requested_at_utc: None,
                connector_attestation: Default::default(),
            },
            report: Default::default(),
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["sim".to_string()],
            provenance: Vec::new(),
            ingest_cleaning_notes: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
            freshness_policy: Default::default(),
            reconciliation_policy: Default::default(),
            budget: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };
        artifact.report.total_metrics.impressions = 1000;
        artifact.report.total_metrics.clicks = 100;
        PersistedAnalyticsRunV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: artifact.request.clone(),
            metadata: artifact.metadata.clone(),
            validation: artifact.validation.clone(),
            artifact,
            stored_at_utc: "2026-02-17T00:00:00Z".to_string(),
        }
    }

    fn unique_tmp_path(file_name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nd_marketing_{}_{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&dir).expect("temp dir create");
        dir.join(file_name)
    }

    #[tokio::test]
    async fn analytics_preflight_command_returns_schema() {
        let value = validate_analytics_connectors_preflight(Some(
            AnalyticsConnectorConfigV1::simulated_defaults(),
        ))
        .await
        .expect("preflight command should serialize");
        assert_eq!(
            value
                .get("schema_version")
                .and_then(Value::as_str)
                .unwrap_or(""),
            "analytics_connector_preflight.v1"
        );
    }

    #[tokio::test]
    async fn analytics_preflight_command_rejects_invalid_config() {
        let mut config = AnalyticsConnectorConfigV1::simulated_defaults();
        config.ga4.property_id = "bad".to_string();

        let value = validate_analytics_connectors_preflight(Some(config))
            .await
            .expect("preflight command should serialize");
        assert_eq!(
            value
                .get("config_valid")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            false
        );
        assert!(value
            .get("blocking_reasons")
            .and_then(Value::as_array)
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn dashboard_policy_blocks_missing_signature_for_production() {
        let _guard = env_mutex().lock().expect("env mutex");
        with_temp_env(
            &[
                ("REQUIRE_SIGNED_ATTESTATIONS", None),
                ("ATTESTATION_KEY_REGISTRY_JSON", None),
                ("ATTESTATION_KEY_REGISTRY_PATH", None),
            ],
            || {
                let run = build_run("production");
                let result = enforce_dashboard_attestation_policy("production", &run);
                assert!(result.is_err());
                let err = result.expect_err("must fail");
                assert!(err.contains("analytics_dashboard_signature_required"));
            },
        );
    }

    #[test]
    fn dashboard_policy_blocks_when_registry_missing_with_signed_attestation() {
        let _guard = env_mutex().lock().expect("env mutex");
        with_temp_env(
            &[
                ("REQUIRE_SIGNED_ATTESTATIONS", Some("true")),
                ("ATTESTATION_KEY_REGISTRY_JSON", None),
                ("ATTESTATION_KEY_REGISTRY_PATH", None),
            ],
            || {
                let mut run = build_run("production");
                run.metadata.connector_attestation.fingerprint_signature =
                    Some("ed25519:abc".to_string());
                run.metadata.connector_attestation.fingerprint_key_id = Some("k1".to_string());
                let result = enforce_dashboard_attestation_policy("production", &run);
                assert!(result.is_err());
                let err = result.expect_err("must fail");
                assert!(err.contains("analytics_attestation_registry_missing"));
            },
        );
    }

    #[test]
    fn governed_dashboard_export_rejects_red_release_gate() {
        let gates = ReleaseGateInput {
            release_id: "rel-1".to_string(),
            scope: "internal".to_string(),
            security_gate: "green".to_string(),
            budget_gate: "red".to_string(),
            evidence_gate: "green".to_string(),
            role_gate: "green".to_string(),
            change_gate: "green".to_string(),
            checked_by: "team_lead".to_string(),
            checked_at_utc: "2026-02-26T00:00:00Z".to_string(),
        };
        let result = export_executive_dashboard_governed(
            "production".to_string(),
            gates,
            Some("json".to_string()),
            Some("operator_download".to_string()),
            Some(24),
            Some(1),
            None,
            None,
        );
        assert!(result.is_err());
        let err = result.expect_err("must fail");
        assert!(err.contains("analytics_dashboard_release_gates_blocked"));
    }

    #[test]
    fn governed_dashboard_export_persists_audit_record() {
        let _guard = env_mutex().lock().expect("env mutex");
        let run_path = unique_tmp_path("runs.jsonl");
        let audit_path = unique_tmp_path("export_audit.jsonl");
        with_temp_env(
            &[
                ("REQUIRE_SIGNED_ATTESTATIONS", Some("false")),
                (
                    "ANALYTICS_RUN_STORE_PATH",
                    Some(run_path.to_string_lossy().as_ref()),
                ),
                (
                    "ANALYTICS_EXPORT_AUDIT_STORE_PATH",
                    Some(audit_path.to_string_lossy().as_ref()),
                ),
                ("ATTESTATION_KEY_REGISTRY_JSON", None),
                ("ATTESTATION_KEY_REGISTRY_PATH", None),
            ],
            || {
                let store = AnalyticsRunStore::new(run_path.clone());
                store
                    .append_run(build_run("dev"))
                    .expect("append test run for export");

                let gates = ReleaseGateInput {
                    release_id: "rel-2".to_string(),
                    scope: "internal".to_string(),
                    security_gate: "green".to_string(),
                    budget_gate: "green".to_string(),
                    evidence_gate: "green".to_string(),
                    role_gate: "green".to_string(),
                    change_gate: "green".to_string(),
                    checked_by: "team_lead".to_string(),
                    checked_at_utc: "2026-02-26T00:00:00Z".to_string(),
                };
                let result = export_executive_dashboard_governed(
                    "dev".to_string(),
                    gates,
                    Some("json".to_string()),
                    Some("operator_download".to_string()),
                    Some(24),
                    Some(1),
                    None,
                    None,
                )
                .expect("governed export");

                assert!(result.get("audit_record").is_some());
                let audit_store = DashboardExportAuditStore::new(audit_path.clone());
                let rows = audit_store
                    .list_recent(Some("dev"), 10)
                    .expect("audit list");
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].release_id, "rel-2");
                assert!(rows[0].export_ready);
            },
        );
    }
}
