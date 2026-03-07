// provenance: decision_id=DEC-0015; change_request_id=CR-QA_FIXER-0032
use super::contracts::{
    AttributionDeltaReportV1, AttributionDeltaRowV1, ChannelMixPointV1, DailyRevenuePointV1,
    DataQualityScorecardV1, DataQualitySummaryV1, DecisionFeedCardV1, ExecutiveDashboardSnapshotV1,
    ExperimentAnalyticsSummaryV1, ExperimentGovernanceItemV1, ExperimentGovernanceReportV1,
    ForecastSummaryV1, FunnelSummaryV1, FunnelSurvivalPointV1, FunnelSurvivalReportV1,
    Ga4SessionRollupV1, HighLeverageReportsV1, KpiTileV1, PersistedAnalyticsRunV1, PortfolioRowV1,
    PublishExportGateV1, QualityCheckApplicabilityV1, RevenueTruthReportV1,
    StorefrontBehaviorSummaryV1,
};
use super::ga4_sessions::{
    build_experiment_analytics_summary_from_sessions_v1, build_funnel_summary_from_sessions_v1,
    build_storefront_behavior_summary_from_sessions_v1,
};
use super::{
    load_attestation_key_registry_from_env_or_file, resolve_attestation_policy_v1,
    resolve_observed_experiment_pair_permission_v1, resolve_observed_experiment_pair_readiness_v1,
    verify_connector_attestation_with_registry_v1,
};
use crate::data_models::analytics::ReportMetrics;
use chrono::Utc;

const SNAPSHOT_SCHEMA_VERSION_V1: &str = "executive_dashboard_snapshot.v1";
const DEFAULT_COMPARE_WINDOW_RUNS: usize = 1;
const EXPERIMENT_MIN_ASSIGNED_SESSIONS_PER_ARM: u64 = 100;
const EXPERIMENT_MIN_OUTCOME_EVENTS_PER_ARM: u64 = 10;
const EXPERIMENT_MIN_ASSIGNMENT_RATE_BPS: u32 = 8_000;
const EXPERIMENT_MAX_AMBIGUITY_RATE_BPS: u32 = 500;
const EXPERIMENT_MAX_PARTIAL_OR_UNASSIGNED_RATE_BPS: u32 = 2_000;
const EXPERIMENT_MIN_GUARDRAIL_COVERAGE_BPS: u32 = 7_000;
const EXPERIMENT_REQUIRED_GUARDRAILS: [&str; 4] =
    ["device_category", "platform", "country", "source_medium"];

#[derive(Debug, Clone, Copy)]
pub struct SnapshotBuildOptions {
    pub compare_window_runs: usize,
    pub target_roas: Option<f64>,
    pub monthly_revenue_target: Option<f64>,
}

impl Default for SnapshotBuildOptions {
    fn default() -> Self {
        Self {
            compare_window_runs: DEFAULT_COMPARE_WINDOW_RUNS,
            target_roas: None,
            monthly_revenue_target: None,
        }
    }
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::executive_dashboard`
/// purpose: Build chart-ready executive dashboard payload from persisted analytics runs.
pub fn build_executive_dashboard_snapshot(
    profile_id: &str,
    runs: &[PersistedAnalyticsRunV1],
    options: SnapshotBuildOptions,
) -> Option<ExecutiveDashboardSnapshotV1> {
    let latest = runs.first()?;
    let compare_offset = options.compare_window_runs.max(1);
    let previous = runs.get(compare_offset);
    let latest_metrics = &latest.artifact.report.total_metrics;
    let previous_metrics = previous
        .map(|run| &run.artifact.report.total_metrics)
        .unwrap_or(latest_metrics);

    let confidence_cap = latest
        .artifact
        .historical_analysis
        .confidence_calibration
        .recommended_confidence_cap
        .clone();
    let source_class = latest
        .artifact
        .provenance
        .first()
        .map(|p| format!("{:?}", p.source_class).to_lowercase())
        .unwrap_or_else(|| "simulated".to_string());

    let mut alerts = Vec::new();
    let attestation_policy = resolve_attestation_policy_v1(&latest.request.profile_id).ok();
    let signature_present = latest
        .metadata
        .connector_attestation
        .fingerprint_signature
        .is_some();
    if !latest.artifact.quality_controls.is_healthy {
        alerts.push("Quality controls degraded".to_string());
    }
    if attestation_policy
        .as_ref()
        .map(|policy| policy.require_signed_attestations && !signature_present)
        .unwrap_or(true)
    {
        alerts.push("Attestation signature missing for required policy.".to_string());
    }
    for flag in &latest.artifact.historical_analysis.anomaly_flags {
        alerts.push(format!("Anomaly {}: {}", flag.metric_key, flag.reason));
    }

    let trust_status = if latest.artifact.quality_controls.is_healthy
        && attestation_policy
            .as_ref()
            .map(|policy| !policy.require_signed_attestations || signature_present)
            .unwrap_or(false)
    {
        "healthy".to_string()
    } else {
        "degraded".to_string()
    };
    let funnel_summary = build_funnel_summary(latest);
    let publish_export_gate = build_publish_export_gate(latest);
    let high_leverage_reports =
        build_high_leverage_reports(latest, &funnel_summary, &publish_export_gate);

    Some(ExecutiveDashboardSnapshotV1 {
        schema_version: SNAPSHOT_SCHEMA_VERSION_V1.to_string(),
        profile_id: profile_id.to_string(),
        generated_at_utc: Utc::now().to_rfc3339(),
        run_id: latest.metadata.run_id.clone(),
        date_range: latest.artifact.report.date_range.clone(),
        compare_window_runs: compare_offset.min(u8::MAX as usize) as u8,
        kpis: build_kpis(
            latest_metrics,
            previous_metrics,
            &confidence_cap,
            &source_class,
            options.target_roas,
        ),
        channel_mix_series: build_channel_mix_series(runs),
        daily_revenue_series: build_daily_revenue_series(latest),
        roas_target_band: options.target_roas,
        funnel_summary,
        storefront_behavior_summary: build_storefront_summary(latest),
        portfolio_rows: build_portfolio_rows(latest),
        forecast_summary: build_forecast(latest_metrics, runs, options),
        data_quality: latest.artifact.data_quality.clone(),
        budget: latest.artifact.budget.clone(),
        decision_feed: build_decision_feed(latest),
        publish_export_gate,
        source_coverage: latest.artifact.source_coverage.clone(),
        quality_controls: latest.artifact.quality_controls.clone(),
        historical_analysis: latest.artifact.historical_analysis.clone(),
        operator_summary: latest.artifact.operator_summary.clone(),
        high_leverage_reports,
        trust_status,
        alerts,
    })
}

fn build_daily_revenue_series(run: &PersistedAnalyticsRunV1) -> Vec<DailyRevenuePointV1> {
    run.artifact.daily_revenue_series.clone()
}

fn build_high_leverage_reports(
    run: &PersistedAnalyticsRunV1,
    funnel_summary: &FunnelSummaryV1,
    publish_export_gate: &PublishExportGateV1,
) -> HighLeverageReportsV1 {
    let experiment_analytics = build_experiment_analytics_report(run);
    HighLeverageReportsV1 {
        revenue_truth: build_revenue_truth_report(run),
        funnel_survival: build_funnel_survival_report(funnel_summary),
        attribution_delta: build_attribution_delta_report(run),
        data_quality_scorecard: build_data_quality_scorecard(run, publish_export_gate),
        experiment_governance: build_experiment_governance_report(run, &experiment_analytics),
        experiment_analytics,
    }
}

fn build_experiment_analytics_report(
    run: &PersistedAnalyticsRunV1,
) -> super::contracts::ExperimentAnalyticsSummaryV1 {
    build_experiment_analytics_summary_from_sessions_v1(&run.artifact.ga4_session_rollups)
}

#[derive(Debug, Clone, Default)]
struct ExperimentVariantCandidate {
    experiment_id: String,
    experiment_name: Option<String>,
    variant_id: String,
    variant_name: Option<String>,
    assigned_sessions: u64,
    landing_family_counts: std::collections::BTreeMap<String, u64>,
    taxonomy_version: Option<String>,
}

#[derive(Debug, Clone)]
struct ExperimentGovernanceCandidate {
    experiment_id: String,
    experiment_name: Option<String>,
    control_variant_id: String,
    control_label: String,
    control_landing_family: String,
    challenger_variant_id: String,
    challenger_label: String,
    challenger_landing_family: String,
    taxonomy_version: Option<String>,
}

fn collect_experiment_governance_candidates(
    sessions: &[Ga4SessionRollupV1],
) -> Vec<ExperimentGovernanceCandidate> {
    use std::collections::BTreeMap;

    let mut by_experiment: BTreeMap<String, Vec<ExperimentVariantCandidate>> = BTreeMap::new();
    for session in sessions {
        if session.experiment_context.assignment_status
            != super::contracts::ExperimentAssignmentStatusV1::Assigned
        {
            continue;
        }
        let Some(experiment_id) = session.experiment_context.experiment_id.clone() else {
            continue;
        };
        let Some(variant_id) = session.experiment_context.variant_id.clone() else {
            continue;
        };
        let variants = by_experiment.entry(experiment_id.clone()).or_default();
        let position = variants
            .iter()
            .position(|candidate| candidate.variant_id == variant_id);
        let candidate = if let Some(index) = position {
            &mut variants[index]
        } else {
            variants.push(ExperimentVariantCandidate {
                experiment_id,
                experiment_name: session.experiment_context.experiment_name.clone(),
                variant_id,
                variant_name: session.experiment_context.variant_name.clone(),
                ..Default::default()
            });
            variants.last_mut().expect("variant candidate inserted")
        };
        candidate.assigned_sessions = candidate.assigned_sessions.saturating_add(1);
        if candidate.experiment_name.is_none() {
            candidate.experiment_name = session.experiment_context.experiment_name.clone();
        }
        if candidate.variant_name.is_none() {
            candidate.variant_name = session.experiment_context.variant_name.clone();
        }
        if candidate.taxonomy_version.is_none() {
            candidate.taxonomy_version = session
                .landing_context
                .as_ref()
                .map(|context| context.taxonomy_version.clone());
        }
        let landing_family = session
            .landing_context
            .as_ref()
            .map(|context| context.landing_family.clone())
            .unwrap_or_else(|| "unknown_landing_family".to_string());
        *candidate
            .landing_family_counts
            .entry(landing_family)
            .or_insert(0) += 1;
    }

    by_experiment
        .into_values()
        .filter_map(|variants| build_experiment_governance_candidate(variants))
        .collect()
}

fn build_experiment_governance_candidate(
    mut variants: Vec<ExperimentVariantCandidate>,
) -> Option<ExperimentGovernanceCandidate> {
    if variants.is_empty() {
        return None;
    }
    variants.sort_by(|left, right| {
        right
            .assigned_sessions
            .cmp(&left.assigned_sessions)
            .then_with(|| left.variant_id.cmp(&right.variant_id))
    });
    let control_index = select_control_variant_index(&variants);
    let control = variants.get(control_index)?.clone();
    let challenger = variants
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != control_index)
        .max_by(|(_, left), (_, right)| {
            left.assigned_sessions
                .cmp(&right.assigned_sessions)
                .then_with(|| left.variant_id.cmp(&right.variant_id))
        })
        .map(|(_, variant)| variant.clone())
        .unwrap_or_else(|| ExperimentVariantCandidate {
            experiment_id: control.experiment_id.clone(),
            experiment_name: control.experiment_name.clone(),
            variant_id: "unresolved_challenger".to_string(),
            variant_name: Some("Unresolved Challenger".to_string()),
            assigned_sessions: 0,
            taxonomy_version: control.taxonomy_version.clone(),
            ..Default::default()
        });

    Some(ExperimentGovernanceCandidate {
        experiment_id: control.experiment_id.clone(),
        experiment_name: control.experiment_name.clone(),
        control_variant_id: control.variant_id.clone(),
        control_label: variant_label(&control),
        control_landing_family: dominant_landing_family(&control),
        challenger_variant_id: challenger.variant_id.clone(),
        challenger_label: variant_label(&challenger),
        challenger_landing_family: dominant_landing_family(&challenger),
        taxonomy_version: control.taxonomy_version.or(challenger.taxonomy_version),
    })
}

fn select_control_variant_index(variants: &[ExperimentVariantCandidate]) -> usize {
    variants
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| control_rank(left).cmp(&control_rank(right)))
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn control_rank(variant: &ExperimentVariantCandidate) -> (u8, u64, String) {
    let label = variant_label(variant).to_ascii_lowercase();
    let dominant_landing = dominant_landing_family(variant);
    let control_hint = u8::from(dominant_landing == "simply_raw_offer_lp");
    let explicit_control = u8::from(
        label.contains("control")
            || label.contains("baseline")
            || label.contains("current")
            || label.contains("simply raw"),
    );
    (
        control_hint
            .saturating_mul(2)
            .saturating_add(explicit_control),
        variant.assigned_sessions,
        variant.variant_id.clone(),
    )
}

fn dominant_landing_family(variant: &ExperimentVariantCandidate) -> String {
    variant
        .landing_family_counts
        .iter()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)))
        .map(|(landing_family, _)| landing_family.clone())
        .unwrap_or_else(|| "unknown_landing_family".to_string())
}

fn variant_label(variant: &ExperimentVariantCandidate) -> String {
    variant
        .variant_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| variant.variant_id.clone())
}

fn build_experiment_governance_report(
    run: &PersistedAnalyticsRunV1,
    experiment_analytics: &ExperimentAnalyticsSummaryV1,
) -> ExperimentGovernanceReportV1 {
    let scoped_candidates =
        collect_experiment_governance_candidates(&run.artifact.ga4_session_rollups);
    if scoped_candidates.is_empty() {
        return ExperimentGovernanceReportV1 {
            summary: "No experiment-assigned sessions available for governance evaluation."
                .to_string(),
            coverage_scope: "no_experiment_scope".to_string(),
            items: Vec::new(),
            notes: vec![
                "No experiment_id/variant_id pairs reached assigned-session status in this run."
                    .to_string(),
            ],
        };
    }

    if scoped_candidates.len() > 1 {
        let experiment_ids = scoped_candidates
            .iter()
            .map(|candidate| candidate.experiment_id.clone())
            .collect::<Vec<_>>();
        return ExperimentGovernanceReportV1 {
            summary: format!(
                "{} experiments observed, but auto-readiness is deferred until experiment-scoped coverage is explicit.",
                experiment_ids.len()
            ),
            coverage_scope: "run_scoped_multi_experiment_not_evaluated".to_string(),
            items: Vec::new(),
            notes: vec![
                format!("observed_experiment_ids={}", experiment_ids.join(",")),
                "Configure experiment-scoped dashboard evaluation before trusting automatic control/challenger claims across multiple experiments.".to_string(),
            ],
        };
    }

    let candidate = &scoped_candidates[0];
    let scoped_sessions = run
        .artifact
        .ga4_session_rollups
        .iter()
        .filter(|session| {
            session.experiment_context.experiment_id.as_deref()
                == Some(candidate.experiment_id.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let scoped_analytics = build_experiment_analytics_summary_from_sessions_v1(&scoped_sessions);
    let input = super::experiment_governance::ObservedExperimentPairAssessmentInputV1 {
        insight_id: format!("INS-EXP-{}", candidate.experiment_id),
        experiment_id: candidate.experiment_id.clone(),
        decision_target: format!("landing_experiment:{}", candidate.experiment_id),
        statement: format!(
            "Assess whether {} improves purchase_session_rate versus {} for experiment {}.",
            candidate.challenger_label, candidate.control_label, candidate.experiment_id
        ),
        control_landing_family: candidate.control_landing_family.clone(),
        challenger_landing_family: candidate.challenger_landing_family.clone(),
        control_variant_id: candidate.control_variant_id.clone(),
        challenger_variant_id: candidate.challenger_variant_id.clone(),
        primary_metric: "purchase_session_rate".to_string(),
        analysis_window: run.artifact.report.date_range.clone(),
        taxonomy_version: candidate.taxonomy_version.clone(),
        minimum_assigned_sessions_per_arm: EXPERIMENT_MIN_ASSIGNED_SESSIONS_PER_ARM,
        minimum_outcome_events_per_arm: EXPERIMENT_MIN_OUTCOME_EVENTS_PER_ARM,
        minimum_assignment_rate_bps: EXPERIMENT_MIN_ASSIGNMENT_RATE_BPS,
        maximum_ambiguity_rate_bps: EXPERIMENT_MAX_AMBIGUITY_RATE_BPS,
        maximum_partial_or_unassigned_rate_bps: EXPERIMENT_MAX_PARTIAL_OR_UNASSIGNED_RATE_BPS,
        minimum_guardrail_coverage_bps: EXPERIMENT_MIN_GUARDRAIL_COVERAGE_BPS,
        required_guardrail_dimensions: EXPERIMENT_REQUIRED_GUARDRAILS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        instrumentation_ready: experiment_analytics.assignment_coverage.assigned_sessions > 0,
        taxonomy_coverage_ready: candidate.taxonomy_version.is_some(),
        causal_design_approved: false,
        observed: scoped_analytics,
    };
    let permission = resolve_observed_experiment_pair_permission_v1(&input);
    let readiness = resolve_observed_experiment_pair_readiness_v1(&input);
    let item = ExperimentGovernanceItemV1 {
        experiment_id: candidate.experiment_id.clone(),
        experiment_name: candidate.experiment_name.clone(),
        permission,
        readiness,
    };

    ExperimentGovernanceReportV1 {
        summary: format!(
            "Evaluated 1 experiment pair using experiment-scoped observed sessions for {}.",
            candidate.experiment_id
        ),
        coverage_scope: "experiment_id_scoped_observed_sessions".to_string(),
        items: vec![item],
        notes: vec![
            format!(
                "control_variant={} challenger_variant={}",
                candidate.control_variant_id, candidate.challenger_variant_id
            ),
            "Experiment claims remain coupled to readiness state and assigned-session denominator scope.".to_string(),
        ],
    }
}

fn build_revenue_truth_report(run: &PersistedAnalyticsRunV1) -> RevenueTruthReportV1 {
    let strict_duplicate_ratio = find_quality_check_ratio(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_duplicate_event_signature_rate",
    )
    .unwrap_or(0.0);
    let near_duplicate_ratio = find_quality_check_ratio(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_near_duplicate_second_rate",
    )
    .unwrap_or(0.0);
    let risk = if near_duplicate_ratio >= 0.10 {
        "high"
    } else if near_duplicate_ratio >= 0.03 {
        "medium"
    } else {
        "low"
    };
    let canonical_revenue = finite_or_zero(run.artifact.report.total_metrics.conversions_value);
    let canonical_conversions = finite_or_zero(run.artifact.report.total_metrics.conversions);
    let estimated_revenue_at_risk = round4(canonical_revenue * near_duplicate_ratio.max(0.0));
    let custom_purchase_rows = find_quality_check_u64(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_overlap_rate",
        "purchase_ndp_rows",
    )
    .unwrap_or(0);
    let custom_purchase_overlap_rows = find_quality_check_u64(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_overlap_rate",
        "rows_with_canonical_purchase",
    )
    .unwrap_or(0);
    let custom_purchase_orphan_rows = find_quality_check_u64(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_orphan_rate",
        "orphan_rows",
    )
    .unwrap_or(0);
    let custom_purchase_overlap_ratio = find_quality_check_f64(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_overlap_rate",
        "overlap_ratio",
    )
    .unwrap_or(0.0);
    let custom_purchase_orphan_ratio = find_quality_check_f64(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_orphan_rate",
        "orphan_ratio",
    )
    .unwrap_or(0.0);
    let custom_schema_failed = has_failed_quality_check(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_schema_integrity",
    );
    let custom_overlap_failed = has_failed_quality_check(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_overlap_rate",
    );
    let custom_orphan_failed = has_failed_quality_check(
        &run.artifact.quality_controls.schema_drift_checks,
        "ga4_custom_purchase_ndp_orphan_rate",
    );
    let truth_guard_status = if custom_purchase_rows == 0 {
        "canonical_only"
    } else if custom_schema_failed || custom_overlap_failed || custom_orphan_failed {
        "guarded_review_required"
    } else {
        "guarded_clean"
    };
    let summary = format!(
        "Canonical purchase metrics enforced. strict_duplicate_ratio={:.4}, near_duplicate_ratio={:.4}, custom_purchase_rows={}, overlap_ratio={:.4}, orphan_ratio={:.4}, truth_guard_status={}, inflation_risk={}",
        strict_duplicate_ratio,
        near_duplicate_ratio,
        custom_purchase_rows,
        custom_purchase_overlap_ratio,
        custom_purchase_orphan_ratio,
        truth_guard_status,
        risk
    );
    RevenueTruthReportV1 {
        canonical_revenue,
        canonical_conversions,
        strict_duplicate_ratio,
        near_duplicate_ratio,
        custom_purchase_rows,
        custom_purchase_overlap_rows,
        custom_purchase_orphan_rows,
        custom_purchase_overlap_ratio,
        custom_purchase_orphan_ratio,
        truth_guard_status: truth_guard_status.to_string(),
        inflation_risk: risk.to_string(),
        estimated_revenue_at_risk,
        summary,
    }
}

fn build_funnel_survival_report(funnel_summary: &FunnelSummaryV1) -> FunnelSurvivalReportV1 {
    if funnel_summary.stages.is_empty() {
        return FunnelSurvivalReportV1 {
            points: Vec::new(),
            bottleneck_stage: "none".to_string(),
        };
    }
    let mut points = Vec::with_capacity(funnel_summary.stages.len());
    let mut cumulative_survival = 1.0;
    let mut bottleneck_stage = funnel_summary.dropoff_hotspot_stage.clone();
    let mut max_hazard = -1.0;

    for (index, stage) in funnel_summary.stages.iter().enumerate() {
        let entrants = finite_or_zero(stage.value.max(0.0));
        let (survival_rate, hazard_rate) = if index == 0 {
            (1.0, 0.0)
        } else {
            let transition = stage
                .conversion_from_previous
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            cumulative_survival *= transition;
            let hazard = 1.0 - transition;
            if hazard > max_hazard {
                max_hazard = hazard;
                bottleneck_stage = stage.stage.clone();
            }
            (cumulative_survival.clamp(0.0, 1.0), hazard)
        };
        points.push(FunnelSurvivalPointV1 {
            stage: stage.stage.clone(),
            entrants,
            survival_rate: round4(survival_rate),
            hazard_rate: round4(hazard_rate.clamp(0.0, 1.0)),
        });
    }

    FunnelSurvivalReportV1 {
        points,
        bottleneck_stage,
    }
}

fn build_attribution_delta_report(run: &PersistedAnalyticsRunV1) -> AttributionDeltaReportV1 {
    let campaigns = &run.artifact.report.campaign_data;
    if campaigns.is_empty() {
        return AttributionDeltaReportV1 {
            rows: Vec::new(),
            dominant_last_touch_campaign: None,
            last_touch_concentration_hhi: 0.0,
            summary: "No campaign rows available for attribution delta analysis.".to_string(),
        };
    }
    let total_impressions = campaigns
        .iter()
        .map(|row| row.metrics.impressions as f64)
        .sum::<f64>();
    let total_clicks = campaigns
        .iter()
        .map(|row| row.metrics.clicks as f64)
        .sum::<f64>();
    let total_revenue = campaigns
        .iter()
        .map(|row| row.metrics.conversions_value)
        .sum::<f64>();

    let mut rows = campaigns
        .iter()
        .map(|row| {
            let first_touch_proxy_share = if total_impressions > 0.0 {
                row.metrics.impressions as f64 / total_impressions
            } else {
                0.0
            };
            let assist_share = if total_clicks > 0.0 {
                row.metrics.clicks as f64 / total_clicks
            } else {
                0.0
            };
            let last_touch_share = if total_revenue > 0.0 {
                row.metrics.conversions_value / total_revenue
            } else {
                0.0
            };
            AttributionDeltaRowV1 {
                campaign: row.campaign_name.clone(),
                first_touch_proxy_share: round4(first_touch_proxy_share),
                assist_share: round4(assist_share),
                last_touch_share: round4(last_touch_share),
                delta_first_vs_last: round4(first_touch_proxy_share - last_touch_share),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.delta_first_vs_last
            .abs()
            .total_cmp(&a.delta_first_vs_last.abs())
    });

    let dominant_last_touch_campaign = rows
        .iter()
        .max_by(|a, b| a.last_touch_share.total_cmp(&b.last_touch_share))
        .map(|row| row.campaign.clone());
    let last_touch_concentration_hhi =
        round4(rows.iter().map(|row| row.last_touch_share.powi(2)).sum());
    let summary = if last_touch_concentration_hhi >= 0.25 {
        format!(
            "Last-touch revenue is concentrated (HHI={:.4}); validate channel credit assignments.",
            last_touch_concentration_hhi
        )
    } else {
        format!(
            "Last-touch revenue concentration is moderate (HHI={:.4}).",
            last_touch_concentration_hhi
        )
    };
    AttributionDeltaReportV1 {
        rows,
        dominant_last_touch_campaign,
        last_touch_concentration_hhi,
        summary,
    }
}

fn build_data_quality_scorecard(
    run: &PersistedAnalyticsRunV1,
    publish_export_gate: &PublishExportGateV1,
) -> DataQualityScorecardV1 {
    let quality = &run.artifact.quality_controls;
    let high_severity_failures = quality
        .schema_drift_checks
        .iter()
        .chain(quality.identity_resolution_checks.iter())
        .chain(quality.freshness_sla_checks.iter())
        .chain(quality.cross_source_checks.iter())
        .chain(quality.budget_checks.iter())
        .filter(|check| {
            check.applicability == QualityCheckApplicabilityV1::Applies
                && !check.passed
                && check.severity.eq_ignore_ascii_case("high")
        })
        .count() as u32;

    DataQualityScorecardV1 {
        quality_score: run.artifact.data_quality.quality_score,
        completeness_ratio: run.artifact.data_quality.completeness_ratio,
        freshness_pass_ratio: run.artifact.data_quality.freshness_pass_ratio,
        reconciliation_pass_ratio: run.artifact.data_quality.reconciliation_pass_ratio,
        cross_source_pass_ratio: run.artifact.data_quality.cross_source_pass_ratio,
        budget_pass_ratio: run.artifact.data_quality.budget_pass_ratio,
        high_severity_failures,
        blocking_reasons_count: publish_export_gate.blocking_reasons.len() as u32,
        warning_reasons_count: publish_export_gate.warning_reasons.len() as u32,
        gate_status: publish_export_gate.gate_status.clone(),
    }
}

fn find_quality_check_ratio(
    checks: &[super::contracts::QualityCheckV1],
    code: &str,
) -> Option<f64> {
    find_quality_check_f64(checks, code, "ratio")
}

fn find_quality_check_f64(
    checks: &[super::contracts::QualityCheckV1],
    code: &str,
    key: &str,
) -> Option<f64> {
    checks
        .iter()
        .find(|check| check.code == code)
        .and_then(|check| parse_observed_metric_f64(&check.observed, key))
}

fn find_quality_check_u64(
    checks: &[super::contracts::QualityCheckV1],
    code: &str,
    key: &str,
) -> Option<u64> {
    checks
        .iter()
        .find(|check| check.code == code)
        .and_then(|check| parse_observed_metric_u64(&check.observed, key))
}

fn has_failed_quality_check(checks: &[super::contracts::QualityCheckV1], code: &str) -> bool {
    checks.iter().any(|check| {
        check.code == code
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    })
}

fn parse_observed_metric_value<'a>(observed: &'a str, key: &str) -> Option<&'a str> {
    observed.split(',').find_map(|segment| {
        let (segment_key, segment_value) = segment.trim().split_once('=')?;
        (segment_key.trim() == key).then_some(segment_value.trim())
    })
}

fn parse_observed_metric_f64(observed: &str, key: &str) -> Option<f64> {
    parse_observed_metric_value(observed, key)?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_observed_metric_u64(observed: &str, key: &str) -> Option<u64> {
    parse_observed_metric_value(observed, key)?
        .parse::<u64>()
        .ok()
}

fn build_decision_feed(run: &PersistedAnalyticsRunV1) -> Vec<DecisionFeedCardV1> {
    let mut cards = Vec::new();
    let quality = &run.artifact.quality_controls;
    let historical = &run.artifact.historical_analysis;
    let experiment_analytics = build_experiment_analytics_report(run);

    let failed_schema = quality
        .schema_drift_checks
        .iter()
        .filter(|check| {
            check.applicability == QualityCheckApplicabilityV1::Applies
                && !check.passed
                && check.severity.eq_ignore_ascii_case("high")
        })
        .count();
    if failed_schema > 0 {
        cards.push(DecisionFeedCardV1 {
            card_id: "schema-drift".to_string(),
            priority: "critical".to_string(),
            status: "blocked".to_string(),
            title: "Schema drift detected".to_string(),
            summary: format!("{failed_schema} schema-drift checks failed in latest run."),
            recommended_action: "Pause publish/export and reconcile upstream field mappings."
                .to_string(),
            evidence_refs: vec![
                "quality_controls.schema_drift_checks".to_string(),
                format!("run_id={}", run.metadata.run_id),
            ],
        });
    }
    if let Some(check) = quality.schema_drift_checks.iter().find(|check| {
        check.code == "ga4_custom_purchase_ndp_schema_integrity"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        cards.push(DecisionFeedCardV1 {
            card_id: "custom-purchase-schema".to_string(),
            priority: "medium".to_string(),
            status: "review_required".to_string(),
            title: "Custom purchase stream lacks truth fields".to_string(),
            summary: format!(
                "`purchase_ndp` is missing transaction/value fields ({}). It remains excluded from truth KPIs.",
                check.observed
            ),
            recommended_action:
                "Fix or retire the custom purchase event; keep relying on canonical `purchase` for revenue truth."
                    .to_string(),
            evidence_refs: vec![
                "quality_controls.schema_drift_checks.ga4_custom_purchase_ndp_schema_integrity"
                    .to_string(),
            ],
        });
    }
    if let Some(check) = quality.schema_drift_checks.iter().find(|check| {
        check.code == "ga4_custom_purchase_ndp_overlap_rate"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        cards.push(DecisionFeedCardV1 {
            card_id: "custom-purchase-overlap".to_string(),
            priority: "medium".to_string(),
            status: "review_required".to_string(),
            title: "Duplicate custom purchase stream still active".to_string(),
            summary: format!(
                "`purchase_ndp` overlaps canonical `purchase` events ({}). Revenue KPIs stay guarded, but duplicate instrumentation remains live.",
                check.observed
            ),
            recommended_action:
                "Disable redundant `purchase_ndp` emission or quarantine it from all downstream exports."
                    .to_string(),
            evidence_refs: vec![
                "quality_controls.schema_drift_checks.ga4_custom_purchase_ndp_overlap_rate"
                    .to_string(),
                "high_leverage_reports.revenue_truth".to_string(),
            ],
        });
    }
    if let Some(check) = quality.schema_drift_checks.iter().find(|check| {
        check.code == "ga4_custom_purchase_ndp_orphan_rate"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        cards.push(DecisionFeedCardV1 {
            card_id: "custom-purchase-orphans".to_string(),
            priority: "high".to_string(),
            status: "investigate".to_string(),
            title: "Custom purchase orphan rows detected".to_string(),
            summary: format!(
                "`purchase_ndp` emitted rows without nearby canonical purchases ({}). Revenue completeness may be understated for some checkouts.",
                check.observed
            ),
            recommended_action:
                "Audit checkout tagging and confirm canonical `purchase` fires on the same sessions before acting on completeness-sensitive revenue decisions."
                    .to_string(),
            evidence_refs: vec![
                "quality_controls.schema_drift_checks.ga4_custom_purchase_ndp_orphan_rate"
                    .to_string(),
                "high_leverage_reports.revenue_truth".to_string(),
            ],
        });
    }

    let failed_identity = quality
        .identity_resolution_checks
        .iter()
        .filter(|check| !check.passed)
        .count();
    if failed_identity > 0 {
        cards.push(DecisionFeedCardV1 {
            card_id: "identity-resolution".to_string(),
            priority: "high".to_string(),
            status: "action_required".to_string(),
            title: "Identity resolution degraded".to_string(),
            summary: format!(
                "{failed_identity} identity checks failed; cross-source joins may be unreliable."
            ),
            recommended_action:
                "Review Wix/GA identity stitching before acting on segment-level recommendations."
                    .to_string(),
            evidence_refs: vec!["quality_controls.identity_resolution_checks".to_string()],
        });
    }
    let failed_cross_source = quality
        .cross_source_checks
        .iter()
        .filter(|check| !check.passed)
        .count();
    if failed_cross_source > 0 {
        cards.push(DecisionFeedCardV1 {
            card_id: "cross-source-reconciliation".to_string(),
            priority: "high".to_string(),
            status: "action_required".to_string(),
            title: "Cross-source reconciliation degraded".to_string(),
            summary: format!(
                "{failed_cross_source} cross-source checks failed; attribution assumptions may be unstable."
            ),
            recommended_action:
                "Reconcile GA4 / Google Ads / Wix rollups before publishing executive guidance."
                    .to_string(),
            evidence_refs: vec!["quality_controls.cross_source_checks".to_string()],
        });
    }
    let cross_source_not_applicable = !quality.cross_source_checks.is_empty()
        && quality
            .cross_source_checks
            .iter()
            .all(|check| check.applicability == QualityCheckApplicabilityV1::NotApplicable);
    if cross_source_not_applicable {
        cards.push(DecisionFeedCardV1 {
            card_id: "cross-source-not-applicable".to_string(),
            priority: "low".to_string(),
            status: "monitor".to_string(),
            title: "Cross-source checks intentionally not applicable".to_string(),
            summary:
                "Current connector topology does not provide independent Ads/Wix source streams."
                    .to_string(),
            recommended_action:
                "Use GA4-derived trends for directional decisions; enable independent connectors before relying on cross-source reconciliation."
                    .to_string(),
            evidence_refs: vec!["quality_controls.cross_source_checks".to_string()],
        });
    }

    for anomaly in historical.anomaly_flags.iter().take(3) {
        cards.push(DecisionFeedCardV1 {
            card_id: format!("anomaly-{}", anomaly.metric_key),
            priority: if anomaly.severity == "high" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            status: "investigate".to_string(),
            title: format!("Anomaly flagged for {}", anomaly.metric_key),
            summary: anomaly.reason.clone(),
            recommended_action: "Inspect campaign mix and attribution windows for root cause."
                .to_string(),
            evidence_refs: vec![format!(
                "historical_analysis.anomaly_flags.{}",
                anomaly.metric_key
            )],
        });
    }

    if !run.artifact.ingest_cleaning_notes.is_empty() {
        cards.push(DecisionFeedCardV1 {
            card_id: "ingest-cleaning".to_string(),
            priority: "medium".to_string(),
            status: "review_required".to_string(),
            title: "Input normalization applied".to_string(),
            summary: format!(
                "{} field-level normalizations were applied at ingest.",
                run.artifact.ingest_cleaning_notes.len()
            ),
            recommended_action: "Review ingest cleaning audit notes before publishing operator packets."
                .to_string(),
            evidence_refs: vec!["ingest_cleaning_notes".to_string()],
        });
    }
    if run.artifact.budget.clipped
        || run.artifact.budget.sampled
        || run.artifact.budget.incomplete_output
    {
        cards.push(DecisionFeedCardV1 {
            card_id: "budget-cap-hit".to_string(),
            priority: "high".to_string(),
            status: "action_required".to_string(),
            title: "Budget cap hit - output incomplete".to_string(),
            summary: format!(
                "Budget policy clipped/sampled execution. skipped_modules={}",
                run.artifact.budget.skipped_modules.join(",")
            ),
            recommended_action:
                "Review budget panel and rerun with higher envelope for full-fidelity output."
                    .to_string(),
            evidence_refs: vec![
                "budget.events".to_string(),
                "budget.skipped_modules".to_string(),
            ],
        });
    }
    for source in run
        .artifact
        .source_coverage
        .iter()
        .filter(|item| item.enabled && !item.observed)
    {
        cards.push(DecisionFeedCardV1 {
            card_id: format!("source-coverage-{}", source.source_system),
            priority: "medium".to_string(),
            status: "review_required".to_string(),
            title: format!("{} has no rows in selected window", source.source_system),
            summary: source
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "source returned zero rows for this run".to_string()),
            recommended_action:
                "Validate date window and upstream source availability before publishing."
                    .to_string(),
            evidence_refs: vec!["source_coverage".to_string()],
        });
    }
    if experiment_analytics
        .assignment_coverage
        .total_observed_sessions
        > 0
    {
        if experiment_analytics.assignment_coverage.assigned_sessions == 0 {
            cards.push(DecisionFeedCardV1 {
                card_id: "experiment-assignment-missing".to_string(),
                priority: "medium".to_string(),
                status: "instrument_first".to_string(),
                title: "Experiment assignment missing from observed sessions".to_string(),
                summary: experiment_analytics.assignment_coverage.summary.clone(),
                recommended_action:
                    "Add explicit experiment_id and variant_id to the GA4 event stream before using landing challengers as decision-grade facts."
                        .to_string(),
                evidence_refs: vec!["high_leverage_reports.experiment_analytics".to_string()],
            });
        } else if experiment_analytics.assignment_coverage.partial_sessions > 0
            || experiment_analytics.assignment_coverage.ambiguous_sessions > 0
        {
            cards.push(DecisionFeedCardV1 {
                card_id: "experiment-assignment-incomplete".to_string(),
                priority: "medium".to_string(),
                status: "review_required".to_string(),
                title: "Experiment assignment coverage is incomplete".to_string(),
                summary: experiment_analytics.assignment_coverage.summary.clone(),
                recommended_action:
                    "Use assigned sessions only for variant funnels, and keep ambiguous or partial sessions out of content-pipeline claims."
                        .to_string(),
                evidence_refs: vec!["high_leverage_reports.experiment_analytics".to_string()],
            });
        }
    }

    if cards.is_empty() {
        cards.push(DecisionFeedCardV1 {
            card_id: "green-status".to_string(),
            priority: "low".to_string(),
            status: "monitor".to_string(),
            title: "Pipeline stable".to_string(),
            summary: "No blocking quality failures or severe anomalies in current window."
                .to_string(),
            recommended_action: "Continue monitoring and publish with standard review cadence."
                .to_string(),
            evidence_refs: vec![format!("run_id={}", run.metadata.run_id)],
        });
    }

    cards
}

fn build_publish_export_gate(run: &PersistedAnalyticsRunV1) -> PublishExportGateV1 {
    let quality = &run.artifact.quality_controls;
    let historical = &run.artifact.historical_analysis;
    let data_quality = &run.artifact.data_quality;
    let mut blocking_reasons = Vec::new();
    let mut warning_reasons = Vec::new();
    if let Some(message) = validate_data_quality_bounds(data_quality) {
        blocking_reasons.push(message);
    }

    if !quality.schema_drift_checks.iter().all(|check| {
        check.applicability == QualityCheckApplicabilityV1::NotApplicable
            || check.passed
            || check.severity != "high"
    }) {
        blocking_reasons.push("High-severity schema-drift failure present.".to_string());
    }
    if quality.schema_drift_checks.iter().any(|check| {
        check.code == "ga4_custom_purchase_ndp_schema_integrity"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        warning_reasons.push(
            "Custom purchase event `purchase_ndp` failed schema integrity (missing transaction_id/value); event remains excluded from truth KPIs."
                .to_string(),
        );
    }
    if quality.schema_drift_checks.iter().any(|check| {
        check.code == "ga4_custom_purchase_ndp_overlap_rate"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        warning_reasons.push(
            "Custom purchase event `purchase_ndp` still overlaps canonical `purchase` events; duplicate stream remains active but excluded from truth KPIs."
                .to_string(),
        );
    }
    if quality.schema_drift_checks.iter().any(|check| {
        check.code == "ga4_custom_purchase_ndp_orphan_rate"
            && check.applicability == QualityCheckApplicabilityV1::Applies
            && !check.passed
    }) {
        warning_reasons.push(
            "Custom purchase event `purchase_ndp` has orphan rows without nearby canonical purchases; investigate possible checkout undercount."
                .to_string(),
        );
    }

    if !quality.freshness_sla_checks.iter().all(|check| {
        check.applicability == QualityCheckApplicabilityV1::NotApplicable
            || check.passed
            || check.severity != "high"
    }) {
        blocking_reasons.push("High-severity freshness SLA failure present.".to_string());
    }

    if !quality.identity_resolution_checks.iter().all(|check| {
        check.applicability == QualityCheckApplicabilityV1::NotApplicable
            || check.passed
            || check.severity != "high"
    }) {
        blocking_reasons.push("High-severity identity-resolution failure present.".to_string());
    }
    if !quality.cross_source_checks.iter().all(|check| {
        check.applicability == QualityCheckApplicabilityV1::NotApplicable
            || check.passed
            || check.severity != "high"
    }) {
        blocking_reasons
            .push("High-severity cross-source reconciliation failure present.".to_string());
    }

    if historical
        .anomaly_flags
        .iter()
        .any(|flag| flag.severity == "high")
    {
        warning_reasons.push("High-severity anomaly flagged; require operator review.".to_string());
    }

    if data_quality.completeness_ratio < 0.99 {
        blocking_reasons.push(format!(
            "Data completeness below threshold: {:.2}%",
            data_quality.completeness_ratio * 100.0
        ));
    }
    if data_quality.identity_join_coverage_ratio < 0.98 {
        blocking_reasons.push(format!(
            "Join coverage below threshold: {:.2}%",
            data_quality.identity_join_coverage_ratio * 100.0
        ));
    }
    if data_quality.freshness_pass_ratio < 0.95 {
        warning_reasons.push(format!(
            "Freshness pass ratio degraded: {:.2}%",
            data_quality.freshness_pass_ratio * 100.0
        ));
    }
    if data_quality.reconciliation_pass_ratio < 1.0 {
        warning_reasons.push("Reconciliation checks not fully passing.".to_string());
    }
    if data_quality.cross_source_pass_ratio < 0.95 {
        warning_reasons.push(format!(
            "Cross-source pass ratio degraded: {:.2}%",
            data_quality.cross_source_pass_ratio * 100.0
        ));
    }
    if data_quality.cross_source_applicability_ratio == 0.0 {
        warning_reasons.push(
            "Cross-source reconciliation is not applicable for the current source topology."
                .to_string(),
        );
    }
    if data_quality.budget_pass_ratio < 1.0 {
        blocking_reasons.push("Budget checks not fully passing.".to_string());
    }
    if run
        .artifact
        .quality_controls
        .budget_checks
        .iter()
        .any(|check| {
            check.applicability == QualityCheckApplicabilityV1::Applies
                && !check.passed
                && check.severity.eq_ignore_ascii_case("high")
        })
    {
        blocking_reasons.push("High-severity budget check failure present.".to_string());
    }
    if run
        .artifact
        .budget
        .events
        .iter()
        .any(|event| event.outcome.eq_ignore_ascii_case("blocked"))
    {
        blocking_reasons.push("Budget spend exceeded envelope.".to_string());
    }
    if run.artifact.budget.daily_spent_after_micros > run.artifact.budget.hard_daily_cap_micros {
        blocking_reasons.push("Daily hard budget cap exceeded.".to_string());
    }
    if run
        .artifact
        .ingest_cleaning_notes
        .iter()
        .any(|note| note.severity.eq_ignore_ascii_case("block"))
    {
        blocking_reasons.push("Blocking ingest cleaning notes present.".to_string());
    }
    match resolve_attestation_policy_v1(&run.request.profile_id) {
        Ok(policy) => {
            if policy.require_signed_attestations {
                let attestation = &run.metadata.connector_attestation;
                let signature_present = attestation
                    .fingerprint_signature
                    .as_ref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
                if !signature_present {
                    blocking_reasons.push("Signed attestation required by policy.".to_string());
                } else {
                    match load_attestation_key_registry_from_env_or_file() {
                        Ok(Some(registry)) => {
                            if let Err(err) = verify_connector_attestation_with_registry_v1(
                                &run.metadata.run_id,
                                &run.metadata.run_id,
                                attestation,
                                &registry,
                            ) {
                                blocking_reasons.push(format!(
                                    "Signed attestation verification failed: {}",
                                    err.code
                                ));
                            }
                        }
                        Ok(None) => {
                            blocking_reasons.push(
                                "Signed attestation required by policy but key registry is not configured."
                                    .to_string(),
                            );
                        }
                        Err(err) => {
                            blocking_reasons.push(format!(
                                "Attestation key registry configuration invalid: {}",
                                err.code
                            ));
                        }
                    }
                }
            }
        }
        Err(_) => {
            blocking_reasons
                .push("Attestation policy configuration invalid (fail-closed).".to_string());
        }
    }

    let publish_ready = blocking_reasons.is_empty();
    let export_ready = blocking_reasons.is_empty();
    let gate_status = if !publish_ready {
        "blocked".to_string()
    } else if warning_reasons.is_empty() {
        "ready".to_string()
    } else {
        "review_required".to_string()
    };

    PublishExportGateV1 {
        publish_ready,
        export_ready,
        blocking_reasons,
        warning_reasons,
        gate_status,
    }
}

fn validate_data_quality_bounds(data_quality: &DataQualitySummaryV1) -> Option<String> {
    let checks = [
        ("completeness_ratio", data_quality.completeness_ratio),
        (
            "identity_join_coverage_ratio",
            data_quality.identity_join_coverage_ratio,
        ),
        (
            "identity_applicability_ratio",
            data_quality.identity_applicability_ratio,
        ),
        ("freshness_pass_ratio", data_quality.freshness_pass_ratio),
        (
            "reconciliation_pass_ratio",
            data_quality.reconciliation_pass_ratio,
        ),
        (
            "cross_source_pass_ratio",
            data_quality.cross_source_pass_ratio,
        ),
        (
            "cross_source_applicability_ratio",
            data_quality.cross_source_applicability_ratio,
        ),
        ("budget_pass_ratio", data_quality.budget_pass_ratio),
        ("quality_score", data_quality.quality_score),
    ];
    for (name, value) in checks {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Some(format!("Data quality ratio '{}' is outside [0,1].", name));
        }
    }
    None
}

fn build_kpis(
    current: &ReportMetrics,
    baseline: &ReportMetrics,
    confidence_label: &str,
    source_class: &str,
    target_roas: Option<f64>,
) -> Vec<KpiTileV1> {
    vec![
        kpi(
            "spend",
            "Spend",
            current.cost,
            baseline.cost,
            format!("${:.2}", current.cost),
            confidence_label,
            source_class,
            None,
        ),
        kpi(
            "revenue",
            "Revenue",
            current.conversions_value,
            baseline.conversions_value,
            format!("${:.2}", current.conversions_value),
            confidence_label,
            source_class,
            None,
        ),
        kpi(
            "roas",
            "ROAS",
            current.roas,
            baseline.roas,
            format!("{:.2}x", current.roas),
            confidence_label,
            source_class,
            target_roas,
        ),
        kpi(
            "conversions",
            "Conversions",
            current.conversions,
            baseline.conversions,
            format!("{:.2}", current.conversions),
            confidence_label,
            source_class,
            None,
        ),
        kpi(
            "ctr",
            "CTR",
            current.ctr,
            baseline.ctr,
            format!("{:.2}%", current.ctr),
            confidence_label,
            source_class,
            None,
        ),
        kpi(
            "cpa",
            "CPA",
            current.cpa,
            baseline.cpa,
            format!("${:.2}", current.cpa),
            confidence_label,
            source_class,
            None,
        ),
        kpi(
            "aov",
            "AOV",
            average_order_value(current),
            average_order_value(baseline),
            format!("${:.2}", average_order_value(current)),
            confidence_label,
            source_class,
            None,
        ),
    ]
}

fn kpi(
    key: &str,
    label: &str,
    value: f64,
    baseline: f64,
    formatted_value: String,
    confidence_label: &str,
    source_class: &str,
    target_value: Option<f64>,
) -> KpiTileV1 {
    let delta_percent = if baseline.abs() > f64::EPSILON {
        Some((value - baseline) / baseline)
    } else {
        None
    };
    let target_delta_percent = target_value.and_then(|target| {
        if target.abs() > f64::EPSILON {
            Some((value - target) / target)
        } else {
            None
        }
    });

    KpiTileV1 {
        key: key.to_string(),
        label: label.to_string(),
        value,
        formatted_value,
        delta_percent,
        target_delta_percent,
        confidence_label: confidence_label.to_string(),
        source_class: source_class.to_string(),
    }
}

fn build_channel_mix_series(runs: &[PersistedAnalyticsRunV1]) -> Vec<ChannelMixPointV1> {
    let mut series = runs
        .iter()
        .take(12)
        .map(|run| ChannelMixPointV1 {
            period_label: run.artifact.report.date_range.replace(" to ", " -> "),
            spend: run.artifact.report.total_metrics.cost,
            revenue: run.artifact.report.total_metrics.conversions_value,
            roas: run.artifact.report.total_metrics.roas,
        })
        .collect::<Vec<_>>();
    series.reverse();
    series
}

fn build_funnel_summary(run: &PersistedAnalyticsRunV1) -> FunnelSummaryV1 {
    build_funnel_summary_from_sessions_v1(&run.artifact.ga4_session_rollups)
}

fn build_storefront_summary(run: &PersistedAnalyticsRunV1) -> StorefrontBehaviorSummaryV1 {
    let observed =
        build_storefront_behavior_summary_from_sessions_v1(&run.artifact.ga4_session_rollups);
    if !observed.rows.is_empty() {
        return observed;
    }

    let ga4_observed = run
        .artifact
        .source_coverage
        .iter()
        .find(|item| item.source_system == "ga4")
        .map(|item| item.observed)
        .unwrap_or(false);
    if !ga4_observed {
        return StorefrontBehaviorSummaryV1 {
            source_system: "storefront_not_available".to_string(),
            identity_confidence: "not_available".to_string(),
            rows: Vec::new(),
        };
    }

    StorefrontBehaviorSummaryV1 {
        source_system: "ga4_session_rollup_unavailable".to_string(),
        identity_confidence: "low".to_string(),
        rows: Vec::new(),
    }
}

fn build_portfolio_rows(run: &PersistedAnalyticsRunV1) -> Vec<PortfolioRowV1> {
    let mut rows = run
        .artifact
        .report
        .campaign_data
        .iter()
        .map(|row| PortfolioRowV1 {
            campaign: row.campaign_name.clone(),
            spend: row.metrics.cost,
            revenue: row.metrics.conversions_value,
            roas: row.metrics.roas,
            ctr: row.metrics.ctr,
            cpa: row.metrics.cpa,
            conversions: row.metrics.conversions,
            drift_severity: run
                .artifact
                .historical_analysis
                .drift_flags
                .iter()
                .find(|flag| flag.metric_key == "conversions")
                .map(|flag| flag.severity.clone()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.roas.total_cmp(&a.roas));
    rows
}

fn build_forecast(
    metrics: &ReportMetrics,
    runs: &[PersistedAnalyticsRunV1],
    options: SnapshotBuildOptions,
) -> ForecastSummaryV1 {
    let avg_roas = if runs.is_empty() {
        metrics.roas
    } else {
        let sample = runs.iter().take(8);
        let count = sample.clone().count();
        if count == 0 {
            metrics.roas
        } else {
            sample
                .map(|run| run.artifact.report.total_metrics.roas)
                .sum::<f64>()
                / count as f64
        }
    };

    let expected_revenue_next_period = finite_or_zero(metrics.cost * avg_roas);
    let month_to_date_revenue = finite_or_zero(metrics.conversions_value);
    let monthly_revenue_target = options.monthly_revenue_target;
    let pacing_status = match monthly_revenue_target {
        Some(target) if target > 0.0 => {
            let ratio = month_to_date_revenue / target;
            if ratio >= 1.0 {
                "ahead".to_string()
            } else if ratio >= 0.9 {
                "on_track".to_string()
            } else {
                "behind".to_string()
            }
        }
        _ => "no_target".to_string(),
    };

    let month_to_date_pacing_ratio = monthly_revenue_target
        .filter(|target| *target > 0.0)
        .map(|target| finite_or_zero(month_to_date_revenue / target))
        .unwrap_or(1.0);

    let confidence_interval_low = finite_or_zero(expected_revenue_next_period * 0.9);
    let confidence_interval_high = finite_or_zero(expected_revenue_next_period * 1.1);
    let (confidence_interval_low, confidence_interval_high) =
        if confidence_interval_low <= confidence_interval_high {
            (confidence_interval_low, confidence_interval_high)
        } else {
            (confidence_interval_high, confidence_interval_low)
        };

    ForecastSummaryV1 {
        expected_revenue_next_period,
        expected_roas_next_period: avg_roas,
        confidence_interval_low,
        confidence_interval_high,
        month_to_date_pacing_ratio,
        month_to_date_revenue,
        monthly_revenue_target,
        target_roas: options.target_roas,
        pacing_status,
    }
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn average_order_value(metrics: &ReportMetrics) -> f64 {
    if metrics.conversions > 0.0 {
        metrics.conversions_value / metrics.conversions
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::marketing_data_analysis::contracts::{
        AnalyticsRunMetadataV1, AnalyticsValidationReportV1, MockAnalyticsArtifactV1,
        MockAnalyticsRequestV1, MOCK_ANALYTICS_SCHEMA_VERSION_V1,
    };
    use once_cell::sync::Lazy;
    use proptest::prelude::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn with_temp_env<F>(pairs: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        let previous = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        f();

        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(&key, value),
                None => std::env::remove_var(&key),
            }
        }
    }

    fn build_run(run_id: &str, profile_id: &str, spend: f64, roas: f64) -> PersistedAnalyticsRunV1 {
        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: MockAnalyticsRequestV1 {
                start_date: "2026-02-01".to_string(),
                end_date: "2026-02-07".to_string(),
                campaign_filter: None,
                ad_group_filter: None,
                seed: Some(1),
                profile_id: profile_id.to_string(),
                include_narratives: true,
                source_window_observations: Vec::new(),
                budget_envelope:
                    crate::subsystems::marketing_data_analysis::contracts::BudgetEnvelopeV1::default(
                    ),
            },
            metadata: AnalyticsRunMetadataV1 {
                run_id: run_id.to_string(),
                connector_id: "mock".to_string(),
                profile_id: profile_id.to_string(),
                seed: 1,
                schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
                date_span_days: 7,
                requested_at_utc: None,
                connector_attestation: Default::default(),
            },
            report: Default::default(),
            daily_revenue_series: Vec::new(),
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["sim".to_string()],
            provenance: Vec::new(),
            source_coverage: Vec::new(),
            ga4_session_rollups: Vec::new(),
            ingest_cleaning_notes: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
            freshness_policy: Default::default(),
            reconciliation_policy: Default::default(),
            budget: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };
        artifact.report.total_metrics.cost = spend;
        artifact.report.total_metrics.roas = roas;
        artifact.report.total_metrics.conversions_value = spend * roas;
        artifact.report.total_metrics.impressions = 1000;
        artifact.report.total_metrics.clicks = 100;
        artifact.report.total_metrics.conversions = 8.0;
        artifact.report.date_range = "2026-02-01 to 2026-02-07".to_string();
        PersistedAnalyticsRunV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: artifact.request.clone(),
            metadata: artifact.metadata.clone(),
            validation: artifact.validation.clone(),
            artifact,
            stored_at_utc: "2026-02-17T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn builds_snapshot_from_history() {
        let mut current = build_run("run-2", "p1", 200.0, 6.5);
        current.artifact.ga4_session_rollups = vec![
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: "user-2:202".to_string(),
                user_pseudo_id: "user-2".to_string(),
                ga_session_id: Some(202),
                session_start_ts_utc: "2026-02-07T10:00:00Z".to_string(),
                first_event_ts_utc: "2026-02-07T10:00:00Z".to_string(),
                landing_path: Some("/simply-raw-freeze-dried-raw-meals".to_string()),
                landing_host: Some("www.naturesdietpet.com".to_string()),
                landing_context: Some(
                    crate::subsystems::marketing_data_analysis::contracts::LandingContextV1 {
                        taxonomy_version: "nd_landing_taxonomy.v2".to_string(),
                        matched_rule_id: "offer.simply_raw".to_string(),
                        landing_path: "/simply-raw-freeze-dried-raw-meals".to_string(),
                        landing_family: "simply_raw_offer_lp".to_string(),
                        landing_page_group: "offer_landing".to_string(),
                    },
                ),
                experiment_context: Default::default(),
                visitor_type:
                    crate::subsystems::marketing_data_analysis::contracts::VisitorTypeV1::New,
                engaged_session: true,
                engagement_time_msec: 1_000,
                country: Some("US".to_string()),
                platform: Some("WEB".to_string()),
                device_category: Some("mobile".to_string()),
                source: Some("google".to_string()),
                medium: Some("cpc".to_string()),
                source_medium: Some("google / cpc".to_string()),
                campaign: Some("Brand Search".to_string()),
                page_view_count: 2,
                user_engagement_count: 1,
                scroll_count: 1,
                view_item_count: 1,
                add_to_cart_count: 1,
                begin_checkout_count: 1,
                purchase_count: 1,
                revenue_usd: 48.5,
                transaction_ids: vec!["tx-run-2".to_string()],
            },
        ];
        let mut previous = build_run("run-1", "p1", 180.0, 5.8);
        previous.artifact.ga4_session_rollups = vec![
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: "user-1:101".to_string(),
                user_pseudo_id: "user-1".to_string(),
                ga_session_id: Some(101),
                session_start_ts_utc: "2026-02-01T09:00:00Z".to_string(),
                first_event_ts_utc: "2026-02-01T09:00:00Z".to_string(),
                landing_path: Some("/".to_string()),
                landing_host: Some("www.naturesdietpet.com".to_string()),
                landing_context: Some(
                    crate::subsystems::marketing_data_analysis::contracts::LandingContextV1 {
                        taxonomy_version: "nd_landing_taxonomy.v2".to_string(),
                        matched_rule_id: "home.root".to_string(),
                        landing_path: "/".to_string(),
                        landing_family: "home".to_string(),
                        landing_page_group: "home".to_string(),
                    },
                ),
                experiment_context: Default::default(),
                visitor_type:
                    crate::subsystems::marketing_data_analysis::contracts::VisitorTypeV1::Returning,
                engaged_session: true,
                engagement_time_msec: 800,
                country: Some("US".to_string()),
                platform: Some("WEB".to_string()),
                device_category: Some("desktop".to_string()),
                source: Some("google".to_string()),
                medium: Some("organic".to_string()),
                source_medium: Some("google / organic".to_string()),
                campaign: None,
                page_view_count: 2,
                user_engagement_count: 1,
                scroll_count: 1,
                view_item_count: 1,
                add_to_cart_count: 0,
                begin_checkout_count: 0,
                purchase_count: 0,
                revenue_usd: 0.0,
                transaction_ids: Vec::new(),
            },
        ];
        let runs = vec![current, previous];
        let snap = build_executive_dashboard_snapshot("p1", &runs, SnapshotBuildOptions::default())
            .expect("snapshot");
        assert_eq!(snap.profile_id, "p1");
        assert!(!snap.kpis.is_empty());
        assert!(!snap.channel_mix_series.is_empty());
        assert!(!snap.decision_feed.is_empty());
        assert!(snap.publish_export_gate.gate_status == "ready");
        assert!(snap
            .high_leverage_reports
            .revenue_truth
            .summary
            .contains("Canonical purchase metrics enforced"));
        assert!(!snap.high_leverage_reports.funnel_survival.points.is_empty());
        assert_eq!(
            snap.high_leverage_reports
                .data_quality_scorecard
                .gate_status,
            "ready"
        );
    }

    #[test]
    fn snapshot_carries_daily_revenue_series() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.daily_revenue_series = vec![
            crate::subsystems::marketing_data_analysis::contracts::DailyRevenuePointV1 {
                date: "2026-02-01".to_string(),
                revenue: 120.5,
                conversions: 2.0,
                source_system: "ga4".to_string(),
            },
            crate::subsystems::marketing_data_analysis::contracts::DailyRevenuePointV1 {
                date: "2026-02-02".to_string(),
                revenue: 130.5,
                conversions: 2.0,
                source_system: "ga4".to_string(),
            },
        ];
        let snap =
            build_executive_dashboard_snapshot("p1", &[run], SnapshotBuildOptions::default())
                .expect("snapshot");
        assert_eq!(snap.daily_revenue_series.len(), 2);
        assert_eq!(snap.daily_revenue_series[0].date, "2026-02-01");
        assert!((snap.daily_revenue_series[1].revenue - 130.5).abs() < 0.0001);
    }

    #[test]
    fn attribution_delta_report_orders_by_abs_delta() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.report.campaign_data = vec![
            crate::data_models::analytics::CampaignReportRow {
                date: "2026-02-01".to_string(),
                campaign_id: "c1".to_string(),
                campaign_name: "Brand Search".to_string(),
                campaign_status: "ENABLED".to_string(),
                metrics: crate::data_models::analytics::ReportMetrics {
                    impressions: 2000,
                    clicks: 150,
                    cost: 120.0,
                    conversions: 9.0,
                    conversions_value: 600.0,
                    ctr: 7.5,
                    cpc: 0.8,
                    cpa: 13.3333,
                    roas: 5.0,
                },
            },
            crate::data_models::analytics::CampaignReportRow {
                date: "2026-02-01".to_string(),
                campaign_id: "c2".to_string(),
                campaign_name: "Prospecting".to_string(),
                campaign_status: "ENABLED".to_string(),
                metrics: crate::data_models::analytics::ReportMetrics {
                    impressions: 8000,
                    clicks: 250,
                    cost: 180.0,
                    conversions: 4.0,
                    conversions_value: 120.0,
                    ctr: 3.125,
                    cpc: 0.72,
                    cpa: 45.0,
                    roas: 0.6667,
                },
            },
        ];
        let snap = build_executive_dashboard_snapshot(
            "p1",
            &[run.clone()],
            SnapshotBuildOptions::default(),
        )
        .expect("snapshot");
        let report = &snap.high_leverage_reports.attribution_delta;
        assert_eq!(report.rows.len(), 2);
        assert!(report.last_touch_concentration_hhi > 0.0);
        assert!(report.dominant_last_touch_campaign.is_some());
        let sum_last_touch = report
            .rows
            .iter()
            .map(|row| row.last_touch_share)
            .sum::<f64>();
        assert!((sum_last_touch - 1.0).abs() < 0.0002);
        if report.rows.len() > 1 {
            assert!(
                report.rows[0].delta_first_vs_last.abs()
                    >= report.rows[1].delta_first_vs_last.abs()
            );
        }
    }

    #[test]
    fn revenue_truth_report_carries_custom_purchase_guard_metrics() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.extend([
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_overlap_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase overlap ratio <= 0.20".to_string(),
            },
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_orphan_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase orphan ratio <= 0.05".to_string(),
            },
        ]);
        let report = build_revenue_truth_report(&run);
        assert_eq!(report.custom_purchase_rows, 6);
        assert_eq!(report.custom_purchase_overlap_rows, 5);
        assert_eq!(report.custom_purchase_orphan_rows, 1);
        assert!((report.custom_purchase_overlap_ratio - 0.8333).abs() < 0.0001);
        assert!((report.custom_purchase_orphan_ratio - 0.1667).abs() < 0.0001);
        assert_eq!(report.truth_guard_status, "guarded_review_required");
    }

    #[test]
    fn publish_gate_blocks_on_high_severity_schema_failure() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.push(
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "schema_required".to_string(),
                passed: false,
                severity: "high".to_string(),
                observed: "missing metrics.clicks".to_string(),
                expected: "metrics.clicks present".to_string(),
            },
        );
        let gate = build_publish_export_gate(&run);
        assert!(!gate.publish_ready);
        assert!(!gate.export_ready);
        assert_eq!(gate.gate_status, "blocked");
        assert!(!gate.blocking_reasons.is_empty());
    }

    #[test]
    fn publish_gate_blocks_when_budget_event_blocked() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.budget.events.push(
            crate::subsystems::marketing_data_analysis::contracts::BudgetEventV1 {
                subsystem: "mock_analytics.fetch".to_string(),
                category: "retrieval".to_string(),
                attempted_units: 9999,
                remaining_units_before: 10,
                outcome: "blocked".to_string(),
                message: "budget cap exceeded".to_string(),
            },
        );
        let gate = build_publish_export_gate(&run);
        assert!(!gate.publish_ready);
        assert_eq!(gate.gate_status, "blocked");
        assert!(gate
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("Budget spend exceeded")));
    }

    #[test]
    fn storefront_summary_fails_closed_without_session_rollups() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.source_coverage = vec![
            crate::subsystems::marketing_data_analysis::contracts::SourceCoverageV1 {
                source_system: "ga4".to_string(),
                enabled: true,
                observed: true,
                row_count: 42,
                unavailable_reason: None,
            },
            crate::subsystems::marketing_data_analysis::contracts::SourceCoverageV1 {
                source_system: "wix_storefront".to_string(),
                enabled: false,
                observed: false,
                row_count: 0,
                unavailable_reason: Some("disabled_by_ga4_unified_topology".to_string()),
            },
        ];
        let summary = build_storefront_summary(&run);
        assert_eq!(summary.source_system, "ga4_session_rollup_unavailable");
        assert_eq!(summary.identity_confidence, "low");
        assert!(summary.rows.is_empty());
    }

    #[test]
    fn storefront_summary_uses_observed_session_rollups_when_available() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.ga4_session_rollups = vec![
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: "user-1:101".to_string(),
                user_pseudo_id: "user-1".to_string(),
                ga_session_id: Some(101),
                session_start_ts_utc: "2026-02-01T10:00:00Z".to_string(),
                first_event_ts_utc: "2026-02-01T10:00:00Z".to_string(),
                landing_path: Some("/simply-raw-freeze-dried-raw-meals".to_string()),
                landing_host: Some("www.naturesdietpet.com".to_string()),
                landing_context: Some(
                    crate::subsystems::marketing_data_analysis::contracts::LandingContextV1 {
                        taxonomy_version: "nd_landing_taxonomy.v2".to_string(),
                        matched_rule_id: "offer.simply_raw".to_string(),
                        landing_path: "/simply-raw-freeze-dried-raw-meals".to_string(),
                        landing_family: "simply_raw_offer_lp".to_string(),
                        landing_page_group: "offer_landing".to_string(),
                    },
                ),
                experiment_context: Default::default(),
                visitor_type:
                    crate::subsystems::marketing_data_analysis::contracts::VisitorTypeV1::New,
                engaged_session: true,
                engagement_time_msec: 1_200,
                country: Some("US".to_string()),
                platform: Some("WEB".to_string()),
                device_category: Some("mobile".to_string()),
                source: Some("google".to_string()),
                medium: Some("cpc".to_string()),
                source_medium: Some("google / cpc".to_string()),
                campaign: Some("Brand Search".to_string()),
                page_view_count: 2,
                user_engagement_count: 1,
                scroll_count: 1,
                view_item_count: 1,
                add_to_cart_count: 1,
                begin_checkout_count: 1,
                purchase_count: 1,
                revenue_usd: 48.5,
                transaction_ids: vec!["tx-1".to_string()],
            },
        ];

        let summary = build_storefront_summary(&run);
        assert_eq!(summary.source_system, "ga4_session_rollups_observed");
        assert_eq!(summary.identity_confidence, "high");
        assert_eq!(summary.rows.len(), 1);
        let row = &summary.rows[0];
        assert_eq!(
            row.landing_path.as_deref(),
            Some("/simply-raw-freeze-dried-raw-meals")
        );
        assert_eq!(row.landing_family.as_deref(), Some("simply_raw_offer_lp"));
        assert!((row.engaged_rate - 1.0).abs() < 0.0001);
        assert!((row.add_to_cart_rate - 1.0).abs() < 0.0001);
        assert!((row.checkout_rate - 1.0).abs() < 0.0001);
        assert!((row.purchase_rate - 1.0).abs() < 0.0001);
        assert!((row.revenue_per_session - 48.5).abs() < 0.0001);
    }

    #[test]
    fn publish_gate_warns_when_cross_source_not_applicable() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.data_quality.cross_source_applicability_ratio = 0.0;
        let gate = build_publish_export_gate(&run);
        assert!(gate
            .warning_reasons
            .iter()
            .any(|reason| { reason.contains("Cross-source reconciliation is not applicable") }));
    }

    #[test]
    fn publish_gate_warns_when_custom_purchase_schema_integrity_fails() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.push(
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_schema_integrity".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=10, with_transaction_id=0, with_value=0".to_string(),
                expected: "all purchase_ndp rows include transaction_id and value".to_string(),
            },
        );
        let gate = build_publish_export_gate(&run);
        assert!(gate.publish_ready);
        assert_eq!(gate.gate_status, "review_required");
        assert!(gate
            .warning_reasons
            .iter()
            .any(|reason| reason.contains("purchase_ndp")));
    }

    #[test]
    fn publish_gate_warns_when_custom_purchase_overlap_and_orphans_fail() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.extend([
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_overlap_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase overlap ratio <= 0.20".to_string(),
            },
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_orphan_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase orphan ratio <= 0.05".to_string(),
            },
        ]);
        let gate = build_publish_export_gate(&run);
        assert!(gate.publish_ready);
        assert_eq!(gate.gate_status, "review_required");
        assert!(gate
            .warning_reasons
            .iter()
            .any(|reason| reason.contains("overlaps canonical")));
        assert!(gate
            .warning_reasons
            .iter()
            .any(|reason| reason.contains("orphan rows")));
    }

    #[test]
    fn decision_feed_surfaces_custom_purchase_duplicate_and_orphan_cards() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.extend([
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_overlap_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase overlap ratio <= 0.20".to_string(),
            },
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
                applicability:
                    crate::subsystems::marketing_data_analysis::contracts::QualityCheckApplicabilityV1::Applies,
                code: "ga4_custom_purchase_ndp_orphan_rate".to_string(),
                passed: false,
                severity: "medium".to_string(),
                observed: "purchase_ndp_rows=6, rows_with_canonical_purchase=5, orphan_rows=1, overlap_ratio=0.8333, orphan_ratio=0.1667".to_string(),
                expected: "custom purchase orphan ratio <= 0.05".to_string(),
            },
        ]);
        let cards = build_decision_feed(&run);
        assert!(cards
            .iter()
            .any(|card| card.card_id == "custom-purchase-overlap"));
        assert!(cards
            .iter()
            .any(|card| card.card_id == "custom-purchase-orphans"));
        assert!(!cards
            .iter()
            .any(|card| card.card_id == "schema-drift" && card.status == "blocked"));
    }

    #[test]
    fn decision_feed_surfaces_experiment_assignment_gap() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.ga4_session_rollups = vec![
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: "assigned-1".to_string(),
                experiment_context: crate::subsystems::marketing_data_analysis::contracts::SessionExperimentContextV1 {
                    experiment_id: Some("exp-a".to_string()),
                    variant_id: Some("control".to_string()),
                    assignment_source: Some(
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentSourceV1::Ga4EventParam,
                    ),
                    assignment_confidence:
                        crate::subsystems::marketing_data_analysis::contracts::AssignmentConfidenceV1::High,
                    assignment_status:
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentStatusV1::Assigned,
                    assignment_observed_at_utc: Some("2026-02-01T10:00:00Z".to_string()),
                    assignment_notes: Vec::new(),
                    ..Default::default()
                },
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                ..Default::default()
            },
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: "partial-1".to_string(),
                experiment_context: crate::subsystems::marketing_data_analysis::contracts::SessionExperimentContextV1 {
                    experiment_id: Some("exp-a".to_string()),
                    assignment_source: Some(
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentSourceV1::UrlQuery,
                    ),
                    assignment_confidence:
                        crate::subsystems::marketing_data_analysis::contracts::AssignmentConfidenceV1::Low,
                    assignment_status:
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentStatusV1::Partial,
                    assignment_observed_at_utc: Some("2026-02-01T10:01:00Z".to_string()),
                    assignment_notes: Vec::new(),
                    ..Default::default()
                },
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                ..Default::default()
            },
        ];

        let cards = build_decision_feed(&run);
        assert!(cards
            .iter()
            .any(|card| card.card_id == "experiment-assignment-incomplete"));
    }

    #[test]
    fn experiment_governance_report_couples_claim_with_readiness_card() {
        let mut run = build_run("run-exp", "p1", 200.0, 6.5);
        let control_sessions = (0..120).map(|index| {
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: format!("control-{index}"),
                user_pseudo_id: format!("user-control-{index}"),
                ga_session_id: Some(1_000 + index),
                session_start_ts_utc: "2026-02-01T10:00:00Z".to_string(),
                first_event_ts_utc: "2026-02-01T10:00:00Z".to_string(),
                landing_path: Some("/simply-raw-freeze-dried-raw-meals".to_string()),
                landing_host: Some("www.naturesdietpet.com".to_string()),
                landing_context: Some(
                    crate::subsystems::marketing_data_analysis::contracts::LandingContextV1 {
                        taxonomy_version: "nd_landing_taxonomy.v2".to_string(),
                        matched_rule_id: "offer.simply_raw".to_string(),
                        landing_path: "/simply-raw-freeze-dried-raw-meals".to_string(),
                        landing_family: "simply_raw_offer_lp".to_string(),
                        landing_page_group: "offer_landing".to_string(),
                    },
                ),
                experiment_context: crate::subsystems::marketing_data_analysis::contracts::SessionExperimentContextV1 {
                    experiment_id: Some("lp_paid_offer_test".to_string()),
                    experiment_name: Some("Landing LP Test".to_string()),
                    variant_id: Some("control".to_string()),
                    variant_name: Some("Simply Raw".to_string()),
                    assignment_source: Some(
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentSourceV1::Ga4EventParam,
                    ),
                    assignment_confidence:
                        crate::subsystems::marketing_data_analysis::contracts::AssignmentConfidenceV1::High,
                    assignment_status:
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentStatusV1::Assigned,
                    assignment_observed_at_utc: Some("2026-02-01T10:00:00Z".to_string()),
                    assignment_notes: Vec::new(),
                },
                country: Some("US".to_string()),
                platform: Some("WEB".to_string()),
                device_category: Some(if index % 2 == 0 {
                    "mobile".to_string()
                } else {
                    "desktop".to_string()
                }),
                source: Some("google".to_string()),
                medium: Some("cpc".to_string()),
                source_medium: Some("google / cpc".to_string()),
                campaign: Some("Landing LP Test".to_string()),
                page_view_count: 2,
                user_engagement_count: 1,
                scroll_count: 1,
                view_item_count: 1,
                add_to_cart_count: u32::from(index < 40),
                begin_checkout_count: u32::from(index < 25),
                purchase_count: u32::from(index < 16),
                revenue_usd: if index < 16 { 48.5 } else { 0.0 },
                transaction_ids: if index < 16 {
                    vec![format!("tx-control-{index}")]
                } else {
                    Vec::new()
                },
                engaged_session: true,
                engagement_time_msec: 1_200,
                visitor_type:
                    crate::subsystems::marketing_data_analysis::contracts::VisitorTypeV1::New,
            }
        });
        let challenger_sessions = (0..120).map(|index| {
            crate::subsystems::marketing_data_analysis::contracts::Ga4SessionRollupV1 {
                session_key: format!("challenger-{index}"),
                user_pseudo_id: format!("user-challenger-{index}"),
                ga_session_id: Some(2_000 + index),
                session_start_ts_utc: "2026-02-01T11:00:00Z".to_string(),
                first_event_ts_utc: "2026-02-01T11:00:00Z".to_string(),
                landing_path: Some("/simply-raw-value-bundle-assortment".to_string()),
                landing_host: Some("www.naturesdietpet.com".to_string()),
                landing_context: Some(
                    crate::subsystems::marketing_data_analysis::contracts::LandingContextV1 {
                        taxonomy_version: "nd_landing_taxonomy.v2".to_string(),
                        matched_rule_id: "offer.bundle".to_string(),
                        landing_path: "/simply-raw-value-bundle-assortment".to_string(),
                        landing_family: "bundle_offer_lp".to_string(),
                        landing_page_group: "offer_landing".to_string(),
                    },
                ),
                experiment_context: crate::subsystems::marketing_data_analysis::contracts::SessionExperimentContextV1 {
                    experiment_id: Some("lp_paid_offer_test".to_string()),
                    experiment_name: Some("Landing LP Test".to_string()),
                    variant_id: Some("challenger_bundle".to_string()),
                    variant_name: Some("Bundle".to_string()),
                    assignment_source: Some(
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentSourceV1::Ga4EventParam,
                    ),
                    assignment_confidence:
                        crate::subsystems::marketing_data_analysis::contracts::AssignmentConfidenceV1::High,
                    assignment_status:
                        crate::subsystems::marketing_data_analysis::contracts::ExperimentAssignmentStatusV1::Assigned,
                    assignment_observed_at_utc: Some("2026-02-01T11:00:00Z".to_string()),
                    assignment_notes: Vec::new(),
                },
                country: Some("US".to_string()),
                platform: Some("WEB".to_string()),
                device_category: Some(if index % 2 == 0 {
                    "mobile".to_string()
                } else {
                    "desktop".to_string()
                }),
                source: Some("google".to_string()),
                medium: Some("cpc".to_string()),
                source_medium: Some("google / cpc".to_string()),
                campaign: Some("Landing LP Test".to_string()),
                page_view_count: 2,
                user_engagement_count: 1,
                scroll_count: 1,
                view_item_count: 1,
                add_to_cart_count: u32::from(index < 50),
                begin_checkout_count: u32::from(index < 30),
                purchase_count: u32::from(index < 18),
                revenue_usd: if index < 18 { 55.0 } else { 0.0 },
                transaction_ids: if index < 18 {
                    vec![format!("tx-challenger-{index}")]
                } else {
                    Vec::new()
                },
                engaged_session: true,
                engagement_time_msec: 1_400,
                visitor_type:
                    crate::subsystems::marketing_data_analysis::contracts::VisitorTypeV1::New,
            }
        });
        run.artifact.ga4_session_rollups = control_sessions.chain(challenger_sessions).collect();

        let report =
            build_experiment_governance_report(&run, &build_experiment_analytics_report(&run));
        assert_eq!(
            report.coverage_scope,
            "experiment_id_scoped_observed_sessions"
        );
        assert_eq!(report.items.len(), 1);
        let item = &report.items[0];
        assert_eq!(item.experiment_id, "lp_paid_offer_test");
        assert_eq!(
            item.readiness.readiness_state,
            crate::subsystems::marketing_data_analysis::contracts::InsightPermissionStateV1::DirectionalOnly
        );
        assert_eq!(item.readiness.permission_level, "directional_only");
        assert_eq!(
            item.readiness.control_variant_id.as_deref(),
            Some("control")
        );
        assert_eq!(
            item.readiness.challenger_variant_id.as_deref(),
            Some("challenger_bundle")
        );
        assert_eq!(
            item.readiness.denominator_scope.as_deref(),
            Some("assigned_sessions_only")
        );
        assert!(item
            .readiness
            .blocking_reasons
            .contains(&"causal_design_not_approved".to_string()));
    }

    #[test]
    fn publish_gate_blocks_in_production_when_signature_missing() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(
            &[
                ("REQUIRE_SIGNED_ATTESTATIONS", None),
                ("ATTESTATION_KEY_REGISTRY_JSON", None),
                ("ATTESTATION_KEY_REGISTRY_PATH", None),
            ],
            || {
                let run = build_run("run-prod", "production", 200.0, 6.5);
                let gate = build_publish_export_gate(&run);
                assert!(!gate.publish_ready);
                assert!(gate
                    .blocking_reasons
                    .iter()
                    .any(|reason| reason.contains("Signed attestation required by policy")));
            },
        );
    }

    #[test]
    fn publish_gate_blocks_when_signature_present_but_registry_missing() {
        let _guard = ENV_MUTEX.lock().expect("env mutex");
        with_temp_env(
            &[
                ("REQUIRE_SIGNED_ATTESTATIONS", Some("true")),
                ("ATTESTATION_KEY_REGISTRY_JSON", None),
                ("ATTESTATION_KEY_REGISTRY_PATH", None),
            ],
            || {
                let mut run = build_run("run-prod", "production", 200.0, 6.5);
                run.metadata.connector_attestation.fingerprint_signature =
                    Some("ed25519:abc".to_string());
                run.metadata.connector_attestation.fingerprint_key_id = Some("k1".to_string());
                let gate = build_publish_export_gate(&run);
                assert!(!gate.publish_ready);
                assert!(gate
                    .blocking_reasons
                    .iter()
                    .any(|reason| reason.contains("key registry is not configured")));
            },
        );
    }

    proptest! {
        #[test]
        fn forecast_confidence_and_pacing_invariants_hold(
            spend in 0.0f64..100_000.0,
            roas in 0.0f64..30.0,
            monthly_target in 1.0f64..500_000.0
        ) {
            let run = build_run("run-prop", "p1", spend, roas);
            let options = SnapshotBuildOptions {
                compare_window_runs: 1,
                target_roas: Some((roas / 2.0).max(0.1)),
                monthly_revenue_target: Some(monthly_target),
            };
            let forecast = build_forecast(&run.artifact.report.total_metrics, std::slice::from_ref(&run), options);
            prop_assert!(forecast.confidence_interval_low <= forecast.expected_revenue_next_period);
            prop_assert!(forecast.expected_revenue_next_period <= forecast.confidence_interval_high);
            prop_assert!(forecast.month_to_date_pacing_ratio.is_finite());
            prop_assert!(forecast.month_to_date_pacing_ratio >= 0.0);
        }
    }
}
