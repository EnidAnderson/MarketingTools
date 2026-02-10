# Market Analysis Suite Architecture (Rust-First, Evidence-First)

Last updated: 2026-02-10  
Owner: Platform Architecture (Tooling + Runtime)

## 1. Objective
Deliver a production-grade market analysis suite that:
1. Produces inspectable evidence (URLs, snippets, timestamps, coverage stats).
2. Separates raw signals from inferred recommendations.
3. Runs as pure Rust tools in `rustBotNetwork/app_core`.
4. Is exposed to the Tauri GUI through async job APIs.
5. Supports deterministic testing and controlled migration from Python.

## 2. Architectural Constraints
1. Tool logic must stay framework-agnostic in `app_core`.
2. Tauri layer (`src-tauri`) acts as transport and orchestration shell only.
3. All long-running tool execution goes through job lifecycle (`start/get/cancel`).
4. Input/output contracts must remain stable and JSON-serializable.
5. Failures must return structured machine-readable `ToolError`.

## 3. Current State (Observed)
1. `competitive_analysis` exists in Rust and returns structured output + markdown report.
2. Tauri already has `start_tool_job`, `get_tool_job`, `cancel_tool_job` in `src-tauri/src/lib.rs`.
3. Runtime has in-memory job lifecycle in `src-tauri/src/runtime.rs` with progress/completion/failure events.
4. Tool registry in `rustBotNetwork/app_core/src/tools/tool_registry.rs` is functional but incomplete for some tools (metadata/schema parity gaps).
5. Python bridge still exists (`src/python_tool_dispatcher.py`, `python_runner.rs`) for legacy compatibility.

## 4. Target Architecture

### 4.1 Domain Layer (`app_core`)
Responsibilities:
1. Typed contracts for each tool request/response.
2. Evidence extraction and normalization pipeline.
3. Signal scoring and inference generation with explicit traceability.
4. Tool metadata for GUI form generation and output rendering.

Core modules:
1. `contracts.rs` for error taxonomy and typed contract traits.
2. `tools/competitive_analysis.rs` for analysis logic.
3. `tools/tool_registry.rs` for discovery metadata.

### 4.2 Runtime Layer (`src-tauri`)
Responsibilities:
1. Accept JSON inputs from frontend and enqueue jobs.
2. Execute tools asynchronously and persist job snapshots in memory.
3. Emit progress + terminal events.
4. Return structured outputs/errors back to UI.

Core modules:
1. `src-tauri/src/lib.rs` command boundary.
2. `src-tauri/src/runtime.rs` job manager and state transitions.

### 4.3 Frontend Layer
Responsibilities:
1. Fetch tool definitions dynamically from backend.
2. Render forms from parameter schemas.
3. Start/cancel jobs and visualize progress/events.
4. Display evidence-first output panels (coverage, sources, signals, inferences).

## 5. Market Analysis Data Contract v1

### Input
Required:
1. `topic: string`

Optional:
1. `max_sources: number` (bounded, e.g. 3..20)
2. `freshness_days: number` (future extension)
3. `region: string` (future extension)
4. `include_domains: string[]` (future extension)
5. `exclude_domains: string[]` (future extension)

### Output
Required sections:
1. Query metadata (`topic`, run timestamp, source count, run id).
2. Coverage metrics (requested sources, fetched sources, parse success ratio).
3. Raw sources (title, canonical URL, snippet).
4. Raw signal tables (keyword frequency, recurring phrases, signal clusters).
5. Inferred notes explicitly tagged as `inferred`.
6. Human-readable markdown brief.

## 6. Evidence and Inference Separation
Rules:
1. Raw evidence sections never contain recommendations.
2. Inference section must include references to supporting signals.
3. Any unsupported inference is blocked at generation time.
4. If coverage is below threshold, output warns and reduces confidence labels.

## 7. Async Tauri Integration Strategy

### 7.1 Command Surface (Canonical)
1. `get_tools() -> ToolDefinition[]`
2. `start_tool_job(tool_name, input) -> JobHandle`
3. `get_tool_job(job_id) -> JobSnapshot`
4. `cancel_tool_job(job_id) -> Result<(), String>`

### 7.2 Job Lifecycle
1. `Queued` -> accepted and validated.
2. `Running` -> tool execution active.
3. `Succeeded` -> output payload present.
4. `Failed` -> structured error payload present.
5. `Canceled` -> cancellation acknowledged and surfaced.

### 7.3 Progress Model for Market Analysis
Stages to emit:
1. `querying_sources`
2. `parsing_sources`
3. `extracting_signals`
4. `building_report`
5. `completed`

## 8. Reliability and Governance
1. Timeout policy per network step.
2. Retry policy only for retryable provider/network errors.
3. Source deduplication and canonical URL normalization.
4. Hard limits on source count and response size.
5. Clear user-facing error messages for empty/noisy result sets.

## 9. Testing Strategy

### Unit tests (app_core)
1. HTML parse fixtures for search providers.
2. Keyword/signal extraction deterministic behavior.
3. Inference generation gating by evidence presence.
4. Input validation edge cases.

### Integration tests (src-tauri + app_core)
1. Job lifecycle happy path.
2. Failure propagation with `ToolError`.
3. Cancellation behavior.
4. Event emission and snapshot consistency.

### Regression tests
1. Golden fixtures for market-analysis output schema stability.
2. Contract snapshots to prevent UI breakage.

## 10. Migration Sequence for Market Analysis
1. Stabilize `competitive_analysis` contract + metadata in registry.
2. Add coverage metrics and canonical evidence fields.
3. Add provider abstraction for search sources (single provider first, pluggable interface).
4. Integrate frontend rendering from dynamic schema.
5. Add historical run comparison in later phase.

## 11. Definition of Done (Market Analysis Suite)
1. Tool output is evidence-grounded and schema-stable.
2. Frontend can run and inspect tool fully through generic async APIs.
3. Failures are transparent and actionable.
4. Tests cover parser, contract, and job-runtime behavior.
5. Python path not required for market-analysis execution.

## 12. Operating Governance
1. Product Steward role owns production-safety acceptance criteria.
2. Exploration and production runs are explicitly separated in artifact metadata.
3. Promotion to production use requires:
- evidence sufficiency
- geometry/content safety signoff
- audit trail completeness
4. See `planning/PRODUCT_STEWARD_OPERATING_MODEL.md` for role-level workflow.

## 13. Hardening Control Binding

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-021`

| Control ID | Plan Section | Owner Role | Verification Artifact | Gate Type |
|---|---|---|---|---|
| HC-01 | 8. Reliability and Governance | Security Steward | `scripts/secret_scan.sh` output | security |
| HC-02 | 12. Operating Governance | Security Steward | `planning/SECURITY_THREAT_MODEL.md` | security |
| HC-03 | 8. Reliability and Governance | Security Steward | `planning/SECURITY_CONTROL_BASELINE.md` | security |
| HC-04 | 14. Budget Envelope and Hard-Stop Policy | Team Lead | budget envelope JSON | budget |
| HC-05 | 14. Budget Envelope and Hard-Stop Policy | Team Lead | `planning/BUDGET_EXCEPTION_LOG.md` | budget |
| HC-06 | 12. Operating Governance | Team Lead | `planning/reports/RELEASE_GATE_LOG.csv` | governance |
| HC-07 | 12. Operating Governance | Platform Architect | ADR file in `planning/adrs/` | change |
| HC-08 | 12. Operating Governance | Team Lead | `planning/AGENT_ROLE_CONTRACTS.md` | role |
| HC-09 | 12. Operating Governance | Team Lead | `planning/ROLE_ESCALATION_PROTOCOL.md` | role |
| HC-10 | 6. Evidence and Inference Separation | Product Steward | Rapid Review evidence logs | evidence |

## 14. Budget Envelope and Hard-Stop Policy

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-023`

Milestone budget envelope:
1. M1: cap `$400`, warning `$300`, hard-stop at cap, fallback `reduced_scope`.
2. M2: cap `$500`, warning `$375`, hard-stop at cap, fallback `lower_cost_provider`.
3. M3: cap `$500`, warning `$375`, hard-stop at cap, fallback `reduced_scope`.
4. M4: cap `$450`, warning `$340`, hard-stop at cap, fallback `reduced_scope`.
5. M5: cap `$350`, warning `$260`, hard-stop at cap, fallback `hard_stop`.

Deterministic hard-stop:
1. If cumulative spend exceeds cap, run state becomes `blocked_budget_cap_exceeded`.
2. Resume requires approved exception with expiry.
3. Owner role: Team Lead.

## 15. SLO and Quantified Acceptance

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-022`

1. Max runtime per run class: `<= 120s`.
2. Failure-rate threshold (weekly): `< 3%` failed runs.
3. Minimum evidence completeness: `>= 95%` externally-facing claims supported/caveated.
4. Max unresolved critical risk count: `0`.

## 16. Security Assumptions and Abuse Cases

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-024`

Security assumptions:
1. External content is untrusted until evidence-bound.
2. Secret material is never committed to tracked/staged scopes.

| Abuse case | Detection | Response owner | Blocked state |
|---|---|---|---|
| Secret leakage path | secret scan fail | Security Steward | release blocked |
| Prompt injection from external content | unsupported claim/caveat gap | Product Steward | evidence gate red |
| Unsafe artifact promotion | release gate red | Team Lead | publish blocked |
| Unauthorized override | role mismatch in approvals | Team Lead | role gate red |

Control refs:
1. `planning/SECURITY_THREAT_MODEL.md`
2. `planning/SECURITY_CONTROL_BASELINE.md`

## 17. Milestone Signoff Matrix

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-025`

| Milestone | Required roles | Can approve | Can block | Evidence required | Escalation path |
|---|---|---|---|---|---|
| M1 Contract/UX parity | Platform Architect, Product Steward | both | either | contract tests + schema evidence | role escalation protocol |
| M2 Evidence quality | Product Steward, Security Steward | both | either | evidence mapping report | role escalation protocol |
| M3 Runtime reliability | Platform Architect, Runtime Engineer | both | either | cancellation/timeout tests | role escalation protocol |
| M4 Frontend validation | Frontend Engineer, Product Steward | both | either | UI evidence panels + screenshots | role escalation protocol |
| M5 Operational readiness | Team Lead, Product Steward | both | either | gate pack + regression suite | role escalation protocol |

## 18. Failure and Rollback

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-026`

Transition map:
1. `green`: all gates pass.
2. `yellow`: non-critical warning threshold breached; mitigation active.
3. `red`: critical control failure; stop criteria triggered.

Stop criteria:
1. red gate.
2. unsupported external claim.
3. budget hard-cap exceeded.

Rollback owner:
1. Team Lead (operational rollback).
2. Platform Architect (technical rollback).

Recovery evidence:
1. new green gate log row.
2. resolved ticket references.

## 19. ADR Trigger Checkpoints

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-027`

ADR checkpoint is required at:
1. runtime model changes.
2. provider boundary changes.
3. trust/security boundary changes.

Completion is blocked if trigger condition is met without ADR per:
- `planning/ADR_TRIGGER_RULES.md`

## 20. Glossary Reference

See:
- `planning/CROSS_PLAN_GLOSSARY.md`
