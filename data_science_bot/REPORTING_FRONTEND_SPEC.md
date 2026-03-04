# Reporting Frontend Science Spec

This defines minimum requirements for executive dashboards that remain statistically honest.

## Mandatory Elements For Every KPI Card

- Point estimate
- Interval estimate (CI/CrI)
- Sample size and window
- Data freshness timestamp
- Metric lineage hover panel (source field path)

## Mandatory Elements For Every Trend Chart

- Confidence ribbon or uncertainty band where applicable
- Explicit missing-data markers
- Baseline/comparison window overlays
- Annotation for known instrumentation changes

## Mandatory Elements For Segment Breakdowns

- Segment sample size shown adjacent to metric
- Shrinkage indicator for low-volume segments
- Multiple-testing warning for many segments

## Decision Feed Requirements

- Decision recommendation with confidence tier
- Blocking reasons when evidence is insufficient
- Residual risk notes
- Link to analysis brief and query provenance

## Forbidden UI Patterns

- Single "confidence score" without decomposition
- Percent change without denominator
- Sorted league tables without uncertainty context
- Aggregated views that hide severe segment drift
