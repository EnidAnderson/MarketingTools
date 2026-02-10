# Colored Teams Operating System

This workspace enforces a pipeline model for multi-agent review and execution.

## Team folders
- `teams/blue`
- `teams/red`
- `teams/green`
- `teams/black`
- `teams/white`
- `teams/grey`
- `teams/qa_fixer`

## Core rules
1. Teams are lenses, not hierarchies.
2. Output moves forward in sequence; no circular debate loops.
3. All non-QA teams are read/analyze/request only.
4. Only `qa_fixer` may modify code/config/schema artifacts.
5. Pipeline files under `pipeline/` are append-only.
6. Every team output must follow the `REQUIRED OUTPUT FORMAT` block in its `spec.md`.
7. Team disagreements must be filed as change requests, not embedded debate.

## Execution sequence
1. Blue
2. Red
3. Green
4. Black
5. White
6. Grey
7. QA Fixer

See `teams/shared/OPERATING_DOCTRINE.md` and `teams/shared/HANDOFF_PROTOCOL.md`.
Validation requests are tracked in `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/TEAM_LEAD_REQUEST_QUEUE_VALIDATION_2026-02-10.md`.
