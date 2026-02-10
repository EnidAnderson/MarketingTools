# MVP Pipeline Integration Plan (Cross-System Demo)

Last updated: 2026-02-10  
Owner: Platform Architecture + Runtime + Frontend

## 1. Goal
Deliver a demonstrable MVP pipeline that proves the full stack works together:
1. Rust tool execution (`app_core`).
2. Async orchestration in Tauri (`src-tauri`).
3. GUI-driven execution/inspection (`frontend`).
4. Reproducible artifacts and decision-ready outputs for marketers.

## 2. MVP Definition
A single-click workflow that runs these steps in sequence:
1. `competitive_analysis` (market signal collection)
2. `seo_analyzer` (quality/readability scan on derived brief text)
3. `data_viz` (basic chart artifact from selected metrics)

Inputs:
1. topic
2. max_sources
3. campaign output directory

Outputs:
1. structured JSON pipeline run summary
2. markdown market brief
3. one chart artifact
4. job timeline with step-level status

## 3. Why This Sequence
1. It exercises external data retrieval + parsing (`competitive_analysis`).
2. It exercises derived-content analysis (`seo_analyzer`).
3. It exercises artifact generation (`data_viz`).
4. It demonstrates both machine-usable and human-usable outputs.

## 4. Current Capability vs Required Capability
Current:
1. individual tool execution works via async job manager.
2. frontend can dynamically discover tools.
3. market-analysis tool exists and returns rich data.

Missing for MVP:
1. first-class pipeline definition and execution contract.
2. step-to-step parameter wiring.
3. pipeline-level result object and trace.
4. frontend pipeline runner screen.

## 5. Immediate Build Plan (Next 1-2 Cycles)

### Step A: Pipeline Engine (app_core)
1. Add pipeline contract structs (`PipelineDefinition`, `PipelineStep`, `PipelineRunResult`).
2. Add sequential executor with step output references.
3. Stop-on-failure behavior for MVP with explicit step error payloads.
4. Add unit tests for input resolution and validation.

Status:
1. Initial engine scaffold added in `rustBotNetwork/app_core/src/pipeline.rs`.
2. Example pipeline payload added in `planning/examples/mvp_pipeline_example.json`.

### Step B: Tauri Command Wiring
1. Add `run_pipeline` command in `src-tauri/src/lib.rs`.
2. Execute pipeline through async job manager path.
3. Emit pipeline progress events per step (`tool-job-progress` with pipeline context).

### Step C: Frontend MVP UI
1. Add "Pipelines" section with one predefined MVP template.
2. Collect topic/max_sources/output_path.
3. Trigger backend pipeline command and stream progress.
4. Render step cards + final artifacts panel.

### Step D: Artifact and Audit Layer
1. Write pipeline summary JSON to run folder.
2. Save markdown report and chart paths in summary.
3. Add run metadata:
- pipeline name/version
- timestamp
- input payload hash
- tool versions (where available)

## 6. MVP Acceptance Criteria
1. A marketer can run pipeline from GUI without editing JSON.
2. Pipeline shows per-step status and stops on first failure with actionable error.
3. Output includes at least one generated artifact file and one markdown brief.
4. Run summary can be reopened for audit.
5. Entire run completes in <= 2 minutes under normal network conditions.

## 7. Medium-Term Roadmap (3-6 cycles)
1. Add parameter templates per persona/use case (sensitive stomach, budget buyer, cat owner).
2. Add optional branch step for `product_crawler`.
3. Add run history and compare view (trend drift and narrative changes).
4. Add retry-per-step and resume-from-step behavior.
5. Add provider abstraction and bounded retry policies in pipeline executor.

## 8. Long-Term Roadmap (6-12 cycles)
1. DAG-based pipeline execution (not only linear sequence).
2. Policy-aware execution (filesystem/secrets scopes enforced per step).
3. Persistent pipeline run store and analytics dashboard.
4. Full campaign pipeline spanning market analysis -> creative spec -> asset generation -> review cell.
5. Remove Python fallback from critical campaign path.

## 9. Risk Register
1. Parsing fragility in market-analysis sources.
- Mitigation: fixture tests + robust fallback selectors.
2. Tool contract drift breaks step mapping.
- Mitigation: schema contract tests and pipeline compile-time checks where feasible.
3. Frontend complexity creeps before MVP is stable.
- Mitigation: one template first, defer generic builder UI.
4. Runtime cancellation semantics unclear for multi-step flow.
- Mitigation: explicit canceled state with current-step context.

## 10. Ownership
1. Platform Architect: pipeline contract and architectural integrity.
2. Runtime Engineer: Tauri command + job integration + events.
3. Frontend Engineer: MVP pipeline UI + output rendering.
4. Tool Engineer: market-analysis and data_viz stability.
5. Product Steward: production-safety and audit signoff.
