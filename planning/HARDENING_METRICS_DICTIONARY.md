# Hardening Metrics Dictionary

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-019`

| KPI | Numerator | Denominator | Source | Cadence |
|---|---|---|---|---|
| Gate pass rate | release attempts with all gates green | total release attempts | `planning/reports/RELEASE_GATE_LOG.csv` | weekly |
| Incident count by severity | incidents in severity bucket | total incidents | incident logs + `planning/INCIDENT_RESPONSE_PLAYBOOK.md` artifacts | weekly |
| Budget exception rate | runs using approved exception | total runs | `planning/BUDGET_EXCEPTION_LOG.md` + run manifests | weekly |
| Unresolved role conflict age | sum of age (hours) of unresolved role conflicts | count of unresolved role conflicts | escalation records | daily |
| Claim-evidence completeness | externally-facing claims supported/caveated | externally-facing claims total | Rapid Review logs | weekly |

## Metric label policy

1. Every KPI name is unique and stable.
2. No KPI may omit numerator or denominator.
3. Source artifact and cadence are mandatory fields.

