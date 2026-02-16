# Hard Things To Do

Last updated: 2026-02-11  
Owner: qa_fixer

Provenance:
- `decision_id=DEC-0002`
- `decision_id=DEC-0003`

These are non-trivial items and follow-ups (not currently hard-blocked).

## 1. Request-ID migration without violating append-only controls

- Status: `RESOLVED` (2026-02-10)
- Outcome:
  - append-only-safe queue replay completed,
  - `RQ-034` validator updated for controlled legacy transition + canonical enforcement on malformed IDs.
- Artifacts involved:
  - `data/team_ops/change_request_queue.csv`
  - `teams/_validation/check_request_id_policy.sh`
  - `teams/_validation/check_append_only.sh`

## 2. Append-only integrity recovery after multi-writer queue rewrites

- Status: `RESOLVED` (2026-02-10)
- Outcome:
  - queue normalized back to `HEAD` baseline + append-only deltas,
  - `RQ-030` now passing.

## 3. Formalize qa_fixer-to-grey loop semantics

- Status: `OPEN (policy follow-up)`
- Why hard: this is a process model decision, not only a validator tweak.
- Needed:
  - confirm whether `qa_fixer -> grey` remains an allowed remediation loop,
  - encode final doctrine text if policy changes.

## 4. De-duplicate overlapping validator implementations

- Status: `OPEN (quality follow-up)`
- Why hard: dual validators can drift and create contradictory pass/fail results.
- Needed:
  - choose single source-of-truth vs layered contract model,
  - simplify orchestration/docs accordingly.
