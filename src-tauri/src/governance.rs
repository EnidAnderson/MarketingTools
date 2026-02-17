use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `tauri_governance::budget_envelope`
/// purpose: Required budget declaration for governed execution entrypoints.
/// invariants:
///   - Caps must be positive.
///   - `run_id`, `workflow_id`, and `subsystem` cannot be empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEnvelope {
    pub run_id: String,
    pub workflow_id: String,
    pub subsystem: String,
    pub per_run_cap_usd: f64,
    pub daily_cap_usd: f64,
    pub monthly_cap_usd: f64,
    pub stop_condition: String,
    pub fallback_mode: String,
    pub owner_role: String,
}

/// # NDOC
/// component: `tauri_governance::release_gate_input`
/// purpose: Strongly typed gate state for pre-execution governance checks.
/// invariants:
///   - `*_gate` values must be one of `green`, `yellow`, `red`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseGateInput {
    pub release_id: String,
    pub scope: String,
    pub security_gate: String,
    pub budget_gate: String,
    pub evidence_gate: String,
    pub role_gate: String,
    pub change_gate: String,
    pub checked_by: String,
    pub checked_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceValidationResult {
    pub ok: bool,
    pub errors: Vec<String>,
}

fn is_valid_gate_value(value: &str) -> bool {
    matches!(value, "green" | "yellow" | "red")
}

fn gate_is_publish_blocking(value: &str) -> bool {
    value == "red"
}

pub fn validate_budget_envelope(input: &BudgetEnvelope) -> GovernanceValidationResult {
    let mut errors = Vec::new();

    if input.run_id.trim().is_empty() {
        errors.push("budget.run_id cannot be empty".to_string());
    }
    if input.workflow_id.trim().is_empty() {
        errors.push("budget.workflow_id cannot be empty".to_string());
    }
    if input.subsystem.trim().is_empty() {
        errors.push("budget.subsystem cannot be empty".to_string());
    }
    if input.stop_condition.trim().is_empty() {
        errors.push("budget.stop_condition cannot be empty".to_string());
    }
    if input.fallback_mode.trim().is_empty() {
        errors.push("budget.fallback_mode cannot be empty".to_string());
    }
    if input.owner_role.trim().is_empty() {
        errors.push("budget.owner_role cannot be empty".to_string());
    }
    if input.per_run_cap_usd <= 0.0 {
        errors.push("budget.per_run_cap_usd must be > 0".to_string());
    }
    if input.daily_cap_usd <= 0.0 {
        errors.push("budget.daily_cap_usd must be > 0".to_string());
    }
    if input.monthly_cap_usd <= 0.0 {
        errors.push("budget.monthly_cap_usd must be > 0".to_string());
    }

    GovernanceValidationResult {
        ok: errors.is_empty(),
        errors,
    }
}

pub fn validate_release_gates(input: &ReleaseGateInput) -> GovernanceValidationResult {
    let mut errors = Vec::new();

    if input.release_id.trim().is_empty() {
        errors.push("release_id cannot be empty".to_string());
    }
    if input.scope.trim().is_empty() {
        errors.push("scope cannot be empty".to_string());
    }
    if input.checked_by.trim().is_empty() {
        errors.push("checked_by cannot be empty".to_string());
    }
    if input.checked_at_utc.trim().is_empty() {
        errors.push("checked_at_utc cannot be empty".to_string());
    }

    for (name, value) in [
        ("security_gate", input.security_gate.as_str()),
        ("budget_gate", input.budget_gate.as_str()),
        ("evidence_gate", input.evidence_gate.as_str()),
        ("role_gate", input.role_gate.as_str()),
        ("change_gate", input.change_gate.as_str()),
    ] {
        if !is_valid_gate_value(value) {
            errors.push(format!("{name} must be one of green|yellow|red"));
        }
    }

    for (name, value) in [
        ("security_gate", input.security_gate.as_str()),
        ("budget_gate", input.budget_gate.as_str()),
        ("evidence_gate", input.evidence_gate.as_str()),
        ("role_gate", input.role_gate.as_str()),
        ("change_gate", input.change_gate.as_str()),
    ] {
        if gate_is_publish_blocking(value) {
            errors.push(format!("{name} is red; publish/execution blocked"));
        }
    }

    GovernanceValidationResult {
        ok: errors.is_empty(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_gate_rejects_red() {
        let input = ReleaseGateInput {
            release_id: "rel-1".to_string(),
            scope: "internal".to_string(),
            security_gate: "green".to_string(),
            budget_gate: "red".to_string(),
            evidence_gate: "green".to_string(),
            role_gate: "green".to_string(),
            change_gate: "green".to_string(),
            checked_by: "team_lead".to_string(),
            checked_at_utc: "2026-02-10T00:00:00Z".to_string(),
        };
        let result = validate_release_gates(&input);
        assert!(!result.ok);
        assert!(result.errors.iter().any(|e| e.contains("budget_gate")));
    }

    #[test]
    fn budget_envelope_rejects_non_positive_caps() {
        let budget = BudgetEnvelope {
            run_id: "run-1".to_string(),
            workflow_id: "wf-1".to_string(),
            subsystem: "marketing_data_analysis".to_string(),
            per_run_cap_usd: 0.0,
            daily_cap_usd: 10.0,
            monthly_cap_usd: 100.0,
            stop_condition: "hard_stop".to_string(),
            fallback_mode: "reduced_scope".to_string(),
            owner_role: "team_lead".to_string(),
        };
        let result = validate_budget_envelope(&budget);
        assert!(!result.ok);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("per_run_cap_usd must be > 0")));
    }
}
