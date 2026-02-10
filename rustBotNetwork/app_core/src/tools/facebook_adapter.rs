use super::marketing_platform_manager::MarketingPlatformAdapterTrait;
use async_trait::async_trait;
use serde_json::Value;
use std::env;
use std::error::Error;

pub struct FacebookAdapter;

impl FacebookAdapter {
    pub fn new() -> Self {
        FacebookAdapter
    }
}

#[async_trait]
impl MarketingPlatformAdapterTrait for FacebookAdapter {
    fn name(&self) -> &'static str {
        "Facebook"
    }

    fn is_available(&self) -> bool {
        env::var("FACEBOOK_API_KEY").is_ok()
    }

    async fn deploy_campaign(
        &self,
        campaign_data: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Simulate API call
        let budget = campaign_data["budget"].as_u64().unwrap_or(0);

        if budget > 1000 {
            Ok(serde_json::json!({
                "status": "success",
                "platform": "Facebook",
                "message": "Campaign deployed to Facebook (mock)."
            }))
        } else {
            Ok(serde_json::json!({
                "status": "error",
                "platform": "Facebook",
                "message": "Facebook campaign deployment failed (mock): budget too low."
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    #[allow(unused_imports)] // Used by TestEnvGuard implicitly
    use std::env::{remove_var, set_var};

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
    fn test_facebook_adapter_name() {
        let adapter = FacebookAdapter::new();
        assert_eq!(adapter.name(), "Facebook");
    }

    #[test]
    fn test_facebook_adapter_is_available_true() {
        let _guard = TestEnvGuard::new();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("FACEBOOK_API_KEY", "test_key");
        _guard.set_vars(&vars_to_set);

        let adapter = FacebookAdapter::new();
        assert!(adapter.is_available());
    }

    #[test]
    fn test_facebook_adapter_is_available_false() {
        let _guard = TestEnvGuard::new();
        env::remove_var("FACEBOOK_API_KEY"); // Ensure it's not set
        let adapter = FacebookAdapter::new();
        assert!(!adapter.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_facebook_adapter_deploy_campaign_success() {
        // Added async
        let _guard = TestEnvGuard::new();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("FACEBOOK_API_KEY", "test_key");
        _guard.set_vars(&vars_to_set);

        let adapter = FacebookAdapter::new();
        let campaign_data = json!({"name": "Test Campaign", "budget": 1500});
        let result = adapter.deploy_campaign(campaign_data).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        assert_eq!(result["platform"], "Facebook");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("deployed to Facebook"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_facebook_adapter_deploy_campaign_failure_budget_low() {
        // Added async
        let _guard = TestEnvGuard::new();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("FACEBOOK_API_KEY", "test_key");
        _guard.set_vars(&vars_to_set);

        let adapter = FacebookAdapter::new();
        let campaign_data = json!({"name": "Test Campaign", "budget": 500});
        let result = adapter.deploy_campaign(campaign_data).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert_eq!(result["platform"], "Facebook");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("budget too low"));
    }
}
