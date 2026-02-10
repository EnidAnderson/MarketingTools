use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Domain boundary for market/competitor signal extraction and interpretation.
/// invariants:
///   - Raw evidence and inferred guidance are represented as separate fields.
///   - Domain models remain transport-neutral and Rust-native.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSignalPack {
    pub topic: String,
    pub evidence_count: usize,
    pub inferred_notes: Vec<String>,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis`
/// purpose: Placeholder service boundary for future typed analysis APIs.
pub trait MarketAnalysisService: Send + Sync {
    fn service_name(&self) -> &'static str;
}
