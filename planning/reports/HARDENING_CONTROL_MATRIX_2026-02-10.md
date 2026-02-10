# Hardening Control Matrix

Date: 2026-02-10  
Owner: Team Lead (directive)

## Purpose

Define the minimum control surface that must exist before tool maturity scales.

## Control classes

| Control ID | Domain | Control | Required Artifact | Verification Cadence | Escalation Trigger |
|---|---|---|---|---|---|
| HC-01 | Security | Secret leakage prevention and detection | `scripts/secret_scan.sh`, `.githooks/*`, `AGENTS.md` | Per ship | Any detected secret in staged/tracked scope |
| HC-02 | Security | Threat model coverage | `planning/SECURITY_THREAT_MODEL.md` | Weekly | Threat without owner or SLA |
| HC-03 | Security | Baseline controls with response owners | `planning/SECURITY_CONTROL_BASELINE.md` | Weekly | Missing detection/response mapping |
| HC-04 | Budget | Hard budget caps per run/workflow | `planning/BUDGET_GUARDRAILS_STANDARD.md` | Per run + weekly | Any run without cap declaration |
| HC-05 | Budget | Exception governance | `planning/BUDGET_EXCEPTION_LOG.md` | Weekly | Exception without approver/expiry |
| HC-06 | Governance | Release gate discipline | `planning/RELEASE_GATES_POLICY.md` | Per publish | Publish attempted with red gate |
| HC-07 | Governance | ADR trigger compliance | `planning/ADR_TRIGGER_RULES.md`, `planning/adrs/*` | Weekly | Architecture change without ADR |
| HC-08 | Role | Role authority boundaries | `planning/AGENT_ROLE_CONTRACTS.md` | Weekly | Safety decision with ambiguous owner |
| HC-09 | Role | Escalation and conflict protocol | `planning/ROLE_ESCALATION_PROTOCOL.md` | Weekly | SLA breach on unresolved conflict |
| HC-10 | Evidence | Claim safety closeout | `planning/RAPID_REVIEW_CELL/*` | Per publish | Claim shipped unsupported |
| HC-11 | Risk | Program risk tracking | `planning/RISK_REGISTER.md` | Weekly | High-risk item without mitigation |
| HC-12 | Resilience | Incident readiness | `planning/INCIDENT_RESPONSE_PLAYBOOK.md` | Monthly drill | Incident class lacks 60-min plan |

## Enforcement policy

1. Controls HC-01 through HC-10 are minimum publish prerequisites.
2. HC-11 and HC-12 may be yellow for internal-only iterations but must be green for external-facing release.
3. Any red control requires explicit block state and owner-assigned remediation task.

## Reporting

1. Implementing bot must produce weekly control status summary (`green/yellow/red`).
2. Summary must include evidence links for each non-green control.
