use super::contracts::AnalyticsError;
use chrono_tz::Tz;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static ENV_VAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Z][A-Z0-9_]*$").expect("env var regex must compile"));
static GA4_PROPERTY_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{6,16}$").expect("ga4 property regex must compile"));
static GOOGLE_ADS_CUSTOMER_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{10}$").expect("google ads customer regex must compile"));
static WIX_SITE_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9_-]{4,128}$").expect("wix site regex must compile"));

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Runtime mode for connector orchestration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsConnectorModeV1 {
    Simulated,
    ObservedReadOnly,
}

impl AnalyticsConnectorModeV1 {
    pub fn is_simulated(&self) -> bool {
        matches!(self, Self::Simulated)
    }
}

impl Default for AnalyticsConnectorModeV1 {
    fn default() -> Self {
        Self::Simulated
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: GA4 connector configuration contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ga4ConfigV1 {
    pub enabled: bool,
    pub property_id: String,
    pub api_secret_env_var: String,
    pub measurement_id_env_var: String,
    pub timezone: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Google Ads connector configuration contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleAdsConfigV1 {
    pub enabled: bool,
    pub customer_id: String,
    pub login_customer_id: Option<String>,
    pub developer_token_env_var: String,
    pub oauth_client_id_env_var: String,
    pub oauth_client_secret_env_var: String,
    pub oauth_refresh_token_env_var: String,
    pub timezone: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Wix connector configuration contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WixConfigV1 {
    pub enabled: bool,
    pub site_id: String,
    pub account_id: Option<String>,
    pub api_token_env_var: String,
    pub timezone: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Top-level connector configuration envelope for analytics ingestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsConnectorConfigV1 {
    pub profile_id: String,
    pub mode: AnalyticsConnectorModeV1,
    pub default_timezone: String,
    pub ga4: Ga4ConfigV1,
    pub google_ads: GoogleAdsConfigV1,
    pub wix: WixConfigV1,
}

impl AnalyticsConnectorConfigV1 {
    pub fn simulated_defaults() -> Self {
        Self {
            profile_id: "simulated_default".to_string(),
            mode: AnalyticsConnectorModeV1::Simulated,
            default_timezone: "UTC".to_string(),
            ga4: Ga4ConfigV1 {
                enabled: true,
                property_id: "123456789".to_string(),
                api_secret_env_var: "GA4_API_SECRET".to_string(),
                measurement_id_env_var: "GA4_MEASUREMENT_ID".to_string(),
                timezone: "UTC".to_string(),
            },
            google_ads: GoogleAdsConfigV1 {
                enabled: true,
                customer_id: "1234567890".to_string(),
                login_customer_id: None,
                developer_token_env_var: "GOOGLE_ADS_DEVELOPER_TOKEN".to_string(),
                oauth_client_id_env_var: "GOOGLE_ADS_OAUTH_CLIENT_ID".to_string(),
                oauth_client_secret_env_var: "GOOGLE_ADS_OAUTH_CLIENT_SECRET".to_string(),
                oauth_refresh_token_env_var: "GOOGLE_ADS_OAUTH_REFRESH_TOKEN".to_string(),
                timezone: "UTC".to_string(),
            },
            wix: WixConfigV1 {
                enabled: true,
                site_id: "natures-diet-store".to_string(),
                account_id: None,
                api_token_env_var: "WIX_API_TOKEN".to_string(),
                timezone: "UTC".to_string(),
            },
        }
    }
}

impl Default for AnalyticsConnectorConfigV1 {
    fn default() -> Self {
        Self::simulated_defaults()
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Validate analytics connector configuration before connector usage.
/// invariants:
///   - enabled sources must provide non-empty identifiers.
///   - env var names must be uppercase token-safe names.
///   - timezone values must parse as IANA timezone IDs.
pub fn validate_analytics_connector_config_v1(
    config: &AnalyticsConnectorConfigV1,
) -> Result<(), AnalyticsError> {
    if config.profile_id.trim().is_empty() {
        return Err(AnalyticsError::validation(
            "analytics_config_profile_required",
            "profile_id must be non-empty",
            "profile_id",
        ));
    }

    parse_tz(&config.default_timezone, "default_timezone")?;

    if !config.ga4.enabled && !config.google_ads.enabled && !config.wix.enabled {
        return Err(AnalyticsError::validation(
            "analytics_config_sources_required",
            "at least one analytics source must be enabled",
            "ga4.enabled",
        ));
    }

    validate_ga4(&config.ga4)?;
    validate_google_ads(&config.google_ads)?;
    validate_wix(&config.wix)?;

    Ok(())
}

fn validate_ga4(config: &Ga4ConfigV1) -> Result<(), AnalyticsError> {
    parse_tz(&config.timezone, "ga4.timezone")?;

    if !config.enabled {
        return Ok(());
    }

    if !GA4_PROPERTY_ID_RE.is_match(config.property_id.trim()) {
        return Err(AnalyticsError::validation(
            "analytics_config_ga4_property_invalid",
            "ga4.property_id must be 6-16 digits",
            "ga4.property_id",
        ));
    }

    validate_env_name(&config.api_secret_env_var, "ga4.api_secret_env_var")?;
    validate_env_name(&config.measurement_id_env_var, "ga4.measurement_id_env_var")?;

    Ok(())
}

fn validate_google_ads(config: &GoogleAdsConfigV1) -> Result<(), AnalyticsError> {
    parse_tz(&config.timezone, "google_ads.timezone")?;

    if !config.enabled {
        return Ok(());
    }

    validate_google_ads_id(&config.customer_id, "google_ads.customer_id")?;
    if let Some(login_customer_id) = &config.login_customer_id {
        if !login_customer_id.trim().is_empty() {
            validate_google_ads_id(login_customer_id, "google_ads.login_customer_id")?;
        }
    }

    validate_env_name(
        &config.developer_token_env_var,
        "google_ads.developer_token_env_var",
    )?;
    validate_env_name(
        &config.oauth_client_id_env_var,
        "google_ads.oauth_client_id_env_var",
    )?;
    validate_env_name(
        &config.oauth_client_secret_env_var,
        "google_ads.oauth_client_secret_env_var",
    )?;
    validate_env_name(
        &config.oauth_refresh_token_env_var,
        "google_ads.oauth_refresh_token_env_var",
    )?;

    Ok(())
}

fn validate_wix(config: &WixConfigV1) -> Result<(), AnalyticsError> {
    parse_tz(&config.timezone, "wix.timezone")?;

    if !config.enabled {
        return Ok(());
    }

    if !WIX_SITE_ID_RE.is_match(config.site_id.trim()) {
        return Err(AnalyticsError::validation(
            "analytics_config_wix_site_invalid",
            "wix.site_id must contain only letters, digits, '-' or '_'",
            "wix.site_id",
        ));
    }

    validate_env_name(&config.api_token_env_var, "wix.api_token_env_var")?;

    Ok(())
}

fn validate_google_ads_id(value: &str, field_path: &str) -> Result<(), AnalyticsError> {
    let normalized = value.trim().replace('-', "");
    if GOOGLE_ADS_CUSTOMER_ID_RE.is_match(&normalized) {
        return Ok(());
    }

    Err(AnalyticsError::validation(
        "analytics_config_google_ads_customer_invalid",
        "google_ads customer ids must resolve to 10 digits",
        field_path.to_string(),
    ))
}

fn validate_env_name(value: &str, field_path: &str) -> Result<(), AnalyticsError> {
    let trimmed = value.trim();
    if ENV_VAR_RE.is_match(trimmed) {
        return Ok(());
    }

    Err(AnalyticsError::validation(
        "analytics_config_env_var_invalid",
        "env var names must match ^[A-Z][A-Z0-9_]*$",
        field_path.to_string(),
    ))
}

fn parse_tz(value: &str, field_path: &str) -> Result<Tz, AnalyticsError> {
    value.trim().parse::<Tz>().map_err(|_| {
        AnalyticsError::validation(
            "analytics_config_timezone_invalid",
            "timezone must be a valid IANA timezone",
            field_path.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_defaults_validate() {
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        let result = validate_analytics_connector_config_v1(&cfg);
        assert!(result.is_ok(), "expected valid simulated config");
    }

    #[test]
    fn rejects_empty_sources() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.ga4.enabled = false;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        let err = validate_analytics_connector_config_v1(&cfg).expect_err("must fail");
        assert_eq!(err.code, "analytics_config_sources_required");
    }

    #[test]
    fn rejects_invalid_timezone() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.default_timezone = "Mars/Phobos".to_string();
        let err = validate_analytics_connector_config_v1(&cfg).expect_err("must fail");
        assert_eq!(err.code, "analytics_config_timezone_invalid");
        assert_eq!(err.field_paths, vec!["default_timezone".to_string()]);
    }

    #[test]
    fn rejects_invalid_env_var_name() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.ga4.api_secret_env_var = "ga4-secret".to_string();
        let err = validate_analytics_connector_config_v1(&cfg).expect_err("must fail");
        assert_eq!(err.code, "analytics_config_env_var_invalid");
        assert_eq!(err.field_paths, vec!["ga4.api_secret_env_var".to_string()]);
    }

    #[test]
    fn rejects_invalid_google_ads_customer_id() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.google_ads.customer_id = "abc".to_string();
        let err = validate_analytics_connector_config_v1(&cfg).expect_err("must fail");
        assert_eq!(err.code, "analytics_config_google_ads_customer_invalid");
        assert_eq!(err.field_paths, vec!["google_ads.customer_id".to_string()]);
    }

    #[test]
    fn allows_disabled_source_with_blank_fields() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.wix.enabled = false;
        cfg.wix.site_id.clear();
        cfg.wix.api_token_env_var.clear();
        let result = validate_analytics_connector_config_v1(&cfg);
        assert!(result.is_ok(), "disabled source fields should be ignored");
    }
}
