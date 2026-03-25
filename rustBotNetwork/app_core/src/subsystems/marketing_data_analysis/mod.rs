pub mod analytics_config;
pub mod attestation;
pub mod attestation_policy;
/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Typed mock analytics subsystem for deterministic, auditable market analysis.
/// invariants:
///   - Request and artifact payloads are schema-versioned.
///   - Request validation and artifact validation are transport-neutral.
///   - Orchestration does not depend on Tauri/UI types.
pub mod budget;
pub mod connector_factory;
pub mod connector_v2;
pub mod contracts;
pub mod executive_dashboard;
pub mod experiment_governance;
pub mod export_audit;
pub mod ga4_sessions;
pub mod ingest;
pub mod longitudinal;
pub mod persistence;
pub mod preflight;
pub mod purchase_truth;
pub mod subbly_wix_report;
pub mod service;
pub mod validators;

pub use analytics_config::{
    analytics_connector_config_fingerprint_v1, analytics_connector_config_from_env,
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
    AnalyticsSourceTopologyV1, Ga4ConfigV1, Ga4ReadBackendV1, GoogleAdsConfigV1, WixConfigV1,
    CONNECTOR_CONFIG_FINGERPRINT_ALG_V1, CONNECTOR_CONFIG_FINGERPRINT_SCHEMA_V1,
};
pub use attestation::{
    attestation_registry_diagnostics_v1, attestation_registry_validation_message_v1,
    canonical_attestation_payload_v1, load_attestation_key_registry_from_env_or_file,
    maybe_sign_connector_attestation_v1, verify_connector_attestation_signature_v1,
    verify_connector_attestation_with_registry_v1, AttestationKeyRegistryV1,
    AttestationRegistryDiagnosticsV1,
};
pub use attestation_policy::{
    is_production_profile_like, resolve_attestation_policy_v1, AttestationPolicySourceV1,
    AttestationPolicyV1,
};
pub use budget::{
    build_budget_plan, enforce_daily_hard_cap, estimate_budget_upper_bound, BudgetCategory,
    BudgetEstimate, BudgetGuard, BudgetPlan, DailyHardCapStatus, HARD_DAILY_SPEND_CAP_MICROS,
};
pub use connector_factory::build_analytics_connector_v2;
pub use connector_v2::{
    generate_simulated_ga4_events, generate_simulated_google_ads_rows,
    generate_simulated_wix_orders, generate_simulated_wix_sessions,
    AnalyticsConnectorCapabilitiesV1, AnalyticsConnectorContractV2, ConnectorHealthStatusV1,
    ConnectorSourceCapabilityV1, ConnectorSourceHealthV1, Ga4RawQueryV1, Ga4RawReportRowV1,
    Ga4RawReportV1, ObservedReadOnlyAnalyticsConnectorV2, SimulatedAnalyticsConnectorV2,
    WixItemRowV1, WixSessionRawV1,
};
pub use contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1,
    AnalyticsValidationReportV1, ArtifactPersistenceRefV1, AssignmentConfidenceV1,
    AttributionDeltaReportV1, AttributionDeltaRowV1, BudgetActualsV1, BudgetEnvelopeV1,
    BudgetEventV1, BudgetPolicyModeV1, BudgetSummaryV1, ChannelMixPointV1, ConfidenceCalibrationV1,
    ConnectorConfigAttestationV1, DailyRevenuePointV1, DashboardExportAuditRecordV1,
    DataQualityScorecardV1, DataQualitySummaryV1, DriftFlagV1, EvidenceItem,
    ExecutiveDashboardSnapshotV1, ExperimentAnalyticsSummaryV1,
    ExperimentAssignmentCoverageReportV1, ExperimentAssignmentSourceV1,
    ExperimentAssignmentStatusV1, ExperimentFunnelRowV1, ExperimentGovernanceItemV1,
    ExperimentGovernanceReportV1, ExperimentGuardrailSliceV1, ExperimentReadinessCardV1,
    ForecastSummaryV1, FreshnessSlaPolicyV1, FunnelStageV1, FunnelSummaryV1, FunnelSurvivalPointV1,
    FunnelSurvivalReportV1, Ga4SessionRollupV1, GuidanceItem, HighLeverageReportsV1,
    HistoricalAnalysisV1, IngestCleaningNoteV1, InsightPermissionCardV1, InsightPermissionStateV1,
    InsightSampleContextV1, KpiAttributionNarrativeV1, KpiDeltaV1, KpiTileV1, LandingContextV1,
    MockAnalyticsArtifactV1, MockAnalyticsRequestV1, OperatorSummaryV1, PersistedAnalyticsRunV1,
    PortfolioRowV1, PurchaseTruthAuditReportV1, PurchaseTruthSliceV1,
    QualityCheckApplicabilityV1, QualityCheckV1, ReconciliationPolicyV1,
    ReconciliationToleranceV1, RevenueTruthReportV1, SessionExperimentContextV1,
    SourceCoverageV1, SourceFreshnessSlaV1, SourceWindowGranularityV1,
    SourceWindowObservationV1, StorefrontBehaviorRowV1, StorefrontBehaviorSummaryV1,
    ValidationCheck, VisitorTypeV1,
};
pub use executive_dashboard::{build_executive_dashboard_snapshot, SnapshotBuildOptions};
pub use experiment_governance::{
    resolve_landing_experiment_permission_v1, resolve_landing_experiment_readiness_v1,
    resolve_observed_experiment_pair_permission_v1, resolve_observed_experiment_pair_readiness_v1,
    ExperimentClaimKindV1, LandingExperimentAssessmentInputV1,
    ObservedExperimentPairAssessmentInputV1,
};
pub use export_audit::DashboardExportAuditStore;
pub use ga4_sessions::{
    build_experiment_analytics_summary_from_sessions_v1, build_funnel_summary_from_sessions_v1,
    build_storefront_behavior_summary_from_sessions_v1, classify_landing_context_v2,
    extract_path_from_page_location, rollup_ga4_sessions_v1,
};
pub use ingest::{
    join_coverage_ratio, parse_ga4_event, parse_google_ads_row, parse_wix_order,
    window_completeness, Cleaned, CleaningNote, CleaningSeverity, Ga4EventRawV1, Ga4EventV1,
    GoogleAdsRowRawV1, GoogleAdsRowV1, IngestError, TimeGranularity, WindowCompletenessCheck,
    WixOrderRawV1, WixOrderV1,
};
pub use longitudinal::build_historical_analysis;
pub use persistence::AnalyticsRunStore;
pub use preflight::{
    evaluate_analytics_connectors_preflight, AnalyticsConnectorPreflightResultV1,
    AnalyticsPreflightSourceStatusV1, ANALYTICS_PREFLIGHT_SCHEMA_VERSION_V1,
};
pub use purchase_truth::{
    build_purchase_truth_audit_v1, ga4_canonical_purchase_truth_key_v1,
    ga4_canonical_purchase_truth_stats_v1, ga4_custom_purchase_match_stats_v1,
    ga4_event_date_utc_v1, ga4_event_epoch_seconds_v1, ga4_purchase_revenue_v1,
    ga4_session_key_v1, ga4_transaction_id_v1, Ga4CanonicalPurchaseTruthStatsV1,
    Ga4CustomPurchaseMatchStatsV1,
};
pub use subbly_wix_report::{
    build_subbly_wix_monthly_report, build_subbly_wix_monthly_report_with_bigquery,
    default_report_paths, default_suggestions_path, default_wix_unmapped_path,
    write_conflicts_csv, write_monthly_report_csv, write_suggestions_csv,
    write_unresolved_csv, write_wix_unmapped_csv, MonthlySkuSalesRow,
    SkuMappingConflict, SkuMappingSuggestion, SubblyWixReportOutput,
    UnresolvedMixMatchItem, WixUnmappedItem,
};
pub use service::{DefaultMarketAnalysisService, MarketAnalysisService};
