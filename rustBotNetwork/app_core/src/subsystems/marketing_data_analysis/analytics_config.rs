use super::contracts::AnalyticsError;
use chrono_tz::Tz;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

static ENV_VAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Z][A-Z0-9_]*$").expect("env var regex must compile"));
static GA4_PROPERTY_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{6,16}$").expect("ga4 property regex must compile"));
static GOOGLE_ADS_CUSTOMER_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[0-9]{10}$").expect("google ads customer regex must compile"));
static WIX_SITE_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9_-]{4,128}$").expect("wix site regex must compile"));

pub const CONNECTOR_CONFIG_FINGERPRINT_ALG_V1: &str = "sha256";
pub const CONNECTOR_CONFIG_FINGERPRINT_SCHEMA_V1: &str = "connector-config-v1";

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

#[derive(Debug, Serialize)]
struct ConnectorFingerprintInputV1<'a> {
    profile_id: &'a str,
    mode: &'a AnalyticsConnectorModeV1,
    default_timezone: &'a str,
    ga4: ConnectorFingerprintGa4V1<'a>,
    google_ads: ConnectorFingerprintGoogleAdsV1<'a>,
    wix: ConnectorFingerprintWixV1<'a>,
}

#[derive(Debug, Serialize)]
struct ConnectorFingerprintGa4V1<'a> {
    enabled: bool,
    property_id: &'a str,
    api_secret_env_var: &'a str,
    measurement_id_env_var: &'a str,
    timezone: &'a str,
}

#[derive(Debug, Serialize)]
struct ConnectorFingerprintGoogleAdsV1<'a> {
    enabled: bool,
    customer_id: &'a str,
    login_customer_id: Option<&'a str>,
    developer_token_env_var: &'a str,
    oauth_client_id_env_var: &'a str,
    oauth_client_secret_env_var: &'a str,
    oauth_refresh_token_env_var: &'a str,
    timezone: &'a str,
}

#[derive(Debug, Serialize)]
struct ConnectorFingerprintWixV1<'a> {
    enabled: bool,
    site_id: &'a str,
    account_id: Option<&'a str>,
    api_token_env_var: &'a str,
    timezone: &'a str,
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
/// purpose: Resolve analytics connector config from environment variables with typed defaults.
/// invariants:
///   - Falls back to deterministic simulated-safe defaults when optional env vars are absent.
///   - Returned config is validated before returning to callers.
pub fn analytics_connector_config_from_env() -> Result<AnalyticsConnectorConfigV1, AnalyticsError> {
    let defaults = AnalyticsConnectorConfigV1::simulated_defaults();
    let mode = match env_or_default("ANALYTICS_CONNECTOR_MODE", "simulated")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "simulated" => AnalyticsConnectorModeV1::Simulated,
        "observed_read_only" => AnalyticsConnectorModeV1::ObservedReadOnly,
        _ => {
            return Err(AnalyticsError::validation(
                "analytics_config_mode_invalid",
                "ANALYTICS_CONNECTOR_MODE must be one of: simulated, observed_read_only",
                "mode",
            ));
        }
    };

    let config = AnalyticsConnectorConfigV1 {
        profile_id: env_or_default("ANALYTICS_PROFILE_ID", &defaults.profile_id),
        mode,
        default_timezone: env_or_default("ANALYTICS_DEFAULT_TIMEZONE", &defaults.default_timezone),
        ga4: Ga4ConfigV1 {
            enabled: env_bool_or_default("ANALYTICS_ENABLE_GA4", defaults.ga4.enabled)?,
            property_id: env_or_default("GA4_PROPERTY_ID", &defaults.ga4.property_id),
            api_secret_env_var: env_or_default(
                "ANALYTICS_GA4_API_SECRET_ENV_VAR",
                "GA4_API_SECRET",
            ),
            measurement_id_env_var: env_or_default(
                "ANALYTICS_GA4_MEASUREMENT_ID_ENV_VAR",
                "GA4_MEASUREMENT_ID",
            ),
            timezone: env_or_default("ANALYTICS_GA4_TIMEZONE", &defaults.ga4.timezone),
        },
        google_ads: GoogleAdsConfigV1 {
            enabled: env_bool_or_default(
                "ANALYTICS_ENABLE_GOOGLE_ADS",
                defaults.google_ads.enabled,
            )?,
            customer_id: env_or_default("GOOGLE_ADS_CUSTOMER_ID", &defaults.google_ads.customer_id),
            login_customer_id: env_opt("GOOGLE_ADS_LOGIN_CUSTOMER_ID"),
            developer_token_env_var: env_or_default(
                "ANALYTICS_GOOGLE_ADS_DEVELOPER_TOKEN_ENV_VAR",
                "GOOGLE_ADS_DEVELOPER_TOKEN",
            ),
            oauth_client_id_env_var: env_or_default(
                "ANALYTICS_GOOGLE_ADS_OAUTH_CLIENT_ID_ENV_VAR",
                "GOOGLE_ADS_OAUTH_CLIENT_ID",
            ),
            oauth_client_secret_env_var: env_or_default(
                "ANALYTICS_GOOGLE_ADS_OAUTH_CLIENT_SECRET_ENV_VAR",
                "GOOGLE_ADS_OAUTH_CLIENT_SECRET",
            ),
            oauth_refresh_token_env_var: env_or_default(
                "ANALYTICS_GOOGLE_ADS_OAUTH_REFRESH_TOKEN_ENV_VAR",
                "GOOGLE_ADS_OAUTH_REFRESH_TOKEN",
            ),
            timezone: env_or_default(
                "ANALYTICS_GOOGLE_ADS_TIMEZONE",
                &defaults.google_ads.timezone,
            ),
        },
        wix: WixConfigV1 {
            enabled: env_bool_or_default("ANALYTICS_ENABLE_WIX", defaults.wix.enabled)?,
            site_id: env_or_default("WIX_SITE_ID", &defaults.wix.site_id),
            account_id: env_opt("WIX_ACCOUNT_ID"),
            api_token_env_var: env_or_default("ANALYTICS_WIX_API_TOKEN_ENV_VAR", "WIX_API_TOKEN"),
            timezone: env_or_default("ANALYTICS_WIX_TIMEZONE", &defaults.wix.timezone),
        },
    };

    validate_analytics_connector_config_v1(&config)?;
    Ok(config)
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::analytics_config`
/// purpose: Build deterministic connector config fingerprint for runtime attestation metadata.
/// invariants:
///   - Uses only non-secret, schema-versioned config fields.
///   - Produces stable output for semantically identical config values.
pub fn analytics_connector_config_fingerprint_v1(
    config: &AnalyticsConnectorConfigV1,
) -> Result<String, AnalyticsError> {
    let canonical = ConnectorFingerprintInputV1 {
        profile_id: config.profile_id.trim(),
        mode: &config.mode,
        default_timezone: config.default_timezone.trim(),
        ga4: ConnectorFingerprintGa4V1 {
            enabled: config.ga4.enabled,
            property_id: config.ga4.property_id.trim(),
            api_secret_env_var: config.ga4.api_secret_env_var.trim(),
            measurement_id_env_var: config.ga4.measurement_id_env_var.trim(),
            timezone: config.ga4.timezone.trim(),
        },
        google_ads: ConnectorFingerprintGoogleAdsV1 {
            enabled: config.google_ads.enabled,
            customer_id: config.google_ads.customer_id.trim(),
            login_customer_id: config
                .google_ads
                .login_customer_id
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty()),
            developer_token_env_var: config.google_ads.developer_token_env_var.trim(),
            oauth_client_id_env_var: config.google_ads.oauth_client_id_env_var.trim(),
            oauth_client_secret_env_var: config.google_ads.oauth_client_secret_env_var.trim(),
            oauth_refresh_token_env_var: config.google_ads.oauth_refresh_token_env_var.trim(),
            timezone: config.google_ads.timezone.trim(),
        },
        wix: ConnectorFingerprintWixV1 {
            enabled: config.wix.enabled,
            site_id: config.wix.site_id.trim(),
            account_id: config
                .wix
                .account_id
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty()),
            api_token_env_var: config.wix.api_token_env_var.trim(),
            timezone: config.wix.timezone.trim(),
        },
    };

    let canonical_bytes = serde_json::to_vec(&canonical).map_err(|err| {
        AnalyticsError::internal(
            "analytics_config_fingerprint_serialize_failed",
            format!("failed to serialize fingerprint input: {err}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(canonical_bytes);
    let digest = hasher.finalize();
    Ok(format!(
        "{}:{}",
        CONNECTOR_CONFIG_FINGERPRINT_ALG_V1,
        hex_lower(&digest)
    ))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_bool_or_default(key: &str, default: bool) -> Result<bool, AnalyticsError> {
    let Some(raw) = std::env::var(key).ok() else {
        return Ok(default);
    };
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(AnalyticsError::validation(
            "analytics_config_bool_invalid",
            format!("{key} must be a boolean-like value (true/false/1/0/yes/no)"),
            key.to_ascii_lowercase(),
        )),
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
    use std::sync::Mutex;

    static ENV_MUTEX: once_cell::sync::Lazy<Mutex<()>> =
        once_cell::sync::Lazy::new(|| Mutex::new(()));

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

    #[test]
    fn from_env_parses_observed_mode_and_feature_flags() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(
            &[
                ("ANALYTICS_CONNECTOR_MODE", Some("observed_read_only")),
                ("ANALYTICS_ENABLE_WIX", Some("false")),
                ("ANALYTICS_PROFILE_ID", Some("ops_profile")),
                ("ANALYTICS_DEFAULT_TIMEZONE", Some("UTC")),
                ("GA4_PROPERTY_ID", Some("123456789")),
                ("GOOGLE_ADS_CUSTOMER_ID", Some("1234567890")),
                ("WIX_SITE_ID", Some("natures-diet-store")),
            ],
            || {
                let cfg = analytics_connector_config_from_env().expect("env config should parse");
                assert_eq!(cfg.mode, AnalyticsConnectorModeV1::ObservedReadOnly);
                assert!(!cfg.wix.enabled);
                assert_eq!(cfg.profile_id, "ops_profile");
            },
        );
    }

    #[test]
    fn from_env_rejects_invalid_bool() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(&[("ANALYTICS_ENABLE_GA4", Some("not-bool"))], || {
            let err = analytics_connector_config_from_env().expect_err("must fail");
            assert_eq!(err.code, "analytics_config_bool_invalid");
        });
    }

    #[test]
    fn fingerprint_is_stable_for_trim_equivalent_values() {
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        let mut cfg_with_whitespace = cfg.clone();
        cfg_with_whitespace.profile_id = format!(" {} ", cfg.profile_id);
        cfg_with_whitespace.ga4.property_id = format!(" {} ", cfg.ga4.property_id);
        cfg_with_whitespace.google_ads.customer_id = format!(" {} ", cfg.google_ads.customer_id);
        cfg_with_whitespace.wix.site_id = format!(" {} ", cfg.wix.site_id);

        let a = analytics_connector_config_fingerprint_v1(&cfg).expect("fingerprint");
        let b =
            analytics_connector_config_fingerprint_v1(&cfg_with_whitespace).expect("fingerprint");
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_when_relevant_field_changes() {
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        let mut changed = cfg.clone();
        changed.google_ads.customer_id = "9999999999".to_string();

        let a = analytics_connector_config_fingerprint_v1(&cfg).expect("fingerprint");
        let b = analytics_connector_config_fingerprint_v1(&changed).expect("fingerprint");
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_never_includes_secret_values() {
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        let fingerprint = analytics_connector_config_fingerprint_v1(&cfg).expect("fingerprint");
        assert!(
            !fingerprint.contains("token"),
            "digest output must not expose secrets"
        );
        assert!(fingerprint.starts_with("sha256:"));
    }

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
}
