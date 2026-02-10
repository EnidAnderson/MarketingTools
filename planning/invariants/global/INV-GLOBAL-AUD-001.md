# INV-GLOBAL-AUD-001

## Invariant Statement
Release gate, decision, and exception records are append-only.

## Rationale
Preserves audit integrity and historical replay.

## Scope
Governance, budget, and review logs.

## Enforcement Points
`planning/reports/RELEASE_GATE_LOG.csv`, `planning/BUDGET_EXCEPTION_LOG.md`, `planning/RAPID_REVIEW_CELL/logs/*`.

## Evidence of Compliance
Corrections use superseding entries, not edits/deletes.

## Failure State
Historical entries are modified or removed.

## Owner Role
Platform Architect.

## Test Requirements
Log-integrity checks detect mutation/deletion attempts.
