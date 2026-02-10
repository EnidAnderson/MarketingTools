# INV-GLOBAL-EVD-002

## Invariant Statement
Unsupported claims must never appear in "safe to say" outputs.

## Rationale
Prevents silent overclaiming.

## Scope
Rapid Review Cell summaries and release summaries.

## Enforcement Points
`planning/RAPID_REVIEW_CELL/SUMMARY_TEMPLATE.md`, disposition logs.

## Evidence of Compliance
Unsupported claims are routed to "Do not claim yet".

## Failure State
Unsupported claim marked as approved/safe.

## Owner Role
Product Steward.

## Test Requirements
Summary validation checks unsupported-claim placement.
