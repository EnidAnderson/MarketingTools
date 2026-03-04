-- Device-level funnel counts and rates.
-- Replace {{project_id}} and {{dataset_id}}.

WITH base AS (
  SELECT
    PARSE_DATE('%Y%m%d', event_date) AS event_day,
    device.category AS device_category,
    event_name,
    user_pseudo_id,
    ecommerce.transaction_id
  FROM `{{project_id}}.{{dataset_id}}.events_*`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 30 DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 1 DAY))
), agg AS (
  SELECT
    device_category,
    COUNTIF(event_name = 'session_start') AS sessions,
    COUNTIF(event_name = 'view_item') AS product_views,
    COUNTIF(event_name = 'add_to_cart') AS add_to_cart,
    COUNTIF(event_name = 'begin_checkout') AS checkout,
    COUNTIF(event_name = 'purchase') AS purchases,
    COUNT(DISTINCT IF(event_name = 'purchase', transaction_id, NULL)) AS distinct_purchase_tx
  FROM base
  GROUP BY 1
)
SELECT
  device_category,
  sessions,
  product_views,
  add_to_cart,
  checkout,
  purchases,
  distinct_purchase_tx,
  SAFE_DIVIDE(product_views, sessions) AS view_rate,
  SAFE_DIVIDE(add_to_cart, product_views) AS add_to_cart_rate,
  SAFE_DIVIDE(checkout, add_to_cart) AS checkout_rate,
  SAFE_DIVIDE(purchases, checkout) AS purchase_rate
FROM agg
ORDER BY purchases DESC;
