// rustBotNetwork/app_core/src/integration_tests.rs

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::data_models::analytics::{GoogleAdsRow, AnalyticsReport, ReportMetrics};
    use crate::data_models::dashboard::{DashboardConfig, DateRangePreset, WidgetConfig, WidgetType};
    use crate::analytics_data_generator::process_google_ads_rows_to_report;
    use crate::analytics_reporter::generate_analytics_report;
    use crate::dashboard_processor::process_dashboard_config;
    use approx::assert_relative_eq;
    use std::fs;

    // Helper to check if two ReportMetrics are approximately equal
    fn assert_report_metrics_approx_eq(m1: &ReportMetrics, m2: &ReportMetrics) {
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
    fn test_end_to_end_analytics_pipeline() {
        // 1. Load mock Google Ads API response data
        let json_data = fs::read_to_string("test_data/mock_google_ads_rows.json")
            .expect("Unable to read mock_google_ads_rows.json");
        let raw_google_ads_rows: Vec<GoogleAdsRow> =
            serde_json::from_str(&json_data).expect("Unable to deserialize mock JSON");

        // 2. Simulate generating an AnalyticsReport (bypassing generate_simulated_google_ads_rows
        // because we are loading specific mock data)
        // We'll effectively call process_google_ads_rows_to_report directly
        let processed_report = process_google_ads_rows_to_report(
            raw_google_ads_rows.clone(),
            "Integration Test Report",
            "2023-01-01 to 2023-01-02",
        );

        // Basic assertions on the processed report
        assert_eq!(processed_report.campaign_data.len(), 2); // Campaign 1 and 2
        assert_eq!(processed_report.ad_group_data.len(), 2); // AdGroup 1.1 and 2.1
        assert_eq!(processed_report.keyword_data.len(), 3); // Healthy dog food, grain-free cat food, best puppy treats

        // Verify total metrics from processed report (manually calculate expected values from mock data)
        let expected_total_impressions = 1000 + 800 + 2500 + 1100 + 2600; // sum of all impressions
        let expected_total_clicks = 100 + 50 + 200 + 120 + 210; // sum of all clicks
        let expected_total_cost_micros = 50000000 + 30000000 + 100000000 + 60000000 + 110000000;
        let expected_total_conversions = 5.0 + 2.0 + 10.0 + 6.0 + 11.0;
        let expected_total_conversions_value = 250.0 + 100.0 + 750.0 + 300.0 + 800.0;

        let expected_total_cost = expected_total_cost_micros as f64 / 1_000_000.0;
        let expected_total_ctr = if expected_total_impressions > 0 {
            (expected_total_clicks as f64 / expected_total_impressions as f64) * 100.0
        } else {
            0.0
        };
        let expected_total_cpc = if expected_total_clicks > 0 {
            expected_total_cost / expected_total_clicks as f64
        } else {
            0.0
        };
        let expected_total_cpa = if expected_total_conversions > 0.0 {
            expected_total_cost / expected_total_conversions
        } else {
            0.0
        };
        let expected_total_roas = if expected_total_cost > 0.0 {
            expected_total_conversions_value / expected_total_cost
        } else {
            0.0
        };

        let expected_total_metrics = ReportMetrics {
            impressions: expected_total_impressions,
            clicks: expected_total_clicks,
            cost: expected_total_cost,
            conversions: expected_total_conversions,
            conversions_value: expected_total_conversions_value,
            ctr: expected_total_ctr,
            cpc: expected_total_cpc,
            cpa: expected_total_cpa,
            roas: expected_total_roas,
        };
        assert_report_metrics_approx_eq(&processed_report.total_metrics, &expected_total_metrics);

        // 3. Define a sample DashboardConfig
        let dashboard_config = DashboardConfig {
            dashboard_name: "Integration Dashboard".to_string(),
            description: Some("Test E2E pipeline".to_string()),
            date_range_preset: None,
            start_date: Some("2023-01-01".to_string()),
            end_date: Some("2023-01-02".to_string()),
            filters: Some(
                [
                    ("campaign_name".to_string(), "Summer Pet Food Promo".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
            widgets: vec![
                WidgetConfig {
                    id: "campaign_clicks_bar".to_string(),
                    r#type: WidgetType::Bar,
                    title: "Campaign Clicks".to_string(),
                    data_source: "campaign_data".to_string(),
                    metrics: vec!["clicks".to_string()],
                    dimension: Some("campaign_name".to_string()),
                    limit: Some(1),
                    sort_by: Some("clicks".to_string()),
                    sort_order: Some("desc".to_string()),
                    chart_options: None,
                },
                WidgetConfig {
                    id: "total_conversions_summary".to_string(),
                    r#type: WidgetType::Summary,
                    title: "Total Conversions".to_string(),
                    data_source: "total_metrics".to_string(),
                    metrics: vec!["conversions".to_string()],
                    dimension: None, limit: None, sort_by: None, sort_order: None, chart_options: None,
                },
            ],
        };

        // 4. Process the dashboard config
        let render_data_result = process_dashboard_config(&dashboard_config);
        assert!(render_data_result.is_ok());
        let render_data = render_data_result.unwrap();

        assert_eq!(render_data.dashboard_name, "Integration Dashboard");
        assert_eq!(render_data.widgets.len(), 2);

        // Verify bar chart widget
        let bar_chart_widget = render_data.widgets.iter().find(|w| w.widget_id == "campaign_clicks_bar").unwrap();
        assert_eq!(bar_chart_widget.r#type, WidgetType::Bar);
        assert!(bar_chart_widget.chart_data.is_some());
        let chart_data = bar_chart_widget.chart_data.as_ref().unwrap();
        assert_eq!(chart_data.labels, vec!["Summer Pet Food Promo"]); // Filtered and limited
        assert_eq!(chart_data.datasets[0].data.len(), 1);
        // Clicks for "Summer Pet Food Promo" = 100 (Jan 1) + 120 (Jan 2) = 220
        assert_relative_eq!(chart_data.datasets[0].data[0], 220.0);

        // Verify summary widget
        let summary_widget = render_data.widgets.iter().find(|w| w.widget_id == "total_conversions_summary").unwrap();
        assert_eq!(summary_widget.r#type, WidgetType::Summary);
        assert!(summary_widget.summary_data.is_some());
        let summary_data = summary_widget.summary_data.as_ref().unwrap();
        // Conversions for "Summer Pet Food Promo" = 5.0 (Jan 1) + 6.0 (Jan 2) = 11.0
        assert_relative_eq!(summary_data["conversions"].as_f64().unwrap(), 11.0);
    }
}