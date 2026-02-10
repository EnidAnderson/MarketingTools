# Team Lead Operations Hardening Queue

Date: 2026-02-10  
Mode: Team Lead (requests only)

## Objective

Harden day-to-day execution so failures are contained early, approvals are explicit, and budget/security drift is automatically surfaced.

## RQ-013 (P0) Policy-as-Code Preflight for Governance Artifacts

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create governance preflight script:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/scripts/governance_preflight.sh`
2. Script must validate presence and non-empty state for required control artifacts:
- `planning/RELEASE_GATES_POLICY.md`
- `planning/SECURITY_THREAT_MODEL.md`
- `planning/SECURITY_CONTROL_BASELINE.md`
- `planning/BUDGET_GUARDRAILS_STANDARD.md`
- `planning/AGENT_ROLE_CONTRACTS.md`
- `planning/RISK_REGISTER.md`
3. Wire preflight as optional gated mode in ship path docs:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/AGENTS.md`

### Acceptance criteria
1. Missing required artifact causes non-zero exit.
2. Output clearly identifies failing control and file path.
3. Pass output is concise and machine-parseable.

---

## RQ-014 (P0) Budget Envelope Manifest Requirement

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create budget manifest schema and template:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/BUDGET_ENVELOPE_SCHEMA.md`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/examples/budget_envelope_example.json`
2. Define mandatory envelope fields:
- `run_id`
- `owner_role`
- `hard_cap_usd`
- `warning_threshold_usd`
- `cutoff_behavior`
- `exception_reference`
- `expiry_utc`
3. Update:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/PLANNING.md`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/AGENTS.md`

### Acceptance criteria
1. Any run without envelope is non-compliant by policy.
2. Cutoff behavior is deterministic and documented.
3. Schema example validates against documented field rules.

---

## RQ-015 (P0) Role-to-Permission Matrix and Least Privilege Baseline

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/ROLE_PERMISSION_MATRIX.md`
2. Matrix must include permissions for:
- code changes,
- policy changes,
- budget exception approvals,
- publish approvals,
- incident declaration.
3. Add least-privilege principles and temporary elevation protocol.

### Acceptance criteria
1. Every critical action has explicit allowed roles.
2. No role receives broad write/approve rights by default.
3. Emergency elevation requires expiry and audit note.

---

## RQ-016 (P0) Two-Person Rule for External Publish Path

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/EXTERNAL_PUBLISH_CONTROL.md`
2. Define two-person approval requirements:
- one technical control reviewer,
- one marketing/business owner.
3. Define blocked states and rollback protocol for post-publish detection.
4. Cross-reference:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/RAPID_REVIEW_CELL/SOP.md`

### Acceptance criteria
1. External publish cannot proceed with single approver.
2. Approval records are append-only and timestamped.
3. Rollback path is actionable within 30 minutes.

---

## RQ-017 (P1) Kill-Switch and Safe-Mode Operations

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/KILL_SWITCH_PROTOCOL.md`
2. Define triggers:
- suspected secret exposure,
- runaway spend,
- repeated false-claim risk,
- policy bypass detection.
3. Define safe-mode constraints:
- no external publish,
- no exception approvals,
- review-only operation.

### Acceptance criteria
1. Trigger conditions are concrete.
2. Activation/deactivation authority is role-bound.
3. Recovery checklist includes post-incident verification gates.

---

## RQ-018 (P1) Monthly Tabletop Drill Program

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/TABLETOP_DRILL_PROGRAM.md`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/TABLETOP_DRILL_TEMPLATE.md`
2. Include scenarios:
- false claim shipped,
- cost runaway,
- role conflict deadlock,
- secret detection after commit.

### Acceptance criteria
1. Program defines cadence, owners, and success metrics.
2. Each drill captures lessons and control updates.

---

## RQ-019 (P1) Metrics Dictionary for Hardening KPIs

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/HARDENING_METRICS_DICTIONARY.md`
2. Include KPI definitions:
- gate pass rate,
- incident count by severity,
- budget exception rate,
- unresolved role conflict age,
- claim-evidence completeness.

### Acceptance criteria
1. Every KPI has numerator/denominator/source cadence.
2. No ambiguous metric labels.

---

## RQ-020 (P2) Anti-Drift Governance Review

Status: `FULFILLED`  
Owner: Implementing Bot

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/GOVERNANCE_DRIFT_REVIEW.md`
2. Define monthly drift checks across:
- security controls,
- budget controls,
- role contracts,
- release gates,
- incident readiness.

### Acceptance criteria
1. Drift review outputs actionable remediation tasks.
2. Review results are linked to risk register updates.

---

## Immediate order of execution

1. Execute `RQ-013`, `RQ-014`, `RQ-015`, `RQ-016` first.
2. Then `RQ-017` and `RQ-018`.
3. Then `RQ-019` and `RQ-020`.
