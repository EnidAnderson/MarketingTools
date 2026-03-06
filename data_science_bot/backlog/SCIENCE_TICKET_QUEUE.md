# Science Ticket Queue

This queue converts the data scientist workstream into named, schedulable tickets.

## Active Queue

| Ticket ID | Title | Priority | Status | Goal |
| --- | --- | --- | --- | --- |
| DS-TKT-0001 | Traffic Taxonomy and Measurement Audit | P0 | in_progress | Build a trustworthy session-level segmentation baseline and detect instrumentation gaps before deeper inference. |
| DS-TKT-0002 | Entry Path to Purchase Cohort Analysis | P1 | planned | Quantify which first-touch landing paths and channels lead to 1-day, 7-day, and 30-day purchase follow-through. |
| DS-TKT-0003 | Segment-Level Funnel Survival | P1 | planned | Localize funnel leakage by device, landing page family, and traffic segment with uncertainty-aware comparisons. |
| DS-TKT-0004 | Purchase Truth and Duplicate-Stream Forensics | P1 | planned | Prove purchase truth against raw exports and isolate any remaining duplicate or orphaned purchase paths. |
| DS-TKT-0005 | Session Quality Scoring | P2 | planned | Separate high-intent traffic from low-information or suspicious traffic before marketing decisions rely on aggregate volume. |
| DS-TKT-0006 | Retention and Repeat-Purchase Survival | P1 | planned | Estimate repeat-purchase timing and retention curves that are credible under small-sample constraints. |
| DS-TKT-0007 | Statistical Power and Decision Thresholds | P1 | planned | Block underpowered claims and define minimum detectable effect rules for dashboard recommendations. |
| DS-TKT-0008 | Causal Opportunity Queue | P2 | planned | Rank the highest-value experiments and quasi-experimental studies supported by current traffic and event coverage. |
| DS-TKT-0009 | Baseline Modeling and Anomaly Triage | P2 | planned | Build weekday and seasonal baselines so anomalies are distinguished from normal operating variance. |
| DS-TKT-0010 | Scientist-Facing Evidence Pack | P2 | planned | Standardize an evidence packet with query lineage, uncertainty, sensitivity checks, and explicit non-claims. |

## Sequencing Logic

1. `DS-TKT-0001` is first because every downstream report depends on stable taxonomy and instrumentation coverage.
2. `DS-TKT-0002`, `DS-TKT-0003`, and `DS-TKT-0006` are the first inference-bearing reports once segmentation trust is established.
3. `DS-TKT-0004` and `DS-TKT-0007` are governance multipliers that harden truth metrics and block weak claims.
4. `DS-TKT-0005`, `DS-TKT-0008`, `DS-TKT-0009`, and `DS-TKT-0010` raise the system from descriptive analytics to scientific operations.

## Current Week Focus

1. `DS-TKT-0001` Traffic Taxonomy and Measurement Audit
2. `DS-TKT-0003` Segment-Level Funnel Survival
3. `DS-TKT-0006` Retention and Repeat-Purchase Survival

## Exit Criteria For This Queue Phase

- Canonical session taxonomy exists for device, landing page, source/medium, visitor type, geography, and time-of-week.
- Raw purchase and checkout events are reconciled tightly enough to support truth metrics.
- Dashboard claims expose uncertainty, sample context, and power status.
- Analyst-facing packets can be audited without reading source code.
