use crate::data_models::analytics::{AnalyticsReport, SourceProvenance};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MOCK_ANALYTICS_SCHEMA_VERSION_V1: &str = "mock_analytics_artifact.v1";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Versioned request contract for deterministic mock analytics generation.
/// invariants:
///   - `start_date` and `end_date` use ISO format `YYYY-MM-DD`.
///   - If `seed` is omitted, a stable seed is derived from request fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockAnalyticsRequestV1 {
    pub start_date: String,
    pub end_date: String,
    pub campaign_filter: Option<String>,
    pub ad_group_filter: Option<String>,
    pub seed: Option<u64>,
    pub profile_id: String,
    pub include_narratives: bool,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Structured observed evidence item extracted from report data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceItem {
    pub evidence_id: String,
    pub label: String,
    pub value: String,
    pub source_class: String,
    #[serde(default)]
    pub metric_key: Option<String>,
    #[serde(default)]
    pub observed_window: Option<String>,
    #[serde(default)]
    pub comparator_value: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Structured inferred guidance separate from observed evidence.
/// invariants:
///   - `confidence_label` must be bounded by source class confidence rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuidanceItem {
    pub guidance_id: String,
    pub text: String,
    pub confidence_label: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub attribution_basis: Option<String>,
    #[serde(default)]
    pub calibration_bps: Option<u16>,
    #[serde(default)]
    pub calibration_band: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Deterministic run metadata for replay and audit.
/// invariants:
///   - `run_id` is derived from request + seed + schema version.
///   - `requested_at_utc` is optional so byte-stable artifacts remain possible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsRunMetadataV1 {
    pub run_id: String,
    pub connector_id: String,
    pub profile_id: String,
    pub seed: u64,
    pub schema_version: String,
    pub date_span_days: u32,
    pub requested_at_utc: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: One validation check result for invariant enforcement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationCheck {
    pub code: String,
    pub passed: bool,
    pub message: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Validation summary attached to every artifact envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsValidationReportV1 {
    pub is_valid: bool,
    pub checks: Vec<ValidationCheck>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Quality control check emitted for schema drift, identity resolution, and freshness SLA.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct QualityCheckV1 {
    pub code: String,
    pub passed: bool,
    pub severity: String,
    pub observed: String,
    pub expected: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Consolidated quality control report attached to every artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsQualityControlsV1 {
    pub schema_drift_checks: Vec<QualityCheckV1>,
    pub identity_resolution_checks: Vec<QualityCheckV1>,
    pub freshness_sla_checks: Vec<QualityCheckV1>,
    pub is_healthy: bool,
}

impl Default for AnalyticsQualityControlsV1 {
    fn default() -> Self {
        Self {
            schema_drift_checks: Vec::new(),
            identity_resolution_checks: Vec::new(),
            freshness_sla_checks: Vec::new(),
            is_healthy: true,
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: KPI delta between current run and recent baseline run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KpiDeltaV1 {
    pub metric_key: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub delta_absolute: f64,
    pub delta_percent: Option<f64>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Baseline drift signal derived from historical runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DriftFlagV1 {
    pub metric_key: String,
    pub baseline_mean: f64,
    pub baseline_std_dev: f64,
    pub current_value: f64,
    pub z_score: f64,
    pub severity: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Anomaly flag for operator triage in dashboards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AnomalyFlagV1 {
    pub metric_key: String,
    pub reason: String,
    pub severity: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Confidence calibration summary across historical simulated runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ConfidenceCalibrationV1 {
    pub sample_count: u32,
    pub recommended_confidence_cap: String,
    pub calibration_note: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Historical/longitudinal analysis payload for trend comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HistoricalAnalysisV1 {
    pub baseline_run_ids: Vec<String>,
    pub period_over_period_deltas: Vec<KpiDeltaV1>,
    pub drift_flags: Vec<DriftFlagV1>,
    pub anomaly_flags: Vec<AnomalyFlagV1>,
    pub confidence_calibration: ConfidenceCalibrationV1,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Operator-facing KPI narrative with explicit evidence references.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KpiAttributionNarrativeV1 {
    pub kpi: String,
    pub narrative: String,
    pub evidence_ids: Vec<String>,
    pub confidence_label: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Operator summary bundle designed for UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OperatorSummaryV1 {
    pub attribution_narratives: Vec<KpiAttributionNarrativeV1>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Persistence metadata for durable artifact storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ArtifactPersistenceRefV1 {
    pub stored_at_utc: String,
    pub storage_path: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Versioned artifact envelope returned by the orchestrator.
/// invariants:
///   - `schema_version` is explicit at root.
///   - Observed evidence and inferred guidance are never merged into one field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAnalyticsArtifactV1 {
    pub schema_version: String,
    pub request: MockAnalyticsRequestV1,
    pub metadata: AnalyticsRunMetadataV1,
    pub report: AnalyticsReport,
    pub observed_evidence: Vec<EvidenceItem>,
    pub inferred_guidance: Vec<GuidanceItem>,
    pub uncertainty_notes: Vec<String>,
    pub provenance: Vec<SourceProvenance>,
    pub validation: AnalyticsValidationReportV1,
    #[serde(default)]
    pub quality_controls: AnalyticsQualityControlsV1,
    #[serde(default)]
    pub historical_analysis: HistoricalAnalysisV1,
    #[serde(default)]
    pub operator_summary: OperatorSummaryV1,
    #[serde(default)]
    pub persistence: Option<ArtifactPersistenceRefV1>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Durable run record for longitudinal analytics and replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAnalyticsRunV1 {
    pub schema_version: String,
    pub request: MockAnalyticsRequestV1,
    pub metadata: AnalyticsRunMetadataV1,
    pub validation: AnalyticsValidationReportV1,
    pub artifact: MockAnalyticsArtifactV1,
    pub stored_at_utc: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Executive KPI tile payload for top dashboard strip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KpiTileV1 {
    pub key: String,
    pub label: String,
    pub value: f64,
    pub formatted_value: String,
    pub delta_percent: Option<f64>,
    pub target_delta_percent: Option<f64>,
    pub confidence_label: String,
    pub source_class: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Funnel stage summary row for executive funnel panel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunnelStageV1 {
    pub stage: String,
    pub value: f64,
    pub conversion_from_previous: Option<f64>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Funnel summary section in executive dashboard snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunnelSummaryV1 {
    pub stages: Vec<FunnelStageV1>,
    pub dropoff_hotspot_stage: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Time point for channel mix and scale/efficiency trend charts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChannelMixPointV1 {
    pub period_label: String,
    pub spend: f64,
    pub revenue: f64,
    pub roas: f64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: One Wix storefront behavior aggregate row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StorefrontBehaviorRowV1 {
    pub segment: String,
    pub product_or_template: String,
    pub sessions: u64,
    pub add_to_cart_rate: f64,
    pub purchase_rate: f64,
    pub aov: f64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Storefront behavior panel model enriched with Wix-like aggregates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StorefrontBehaviorSummaryV1 {
    pub source_system: String,
    pub identity_confidence: String,
    pub rows: Vec<StorefrontBehaviorRowV1>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Campaign portfolio table row for executive ranking panel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PortfolioRowV1 {
    pub campaign: String,
    pub spend: f64,
    pub revenue: f64,
    pub roas: f64,
    pub ctr: f64,
    pub cpa: f64,
    pub conversions: f64,
    pub drift_severity: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Forecast and pace status section for executive planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ForecastSummaryV1 {
    pub expected_revenue_next_period: f64,
    pub expected_roas_next_period: f64,
    pub confidence_interval_low: f64,
    pub confidence_interval_high: f64,
    pub month_to_date_pacing_ratio: f64,
    pub month_to_date_revenue: f64,
    pub monthly_revenue_target: Option<f64>,
    pub target_roas: Option<f64>,
    pub pacing_status: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Stable multi-chart payload for frontend executive dashboard rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExecutiveDashboardSnapshotV1 {
    pub schema_version: String,
    pub profile_id: String,
    pub generated_at_utc: String,
    pub run_id: String,
    pub date_range: String,
    pub compare_window_runs: u8,
    pub kpis: Vec<KpiTileV1>,
    pub channel_mix_series: Vec<ChannelMixPointV1>,
    pub roas_target_band: Option<f64>,
    pub funnel_summary: FunnelSummaryV1,
    pub storefront_behavior_summary: StorefrontBehaviorSummaryV1,
    pub portfolio_rows: Vec<PortfolioRowV1>,
    pub forecast_summary: ForecastSummaryV1,
    pub quality_controls: AnalyticsQualityControlsV1,
    pub historical_analysis: HistoricalAnalysisV1,
    pub operator_summary: OperatorSummaryV1,
    pub trust_status: String,
    pub alerts: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Machine-readable analytics error payload with user-safe message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsError {
    pub code: String,
    pub message: String,
    pub field_paths: Vec<String>,
    pub context: Option<Value>,
}

impl AnalyticsError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        field_paths: Vec<String>,
        context: Option<Value>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field_paths,
            context,
        }
    }

    pub fn validation(
        code: impl Into<String>,
        message: impl Into<String>,
        field_path: impl Into<String>,
    ) -> Self {
        Self::new(code, message, vec![field_path.into()], None)
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message, Vec::new(), None)
    }
}

impl std::fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AnalyticsError {}
