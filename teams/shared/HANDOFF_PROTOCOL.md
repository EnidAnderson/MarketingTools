# Handoff Protocol

## Required handoff payload (all teams)
1. `run_id`
2. `team_id`
3. `input_refs`
4. `output_summary`
5. `change_requests`
6. `risks_or_open_questions`
7. `done_criteria`
8. `timestamp_utc`

## File targets
- Narrative outputs: `pipeline/0X_<team>_output.md`
- Structured logs: `data/team_ops/handoff_log.csv`

## Enforcement
1. Missing required payload fields => handoff invalid.
2. Non-QA code edits => hard failure and rollback request.
3. Grey must preserve disagreements and unresolved tradeoffs.
4. Any newly issued `change_requests` IDs must match `CR-<TEAM>-<NNNN>`; nonconforming IDs invalidate handoff.
