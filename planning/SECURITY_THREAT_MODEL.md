# Security Threat Model

Effective date: 2026-02-10
Owner: Security Steward

## Data classes

1. `public`: safe for external publication.
2. `internal`: operational content not intended for public.
3. `confidential`: business-sensitive internal data.
4. `restricted`: secrets/credentials/regulated sensitive data.

## Threat register

| Threat ID | Threat | Data class risk | Owner | Detection signal | Response SLA |
|---|---|---|---|---|---|
| TH-01 | Secret leakage in repo/artifacts | restricted | Security Steward | `scripts/secret_scan.sh` fail, hook fail | 15 minutes |
| TH-02 | Prompt injection via external content | internal/confidential | Tool Engineer Lead | suspicious instruction diff, anomalous output | 4 hours |
| TH-03 | Unsafe artifact publication | public/internal | Product Steward | evidence gate red, unsupported claim state | 1 hour |
| TH-04 | Supply-chain dependency risk | internal/confidential | Platform Architect | new dependency without review/ADR | 1 business day |
| TH-05 | Privilege abuse/unauthorized role action | all | Team Lead | role mismatch in approvals/audit logs | 1 hour |

