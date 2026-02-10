# Team Validation

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-033`

## Purpose

Machine-checkable enforcement for colored-team governance rules.

## Checks

1. `check_pipeline_order.sh` (`RQ-029`)
2. `check_append_only.sh` (`RQ-030`)
3. `check_qa_edit_authority.sh` (`RQ-031`)
4. `check_request_id_policy.sh` (`RQ-034`)
5. `check_budget_and_release_gates.sh` (`CR-BLACK-0001`, `CR-BLACK-0002`)
6. `check_review_artifact_contract.sh` (`CR-0018`, `CR-0019`, `CR-WHITE-0001`, `CR-WHITE-0002`, `CR-WHITE-0003`, `CR-BLACK-0003`, `CR-BLACK-0004`)
7. `run_all_validations.sh` orchestrator (`RQ-032`)

## Local reproduction

```bash
bash teams/_validation/run_all_validations.sh HEAD
```

## Report

`teams/_validation/validation_report.json` includes per-check pass/fail and rule references.
