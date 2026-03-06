# DS-TKT-0001 Execution Brief

## Metadata

- Analysis id: `pending_run_id`
- Ticket id: `DS-TKT-0001`
- Owner: `data_science_bot`
- Decision owner: `marketing leadership + analytics platform owner`
- Date: `pending_execution_date`
- Decision deadline: `before any new segment-level dashboard claims ship`

## Question

- Question type: `descriptive` and `instrumentation audit`
- Business decision impacted: whether current traffic data supports trustworthy segmentation and follow-on scientific reports
- Primary hypothesis: at least 95% of decision-relevant sessions can be classified into the canonical traffic taxonomy without forcing silent imputation
- Alternative hypotheses:
  - landing-page or source attribution missingness is too high for trustworthy segment reporting
  - suspicious or low-information sessions materially distort aggregate traffic counts
  - purchase-bearing sessions are missing core context needed for downstream cohort analysis

## Data Scope

- Sources: `GA4 BigQuery export events_*`
- Time window: trailing 30 complete UTC days unless a decision owner specifies a different fixed window
- Inclusion criteria:
  - all raw GA4 events with valid `event_date`
  - session rollup keyed by `user_pseudo_id + ga_session_id`
- Exclusion criteria:
  - none at raw ingest; suspicious sessions are flagged, not discarded
- Known missingness:
  - `traffic_source.source` and `traffic_source.medium` may be null, partial, or explicit direct values
  - landing page is only available where `page_view` emitted `page_location`

## Method

- Design: session-level taxonomy inventory plus measurement audit
- Primary metric: `landing_page_missing_ratio` and `source_medium_missing_ratio`
- Guardrails:
  - `purchase_sessions_missing_landing_ratio`
  - `purchase_sessions_missing_source_ratio`
  - `suspicious_session_ratio`
  - `session_key_missing_ratio`
- Interval method: descriptive ratios first; confidence intervals added only after audit passes and segment reports begin
- Multiple-testing correction: not applicable for the audit itself; any segment comparisons triggered by the audit must use controlled procedures

## Query Assets

1. `data_science_bot/queries/bigquery/traffic_measurement_audit.sql`
2. `data_science_bot/queries/bigquery/traffic_taxonomy_dimension_inventory.sql`

## Execution Notes

- Record exact query text hash here after execution.
- Record BigQuery project and dataset used here after execution.
- If any query requires a schema-specific adjustment, note the delta before interpreting results.

## Results

- Estimate: `pending`
- Interval: `not yet computed`
- Practical significance threshold:
  - `session_key_missing_ratio <= 0.02`
  - `landing_page_missing_ratio <= 0.05`
  - `source_medium_missing_ratio <= 0.10`
  - `purchase_sessions_missing_landing_ratio = 0`
  - `purchase_sessions_missing_source_ratio <= 0.05`
  - `suspicious_session_ratio <= 0.02`
- Outcome vs threshold: `pending`

## Validity Checks

- Data quality gates status: `pending`
- Sensitivity analyses:
  - compare direct share versus source-missing share
  - inspect top landing paths for unexpected taxonomy drift
  - inspect suspicious-session concentration by device and source
- Residual confounding risks:
  - direct traffic is not inherently a bug
  - GA4 traffic source fields can reflect acquisition rather than exact session attribution semantics
  - missing page views may reflect consent or instrumentation choices, not only implementation defects

## Decision

- Confidence tier: `pending`
- Action state: `pending`
- Required implementation changes: `pending`
