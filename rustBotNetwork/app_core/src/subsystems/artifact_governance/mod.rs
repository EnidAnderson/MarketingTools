use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::artifact_governance`
/// purpose: Canonical artifact lineage and approval metadata contracts.
/// invariants:
///   - Every production artifact includes run/source provenance.
///   - Approval status is explicit and auditable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifestEntry {
    pub artifact_id: String,
    pub artifact_path: String,
    pub producing_tool: String,
    pub approved: bool,
}

/// # NDOC
/// component: `subsystems::artifact_governance`
/// purpose: Placeholder trait for manifest persistence and validation.
pub trait ArtifactGovernanceService: Send + Sync {
    fn service_name(&self) -> &'static str;
}
