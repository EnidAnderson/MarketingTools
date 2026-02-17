// rustBotNetwork/app_core/src/dashboard_processor.rs

use crate::analytics_data_transformer::transform_data_for_widget;
use crate::analytics_reporter::generate_analytics_report;
use crate::data_models::dashboard::{DashboardConfig, DashboardRenderData, WidgetRenderData};
use chrono::{Datelike, Duration, Local, NaiveDate}; // Added Datelike
use std::collections::HashMap;

/// Processes a DashboardConfig and generates visualization-ready data.
pub fn process_dashboard_config(config: &DashboardConfig) -> Result<DashboardRenderData, String> {
    // Determine actual start and end dates based on preset or explicit dates
    let (start_date_str, end_date_str) = match &config.date_range_preset {
        Some(preset) => resolve_date_preset(preset)?,
        None => {
            let start = config
                .start_date
                .as_ref()
                .ok_or("start_date is required if no date_range_preset is provided")?
                .as_str();
            let end = config
                .end_date
                .as_ref()
                .ok_or("end_date is required if no date_range_preset is provided")?
                .as_str();
            (start.to_string(), end.to_string())
        }
    };

    // Generate the base AnalyticsReport
    let report = generate_analytics_report(
        &start_date_str,
        &end_date_str,
        config
            .filters
            .as_ref()
            .and_then(|f| f.get("campaign_name"))
            .map(|s| s.as_str()),
        config
            .filters
            .as_ref()
            .and_then(|f| f.get("ad_group_name"))
            .map(|s| s.as_str()),
    );

    let mut widget_render_data = Vec::new();
    for widget_config in &config.widgets {
        let render_data = transform_data_for_widget(&report, widget_config);
        widget_render_data.push(render_data);
    }

    Ok(DashboardRenderData {
        dashboard_name: config.dashboard_name.clone(),
        date_range: format!("{} to {}", start_date_str, end_date_str),
        widgets: widget_render_data,
    })
}

fn resolve_date_preset(
    preset: &crate::data_models::dashboard::DateRangePreset,
) -> Result<(String, String), String> {
    let today = Local::now().date_naive();
    let (start, end) = match preset {
        crate::data_models::dashboard::DateRangePreset::Last7Days => {
            (today - Duration::days(6), today)
        }
        crate::data_models::dashboard::DateRangePreset::Last30Days => {
            (today - Duration::days(29), today)
        }
        crate::data_models::dashboard::DateRangePreset::ThisMonth => {
            let first_day_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .ok_or("Invalid date calculation for this_month")?;
            (first_day_of_month, today)
        }
        crate::data_models::dashboard::DateRangePreset::LastMonth => {
            let first_day_of_this_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .ok_or("Invalid date calculation for last_month")?;
            let last_day_of_last_month = first_day_of_this_month - Duration::days(1);
            let first_day_of_last_month = NaiveDate::from_ymd_opt(
                last_day_of_last_month.year(),
                last_day_of_last_month.month(),
                1,
            )
            .ok_or("Invalid date calculation for last_month")?;
            (first_day_of_last_month, last_day_of_last_month)
        }
        crate::data_models::dashboard::DateRangePreset::ThisYear => {
            let first_day_of_year = NaiveDate::from_ymd_opt(today.year(), 1, 1)
                .ok_or("Invalid date calculation for this_year")?;
            (first_day_of_year, today)
        }
        crate::data_models::dashboard::DateRangePreset::LastYear => {
            let first_day_of_this_year = NaiveDate::from_ymd_opt(today.year(), 1, 1)
                .ok_or("Invalid date calculation for last_year")?;
            let last_day_of_last_year = first_day_of_this_year - Duration::days(1);
            let first_day_of_last_year =
                NaiveDate::from_ymd_opt(last_day_of_last_year.year(), 1, 1)
                    .ok_or("Invalid date calculation for last_year")?;
            (first_day_of_last_year, last_day_of_last_year)
        }
    };
    Ok((
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_models::analytics::{AnalyticsReport, ReportMetrics};
    use crate::data_models::dashboard::{DateRangePreset, WidgetConfig, WidgetType};
    use chrono::Datelike;

    // Helper to create a mock AnalyticsReport
    fn create_mock_analytics_report() -> AnalyticsReport {
        AnalyticsReport {
            report_name: "Mock Report".to_string(),
            date_range: "2023-01-01 to 2023-01-31".to_string(),
            total_metrics: ReportMetrics {
                impressions: 1000,
                clicks: 100,
                cost: 50.0,
                conversions: 5.0,
                conversions_value: 250.0,
                ctr: 10.0,
                cpc: 0.5,
                cpa: 10.0,
                roas: 5.0,
            },
            campaign_data: Vec::new(),
            ad_group_data: Vec::new(),
            keyword_data: Vec::new(),
        }
    }

    #[test]
    fn test_resolve_date_preset_last_7_days() {
        let (start, end) = resolve_date_preset(&DateRangePreset::Last7Days).unwrap();
        let today = Local::now().date_naive();
        let expected_start = (today - Duration::days(6)).format("%Y-%m-%d").to_string();
        let expected_end = today.format("%Y-%m-%d").to_string();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_resolve_date_preset_this_month() {
        let (start, end) = resolve_date_preset(&DateRangePreset::ThisMonth).unwrap();
        let today = Local::now().date_naive();
        let expected_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        let expected_end = today.format("%Y-%m-%d").to_string();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn test_resolve_date_preset_last_month() {
        let (start, end) = resolve_date_preset(&DateRangePreset::LastMonth).unwrap();
        let today = Local::now().date_naive();
        let first_day_of_this_month =
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let expected_last_day_of_last_month = (first_day_of_this_month - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let expected_first_day_of_last_month = NaiveDate::from_ymd_opt(
            (first_day_of_this_month - Duration::days(1)).year(),
            (first_day_of_this_month - Duration::days(1)).month(),
            1,
        )
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
        assert_eq!(start, expected_first_day_of_last_month);
        assert_eq!(end, expected_last_day_of_last_month);
    }

    #[test]
    fn test_process_dashboard_config_valid() {
        let config = DashboardConfig {
            dashboard_name: "Test Dashboard".to_string(),
            description: Some("A test dashboard".to_string()),
            date_range_preset: Some(DateRangePreset::Last7Days),
            start_date: None,
            end_date: None,
            filters: None,
            widgets: vec![WidgetConfig {
                id: "widget1".to_string(),
                r#type: WidgetType::Summary,
                title: "Summary Widget".to_string(),
                data_source: "total_metrics".to_string(),
                metrics: vec!["impressions".to_string(), "clicks".to_string()],
                dimension: None,
                limit: None,
                sort_by: None,
                sort_order: None,
                chart_options: None,
            }],
        };

        // Mock generate_analytics_report and transform_data_for_widget
        // For now, we'll just let them use their internal simulated data for testing the flow
        let render_data = process_dashboard_config(&config).unwrap();

        assert_eq!(render_data.dashboard_name, "Test Dashboard");
        assert!(!render_data.date_range.is_empty());
        assert_eq!(render_data.widgets.len(), 1);
        assert_eq!(render_data.widgets[0].widget_id, "widget1");
    }

    #[test]
    fn test_process_dashboard_config_invalid_date_config() {
        let config = DashboardConfig {
            dashboard_name: "Invalid Dashboard".to_string(),
            description: None,
            date_range_preset: None,
            start_date: None, // Missing start_date when no preset
            end_date: Some("2023-01-31".to_string()),
            filters: None,
            widgets: Vec::new(),
        };

        let result = process_dashboard_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start_date is required"));
    }
}
