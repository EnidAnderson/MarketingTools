# INV-GLOBAL-ROL-002

## Invariant Statement
Role conflicts beyond SLA automatically block safety-critical progression.

## Rationale
Prevents unresolved conflicts from silently passing risk.

## Scope
Safety-critical approval workflows.

## Enforcement Points
`planning/ROLE_ESCALATION_PROTOCOL.md`, risk/review logs.

## Evidence of Compliance
Escalated conflicts trigger blocked state at SLA timeout.

## Failure State
Safety-critical flow continues with unresolved role conflict past SLA.

## Owner Role
Team Lead.

## Test Requirements
SLA timeout simulation causes blocked transition.
