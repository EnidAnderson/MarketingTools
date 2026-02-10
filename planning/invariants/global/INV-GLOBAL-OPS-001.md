# INV-GLOBAL-OPS-001

## Invariant Statement
Kill-switch safe mode prohibits external publish and exception approvals until recovery gates pass.

## Rationale
Contains risk during active incidents.

## Scope
Incident and degraded-operation windows.

## Enforcement Points
`planning/KILL_SWITCH_PROTOCOL.md`, release and budget controls.

## Evidence of Compliance
Safe mode state blocks prohibited actions.

## Failure State
External publish or exception approvals occur during active safe mode.

## Owner Role
Team Lead.

## Test Requirements
Safe-mode simulation blocks prohibited operations.
