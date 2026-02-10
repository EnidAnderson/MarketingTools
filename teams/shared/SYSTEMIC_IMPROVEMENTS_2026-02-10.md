# Systemic Improvements Applied (2026-02-10)

## Improvement 1: Formal micro-manager cycle layer
Added team-specific coaching notes and a cycle-level findings report to create fast feedback loops without changing authority boundaries.

## Improvement 2: Pipeline breach policy
Defined explicit response for out-of-order handoffs: record a block decision, update run status, and stop downstream progression until missing stages complete.

## Improvement 3: Request-ID hygiene policy
Established requirement for globally unique `request_id` values; superseding must use `supersedes_request_id` instead of reusing IDs.

## Improvement 4: Run-state accountability
Required append-only run-state updates per handoff/block so current phase and status are auditable.
