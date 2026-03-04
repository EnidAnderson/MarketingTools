-- Duplicate purchase audit by transaction and event signature.
-- Replace {{project_id}} and {{dataset_id}}.

WITH purchase_rows AS (
  SELECT
    PARSE_DATE('%Y%m%d', event_date) AS event_day,
    user_pseudo_id,
    event_timestamp,
    ecommerce.transaction_id AS transaction_id,
    COALESCE(ecommerce.purchase_revenue_in_usd, ecommerce.purchase_revenue, 0) AS purchase_revenue,
    (SELECT value.int_value FROM UNNEST(event_params) WHERE key = 'ga_session_id') AS ga_session_id,
    event_bundle_sequence_id,
    batch_event_index
  FROM `{{project_id}}.{{dataset_id}}.events_*`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 30 DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 1 DAY))
    AND event_name = 'purchase'
), duplicate_eval AS (
  SELECT
    event_day,
    transaction_id,
    COUNT(*) AS row_count,
    SUM(purchase_revenue) AS summed_revenue,
    COUNT(DISTINCT TO_JSON_STRING(STRUCT(
      user_pseudo_id,
      event_timestamp,
      ga_session_id,
      event_bundle_sequence_id,
      batch_event_index,
      purchase_revenue
    ))) AS unique_signatures
  FROM purchase_rows
  GROUP BY 1,2
)
SELECT
  event_day,
  transaction_id,
  row_count,
  unique_signatures,
  summed_revenue,
  row_count - unique_signatures AS strict_duplicate_rows,
  SAFE_DIVIDE(row_count - unique_signatures, row_count) AS strict_duplicate_ratio
FROM duplicate_eval
WHERE row_count > 1 OR unique_signatures < row_count
ORDER BY strict_duplicate_ratio DESC, row_count DESC;
