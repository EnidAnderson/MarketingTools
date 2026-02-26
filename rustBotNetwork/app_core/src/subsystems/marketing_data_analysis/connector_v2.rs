use super::analytics_config::{
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
};
use super::contracts::{AnalyticsError, MockAnalyticsRequestV1};
use super::ingest::{Ga4EventRawV1, WixOrderRawV1};
use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupResource, CampaignResource, GoogleAdsRow, KeywordData,
    MetricsData, SegmentsData,
};
use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

const CONNECTOR_CONTRACT_VERSION_V2: &str = "analytics_connector_contract.v2";
const CAMPAIGN_NAMES: &[&str] = &[
    "Summer Pet Food Promo",
    "New Puppy Essentials",
    "Senior Dog Health",
    "Organic Cat Treats",
];
const AD_GROUP_NAMES: &[&str] = &["Dry Food", "Wet Food", "Treats", "Supplements"];
const KEYWORD_TEXTS: &[&str] = &[
    "healthy dog food",
    "grain-free cat food",
    "best puppy treats",
    "senior pet vitamins",
];

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Source capability metadata published by analytics connectors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorSourceCapabilityV1 {
    pub source_system: String,
    pub granularity: Vec<String>,
    pub read_mode: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Connector capability contract for orchestration and UI discoverability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsConnectorCapabilitiesV1 {
    pub connector_id: String,
    pub contract_version: String,
    pub supports_healthcheck: bool,
    pub sources: Vec<ConnectorSourceCapabilityV1>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Source-level health state for connector preflight checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorSourceHealthV1 {
    pub source_system: String,
    pub enabled: bool,
    pub credentials_present: bool,
    pub blocking_reasons: Vec<String>,
    pub warning_reasons: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Connector health response for governance and preflight workflows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorHealthStatusV1 {
    pub connector_id: String,
    pub ok: bool,
    pub mode: String,
    pub source_status: Vec<ConnectorSourceHealthV1>,
    pub blocking_reasons: Vec<String>,
    pub warning_reasons: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Canonical raw Wix session shape for ingest-boundary normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WixSessionRawV1 {
    pub session_id: String,
    pub started_at_utc: String,
    pub visitor_id: String,
    pub landing_path: String,
    pub traffic_source: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Async connector contract for GA4 + Google Ads + Wix observed/simulated fetches.
#[async_trait]
pub trait AnalyticsConnectorContractV2: Send + Sync {
    fn capabilities(&self) -> AnalyticsConnectorCapabilitiesV1;

    async fn healthcheck(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<ConnectorHealthStatusV1, AnalyticsError>;

    async fn fetch_ga4_events(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError>;

    async fn fetch_google_ads_rows(
        &self,
        config: &AnalyticsConnectorConfigV1,
        request: &MockAnalyticsRequestV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<GoogleAdsRow>, AnalyticsError>;

    async fn fetch_wix_orders(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<WixOrderRawV1>, AnalyticsError>;

    async fn fetch_wix_sessions(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<WixSessionRawV1>, AnalyticsError>;
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Default deterministic simulated connector implementation.
#[derive(Debug, Default)]
pub struct SimulatedAnalyticsConnectorV2;

impl SimulatedAnalyticsConnectorV2 {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AnalyticsConnectorContractV2 for SimulatedAnalyticsConnectorV2 {
    fn capabilities(&self) -> AnalyticsConnectorCapabilitiesV1 {
        AnalyticsConnectorCapabilitiesV1 {
            connector_id: "mock_analytics_connector_v2".to_string(),
            contract_version: CONNECTOR_CONTRACT_VERSION_V2.to_string(),
            supports_healthcheck: true,
            sources: vec![
                ConnectorSourceCapabilityV1 {
                    source_system: "ga4".to_string(),
                    granularity: vec!["hour".to_string(), "day".to_string()],
                    read_mode: "simulated".to_string(),
                },
                ConnectorSourceCapabilityV1 {
                    source_system: "google_ads".to_string(),
                    granularity: vec!["day".to_string()],
                    read_mode: "simulated".to_string(),
                },
                ConnectorSourceCapabilityV1 {
                    source_system: "wix_storefront".to_string(),
                    granularity: vec!["hour".to_string(), "day".to_string()],
                    read_mode: "simulated".to_string(),
                },
            ],
        }
    }

    async fn healthcheck(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<ConnectorHealthStatusV1, AnalyticsError> {
        validate_analytics_connector_config_v1(config)?;

        let mut status = Vec::new();
        status.push(source_health(
            "ga4",
            config.ga4.enabled,
            &[
                &config.ga4.api_secret_env_var,
                &config.ga4.measurement_id_env_var,
            ],
        ));
        status.push(source_health(
            "google_ads",
            config.google_ads.enabled,
            &[
                &config.google_ads.developer_token_env_var,
                &config.google_ads.oauth_client_id_env_var,
                &config.google_ads.oauth_client_secret_env_var,
                &config.google_ads.oauth_refresh_token_env_var,
            ],
        ));
        status.push(source_health(
            "wix_storefront",
            config.wix.enabled,
            &[&config.wix.api_token_env_var],
        ));

        let mut blocking_reasons = Vec::new();
        let mut warning_reasons = Vec::new();

        for source in &status {
            if config.mode == AnalyticsConnectorModeV1::ObservedReadOnly
                && source.enabled
                && !source.credentials_present
            {
                blocking_reasons.push(format!(
                    "{} credentials missing for observed_read_only mode",
                    source.source_system
                ));
            }
            warning_reasons.extend(source.warning_reasons.iter().cloned());
        }

        let ok = blocking_reasons.is_empty();

        Ok(ConnectorHealthStatusV1 {
            connector_id: self.capabilities().connector_id,
            ok,
            mode: match config.mode {
                AnalyticsConnectorModeV1::Simulated => "simulated".to_string(),
                AnalyticsConnectorModeV1::ObservedReadOnly => "observed_read_only".to_string(),
            },
            source_status: status,
            blocking_reasons,
            warning_reasons,
        })
    }

    async fn fetch_ga4_events(
        &self,
        _config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError> {
        Ok(generate_simulated_ga4_events(start, end, seed))
    }

    async fn fetch_google_ads_rows(
        &self,
        _config: &AnalyticsConnectorConfigV1,
        request: &MockAnalyticsRequestV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
        Ok(generate_simulated_google_ads_rows(
            request, start, end, seed,
        ))
    }

    async fn fetch_wix_orders(
        &self,
        _config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<WixOrderRawV1>, AnalyticsError> {
        Ok(generate_simulated_wix_orders(start, end, seed))
    }

    async fn fetch_wix_sessions(
        &self,
        _config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        seed: u64,
    ) -> Result<Vec<WixSessionRawV1>, AnalyticsError> {
        Ok(generate_simulated_wix_sessions(start, end, seed))
    }
}

fn source_health(
    source_system: &str,
    enabled: bool,
    required_env_vars: &[&str],
) -> ConnectorSourceHealthV1 {
    if !enabled {
        return ConnectorSourceHealthV1 {
            source_system: source_system.to_string(),
            enabled,
            credentials_present: false,
            blocking_reasons: Vec::new(),
            warning_reasons: vec!["source disabled in connector config".to_string()],
        };
    }

    let missing = required_env_vars
        .iter()
        .filter(|env_name| !credential_present(env_name))
        .map(|env_name| (*env_name).to_string())
        .collect::<Vec<_>>();

    if missing.is_empty() {
        ConnectorSourceHealthV1 {
            source_system: source_system.to_string(),
            enabled,
            credentials_present: true,
            blocking_reasons: Vec::new(),
            warning_reasons: Vec::new(),
        }
    } else {
        ConnectorSourceHealthV1 {
            source_system: source_system.to_string(),
            enabled,
            credentials_present: false,
            blocking_reasons: Vec::new(),
            warning_reasons: vec![format!("missing env vars: {}", missing.join(", "))],
        }
    }
}

fn credential_present(env_name: &str) -> bool {
    std::env::var(env_name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub fn generate_simulated_ga4_events(
    start: NaiveDate,
    end: NaiveDate,
    seed: u64,
) -> Vec<Ga4EventRawV1> {
    let mut current = start;
    let mut events = Vec::new();

    while current <= end {
        events.push(Ga4EventRawV1 {
            event_name: " purchase ".to_string(),
            event_timestamp_utc: format!("{}T12:00:00Z", current.format("%Y-%m-%d")),
            user_pseudo_id: format!(" user_{}_{} ", seed % 1000, current.ordinal()),
            session_id: Some(format!("sess_{}_{}", seed, current.ordinal())),
            campaign: Some("spring_launch".to_string()),
        });
        let Some(next) = current.checked_add_signed(Duration::days(1)) else {
            break;
        };
        current = next;
    }

    events
}

pub fn generate_simulated_wix_orders(
    start: NaiveDate,
    end: NaiveDate,
    seed: u64,
) -> Vec<WixOrderRawV1> {
    let mut current = start;
    let mut orders = Vec::new();

    while current <= end {
        let day_offset = current.ordinal() as u64;
        orders.push(WixOrderRawV1 {
            order_id: format!("WIX-{}-{}", seed % 10_000, day_offset),
            placed_at_utc: format!("{}T18:15:00Z", current.format("%Y-%m-%d")),
            gross_amount: format!("{:.2}", 100.0 + (day_offset % 25) as f64),
            currency: "USD".to_string(),
        });
        let Some(next) = current.checked_add_signed(Duration::days(1)) else {
            break;
        };
        current = next;
    }

    orders
}

pub fn generate_simulated_wix_sessions(
    start: NaiveDate,
    end: NaiveDate,
    seed: u64,
) -> Vec<WixSessionRawV1> {
    let mut current = start;
    let mut sessions = Vec::new();

    while current <= end {
        sessions.push(WixSessionRawV1 {
            session_id: format!("wixsess-{}-{}", seed, current.ordinal()),
            started_at_utc: format!("{}T11:00:00Z", current.format("%Y-%m-%d")),
            visitor_id: format!("visitor-{}", seed % 5000),
            landing_path: "/collections/dog-food".to_string(),
            traffic_source: Some("google/cpc".to_string()),
        });
        let Some(next) = current.checked_add_signed(Duration::days(1)) else {
            break;
        };
        current = next;
    }

    sessions
}

pub fn generate_simulated_google_ads_rows(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
    seed: u64,
) -> Vec<GoogleAdsRow> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut rows = Vec::new();
    let mut current = start;

    while current <= end {
        let date_str = current.format("%Y-%m-%d").to_string();
        for (campaign_idx, campaign_name) in CAMPAIGN_NAMES.iter().enumerate() {
            if let Some(filter) = &request.campaign_filter {
                if !campaign_name.contains(filter) {
                    continue;
                }
            }
            let campaign_id = format!("{}", campaign_idx + 1);
            let campaign_resource = format!("customers/123/campaigns/{}", campaign_id);
            let campaign_status = if rng.gen_bool(0.9) {
                "ENABLED"
            } else {
                "PAUSED"
            };
            let campaign = CampaignResource {
                resource_name: campaign_resource.clone(),
                id: campaign_id.clone(),
                name: (*campaign_name).to_string(),
                status: campaign_status.to_string(),
            };

            for (ad_group_idx, ad_group_name) in AD_GROUP_NAMES.iter().enumerate() {
                if let Some(filter) = &request.ad_group_filter {
                    if !ad_group_name.contains(filter) {
                        continue;
                    }
                }
                let ad_group_id = format!("{}.{}", campaign_id, ad_group_idx + 1);
                let ad_group_resource = format!("customers/123/adGroups/{}", ad_group_id);
                let ad_group_status = if rng.gen_bool(0.9) {
                    "ENABLED"
                } else {
                    "PAUSED"
                };
                let ad_group = AdGroupResource {
                    resource_name: ad_group_resource.clone(),
                    id: ad_group_id.clone(),
                    name: (*ad_group_name).to_string(),
                    status: ad_group_status.to_string(),
                    campaign_resource_name: campaign_resource.clone(),
                };

                for (kw_idx, keyword_text) in KEYWORD_TEXTS.iter().enumerate() {
                    let impressions: u64 = rng.gen_range(100..1200);
                    let max_clicks = (impressions / 2).max(1);
                    let clicks: u64 = rng.gen_range(1..=max_clicks);
                    let cost_micros = clicks * rng.gen_range(200_000..1_300_000);
                    let conversions = round4(rng.gen_range(0.0..(clicks as f64 / 5.0)));
                    let conversions_value = round4(conversions * rng.gen_range(10.0..60.0));

                    let metrics = MetricsData {
                        clicks,
                        impressions,
                        cost_micros: cost_micros,
                        conversions,
                        conversions_value: conversions_value,
                        ctr: round4((clicks as f64 / impressions as f64) * 100.0),
                        average_cpc: round4(cost_micros as f64 / clicks as f64 / 1_000_000.0),
                    };

                    let criterion_id =
                        format!("{}{}{}", campaign_idx + 1, ad_group_idx + 1, kw_idx + 1);
                    let criterion = AdGroupCriterionResource {
                        resource_name: format!(
                            "customers/123/adGroupCriteria/{}.{}",
                            ad_group_id, criterion_id
                        ),
                        criterion_id: criterion_id,
                        status: "ENABLED".to_string(),
                        keyword: Some(KeywordData {
                            text: (*keyword_text).to_string(),
                            match_type: "EXACT".to_string(),
                        }),
                        quality_score: Some(rng.gen_range(1..=10)),
                        ad_group_resource_name: ad_group_resource.clone(),
                    };

                    rows.push(GoogleAdsRow {
                        campaign: Some(campaign.clone()),
                        ad_group: Some(ad_group.clone()),
                        keyword_view: None,
                        ad_group_criterion: Some(criterion),
                        metrics: Some(metrics),
                        segments: Some(SegmentsData {
                            date: Some(date_str.clone()),
                            device: Some("DESKTOP".to_string()),
                        }),
                    });
                }
            }
        }
        let Some(next) = current.checked_add_signed(Duration::days(1)) else {
            break;
        };
        current = next;
    }

    rows
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_publish_contract_metadata() {
        let connector = SimulatedAnalyticsConnectorV2::new();
        let caps = connector.capabilities();
        assert_eq!(caps.contract_version, CONNECTOR_CONTRACT_VERSION_V2);
        assert_eq!(caps.sources.len(), 3);
        assert!(caps.supports_healthcheck);
    }

    #[tokio::test]
    async fn healthcheck_blocks_observed_mode_when_credentials_missing() {
        let connector = SimulatedAnalyticsConnectorV2::new();
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.ga4.api_secret_env_var = "GA4_MISSING_SECRET".to_string();
        cfg.ga4.measurement_id_env_var = "GA4_MISSING_MEASUREMENT".to_string();
        cfg.google_ads.developer_token_env_var = "GOOGLE_ADS_MISSING_DEVELOPER".to_string();
        cfg.google_ads.oauth_client_id_env_var = "GOOGLE_ADS_MISSING_CLIENT_ID".to_string();
        cfg.google_ads.oauth_client_secret_env_var = "GOOGLE_ADS_MISSING_CLIENT_SECRET".to_string();
        cfg.google_ads.oauth_refresh_token_env_var = "GOOGLE_ADS_MISSING_REFRESH".to_string();
        cfg.wix.api_token_env_var = "WIX_MISSING_TOKEN".to_string();

        let status = connector.healthcheck(&cfg).await.expect("healthcheck");
        assert!(!status.ok);
        assert!(!status.blocking_reasons.is_empty());
    }

    #[test]
    fn simulated_google_ads_rows_are_seed_stable() {
        let request = MockAnalyticsRequestV1 {
            start_date: "2026-02-01".to_string(),
            end_date: "2026-02-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(42),
            profile_id: "stable".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };
        let start = NaiveDate::from_ymd_opt(2026, 2, 1).expect("date");
        let end = NaiveDate::from_ymd_opt(2026, 2, 2).expect("date");

        let a = generate_simulated_google_ads_rows(&request, start, end, 42);
        let b = generate_simulated_google_ads_rows(&request, start, end, 42);
        assert_eq!(a.len(), b.len());
        assert_eq!(
            serde_json::to_string(&a).expect("serialize"),
            serde_json::to_string(&b).expect("serialize")
        );
    }
}
