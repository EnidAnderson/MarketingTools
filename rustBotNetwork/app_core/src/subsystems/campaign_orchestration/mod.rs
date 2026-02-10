use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Domain contracts for campaign execution plans and run state.
/// invariants:
///   - Campaign runs are immutable after completion.
///   - Every run references a pipeline definition version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignRunDescriptor {
    pub campaign_id: String,
    pub pipeline_name: String,
    pub pipeline_version: String,
}

/// # NDOC
/// component: `subsystems::campaign_orchestration`
/// purpose: Placeholder orchestration trait for future campaign runtimes.
pub trait CampaignOrchestrator: Send + Sync {
    fn orchestrator_name(&self) -> &'static str;
}
