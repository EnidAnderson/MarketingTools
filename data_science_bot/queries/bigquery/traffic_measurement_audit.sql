-- Session-level measurement audit for GA4 BigQuery export.
-- Replace {{project_id}} and {{dataset_id}}.
-- Window: trailing 30 complete UTC days.
-- This query flags instrumentation gaps before any segment-level dashboard claims ship.

WITH raw_events AS (
  SELECT
    PARSE_DATE('%Y%m%d', event_date) AS event_day,
    TIMESTAMP_MICROS(event_timestamp) AS event_ts,
    user_pseudo_id,
    event_name,
    COALESCE(NULLIF(device.category, ''), 'unknown') AS device_category,
    COALESCE(NULLIF(geo.country, ''), 'unknown') AS country,
    COALESCE(NULLIF(platform, ''), 'unknown') AS platform,
    NULLIF(traffic_source.source, '') AS raw_source_name,
    NULLIF(traffic_source.medium, '') AS raw_medium_name,
    CAST(
      (SELECT ep.value.int_value FROM UNNEST(event_params) ep WHERE ep.key = 'ga_session_id' LIMIT 1)
      AS STRING
    ) AS ga_session_id,
    COALESCE(
      (SELECT ep.value.int_value FROM UNNEST(event_params) ep WHERE ep.key = 'ga_session_number' LIMIT 1),
      SAFE_CAST(
        (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'ga_session_number' LIMIT 1)
        AS INT64
      )
    ) AS ga_session_number,
    (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'page_location' LIMIT 1) AS page_location,
    COALESCE(
      (SELECT ep.value.int_value FROM UNNEST(event_params) ep WHERE ep.key = 'engagement_time_msec' LIMIT 1),
      SAFE_CAST(
        (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'engagement_time_msec' LIMIT 1)
        AS INT64
      ),
      0
    ) AS engagement_time_msec,
    COALESCE(
      SAFE_CAST(
        (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'session_engaged' LIMIT 1)
        AS INT64
      ),
      (SELECT ep.value.int_value FROM UNNEST(event_params) ep WHERE ep.key = 'session_engaged' LIMIT 1),
      0
    ) AS session_engaged_flag,
    ecommerce.transaction_id AS transaction_id,
    COALESCE(ecommerce.purchase_revenue_in_usd, ecommerce.purchase_revenue, 0) AS purchase_revenue_usd
  FROM `{{project_id}}.{{dataset_id}}.events_*`
  WHERE _TABLE_SUFFIX BETWEEN FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 30 DAY))
    AND FORMAT_DATE('%Y%m%d', DATE_SUB(CURRENT_DATE('UTC'), INTERVAL 1 DAY))
),
session_events AS (
  SELECT
    *,
    COALESCE(
      ga_session_id,
      CONCAT('unknown-session:', user_pseudo_id, ':', FORMAT_TIMESTAMP('%Y%m%d%H', event_ts))
    ) AS session_key
  FROM raw_events
),
landing_pages AS (
  SELECT
    user_pseudo_id,
    session_key,
    ARRAY_AGG(page_location IGNORE NULLS ORDER BY event_ts LIMIT 1)[SAFE_OFFSET(0)] AS first_page_location
  FROM session_events
  WHERE event_name = 'page_view'
  GROUP BY 1, 2
),
session_rollup AS (
  SELECT
    se.event_day,
    se.user_pseudo_id,
    se.session_key,
    STARTS_WITH(se.session_key, 'unknown-session:') AS missing_session_key,
    ARRAY_AGG(se.device_category ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS device_category,
    ARRAY_AGG(se.country ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS country,
    ARRAY_AGG(se.platform ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS platform,
    LOGICAL_OR(se.raw_source_name IS NULL OR se.raw_medium_name IS NULL) AS source_attribution_missing,
    ARRAY_AGG(
      CASE
        WHEN se.raw_source_name IS NULL AND se.raw_medium_name IS NULL THEN 'unknown'
        WHEN se.raw_source_name IS NULL THEN CONCAT('unknown / ', se.raw_medium_name)
        WHEN se.raw_medium_name IS NULL THEN CONCAT(se.raw_source_name, ' / unknown')
        ELSE CONCAT(se.raw_source_name, ' / ', se.raw_medium_name)
      END
      ORDER BY se.event_ts
      LIMIT 1
    )[SAFE_OFFSET(0)] AS source_medium,
    ARRAY_AGG(
      CASE
        WHEN se.ga_session_number = 1 THEN 'new'
        WHEN se.ga_session_number > 1 THEN 'returning'
        ELSE 'unknown'
      END
      ORDER BY se.event_ts
      LIMIT 1
    )[SAFE_OFFSET(0)] AS visitor_type,
    lp.first_page_location,
    COUNT(*) AS event_count,
    COUNTIF(se.event_name = 'page_view') AS page_views,
    COUNTIF(se.event_name = 'user_engagement') AS user_engagement_events,
    COUNTIF(se.event_name = 'scroll') AS scroll_events,
    COUNTIF(se.event_name = 'view_item') AS product_views,
    COUNTIF(se.event_name = 'add_to_cart') AS add_to_cart_events,
    COUNTIF(se.event_name = 'begin_checkout') AS begin_checkout_events,
    COUNTIF(se.event_name = 'purchase') AS purchase_events,
    COUNT(DISTINCT IF(se.event_name = 'purchase', se.transaction_id, NULL)) AS purchase_transactions,
    SUM(IF(se.event_name = 'purchase', se.purchase_revenue_usd, 0)) AS purchase_revenue_usd,
    MAX(se.session_engaged_flag) > 0 OR SUM(se.engagement_time_msec) > 0 AS engaged_session
  FROM session_events se
  LEFT JOIN landing_pages lp
    ON se.user_pseudo_id = lp.user_pseudo_id
   AND se.session_key = lp.session_key
  GROUP BY 1, 2, 3, 4, lp.first_page_location
),
scored_sessions AS (
  SELECT
    *,
    CASE
      WHEN page_views >= 10
        AND user_engagement_events = 0
        AND scroll_events = 0
        AND product_views = 0
        AND add_to_cart_events = 0
        AND begin_checkout_events = 0
        AND purchase_events = 0 THEN TRUE
      WHEN event_count >= 50
        AND page_views = 0
        AND purchase_events = 0 THEN TRUE
      ELSE FALSE
    END AS suspicious_session_flag
  FROM session_rollup
),
totals AS (
  SELECT
    COUNT(*) AS total_sessions,
    SUM(CAST(missing_session_key AS INT64)) AS missing_session_key_sessions,
    SUM(CAST((first_page_location IS NULL) AS INT64)) AS missing_landing_sessions,
    SUM(CAST(source_attribution_missing AS INT64)) AS missing_source_sessions,
    SUM(CAST((country = 'unknown') AS INT64)) AS missing_country_sessions,
    SUM(CAST((page_views = 0) AS INT64)) AS sessions_without_page_view,
    SUM(CAST((NOT engaged_session) AS INT64)) AS zero_engagement_sessions,
    SUM(CAST(suspicious_session_flag AS INT64)) AS suspicious_sessions,
    SUM(CAST((source_attribution_missing OR source_medium = '(direct) / (none)') AS INT64)) AS unattributed_sessions,
    SUM(CAST((purchase_transactions > 0) AS INT64)) AS purchase_sessions,
    SUM(CAST((purchase_transactions > 0 AND first_page_location IS NULL) AS INT64)) AS purchase_sessions_missing_landing,
    SUM(CAST((purchase_transactions > 0 AND source_attribution_missing) AS INT64)) AS purchase_sessions_missing_source,
    SUM(CAST((purchase_transactions > 0 AND missing_session_key) AS INT64)) AS purchase_sessions_missing_session_key
  FROM scored_sessions
),
audit_rows AS (
  SELECT
    'session_key_missing_ratio' AS audit_name,
    'high' AS severity,
    SAFE_DIVIDE(missing_session_key_sessions, total_sessions) AS observed_ratio,
    0.02 AS threshold_ratio,
    missing_session_key_sessions AS affected_sessions,
    total_sessions,
    'Sessionization falls back to synthetic keys; downstream cohort and funnel joins become fragile when this ratio rises.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'landing_page_missing_ratio' AS audit_name,
    'high' AS severity,
    SAFE_DIVIDE(missing_landing_sessions, total_sessions) AS observed_ratio,
    0.05 AS threshold_ratio,
    missing_landing_sessions AS affected_sessions,
    total_sessions,
    'Landing page taxonomy depends on first observed page_view with page_location.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'source_medium_missing_ratio' AS audit_name,
    'medium' AS severity,
    SAFE_DIVIDE(missing_source_sessions, total_sessions) AS observed_ratio,
    0.10 AS threshold_ratio,
    missing_source_sessions AS affected_sessions,
    total_sessions,
    'Unknown source attribution reduces trust in channel and landing-entry analyses.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'country_missing_ratio' AS audit_name,
    'low' AS severity,
    SAFE_DIVIDE(missing_country_sessions, total_sessions) AS observed_ratio,
    0.05 AS threshold_ratio,
    missing_country_sessions AS affected_sessions,
    total_sessions,
    'Geo gaps degrade geography cuts but are not an automatic block on all analysis.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'sessions_without_page_view_ratio' AS audit_name,
    'medium' AS severity,
    SAFE_DIVIDE(sessions_without_page_view, total_sessions) AS observed_ratio,
    0.10 AS threshold_ratio,
    sessions_without_page_view AS affected_sessions,
    total_sessions,
    'No page_view means landing-page derivation is impossible for that session.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'zero_engagement_session_ratio' AS audit_name,
    'low' AS severity,
    SAFE_DIVIDE(zero_engagement_sessions, total_sessions) AS observed_ratio,
    0.35 AS threshold_ratio,
    zero_engagement_sessions AS affected_sessions,
    total_sessions,
    'High zero-engagement share can indicate low-quality traffic, consent loss, or instrumentation drift.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'suspicious_session_ratio' AS audit_name,
    'medium' AS severity,
    SAFE_DIVIDE(suspicious_sessions, total_sessions) AS observed_ratio,
    0.02 AS threshold_ratio,
    suspicious_sessions AS affected_sessions,
    total_sessions,
    'Heuristic: unusually high event volume without engagement, scroll, commerce, or page-view context.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'purchase_sessions_missing_landing_ratio' AS audit_name,
    'high' AS severity,
    SAFE_DIVIDE(purchase_sessions_missing_landing, NULLIF(purchase_sessions, 0)) AS observed_ratio,
    0.00 AS threshold_ratio,
    purchase_sessions_missing_landing AS affected_sessions,
    purchase_sessions AS total_sessions,
    'Purchase-bearing sessions should retain enough context to reconstruct entry path.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'purchase_sessions_missing_source_ratio' AS audit_name,
    'high' AS severity,
    SAFE_DIVIDE(purchase_sessions_missing_source, NULLIF(purchase_sessions, 0)) AS observed_ratio,
    0.05 AS threshold_ratio,
    purchase_sessions_missing_source AS affected_sessions,
    purchase_sessions AS total_sessions,
    'Purchase-bearing sessions with unknown source attribution weaken channel-level recommendations.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'purchase_sessions_missing_session_key_ratio' AS audit_name,
    'high' AS severity,
    SAFE_DIVIDE(purchase_sessions_missing_session_key, NULLIF(purchase_sessions, 0)) AS observed_ratio,
    0.00 AS threshold_ratio,
    purchase_sessions_missing_session_key AS affected_sessions,
    purchase_sessions AS total_sessions,
    'Purchase-bearing rows must be sessionized reliably before cohort or funnel claims are trusted.' AS notes
  FROM totals

  UNION ALL

  SELECT
    'unattributed_session_share' AS audit_name,
    'low' AS severity,
    SAFE_DIVIDE(unattributed_sessions, total_sessions) AS observed_ratio,
    0.40 AS threshold_ratio,
    unattributed_sessions AS affected_sessions,
    total_sessions,
    'Direct or unknown sessions are monitored for attribution blind spots but are not, by themselves, proof of bad data.' AS notes
  FROM totals
)
SELECT
  audit_name,
  severity,
  observed_ratio,
  threshold_ratio,
  observed_ratio <= threshold_ratio AS passed,
  affected_sessions,
  total_sessions,
  notes
FROM audit_rows
ORDER BY
  CASE severity
    WHEN 'high' THEN 1
    WHEN 'medium' THEN 2
    ELSE 3
  END,
  audit_name;
