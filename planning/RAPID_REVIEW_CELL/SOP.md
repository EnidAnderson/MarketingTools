# SOP: Sequential Claim Safety Review

## Global rules

1. Logs are append-only. Do not update or delete historical rows.
2. If a row is corrected, append a new row and set `supersedes_row_id`.
3. Every review run uses one `review_run_id` (example: `rrc_2026-02-10_001`).
4. Every claim uses one `claim_id` stable for that artifact version.

## Phase 0: Intake

Goal: register exactly what is being reviewed.

Required output:
- Append one row to `planning/RAPID_REVIEW_CELL/logs/INTAKE.csv`.

Gate to continue:
- `artifact_id`, `artifact_version`, `channel`, and `artifact_locator` are populated.

## Phase 1: Falsification pass

Goal: identify how the artifact could mislead, overclaim, drift, or fail contextually.

Allowed actions:
- adversarial reading,
- ambiguity extraction,
- stale-claim detection,
- hidden-assumption extraction.

Disallowed actions:
- rewriting copy,
- "probably fine" conclusions.

Required output:
- Append rows to `planning/RAPID_REVIEW_CELL/logs/RISK_INVARIANT.csv`.

Gate to continue:
- Each risky claim has at least one `failure_mode` and one `invariant` entry.

## Phase 2: Interpretation pass

Goal: normalize each explicit/implicit claim into checkable language.

Claim format:
- subject + predicate + condition + boundary
- avoid tone words and persuasion language

Required output:
- Append rows to `planning/RAPID_REVIEW_CELL/logs/CLAIM_REGISTER.csv`.

Gate to continue:
- Every disposition-eligible claim has a normalized statement.

## Phase 3: Evidence binding pass

Goal: bind each claim to support, or explicitly mark unsupported/aspirational.

Evidence classes:
- `code_ref`
- `doc_ref`
- `test_ref`
- `artifact_ref` (e.g., screenshot/demo)
- `caveat`

Required output:
- Append rows to `planning/RAPID_REVIEW_CELL/logs/EVIDENCE_SUPPORT.csv`.

Gate to continue:
- Every claim has `support_status` in: `supported | caveated | unsupported | aspirational`.

## Phase 4: Disposition pass

Goal: decide shipment state for the artifact version.

Allowed outcomes:
- `approved_as_is`
- `approved_with_caveat`
- `needs_revision`
- `blocked`

Required output:
- Append one row to `planning/RAPID_REVIEW_CELL/logs/DECISION_DISPOSITION.csv`.
- Include rationale tied to claim IDs.

## Final stakeholder output

Create a concise summary using `planning/RAPID_REVIEW_CELL/SUMMARY_TEMPLATE.md` with:
- what is safe to say,
- what needs adjustment,
- what should not be claimed yet.
