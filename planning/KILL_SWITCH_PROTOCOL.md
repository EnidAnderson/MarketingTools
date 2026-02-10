# Kill-Switch and Safe-Mode Protocol

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-017`

## Trigger conditions

1. Suspected secret exposure.
2. Runaway spend (cap exceeded without approved exception).
3. Repeated false-claim risk in publish path.
4. Policy bypass detection.

## Activation authority (role-bound)

1. Team Lead.
2. Security Steward (SEV-1 security conditions).

## Safe-mode constraints

1. No external publish.
2. No budget/security exception approvals.
3. Review-only operation (analysis and remediation planning allowed).

## Deactivation authority

1. Team Lead plus one independent safety reviewer.

## Recovery checklist

1. Confirm trigger root cause is contained.
2. Re-run release gates and evidence gates.
3. Validate role-signoff integrity.
4. Record recovery decision with timestamp and evidence.

