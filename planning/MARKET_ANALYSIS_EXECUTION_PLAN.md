# Market Analysis Execution Plan (Rust + Tauri)

Last updated: 2026-02-10  
Owner: Platform Architecture

## 1. Purpose
Provide an implementation-grade plan to make the market-analysis toolchain human-usable, agent-usable, and production-safe inside the Rust/Tauri platform.

## 2. Scope
In-scope:
1. `competitive_analysis` in `app_core`.
2. Tool registry metadata/schema for UI generation.
3. Async execution model via Tauri job runtime.
4. Testing and release gates for reliability and trust.

Out-of-scope (this phase):
1. Full multi-source social scraping.
2. Historical analytics warehouse.
3. Advanced ML model training.

## 3. Design Goals
1. Evidence-first output that marketers can validate.
2. Deterministic schema for agent workflows.
3. Low-friction GUI usage with async progress and cancellation.
4. Clear errors and bounded runtime behavior.

## 4. Milestone Plan

### Milestone 1: Contract and UX Parity (Week 1-2)
Deliverables:
1. Complete registry metadata for `competitive_analysis`.
2. Input examples for realistic marketing queries.
3. Output schema declaration with required fields.
4. Contract tests to detect metadata/runtime drift.

Acceptance:
1. Frontend can render tool form from backend metadata without hardcoding.
2. Example payload runs successfully through `start_tool_job`.

### Milestone 2: Evidence Quality Upgrade (Week 2-3)
Deliverables:
1. Canonical URL normalization from provider redirects.
2. Coverage metrics:
- requested source count
- parsed source count
- deduped source count
- warning flags
3. Explicit inference-to-evidence reference mapping.
4. Report formatting improvements for human readability.

Acceptance:
1. At least one golden fixture verifies stable output structure.
2. Inference section is always clearly labeled and traceable.

### Milestone 3: Runtime Reliability Integration (Week 3-4)
Deliverables:
1. Stage-level progress messages for market-analysis jobs.
2. Timeout and retry wrappers for network operations.
3. Error payload normalization to `ToolError` taxonomy.
4. Cancellation checkpoints between pipeline stages.

Acceptance:
1. Cancellation test passes during network and analysis phases.
2. Timeout path returns retryable structured error.

### Milestone 4: Frontend Validation Surface (Week 4-5)
Deliverables:
1. Evidence panel rendering:
- sources
- keyword table
- recurring phrases
- signals
2. Inference panel separated and labeled.
3. Coverage panel with warnings.
4. Copy/export actions for JSON and markdown output.

Acceptance:
1. Non-technical user can run analysis and inspect evidence in one flow.
2. No runtime-specific parsing logic in frontend for tool internals.

### Milestone 5: Regression and Operational Readiness (Week 5-6)
Deliverables:
1. Unit + integration + regression test pack.
2. Performance and timeout thresholds documented.
3. Release checklist for Rust-only market-analysis path.
4. Decommission recommendation for Python equivalent path (if no critical gaps).

Acceptance:
1. All gates pass in CI for two consecutive release candidates.
2. No critical regressions in campaign workflows using market analysis.

## 5. Engineering Backlog (Actionable)
1. Add `competitive_analysis` UI metadata (`category`, `display_name`, tags, complexity).
2. Add `max_sources` parameter definition and input examples.
3. Add output schema entry for `competitive_analysis` in registry.
4. Refactor search parsing helpers for canonical URLs and better snippet fallbacks.
5. Add coverage/inference confidence fields to output.
6. Add integration test that executes via job manager and verifies terminal snapshot.

## 6. Quality Gates
1. Contract gate:
- tool schema serializes and is stable
- examples validate against runtime input parsing
2. Runtime gate:
- start/get/cancel flows are deterministic
- failures include category + retryable + message
3. Evidence gate:
- inferred notes map to evidence entries
- source list is deduped and non-empty on success
4. UX gate:
- output readable without raw JSON inspection

## 7. Risk Management
1. Provider HTML drift breaks parsing.
- Mitigation: fixture-based parser tests + fallback extraction logic.
2. Slow/blocked network degrades UX.
- Mitigation: timeout + retry + explicit partial failure messaging.
3. Contract changes break frontend.
- Mitigation: contract snapshots and CI checks on schema diffs.
4. Overly broad inference harms trust.
- Mitigation: inference gating by evidence count threshold.

## 8. Rollout Strategy
1. Ship behind capability flag if needed.
2. Run side-by-side validation against legacy outputs during transition.
3. Collect internal marketer feedback on confidence and usability.
4. Promote to default path after passing acceptance gates.

## 9. Team Operating Model
1. Weekly architecture review and backlog triage.
2. Daily async status on milestone blockers.
3. Explicit owner for each milestone and test gate.
4. Post-release retrospective documenting drift, incidents, and follow-ups.
5. Product Steward signs off on production-safety and auditability gates before default rollout.

## 10. Product Steward Integration
1. Treat prompt/spec defects as first-class bugs with ticketed fixes.
2. Require exploration vs production mode labeling for every run artifact.
3. Enforce checklist fields in final output metadata:
- geometry integrity
- claim safety
- evidence sufficiency
- approval decision
4. Keep decision trace in filesystem artifacts, not chat context.

## 11. Definition of Success
1. Market-analysis tool is trusted by both humans and downstream agents.
2. Tool execution is fully async and inspectable in Tauri UI.
3. Contracts are stable and typed in Rust.
4. Legacy Python path is no longer required for this capability.
