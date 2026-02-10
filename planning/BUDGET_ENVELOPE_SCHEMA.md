# Budget Envelope Schema

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-014`

## Required fields

1. `run_id`: non-empty string.
2. `owner_role`: non-empty string identifying accountable role.
3. `hard_cap_usd`: positive number; exceeding this hard-stops run.
4. `warning_threshold_usd`: positive number less than or equal to `hard_cap_usd`.
5. `cutoff_behavior`: one of `hard_stop | reduced_scope | lower_cost_provider`.
6. `exception_reference`: string (`none` when no exception is active).
7. `expiry_utc`: RFC3339 UTC timestamp for exception expiry (or run expiry policy horizon).

## Validation rules

1. Missing any required field is non-compliant.
2. `warning_threshold_usd > hard_cap_usd` is invalid.
3. `hard_cap_usd <= 0` is invalid.
4. `cutoff_behavior` must be deterministic and explicitly declared.
5. When `exception_reference != "none"`, `expiry_utc` must be in the future at approval time.

## Compliance consequence

Any run without a valid envelope is blocked by policy.

