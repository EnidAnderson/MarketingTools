# Release Gates Policy

Effective date: 2026-02-10
Owner: Team Lead

## Purpose

Define non-negotiable release gates for internal and external publication.

## Gate rule

1. Any red gate blocks publish.
2. Gate checks must be append-only and timestamped.
3. Normal changes must complete gate checklist in under 10 minutes.

## Mandatory gates

1. Security gate:
- `./scripts/secret_scan.sh staged` passes.
- No unresolved critical security finding in active scope.
2. Budget gate:
- Run declares per-run, daily workflow, and monthly subsystem caps.
- Stop condition exists when cap is exceeded.
3. Evidence gate:
- Externally-facing claims map to evidence or explicit caveat in `planning/RAPID_REVIEW_CELL/logs/*`.
4. Role gate:
- Required roles signed off per `planning/AGENT_ROLE_CONTRACTS.md`.
5. Change gate:
- ADR exists for architecture-impacting changes per `planning/ADR_TRIGGER_RULES.md`.

## Operational checklist

1. Run security gate checks.
2. Confirm budget envelope declaration.
3. Confirm evidence mapping/caveat state.
4. Confirm role sign-off state.
5. Confirm ADR trigger compliance.
6. Append gate result row to release gate log.

## Governed runtime entrypoints

Use governed commands when starting execution through Tauri:
1. `start_tool_job_governed`
2. `start_pipeline_job_governed`
3. `validate_governance_inputs`

These commands enforce budget envelope and gate-state validation before execution starts.

## Append-only log

Use `planning/reports/RELEASE_GATE_LOG.csv` and never edit prior rows.
