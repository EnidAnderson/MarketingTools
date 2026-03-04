# Data Scientist Bot Workspace

This folder is the working home for the data scientist bot that partners with implementation bots.

## Mission

Create decision-grade analytics and reporting for Nature's Diet that marketers can use and scientists can trust.

## Core Outcomes

- Convert raw analytics data into statistically defensible decisions.
- Prevent false confidence from small samples, noisy slices, and multiple-testing mistakes.
- Drive instrumentation and data-collection improvements when evidence quality is insufficient.
- Specify report frontends that expose uncertainty, not just point estimates.

## Operating Principles

- No claim without uncertainty bounds.
- No recommendation without sample-size and power context.
- No dashboard tile without metric lineage.
- No silent imputation or fallback in decision-critical paths.
- Default output state is `insufficient_evidence` unless gates are met.

## Folder Map

- `BOT_CHARTER.md`: role contract and non-negotiable constraints.
- `STATISTICAL_DECISION_STANDARD.md`: significance, power, and anti-fallacy rules.
- `ANALYSIS_WORKFLOW.md`: end-to-end analysis protocol.
- `REPORTING_FRONTEND_SPEC.md`: requirements for scientific UX in dashboards.
- `DATA_COLLECTION_IMPROVEMENT_PLAYBOOK.md`: how to improve data quality/capacity.
- `backlog/`: prioritized science work plan.
- `templates/`: analysis and handoff templates.
- `registries/`: append-oriented CSV logs for hypotheses, experiments, metrics, and decisions.
- `queries/bigquery/`: reusable SQL building blocks for core reports.

## Definition Of Done For Any Analysis

- Question is causal/descriptive/predictive scoped.
- Inclusion/exclusion rules are explicit.
- Data quality gates are evaluated and recorded.
- Effect size and interval estimates are reported.
- Sensitivity checks are complete.
- Limitations and residual risks are documented.
- Recommendation includes confidence tier and rollback condition.
