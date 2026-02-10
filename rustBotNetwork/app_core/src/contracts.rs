use serde::{Deserialize, Serialize};
use serde_json::Value;

/// # NDOC
/// component: `contracts`
/// purpose: Standard result alias for typed tool contracts.
pub type ToolResult<T> = Result<T, ToolError>;

/// # NDOC
/// component: `contracts`
/// purpose: Stable machine-readable tool error category.
/// invariants:
///   - Variants are part of external API surface; changes require compatibility review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolErrorKind {
    ValidationError,
    ConfigurationError,
    ProviderError,
    RateLimitError,
    TimeoutError,
    PermissionError,
    InternalError,
}

/// # NDOC
/// component: `contracts`
/// purpose: Canonical error payload used across tools and runtimes.
/// invariants:
///   - `message` is user-safe.
///   - `retryable` indicates whether automated retry is acceptable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub kind: ToolErrorKind,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

impl ToolError {
    pub fn new(
        kind: ToolErrorKind,
        message: impl Into<String>,
        retryable: bool,
        details: Option<Value>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
            details,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ValidationError, message, false, None)
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ConfigurationError, message, false, None)
    }

    pub fn provider(message: impl Into<String>, retryable: bool) -> Self {
        Self::new(ToolErrorKind::ProviderError, message, retryable, None)
    }

    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::RateLimitError, message, true, None)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::TimeoutError, message, true, None)
    }

    pub fn permission(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::PermissionError, message, false, None)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::InternalError, message, false, None)
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn from_legacy_error(message: impl Into<String>) -> Self {
        Self::internal(message)
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ToolError {}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        ToolError::internal(value.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(value: serde_json::Error) -> Self {
        ToolError::validation(value.to_string())
    }
}

/// # NDOC
/// component: `contracts`
/// purpose: Declares a typed tool request/response contract.
pub trait ToolContract {
    const NAME: &'static str;
    const VERSION: &'static str;
    type Input: serde::de::DeserializeOwned + Send + Sync + 'static;
    type Output: Serialize + Send + Sync + 'static;
}

/// # NDOC
/// component: `contracts`
/// purpose: Trait for strongly typed tool implementations.
#[async_trait::async_trait]
pub trait TypedTool<C: ToolContract>: Send + Sync {
    async fn run_typed(&self, input: C::Input) -> ToolResult<C::Output>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_error_builders_set_expected_values() {
        let err = ToolError::timeout("provider timed out");
        assert_eq!(err.kind, ToolErrorKind::TimeoutError);
        assert!(err.retryable);
        assert_eq!(err.message, "provider timed out");
    }
}
