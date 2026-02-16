/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Typed mock analytics subsystem for deterministic, auditable market analysis.
/// invariants:
///   - Request and artifact payloads are schema-versioned.
///   - Request validation and artifact validation are transport-neutral.
///   - Orchestration does not depend on Tauri/UI types.
pub mod contracts;
pub mod service;
pub mod validators;

pub use contracts::{
    AnalyticsError, AnalyticsRunMetadataV1, AnalyticsValidationReportV1, EvidenceItem,
    GuidanceItem, MockAnalyticsArtifactV1, MockAnalyticsRequestV1, ValidationCheck,
};
pub use service::{DefaultMarketAnalysisService, MarketAnalysisService};
