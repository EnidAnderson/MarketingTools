# Budget Guardrails Standard

Effective date: 2026-02-10
Owner: Team Lead

## Mandatory controls

1. Per-run cap (hard stop).
2. Daily cap per workflow.
3. Monthly cap per subsystem.
4. Mandatory fallback behavior when cap exceeded.
5. Exception path with role-based approval and expiry.

## Required budget envelope fields

1. `run_id`
2. `workflow_id`
3. `subsystem`
4. `per_run_cap_usd`
5. `daily_cap_usd`
6. `monthly_cap_usd`
7. `fallback_mode`
8. `owner_role`

## Enforcement

1. No run proceeds without declared budget envelope.
2. Exceeded cap transitions run to `blocked_budget_cap_exceeded`.
3. Only approved exception may temporarily unblock.
4. Exception must include approver role and expiry timestamp.

## Fallback modes

1. Degrade to lower-cost provider/settings.
2. Reduce run scope to highest-priority tasks only.
3. Stop run and require manual approval to resume.

