// rustBotNetwork/app_core/src/analytics_data_generator.rs

use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupResource, AnalyticsReport, CampaignResource, GoogleAdsRow,
    KeywordData, MetricsData, ReportMetrics, SegmentsData,
};
use chrono::{Duration, NaiveDate};
use rand::Rng;
use std::collections::HashMap;

const CAMPAIGN_NAMES: &[&str] = &[
    "Summer Pet Food Promo",
    "New Puppy Essentials",
    "Senior Dog Health",
    "Organic Cat Treats",
    "Winter Warmth Campaign",
];
const AD_GROUP_NAMES: &[&str] = &[
    "Dry Food",
    "Wet Food",
    "Treats",
    "Accessories",
    "Health Supplements",
];
const KEYWORD_TEXTS: &[&str] = &[
    "healthy dog food",
    "grain-free cat food",
    "best puppy treats",
    "senior pet vitamins",
    "organic pet snacks",
    "hypoallergenic dog food",
];
const MATCH_TYPES: &[&str] = &["EXACT", "PHRASE", "BROAD"];

/// Generates simulated Google Ads API-like raw data (Vec<GoogleAdsRow>).
///
/// # Arguments
/// * `start_date_str` - The start date for the simulated data (YYYY-MM-DD).
/// * `end_date_str` - The end date for the simulated data (YYYY-MM-DD).
///
/// # Returns
/// A `Vec<GoogleAdsRow>` containing simulated raw data.
pub fn generate_simulated_google_ads_rows(
    start_date_str: &str,
    end_date_str: &str,
) -> Vec<GoogleAdsRow> {
    let mut rng = rand::thread_rng();
    let mut rows = Vec::new();

    let start_date = NaiveDate::parse_from_str(start_date_str, "%Y-%m-%d").unwrap();
    let end_date = NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d").unwrap();

    let mut current_date = start_date;
    while current_date <= end_date {
        let date_str = current_date.format("%Y-%m-%d").to_string();

        for (campaign_idx, &campaign_name) in CAMPAIGN_NAMES.iter().enumerate() {
            let campaign_id = format!("{}", campaign_idx + 1);
            let campaign_resource_name =
                format!("customers/123456789/campaigns/{}", campaign_id);
            let campaign_status = if rng.gen_bool(0.9) {
                "ENABLED".to_string()
            } else {
                "PAUSED".to_string()
            };

            let campaign_res = CampaignResource {
                resource_name: campaign_resource_name.clone(),
                id: campaign_id.clone(),
                name: campaign_name.to_string(),
                status: campaign_status.clone(),
            };

            for (ad_group_idx, &ad_group_name) in AD_GROUP_NAMES.iter().enumerate() {
                let ad_group_id = format!("{}.{}", campaign_id, ad_group_idx + 1);
                let ad_group_resource_name =
                    format!("customers/123456789/adGroups/{}", ad_group_id);
                let ad_group_status = if rng.gen_bool(0.9) {
                    "ENABLED".to_string()
                } else {
                    "PAUSED".to_string()
                };

                let ad_group_res = AdGroupResource {
                    resource_name: ad_group_resource_name.clone(),
                    id: ad_group_id.clone(),
                    name: ad_group_name.to_string(),
                    status: ad_group_status.clone(),
                    campaign_resource_name: campaign_resource_name.clone(),
                };

                for (keyword_idx, &keyword_text) in KEYWORD_TEXTS.iter().enumerate() {
                    let impressions: u64 = rng.gen_range(500..5000);
                    let clicks: u64 = rng.gen_range(10..impressions / 20);
                    let cost_micros: u64 = (clicks as u64) * rng.gen_range(500_000..2_500_000); // 0.5 to 2.5 currency units
                    let conversions: f64 = rng.gen_range(0.0..clicks as f64 / 50.0);
                    let conversions_value: f64 = conversions * rng.gen_range(20.0..100.0);
                    let quality_score: u32 = rng.gen_range(1..10);

                    let metrics_data = MetricsData {
                        impressions,
                        clicks,
                        cost_micros,
                        conversions,
                        conversions_value,
                        ctr: (clicks as f64 / impressions as f64) * 100.0,
                        average_cpc: cost_micros as f64 / clicks as f64 / 1_000_000.0,
                    };

                    let match_type = MATCH_TYPES[rng.gen_range(0..MATCH_TYPES.len())];
                    let criterion_id = format!("{}", rng.gen_range(10000..99999));

                    let ad_group_criterion = AdGroupCriterionResource {
                        resource_name: format!(
                            "customers/123456789/adGroupCriteria/{}.{}",
                            ad_group_id, criterion_id
                        ),
                        criterion_id: criterion_id.clone(),
                        status: "ENABLED".to_string(),
                        keyword: Some(KeywordData {
                            text: keyword_text.to_string(),
                            match_type: match_type.to_string(),
                        }),
                        quality_score: Some(quality_score),
                        ad_group_resource_name: ad_group_resource_name.clone(),
                    };

                    let segments_data = SegmentsData {
                        date: Some(date_str.clone()),
                        device: Some(if rng.gen_bool(0.7) {
                            "MOBILE".to_string()
                        } else {
                            "DESKTOP".to_string()
                        }),
                    };

                    rows.push(GoogleAdsRow {
                        campaign: Some(campaign_res.clone()),
                        ad_group: Some(ad_group_res.clone()),
                        keyword_view: None, // Not directly querying keyword_view resource
                        ad_group_criterion: Some(ad_group_criterion),
                        metrics: Some(metrics_data),
                        segments: Some(segments_data),
                    });
                }
            }
        }

        current_date = current_date.checked_add_signed(Duration::days(1)).unwrap();
    }
    rows
}

/// Converts raw GoogleAdsRows into the flattened AnalyticsReport format.
///
/// # Arguments
/// * `rows` - A `Vec<GoogleAdsRow>` containing raw Google Ads data.
/// * `report_name` - The name for the generated report.
/// * `date_range` - The date range covered by the report.
///
/// # Returns
/// An `AnalyticsReport` containing processed and aggregated data.
pub fn process_google_ads_rows_to_report(
    rows: Vec<GoogleAdsRow>,
    report_name: &str,
    date_range: &str,
) -> AnalyticsReport {
    let mut total_metrics = ReportMetrics::default();
    let mut campaign_metrics_map: HashMap<String, (String, String, ReportMetrics)> =
        HashMap::new(); // id -> (name, status, metrics)
    let mut ad_group_metrics_map: HashMap<String, (String, String, String, ReportMetrics)> =
        HashMap::new(); // id -> (campaign_id, name, status, metrics)
    let mut keyword_metrics_map: HashMap<String, (String, String, String, String, ReportMetrics, Option<u32>)> =
        HashMap::new(); // id -> (campaign_id, ad_group_id, text, type, metrics, quality_score)

    for row in rows {
        let campaign_id = row
            .campaign
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_default();
        let campaign_name = row
            .campaign
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let campaign_status = row
            .campaign
            .as_ref()
            .map(|c| c.status.clone())
            .unwrap_or_default();

        let ad_group_id = row
            .ad_group
            .as_ref()
            .map(|ag| ag.id.clone())
            .unwrap_or_default();
        let ad_group_name = row
            .ad_group
            .as_ref()
            .map(|ag| ag.name.clone())
            .unwrap_or_default();
        let ad_group_status = row
            .ad_group
            .as_ref()
            .map(|ag| ag.status.clone())
            .unwrap_or_default();

        let keyword_id = row
            .ad_group_criterion
            .as_ref()
            .map(|agc| agc.criterion_id.clone())
            .unwrap_or_default();
        let keyword_text = row
            .ad_group_criterion
            .as_ref()
            .and_then(|agc| agc.keyword.as_ref())
            .map(|kw| kw.text.clone())
            .unwrap_or_default();
        let match_type = row
            .ad_group_criterion
            .as_ref()
            .and_then(|agc| agc.keyword.as_ref())
            .map(|kw| kw.match_type.clone())
            .unwrap_or_default();
        let quality_score = row
            .ad_group_criterion
            .as_ref()
            .and_then(|agc| agc.quality_score);

        if let Some(metrics) = row.metrics {
            let processed_metrics = calculate_report_metrics(
                metrics.impressions,
                metrics.clicks,
                metrics.cost_micros,
                metrics.conversions,
                metrics.conversions_value,
            );

            // Aggregate total metrics
            total_metrics = aggregate_report_metrics(&total_metrics, &processed_metrics);

            // Aggregate campaign metrics
            let (c_name, c_status, c_metrics) = campaign_metrics_map
                .entry(campaign_id.clone())
                .or_insert((campaign_name.clone(), campaign_status.clone(), ReportMetrics::default()));
            *c_name = campaign_name.clone();
            *c_status = campaign_status.clone();
            *c_metrics = aggregate_report_metrics(c_metrics, &processed_metrics);

            // Aggregate ad group metrics
            let (ag_c_id, ag_name, ag_status, ag_metrics) = ad_group_metrics_map
                .entry(ad_group_id.clone())
                .or_insert((campaign_id.clone(), ad_group_name.clone(), ad_group_status.clone(), ReportMetrics::default()));
            *ag_c_id = campaign_id.clone();
            *ag_name = ad_group_name.clone();
            *ag_status = ad_group_status.clone();
            *ag_metrics = aggregate_report_metrics(ag_metrics, &processed_metrics);

            // Aggregate keyword metrics
            let (kw_c_id, kw_ag_id, kw_text, kw_type, kw_metrics, kw_quality_score) = keyword_metrics_map
                .entry(keyword_id.clone())
                .or_insert((campaign_id.clone(), ad_group_id.clone(), keyword_text.clone(), match_type.clone(), ReportMetrics::default(), quality_score));
            *kw_c_id = campaign_id.clone();
            *kw_ag_id = ad_group_id.clone();
            *kw_text = keyword_text.clone();
            *kw_type = match_type.clone();
            *kw_quality_score = quality_score;
            *kw_metrics = aggregate_report_metrics(kw_metrics, &processed_metrics);
        }
    }

    // Convert aggregated maps to Vecs for AnalyticsReport
    let campaign_data: Vec<crate::data_models::analytics::CampaignReportRow> = campaign_metrics_map.iter()
        .map(|(id, (name, status, metrics))| crate::data_models::analytics::CampaignReportRow {
            date: "".to_string(), // Date will be set by analytics_reporter
            campaign_id: id.clone(),
            campaign_name: name.clone(),
            campaign_status: status.clone(),
            metrics: metrics.clone(),
        })
        .collect();

    let ad_group_data: Vec<crate::data_models::analytics::AdGroupReportRow> = ad_group_metrics_map.iter()
        .map(|(id, (campaign_id, name, status, metrics))| {
            let campaign_name = campaign_metrics_map
                .get(campaign_id)
                .map(|(n, _, _)| n.clone())
                .unwrap_or_default();
            crate::data_models::analytics::AdGroupReportRow {
                date: "".to_string(), // Date will be set by analytics_reporter
                campaign_id: campaign_id.clone(),
                campaign_name,
                ad_group_id: id.clone(),
                ad_group_name: name.clone(),
                ad_group_status: status.clone(),
                metrics: metrics.clone(),
            }
        })
        .collect();

    let keyword_data: Vec<crate::data_models::analytics::KeywordReportRow> = keyword_metrics_map.into_iter()
        .map(|(id, (campaign_id, ad_group_id, text, r#type, metrics, quality_score))| {
            let campaign_name = campaign_metrics_map
                .get(&campaign_id)
                .map(|(n, _, _)| n.clone())
                .unwrap_or_default();
            let ad_group_name = ad_group_metrics_map
                .get(&ad_group_id)
                .map(|(_, n, _, _)| n.clone())
                .unwrap_or_default();
            crate::data_models::analytics::KeywordReportRow {
                date: "".to_string(), // Date will be set by analytics_reporter
                campaign_id: campaign_id.clone(),
                campaign_name,
                ad_group_id: ad_group_id.clone(),
                ad_group_name,
                keyword_id: id,
                keyword_text: text,
                match_type: r#type,
                quality_score,
                metrics,
            }
        })
        .collect();

    AnalyticsReport {
        report_name: report_name.to_string(),
        date_range: date_range.to_string(),
        total_metrics,
        campaign_data,
        ad_group_data,
        keyword_data,
    }
}

/// Helper function to aggregate ReportMetrics.
fn aggregate_report_metrics(a: &ReportMetrics, b: &ReportMetrics) -> ReportMetrics {
    let impressions = a.impressions + b.impressions;
    let clicks = a.clicks + b.clicks;
    let cost = a.cost + b.cost;
    let conversions = a.conversions + b.conversions;
    let conversions_value = a.conversions_value + b.conversions_value;

    calculate_report_metrics(impressions, clicks, (cost * 1_000_000.0) as u64, conversions, conversions_value)
}

/// Helper function to calculate derived metrics for ReportMetrics.
fn calculate_report_metrics(
    impressions: u64,
    clicks: u64,
    cost_micros: u64,
    conversions: f64,
    conversions_value: f64,
) -> ReportMetrics {
    let cost = cost_micros as f64 / 1_000_000.0; // Convert micros to actual currency

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
    use approx::assert_relative_eq;
    use crate::data_models::analytics::ReportMetrics; // Use ReportMetrics

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
    fn test_generate_simulated_google_ads_rows_structure() {
        let rows = generate_simulated_google_ads_rows("2023-01-01", "2023-01-01");
        assert!(!rows.is_empty());

        let first_row = rows.first().unwrap();
        assert!(first_row.campaign.is_some());
        assert!(first_row.ad_group.is_some());
        assert!(first_row.ad_group_criterion.is_some());
        assert!(first_row.metrics.is_some());
        assert!(first_row.segments.is_some());

        let metrics = first_row.metrics.as_ref().unwrap();
        assert!(metrics.impressions > 0);
        assert!(metrics.clicks > 0);
        assert!(metrics.cost_micros > 0);
    }

    #[test]
    fn test_process_google_ads_rows_to_report() {
        let rows = generate_simulated_google_ads_rows("2023-01-01", "2023-01-01");
        let report = process_google_ads_rows_to_report(rows.clone(), "Test Report", "2023-01-01");

        assert_eq!(report.report_name, "Test Report");
        assert_eq!(report.date_range, "2023-01-01");
        assert!(!report.campaign_data.is_empty());
        assert!(!report.ad_group_data.is_empty());
        assert!(!report.keyword_data.is_empty());

        // Basic check for aggregation
        let total_impressions_from_campaigns: u64 = report.campaign_data.iter().map(|c| c.metrics.impressions).sum();
        assert_eq!(report.total_metrics.impressions, total_impressions_from_campaigns);
    }

    #[test]
    fn test_calculate_report_metrics() {
        let metrics = calculate_report_metrics(1000, 50, 200_000_000, 5.0, 1000.0); // 200 units cost
        assert_relative_eq!(metrics.ctr, 5.0, epsilon = 0.001);
        assert_relative_eq!(metrics.cpc, 4.0, epsilon = 0.001);
        assert_relative_eq!(metrics.cpa, 40.0, epsilon = 0.001);
        assert_relative_eq!(metrics.roas, 5.0, epsilon = 0.001);
        assert_relative_eq!(metrics.cost, 200.0, epsilon = 0.001);
    }

    #[test]
    fn test_aggregate_report_metrics() {
        let m1 = calculate_report_metrics(100, 10, 50_000_000, 1.0, 100.0); // Cost 50
        let m2 = calculate_report_metrics(200, 20, 100_000_000, 2.0, 200.0); // Cost 100
        let aggregated = aggregate_report_metrics(&m1, &m2);

        assert_eq!(aggregated.impressions, 300);
        assert_eq!(aggregated.clicks, 30);
        assert_relative_eq!(aggregated.cost, 150.0, epsilon = 0.001);
        assert_relative_eq!(aggregated.conversions, 3.0, epsilon = 0.001);
        assert_relative_eq!(aggregated.conversions_value, 300.0, epsilon = 0.001);
        // Derived metrics should be recalculated based on aggregated base metrics
        assert_relative_eq!(aggregated.ctr, (30.0/300.0)*100.0, epsilon = 0.001); // 10%
    }
}