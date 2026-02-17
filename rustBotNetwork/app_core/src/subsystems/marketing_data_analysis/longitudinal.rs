// provenance: decision_id=DEC-0014; change_request_id=CR-QA_FIXER-0031
use super::contracts::{
    AnomalyFlagV1, ConfidenceCalibrationV1, DriftFlagV1, HistoricalAnalysisV1, KpiDeltaV1,
    MockAnalyticsArtifactV1, PersistedAnalyticsRunV1,
};

const DRIFT_Z_SCORE_MEDIUM: f64 = 1.5;
const DRIFT_Z_SCORE_HIGH: f64 = 2.5;
const DELTA_ANOMALY_PCT: f64 = 0.35;

/// # NDOC
/// component: `subsystems::marketing_data_analysis::longitudinal`
/// purpose: Build trend and drift analysis from persisted analytics runs.
/// invariants:
///   - Baseline comparisons are profile-scoped.
///   - Simulated datasets cap confidence at medium unless calibration proves otherwise.
pub fn build_historical_analysis(
    current: &MockAnalyticsArtifactV1,
    history: &[PersistedAnalyticsRunV1],
) -> HistoricalAnalysisV1 {
    if history.is_empty() {
        return HistoricalAnalysisV1 {
            confidence_calibration: ConfidenceCalibrationV1 {
                sample_count: 0,
                recommended_confidence_cap: "medium".to_string(),
                calibration_note: "No baseline history available.".to_string(),
            },
            ..HistoricalAnalysisV1::default()
        };
    }

    let mut baseline_runs = history.to_vec();
    baseline_runs.sort_by(|a, b| b.stored_at_utc.cmp(&a.stored_at_utc));
    let baseline_run_ids: Vec<String> = baseline_runs
        .iter()
        .take(8)
        .map(|r| r.metadata.run_id.clone())
        .collect();

    let most_recent = baseline_runs
        .first()
        .map(|r| &r.artifact.report.total_metrics)
        .cloned();
    let mut deltas = Vec::new();
    if let Some(previous) = most_recent {
        deltas.push(delta(
            "impressions",
            current.report.total_metrics.impressions as f64,
            previous.impressions as f64,
        ));
        deltas.push(delta(
            "clicks",
            current.report.total_metrics.clicks as f64,
            previous.clicks as f64,
        ));
        deltas.push(delta(
            "cost",
            current.report.total_metrics.cost,
            previous.cost,
        ));
        deltas.push(delta(
            "conversions",
            current.report.total_metrics.conversions,
            previous.conversions,
        ));
        deltas.push(delta(
            "roas",
            current.report.total_metrics.roas,
            previous.roas,
        ));
        deltas.push(delta("ctr", current.report.total_metrics.ctr, previous.ctr));
    }

    let all_impressions: Vec<f64> = baseline_runs
        .iter()
        .map(|r| r.artifact.report.total_metrics.impressions as f64)
        .collect();
    let all_clicks: Vec<f64> = baseline_runs
        .iter()
        .map(|r| r.artifact.report.total_metrics.clicks as f64)
        .collect();
    let all_cost: Vec<f64> = baseline_runs
        .iter()
        .map(|r| r.artifact.report.total_metrics.cost)
        .collect();
    let all_conversions: Vec<f64> = baseline_runs
        .iter()
        .map(|r| r.artifact.report.total_metrics.conversions)
        .collect();

    let mut drift_flags = Vec::new();
    let mut anomaly_flags = Vec::new();
    for (metric, series, current_value) in [
        (
            "impressions",
            all_impressions,
            current.report.total_metrics.impressions as f64,
        ),
        (
            "clicks",
            all_clicks,
            current.report.total_metrics.clicks as f64,
        ),
        ("cost", all_cost, current.report.total_metrics.cost),
        (
            "conversions",
            all_conversions,
            current.report.total_metrics.conversions,
        ),
    ] {
        if let Some(drift) = drift_for(metric, &series, current_value) {
            if drift.severity != "low" {
                drift_flags.push(drift.clone());
            }
            if drift.severity == "high" {
                anomaly_flags.push(AnomalyFlagV1 {
                    metric_key: drift.metric_key.clone(),
                    reason: format!(
                        "z-score {:.2} exceeds drift threshold for {}",
                        drift.z_score, drift.metric_key
                    ),
                    severity: "high".to_string(),
                });
            }
        }
    }
    for d in &deltas {
        if let Some(delta_pct) = d.delta_percent {
            if delta_pct.abs() >= DELTA_ANOMALY_PCT {
                anomaly_flags.push(AnomalyFlagV1 {
                    metric_key: d.metric_key.clone(),
                    reason: format!(
                        "period-over-period delta {:.2}% exceeds threshold",
                        delta_pct * 100.0
                    ),
                    severity: "medium".to_string(),
                });
            }
        }
    }

    let sample_count = baseline_runs.len() as u32;
    let recommended_confidence_cap = if sample_count >= 8 && anomaly_flags.is_empty() {
        "medium"
    } else {
        "low"
    };
    let calibration_note = format!(
        "Calibration based on {} historical run(s); anomaly count={}.",
        sample_count,
        anomaly_flags.len()
    );

    HistoricalAnalysisV1 {
        baseline_run_ids,
        period_over_period_deltas: deltas,
        drift_flags,
        anomaly_flags,
        confidence_calibration: ConfidenceCalibrationV1 {
            sample_count,
            recommended_confidence_cap: recommended_confidence_cap.to_string(),
            calibration_note,
        },
    }
}

fn delta(metric_key: &str, current_value: f64, baseline_value: f64) -> KpiDeltaV1 {
    let delta_absolute = current_value - baseline_value;
    let delta_percent = if baseline_value.abs() > f64::EPSILON {
        Some(delta_absolute / baseline_value)
    } else {
        None
    };
    KpiDeltaV1 {
        metric_key: metric_key.to_string(),
        current_value,
        baseline_value,
        delta_absolute,
        delta_percent,
    }
}

fn drift_for(metric_key: &str, baseline: &[f64], current_value: f64) -> Option<DriftFlagV1> {
    if baseline.len() < 2 {
        return None;
    }
    let mean = baseline.iter().sum::<f64>() / baseline.len() as f64;
    let variance = baseline
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f64>()
        / baseline.len() as f64;
    let std_dev = variance.sqrt();
    if std_dev <= f64::EPSILON {
        return Some(DriftFlagV1 {
            metric_key: metric_key.to_string(),
            baseline_mean: mean,
            baseline_std_dev: 0.0,
            current_value,
            z_score: 0.0,
            severity: "low".to_string(),
        });
    }
    let z_score = (current_value - mean) / std_dev;
    let abs_z = z_score.abs();
    let severity = if abs_z >= DRIFT_Z_SCORE_HIGH {
        "high"
    } else if abs_z >= DRIFT_Z_SCORE_MEDIUM {
        "medium"
    } else {
        "low"
    };
    Some(DriftFlagV1 {
        metric_key: metric_key.to_string(),
        baseline_mean: mean,
        baseline_std_dev: std_dev,
        current_value,
        z_score,
        severity: severity.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::marketing_data_analysis::contracts::{
        AnalyticsRunMetadataV1, AnalyticsValidationReportV1, MockAnalyticsRequestV1,
        MOCK_ANALYTICS_SCHEMA_VERSION_V1,
    };

    fn run(
        run_id: &str,
        profile_id: &str,
        impressions: u64,
        clicks: u64,
    ) -> PersistedAnalyticsRunV1 {
        let request = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-03".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(1),
            profile_id: profile_id.to_string(),
            include_narratives: true,
        };
        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: request.clone(),
            metadata: AnalyticsRunMetadataV1 {
                run_id: run_id.to_string(),
                connector_id: "mock".to_string(),
                profile_id: profile_id.to_string(),
                seed: 1,
                schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
                date_span_days: 3,
                requested_at_utc: None,
            },
            report: Default::default(),
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["sim".to_string()],
            provenance: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };
        artifact.report.total_metrics.impressions = impressions;
        artifact.report.total_metrics.clicks = clicks;
        PersistedAnalyticsRunV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request,
            metadata: artifact.metadata.clone(),
            validation: artifact.validation.clone(),
            artifact,
            stored_at_utc: format!("2026-02-{:02}T00:00:00Z", run_id.len()),
        }
    }

    #[test]
    fn emits_period_delta_and_calibration() {
        let current = run("curr", "p1", 500, 45).artifact;
        let history = vec![run("old1", "p1", 300, 30), run("old2", "p1", 320, 35)];
        let out = build_historical_analysis(&current, &history);
        assert!(!out.period_over_period_deltas.is_empty());
        assert_eq!(out.confidence_calibration.sample_count, 2);
    }
}
