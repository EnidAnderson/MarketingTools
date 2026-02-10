# Team Lead Request Queue: Micro-Manager Corrections

Date: 2026-02-10  
Mode: Team Lead (requests only)

## RQ-MGR-001 (P0) Enforce global uniqueness for `request_id`

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. Add validation that `data/team_ops/change_request_queue.csv` has unique `request_id` values.
2. Add migration guidance for existing duplicates using `supersedes_request_id`.

### Acceptance criteria
1. Duplicate IDs fail validation with offending IDs listed.
2. Legacy duplicates are resolved with explicit supersede entries.

---

## RQ-MGR-002 (P0) Enforce phase-order handoff gate

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. Add check that handoff order follows `team_registry.csv phase_order`.
2. Add explicit block behavior when stage is skipped.

### Acceptance criteria
1. Out-of-order handoff fails with expected vs observed phase.
2. Block reason is logged in `decision_log.csv`.

---

## RQ-MGR-003 (P0) Require run-state updates per handoff/block

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. On each handoff append, require corresponding run-state append entry.
2. On block condition, run status must transition to blocked state.

### Acceptance criteria
1. Any handoff without run-state update fails validation.
2. Block events require run-state row within same cycle.

---

## RQ-MGR-004 (P1) Required output-format validator

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. Validate each stage output contains required sections from team `spec.md`.
2. Fail when any required section missing.

### Acceptance criteria
1. Validator reports missing section names and stage file path.

---

## RQ-MGR-005 (P1) Cycle health summary artifact

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. Generate machine-readable cycle health summary:
- stage completion map
- block reasons
- duplicate ID count
- unresolved requests

### Acceptance criteria
1. Summary generated each cycle with deterministic schema.
