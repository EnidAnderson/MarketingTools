# Intelligent Marketing System Execution Plan (Cost + Token Efficient)

## 1. Objective
Build a production-grade, graph-driven marketing intelligence platform that is:
- safer than ad-hoc prompting,
- consistently reproducible,
- clearly auditable,
- hard-capped for spend (`$10/day`),
- and ready to switch from mock to live providers with minimal rework.

This plan treats every artifact as a compilable output with deterministic contracts, explicit gating, and measurable quality.

## 2. Current Baseline (Implemented)
- Deterministic mock analytics with persistence, longitudinal deltas, drift/anomaly checks, and Tauri job orchestration.
- Executive dashboard contract + multi-panel frontend with quality scorecards and publish/export gate display.
- Hard spend governor (`PaidCallPermit`) with enforced hard daily cap (`$10`) and fail-closed behavior.
- New graph/text subsystem contracts:
  - DAG validation with deterministic topological order.
  - Weighted gate policy (block only critical risk classes).
  - Priority campaign templates (message house, email+landing, ad variants).
- New deterministic text workflow runtime slice:
  - Graph execution traces,
  - per-node route planning,
  - token/cost estimation,
  - weighted gate artifact assembly.

## 3. Product Principle Stack
1. **Safety first, then quality, then speed**
2. **Determinism before model cleverness**
3. **Typed contracts over convention**
4. **Budget-aware by default (never opt-in)**
5. **Evidence-linked outputs for high-risk claims**
6. **Operator clarity over hidden autonomy**

## 4. Target Architecture (Medium-Term)

### 4.1 Ingest and Validation Boundary
- Source-specific raw contracts (`GA4`, `Google Ads`, `Wix`) with strict deserialization.
- Canonical normalized models using strong newtypes (IDs, currency, time windows).
- Explicit transform errors (typed codes + field paths + sample context).
- Audited cleaning notes (`rule_id`, before/after, severity) attached to artifacts.

### 4.2 Data Quality Core
- Completeness checks (window coverage).
- Freshness SLA checks per source.
- Reconciliation matrix (intra-source + cross-source).
- Identity-resolution coverage scoring.
- Schema-drift signatures/version checks.
- Composite data quality score feeding publish/export gates.

### 4.3 Workflow Graph Runtime
- Typed DAG execution with branch/merge semantics.
- Node-level trace and provenance records.
- Weighted quality/safety gates with critical block classes.
- Deterministic replay (same request + seed => same artifact for mock mode).

### 4.4 Provider Platform Layer
- Capability-based routing (`text`, `image`, `video`).
- Cost/token route planner with hard budget envelope.
- Explicit provider policy per task class:
  - low-risk drafting: cheapest acceptable route,
  - safety-sensitive synthesis: stricter route only when budget allows,
  - fallback to local/mock if budget disallows paid execution.

### 4.5 Operator Surfaces
- Executive dashboard (decision-ready summary + risk signal clarity).
- Workflow registry/discoverability in UI.
- Run-level export packet gated by governance policy.
- Human review queue for blocked runs.

## 5. Cost and Token Efficiency Strategy

### 5.1 Non-Negotiable Controls
- Every paid call must go through reservation/commit/refund guard (`PaidCallPermit`).
- Hard daily cap cannot exceed `$10` anywhere in code.
- Route planning must reject paid paths above per-run or daily remaining budget.

### 5.2 Token Compression Tactics
- Message spine reuse across all branch outputs.
- Structured summaries rather than full conversation replay.
- Evidence references by ID with short excerpts, not full documents.
- Critic/refiner loops bounded by strict iteration limits.
- Per-node token quotas set from role complexity.

### 5.3 Model Routing Policy
- Default to local/mock for development and deterministic tests.
- Use cheapest capable model tier for non-critical tasks.
- Escalate tier only if quality threshold is unmet and budget envelope permits.
- Record selected route + estimated cost in run trace for every node.

### 5.4 Cache and Reuse Policy
- Cache deterministic sub-artifacts keyed by:
  - `campaign_spine_id`,
  - `template_id`,
  - `node_id`,
  - hash of normalized inputs.
- Reuse stable artifacts unless source freshness invalidates them.

## 6. Intelligence Quality Strategy

### 6.1 Workflow-Level Quality
- Scorecard dimensions:
  - instruction coverage,
  - audience alignment,
  - claims risk,
  - brand voice consistency,
  - novelty,
  - revision gain.
- Weighted gates:
  - critical issues block,
  - non-critical issues warn.

### 6.2 Evidence-Centric Generation
- High-risk claims require explicit evidence links.
- Missing evidence triggers critical findings and blocks publish.
- Evidence lineage included in decision feed cards.

### 6.3 Comparative Evaluation Harness
- Baseline vs graph-run comparison for each workflow family.
- Metrics:
  - gate pass rate,
  - critical finding density,
  - revision gain,
  - estimated cost per accepted artifact,
  - operator acceptance rate.

## 7. Weekly Execution Checklist

## Week 1 (Current Focus)
- [x] Add deterministic graph + text contracts.
- [x] Add weighted gate policy for critical block classes.
- [x] Add prioritized templates tied to shared campaign spine.
- [x] Add mock text workflow runtime with trace and token/cost planning.
- [x] Expose Tauri command entrypoints for text workflow jobs.
- [ ] Add frontend panel for text workflow template selection + run status.
- [ ] Add run artifact view for text workflow traces and gate reasons.

## Week 2
- [ ] Add workflow cache store for node-level deterministic reuse.
- [ ] Add route planner policy profiles (`economy`, `balanced`, `quality`).
- [ ] Add hard assertions for token/cost overrun paths in runtime.
- [ ] Add regression tests for route selection under tight budgets.
- [ ] Add export packet contract for text workflows.

## Week 3
- [ ] Implement ingest-boundary contracts for GA4 + Wix + Ads raw payloads.
- [ ] Add typed normalization transforms and audited cleaning notes.
- [ ] Add completeness/freshness/reconciliation checks to quality core.
- [ ] Surface quality ratios in UI as first-class gating inputs.

## Week 4
- [ ] Add hybrid retrieval context builder for campaign planning nodes.
- [ ] Implement evidence resolver with short citation snippets.
- [ ] Add claim-risk classifier pass before gate evaluation.
- [ ] Add policy-driven “red-team” critic mode for regulated claim classes.

## Week 5
- [ ] Add live provider adapters behind provider-platform traits (feature-flagged).
- [ ] Add canary mode (`read-only`, `no publish`) for first live runs.
- [ ] Add provider outage fallback and retry/backoff envelopes.
- [ ] Add real-run ledger reconciliation vs estimated planning costs.

## Week 6
- [ ] Formalize governance packet automation (append-only gate log + decision refs).
- [ ] Add release-readiness checklist command for operators.
- [ ] Add benchmark report across 3 priority workflows.
- [ ] Finalize cutover criteria from mock to live environment.

## 8. Testing and Verification Plan

### 8.1 Determinism
- Property tests: same input + seed => identical artifact bytes (excluding optional wall-clock metadata if documented).
- Stable serialization checks: avoid unordered maps for artifact payloads.

### 8.2 Safety
- Property tests for impossible states:
  - no publish-ready if critical blockers exist,
  - claims-risk threshold forces blocked gate,
  - no costed route beyond budget envelope.

### 8.3 Robustness
- Hostile-input tests for ingest boundaries: parser returns structured errors, never panic.
- Cancel-path tests for running jobs in Tauri runtime.

### 8.4 Economic Correctness
- Tests proving all paid paths require spend reservation.
- Tests proving hard cap cannot be configured above `$10`.
- Tests for refund-on-drop behavior when calls fail.

## 9. Readiness for Real API Keys

### 9.1 Before first live call
- Confirm environment key loading only from expected secure paths.
- Run secret scan on staged + tracked content.
- Verify budget envelope config is explicit and valid for each run.
- Verify provider route policy defaults to economical tier.

### 9.2 First live run protocol
- Use tiny canary workload with strict low per-run cap.
- Capture full route/cost trace and compare with estimate.
- Enforce manual review before enabling broader traffic.

### 9.3 Rollout gates
- Gate A: deterministic + safety tests all green.
- Gate B: cost/ledger reconciliation within tolerance.
- Gate C: operator acceptance and clarity review complete.

## 10. Operating Rules for Future Implementers
- Do not call provider APIs directly from feature code.
- Route all paid calls through spend-governed provider layer.
- Keep artifact schemas versioned; never mutate old schema semantics.
- Add tests for every new invariant before wiring UI.
- Prefer deterministic mock paths for development and CI.

## 11. Next Immediate Build Slice
1. Add text workflow panels to Tauri frontend:
   - template picker,
   - run button,
   - live stage/progress,
   - output gate + trace table.
2. Add caching store for text workflow node outputs.
3. Add route-policy presets with explicit UX controls (`economy`, `balanced`, `quality`).
4. Add integration tests that verify full command path:
   - start job,
   - poll status,
   - validate blocked/non-blocked scenarios.

This sequence delivers visible operator value while preserving strict cost and safety guarantees.
