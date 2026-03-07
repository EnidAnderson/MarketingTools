use crate::data_models::analytics::{AnalyticsReport, SourceProvenance};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

pub const MOCK_ANALYTICS_SCHEMA_VERSION_V1: &str = "mock_analytics_artifact.v1";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Versioned request contract for deterministic mock analytics generation.
/// invariants:
///   - `start_date` and `end_date` use ISO format `YYYY-MM-DD`.
///   - If `seed` is omitted, a stable seed is derived from request fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate)]
pub struct MockAnalyticsRequestV1 {
    #[validate(length(min = 10, max = 10))]
    pub start_date: String,
    #[validate(length(min = 10, max = 10))]
    pub end_date: String,
    pub campaign_filter: Option<String>,
    pub ad_group_filter: Option<String>,
    pub seed: Option<u64>,
    #[validate(length(min = 1, max = 128))]
    pub profile_id: String,
    pub include_narratives: bool,
    #[serde(default)]
    pub source_window_observations: Vec<SourceWindowObservationV1>,
    pub budget_envelope: BudgetEnvelopeV1,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Time granularity for source-window completeness evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceWindowGranularityV1 {
    Day,
    Hour,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Source-window observation payload supplied by connectors or ingestion jobs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceWindowObservationV1 {
    pub source_system: String,
    pub granularity: SourceWindowGranularityV1,
    #[serde(default)]
    pub observed_timestamps_utc: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Budget policy mode for handling envelope pressure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPolicyModeV1 {
    FailClosed,
    Degrade,
    Sample,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Required budget envelope for every analytics run.
/// invariants:
///   - all caps are positive
///   - `max_total_cost_micros` is a hard run ceiling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetEnvelopeV1 {
    pub max_retrieval_units: u64,
    pub max_analysis_units: u64,
    pub max_llm_tokens_in: u64,
    pub max_llm_tokens_out: u64,
    pub max_total_cost_micros: u64,
    pub policy: BudgetPolicyModeV1,
    pub provenance_ref: String,
}

impl Default for BudgetEnvelopeV1 {
    fn default() -> Self {
        Self {
            max_retrieval_units: 20_000,
            max_analysis_units: 10_000,
            max_llm_tokens_in: 15_000,
            max_llm_tokens_out: 8_000,
            max_total_cost_micros: 50_000_000,
            policy: BudgetPolicyModeV1::FailClosed,
            provenance_ref: "budget.default.v1".to_string(),
        }
    }
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
    #[serde(default)]
    pub connector_attestation: ConnectorConfigAttestationV1,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Cryptographic connector-config attestation for replay and runtime audits.
/// invariants:
///   - `connector_config_fingerprint` excludes secret values.
///   - `fingerprint_alg` + `fingerprint_input_schema` together define a reproducible hash contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ConnectorConfigAttestationV1 {
    pub connector_mode_effective: String,
    pub connector_config_fingerprint: String,
    pub fingerprint_alg: String,
    pub fingerprint_input_schema: String,
    pub fingerprint_created_at: Option<String>,
    pub runtime_build: Option<String>,
    pub fingerprint_salt_id: Option<String>,
    pub fingerprint_signature: Option<String>,
    pub fingerprint_key_id: Option<String>,
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
#[serde(rename_all = "snake_case")]
pub enum QualityCheckApplicabilityV1 {
    #[default]
    Applies,
    NotApplicable,
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
    #[serde(default)]
    pub applicability: QualityCheckApplicabilityV1,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Consolidated quality control report attached to every artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyticsQualityControlsV1 {
    pub schema_drift_checks: Vec<QualityCheckV1>,
    pub identity_resolution_checks: Vec<QualityCheckV1>,
    pub freshness_sla_checks: Vec<QualityCheckV1>,
    pub cross_source_checks: Vec<QualityCheckV1>,
    pub budget_checks: Vec<QualityCheckV1>,
    pub is_healthy: bool,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Source-level freshness SLA threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceFreshnessSlaV1 {
    pub source_system: String,
    pub max_freshness_minutes: u32,
    pub min_completeness_ratio: f64,
    pub timezone: String,
    pub severity: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Explicit freshness policy contract used to evaluate source latency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreshnessSlaPolicyV1 {
    pub policy_id: String,
    pub thresholds: Vec<SourceFreshnessSlaV1>,
}

impl Default for FreshnessSlaPolicyV1 {
    fn default() -> Self {
        Self {
            policy_id: "freshness_policy.default.v1".to_string(),
            thresholds: vec![
                SourceFreshnessSlaV1 {
                    source_system: "google_ads".to_string(),
                    max_freshness_minutes: 180,
                    min_completeness_ratio: 0.98,
                    timezone: "UTC".to_string(),
                    severity: "high".to_string(),
                },
                SourceFreshnessSlaV1 {
                    source_system: "ga4".to_string(),
                    max_freshness_minutes: 120,
                    min_completeness_ratio: 0.98,
                    timezone: "UTC".to_string(),
                    severity: "high".to_string(),
                },
                SourceFreshnessSlaV1 {
                    source_system: "wix_storefront".to_string(),
                    max_freshness_minutes: 240,
                    min_completeness_ratio: 0.95,
                    timezone: "UTC".to_string(),
                    severity: "medium".to_string(),
                },
            ],
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Tolerance configuration for reconciliation checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationToleranceV1 {
    pub check_code: String,
    #[serde(default)]
    pub max_abs_delta: Option<f64>,
    #[serde(default)]
    pub max_relative_delta: Option<f64>,
    pub severity: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Explicit policy contract for within-source and cross-source reconciliation rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationPolicyV1 {
    pub policy_id: String,
    pub tolerances: Vec<ReconciliationToleranceV1>,
}

impl Default for ReconciliationPolicyV1 {
    fn default() -> Self {
        Self {
            policy_id: "reconciliation_policy.default.v1".to_string(),
            tolerances: vec![
                ReconciliationToleranceV1 {
                    check_code: "identity_campaign_rollup_reconciliation".to_string(),
                    max_abs_delta: Some(0.01),
                    max_relative_delta: None,
                    severity: "high".to_string(),
                },
                ReconciliationToleranceV1 {
                    check_code: "cross_source_attributed_revenue_within_wix_gross".to_string(),
                    max_abs_delta: None,
                    max_relative_delta: Some(0.05),
                    severity: "high".to_string(),
                },
                ReconciliationToleranceV1 {
                    check_code: "cross_source_ga4_sessions_within_click_bound".to_string(),
                    max_abs_delta: None,
                    max_relative_delta: Some(0.0),
                    severity: "medium".to_string(),
                },
            ],
        }
    }
}

impl ReconciliationPolicyV1 {
    pub fn tolerance_for(&self, check_code: &str) -> Option<&ReconciliationToleranceV1> {
        self.tolerances
            .iter()
            .find(|item| item.check_code == check_code)
    }
}

impl FreshnessSlaPolicyV1 {
    pub fn threshold_for(&self, source_system: &str) -> Option<&SourceFreshnessSlaV1> {
        self.thresholds
            .iter()
            .find(|item| item.source_system == source_system)
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Quantitative quality scorecard for completeness, joins, freshness, and reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataQualitySummaryV1 {
    pub completeness_ratio: f64,
    pub identity_join_coverage_ratio: f64,
    #[serde(default = "default_ratio_one")]
    pub identity_applicability_ratio: f64,
    pub freshness_pass_ratio: f64,
    pub reconciliation_pass_ratio: f64,
    pub cross_source_pass_ratio: f64,
    #[serde(default = "default_ratio_one")]
    pub cross_source_applicability_ratio: f64,
    pub budget_pass_ratio: f64,
    pub quality_score: f64,
}

impl Default for DataQualitySummaryV1 {
    fn default() -> Self {
        Self {
            completeness_ratio: 1.0,
            identity_join_coverage_ratio: 1.0,
            identity_applicability_ratio: 1.0,
            freshness_pass_ratio: 1.0,
            reconciliation_pass_ratio: 1.0,
            cross_source_pass_ratio: 1.0,
            cross_source_applicability_ratio: 1.0,
            budget_pass_ratio: 1.0,
            quality_score: 1.0,
        }
    }
}

fn default_ratio_one() -> f64 {
    1.0
}

impl Default for AnalyticsQualityControlsV1 {
    fn default() -> Self {
        Self {
            schema_drift_checks: Vec::new(),
            identity_resolution_checks: Vec::new(),
            freshness_sla_checks: Vec::new(),
            cross_source_checks: Vec::new(),
            budget_checks: Vec::new(),
            is_healthy: true,
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: One budget tracking event emitted during guarded execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BudgetEventV1 {
    pub subsystem: String,
    pub category: String,
    pub attempted_units: u64,
    pub remaining_units_before: u64,
    pub outcome: String,
    pub message: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Actual budget consumption counters for a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BudgetActualsV1 {
    pub retrieval_units: u64,
    pub analysis_units: u64,
    pub llm_tokens_in: u64,
    pub llm_tokens_out: u64,
    pub total_cost_micros: u64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Budget state attached to artifact for audit and UI panels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BudgetSummaryV1 {
    pub envelope: BudgetEnvelopeV1,
    pub actuals: BudgetActualsV1,
    pub remaining: BudgetActualsV1,
    pub estimated: BudgetActualsV1,
    pub hard_daily_cap_micros: u64,
    pub daily_spent_before_micros: u64,
    pub daily_spent_after_micros: u64,
    pub clipped: bool,
    pub sampled: bool,
    pub incomplete_output: bool,
    #[serde(default)]
    pub skipped_modules: Vec<String>,
    #[serde(default)]
    pub events: Vec<BudgetEventV1>,
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
/// purpose: Confidence calibration summary across historical runs.
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
/// purpose: Persisted ingest cleaning note for audit and gate evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IngestCleaningNoteV1 {
    pub source_system: String,
    pub rule_id: String,
    pub severity: String,
    pub affected_field: String,
    pub raw_value: String,
    pub clean_value: String,
    pub message: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Source availability and observation coverage summary for dashboard consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SourceCoverageV1 {
    pub source_system: String,
    pub enabled: bool,
    pub observed: bool,
    #[serde(default)]
    pub row_count: u64,
    #[serde(default)]
    pub unavailable_reason: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Session-level visitor classification derived from `ga_session_number`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VisitorTypeV1 {
    New,
    Returning,
    #[default]
    Unknown,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Origin of experiment assignment evidence for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentAssignmentSourceV1 {
    Ga4EventParam,
    UrlQuery,
    Backend,
    DataLayer,
    #[default]
    Unknown,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Confidence label for one session's experiment assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentConfidenceV1 {
    High,
    Medium,
    Low,
    Ambiguous,
    #[default]
    Unassigned,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Assignment state for one session relative to experiment metadata coverage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentAssignmentStatusV1 {
    Assigned,
    Partial,
    Ambiguous,
    #[default]
    Unassigned,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Session-stable experiment assignment context resolved from the earliest credible signal.
/// invariants:
///   - `assignment_status=assigned` requires both `experiment_id` and `variant_id`.
///   - `assignment_status=ambiguous` must never be used for variant performance claims.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SessionExperimentContextV1 {
    #[serde(default)]
    pub experiment_id: Option<String>,
    #[serde(default)]
    pub experiment_name: Option<String>,
    #[serde(default)]
    pub variant_id: Option<String>,
    #[serde(default)]
    pub variant_name: Option<String>,
    #[serde(default)]
    pub assignment_source: Option<ExperimentAssignmentSourceV1>,
    #[serde(default)]
    pub assignment_confidence: AssignmentConfidenceV1,
    #[serde(default)]
    pub assignment_status: ExperimentAssignmentStatusV1,
    #[serde(default)]
    pub assignment_observed_at_utc: Option<String>,
    #[serde(default)]
    pub assignment_notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Rollup of GA4 event data to session semantics for funnels and landing analysis.
/// invariants:
///   - `session_key` is stable and unique within one artifact.
///   - `landing_context`, when present, must agree with `landing_path`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Ga4SessionRollupV1 {
    pub session_key: String,
    pub user_pseudo_id: String,
    #[serde(default)]
    pub ga_session_id: Option<i64>,
    pub session_start_ts_utc: String,
    pub first_event_ts_utc: String,
    #[serde(default)]
    pub landing_path: Option<String>,
    #[serde(default)]
    pub landing_host: Option<String>,
    #[serde(default)]
    pub landing_context: Option<LandingContextV1>,
    #[serde(default)]
    pub experiment_context: SessionExperimentContextV1,
    #[serde(default)]
    pub visitor_type: VisitorTypeV1,
    pub engaged_session: bool,
    #[serde(default)]
    pub engagement_time_msec: u64,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub device_category: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub source_medium: Option<String>,
    #[serde(default)]
    pub campaign: Option<String>,
    #[serde(default)]
    pub page_view_count: u32,
    #[serde(default)]
    pub user_engagement_count: u32,
    #[serde(default)]
    pub scroll_count: u32,
    #[serde(default)]
    pub view_item_count: u32,
    #[serde(default)]
    pub add_to_cart_count: u32,
    #[serde(default)]
    pub begin_checkout_count: u32,
    #[serde(default)]
    pub purchase_count: u32,
    #[serde(default)]
    pub revenue_usd: f64,
    #[serde(default)]
    pub transaction_ids: Vec<String>,
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
    #[serde(default)]
    pub daily_revenue_series: Vec<DailyRevenuePointV1>,
    pub observed_evidence: Vec<EvidenceItem>,
    pub inferred_guidance: Vec<GuidanceItem>,
    pub uncertainty_notes: Vec<String>,
    pub provenance: Vec<SourceProvenance>,
    #[serde(default)]
    pub source_coverage: Vec<SourceCoverageV1>,
    #[serde(default)]
    pub ga4_session_rollups: Vec<Ga4SessionRollupV1>,
    #[serde(default)]
    pub ingest_cleaning_notes: Vec<IngestCleaningNoteV1>,
    pub validation: AnalyticsValidationReportV1,
    #[serde(default)]
    pub quality_controls: AnalyticsQualityControlsV1,
    #[serde(default)]
    pub data_quality: DataQualitySummaryV1,
    #[serde(default)]
    pub freshness_policy: FreshnessSlaPolicyV1,
    #[serde(default)]
    pub reconciliation_policy: ReconciliationPolicyV1,
    #[serde(default)]
    pub budget: BudgetSummaryV1,
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
/// purpose: Append-only audit record for governed executive dashboard exports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DashboardExportAuditRecordV1 {
    pub schema_version: String,
    pub export_id: String,
    pub profile_id: String,
    pub run_id: String,
    pub exported_at_utc: String,
    pub export_format: String,
    pub target_ref: String,
    pub gate_status: String,
    pub publish_ready: bool,
    pub export_ready: bool,
    #[serde(default)]
    pub blocking_reasons: Vec<String>,
    #[serde(default)]
    pub warning_reasons: Vec<String>,
    pub attestation_policy_required: bool,
    pub attestation_verified: bool,
    pub attestation_key_id: Option<String>,
    pub export_payload_checksum_alg: String,
    pub export_payload_checksum: String,
    pub export_payload_ref: String,
    pub checked_by: String,
    pub release_id: String,
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
/// purpose: Survival/hazard point for each funnel stage based on stage transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunnelSurvivalPointV1 {
    pub stage: String,
    pub entrants: f64,
    pub survival_rate: f64,
    pub hazard_rate: f64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Survival analysis report for funnel diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunnelSurvivalReportV1 {
    pub points: Vec<FunnelSurvivalPointV1>,
    pub bottleneck_stage: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Revenue-truth report quantifying duplicate risk and canonical KPI posture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RevenueTruthReportV1 {
    pub canonical_revenue: f64,
    pub canonical_conversions: f64,
    pub strict_duplicate_ratio: f64,
    pub near_duplicate_ratio: f64,
    #[serde(default)]
    pub custom_purchase_rows: u64,
    #[serde(default)]
    pub custom_purchase_overlap_rows: u64,
    #[serde(default)]
    pub custom_purchase_orphan_rows: u64,
    #[serde(default)]
    pub custom_purchase_overlap_ratio: f64,
    #[serde(default)]
    pub custom_purchase_orphan_ratio: f64,
    #[serde(default)]
    pub truth_guard_status: String,
    pub inflation_risk: String,
    pub estimated_revenue_at_risk: f64,
    pub summary: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Attribution delta row comparing first-touch proxy, assist, and last-touch shares.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AttributionDeltaRowV1 {
    pub campaign: String,
    pub first_touch_proxy_share: f64,
    pub assist_share: f64,
    pub last_touch_share: f64,
    pub delta_first_vs_last: f64,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Attribution delta report exposing concentration and share-disagreement by campaign.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AttributionDeltaReportV1 {
    pub rows: Vec<AttributionDeltaRowV1>,
    pub dominant_last_touch_campaign: Option<String>,
    pub last_touch_concentration_hhi: f64,
    pub summary: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Executive quality scorecard tying ratio metrics to gate posture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DataQualityScorecardV1 {
    pub quality_score: f64,
    pub completeness_ratio: f64,
    pub freshness_pass_ratio: f64,
    pub reconciliation_pass_ratio: f64,
    pub cross_source_pass_ratio: f64,
    pub budget_pass_ratio: f64,
    pub high_severity_failures: u32,
    pub blocking_reasons_count: u32,
    pub warning_reasons_count: u32,
    pub gate_status: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Coverage posture for experiment assignment metadata across observed sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExperimentAssignmentCoverageReportV1 {
    pub total_observed_sessions: u64,
    pub assigned_sessions: u64,
    pub partial_sessions: u64,
    pub ambiguous_sessions: u64,
    pub unassigned_sessions: u64,
    pub assignment_coverage_ratio: String,
    pub denominator_scope: String,
    pub summary: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Variant-level funnel metrics computed only from assigned sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExperimentFunnelRowV1 {
    pub experiment_id: String,
    #[serde(default)]
    pub experiment_name: Option<String>,
    pub variant_id: String,
    #[serde(default)]
    pub variant_name: Option<String>,
    pub sessions: u64,
    pub engaged_sessions: u64,
    pub product_view_sessions: u64,
    pub add_to_cart_sessions: u64,
    pub checkout_sessions: u64,
    pub purchase_sessions: u64,
    pub revenue_usd: f64,
    pub denominator_scope: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Guardrail slice showing where experiment assignment coverage is weak or biased.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExperimentGuardrailSliceV1 {
    pub dimension_key: String,
    pub dimension_value: String,
    pub total_sessions: u64,
    pub assigned_sessions: u64,
    pub partial_sessions: u64,
    pub ambiguous_sessions: u64,
    pub coverage_ratio: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Experiment analytics summary derived from session-level assignment evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExperimentAnalyticsSummaryV1 {
    pub assignment_coverage: ExperimentAssignmentCoverageReportV1,
    #[serde(default)]
    pub funnel_rows: Vec<ExperimentFunnelRowV1>,
    #[serde(default)]
    pub guardrail_slices: Vec<ExperimentGuardrailSliceV1>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: One experiment claim bundled with its permission and readiness posture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExperimentGovernanceItemV1 {
    pub experiment_id: String,
    #[serde(default)]
    pub experiment_name: Option<String>,
    pub permission: InsightPermissionCardV1,
    pub readiness: ExperimentReadinessCardV1,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Dashboard-facing experiment governance report tying claims to explicit readiness state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExperimentGovernanceReportV1 {
    pub summary: String,
    pub coverage_scope: String,
    #[serde(default)]
    pub items: Vec<ExperimentGovernanceItemV1>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Consolidated high-leverage reports derived from dashboard artifact evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HighLeverageReportsV1 {
    pub revenue_truth: RevenueTruthReportV1,
    pub funnel_survival: FunnelSurvivalReportV1,
    pub attribution_delta: AttributionDeltaReportV1,
    pub data_quality_scorecard: DataQualityScorecardV1,
    #[serde(default)]
    pub experiment_analytics: ExperimentAnalyticsSummaryV1,
    #[serde(default)]
    pub experiment_governance: ExperimentGovernanceReportV1,
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
/// purpose: Daily revenue point for date-level revenue trend and reconciliation views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DailyRevenuePointV1 {
    pub date: String,
    pub revenue: f64,
    pub conversions: f64,
    pub source_system: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: One Wix storefront behavior aggregate row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StorefrontBehaviorRowV1 {
    pub segment: String,
    pub product_or_template: String,
    pub sessions: u64,
    #[serde(default)]
    pub landing_path: Option<String>,
    #[serde(default)]
    pub landing_family: Option<String>,
    #[serde(default)]
    pub engaged_rate: f64,
    #[serde(default)]
    pub product_view_rate: f64,
    pub add_to_cart_rate: f64,
    #[serde(default)]
    pub checkout_rate: f64,
    pub purchase_rate: f64,
    #[serde(default)]
    pub revenue_per_session: f64,
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
/// purpose: Governance-grade decision feed card for operator actioning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DecisionFeedCardV1 {
    pub card_id: String,
    pub priority: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub recommended_action: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Publish/export gate state with explicit block reasons.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PublishExportGateV1 {
    pub publish_ready: bool,
    pub export_ready: bool,
    #[serde(default)]
    pub blocking_reasons: Vec<String>,
    #[serde(default)]
    pub warning_reasons: Vec<String>,
    pub gate_status: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Versioned landing-taxonomy assignment emitted by analytics and experiment workflows.
/// invariants:
///   - `taxonomy_version` identifies the exact mapping contract used at classification time.
///   - `matched_rule_id` must be stable so audit logs can explain why a route was bucketed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LandingContextV1 {
    pub taxonomy_version: String,
    pub matched_rule_id: String,
    pub landing_path: String,
    pub landing_family: String,
    pub landing_page_group: String,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Machine-readable policy state describing whether an insight is decision-safe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsightPermissionStateV1 {
    AllowedOperationalClaim,
    DirectionalOnly,
    InsufficientEvidence,
    InstrumentFirst,
    Blocked,
}

impl Default for InsightPermissionStateV1 {
    fn default() -> Self {
        Self::InsufficientEvidence
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Sample context attached to experiment-readiness and insight-permission cards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InsightSampleContextV1 {
    pub analysis_window: String,
    pub units_observed: u64,
    pub outcome_events: Option<u64>,
    #[serde(default)]
    pub coverage_notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Typed insight-permission card consumed by dashboards and content workflows.
/// invariants:
///   - `allowed_uses` and `blocked_uses` are policy outputs, not narrative suggestions.
///   - `permission_state` must downgrade to `instrument_first` when instrumentation is insufficient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InsightPermissionCardV1 {
    pub insight_id: String,
    pub decision_target: String,
    pub statement: String,
    pub permission_state: InsightPermissionStateV1,
    pub confidence_tier: String,
    pub action_state: String,
    pub sample_context: InsightSampleContextV1,
    #[serde(default)]
    pub allowed_uses: Vec<String>,
    #[serde(default)]
    pub blocked_uses: Vec<String>,
    #[serde(default)]
    pub next_data_actions: Vec<String>,
    #[serde(default)]
    pub taxonomy_version: Option<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::contracts`
/// purpose: Typed experiment-readiness card for landing and campaign experiments.
/// invariants:
///   - `control_landing_family` names the operational control candidate when one exists.
///   - If `required_sample_size` exceeds `observed_sample_size`, readiness cannot be `allowed_operational_claim`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExperimentReadinessCardV1 {
    pub experiment_id: String,
    pub objective: String,
    pub control_landing_family: String,
    #[serde(default)]
    pub challenger_landing_families: Vec<String>,
    pub primary_metric: String,
    pub baseline_value: Option<String>,
    pub minimum_detectable_effect: Option<String>,
    pub required_sample_size: Option<u64>,
    pub observed_sample_size: Option<u64>,
    pub readiness_state: InsightPermissionStateV1,
    #[serde(default)]
    pub control_variant_id: Option<String>,
    #[serde(default)]
    pub challenger_variant_id: Option<String>,
    #[serde(default)]
    pub permission_level: String,
    #[serde(default)]
    pub supporting_reasons: Vec<String>,
    #[serde(default)]
    pub blocking_reasons: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
    #[serde(default)]
    pub assigned_sessions_control: Option<u64>,
    #[serde(default)]
    pub assigned_sessions_challenger: Option<u64>,
    #[serde(default)]
    pub control_outcome_events: Option<u64>,
    #[serde(default)]
    pub challenger_outcome_events: Option<u64>,
    #[serde(default)]
    pub assignment_rate_bps: Option<u32>,
    #[serde(default)]
    pub ambiguity_rate_bps: Option<u32>,
    #[serde(default)]
    pub partial_or_unassigned_rate_bps: Option<u32>,
    #[serde(default)]
    pub denominator_scope: Option<String>,
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
    #[serde(default)]
    pub daily_revenue_series: Vec<DailyRevenuePointV1>,
    pub roas_target_band: Option<f64>,
    pub funnel_summary: FunnelSummaryV1,
    pub storefront_behavior_summary: StorefrontBehaviorSummaryV1,
    pub portfolio_rows: Vec<PortfolioRowV1>,
    pub forecast_summary: ForecastSummaryV1,
    #[serde(default)]
    pub data_quality: DataQualitySummaryV1,
    #[serde(default)]
    pub budget: BudgetSummaryV1,
    #[serde(default)]
    pub decision_feed: Vec<DecisionFeedCardV1>,
    #[serde(default)]
    pub publish_export_gate: PublishExportGateV1,
    #[serde(default)]
    pub source_coverage: Vec<SourceCoverageV1>,
    pub quality_controls: AnalyticsQualityControlsV1,
    pub historical_analysis: HistoricalAnalysisV1,
    pub operator_summary: OperatorSummaryV1,
    #[serde(default)]
    pub high_leverage_reports: HighLeverageReportsV1,
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
