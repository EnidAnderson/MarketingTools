// provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0016

use crate::analytics_data_generator::generate_simulated_google_ads_rows;
use crate::data_models::analytics::{
    AttributionWindowMetadata, ConfidenceAnnotation, Ga4NormalizedEvent, GoogleAdsRow,
    SourceClassLabel, SourceProvenance,
};

pub trait AnalyticsConnectorContract {
    fn fetch_google_ads_rows(&self, start_date: &str, end_date: &str) -> Vec<GoogleAdsRow>;
    fn fetch_ga4_events(&self, start_date: &str, end_date: &str) -> Vec<Ga4NormalizedEvent>;
}

pub struct SimulatedConnectorContract;

impl SimulatedConnectorContract {
    pub fn new() -> Self {
        Self
    }
}

impl AnalyticsConnectorContract for SimulatedConnectorContract {
    fn fetch_google_ads_rows(&self, start_date: &str, end_date: &str) -> Vec<GoogleAdsRow> {
        generate_simulated_google_ads_rows(start_date, end_date)
    }

    fn fetch_ga4_events(&self, _start_date: &str, _end_date: &str) -> Vec<Ga4NormalizedEvent> {
        vec![Ga4NormalizedEvent {
            event_name: "purchase".to_string(),
            event_timestamp_utc: "2026-02-16T00:00:00Z".to_string(),
            session_id: "sess_001".to_string(),
            user_pseudo_id: "user_001".to_string(),
            traffic_source: Some("google".to_string()),
            medium: Some("cpc".to_string()),
            campaign: Some("spring_launch".to_string()),
            revenue_micros: Some(12_500_000),
            provenance: SourceProvenance {
                connector_id: "ga4_connector_v1".to_string(),
                source_class: SourceClassLabel::Observed,
                source_system: "ga4".to_string(),
                collected_at_utc: "2026-02-16T00:05:00Z".to_string(),
                freshness_minutes: 5,
            },
        }]
    }
}

pub fn default_confidence() -> ConfidenceAnnotation {
    ConfidenceAnnotation {
        confidence_label: "medium".to_string(),
        rationale: "Observed connector data with bounded attribution assumptions".to_string(),
        uncertainty_note: "Cross-channel identity reconciliation may reduce precision".to_string(),
    }
}

pub fn default_attribution_window() -> AttributionWindowMetadata {
    AttributionWindowMetadata {
        lookback_days: 7,
        model: "last_non_direct_click".to_string(),
        safeguarded: true,
    }
}
