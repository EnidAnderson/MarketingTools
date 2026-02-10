# INV-GLOBAL-BUD-002

## Invariant Statement
Budget cap exceedance must force an explicit blocked state.

## Rationale
Ensures deterministic spend containment.

## Scope
All budgeted workflows.

## Enforcement Points
`planning/BUDGET_GUARDRAILS_STANDARD.md`, budget logs, run-state docs.

## Evidence of Compliance
Exceeded-cap scenario transitions to blocked status.

## Failure State
Run continues after cap exceedance without approved exception.

## Owner Role
Team Lead.

## Test Requirements
Threshold exceed simulation validates blocked transition.
