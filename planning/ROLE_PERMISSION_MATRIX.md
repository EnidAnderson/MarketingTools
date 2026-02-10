# Role Permission Matrix

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-015`

## Permission table

| Critical action | Allowed roles | Can approve | Can block | Notes |
|---|---|---|---|---|
| Code changes | Tool Engineer, Platform Architect, QA/Validation | Platform Architect | Team Lead, Security Steward | Production safety constraints apply. |
| Policy changes | Team Lead, Platform Architect | Team Lead | Team Lead, Security Steward | Requires rationale and audit trace. |
| Budget exception approvals | Team Lead (primary), Product Steward (secondary) | Team Lead | Team Lead | Must include expiry and audit note. |
| External publish approvals | Technical Control Reviewer + Marketing/Business Owner (two-person rule) | Both required | Either can block | Single approver is invalid. |
| Incident declaration | Team Lead, Security Steward | Team Lead or Security Steward | Team Lead, Security Steward | SEV-1 may be declared by either. |

## Least-privilege principles

1. No role has global write+approve rights across all critical actions.
2. Approval rights are scoped to domain ownership and safety boundaries.
3. Blocking authority may be broader than approval authority for safety containment.

## Temporary elevation protocol

1. Elevation requires:
- approver role,
- scope,
- reason,
- expiry UTC,
- audit note reference.
2. Elevated rights auto-expire at declared time.
3. Expired elevation without renewal immediately revokes temporary permissions.

