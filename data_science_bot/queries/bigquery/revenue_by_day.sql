-- Revenue by day from canonical purchase events
-- Replace {{project_id}} and {{dataset_id}}.

SELECT
  PARSE_DATE('%Y%m%d', event_date) AS event_day,
  SUM(COALESCE(ecommerce.purchase_revenue_in_usd, ecommerce.purchase_revenue, 0)) AS revenue,
  COUNT(*) AS purchase_events,
  COUNT(DISTINCT ecommerce.transaction_id) AS distinct_transactions
FROM `{{project_id}}.{{dataset_id}}.events_*`
WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 90 DAY))
  AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 1 DAY))
  AND event_name = 'purchase'
GROUP BY 1
ORDER BY 1;
