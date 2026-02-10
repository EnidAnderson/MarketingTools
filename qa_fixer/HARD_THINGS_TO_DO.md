# Hard Things To Do

Last updated: 2026-02-10  
Owner: qa_fixer

Provenance:
- `decision_id=DEC-0002`
- `decision_id=DEC-0003`

These are blocked/non-trivial items that are not simple edits.

## 1. Resolve real pipeline-order violation in handoff logs

- Current validator failure: expected `green -> black`, observed `white -> grey`.
- Why hard: this is a process-state/data integrity issue across teams, not a local code patch.
- Needed: either backfill missing stage handoffs with authoritative superseding rows, or invalidate/reopen later-stage handoffs.
- Artifacts involved:
  - `data/team_ops/handoff_log.csv`
  - `pipeline/03_green_output.md`
  - `pipeline/04_black_output.md`
  - `pipeline/06_grey_output.md`

## 2. Enforce provenance refs for all executable-asset edits

- Current validator failure flags files changed without embedded `decision_id`/`change_request_id`.
- Why hard: many existing code/config files predate this requirement and adding metadata uniformly needs a standard format and migration plan.
- Needed: define canonical provenance annotation style per file type (`.rs`, `.json`, `.yml`, `.sh`, etc.), then run staged rollout.
- Candidate policy doc update:
  - `teams/shared/OPERATING_DOCTRINE.md`
  - `teams/qa_fixer/spec.md`

## 3. Merge concurrent validator evolution safely

- `teams/_validation/run_all_validations.sh` now includes `RQ-034` checks from parallel edits.
- Why hard: shared script is a multi-writer contention point; accidental rule drift can break CI.
- Needed: establish single-writer branch policy or staged merge protocol for `_validation`.

## 4. Close remaining non-P0 governance queues with consistency checks

- Remaining request families:
  - `RQ-017`..`RQ-020`
  - `RQ-025`..`RQ-028`
- Why hard: cross-document consistency, role governance, and drill/metric definitions must stay aligned with already-shipped P0 controls.
- Needed: one consolidated acceptance report that verifies no conflicts with release gate, budget, and role-control baselines.
