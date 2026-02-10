# INV-GLOBAL-SEC-001

## Invariant Statement
Secret scans must pass for staged and tracked scopes before any ship action.

## Rationale
Prevents secret leakage through commit/push path.

## Scope
All repository ship operations.

## Enforcement Points
`AGENTS.md`, `.githooks/pre-commit`, `.githooks/pre-push`, `scripts/git_ship.sh`, `scripts/secret_scan.sh`.

## Evidence of Compliance
Passing scan outputs and blocked commits on failure.

## Failure State
Any detected secret blocks commit/push.

## Owner Role
Security Steward.

## Test Requirements
Positive and negative tests for staged and tracked scopes.
