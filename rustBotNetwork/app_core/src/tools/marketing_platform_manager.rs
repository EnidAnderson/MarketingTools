use super::base_tool::BaseTool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

/// Trait defining the interface for a Marketing Platform Adapter.
/// Concrete adapters will implement this to deploy campaigns to specific platforms.
#[async_trait]
pub trait MarketingPlatformAdapterTrait: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    async fn deploy_campaign(
        &self,
        campaign_data: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>>;
}

/// A Marketing Platform Manager tool that deploys campaigns to various platforms
/// via their respective adapters.
pub struct MarketingPlatformManager {
    adapters: HashMap<String, Box<dyn MarketingPlatformAdapterTrait>>,
}

impl MarketingPlatformManager {
    pub fn new() -> Self {
        MarketingPlatformManager {
            adapters: HashMap::new(),
        }
    }

    pub fn register_adapter(&mut self, adapter: Box<dyn MarketingPlatformAdapterTrait>) {
        self.adapters.insert(adapter.name().to_string(), adapter);
    }

    // This is essentially the `run` logic from the Python manager,
    // but the BaseTool trait will wrap it.
    async fn _run_manager_logic(
        &self,
        input: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let platform_name = input["platform"].as_str();
        let campaign_data = input["campaign_data"].clone();

        if platform_name.is_none() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "Platform name is required."
            }));
        }

        let platform_name_str = platform_name.unwrap();
        let adapter = self.adapters.get(platform_name_str);

        if adapter.is_none() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": format!("Unknown marketing platform: {}", platform_name_str)
            }));
        }

        let adapter_instance = adapter.unwrap();

        if !adapter_instance.is_available() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": format!("Marketing platform '{}' is not available (API keys missing?).", platform_name_str)
            }));
        }

        adapter_instance.deploy_campaign(campaign_data).await
    }
}

#[async_trait]
impl BaseTool for MarketingPlatformManager {
    fn name(&self) -> &'static str {
        "MarketingPlatformManager"
    }

    fn description(&self) -> &'static str {
        "Manages deployment of marketing campaigns to various platforms (e.g., Facebook, Google Ads). Input requires 'platform' and 'campaign_data'."
    }

    fn is_available(&self) -> bool {
        // The manager is available if at least one registered adapter is available.
        self.adapters.values().any(|adapter| adapter.is_available())
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self._run_manager_logic(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex}; // For shared ownership in mock adapters

    // Mock adapter for testing MarketingPlatformManager
    struct MockAdapter {
        name_val: &'static str,
        is_available_val: Mutex<bool>,
        deploy_return_value: Mutex<Value>, // Store the Value directly
    }

    impl MockAdapter {
        fn new(name: &'static str, available: bool) -> Self {
            MockAdapter {
                name_val: name,
                is_available_val: Mutex::new(available),
                deploy_return_value: Mutex::new(
                    json!({"status": "mock_success", "platform": name}),
                ),
            }
        }
    }

    #[async_trait] // Added this
    impl MarketingPlatformAdapterTrait for MockAdapter {
        fn name(&self) -> &'static str {
            self.name_val
        }
        fn is_available(&self) -> bool {
            *self.is_available_val.lock().unwrap()
        }
        async fn deploy_campaign(
            &self,
            _campaign_data: Value,
        ) -> Result<Value, Box<dyn Error + Send + Sync>> {
            // Added async
            Ok(self.deploy_return_value.lock().unwrap().clone()) // Return Ok with the stored Value
        }
    }

    // Implement trait for Arc<MockAdapter> to be used in Box<dyn Trait>
    #[async_trait]
    impl MarketingPlatformAdapterTrait for Arc<MockAdapter> {
        fn name(&self) -> &'static str {
            self.as_ref().name()
        }
        fn is_available(&self) -> bool {
            self.as_ref().is_available()
        }
        async fn deploy_campaign(
            &self,
            campaign_data: Value,
        ) -> Result<Value, Box<dyn Error + Send + Sync>> {
            // Added async
            self.as_ref().deploy_campaign(campaign_data).await
        }
    }

    #[test]
    fn test_manager_name() {
        let manager = MarketingPlatformManager::new();
        assert_eq!(manager.name(), "MarketingPlatformManager");
    }

    #[test]
    fn test_register_adapter() {
        let mut manager = MarketingPlatformManager::new();
        let adapter = Box::new(Arc::new(MockAdapter::new("TestPlatform", true)));
        manager.register_adapter(adapter);
        assert!(manager.adapters.contains_key("TestPlatform"));
    }

    #[test]
    fn test_manager_is_available_true_one_available() {
        let mut manager = MarketingPlatformManager::new();
        manager.register_adapter(Box::new(Arc::new(MockAdapter::new("Unavailable", false))));
        manager.register_adapter(Box::new(Arc::new(MockAdapter::new("Available", true))));
        assert!(manager.is_available());
    }

    #[test]
    fn test_manager_is_available_false_none_available() {
        let mut manager = MarketingPlatformManager::new();
        manager.register_adapter(Box::new(Arc::new(MockAdapter::new("Unavailable1", false))));
        manager.register_adapter(Box::new(Arc::new(MockAdapter::new("Unavailable2", false))));
        assert!(!manager.is_available());
    }

    #[test]
    fn test_manager_is_available_false_empty_manager() {
        let manager = MarketingPlatformManager::new();
        assert!(!manager.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_platform_name() {
        // Added async
        let manager = MarketingPlatformManager::new();
        let input = serde_json::json!({
            "campaign_data": {"budget": 100}
        });
        let result = manager.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Platform name is required."));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_unknown_platform() {
        // Added async
        let manager = MarketingPlatformManager::new();
        let input = serde_json::json!({
            "platform": "UnknownPlatform",
            "campaign_data": {"budget": 100}
        });
        let result = manager.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Unknown marketing platform"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_unavailable_platform() {
        // Added async
        let mut manager = MarketingPlatformManager::new();
        let mock_adapter_rc = Arc::new(MockAdapter::new("TestPlatform", false)); // Set as unavailable
        manager.register_adapter(Box::new(mock_adapter_rc.clone()));

        let input = serde_json::json!({
            "platform": "TestPlatform",
            "campaign_data": {"budget": 100}
        });
        let result = manager.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("is not available"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_successful_deployment() {
        // Added async
        let mut manager = MarketingPlatformManager::new();
        let mock_adapter_rc = Arc::new(MockAdapter::new("TestPlatform", true)); // Set as available
        manager.register_adapter(Box::new(mock_adapter_rc.clone()));

        let input = serde_json::json!({
            "platform": "TestPlatform",
            "campaign_data": {"budget": 100}
        });
        let result = manager.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "mock_success"); // Based on MockAdapter's deploy_campaign
        assert_eq!(result["platform"], "TestPlatform");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_adapter_deployment_failure() {
        // Added async
        let mut manager = MarketingPlatformManager::new();
        let mock_adapter_rc = Arc::new(MockAdapter::new("TestPlatform", true));
        // Configure mock adapter to return an error from deploy_campaign
        *mock_adapter_rc.deploy_return_value.lock().unwrap() = json!({
            "status": "adapter_error",
            "message": "Simulated adapter failure"
        });
        manager.register_adapter(Box::new(mock_adapter_rc.clone()));

        let input = serde_json::json!({
            "platform": "TestPlatform",
            "campaign_data": {"budget": 50}
        });
        let result = manager.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "adapter_error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Simulated adapter failure"));
    }
}
