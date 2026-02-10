# Rust Migration Master Plan (Python -> Rust)

Last updated: 2026-02-09

## Scope
Migrate marketing toolchain from Python implementation (`src/tools`, `src/python_tool_dispatcher.py`) to typed Rust tools in `rustBotNetwork/app_core`, and expose via resilient async Tauri runtime.

## 1) Migration Strategy
1. Prioritize business-critical, frequently used tools first.
2. For each tool, implement parity before optimization.
3. Use side-by-side validation until confidence threshold is met.
4. Remove Python code only after production acceptance gates are passed.

## 2) Tool Inventory and Migration Order

### Wave 1: Core Campaign Generation
1. `image_generation.py`
2. `memory_retrieval.py`
3. `product_crawler.py`
4. `generation_budget_manager.py`

### Wave 2: Creative Production and QA
1. `image_manipulation_tool.py`
2. `video_generator_tool.py`
3. `gif_generator_tool.py`
4. `css_analyzer.py`
5. `html_bundler.py`
6. `screenshot_tool.py`

### Wave 3: Distribution and Ops
1. `email_sender_tool.py`
2. `marketing_platform_manager.py`
3. `event_calendar_tool.py`
4. `human_feedback_tool.py`

## 3) Per-Tool Migration Playbook
1. Define typed `Input`/`Output` models.
2. Port core logic with provider trait abstractions.
3. Build parity fixtures from Python outputs.
4. Add unit tests + integration tests.
5. Register tool metadata in registry v2.
6. Expose through Tauri async job API.
7. Run canary validation in campaign workflows.
8. Decommission Python equivalent.

## 4) Acceptance Gates (Mandatory)
A tool can be marked migrated only when:
1. Functional parity test suite passes.
2. Runtime failure modes are classified via ToolError taxonomy.
3. Timeout and retry policy is explicit.
4. Observability hooks exist.
5. Frontend can execute and inspect outputs through generic UI path.

## 5) Workstream Breakdown

### Workstream A: Contracts + Registry
Deliverables:
1. Typed schemas in `app_core`.
2. Unified tool definition source.
3. JSON schema emission for frontend generation.

### Workstream B: Async Runtime + Orchestration
Deliverables:
1. Job manager.
2. Progress + cancellation.
3. Rate limiting and concurrency guardrails.

### Workstream C: Tool Porting
Deliverables:
1. Migrated tools by wave.
2. Golden parity tests.
3. Artifact manifest support.

### Workstream D: Frontend UX
Deliverables:
1. Dynamic tool list.
2. Dynamic input rendering.
3. Job progress view and artifact previews.

### Workstream E: Decommissioning
Deliverables:
1. Python dispatcher removed.
2. Python tool files removed or archived.
3. Documentation and onboarding updated.

## 6) 90-Day Milestone Plan

### Milestone 1 (Weeks 1-3)
1. Registry v2 contract model approved and implemented.
2. Tauri job manager skeleton with `start/get/cancel` APIs.
3. Frontend consumes `list_tools` dynamically.

### Milestone 2 (Weeks 4-6)
1. Wave 1 tools migrated with parity tests.
2. Legacy direct command wrappers marked deprecated.
3. Basic observability dashboard/report output available.

### Milestone 3 (Weeks 7-9)
1. Wave 2 tools migrated and integrated.
2. Job progress + cancellation standardized.
3. Artifact manifest and campaign run metadata in place.

### Milestone 4 (Weeks 10-12)
1. Wave 3 tools migrated.
2. Python fallback disabled in default runtime.
3. Final stabilization, docs refresh, and production readiness review.

## 7) Ownership Matrix (Role-Oriented)
1. Platform Architect:
- contracts, registry, architectural governance.
2. Runtime Engineer:
- Tauri job manager, async controls, reliability.
3. Tool Engineers:
- migration implementation + tests per tool.
4. Frontend Engineer:
- schema-driven forms and async job UX.
5. QA/Validation:
- parity and regression testing across campaign scenarios.

## 8) Quality Gates Per Release
1. `cargo test` for `app_core` and `src-tauri` passes.
2. Representative campaign dry-run produces expected artifact set.
3. Failure scenario tests pass (network timeout, invalid config, canceled job).
4. Documentation updates merged with code.

## 9) Cutover Plan
1. Introduce feature flag: `python_fallback_enabled`.
2. Default ON during migration.
3. Default OFF after Wave 3 + two stable releases.
4. Remove flag and Python bridge after final acceptance review.

## 10) Decommission Checklist
1. Remove `src/python_tool_dispatcher.py`.
2. Remove `rustBotNetwork/app_core/src/python_runner.rs`.
3. Remove unused Python dependencies from `src/requirements.txt` where possible.
4. Update root docs (`README.md`, `PLANNING.md`, `VISION.md`).

## 11) Program Risks and Mitigations
1. API provider drift:
- Mitigation: provider adapters with integration smoke tests.
2. Incomplete parity for edge cases:
- Mitigation: preserve golden fixtures and campaign regression corpus.
3. Team throughput bottlenecks:
- Mitigation: parallelize by tool wave and workstream.

## 12) Reporting Cadence
1. Weekly migration dashboard update.
2. Per-wave completion report with test coverage and known gaps.
3. Release readiness checklist sign-off at each milestone.
