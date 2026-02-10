# Team Lead Standing Orders

Effective date: 2026-02-10

## Operating posture

1. Prioritize system hardening over feature expansion.
2. Do not optimize immature tools before control framework is stable.
3. Prefer explicit process controls over implicit assumptions.

## Standing priorities (always-on)

1. Security hardening
- prevent secret leakage,
- reduce unsafe publication risk,
- maintain threat/control ownership clarity.

2. Budget hardening
- enforce hard caps,
- stop uncontrolled spend paths,
- log and expire exceptions.

3. Role hardening
- maintain clear decision rights,
- enforce escalation protocol,
- prevent authority ambiguity in safety decisions.

4. Approach hardening
- mandatory release gates,
- explicit risk register,
- incident response readiness,
- append-only evidence trails.

## Mandatory weekly outputs from implementing bot

1. Control status snapshot against `planning/reports/HARDENING_CONTROL_MATRIX_2026-02-10.md`.
2. Top 5 open risks with owner and mitigation ETA.
3. Budget exception summary (new/active/expired).
4. Role conflict incidents and resolution latency.

## Prohibited behaviors

1. Shipping externally-facing claims without evidence gate closure.
2. Bypassing secret scans or release gate checks.
3. Introducing new workflows without explicit owner and budget envelope.
4. Approving role changes without updating role contract docs.

## Escalation thresholds

Escalate immediately if any of the following occur:
1. suspected secret exposure,
2. spend cap exceeded without approved exception,
3. externally-facing false claim reaches publish path,
4. unresolved role conflict > 24 hours for safety-critical decision.
