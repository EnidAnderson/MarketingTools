# Rapid Independent Review Cell

A lightweight composite review team for marketing artifacts that applies five lenses (red/black/gray/blue/white) in one sequential workflow.

## Scope

This cell evaluates only:
- truthfulness of claims,
- clarity of interpretation,
- stability of supporting invariants,
- evidence sufficiency for shipment.

This cell does not:
- rewrite copy,
- decide marketing strategy,
- approve based on tone or persuasion quality.

## Design invariants

1. Falsification happens before interpretation.
2. Interpretation happens before evidence binding.
3. No disposition without claim-level evidence state.
4. Logs are append-only; corrections are additive.
5. Uncertainty is explicit (`unsupported` or `caveated`) and never implied away.

## File layout

- `planning/RAPID_REVIEW_CELL/SOP.md` : sequential operating procedure.
- `planning/RAPID_REVIEW_CELL/RUN_PROMPT.md` : copy/paste operator prompt for AI reviewer.
- `planning/RAPID_REVIEW_CELL/SUMMARY_TEMPLATE.md` : concise output for marketing stakeholders.
- `planning/RAPID_REVIEW_CELL/TICKETING.md` : ticket handoff and engineering response workflow.
- `planning/RAPID_REVIEW_CELL/logs/INTAKE.csv` : reviewed artifacts and context.
- `planning/RAPID_REVIEW_CELL/logs/CLAIM_REGISTER.csv` : explicit and implicit claims.
- `planning/RAPID_REVIEW_CELL/logs/RISK_INVARIANT.csv` : falsification paths and invariants.
- `planning/RAPID_REVIEW_CELL/logs/EVIDENCE_SUPPORT.csv` : evidence/caveats per claim.
- `planning/RAPID_REVIEW_CELL/logs/DECISION_DISPOSITION.csv` : immutable outcomes.
- `planning/RAPID_REVIEW_CELL/logs/TICKET_QUEUE.csv` : append-only engineering tickets opened from findings.
- `planning/RAPID_REVIEW_CELL/logs/TICKET_RESPONSES.csv` : append-only engineering responses/status updates.

## Usage

1. Add one row to `INTAKE.csv` for each artifact version.
2. Run the four-pass SOP exactly in order.
3. Add log rows in each phase (never edit past rows).
4. Publish the stakeholder summary using `SUMMARY_TEMPLATE.md`.
5. Convert review findings into tickets:
   `python3 scripts/rapid_review/tickets.py from-review --review-run-id <id> --owner-team <team> --opened-by teams_leader`
6. Engineers respond via:
   `python3 scripts/rapid_review/tickets.py respond ... --non-breaking-change true`
