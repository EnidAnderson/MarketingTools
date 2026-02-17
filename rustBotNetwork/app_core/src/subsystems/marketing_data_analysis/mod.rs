/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Typed mock analytics subsystem for deterministic, auditable market analysis.
/// invariants:
///   - Request and artifact payloads are schema-versioned.
///   - Request validation and artifact validation are transport-neutral.
///   - Orchestration does not depend on Tauri/UI types.
pub mod contracts;
pub mod executive_dashboard;
pub mod longitudinal;
pub mod persistence;
pub mod service;
pub mod validators;

pub use contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1,
    AnalyticsValidationReportV1, ArtifactPersistenceRefV1, ChannelMixPointV1,
    ConfidenceCalibrationV1, DriftFlagV1, EvidenceItem, ExecutiveDashboardSnapshotV1,
    ForecastSummaryV1, FunnelStageV1, FunnelSummaryV1, GuidanceItem, HistoricalAnalysisV1,
    KpiAttributionNarrativeV1, KpiDeltaV1, KpiTileV1, MockAnalyticsArtifactV1,
    MockAnalyticsRequestV1, OperatorSummaryV1, PersistedAnalyticsRunV1, PortfolioRowV1,
    QualityCheckV1, StorefrontBehaviorRowV1, StorefrontBehaviorSummaryV1, ValidationCheck,
};
pub use executive_dashboard::build_executive_dashboard_snapshot;
pub use longitudinal::build_historical_analysis;
pub use persistence::AnalyticsRunStore;
pub use service::{DefaultMarketAnalysisService, MarketAnalysisService};
