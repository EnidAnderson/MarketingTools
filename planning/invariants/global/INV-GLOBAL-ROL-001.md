# INV-GLOBAL-ROL-001

## Invariant Statement
Every safety-critical decision must have one explicit authority owner role.

## Rationale
Avoids ambiguous accountability.

## Scope
Release, incident, budget exception, role escalation decisions.

## Enforcement Points
`planning/AGENT_ROLE_CONTRACTS.md`, `planning/ROLE_PERMISSION_MATRIX.md`.

## Evidence of Compliance
Decision records include owner role and authority basis.

## Failure State
Safety decision recorded without clear authority owner.

## Owner Role
Team Lead.

## Test Requirements
Decision-record validation fails on missing/ambiguous owner role.
