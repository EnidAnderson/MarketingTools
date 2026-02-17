// src/lib.rs
pub mod analytics_connector_contracts;
pub mod analytics_data_generator; // Added analytics_data_generator module
pub mod analytics_data_transformer; // Added analytics_data_transformer module
pub mod analytics_reporter; // Added analytics_reporter module
pub mod contracts;
pub mod dashboard_processor; // Added dashboard_processor module
pub mod data_models;
pub mod image_generator;
pub mod integration_tests;
pub mod invariants;
pub mod legacy_adapter;
pub mod llm_client; // Added llm_client module
pub mod persona_loader; // Added persona_loader module
pub mod pipeline;
pub mod python_runner;
pub mod subsystems;
pub mod tools;
pub mod utils;
