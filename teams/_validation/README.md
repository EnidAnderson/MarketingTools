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
5. `check_request_id_global_uniqueness.sh` (`RQ-MGR-001`)
6. `check_handoff_run_state_sync.sh` (`RQ-MGR-003`)
7. `check_stage_output_format.sh` (`RQ-MGR-004`)
8. `generate_cycle_health_summary.sh` (`RQ-MGR-005`)
9. `check_budget_and_release_gates.sh` (`CR-BLACK-0001`, `CR-BLACK-0002`)
10. `check_review_artifact_contract.sh` (`CR-0018`, `CR-0019`, `CR-WHITE-0001..0012/0015`, `CR-BLACK-0003..0007`)
11. `run_all_validations.sh` orchestrator (`RQ-032`)

## Local reproduction

```bash
bash teams/_validation/run_all_validations.sh HEAD
```

## Report

`teams/_validation/validation_report.json` includes per-check pass/fail and rule references.
`teams/_validation/cycle_health_summary.json` includes deterministic cycle health fields for `RQ-MGR-005`.
