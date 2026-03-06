use super::analytics_config::{
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
    Ga4ReadBackendV1,
};
use super::contracts::{AnalyticsError, MockAnalyticsRequestV1};
use super::ingest::{Ga4EventRawV1, WixOrderRawV1};
use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupResource, CampaignResource, GoogleAdsRow, KeywordData,
    MetricsData, SegmentsData,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const CONNECTOR_CONTRACT_VERSION_V2: &str = "analytics_connector_contract.v2";
const OBSERVED_CONNECTOR_ID_V2: &str = "analytics_observed_read_only_connector_v2";
const DEFAULT_GA4_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_GA4_DATA_API_BASE_URL: &str = "https://analyticsdata.googleapis.com/v1beta";
const GA4_SCOPE_READONLY: &str = "https://www.googleapis.com/auth/analytics.readonly";
const BIGQUERY_SCOPE_READONLY: &str = "https://www.googleapis.com/auth/bigquery";
const GA4_RAW_REPORT_SCHEMA_VERSION_V1: &str = "ga4_raw_report.v1";
const DEFAULT_BIGQUERY_API_BASE_URL: &str = "https://bigquery.googleapis.com/bigquery/v2";
const DEFAULT_GA4_PAGE_LIMIT: u32 = 10_000;
const MAX_GA4_PAGE_LIMIT: u32 = 100_000;
const DEFAULT_GA4_MAX_PAGES: u32 = 25;
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
    #[serde(default)]
    pub live_probe_ok: bool,
    #[serde(default)]
    pub probe_status: String,
    #[serde(default)]
    pub probe_message: Option<String>,
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
/// purpose: Query contract for fetching GA4 raw rows through Data API runReport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ga4RawQueryV1 {
    pub start_date: String,
    pub end_date: String,
    pub dimensions: Vec<String>,
    pub metrics: Vec<String>,
    #[serde(default)]
    pub event_names: Vec<String>,
    #[serde(default)]
    pub page_limit: Option<u32>,
    #[serde(default)]
    pub max_pages: Option<u32>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: One GA4 raw row as named dimension/metric maps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Ga4RawReportRowV1 {
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Stable envelope for paginated GA4 raw report export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Ga4RawReportV1 {
    pub schema_version: String,
    pub property_id: String,
    pub start_date: String,
    pub end_date: String,
    pub dimensions: Vec<String>,
    pub metrics: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Ga4RawReportRowV1>,
    pub row_count_hint: Option<u32>,
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

#[derive(Debug, Deserialize)]
struct ServiceAccountCredentials {
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServiceAccountClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Serialize)]
struct ServiceAccountJwtHeader {
    alg: &'static str,
    typ: &'static str,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct Ga4RunReportResponse {
    #[serde(rename = "dimensionHeaders", default)]
    dimension_headers: Vec<Ga4RunReportHeader>,
    #[serde(rename = "metricHeaders", default)]
    metric_headers: Vec<Ga4RunReportHeader>,
    #[serde(rename = "rowCount", default)]
    row_count: Option<u32>,
    #[serde(default)]
    rows: Vec<Ga4RunReportRow>,
}

#[derive(Debug, Deserialize)]
struct Ga4RunReportHeader {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct Ga4RunReportRow {
    #[serde(rename = "dimensionValues", default)]
    dimension_values: Vec<Ga4RunReportValue>,
    #[serde(rename = "metricValues", default)]
    metric_values: Vec<Ga4RunReportValue>,
}

#[derive(Debug, Deserialize)]
struct Ga4RunReportValue {
    #[serde(default)]
    value: String,
}

#[derive(Debug, Deserialize)]
struct BigQueryQueryResponse {
    #[serde(default)]
    schema: Option<BigQueryTableSchema>,
    #[serde(default)]
    rows: Vec<BigQueryQueryRow>,
}

#[derive(Debug, Deserialize)]
struct BigQueryTableSchema {
    #[serde(default)]
    fields: Vec<BigQueryFieldSchema>,
}

#[derive(Debug, Deserialize)]
struct BigQueryFieldSchema {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct BigQueryQueryRow {
    #[serde(default)]
    f: Vec<BigQueryFieldValue>,
}

#[derive(Debug, Deserialize)]
struct BigQueryFieldValue {
    #[serde(default)]
    v: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_v2`
/// purpose: Observed/read-only connector implementation for live GA4 reads.
#[derive(Debug, Clone)]
pub struct ObservedReadOnlyAnalyticsConnectorV2 {
    http: Client,
}

impl Default for ObservedReadOnlyAnalyticsConnectorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservedReadOnlyAnalyticsConnectorV2 {
    pub fn new() -> Self {
        // Avoid platform proxy-resolution panics in constrained/headless environments.
        let http = Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { http }
    }

    pub async fn fetch_ga4_raw_report(
        &self,
        config: &AnalyticsConnectorConfigV1,
        query: Ga4RawQueryV1,
    ) -> Result<Ga4RawReportV1, AnalyticsError> {
        validate_analytics_connector_config_v1(config)?;
        if config.mode != AnalyticsConnectorModeV1::ObservedReadOnly {
            return Err(AnalyticsError::validation(
                "analytics_ga4_raw_requires_observed_mode",
                "GA4 raw report export requires observed_read_only connector mode",
                "mode",
            ));
        }
        if !config.ga4.enabled {
            return Err(AnalyticsError::validation(
                "analytics_source_not_enabled",
                "ga4 source is disabled in connector config",
                "ga4.enabled",
            ));
        }
        let query = self.normalize_ga4_query(query)?;
        let start = parse_iso_date(&query.start_date, "start_date")?;
        let end = parse_iso_date(&query.end_date, "end_date")?;
        if start > end {
            return Err(AnalyticsError::new(
                "analytics_ga4_raw_invalid_date_range",
                "start_date must be <= end_date",
                vec!["start_date".to_string(), "end_date".to_string()],
                None,
            ));
        }

        let page_limit = query
            .page_limit
            .unwrap_or(DEFAULT_GA4_PAGE_LIMIT)
            .clamp(1, MAX_GA4_PAGE_LIMIT);
        let max_pages = query.max_pages.unwrap_or(DEFAULT_GA4_MAX_PAGES).max(1);
        let mut rows_out = Vec::new();
        let mut offset = 0_u32;
        let mut row_count_hint = None;
        let mut pages = 0_u32;
        loop {
            let page = self
                .run_ga4_report_page(
                    config,
                    start,
                    end,
                    &query.dimensions,
                    &query.metrics,
                    page_limit,
                    offset,
                    &query.event_names,
                )
                .await?;
            row_count_hint = row_count_hint.or(page.row_count);
            let page_rows = self.rows_to_named_maps(&page);
            if page_rows.is_empty() {
                break;
            }
            offset = offset.saturating_add(page_rows.len() as u32);
            rows_out.extend(page_rows);
            pages = pages.saturating_add(1);
            if pages >= max_pages {
                break;
            }
            if let Some(total) = page.row_count {
                if offset >= total {
                    break;
                }
            }
            if page.rows.len() < page_limit as usize {
                break;
            }
        }

        Ok(Ga4RawReportV1 {
            schema_version: GA4_RAW_REPORT_SCHEMA_VERSION_V1.to_string(),
            property_id: config.ga4.property_id.trim().to_string(),
            start_date: query.start_date,
            end_date: query.end_date,
            dimensions: query.dimensions,
            metrics: query.metrics,
            rows: rows_out,
            row_count_hint,
        })
    }

    fn normalize_ga4_query(
        &self,
        mut query: Ga4RawQueryV1,
    ) -> Result<Ga4RawQueryV1, AnalyticsError> {
        query.start_date = query.start_date.trim().to_string();
        query.end_date = query.end_date.trim().to_string();
        query.dimensions = normalize_field_list(query.dimensions);
        query.metrics = normalize_field_list(query.metrics);
        query.event_names = query
            .event_names
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if query.start_date.is_empty() || query.end_date.is_empty() {
            return Err(AnalyticsError::validation(
                "analytics_ga4_raw_query_dates_required",
                "start_date and end_date are required",
                "start_date",
            ));
        }
        if query.dimensions.is_empty() {
            return Err(AnalyticsError::validation(
                "analytics_ga4_raw_query_dimensions_required",
                "at least one GA4 dimension is required",
                "dimensions",
            ));
        }
        if query.metrics.is_empty() {
            return Err(AnalyticsError::validation(
                "analytics_ga4_raw_query_metrics_required",
                "at least one GA4 metric is required",
                "metrics",
            ));
        }
        for name in query.dimensions.iter().chain(query.metrics.iter()) {
            if !is_valid_ga4_field_name(name) {
                return Err(AnalyticsError::validation(
                    "analytics_ga4_raw_query_field_invalid",
                    format!("GA4 field '{}' contains invalid characters", name),
                    "dimensions",
                ));
            }
        }
        Ok(query)
    }

    fn rows_to_named_maps(&self, response: &Ga4RunReportResponse) -> Vec<Ga4RawReportRowV1> {
        let mut rows_out = Vec::with_capacity(response.rows.len());
        for row in &response.rows {
            let mut dimensions = BTreeMap::new();
            for (idx, value) in row.dimension_values.iter().enumerate() {
                let key = response
                    .dimension_headers
                    .get(idx)
                    .map(|header| header.name.trim())
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("dimension_{}", idx));
                dimensions.insert(key, value.value.clone());
            }

            let mut metrics = BTreeMap::new();
            for (idx, value) in row.metric_values.iter().enumerate() {
                let key = response
                    .metric_headers
                    .get(idx)
                    .map(|header| header.name.trim())
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("metric_{}", idx));
                metrics.insert(key, value.value.clone());
            }

            rows_out.push(Ga4RawReportRowV1 {
                dimensions,
                metrics,
            });
        }
        rows_out
    }

    fn credentials_path(&self, config: &AnalyticsConnectorConfigV1) -> Option<String> {
        std::env::var(config.ga4.read_credentials_env_var.trim())
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn load_service_account(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<ServiceAccountCredentials, AnalyticsError> {
        let Some(path) = self.credentials_path(config) else {
            return Err(AnalyticsError::new(
                "analytics_ga4_credentials_missing",
                format!(
                    "GA4 credentials env var '{}' is missing or empty",
                    config.ga4.read_credentials_env_var
                ),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            ));
        };
        let raw = fs::read_to_string(&path).map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_credentials_file_unreadable",
                format!("failed to read GA4 credentials file at '{}': {}", path, err),
                vec!["ga4.read_credentials_env_var".to_string()],
                Some(json!({ "path": path })),
            )
        })?;
        let credentials: ServiceAccountCredentials = serde_json::from_str(&raw).map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_credentials_parse_failed",
                format!("failed to parse GA4 credentials JSON: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                Some(json!({ "path": path })),
            )
        })?;
        if credentials.client_email.trim().is_empty() || credentials.private_key.trim().is_empty() {
            return Err(AnalyticsError::new(
                "analytics_ga4_credentials_invalid",
                "GA4 service-account credentials require non-empty client_email and private_key",
                vec!["ga4.read_credentials_env_var".to_string()],
                Some(json!({ "path": path })),
            ));
        }
        Ok(credentials)
    }

    fn oauth_token_url(&self, credentials: &ServiceAccountCredentials) -> String {
        std::env::var("ANALYTICS_GA4_OAUTH_TOKEN_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                credentials
                    .token_uri
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| DEFAULT_GA4_OAUTH_TOKEN_URL.to_string())
    }

    fn ga4_data_api_base_url(&self) -> String {
        std::env::var("ANALYTICS_GA4_DATA_API_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_GA4_DATA_API_BASE_URL.to_string())
    }

    fn signed_assertion(
        &self,
        credentials: &ServiceAccountCredentials,
        token_url: &str,
        scope: &str,
    ) -> Result<String, AnalyticsError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or(0);
        let claims = ServiceAccountClaims {
            iss: credentials.client_email.trim(),
            sub: credentials.client_email.trim(),
            scope,
            aud: token_url,
            iat: now.saturating_sub(30),
            exp: now.saturating_add(3600),
        };
        let header = ServiceAccountJwtHeader {
            alg: "RS256",
            typ: "JWT",
        };
        let header_json = serde_json::to_vec(&header).map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_assertion_sign_failed",
                format!("failed to serialize OAuth assertion header: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            )
        })?;
        let claims_json = serde_json::to_vec(&claims).map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_assertion_sign_failed",
                format!("failed to serialize OAuth assertion claims: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            )
        })?;

        let header_b64 = URL_SAFE_NO_PAD.encode(header_json);
        let claims_b64 = URL_SAFE_NO_PAD.encode(claims_json);
        let signing_input = format!("{}.{}", header_b64, claims_b64);

        let signature = self.sign_rs256_with_openssl(
            credentials.private_key.as_bytes(),
            signing_input.as_bytes(),
        )?;

        Ok(format!(
            "{}.{}",
            signing_input,
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    fn sign_rs256_with_openssl(
        &self,
        private_key_pem: &[u8],
        payload: &[u8],
    ) -> Result<Vec<u8>, AnalyticsError> {
        let mut key_path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        key_path.push(format!("ga4-sa-key-{}.pem", nonce));
        fs::write(&key_path, private_key_pem).map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_private_key_invalid",
                format!("failed to materialize temporary private key file: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            )
        })?;

        let mut child = Command::new("openssl")
            .arg("dgst")
            .arg("-sha256")
            .arg("-sign")
            .arg(&key_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                let _ = fs::remove_file(&key_path);
                AnalyticsError::new(
                    "analytics_ga4_assertion_sign_failed",
                    format!("failed to launch openssl for OAuth signing: {}", err),
                    vec!["ga4.read_credentials_env_var".to_string()],
                    None,
                )
            })?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(payload).map_err(|err| {
                let _ = fs::remove_file(&key_path);
                AnalyticsError::new(
                    "analytics_ga4_assertion_sign_failed",
                    format!("failed to write OAuth signing payload: {}", err),
                    vec!["ga4.read_credentials_env_var".to_string()],
                    None,
                )
            })?;
        }

        let output = child.wait_with_output().map_err(|err| {
            let _ = fs::remove_file(&key_path);
            AnalyticsError::new(
                "analytics_ga4_assertion_sign_failed",
                format!("failed while waiting for openssl signer: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            )
        })?;
        let _ = fs::remove_file(&key_path);

        if !output.status.success() {
            return Err(AnalyticsError::new(
                "analytics_ga4_assertion_sign_failed",
                format!(
                    "openssl signer failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            ));
        }
        if output.stdout.is_empty() {
            return Err(AnalyticsError::new(
                "analytics_ga4_assertion_sign_failed",
                "openssl signer returned empty signature output",
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            ));
        }
        Ok(output.stdout)
    }

    async fn fetch_access_token_for_scope(
        &self,
        config: &AnalyticsConnectorConfigV1,
        scope: &str,
    ) -> Result<String, AnalyticsError> {
        let credentials = self.load_service_account(config)?;
        self.fetch_access_token_for_scope_with_credentials(&credentials, scope)
            .await
    }

    async fn fetch_access_token_for_scope_with_credentials(
        &self,
        credentials: &ServiceAccountCredentials,
        scope: &str,
    ) -> Result<String, AnalyticsError> {
        let token_url = self.oauth_token_url(credentials);
        let assertion = self.signed_assertion(credentials, &token_url, scope)?;
        let response = self
            .http
            .post(token_url.clone())
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|err| {
                AnalyticsError::new(
                    "analytics_ga4_token_exchange_failed",
                    format!("GA4 OAuth token exchange request failed: {}", err),
                    vec!["ga4.read_credentials_env_var".to_string()],
                    Some(json!({ "token_url": token_url })),
                )
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AnalyticsError::new(
                "analytics_ga4_token_exchange_failed",
                format!("GA4 OAuth token exchange failed with status {}", status),
                vec!["ga4.read_credentials_env_var".to_string()],
                Some(json!({ "status": status, "body": body })),
            ));
        }
        let payload: OAuthTokenResponse = response.json().await.map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_token_response_invalid",
                format!("failed to parse GA4 OAuth token response: {}", err),
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            )
        })?;
        if payload.access_token.trim().is_empty() {
            return Err(AnalyticsError::new(
                "analytics_ga4_token_response_invalid",
                "GA4 OAuth token response did not include access_token",
                vec!["ga4.read_credentials_env_var".to_string()],
                None,
            ));
        }
        Ok(payload.access_token)
    }

    async fn run_ga4_report_page(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        dimensions: &[String],
        metrics: &[String],
        limit: u32,
        offset: u32,
        event_names: &[String],
    ) -> Result<Ga4RunReportResponse, AnalyticsError> {
        let access_credential = self
            .fetch_access_token_for_scope(config, GA4_SCOPE_READONLY)
            .await?;
        let base_url = self.ga4_data_api_base_url();
        let url = format!(
            "{}/properties/{}:runReport",
            base_url,
            config.ga4.property_id.trim()
        );
        let dimensions_json = dimensions
            .iter()
            .map(|name| json!({ "name": name }))
            .collect::<Vec<_>>();
        let metrics_json = metrics
            .iter()
            .map(|name| json!({ "name": name }))
            .collect::<Vec<_>>();
        let mut payload = json!({
            "dateRanges": [{
                "startDate": start.format("%Y-%m-%d").to_string(),
                "endDate": end.format("%Y-%m-%d").to_string()
            }],
            "dimensions": dimensions_json,
            "metrics": metrics_json,
            "limit": limit.to_string(),
            "offset": offset.to_string()
        });
        if !event_names.is_empty() && dimensions.iter().any(|name| name == "eventName") {
            payload["dimensionFilter"] = json!({
                "filter": {
                    "fieldName": "eventName",
                    "inListFilter": {
                        "values": event_names,
                        "caseSensitive": false
                    }
                }
            });
        }
        let response = self
            .http
            .post(url.clone())
            .bearer_auth(access_credential)
            .json(&payload)
            .send()
            .await
            .map_err(|err| {
                AnalyticsError::new(
                    "analytics_ga4_run_report_failed",
                    format!("GA4 runReport request failed: {}", err),
                    vec!["ga4.property_id".to_string()],
                    Some(json!({ "url": url })),
                )
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AnalyticsError::new(
                "analytics_ga4_run_report_failed",
                format!("GA4 runReport failed with status {}", status),
                vec!["ga4.property_id".to_string()],
                Some(json!({ "status": status, "body": body })),
            ));
        }
        response.json().await.map_err(|err| {
            AnalyticsError::new(
                "analytics_ga4_schema_parse_failed",
                format!("failed to parse GA4 runReport response: {}", err),
                vec!["ga4.property_id".to_string()],
                None,
            )
        })
    }

    async fn run_ga4_report(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        limit: u32,
    ) -> Result<Ga4RunReportResponse, AnalyticsError> {
        let dimensions = vec![
            "eventName".to_string(),
            "dateHour".to_string(),
            "campaignName".to_string(),
            "deviceCategory".to_string(),
            "sessionSourceMedium".to_string(),
        ];
        let metrics = vec!["eventCount".to_string()];
        self.run_ga4_report_page(config, start, end, &dimensions, &metrics, limit, 0, &[])
            .await
    }

    fn bigquery_api_base_url(&self) -> String {
        std::env::var("ANALYTICS_BIGQUERY_API_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_BIGQUERY_API_BASE_URL.to_string())
    }

    fn resolve_bigquery_project_id(
        &self,
        config: &AnalyticsConnectorConfigV1,
        credentials: &ServiceAccountCredentials,
    ) -> Result<String, AnalyticsError> {
        let candidate = config
            .ga4
            .bigquery_project_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("GOOGLE_CLOUD_PROJECT")
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| {
                credentials
                    .project_id
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
            .ok_or_else(|| {
                AnalyticsError::new(
                    "analytics_bigquery_project_id_missing",
                    "BigQuery GA4 backend requires a project id (set ANALYTICS_GA4_BIGQUERY_PROJECT_ID or provide project_id in service account JSON)",
                    vec!["ga4.bigquery_project_id".to_string()],
                    None,
                )
            })?;

        if !is_valid_bigquery_project_id(&candidate) {
            return Err(AnalyticsError::new(
                "analytics_bigquery_project_id_invalid",
                "BigQuery project id contains invalid characters",
                vec!["ga4.bigquery_project_id".to_string()],
                Some(json!({ "project_id": candidate })),
            ));
        }
        Ok(candidate)
    }

    fn resolve_bigquery_dataset_id(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<String, AnalyticsError> {
        let dataset = config
            .ga4
            .bigquery_dataset_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("analytics_{}", config.ga4.property_id.trim()));
        if !is_valid_bigquery_dataset_id(&dataset) {
            return Err(AnalyticsError::new(
                "analytics_bigquery_dataset_id_invalid",
                "BigQuery dataset id contains invalid characters",
                vec!["ga4.bigquery_dataset_id".to_string()],
                Some(json!({ "dataset_id": dataset })),
            ));
        }
        Ok(dataset)
    }

    fn bigquery_max_rows(&self, config: &AnalyticsConnectorConfigV1) -> u32 {
        config.ga4.bigquery_max_rows.clamp(1, 500_000)
    }

    async fn run_bigquery_query(
        &self,
        config: &AnalyticsConnectorConfigV1,
        sql: &str,
        max_results: u32,
    ) -> Result<BigQueryQueryResponse, AnalyticsError> {
        let credentials = self.load_service_account(config)?;
        let project_id = self.resolve_bigquery_project_id(config, &credentials)?;
        let access_credential = self
            .fetch_access_token_for_scope_with_credentials(&credentials, BIGQUERY_SCOPE_READONLY)
            .await?;
        let url = format!(
            "{}/projects/{}/queries",
            self.bigquery_api_base_url(),
            project_id
        );

        let payload = json!({
            "query": sql,
            "useLegacySql": false,
            "maxResults": max_results,
            "timeoutMs": 30000,
            "useQueryCache": true,
            "maximumBytesBilled": config.ga4.bigquery_max_bytes_billed.to_string()
        });

        let response = self
            .http
            .post(url.clone())
            .bearer_auth(access_credential)
            .json(&payload)
            .send()
            .await
            .map_err(|err| {
                AnalyticsError::new(
                    "analytics_bigquery_query_failed",
                    format!("BigQuery query request failed: {}", err),
                    vec!["ga4.bigquery_project_id".to_string()],
                    Some(json!({ "url": url })),
                )
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AnalyticsError::new(
                "analytics_bigquery_query_failed",
                format!("BigQuery query failed with status {}", status),
                vec!["ga4.bigquery_project_id".to_string()],
                Some(json!({ "status": status, "body": body })),
            ));
        }
        response.json().await.map_err(|err| {
            AnalyticsError::new(
                "analytics_bigquery_schema_parse_failed",
                format!("failed to parse BigQuery query response: {}", err),
                vec!["ga4.bigquery_dataset_id".to_string()],
                None,
            )
        })
    }

    fn bigquery_rows_to_named_maps(
        &self,
        response: &BigQueryQueryResponse,
    ) -> Vec<BTreeMap<String, String>> {
        let field_names = response
            .schema
            .as_ref()
            .map(|schema| {
                schema
                    .fields
                    .iter()
                    .map(|field| field.name.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut rows_out = Vec::with_capacity(response.rows.len());
        for row in &response.rows {
            let mut out = BTreeMap::new();
            for (idx, value) in row.f.iter().enumerate() {
                let key = field_names
                    .get(idx)
                    .cloned()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| format!("field_{}", idx));
                out.insert(key, value.v.clone().unwrap_or_default());
            }
            rows_out.push(out);
        }
        rows_out
    }

    async fn run_bigquery_probe(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<BigQueryQueryResponse, AnalyticsError> {
        let credentials = self.load_service_account(config)?;
        let project_id = self.resolve_bigquery_project_id(config, &credentials)?;
        let dataset_id = self.resolve_bigquery_dataset_id(config)?;
        let sql = format!(
            "SELECT table_name FROM `{}.{}.INFORMATION_SCHEMA.TABLES` WHERE STARTS_WITH(table_name, 'events_') ORDER BY table_name DESC LIMIT 1",
            project_id, dataset_id
        );
        self.run_bigquery_query(config, &sql, 1).await
    }

    async fn fetch_ga4_events_bigquery(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError> {
        let credentials = self.load_service_account(config)?;
        let project_id = self.resolve_bigquery_project_id(config, &credentials)?;
        let dataset_id = self.resolve_bigquery_dataset_id(config)?;
        let start_suffix = start.format("%Y%m%d").to_string();
        let end_suffix = end.format("%Y%m%d").to_string();
        let limit = self.bigquery_max_rows(config);
        let sql = format!(
            "SELECT \
                event_name, \
                CAST(event_timestamp AS STRING) AS event_timestamp_micros, \
                user_pseudo_id, \
                device.category AS device_category, \
                platform, \
                geo.country AS country, \
                traffic_source.source AS traffic_source_source, \
                traffic_source.medium AS traffic_source_medium, \
                (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'campaign' LIMIT 1) AS campaign_name, \
                (SELECT CAST(ep.value.int_value AS STRING) FROM UNNEST(event_params) ep WHERE ep.key = 'ga_session_id' LIMIT 1) AS ga_session_id, \
                (SELECT CAST(COALESCE(ep.value.int_value, SAFE_CAST(ep.value.string_value AS INT64)) AS STRING) FROM UNNEST(event_params) ep WHERE ep.key = 'ga_session_number' LIMIT 1) AS ga_session_number, \
                (SELECT ep.value.string_value FROM UNNEST(event_params) ep WHERE ep.key = 'page_location' LIMIT 1) AS page_location, \
                (SELECT COALESCE(ep.value.string_value, CAST(ep.value.int_value AS STRING)) FROM UNNEST(event_params) ep WHERE ep.key = 'session_engaged' LIMIT 1) AS session_engaged, \
                (SELECT CAST(COALESCE(ep.value.int_value, SAFE_CAST(ep.value.string_value AS INT64)) AS STRING) FROM UNNEST(event_params) ep WHERE ep.key = 'engagement_time_msec' LIMIT 1) AS engagement_time_msec, \
                ecommerce.transaction_id AS transaction_id, \
                CAST(ecommerce.purchase_revenue AS STRING) AS purchase_revenue, \
                CAST(ecommerce.purchase_revenue_in_usd AS STRING) AS purchase_revenue_in_usd, \
                CAST(event_bundle_sequence_id AS STRING) AS event_bundle_sequence_id, \
                CAST(event_server_timestamp_offset AS STRING) AS event_server_timestamp_offset, \
                CAST(batch_event_index AS STRING) AS batch_event_index, \
                _TABLE_SUFFIX AS table_suffix \
            FROM `{}.{}.events_*` \
            WHERE _TABLE_SUFFIX BETWEEN '{}' AND '{}' \
            ORDER BY event_timestamp \
            LIMIT {}",
            project_id, dataset_id, start_suffix, end_suffix, limit
        );
        let response = self.run_bigquery_query(config, &sql, limit).await?;
        let rows = self.bigquery_rows_to_named_maps(&response);
        let mut events = Vec::with_capacity(rows.len());
        for (idx, mut row) in rows.into_iter().enumerate() {
            let event_name = row
                .remove("event_name")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AnalyticsError::new(
                        "analytics_bigquery_schema_mismatch",
                        "BigQuery row missing event_name",
                        vec!["ga4.bigquery_dataset_id".to_string()],
                        Some(json!({ "row_index": idx })),
                    )
                })?;
            let micros = row
                .remove("event_timestamp_micros")
                .and_then(|value| value.trim().parse::<i64>().ok())
                .ok_or_else(|| {
                    AnalyticsError::new(
                        "analytics_bigquery_schema_mismatch",
                        "BigQuery row missing valid event_timestamp_micros",
                        vec!["ga4.bigquery_dataset_id".to_string()],
                        Some(json!({ "row_index": idx })),
                    )
                })?;
            let timestamp = micros_to_rfc3339(micros).ok_or_else(|| {
                AnalyticsError::new(
                    "analytics_bigquery_schema_mismatch",
                    "BigQuery row event_timestamp_micros could not be converted to RFC3339",
                    vec!["ga4.bigquery_dataset_id".to_string()],
                    Some(json!({ "row_index": idx, "event_timestamp_micros": micros })),
                )
            })?;
            let user_pseudo_id = row
                .remove("user_pseudo_id")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("ga4_bq_user_{}", idx));
            let campaign = row
                .remove("campaign_name")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let device_category = row
                .remove("device_category")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let platform = row
                .remove("platform")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let country = row
                .remove("country")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let source = row
                .remove("traffic_source_source")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let medium = row
                .remove("traffic_source_medium")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let source_medium = match (source.as_ref(), medium.as_ref()) {
                (Some(source), Some(medium)) => Some(format!("{} / {}", source, medium)),
                (Some(source), None) => Some(source.clone()),
                (None, Some(medium)) => Some(medium.clone()),
                (None, None) => None,
            };
            let session_id = row
                .remove("ga_session_id")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|value| format!("ga_session:{}", value));
            let ga_session_id = session_id
                .as_ref()
                .and_then(|value| value.trim().strip_prefix("ga_session:"))
                .and_then(|value| value.parse::<i64>().ok());
            let ga_session_number = row
                .remove("ga_session_number")
                .and_then(|value| value.trim().parse::<i64>().ok());
            let page_location = row
                .remove("page_location")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let session_engaged = row
                .remove("session_engaged")
                .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
            let engagement_time_msec = row
                .remove("engagement_time_msec")
                .and_then(|value| value.trim().parse::<i64>().ok());
            let transaction_id = row
                .remove("transaction_id")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let purchase_revenue = row
                .remove("purchase_revenue")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let purchase_revenue_in_usd = row
                .remove("purchase_revenue_in_usd")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let purchase_revenue_value = purchase_revenue
                .as_ref()
                .and_then(|value| value.parse::<f64>().ok());
            let purchase_revenue_in_usd_value = purchase_revenue_in_usd
                .as_ref()
                .and_then(|value| value.parse::<f64>().ok());
            let event_bundle_sequence_id = row
                .get("event_bundle_sequence_id")
                .and_then(|value| value.trim().parse::<i64>().ok());
            let batch_event_index = row
                .get("batch_event_index")
                .and_then(|value| value.trim().parse::<i64>().ok());
            let mut dimensions = BTreeMap::new();
            dimensions.insert(
                "ga4_read_backend".to_string(),
                "bigquery_export".to_string(),
            );
            dimensions.insert("event_timestamp_micros".to_string(), micros.to_string());
            if let Some(value) = transaction_id.as_ref() {
                dimensions.insert("transaction_id".to_string(), value.clone());
            }
            if let Some(value) = purchase_revenue {
                dimensions.insert("purchase_revenue".to_string(), value);
            }
            if let Some(value) = purchase_revenue_in_usd {
                dimensions.insert("purchase_revenue_in_usd".to_string(), value);
            }
            for key in [
                "event_bundle_sequence_id",
                "event_server_timestamp_offset",
                "batch_event_index",
                "table_suffix",
            ] {
                if let Some(value) = row.remove(key) {
                    if !value.trim().is_empty() {
                        dimensions.insert(key.to_string(), value);
                    }
                }
            }
            let mut metrics = BTreeMap::new();
            metrics.insert("eventCount".to_string(), "1".to_string());
            events.push(Ga4EventRawV1 {
                event_name,
                event_timestamp_utc: timestamp,
                event_timestamp_micros: Some(micros),
                user_pseudo_id,
                session_id,
                ga_session_id,
                ga_session_number,
                campaign,
                device_category,
                platform,
                country,
                source_medium,
                traffic_source_source: source,
                traffic_source_medium: medium,
                page_location,
                session_engaged,
                engagement_time_msec,
                transaction_id,
                purchase_revenue: purchase_revenue_value,
                purchase_revenue_in_usd: purchase_revenue_in_usd_value,
                event_bundle_sequence_id,
                batch_event_index,
                dimensions,
                metrics,
            });
        }
        Ok(events)
    }

    fn parse_date_hour_rfc3339(&self, value: &str, timezone: &str) -> Option<String> {
        let trimmed = value.trim();
        if trimmed.len() != 10 {
            return None;
        }
        let day = chrono::NaiveDate::parse_from_str(&trimmed[0..8], "%Y%m%d").ok()?;
        let hour: u32 = trimmed[8..10].parse().ok()?;
        let naive = day.and_hms_opt(hour, 0, 0)?;
        let tz_name = timezone.trim();
        if tz_name.is_empty() || tz_name.eq_ignore_ascii_case("utc") {
            return Some(
                chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).to_rfc3339(),
            );
        }
        let tz: Tz = match tz_name.parse() {
            Ok(value) => value,
            Err(_) => {
                return Some(
                    chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).to_rfc3339(),
                )
            }
        };
        let local = tz
            .from_local_datetime(&naive)
            .single()
            .or_else(|| tz.from_local_datetime(&naive).earliest())?;
        Some(local.with_timezone(&Utc).to_rfc3339())
    }

    fn map_ga4_rows(
        &self,
        config: &AnalyticsConnectorConfigV1,
        response: &Ga4RunReportResponse,
    ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError> {
        let named_rows = self.rows_to_named_maps(response);
        let mut events = Vec::with_capacity(named_rows.len());
        for (idx, mut row) in named_rows.into_iter().enumerate() {
            let event_name = row
                .dimensions
                .get("eventName")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    let available_dimensions =
                        row.dimensions.keys().cloned().collect::<Vec<String>>();
                    AnalyticsError::new(
                        "analytics_ga4_schema_mismatch",
                        "GA4 runReport row did not include required 'eventName' dimension",
                        vec!["ga4.property_id".to_string()],
                        Some(json!({
                            "row_index": idx,
                            "available_dimensions": available_dimensions
                        })),
                    )
                })?;
            let date_hour = row
                .dimensions
                .get("dateHour")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    let available_dimensions =
                        row.dimensions.keys().cloned().collect::<Vec<String>>();
                    AnalyticsError::new(
                        "analytics_ga4_schema_mismatch",
                        "GA4 runReport row did not include required 'dateHour' dimension",
                        vec!["ga4.property_id".to_string()],
                        Some(json!({
                            "row_index": idx,
                            "available_dimensions": available_dimensions
                        })),
                    )
                })?;
            let timestamp = self
                .parse_date_hour_rfc3339(&date_hour, config.ga4.timezone.trim())
                .ok_or_else(|| {
                    AnalyticsError::new(
                        "analytics_ga4_schema_mismatch",
                        "GA4 runReport row had an invalid dateHour value for configured timezone",
                        vec!["ga4.timezone".to_string(), "ga4.property_id".to_string()],
                        Some(json!({
                            "row_index": idx,
                            "date_hour": date_hour,
                            "timezone": config.ga4.timezone,
                        })),
                    )
                })?;
            let campaign = row
                .dimensions
                .get("campaignName")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let device_category = row
                .dimensions
                .get("deviceCategory")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let source_medium = row
                .dimensions
                .get("sessionSourceMedium")
                .or_else(|| row.dimensions.get("sourceMedium"))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let user = format!("ga4_row_{}_{}_{}", idx, date_hour, event_name);
            let event_count = row
                .metrics
                .get("eventCount")
                .and_then(|metric| metric.trim().parse::<u64>().ok())
                .filter(|count| *count > 0)
                .unwrap_or(1);
            row.dimensions
                .entry("ga4_read_backend".to_string())
                .or_insert_with(|| "data_api_run_report".to_string());
            events.push(Ga4EventRawV1 {
                event_name,
                event_timestamp_utc: timestamp,
                event_timestamp_micros: None,
                user_pseudo_id: user,
                session_id: Some(format!("ga4_count:{}", event_count)),
                ga_session_id: None,
                ga_session_number: None,
                campaign,
                device_category,
                platform: None,
                country: None,
                source_medium,
                traffic_source_source: None,
                traffic_source_medium: None,
                page_location: None,
                session_engaged: None,
                engagement_time_msec: None,
                transaction_id: None,
                purchase_revenue: None,
                purchase_revenue_in_usd: None,
                event_bundle_sequence_id: None,
                batch_event_index: None,
                dimensions: row.dimensions,
                metrics: row.metrics,
            });
        }
        Ok(events)
    }
}

#[async_trait]
impl AnalyticsConnectorContractV2 for ObservedReadOnlyAnalyticsConnectorV2 {
    fn capabilities(&self) -> AnalyticsConnectorCapabilitiesV1 {
        AnalyticsConnectorCapabilitiesV1 {
            connector_id: OBSERVED_CONNECTOR_ID_V2.to_string(),
            contract_version: CONNECTOR_CONTRACT_VERSION_V2.to_string(),
            supports_healthcheck: true,
            sources: vec![
                ConnectorSourceCapabilityV1 {
                    source_system: "ga4".to_string(),
                    granularity: vec!["hour".to_string(), "day".to_string()],
                    read_mode: "observed_read_only".to_string(),
                },
                ConnectorSourceCapabilityV1 {
                    source_system: "google_ads".to_string(),
                    granularity: vec!["day".to_string()],
                    read_mode: "not_implemented".to_string(),
                },
                ConnectorSourceCapabilityV1 {
                    source_system: "wix_storefront".to_string(),
                    granularity: vec!["hour".to_string(), "day".to_string()],
                    read_mode: "not_implemented".to_string(),
                },
            ],
        }
    }

    async fn healthcheck(
        &self,
        config: &AnalyticsConnectorConfigV1,
    ) -> Result<ConnectorHealthStatusV1, AnalyticsError> {
        validate_analytics_connector_config_v1(config)?;
        let mut source_status = Vec::new();
        let mut blocking_reasons = Vec::new();
        let mut warning_reasons = Vec::new();

        if !config.ga4.enabled {
            source_status.push(source_health_disabled("ga4"));
        } else {
            let mut status = source_health("ga4", true, &[&config.ga4.read_credentials_env_var]);
            if status.credentials_present {
                let probe_result = match config.ga4.read_backend {
                    Ga4ReadBackendV1::DataApiRunReport => self
                        .run_ga4_report(
                            config,
                            Utc::now().date_naive() - Duration::days(1),
                            Utc::now().date_naive() - Duration::days(1),
                            1,
                        )
                        .await
                        .map(|_| ()),
                    Ga4ReadBackendV1::BigqueryExport => {
                        self.run_bigquery_probe(config).await.map(|_| ())
                    }
                };
                match probe_result {
                    Ok(_) => {
                        status.live_probe_ok = true;
                        status.probe_status = "passed".to_string();
                        let probe_backend = match config.ga4.read_backend {
                            Ga4ReadBackendV1::DataApiRunReport => "ga4_data_api_runreport",
                            Ga4ReadBackendV1::BigqueryExport => "bigquery_export",
                        };
                        status.probe_message =
                            Some(format!("ga4 probe succeeded via {}", probe_backend));
                    }
                    Err(err) => {
                        status.live_probe_ok = false;
                        status.probe_status = "failed".to_string();
                        status.probe_message = Some(format!("{}: {}", err.code, err.message));
                        status
                            .blocking_reasons
                            .push("ga4 live probe failed".to_string());
                        status
                            .warning_reasons
                            .push(format!("ga4 probe details: {}: {}", err.code, err.message));
                    }
                }
            } else {
                status.live_probe_ok = false;
                status.probe_status = "failed".to_string();
                status.probe_message = Some("credentials missing".to_string());
                status
                    .blocking_reasons
                    .push("ga4 credentials missing".to_string());
            }
            blocking_reasons.extend(status.blocking_reasons.iter().cloned());
            warning_reasons.extend(status.warning_reasons.iter().cloned());
            source_status.push(status);
        }

        if config.google_ads.enabled {
            let mut status = source_health("google_ads", true, &[]);
            status.probe_status = "failed".to_string();
            status.live_probe_ok = false;
            status.probe_message =
                Some("google ads observed connector is not implemented".to_string());
            status
                .blocking_reasons
                .push("google_ads observed connector not implemented".to_string());
            blocking_reasons.extend(status.blocking_reasons.iter().cloned());
            source_status.push(status);
        } else {
            source_status.push(source_health_disabled("google_ads"));
        }

        if config.wix.enabled {
            let mut status = source_health("wix_storefront", true, &[]);
            status.probe_status = "failed".to_string();
            status.live_probe_ok = false;
            status.probe_message = Some("wix observed connector is not implemented".to_string());
            status
                .blocking_reasons
                .push("wix_storefront observed connector not implemented".to_string());
            blocking_reasons.extend(status.blocking_reasons.iter().cloned());
            source_status.push(status);
        } else {
            source_status.push(source_health_disabled("wix_storefront"));
        }

        Ok(ConnectorHealthStatusV1 {
            connector_id: self.capabilities().connector_id,
            ok: blocking_reasons.is_empty(),
            mode: "observed_read_only".to_string(),
            source_status,
            blocking_reasons,
            warning_reasons,
        })
    }

    async fn fetch_ga4_events(
        &self,
        config: &AnalyticsConnectorConfigV1,
        start: NaiveDate,
        end: NaiveDate,
        _seed: u64,
    ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError> {
        if !config.ga4.enabled {
            return Err(AnalyticsError::validation(
                "analytics_source_not_enabled",
                "ga4 source is disabled in connector config",
                "ga4.enabled",
            ));
        }
        match config.ga4.read_backend {
            Ga4ReadBackendV1::DataApiRunReport => {
                let response = self.run_ga4_report(config, start, end, 1000).await?;
                self.map_ga4_rows(config, &response)
            }
            Ga4ReadBackendV1::BigqueryExport => {
                self.fetch_ga4_events_bigquery(config, start, end).await
            }
        }
    }

    async fn fetch_google_ads_rows(
        &self,
        config: &AnalyticsConnectorConfigV1,
        _request: &MockAnalyticsRequestV1,
        _start: NaiveDate,
        _end: NaiveDate,
        _seed: u64,
    ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
        if !config.google_ads.enabled {
            return Ok(Vec::new());
        }
        Err(AnalyticsError::new(
            "analytics_google_ads_not_implemented",
            "google ads observed connector is not implemented in this slice",
            vec!["google_ads.enabled".to_string()],
            None,
        ))
    }

    async fn fetch_wix_orders(
        &self,
        config: &AnalyticsConnectorConfigV1,
        _start: NaiveDate,
        _end: NaiveDate,
        _seed: u64,
    ) -> Result<Vec<WixOrderRawV1>, AnalyticsError> {
        if !config.wix.enabled {
            return Ok(Vec::new());
        }
        Err(AnalyticsError::new(
            "analytics_wix_not_implemented",
            "wix observed connector is not implemented in this slice",
            vec!["wix.enabled".to_string()],
            None,
        ))
    }

    async fn fetch_wix_sessions(
        &self,
        config: &AnalyticsConnectorConfigV1,
        _start: NaiveDate,
        _end: NaiveDate,
        _seed: u64,
    ) -> Result<Vec<WixSessionRawV1>, AnalyticsError> {
        if !config.wix.enabled {
            return Ok(Vec::new());
        }
        Err(AnalyticsError::new(
            "analytics_wix_not_implemented",
            "wix observed connector is not implemented in this slice",
            vec!["wix.enabled".to_string()],
            None,
        ))
    }
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
            &[&config.ga4.read_credentials_env_var],
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

fn parse_iso_date(value: &str, field_path: &str) -> Result<NaiveDate, AnalyticsError> {
    let trimmed = value.trim();
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").map_err(|_| {
        AnalyticsError::validation(
            "analytics_ga4_raw_invalid_date",
            format!("{} must be in YYYY-MM-DD format", field_path),
            field_path,
        )
    })
}

fn normalize_field_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn is_valid_ga4_field_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '.' || ch == '-'
        })
}

fn is_valid_bigquery_project_id(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 6 || trimmed.len() > 63 {
        return false;
    }
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap_or_default();
    let last = trimmed.chars().last().unwrap_or_default();
    if !first.is_ascii_lowercase() || !last.is_ascii_alphanumeric() {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_valid_bigquery_dataset_id(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 1024 {
        return false;
    }
    let mut chars = trimmed.chars();
    let first = chars.next().unwrap_or_default();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn micros_to_rfc3339(micros: i64) -> Option<String> {
    let seconds = micros.div_euclid(1_000_000);
    let micros_remainder = micros.rem_euclid(1_000_000) as u32;
    let nanos = micros_remainder.saturating_mul(1000);
    chrono::DateTime::<Utc>::from_timestamp(seconds, nanos).map(|dt| dt.to_rfc3339())
}

fn source_health(
    source_system: &str,
    enabled: bool,
    required_env_vars: &[&str],
) -> ConnectorSourceHealthV1 {
    if !enabled {
        return source_health_disabled(source_system);
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
            live_probe_ok: false,
            probe_status: "not_run".to_string(),
            probe_message: None,
            blocking_reasons: Vec::new(),
            warning_reasons: Vec::new(),
        }
    } else {
        ConnectorSourceHealthV1 {
            source_system: source_system.to_string(),
            enabled,
            credentials_present: false,
            live_probe_ok: false,
            probe_status: "failed".to_string(),
            probe_message: Some("credentials missing".to_string()),
            blocking_reasons: Vec::new(),
            warning_reasons: vec![format!("missing env vars: {}", missing.join(", "))],
        }
    }
}

fn source_health_disabled(source_system: &str) -> ConnectorSourceHealthV1 {
    ConnectorSourceHealthV1 {
        source_system: source_system.to_string(),
        enabled: false,
        credentials_present: false,
        live_probe_ok: false,
        probe_status: "not_applicable".to_string(),
        probe_message: Some("source disabled in connector config".to_string()),
        blocking_reasons: Vec::new(),
        warning_reasons: vec!["source disabled in connector config".to_string()],
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
            event_timestamp_micros: None,
            user_pseudo_id: format!(" user_{}_{} ", seed % 1000, current.ordinal()),
            session_id: Some(format!("sess_{}_{}", seed, current.ordinal())),
            ga_session_id: Some((seed as i64 % 10_000) + current.ordinal() as i64),
            ga_session_number: Some(1),
            campaign: Some("spring_launch".to_string()),
            device_category: Some("mobile".to_string()),
            platform: Some("WEB".to_string()),
            country: Some("United States".to_string()),
            source_medium: Some("google / cpc".to_string()),
            traffic_source_source: Some("google".to_string()),
            traffic_source_medium: Some("cpc".to_string()),
            page_location: Some(
                "https://naturesdietpet.com/simply-raw-freeze-dried-raw-meals".to_string(),
            ),
            session_engaged: Some(true),
            engagement_time_msec: Some(1200),
            transaction_id: None,
            purchase_revenue: None,
            purchase_revenue_in_usd: None,
            event_bundle_sequence_id: None,
            batch_event_index: None,
            dimensions: BTreeMap::new(),
            metrics: BTreeMap::new(),
            ..Default::default()
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
    use httptest::matchers::*;
    use httptest::responders::*;
    use httptest::{Expectation, Server};
    use once_cell::sync::Lazy;
    use std::future::Future;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::process::Command;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    const TEST_RSA_PRIVATE_KEY_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIICXQIBAAKBgQDagNZuyR4U9oAuEOrvMnVfGGoP2zU9cLt1BmsPmiGBW8nvCKSQ\np0Mua3uKe5gZPKfs24t6eyng7rXGtmuLvD6ayVh9VXUf3uz2xCY3EhkGaKml0Sh8\nYU9XW64e23ogSncA3du0oiqiieeLuEs2wEaKmoN8yltYZ4Iu7oh2k4JaXQIDAQAB\nAoGBALYoqpv5duarCfldiT6Yplj9FY7ahOwPy3eoPiDnsf8R8qsgXXFqwAs29+tf\nVlHTy3sfHIyjmSo4V7qt4cLA0L7Xuw2vTT3nsX3sgoA5NBS7Vdq8wduVPYe583oq\ndh+Ldi4SLmeaFjXpXo+ZEL1THfG11yXP2a57mQ14aFcliXmBAkEA7Y6o06pwnpbO\nDEwJsQ6g1KoMAN0dJ6ei21DfWlFcrAiE93FaZJSBBFzjliO+GNBpqJ0Acupb0iGw\nkJ2VRy/ilQJBAOt3fSg/FjxFtuy9pe4v1a2WlBXE4E6fV24TWNDxikdsDx5XCicb\nJP2tU0oEbk8h/bEavZtPLvmv/jz6m1yzLqkCQE877P2kdKnAvPsHBZiDu4sTKKvF\nFGtck4o5IDY8uv86XDc4HKE9kwbEgLhcNZSLNyKhMzwhBP1CdWTW2qqCwz0CQAjS\n+YXAl3y6wBgvI0DB2igfNH18W0uW/RfK8dEivCPhEM/6Qw8kHUbEcBKeB+Q/SdqR\nPfnMBd6lkcmHOrtGm8ECQQDgjyzmmuCo8GcpPUD/IrQ4RwlSoeMklPky6OSQaBpG\n+HpcTZVHFcNR78zbjYFuLbE4c2aTyQbyHle98zo27BdA\n-----END RSA PRIVATE KEY-----\n";

    fn with_temp_env<F>(pairs: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        let previous = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in pairs {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        f();
        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }

    async fn with_temp_env_async<F, Fut>(pairs: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let previous = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        f().await;

        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }

    fn test_config_observed_ga4_only() -> AnalyticsConnectorConfigV1 {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        cfg.ga4.read_credentials_env_var = "TEST_GA4_CREDENTIALS_PATH".to_string();
        cfg
    }

    fn openssl_available() -> bool {
        Command::new("openssl")
            .arg("version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn maybe_spawn_test_server() -> Option<Server> {
        catch_unwind(AssertUnwindSafe(Server::run)).ok()
    }

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
        cfg.ga4.read_credentials_env_var = "GA4_MISSING_CREDENTIALS".to_string();
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

    #[tokio::test]
    async fn observed_healthcheck_reports_missing_ga4_credentials() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(&[("TEST_GA4_CREDENTIALS_PATH", None)], || {});
        let connector = ObservedReadOnlyAnalyticsConnectorV2::new();
        let config = test_config_observed_ga4_only();
        let status = connector.healthcheck(&config).await.expect("healthcheck");
        assert!(!status.ok);
        assert!(status
            .blocking_reasons
            .iter()
            .any(|item| item.contains("ga4 credentials missing")));
    }

    #[tokio::test]
    async fn observed_healthcheck_blocks_ads_and_wix_when_enabled() {
        if !openssl_available() {
            return;
        }
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        let dir = tempdir().expect("tempdir");
        let creds_path = dir.path().join("sa.json");
        std::fs::write(
            &creds_path,
            format!(
                r#"{{"client_email":"test@example.iam.gserviceaccount.com","private_key":"{}","token_uri":"{}"}}"#,
                TEST_RSA_PRIVATE_KEY_PEM.replace('\n', "\\n"),
                "http://localhost:9/token"
            ),
        )
        .expect("write creds");
        let mut config = test_config_observed_ga4_only();
        config.google_ads.enabled = true;
        config.wix.enabled = true;
        with_temp_env_async(
            &[(
                "TEST_GA4_CREDENTIALS_PATH",
                Some(creds_path.to_string_lossy().as_ref()),
            )],
            || async {
                let connector = ObservedReadOnlyAnalyticsConnectorV2::new();
                let status = connector.healthcheck(&config).await.expect("healthcheck");
                assert!(!status.ok);
                assert!(status
                    .blocking_reasons
                    .iter()
                    .any(|item| item.contains("google_ads observed connector not implemented")));
                assert!(
                    status
                        .blocking_reasons
                        .iter()
                        .any(|item| item
                            .contains("wix_storefront observed connector not implemented"))
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn observed_healthcheck_fails_on_token_exchange_error() {
        if !openssl_available() {
            return;
        }
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        let Some(server) = maybe_spawn_test_server() else {
            return;
        };
        server.expect(
            Expectation::matching(request::method_path("POST", "/token"))
                .respond_with(status_code(401).body("unauthorized")),
        );
        let dir = tempdir().expect("tempdir");
        let creds_path = dir.path().join("sa.json");
        std::fs::write(
            &creds_path,
            format!(
                r#"{{"client_email":"test@example.iam.gserviceaccount.com","private_key":"{}","token_uri":"{}"}}"#,
                TEST_RSA_PRIVATE_KEY_PEM.replace('\n', "\\n"),
                server.url_str("/token")
            ),
        )
        .expect("write creds");

        let config = test_config_observed_ga4_only();
        with_temp_env_async(
            &[(
                "TEST_GA4_CREDENTIALS_PATH",
                Some(creds_path.to_string_lossy().as_ref()),
            )],
            || async {
                let connector = ObservedReadOnlyAnalyticsConnectorV2::new();
                let status = connector.healthcheck(&config).await.expect("healthcheck");
                assert!(!status.ok);
                assert!(status
                    .warning_reasons
                    .iter()
                    .any(|item| item.contains("analytics_ga4_token_exchange_failed")));
            },
        )
        .await;
    }

    #[tokio::test]
    async fn observed_healthcheck_succeeds_when_probe_succeeds() {
        if !openssl_available() {
            return;
        }
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        let Some(server) = maybe_spawn_test_server() else {
            return;
        };
        server.expect(
            Expectation::matching(request::method_path("POST", "/token")).respond_with(
                json_encoded(serde_json::json!({
                    "access_token": "test-access-token",
                    "token_type": "Bearer",
                    "expires_in": 3600
                })),
            ),
        );
        server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/v1beta/properties/123456789:runReport",
            ))
            .respond_with(json_encoded(serde_json::json!({
                "rows": [{
                    "dimensionValues": [
                        {"value": "purchase"},
                        {"value": "2026020112"},
                        {"value": "spring_launch"}
                    ],
                    "metricValues": [{"value": "1"}]
                }]
            }))),
        );
        let dir = tempdir().expect("tempdir");
        let creds_path = dir.path().join("sa.json");
        std::fs::write(
            &creds_path,
            format!(
                r#"{{"client_email":"test@example.iam.gserviceaccount.com","private_key":"{}","token_uri":"{}"}}"#,
                TEST_RSA_PRIVATE_KEY_PEM.replace('\n', "\\n"),
                server.url_str("/token")
            ),
        )
        .expect("write creds");
        let config = test_config_observed_ga4_only();
        with_temp_env_async(
            &[
                (
                    "TEST_GA4_CREDENTIALS_PATH",
                    Some(creds_path.to_string_lossy().as_ref()),
                ),
                (
                    "ANALYTICS_GA4_DATA_API_BASE_URL",
                    Some(server.url_str("/v1beta").as_str()),
                ),
            ],
            || async {
                let connector = ObservedReadOnlyAnalyticsConnectorV2::new();
                let status = connector.healthcheck(&config).await.expect("healthcheck");
                assert!(status.ok);
                let ga4 = status
                    .source_status
                    .iter()
                    .find(|item| item.source_system == "ga4")
                    .expect("ga4 status");
                assert!(ga4.live_probe_ok);
                assert_eq!(ga4.probe_status, "passed");
            },
        )
        .await;
    }
}
