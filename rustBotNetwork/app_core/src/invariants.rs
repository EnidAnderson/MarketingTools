use crate::contracts::ToolError;
// provenance: decision_id=DEC-0003; change_request_id=CR-WHITE-0012

/// # NDOC
/// component: `app_core::invariants`
/// purpose: Shared invariant checks used by multiple subsystems.
/// invariants:
///   - Validation helpers must return `ToolError::validation` for caller-safe failures.
///   - Helpers must not perform I/O or mutate external state.
pub fn ensure_non_empty_trimmed(value: &str, field: &str) -> Result<(), ToolError> {
    if value.trim().is_empty() {
        return Err(ToolError::validation(format!(
            "'{}' cannot be empty",
            field
        )));
    }
    Ok(())
}

/// # NDOC
/// component: `app_core::invariants`
/// purpose: Enforce an inclusive numeric bound with a user-safe error.
/// invariants:
///   - `name` is used directly in error messages and should be stable.
pub fn ensure_range_usize(
    value: usize,
    min: usize,
    max: usize,
    name: &str,
) -> Result<(), ToolError> {
    if value < min || value > max {
        return Err(ToolError::validation(format!(
            "'{}' must be in range {}..={}",
            name, min, max
        )));
    }
    Ok(())
}

/// # NDOC
/// component: `app_core::invariants`
/// purpose: Ensure JSON pointer strings are explicit and deterministic.
/// invariants:
///   - JSON pointer paths must start with `/` to avoid ambiguous parsing.
pub fn ensure_json_pointer(path: &str, field: &str) -> Result<(), ToolError> {
    if !path.starts_with('/') {
        return Err(ToolError::validation(format!(
            "'{}' must be a JSON pointer starting with '/'",
            field
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Naming scheme: inv_global_<domain>_<nnn>_<behavior>
    #[test]
    fn inv_global_core_001_rejects_empty_trimmed_string() {
        let err = ensure_non_empty_trimmed("   ", "topic").expect_err("must fail");
        assert!(err.message.contains("topic"));
    }

    #[test]
    fn inv_global_core_002_validates_usize_range() {
        assert!(ensure_range_usize(5, 1, 10, "max_sources").is_ok());
        assert!(ensure_range_usize(11, 1, 10, "max_sources").is_err());
    }

    #[test]
    fn inv_global_core_003_validates_json_pointer_prefix() {
        assert!(ensure_json_pointer("/foo/bar", "path").is_ok());
        assert!(ensure_json_pointer("foo/bar", "path").is_err());
    }
}
