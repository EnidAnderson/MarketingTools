use super::contracts::{
    AnalyticsError, AnalyticsValidationReportV1, MockAnalyticsArtifactV1, MockAnalyticsRequestV1,
    ValidationCheck, MOCK_ANALYTICS_SCHEMA_VERSION_V1,
};
use chrono::NaiveDate;
use validator::Validate;

const MAX_DATE_SPAN_DAYS: i64 = 93;
const METRIC_EPSILON: f64 = 0.0001;

/// # NDOC
/// component: `subsystems::marketing_data_analysis::validators`
/// purpose: Validate request shape and date constraints before any connector call.
pub fn validate_mock_analytics_request_v1(
    req: &MockAnalyticsRequestV1,
) -> Result<(NaiveDate, NaiveDate), AnalyticsError> {
    if let Err(err) = req.validate() {
        return Err(AnalyticsError::new(
            "request_contract_violation",
            format!("request contract validation failed: {err}"),
            vec![
                "start_date".to_string(),
                "end_date".to_string(),
                "profile_id".to_string(),
            ],
            None,
        ));
    }

    if req.profile_id.trim().is_empty() {
        return Err(AnalyticsError::validation(
            "invalid_profile_id",
            "profile_id is required",
            "profile_id",
        ));
    }
    if req.budget_envelope.max_retrieval_units == 0
        || req.budget_envelope.max_analysis_units == 0
        || req.budget_envelope.max_llm_tokens_in == 0
        || req.budget_envelope.max_llm_tokens_out == 0
        || req.budget_envelope.max_total_cost_micros == 0
    {
        return Err(AnalyticsError::validation(
            "invalid_budget_envelope",
            "budget envelope caps must be positive",
            "budget_envelope",
        ));
    }
    if req.budget_envelope.provenance_ref.trim().is_empty() {
        return Err(AnalyticsError::validation(
            "invalid_budget_provenance_ref",
            "budget_envelope.provenance_ref is required",
            "budget_envelope.provenance_ref",
        ));
    }

    let start = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
        AnalyticsError::validation(
            "invalid_start_date",
            "start_date must use YYYY-MM-DD",
            "start_date",
        )
    })?;
    let end = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
        AnalyticsError::validation(
            "invalid_end_date",
            "end_date must use YYYY-MM-DD",
            "end_date",
        )
    })?;

    if start > end {
        return Err(AnalyticsError::new(
            "invalid_date_range",
            "start_date must be less than or equal to end_date",
            vec!["start_date".to_string(), "end_date".to_string()],
            None,
        ));
    }

    let span_days = (end - start).num_days() + 1;
    if span_days > MAX_DATE_SPAN_DAYS {
        return Err(AnalyticsError::new(
            "date_span_exceeded",
            format!("date range cannot exceed {} days", MAX_DATE_SPAN_DAYS),
            vec!["start_date".to_string(), "end_date".to_string()],
            None,
        ));
    }

    Ok((start, end))
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::validators`
/// purpose: Validate artifact invariants and produce a structured check report.
pub fn validate_mock_analytics_artifact_v1(
    artifact: &MockAnalyticsArtifactV1,
) -> AnalyticsValidationReportV1 {
    let mut checks = Vec::new();

    checks.push(check(
        "schema_version",
        artifact.schema_version == MOCK_ANALYTICS_SCHEMA_VERSION_V1,
        "schema_version must match v1 constant",
    ));

    checks.push(check(
        "report_impressions_gte_clicks",
        artifact.report.total_metrics.impressions >= artifact.report.total_metrics.clicks,
        "total impressions must be >= total clicks",
    ));

    checks.push(check(
        "report_non_negative",
        artifact.report.total_metrics.cost >= 0.0
            && artifact.report.total_metrics.conversions >= 0.0
            && artifact.report.total_metrics.conversions_value >= 0.0,
        "total cost/conversions/conversion value must be non-negative",
    ));

    let derived_ctr = if artifact.report.total_metrics.impressions > 0 {
        (artifact.report.total_metrics.clicks as f64
            / artifact.report.total_metrics.impressions as f64)
            * 100.0
    } else {
        0.0
    };
    checks.push(check(
        "report_ctr_consistency",
        (artifact.report.total_metrics.ctr - derived_ctr).abs() <= METRIC_EPSILON,
        "CTR must match derived CTR within epsilon",
    ));

    let simulated_high_confidence = artifact
        .inferred_guidance
        .iter()
        .any(|g| g.confidence_label.eq_ignore_ascii_case("high"));
    checks.push(check(
        "simulated_confidence_not_high",
        !simulated_high_confidence,
        "simulated guidance cannot be marked high confidence",
    ));

    checks.push(check(
        "provenance_present",
        !artifact.provenance.is_empty(),
        "artifact must include at least one provenance record",
    ));
    checks.push(check(
        "provenance_contract_version_present",
        artifact.provenance.iter().all(|item| {
            item.validated_contract_version
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        }),
        "every provenance row must include validated_contract_version",
    ));

    checks.push(check(
        "uncertainty_notes_present",
        !artifact.uncertainty_notes.is_empty(),
        "artifact must include uncertainty notes",
    ));

    let high_severity_failures = artifact
        .quality_controls
        .schema_drift_checks
        .iter()
        .chain(artifact.quality_controls.identity_resolution_checks.iter())
        .chain(artifact.quality_controls.freshness_sla_checks.iter())
        .chain(artifact.quality_controls.cross_source_checks.iter())
        .chain(artifact.quality_controls.budget_checks.iter())
        .any(|check| !check.passed && check.severity.eq_ignore_ascii_case("high"));
    let all_quality_checks_passed = artifact
        .quality_controls
        .schema_drift_checks
        .iter()
        .chain(artifact.quality_controls.identity_resolution_checks.iter())
        .chain(artifact.quality_controls.freshness_sla_checks.iter())
        .chain(artifact.quality_controls.cross_source_checks.iter())
        .chain(artifact.quality_controls.budget_checks.iter())
        .all(|check| check.passed);
    checks.push(check(
        "quality_controls_high_severity",
        !high_severity_failures,
        "quality controls cannot contain failing high severity checks",
    ));

    checks.push(check(
        "quality_controls_consistency",
        artifact.quality_controls.is_healthy == all_quality_checks_passed,
        "quality control health should match quality check pass/fail aggregate",
    ));
    let freshness_policy_covers_sources = artifact.provenance.iter().all(|item| {
        artifact
            .freshness_policy
            .threshold_for(&item.source_system)
            .is_some()
    });
    checks.push(check(
        "freshness_policy_coverage",
        freshness_policy_covers_sources,
        "freshness policy must define thresholds for each provenance source",
    ));
    let budget_exceeded = artifact
        .budget
        .events
        .iter()
        .any(|event| event.outcome.eq_ignore_ascii_case("blocked"));
    checks.push(check(
        "budget_fail_closed",
        !budget_exceeded,
        "budget exceeded events must block artifact validity",
    ));
    checks.push(check(
        "budget_daily_hard_cap",
        artifact.budget.daily_spent_after_micros <= artifact.budget.hard_daily_cap_micros,
        "daily spend must remain below or equal to hard daily cap",
    ));
    let has_blocking_cleaning = artifact
        .ingest_cleaning_notes
        .iter()
        .any(|note| note.severity.eq_ignore_ascii_case("block"));
    checks.push(check(
        "ingest_cleaning_blocking_count",
        !has_blocking_cleaning,
        "ingest cleaning notes cannot contain blocking severity in publishable artifacts",
    ));

    let ratios_valid = [
        artifact.data_quality.completeness_ratio,
        artifact.data_quality.identity_join_coverage_ratio,
        artifact.data_quality.freshness_pass_ratio,
        artifact.data_quality.reconciliation_pass_ratio,
        artifact.data_quality.cross_source_pass_ratio,
        artifact.data_quality.budget_pass_ratio,
        artifact.data_quality.quality_score,
    ]
    .iter()
    .all(|value| value.is_finite() && (0.0..=1.0).contains(value));
    checks.push(check(
        "data_quality_ratio_bounds",
        ratios_valid,
        "data quality ratios must be finite and within [0.0, 1.0]",
    ));

    let is_valid = checks.iter().all(|c| c.passed);
    AnalyticsValidationReportV1 { is_valid, checks }
}

fn check(code: &str, passed: bool, message: &str) -> ValidationCheck {
    ValidationCheck {
        code: code.to_string(),
        passed,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_models::analytics::{AnalyticsReport, SourceClassLabel, SourceProvenance};
    use crate::subsystems::marketing_data_analysis::contracts::{
        AnalyticsRunMetadataV1, EvidenceItem, GuidanceItem, MockAnalyticsArtifactV1,
    };

    #[test]
    fn request_validator_rejects_bad_dates() {
        let bad = MockAnalyticsRequestV1 {
            start_date: "2026/01/01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: None,
            profile_id: "small".to_string(),
            include_narratives: true,
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };
        assert!(validate_mock_analytics_request_v1(&bad).is_err());
    }

    #[test]
    fn artifact_validator_rejects_high_confidence_simulated_guidance() {
        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: MockAnalyticsRequestV1 {
                start_date: "2026-01-01".to_string(),
                end_date: "2026-01-02".to_string(),
                campaign_filter: None,
                ad_group_filter: None,
                seed: Some(1),
                profile_id: "small".to_string(),
                include_narratives: true,
                budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
            },
            metadata: AnalyticsRunMetadataV1 {
                run_id: "r".to_string(),
                connector_id: "simulated".to_string(),
                profile_id: "small".to_string(),
                seed: 1,
                schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
                date_span_days: 2,
                requested_at_utc: None,
            },
            report: AnalyticsReport::default(),
            observed_evidence: vec![EvidenceItem {
                evidence_id: "e".to_string(),
                label: "x".to_string(),
                value: "y".to_string(),
                source_class: "simulated".to_string(),
                metric_key: None,
                observed_window: None,
                comparator_value: None,
                notes: Vec::new(),
            }],
            inferred_guidance: vec![GuidanceItem {
                guidance_id: "g".to_string(),
                text: "bad".to_string(),
                confidence_label: "high".to_string(),
                evidence_refs: Vec::new(),
                attribution_basis: None,
                calibration_bps: None,
                calibration_band: None,
            }],
            uncertainty_notes: vec!["simulated".to_string()],
            provenance: vec![SourceProvenance {
                connector_id: "simulated".to_string(),
                source_class: SourceClassLabel::Simulated,
                source_system: "mock".to_string(),
                collected_at_utc: "synthetic".to_string(),
                freshness_minutes: 0,
                validated_contract_version: Some("ingest_contract.v1".to_string()),
                rejected_rows_count: 0,
                cleaning_note_count: 0,
            }],
            ingest_cleaning_notes: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
            freshness_policy: Default::default(),
            budget: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };

        artifact.report.total_metrics.impressions = 1;
        let report = validate_mock_analytics_artifact_v1(&artifact);
        assert!(!report.is_valid);
    }
}
