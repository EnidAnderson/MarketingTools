use super::spend_policy::ENFORCED_HARD_DAILY_CAP_USD;
use serde::{Deserialize, Serialize};

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Supported generation providers for route planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProviderV1 {
    OpenAi,
    Google,
    LocalMock,
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Capability class required by the requesting node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationCapabilityV1 {
    Text,
    Image,
    Video,
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Relative quality/cost tier for model selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ModelTierV1 {
    Nano,
    Mini,
    Standard,
    Premium,
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Routing candidate with explicit cost and token envelopes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRouteV1 {
    pub provider: GenerationProviderV1,
    pub model: String,
    pub capability: GenerationCapabilityV1,
    pub tier: ModelTierV1,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub cost_per_1k_input_usd: f64,
    pub cost_per_1k_output_usd: f64,
    pub rationale: String,
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Request-time budget envelope used by the model router.
/// invariants:
///   - Hard cap cannot exceed enforced `$10/day` policy.
///   - Max tokens must be non-zero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingBudgetEnvelopeV1 {
    pub remaining_daily_budget_usd: f64,
    pub max_cost_per_run_usd: f64,
    pub max_total_input_tokens: u32,
    pub max_total_output_tokens: u32,
    pub hard_daily_cap_usd: f64,
}

impl Default for RoutingBudgetEnvelopeV1 {
    fn default() -> Self {
        Self {
            remaining_daily_budget_usd: ENFORCED_HARD_DAILY_CAP_USD,
            max_cost_per_run_usd: 2.0,
            max_total_input_tokens: 24_000,
            max_total_output_tokens: 8_000,
            hard_daily_cap_usd: ENFORCED_HARD_DAILY_CAP_USD,
        }
    }
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Route selection request from orchestration/runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingRequestV1 {
    pub capability: GenerationCapabilityV1,
    pub complexity_score: u8,
    pub quality_priority: u8,
    pub latency_priority: u8,
    pub expected_input_tokens: u32,
    pub expected_output_tokens: u32,
    pub paid_calls_allowed: bool,
    pub allow_openai: bool,
    pub allow_google: bool,
    pub allow_mock_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRoutingError {
    pub code: String,
    pub message: String,
}

impl ModelRoutingError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ModelRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ModelRoutingError {}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Select the most economical route that satisfies capability and budget constraints.
/// invariants:
///   - If `paid_calls_allowed == false`, always returns local mock route.
///   - Never returns a paid route above budget caps.
pub fn route_model_v1(
    request: &RoutingRequestV1,
    budget: &RoutingBudgetEnvelopeV1,
) -> Result<ModelRouteV1, ModelRoutingError> {
    validate_budget(budget)?;
    validate_request(request)?;

    if !request.paid_calls_allowed {
        return Ok(local_mock_route(&request.capability));
    }

    let required_tier = required_tier(request.complexity_score, request.quality_priority);
    let mut candidates = model_catalog_v1()
        .into_iter()
        .filter(|candidate| candidate.capability == request.capability)
        .filter(|candidate| match candidate.provider {
            GenerationProviderV1::OpenAi => request.allow_openai,
            GenerationProviderV1::Google => request.allow_google,
            GenerationProviderV1::LocalMock => request.allow_mock_fallback,
        })
        .collect::<Vec<_>>();

    // Sort cheapest-first while preserving minimum required tier preference.
    candidates.sort_by(|a, b| {
        let a_meets = a.tier >= required_tier;
        let b_meets = b.tier >= required_tier;
        a_meets.cmp(&b_meets).reverse().then_with(|| {
            estimate_cost_v1(
                a,
                request.expected_input_tokens,
                request.expected_output_tokens,
            )
            .partial_cmp(&estimate_cost_v1(
                b,
                request.expected_input_tokens,
                request.expected_output_tokens,
            ))
            .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    for candidate in candidates {
        let estimated_cost = estimate_cost_v1(
            &candidate,
            request.expected_input_tokens,
            request.expected_output_tokens,
        );
        let within_tokens = request.expected_input_tokens <= candidate.max_input_tokens
            && request.expected_output_tokens <= candidate.max_output_tokens
            && request.expected_input_tokens <= budget.max_total_input_tokens
            && request.expected_output_tokens <= budget.max_total_output_tokens;
        let within_cost = estimated_cost <= budget.max_cost_per_run_usd
            && estimated_cost <= budget.remaining_daily_budget_usd
            && estimated_cost <= budget.hard_daily_cap_usd;

        if within_tokens && within_cost {
            return Ok(candidate);
        }
    }

    if request.allow_mock_fallback {
        return Ok(local_mock_route(&request.capability));
    }

    Err(ModelRoutingError::new(
        "no_affordable_route",
        "no provider route satisfies capability and budget envelope",
    ))
}

/// # NDOC
/// component: `subsystems::provider_platform::model_routing`
/// purpose: Deterministic cost estimate for route selection and planning.
pub fn estimate_cost_v1(route: &ModelRouteV1, input_tokens: u32, output_tokens: u32) -> f64 {
    ((input_tokens as f64 / 1000.0) * route.cost_per_1k_input_usd)
        + ((output_tokens as f64 / 1000.0) * route.cost_per_1k_output_usd)
}

fn validate_budget(budget: &RoutingBudgetEnvelopeV1) -> Result<(), ModelRoutingError> {
    if !budget.remaining_daily_budget_usd.is_finite() || budget.remaining_daily_budget_usd < 0.0 {
        return Err(ModelRoutingError::new(
            "invalid_budget",
            "remaining_daily_budget_usd must be finite and >= 0",
        ));
    }
    if !budget.max_cost_per_run_usd.is_finite() || budget.max_cost_per_run_usd <= 0.0 {
        return Err(ModelRoutingError::new(
            "invalid_budget",
            "max_cost_per_run_usd must be finite and > 0",
        ));
    }
    if budget.max_total_input_tokens == 0 || budget.max_total_output_tokens == 0 {
        return Err(ModelRoutingError::new(
            "invalid_budget",
            "max_total_input_tokens and max_total_output_tokens must be > 0",
        ));
    }
    if !budget.hard_daily_cap_usd.is_finite()
        || budget.hard_daily_cap_usd <= 0.0
        || budget.hard_daily_cap_usd > ENFORCED_HARD_DAILY_CAP_USD
    {
        return Err(ModelRoutingError::new(
            "invalid_budget",
            format!(
                "hard_daily_cap_usd must be > 0 and <= {:.2}",
                ENFORCED_HARD_DAILY_CAP_USD
            ),
        ));
    }
    Ok(())
}

fn validate_request(request: &RoutingRequestV1) -> Result<(), ModelRoutingError> {
    if !(1..=10).contains(&request.complexity_score)
        || !(1..=10).contains(&request.quality_priority)
        || !(1..=10).contains(&request.latency_priority)
    {
        return Err(ModelRoutingError::new(
            "invalid_request",
            "complexity_score, quality_priority, latency_priority must be in 1..=10",
        ));
    }
    if request.expected_input_tokens == 0 || request.expected_output_tokens == 0 {
        return Err(ModelRoutingError::new(
            "invalid_request",
            "expected_input_tokens and expected_output_tokens must be > 0",
        ));
    }
    Ok(())
}

fn required_tier(complexity_score: u8, quality_priority: u8) -> ModelTierV1 {
    if complexity_score >= 8 || quality_priority >= 9 {
        ModelTierV1::Standard
    } else if complexity_score >= 5 || quality_priority >= 6 {
        ModelTierV1::Mini
    } else {
        ModelTierV1::Nano
    }
}

fn local_mock_route(capability: &GenerationCapabilityV1) -> ModelRouteV1 {
    ModelRouteV1 {
        provider: GenerationProviderV1::LocalMock,
        model: "local.mock.det.v1".to_string(),
        capability: capability.clone(),
        tier: ModelTierV1::Nano,
        max_input_tokens: 64_000,
        max_output_tokens: 32_000,
        cost_per_1k_input_usd: 0.0,
        cost_per_1k_output_usd: 0.0,
        rationale: "paid calls disabled or no affordable paid route".to_string(),
    }
}

fn model_catalog_v1() -> Vec<ModelRouteV1> {
    vec![
        ModelRouteV1 {
            provider: GenerationProviderV1::OpenAi,
            model: "gpt-4o-mini".to_string(),
            capability: GenerationCapabilityV1::Text,
            tier: ModelTierV1::Mini,
            max_input_tokens: 128_000,
            max_output_tokens: 16_000,
            cost_per_1k_input_usd: 0.0006,
            cost_per_1k_output_usd: 0.0024,
            rationale: "default economical text route for most generation/critique tasks"
                .to_string(),
        },
        ModelRouteV1 {
            provider: GenerationProviderV1::OpenAi,
            model: "gpt-4.1-mini".to_string(),
            capability: GenerationCapabilityV1::Text,
            tier: ModelTierV1::Standard,
            max_input_tokens: 128_000,
            max_output_tokens: 16_000,
            cost_per_1k_input_usd: 0.0012,
            cost_per_1k_output_usd: 0.0048,
            rationale: "higher-quality route for complex planning and synthesis".to_string(),
        },
        ModelRouteV1 {
            provider: GenerationProviderV1::Google,
            model: "gemini-2.5-flash".to_string(),
            capability: GenerationCapabilityV1::Text,
            tier: ModelTierV1::Mini,
            max_input_tokens: 128_000,
            max_output_tokens: 8_000,
            cost_per_1k_input_usd: 0.0004,
            cost_per_1k_output_usd: 0.0016,
            rationale: "cross-provider economical fallback for text generation".to_string(),
        },
        ModelRouteV1 {
            provider: GenerationProviderV1::Google,
            model: "gemini-2.5-pro".to_string(),
            capability: GenerationCapabilityV1::Text,
            tier: ModelTierV1::Standard,
            max_input_tokens: 256_000,
            max_output_tokens: 16_000,
            cost_per_1k_input_usd: 0.0015,
            cost_per_1k_output_usd: 0.0060,
            rationale: "higher-depth synthesis route when quality priority dominates".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_request() -> RoutingRequestV1 {
        RoutingRequestV1 {
            capability: GenerationCapabilityV1::Text,
            complexity_score: 6,
            quality_priority: 6,
            latency_priority: 5,
            expected_input_tokens: 1200,
            expected_output_tokens: 600,
            paid_calls_allowed: true,
            allow_openai: true,
            allow_google: true,
            allow_mock_fallback: true,
        }
    }

    #[test]
    fn uses_local_mock_when_paid_calls_disabled() {
        let mut request = default_request();
        request.paid_calls_allowed = false;
        let route = route_model_v1(&request, &RoutingBudgetEnvelopeV1::default()).expect("route");
        assert_eq!(route.provider, GenerationProviderV1::LocalMock);
        assert!((estimate_cost_v1(&route, 1000, 500) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn prefers_affordable_mini_route_for_mid_complexity() {
        let route =
            route_model_v1(&default_request(), &RoutingBudgetEnvelopeV1::default()).expect("route");
        assert!(matches!(
            route.tier,
            ModelTierV1::Mini | ModelTierV1::Standard
        ));
    }

    #[test]
    fn falls_back_to_local_mock_when_budget_too_low() {
        let budget = RoutingBudgetEnvelopeV1 {
            remaining_daily_budget_usd: 0.0,
            max_cost_per_run_usd: 0.00001,
            max_total_input_tokens: 2000,
            max_total_output_tokens: 1000,
            hard_daily_cap_usd: ENFORCED_HARD_DAILY_CAP_USD,
        };
        let route = route_model_v1(&default_request(), &budget).expect("route");
        assert_eq!(route.provider, GenerationProviderV1::LocalMock);
    }

    #[test]
    fn rejects_hard_cap_above_policy_limit() {
        let budget = RoutingBudgetEnvelopeV1 {
            hard_daily_cap_usd: ENFORCED_HARD_DAILY_CAP_USD + 0.01,
            ..RoutingBudgetEnvelopeV1::default()
        };
        let err = route_model_v1(&default_request(), &budget).expect_err("must fail");
        assert_eq!(err.code, "invalid_budget");
    }
}
