# Agent Platform Target Architecture (Rust-First)

Last updated: 2026-02-09

## 1) Objective
Design a durable architecture for Nature's Diet marketing agents where core capabilities are implemented in Rust and consumed by Tauri through stable async interfaces.

## 2) Core Principles
1. Domain logic lives in `app_core`; UI transport lives in `src-tauri`.
2. All tools have explicit input/output contracts with versioning.
3. Long-running work is modeled as jobs, not direct synchronous return paths.
4. Platform behavior is observable by default (logs, metrics, trace IDs, artifacts).
5. Backwards compatibility is managed intentionally, then removed on schedule.

## 3) Proposed Layered Design

### Layer A: Domain + Tools (`app_core`)
Responsibilities:
- Tool trait and typed contracts.
- Agent workflow engine (future LangGraph-equivalent orchestration in Rust).
- Provider adapters (Gemini, email, social APIs, etc.).
- Validation, policy checks, and error taxonomy.

Key modules to add/refactor:
- `contracts/`: typed `Request`, `Response`, and `Error` types.
- `registry/`: tool metadata, version, capabilities, and policy requirements.
- `execution/`: job execution context, cancellation token, progress events.
- `providers/`: external API clients behind trait interfaces.

### Layer B: Application Runtime (`src-tauri`)
Responsibilities:
- Tauri command handlers.
- Job scheduling and lifecycle state machine.
- Frontend-safe serialization and event streaming.
- Filesystem and secret access policy enforcement.

Key modules to add/refactor:
- `commands/tool_commands.rs`
- `runtime/job_manager.rs`
- `runtime/event_bus.rs`
- `state/app_state.rs`

### Layer C: Frontend (`frontend`)
Responsibilities:
- Dynamic tool discovery and form rendering.
- Start/cancel/retry tool jobs.
- Render progress/log stream and output artifacts.

## 4) Current-to-Target Delta
1. Current: ad-hoc command set and hardcoded frontend tool list.
- Target: registry-driven dynamic command surface.
2. Current: `serde_json::Value` only.
- Target: typed Rust contracts with JSON adapters.
3. Current: direct command execution for long jobs.
- Target: async job handles with progress/cancel support.
4. Current: Python fallback path exists and is mixed into core.
- Target: compatibility module isolated and eventually removed.

## 5) Canonical Tool Contract Model

```rust
pub trait ToolContract {
    const NAME: &'static str;
    const VERSION: &'static str;
    type Input: serde::de::DeserializeOwned + Send + Sync + 'static;
    type Output: serde::Serialize + Send + Sync + 'static;
}

#[async_trait::async_trait]
pub trait Tool<I, O>: Send + Sync {
    async fn run(&self, ctx: ToolContext, input: I) -> Result<O, ToolError>;
}
```

Tool metadata must include:
1. `name`, `version`, `description`.
2. `input_schema` and `output_schema` (for UI generation).
3. `estimated_runtime`, `supports_cancel`, `produces_artifacts`.
4. `required_secrets`, `required_permissions`.

## 6) Job Runtime Model
Job states:
1. `Queued`
2. `Running`
3. `Succeeded`
4. `Failed`
5. `Canceled`

Job record fields:
1. `job_id`, `tool_name`, `tool_version`.
2. `submitted_at`, `started_at`, `finished_at`.
3. `progress_pct`, `stage`, `message`.
4. `result_payload` or `error_payload`.
5. `artifacts[]` with canonical URIs/paths.

## 7) Error Taxonomy
Use stable machine-readable categories:
1. `ValidationError`
2. `ConfigurationError`
3. `ProviderError`
4. `RateLimitError`
5. `TimeoutError`
6. `PermissionError`
7. `InternalError`

Each error should include:
- user-safe message,
- debug message,
- retryability,
- provider/tool context.

## 8) Security and Policy Boundaries
1. Filesystem access must be scoped to campaign/workspace roots.
2. Secret resolution through provider abstraction; avoid ad-hoc env lookups in tool logic.
3. Command execution tools (if any) must have allowlisted binaries/arguments.
4. Output artifacts should be indexed and auditable.

## 9) Observability Baseline
1. Structured logs with `job_id`, `tool_name`, `campaign_id`.
2. Metrics:
- invocation count,
- success/failure,
- duration,
- external API latency.
3. Optional tracing hooks for end-to-end campaign runs.

## 10) Data and Artifact Governance
1. Every tool output should include a compact artifact manifest.
2. Campaign runs get immutable run IDs.
3. Generated assets include metadata file:
- prompt,
- tool version,
- timestamp,
- source references.

## 11) Adoption Sequence
1. Introduce contract model and registry v2 in `app_core` while preserving current registry.
2. Add job runtime in Tauri and migrate `run_tool` to use it.
3. Convert top priority tools to typed contracts.
4. Remove python bridge once parity + acceptance are met.

## 12) Definition of Done (Architecture)
1. All active tools discoverable from single registry endpoint.
2. Every tool invocable via async job API.
3. Frontend form generation uses backend schema only.
4. Python dispatcher disabled by default and removable without regressions.
