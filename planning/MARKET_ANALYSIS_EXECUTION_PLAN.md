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

## 12. Hardening Control Binding

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-021`

| Control ID | Plan Section | Owner Role | Verification Artifact | Gate Type |
|---|---|---|---|---|
| HC-01 | 6. Quality Gates | Security Steward | secret scan output | security |
| HC-02 | 7. Risk Management | Security Steward | threat model review record | security |
| HC-03 | 6. Quality Gates | Security Steward | control baseline mapping | security |
| HC-04 | 13. Budget Envelope and Hard-Stop Policy | Team Lead | budget manifest per milestone | budget |
| HC-05 | 13. Budget Envelope and Hard-Stop Policy | Team Lead | exception log row | budget |
| HC-06 | 6. Quality Gates | Team Lead | release gate log row | governance |
| HC-07 | 9. Team Operating Model | Platform Architect | ADR checkpoint artifact | change |
| HC-08 | 9. Team Operating Model | Team Lead | role contract signoff | role |
| HC-09 | 9. Team Operating Model | Team Lead | escalation protocol reference | role |
| HC-10 | 6. Quality Gates | Product Steward | evidence support coverage report | evidence |

## 13. Budget Envelope and Hard-Stop Policy

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-023`

Milestone envelopes:
1. M1 `$350` warn `$260` fallback `reduced_scope`.
2. M2 `$500` warn `$375` fallback `lower_cost_provider`.
3. M3 `$550` warn `$410` fallback `reduced_scope`.
4. M4 `$450` warn `$340` fallback `reduced_scope`.
5. M5 `$300` warn `$225` fallback `hard_stop`.

Hard-stop condition:
1. Spend > hard cap forces `blocked_budget_cap_exceeded`.
2. Only Team Lead-approved unexpired exception can unblock.

## 14. SLO and Quantified Acceptance

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-022`

1. Max runtime per run class: `<= 120s`.
2. Failure-rate threshold over 7 days: `< 3%`.
3. Minimum evidence completeness: `>= 95%`.
4. Max unresolved critical risk count: `0`.

## 15. Security Assumptions and Abuse Cases

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-024`

| Abuse case | Detection | Response owner | Blocked by policy |
|---|---|---|---|
| Secret leakage path | scan/hook fail | Security Steward | yes |
| Prompt injection from external content | unsupported evidence state | Product Steward | yes |
| Unsafe artifact promotion | release gate red | Team Lead | yes |
| Unauthorized override | role mismatch / missing signoff | Team Lead | yes |

References:
1. `planning/SECURITY_THREAT_MODEL.md`
2. `planning/SECURITY_CONTROL_BASELINE.md`

## 16. Milestone Signoff Matrix

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-025`

| Milestone | Required roles | Can approve | Can block | Evidence required | Escalation path |
|---|---|---|---|---|---|
| M1 Contract parity | Platform Architect, Product Steward | both | either | schema + run proof | role escalation protocol |
| M2 Evidence upgrade | Product Steward, Security Steward | both | either | evidence/inference map | role escalation protocol |
| M3 Runtime reliability | Runtime Engineer, Platform Architect | both | either | timeout/cancel tests | role escalation protocol |
| M4 Frontend validation | Frontend Engineer, Product Steward | both | either | UI validation artifacts | role escalation protocol |
| M5 Operational readiness | Team Lead, Product Steward | both | either | gate pack + release checklist | role escalation protocol |

## 17. Failure and Rollback

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-026`

State transitions:
1. `green -> yellow`: warning threshold breached.
2. `yellow -> red`: critical failure or unmitigated warning.
3. `red -> green`: remediation evidence accepted.

Stop criteria:
1. release gate red.
2. critical incident active.
3. budget hard-stop.

Rollback owner:
1. Platform Architect for technical rollback.
2. Team Lead for release/publish rollback.

Recovery evidence:
1. passing validation report.
2. updated risk register entry with closure.

## 18. ADR Trigger Checkpoints

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-027`

Checkpoint must run after milestones with architecture impact:
1. M1 contract changes.
2. M3 runtime reliability wrappers.

Completion is blocked when ADR trigger hits without ADR artifact:
- `planning/ADR_TRIGGER_RULES.md`

## 19. Glossary Reference

See:
- `planning/CROSS_PLAN_GLOSSARY.md`
