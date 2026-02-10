use async_trait::async_trait;
use serde_json::Value;

/// # NDOC
/// component: `tools::base_tool`
/// purpose: Runtime-facing trait used by registry and Tauri execution paths.
/// invariants:
///   - `name()` is stable and unique across active tools.
///   - `run()` must be side-effect bounded and return deterministic schema for same input shape.
#[async_trait]
pub trait BaseTool: Send + Sync {
    /// # NDOC
    /// component: `tools::base_tool::name`
    /// purpose: Stable identifier used by registry lookup and frontend invocation.
    fn name(&self) -> &'static str;

    /// # NDOC
    /// component: `tools::base_tool::description`
    /// purpose: Human-readable description shown in tool discovery surfaces.
    fn description(&self) -> &'static str;

    /// # NDOC
    /// component: `tools::base_tool::is_available`
    /// purpose: Runtime availability check (config, credentials, environment).
    fn is_available(&self) -> bool;

    /// # NDOC
    /// component: `tools::base_tool::run`
    /// purpose: Execute tool with dynamic JSON input at runtime boundary.
    /// invariants:
    ///   - Must return `Err` for invalid input instead of panicking.
    ///   - On success, output must be JSON-serializable and schema-stable for consumers.
    async fn run(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
}
