use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::review_and_compliance`
/// purpose: Domain models for review outcomes and claim-safety checks.
/// invariants:
///   - Review outcomes are explicit (`approved`, `approved_with_caveat`, `blocked`).
///   - Claims must map to evidence references before approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDecision {
    pub review_run_id: String,
    pub outcome: String,
    pub blocking_reasons: Vec<String>,
}

/// # NDOC
/// component: `subsystems::review_and_compliance`
/// purpose: Placeholder trait for automated/manual review integration.
pub trait ReviewService: Send + Sync {
    fn service_name(&self) -> &'static str;
}
