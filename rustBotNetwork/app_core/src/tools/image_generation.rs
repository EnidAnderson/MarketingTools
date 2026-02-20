use crate::tools::generation_budget_manager;
use base64::engine::general_purpose;
use base64::Engine as _; // Import the Engine trait
use reqwest;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use url::Url; // Import the standard engine

pub fn generate_image(
    prompt: &str,
    campaign_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let api_base_str =
        env::var("STABILITY_API_BASE").unwrap_or_else(|_| "https://api.stability.ai".to_string());
    let mut api_url = Url::parse(&api_base_str)?;
    api_url
        .path_segments_mut()
        .map_err(|_| "cannot be a base")?
        .push("v1")
        .push("generation")
        .push("stable-diffusion-xl-1024-v1-0")
        .push("text-to-image");

    let api_key = env::var("STABILITY_API_KEY").map_err(|_| "STABILITY_API_KEY not set")?;

    let model_name = "stable-diffusion-xl-1024-v1-0";
    let cost = generation_budget_manager::estimate_image_cost_strict(model_name)
        .map_err(|e| format!("failed to estimate model cost: {e}"))?;
    let reservation = generation_budget_manager::reserve_for_paid_call(
        cost,
        "image_generation",
        "stability",
        model_name,
    )
    .map_err(|e| format!("paid call blocked by spend governor: {e}"))?;

    let call_result: Result<String, Box<dyn std::error::Error>> = (|| {
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(api_url.as_str())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "text_prompts": [{
                    "text": prompt
                }],
                "cfg_scale": 7,
                "height": 1024,
                "width": 1024,
                "samples": 1,
                "steps": 30,
            }))
            .send()?;

        if response.status().is_success() {
            let body: serde_json::Value = response.json()?;
            if let Some(artifacts) = body.get("artifacts").and_then(|a| a.as_array()) {
                for artifact in artifacts {
                    if let Some(base64) = artifact.get("base64").and_then(|b| b.as_str()) {
                        let image_data = general_purpose::STANDARD.decode(base64)?;
                        let img_path = PathBuf::from(campaign_dir).join("generated_image.png");
                        fs::write(&img_path, &image_data)?;
                        return Ok(img_path.to_string_lossy().into_owned());
                    }
                }
            }
            Err("No image data found in response".into())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Could not read error body".to_string());
            Err(format!("Failed to generate image: {} - {}", status, error_text).into())
        }
    })();

    match call_result {
        Ok(path) => {
            generation_budget_manager::commit_paid_call(&reservation)
                .map_err(|e| format!("failed to commit spend reservation: {e}"))?;
            Ok(path)
        }
        Err(err) => {
            let _ = generation_budget_manager::refund_paid_call(&reservation);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http;
    use httptest::{Expectation, Server};
    // Correct httptest imports based on your guidance
    use httptest::matchers::{all_of, contains, eq, json_decoded, key, request};
    use httptest::responders::{json_encoded, status_code};

    use crate::tools::generation_budget_manager as budget_manager_mock;
    use std::io::Write;
    use std::path::{Path, PathBuf as StdPathBuf};
    use std::sync::{Mutex, MutexGuard};
    use tempfile::tempdir;

    static TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

    // Helper to create a dummy api_costs.json
    fn create_dummy_api_costs_file(dir: &Path, content: &str) -> StdPathBuf {
        let file_path = dir.join("api_costs.json");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    // Helper to create a dummy generation_budget.json
    fn create_dummy_budget_file(dir: &Path, content: &str) -> StdPathBuf {
        let file_path = dir.join("generation_budget.json");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    // Base64 encoded 1x1 transparent PNG
    const DUMMY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    fn test_guard() -> MutexGuard<'static, ()> {
        let guard = TEST_ENV_MUTEX.lock().expect("test lock poisoned");
        std::env::remove_var("STABILITY_API_BASE");
        std::env::remove_var("STABILITY_API_KEY");
        budget_manager_mock::set_test_api_costs_file_path(None);
        budget_manager_mock::set_test_budget_file_path(None);
        guard
    }

    #[test]
    fn test_generate_image_success() {
        let _guard = test_guard();
        let server = Server::run();
        std::env::set_var("STABILITY_API_BASE", server.url("").to_string());

        server.expect(
            Expectation::matching(all_of![
                request::method("POST"),
                request::path("/v1/generation/stable-diffusion-xl-1024-v1-0/text-to-image")
            ])
            .respond_with(
                // For 200 OK with JSON, json_encoded is enough. It defaults to 200 OK and sets content-type.
                json_encoded(json!({
                    "artifacts": [
                        {
                            "base64": DUMMY_PNG_BASE64,
                            "seed": 123,
                            "finishReason": "SUCCESS"
                        }
                    ]
                })),
            ),
        );

        // Setup mock budget manager files
        let temp_dir_instance = tempdir().unwrap();
        let api_costs_path = create_dummy_api_costs_file(
            temp_dir_instance.path(),
            r#"{"stable-diffusion-xl-1024-v1-0": {"per_image": 0.02}}"#,
        );
        let budget_path = create_dummy_budget_file(
            temp_dir_instance.path(),
            r#"{"daily_spend": 0.0, "daily_resets_on": "2099-01-01", "generations": []}"#,
        );
        budget_manager_mock::set_test_api_costs_file_path(Some(api_costs_path));
        budget_manager_mock::set_test_budget_file_path(Some(budget_path));

        // Set environment variable for the API key
        std::env::set_var("STABILITY_API_KEY", "test_api_key");

        let campaign_output_dir = temp_dir_instance.path().join("campaign_output");
        fs::create_dir_all(&campaign_output_dir).unwrap();
        let prompt = "a dog wearing a hat";
        let result = generate_image(prompt, campaign_output_dir.to_str().unwrap());

        assert!(result.is_ok());
        let img_path = result.unwrap();
        assert!(StdPathBuf::from(&img_path).exists());
        assert_eq!(budget_manager_mock::get_budget_state().daily_spend, 0.02);

        // Clean up environment variables
        std::env::remove_var("STABILITY_API_BASE");
        std::env::remove_var("STABILITY_API_KEY");
        budget_manager_mock::set_test_api_costs_file_path(None);
        budget_manager_mock::set_test_budget_file_path(None);
    }

    #[test]
    fn test_generate_image_no_api_key() {
        let _guard = test_guard();
        std::env::remove_var("STABILITY_API_KEY");

        let temp_dir_instance = tempdir().unwrap();
        let campaign_output_dir = temp_dir_instance.path().to_path_buf();
        let prompt = "a cat playing piano";
        let result = generate_image(prompt, campaign_output_dir.to_str().unwrap());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("STABILITY_API_KEY not set"));
    }

    #[test]
    fn test_generate_image_budget_exceeded() {
        let _guard = test_guard();
        let temp_dir_instance = tempdir().unwrap();
        let api_costs_path = create_dummy_api_costs_file(
            temp_dir_instance.path(),
            r#"{"stable-diffusion-xl-1024-v1-0": {"per_image": 0.02}}"#,
        );
        let budget_path = create_dummy_budget_file(
            temp_dir_instance.path(),
            r#"{"daily_spend": 9999.0, "daily_resets_on": "2099-01-01", "generations": []}"#,
        );
        budget_manager_mock::set_test_api_costs_file_path(Some(api_costs_path));
        budget_manager_mock::set_test_budget_file_path(Some(budget_path));

        std::env::set_var("STABILITY_API_KEY", "test_api_key_budget");

        let campaign_output_dir = temp_dir_instance.path().to_path_buf();
        let prompt = "a small bird";
        let result = generate_image(prompt, campaign_output_dir.to_str().unwrap());

        std::env::remove_var("STABILITY_API_KEY");
        budget_manager_mock::set_test_api_costs_file_path(None);
        budget_manager_mock::set_test_budget_file_path(None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("paid call blocked"));
    }

    #[test]
    fn test_generate_image_api_error() {
        let _guard = test_guard();
        let server = Server::run();
        std::env::set_var("STABILITY_API_BASE", server.url("").to_string());
        server.expect(
            Expectation::matching(request::method("POST")).respond_with(
                status_code(500)
                    .body(
                        serde_json::to_string(&json!({"message": "API rate limit exceeded"}))
                            .unwrap(),
                    )
                    .insert_header("Content-Type", "application/json"),
            ),
        );

        let temp_dir_instance = tempdir().unwrap();
        let api_costs_path = create_dummy_api_costs_file(
            temp_dir_instance.path(),
            r#"{"stable-diffusion-xl-1024-v1-0": {"per_image": 0.02}}"#,
        );
        let budget_path = create_dummy_budget_file(
            temp_dir_instance.path(),
            r#"{"daily_spend": 0.0, "daily_resets_on": "2099-01-01", "generations": []}"#,
        );
        budget_manager_mock::set_test_api_costs_file_path(Some(api_costs_path));
        budget_manager_mock::set_test_budget_file_path(Some(budget_path));

        std::env::set_var("STABILITY_API_KEY", "test_api_key_error");

        let campaign_output_dir = temp_dir_instance.path().to_path_buf();
        let prompt = "an alien spacehip";
        let result = generate_image(prompt, campaign_output_dir.to_str().unwrap());

        std::env::remove_var("STABILITY_API_KEY");
        budget_manager_mock::set_test_api_costs_file_path(None);
        budget_manager_mock::set_test_budget_file_path(None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to generate image: 500 Internal Server Error"));
    }

    #[test]
    fn test_generate_image_no_image_data_in_response() {
        let _guard = test_guard();
        let server = Server::run();
        std::env::set_var("STABILITY_API_BASE", server.url("").to_string());
        server.expect(
            Expectation::matching(request::method("POST")).respond_with(json_encoded(json!({
                "artifacts": [
                    {
                        "seed": 456,
                        "finishReason": "ERROR"
                    }
                ]
            }))),
        );

        let temp_dir_instance = tempdir().unwrap();
        let api_costs_path = create_dummy_api_costs_file(
            temp_dir_instance.path(),
            r#"{"stable-diffusion-xl-1024-v1-0": {"per_image": 0.02}}"#,
        );
        let budget_path = create_dummy_budget_file(
            temp_dir_instance.path(),
            r#"{"daily_spend": 0.0, "daily_resets_on": "2099-01-01", "generations": []}"#,
        );
        budget_manager_mock::set_test_api_costs_file_path(Some(api_costs_path));
        budget_manager_mock::set_test_budget_file_path(Some(budget_path));

        std::env::set_var("STABILITY_API_KEY", "test_api_key_no_data");

        let campaign_output_dir = temp_dir_instance.path().to_path_buf();
        let prompt = "a flying car";
        let result = generate_image(prompt, campaign_output_dir.to_str().unwrap());

        std::env::remove_var("STABILITY_API_KEY");
        budget_manager_mock::set_test_api_costs_file_path(None);
        budget_manager_mock::set_test_budget_file_path(None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No image data found in response"));
    }
}
