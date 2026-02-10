use log::kv::{self, Key, Value, Visitor};

use log::{Log, Metadata, Record, SetLoggerError};

use once_cell::sync::OnceCell;

use serde::{Deserialize, Serialize};

use serde_json::json;

use std::cell::RefCell;

use std::collections::HashMap;

// --- Thread-local campaign ID storage ---

thread_local! {

    static CURRENT_CAMPAIGN_ID: RefCell<Option<String>> = RefCell::new(None);

}

/// Sets the current campaign ID for the calling thread.

pub fn set_current_campaign_id(campaign_id: String) {
    CURRENT_CAMPAIGN_ID.with(|id_cell| {
        *id_cell.borrow_mut() = Some(campaign_id);
    });
}

/// Clears the current campaign ID for the calling thread.

pub fn clear_current_campaign_id() {
    CURRENT_CAMPAIGN_ID.with(|id_cell| {
        *id_cell.borrow_mut() = None;
    });
}

// --- Log Record Structure for JSON output ---

#[derive(Debug, Serialize, Deserialize)]

pub struct JsonLogRecord {
    pub timestamp: String,

    pub level: String,

    pub message: String,

    pub campaign_id: Option<String>,

    pub name: String,

    pub module_path: Option<String>,

    pub file: Option<String>,

    pub line: Option<u32>,

    #[serde(flatten)] // Flatten to include extra fields directly
    pub extra: HashMap<String, serde_json::Value>,
}

// --- Custom Logger Implementation ---

struct JsonLogger;

/// Helper to collect key-value pairs from `log::kv::Source`

struct KeyValueCollector<'a>(&'a mut HashMap<String, serde_json::Value>);

impl<'a> Visitor<'a> for KeyValueCollector<'a> {
    fn visit_pair(&mut self, key: Key<'a>, value: Value<'a>) -> Result<(), kv::Error> {
        self.0
            .insert(key.as_str().to_string(), json!(value.to_string()));

        Ok(())
    }
}

impl Log for JsonLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let campaign_id = CURRENT_CAMPAIGN_ID.with(|id_cell| id_cell.borrow().clone());

        let mut extra_fields = HashMap::new();

        let mut collector = KeyValueCollector(&mut extra_fields);

        if let Err(e) = record.key_values().visit(&mut collector) {
            eprintln!("Error visiting key-value pairs: {:?}", e);
        }

        let log_record = JsonLogRecord {
            timestamp: chrono::Utc::now().to_rfc3339(),

            level: record.level().to_string(),

            message: format!("{}", record.args()),

            campaign_id,

            name: record.target().to_string(),

            module_path: record.module_path().map(|s| s.to_string()),

            file: record.file().map(|s| s.to_string()),

            line: record.line(),

            extra: extra_fields,
        };

        println!(
            "{}",
            serde_json::to_string(&log_record).expect("Failed to serialize log record to JSON")
        );
    }

    fn flush(&self) {}
}

static LOGGER: JsonLogger = JsonLogger;

static INIT_LOGGER_ONCE: OnceCell<()> = OnceCell::new();

/// Initializes the custom JSON logger. This function should be called once at the start of the application.

pub fn init_logger() -> Result<(), SetLoggerError> {
    INIT_LOGGER_ONCE
        .get_or_try_init(|| {
            log::set_logger(&LOGGER)?;

            log::set_max_level(log::LevelFilter::Info);

            Ok(())
        })
        .map(|_| ())
}

// --- Specific logging functions (Python equivalents) ---

/// Logs an agent-specific event.

pub fn log_agent_event(agent_name: &str, event_type: &str, details: &serde_json::Value) {
    let details_str = details.to_string();

    log::info!(

        target: agent_name,

        event_type = event_type,

        details = details_str.as_str();

        "Agent Event"

    );
}

/// Logs an LLM call.

pub fn log_llm_call(agent_name: &str, prompt: &str, response: &str, model: &str) {
    log::info!(

        target: agent_name,

        prompt = prompt,

        response = response,

        model = model;

        "LLM Call"

    );
}

/// Logs a tool usage event.

pub fn log_tool_use(
    agent_name: &str,

    tool_name: &str,

    input_data: &serde_json::Value,

    output_data: &serde_json::Value,
) {
    let input_str = input_data.to_string();

    let output_str = output_data.to_string();

    log::info!(

        target: agent_name,

        tool_name = tool_name,

        input = input_str.as_str(),

        output = output_str.as_str();

        "Tool Use"

    );
}

// Example usage

#[cfg(test)]

mod tests {

    use super::*;

    use log::info;

    use serde_json::json; // Use serde_json::json for creating test data

    #[test]

    fn test_logger_functionality() {
        // Ensure logger is initialized only once

        let _ = init_logger();

        set_current_campaign_id("test_campaign_123".to_string());

        // Use the log::info! macro directly for a generic message

        info!(target: "root", "This is a root logger message.");

        // Use the specialized logging functions

        let details = json!({ "task": "initial_planning" });

        log_agent_event("StrategistAgent", "task_started", &details);

        log_llm_call(
            "StrategistAgent",
            "Plan a campaign",
            "Campaign plan generated.",
            "gemini-pro",
        );

        let input = json!({ "to": "test@example.com" });

        let output = json!({ "status": "success" });

        log_tool_use("StrategistAgent", "EmailSenderTool", &input, &output);

        clear_current_campaign_id(); // Clear campaign ID for the next part of the test

        info!(target: "root", "This message should not have a campaign_id.");

        set_current_campaign_id("another_campaign_456".to_string());

        log::warn!(target: "root", "This is a warning for another campaign.");
    }
}
