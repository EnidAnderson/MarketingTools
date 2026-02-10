# INV-GLOBAL-CHG-001

## Invariant Statement
Architecture-impacting changes require ADR approval before merge.

## Rationale
Prevents untracked architectural drift.

## Scope
Core contracts, runtime model, security controls, governance model.

## Enforcement Points
`planning/ADR_TRIGGER_RULES.md`, `planning/adrs/*`, `AGENTS.md`.

## Evidence of Compliance
ADR linked for each triggered change.

## Failure State
Triggered change merged without ADR.

## Owner Role
Platform Architect.

## Test Requirements
ADR-trigger checks fail when required ADR reference is missing.
