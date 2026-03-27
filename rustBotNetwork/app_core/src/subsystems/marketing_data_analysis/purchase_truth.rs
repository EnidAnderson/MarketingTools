// provenance: decision_id=DEC-0015; change_request_id=CR-QA_FIXER-0032
use super::contracts::{PurchaseTruthAuditReportV1, PurchaseTruthSliceV1};
use super::ingest::Ga4EventRawV1;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ga4CanonicalPurchaseTruthStatsV1 {
    pub total_rows: usize,
    pub unique_truth_rows: usize,
    pub duplicate_rows: usize,
    pub rows_with_transaction_id: usize,
    pub rows_with_revenue: usize,
    pub rows_using_fallback_key: usize,
    pub rows_missing_truth_key: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ga4CustomPurchaseMatchStatsV1 {
    pub total_rows: usize,
    pub rows_with_canonical_purchase: usize,
    pub orphan_rows: usize,
    pub overlap_ratio: f64,
    pub orphan_ratio: f64,
}

#[derive(Debug, Clone, Default)]
struct SliceAccumulator {
    canonical_unique_purchases: u64,
    canonical_revenue_usd: f64,
    custom_purchase_rows: u64,
    custom_purchase_overlap_rows: u64,
    custom_purchase_orphan_rows: u64,
}

pub fn is_ga4_canonical_purchase_event_v1(event_name: &str) -> bool {
    event_name.trim().eq_ignore_ascii_case("purchase")
}

pub fn is_ga4_custom_purchase_event_v1(event_name: &str) -> bool {
    event_name.trim().eq_ignore_ascii_case("purchase_ndp")
}

pub fn ga4_purchase_revenue_v1(event: &Ga4EventRawV1) -> Option<f64> {
    event
        .purchase_revenue
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

pub fn ga4_transaction_id_v1(event: &Ga4EventRawV1) -> Option<String> {
    event
        .transaction_id
        .as_deref()
        .or_else(|| event.dimensions.get("transaction_id").map(String::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn ga4_event_timestamp_micros_v1(event: &Ga4EventRawV1) -> Option<i64> {
    event.event_timestamp_micros.or_else(|| {
        event
            .dimensions
            .get("event_timestamp_micros")
            .and_then(|value| value.trim().parse::<i64>().ok())
    })
}

pub fn ga4_event_epoch_seconds_v1(event: &Ga4EventRawV1) -> Option<i64> {
    ga4_event_timestamp_micros_v1(event)
        .map(|micros| micros.div_euclid(1_000_000))
        .or_else(|| {
            DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim())
                .ok()
                .map(|value| value.timestamp())
        })
}

pub fn ga4_event_date_utc_v1(event: &Ga4EventRawV1) -> Option<NaiveDate> {
    ga4_event_epoch_seconds_v1(event)
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        .map(|timestamp| timestamp.date_naive())
        .or_else(|| {
            DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim())
                .ok()
                .map(|value| value.with_timezone(&Utc).date_naive())
        })
}

pub fn ga4_session_key_v1(event: &Ga4EventRawV1) -> String {
    event
        .dimensions
        .get("ga_session_id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            event
                .session_id
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

pub fn ga4_canonical_purchase_fallback_truth_key_v1(event: &Ga4EventRawV1) -> Option<String> {
    let timestamp_micros = ga4_event_timestamp_micros_v1(event)?;
    let revenue = round4(ga4_purchase_revenue_v1(event)?);
    let user_pseudo_id = event.user_pseudo_id.trim();
    if user_pseudo_id.is_empty() {
        return None;
    }
    let session_key = ga4_session_key_v1(event);
    Some(format!(
        "fallback:{user_pseudo_id}:{session_key}:{timestamp_micros}:{revenue:.4}"
    ))
}

pub fn ga4_canonical_purchase_truth_key_v1(event: &Ga4EventRawV1) -> Option<String> {
    ga4_transaction_id_v1(event)
        .map(|value| format!("tx:{value}"))
        .or_else(|| ga4_canonical_purchase_fallback_truth_key_v1(event))
}

pub fn ga4_canonical_purchase_truth_stats_v1(
    ga4_events: &[Ga4EventRawV1],
) -> Ga4CanonicalPurchaseTruthStatsV1 {
    let mut stats = Ga4CanonicalPurchaseTruthStatsV1::default();
    let mut seen_truth_keys = BTreeSet::new();
    for event in ga4_events {
        if !is_ga4_canonical_purchase_event_v1(&event.event_name) {
            continue;
        }
        stats.total_rows += 1;
        if ga4_transaction_id_v1(event).is_some() {
            stats.rows_with_transaction_id += 1;
        }
        if ga4_purchase_revenue_v1(event).is_some() {
            stats.rows_with_revenue += 1;
        }
        if ga4_transaction_id_v1(event).is_none()
            && ga4_canonical_purchase_fallback_truth_key_v1(event).is_some()
        {
            stats.rows_using_fallback_key += 1;
        }

        if let Some(truth_key) = ga4_canonical_purchase_truth_key_v1(event) {
            if seen_truth_keys.insert(truth_key) {
                stats.unique_truth_rows += 1;
            } else {
                stats.duplicate_rows += 1;
            }
        } else {
            stats.rows_missing_truth_key += 1;
        }
    }
    stats
}

pub fn ga4_custom_purchase_match_stats_v1(
    ga4_events: &[Ga4EventRawV1],
    tolerance_seconds: i64,
) -> Ga4CustomPurchaseMatchStatsV1 {
    let canonical_purchase_seconds =
        canonical_purchase_seconds_by_user_session(ga4_events, tolerance_seconds);
    let mut stats = Ga4CustomPurchaseMatchStatsV1::default();
    for event in ga4_events {
        if !is_ga4_custom_purchase_event_v1(&event.event_name) {
            continue;
        }
        stats.total_rows += 1;
        let Some(event_second) = ga4_event_epoch_seconds_v1(event) else {
            stats.orphan_rows += 1;
            continue;
        };
        if has_canonical_purchase_within_window_v1(
            &canonical_purchase_seconds,
            event.user_pseudo_id.trim(),
            &ga4_session_key_v1(event),
            event_second,
            tolerance_seconds,
        ) {
            stats.rows_with_canonical_purchase += 1;
        } else {
            stats.orphan_rows += 1;
        }
    }
    if stats.total_rows > 0 {
        stats.overlap_ratio = stats.rows_with_canonical_purchase as f64 / stats.total_rows as f64;
        stats.orphan_ratio = stats.orphan_rows as f64 / stats.total_rows as f64;
    }
    stats
}

pub fn build_purchase_truth_audit_v1(
    ga4_events: &[Ga4EventRawV1],
    tolerance_seconds: i64,
) -> PurchaseTruthAuditReportV1 {
    let canonical_stats = ga4_canonical_purchase_truth_stats_v1(ga4_events);
    let custom_stats = ga4_custom_purchase_match_stats_v1(ga4_events, tolerance_seconds);
    let canonical_purchase_seconds =
        canonical_purchase_seconds_by_user_session(ga4_events, tolerance_seconds);

    let mut by_day: BTreeMap<String, SliceAccumulator> = BTreeMap::new();
    let mut by_device: BTreeMap<String, SliceAccumulator> = BTreeMap::new();
    let mut by_source: BTreeMap<String, SliceAccumulator> = BTreeMap::new();
    let mut seen_canonical_truth_keys = BTreeSet::new();

    for event in ga4_events {
        if is_ga4_canonical_purchase_event_v1(&event.event_name) {
            let Some(truth_key) = ga4_canonical_purchase_truth_key_v1(event) else {
                continue;
            };
            if !seen_canonical_truth_keys.insert(truth_key) {
                continue;
            }
            let revenue = ga4_purchase_revenue_v1(event).unwrap_or(0.0).max(0.0);
            increment_slice(&mut by_day, &slice_day(event), |slice| {
                slice.canonical_unique_purchases += 1;
                slice.canonical_revenue_usd += revenue;
            });
            increment_slice(&mut by_device, &slice_device(event), |slice| {
                slice.canonical_unique_purchases += 1;
                slice.canonical_revenue_usd += revenue;
            });
            increment_slice(&mut by_source, &slice_source_medium(event), |slice| {
                slice.canonical_unique_purchases += 1;
                slice.canonical_revenue_usd += revenue;
            });
        } else if is_ga4_custom_purchase_event_v1(&event.event_name) {
            let matched = ga4_event_epoch_seconds_v1(event)
                .map(|event_second| {
                    has_canonical_purchase_within_window_v1(
                        &canonical_purchase_seconds,
                        event.user_pseudo_id.trim(),
                        &ga4_session_key_v1(event),
                        event_second,
                        tolerance_seconds,
                    )
                })
                .unwrap_or(false);
            let updater = |slice: &mut SliceAccumulator| {
                slice.custom_purchase_rows += 1;
                if matched {
                    slice.custom_purchase_overlap_rows += 1;
                } else {
                    slice.custom_purchase_orphan_rows += 1;
                }
            };
            increment_slice(&mut by_day, &slice_day(event), updater);
            increment_slice(&mut by_device, &slice_device(event), updater);
            increment_slice(&mut by_source, &slice_source_medium(event), updater);
        }
    }

    let slices_by_day = finalize_slices("day", by_day, SliceOrder::DayAsc);
    let slices_by_device_category =
        finalize_slices("device_category", by_device, SliceOrder::RiskDesc);
    let slices_by_source_medium = finalize_slices("source_medium", by_source, SliceOrder::RiskDesc);
    let dominant_orphan_device_category = slices_by_device_category
        .iter()
        .max_by_key(|slice| slice.custom_purchase_orphan_rows)
        .filter(|slice| slice.custom_purchase_orphan_rows > 0)
        .map(|slice| slice.slice_value.clone());
    let dominant_orphan_source_medium = slices_by_source_medium
        .iter()
        .max_by_key(|slice| slice.custom_purchase_orphan_rows)
        .filter(|slice| slice.custom_purchase_orphan_rows > 0)
        .map(|slice| slice.slice_value.clone());
    let summary = format!(
        "canonical_unique_purchases={}, custom_purchase_rows={}, custom_orphan_rows={}, dominant_orphan_device={}, dominant_orphan_source={}",
        canonical_stats.unique_truth_rows,
        custom_stats.total_rows,
        custom_stats.orphan_rows,
        option_label(
            &slices_label_or_unknown(
                &slices_by_device_category,
                |slice| slice.custom_purchase_orphan_rows
            )
        ),
        option_label(
            &slices_label_or_unknown(
                &slices_by_source_medium,
                |slice| slice.custom_purchase_orphan_rows
            )
        )
    );

    PurchaseTruthAuditReportV1 {
        canonical_purchase_rows: canonical_stats.total_rows as u64,
        canonical_unique_purchases: canonical_stats.unique_truth_rows as u64,
        canonical_rows_with_transaction_id: canonical_stats.rows_with_transaction_id as u64,
        canonical_rows_using_fallback_key: canonical_stats.rows_using_fallback_key as u64,
        canonical_rows_missing_truth_key: canonical_stats.rows_missing_truth_key as u64,
        canonical_rows_with_revenue: canonical_stats.rows_with_revenue as u64,
        custom_purchase_rows: custom_stats.total_rows as u64,
        custom_purchase_overlap_rows: custom_stats.rows_with_canonical_purchase as u64,
        custom_purchase_orphan_rows: custom_stats.orphan_rows as u64,
        dominant_orphan_device_category,
        dominant_orphan_source_medium,
        slices_by_day,
        slices_by_device_category,
        slices_by_source_medium,
        summary,
    }
}

pub fn has_canonical_purchase_within_window_v1(
    canonical_purchase_seconds: &BTreeMap<(String, String), Vec<i64>>,
    user_pseudo_id: &str,
    session_key: &str,
    event_second: i64,
    tolerance_seconds: i64,
) -> bool {
    canonical_purchase_seconds
        .get(&(user_pseudo_id.to_string(), session_key.to_string()))
        .map(|seconds| {
            seconds
                .iter()
                .any(|candidate| (candidate - event_second).abs() <= tolerance_seconds)
        })
        .unwrap_or(false)
}

fn canonical_purchase_seconds_by_user_session(
    ga4_events: &[Ga4EventRawV1],
    _tolerance_seconds: i64,
) -> BTreeMap<(String, String), Vec<i64>> {
    let mut canonical_purchase_seconds: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    let mut seen_truth_keys = BTreeSet::new();
    for event in ga4_events {
        if !is_ga4_canonical_purchase_event_v1(&event.event_name) {
            continue;
        }
        let Some(truth_key) = ga4_canonical_purchase_truth_key_v1(event) else {
            continue;
        };
        if !seen_truth_keys.insert(truth_key) {
            continue;
        }
        let Some(event_second) = ga4_event_epoch_seconds_v1(event) else {
            continue;
        };
        let user = event.user_pseudo_id.trim().to_string();
        let session = ga4_session_key_v1(event);
        canonical_purchase_seconds
            .entry((user, session))
            .or_default()
            .push(event_second);
    }
    for seconds in canonical_purchase_seconds.values_mut() {
        seconds.sort_unstable();
    }
    canonical_purchase_seconds
}

fn increment_slice<F>(map: &mut BTreeMap<String, SliceAccumulator>, key: &str, updater: F)
where
    F: Fn(&mut SliceAccumulator),
{
    let entry = map.entry(key.to_string()).or_default();
    updater(entry);
}

#[derive(Clone, Copy)]
enum SliceOrder {
    DayAsc,
    RiskDesc,
}

fn finalize_slices(
    dimension: &str,
    accumulators: BTreeMap<String, SliceAccumulator>,
    order: SliceOrder,
) -> Vec<PurchaseTruthSliceV1> {
    let mut rows = accumulators
        .into_iter()
        .map(|(slice_value, accumulator)| PurchaseTruthSliceV1 {
            slice_dimension: dimension.to_string(),
            slice_value,
            canonical_unique_purchases: accumulator.canonical_unique_purchases,
            canonical_revenue_usd: round4(accumulator.canonical_revenue_usd.max(0.0)),
            custom_purchase_rows: accumulator.custom_purchase_rows,
            custom_purchase_overlap_rows: accumulator.custom_purchase_overlap_rows,
            custom_purchase_orphan_rows: accumulator.custom_purchase_orphan_rows,
        })
        .collect::<Vec<_>>();
    match order {
        SliceOrder::DayAsc => rows.sort_by(|left, right| left.slice_value.cmp(&right.slice_value)),
        SliceOrder::RiskDesc => rows.sort_by(|left, right| {
            right
                .custom_purchase_orphan_rows
                .cmp(&left.custom_purchase_orphan_rows)
                .then_with(|| right.custom_purchase_rows.cmp(&left.custom_purchase_rows))
                .then_with(|| left.slice_value.cmp(&right.slice_value))
        }),
    }
    rows
}

fn slice_day(event: &Ga4EventRawV1) -> String {
    ga4_event_date_utc_v1(event)
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown_day".to_string())
}

fn slice_device(event: &Ga4EventRawV1) -> String {
    event
        .device_category
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown_device".to_string())
}

fn slice_source_medium(event: &Ga4EventRawV1) -> String {
    event
        .source_medium
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            let source = event
                .traffic_source_source
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            let medium = event
                .traffic_source_medium
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            match (source, medium) {
                (Some(source), Some(medium)) => Some(format!("{source} / {medium}")),
                (Some(source), None) => Some(source.to_string()),
                (None, Some(medium)) => Some(medium.to_string()),
                (None, None) => None,
            }
        })
        .unwrap_or_else(|| "unknown_source_medium".to_string())
}

fn slices_label_or_unknown<F>(slices: &[PurchaseTruthSliceV1], metric: F) -> Option<String>
where
    F: Fn(&PurchaseTruthSliceV1) -> u64,
{
    slices
        .iter()
        .max_by_key(|slice| metric(slice))
        .filter(|slice| metric(slice) > 0)
        .map(|slice| slice.slice_value.clone())
}

fn option_label(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "none".to_string())
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_event(name: &str, timestamp_micros: i64, session: &str) -> Ga4EventRawV1 {
        let mut dimensions = BTreeMap::new();
        dimensions.insert(
            "event_timestamp_micros".to_string(),
            timestamp_micros.to_string(),
        );
        dimensions.insert("ga_session_id".to_string(), session.to_string());
        Ga4EventRawV1 {
            event_name: name.to_string(),
            event_timestamp_utc: "2026-03-07T12:00:00Z".to_string(),
            event_timestamp_micros: Some(timestamp_micros),
            user_pseudo_id: "user-1".to_string(),
            session_id: Some(format!("ga_session:{session}")),
            ga_session_id: session.parse::<i64>().ok(),
            device_category: Some("mobile".to_string()),
            source_medium: Some("google / organic".to_string()),
            dimensions,
            metrics: BTreeMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn purchase_truth_audit_groups_orphans_by_device_and_source() {
        let mut canonical = base_event("purchase", 1_700_000_000_000_000, "101");
        canonical.transaction_id = Some("tx-1".to_string());
        canonical.purchase_revenue = Some(64.25);
        canonical
            .dimensions
            .insert("transaction_id".to_string(), "tx-1".to_string());
        canonical
            .dimensions
            .insert("purchase_revenue".to_string(), "64.25".to_string());

        let mut overlap_custom = base_event("purchase_ndp", 1_700_000_000_010_000, "101");
        overlap_custom.device_category = Some("mobile".to_string());
        overlap_custom.source_medium = Some("google / organic".to_string());

        let mut orphan_custom = base_event("purchase_ndp", 1_700_000_900_000_000, "202");
        orphan_custom.device_category = Some("desktop".to_string());
        orphan_custom.source_medium = Some("direct / (none)".to_string());
        orphan_custom
            .dimensions
            .insert("ga_session_id".to_string(), "202".to_string());
        orphan_custom.ga_session_id = Some(202);
        orphan_custom.session_id = Some("ga_session:202".to_string());

        let report = build_purchase_truth_audit_v1(&[canonical, overlap_custom, orphan_custom], 30);

        assert_eq!(report.canonical_unique_purchases, 1);
        assert_eq!(report.custom_purchase_rows, 2);
        assert_eq!(report.custom_purchase_overlap_rows, 1);
        assert_eq!(report.custom_purchase_orphan_rows, 1);
        assert_eq!(
            report.dominant_orphan_device_category.as_deref(),
            Some("desktop")
        );
        assert_eq!(
            report.dominant_orphan_source_medium.as_deref(),
            Some("direct / (none)")
        );
        assert!(report
            .slices_by_device_category
            .iter()
            .any(|slice| slice.slice_value == "desktop" && slice.custom_purchase_orphan_rows == 1));
        assert!(report
            .slices_by_source_medium
            .iter()
            .any(|slice| slice.slice_value == "direct / (none)"
                && slice.custom_purchase_orphan_rows == 1));
    }
}
