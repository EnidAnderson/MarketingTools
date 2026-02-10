# Team Ops Cleanup Script

## Command
- Preview: `./scripts/team_ops_cleanup.sh --dry-run`
- Apply: `./scripts/team_ops_cleanup.sh`

## What it does
1. Detects active runs from the latest `run_registry.csv` status.
2. Keeps only active work rows in living logs.
3. Archives non-active rows for:
- `data/team_ops/run_registry.csv`
- `data/team_ops/change_request_queue.csv`
- `data/team_ops/handoff_log.csv`
- `data/team_ops/decision_log.csv`

## Archive outputs
Each run writes to:
- `data/team_ops/archive/<timestamp>/backup/`
- `data/team_ops/archive/<timestamp>/archived_rows/`
- `data/team_ops/archive/<timestamp>/manifest.json`
- `data/team_ops/archive/archive_index.csv`

## Useful options
- `--dry-run`
- `--keep-run-history`
- `--active-run-statuses <csv>`
- `--active-request-statuses <csv>`
