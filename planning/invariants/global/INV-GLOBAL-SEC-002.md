# INV-GLOBAL-SEC-002

## Invariant Statement
External content is always treated as untrusted until claim-level evidence binding completes.

## Rationale
Reduces prompt-injection and misinformation propagation risk.

## Scope
Market analysis, research, and claim-generation workflows.

## Enforcement Points
`planning/RAPID_REVIEW_CELL/*`, `planning/SECURITY_THREAT_MODEL.md`, `planning/SECURITY_CONTROL_BASELINE.md`.

## Evidence of Compliance
Logs show unsupported claims are caveated/blocked.

## Failure State
External content directly promoted to publish claims without review.

## Owner Role
Security Steward.

## Test Requirements
Simulated untrusted-source input cannot bypass evidence gate.
