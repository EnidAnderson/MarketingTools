# High-Importance Ticket Tracker

Last updated: 2026-02-10  
Owner: qa_fixer

Provenance:
- `decision_id=DEC-0001`
- `change_request_id=RQ-029`

## Fulfilled

1. `RQ-029` (P0) pipeline-order validator
2. `RQ-030` (P0) append-only validator
3. `RQ-031` (P0) QA edit-authority validator
4. `RQ-032` (P1) validation orchestrator/report
5. `RQ-033` (P1) CI gate + operator doc
6. `RQ-013` (P0) governance preflight script + AGENTS wiring
7. `RQ-014` (P0) budget envelope schema/template + policy references
8. `RQ-015` (P0) role-permission matrix + least-privilege baseline
9. `RQ-016` (P0) external publish two-person control + rollback protocol
10. `RQ-021` (P0) hardening control-binding tables in core plans
11. `RQ-022` (P0) quantified acceptance/SLO thresholds in core plans
12. `RQ-023` (P0) budget envelope + hard-stop policy per milestone
13. `RQ-024` (P0) security assumptions and abuse-case handling in core plans
14. `RQ-017` (P1) kill-switch and safe-mode operations protocol
15. `RQ-018` (P1) monthly tabletop drill program + template
16. `RQ-019` (P1) hardening metrics dictionary
17. `RQ-020` (P2) governance drift review
18. `RQ-025` (P1) role-bound signoff matrix in core plans
19. `RQ-026` (P1) failure and rollback transition map in core plans
20. `RQ-027` (P1) ADR trigger checkpoints in core plans
21. `RQ-028` (P2) cross-plan glossary and schema dictionary

## Remaining (P0)

1. None in currently scoped P0 queues.

## Active hard-failure blockers (from validation run)

1. Pipeline-order violation:
- expected `green -> black` handoff, observed `white -> grey`.
- logged as `DEC-0002` in `data/team_ops/decision_log.csv`.
2. Missing QA provenance reference on executable artifact edit:
- `planning/examples/mvp_pipeline_example.json`.
- logged as `DEC-0003` in `data/team_ops/decision_log.csv`.

## Remaining (P1/P2)

1. None in currently scoped Team Lead operations/spec queues.
