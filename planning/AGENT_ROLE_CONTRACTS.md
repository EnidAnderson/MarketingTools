# Agent Role Contracts

Effective date: 2026-02-10
Owner: Team Lead

## Decision rights matrix

| Role | Can approve | Can block | Can advise | Forbidden actions |
|---|---|---|---|---|
| Team Lead | release gate closure, exception approvals | any safety-critical release | all domains | bypass documented controls |
| Product Steward | claim safety dispositions | evidence gate failures | product-risk guidance | approve budget/security exceptions alone |
| Security Steward | security remediations | security gate failures | secure-by-default guidance | approve false negatives without evidence |
| Platform Architect | ADR and architecture decisions | architecture changes without ADR | implementation sequencing | approve marketing claim safety alone |
| Tool Engineer | implementation details in owned scope | unsafe implementation proposal | technical feasibility | self-approve release gates |
| QA/Validation | verification results | failed verification | test strategy | approve role/security exceptions |

## Handoff contract fields

1. `request_id`
2. `owner_role`
3. `input_artifacts`
4. `expected_output_artifacts`
5. `done_criteria`
6. `risk_notes`
7. `deadline_utc`

## Safety-critical authority rule

No safety-critical decision is valid without a uniquely accountable owner role from the matrix above.

