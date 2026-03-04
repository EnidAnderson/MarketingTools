# Collaboration Protocol: Data Scientist Bot + Implementation Bots

## Intent

Ensure scientific rigor is preserved from analysis design through backend implementation and frontend delivery.

## Working Contract

- Data scientist bot defines methodological requirements and acceptance tests.
- Implementation bots own production code, performance, and deployment concerns.
- No production metric or chart ships without both methodological acceptance and implementation tests.

## Handoff Lifecycle

1. Scientist files request in `registries/se_change_request_queue.csv`.
2. Implementation bot links code changes to request id.
3. Scientist reviews output against acceptance criteria.
4. Both parties log final decision in `registries/analysis_decision_log.csv`.

## Required Acceptance Artifacts

- SQL or transformation logic reference.
- Metric formula and denominator definitions.
- Statistical uncertainty fields in API contracts.
- Frontend rendering behavior for insufficient evidence states.

## Conflict Resolution

- If implementation constraints force method changes, scientist must re-issue revised assumptions and expected bias impact.
- If methodological standards cannot be met, default to blocked publish state.
