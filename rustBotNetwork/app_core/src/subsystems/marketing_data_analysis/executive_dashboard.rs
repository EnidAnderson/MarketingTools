// provenance: decision_id=DEC-0015; change_request_id=CR-QA_FIXER-0032
use super::contracts::{
    ChannelMixPointV1, DataQualitySummaryV1, DecisionFeedCardV1, ExecutiveDashboardSnapshotV1,
    ForecastSummaryV1, FunnelStageV1, FunnelSummaryV1, KpiTileV1, PersistedAnalyticsRunV1,
    PortfolioRowV1, PublishExportGateV1, StorefrontBehaviorRowV1, StorefrontBehaviorSummaryV1,
};
use crate::data_models::analytics::ReportMetrics;
use chrono::Utc;

const SNAPSHOT_SCHEMA_VERSION_V1: &str = "executive_dashboard_snapshot.v1";
const DEFAULT_COMPARE_WINDOW_RUNS: usize = 1;

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
    if !latest.artifact.quality_controls.is_healthy {
        alerts.push("Quality controls degraded".to_string());
    }
    for flag in &latest.artifact.historical_analysis.anomaly_flags {
        alerts.push(format!("Anomaly {}: {}", flag.metric_key, flag.reason));
    }

    let trust_status = if latest.artifact.quality_controls.is_healthy {
        "healthy".to_string()
    } else {
        "degraded".to_string()
    };

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
        roas_target_band: options.target_roas,
        funnel_summary: build_funnel_summary(latest_metrics),
        storefront_behavior_summary: build_storefront_summary(latest_metrics),
        portfolio_rows: build_portfolio_rows(latest),
        forecast_summary: build_forecast(latest_metrics, runs, options),
        data_quality: latest.artifact.data_quality.clone(),
        budget: latest.artifact.budget.clone(),
        decision_feed: build_decision_feed(latest),
        publish_export_gate: build_publish_export_gate(latest),
        quality_controls: latest.artifact.quality_controls.clone(),
        historical_analysis: latest.artifact.historical_analysis.clone(),
        operator_summary: latest.artifact.operator_summary.clone(),
        trust_status,
        alerts,
    })
}

fn build_decision_feed(run: &PersistedAnalyticsRunV1) -> Vec<DecisionFeedCardV1> {
    let mut cards = Vec::new();
    let quality = &run.artifact.quality_controls;
    let historical = &run.artifact.historical_analysis;

    let failed_schema = quality
        .schema_drift_checks
        .iter()
        .filter(|check| !check.passed)
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
    validate_data_quality_bounds(data_quality);
    let mut blocking_reasons = Vec::new();
    let mut warning_reasons = Vec::new();

    if !quality
        .schema_drift_checks
        .iter()
        .all(|check| check.passed || check.severity != "high")
    {
        blocking_reasons.push("High-severity schema-drift failure present.".to_string());
    }

    if !quality
        .freshness_sla_checks
        .iter()
        .all(|check| check.passed || check.severity != "high")
    {
        blocking_reasons.push("High-severity freshness SLA failure present.".to_string());
    }

    if !quality
        .identity_resolution_checks
        .iter()
        .all(|check| check.passed || check.severity != "high")
    {
        blocking_reasons.push("High-severity identity-resolution failure present.".to_string());
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
    if data_quality.budget_pass_ratio < 1.0 {
        blocking_reasons.push("Budget checks not fully passing.".to_string());
    }
    if run
        .artifact
        .quality_controls
        .budget_checks
        .iter()
        .any(|check| !check.passed && check.severity.eq_ignore_ascii_case("high"))
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

    let publish_ready = blocking_reasons.is_empty();
    let export_ready = blocking_reasons.is_empty();
    let gate_status = if !publish_ready {
        "blocked".to_string()
    } else if warning_reasons.is_empty() {
        "ready".to_string()
    } else {
        "review_required".to_string()
    };

    assert!(
        !(publish_ready && !blocking_reasons.is_empty()),
        "publish cannot be ready when blocking reasons exist"
    );
    assert!(
        !(export_ready && !blocking_reasons.is_empty()),
        "export cannot be ready when blocking reasons exist"
    );

    PublishExportGateV1 {
        publish_ready,
        export_ready,
        blocking_reasons,
        warning_reasons,
        gate_status,
    }
}

fn validate_data_quality_bounds(data_quality: &DataQualitySummaryV1) {
    assert!((0.0..=1.0).contains(&data_quality.completeness_ratio));
    assert!((0.0..=1.0).contains(&data_quality.identity_join_coverage_ratio));
    assert!((0.0..=1.0).contains(&data_quality.freshness_pass_ratio));
    assert!((0.0..=1.0).contains(&data_quality.reconciliation_pass_ratio));
    assert!((0.0..=1.0).contains(&data_quality.budget_pass_ratio));
    assert!((0.0..=1.0).contains(&data_quality.quality_score));
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

fn build_funnel_summary(metrics: &ReportMetrics) -> FunnelSummaryV1 {
    let stage_impressions = metrics.impressions as f64;
    let stage_clicks = metrics.clicks as f64;
    let stage_sessions = stage_clicks * 0.92;
    let stage_product_view = stage_sessions * 0.67;
    let stage_add_to_cart = stage_product_view * 0.28;
    let stage_checkout = stage_add_to_cart * 0.57;
    let stage_purchase = metrics.conversions.max(0.0);

    let stages = vec![
        stage("Impression", stage_impressions, None),
        stage(
            "Click",
            stage_clicks,
            Some(stage_clicks / stage_impressions.max(1.0)),
        ),
        stage(
            "Session",
            stage_sessions,
            Some(stage_sessions / stage_clicks.max(1.0)),
        ),
        stage(
            "Product View",
            stage_product_view,
            Some(stage_product_view / stage_sessions.max(1.0)),
        ),
        stage(
            "Add To Cart",
            stage_add_to_cart,
            Some(stage_add_to_cart / stage_product_view.max(1.0)),
        ),
        stage(
            "Checkout",
            stage_checkout,
            Some(stage_checkout / stage_add_to_cart.max(1.0)),
        ),
        stage(
            "Purchase",
            stage_purchase,
            Some(stage_purchase / stage_checkout.max(1.0)),
        ),
    ];

    let mut hotspot = "None".to_string();
    let mut min_rate = 1.0;
    for item in stages.iter().skip(1) {
        if let Some(rate) = item.conversion_from_previous {
            if rate < min_rate {
                min_rate = rate;
                hotspot = item.stage.clone();
            }
        }
    }

    FunnelSummaryV1 {
        stages,
        dropoff_hotspot_stage: hotspot,
    }
}

fn stage(name: &str, value: f64, conversion_from_previous: Option<f64>) -> FunnelStageV1 {
    FunnelStageV1 {
        stage: name.to_string(),
        value,
        conversion_from_previous,
    }
}

fn build_storefront_summary(metrics: &ReportMetrics) -> StorefrontBehaviorSummaryV1 {
    let sessions = (metrics.clicks as f64 * 0.92).round() as u64;
    let add_to_cart_rate = 0.18;
    let purchase_rate = if sessions > 0 {
        metrics.conversions / sessions as f64
    } else {
        0.0
    };
    let aov = average_order_value(metrics);

    StorefrontBehaviorSummaryV1 {
        source_system: "wix_storefront_mock".to_string(),
        identity_confidence: "medium".to_string(),
        rows: vec![
            StorefrontBehaviorRowV1 {
                segment: "mobile".to_string(),
                product_or_template: "ready-raw-hero-landing".to_string(),
                sessions: (sessions as f64 * 0.58).round() as u64,
                add_to_cart_rate: add_to_cart_rate + 0.02,
                purchase_rate: purchase_rate + 0.01,
                aov: aov * 0.97,
            },
            StorefrontBehaviorRowV1 {
                segment: "desktop".to_string(),
                product_or_template: "value-bundle-collection".to_string(),
                sessions: (sessions as f64 * 0.42).round() as u64,
                add_to_cart_rate: add_to_cart_rate - 0.01,
                purchase_rate: purchase_rate + 0.015,
                aov: aov * 1.06,
            },
        ],
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

    let expected_revenue_next_period = metrics.cost * avg_roas;
    let month_to_date_revenue = metrics.conversions_value;
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
        .map(|target| month_to_date_revenue / target)
        .unwrap_or(1.0);

    let confidence_interval_low = expected_revenue_next_period * 0.9;
    let confidence_interval_high = expected_revenue_next_period * 1.1;
    assert!(confidence_interval_low <= expected_revenue_next_period);
    assert!(expected_revenue_next_period <= confidence_interval_high);
    assert!(month_to_date_pacing_ratio.is_finite());

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
    use proptest::prelude::*;

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
            },
            report: Default::default(),
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["sim".to_string()],
            provenance: Vec::new(),
            ingest_cleaning_notes: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
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
        let runs = vec![
            build_run("run-2", "p1", 200.0, 6.5),
            build_run("run-1", "p1", 180.0, 5.8),
        ];
        let snap = build_executive_dashboard_snapshot("p1", &runs, SnapshotBuildOptions::default())
            .expect("snapshot");
        assert_eq!(snap.profile_id, "p1");
        assert!(!snap.kpis.is_empty());
        assert!(!snap.channel_mix_series.is_empty());
        assert!(!snap.decision_feed.is_empty());
        assert!(snap.publish_export_gate.gate_status == "ready");
    }

    #[test]
    fn publish_gate_blocks_on_high_severity_schema_failure() {
        let mut run = build_run("run-2", "p1", 200.0, 6.5);
        run.artifact.quality_controls.schema_drift_checks.push(
            crate::subsystems::marketing_data_analysis::contracts::QualityCheckV1 {
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
