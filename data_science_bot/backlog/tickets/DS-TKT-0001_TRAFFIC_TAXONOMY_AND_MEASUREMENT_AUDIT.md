# DS-TKT-0001 Traffic Taxonomy And Measurement Audit

## Metadata

- Ticket ID: `DS-TKT-0001`
- Priority: `P0`
- Status: `in_progress`
- Question type: `descriptive` and `instrumentation audit`
- Decision owner: marketing leadership plus analytics platform owner
- Primary consumer: data scientist bot, dashboard implementer, marketing operators
- Dependencies:
  - observed read-only GA4 BigQuery access
  - canonical purchase truth guardrails already enforced in the analytics pipeline

## Objective

Create a trustworthy session-level taxonomy for traffic analysis and identify instrumentation or data-collection gaps that would make downstream funnel, cohort, or attribution analysis unsafe.

## Questions This Ticket Must Answer

1. What share of sessions can be classified by device, landing page family, landing path, source/medium, visitor type, geography, weekday, and hour?
2. Which dimensions have unacceptable unknown or missing share?
3. What share of traffic is suspicious, low-information, or unattributed enough to distort executive reporting?
4. Are purchase-bearing sessions missing critical context such as landing page or source attribution?
5. Which instrumentation gaps must be fixed before segment-level decision claims are allowed?

## Deliverables

1. Query pack:
   - `data_science_bot/queries/bigquery/traffic_taxonomy_dimension_inventory.sql`
   - `data_science_bot/queries/bigquery/traffic_measurement_audit.sql`
2. Analysis brief:
   - `data_science_bot/backlog/tickets/DS-TKT-0001_EXECUTION_BRIEF.md`
3. Backlog and metric-lineage updates:
   - `data_science_bot/backlog/SCIENCE_TICKET_QUEUE.md`
   - `data_science_bot/registries/metric_registry.csv`

## Acceptance Criteria

1. The taxonomy query rolls raw GA4 events to a session-level table keyed by `user_pseudo_id + ga_session_id` with an explicit fallback for missing session ids.
2. The taxonomy inventory reports the following dimensions:
   - `device_category`
   - `visitor_type`
   - `source_medium`
   - `landing_family`
   - `landing_page_group`
   - `landing_path`
   - `country`
   - `weekday`
   - `hour_of_day_utc`
   - `platform`
3. The measurement audit reports explicit pass/fail thresholds for:
   - missing session key ratio
   - missing landing page ratio
   - missing source/medium ratio
   - missing country ratio
   - sessions without page views
   - suspicious session ratio
   - purchase sessions with missing landing page
   - purchase sessions with missing source attribution
4. Suspicious traffic logic is heuristic and documented, not silently folded into truth metrics.
5. The output is sufficient for a scientist to decide whether downstream segment analyses should be `ship`, `hold`, or `instrument_first`.

## Non-Goals

- Final causal attribution
- Multi-touch media mix modeling
- Productizing the audit into the executive dashboard in this ticket
- Treating high direct traffic share as an automatic data-quality failure

## Runbook

1. Run `traffic_measurement_audit.sql` for the trailing 30 complete days.
2. If any high-severity gate fails, label downstream segment claims `instrument_first` until corrected.
3. Run `traffic_taxonomy_dimension_inventory.sql` for the same window.
4. Record the exact query variant hash and any modifications in the analysis brief.
5. If the audit reveals missing raw fields or broken taxonomy assumptions, file an SE change request before implementing higher-order reports.

## Expected Follow-On Tickets Unblocked By Completion

- `DS-TKT-0002` Entry Path to Purchase Cohort Analysis
- `DS-TKT-0003` Segment-Level Funnel Survival
- `DS-TKT-0005` Session Quality Scoring
- `DS-TKT-0007` Statistical Power and Decision Thresholds
