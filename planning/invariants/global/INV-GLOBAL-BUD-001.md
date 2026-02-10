# INV-GLOBAL-BUD-001

## Invariant Statement
No run may start without a declared budget envelope.

## Rationale
Prevents uncontrolled spend.

## Scope
All campaign and analysis runs.

## Enforcement Points
`planning/BUDGET_GUARDRAILS_STANDARD.md`, `AGENTS.md`, run intake/process docs.

## Evidence of Compliance
Every run record includes required budget fields.

## Failure State
Run starts with missing budget envelope.

## Owner Role
Team Lead.

## Test Requirements
Start-run checks fail when envelope missing.
