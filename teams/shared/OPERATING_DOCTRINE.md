# Operating Doctrine

## Principle 1: Single-writer rule
Only QA Fixer may edit executable artifacts (code, config, schema, hooks, scripts).

## Principle 2: Colored teams are lenses
Blue/Red/Green/Black/White/Grey contribute orthogonal viewpoints; they do not supersede one another.

## Principle 3: Pipeline, not free-form conversation
Work proceeds in fixed sequence with append-only handoff outputs.

## Principle 3a: Adaptive pipeline modes
Two modes are allowed:
1. `full` mode: Blue -> Red -> Green -> Black -> White -> Grey -> QA Fixer.
2. `lite` mode: Blue -> Red -> White -> QA Fixer, with Green/Black/Grey optional and wakeable.
When `lite` skips optional teams, residual risk and escalation posture must be explicitly recorded.

## Principle 4: Actionable critique only
Any critique must be expressed as a concrete, testable request.

## Principle 5: Escalation, not argument
If a team disagrees with a prior output, it must file a formal change request.
Debate embedded inside team output artifacts is invalid.

## Principle 6: Globally unique change-request IDs
Every change request ID must be team-scoped and globally unique.
Required format: `CR-<TEAM>-<NNNN>` where:
1. `<TEAM>` is one of `BLUE`, `RED`, `GREEN`, `BLACK`, `WHITE`, `GREY`, `QA`.
2. `<NNNN>` is a zero-padded 4-digit sequence number chosen by the issuing team.
Examples: `CR-BLUE-0011`, `CR-RED-0042`, `CR-WHITE-0007`.

## Prohibited behaviors
1. Non-QA teams editing code/config.
2. Reopening settled prior-stage questions without a documented blocker.
3. Vague critiques without acceptance criteria.
4. QA Fixer edits that do not reference a `decision_id` or `change_request_id`.
5. Creating or consuming nonconforming/ambiguous change-request IDs when producing new requests.
