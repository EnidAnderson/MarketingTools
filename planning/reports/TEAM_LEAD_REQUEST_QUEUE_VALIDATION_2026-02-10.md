# Team Lead Request Queue: Validation and CI Enforcement

Date: 2026-02-10  
Mode: Team Lead (requests only)

## Objective

Convert colored-team governance from policy docs into machine-checkable enforcement.

## RQ-029 (P0) Implement pipeline-order validator

Status: `FULFILLED`  
Owner: QA Fixer

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/check_pipeline_order.sh`
2. Validate fixed order:
- `blue -> red -> green -> black -> white -> grey -> qa_fixer`
3. Script must fail loudly with violated rule and doctrine reference:
- `teams/shared/OPERATING_DOCTRINE.md`

### Acceptance criteria
1. Out-of-order handoff exits non-zero.
2. Error output includes expected phase and observed phase.

---

## RQ-030 (P0) Implement append-only validator

Status: `FULFILLED`  
Owner: QA Fixer

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/check_append_only.sh`
2. Validate append-only behavior for:
- `pipeline/*.md`
- `data/team_ops/*.csv`
3. Allow superseding rows via `supersedes_*` fields; disallow row mutation/deletion.

### Acceptance criteria
1. Mutation/delete simulation fails with file-specific diagnostic.
2. Valid append operation passes.

---

## RQ-031 (P0) Implement QA edit-authority validator

Status: `FULFILLED`  
Owner: QA Fixer

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/check_qa_edit_authority.sh`
2. Ensure non-`qa_fixer` authors cannot modify executable assets.
3. Ensure any QA edit references `decision_id` or `change_request_id`.

### Acceptance criteria
1. Unauthorized editor path exits non-zero.
2. Missing provenance reference exits non-zero.
3. Valid QA edit with provenance reference passes.

---

## RQ-032 (P1) Add validation orchestrator

Status: `FULFILLED`  
Owner: QA Fixer

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/run_all_validations.sh`
2. Orchestrate checks:
- pipeline order
- append-only integrity
- QA edit authority
3. Emit summary report:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/validation_report.json`

### Acceptance criteria
1. Any failed check returns non-zero overall.
2. Report includes per-check pass/fail and failing rule reference.

---

## RQ-033 (P1) Add CI gate for team model

Status: `FULFILLED`  
Owner: QA Fixer

### Required changes
1. Integrate `run_all_validations.sh` into project CI workflow (existing or new).
2. Block merge on validation failure.
3. Add short operator doc:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/README.md`

### Acceptance criteria
1. CI job fails when any validator fails.
2. CI job passes with compliant artifacts.
3. Documentation includes local reproduction command.

---

## RQ-034 (P0) Enforce team-scoped request-id format and uniqueness on new queue rows

Status: `OPEN`  
Owner: QA Fixer

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/teams/_validation/check_request_id_policy.sh`
2. Validate newly added rows in:
- `data/team_ops/change_request_queue.csv`
3. Require format and ownership consistency:
- `request_id` must match `CR-<TEAM>-<NNNN>`
- `<TEAM>` must match `source_team`
4. Fail when a newly added `request_id` is not globally unique.

### Acceptance criteria
1. Newly appended nonconforming IDs fail with row-level diagnostics.
2. Team-code mismatch fails with expected vs observed output.
3. Newly appended duplicate IDs fail with offending IDs listed.

---

## Execution order

1. `RQ-029`, `RQ-030`, `RQ-031`
2. `RQ-032`
3. `RQ-033`
