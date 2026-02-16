# qa_fixer Work Queue

Date: 2026-02-11

## Completed

1. Critical invariant tests:
- `RQ-INV-001`, `003`, `004`, `005`, `006`, `008`, `011`, `013`, `014`
2. High-severity invariant tests:
- `RQ-INV-002`, `007`, `009`, `010`, `012`, `015`
3. Full suite run:
- `scripts/tests/invariants/run_all_invariant_tests.sh`
- result: `15/15 pass`
- report: `scripts/tests/invariants/results/invariant_test_report.json`
4. Team validation queue (high-importance):
- `RQ-029` pipeline-order validator
- `RQ-030` append-only validator
- `RQ-031` QA edit-authority validator
- `RQ-032` validation orchestrator + report
- `RQ-033` CI gate + operator doc
5. Micro-manager validator expansion:
- `RQ-MGR-001` global request-id uniqueness validator
- `RQ-MGR-003` handoff/run-state sync validator
- `RQ-MGR-004` stage output-format validator
- `RQ-MGR-005` cycle health summary generator
6. White/Black extended QA controls:
- `CR-WHITE-0004`..`CR-WHITE-0012`, `CR-WHITE-0014`, `CR-WHITE-0015`
- `CR-BLACK-0005`, `CR-BLACK-0006`, `CR-BLACK-0007`
- implemented in `teams/_validation/check_review_artifact_contract.sh` and supporting artifact/schema updates.

## In progress

1. Validator overlap cleanup (`check_review_artifact_contract.sh` vs `check_extended_contracts.sh`).
2. Policy decision follow-up for `qa_fixer -> grey` loop semantics.

## Remaining high-importance tickets

1. No open latest-state queue tickets in `data/team_ops/change_request_queue.csv` (`OPEN_TOTAL=0`).
2. Validation hard blockers (`RQ-030`, `RQ-031`, `RQ-034`, `RQ-MGR-001`) are now passing.
