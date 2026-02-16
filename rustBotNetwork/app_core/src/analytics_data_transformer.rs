// rustBotNetwork/app_core/src/analytics_data_transformer.rs

use crate::data_models::analytics::{AnalyticsReport, ReportMetrics};
use crate::data_models::dashboard::{ChartDataset, ChartData, WidgetConfig, WidgetRenderData, WidgetType};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Transforms raw AnalyticsReport data into a format suitable for a specific widget.
pub fn transform_data_for_widget(
    report: &AnalyticsReport,
    widget_config: &WidgetConfig,
) -> WidgetRenderData {
    let mut render_data = WidgetRenderData {
        widget_id: widget_config.id.clone(),
        r#type: widget_config.r#type.clone(),
        title: widget_config.title.clone(),
        chart_data: None,
        table_data: None,
        summary_data: None,
        chart_options: widget_config.chart_options.clone(),
    };

    match widget_config.r#type {
        WidgetType::Bar | WidgetType::Line | WidgetType::Pie | WidgetType::Doughnut => {
            render_data.chart_data = Some(prepare_chart_data(report, widget_config));
        }
        WidgetType::Table => {
            render_data.table_data = Some(prepare_table_data(report, widget_config));
        }
        WidgetType::Summary => {
            render_data.summary_data = Some(prepare_summary_data(report, widget_config));
        }
    }

    render_data
}

fn prepare_chart_data(report: &AnalyticsReport, widget_config: &WidgetConfig) -> ChartData {
    let mut labels = Vec::new();
    let mut datasets = Vec::new();

    // Determine data source
    let (data_entries, _dimension_key) = match widget_config.data_source.as_str() {
        "campaign_data" => (
            report.campaign_data.iter().map(|c| (c.campaign_name.clone(), c.metrics.clone())).collect::<Vec<_>>(),
            "campaign_name"
        ),
        "ad_group_data" => (
            report.ad_group_data.iter().map(|ag| (ag.ad_group_name.clone(), ag.metrics.clone())).collect::<Vec<_>>(),
            "ad_group_name"
        ),
        "keyword_data" => (
            report.keyword_data.iter().map(|kw| (kw.keyword_text.clone(), kw.metrics.clone())).collect::<Vec<_>>(),
            "keyword_text"
        ),
        _ => (Vec::new(), ""), // total_metrics handled separately if needed for a single point chart
    };

    // Aggregate by dimension if specified and not empty
    let mut aggregated_data: HashMap<String, ReportMetrics> = HashMap::new();
    if let Some(_dim) = &widget_config.dimension {
        if !data_entries.is_empty() {
            for (name, metrics) in data_entries.iter() {
                // This assumes name is the dimension value
                *aggregated_data.entry(name.clone()).or_default() = aggregate_metrics(
                    aggregated_data.get(name).unwrap_or(&ReportMetrics::default()),
                    metrics,
                );
            }
        }
    } else {
        // Fallback for single data point charts (e.g., total metrics pie chart)
        // For simplicity, if no dimension is given, we'll just use total metrics
        aggregated_data.insert("Total".to_string(), report.total_metrics.clone());
    }

    // Sort and limit data
    let mut sorted_entries: Vec<(String, ReportMetrics)> = aggregated_data.into_iter().collect();
    if let Some(sort_by_metric) = &widget_config.sort_by {
        sorted_entries.sort_by(|a, b| {
            let metric_a = get_metric_value(&a.1, sort_by_metric);
            let metric_b = get_metric_value(&b.1, sort_by_metric);
            if widget_config.sort_order.as_deref() == Some("asc") {
                metric_a.partial_cmp(&metric_b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                metric_b.partial_cmp(&metric_a).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
    }

    let limit_count = widget_config.limit.unwrap_or(sorted_entries.len() as u32) as usize;
    let limited_entries: Vec<(String, ReportMetrics)> = sorted_entries
        .into_iter()
        .take(limit_count)
        .collect();

    // Prepare labels and datasets
    labels = limited_entries.iter().map(|(name, _metrics): &(String, ReportMetrics)| name.clone()).collect();

    for metric_name in &widget_config.metrics {
        let data: Vec<f64> = limited_entries.iter().map(|(_, metrics)| {
            get_metric_value(metrics, metric_name)
        }).collect();

        datasets.push(ChartDataset {
            label: metric_name.clone(),
            data,
            options: None, // Chart.js specific options can be passed through chart_options at widget level
        });
    }

    ChartData { labels, datasets }
}

fn prepare_table_data(report: &AnalyticsReport, widget_config: &WidgetConfig) -> Vec<HashMap<String, Value>> {
    // Determine data source
    let (data_entries, _) = match widget_config.data_source.as_str() {
        "campaign_data" => (
            report.campaign_data.iter().map(|c| {
                let mut map = HashMap::new();
                map.insert("date".to_string(), json!(c.date.clone()));
                map.insert("campaign_id".to_string(), json!(c.campaign_id.clone()));
                map.insert("campaign_name".to_string(), json!(c.campaign_name.clone()));
                map.insert("campaign_status".to_string(), json!(c.campaign_status.clone()));
                map.extend(metrics_to_json_map(&c.metrics));
                map
            }).collect::<Vec<_>>(), ""
        ),
        "ad_group_data" => (
            report.ad_group_data.iter().map(|ag| {
                let mut map = HashMap::new();
                map.insert("date".to_string(), json!(ag.date.clone()));
                map.insert("campaign_id".to_string(), json!(ag.campaign_id.clone()));
                map.insert("campaign_name".to_string(), json!(ag.campaign_name.clone()));
                map.insert("ad_group_id".to_string(), json!(ag.ad_group_id.clone()));
                map.insert("ad_group_name".to_string(), json!(ag.ad_group_name.clone()));
                map.insert("ad_group_status".to_string(), json!(ag.ad_group_status.clone()));
                map.extend(metrics_to_json_map(&ag.metrics));
                map
            }).collect::<Vec<_>>(), ""
        ),
        "keyword_data" => (
            report.keyword_data.iter().map(|kw| {
                let mut map = HashMap::new();
                map.insert("date".to_string(), json!(kw.date.clone()));
                map.insert("campaign_id".to_string(), json!(kw.campaign_id.clone()));
                map.insert("campaign_name".to_string(), json!(kw.campaign_name.clone()));
                map.insert("ad_group_id".to_string(), json!(kw.ad_group_id.clone()));
                map.insert("ad_group_name".to_string(), json!(kw.ad_group_name.clone()));
                map.insert("keyword_id".to_string(), json!(kw.keyword_id.clone()));
                map.insert("keyword_text".to_string(), json!(kw.keyword_text.clone()));
                map.insert("match_type".to_string(), json!(kw.match_type.clone()));
                map.insert("quality_score".to_string(), json!(kw.quality_score));
                map.extend(metrics_to_json_map(&kw.metrics));
                map
            }).collect::<Vec<_>>(), ""
        ),
        "total_metrics" => (
            vec![metrics_to_json_map(&report.total_metrics)], ""
        ),
        _ => (Vec::new(), ""),
    };

    // Apply sorting if specified
    // Note: Complex sorting by multiple metrics or nested fields will require more advanced logic.
    // For simplicity, we assume metrics are directly accessible in the map.
    let mut sortable_rows = data_entries;
    if let Some(sort_by_metric) = &widget_config.sort_by {
        sortable_rows.sort_by(|a, b| {
            let metric_a = a.get(sort_by_metric).and_then(Value::as_f64).unwrap_or_default();
            let metric_b = b.get(sort_by_metric).and_then(Value::as_f64).unwrap_or_default();
            if widget_config.sort_order.as_deref() == Some("asc") {
                metric_a.partial_cmp(&metric_b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                metric_b.partial_cmp(&metric_a).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
    }

    // Apply limit
    let limit_count = widget_config.limit.unwrap_or(sortable_rows.len() as u32) as usize;
    sortable_rows.into_iter().take(limit_count).collect()
}

fn prepare_summary_data(report: &AnalyticsReport, widget_config: &WidgetConfig) -> HashMap<String, Value> {
    let mut summary_map = HashMap::new();
    
    // For summary, we primarily focus on total_metrics
    let metrics = &report.total_metrics;

    for metric_name in &widget_config.metrics {
        let value = get_metric_value(metrics, metric_name);
        summary_map.insert(metric_name.clone(), json!(value));
    }

    summary_map
}

fn get_metric_value(metrics: &ReportMetrics, metric_name: &str) -> f64 {
    match metric_name {
        "impressions" => metrics.impressions as f64,
        "clicks" => metrics.clicks as f64,
        "cost" => metrics.cost,
        "conversions" => metrics.conversions,
        "conversions_value" => metrics.conversions_value,
        "ctr" => metrics.ctr,
        "cpc" => metrics.cpc,
        "cpa" => metrics.cpa,
        "roas" => metrics.roas,
        _ => 0.0,
    }
}

fn aggregate_metrics(a: &ReportMetrics, b: &ReportMetrics) -> ReportMetrics {
    let impressions = a.impressions + b.impressions;
    let clicks = a.clicks + b.clicks;
    let cost = a.cost + b.cost;
    let conversions = a.conversions + b.conversions;
    let conversions_value = a.conversions_value + b.conversions_value;

    calculate_derived_metrics(impressions, clicks, cost, conversions, conversions_value)
}

fn metrics_to_json_map(metrics: &ReportMetrics) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert("impressions".to_string(), json!(metrics.impressions));
    map.insert("clicks".to_string(), json!(metrics.clicks));
    map.insert("cost".to_string(), json!(metrics.cost));
    map.insert("conversions".to_string(), json!(metrics.conversions));
    map.insert("conversions_value".to_string(), json!(metrics.conversions_value));
    map.insert("ctr".to_string(), json!(metrics.ctr));
    map.insert("cpc".to_string(), json!(metrics.cpc));
    map.insert("cpa".to_string(), json!(metrics.cpa));
    map.insert("roas".to_string(), json!(metrics.roas));
    map
}

// Helper function to calculate derived metrics, moved here for clarity
// In a real app, this would likely be part of the GoogleAdsMetrics impl or a separate utility.
fn calculate_derived_metrics(
    impressions: u64,
    clicks: u64,
    cost: f64,
    conversions: f64,
    conversions_value: f64,
) -> ReportMetrics {
    let ctr = if impressions > 0 {
        (clicks as f64 / impressions as f64) * 100.0
    } else {
        0.0
    };
    let cpc = if clicks > 0 {
        cost / clicks as f64
    } else {
        0.0
    };
    let cpa = if conversions > 0.0 {
        cost / conversions
    } else {
        0.0
    };
    let roas = if cost > 0.0 {
        conversions_value / cost
    } else {
        0.0
    };

    ReportMetrics {
        impressions,
        clicks,
        cost,
        conversions,
        conversions_value,
        ctr,
        cpc,
        cpa,
        roas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_models::analytics::{CampaignReportRow, ReportMetrics};
    use crate::data_models::dashboard::{WidgetConfig, WidgetType};
    use approx::assert_relative_eq;

    fn create_mock_analytics_report() -> AnalyticsReport {
        let metrics1 = ReportMetrics {
            impressions: 1000, clicks: 100, cost: 50.0, conversions: 5.0, conversions_value: 250.0,
            ctr: 10.0, cpc: 0.5, cpa: 10.0, roas: 5.0,
        };
        let metrics2 = ReportMetrics {
            impressions: 2000, clicks: 150, cost: 75.0, conversions: 10.0, conversions_value: 500.0,
            ctr: 7.5, cpc: 0.5, cpa: 7.5, roas: 6.66,
        };
        let metrics3 = ReportMetrics {
            impressions: 500, clicks: 20, cost: 20.0, conversions: 1.0, conversions_value: 100.0,
            ctr: 4.0, cpc: 1.0, cpa: 20.0, roas: 5.0,
        };

        let campaign1 = CampaignReportRow {
            date: "2023-01-01".to_string(),
            campaign_id: "cmp-1".to_string(),
            campaign_name: "Campaign A".to_string(),
            campaign_status: "ENABLED".to_string(),
            metrics: metrics1,
        };
        let campaign2 = CampaignReportRow {
            date: "2023-01-01".to_string(),
            campaign_id: "cmp-2".to_string(),
            campaign_name: "Campaign B".to_string(),
            campaign_status: "ENABLED".to_string(),
            metrics: metrics2,
        };
        let campaign3 = CampaignReportRow {
            date: "2023-01-01".to_string(),
            campaign_id: "cmp-3".to_string(),
            campaign_name: "Campaign C".to_string(),
            campaign_status: "PAUSED".to_string(),
            metrics: metrics3,
        };

        AnalyticsReport {
            report_name: "Mock Report".to_string(),
            date_range: "2023-01-01 to 2023-01-01".to_string(),
            total_metrics: ReportMetrics {
                impressions: 3500, clicks: 270, cost: 145.0, conversions: 16.0, conversions_value: 850.0,
                ctr: (270.0/3500.0)*100.0, cpc: 145.0/270.0, cpa: 145.0/16.0, roas: 850.0/145.0,
            },
            campaign_data: vec![campaign1, campaign2, campaign3],
            ad_group_data: Vec::new(), // Not populating for simplicity in these tests
            keyword_data: Vec::new(), // Not populating for simplicity in these tests
        }
    }

    #[test]
    fn test_transform_bar_chart_data() {
        let report = create_mock_analytics_report();
        let widget_config = WidgetConfig {
            id: "test_bar_chart".to_string(),
            r#type: WidgetType::Bar,
            title: "Clicks by Campaign".to_string(),
            data_source: "campaign_data".to_string(),
            metrics: vec!["clicks".to_string()],
            dimension: Some("campaign_name".to_string()),
            limit: Some(2),
            sort_by: Some("clicks".to_string()),
            sort_order: Some("desc".to_string()),
            chart_options: None,
        };

        let render_data = transform_data_for_widget(&report, &widget_config);

        assert_eq!(render_data.widget_id, "test_bar_chart");
        assert!(render_data.chart_data.is_some());
        let chart_data = render_data.chart_data.unwrap();
        
        // Expect Campaign B (150 clicks), Campaign A (100 clicks) due to limit 2 and sort desc
        assert_eq!(chart_data.labels, vec!["Campaign B", "Campaign A"]);
        assert_eq!(chart_data.datasets.len(), 1);
        assert_eq!(chart_data.datasets[0].label, "clicks");
        assert_relative_eq!(chart_data.datasets[0].data[0], 150.0);
        assert_relative_eq!(chart_data.datasets[0].data[1], 100.0);
    }

    #[test]
    fn test_transform_summary_data() {
        let report = create_mock_analytics_report();
        let widget_config = WidgetConfig {
            id: "test_summary".to_string(),
            r#type: WidgetType::Summary,
            title: "Overall Summary".to_string(),
            data_source: "total_metrics".to_string(),
            metrics: vec!["impressions".to_string(), "roas".to_string()],
            dimension: None, limit: None, sort_by: None, sort_order: None, chart_options: None,
        };

        let render_data = transform_data_for_widget(&report, &widget_config);

        assert!(render_data.summary_data.is_some());
        let summary_data = render_data.summary_data.unwrap();
        assert_relative_eq!(summary_data["impressions"].as_f64().unwrap(), report.total_metrics.impressions as f64);
        assert_relative_eq!(summary_data["roas"].as_f64().unwrap(), report.total_metrics.roas);
    }

    #[test]
    fn test_transform_table_data_with_sort_limit() {
        let report = create_mock_analytics_report();
        let widget_config = WidgetConfig {
            id: "test_table".to_string(),
            r#type: WidgetType::Table,
            title: "Campaign Performance Table".to_string(),
            data_source: "campaign_data".to_string(),
            metrics: vec![], // All metrics will be flattened
            dimension: None,
            limit: Some(2),
            sort_by: Some("cost".to_string()),
            sort_order: Some("asc".to_string()),
            chart_options: None,
        };

        let render_data = transform_data_for_widget(&report, &widget_config);

        assert!(render_data.table_data.is_some());
        let table_data = render_data.table_data.unwrap();
        assert_eq!(table_data.len(), 2);
        
        // Expect Campaign C (20.0 cost), Campaign A (50.0 cost) due to limit 2 and sort asc
        assert_eq!(table_data[0]["campaign_name"], "Campaign C");
        assert_eq!(table_data[1]["campaign_name"], "Campaign A");
        assert_relative_eq!(table_data[0]["cost"].as_f64().unwrap(), 20.0);
    }

    #[test]
    fn test_transform_empty_report() {
        let empty_report = AnalyticsReport::default();
        let widget_config = WidgetConfig {
            id: "test_empty".to_string(),
            r#type: WidgetType::Bar,
            title: "Empty Chart".to_string(),
            data_source: "campaign_data".to_string(),
            metrics: vec!["clicks".to_string()],
            dimension: Some("campaign_name".to_string()),
            limit: None, sort_by: None, sort_order: None, chart_options: None,
        };

        let render_data = transform_data_for_widget(&empty_report, &widget_config);
        assert!(render_data.chart_data.is_some());
        assert!(render_data.chart_data.unwrap().labels.is_empty());
    }
}