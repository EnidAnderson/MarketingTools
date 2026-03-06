use super::contracts::{
    ExperimentReadinessCardV1, InsightPermissionCardV1, InsightPermissionStateV1,
    InsightSampleContextV1,
};

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Claim class controlling how evidence may be used in decision workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentClaimKindV1 {
    ControlCandidateSelection,
    ChallengerLiftClaim,
    ObservationalPrioritization,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Typed input used to resolve experiment-readiness and insight-permission cards.
/// invariants:
///   - `control_landing_family` is required for all landing experiment claims.
///   - `required_sample_size`, when present, must be positive.
///   - `challenger_landing_families` is required for challenger-lift claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandingExperimentAssessmentInputV1 {
    pub insight_id: String,
    pub experiment_id: String,
    pub decision_target: String,
    pub statement: String,
    pub claim_kind: ExperimentClaimKindV1,
    pub control_landing_family: String,
    pub challenger_landing_families: Vec<String>,
    pub primary_metric: String,
    pub analysis_window: String,
    pub taxonomy_version: Option<String>,
    pub units_observed: u64,
    pub outcome_events: Option<u64>,
    pub baseline_value: Option<String>,
    pub minimum_detectable_effect: Option<String>,
    pub required_sample_size: Option<u64>,
    pub observed_sample_size: Option<u64>,
    pub instrumentation_ready: bool,
    pub taxonomy_coverage_ready: bool,
    pub causal_design_approved: bool,
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Resolve whether a landing-page insight may be used operationally.
/// invariants:
///   - causal lift claims fail closed until both design approval and sample sufficiency are present.
///   - observational evidence never upgrades to `allowed_operational_claim` for challenger lift.
pub fn resolve_landing_experiment_permission_v1(
    input: &LandingExperimentAssessmentInputV1,
) -> InsightPermissionCardV1 {
    let coverage_notes = build_coverage_notes(input);
    let permission_state = resolve_permission_state(input);
    let action_state = permission_action_state(&permission_state).to_string();

    InsightPermissionCardV1 {
        insight_id: input.insight_id.clone(),
        decision_target: input.decision_target.clone(),
        statement: input.statement.clone(),
        permission_state: permission_state.clone(),
        confidence_tier: confidence_tier(input, &permission_state).to_string(),
        action_state,
        sample_context: InsightSampleContextV1 {
            analysis_window: input.analysis_window.clone(),
            units_observed: input.units_observed,
            outcome_events: input.outcome_events,
            coverage_notes,
        },
        allowed_uses: allowed_uses(input, &permission_state),
        blocked_uses: blocked_uses(input, &permission_state),
        next_data_actions: next_data_actions(input, &permission_state),
        taxonomy_version: input.taxonomy_version.clone(),
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Resolve whether a landing experiment is ready for decision-grade use.
pub fn resolve_landing_experiment_readiness_v1(
    input: &LandingExperimentAssessmentInputV1,
) -> ExperimentReadinessCardV1 {
    let readiness_state = resolve_permission_state(input);
    ExperimentReadinessCardV1 {
        experiment_id: input.experiment_id.clone(),
        objective: input.statement.clone(),
        control_landing_family: input.control_landing_family.clone(),
        challenger_landing_families: input.challenger_landing_families.clone(),
        primary_metric: input.primary_metric.clone(),
        baseline_value: input.baseline_value.clone(),
        minimum_detectable_effect: input.minimum_detectable_effect.clone(),
        required_sample_size: input.required_sample_size,
        observed_sample_size: input.observed_sample_size,
        readiness_state: readiness_state.clone(),
        blocking_reasons: blocking_reasons(input, &readiness_state),
        next_actions: next_data_actions(input, &readiness_state),
    }
}

fn resolve_permission_state(
    input: &LandingExperimentAssessmentInputV1,
) -> InsightPermissionStateV1 {
    if input.control_landing_family.trim().is_empty()
        || input.insight_id.trim().is_empty()
        || input.experiment_id.trim().is_empty()
        || input.statement.trim().is_empty()
        || input.decision_target.trim().is_empty()
        || input.primary_metric.trim().is_empty()
        || matches!(input.required_sample_size, Some(0))
        || matches!(input.observed_sample_size, Some(0))
    {
        return InsightPermissionStateV1::Blocked;
    }

    if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
        && input.challenger_landing_families.is_empty()
    {
        return InsightPermissionStateV1::Blocked;
    }

    if !input.instrumentation_ready || !input.taxonomy_coverage_ready {
        return InsightPermissionStateV1::InstrumentFirst;
    }

    match input.claim_kind {
        ExperimentClaimKindV1::ControlCandidateSelection => {
            if input.units_observed == 0 {
                InsightPermissionStateV1::InsufficientEvidence
            } else {
                InsightPermissionStateV1::AllowedOperationalClaim
            }
        }
        ExperimentClaimKindV1::ObservationalPrioritization => {
            if input.units_observed == 0 {
                InsightPermissionStateV1::InsufficientEvidence
            } else {
                InsightPermissionStateV1::DirectionalOnly
            }
        }
        ExperimentClaimKindV1::ChallengerLiftClaim => {
            let Some(required_sample_size) = input.required_sample_size else {
                return InsightPermissionStateV1::InsufficientEvidence;
            };
            let Some(observed_sample_size) = input.observed_sample_size else {
                return InsightPermissionStateV1::InsufficientEvidence;
            };

            if !input.causal_design_approved {
                return if observed_sample_size > 0 {
                    InsightPermissionStateV1::DirectionalOnly
                } else {
                    InsightPermissionStateV1::InsufficientEvidence
                };
            }

            if observed_sample_size < required_sample_size {
                InsightPermissionStateV1::InsufficientEvidence
            } else {
                InsightPermissionStateV1::AllowedOperationalClaim
            }
        }
    }
}

fn build_coverage_notes(input: &LandingExperimentAssessmentInputV1) -> Vec<String> {
    let mut notes = Vec::new();
    if !input.instrumentation_ready {
        notes.push("instrumentation_not_ready".to_string());
    }
    if !input.taxonomy_coverage_ready {
        notes.push("landing_taxonomy_coverage_not_ready".to_string());
    }
    if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
        && !input.causal_design_approved
    {
        notes.push("causal_design_not_approved".to_string());
    }
    if let (Some(observed), Some(required)) =
        (input.observed_sample_size, input.required_sample_size)
    {
        if observed < required {
            notes.push(format!(
                "sample_gap={} below_required={}",
                observed, required
            ));
        }
    }
    notes
}

fn permission_action_state(permission_state: &InsightPermissionStateV1) -> &'static str {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => "operational_use_permitted",
        InsightPermissionStateV1::DirectionalOnly => "directional_review_only",
        InsightPermissionStateV1::InsufficientEvidence => "collect_more_data",
        InsightPermissionStateV1::InstrumentFirst => "instrumentation_required",
        InsightPermissionStateV1::Blocked => "blocked",
    }
}

fn confidence_tier(
    input: &LandingExperimentAssessmentInputV1,
    permission_state: &InsightPermissionStateV1,
) -> &'static str {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => {
            if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim) {
                "decision_grade"
            } else {
                "operational"
            }
        }
        InsightPermissionStateV1::DirectionalOnly => "directional",
        InsightPermissionStateV1::InsufficientEvidence => "insufficient",
        InsightPermissionStateV1::InstrumentFirst => "not_measured",
        InsightPermissionStateV1::Blocked => "blocked",
    }
}

fn allowed_uses(
    input: &LandingExperimentAssessmentInputV1,
    permission_state: &InsightPermissionStateV1,
) -> Vec<String> {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => match input.claim_kind {
            ExperimentClaimKindV1::ControlCandidateSelection => vec![
                "use_as_control_group_for_future_landing_experiments".to_string(),
                "reference_as_current_paid_traffic_baseline".to_string(),
            ],
            ExperimentClaimKindV1::ChallengerLiftClaim => vec![
                "use_for_routing_or_budget_reallocation_decisions".to_string(),
                "promote_to_content_pipeline_as_approved_fact".to_string(),
            ],
            ExperimentClaimKindV1::ObservationalPrioritization => {
                vec!["prioritize_for_next_experiment_design".to_string()]
            }
        },
        InsightPermissionStateV1::DirectionalOnly => vec![
            "use_to_prioritize_follow_up_experiments".to_string(),
            "use_in_scientist_notes_with_directional_label".to_string(),
        ],
        InsightPermissionStateV1::InsufficientEvidence
        | InsightPermissionStateV1::InstrumentFirst
        | InsightPermissionStateV1::Blocked => Vec::new(),
    }
}

fn blocked_uses(
    input: &LandingExperimentAssessmentInputV1,
    permission_state: &InsightPermissionStateV1,
) -> Vec<String> {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => match input.claim_kind {
            ExperimentClaimKindV1::ControlCandidateSelection => {
                vec!["claim_control_is_best_performing_variant".to_string()]
            }
            ExperimentClaimKindV1::ChallengerLiftClaim => Vec::new(),
            ExperimentClaimKindV1::ObservationalPrioritization => {
                vec!["treat_observational_signal_as_causal_lift".to_string()]
            }
        },
        InsightPermissionStateV1::DirectionalOnly => vec![
            "auto_reroute_google_ads_traffic_based_on_this_claim".to_string(),
            "treat_observational_delta_as_causal_lift".to_string(),
        ],
        InsightPermissionStateV1::InsufficientEvidence => vec![
            "present_as_marketing_fact".to_string(),
            "use_for_budget_or_routing_decisions".to_string(),
        ],
        InsightPermissionStateV1::InstrumentFirst => vec![
            "treat_unmeasured_landing_delta_as_valid".to_string(),
            "use_for_content_prioritization_without_instrumentation_fix".to_string(),
        ],
        InsightPermissionStateV1::Blocked => vec![
            "use_in_any_decision_workflow".to_string(),
            "render_as_actionable_experiment_status".to_string(),
        ],
    }
}

fn next_data_actions(
    input: &LandingExperimentAssessmentInputV1,
    permission_state: &InsightPermissionStateV1,
) -> Vec<String> {
    let mut actions = Vec::new();
    if !input.instrumentation_ready {
        actions.push("add_experiment_id_variant_id_and_ad_metadata_to_event_stream".to_string());
    }
    if !input.taxonomy_coverage_ready {
        actions.push("ship_landing_taxonomy_v2_to_runtime_classifier".to_string());
    }
    if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
        && !input.causal_design_approved
    {
        actions.push("approve_randomized_or_quasi_experiment_design".to_string());
    }
    if matches!(
        permission_state,
        InsightPermissionStateV1::InsufficientEvidence | InsightPermissionStateV1::DirectionalOnly
    ) {
        actions.push("collect_additional_observations_until_sample_gate_is_green".to_string());
    }
    if matches!(
        permission_state,
        InsightPermissionStateV1::AllowedOperationalClaim
    ) && matches!(
        input.claim_kind,
        ExperimentClaimKindV1::ControlCandidateSelection
    ) {
        actions.push("keep_current_control_route_fixed_during_challenger_test".to_string());
    }
    actions
}

fn blocking_reasons(
    input: &LandingExperimentAssessmentInputV1,
    permission_state: &InsightPermissionStateV1,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if input.control_landing_family.trim().is_empty() {
        reasons.push("missing_control_landing_family".to_string());
    }
    if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
        && input.challenger_landing_families.is_empty()
    {
        reasons.push("missing_challenger_landing_family".to_string());
    }
    if !input.instrumentation_ready {
        reasons.push("instrumentation_not_ready".to_string());
    }
    if !input.taxonomy_coverage_ready {
        reasons.push("landing_taxonomy_coverage_not_ready".to_string());
    }
    if matches!(permission_state, InsightPermissionStateV1::DirectionalOnly)
        && matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
    {
        reasons.push("causal_design_not_approved".to_string());
    }
    if matches!(
        permission_state,
        InsightPermissionStateV1::InsufficientEvidence
    ) && matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim)
    {
        reasons.push("sample_threshold_not_met".to_string());
    }
    if matches!(permission_state, InsightPermissionStateV1::Blocked) && reasons.is_empty() {
        reasons.push("invalid_experiment_governance_input".to_string());
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> LandingExperimentAssessmentInputV1 {
        LandingExperimentAssessmentInputV1 {
            insight_id: "INS-TEST-0001".to_string(),
            experiment_id: "EXP-TEST-0001".to_string(),
            decision_target: "paid_landing_strategy".to_string(),
            statement: "Use Simply Raw as the control landing family.".to_string(),
            claim_kind: ExperimentClaimKindV1::ControlCandidateSelection,
            control_landing_family: "simply_raw_offer_lp".to_string(),
            challenger_landing_families: vec!["bundle_offer_lp".to_string()],
            primary_metric: "purchase_session_rate".to_string(),
            analysis_window: "2026-02-04/2026-03-05".to_string(),
            taxonomy_version: Some("nd_landing_taxonomy.v2".to_string()),
            units_observed: 1_200,
            outcome_events: Some(27),
            baseline_value: Some("0.0185".to_string()),
            minimum_detectable_effect: Some("0.15".to_string()),
            required_sample_size: Some(1_500),
            observed_sample_size: Some(1_200),
            instrumentation_ready: true,
            taxonomy_coverage_ready: true,
            causal_design_approved: false,
        }
    }

    #[test]
    fn control_candidate_selection_is_allowed_when_measurement_is_ready() {
        let input = base_input();
        let card = resolve_landing_experiment_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::AllowedOperationalClaim
        );
        assert!(card
            .allowed_uses
            .contains(&"use_as_control_group_for_future_landing_experiments".to_string()));
    }

    #[test]
    fn challenger_lift_without_design_is_directional_only() {
        let mut input = base_input();
        input.claim_kind = ExperimentClaimKindV1::ChallengerLiftClaim;
        input.statement =
            "Bundle landing pages outperform Simply Raw for the same paid traffic.".to_string();
        let card = resolve_landing_experiment_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::DirectionalOnly
        );
        assert!(card
            .blocked_uses
            .contains(&"auto_reroute_google_ads_traffic_based_on_this_claim".to_string()));
    }

    #[test]
    fn challenger_lift_with_design_but_insufficient_sample_stays_insufficient() {
        let mut input = base_input();
        input.claim_kind = ExperimentClaimKindV1::ChallengerLiftClaim;
        input.causal_design_approved = true;
        let readiness = resolve_landing_experiment_readiness_v1(&input);
        assert_eq!(
            readiness.readiness_state,
            InsightPermissionStateV1::InsufficientEvidence
        );
        assert!(readiness
            .blocking_reasons
            .contains(&"sample_threshold_not_met".to_string()));
    }

    #[test]
    fn challenger_lift_with_design_and_sample_becomes_operational() {
        let mut input = base_input();
        input.claim_kind = ExperimentClaimKindV1::ChallengerLiftClaim;
        input.causal_design_approved = true;
        input.observed_sample_size = Some(1_700);
        let card = resolve_landing_experiment_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::AllowedOperationalClaim
        );
        assert!(card
            .allowed_uses
            .contains(&"use_for_routing_or_budget_reallocation_decisions".to_string()));
    }

    #[test]
    fn taxonomy_gap_forces_instrument_first() {
        let mut input = base_input();
        input.taxonomy_coverage_ready = false;
        let card = resolve_landing_experiment_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::InstrumentFirst
        );
        assert!(card
            .sample_context
            .coverage_notes
            .contains(&"landing_taxonomy_coverage_not_ready".to_string()));
    }

    #[test]
    fn missing_control_family_blocks_resolution() {
        let mut input = base_input();
        input.control_landing_family.clear();
        let readiness = resolve_landing_experiment_readiness_v1(&input);
        assert_eq!(readiness.readiness_state, InsightPermissionStateV1::Blocked);
        assert!(readiness
            .blocking_reasons
            .contains(&"missing_control_landing_family".to_string()));
    }
}
