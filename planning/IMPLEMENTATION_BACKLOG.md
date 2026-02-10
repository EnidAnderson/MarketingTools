# Implementation Backlog and Execution Board

Last updated: 2026-02-10

## Priority Legend
- P0: blocks architecture integrity or release safety.
- P1: high value, required for migration pace.
- P2: important quality/UX enhancements.

## P0 Backlog
1. Define typed tool contract layer in `app_core`.
- Outcome: each tool has explicit request/response model.
- Acceptance: compile-time typed execution for at least 3 migrated tools.

2. Implement Tauri job manager and lifecycle APIs.
- Outcome: all tool invocations run via job runtime.
- Acceptance: `start/get/cancel` commands with tests.

3. Unify registry metadata and frontend discovery.
- Outcome: remove hardcoded tool definitions from frontend.
- Acceptance: UI renders at least 5 tools from backend metadata.

4. Standardize tool naming and parameter schemas.
- Outcome: no mismatch between registry metadata and runtime handlers.
- Acceptance: schema contract tests pass for all active tools.

## P1 Backlog
1. Migrate Wave 1 tools fully with parity testing.
2. Add progress event reporting contract for long-running tasks.
3. Add artifact manifest and campaign run metadata outputs.
4. Implement provider abstraction for Gemini and future providers.
5. Add centralized timeout/retry/rate-limit policy.
6. Finalize market-analysis contract/registry parity for GUI-driven execution.
7. Add evidence-coverage metrics and inference traceability fields to market-analysis outputs.
8. Ship MVP sequential pipeline execution engine in Rust with step output wiring.
9. Expose pipeline execution in Tauri and frontend with step-level trace view.
10. Enforce runtime/domain invariants in critical modules (job manager, pipeline, registry contracts).
11. Roll out NDOC structured docstrings on all public contracts and runtime APIs.

## P2 Backlog
1. Add advanced frontend execution UX (history, retries, artifact previews).
2. Add benchmark suite for tool latency and throughput.
3. Add policy engine for filesystem and secrets scopes.
4. Add optional persistent job history storage.
5. Add historical comparison view for recurring market-analysis runs.

## Detailed Task Slices

### Slice A: Contract System (P0)
1. Add `contracts` module to `app_core`.
2. Define `ToolError` enum and `ToolResult<T>` alias.
3. Implement serde JSON adapters for UI transport.
4. Add schema serialization for frontend form generation.

### Slice B: Job Runtime (P0)
1. Add `job_manager` with in-memory store.
2. Add job cancellation token support.
3. Add event emission integration in Tauri runtime.
4. Add job-state tests for happy/error/cancel paths.

### Slice C: Frontend Dynamic Integration (P0/P1)
1. Replace `availableTools` hardcoded list.
2. Render form controls from backend parameter metadata.
3. Execute jobs through generic command.
4. Subscribe to progress/completion events.

### Slice D: Migration Execution (P1)
1. Pick one tool from each category for first parity pass:
- generation: `image_generation`,
- retrieval: `memory_retrieval`,
- utility: `product_crawler`.
2. Create golden fixtures from existing behavior.
3. Implement side-by-side comparison harness.
4. Certify parity and flip default to Rust path.

## Testing Backlog
1. Tool contract tests for all migrated tools.
2. Provider adapter tests with mocked HTTP responses.
3. Tauri command integration tests.
4. End-to-end campaign scenario tests.
5. Pipeline contract tests for step input reference resolution and failure propagation.
6. Invariant tests for job snapshot state machine and contract validation helpers.

## Documentation Backlog
1. Keep `/MEMORY.md` updated weekly.
2. Maintain migration status table in `planning/RUST_MIGRATION_MASTER_PLAN.md`.
3. Add ADRs for major architecture decisions under `planning/adrs/`.
4. Maintain `planning/MARKET_ANALYSIS_SUITE_ARCHITECTURE.md` and `planning/MARKET_ANALYSIS_EXECUTION_PLAN.md` as source-of-truth for market-analysis modernization.
5. Maintain `planning/PRODUCT_STEWARD_OPERATING_MODEL.md` and align acceptance checklists with tool output schemas.
6. Maintain NDOC reports under `planning/reports/` and track coverage trend over time.

## Exit Criteria for Backlog Burn-Down
1. P0 items complete and stable across two release cycles.
2. All Wave 1 + Wave 2 tools migrated.
3. Python fallback off by default.
4. Remaining P2 items converted into post-migration optimization roadmap.
