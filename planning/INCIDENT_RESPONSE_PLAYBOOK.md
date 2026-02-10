# Incident Response Playbook

Effective date: 2026-02-10

## Severity model

1. `SEV-1`: active high-impact external risk.
2. `SEV-2`: high-impact internal risk with containment.
3. `SEV-3`: localized or low-impact issue.

## First 60-minute timeline

1. 0-10 min: classify severity, assign incident commander, open incident log.
2. 10-20 min: contain affected systems/artifacts.
3. 20-40 min: verify blast radius and impacted outputs.
4. 40-60 min: communicate status, define recovery checklist owner + ETA.

## Incident classes

| Incident class | Triage severity | Containment | Comms owner | Recovery checklist |
|---|---|---|---|---|
| Secret exposure | SEV-1 | revoke/rotate credentials, block publish | Security Steward | secret rotation, scan verification, audit logs update |
| False marketing claim published | SEV-1 | unpublish/flag content, block related releases | Product Steward | corrected claim with evidence/caveat, rerun review cycle |
| Runaway spend | SEV-2 | force budget stop, disable costly workflow path | Team Lead | budget envelope correction, approved exception or reduced scope |
| Corrupted artifact lineage | SEV-2 | freeze artifact promotion | Platform Architect | rebuild lineage links, validate audit trail integrity |
| Unauthorized role action | SEV-1 | revoke action path and enforce role block | Team Lead | protocol review, role contract update, approval trace correction |

