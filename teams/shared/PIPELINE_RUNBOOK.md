# Pipeline Runbook

## Start a new run
1. Add run row in `data/team_ops/run_registry.csv`.
2. Set `current_phase=blue` and `status=active`.

## Stage completion
1. Append stage output to `pipeline/0X_<team>_output.md`.
2. Append handoff row to `data/team_ops/handoff_log.csv`.
3. If requests are produced, append to `data/team_ops/change_request_queue.csv`.
4. New `request_id` values must use team-scoped format `CR-<TEAM>-<NNNN>` and remain globally unique.

## QA execution
1. QA Fixer implements only Grey-prioritized requests.
2. QA Fixer appends evidence to `pipeline/07_qa_fix_log.md`.

## Close run
1. Update run status to `completed` in `run_registry.csv` (append superseding row).
2. Record closure decision in `decision_log.csv`.

## Routine cleanup and archival
1. Run `./scripts/team_ops_cleanup.sh --dry-run` to preview what will be archived.
2. Run `./scripts/team_ops_cleanup.sh` to archive non-active tickets/log rows.
3. Archive bundles are stored under `data/team_ops/archive/<timestamp>/` with:
- `backup/` original file snapshots
- `archived_rows/` archived CSV rows
- `manifest.json` operation summary
