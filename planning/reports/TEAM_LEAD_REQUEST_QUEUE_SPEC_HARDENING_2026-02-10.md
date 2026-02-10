# Team Lead Request Queue: Spec and Plan Hardening

Date: 2026-02-10  
Mode: Team Lead (requests only)

Based on review:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/TEAM_LEAD_SPEC_HARDENING_REVIEW_2026-02-10.md`

## RQ-021 (P0) Add hardening control-binding tables to core plans

Status: `FULFILLED`  
Owner: Implementing Bot

### Target files
1. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MARKET_ANALYSIS_SUITE_ARCHITECTURE.md`
2. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MARKET_ANALYSIS_EXECUTION_PLAN.md`
3. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MVP_PIPELINE_INTEGRATION_PLAN.md`

### Required changes
1. Add section: `Hardening Control Binding`.
2. Include table columns:
- `Control ID`
- `Plan Section`
- `Owner Role`
- `Verification Artifact`
- `Gate Type`
3. Bind at minimum HC-01 through HC-10.

### Acceptance criteria
1. Every milestone maps to at least one control.
2. No control is unowned.

---

## RQ-022 (P0) Quantify acceptance and SLO thresholds

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Replace qualitative acceptance wording with measurable thresholds.
2. Add SLO section in each target plan with minimum:
- max runtime per run class,
- failure-rate threshold,
- minimum evidence completeness,
- max unresolved critical risk count.

### Acceptance criteria
1. All acceptance criteria are testable with pass/fail.
2. Threshold values are explicit and numerically bounded.

---

## RQ-023 (P0) Add budget envelope and hard-stop policy to each plan

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Add budget section in each target plan referencing:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/BUDGET_GUARDRAILS_STANDARD.md`
2. For each milestone, specify:
- budget envelope,
- warning threshold,
- hard-stop condition,
- fallback mode.

### Acceptance criteria
1. No milestone lacks a budget envelope.
2. Hard-stop behavior is deterministic and role-owned.

---

## RQ-024 (P0) Add security assumptions and abuse-case handling

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Add `Security Assumptions and Abuse Cases` section in each target plan.
2. Include minimum abuse cases:
- secret leakage path,
- prompt-injection from external content,
- unsafe artifact promotion,
- unauthorized override.
3. Add control references to:
- `planning/SECURITY_THREAT_MODEL.md`
- `planning/SECURITY_CONTROL_BASELINE.md`

### Acceptance criteria
1. Every abuse case has detection and response owner.
2. Unsafe states are explicitly blocked by policy.

---

## RQ-025 (P1) Add role-bound signoff matrix to milestones

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Add milestone signoff matrix to each target plan.
2. Required fields:
- milestone,
- required roles,
- can approve/can block,
- evidence required,
- escalation path.

### Acceptance criteria
1. Safety-critical milestones require at least 2-role signoff.
2. Role ambiguity is removed from all milestone gates.

---

## RQ-026 (P1) Add failure-state and rollback transition map

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Add explicit `Failure and Rollback` section in each target plan.
2. Define transitions:
- `green -> yellow -> red` conditions,
- stop criteria,
- rollback owner,
- recovery evidence.

### Acceptance criteria
1. Each red-state condition has a mandatory containment action.
2. Rollback path can be executed without ad-hoc decision making.

---

## RQ-027 (P1) Add ADR trigger checkpoints in plan milestones

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Insert ADR checkpoint after architecture-impact milestones.
2. Link to:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/ADR_TRIGGER_RULES.md`

### Acceptance criteria
1. Milestone completion is blocked if ADR trigger is hit without ADR.

---

## RQ-028 (P2) Create cross-plan glossary and schema dictionary

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/CROSS_PLAN_GLOSSARY.md`
2. Normalize terms:
- evidence sufficiency,
- confidence label,
- production-safe,
- blocked state,
- exception approval.
3. Cross-link glossary from all three target plans.

### Acceptance criteria
1. Ambiguous terms are removed or mapped to one canonical definition.
2. All three target plans include glossary reference.

---

## Immediate order of execution

1. Execute `RQ-021`, `RQ-022`, `RQ-023`, `RQ-024` first.
2. Then `RQ-025`, `RQ-026`, `RQ-027`.
3. Finish with `RQ-028`.
