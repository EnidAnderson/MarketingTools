# INV-GLOBAL-GOV-001

## Invariant Statement
Any red mandatory release gate blocks publish.

## Rationale
Enforces non-negotiable safety/governance boundary.

## Scope
All internal and external publish decisions.

## Enforcement Points
`planning/RELEASE_GATES_POLICY.md`, `planning/reports/RELEASE_GATE_LOG.csv`, `AGENTS.md`.

## Evidence of Compliance
Blocked publish attempts when any gate is red.

## Failure State
Publish proceeds despite red gate.

## Owner Role
Team Lead.

## Test Requirements
Gate simulation tests for each mandatory gate state.
