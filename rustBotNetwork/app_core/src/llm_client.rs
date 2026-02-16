// rustBotNetwork/app_core/src/llm_client.rs

use std::env;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{sleep, Duration};
use once_cell::sync::Lazy; // For lazy static initialization
use serde_json::{json, Value}; // Import json! macro and Value type
use reqwest::Client; // Import reqwest client

// --- Rate Limiting Configuration ---
// Maximum LLM calls allowed per minute.
const MAX_LLM_CALLS_PER_MINUTE: usize = 10;
// Minimum delay between LLM calls in milliseconds to avoid hitting burst limits.
const MIN_DELAY_BETWEEN_CALLS_MS: u64 = (60_000 / MAX_LLM_CALLS_PER_MINUTE) as u64; // e.g., 6000ms for 10 calls/min

// Global atomic counter for LLM calls within the current minute.
static LLM_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
// Timestamp of the last minute reset for the call counter.
static LAST_RESET_TIME: Lazy<std::sync::Mutex<std::time::Instant>> = Lazy::new(|| {
    std::sync::Mutex::new(std::time::Instant::now())
});

/// Sends a text-only prompt to the Gemini model using reqwest.
///
/// # Arguments
/// * `prompt` - The text prompt to send to the Gemini model.
///
/// # Returns
/// A `Result` containing the model's response text on success, or an error if the API call fails.
pub async fn send_text_prompt(prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    // --- Cost Control: Per-call rate limiting ---
    loop { // Use a loop for retries
        let mut last_reset = LAST_RESET_TIME.lock().unwrap();
        let elapsed = last_reset.elapsed();

        if elapsed >= Duration::from_secs(60) {
            // A minute has passed, reset the counter
            LLM_CALL_COUNT.store(0, Ordering::SeqCst);
            *last_reset = std::time::Instant::now();
        }

        let current_calls = LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);

        if current_calls >= MAX_LLM_CALLS_PER_MINUTE {
            // We've hit the per-minute limit. Wait until the next minute starts.
            let time_to_wait = Duration::from_secs(60) - elapsed;
            println!("Rate limit hit. Waiting for {:?} before next LLM call.", time_to_wait);
            sleep(time_to_wait).await;
            // After waiting, reset the counter and retry the loop
            LLM_CALL_COUNT.store(0, Ordering::SeqCst);
            *last_reset = std::time::Instant::now();
            continue; // Continue the loop to re-check conditions
        } else if current_calls > 0 { // Apply min delay after the first call
            // Ensure minimum delay between calls
            sleep(Duration::from_millis(MIN_DELAY_BETWEEN_CALLS_MS)).await;
        }
        break; // Exit the loop if conditions are met to make the API call
    }
    // --- End Cost Control: Per-call rate limiting ---

    // --- Placeholder for Per-day rate limiting ---
    // A more sophisticated mechanism would read/write to a persistent store.
    // For now, this is a conceptual check.
    // if check_per_day_limit().is_err() {
    //     return Err("Daily LLM call limit reached. Please try again tomorrow.".into());
    // }
    // increment_per_day_counter();
    // --- End Placeholder for Per-day rate limiting ---


    // Load API key from environment variables
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY not set in environment variables")?;

    let client = Client::new();
    let model_name = "gemini-pro"; // Using gemini-pro for text-only, will use gemini-flash later as specified by user.
    let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model_name, api_key);

    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {
                        "text": prompt
                    }
                ]
            }
        ],
        "generationConfig": {
            "temperature": 0.7,
            "topP": 1.0,
            "topK": 40,
            "candidateCount": 1,
            "maxOutputTokens": 1024
        }
    });

    let response = client.post(&api_url)
        .json(&request_body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    // Extract the text from the first candidate
    if let Some(candidate) = response["candidates"].as_array().and_then(|arr| arr.get(0)) {
        if let Some(part) = candidate["content"]["parts"].as_array().and_then(|arr| arr.get(0)) {
            if let Some(text) = part["text"].as_str() {
                return Ok(text.to_string());
            }
        }
    }

    Err("Failed to get text from Gemini API response".into())
}
