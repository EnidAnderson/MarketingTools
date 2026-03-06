use super::contracts::{
    ExperimentAnalyticsSummaryV1, ExperimentFunnelRowV1, ExperimentReadinessCardV1,
    InsightPermissionCardV1, InsightPermissionStateV1, InsightSampleContextV1,
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
/// purpose: Observed-data input for one control/challenger experiment comparison.
/// invariants:
///   - variant ids are canonical join keys and must not be inferred from labels.
///   - rate thresholds are expressed in basis points.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedExperimentPairAssessmentInputV1 {
    pub insight_id: String,
    pub experiment_id: String,
    pub decision_target: String,
    pub statement: String,
    pub control_landing_family: String,
    pub challenger_landing_family: String,
    pub control_variant_id: String,
    pub challenger_variant_id: String,
    pub primary_metric: String,
    pub analysis_window: String,
    pub taxonomy_version: Option<String>,
    pub minimum_assigned_sessions_per_arm: u64,
    pub minimum_outcome_events_per_arm: u64,
    pub minimum_assignment_rate_bps: u32,
    pub maximum_ambiguity_rate_bps: u32,
    pub maximum_partial_or_unassigned_rate_bps: u32,
    pub minimum_guardrail_coverage_bps: u32,
    pub required_guardrail_dimensions: Vec<String>,
    pub instrumentation_ready: bool,
    pub taxonomy_coverage_ready: bool,
    pub causal_design_approved: bool,
    pub observed: ExperimentAnalyticsSummaryV1,
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
        permission_level: permission_level(&readiness_state).to_string(),
        supporting_reasons: supporting_reasons_for_planning_input(input, &readiness_state),
        blocking_reasons: blocking_reasons(input, &readiness_state),
        next_actions: next_data_actions(input, &readiness_state),
        ..Default::default()
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Resolve decision permission for one observed control/challenger experiment pair.
pub fn resolve_observed_experiment_pair_permission_v1(
    input: &ObservedExperimentPairAssessmentInputV1,
) -> InsightPermissionCardV1 {
    let evaluation = evaluate_observed_experiment_pair(input);
    InsightPermissionCardV1 {
        insight_id: input.insight_id.clone(),
        decision_target: input.decision_target.clone(),
        statement: input.statement.clone(),
        permission_state: evaluation.permission_state.clone(),
        confidence_tier: confidence_tier_for_observed_pair(&evaluation.permission_state)
            .to_string(),
        action_state: permission_action_state(&evaluation.permission_state).to_string(),
        sample_context: InsightSampleContextV1 {
            analysis_window: input.analysis_window.clone(),
            units_observed: evaluation.total_observed_sessions,
            outcome_events: Some(
                evaluation
                    .control_outcome_events
                    .saturating_add(evaluation.challenger_outcome_events),
            ),
            coverage_notes: evaluation.coverage_notes.clone(),
        },
        allowed_uses: allowed_uses_for_observed_pair(&evaluation.permission_state),
        blocked_uses: blocked_uses_for_observed_pair(&evaluation.permission_state),
        next_data_actions: evaluation.next_actions.clone(),
        taxonomy_version: input.taxonomy_version.clone(),
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::experiment_governance`
/// purpose: Resolve readiness for one observed control/challenger experiment pair.
pub fn resolve_observed_experiment_pair_readiness_v1(
    input: &ObservedExperimentPairAssessmentInputV1,
) -> ExperimentReadinessCardV1 {
    let evaluation = evaluate_observed_experiment_pair(input);
    ExperimentReadinessCardV1 {
        experiment_id: input.experiment_id.clone(),
        objective: input.statement.clone(),
        control_landing_family: input.control_landing_family.clone(),
        challenger_landing_families: vec![input.challenger_landing_family.clone()],
        primary_metric: input.primary_metric.clone(),
        baseline_value: Some(format!(
            "control_variant={} outcome_events={}",
            input.control_variant_id, evaluation.control_outcome_events
        )),
        minimum_detectable_effect: None,
        required_sample_size: Some(input.minimum_assigned_sessions_per_arm),
        observed_sample_size: Some(
            evaluation
                .control_assigned_sessions
                .min(evaluation.challenger_assigned_sessions),
        ),
        readiness_state: evaluation.permission_state.clone(),
        control_variant_id: Some(input.control_variant_id.clone()),
        challenger_variant_id: Some(input.challenger_variant_id.clone()),
        permission_level: permission_level(&evaluation.permission_state).to_string(),
        supporting_reasons: evaluation.supporting_reasons.clone(),
        blocking_reasons: evaluation.blocking_reasons.clone(),
        next_actions: evaluation.next_actions.clone(),
        assigned_sessions_control: Some(evaluation.control_assigned_sessions),
        assigned_sessions_challenger: Some(evaluation.challenger_assigned_sessions),
        control_outcome_events: Some(evaluation.control_outcome_events),
        challenger_outcome_events: Some(evaluation.challenger_outcome_events),
        assignment_rate_bps: Some(evaluation.assignment_rate_bps),
        ambiguity_rate_bps: Some(evaluation.ambiguity_rate_bps),
        partial_or_unassigned_rate_bps: Some(evaluation.partial_or_unassigned_rate_bps),
        denominator_scope: Some("assigned_sessions_only".to_string()),
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

fn confidence_tier_for_observed_pair(permission_state: &InsightPermissionStateV1) -> &'static str {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => "decision_grade",
        InsightPermissionStateV1::DirectionalOnly => "directional",
        InsightPermissionStateV1::InsufficientEvidence => "insufficient",
        InsightPermissionStateV1::InstrumentFirst => "not_measured",
        InsightPermissionStateV1::Blocked => "blocked",
    }
}

fn permission_level(permission_state: &InsightPermissionStateV1) -> &'static str {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => "decision_ready",
        InsightPermissionStateV1::DirectionalOnly => "directional_only",
        InsightPermissionStateV1::InsufficientEvidence => "descriptive_only",
        InsightPermissionStateV1::InstrumentFirst => "instrument_first",
        InsightPermissionStateV1::Blocked => "blocked",
    }
}

fn supporting_reasons_for_planning_input(
    input: &LandingExperimentAssessmentInputV1,
    readiness_state: &InsightPermissionStateV1,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if input.instrumentation_ready {
        reasons.push("instrumentation_ready".to_string());
    }
    if input.taxonomy_coverage_ready {
        reasons.push("landing_taxonomy_coverage_ready".to_string());
    }
    if input.units_observed > 0 {
        reasons.push(format!("units_observed={}", input.units_observed));
    }
    if let Some(outcome_events) = input.outcome_events {
        reasons.push(format!("outcome_events={}", outcome_events));
    }
    if matches!(input.claim_kind, ExperimentClaimKindV1::ChallengerLiftClaim) {
        reasons.push("control_and_challenger_defined".to_string());
    }
    if input.causal_design_approved {
        reasons.push("causal_design_approved".to_string());
    }
    if matches!(
        readiness_state,
        InsightPermissionStateV1::AllowedOperationalClaim
    ) {
        reasons.push("all_active_readiness_gates_passed".to_string());
    }
    dedupe_strings(reasons)
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

fn allowed_uses_for_observed_pair(permission_state: &InsightPermissionStateV1) -> Vec<String> {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => vec![
            "use_for_routing_or_budget_reallocation_decisions".to_string(),
            "promote_to_content_pipeline_as_approved_fact".to_string(),
        ],
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

fn blocked_uses_for_observed_pair(permission_state: &InsightPermissionStateV1) -> Vec<String> {
    match permission_state {
        InsightPermissionStateV1::AllowedOperationalClaim => Vec::new(),
        InsightPermissionStateV1::DirectionalOnly => vec![
            "auto_reroute_google_ads_traffic_based_on_this_claim".to_string(),
            "treat_observed_variant_delta_as_causal_lift".to_string(),
        ],
        InsightPermissionStateV1::InsufficientEvidence => vec![
            "present_variant_winner_as_established_fact".to_string(),
            "use_for_budget_or_routing_decisions".to_string(),
        ],
        InsightPermissionStateV1::InstrumentFirst => vec![
            "treat_unassigned_or_ambiguous_sessions_as_decision_grade_evidence".to_string(),
            "render_variant_lift_without_assignment_coverage".to_string(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedExperimentPairEvaluation {
    permission_state: InsightPermissionStateV1,
    total_observed_sessions: u64,
    control_assigned_sessions: u64,
    challenger_assigned_sessions: u64,
    control_outcome_events: u64,
    challenger_outcome_events: u64,
    assignment_rate_bps: u32,
    ambiguity_rate_bps: u32,
    partial_or_unassigned_rate_bps: u32,
    coverage_notes: Vec<String>,
    supporting_reasons: Vec<String>,
    blocking_reasons: Vec<String>,
    next_actions: Vec<String>,
}

fn evaluate_observed_experiment_pair(
    input: &ObservedExperimentPairAssessmentInputV1,
) -> ObservedExperimentPairEvaluation {
    let coverage = &input.observed.assignment_coverage;
    let total_observed_sessions = coverage.total_observed_sessions;
    let assignment_rate_bps =
        ratio_bps(coverage.assigned_sessions, coverage.total_observed_sessions);
    let ambiguity_rate_bps = ratio_bps(
        coverage.ambiguous_sessions,
        coverage.total_observed_sessions,
    );
    let partial_or_unassigned_sessions = coverage
        .partial_sessions
        .saturating_add(coverage.unassigned_sessions);
    let partial_or_unassigned_rate_bps = ratio_bps(
        partial_or_unassigned_sessions,
        coverage.total_observed_sessions,
    );

    let control_row = find_variant_row(
        &input.observed,
        input.experiment_id.as_str(),
        input.control_variant_id.as_str(),
    );
    let challenger_row = find_variant_row(
        &input.observed,
        input.experiment_id.as_str(),
        input.challenger_variant_id.as_str(),
    );
    let control_assigned_sessions = control_row.map_or(0, |row| row.sessions);
    let challenger_assigned_sessions = challenger_row.map_or(0, |row| row.sessions);
    let control_outcome_events =
        control_row.map_or(0, |row| primary_metric_events(row, &input.primary_metric));
    let challenger_outcome_events =
        challenger_row.map_or(0, |row| primary_metric_events(row, &input.primary_metric));
    let missing_guardrails = missing_guardrail_dimensions(
        &input.required_guardrail_dimensions,
        &input.observed.guardrail_slices,
    );
    let low_guardrails = low_guardrail_dimensions(
        &input.required_guardrail_dimensions,
        &input.observed.guardrail_slices,
        total_observed_sessions,
        input.minimum_guardrail_coverage_bps,
    );

    let mut hard_blocking_reasons = Vec::new();
    if input.insight_id.trim().is_empty() {
        hard_blocking_reasons.push("missing_insight_id".to_string());
    }
    if input.experiment_id.trim().is_empty() {
        hard_blocking_reasons.push("missing_experiment_id".to_string());
    }
    if input.decision_target.trim().is_empty() {
        hard_blocking_reasons.push("missing_decision_target".to_string());
    }
    if input.statement.trim().is_empty() {
        hard_blocking_reasons.push("missing_statement".to_string());
    }
    if input.control_landing_family.trim().is_empty() {
        hard_blocking_reasons.push("missing_control_landing_family".to_string());
    }
    if input.challenger_landing_family.trim().is_empty() {
        hard_blocking_reasons.push("missing_challenger_landing_family".to_string());
    }
    if input.control_variant_id.trim().is_empty() {
        hard_blocking_reasons.push("missing_control_variant_id".to_string());
    }
    if input.challenger_variant_id.trim().is_empty() {
        hard_blocking_reasons.push("missing_challenger_variant_id".to_string());
    }
    if input.primary_metric.trim().is_empty() {
        hard_blocking_reasons.push("missing_primary_metric".to_string());
    }
    if input.control_variant_id == input.challenger_variant_id
        && !input.control_variant_id.trim().is_empty()
    {
        hard_blocking_reasons.push("control_and_challenger_variant_ids_must_differ".to_string());
    }
    if input.minimum_assignment_rate_bps > 10_000
        || input.maximum_ambiguity_rate_bps > 10_000
        || input.maximum_partial_or_unassigned_rate_bps > 10_000
        || input.minimum_guardrail_coverage_bps > 10_000
    {
        hard_blocking_reasons.push("invalid_rate_threshold_bps".to_string());
    }

    let mut blocking_reasons = hard_blocking_reasons.clone();
    if !input.instrumentation_ready {
        blocking_reasons.push("instrumentation_not_ready".to_string());
    }
    if !input.taxonomy_coverage_ready {
        blocking_reasons.push("landing_taxonomy_coverage_not_ready".to_string());
    }
    if total_observed_sessions == 0 {
        blocking_reasons.push("no_observed_sessions".to_string());
    }
    if assignment_rate_bps < input.minimum_assignment_rate_bps {
        blocking_reasons.push(format!(
            "assignment_rate_below_threshold={}bps<{}bps",
            assignment_rate_bps, input.minimum_assignment_rate_bps
        ));
    }
    if ambiguity_rate_bps > input.maximum_ambiguity_rate_bps {
        blocking_reasons.push(format!(
            "ambiguity_rate_above_threshold={}bps>{}bps",
            ambiguity_rate_bps, input.maximum_ambiguity_rate_bps
        ));
    }
    if partial_or_unassigned_rate_bps > input.maximum_partial_or_unassigned_rate_bps {
        blocking_reasons.push(format!(
            "partial_or_unassigned_rate_above_threshold={}bps>{}bps",
            partial_or_unassigned_rate_bps, input.maximum_partial_or_unassigned_rate_bps
        ));
    }
    for dimension in &missing_guardrails {
        blocking_reasons.push(format!("missing_guardrail_dimension={dimension}"));
    }
    if control_row.is_none() {
        blocking_reasons.push(format!(
            "control_variant_missing_from_assigned_funnels={}",
            input.control_variant_id
        ));
    }
    if challenger_row.is_none() {
        blocking_reasons.push(format!(
            "challenger_variant_missing_from_assigned_funnels={}",
            input.challenger_variant_id
        ));
    }
    if control_assigned_sessions < input.minimum_assigned_sessions_per_arm {
        blocking_reasons.push(format!(
            "control_assigned_sessions_below_threshold={}<{}",
            control_assigned_sessions, input.minimum_assigned_sessions_per_arm
        ));
    }
    if challenger_assigned_sessions < input.minimum_assigned_sessions_per_arm {
        blocking_reasons.push(format!(
            "challenger_assigned_sessions_below_threshold={}<{}",
            challenger_assigned_sessions, input.minimum_assigned_sessions_per_arm
        ));
    }
    if control_outcome_events < input.minimum_outcome_events_per_arm {
        blocking_reasons.push(format!(
            "control_outcome_events_below_threshold={}<{}",
            control_outcome_events, input.minimum_outcome_events_per_arm
        ));
    }
    if challenger_outcome_events < input.minimum_outcome_events_per_arm {
        blocking_reasons.push(format!(
            "challenger_outcome_events_below_threshold={}<{}",
            challenger_outcome_events, input.minimum_outcome_events_per_arm
        ));
    }
    for dimension in &low_guardrails {
        blocking_reasons.push(format!(
            "guardrail_coverage_below_threshold_for_dimension={dimension}"
        ));
    }
    if !input.causal_design_approved {
        blocking_reasons.push("causal_design_not_approved".to_string());
    }

    let permission_state = if !hard_blocking_reasons.is_empty() {
        InsightPermissionStateV1::Blocked
    } else if !input.instrumentation_ready
        || !input.taxonomy_coverage_ready
        || assignment_rate_bps < input.minimum_assignment_rate_bps
        || ambiguity_rate_bps > input.maximum_ambiguity_rate_bps
        || partial_or_unassigned_rate_bps > input.maximum_partial_or_unassigned_rate_bps
        || !missing_guardrails.is_empty()
    {
        InsightPermissionStateV1::InstrumentFirst
    } else if total_observed_sessions == 0
        || control_row.is_none()
        || challenger_row.is_none()
        || control_assigned_sessions < input.minimum_assigned_sessions_per_arm
        || challenger_assigned_sessions < input.minimum_assigned_sessions_per_arm
        || control_outcome_events < input.minimum_outcome_events_per_arm
        || challenger_outcome_events < input.minimum_outcome_events_per_arm
        || !low_guardrails.is_empty()
    {
        InsightPermissionStateV1::InsufficientEvidence
    } else if !input.causal_design_approved {
        InsightPermissionStateV1::DirectionalOnly
    } else {
        InsightPermissionStateV1::AllowedOperationalClaim
    };

    let mut coverage_notes = coverage.notes.clone();
    coverage_notes.push(format!("assignment_rate_bps={assignment_rate_bps}"));
    coverage_notes.push(format!("ambiguity_rate_bps={ambiguity_rate_bps}"));
    coverage_notes.push(format!(
        "partial_or_unassigned_rate_bps={partial_or_unassigned_rate_bps}"
    ));
    if !missing_guardrails.is_empty() {
        coverage_notes.push(format!(
            "missing_guardrail_dimensions={}",
            missing_guardrails.join(",")
        ));
    }
    if !low_guardrails.is_empty() {
        coverage_notes.push(format!(
            "guardrail_dimensions_below_threshold={}",
            low_guardrails.join(",")
        ));
    }
    if control_row.is_none() || challenger_row.is_none() {
        coverage_notes.push("variant_assignment_rows_incomplete".to_string());
    }

    let mut supporting_reasons = Vec::new();
    if input.instrumentation_ready {
        supporting_reasons.push("instrumentation_ready".to_string());
    }
    if input.taxonomy_coverage_ready {
        supporting_reasons.push("landing_taxonomy_coverage_ready".to_string());
    }
    if assignment_rate_bps >= input.minimum_assignment_rate_bps {
        supporting_reasons.push(format!(
            "assignment_rate_meets_threshold={}bps",
            assignment_rate_bps
        ));
    }
    if ambiguity_rate_bps <= input.maximum_ambiguity_rate_bps {
        supporting_reasons.push(format!(
            "ambiguity_rate_within_threshold={}bps",
            ambiguity_rate_bps
        ));
    }
    if partial_or_unassigned_rate_bps <= input.maximum_partial_or_unassigned_rate_bps {
        supporting_reasons.push(format!(
            "partial_or_unassigned_rate_within_threshold={}bps",
            partial_or_unassigned_rate_bps
        ));
    }
    if control_row.is_some() && challenger_row.is_some() {
        supporting_reasons.push("control_and_challenger_present_in_assigned_funnels".to_string());
    }
    if control_assigned_sessions >= input.minimum_assigned_sessions_per_arm {
        supporting_reasons.push(format!(
            "control_assigned_sessions_meet_threshold={control_assigned_sessions}"
        ));
    }
    if challenger_assigned_sessions >= input.minimum_assigned_sessions_per_arm {
        supporting_reasons.push(format!(
            "challenger_assigned_sessions_meet_threshold={challenger_assigned_sessions}"
        ));
    }
    if control_outcome_events >= input.minimum_outcome_events_per_arm {
        supporting_reasons.push(format!(
            "control_outcome_events_meet_threshold={control_outcome_events}"
        ));
    }
    if challenger_outcome_events >= input.minimum_outcome_events_per_arm {
        supporting_reasons.push(format!(
            "challenger_outcome_events_meet_threshold={challenger_outcome_events}"
        ));
    }
    if missing_guardrails.is_empty() {
        supporting_reasons.push("required_guardrail_dimensions_present".to_string());
    }
    if low_guardrails.is_empty() {
        supporting_reasons.push("guardrail_coverage_within_threshold".to_string());
    }
    if input.causal_design_approved {
        supporting_reasons.push("causal_design_approved".to_string());
    }
    if matches!(
        permission_state,
        InsightPermissionStateV1::AllowedOperationalClaim
    ) {
        supporting_reasons.push("all_observed_experiment_readiness_gates_passed".to_string());
    }

    let mut next_actions = Vec::new();
    if !input.instrumentation_ready {
        next_actions
            .push("emit_experiment_id_and_variant_id_on_landing_and_funnel_events".to_string());
    }
    if !input.taxonomy_coverage_ready {
        next_actions.push("ship_landing_taxonomy_v2_to_runtime_classifier".to_string());
    }
    if assignment_rate_bps < input.minimum_assignment_rate_bps
        || partial_or_unassigned_rate_bps > input.maximum_partial_or_unassigned_rate_bps
    {
        next_actions.push(
            "persist_experiment_assignment_across_all_session_events_before_readout".to_string(),
        );
    }
    if ambiguity_rate_bps > input.maximum_ambiguity_rate_bps {
        next_actions.push("reduce_conflicting_experiment_assignments_before_readout".to_string());
    }
    if !missing_guardrails.is_empty() {
        next_actions.push("add_guardrail_slice_emission_for_missing_dimensions".to_string());
    }
    if control_row.is_none() || challenger_row.is_none() {
        next_actions.push(
            "ensure_both_control_and_challenger_variants_receive_assigned_sessions".to_string(),
        );
    }
    if control_assigned_sessions < input.minimum_assigned_sessions_per_arm
        || challenger_assigned_sessions < input.minimum_assigned_sessions_per_arm
        || control_outcome_events < input.minimum_outcome_events_per_arm
        || challenger_outcome_events < input.minimum_outcome_events_per_arm
    {
        next_actions.push(
            "collect_additional_assigned_sessions_until_per_arm_thresholds_are_green".to_string(),
        );
    }
    if !low_guardrails.is_empty() {
        next_actions
            .push("improve_guardrail_assignment_coverage_before_variant_comparison".to_string());
    }
    if !input.causal_design_approved {
        next_actions.push("approve_randomized_or_quasi_experiment_design".to_string());
    }

    ObservedExperimentPairEvaluation {
        permission_state,
        total_observed_sessions,
        control_assigned_sessions,
        challenger_assigned_sessions,
        control_outcome_events,
        challenger_outcome_events,
        assignment_rate_bps,
        ambiguity_rate_bps,
        partial_or_unassigned_rate_bps,
        coverage_notes: dedupe_strings(coverage_notes),
        supporting_reasons: dedupe_strings(supporting_reasons),
        blocking_reasons: dedupe_strings(blocking_reasons),
        next_actions: dedupe_strings(next_actions),
    }
}

fn find_variant_row<'a>(
    observed: &'a ExperimentAnalyticsSummaryV1,
    experiment_id: &str,
    variant_id: &str,
) -> Option<&'a ExperimentFunnelRowV1> {
    observed.funnel_rows.iter().find(|row| {
        row.experiment_id == experiment_id
            && row.variant_id == variant_id
            && row.denominator_scope == "assigned_sessions_only"
    })
}

fn primary_metric_events(row: &ExperimentFunnelRowV1, primary_metric: &str) -> u64 {
    match primary_metric {
        "engaged_session_rate" => row.engaged_sessions,
        "product_view_session_rate" => row.product_view_sessions,
        "add_to_cart_session_rate" => row.add_to_cart_sessions,
        "checkout_session_rate" => row.checkout_sessions,
        "purchase_session_rate" | "revenue_per_session" => row.purchase_sessions,
        _ => row.purchase_sessions,
    }
}

fn ratio_bps(numerator: u64, denominator: u64) -> u32 {
    if denominator == 0 {
        return 0;
    }
    (((numerator as u128) * 10_000u128 + (denominator as u128 / 2)) / denominator as u128) as u32
}

fn missing_guardrail_dimensions(
    required_guardrail_dimensions: &[String],
    guardrail_slices: &[super::contracts::ExperimentGuardrailSliceV1],
) -> Vec<String> {
    let present = guardrail_slices
        .iter()
        .map(|slice| slice.dimension_key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    required_guardrail_dimensions
        .iter()
        .filter(|dimension| !present.contains(dimension.as_str()))
        .cloned()
        .collect()
}

fn low_guardrail_dimensions(
    required_guardrail_dimensions: &[String],
    guardrail_slices: &[super::contracts::ExperimentGuardrailSliceV1],
    total_observed_sessions: u64,
    minimum_guardrail_coverage_bps: u32,
) -> Vec<String> {
    let material_slice_min_sessions = total_observed_sessions.saturating_add(9) / 10;
    required_guardrail_dimensions
        .iter()
        .filter(|dimension| {
            guardrail_slices.iter().any(|slice| {
                slice.dimension_key == **dimension
                    && slice.total_sessions >= material_slice_min_sessions.max(1)
                    && ratio_bps(slice.assigned_sessions, slice.total_sessions)
                        < minimum_guardrail_coverage_bps
            })
        })
        .cloned()
        .collect()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut deduped = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }
    deduped
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

    fn observed_pair_input() -> ObservedExperimentPairAssessmentInputV1 {
        ObservedExperimentPairAssessmentInputV1 {
            insight_id: "INS-OBS-0001".to_string(),
            experiment_id: "EXP-OBS-0001".to_string(),
            decision_target: "paid_landing_routing".to_string(),
            statement: "Bundle challenger outperforms Simply Raw control for Google CPC traffic."
                .to_string(),
            control_landing_family: "simply_raw_offer_lp".to_string(),
            challenger_landing_family: "bundle_offer_lp".to_string(),
            control_variant_id: "control".to_string(),
            challenger_variant_id: "challenger".to_string(),
            primary_metric: "purchase_session_rate".to_string(),
            analysis_window: "2026-02-04/2026-03-05".to_string(),
            taxonomy_version: Some("nd_landing_taxonomy.v2".to_string()),
            minimum_assigned_sessions_per_arm: 100,
            minimum_outcome_events_per_arm: 10,
            minimum_assignment_rate_bps: 8_000,
            maximum_ambiguity_rate_bps: 500,
            maximum_partial_or_unassigned_rate_bps: 2_000,
            minimum_guardrail_coverage_bps: 7_000,
            required_guardrail_dimensions: vec![
                "device_category".to_string(),
                "platform".to_string(),
                "country".to_string(),
                "source_medium".to_string(),
            ],
            instrumentation_ready: true,
            taxonomy_coverage_ready: true,
            causal_design_approved: false,
            observed: ExperimentAnalyticsSummaryV1 {
                assignment_coverage: super::super::contracts::ExperimentAssignmentCoverageReportV1 {
                    total_observed_sessions: 400,
                    assigned_sessions: 360,
                    partial_sessions: 20,
                    ambiguous_sessions: 8,
                    unassigned_sessions: 12,
                    assignment_coverage_ratio: "0.9000".to_string(),
                    denominator_scope: "all_observed_sessions".to_string(),
                    summary: "assigned=360, partial=20, ambiguous=8, unassigned=12 across 400 observed sessions".to_string(),
                    notes: vec!["assigned_sessions_only".to_string()],
                },
                funnel_rows: vec![
                    ExperimentFunnelRowV1 {
                        experiment_id: "EXP-OBS-0001".to_string(),
                        experiment_name: Some("Landing LP Test".to_string()),
                        variant_id: "control".to_string(),
                        variant_name: Some("Simply Raw".to_string()),
                        sessions: 180,
                        engaged_sessions: 120,
                        product_view_sessions: 95,
                        add_to_cart_sessions: 42,
                        checkout_sessions: 28,
                        purchase_sessions: 16,
                        revenue_usd: 1200.0,
                        denominator_scope: "assigned_sessions_only".to_string(),
                    },
                    ExperimentFunnelRowV1 {
                        experiment_id: "EXP-OBS-0001".to_string(),
                        experiment_name: Some("Landing LP Test".to_string()),
                        variant_id: "challenger".to_string(),
                        variant_name: Some("Bundle".to_string()),
                        sessions: 180,
                        engaged_sessions: 130,
                        product_view_sessions: 110,
                        add_to_cart_sessions: 55,
                        checkout_sessions: 34,
                        purchase_sessions: 18,
                        revenue_usd: 1500.0,
                        denominator_scope: "assigned_sessions_only".to_string(),
                    },
                ],
                guardrail_slices: vec![
                    super::super::contracts::ExperimentGuardrailSliceV1 {
                        dimension_key: "device_category".to_string(),
                        dimension_value: "mobile".to_string(),
                        total_sessions: 200,
                        assigned_sessions: 170,
                        partial_sessions: 18,
                        ambiguous_sessions: 4,
                        coverage_ratio: "0.8500".to_string(),
                    },
                    super::super::contracts::ExperimentGuardrailSliceV1 {
                        dimension_key: "device_category".to_string(),
                        dimension_value: "desktop".to_string(),
                        total_sessions: 200,
                        assigned_sessions: 190,
                        partial_sessions: 2,
                        ambiguous_sessions: 4,
                        coverage_ratio: "0.9500".to_string(),
                    },
                    super::super::contracts::ExperimentGuardrailSliceV1 {
                        dimension_key: "platform".to_string(),
                        dimension_value: "web".to_string(),
                        total_sessions: 400,
                        assigned_sessions: 360,
                        partial_sessions: 20,
                        ambiguous_sessions: 8,
                        coverage_ratio: "0.9000".to_string(),
                    },
                    super::super::contracts::ExperimentGuardrailSliceV1 {
                        dimension_key: "country".to_string(),
                        dimension_value: "US".to_string(),
                        total_sessions: 400,
                        assigned_sessions: 360,
                        partial_sessions: 20,
                        ambiguous_sessions: 8,
                        coverage_ratio: "0.9000".to_string(),
                    },
                    super::super::contracts::ExperimentGuardrailSliceV1 {
                        dimension_key: "source_medium".to_string(),
                        dimension_value: "google / cpc".to_string(),
                        total_sessions: 400,
                        assigned_sessions: 360,
                        partial_sessions: 20,
                        ambiguous_sessions: 8,
                        coverage_ratio: "0.9000".to_string(),
                    },
                ],
            },
        }
    }

    #[test]
    fn observed_pair_low_assignment_coverage_requires_instrumentation() {
        let mut input = observed_pair_input();
        input.observed.assignment_coverage.assigned_sessions = 280;
        input.observed.assignment_coverage.partial_sessions = 70;
        input.observed.assignment_coverage.unassigned_sessions = 42;
        let card = resolve_observed_experiment_pair_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::InstrumentFirst
        );
        assert!(card
            .sample_context
            .coverage_notes
            .iter()
            .any(|note| note.contains("assignment_rate_bps=7000")));
    }

    #[test]
    fn observed_pair_without_causal_design_is_directional_only() {
        let input = observed_pair_input();
        let readiness = resolve_observed_experiment_pair_readiness_v1(&input);
        assert_eq!(
            readiness.readiness_state,
            InsightPermissionStateV1::DirectionalOnly
        );
        assert_eq!(readiness.permission_level, "directional_only");
        assert_eq!(readiness.assigned_sessions_control, Some(180));
        assert_eq!(readiness.assigned_sessions_challenger, Some(180));
        assert_eq!(readiness.control_outcome_events, Some(16));
        assert_eq!(readiness.challenger_outcome_events, Some(18));
        assert_eq!(readiness.assignment_rate_bps, Some(9000));
        assert_eq!(readiness.ambiguity_rate_bps, Some(200));
        assert_eq!(readiness.partial_or_unassigned_rate_bps, Some(800));
        assert_eq!(
            readiness.denominator_scope.as_deref(),
            Some("assigned_sessions_only")
        );
    }

    #[test]
    fn observed_pair_with_design_and_thresholds_is_decision_ready() {
        let mut input = observed_pair_input();
        input.causal_design_approved = true;
        let card = resolve_observed_experiment_pair_permission_v1(&input);
        assert_eq!(
            card.permission_state,
            InsightPermissionStateV1::AllowedOperationalClaim
        );
        assert!(card
            .allowed_uses
            .contains(&"use_for_routing_or_budget_reallocation_decisions".to_string()));
    }

    #[test]
    fn observed_pair_missing_variant_row_stays_insufficient() {
        let mut input = observed_pair_input();
        input.causal_design_approved = true;
        input.observed.funnel_rows.pop();
        let readiness = resolve_observed_experiment_pair_readiness_v1(&input);
        assert_eq!(
            readiness.readiness_state,
            InsightPermissionStateV1::InsufficientEvidence
        );
        assert!(readiness
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("challenger_variant_missing_from_assigned_funnels")));
    }

    #[test]
    fn observed_pair_missing_required_guardrail_dimension_requires_instrumentation() {
        let mut input = observed_pair_input();
        input
            .observed
            .guardrail_slices
            .retain(|slice| slice.dimension_key != "country");
        let readiness = resolve_observed_experiment_pair_readiness_v1(&input);
        assert_eq!(
            readiness.readiness_state,
            InsightPermissionStateV1::InstrumentFirst
        );
        assert!(readiness
            .blocking_reasons
            .contains(&"missing_guardrail_dimension=country".to_string()));
    }
}
