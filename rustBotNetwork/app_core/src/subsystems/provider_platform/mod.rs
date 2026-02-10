use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::provider_platform`
/// purpose: Shared provider capability model and policy boundaries.
/// invariants:
///   - Provider clients are configured through explicit contracts, not ad-hoc globals.
///   - Retry/timeout/rate-limit policy is declared per provider class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub provider_name: String,
    pub supports_images: bool,
    pub supports_search: bool,
}

/// # NDOC
/// component: `subsystems::provider_platform`
/// purpose: Placeholder trait for provider discovery and policy checks.
pub trait ProviderPlatformService: Send + Sync {
    fn service_name(&self) -> &'static str;
}
