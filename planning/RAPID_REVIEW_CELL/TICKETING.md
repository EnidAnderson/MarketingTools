# Rapid Review Ticketing Workflow

This workflow converts review findings into engineering tickets and records responses as append-only logs.

## Goals

1. Give the teams leader a fast way to file actionable tickets from review runs.
2. Ensure engineering responses are traceable and explicitly marked as non-breaking.
3. Keep all records additive and auditable.

## Files

- `planning/RAPID_REVIEW_CELL/logs/TICKET_QUEUE.csv`: ticket intake owned by review/lead.
- `planning/RAPID_REVIEW_CELL/logs/TICKET_RESPONSES.csv`: engineering responses and status transitions.

## Command entrypoint

- `python3 scripts/rapid_review/tickets.py`

## Teams leader: send tickets

1. Run a review cycle and ensure `EVIDENCE_SUPPORT.csv` + `DECISION_DISPOSITION.csv` are updated.
2. Create tickets automatically from unsupported/caveated findings:

```bash
python3 scripts/rapid_review/tickets.py from-review \
  --review-run-id rrc_2026-02-10_001 \
  --owner-team marketing_tools \
  --opened-by teams_leader
```

3. For one-off manual tickets, use:

```bash
python3 scripts/rapid_review/tickets.py create \
  --review-run-id rrc_2026-02-10_001 \
  --artifact-id artifact_001 \
  --artifact-version v1 \
  --claim-id claim_001 \
  --priority high \
  --title "UNSUPPORTED: probiotic claim lacks evidence" \
  --requested-change "Bind claim to current formula spec or narrow claim language." \
  --acceptance-criteria "Evidence reference added and claim boundaries updated." \
  --owner-team marketing_tools \
  --opened-by teams_leader
```

## Engineering: respond to tickets

1. Pull current open work:

```bash
python3 scripts/rapid_review/tickets.py list --status open
```

2. Mark investigation started:

```bash
python3 scripts/rapid_review/tickets.py respond \
  --ticket-id RRC-20260210-001 \
  --responder-id eng_alex \
  --response-type analysis \
  --status-after in_progress \
  --change-ref planning/RAPID_REVIEW_CELL/logs/TICKET_QUEUE.csv \
  --non-breaking-change true \
  --notes "Confirmed issue and drafted additive fix plan."
```

3. After implementing additive changes, record completion:

```bash
python3 scripts/rapid_review/tickets.py respond \
  --ticket-id RRC-20260210-001 \
  --responder-id eng_alex \
  --response-type implemented \
  --status-after awaiting_review \
  --change-ref "src/file_a.ts;src/file_b.ts" \
  --verification-ref "cargo test -p comrade_lisp" \
  --non-breaking-change true \
  --notes "Added guardrail path and fallback; no existing flow removed."
```

4. After reviewer confirmation, close:

```bash
python3 scripts/rapid_review/tickets.py respond \
  --ticket-id RRC-20260210-001 \
  --responder-id teams_leader \
  --response-type verification \
  --status-after closed \
  --change-ref planning/RAPID_REVIEW_CELL/logs/DECISION_DISPOSITION.csv \
  --verification-ref "review rerun rrc_2026-02-10_002 approved_with_caveat" \
  --non-breaking-change true \
  --notes "Ticket accepted and closed after evidence rebinding."
```

## Non-breaking response policy

1. Prefer additive changes:
- add guards, defaults, adapters, fallback handling, feature flags.
2. Avoid removals or behavior flips in ticket scope unless explicitly approved.
3. Set `--non-breaking-change false` only when a breaking action is unavoidable and documented.

## Status model

- `open`: queued and unclaimed.
- `in_progress`: assigned and being implemented.
- `awaiting_review`: engineering done; waiting for leader/review confirmation.
- `resolved`: validated fix, pending final closure bookkeeping.
- `closed`: complete and accepted.
