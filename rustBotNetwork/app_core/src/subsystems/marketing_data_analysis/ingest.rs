// provenance: decision_id=DEC-0016; change_request_id=CR-QA_FIXER-0033
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// # NDOC
/// component: `subsystems::marketing_data_analysis::ingest`
/// purpose: Cleaning note emitted for every non-blocking normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleaningNote {
    pub rule_id: String,
    pub severity: CleaningSeverity,
    pub affected_field: String,
    pub raw_value: String,
    pub clean_value: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CleaningSeverity {
    Warn,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cleaned<T> {
    pub value: T,
    pub notes: Vec<CleaningNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngestError {
    pub code: String,
    pub field: String,
    pub reason: String,
    pub sample: Option<String>,
}

#[derive(Debug, Error)]
#[error("{code} field={field}: {reason}")]
pub struct IngestFailure {
    pub code: String,
    pub field: String,
    pub reason: String,
    pub sample: Option<String>,
}

impl From<IngestFailure> for IngestError {
    fn from(value: IngestFailure) -> Self {
        let sample = value.sample.map(|s| {
            if s.len() > 96 {
                s.chars().take(96).collect()
            } else {
                s
            }
        });
        Self {
            code: value.code,
            field: if value.field.trim().is_empty() {
                "__root__".to_string()
            } else {
                value.field
            },
            reason: value.reason,
            sample,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserId(String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CampaignId(String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdGroupId(String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrencyCode(String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Money {
    pub currency: CurrencyCode,
    pub amount: Decimal,
}

impl Money {
    pub fn checked_add(&self, rhs: &Money) -> Result<Money, IngestError> {
        if self.currency != rhs.currency {
            return Err(IngestFailure {
                code: "mixed_currency_aggregation".to_string(),
                field: "currency".to_string(),
                reason: "cannot aggregate money values with different currencies".to_string(),
                sample: Some(format!(
                    "left={}, right={}",
                    self.currency.0.as_str(),
                    rhs.currency.0.as_str()
                )),
            }
            .into());
        }
        Ok(Money {
            currency: self.currency.clone(),
            amount: self.amount + rhs.amount,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ga4EventRawV1 {
    pub event_name: String,
    pub event_timestamp_utc: String,
    pub user_pseudo_id: String,
    pub session_id: Option<String>,
    pub campaign: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleAdsRowRawV1 {
    pub campaign_id: String,
    pub ad_group_id: String,
    pub date: String,
    pub impressions: u64,
    pub clicks: u64,
    pub cost_micros: u64,
    pub conversions_micros: u64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WixOrderRawV1 {
    pub order_id: String,
    pub placed_at_utc: String,
    pub gross_amount: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ga4EventV1 {
    pub event_name: String,
    pub event_time_utc: DateTime<Utc>,
    pub user_id: UserId,
    pub session_id: Option<String>,
    pub campaign: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleAdsRowV1 {
    pub campaign_id: CampaignId,
    pub ad_group_id: AdGroupId,
    pub date: NaiveDate,
    pub impressions: u64,
    pub clicks: u64,
    pub cost: Money,
    pub conversion_value: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WixOrderV1 {
    pub order_id: String,
    pub placed_at_utc: DateTime<Utc>,
    pub gross: Money,
}

pub fn parse_ga4_event(raw_json: &str) -> Result<Cleaned<Ga4EventV1>, IngestError> {
    let raw: Ga4EventRawV1 = serde_json::from_str(raw_json).map_err(|err| IngestError {
        code: "ga4_parse_error".to_string(),
        field: "payload".to_string(),
        reason: err.to_string(),
        sample: Some(raw_json.chars().take(96).collect()),
    })?;
    Cleaned::<Ga4EventV1>::try_from(raw)
}

pub fn parse_google_ads_row(raw_json: &str) -> Result<Cleaned<GoogleAdsRowV1>, IngestError> {
    let raw: GoogleAdsRowRawV1 = serde_json::from_str(raw_json).map_err(|err| IngestError {
        code: "ads_parse_error".to_string(),
        field: "payload".to_string(),
        reason: err.to_string(),
        sample: Some(raw_json.chars().take(96).collect()),
    })?;
    Cleaned::<GoogleAdsRowV1>::try_from(raw)
}

pub fn parse_wix_order(raw_json: &str) -> Result<Cleaned<WixOrderV1>, IngestError> {
    let raw: WixOrderRawV1 = serde_json::from_str(raw_json).map_err(|err| IngestError {
        code: "wix_parse_error".to_string(),
        field: "payload".to_string(),
        reason: err.to_string(),
        sample: Some(raw_json.chars().take(96).collect()),
    })?;
    Cleaned::<WixOrderV1>::try_from(raw)
}

impl TryFrom<Ga4EventRawV1> for Cleaned<Ga4EventV1> {
    type Error = IngestError;

    fn try_from(raw: Ga4EventRawV1) -> Result<Self, Self::Error> {
        let mut notes = Vec::new();
        let event_name = normalized_non_empty("event_name", raw.event_name, &mut notes)?;
        let user_pseudo_id =
            normalized_non_empty("user_pseudo_id", raw.user_pseudo_id, &mut notes)?;
        let event_time_utc =
            DateTime::parse_from_rfc3339(&raw.event_timestamp_utc).map_err(|_| IngestFailure {
                code: "ga4_invalid_timestamp".to_string(),
                field: "event_timestamp_utc".to_string(),
                reason: "expected RFC3339 timestamp".to_string(),
                sample: Some(raw.event_timestamp_utc.clone()),
            })?;

        Ok(Cleaned {
            value: Ga4EventV1 {
                event_name,
                event_time_utc: event_time_utc.with_timezone(&Utc),
                user_id: UserId(user_pseudo_id),
                session_id: raw.session_id.and_then(clean_optional),
                campaign: raw.campaign.and_then(clean_optional),
            },
            notes,
        })
    }
}

impl TryFrom<GoogleAdsRowRawV1> for Cleaned<GoogleAdsRowV1> {
    type Error = IngestError;

    fn try_from(raw: GoogleAdsRowRawV1) -> Result<Self, Self::Error> {
        let mut notes = Vec::new();
        let campaign_id = normalized_non_empty("campaign_id", raw.campaign_id, &mut notes)?;
        let ad_group_id = normalized_non_empty("ad_group_id", raw.ad_group_id, &mut notes)?;
        let date = NaiveDate::parse_from_str(&raw.date, "%Y-%m-%d").map_err(|_| IngestFailure {
            code: "ads_invalid_date".to_string(),
            field: "date".to_string(),
            reason: "expected YYYY-MM-DD".to_string(),
            sample: Some(raw.date.clone()),
        })?;
        if raw.clicks > raw.impressions {
            return Err(IngestFailure {
                code: "ads_clicks_gt_impressions".to_string(),
                field: "clicks".to_string(),
                reason: "clicks cannot exceed impressions".to_string(),
                sample: Some(format!(
                    "clicks={}, impressions={}",
                    raw.clicks, raw.impressions
                )),
            }
            .into());
        }

        let currency = normalized_currency(raw.currency, &mut notes)?;
        let cost = Decimal::from(raw.cost_micros) / Decimal::from(1_000_000u64);
        let conversion_value = Decimal::from(raw.conversions_micros) / Decimal::from(1_000_000u64);

        Ok(Cleaned {
            value: GoogleAdsRowV1 {
                campaign_id: CampaignId(campaign_id),
                ad_group_id: AdGroupId(ad_group_id),
                date,
                impressions: raw.impressions,
                clicks: raw.clicks,
                cost: Money {
                    currency: currency.clone(),
                    amount: cost,
                },
                conversion_value: Money {
                    currency,
                    amount: conversion_value,
                },
            },
            notes,
        })
    }
}

impl TryFrom<WixOrderRawV1> for Cleaned<WixOrderV1> {
    type Error = IngestError;

    fn try_from(raw: WixOrderRawV1) -> Result<Self, Self::Error> {
        let mut notes = Vec::new();
        let order_id = normalized_non_empty("order_id", raw.order_id, &mut notes)?;
        let placed_at_utc =
            DateTime::parse_from_rfc3339(&raw.placed_at_utc).map_err(|_| IngestFailure {
                code: "wix_invalid_timestamp".to_string(),
                field: "placed_at_utc".to_string(),
                reason: "expected RFC3339 timestamp".to_string(),
                sample: Some(raw.placed_at_utc.clone()),
            })?;
        let currency = normalized_currency(raw.currency, &mut notes)?;
        let gross_amount =
            Decimal::from_str_exact(raw.gross_amount.trim()).map_err(|_| IngestFailure {
                code: "wix_invalid_decimal".to_string(),
                field: "gross_amount".to_string(),
                reason: "expected decimal currency amount".to_string(),
                sample: Some(raw.gross_amount.clone()),
            })?;

        Ok(Cleaned {
            value: WixOrderV1 {
                order_id,
                placed_at_utc: placed_at_utc.with_timezone(&Utc),
                gross: Money {
                    currency,
                    amount: gross_amount,
                },
            },
            notes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowCompletenessCheck {
    pub expected_units: u32,
    pub observed_units: u32,
    pub completeness_ratio: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeGranularity {
    Day,
    Hour,
}

pub fn window_completeness(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    granularity: TimeGranularity,
    timezone: Tz,
    observed_units: &[DateTime<Utc>],
) -> WindowCompletenessCheck {
    if start_utc > end_utc {
        return WindowCompletenessCheck {
            expected_units: 0,
            observed_units: 0,
            completeness_ratio: 0.0,
        };
    }
    let expected_units = count_expected_units(start_utc, end_utc, granularity, timezone);
    let mut observed_bucket_keys = std::collections::BTreeSet::new();
    for ts in observed_units {
        let local = ts.with_timezone(&timezone);
        match granularity {
            TimeGranularity::Day => {
                observed_bucket_keys.insert(format!(
                    "{:04}-{:02}-{:02}",
                    local.year(),
                    local.month(),
                    local.day()
                ));
            }
            TimeGranularity::Hour => {
                observed_bucket_keys.insert(format!(
                    "{:04}-{:02}-{:02}T{:02}",
                    local.year(),
                    local.month(),
                    local.day(),
                    local.hour()
                ));
            }
        }
    }
    let observed_units = observed_bucket_keys.len() as u32;
    let ratio = if expected_units == 0 {
        1.0
    } else {
        (observed_units as f64 / expected_units as f64).clamp(0.0, 1.0)
    };
    WindowCompletenessCheck {
        expected_units,
        observed_units,
        completeness_ratio: ratio,
    }
}

pub fn join_coverage_ratio(total_rows: u64, joined_rows: u64) -> f64 {
    if total_rows == 0 {
        return 1.0;
    }
    (joined_rows as f64 / total_rows as f64).clamp(0.0, 1.0)
}

fn count_expected_units(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    granularity: TimeGranularity,
    timezone: Tz,
) -> u32 {
    let mut cursor = start_utc.with_timezone(&timezone);
    let end = end_utc.with_timezone(&timezone);
    let mut expected = std::collections::BTreeSet::new();

    while cursor <= end {
        match granularity {
            TimeGranularity::Day => {
                expected.insert(format!(
                    "{:04}-{:02}-{:02}",
                    cursor.year(),
                    cursor.month(),
                    cursor.day()
                ));
                cursor += Duration::days(1);
            }
            TimeGranularity::Hour => {
                expected.insert(format!(
                    "{:04}-{:02}-{:02}T{:02}",
                    cursor.year(),
                    cursor.month(),
                    cursor.day(),
                    cursor.hour()
                ));
                cursor += Duration::hours(1);
            }
        }
    }

    expected.len() as u32
}

fn normalized_non_empty(
    field: &str,
    raw: String,
    notes: &mut Vec<CleaningNote>,
) -> Result<String, IngestError> {
    let clean = raw.trim().to_string();
    if clean.is_empty() {
        return Err(IngestFailure {
            code: "ingest_empty_required_field".to_string(),
            field: field.to_string(),
            reason: "required field is empty after trimming".to_string(),
            sample: Some(raw),
        }
        .into());
    }
    if clean != raw {
        notes.push(CleaningNote {
            rule_id: "trim_whitespace".to_string(),
            severity: CleaningSeverity::Warn,
            affected_field: field.to_string(),
            raw_value: raw,
            clean_value: clean.clone(),
            message: "trimmed surrounding whitespace".to_string(),
        });
    }
    Ok(clean)
}

fn normalized_currency(
    raw: String,
    notes: &mut Vec<CleaningNote>,
) -> Result<CurrencyCode, IngestError> {
    let clean = raw.trim().to_ascii_uppercase();
    if clean.len() != 3 || !clean.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return Err(IngestFailure {
            code: "ingest_invalid_currency_code".to_string(),
            field: "currency".to_string(),
            reason: "currency must be a 3-letter ISO-like code".to_string(),
            sample: Some(raw),
        }
        .into());
    }
    if clean != raw {
        notes.push(CleaningNote {
            rule_id: "normalize_currency_code".to_string(),
            severity: CleaningSeverity::Warn,
            affected_field: "currency".to_string(),
            raw_value: raw,
            clean_value: clean.clone(),
            message: "normalized currency code to uppercase trimmed format".to_string(),
        });
    }
    Ok(CurrencyCode(clean))
}

fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::Denver;
    use proptest::prelude::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn ga4_parser_rejects_bad_timestamp() {
        let raw = Ga4EventRawV1 {
            event_name: "purchase".to_string(),
            event_timestamp_utc: "not-a-time".to_string(),
            user_pseudo_id: "u1".to_string(),
            session_id: None,
            campaign: None,
        };
        let parsed = Cleaned::<Ga4EventV1>::try_from(raw);
        assert!(parsed.is_err());
    }

    #[test]
    fn wix_parser_rejects_bad_decimal() {
        let raw = WixOrderRawV1 {
            order_id: "ord-1".to_string(),
            placed_at_utc: "2026-02-01T12:00:00Z".to_string(),
            gross_amount: "not-a-decimal".to_string(),
            currency: "usd".to_string(),
        };
        let parsed = Cleaned::<WixOrderV1>::try_from(raw);
        assert!(parsed.is_err());
    }

    #[test]
    fn join_coverage_stays_in_range_and_is_monotone_when_removing_unmatched() {
        let baseline = join_coverage_ratio(100, 75);
        let no_unmatched_removed = join_coverage_ratio(90, 75);
        assert!((0.0..=1.0).contains(&baseline));
        assert!((0.0..=1.0).contains(&no_unmatched_removed));
        assert!(no_unmatched_removed >= baseline);
    }

    #[test]
    fn money_checked_add_rejects_mixed_currency() {
        let usd = Money {
            currency: CurrencyCode("USD".to_string()),
            amount: Decimal::from(10),
        };
        let cad = Money {
            currency: CurrencyCode("CAD".to_string()),
            amount: Decimal::from(10),
        };
        let result = usd.checked_add(&cad);
        assert!(result.is_err());
    }

    #[test]
    fn window_completeness_handles_dst_boundary_denver_hourly() {
        let start = DateTime::parse_from_rfc3339("2026-03-08T07:00:00Z")
            .expect("valid")
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2026-03-08T10:00:00Z")
            .expect("valid")
            .with_timezone(&Utc);
        let observed = vec![start, end];
        let check = window_completeness(start, end, TimeGranularity::Hour, Denver, &observed);
        assert!(check.expected_units >= 3);
        assert!((0.0..=1.0).contains(&check.completeness_ratio));
    }

    #[test]
    fn window_completeness_empty_observed_is_zero_ratio_when_expected_nonzero() {
        let start = DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
            .expect("valid")
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2026-02-03T00:00:00Z")
            .expect("valid")
            .with_timezone(&Utc);
        let check = window_completeness(start, end, TimeGranularity::Day, Denver, &[]);
        assert!(check.expected_units > 0);
        assert_eq!(check.observed_units, 0);
        assert_eq!(check.completeness_ratio, 0.0);
    }

    proptest! {
        #[test]
        fn hostile_ga4_json_never_panics(input in ".*") {
            let result = catch_unwind(AssertUnwindSafe(|| parse_ga4_event(&input)));
            prop_assert!(result.is_ok());
            if let Ok(Err(err)) = result {
                prop_assert!(!err.code.trim().is_empty());
                prop_assert!(!err.field.trim().is_empty());
                if let Some(sample) = err.sample {
                    prop_assert!(sample.len() <= 96);
                }
            }
        }

        #[test]
        fn hostile_ads_json_never_panics(input in ".*") {
            let result = catch_unwind(AssertUnwindSafe(|| parse_google_ads_row(&input)));
            prop_assert!(result.is_ok());
            if let Ok(Err(err)) = result {
                prop_assert!(!err.code.trim().is_empty());
                prop_assert!(!err.field.trim().is_empty());
                if let Some(sample) = err.sample {
                    prop_assert!(sample.len() <= 96);
                }
            }
        }

        #[test]
        fn hostile_wix_json_never_panics(input in ".*") {
            let result = catch_unwind(AssertUnwindSafe(|| parse_wix_order(&input)));
            prop_assert!(result.is_ok());
            if let Ok(Err(err)) = result {
                prop_assert!(!err.code.trim().is_empty());
                prop_assert!(!err.field.trim().is_empty());
                if let Some(sample) = err.sample {
                    prop_assert!(sample.len() <= 96);
                }
            }
        }

        #[test]
        fn join_coverage_property_bounds(total in 0u64..10_000, joined in 0u64..10_000) {
            let ratio = join_coverage_ratio(total, joined);
            prop_assert!((0.0..=1.0).contains(&ratio));
        }
    }
}
