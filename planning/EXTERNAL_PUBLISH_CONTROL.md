# External Publish Control

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-016`

## Two-person rule

External publish requires two independent approvals:
1. Technical control reviewer.
2. Marketing/business owner.

Single-approver publish attempts are blocked.

## Approval record requirements

Append-only record must include:
1. `release_id`
2. `technical_reviewer`
3. `business_owner`
4. `timestamp_utc`
5. `evidence_refs`
6. `rollback_owner`

## Blocked states

1. Missing either approver role.
2. Any red mandatory release gate.
3. Unsupported claims in publish scope.
4. Active safe mode.

## Rollback protocol (30-minute actionable path)

1. 0-10 min: unpublish or disable external distribution.
2. 10-20 min: notify owners and incident channel.
3. 20-30 min: publish corrective holding statement and open remediation ticket.

## Cross-reference

Rapid Review process linkage:
- `planning/RAPID_REVIEW_CELL/SOP.md`

