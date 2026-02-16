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
