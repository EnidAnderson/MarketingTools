/// # NDOC
/// component: `subsystems`
/// purpose: High-level domain subsystem boundaries for long-term platform growth.
/// invariants:
///   - Subsystems own domain contracts and orchestration, not UI transport.
///   - Cross-subsystem calls should happen via typed contracts.
pub mod artifact_governance;
pub mod campaign_orchestration;
pub mod marketing_data_analysis;
pub mod provider_platform;
pub mod review_and_compliance;
