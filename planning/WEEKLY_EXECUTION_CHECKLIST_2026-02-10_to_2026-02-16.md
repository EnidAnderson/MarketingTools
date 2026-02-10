# Weekly Execution Checklist (2026-02-10 to 2026-02-16)

Owner: Platform Engineering (Rust + Tauri)  
Primary objective: deliver a stable MVP pipeline path with enforceable invariants, while raising maintainability baseline.

## 1. Weekly Outcomes (Must-Hit)
- [x] MVP pipeline runnable from Tauri command surface (`start_pipeline_job` + `run_pipeline`) with step-level status fidelity.
- [x] Frontend can invoke at least one predefined pipeline template and display completion/failure status.
- [ ] Market-analysis output upgraded with coverage and evidence-confidence fields for human trust.
- [ ] Invariant checks expanded to critical runtime paths and covered by tests.
- [ ] NDOC coverage on public declarations increased materially from current baseline report.

## 2. Non-Negotiable Gates
- [ ] No new command path bypasses `JobManager`.
- [ ] All new/changed contracts include machine-readable error behavior.
- [ ] Secret scan passes before every ship command.
- [ ] Any architecture-impacting change triggers ADR check per `planning/ADR_TRIGGER_RULES.md`.
- [ ] No destructive git operations; no broad staging (`git add .` / `git add -A`).

## 3. Workstream A: Pipeline MVP (P0)
### A1. Runtime and command parity
- [x] Add/verify `start_pipeline_job` and `run_pipeline` command behavior against tool-job semantics.
- [x] Ensure canceled/failed/succeeded states serialize consistently in `JobSnapshot`.
- [ ] Add test cases for:
  - [ ] pipeline success snapshot
  - [ ] pipeline validation failure snapshot
  - [ ] pipeline canceled snapshot

### A2. Frontend MVP pipeline runner
- [x] Add a minimal "Pipelines" UI section (single template first).
- [x] Wire invocation to `start_pipeline_job`.
- [x] Show polling/event updates until terminal state.
- [x] Render final summary with:
  - [x] pipeline name
  - [x] succeeded flag
  - [x] per-step status and error message

### A3. Pipeline artifact output
- [x] Persist `pipeline_run.json` manifest for each run.
- [ ] Include:
  - [x] run timestamp
  - [ ] input payload
  - [x] step outputs/errors
  - [x] artifact paths (if any)

## 4. Workstream B: Market Analysis Quality (P1)
### B1. Evidence and coverage
- [ ] Add `coverage` section to `competitive_analysis` output:
  - [ ] requested_source_count
  - [ ] fetched_source_count
  - [ ] deduped_source_count
  - [ ] warning flags
- [ ] Add confidence guidance field tied to evidence quantity/diversity.
- [ ] Preserve clear separation: raw signal vs inferred notes.

### B2. Parsing robustness
- [ ] Harden URL normalization/canonicalization for search result links.
- [ ] Add parser fixtures for at least two representative HTML layouts.
- [ ] Add deterministic tests for recurring phrase and signal extraction.

## 5. Workstream C: Invariants + Maintainability (P0/P1)
### C1. Invariant expansion
- [ ] Apply shared invariants in additional boundaries:
  - [ ] tool registry input/schema checks
  - [ ] pipeline output shape checks
  - [ ] job update transitions
- [ ] Add tests proving invariant violations fail safely (validation errors, no panic).

### C2. NDOC coverage uplift
- [ ] NDOC all public declarations in:
  - [ ] `rustBotNetwork/app_core/src/pipeline.rs`
  - [ ] `rustBotNetwork/app_core/src/tools/tool_registry.rs`
  - [ ] `src-tauri/src/runtime.rs`
- [ ] Regenerate reports:
  - [ ] `planning/reports/ndoc_index.json`
  - [ ] `planning/reports/ndoc_summary.md`
  - [ ] `planning/reports/ndoc_coverage.md`
- [ ] Target this week: NDOC public coverage >= 25%.

### C3. Subsystem skeleton maturation
- [ ] Define first concrete contract per subsystem under `app_core/src/subsystems/*`.
- [ ] Add one minimal integration path (pipeline -> artifact_governance manifest contract).
- [ ] Add corresponding planning notes in `planning/subsystems/*`.

## 6. Workstream D: Migration and de-risking (P1)
- [ ] Build parity harness scope for one Python -> Rust tool (`product_crawler` or `memory_retrieval`).
- [ ] Capture golden fixture set and expected output schema.
- [ ] Document migration blockers in `planning/RUST_MIGRATION_MASTER_PLAN.md`.

## 7. Day-by-Day Execution Plan
## Tuesday (2026-02-10)
- [ ] Finalize weekly checklist and align with backlog/docs.
- [ ] Verify Tauri pipeline command behavior and tests pass.
- [ ] Open first frontend MVP pipeline integration changes.

## Wednesday (2026-02-11)
- [ ] Complete frontend pipeline template runner.
- [ ] Add pipeline terminal-state rendering and basic error UX.
- [ ] Add/verify pipeline run manifest write path.

## Thursday (2026-02-12)
- [ ] Implement market-analysis coverage/confidence output updates.
- [ ] Add parser fixtures and deterministic extraction tests.
- [ ] Update output schema metadata in tool registry.

## Friday (2026-02-13)
- [ ] Expand invariant checks in runtime + registry.
- [ ] Increase NDOC coverage and regenerate reports.
- [ ] Run focused test suites and triage failures.

## Saturday (2026-02-14)
- [ ] Stabilization pass: fix regressions, tighten error messages.
- [ ] Prepare demo scenario and reproducible run instructions.
- [ ] Update memory and planning docs with actual outcomes.

## Sunday (2026-02-15)
- [ ] Buffer day for unresolved defects / flaky tests.
- [ ] Validate release gates and security checklist.
- [ ] Draft weekly retrospective notes.

## Monday (2026-02-16)
- [ ] Ship weekly summary report with:
  - [ ] delivered features
  - [ ] test evidence
  - [ ] risk status
  - [ ] next-week carryover

## 8. Verification Checklist (Before Each Ship)
- [ ] `git status --short` reviewed.
- [ ] only intentional files staged explicitly.
- [ ] `./scripts/secret_scan.sh staged` passes.
- [ ] relevant tests/checks run and logged in summary.
- [ ] `./scripts/git_ship.sh -m "<message>"` used for commit+push.

## 9. Weekly Report Deliverables
- [ ] `planning/reports/WEEKLY_EXECUTION_REPORT_2026-02-16.md`
- [ ] Updated `planning/reports/ndoc_coverage.md`
- [ ] Updated `MEMORY.md` (outcomes + next concrete deliverables)
- [ ] Backlog delta update in `planning/IMPLEMENTATION_BACKLOG.md`

## 10. Explicit Deferrals (Not This Week)
- [ ] full DAG pipeline engine
- [ ] persistent job storage backend
- [ ] broad frontend redesign beyond MVP pipeline panel
- [ ] full Python bridge removal
