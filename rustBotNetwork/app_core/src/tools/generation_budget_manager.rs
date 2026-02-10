use chrono::{prelude::*, Duration};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value; // Added this import
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf}; // Added PathBuf here

// Define structs for serialization/deserialization
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)] // Added PartialEq and Clone for testing
pub struct BudgetState {
    // Made public for testing
    pub daily_spend: f64,
    pub daily_resets_on: String,
    pub generations: Vec<GenerationRecord>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)] // Added PartialEq and Clone for testing
pub struct GenerationRecord {
    // Made public for testing
    pub timestamp: String,
    pub tool: String,
    pub cost: f64, // Fixed typo f60 to f64
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone for API_COSTS
struct ApiCosts {
    #[serde(flatten)]
    costs: HashMap<String, ApiCost>,
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone for API_COSTS
pub struct ApiCost {
    // Made public for testing
    pub input: f64,
    pub output: f64,
}

// Default file paths
static DEFAULT_API_COSTS_FILE: &str = "src/data/api_costs.json";
static DEFAULT_BUDGET_FILE: &str = "src/data/generation_budget.json";

// Thread-local storage for overriding paths in tests
#[cfg(test)]
thread_local! {
    static TEST_BUDGET_FILE_PATH: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
    static TEST_API_COSTS_FILE_PATH: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
}

// Functions to set test paths
#[cfg(test)]
pub fn set_test_budget_file_path(path: Option<PathBuf>) {
    TEST_BUDGET_FILE_PATH.with(|p| *p.borrow_mut() = path);
}

#[cfg(test)]
pub fn set_test_api_costs_file_path(path: Option<PathBuf>) {
    TEST_API_COSTS_FILE_PATH.with(|p| *p.borrow_mut() = path);
}

fn get_budget_file_path() -> &'static Path {
    #[cfg(test)]
    {
        if let Some(path) = TEST_BUDGET_FILE_PATH.with(|p| p.borrow().clone()) {
            // Need to make sure the returned path has a static lifetime,
            // or return an owned PathBuf and let the caller manage its lifetime.
            // For now, using Box::leak as a quick fix for static lifetime in test.
            return Box::leak(path.into_boxed_path());
        }
    }
    Path::new(DEFAULT_BUDGET_FILE)
}

fn get_api_costs_file_path() -> &'static Path {
    #[cfg(test)]
    {
        if let Some(path) = TEST_API_COSTS_FILE_PATH.with(|p| p.borrow().clone()) {
            return Box::leak(path.into_boxed_path());
        }
    }
    Path::new(DEFAULT_API_COSTS_FILE)
}

// Global variable to store API_COSTS, loaded once
static API_COSTS: Lazy<HashMap<String, ApiCost>> = Lazy::new(|| {
    load_api_costs().unwrap_or_else(|_| {
        println!(
            "Warning: API costs file not found at {:?}. Using empty costs.",
            get_api_costs_file_path()
        );
        HashMap::new()
    })
});

fn load_api_costs() -> Result<HashMap<String, ApiCost>, Box<dyn std::error::Error>> {
    let path = get_api_costs_file_path();
    let contents = fs::read_to_string(path)?;
    let costs: ApiCosts = serde_json::from_str(&contents)?;
    Ok(costs.costs)
}

pub fn get_budget_state() -> BudgetState {
    // Made public
    let path = get_budget_file_path();
    if !path.exists() {
        return BudgetState {
            daily_spend: 0.0,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![],
        };
    }
    let contents = fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&contents).unwrap_or_else(|_| BudgetState {
        daily_spend: 0.0,
        daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
        generations: vec![],
    })
}

pub fn save_budget_state(state: &BudgetState) {
    // Made public
    let path = get_budget_file_path();
    let contents = serde_json::to_string_pretty(state).unwrap();
    fs::write(path, contents).unwrap();
}

fn check_and_update_budget() -> BudgetState {
    let mut state = get_budget_state();
    let today_str = Utc::now().date_naive().to_string();

    if today_str >= state.daily_resets_on {
        println!("Resetting daily budget.");
        state.daily_spend = 0.0;
        state.daily_resets_on = (Utc::now().date_naive() + Duration::days(1)).to_string();
        let one_month_ago = Utc::now() - Duration::days(30);
        state
            .generations
            .retain(|g| DateTime::parse_from_rfc3339(&g.timestamp).unwrap() > one_month_ago);
        save_budget_state(&state);
    }
    state
}

pub fn estimate_llm_cost(model_name: &str, input_text: &str, output_text: &str) -> f64 {
    if let Some(cost) = API_COSTS.get(model_name) {
        let input_tokens = input_text.len() as f64 / 4.0;
        let output_tokens = output_text.len() as f64 / 4.0;
        (input_tokens * cost.input) + (output_tokens * cost.output)
    } else {
        0.0
    }
}

pub fn estimate_embedding_cost(model_name: &str, text: &str) -> f64 {
    if let Some(cost) = API_COSTS.get(model_name) {
        let tokens = text.len() as f64 / 4.0;
        tokens * cost.input
    } else {
        0.0
    }
}

pub fn can_generate(cost: f64) -> bool {
    let state = check_and_update_budget();
    let daily_budget_usd: f64 = std::env::var("DAILY_BUDGET_USD")
        .unwrap_or_else(|_| "10.0".to_string())
        .parse()
        .unwrap_or(10.0);
    if state.daily_spend + cost > daily_budget_usd {
        println!(
            "Daily budget exceeded. Current spend: ${:.2}",
            state.daily_spend
        );
        false
    } else {
        true
    }
}

pub fn record_generation(cost: f64, tool_name: &str) {
    let mut state = check_and_update_budget();
    state.daily_spend += cost;
    state.generations.push(GenerationRecord {
        timestamp: Utc::now().to_rfc3339(),
        tool: tool_name.to_string(),
        cost,
    });
    save_budget_state(&state);
    println!(
        "Recorded generation from '{}' with cost ${:.4}.",
        tool_name, cost
    );
}

pub fn get_budget_status() -> String {
    let state = check_and_update_budget();
    let daily_budget_usd: f64 = std::env::var("DAILY_BUDGET_USD")
        .unwrap_or_else(|_| "10.0".to_string())
        .parse()
        .unwrap_or(10.0);
    format!(
        "Daily spend: ${:.4} / ${:.2}",
        state.daily_spend, daily_budget_usd
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_initial_budget_state() {
        let dir = tempdir().unwrap();
        set_test_budget_file_path(Some(dir.path().join("test_budget.json")));

        let state = get_budget_state();
        assert_eq!(state.daily_spend, 0.0);
        assert!(!state.daily_resets_on.is_empty());
        assert!(state.generations.is_empty());

        set_test_budget_file_path(None); // Clean up
    }

    #[test]
    fn test_save_and_load_budget_state() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test_budget.json");
        set_test_budget_file_path(Some(test_file.clone()));

        let initial_state = BudgetState {
            daily_spend: 5.0,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![GenerationRecord {
                timestamp: Utc::now().to_rfc3339(),
                tool: "test_tool".to_string(),
                cost: 1.0,
            }],
        };
        save_budget_state(&initial_state);

        let loaded_state = get_budget_state();
        assert_eq!(initial_state, loaded_state);

        set_test_budget_file_path(None); // Clean up
    }

    #[test]
    fn test_can_generate_with_sufficient_budget() {
        let dir = tempdir().unwrap();
        let test_budget_file = dir.path().join("test_budget.json");
        set_test_budget_file_path(Some(test_budget_file.clone()));

        // Set up initial budget state
        let initial_state = BudgetState {
            daily_spend: 1.0,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![],
        };
        save_budget_state(&initial_state);

        // Set a high daily budget for this test
        std::env::set_var("DAILY_BUDGET_USD", "10.0");
        assert!(can_generate(5.0)); // 1.0 + 5.0 = 6.0, which is <= 10.0
        std::env::remove_var("DAILY_BUDGET_USD");

        set_test_budget_file_path(None); // Clean up
    }

    #[test]
    fn test_can_generate_with_insufficient_budget() {
        let dir = tempdir().unwrap();
        let test_budget_file = dir.path().join("test_budget.json");
        set_test_budget_file_path(Some(test_budget_file.clone()));

        // Set up initial budget state
        let initial_state = BudgetState {
            daily_spend: 9.0,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![],
        };
        save_budget_state(&initial_state);

        // Set a daily budget
        std::env::set_var("DAILY_BUDGET_USD", "10.0");
        assert!(!can_generate(2.0)); // 9.0 + 2.0 = 11.0, which is > 10.0
        std::env::remove_var("DAILY_BUDGET_USD");

        set_test_budget_file_path(None); // Clean up
    }

    #[test]
    fn test_record_generation() {
        let dir = tempdir().unwrap();
        let test_budget_file = dir.path().join("test_budget.json");
        set_test_budget_file_path(Some(test_budget_file.clone()));

        // Set up initial budget state
        let initial_state = BudgetState {
            daily_spend: 0.0,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![],
        };
        save_budget_state(&initial_state);

        record_generation(2.5, "image_tool");
        let state_after_record = get_budget_state();

        assert_eq!(state_after_record.daily_spend, 2.5);
        assert_eq!(state_after_record.generations.len(), 1);
        assert_eq!(state_after_record.generations[0].tool, "image_tool");
        assert_eq!(state_after_record.generations[0].cost, 2.5);

        set_test_budget_file_path(None); // Clean up
    }

    #[test]
    fn test_daily_budget_reset() {
        let dir = tempdir().unwrap();
        let test_budget_file = dir.path().join("test_budget.json");
        set_test_budget_file_path(Some(test_budget_file.clone()));

        let yesterday = Utc::now().date_naive() - Duration::days(1);
        let mut state = BudgetState {
            daily_spend: 5.0,
            daily_resets_on: yesterday.to_string(), // Should trigger a reset
            generations: vec![],
        };
        save_budget_state(&state);

        // Call a function that triggers check_and_update_budget
        std::env::set_var("DAILY_BUDGET_USD", "10.0");
        let can_gen = can_generate(1.0);
        std::env::remove_var("DAILY_BUDGET_USD");

        let reset_state = get_budget_state();
        assert_eq!(reset_state.daily_spend, 0.0);
        assert!(reset_state.daily_resets_on > yesterday.to_string());
        assert!(can_gen);

        set_test_budget_file_path(None); // Clean up
    }
}
