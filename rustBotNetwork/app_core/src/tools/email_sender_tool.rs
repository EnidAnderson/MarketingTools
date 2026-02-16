use super::base_tool::BaseTool;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json::Value;
use std::env;
use std::error::Error;

pub struct EmailSenderTool;

impl EmailSenderTool {
    pub fn new() -> Self {
        EmailSenderTool
    }

    fn get_smtp_config(&self) -> Option<(String, u16, String, Credentials)> {
        let host = env::var("SMTP_HOST").ok()?;
        let port_str = env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string());
        let port: u16 = port_str.parse().ok()?;
        let username = env::var("SMTP_USERNAME").ok()?;
        let password = env::var("SMTP_PASSWORD").ok()?;

        Some((
            host,
            port,
            username.clone(),
            Credentials::new(username, password),
        ))
    }
}
#[async_trait]
impl BaseTool for EmailSenderTool {
    fn name(&self) -> &'static str {
        "EmailSenderTool"
    }

    fn description(&self) -> &'static str {
        "Sends emails to specified recipients. Requires 'to', 'subject', and 'body' in the input JSON. Optionally 'from' can be specified."
    }

    fn is_available(&self) -> bool {
        self.get_smtp_config().is_some()
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let (host, port, username, credentials) = match self.get_smtp_config() {
            Some(config) => config,
            None => {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": "SMTP configuration is incomplete. Missing one of: SMTP_HOST, SMTP_PORT, SMTP_USERNAME, SMTP_PASSWORD."
                }));
            }
        };

        let to_email = match input["to"].as_str() {
            Some(s) => s.to_string(),
            None => {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": "Recipient 'to' email is required and must be a string."
                }))
            }
        };
        let subject = input["subject"]
            .as_str()
            .unwrap_or("No Subject")
            .to_string();
        let body = input["body"].as_str().unwrap_or("").to_string();
        let from_email = input["from"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or(username); // Use the retrieved username as default from_email

        let email = Message::builder()
            .from(from_email.parse()?)
            .to(to_email.parse()?)
            .subject(subject)
            .body(body)?;

        let mailer = SmtpTransport::relay(&host)?
            .port(port)
            .credentials(credentials)
            .build();

        match mailer.send(&email) {
            Ok(_) => Ok(serde_json::json!({
                "status": "success",
                "message": format!("Email sent successfully to {}", to_email)
            })),
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;
    use std::sync::{Mutex, MutexGuard};
    use tokio; // Import tokio

    static SMTP_TEST_MUTEX: Mutex<()> = Mutex::new(());

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
            env::remove_var("SMTP_HOST");
            env::remove_var("SMTP_PORT");
            env::remove_var("SMTP_USERNAME");
            env::remove_var("SMTP_PASSWORD");
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

    fn smtp_test_guard() -> (MutexGuard<'static, ()>, TestEnvGuard) {
        let lock = SMTP_TEST_MUTEX.lock().expect("smtp test lock poisoned");
        let env_guard = TestEnvGuard::new();
        env::remove_var("SMTP_HOST");
        env::remove_var("SMTP_PORT");
        env::remove_var("SMTP_USERNAME");
        env::remove_var("SMTP_PASSWORD");
        (lock, env_guard)
    }

    #[test]
    fn test_is_available_true() {
        let (_lock, _guard) = smtp_test_guard();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("SMTP_HOST", "smtp.test.com");
        vars_to_set.insert("SMTP_PORT", "587");
        vars_to_set.insert("SMTP_USERNAME", "testuser@test.com");
        vars_to_set.insert("SMTP_PASSWORD", "testpass");
        _guard.set_vars(&vars_to_set); // Corrected call

        let tool = EmailSenderTool::new();
        assert!(tool.is_available());
    }

    #[test]
    fn test_is_available_false_missing_host() {
        let (_lock, _guard) = smtp_test_guard();
        let mut vars_to_set = HashMap::new();
        // Missing SMTP_HOST
        vars_to_set.insert("SMTP_PORT", "587");
        vars_to_set.insert("SMTP_USERNAME", "testuser@test.com");
        vars_to_set.insert("SMTP_PASSWORD", "testpass");
        _guard.set_vars(&vars_to_set); // Corrected call

        let tool = EmailSenderTool::new();
        assert!(!tool.is_available());
    }

    #[test]
    fn test_is_available_false_missing_username() {
        let (_lock, _guard) = smtp_test_guard();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("SMTP_HOST", "smtp.test.com");
        vars_to_set.insert("SMTP_PORT", "587");
        // Missing SMTP_USERNAME
        vars_to_set.insert("SMTP_PASSWORD", "testpass");
        _guard.set_vars(&vars_to_set); // Corrected call

        let tool = EmailSenderTool::new();
        assert!(!tool.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_to_email() {
        // Added async
        let (_lock, _guard) = smtp_test_guard();
        let mut vars_to_set = HashMap::new();
        vars_to_set.insert("SMTP_HOST", "smtp.test.com");
        vars_to_set.insert("SMTP_PORT", "587");
        vars_to_set.insert("SMTP_USERNAME", "testuser@test.com");
        vars_to_set.insert("SMTP_PASSWORD", "testpass");
        _guard.set_vars(&vars_to_set); // Corrected call

        let tool = EmailSenderTool::new();
        let input = serde_json::json!({
            "subject": "Test Subject",
            "body": "Test Body"
        });
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("Recipient 'to' email is required"));
    }

    // This test will attempt to send an email.
    // It will likely fail if no real SMTP server is configured and reachable.
    // It serves to ensure the code structure around lettre is correct.
    // For proper automated testing, a mock SMTP server or a mocked `lettre::Transport`
    // would be necessary.
    #[tokio::test] // Changed to tokio::test
    #[ignore = "Requires a live SMTP server configured via env vars to pass. Run manually."]
    async fn test_run_success_integration() {
        // Added async
        // IMPORTANT: For this test to pass, you need to set actual, valid SMTP credentials
        // in your environment before running `cargo test -- --ignored test_run_success_integration`
        // e.g., export SMTP_HOST="your.smtp.host" etc.
        // We do NOT use TestEnvGuard here for this test to allow real env vars to be used
        // if they are set by the user for this specific integration test.

        let tool = EmailSenderTool::new();
        // This test only makes sense if the tool is actually available (i.e., real env vars are set)
        if !tool.is_available() {
            eprintln!("Skipping test_run_success_integration: SMTP credentials not properly configured in environment.");
            return;
        }

        let input = serde_json::json!({
            "to": "recipient@example.com", // Replace with a valid recipient for actual test
            "subject": "Test Email from Rust",
            "body": "This is a test email sent using the Rust EmailSenderTool.",
            "from": "sender@example.com" // Optional, will use SMTP_USERNAME if not provided
        });

        let result = tool.run(input).await; // Added .await
        assert!(
            result.is_ok(),
            "Expected email sending to succeed or return a detailed error, but got: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert_eq!(output["status"], "success");
        assert!(output["message"]
            .as_str()
            .unwrap()
            .contains("Email sent successfully"));
    }
}
