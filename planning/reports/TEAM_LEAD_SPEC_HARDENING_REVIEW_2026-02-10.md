# Team Lead Spec Hardening Review

Date: 2026-02-10  
Scope reviewed:
1. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MARKET_ANALYSIS_SUITE_ARCHITECTURE.md`
2. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MARKET_ANALYSIS_EXECUTION_PLAN.md`
3. `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/MVP_PIPELINE_INTEGRATION_PLAN.md`

## Findings (ordered by severity)

### F-01 (Critical): No mandatory control-binding to hardening controls
Risk:
Plans describe goals but do not bind milestones/deliverables to named controls (security/budget/role/evidence/release gate), making compliance optional in execution.

Required correction:
Add a control-binding table to each spec mapping sections/milestones to control IDs from:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/HARDENING_CONTROL_MATRIX_2026-02-10.md`

### F-02 (Critical): Budget and spend controls are implied, not enforceable
Risk:
No explicit budget envelope, warning thresholds, or stop conditions in the three plans. This allows silent cost overrun during iterative runs.

Required correction:
Add budget envelope fields and hard-stop behavior per milestone and per run type.

### F-03 (High): Security threat assumptions are underspecified
Risk:
No explicit threat boundary assumptions, data classification handling, or prompt-injection containment requirements tied to each phase.

Required correction:
Add a “Security assumptions + abuse cases” section in each plan, referencing the security baseline docs.

### F-04 (High): Acceptance criteria are partially non-quantitative
Risk:
Several success criteria are qualitative (e.g., “trusted,” “readable,” “low-friction”) and not auditable.

Required correction:
Convert to measurable thresholds with objective pass/fail checks.

### F-05 (High): Role accountability is present but not authority-bound
Risk:
Named owners exist, but decision rights (approve/block/escalate) and handoff payload requirements are not explicitly enforced in plan execution.

Required correction:
Add role decision matrix and milestone signoff requirements in each plan.

### F-06 (Medium): Failure/rollback play paths are incomplete
Risk:
Plans define mitigations but not a concrete stop/rollback procedure when confidence drops or quality gates fail.

Required correction:
Add explicit failure state transitions and rollback path per milestone.

### F-07 (Medium): Change control/ADR trigger hooks are missing
Risk:
Architecture-impacting deviations can enter execution without formal decision trace.

Required correction:
Require ADR trigger checks at milestone boundaries.

### F-08 (Medium): Cross-plan glossary/schema alignment not guaranteed
Risk:
Terms like “evidence sufficiency,” “confidence,” and “production-safe” can drift across docs.

Required correction:
Add normalized glossary + schema field dictionary cross-reference.

## Decision

Status: `REQUIRES REVISION BEFORE CLAIMING HARDENED PLANNING BASELINE`

## Required deliverable from implementing bot

Submit revised versions of the three specs plus a one-page “hardening delta summary” showing where each finding is closed.
