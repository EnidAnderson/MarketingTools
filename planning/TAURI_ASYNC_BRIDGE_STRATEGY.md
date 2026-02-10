# Tauri Async Bridge Strategy for Pure Rust Tools

Last updated: 2026-02-09

## Purpose
Provide a concrete strategy for exposing pure Rust tools as async operations in the Tauri app without coupling business logic to UI transport concerns.

## 1) Problem Statement
Current command handlers mix direct execution and ad-hoc behavior. This creates risk for:
1. UI blocking on long-running calls.
2. Inconsistent error/progress handling.
3. Tight coupling of frontend to individual commands.

## 2) Target Interface
Expose two stable command classes:
1. `start_tool_job(tool_name, input_json) -> JobHandle`
2. `get_job(job_id) -> JobSnapshot`
3. `cancel_job(job_id) -> ()`
4. optional: `list_tools() -> ToolDefinition[]`

Emit events for realtime updates:
- `tool-job-progress`
- `tool-job-completed`
- `tool-job-failed`

## 3) Runtime Pattern

### Step A: Typed decode at edge
- Command receives `serde_json::Value`.
- Registry resolves tool and decodes input into typed struct.
- Validation errors return immediately.

### Step B: Spawn in background
- Use `tauri::async_runtime::spawn`.
- Pass a cancellation token and progress sender.
- Never run blocking operations directly on async executor.

### Step C: Store lifecycle state
- Keep in `AppState` with `Arc<RwLock<HashMap<JobId, JobState>>>`.
- Update state transitions atomically.

### Step D: Push UI updates
- Emit progress events via `app_handle.emit_all` (or equivalent Tauri v2 API).
- Frontend updates progress bar/log feed.

## 4) Suggested Rust Types

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct JobHandle {
    pub job_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub job_id: String,
    pub status: JobStatus,
    pub progress_pct: u8,
    pub stage: String,
    pub message: Option<String>,
    pub output: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}
```

## 5) Trait for Progress-Capable Tools

```rust
#[async_trait::async_trait]
pub trait ExecutableTool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(
        &self,
        ctx: ToolExecutionContext,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError>;
}

pub struct ToolExecutionContext {
    pub job_id: String,
    pub cancel: tokio_util::sync::CancellationToken,
    pub progress: ProgressReporter,
}
```

## 6) Blocking Work Policy
If a tool requires blocking I/O or CPU-bound work:
1. Wrap with `tokio::task::spawn_blocking`.
2. Preserve cancellation checks between phases.
3. Keep external calls under timeout and retry policy.

## 7) Frontend Integration Contract
Frontend should:
1. Fetch tool definitions from backend on load.
2. Render inputs from schema metadata.
3. Submit tool jobs via `start_tool_job`.
4. Subscribe to progress events and reconcile with `get_job` polling fallback.

## 8) Compatibility Bridge During Migration
Short-term:
1. Keep existing `run_tool` command as shim.
2. Internally call job runtime for consistency.
3. Mark old direct commands (`generate_image_command`) as deprecated wrappers.

Mid-term:
1. Remove single-purpose commands once UI consumes generic job API.
2. Keep only tool registry + job APIs.

## 9) Reliability Controls
1. Add per-tool timeout defaults.
2. Add provider-specific retries with exponential backoff.
3. Add queue depth and concurrency limits.
4. Add cancellation everywhere possible.

## 10) Testing Strategy
1. Unit tests for job manager state transitions.
2. Contract tests for registry input/output decode.
3. Integration tests for event emission and job completion.
4. Failure-path tests:
- invalid input,
- provider timeout,
- user cancellation,
- artifact write failure.

## 11) Migration Deliverables
1. `src-tauri/src/runtime/job_manager.rs` + tests.
2. `src-tauri/src/state/app_state.rs`.
3. Generic commands wired in `src-tauri/src/lib.rs`.
4. Frontend migration away from hardcoded tool list in `frontend/main.js`.

## 12) Exit Criteria
1. All tool execution goes through job manager.
2. UI can run, monitor, and cancel any tool without bespoke frontend code.
3. Legacy direct-invoke command paths removed.
