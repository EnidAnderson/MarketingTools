/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Typed mock analytics subsystem for deterministic, auditable market analysis.
/// invariants:
///   - Request and artifact payloads are schema-versioned.
///   - Request validation and artifact validation are transport-neutral.
///   - Orchestration does not depend on Tauri/UI types.
pub mod budget;
pub mod contracts;
pub mod executive_dashboard;
pub mod ingest;
pub mod longitudinal;
pub mod persistence;
pub mod service;
pub mod validators;

pub use budget::{
    build_budget_plan, enforce_daily_hard_cap, estimate_budget_upper_bound, BudgetCategory,
    BudgetEstimate, BudgetGuard, BudgetPlan, DailyHardCapStatus, HARD_DAILY_SPEND_CAP_MICROS,
};
pub use contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1,
    AnalyticsValidationReportV1, ArtifactPersistenceRefV1, BudgetActualsV1, BudgetEnvelopeV1,
    BudgetEventV1, BudgetPolicyModeV1, BudgetSummaryV1, ChannelMixPointV1, ConfidenceCalibrationV1,
    DataQualitySummaryV1, DriftFlagV1, EvidenceItem, ExecutiveDashboardSnapshotV1,
    ForecastSummaryV1, FreshnessSlaPolicyV1, FunnelStageV1, FunnelSummaryV1, GuidanceItem,
    HistoricalAnalysisV1, IngestCleaningNoteV1, KpiAttributionNarrativeV1, KpiDeltaV1, KpiTileV1,
    MockAnalyticsArtifactV1, MockAnalyticsRequestV1, OperatorSummaryV1, PersistedAnalyticsRunV1,
    PortfolioRowV1, QualityCheckV1, SourceFreshnessSlaV1, StorefrontBehaviorRowV1,
    StorefrontBehaviorSummaryV1, ValidationCheck,
};
pub use executive_dashboard::{build_executive_dashboard_snapshot, SnapshotBuildOptions};
pub use ingest::{
    join_coverage_ratio, parse_ga4_event, window_completeness, Cleaned, CleaningNote,
    CleaningSeverity, Ga4EventRawV1, Ga4EventV1, GoogleAdsRowRawV1, GoogleAdsRowV1, IngestError,
    TimeGranularity, WindowCompletenessCheck, WixOrderRawV1, WixOrderV1,
};
pub use longitudinal::build_historical_analysis;
pub use persistence::AnalyticsRunStore;
pub use service::{DefaultMarketAnalysisService, MarketAnalysisService};
