use super::analytics_config::{
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
};
use super::connector_v2::AnalyticsConnectorContractV2;
use serde::{Deserialize, Serialize};

pub const ANALYTICS_PREFLIGHT_SCHEMA_VERSION_V1: &str = "analytics_connector_preflight.v1";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::preflight`
/// purpose: Source-level readiness signal emitted by connector preflight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsPreflightSourceStatusV1 {
    pub source_system: String,
    pub enabled: bool,
    pub credentials_present: bool,
    pub ready: bool,
    pub blocking_reasons: Vec<String>,
    pub warning_reasons: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::preflight`
/// purpose: Typed preflight summary for analytics connector readiness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsConnectorPreflightResultV1 {
    pub schema_version: String,
    pub connector_id: String,
    pub contract_version: String,
    pub mode: String,
    pub ok: bool,
    pub config_valid: bool,
    pub credentials_present: bool,
    pub source_enablement: Vec<AnalyticsPreflightSourceStatusV1>,
    pub blocking_reasons: Vec<String>,
    pub warning_reasons: Vec<String>,
}

impl AnalyticsConnectorPreflightResultV1 {
    fn new_base(
        connector_id: String,
        contract_version: String,
        mode: &AnalyticsConnectorModeV1,
    ) -> Self {
        Self {
            schema_version: ANALYTICS_PREFLIGHT_SCHEMA_VERSION_V1.to_string(),
            connector_id,
            contract_version,
            mode: match mode {
                AnalyticsConnectorModeV1::Simulated => "simulated".to_string(),
                AnalyticsConnectorModeV1::ObservedReadOnly => "observed_read_only".to_string(),
            },
            ok: false,
            config_valid: false,
            credentials_present: false,
            source_enablement: Vec::new(),
            blocking_reasons: Vec::new(),
            warning_reasons: Vec::new(),
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::preflight`
/// purpose: Evaluate analytics connector readiness without starting a run.
/// invariants:
///   - invalid config returns deterministic structured failures (never panic).
///   - `ok` is true only when config is valid and no blocking reasons are present.
pub async fn evaluate_analytics_connectors_preflight(
    connector: &dyn AnalyticsConnectorContractV2,
    config: &AnalyticsConnectorConfigV1,
) -> AnalyticsConnectorPreflightResultV1 {
    let capabilities = connector.capabilities();
    let mut result = AnalyticsConnectorPreflightResultV1::new_base(
        capabilities.connector_id,
        capabilities.contract_version,
        &config.mode,
    );

    if let Err(err) = validate_analytics_connector_config_v1(config) {
        result.config_valid = false;
        result
            .blocking_reasons
            .push(format!("{}: {}", err.code, err.message));
        result.warning_reasons.extend(
            err.field_paths
                .iter()
                .map(|path| format!("invalid field: {path}")),
        );
        return result;
    }
    result.config_valid = true;

    match connector.healthcheck(config).await {
        Ok(health) => {
            result.blocking_reasons = health.blocking_reasons;
            result.warning_reasons = health.warning_reasons;
            result.source_enablement = health
                .source_status
                .into_iter()
                .map(|source| {
                    let ready = source.enabled
                        && source.credentials_present
                        && source.blocking_reasons.is_empty();
                    AnalyticsPreflightSourceStatusV1 {
                        source_system: source.source_system,
                        enabled: source.enabled,
                        credentials_present: source.credentials_present,
                        ready,
                        blocking_reasons: source.blocking_reasons,
                        warning_reasons: source.warning_reasons,
                    }
                })
                .collect();
            result.credentials_present = result
                .source_enablement
                .iter()
                .filter(|source| source.enabled)
                .all(|source| source.credentials_present);
            result.ok = result.config_valid && result.blocking_reasons.is_empty();
            result
        }
        Err(err) => {
            result.blocking_reasons = vec![format!("{}: {}", err.code, err.message)];
            result.warning_reasons = err
                .field_paths
                .iter()
                .map(|path| format!("connector healthcheck field: {path}"))
                .collect();
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::marketing_data_analysis::{
        AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1, SimulatedAnalyticsConnectorV2,
    };

    #[tokio::test]
    async fn preflight_rejects_invalid_config() {
        let connector = SimulatedAnalyticsConnectorV2::new();
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.ga4.property_id = "abc".to_string();

        let result = evaluate_analytics_connectors_preflight(&connector, &cfg).await;
        assert!(!result.ok);
        assert!(!result.config_valid);
        assert!(!result.blocking_reasons.is_empty());
    }

    #[tokio::test]
    async fn preflight_reports_missing_credentials_in_observed_mode() {
        let connector = SimulatedAnalyticsConnectorV2::new();
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.ga4.api_secret_env_var = "GA4_PREFLIGHT_MISSING_SECRET".to_string();
        cfg.ga4.measurement_id_env_var = "GA4_PREFLIGHT_MISSING_MEASUREMENT".to_string();
        cfg.google_ads.developer_token_env_var =
            "GOOGLE_ADS_PREFLIGHT_MISSING_DEVELOPER".to_string();
        cfg.google_ads.oauth_client_id_env_var =
            "GOOGLE_ADS_PREFLIGHT_MISSING_CLIENT_ID".to_string();
        cfg.google_ads.oauth_client_secret_env_var =
            "GOOGLE_ADS_PREFLIGHT_MISSING_CLIENT_SECRET".to_string();
        cfg.google_ads.oauth_refresh_token_env_var =
            "GOOGLE_ADS_PREFLIGHT_MISSING_REFRESH".to_string();
        cfg.wix.api_token_env_var = "WIX_PREFLIGHT_MISSING_TOKEN".to_string();

        let result = evaluate_analytics_connectors_preflight(&connector, &cfg).await;
        assert!(result.config_valid);
        assert!(!result.ok);
        assert!(!result.blocking_reasons.is_empty());
        assert_eq!(result.mode, "observed_read_only");
    }

    #[tokio::test]
    async fn preflight_is_structurally_valid_for_simulated_defaults() {
        let connector = SimulatedAnalyticsConnectorV2::new();
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();

        let result = evaluate_analytics_connectors_preflight(&connector, &cfg).await;
        assert_eq!(result.schema_version, ANALYTICS_PREFLIGHT_SCHEMA_VERSION_V1);
        assert!(result.config_valid);
        assert_eq!(result.mode, "simulated");
    }
}
