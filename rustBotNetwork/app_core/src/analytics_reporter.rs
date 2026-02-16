// rustBotNetwork/app_core/src/analytics_reporter.rs
// provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0016; change_request_id=CR-WHITE-0017

use crate::analytics_connector_contracts::{
    default_attribution_window, default_confidence, AnalyticsConnectorContract,
    AttributionWindowMetadata, ConfidenceAnnotation, Ga4NormalizedEvent, NormalizedKpiNarrative,
    SourceClassLabel, SourceProvenance, TrustedAnalyticsReportArtifact,
};
use crate::data_models::analytics::{
    AnalyticsReport, GoogleAdsRow, ReportMetrics, // Only import these from data_models::analytics
};
use crate::analytics_data_generator::{
    generate_simulated_google_ads_rows, process_google_ads_rows_to_report,
};
use regex::Regex;

/// Generates a comprehensive Google Ads Analytics report.
///
/// # Arguments
/// * `start_date` - The start date for the report (YYYY-MM-DD).
/// * `end_date` - The end date for the report (YYYY-MM-DD).
/// * `campaign_filter` - Optional, filter by campaign name.
/// * `ad_group_filter` - Optional, filter by ad group name.
///
/// # Returns
/// An `AnalyticsReport` containing processed data.
pub fn generate_analytics_report(
    start_date: &str,
    end_date: &str,
    campaign_filter: Option<&str>,
    ad_group_filter: Option<&str>,
) -> AnalyticsReport {
    // 1. Generate simulated raw Google Ads Rows
    let mut raw_rows = generate_simulated_google_ads_rows(start_date, end_date);

    // 2. Apply filters to the raw rows
    if let Some(cf) = campaign_filter {
        raw_rows.retain(|row| {
            row.campaign
                .as_ref()
                .map_or(false, |c| c.name.contains(cf))
        });
    }
    if let Some(agf) = ad_group_filter {
        raw_rows.retain(|row| {
            row.adGroup // Use camelCase field name
                .as_ref()
                .map_or(false, |ag| ag.name.contains(agf))
        });
    }

    // 3. Process filtered raw rows into the AnalyticsReport format
    let report_name = format!("Google Ads Analytics Report: {} to {}", start_date, end_date);
    let date_range = format!("{} to {}", start_date, end_date);

    process_google_ads_rows_to_report(raw_rows, &report_name, &date_range)
}

pub fn generate_typed_trusted_report(
    connector: &dyn AnalyticsConnectorContract,
    start_date: &str,
    end_date: &str,
) -> TrustedAnalyticsReportArtifact {
    let ads_rows = connector.fetch_google_ads_rows(start_date, end_date);
    let ga4_events = connector.fetch_ga4_events(start_date, end_date);
    let report_name = format!("Observed Analytics Report: {} to {}", start_date, end_date);
    let date_range = format!("{} to {}", start_date, end_date);
    let report = process_google_ads_rows_to_report(ads_rows, &report_name, &date_range);

    let narratives = ga4_events
        .iter()
        .map(build_kpi_narrative)
        .collect::<Vec<_>>();
    let provenance = ga4_events.into_iter().map(|e| e.provenance).collect::<Vec<_>>();

    TrustedAnalyticsReportArtifact {
        report,
        narratives,
        provenance,
    }
}

fn build_kpi_narrative(event: &Ga4NormalizedEvent) -> NormalizedKpiNarrative {
    NormalizedKpiNarrative {
        section_id: format!("kpi_{}", event.eventName), // Use camelCase
        text: format!(
            "Observed '{}' event from {} with {} confidence.",
            event.eventName, event.provenance.sourceSystem, default_confidence().confidenceLabel // Use camelCase
        ),
        source_class: event.provenance.sourceClass.clone(), // Use camelCase
        confidence: default_confidence(),
        attribution_window: default_attribution_window(),
    }
}

pub fn validate_schema_drift(required_fields: &[&str], payload_fields: &[&str]) -> Result<(), String> {
    for required in required_fields {
        if !payload_fields.iter().any(|present| present == required) {
            return Err(format!("schema_drift_missing_field={}", required));
        }
    }
    Ok(())
}

pub fn detect_identity_mismatch(ga4_user_id: &str, ads_identity_key: &str) -> bool {
    ga4_user_id.trim() != ads_identity_key.trim()
}

pub fn validate_attribution_window_safeguard(meta: &AttributionWindowMetadata) -> Result<(), String> {
    if meta.lookbackDays == 0 { // Use camelCase
        return Err("invalid_attribution_window_zero_days".to_string());
    }
    if !meta.safeguarded {
        return Err("attribution_window_not_safeguarded".to_string());
    }
    Ok(())
}

pub fn validate_kpi_narratives(
    narratives: &[NormalizedKpiNarrative],
) -> Result<(), String> {
    let causal_verbs = Regex::new(r"(?i)\b(caused|proved|guaranteed|definitely drove)\b")
        .map_err(|e| e.to_string())?;

    for narrative in narratives {
        if matches!(narrative.sourceClass, SourceClassLabel::Simulated) // Use camelCase
            && narrative.confidence.confidenceLabel.eq_ignore_ascii_case("high") // Use camelCase
        {
            return Err(format!(
                "source_class_confidence_violation={}",
                narrative.sectionId // Use camelCase
            ));
        }
        if causal_verbs.is_match(&narrative.text)
            && narrative.confidence.uncertaintyNote.trim().is_empty() // Use camelCase
        {
            return Err(format!(
                "causal_guard_missing_uncertainty={}",
                narrative.sectionId // Use camelCase
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::analytics_connector_contracts::SimulatedConnectorContract;
    use crate::data_models::analytics::ReportMetrics;

    // Helper to check if two ReportMetrics are approximately equal
    fn assert_metrics_approx_eq(m1: &ReportMetrics, m2: &ReportMetrics) {
        assert_eq!(m1.impressions, m2.impressions);
        assert_eq!(m1.clicks, m2.clicks);
        assert_relative_eq!(m1.cost, m2.cost, epsilon = 0.001);
        assert_relative_eq!(m1.conversions, m2.conversions, epsilon = 0.001);
        assert_relative_eq!(m1.conversions_value, m2.conversions_value, epsilon = 0.001);
        assert_relative_eq!(m1.ctr, m2.ctr, epsilon = 0.001);
        assert_relative_eq!(m1.cpc, m2.cpc, epsilon = 0.001);
        assert_relative_eq!(m1.cpa, m2.cpa, epsilon = 0.001);
        assert_relative_eq!(m1.roas, m2.roas, epsilon = 0.001);
    }

    #[test]
    fn test_generate_analytics_report_no_filters() {
        let report = generate_analytics_report("2023-01-01", "2023-01-01", None, None);

        assert!(!report.campaign_data.is_empty());
        assert!(!report.adGroup_data.is_empty()); // Use camelCase field name
        assert!(!report.keyword_data.is_empty());
        assert!(report.total_metrics.impressions > 0);
        assert_eq!(
            report.report_name,
            "Google Ads Analytics Report: 2023-01-01 to 2023-01-01"
        );
    }

    #[test]
    fn test_generate_analytics_report_campaign_filter() {
        let report = generate_analytics_report("2023-01-01", "2023-01-01", Some("Summer"), None);

        assert!(!report.campaign_data.is_empty());
        assert!(report
            .campaign_data
            .iter()
            .all(|c| c.campaign_name.contains("Summer")));

        assert!(!report.adGroup_data.is_empty()); // Use camelCase field name
        assert!(report
            .adGroup_data
            .iter()
            .all(|ag| ag.campaign_name.contains("Summer")));

        assert!(!report.keyword_data.is_empty());
        assert!(report
            .keyword_data
            .iter()
            .all(|kw| kw.campaign_name.contains("Summer")));

        // Verify total metrics recalculation (this is now implicitly tested by
        // process_google_ads_rows_to_report, but we can do a sanity check)
        let total_impressions: u64 = report.campaign_data.iter().map(|c| c.metrics.impressions).sum();
        assert_eq!(report.total_metrics.impressions, total_impressions);
        assert!(report.total_metrics.impressions > 0); // Ensure filtered data is not empty
    }

    #[test]
    fn test_generate_analytics_report_ad_group_filter() {
        let report =
            generate_analytics_report("2023-01-01", "2023-01-01", None, Some("Dry Food"));

        assert!(!report.campaign_data.is_empty()); // Campaigns might still be there, but their metrics may be 0 if all their adgroups were filtered out.
        assert!(!report.adGroup_data.is_empty()); // Use camelCase field name
        assert!(report
            .adGroup_data
            .iter()
            .all(|ag| ag.ad_group_name.contains("Dry Food")));

        assert!(!report.keyword_data.is_empty());
        assert!(report
            .keyword_data
            .iter()
            .all(|kw| kw.ad_group_name.contains("Dry Food")));
        
        assert!(report.total_metrics.impressions > 0); // Ensure filtered data is not empty
    }

    #[test]
    fn test_generate_analytics_report_no_matching_filter() {
        let report = generate_analytics_report(
            "2023-01-01",
            "2023-01-01",
            Some("NonExistentCampaign"),
            None,
        );

        assert!(report.campaign_data.is_empty());
        assert!(report.adGroup_data.is_empty()); // Use camelCase field name
        assert!(report.keyword_data.is_empty());
        assert_metrics_approx_eq(&report.total_metrics, &ReportMetrics::default());
    }

    #[test]
    fn test_generate_typed_trusted_report_includes_provenance() {
        let connector = SimulatedConnectorContract::new();
        let artifact = generate_typed_trusted_report(&connector, "2026-02-01", "2026-02-02");
        assert!(!artifact.provenance.is_empty());
        assert!(!artifact.narratives.is_empty());
        assert_eq!(artifact.provenance[0].sourceSystem, "ga4"); // Use camelCase
    }

    #[test]
    fn test_validate_schema_drift_detects_missing_field() {
        let err = validate_schema_drift(&["eventName", "sessionId"], &["eventName"]) // Use camelCase
            .expect_err("missing field should fail");
        assert!(err.contains("schema_drift_missing_field=sessionId")); // Use camelCase
    }

    #[test]
    fn test_detect_identity_mismatch() {
        assert!(detect_identity_mismatch("user_a", "user_b"));
        assert!(!detect_identity_mismatch("user_a", "user_a"));
    }

    #[test]
    fn test_validate_attribution_window_safeguard() {
        let ok = AttributionWindowMetadata {
            lookbackDays: 7, // Use camelCase
            model: "last_non_direct_click".to_string(),
            safeguarded: true,
        };
        assert!(validate_attribution_window_safeguard(&ok).is_ok());

        let bad = AttributionWindowMetadata {
            lookbackDays: 0, // Use camelCase
            model: "last_non_direct_click".to_string(),
            safeguarded: false,
        };
        assert!(validate_attribution_window_safeguard(&bad).is_err());
    }

    #[test]
    fn test_validate_kpi_narratives_source_class_and_causal_guards() {
        let mut narratives = vec![NormalizedKpiNarrative {
            sectionId: "kpi_1".to_string(), // Use camelCase
            text: "Observed conversion increase.".to_string(),
            sourceClass: SourceClassLabel::Observed, // Use camelCase
            confidence: ConfidenceAnnotation {
                confidenceLabel: "medium".to_string(), // Use camelCase
                rationale: "observed".to_string(),
                uncertaintyNote: "subject to attribution limits".to_string(), // Use camelCase
            },
            attributionWindow: AttributionWindowMetadata { // Use camelCase
                lookbackDays: 7, // Use camelCase
                model: "last_non_direct_click".to_string(),
                safeguarded: true,
            },
        }];
        assert!(validate_kpi_narratives(&narratives).is_ok());

        narratives[0].sourceClass = SourceClassLabel::Simulated; // Use camelCase
        narratives[0].confidence.confidenceLabel = "high".to_string(); // Use camelCase
        assert!(validate_kpi_narratives(&narratives).is_err());
    }
}