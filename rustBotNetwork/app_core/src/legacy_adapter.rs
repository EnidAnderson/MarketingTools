use crate::contracts::{ToolError, ToolErrorKind, ToolResult};
use serde_json::json;
use std::error::Error;
use std::future::Future;

pub fn adapt_legacy_error(error: &(dyn Error + 'static)) -> ToolError {
    if error.downcast_ref::<std::io::Error>().is_some() {
        return ToolError::new(
            ToolErrorKind::InternalError,
            "I/O operation failed",
            true,
            Some(json!({"debug": error.to_string()})),
        );
    }

    let msg = error.to_string();
    let lower = msg.to_lowercase();

    if lower.contains("rate limit") || lower.contains("429") {
        return ToolError::new(
            ToolErrorKind::RateLimitError,
            "Provider rate limit exceeded",
            true,
            Some(json!({"debug": msg})),
        );
    }

    if lower.contains("timeout") || lower.contains("timed out") {
        return ToolError::new(
            ToolErrorKind::TimeoutError,
            "Provider request timed out",
            true,
            Some(json!({"debug": msg})),
        );
    }

    if lower.contains("permission") || lower.contains("forbidden") || lower.contains("unauthorized")
    {
        return ToolError::new(
            ToolErrorKind::PermissionError,
            "Permission denied while executing tool",
            false,
            Some(json!({"debug": msg})),
        );
    }

    ToolError::new(ToolErrorKind::InternalError, msg, false, None)
}

pub async fn run_legacy_tool<F, T>(tool_future: F) -> ToolResult<T>
where
    F: Future<Output = Result<T, Box<dyn Error + Send + Sync>>>,
{
    tool_future
        .await
        .map_err(|error| adapt_legacy_error(error.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn maps_rate_limit_to_retryable() {
        let result: ToolResult<()> = run_legacy_tool(async {
            Err::<(), Box<dyn Error + Send + Sync>>("rate limit exceeded".into())
        })
        .await;

        let err = result.expect_err("expected mapped error");
        assert!(matches!(err.kind, ToolErrorKind::RateLimitError));
        assert!(err.retryable);
    }
}
