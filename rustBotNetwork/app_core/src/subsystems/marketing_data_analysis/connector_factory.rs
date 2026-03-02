use super::analytics_config::{AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1};
use super::connector_v2::{
    AnalyticsConnectorContractV2, ObservedReadOnlyAnalyticsConnectorV2,
    SimulatedAnalyticsConnectorV2,
};
use std::sync::Arc;

/// # NDOC
/// component: `subsystems::marketing_data_analysis::connector_factory`
/// purpose: Resolve analytics connector implementation from validated runtime config.
/// invariants:
///   - `simulated` mode always uses `SimulatedAnalyticsConnectorV2`.
///   - `observed_read_only` mode always uses `ObservedReadOnlyAnalyticsConnectorV2`.
pub fn build_analytics_connector_v2(
    config: &AnalyticsConnectorConfigV1,
) -> Arc<dyn AnalyticsConnectorContractV2> {
    match config.mode {
        AnalyticsConnectorModeV1::Simulated => Arc::new(SimulatedAnalyticsConnectorV2::new()),
        AnalyticsConnectorModeV1::ObservedReadOnly => {
            Arc::new(ObservedReadOnlyAnalyticsConnectorV2::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_selects_observed_connector_for_observed_mode() {
        let mut cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = AnalyticsConnectorModeV1::ObservedReadOnly;
        let connector = build_analytics_connector_v2(&cfg);
        let connector_id = connector.capabilities().connector_id;
        assert_eq!(connector_id, "analytics_observed_read_only_connector_v2");
    }

    #[test]
    fn factory_selects_simulated_connector_for_simulated_mode() {
        let cfg = AnalyticsConnectorConfigV1::simulated_defaults();
        let connector = build_analytics_connector_v2(&cfg);
        let connector_id = connector.capabilities().connector_id;
        assert_eq!(connector_id, "mock_analytics_connector_v2");
    }
}
