use super::marketing_platform_manager::MarketingPlatformAdapterTrait;
use async_trait::async_trait;
#[allow(unused_imports)] // json! macro is used in deploy_campaign
use serde_json::json;
use serde_json::Value;
#[allow(unused_imports)] // Used by TestEnvGuard implicitly
use std::collections::HashMap; // Import HashMap for TestEnvGuard
use std::env;
#[allow(unused_imports)] // Used by TestEnvGuard implicitly
use std::env::{remove_var, set_var}; // Import set_var and remove_var
use std::error::Error; // Add json macro import

pub struct GoogleAdsAdapter;

impl GoogleAdsAdapter {
    pub fn new() -> Self {
        GoogleAdsAdapter
    }
}

#[async_trait]
impl MarketingPlatformAdapterTrait for GoogleAdsAdapter {
    fn name(&self) -> &'static str {
        "GoogleAds"
    }

    fn is_available(&self) -> bool {
        env::var("GOOGLE_ADS_API_KEY").is_ok()
    }

    async fn deploy_campaign(
        &self,
        _campaign_data: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Simulate API call, always success for Google Ads mock
        Ok(serde_json::json!({
            "status": "success",
            "platform": "GoogleAds",
            "message": "Campaign deployed to Google Ads (mock)."
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should be acquirable")
    }

    // Helper to snapshot and restore environment variables for test isolation
    struct TestEnvGuard {
        original_env: HashMap<String, String>,
    }

    impl TestEnvGuard {
        fn new() -> Self {
            let original_env: HashMap<String, String> = env::vars().collect();
            TestEnvGuard { original_env }
        }

        fn set_vars(&self, vars_to_set: &HashMap<&str, &str>) {
            for (key, value) in vars_to_set {
                env::set_var(key, value);
            }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            let current_env: HashMap<String, String> = env::vars().collect();
            for (key, _value) in current_env {
                if !self.original_env.contains_key(&key) {
                    env::remove_var(&key);
                }
            }
            for (key, value) in &self.original_env {
                env::set_var(key, value);
            }
        }
    }

    #[test]
    fn test_google_ads_adapter_name() {
        let adapter = GoogleAdsAdapter::new();
        assert_eq!(adapter.name(), "GoogleAds");
    }

    #[test]
    fn test_google_ads_adapter_is_available_true() {
        let _lock = env_lock();
        let _guard = TestEnvGuard::new();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("GOOGLE_ADS_API_KEY", "test_key");
        _guard.set_vars(&vars_to_set);

        let adapter = GoogleAdsAdapter::new();
        assert!(adapter.is_available());
    }

    #[test]
    fn test_google_ads_adapter_is_available_false() {
        let _lock = env_lock();
        let _guard = TestEnvGuard::new();
        env::remove_var("GOOGLE_ADS_API_KEY"); // Ensure it's not set
        let adapter = GoogleAdsAdapter::new();
        assert!(!adapter.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_google_ads_adapter_deploy_campaign_success() {
        let _lock = env_lock();
        // Added async
        let _guard = TestEnvGuard::new();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("GOOGLE_ADS_API_KEY", "test_key");
        _guard.set_vars(&vars_to_set);

        let adapter = GoogleAdsAdapter::new();
        let campaign_data = json!({"name": "Test Campaign", "budget": 2000});
        let result = adapter.deploy_campaign(campaign_data).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        assert_eq!(result["platform"], "GoogleAds");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("deployed to Google Ads"));
    }
}
