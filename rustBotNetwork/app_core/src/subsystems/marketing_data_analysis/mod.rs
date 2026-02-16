/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Typed mock analytics subsystem for deterministic, auditable market analysis.
/// invariants:
///   - Request and artifact payloads are schema-versioned.
///   - Request validation and artifact validation are transport-neutral.
///   - Orchestration does not depend on Tauri/UI types.
pub mod contracts;
pub mod longitudinal;
pub mod persistence;
pub mod service;
pub mod validators;

pub use contracts::{
    AnalyticsError, AnalyticsRunMetadataV1, AnalyticsValidationReportV1,
    AnalyticsQualityControlsV1, ArtifactPersistenceRefV1, ConfidenceCalibrationV1, DriftFlagV1,
    EvidenceItem, GuidanceItem, HistoricalAnalysisV1, KpiAttributionNarrativeV1, KpiDeltaV1,
    MockAnalyticsArtifactV1, MockAnalyticsRequestV1, OperatorSummaryV1, PersistedAnalyticsRunV1,
    QualityCheckV1, ValidationCheck,
};
pub use longitudinal::build_historical_analysis;
pub use persistence::AnalyticsRunStore;
pub use service::{DefaultMarketAnalysisService, MarketAnalysisService};
