// src/lib.rs
#[cfg(feature = "legacy_analytics")]
pub mod analytics_connector_contracts;
#[cfg(feature = "legacy_analytics")]
pub mod analytics_reporter;
pub mod contracts;
pub mod data_models;
pub mod image_generator;
pub mod invariants;
pub mod llm_client; // Added llm_client module
pub mod persona_loader; // Added persona_loader module
pub mod pipeline;
pub mod python_runner;
pub mod subsystems;
pub mod tools;
pub mod utils;
