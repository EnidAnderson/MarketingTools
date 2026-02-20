use crate::tools::generation_budget_manager::{self, PaidCallPermit};
use dotenv::dotenv;
use log::{error, info};
use serde_json::json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// Use the image crate for basic image handling
use base64::{engine::general_purpose, Engine as _};

/// Generates an image using the Google Gemini API.
///
/// This function sends a prompt to the "models/nano-banana-pro-preview" model
/// via the Gemini API, retrieves the generated image (base64 encoded),
/// decodes it, and saves it to the specified campaign directory.
///
/// # Arguments
/// * `prompt` - A string slice representing the prompt for image generation.
/// * `campaign_dir` - A string slice representing the directory where the image will be saved.
///
/// # Returns
/// A `Result` which is `Ok(PathBuf)` containing the path to the saved image on success,
/// or `Err(String)` containing an error message on failure.
pub async fn generate_image(prompt: &str, campaign_dir: &str) -> Result<PathBuf, String> {
    info!("Attempting to load .env file...");
    if let Err(e) = dotenv() {
        error!("Failed to load .env file: {}", e);
    }

    let api_key = env::var("GEMINI_API_KEY")
        .or_else(|_| env::var("GOOGLE_API_KEY"))
        .map_err(|_| {
            error!("GEMINI_API_KEY or GOOGLE_API_KEY not set in environment.");
            "GEMINI_API_KEY or GOOGLE_API_KEY not set in environment.".to_string()
        })?;
    info!("GEMINI_API_KEY successfully retrieved.");

    let model_name = env::var("GOOGLE_MODEL_IMAGE_FALLBACK")
        .unwrap_or_else(|_| "gemini-image-generation".to_string());
    let estimated_cost = generation_budget_manager::estimate_image_cost_strict(&model_name)
        .map_err(|e| format!("failed to estimate image model cost: {e}"))?;
    let permit = PaidCallPermit::reserve(
        estimated_cost,
        "image_generator.generate_image",
        "google",
        &model_name,
    )
    .map_err(|e| format!("paid call blocked by spend governor: {e}"))?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );

    let request_body = json!({
        "contents": [
            {
                "parts": [
                    {"text": prompt}
                ]
            }
        ]
    });

    let response = match client.post(&url).json(&request_body).send().await {
        Ok(resp) => resp,
        Err(err) => return Err(format!("Failed to send request to Gemini API: {}", err)),
    };

    let response_body: serde_json::Value = match response.json().await {
        Ok(body) => body,
        Err(err) => return Err(format!("Failed to parse Gemini API response: {}", err)),
    };

    // Extract image data
    let image_data_base64 = response_body["candidates"][0]["content"]["parts"][0]["inlineData"]
        ["data"]
        .as_str()
        .ok_or_else(|| "Could not find image data in Gemini API response.".to_string())?;

    let decoded_image_data = general_purpose::STANDARD
        .decode(image_data_base64)
        .map_err(|e| format!("Failed to decode base64 image data: {}", e))?;

    info!(
        "Decoded image data size: {} bytes.",
        decoded_image_data.len()
    );

    // Create campaign directory if it doesn't exist
    let campaign_path = PathBuf::from(campaign_dir);
    info!(
        "Attempting to create campaign directory: {:?}",
        campaign_path
    );
    fs::create_dir_all(&campaign_path)
        .map_err(|e| format!("Failed to create campaign directory: {}", e))?;
    info!("Campaign directory created/exists: {:?}", campaign_path);

    // Generate unique filename
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let filename = format!("generated_image_{}.png", timestamp);
    let output_path = campaign_path.join(filename);
    info!("Generated output path for image: {:?}", output_path);

    info!("Attempting to create and write image file.");
    let mut file = fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    file.write_all(&decoded_image_data)
        .map_err(|e| format!("Failed to write image data to file: {}", e))?;
    info!("Image file successfully written to: {:?}", output_path);

    permit
        .commit()
        .map_err(|e| format!("Failed to commit spend reservation: {}", e))?;

    Ok(output_path)
}
