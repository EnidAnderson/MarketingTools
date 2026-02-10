# Review Artifacts

Append-only review/approval payloads validated against `teams/schemas/review_artifact.schema.json`.

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=CR-WHITE-0002`
- `change_request_id=CR-BLACK-0003`

Mutation policy:
1. Add new files for new approvals.
2. Do not rewrite existing artifact payload files.
