use super::contracts::{
    AssignmentConfidenceV1, ExperimentAnalyticsSummaryV1, ExperimentAssignmentCoverageReportV1,
    ExperimentAssignmentSourceV1, ExperimentAssignmentStatusV1, ExperimentFunnelRowV1,
    ExperimentGuardrailSliceV1, FunnelStageV1, FunnelSummaryV1, Ga4SessionRollupV1,
    LandingContextV1, SessionExperimentContextV1, StorefrontBehaviorRowV1,
    StorefrontBehaviorSummaryV1, VisitorTypeV1,
};
use super::ingest::Ga4EventRawV1;
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::{BTreeMap, BTreeSet};
use url::{form_urlencoded, Url};

const LANDING_TAXONOMY_VERSION_V2: &str = "nd_landing_taxonomy.v2";

#[derive(Debug, Default)]
struct SessionAccumulator {
    user_pseudo_id: String,
    ga_session_id: Option<i64>,
    session_start_micros: i64,
    first_event_micros: i64,
    landing_path: Option<String>,
    landing_host: Option<String>,
    landing_path_micros: Option<i64>,
    experiment_context: SessionExperimentContextV1,
    visitor_type: VisitorTypeV1,
    engaged_session: bool,
    engagement_time_msec: u64,
    country: Option<String>,
    platform: Option<String>,
    device_category: Option<String>,
    source: Option<String>,
    medium: Option<String>,
    source_medium: Option<String>,
    campaign: Option<String>,
    page_view_count: u32,
    user_engagement_count: u32,
    scroll_count: u32,
    view_item_count: u32,
    add_to_cart_count: u32,
    begin_checkout_count: u32,
    purchase_count: u32,
    revenue_usd: f64,
    transaction_ids: BTreeSet<String>,
    purchase_fallback_keys: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct StorefrontAccumulator {
    landing_path: String,
    landing_family: String,
    sessions: u64,
    engaged_sessions: u64,
    product_view_sessions: u64,
    add_to_cart_sessions: u64,
    checkout_sessions: u64,
    purchase_sessions: u64,
    revenue_usd: f64,
    transaction_count: u64,
}

#[derive(Debug, Default)]
struct ExperimentFunnelAccumulator {
    experiment_id: String,
    experiment_name: Option<String>,
    variant_id: String,
    variant_name: Option<String>,
    sessions: u64,
    engaged_sessions: u64,
    product_view_sessions: u64,
    add_to_cart_sessions: u64,
    checkout_sessions: u64,
    purchase_sessions: u64,
    revenue_usd: f64,
}

#[derive(Debug, Default)]
struct ExperimentGuardrailAccumulator {
    dimension_key: String,
    dimension_value: String,
    total_sessions: u64,
    assigned_sessions: u64,
    partial_sessions: u64,
    ambiguous_sessions: u64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Extract a normalized path from a GA4 `page_location`.
pub fn extract_path_from_page_location(page_location: &str) -> Option<String> {
    let trimmed = page_location.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = if trimmed.starts_with('/') {
        trimmed
            .split(['?', '#'])
            .next()
            .map(|value| value.trim().to_string())
    } else {
        Url::parse(trimmed)
            .ok()
            .map(|url| url.path().trim().to_string())
    }?;
    normalize_path(&path)
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Classify a landing path into Nature's Diet taxonomy v2.
pub fn classify_landing_context_v2(landing_path: &str) -> LandingContextV1 {
    let normalized = normalize_path(landing_path).unwrap_or_else(|| "/".to_string());
    let lower = normalized.to_ascii_lowercase();

    let (matched_rule_id, landing_family, landing_page_group) = if lower == "/" {
        ("home.root", "home", "home")
    } else if lower == "/simply-raw-value-bundle-assortment"
        || lower.contains("bundle")
        || lower.contains("assortment")
    {
        ("offer.bundle", "bundle_offer_lp", "offer_landing")
    } else if lower.starts_with("/ready-raw") {
        ("offer.ready_raw", "ready_raw_offer_lp", "offer_landing")
    } else if lower.starts_with("/simply-raw") {
        ("offer.simply_raw", "simply_raw_offer_lp", "offer_landing")
    } else if lower.starts_with("/product-page/") {
        ("product.detail", "product_detail_lp", "product_detail")
    } else if matches!(
        lower.as_str(),
        "/our-products" | "/dog-treats" | "/bone-broth"
    ) || lower.starts_with("/collection")
        || lower.starts_with("/collections")
        || lower.starts_with("/shop")
    {
        (
            "catalog.category",
            "category_or_catalog_lp",
            "category_or_catalog",
        )
    } else if lower == "/freebook-rawfeedingguide"
        || lower.contains("freebook")
        || lower.contains("guide")
        || lower.contains("ebook")
        || lower.contains("lead")
    {
        ("lead.freebook", "lead_magnet_lp", "lead_magnet")
    } else if lower.starts_with("/post/")
        || lower.starts_with("/blog")
        || lower.starts_with("/article")
        || lower.starts_with("/learn")
        || lower.starts_with("/education")
    {
        ("content.post", "content_lp", "content")
    } else if lower.starts_with("/account/") {
        ("account.portal", "account_portal_lp", "account_or_support")
    } else if lower == "/our-story" || lower.starts_with("/about") || lower.starts_with("/mission")
    {
        ("brand.story", "brand_story_lp", "brand_or_info")
    } else if lower.starts_with("/contact")
        || lower.starts_with("/faq")
        || lower.starts_with("/policy")
        || lower.starts_with("/support")
    {
        (
            "support.policy",
            "support_or_policy_lp",
            "account_or_support",
        )
    } else if lower.starts_with("/cart") || lower.starts_with("/checkout") {
        ("cart.checkout", "cart_or_checkout_lp", "cart_or_checkout")
    } else {
        ("fallback.other", "other_marketing_lp", "other_marketing")
    };

    LandingContextV1 {
        taxonomy_version: LANDING_TAXONOMY_VERSION_V2.to_string(),
        matched_rule_id: matched_rule_id.to_string(),
        landing_path: normalized,
        landing_family: landing_family.to_string(),
        landing_page_group: landing_page_group.to_string(),
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Roll up GA4 events into session semantics for runtime analytics.
pub fn rollup_ga4_sessions_v1(events: &[Ga4EventRawV1]) -> Vec<Ga4SessionRollupV1> {
    let mut ordered_events = events
        .iter()
        .filter(|event| supports_session_rollup(event))
        .collect::<Vec<_>>();
    ordered_events.sort_by_key(|event| event_timestamp_micros(event).unwrap_or(i64::MAX));

    let mut accumulators: BTreeMap<String, SessionAccumulator> = BTreeMap::new();
    for event in ordered_events {
        let Some(timestamp_micros) = event_timestamp_micros(event) else {
            continue;
        };
        let user_pseudo_id = event.user_pseudo_id.trim();
        if user_pseudo_id.is_empty() {
            continue;
        }
        let ga_session_id = ga_session_id(event);
        let session_key = build_session_key(user_pseudo_id, ga_session_id, timestamp_micros);
        let accumulator = accumulators
            .entry(session_key)
            .or_insert_with(|| SessionAccumulator {
                user_pseudo_id: user_pseudo_id.to_string(),
                ga_session_id,
                session_start_micros: timestamp_micros,
                first_event_micros: timestamp_micros,
                ..Default::default()
            });
        accumulator.session_start_micros = accumulator.session_start_micros.min(timestamp_micros);
        accumulator.first_event_micros = accumulator.first_event_micros.min(timestamp_micros);

        if accumulator.ga_session_id.is_none() {
            accumulator.ga_session_id = ga_session_id;
        }
        choose_if_absent(
            &mut accumulator.country,
            clean_option(event.country.as_deref()),
        );
        choose_if_absent(
            &mut accumulator.platform,
            clean_option(event.platform.as_deref()),
        );
        choose_if_absent(
            &mut accumulator.device_category,
            clean_option(event.device_category.as_deref()),
        );
        let (source, medium, source_medium) = source_medium_fields(event);
        choose_if_absent(&mut accumulator.source, source);
        choose_if_absent(&mut accumulator.medium, medium);
        choose_if_absent(&mut accumulator.source_medium, source_medium);
        choose_if_absent(
            &mut accumulator.campaign,
            clean_option(event.campaign.as_deref()),
        );
        if let Some(candidate) = experiment_assignment_candidate(event, timestamp_micros) {
            merge_experiment_context(&mut accumulator.experiment_context, candidate);
        }

        if matches!(accumulator.visitor_type, VisitorTypeV1::Unknown) {
            accumulator.visitor_type = derive_visitor_type(event.ga_session_number);
        }

        if session_engaged(event) {
            accumulator.engaged_session = true;
        }
        let engagement_time = event.engagement_time_msec.unwrap_or(0);
        if engagement_time > 0 {
            accumulator.engagement_time_msec = accumulator
                .engagement_time_msec
                .saturating_add(engagement_time as u64);
        }

        let event_name = event.event_name.trim().to_ascii_lowercase();
        match event_name.as_str() {
            "page_view" => {
                accumulator.page_view_count = accumulator.page_view_count.saturating_add(1)
            }
            "user_engagement" => {
                accumulator.user_engagement_count =
                    accumulator.user_engagement_count.saturating_add(1);
                accumulator.engaged_session = true;
            }
            "scroll" => accumulator.scroll_count = accumulator.scroll_count.saturating_add(1),
            "view_item" => {
                accumulator.view_item_count = accumulator.view_item_count.saturating_add(1)
            }
            "add_to_cart" => {
                accumulator.add_to_cart_count = accumulator.add_to_cart_count.saturating_add(1)
            }
            "begin_checkout" => {
                accumulator.begin_checkout_count =
                    accumulator.begin_checkout_count.saturating_add(1)
            }
            "purchase" => {
                accumulator.purchase_count = accumulator.purchase_count.saturating_add(1);
                register_purchase(event, timestamp_micros, accumulator);
            }
            _ => {}
        }

        if let Some(page_location) = page_location(event) {
            let landing_path = extract_path_from_page_location(&page_location);
            let landing_host = extract_host_from_page_location(&page_location);
            let should_update_landing = landing_path.is_some()
                && (accumulator.landing_path.is_none()
                    || event_name == "page_view"
                        && accumulator
                            .landing_path_micros
                            .map(|current| timestamp_micros < current)
                            .unwrap_or(true));
            if should_update_landing {
                accumulator.landing_path = landing_path;
                accumulator.landing_host = landing_host;
                accumulator.landing_path_micros = Some(timestamp_micros);
            }
        }
    }

    accumulators
        .into_iter()
        .map(|(session_key, accumulator)| {
            let landing_context = accumulator
                .landing_path
                .as_ref()
                .map(|path| classify_landing_context_v2(path));
            Ga4SessionRollupV1 {
                session_key,
                user_pseudo_id: accumulator.user_pseudo_id,
                ga_session_id: accumulator.ga_session_id,
                session_start_ts_utc: micros_to_rfc3339(accumulator.session_start_micros),
                first_event_ts_utc: micros_to_rfc3339(accumulator.first_event_micros),
                landing_path: accumulator.landing_path,
                landing_host: accumulator.landing_host,
                landing_context,
                experiment_context: accumulator.experiment_context,
                visitor_type: accumulator.visitor_type,
                engaged_session: accumulator.engaged_session
                    || accumulator.engagement_time_msec > 0
                    || accumulator.user_engagement_count > 0,
                engagement_time_msec: accumulator.engagement_time_msec,
                country: accumulator.country,
                platform: accumulator.platform,
                device_category: accumulator.device_category,
                source: accumulator.source,
                medium: accumulator.medium,
                source_medium: accumulator.source_medium,
                campaign: accumulator.campaign,
                page_view_count: accumulator.page_view_count,
                user_engagement_count: accumulator.user_engagement_count,
                scroll_count: accumulator.scroll_count,
                view_item_count: accumulator.view_item_count,
                add_to_cart_count: accumulator.add_to_cart_count,
                begin_checkout_count: accumulator.begin_checkout_count,
                purchase_count: accumulator.purchase_count,
                revenue_usd: round4(accumulator.revenue_usd.max(0.0)),
                transaction_ids: accumulator.transaction_ids.into_iter().collect(),
            }
        })
        .collect()
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Build a dashboard funnel summary from observed session rollups.
pub fn build_funnel_summary_from_sessions_v1(
    session_rollups: &[Ga4SessionRollupV1],
) -> FunnelSummaryV1 {
    if session_rollups.is_empty() {
        return FunnelSummaryV1 {
            stages: Vec::new(),
            dropoff_hotspot_stage: "None".to_string(),
        };
    }

    let sessions = session_rollups.len() as f64;
    let engaged_sessions = session_rollups
        .iter()
        .filter(|session| session.engaged_session)
        .count() as f64;
    let product_view_sessions = session_rollups
        .iter()
        .filter(|session| session.view_item_count > 0)
        .count() as f64;
    let add_to_cart_sessions = session_rollups
        .iter()
        .filter(|session| session.add_to_cart_count > 0)
        .count() as f64;
    let checkout_sessions = session_rollups
        .iter()
        .filter(|session| session.begin_checkout_count > 0)
        .count() as f64;
    let purchase_sessions = session_rollups
        .iter()
        .filter(|session| session.purchase_count > 0 || session.revenue_usd > 0.0)
        .count() as f64;

    let stages = vec![
        stage("Session", sessions, None),
        stage(
            "Engaged Session",
            engaged_sessions,
            Some(engaged_sessions / sessions.max(1.0)),
        ),
        stage(
            "Product View",
            product_view_sessions,
            Some(product_view_sessions / engaged_sessions.max(1.0)),
        ),
        stage(
            "Add To Cart",
            add_to_cart_sessions,
            Some(add_to_cart_sessions / product_view_sessions.max(1.0)),
        ),
        stage(
            "Checkout",
            checkout_sessions,
            Some(checkout_sessions / add_to_cart_sessions.max(1.0)),
        ),
        stage(
            "Purchase",
            purchase_sessions,
            Some(purchase_sessions / checkout_sessions.max(1.0)),
        ),
    ];

    let mut hotspot = "None".to_string();
    let mut min_rate = 1.0;
    for item in stages.iter().skip(1) {
        if let Some(rate) = item.conversion_from_previous {
            if rate < min_rate {
                min_rate = rate;
                hotspot = item.stage.clone();
            }
        }
    }

    FunnelSummaryV1 {
        stages,
        dropoff_hotspot_stage: hotspot,
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Build landing-path storefront behavior table from observed session rollups.
pub fn build_storefront_behavior_summary_from_sessions_v1(
    session_rollups: &[Ga4SessionRollupV1],
) -> StorefrontBehaviorSummaryV1 {
    if session_rollups.is_empty() {
        return StorefrontBehaviorSummaryV1 {
            source_system: "ga4_session_rollup_unavailable".to_string(),
            identity_confidence: "not_available".to_string(),
            rows: Vec::new(),
        };
    }

    let mut grouped: BTreeMap<String, StorefrontAccumulator> = BTreeMap::new();
    for session in session_rollups {
        let Some(landing_path) = session.landing_path.as_ref() else {
            continue;
        };
        let landing_family = session
            .landing_context
            .as_ref()
            .map(|context| context.landing_family.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let entry = grouped
            .entry(landing_path.clone())
            .or_insert_with(|| StorefrontAccumulator {
                landing_path: landing_path.clone(),
                landing_family,
                ..Default::default()
            });
        entry.sessions += 1;
        if session.engaged_session {
            entry.engaged_sessions += 1;
        }
        if session.view_item_count > 0 {
            entry.product_view_sessions += 1;
        }
        if session.add_to_cart_count > 0 {
            entry.add_to_cart_sessions += 1;
        }
        if session.begin_checkout_count > 0 {
            entry.checkout_sessions += 1;
        }
        if session.purchase_count > 0 || session.revenue_usd > 0.0 {
            entry.purchase_sessions += 1;
        }
        entry.revenue_usd += session.revenue_usd.max(0.0);
        entry.transaction_count += session.transaction_ids.len() as u64;
    }

    let mut rows = grouped
        .into_values()
        .map(|row| {
            let sessions = row.sessions.max(1);
            let purchase_denominator = row.transaction_count.max(row.purchase_sessions).max(1);
            StorefrontBehaviorRowV1 {
                segment: row.landing_family.clone(),
                product_or_template: row.landing_path.clone(),
                sessions: row.sessions,
                landing_path: Some(row.landing_path),
                landing_family: Some(row.landing_family),
                engaged_rate: row.engaged_sessions as f64 / sessions as f64,
                product_view_rate: row.product_view_sessions as f64 / sessions as f64,
                add_to_cart_rate: row.add_to_cart_sessions as f64 / sessions as f64,
                checkout_rate: row.checkout_sessions as f64 / sessions as f64,
                purchase_rate: row.purchase_sessions as f64 / sessions as f64,
                revenue_per_session: round4(row.revenue_usd / sessions as f64),
                aov: round4(row.revenue_usd / purchase_denominator as f64),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .sessions
            .cmp(&left.sessions)
            .then_with(|| {
                right
                    .revenue_per_session
                    .partial_cmp(&left.revenue_per_session)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.product_or_template.cmp(&right.product_or_template))
    });
    rows.truncate(5);

    StorefrontBehaviorSummaryV1 {
        source_system: "ga4_session_rollups_observed".to_string(),
        identity_confidence: "high".to_string(),
        rows,
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ga4_sessions`
/// purpose: Summarize experiment assignment coverage and variant funnels from observed sessions.
pub fn build_experiment_analytics_summary_from_sessions_v1(
    session_rollups: &[Ga4SessionRollupV1],
) -> ExperimentAnalyticsSummaryV1 {
    let total_observed_sessions = session_rollups.len() as u64;
    if total_observed_sessions == 0 {
        return ExperimentAnalyticsSummaryV1 {
            assignment_coverage: ExperimentAssignmentCoverageReportV1 {
                denominator_scope: "all_observed_sessions".to_string(),
                summary: "No observed sessions available for experiment assignment analysis."
                    .to_string(),
                ..Default::default()
            },
            funnel_rows: Vec::new(),
            guardrail_slices: Vec::new(),
        };
    }

    let mut assigned_sessions = 0u64;
    let mut partial_sessions = 0u64;
    let mut ambiguous_sessions = 0u64;
    let mut unassigned_sessions = 0u64;
    let mut funnel_by_variant: BTreeMap<(String, String), ExperimentFunnelAccumulator> =
        BTreeMap::new();
    let mut guardrails: BTreeMap<(String, String), ExperimentGuardrailAccumulator> =
        BTreeMap::new();

    for session in session_rollups {
        match session.experiment_context.assignment_status {
            ExperimentAssignmentStatusV1::Assigned => assigned_sessions += 1,
            ExperimentAssignmentStatusV1::Partial => partial_sessions += 1,
            ExperimentAssignmentStatusV1::Ambiguous => ambiguous_sessions += 1,
            ExperimentAssignmentStatusV1::Unassigned => unassigned_sessions += 1,
        }

        register_guardrail_slice(
            &mut guardrails,
            "device_category",
            session.device_category.as_deref(),
            &session.experiment_context,
        );
        register_guardrail_slice(
            &mut guardrails,
            "platform",
            session.platform.as_deref(),
            &session.experiment_context,
        );
        register_guardrail_slice(
            &mut guardrails,
            "country",
            session.country.as_deref(),
            &session.experiment_context,
        );
        register_guardrail_slice(
            &mut guardrails,
            "source_medium",
            session.source_medium.as_deref(),
            &session.experiment_context,
        );

        if !is_assigned_experiment_context(&session.experiment_context) {
            continue;
        }
        let experiment_id = session
            .experiment_context
            .experiment_id
            .clone()
            .unwrap_or_default();
        let variant_id = session
            .experiment_context
            .variant_id
            .clone()
            .unwrap_or_default();
        let entry = funnel_by_variant
            .entry((experiment_id.clone(), variant_id.clone()))
            .or_insert_with(|| ExperimentFunnelAccumulator {
                experiment_id,
                experiment_name: session.experiment_context.experiment_name.clone(),
                variant_id,
                variant_name: session.experiment_context.variant_name.clone(),
                ..Default::default()
            });
        if entry.experiment_name.is_none() {
            entry.experiment_name = session.experiment_context.experiment_name.clone();
        }
        if entry.variant_name.is_none() {
            entry.variant_name = session.experiment_context.variant_name.clone();
        }
        entry.sessions += 1;
        if session.engaged_session {
            entry.engaged_sessions += 1;
        }
        if session.view_item_count > 0 {
            entry.product_view_sessions += 1;
        }
        if session.add_to_cart_count > 0 {
            entry.add_to_cart_sessions += 1;
        }
        if session.begin_checkout_count > 0 {
            entry.checkout_sessions += 1;
        }
        if session.purchase_count > 0 || session.revenue_usd > 0.0 {
            entry.purchase_sessions += 1;
        }
        entry.revenue_usd += session.revenue_usd.max(0.0);
    }

    let coverage_ratio = ratio_string(assigned_sessions, total_observed_sessions);
    let mut coverage_notes = Vec::new();
    if ambiguous_sessions > 0 {
        coverage_notes.push(format!(
            "ambiguous_sessions={} require explicit experiment instrumentation before variant claims",
            ambiguous_sessions
        ));
    }
    if partial_sessions > 0 {
        coverage_notes.push(format!(
            "partial_sessions={} are excluded from variant funnels because experiment_id or variant_id is missing",
            partial_sessions
        ));
    }
    if assigned_sessions == 0 {
        coverage_notes.push(
            "No fully assigned sessions observed; experiment insights remain instrument_first."
                .to_string(),
        );
    }

    let mut funnel_rows = funnel_by_variant
        .into_values()
        .map(|row| ExperimentFunnelRowV1 {
            experiment_id: row.experiment_id,
            experiment_name: row.experiment_name,
            variant_id: row.variant_id,
            variant_name: row.variant_name,
            sessions: row.sessions,
            engaged_sessions: row.engaged_sessions,
            product_view_sessions: row.product_view_sessions,
            add_to_cart_sessions: row.add_to_cart_sessions,
            checkout_sessions: row.checkout_sessions,
            purchase_sessions: row.purchase_sessions,
            revenue_usd: round4(row.revenue_usd),
            denominator_scope: "assigned_sessions_only".to_string(),
        })
        .collect::<Vec<_>>();
    funnel_rows.sort_by(|left, right| {
        right
            .sessions
            .cmp(&left.sessions)
            .then_with(|| right.revenue_usd.total_cmp(&left.revenue_usd))
            .then_with(|| left.experiment_id.cmp(&right.experiment_id))
            .then_with(|| left.variant_id.cmp(&right.variant_id))
    });

    let mut guardrail_slices = guardrails
        .into_values()
        .map(|slice| ExperimentGuardrailSliceV1 {
            dimension_key: slice.dimension_key,
            dimension_value: slice.dimension_value,
            total_sessions: slice.total_sessions,
            assigned_sessions: slice.assigned_sessions,
            partial_sessions: slice.partial_sessions,
            ambiguous_sessions: slice.ambiguous_sessions,
            coverage_ratio: ratio_string(slice.assigned_sessions, slice.total_sessions),
        })
        .collect::<Vec<_>>();
    guardrail_slices.sort_by(|left, right| {
        right
            .total_sessions
            .cmp(&left.total_sessions)
            .then_with(|| left.dimension_key.cmp(&right.dimension_key))
            .then_with(|| left.dimension_value.cmp(&right.dimension_value))
    });
    guardrail_slices.truncate(16);

    ExperimentAnalyticsSummaryV1 {
        assignment_coverage: ExperimentAssignmentCoverageReportV1 {
            total_observed_sessions,
            assigned_sessions,
            partial_sessions,
            ambiguous_sessions,
            unassigned_sessions,
            assignment_coverage_ratio: coverage_ratio,
            denominator_scope: "all_observed_sessions".to_string(),
            summary: format!(
                "assigned={}, partial={}, ambiguous={}, unassigned={} across {} observed sessions",
                assigned_sessions,
                partial_sessions,
                ambiguous_sessions,
                unassigned_sessions,
                total_observed_sessions
            ),
            notes: coverage_notes,
        },
        funnel_rows,
        guardrail_slices,
    }
}

fn stage(name: &str, value: f64, conversion_from_previous: Option<f64>) -> FunnelStageV1 {
    FunnelStageV1 {
        stage: name.to_string(),
        value,
        conversion_from_previous,
    }
}

#[derive(Debug, Clone)]
struct ExperimentAssignmentCandidate {
    experiment_id: Option<String>,
    experiment_name: Option<String>,
    variant_id: Option<String>,
    variant_name: Option<String>,
    source: ExperimentAssignmentSourceV1,
    confidence: AssignmentConfidenceV1,
    status: ExperimentAssignmentStatusV1,
    observed_at_utc: String,
    notes: Vec<String>,
}

fn experiment_assignment_candidate(
    event: &Ga4EventRawV1,
    timestamp_micros: i64,
) -> Option<ExperimentAssignmentCandidate> {
    explicit_experiment_assignment_candidate(event, timestamp_micros)
        .or_else(|| url_query_experiment_assignment_candidate(event, timestamp_micros))
}

fn explicit_experiment_assignment_candidate(
    event: &Ga4EventRawV1,
    timestamp_micros: i64,
) -> Option<ExperimentAssignmentCandidate> {
    let experiment_id = clean_option(event.experiment_id.as_deref()).or_else(|| {
        event_param_string(event, &["experiment_id", "landing_experiment_id", "exp_id"])
    });
    let experiment_name = clean_option(event.experiment_name.as_deref()).or_else(|| {
        event_param_string(
            event,
            &["experiment_name", "landing_experiment_name", "exp_name"],
        )
    });
    let variant_id = clean_option(event.variant_id.as_deref()).or_else(|| {
        event_param_string(
            event,
            &[
                "variant_id",
                "landing_variant_id",
                "experiment_variant_id",
                "variant",
            ],
        )
    });
    let variant_name = clean_option(event.variant_name.as_deref()).or_else(|| {
        event_param_string(
            event,
            &[
                "variant_name",
                "landing_variant_name",
                "experiment_variant_name",
            ],
        )
    });
    build_experiment_candidate(
        experiment_id,
        experiment_name,
        variant_id,
        variant_name,
        ExperimentAssignmentSourceV1::Ga4EventParam,
        micros_to_rfc3339(timestamp_micros),
    )
}

fn url_query_experiment_assignment_candidate(
    event: &Ga4EventRawV1,
    timestamp_micros: i64,
) -> Option<ExperimentAssignmentCandidate> {
    let page_location = page_location(event)?;
    let params = parse_query_params(&page_location);
    if params.is_empty() {
        return None;
    }
    build_experiment_candidate(
        query_param(
            &params,
            &["experiment_id", "exp_id", "landing_experiment_id"],
        ),
        query_param(
            &params,
            &["experiment_name", "exp_name", "landing_experiment_name"],
        ),
        query_param(
            &params,
            &[
                "variant_id",
                "variant",
                "landing_variant_id",
                "experiment_variant_id",
            ],
        ),
        query_param(
            &params,
            &[
                "variant_name",
                "landing_variant_name",
                "experiment_variant_name",
            ],
        ),
        ExperimentAssignmentSourceV1::UrlQuery,
        micros_to_rfc3339(timestamp_micros),
    )
}

fn build_experiment_candidate(
    experiment_id: Option<String>,
    experiment_name: Option<String>,
    variant_id: Option<String>,
    variant_name: Option<String>,
    source: ExperimentAssignmentSourceV1,
    observed_at_utc: String,
) -> Option<ExperimentAssignmentCandidate> {
    let has_any_signal = experiment_id.is_some()
        || experiment_name.is_some()
        || variant_id.is_some()
        || variant_name.is_some();
    if !has_any_signal {
        return None;
    }
    let status = if experiment_id.is_some() && variant_id.is_some() {
        ExperimentAssignmentStatusV1::Assigned
    } else {
        ExperimentAssignmentStatusV1::Partial
    };
    let confidence = match (&source, &status) {
        (ExperimentAssignmentSourceV1::Ga4EventParam, ExperimentAssignmentStatusV1::Assigned) => {
            AssignmentConfidenceV1::High
        }
        (ExperimentAssignmentSourceV1::UrlQuery, ExperimentAssignmentStatusV1::Assigned) => {
            AssignmentConfidenceV1::Medium
        }
        (ExperimentAssignmentSourceV1::Ga4EventParam, ExperimentAssignmentStatusV1::Partial) => {
            AssignmentConfidenceV1::Low
        }
        (ExperimentAssignmentSourceV1::UrlQuery, ExperimentAssignmentStatusV1::Partial) => {
            AssignmentConfidenceV1::Low
        }
        _ => AssignmentConfidenceV1::Unassigned,
    };
    Some(ExperimentAssignmentCandidate {
        experiment_id,
        experiment_name,
        variant_id,
        variant_name,
        source,
        confidence,
        status,
        observed_at_utc,
        notes: Vec::new(),
    })
}

fn merge_experiment_context(
    context: &mut SessionExperimentContextV1,
    candidate: ExperimentAssignmentCandidate,
) {
    if matches!(
        context.assignment_status,
        ExperimentAssignmentStatusV1::Ambiguous
    ) {
        return;
    }
    if matches!(
        context.assignment_status,
        ExperimentAssignmentStatusV1::Unassigned
    ) {
        *context = SessionExperimentContextV1 {
            experiment_id: candidate.experiment_id,
            experiment_name: candidate.experiment_name,
            variant_id: candidate.variant_id,
            variant_name: candidate.variant_name,
            assignment_source: Some(candidate.source),
            assignment_confidence: candidate.confidence,
            assignment_status: candidate.status,
            assignment_observed_at_utc: Some(candidate.observed_at_utc),
            assignment_notes: candidate.notes,
        };
        return;
    }

    if experiment_context_conflicts(context, &candidate) {
        let mut notes = context.assignment_notes.clone();
        notes.push(format!(
            "conflicting_assignment_detected existing_experiment_id={} existing_variant_id={} candidate_experiment_id={} candidate_variant_id={}",
            context.experiment_id.as_deref().unwrap_or("none"),
            context.variant_id.as_deref().unwrap_or("none"),
            candidate.experiment_id.as_deref().unwrap_or("none"),
            candidate.variant_id.as_deref().unwrap_or("none"),
        ));
        context.experiment_id = None;
        context.experiment_name = None;
        context.variant_id = None;
        context.variant_name = None;
        context.assignment_source = None;
        context.assignment_confidence = AssignmentConfidenceV1::Ambiguous;
        context.assignment_status = ExperimentAssignmentStatusV1::Ambiguous;
        context.assignment_notes = notes;
        return;
    }

    if context.experiment_id.is_none() {
        context.experiment_id = candidate.experiment_id;
    }
    if context.experiment_name.is_none() {
        context.experiment_name = candidate.experiment_name;
    }
    if context.variant_id.is_none() {
        context.variant_id = candidate.variant_id;
    }
    if context.variant_name.is_none() {
        context.variant_name = candidate.variant_name;
    }
    if context.assignment_source.is_none() {
        context.assignment_source = Some(candidate.source);
    }
    if matches!(
        context.assignment_status,
        ExperimentAssignmentStatusV1::Partial
    ) && context.experiment_id.is_some()
        && context.variant_id.is_some()
    {
        context.assignment_status = ExperimentAssignmentStatusV1::Assigned;
        context.assignment_confidence = match context.assignment_source {
            Some(ExperimentAssignmentSourceV1::Ga4EventParam) => AssignmentConfidenceV1::High,
            Some(ExperimentAssignmentSourceV1::UrlQuery) => AssignmentConfidenceV1::Medium,
            Some(ExperimentAssignmentSourceV1::Backend) => AssignmentConfidenceV1::High,
            Some(ExperimentAssignmentSourceV1::DataLayer) => AssignmentConfidenceV1::High,
            _ => AssignmentConfidenceV1::Low,
        };
    }
}

fn experiment_context_conflicts(
    context: &SessionExperimentContextV1,
    candidate: &ExperimentAssignmentCandidate,
) -> bool {
    has_conflicting_value(
        context.experiment_id.as_deref(),
        candidate.experiment_id.as_deref(),
    ) || has_conflicting_value(
        context.variant_id.as_deref(),
        candidate.variant_id.as_deref(),
    )
}

fn has_conflicting_value(current: Option<&str>, candidate: Option<&str>) -> bool {
    matches!((current, candidate), (Some(left), Some(right)) if left != right)
}

fn is_assigned_experiment_context(context: &SessionExperimentContextV1) -> bool {
    matches!(
        context.assignment_status,
        ExperimentAssignmentStatusV1::Assigned
    ) && context.experiment_id.is_some()
        && context.variant_id.is_some()
}

fn register_guardrail_slice(
    guardrails: &mut BTreeMap<(String, String), ExperimentGuardrailAccumulator>,
    dimension_key: &str,
    raw_value: Option<&str>,
    context: &SessionExperimentContextV1,
) {
    let dimension_value = raw_value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let entry = guardrails
        .entry((dimension_key.to_string(), dimension_value.to_string()))
        .or_insert_with(|| ExperimentGuardrailAccumulator {
            dimension_key: dimension_key.to_string(),
            dimension_value: dimension_value.to_string(),
            ..Default::default()
        });
    entry.total_sessions += 1;
    match context.assignment_status {
        ExperimentAssignmentStatusV1::Assigned => entry.assigned_sessions += 1,
        ExperimentAssignmentStatusV1::Partial => entry.partial_sessions += 1,
        ExperimentAssignmentStatusV1::Ambiguous => entry.ambiguous_sessions += 1,
        ExperimentAssignmentStatusV1::Unassigned => {}
    }
}

fn ratio_string(numerator: u64, denominator: u64) -> String {
    if denominator == 0 {
        "0.0000".to_string()
    } else {
        format!("{:.4}", numerator as f64 / denominator as f64)
    }
}

fn supports_session_rollup(event: &Ga4EventRawV1) -> bool {
    !event.user_pseudo_id.trim().is_empty()
        && event
            .dimensions
            .get("ga4_read_backend")
            .map(|value| !value.trim().eq_ignore_ascii_case("data_api_run_report"))
            .unwrap_or(true)
}

fn event_timestamp_micros(event: &Ga4EventRawV1) -> Option<i64> {
    event
        .event_timestamp_micros
        .or_else(|| {
            event
                .dimensions
                .get("event_timestamp_micros")
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .or_else(|| {
            DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim())
                .ok()
                .map(|value| value.timestamp_micros())
        })
}

fn ga_session_id(event: &Ga4EventRawV1) -> Option<i64> {
    event
        .ga_session_id
        .or_else(|| {
            event
                .dimensions
                .get("ga_session_id")
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .or_else(|| {
            event
                .session_id
                .as_ref()
                .map(|value| value.trim())
                .and_then(|value| value.strip_prefix("ga_session:").or(Some(value)))
                .filter(|value| !value.starts_with("ga4_count:"))
                .and_then(|value| value.parse::<i64>().ok())
        })
}

fn build_session_key(
    user_pseudo_id: &str,
    ga_session_id: Option<i64>,
    timestamp_micros: i64,
) -> String {
    if let Some(ga_session_id) = ga_session_id {
        format!("{user_pseudo_id}:{ga_session_id}")
    } else {
        let bucket = DateTime::<Utc>::from_timestamp_micros(timestamp_micros)
            .map(|value| value.format("%Y%m%d%H").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!("unknown-session:{user_pseudo_id}:{bucket}")
    }
}

fn derive_visitor_type(ga_session_number: Option<i64>) -> VisitorTypeV1 {
    match ga_session_number {
        Some(1) => VisitorTypeV1::New,
        Some(value) if value > 1 => VisitorTypeV1::Returning,
        _ => VisitorTypeV1::Unknown,
    }
}

fn session_engaged(event: &Ga4EventRawV1) -> bool {
    event.session_engaged.unwrap_or(false) || event.engagement_time_msec.unwrap_or(0) > 0
}

fn source_medium_fields(event: &Ga4EventRawV1) -> (Option<String>, Option<String>, Option<String>) {
    let source = clean_option(event.traffic_source_source.as_deref());
    let medium = clean_option(event.traffic_source_medium.as_deref());
    let source_medium = if source.is_some() || medium.is_some() {
        match (source.clone(), medium.clone()) {
            (Some(source), Some(medium)) => Some(format!("{source} / {medium}")),
            (Some(source), None) => Some(source),
            (None, Some(medium)) => Some(medium),
            (None, None) => None,
        }
    } else {
        clean_option(event.source_medium.as_deref()).map(|combined| combined.trim().to_string())
    };

    if source.is_some() || medium.is_some() {
        return (source, medium, source_medium);
    }

    let Some(combined) = source_medium.clone() else {
        return (None, None, None);
    };
    let mut pieces = combined
        .splitn(2, '/')
        .map(|piece| piece.trim().to_string());
    let source = pieces.next().filter(|value| !value.is_empty());
    let medium = pieces.next().filter(|value| !value.is_empty());
    (source, medium, source_medium)
}

fn page_location(event: &Ga4EventRawV1) -> Option<String> {
    clean_option(event.page_location.as_deref()).or_else(|| {
        event
            .dimensions
            .get("page_location")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn event_param_string(event: &Ga4EventRawV1, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        event
            .dimensions
            .get(*key)
            .map(|value| value.as_str())
            .and_then(|value| clean_option(Some(value)))
    })
}

fn parse_query_params(page_location: &str) -> BTreeMap<String, String> {
    let trimmed = page_location.trim();
    let query = if trimmed.starts_with('/') {
        trimmed
            .split_once('?')
            .map(|(_, query)| query.to_string())
            .unwrap_or_default()
    } else {
        Url::parse(trimmed)
            .ok()
            .and_then(|url| url.query().map(|query| query.to_string()))
            .unwrap_or_default()
    };
    if query.is_empty() {
        return BTreeMap::new();
    }
    form_urlencoded::parse(query.as_bytes())
        .filter_map(|(key, value)| {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            (!key.is_empty() && !value.is_empty()).then_some((key, value))
        })
        .collect()
}

fn query_param(params: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| params.get(&key.to_ascii_lowercase()).cloned())
        .and_then(|value| clean_option(Some(value.as_str())))
}

fn extract_host_from_page_location(page_location: &str) -> Option<String> {
    let trimmed = page_location.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return None;
    }
    Url::parse(trimmed)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_string()))
}

fn normalize_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_query = trimmed.split(['?', '#']).next()?.trim();
    if without_query.is_empty() {
        return None;
    }
    let normalized = if without_query == "/" {
        "/".to_string()
    } else {
        let mut value = without_query.trim_end_matches('/').to_string();
        if !value.starts_with('/') {
            value.insert(0, '/');
        }
        value
    };
    Some(normalized)
}

fn clean_option(value: Option<&str>) -> Option<String> {
    value
        .map(|inner| inner.trim().to_string())
        .filter(|inner| !inner.is_empty())
}

fn choose_if_absent(target: &mut Option<String>, candidate: Option<String>) {
    if target.is_none() {
        *target = candidate;
    }
}

fn purchase_revenue_usd(event: &Ga4EventRawV1) -> Option<f64> {
    event
        .purchase_revenue
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or(event.purchase_revenue_in_usd)
        .or_else(|| {
            event
                .dimensions
                .get("purchase_revenue")
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
        .or_else(|| {
            event
                .dimensions
                .get("purchase_revenue_in_usd")
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn register_purchase(
    event: &Ga4EventRawV1,
    timestamp_micros: i64,
    accumulator: &mut SessionAccumulator,
) {
    let revenue = purchase_revenue_usd(event).unwrap_or(0.0).max(0.0);
    if let Some(transaction_id) = clean_option(event.transaction_id.as_deref().or_else(|| {
        event
            .dimensions
            .get("transaction_id")
            .map(|value| value.as_str())
    })) {
        if accumulator.transaction_ids.insert(transaction_id) {
            accumulator.revenue_usd += revenue;
        }
        return;
    }

    let fallback_key = format!("{}:{:.4}", timestamp_micros, revenue);
    if accumulator.purchase_fallback_keys.insert(fallback_key) {
        accumulator.revenue_usd += revenue;
    }
}

fn micros_to_rfc3339(timestamp_micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(timestamp_micros)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn event(
        event_name: &str,
        event_timestamp_utc: &str,
        user_pseudo_id: &str,
        ga_session_id: i64,
    ) -> Ga4EventRawV1 {
        Ga4EventRawV1 {
            event_name: event_name.to_string(),
            event_timestamp_utc: event_timestamp_utc.to_string(),
            user_pseudo_id: user_pseudo_id.to_string(),
            ga_session_id: Some(ga_session_id),
            dimensions: BTreeMap::from([(
                "ga4_read_backend".to_string(),
                "bigquery_export".to_string(),
            )]),
            ..Default::default()
        }
    }

    #[test]
    fn extracts_path_from_absolute_and_relative_locations() {
        assert_eq!(
            extract_path_from_page_location(
                "https://naturesdietpet.com/simply-raw-freeze-dried-raw-meals?x=1"
            ),
            Some("/simply-raw-freeze-dried-raw-meals".to_string())
        );
        assert_eq!(
            extract_path_from_page_location("/product-page/simply-raw-all-flavors-mix?variant=1"),
            Some("/product-page/simply-raw-all-flavors-mix".to_string())
        );
    }

    #[test]
    fn classifies_landing_context_v2_rules() {
        let simply_raw = classify_landing_context_v2("/simply-raw-freeze-dried-raw-meals");
        assert_eq!(simply_raw.matched_rule_id, "offer.simply_raw");
        assert_eq!(simply_raw.landing_family, "simply_raw_offer_lp");

        let bundle = classify_landing_context_v2("/simply-raw-value-bundle-assortment");
        assert_eq!(bundle.matched_rule_id, "offer.bundle");
        assert_eq!(bundle.landing_page_group, "offer_landing");
    }

    #[test]
    fn rolls_up_sessions_with_real_landing_and_funnel_counts() {
        let mut page_view = event("page_view", "2026-03-01T12:00:00Z", "user-1", 111);
        page_view.page_location =
            Some("https://naturesdietpet.com/simply-raw-freeze-dried-raw-meals".to_string());
        page_view.session_engaged = Some(true);
        page_view.engagement_time_msec = Some(1200);
        page_view.ga_session_number = Some(1);
        page_view.device_category = Some("mobile".to_string());
        page_view.country = Some("United States".to_string());
        page_view.platform = Some("WEB".to_string());
        page_view.traffic_source_source = Some("google".to_string());
        page_view.traffic_source_medium = Some("cpc".to_string());
        page_view.campaign = Some("Spring Launch".to_string());

        let mut view_item = event("view_item", "2026-03-01T12:00:05Z", "user-1", 111);
        view_item.ga_session_number = Some(1);

        let mut add_to_cart = event("add_to_cart", "2026-03-01T12:00:10Z", "user-1", 111);
        add_to_cart.ga_session_number = Some(1);

        let mut purchase = event("purchase", "2026-03-01T12:00:30Z", "user-1", 111);
        purchase.ga_session_number = Some(1);
        purchase.transaction_id = Some("tx-1".to_string());
        purchase.purchase_revenue_in_usd = Some(64.25);

        let mut second_session = event("page_view", "2026-03-01T13:00:00Z", "user-1", 222);
        second_session.page_location =
            Some("https://naturesdietpet.com/product-page/simply-raw-all-flavors-mix".to_string());
        second_session.ga_session_number = Some(2);
        second_session.device_category = Some("desktop".to_string());

        let rollups =
            rollup_ga4_sessions_v1(&[page_view, view_item, add_to_cart, purchase, second_session]);
        assert_eq!(rollups.len(), 2);
        assert_eq!(
            rollups[0].landing_path.as_deref(),
            Some("/simply-raw-freeze-dried-raw-meals")
        );
        assert_eq!(rollups[0].visitor_type, VisitorTypeV1::New);
        assert_eq!(rollups[0].view_item_count, 1);
        assert_eq!(rollups[0].add_to_cart_count, 1);
        assert_eq!(rollups[0].purchase_count, 1);
        assert!((rollups[0].revenue_usd - 64.25).abs() < 0.0001);
        assert_eq!(rollups[1].visitor_type, VisitorTypeV1::Returning);
    }

    #[test]
    fn resolves_experiment_assignment_from_explicit_event_params_before_url_query() {
        let mut page_view = event("page_view", "2026-03-02T12:00:00Z", "user-1", 333);
        page_view.page_location = Some(
            "https://naturesdietpet.com/simply-raw-freeze-dried-raw-meals?experiment_id=url_test&variant_id=url_control".to_string(),
        );
        page_view.experiment_id = Some("lp_paid_offer_test".to_string());
        page_view.experiment_name = Some("Landing Offer Test".to_string());
        page_view.variant_id = Some("challenger_bundle".to_string());
        page_view.variant_name = Some("Bundle Challenger".to_string());

        let rollups = rollup_ga4_sessions_v1(&[page_view]);
        assert_eq!(rollups.len(), 1);
        let context = &rollups[0].experiment_context;
        assert_eq!(
            context.assignment_status,
            ExperimentAssignmentStatusV1::Assigned
        );
        assert_eq!(
            context.assignment_source,
            Some(ExperimentAssignmentSourceV1::Ga4EventParam)
        );
        assert_eq!(context.assignment_confidence, AssignmentConfidenceV1::High);
        assert_eq!(context.experiment_id.as_deref(), Some("lp_paid_offer_test"));
        assert_eq!(context.variant_id.as_deref(), Some("challenger_bundle"));
    }

    #[test]
    fn marks_conflicting_experiment_assignments_as_ambiguous() {
        let mut page_view = event("page_view", "2026-03-02T12:00:00Z", "user-2", 444);
        page_view.page_location = Some(
            "https://naturesdietpet.com/simply-raw-freeze-dried-raw-meals?experiment_id=lp_paid_offer_test&variant_id=control".to_string(),
        );

        let mut purchase = event("purchase", "2026-03-02T12:00:20Z", "user-2", 444);
        purchase.experiment_id = Some("lp_paid_offer_test".to_string());
        purchase.variant_id = Some("challenger_bundle".to_string());

        let rollups = rollup_ga4_sessions_v1(&[page_view, purchase]);
        assert_eq!(rollups.len(), 1);
        let context = &rollups[0].experiment_context;
        assert_eq!(
            context.assignment_status,
            ExperimentAssignmentStatusV1::Ambiguous
        );
        assert_eq!(
            context.assignment_confidence,
            AssignmentConfidenceV1::Ambiguous
        );
        assert!(context.experiment_id.is_none());
        assert!(context.variant_id.is_none());
        assert!(context
            .assignment_notes
            .iter()
            .any(|note| note.contains("conflicting_assignment_detected")));
    }

    #[test]
    fn builds_experiment_analytics_summary_from_sessions() {
        let summary = build_experiment_analytics_summary_from_sessions_v1(&[
            Ga4SessionRollupV1 {
                session_key: "assigned-1".to_string(),
                experiment_context: SessionExperimentContextV1 {
                    experiment_id: Some("exp-a".to_string()),
                    experiment_name: Some("Landing Test A".to_string()),
                    variant_id: Some("control".to_string()),
                    variant_name: Some("Control".to_string()),
                    assignment_source: Some(ExperimentAssignmentSourceV1::Ga4EventParam),
                    assignment_confidence: AssignmentConfidenceV1::High,
                    assignment_status: ExperimentAssignmentStatusV1::Assigned,
                    assignment_observed_at_utc: Some("2026-03-02T12:00:00Z".to_string()),
                    assignment_notes: Vec::new(),
                },
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                engaged_session: true,
                view_item_count: 1,
                add_to_cart_count: 1,
                begin_checkout_count: 1,
                purchase_count: 1,
                revenue_usd: 90.0,
                ..Default::default()
            },
            Ga4SessionRollupV1 {
                session_key: "partial-1".to_string(),
                experiment_context: SessionExperimentContextV1 {
                    experiment_id: Some("exp-a".to_string()),
                    assignment_source: Some(ExperimentAssignmentSourceV1::UrlQuery),
                    assignment_confidence: AssignmentConfidenceV1::Low,
                    assignment_status: ExperimentAssignmentStatusV1::Partial,
                    assignment_observed_at_utc: Some("2026-03-02T12:10:00Z".to_string()),
                    assignment_notes: Vec::new(),
                    ..Default::default()
                },
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                ..Default::default()
            },
            Ga4SessionRollupV1 {
                session_key: "ambiguous-1".to_string(),
                experiment_context: SessionExperimentContextV1 {
                    assignment_confidence: AssignmentConfidenceV1::Ambiguous,
                    assignment_status: ExperimentAssignmentStatusV1::Ambiguous,
                    assignment_notes: vec!["conflicting_assignment_detected".to_string()],
                    ..Default::default()
                },
                device_category: Some("desktop".to_string()),
                source_medium: Some("google / cpc".to_string()),
                ..Default::default()
            },
            Ga4SessionRollupV1 {
                session_key: "unassigned-1".to_string(),
                device_category: Some("desktop".to_string()),
                source_medium: Some("google / organic".to_string()),
                ..Default::default()
            },
        ]);
        assert_eq!(summary.assignment_coverage.total_observed_sessions, 4);
        assert_eq!(summary.assignment_coverage.assigned_sessions, 1);
        assert_eq!(summary.assignment_coverage.partial_sessions, 1);
        assert_eq!(summary.assignment_coverage.ambiguous_sessions, 1);
        assert_eq!(summary.assignment_coverage.unassigned_sessions, 1);
        assert_eq!(
            summary.assignment_coverage.assignment_coverage_ratio,
            "0.2500"
        );
        assert_eq!(summary.funnel_rows.len(), 1);
        assert_eq!(summary.funnel_rows[0].experiment_id, "exp-a");
        assert_eq!(summary.funnel_rows[0].variant_id, "control");
        assert_eq!(summary.funnel_rows[0].sessions, 1);
        assert_eq!(summary.funnel_rows[0].purchase_sessions, 1);
        assert!(!summary.guardrail_slices.is_empty());
    }

    #[test]
    fn builds_funnel_summary_from_sessions() {
        let sessions = vec![
            Ga4SessionRollupV1 {
                session_key: "a".to_string(),
                engaged_session: true,
                view_item_count: 1,
                add_to_cart_count: 1,
                begin_checkout_count: 1,
                purchase_count: 1,
                ..Default::default()
            },
            Ga4SessionRollupV1 {
                session_key: "b".to_string(),
                engaged_session: true,
                view_item_count: 1,
                ..Default::default()
            },
        ];
        let summary = build_funnel_summary_from_sessions_v1(&sessions);
        assert_eq!(summary.stages.len(), 6);
        assert_eq!(summary.stages[0].stage, "Session");
        assert_eq!(summary.stages[0].value, 2.0);
        assert_eq!(summary.stages[5].value, 1.0);
    }

    #[test]
    fn builds_storefront_summary_from_sessions() {
        let sessions = vec![
            Ga4SessionRollupV1 {
                session_key: "a".to_string(),
                landing_path: Some("/simply-raw-freeze-dried-raw-meals".to_string()),
                landing_context: Some(classify_landing_context_v2(
                    "/simply-raw-freeze-dried-raw-meals",
                )),
                engaged_session: true,
                view_item_count: 1,
                add_to_cart_count: 1,
                begin_checkout_count: 1,
                purchase_count: 1,
                revenue_usd: 100.0,
                transaction_ids: vec!["tx-1".to_string()],
                ..Default::default()
            },
            Ga4SessionRollupV1 {
                session_key: "b".to_string(),
                landing_path: Some("/simply-raw-freeze-dried-raw-meals".to_string()),
                landing_context: Some(classify_landing_context_v2(
                    "/simply-raw-freeze-dried-raw-meals",
                )),
                engaged_session: false,
                ..Default::default()
            },
        ];
        let summary = build_storefront_behavior_summary_from_sessions_v1(&sessions);
        assert_eq!(summary.source_system, "ga4_session_rollups_observed");
        assert_eq!(summary.rows.len(), 1);
        assert_eq!(
            summary.rows[0].landing_family.as_deref(),
            Some("simply_raw_offer_lp")
        );
        assert!((summary.rows[0].engaged_rate - 0.5).abs() < 0.0001);
        assert!((summary.rows[0].revenue_per_session - 50.0).abs() < 0.0001);
    }
}
