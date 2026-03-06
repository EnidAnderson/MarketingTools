-- Google CPC landing-family performance using Nature's Diet landing taxonomy v2.
-- Replace {{project_id}} and {{dataset_id}}.
-- Window: trailing 30 complete UTC days.
-- Important: observational report only. Use for control/challenger design, not causal lift claims.

WITH raw_events AS (
  SELECT
    TIMESTAMP_MICROS(event_timestamp) AS event_ts,
    user_pseudo_id,
    event_name,
    COALESCE(NULLIF(device.category, ''), 'unknown') AS device_category,
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
    se.user_pseudo_id,
    se.session_key,
    ARRAY_AGG(se.device_category ORDER BY se.event_ts LIMIT 1)[SAFE_OFFSET(0)] AS device_category,
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
    COALESCE(
      REGEXP_EXTRACT(lp.first_page_location, r'https?://[^/]+(/[^?#]*)'),
      REGEXP_EXTRACT(lp.first_page_location, r'(^/[^?#]*)')
    ) AS landing_path,
    COUNT(*) AS event_count,
    COUNTIF(se.event_name = 'page_view') AS page_views,
    COUNTIF(se.event_name = 'purchase') AS purchase_events,
    COUNT(DISTINCT IF(se.event_name = 'purchase', se.transaction_id, NULL)) AS purchase_transactions,
    SUM(IF(se.event_name = 'purchase', se.purchase_revenue_usd, 0)) AS purchase_revenue_usd,
    MAX(se.session_engaged_flag) > 0 OR SUM(se.engagement_time_msec) > 0 AS engaged_session
  FROM session_events se
  LEFT JOIN landing_pages lp
    ON se.user_pseudo_id = lp.user_pseudo_id
   AND se.session_key = lp.session_key
  GROUP BY 1, 2, 6, 7
),
classified AS (
  SELECT
    'nd_landing_taxonomy.v2' AS taxonomy_version,
    device_category,
    source_medium,
    visitor_type,
    landing_path,
    CASE
      WHEN first_page_location IS NULL OR landing_path IS NULL THEN 'unknown'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/$') THEN 'home'
      WHEN landing_path = '/simply-raw-value-bundle-assortment'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'bundle|assortment') THEN 'bundle_offer_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/ready-raw') THEN 'ready_raw_offer_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/simply-raw') THEN 'simply_raw_offer_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/product-page/') THEN 'product_detail_lp'
      WHEN landing_path IN ('/our-products', '/dog-treats', '/bone-broth')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(collections?|shop)(/|$)') THEN 'category_or_catalog_lp'
      WHEN landing_path = '/freebook-rawfeedingguide'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'freebook|guide|ebook|lead') THEN 'lead_magnet_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/post/')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(blog|article|learn|education)(/|$)') THEN 'content_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/account/') THEN 'account_portal_lp'
      WHEN landing_path = '/our-story'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(about|our-story|mission)(/|$)') THEN 'brand_story_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(contact|faq|policy|support)(/|$)') THEN 'support_or_policy_lp'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(cart|checkout)(/|$)') THEN 'cart_or_checkout_lp'
      ELSE 'other_marketing_lp'
    END AS landing_family,
    CASE
      WHEN first_page_location IS NULL OR landing_path IS NULL THEN 'unknown'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/$') THEN 'home'
      WHEN landing_path = '/simply-raw-value-bundle-assortment'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'bundle|assortment')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/ready-raw')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/simply-raw') THEN 'offer_landing'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/product-page/') THEN 'product_detail'
      WHEN landing_path IN ('/our-products', '/dog-treats', '/bone-broth')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(collections?|shop)(/|$)') THEN 'category_or_catalog'
      WHEN landing_path = '/freebook-rawfeedingguide'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'freebook|guide|ebook|lead') THEN 'lead_magnet'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/post/')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(blog|article|learn|education)(/|$)') THEN 'content'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/account/')
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(contact|faq|policy|support)(/|$)') THEN 'account_or_support'
      WHEN landing_path = '/our-story'
        OR REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(about|our-story|mission)(/|$)') THEN 'brand_or_info'
      WHEN REGEXP_CONTAINS(COALESCE(landing_path, ''), r'^/(cart|checkout)(/|$)') THEN 'cart_or_checkout'
      ELSE 'other_marketing'
    END AS landing_page_group,
    engaged_session,
    purchase_transactions,
    purchase_revenue_usd
  FROM session_rollup
)
SELECT
  taxonomy_version,
  landing_family,
  landing_page_group,
  landing_path,
  device_category,
  visitor_type,
  COUNT(*) AS sessions,
  SUM(CAST(engaged_session AS INT64)) AS engaged_sessions,
  SAFE_DIVIDE(SUM(CAST(engaged_session AS INT64)), COUNT(*)) AS engaged_session_rate,
  SUM(purchase_transactions) AS purchase_transactions,
  SAFE_DIVIDE(SUM(purchase_transactions), COUNT(*)) AS purchase_session_rate,
  SUM(purchase_revenue_usd) AS purchase_revenue_usd,
  SAFE_DIVIDE(SUM(purchase_revenue_usd), COUNT(*)) AS revenue_per_session,
  SAFE_DIVIDE(SUM(purchase_revenue_usd), NULLIF(SUM(purchase_transactions), 0)) AS average_order_value,
  landing_family = 'simply_raw_offer_lp' AND landing_path = '/simply-raw-freeze-dried-raw-meals' AS control_candidate
FROM classified
WHERE source_medium = 'google / cpc'
GROUP BY 1,2,3,4,5,6
ORDER BY sessions DESC, purchase_revenue_usd DESC, landing_path;
