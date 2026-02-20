# Spend Guardrail Policy

This repository enforces a hard daily cap of `$10.00` for paid API calls.

## Required implementation pattern
1. Any paid provider call must reserve spend before the network call:
   - `PaidCallPermit::reserve(...)`
2. Successful call path must commit:
   - `permit.commit()`
3. Non-success paths must not manually bypass refund logic:
   - dropping `PaidCallPermit` auto-refunds reservation.

## Non-negotiable constraints
1. `DAILY_BUDGET_USD` values above `$10.00` are rejected (fail-closed).
2. Paid-call modules using API key env vars must reference spend reservation logic.
3. Direct paid network calls without spend reservation are policy violations.

## Primary enforcement locations
1. `rustBotNetwork/app_core/src/tools/generation_budget_manager.rs`
2. `rustBotNetwork/app_core/src/subsystems/provider_platform/spend_policy.rs`

## Verification
1. `cargo test -p app_core generation_budget_manager`
2. `cargo test -p app_core`
