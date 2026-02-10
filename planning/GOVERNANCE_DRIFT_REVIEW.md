# Governance Drift Review

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-020`

## Monthly drift checks

1. Security controls:
- compare active controls vs `planning/SECURITY_CONTROL_BASELINE.md`.
2. Budget controls:
- compare run envelopes and exceptions vs `planning/BUDGET_GUARDRAILS_STANDARD.md`.
3. Role contracts:
- compare approvals/escalations vs `planning/AGENT_ROLE_CONTRACTS.md`.
4. Release gates:
- verify gate records and red-gate block behavior.
5. Incident readiness:
- verify playbook freshness and drill completion.

## Output format

1. Drift finding.
2. Severity.
3. Owner.
4. Remediation task.
5. Due date.
6. Linked risk register item.

## Risk register linkage

Every non-trivial drift finding must create or update an entry in:
- `planning/RISK_REGISTER.md`

