// provenance: decision_id=DEC-0016; change_request_id=CR-QA_FIXER-0033
use chrono::{DateTime, NaiveDate, Utc};
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
        Self {
            code: value.code,
            field: value.field,
            reason: value.reason,
            sample: value.sample,
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

pub fn window_completeness(expected_units: u32, observed_units: u32) -> WindowCompletenessCheck {
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
    fn join_coverage_stays_in_range_and_is_monotone_when_removing_unmatched() {
        let baseline = join_coverage_ratio(100, 75);
        let no_unmatched_removed = join_coverage_ratio(90, 75);
        assert!((0.0..=1.0).contains(&baseline));
        assert!((0.0..=1.0).contains(&no_unmatched_removed));
        assert!(no_unmatched_removed >= baseline);
    }

    proptest! {
        #[test]
        fn hostile_ga4_json_never_panics(input in ".*") {
            let result = catch_unwind(AssertUnwindSafe(|| parse_ga4_event(&input)));
            prop_assert!(result.is_ok());
        }

        #[test]
        fn join_coverage_property_bounds(total in 0u64..10_000, joined in 0u64..10_000) {
            let ratio = join_coverage_ratio(total, joined);
            prop_assert!((0.0..=1.0).contains(&ratio));
        }
    }
}
