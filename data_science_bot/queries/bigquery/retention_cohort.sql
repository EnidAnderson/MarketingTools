-- Weekly cohort retention by first purchase week.
-- Replace {{project_id}} and {{dataset_id}}.

WITH purchases AS (
  SELECT
    user_pseudo_id,
    DATE(TIMESTAMP_MICROS(event_timestamp), 'UTC') AS purchase_date
  FROM `{{project_id}}.{{dataset_id}}.events_*`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 180 DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 1 DAY))
    AND event_name = 'purchase'
), first_purchase AS (
  SELECT
    user_pseudo_id,
    MIN(purchase_date) AS first_purchase_date
  FROM purchases
  GROUP BY 1
), cohort_events AS (
  SELECT
    fp.user_pseudo_id,
    DATE_TRUNC(fp.first_purchase_date, WEEK(MONDAY)) AS cohort_week,
    DATE_DIFF(p.purchase_date, fp.first_purchase_date, WEEK) AS week_number
  FROM first_purchase fp
  JOIN purchases p USING (user_pseudo_id)
  WHERE DATE_DIFF(p.purchase_date, fp.first_purchase_date, WEEK) BETWEEN 0 AND 12
)
SELECT
  cohort_week,
  week_number,
  COUNT(DISTINCT user_pseudo_id) AS retained_users
FROM cohort_events
GROUP BY 1,2
ORDER BY cohort_week, week_number;
