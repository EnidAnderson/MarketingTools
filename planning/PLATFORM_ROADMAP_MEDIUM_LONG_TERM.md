# Platform Roadmap (Medium + Long Term)

Last updated: 2026-02-10  
Owner: Platform Architecture

## Horizon Definitions
1. Medium term: next 6-12 weeks.
2. Long term: next 3-6 months.

## North-Star Outcomes
1. All core campaign tools run natively in Rust.
2. Tool execution is fully async, observable, and cancellable in Tauri.
3. Marketers can run repeatable pipelines with evidence-backed outputs.
4. Artifact lineage and safety checks are default behavior.

## Medium-Term Plan (6-12 weeks)

### Track A: MVP Pipeline to Production-Grade Linear Flow
1. Stabilize `app_core` pipeline executor contract and tests.
2. Add `run_pipeline` command in Tauri with job integration.
3. Implement frontend pipeline panel with predefined templates.
4. Add pipeline run summary artifact (`pipeline_run.json`).
5. Add retry/resume-from-step behavior.

### Track B: Market Analysis Quality
1. Add coverage metrics and query metadata in outputs.
2. Improve canonical URL extraction and source diversity tracking.
3. Add fixture packs to catch parser drift.
4. Add confidence labels tied to evidence thresholds.

### Track C: Runtime Reliability
1. Add per-tool timeout defaults and retry policies.
2. Add bounded concurrency by provider/tool class.
3. Expand progress event stages and telemetry fields.
4. Harden cancellation semantics during long-running steps.

### Track D: Frontend Dynamic UX
1. Remove remaining hardcoded tool special cases.
2. Render forms and outputs fully from backend schema/metadata.
3. Add job history, rerun, and artifact preview flows.

## Long-Term Plan (3-6 months)

### Track E: Advanced Orchestration
1. Move from linear pipelines to DAG workflows.
2. Add conditional routing and policy checks between steps.
3. Add reusable pipeline templates by campaign archetype.

### Track F: Governance and Safety
1. Embed Product Steward checklists into runtime output schema.
2. Add gate-based promotion (`explore` -> `candidate` -> `production`).
3. Integrate Rapid Review Cell logs into pipeline closeout.

### Track G: Data and Observability
1. Add persistent run store and queryable history.
2. Build quality dashboard:
- success/failure by tool
- latency percentiles
- evidence coverage trends
3. Add drift alerts for major output schema or quality changes.

### Track H: Decommission and Simplification
1. Disable Python fallback by default after parity gates.
2. Remove legacy command paths and duplicate frontend handling.
3. Archive/deprecate Python tooling and stale docs.

## Milestone Checkpoints
1. M1 (2 weeks): Pipeline executor + contract tests merged.
2. M2 (4 weeks): Tauri pipeline command + frontend MVP path demoable.
3. M3 (6 weeks): Artifact manifest + run summary + review checklist fields.
4. M4 (8-10 weeks): Retry/resume and reliability controls in place.
5. M5 (12+ weeks): Production candidate for Rust-first pipeline path.

## Program-Level Risks
1. Contract churn across tools slows frontend stability.
2. External provider drift causes brittle tests/results.
3. Scope creep on generic pipeline builder delays MVP value.
4. Legacy paths continue to absorb engineering attention.

## Governance Cadence
1. Weekly architecture backlog review.
2. Bi-weekly milestone readiness check.
3. Monthly roadmap recalibration with stakeholder signoff.
