# Query Pack

These queries are baseline analysis assets for the data scientist bot.

## Usage

- Treat these as reference templates.
- Parameterize project/dataset/date ranges in execution scripts or notebooks.
- Record exact query variant hash in analysis briefs for reproducibility.

## Current Pack

- `bigquery/revenue_by_day.sql`
- `bigquery/funnel_by_device.sql`
- `bigquery/retention_cohort.sql`
- `bigquery/duplicate_purchase_audit.sql`
- `bigquery/traffic_taxonomy_dimension_inventory.sql`
- `bigquery/traffic_measurement_audit.sql`

## Recommended Execution Order For Traffic Science

1. `bigquery/traffic_measurement_audit.sql`
2. `bigquery/traffic_taxonomy_dimension_inventory.sql`
3. `bigquery/funnel_by_device.sql`
4. `bigquery/retention_cohort.sql`
5. `bigquery/duplicate_purchase_audit.sql`
