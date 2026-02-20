use app_core::pipeline::{execute_pipeline, PipelineDefinition};
use app_core::subsystems::campaign_orchestration::runtime::{
    run_prioritized_text_workflow_v1, TextWorkflowRunRequestV1,
};
use app_core::subsystems::marketing_data_analysis::{
    build_historical_analysis, AnalyticsRunStore, DefaultMarketAnalysisService, GuidanceItem,
    MarketAnalysisService, MockAnalyticsRequestV1, PersistedAnalyticsRunV1,
};
use app_core::tools::tool_registry::ToolRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration, Instant};

/// # NDOC
/// component: `tauri_runtime::jobs`
/// purpose: Canonical state machine for async tool jobs.
/// invariants:
///   - Terminal states are `Succeeded`, `Failed`, `Canceled`.
///   - `Succeeded` implies `output.is_some()` and `error.is_none()`.
///   - `Failed`/`Canceled` imply `error.is_some()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

/// # NDOC
/// component: `tauri_runtime::jobs`
/// purpose: User-facing snapshot returned to frontend polling and events.
/// invariants:
///   - `progress_pct` must be in `0..=100`.
///   - `job_id` and `tool_name` are never empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub job_id: String,
    pub tool_name: String,
    pub status: JobStatus,
    pub progress_pct: u8,
    pub stage: String,
    pub message: Option<String>,
    pub output: Option<Value>,
    pub error: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobHandle {
    pub job_id: String,
}

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<RwLock<HashMap<String, JobSnapshot>>>,
    canceled: Arc<RwLock<HashSet<String>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            canceled: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn start_tool_job(
        &self,
        app_handle: &AppHandle,
        tool_name: String,
        input: Value,
    ) -> Result<JobHandle, String> {
        if tool_name.trim().is_empty() {
            return Err("tool_name cannot be empty".to_string());
        }
        if !input.is_object() {
            return Err("input must be a JSON object".to_string());
        }

        let registry = ToolRegistry::new();
        if registry.get_tool_instance(&tool_name).is_none() {
            return Err(format!("Tool '{}' not found or not available.", tool_name));
        }

        let job_id = next_job_id();
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            tool_name: tool_name.clone(),
            status: JobStatus::Queued,
            progress_pct: 0,
            stage: "queued".to_string(),
            message: Some("Job accepted".to_string()),
            output: None,
            error: None,
        };

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| "Failed to acquire write lock for jobs".to_string())?;
            jobs.insert(job_id.clone(), snapshot);
        }
        self.assert_snapshot_invariant(&job_id);

        self.emit_progress(app_handle, &job_id);

        let manager = self.clone();
        let app_handle = app_handle.clone();
        let spawned_job_id = job_id.clone();
        tauri::async_runtime::spawn(async move {
            manager.update_running(&spawned_job_id);
            manager.assert_snapshot_invariant(&spawned_job_id);
            manager.emit_progress(&app_handle, &spawned_job_id);

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Job canceled before execution");
                manager.assert_snapshot_invariant(&spawned_job_id);
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            let registry = ToolRegistry::new();
            if registry.get_tool_instance(&tool_name).is_none() {
                manager.update_failed(
                    &spawned_job_id,
                    serde_json::json!({
                        "code": "internal_error",
                        "category": "internal",
                        "source": "runtime",
                        "message": "Tool became unavailable before execution",
                        "retryable": false,
                        "field_paths": [],
                        "trace_id": "",
                        "context": {}
                    }),
                );
                manager.assert_snapshot_invariant(&spawned_job_id);
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            match registry.execute_tool(&tool_name, input).await {
                Ok(output) => {
                    if manager.is_canceled(&spawned_job_id) {
                        manager.update_canceled(&spawned_job_id, "Job canceled during execution");
                        manager.assert_snapshot_invariant(&spawned_job_id);
                        manager.emit_failed(&app_handle, &spawned_job_id);
                    } else {
                        manager.update_succeeded(&spawned_job_id, output);
                        manager.assert_snapshot_invariant(&spawned_job_id);
                        manager.emit_completed(&app_handle, &spawned_job_id);
                    }
                }
                Err(err) => {
                    manager.update_failed(
                        &spawned_job_id,
                        serde_json::json!({
                            "code": err.code,
                            "category": err.category,
                            "source": err.source,
                            "message": err.message,
                            "retryable": err.retryable,
                            "field_paths": err.field_paths,
                            "trace_id": err.trace_id,
                            "context": err.context
                        }),
                    );
                    manager.assert_snapshot_invariant(&spawned_job_id);
                    manager.emit_failed(&app_handle, &spawned_job_id);
                }
            }
        });

        Ok(JobHandle { job_id })
    }

    /// # NDOC
    /// component: `tauri_runtime::jobs::start_pipeline_job`
    /// purpose: Start asynchronous pipeline execution as a managed job.
    /// invariants:
    ///   - Uses the same snapshot state machine as tool jobs.
    ///   - `tool_name` field stores `pipeline::<pipeline_name>` for UI compatibility.
    pub fn start_pipeline_job(
        &self,
        app_handle: &AppHandle,
        definition: PipelineDefinition,
    ) -> Result<JobHandle, String> {
        if definition.name.trim().is_empty() {
            return Err("pipeline name cannot be empty".to_string());
        }

        let job_id = next_job_id();
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            tool_name: format!("pipeline::{}", definition.name),
            status: JobStatus::Queued,
            progress_pct: 0,
            stage: "queued".to_string(),
            message: Some("Pipeline job accepted".to_string()),
            output: None,
            error: None,
        };

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| "Failed to acquire write lock for jobs".to_string())?;
            jobs.insert(job_id.clone(), snapshot);
        }
        self.assert_snapshot_invariant(&job_id);
        self.emit_progress(app_handle, &job_id);

        let manager = self.clone();
        let app_handle = app_handle.clone();
        let spawned_job_id = job_id.clone();
        let manifest_path = definition.output_manifest_path.clone();
        tauri::async_runtime::spawn(async move {
            manager.update_running(&spawned_job_id);
            manager.assert_snapshot_invariant(&spawned_job_id);
            manager.emit_progress(&app_handle, &spawned_job_id);

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Pipeline canceled before execution");
                manager.assert_snapshot_invariant(&spawned_job_id);
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            match execute_pipeline(definition).await {
                Ok(result) => {
                    if manager.is_canceled(&spawned_job_id) {
                        manager
                            .update_canceled(&spawned_job_id, "Pipeline canceled during execution");
                        manager.assert_snapshot_invariant(&spawned_job_id);
                        manager.emit_failed(&app_handle, &spawned_job_id);
                    } else {
                        match serde_json::to_value(result) {
                            Ok(output) => {
                                if let Some(path) = manifest_path.as_deref() {
                                    if let Err(err) = write_pipeline_manifest(path, &output) {
                                        manager.update_failed(
                                            &spawned_job_id,
                                            serde_json::json!({
                                                "code": "internal_error",
                                                "category": "internal",
                                                "source": "storage",
                                                "message": format!("Failed to write pipeline manifest: {}", err),
                                                "retryable": false,
                                                "field_paths": [],
                                                "trace_id": "",
                                                "context": {}
                                            }),
                                        );
                                        manager.assert_snapshot_invariant(&spawned_job_id);
                                        manager.emit_failed(&app_handle, &spawned_job_id);
                                        return;
                                    }
                                }
                                manager.update_succeeded(&spawned_job_id, output);
                                manager.assert_snapshot_invariant(&spawned_job_id);
                                manager.emit_completed(&app_handle, &spawned_job_id);
                            }
                            Err(err) => {
                                manager.update_failed(
                                    &spawned_job_id,
                                    serde_json::json!({
                                        "code": "internal_error",
                                        "category": "internal",
                                        "source": "runtime",
                                        "message": format!("Failed to serialize pipeline result: {}", err),
                                        "retryable": false,
                                        "field_paths": [],
                                        "trace_id": "",
                                        "context": {}
                                    }),
                                );
                                manager.assert_snapshot_invariant(&spawned_job_id);
                                manager.emit_failed(&app_handle, &spawned_job_id);
                            }
                        }
                    }
                }
                Err(err) => {
                    manager.update_failed(
                        &spawned_job_id,
                        serde_json::json!({
                            "code": format!("{:?}", err.kind),
                            "category": "internal",
                            "source": "runtime",
                            "message": err.message,
                            "retryable": err.retryable,
                            "field_paths": [],
                            "trace_id": "",
                            "context": err.details
                        }),
                    );
                    manager.assert_snapshot_invariant(&spawned_job_id);
                    manager.emit_failed(&app_handle, &spawned_job_id);
                }
            }
        });

        Ok(JobHandle { job_id })
    }

    /// # NDOC
    /// component: `tauri_runtime::jobs::start_mock_analytics_job`
    /// purpose: Start deterministic mock analytics run as a managed async job.
    /// invariants:
    ///   - Uses stage names: validating_input -> generating_data -> assembling_report -> validating_invariants -> completed.
    pub fn start_mock_analytics_job(
        &self,
        app_handle: &AppHandle,
        request: MockAnalyticsRequestV1,
    ) -> Result<JobHandle, String> {
        let job_id = next_job_id();
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            tool_name: "analytics::mock_pipeline".to_string(),
            status: JobStatus::Queued,
            progress_pct: 0,
            stage: "queued".to_string(),
            message: Some("Mock analytics job accepted".to_string()),
            output: None,
            error: None,
        };

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| "Failed to acquire write lock for jobs".to_string())?;
            jobs.insert(job_id.clone(), snapshot);
        }
        self.assert_snapshot_invariant(&job_id);
        self.emit_progress(app_handle, &job_id);

        let manager = self.clone();
        let app_handle = app_handle.clone();
        let spawned_job_id = job_id.clone();
        tauri::async_runtime::spawn(async move {
            manager.update_running(&spawned_job_id);
            manager.update_stage(
                &spawned_job_id,
                "validating_input",
                15,
                "Validating mock analytics request",
            );
            manager.assert_snapshot_invariant(&spawned_job_id);
            manager.emit_progress(&app_handle, &spawned_job_id);

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Job canceled before generation");
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            manager.update_stage(
                &spawned_job_id,
                "generating_data",
                40,
                "Generating deterministic mock analytics data",
            );
            manager.emit_progress(&app_handle, &spawned_job_id);
            sleep(Duration::from_millis(150)).await;

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Job canceled during generating_data");
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            manager.update_stage(
                &spawned_job_id,
                "assembling_report",
                70,
                "Assembling analytics report artifact",
            );
            manager.emit_progress(&app_handle, &spawned_job_id);

            let service = DefaultMarketAnalysisService::new();
            match service.run_mock_analysis(request).await {
                Ok(mut artifact) => {
                    manager.update_stage(
                        &spawned_job_id,
                        "validating_invariants",
                        90,
                        "Validating artifact invariants",
                    );
                    manager.emit_progress(&app_handle, &spawned_job_id);

                    if manager.is_canceled(&spawned_job_id) {
                        manager.update_canceled(&spawned_job_id, "Job canceled before completion");
                        manager.emit_failed(&app_handle, &spawned_job_id);
                        return;
                    }

                    manager.update_stage(
                        &spawned_job_id,
                        "historical_analysis",
                        94,
                        "Computing longitudinal trend and drift analysis",
                    );
                    manager.emit_progress(&app_handle, &spawned_job_id);

                    let store = AnalyticsRunStore::default();
                    let history = match store.list_recent(Some(&artifact.request.profile_id), 64) {
                        Ok(values) => values,
                        Err(err) => {
                            manager.update_failed(
                                &spawned_job_id,
                                serde_json::json!({
                                    "code": err.code,
                                    "category": "internal",
                                    "source": "storage",
                                    "message": err.message,
                                    "retryable": false,
                                    "field_paths": err.field_paths,
                                    "trace_id": "",
                                    "context": err.context
                                }),
                            );
                            manager.emit_failed(&app_handle, &spawned_job_id);
                            return;
                        }
                    };

                    artifact.historical_analysis = build_historical_analysis(&artifact, &history);
                    apply_confidence_calibration(
                        &mut artifact.inferred_guidance,
                        &artifact
                            .historical_analysis
                            .confidence_calibration
                            .recommended_confidence_cap,
                    );

                    manager.update_stage(
                        &spawned_job_id,
                        "persisting_artifact",
                        98,
                        "Persisting analytics run and artifact",
                    );
                    manager.emit_progress(&app_handle, &spawned_job_id);

                    let to_persist = PersistedAnalyticsRunV1 {
                        schema_version: artifact.schema_version.clone(),
                        request: artifact.request.clone(),
                        metadata: artifact.metadata.clone(),
                        validation: artifact.validation.clone(),
                        artifact: artifact.clone(),
                        stored_at_utc: String::new(),
                    };
                    match store.append_run(to_persist) {
                        Ok(saved) => {
                            artifact.persistence = Some(app_core::subsystems::marketing_data_analysis::ArtifactPersistenceRefV1 {
                                stored_at_utc: saved.stored_at_utc,
                                storage_path: store.path().to_string_lossy().to_string(),
                            });
                        }
                        Err(err) => {
                            manager.update_failed(
                                &spawned_job_id,
                                serde_json::json!({
                                    "code": err.code,
                                    "category": "internal",
                                    "source": "storage",
                                    "message": err.message,
                                    "retryable": false,
                                    "field_paths": err.field_paths,
                                    "trace_id": "",
                                    "context": err.context
                                }),
                            );
                            manager.emit_failed(&app_handle, &spawned_job_id);
                            return;
                        }
                    }

                    match serde_json::to_value(artifact) {
                        Ok(output) => {
                            manager.update_succeeded(&spawned_job_id, output);
                            manager.assert_snapshot_invariant(&spawned_job_id);
                            manager.emit_completed(&app_handle, &spawned_job_id);
                        }
                        Err(err) => {
                            manager.update_failed(
                                &spawned_job_id,
                                serde_json::json!({
                                    "code": "internal_error",
                                    "category": "internal",
                                    "source": "runtime",
                                    "message": format!("Failed to serialize artifact: {}", err),
                                    "retryable": false,
                                    "field_paths": [],
                                    "trace_id": "",
                                    "context": {}
                                }),
                            );
                            manager.emit_failed(&app_handle, &spawned_job_id);
                        }
                    }
                }
                Err(err) => {
                    manager.update_failed(
                        &spawned_job_id,
                        serde_json::json!({
                            "code": err.code,
                            "category": "internal",
                            "source": "tool",
                            "message": err.message,
                            "retryable": false,
                            "field_paths": err.field_paths,
                            "trace_id": "",
                            "context": err.context
                        }),
                    );
                    manager.assert_snapshot_invariant(&spawned_job_id);
                    manager.emit_failed(&app_handle, &spawned_job_id);
                }
            }
        });

        Ok(JobHandle { job_id })
    }

    /// # NDOC
    /// component: `tauri_runtime::jobs::start_mock_text_workflow_job`
    /// purpose: Start deterministic mock text workflow execution with graph-based orchestration.
    /// invariants:
    ///   - Uses stage names: validating_graph -> planning_routes -> generating_artifact -> evaluating_gate -> completed.
    pub fn start_mock_text_workflow_job(
        &self,
        app_handle: &AppHandle,
        request: TextWorkflowRunRequestV1,
    ) -> Result<JobHandle, String> {
        let job_id = next_job_id();
        let snapshot = JobSnapshot {
            job_id: job_id.clone(),
            tool_name: "text::workflow_pipeline".to_string(),
            status: JobStatus::Queued,
            progress_pct: 0,
            stage: "queued".to_string(),
            message: Some("Mock text workflow job accepted".to_string()),
            output: None,
            error: None,
        };

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| "Failed to acquire write lock for jobs".to_string())?;
            jobs.insert(job_id.clone(), snapshot);
        }
        self.assert_snapshot_invariant(&job_id);
        self.emit_progress(app_handle, &job_id);

        let manager = self.clone();
        let app_handle = app_handle.clone();
        let spawned_job_id = job_id.clone();
        tauri::async_runtime::spawn(async move {
            manager.update_running(&spawned_job_id);
            manager.update_stage(
                &spawned_job_id,
                "validating_graph",
                20,
                "Validating text workflow graph template",
            );
            manager.emit_progress(&app_handle, &spawned_job_id);

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Job canceled before route planning");
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            manager.update_stage(
                &spawned_job_id,
                "planning_routes",
                45,
                "Planning token and model routing under budget envelope",
            );
            manager.emit_progress(&app_handle, &spawned_job_id);
            sleep(Duration::from_millis(80)).await;

            if manager.is_canceled(&spawned_job_id) {
                manager.update_canceled(&spawned_job_id, "Job canceled during route planning");
                manager.emit_failed(&app_handle, &spawned_job_id);
                return;
            }

            manager.update_stage(
                &spawned_job_id,
                "generating_artifact",
                78,
                "Generating deterministic workflow artifact",
            );
            manager.emit_progress(&app_handle, &spawned_job_id);

            match run_prioritized_text_workflow_v1(request) {
                Ok(result) => {
                    manager.update_stage(
                        &spawned_job_id,
                        "evaluating_gate",
                        92,
                        "Evaluating weighted quality/safety gate",
                    );
                    manager.emit_progress(&app_handle, &spawned_job_id);

                    match serde_json::to_value(result) {
                        Ok(output) => {
                            manager.update_succeeded(&spawned_job_id, output);
                            manager.assert_snapshot_invariant(&spawned_job_id);
                            manager.emit_completed(&app_handle, &spawned_job_id);
                        }
                        Err(err) => {
                            manager.update_failed(
                                &spawned_job_id,
                                serde_json::json!({
                                    "code": "internal_error",
                                    "category": "internal",
                                    "source": "runtime",
                                    "message": format!("Failed to serialize text workflow result: {}", err),
                                    "retryable": false,
                                    "field_paths": [],
                                    "trace_id": "",
                                    "context": {}
                                }),
                            );
                            manager.assert_snapshot_invariant(&spawned_job_id);
                            manager.emit_failed(&app_handle, &spawned_job_id);
                        }
                    }
                }
                Err(err) => {
                    manager.update_failed(
                        &spawned_job_id,
                        serde_json::json!({
                            "code": "validation_error",
                            "category": "validation",
                            "source": "orchestration",
                            "message": err,
                            "retryable": false,
                            "field_paths": [],
                            "trace_id": "",
                            "context": {}
                        }),
                    );
                    manager.assert_snapshot_invariant(&spawned_job_id);
                    manager.emit_failed(&app_handle, &spawned_job_id);
                }
            }
        });

        Ok(JobHandle { job_id })
    }

    pub fn get_job(&self, job_id: &str) -> Option<JobSnapshot> {
        self.jobs
            .read()
            .ok()
            .and_then(|jobs| jobs.get(job_id).cloned())
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<(), String> {
        {
            let mut canceled = self
                .canceled
                .write()
                .map_err(|_| "Failed to acquire write lock for canceled jobs".to_string())?;
            canceled.insert(job_id.to_string());
        }

        {
            let mut jobs = self
                .jobs
                .write()
                .map_err(|_| "Failed to acquire write lock for jobs".to_string())?;

            let Some(snapshot) = jobs.get_mut(job_id) else {
                return Err(format!("Job '{}' not found.", job_id));
            };

            if matches!(snapshot.status, JobStatus::Queued | JobStatus::Running) {
                snapshot.status = JobStatus::Canceled;
                snapshot.progress_pct = snapshot.progress_pct.min(99);
                snapshot.stage = "canceled".to_string();
                snapshot.message = Some("Cancellation requested".to_string());
                snapshot.error = Some(serde_json::json!({
                    "code": "canceled",
                    "category": "internal",
                    "source": "runtime",
                    "message": "Job canceled by user",
                    "retryable": false,
                    "field_paths": [],
                    "trace_id": "",
                    "context": {}
                }));
            }
        }
        self.assert_snapshot_invariant(job_id);

        Ok(())
    }

    pub async fn wait_for_terminal_state(
        &self,
        job_id: &str,
        timeout: Duration,
    ) -> Result<JobSnapshot, String> {
        let start = Instant::now();

        loop {
            let Some(snapshot) = self.get_job(job_id) else {
                return Err(format!("Job '{}' not found.", job_id));
            };

            if matches!(
                snapshot.status,
                JobStatus::Succeeded | JobStatus::Failed | JobStatus::Canceled
            ) {
                return Ok(snapshot);
            }

            if start.elapsed() >= timeout {
                return Err(format!(
                    "Timed out waiting for job '{}' to complete.",
                    job_id
                ));
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    fn is_canceled(&self, job_id: &str) -> bool {
        self.canceled
            .read()
            .map(|canceled| canceled.contains(job_id))
            .unwrap_or(false)
    }

    fn update_running(&self, job_id: &str) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(snapshot) = jobs.get_mut(job_id) {
                snapshot.status = JobStatus::Running;
                snapshot.progress_pct = 10;
                snapshot.stage = "running".to_string();
                snapshot.message = Some("Execution started".to_string());
            }
        }
    }

    fn update_succeeded(&self, job_id: &str, output: Value) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(snapshot) = jobs.get_mut(job_id) {
                snapshot.status = JobStatus::Succeeded;
                snapshot.progress_pct = 100;
                snapshot.stage = "completed".to_string();
                snapshot.message = Some("Execution completed".to_string());
                snapshot.output = Some(output);
                snapshot.error = None;
            }
        }
    }

    fn update_failed(&self, job_id: &str, error: Value) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(snapshot) = jobs.get_mut(job_id) {
                snapshot.status = JobStatus::Failed;
                snapshot.progress_pct = snapshot.progress_pct.min(99);
                snapshot.stage = "failed".to_string();
                snapshot.message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                snapshot.error = Some(error);
            }
        }
    }

    fn update_stage(&self, job_id: &str, stage: &str, progress_pct: u8, message: &str) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(snapshot) = jobs.get_mut(job_id) {
                snapshot.status = JobStatus::Running;
                snapshot.stage = stage.to_string();
                snapshot.progress_pct = progress_pct.min(99);
                snapshot.message = Some(message.to_string());
            }
        }
    }

    fn update_canceled(&self, job_id: &str, message: &str) {
        if let Ok(mut jobs) = self.jobs.write() {
            if let Some(snapshot) = jobs.get_mut(job_id) {
                snapshot.status = JobStatus::Canceled;
                snapshot.progress_pct = snapshot.progress_pct.min(99);
                snapshot.stage = "canceled".to_string();
                snapshot.message = Some(message.to_string());
                snapshot.error = Some(serde_json::json!({
                    "code": "canceled",
                    "category": "internal",
                    "source": "runtime",
                    "message": message,
                    "retryable": false,
                    "field_paths": [],
                    "trace_id": "",
                    "context": {}
                }));
            }
        }
    }

    fn emit_progress(&self, app_handle: &AppHandle, job_id: &str) {
        if let Some(snapshot) = self.get_job(job_id) {
            let _ = app_handle.emit("tool-job-progress", snapshot);
        }
    }

    fn emit_completed(&self, app_handle: &AppHandle, job_id: &str) {
        if let Some(snapshot) = self.get_job(job_id) {
            let _ = app_handle.emit("tool-job-completed", snapshot);
        }
    }

    fn emit_failed(&self, app_handle: &AppHandle, job_id: &str) {
        if let Some(snapshot) = self.get_job(job_id) {
            let _ = app_handle.emit("tool-job-failed", snapshot);
        }
    }

    fn assert_snapshot_invariant(&self, job_id: &str) {
        if let Some(snapshot) = self.get_job(job_id) {
            debug_assert!(
                validate_job_snapshot(&snapshot).is_ok(),
                "invalid job snapshot invariant for {}",
                job_id
            );
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

fn next_job_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("job-{}-{}", ts, counter)
}

fn validate_job_snapshot(snapshot: &JobSnapshot) -> Result<(), String> {
    if snapshot.job_id.trim().is_empty() {
        return Err("job_id cannot be empty".to_string());
    }
    if snapshot.tool_name.trim().is_empty() {
        return Err("tool_name cannot be empty".to_string());
    }
    if snapshot.progress_pct > 100 {
        return Err("progress_pct cannot exceed 100".to_string());
    }

    match snapshot.status {
        JobStatus::Succeeded => {
            if snapshot.output.is_none() || snapshot.error.is_some() {
                return Err("succeeded state requires output and no error".to_string());
            }
            if snapshot.progress_pct != 100 {
                return Err("succeeded state requires progress_pct = 100".to_string());
            }
        }
        JobStatus::Failed | JobStatus::Canceled => {
            let Some(error) = snapshot.error.as_ref() else {
                return Err("failed/canceled state requires error payload".to_string());
            };
            for required in [
                "code",
                "category",
                "source",
                "message",
                "retryable",
                "field_paths",
                "trace_id",
                "context",
            ] {
                if error.get(required).is_none() {
                    return Err(format!(
                        "failed/canceled state error payload missing key '{}'",
                        required
                    ));
                }
            }
        }
        JobStatus::Queued | JobStatus::Running => {}
    }
    Ok(())
}

fn apply_confidence_calibration(guidance: &mut [GuidanceItem], cap: &str) {
    for item in guidance.iter_mut() {
        let normalized = item.confidence_label.to_ascii_lowercase();
        let adjusted = match cap {
            "low" => "low".to_string(),
            "medium" => {
                if normalized == "high" {
                    "medium".to_string()
                } else {
                    normalized
                }
            }
            _ => normalized,
        };
        item.confidence_label = adjusted;
        item.calibration_band = Some(cap.to_string());
    }
}

fn write_pipeline_manifest(path: &str, output: &Value) -> Result<(), String> {
    let manifest = serde_json::to_string_pretty(output).map_err(|e| e.to_string())?;
    let manifest_path = std::path::Path::new(path);
    if let Some(parent) = manifest_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(manifest_path, manifest).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canceling_missing_job_returns_error() {
        let manager = JobManager::new();
        let result = manager.cancel_job("missing-job");
        assert!(result.is_err());
    }

    #[test]
    fn validates_snapshot_invariants() {
        let ok = JobSnapshot {
            job_id: "job-1".to_string(),
            tool_name: "competitive_analysis".to_string(),
            status: JobStatus::Succeeded,
            progress_pct: 100,
            stage: "completed".to_string(),
            message: None,
            output: Some(serde_json::json!({"ok": true})),
            error: None,
        };
        assert!(validate_job_snapshot(&ok).is_ok());

        let bad = JobSnapshot { output: None, ..ok };
        assert!(validate_job_snapshot(&bad).is_err());
    }

    #[test]
    fn failed_snapshot_requires_typed_error_envelope_shape() {
        let snapshot = JobSnapshot {
            job_id: "job-typed-error".to_string(),
            tool_name: "echo_tool".to_string(),
            status: JobStatus::Failed,
            progress_pct: 50,
            stage: "failed".to_string(),
            message: Some("failed".to_string()),
            output: None,
            error: Some(serde_json::json!({
                "code": "validation_error",
                "category": "validation",
                "source": "tool",
                "message": "missing message",
                "retryable": false,
                "field_paths": ["/message"],
                "trace_id": "trace-test",
                "context": {}
            })),
        };
        assert!(validate_job_snapshot(&snapshot).is_ok());
    }

    #[test]
    fn cancel_mock_analytics_during_generating_data() {
        let manager = JobManager::new();
        let job_id = "job-generating-data".to_string();
        {
            let mut jobs = manager.jobs.write().expect("lock");
            jobs.insert(
                job_id.clone(),
                JobSnapshot {
                    job_id: job_id.clone(),
                    tool_name: "analytics::mock_pipeline".to_string(),
                    status: JobStatus::Running,
                    progress_pct: 40,
                    stage: "generating_data".to_string(),
                    message: Some("Generating deterministic mock analytics data".to_string()),
                    output: None,
                    error: None,
                },
            );
        }

        manager.cancel_job(&job_id).expect("cancel should succeed");
        let snapshot = manager.get_job(&job_id).expect("snapshot should exist");
        assert_eq!(snapshot.status, JobStatus::Canceled);
        assert_eq!(snapshot.stage, "canceled");
    }
}
