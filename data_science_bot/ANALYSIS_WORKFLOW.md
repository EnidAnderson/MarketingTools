# Analysis Workflow

## Phase 1: Intake And Decision Framing

- Define decision owner and decision deadline.
- Convert request into measurable hypothesis.
- Specify target metric, guardrail metrics, and failure modes.

## Phase 2: Data Contract Check

- Confirm source systems and raw-field lineage.
- Run data quality gates from `STATISTICAL_DECISION_STANDARD.md`.
- Log any blocker in `registries/analysis_decision_log.csv`.

## Phase 3: Analysis Design

- Choose design: descriptive trend, quasi-experiment, A/B test, forecast.
- Define primary estimate, interval method, and sensitivity checks.
- Freeze inclusion/exclusion criteria before computing outcomes.

## Phase 4: Computation And Diagnostics

- Run canonical SQL from `queries/bigquery/` or approved variants.
- Compute effect size, uncertainty, and practical significance.
- Perform diagnostics for leakage, imbalance, and outlier sensitivity.

## Phase 5: Interpretation

- Separate observed evidence from inference.
- Document assumptions and what would falsify conclusions.
- Assign confidence tier and action state.

## Phase 6: Handoff And Productization

- Create implementation request via `templates/se_change_request_template.md`.
- Define acceptance tests for backend and frontend.
- Add dashboard requirements via `templates/frontend_report_spec_template.md`.

## Phase 7: Post-Decision Follow-Up

- Track realized outcomes vs expected outcomes.
- Update priors/thresholds when calibration drifts.
- Record lessons in `backlog/WEEKLY_SCIENCE_BACKLOG.md`.
