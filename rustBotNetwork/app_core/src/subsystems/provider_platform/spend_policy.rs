use crate::tools::generation_budget_manager::HARD_DAILY_BUDGET_USD;

/// # NDOC
/// component: `subsystems::provider_platform::spend_policy`
/// purpose: Canonical spend-enforcement policy surface for paid provider calls.
/// invariants:
///   - All paid API calls must reserve spend via `PaidCallPermit::reserve(...)`.
///   - Enforced hard daily cap may not exceed `$10.00`.
///   - Any call path that does not commit a permit must auto-refund.
pub const ENFORCED_HARD_DAILY_CAP_USD: f64 = HARD_DAILY_BUDGET_USD;

/// # NDOC
/// component: `subsystems::provider_platform::spend_policy`
/// purpose: Explicit marker string for required paid-call reservation API.
pub const REQUIRED_RESERVATION_API: &str = "PaidCallPermit::reserve";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_cap_contract_is_ten_usd() {
        assert!((ENFORCED_HARD_DAILY_CAP_USD - 10.0).abs() < f64::EPSILON);
    }
}
