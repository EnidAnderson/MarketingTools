// rustBotNetwork/app_core/src/data_models/analytics.rs
// provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0016; change_request_id=CR-WHITE-0017

use serde::{Serialize, Deserialize};

// --- Google Ads API Response Structures (Raw Data) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignResource {
    pub resourceName: String,
    pub id: String,
    pub name: String,
    pub status: String, // e.g., "ENABLED", "PAUSED"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdGroupResource {
    pub resourceName: String,
    pub id: String,
    pub name: String,
    pub status: String, // e.g., "ENABLED", "PAUSED"
    pub campaignResourceName: String, // Reference to campaign resource name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordViewResource {
    pub resourceName: String,
    // Keyword views often don't have an ID directly, but reference criterion
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdGroupCriterionResource {
    pub resourceName: String,
    pub criterionId: String, // ID of the criterion (keyword)
    pub status: String,
    #[serde(default)] // Keep default for optional keyword field
    pub keyword: Option<KeywordData>, // Actual keyword text and match type
    pub qualityScore: Option<u32>, // Quality Score is often here
    pub adGroupResourceName: String, // Reference to ad group resource name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordData {
    pub text: String,
    pub matchType: String, // e.g., "EXACT", "PHRASE", "BROAD"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    pub clicks: u64,
    pub impressions: u64,
    pub costMicros: u64,
    pub conversions: f64,
    pub conversionsValue: f64,
    pub ctr: f64, // Click-through rate
    pub averageCpc: f64, // Average Cost-per-click
    // Add other relevant metrics as needed from Google Ads API documentation
    // e.g., view_through_conversions, all_conversions, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentsData {
    pub date: Option<String>, // YYYY-MM-DD
    pub device: Option<String>, // e.g., "MOBILE", "DESKTOP"
    // Add other relevant segments as needed
}

/// Represents a single row from a Google Ads API report query.
/// Fields are optional because not all queries return all fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAdsRow {
    pub campaign: Option<CampaignResource>,
    pub adGroup: Option<AdGroupResource>,
    pub keywordView: Option<KeywordViewResource>,
    pub adGroupCriterion: Option<AdGroupCriterionResource>,
    pub metrics: Option<MetricsData>,
    pub segments: Option<SegmentsData>,
}

// --- Flattened Analytics Report Structures (Processed Data) ---

/// Represents common Google Ads metrics for a given period or entity (processed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetrics {
    pub impressions: u64,
    pub clicks: u64,
    pub cost: f64, // In account currency, e.g., USD
    pub conversions: f64,
    pub conversions_value: f64, // Total value of all conversions
    pub ctr: f64, // Click-through rate (Clicks / Impressions)
    pub cpc: f64, // Cost per click (Cost / Clicks)
    pub cpa: f64, // Cost per acquisition (Cost / Conversions)
    pub roas: f64, // Return on ad spend (Conversions Value / Cost)
}

impl Default for ReportMetrics {
    fn default() -> Self {
        ReportMetrics {
            impressions: 0,
            clicks: 0,
            cost: 0.0,
            conversions: 0.0,
            conversions_value: 0.0,
            ctr: 0.0,
            cpc: 0.0,
            cpa: 0.0,
            roas: 0.0,
        }
    }
}

/// Data for a single campaign report row (processed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReportRow {
    pub date: String, // YYYY-MM-DD
    pub campaign_id: String,
    pub campaign_name: String,
    pub campaign_status: String,
    #[serde(flatten)]
    pub metrics: ReportMetrics,
}

/// Data for a single ad group report row (processed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdGroupReportRow {
    pub date: String, // YYYY-MM-DD
    pub campaign_id: String,
    pub campaign_name: String,
    pub ad_group_id: String,
    pub ad_group_name: String,
    pub ad_group_status: String,
    #[serde(flatten)]
    pub metrics: ReportMetrics,
}

/// Data for a single keyword report row (processed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordReportRow {
    pub date: String, // YYYY-MM-DD
    pub campaign_id: String,
    pub campaign_name: String,
    pub ad_group_id: String,
    pub ad_group_name: String,
    pub keyword_id: String, // This will be ad_group_criterion.criterionId
    pub keyword_text: String,
    pub match_type: String, // e.g., "Exact", "Phrase", "Broad"
    pub quality_score: Option<u32>,
    #[serde(flatten)]
    pub metrics: ReportMetrics,
}

/// Overall structure for a Google Ads Analytics report (processed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub report_name: String,
    pub date_range: String, // e.g., "2023-01-01 to 2023-01-31"
    pub total_metrics: ReportMetrics,
    pub campaign_data: Vec<CampaignReportRow>,
    pub ad_group_data: Vec<AdGroupReportRow>,
    pub keyword_data: Vec<KeywordReportRow>,
}

impl Default for AnalyticsReport {
    fn default() -> Self {
        AnalyticsReport {
            report_name: "Google Ads Analytics Report".to_string(),
            date_range: "N/A".to_string(),
            total_metrics: ReportMetrics::default(),
            campaign_data: Vec::new(),
            ad_group_data: Vec::new(),
            keyword_data: Vec::new(),
        }
    }
}

// --- GA dataflow typed normalization structures (additive) ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceClassLabel {
    Observed,
    ScrapedFirstParty,
    Simulated,
    ConnectorDerived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceProvenance {
    pub connector_id: String,
    pub source_class: SourceClassLabel,
    pub source_system: String,
    pub collected_at_utc: String,
    pub freshness_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfidenceAnnotation {
    pub confidence_label: String,
    pub rationale: String,
    pub uncertainty_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributionWindowMetadata {
    pub lookback_days: u16,
    pub model: String,
    pub safeguarded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ga4NormalizedEvent {
    pub event_name: String,
    pub event_timestamp_utc: String,
    pub session_id: String,
    pub user_pseudo_id: String,
    pub traffic_source: Option<String>,
    pub medium: Option<String>,
    pub campaign: Option<String>,
    pub revenue_micros: Option<u64>,
    pub provenance: SourceProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedKpiNarrative {
    pub section_id: String,
    pub text: String,
    pub source_class: SourceClassLabel,
    pub confidence: ConfidenceAnnotation,
    pub attribution_window: AttributionWindowMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustedAnalyticsReportArtifact {
    pub report: AnalyticsReport,
    pub narratives: Vec<NormalizedKpiNarrative>,
    pub provenance: Vec<SourceProvenance>,
}
