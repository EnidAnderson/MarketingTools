# Statistical Decision Standard

This standard defines when data is strong enough to drive real product and marketing decisions.

## 1. Question Classification

Every analysis must declare one of:

- Descriptive: what happened
- Predictive: what is likely next
- Causal: what intervention changed outcomes

Causal claims require explicit design assumptions.

## 2. Data Readiness Gates

Decision claims are blocked when any gate fails.

- Freshness: critical metrics delayed beyond SLA.
- Completeness: key dimensions materially missing.
- Deduplication: duplicate-event risk above tolerance.
- Identity resolution: join coverage below threshold.
- Lineage: metric cannot be traced to source fields.

## 3. Effect-Size-First Reporting

Each result must include:

- Point estimate
- Interval estimate
- Baseline and percent change
- Practical significance against business threshold

P-values are never reported alone.

## 4. Significance And Multiple Testing

- Default alpha is 0.05 two-sided.
- For test families, control false discovery rate (Benjamini-Hochberg).
- Sequential looks require declared stopping rule.
- If stopping rule is violated, label output `exploratory_only`.

## 5. Small-Sample Policy

When sample size is low, apply conservative inference.

- Use exact or bootstrap intervals for unstable rates.
- Use hierarchical pooling/shrinkage for sparse segments.
- Prefer posterior probability of direction with sensitivity checks.
- Default classification is `insufficient_evidence` unless minimum precision target is met.

## 6. Power And Detectability

Before experiments:

- Define minimum detectable effect tied to business value.
- Compute required sample size and planned horizon.
- Abort early claims unless predeclared early-stop criteria are met.

## 7. Decision States

- `ship`: estimate precise, effect practically meaningful, all gates pass.
- `iterate`: promising but precision or data quality insufficient.
- `hold`: negative or null with adequate power, or quality risk is high.
- `instrument_first`: missing instrumentation prevents valid decision.

## 8. Anti-Pattern Blocklist

- Simpson's paradox ignored in aggregate-only claims.
- Segment slicing until significance appears.
- Treating modeled attribution as ground truth revenue.
- Mixing calendar windows between compared groups.
- Ignoring selection bias in campaign cohorts.
- Equating non-significant with "no effect" under low power.
