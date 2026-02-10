# Team Ops Data Files

Structured, append-only operational logs for synchronized team work.

## Files
- `team_registry.csv`: authority matrix by team.
- `run_registry.csv`: run lifecycle.
- `handoff_log.csv`: stage-to-stage transfers.
- `change_request_queue.csv`: actionable request backlog.
- `decision_log.csv`: governance decisions.
- `qa_edit_authority.csv`: single-writer policy rules.
- `budget_envelopes.csv`: declared run budget envelopes.
- `review_artifacts/`: append-only review/approval payloads.

## Mutation policy
1. Prefer append-only updates.
2. Use `supersedes_*` columns for corrections.
3. New rows in `change_request_queue.csv` must use team-coded IDs: `CR-<TEAM>-<NNNN>` (for example `CR-RED-0011`).
