-- Canonical session-level taxonomy inventory for GA4 BigQuery export.
-- Replace {{project_id}} and {{dataset_id}}.
-- Window: trailing 30 complete UTC days.

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
    (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'page_referrer' LIMIT 1) AS page_referrer,
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
    ARRAY_AGG(page_location IGNORE NULLS ORDER BY event_ts LIMIT 1)[SAFE_OFFSET(0)] AS first_page_location,
    ARRAY_AGG(page_referrer IGNORE NULLS ORDER BY event_ts LIMIT 1)[SAFE_OFFSET(0)] AS first_page_referrer
  FROM session_events
  WHERE event_name = 'page_view'
  GROUP BY 1, 2
),
session_rollup_base AS (
  SELECT
    se.event_day,
    EXTRACT(DAYOFWEEK FROM se.event_day) AS weekday_index,
    EXTRACT(HOUR FROM MIN(se.event_ts)) AS hour_of_day_utc,
    se.user_pseudo_id,
    se.session_key,
    ARRAY_AGG(se.device_category ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS device_category,
    ARRAY_AGG(se.country ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS country,
    ARRAY_AGG(se.platform ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS platform,
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
  GROUP BY 1, 2, 4, 5, lp.first_page_location
),
session_rollup_paths AS (
  SELECT
    event_day,
    weekday_index,
    CASE weekday_index
      WHEN 1 THEN 'sunday'
      WHEN 2 THEN 'monday'
      WHEN 3 THEN 'tuesday'
      WHEN 4 THEN 'wednesday'
      WHEN 5 THEN 'thursday'
      WHEN 6 THEN 'friday'
      WHEN 7 THEN 'saturday'
      ELSE 'unknown'
    END AS weekday_name,
    hour_of_day_utc,
    user_pseudo_id,
    session_key,
    device_category,
    country,
    platform,
    source_medium,
    visitor_type,
    first_page_location,
    COALESCE(
      REGEXP_EXTRACT(first_page_location, r'https?://[^/]+(/[^?#]*)'),
      REGEXP_EXTRACT(first_page_location, r'(^/[^?#]*)')
    ) AS landing_path,
    event_count,
    page_views,
    user_engagement_events,
    scroll_events,
    product_views,
    add_to_cart_events,
    begin_checkout_events,
    purchase_events,
    purchase_transactions,
    purchase_revenue_usd,
    engaged_session
  FROM session_rollup_base
),
session_rollup AS (
  SELECT
    event_day,
    weekday_index,
    weekday_name,
    hour_of_day_utc,
    user_pseudo_id,
    session_key,
    device_category,
    country,
    platform,
    source_medium,
    visitor_type,
    first_page_location,
    landing_path,
    CASE
      WHEN first_page_location IS NULL THEN 'unknown'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'/products?/') THEN 'product_detail'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'/collections?/|/shop') THEN 'collection_or_shop'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'/cart|/checkout') THEN 'cart_or_checkout'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'/blog|/article|/learn|/education') THEN 'content'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'/pages?/|/about|/contact|/faq|/policy') THEN 'brand_or_info'
      WHEN REGEXP_CONTAINS(LOWER(first_page_location), r'https?://[^/]+/?$')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/$') THEN 'home'
      ELSE 'other'
    END AS landing_page_group,
    event_count,
    page_views,
    user_engagement_events,
    scroll_events,
    product_views,
    add_to_cart_events,
    begin_checkout_events,
    purchase_events,
    purchase_transactions,
    purchase_revenue_usd,
    engaged_session
  FROM session_rollup_paths
),
dimension_inventory AS (
  SELECT
    'device_category' AS dimension_name,
    device_category AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'visitor_type' AS dimension_name,
    visitor_type AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'landing_page_group' AS dimension_name,
    landing_page_group AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'platform' AS dimension_name,
    platform AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'weekday' AS dimension_name,
    weekday_name AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'hour_of_day_utc' AS dimension_name,
    CAST(hour_of_day_utc AS STRING) AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2

  UNION ALL

  SELECT
    'source_medium' AS dimension_name,
    source_medium AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2
  QUALIFY ROW_NUMBER() OVER (PARTITION BY dimension_name ORDER BY sessions DESC, dimension_bucket) <= 20

  UNION ALL

  SELECT
    'country' AS dimension_name,
    country AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2
  QUALIFY ROW_NUMBER() OVER (PARTITION BY dimension_name ORDER BY sessions DESC, dimension_bucket) <= 20

  UNION ALL

  SELECT
    'landing_path' AS dimension_name,
    COALESCE(landing_path, 'unknown') AS dimension_bucket,
    COUNT(*) AS sessions,
    SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
    SUM(purchase_transactions) AS purchase_transactions,
    SUM(purchase_revenue_usd) AS purchase_revenue_usd
  FROM session_rollup
  GROUP BY 1, 2
  QUALIFY ROW_NUMBER() OVER (PARTITION BY dimension_name ORDER BY sessions DESC, dimension_bucket) <= 25
)
SELECT
  dimension_name,
  dimension_bucket,
  sessions,
  SAFE_DIVIDE(sessions, SUM(sessions) OVER (PARTITION BY dimension_name)) AS session_share,
  engaged_sessions,
  SAFE_DIVIDE(engaged_sessions, sessions) AS engaged_session_rate,
  purchase_transactions,
  SAFE_DIVIDE(purchase_transactions, sessions) AS purchase_session_rate,
  purchase_revenue_usd
FROM dimension_inventory
ORDER BY dimension_name, sessions DESC, dimension_bucket;
