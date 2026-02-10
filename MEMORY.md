# Nature's Diet Agent Platform Memory

Last updated: 2026-02-10  
Owner: Platform Architecture (Rust + Tauri)

## Mission
Build a production-grade, Rust-first agentic marketing platform for Nature's Diet that:
1. Replaces Python tool execution with typed Rust tools.
2. Exposes all tools through consistent async Tauri job APIs.
3. Delivers marketer-trustworthy outputs with evidence, clear failure modes, and inspectable artifacts.

## Executive Snapshot
1. Rust foundations are present: `app_core` tool abstractions, contracts, and tool registry exist.
2. Tauri async job runtime exists today: `start/get/cancel` commands and in-memory job manager are implemented.
3. Migration is incomplete: Python compatibility path still exists and frontend is only partially dynamic.
4. Immediate leverage point: market-analysis and tool metadata quality upgrades improve both human UX and agent utility.

## Architecture Baseline (Current)
1. Domain logic in `rustBotNetwork/app_core`.
2. Tauri transport/runtime in `src-tauri`.
3. Frontend still contains hardcoded behavior in `frontend/main.js`.
4. Tool contracts are partially typed, but many tool I/O surfaces still effectively rely on loose `Value`.
5. Runtime lifecycle states are present but persistence and richer progress semantics are not yet complete.

## Principal Technical Risks
1. Contract drift between runtime tool behavior and registry metadata can break UI generation.
2. Mixed legacy and modern command paths increase maintenance burden and ambiguity.
3. Limited timeout/retry/cancellation granularity can degrade reliability under provider/network variance.
4. Missing artifact/coverage standards can reduce trust in analysis outputs.
5. Migration pace risk: tool parity gaps may keep Python dependency alive longer than planned.

## Strategic Decisions (Active)
1. Keep `app_core` independent of Tauri and UI concerns.
2. Standardize on async job execution for all non-trivial tool calls.
3. Preserve JSON at frontend boundary while increasing typed contracts internally.
4. Require evidence-first output for market-analysis workflows.
5. Decommission Python only behind explicit acceptance gates with parity evidence.
6. Adopt Product Steward operating model for marketing-system governance and artifact safety.
7. Enforce code-level invariants and structured NDOC docstrings for core declarations.

## Workstream Map

### Workstream A: Contracts and Registry Integrity
1. Enforce tool parameter/schema parity tests.
2. Ensure every active tool has:
- complete parameter list
- UI metadata
- example input payload
- output schema
3. Introduce contract version field where missing.
4. Expand invariant validation utilities and adopt them across runtime boundaries.
5. Track NDOC coverage for public declarations.

### Workstream B: Runtime Hardening
1. Keep `start/get/cancel` as canonical command set.
2. Add richer progress staging (tool-specific stage labels).
3. Add runtime safeguards:
- per-tool timeout defaults
- retry policy for retryable errors
- bounded concurrency
4. Prepare optional persistent job history (P2).

### Workstream C: Tool Migration and Quality
1. Continue wave-based Python -> Rust migration.
2. Use parity fixtures and golden contract snapshots.
3. Upgrade market-analysis signal quality and evidence traceability as a flagship workflow.
4. Stand up MVP multi-step pipeline runner to prove end-to-end orchestration.

### Workstream D: Frontend Dynamic Tool UX
1. Remove hardcoded tool list.
2. Render forms from backend tool definitions.
3. Render output by schema-aware panels (raw signal, inferred notes, artifact links).
4. Add retry and rerun flows bound to stored input payloads.

## Market Analysis Focus (Immediate Program Priority)
1. Treat `competitive_analysis` as the primary proving ground for:
- evidence-grounded output
- schema consistency
- async job UX
2. Upgrade output semantics:
- coverage metrics
- canonical source links
- explicit inferred-vs-raw boundaries
3. Ensure registry reflects runtime behavior so GUI can expose full control surface.
4. Add deterministic fixture tests for parsing and signal extraction stability.

## 30/60/90 Day Plan

### 0-30 days
1. Finish metadata/schema parity for active Rust tools.
2. Stabilize market-analysis contract and improve coverage transparency.
3. Wire frontend to backend `get_tools()` definitions for execution UI.
4. Add runtime contract tests for job lifecycle and error serialization.
5. Ship MVP linear pipeline (market analysis -> SEO analyzer -> visualization) with step-level trace.

### 31-60 days
1. Complete Wave 1 parity certification (image generation, memory retrieval, product crawler, budget manager).
2. Introduce provider abstraction standards for networked tools.
3. Add artifact manifest schema for campaign outputs.
4. Expand progress events and cancellation checkpoints.

### 61-90 days
1. Complete Wave 2 tool migrations and regression suites.
2. Set Python fallback OFF by default in staging.
3. Validate end-to-end campaign generation paths on Rust-first runtime.
4. Ship operations dashboard/report for tool health metrics.

## Acceptance Gates (Program-Level)
1. Each migrated tool has typed request/response contracts.
2. Tool outputs are schema-stable and integration-tested.
3. UI can run/inspect/cancel tools via generic job APIs only.
4. Failure modes are explicit, categorized, and user-actionable.
5. Python fallback disabled without critical-path regressions.

## Operational Cadence
1. Weekly architecture review:
- contract drift
- migration status
- runtime reliability
2. Bi-weekly release readiness:
- test status
- risk review
- decommission progress
3. Monthly strategic checkpoint:
- milestone burn-down
- roadmap updates
- staffing/ownership adjustments

## Ownership Model (Recommended)
1. Platform Architect:
- contracts, architecture governance, migration integrity
2. Product Steward, Marketing Systems:
- design-to-spec governance, production-safety checks, audit quality
3. Runtime Engineer:
- job manager reliability, async semantics, event model
4. Tool Engineers:
- migration + parity + provider adapters
5. Frontend Engineer:
- dynamic form/output rendering, job UX
6. QA/Validation:
- golden fixtures, integration, regression packs

## Next Concrete Deliverables
1. Finalize market-analysis architecture spec (completed in planning docs).
2. Add market-analysis execution roadmap with milestone tasks (new planning doc).
3. Patch tool-registry parity gaps for competitive-analysis metadata/schema/examples.
4. Add/expand tests for parser behavior, schema stability, and job-state edge cases.
5. Wire `app_core` pipeline executor into Tauri command + frontend MVP pipeline screen.
6. Raise NDOC coverage on public declarations from baseline and enforce in CI.

## References
1. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/AGENT_PLATFORM_ARCHITECTURE.md`
2. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/TAURI_ASYNC_BRIDGE_STRATEGY.md`
3. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/RUST_MIGRATION_MASTER_PLAN.md`
4. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MARKET_ANALYSIS_SUITE_ARCHITECTURE.md`
5. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/rustBotNetwork/app_core/src/tools/competitive_analysis.rs`
6. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/src-tauri/src/runtime.rs`
7. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/PRODUCT_STEWARD_OPERATING_MODEL.md`
8. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MVP_PIPELINE_INTEGRATION_PLAN.md`
9. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/PLATFORM_ROADMAP_MEDIUM_LONG_TERM.md`
10. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/DOCSTRING_STANDARD.md`
11. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/scripts/rag/README.md`
12. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/SUBSYSTEM_SKELETON_OVERVIEW.md`
