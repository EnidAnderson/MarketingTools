use super::analytics_config::{
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
};
use super::budget::{build_budget_plan, enforce_daily_hard_cap, BudgetCategory, BudgetGuard};
use super::connector_v2::{
    generate_simulated_ga4_events, generate_simulated_google_ads_rows,
    generate_simulated_wix_orders, AnalyticsConnectorContractV2, SimulatedAnalyticsConnectorV2,
};
use super::contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1, BudgetSummaryV1,
    DataQualitySummaryV1, EvidenceItem, FreshnessSlaPolicyV1, GuidanceItem, IngestCleaningNoteV1,
    KpiAttributionNarrativeV1, MockAnalyticsArtifactV1, MockAnalyticsRequestV1, OperatorSummaryV1,
    QualityCheckV1, ReconciliationPolicyV1, SourceWindowGranularityV1,
    MOCK_ANALYTICS_SCHEMA_VERSION_V1,
};
use super::ingest::{
    parse_ga4_event, parse_wix_order, window_completeness, CleaningNote, Ga4EventRawV1,
    TimeGranularity, WixOrderRawV1,
};
use super::validators::{validate_mock_analytics_artifact_v1, validate_mock_analytics_request_v1};
use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupReportRow, AdGroupResource, AnalyticsReport,
    CampaignReportRow, CampaignResource, GoogleAdsRow, KeywordReportRow, MetricsData,
    ReportMetrics, SourceClassLabel, SourceProvenance,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MIN_IDENTITY_COVERAGE_RATIO: f64 = 0.98;
const INGEST_CONTRACT_VERSION: &str = "ingest_contract.v1";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::service`
/// purpose: Transport-neutral service contract for mock analytics orchestration.
#[async_trait]
pub trait MarketAnalysisService: Send + Sync {
    async fn run_mock_analysis(
        &self,
        request: MockAnalyticsRequestV1,
    ) -> Result<MockAnalyticsArtifactV1, AnalyticsError>;
}

/// # NDOC
/// component: `subsystems::marketing_data_analysis::service`
/// purpose: Default deterministic mock analytics implementation.
pub struct DefaultMarketAnalysisService {
    connector: Arc<dyn AnalyticsConnectorContractV2>,
    connector_config: AnalyticsConnectorConfigV1,
}

impl DefaultMarketAnalysisService {
    pub fn new() -> Self {
        let connector = Arc::new(SimulatedAnalyticsConnectorV2::new());
        let connector_config = AnalyticsConnectorConfigV1::simulated_defaults();
        debug_assert!(validate_analytics_connector_config_v1(&connector_config).is_ok());
        Self {
            connector,
            connector_config,
        }
    }

    pub fn with_connector(connector: Arc<dyn AnalyticsConnectorContractV2>) -> Self {
        Self {
            connector,
            connector_config: AnalyticsConnectorConfigV1::simulated_defaults(),
        }
    }

    pub fn with_connector_and_config(
        connector: Arc<dyn AnalyticsConnectorContractV2>,
        connector_config: AnalyticsConnectorConfigV1,
    ) -> Result<Self, AnalyticsError> {
        validate_analytics_connector_config_v1(&connector_config)?;
        Ok(Self {
            connector,
            connector_config,
        })
    }
}

impl Default for DefaultMarketAnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketAnalysisService for DefaultMarketAnalysisService {
    async fn run_mock_analysis(
        &self,
        request: MockAnalyticsRequestV1,
    ) -> Result<MockAnalyticsArtifactV1, AnalyticsError> {
        let (start, end) = validate_mock_analytics_request_v1(&request)?;
        let budget_plan = build_budget_plan(&request, start, end)?;
        let daily_status = enforce_daily_hard_cap(
            budget_plan.estimated.total_cost_micros,
            chrono::Utc::now().date_naive(),
        )?;
        let mut budget_guard = BudgetGuard::new(request.budget_envelope.clone());
        let seed = resolve_seed(&request);
        let connector_health = self.connector.healthcheck(&self.connector_config).await?;
        if self.connector_config.mode == AnalyticsConnectorModeV1::ObservedReadOnly
            && !connector_health.ok
        {
            return Err(AnalyticsError::new(
                "analytics_preflight_blocked",
                "connector healthcheck failed in observed_read_only mode",
                vec!["connector_config.mode".to_string()],
                Some(serde_json::json!({
                    "blocking_reasons": connector_health.blocking_reasons,
                    "warning_reasons": connector_health.warning_reasons,
                })),
            ));
        }

        budget_guard.spend(
            BudgetCategory::Retrieval,
            budget_plan.estimated.retrieval_units,
            "mock_analytics.fetch",
        )?;
        let rows = match self
            .connector
            .fetch_google_ads_rows(
                &self.connector_config,
                &request,
                start,
                budget_plan.effective_end,
                seed,
            )
            .await
        {
            Ok(rows) if !rows.is_empty() => rows,
            Ok(_) | Err(_) if self.connector_config.mode.is_simulated() => {
                generate_simulated_google_ads_rows(&request, start, budget_plan.effective_end, seed)
            }
            Ok(_) => {
                return Err(AnalyticsError::internal(
                    "analytics_connector_empty_rows",
                    "connector returned no google ads rows in observed mode",
                ));
            }
            Err(err) => return Err(err),
        };
        let ga4_events = match self
            .connector
            .fetch_ga4_events(
                &self.connector_config,
                start,
                budget_plan.effective_end,
                seed,
            )
            .await
        {
            Ok(events) if !events.is_empty() => events,
            Ok(_) | Err(_) if self.connector_config.mode.is_simulated() => {
                generate_simulated_ga4_events(start, budget_plan.effective_end, seed)
            }
            Ok(_) => {
                return Err(AnalyticsError::internal(
                    "analytics_connector_empty_ga4",
                    "connector returned no GA4 events in observed mode",
                ));
            }
            Err(err) => return Err(err),
        };
        let wix_orders = match self
            .connector
            .fetch_wix_orders(
                &self.connector_config,
                start,
                budget_plan.effective_end,
                seed,
            )
            .await
        {
            Ok(orders) if !orders.is_empty() => orders,
            Ok(_) | Err(_) if self.connector_config.mode.is_simulated() => {
                generate_simulated_wix_orders(start, budget_plan.effective_end, seed)
            }
            Ok(_) => {
                return Err(AnalyticsError::internal(
                    "analytics_connector_empty_wix_orders",
                    "connector returned no Wix orders in observed mode",
                ));
            }
            Err(err) => return Err(err),
        };
        match self
            .connector
            .fetch_wix_sessions(
                &self.connector_config,
                start,
                budget_plan.effective_end,
                seed,
            )
            .await
        {
            Ok(sessions) if !sessions.is_empty() => sessions,
            Ok(_) | Err(_) if self.connector_config.mode.is_simulated() => Vec::new(),
            Ok(_) => {
                return Err(AnalyticsError::internal(
                    "analytics_connector_empty_wix_sessions",
                    "connector returned no Wix sessions in observed mode",
                ));
            }
            Err(err) => return Err(err),
        };
        budget_guard.spend(
            BudgetCategory::Analysis,
            budget_plan.estimated.analysis_units,
            "mock_analytics.transform",
        )?;
        if budget_plan.include_narratives {
            budget_guard.spend(
                BudgetCategory::LlmTokensIn,
                budget_plan.estimated.llm_tokens_in,
                "mock_analytics.narrative_tokens_in",
            )?;
            budget_guard.spend(
                BudgetCategory::LlmTokensOut,
                budget_plan.estimated.llm_tokens_out,
                "mock_analytics.narrative_tokens_out",
            )?;
        }
        budget_guard.spend(
            BudgetCategory::CostMicros,
            budget_plan.estimated.total_cost_micros,
            "mock_analytics.total_cost",
        )?;

        let report = rows_to_report(rows, &request, start, budget_plan.effective_end);
        let ingest_audit = collect_ingest_cleaning_notes(&ga4_events, &wix_orders)?;
        let freshness_policy = FreshnessSlaPolicyV1::default();
        let reconciliation_policy = ReconciliationPolicyV1::default();
        let connector_id = self.connector.capabilities().connector_id;
        let provenance = build_mock_provenance(&connector_id, seed, &ingest_audit.note_counts);
        let (observed_units_by_source, granularity_by_source, provided_observation_sources) =
            resolve_source_window_inputs(&request, start, budget_plan.effective_end)?;
        let cross_source_checks = build_cross_source_checks(&report, seed, &reconciliation_policy);
        let (observed_evidence, inferred_guidance, mut uncertainty_notes) =
            build_evidence_and_guidance(&report, budget_plan.include_narratives);
        if budget_plan.clipped || budget_plan.sampled || !budget_plan.include_narratives {
            uncertainty_notes.push(
                "Budget policy modified run scope; see artifact.budget for clipping/sampling details."
                    .to_string(),
            );
        }
        let budget = budget_guard.summary(
            &budget_plan.estimated,
            daily_status,
            budget_plan.clipped,
            budget_plan.sampled,
            budget_plan.skipped_modules,
            budget_plan.clipped || budget_plan.sampled || !budget_plan.include_narratives,
        );
        let budget_checks = build_budget_checks(&budget);
        let quality_controls = build_quality_controls(
            &report,
            &provenance,
            budget_checks,
            cross_source_checks,
            &freshness_policy,
            &reconciliation_policy,
            start,
            budget_plan.effective_end,
            &observed_units_by_source,
            &granularity_by_source,
            provided_observation_sources.as_ref(),
        );
        let data_quality = build_data_quality_summary(&quality_controls);
        let operator_summary = build_operator_summary(&report, &observed_evidence);

        let metadata = AnalyticsRunMetadataV1 {
            run_id: deterministic_run_id(&request, seed),
            connector_id: connector_id.clone(),
            profile_id: request.profile_id.clone(),
            seed,
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            date_span_days: ((budget_plan.effective_end - start).num_days() + 1) as u32,
            requested_at_utc: None,
        };

        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request,
            metadata,
            report,
            observed_evidence,
            inferred_guidance,
            uncertainty_notes,
            provenance,
            ingest_cleaning_notes: ingest_audit.notes,
            validation: super::contracts::AnalyticsValidationReportV1 {
                is_valid: false,
                checks: Vec::new(),
            },
            quality_controls,
            data_quality,
            freshness_policy,
            reconciliation_policy,
            budget,
            historical_analysis: Default::default(),
            operator_summary,
            persistence: None,
        };

        artifact.validation = validate_mock_analytics_artifact_v1(&artifact);
        if !artifact.validation.is_valid {
            return Err(AnalyticsError::internal(
                "artifact_invariant_violation",
                "generated artifact failed invariant checks",
            ));
        }

        Ok(artifact)
    }
}

fn resolve_seed(request: &MockAnalyticsRequestV1) -> u64 {
    if let Some(seed) = request.seed {
        return seed;
    }
    let mut hasher = Sha256::new();
    hasher.update(request.start_date.as_bytes());
    hasher.update(request.end_date.as_bytes());
    hasher.update(request.profile_id.as_bytes());
    if let Some(v) = &request.campaign_filter {
        hasher.update(v.as_bytes());
    }
    if let Some(v) = &request.ad_group_filter {
        hasher.update(v.as_bytes());
    }
    for observation in &request.source_window_observations {
        hasher.update(observation.source_system.as_bytes());
        match observation.granularity {
            SourceWindowGranularityV1::Day => hasher.update(b"day"),
            SourceWindowGranularityV1::Hour => hasher.update(b"hour"),
        }
        for ts in &observation.observed_timestamps_utc {
            hasher.update(ts.as_bytes());
        }
    }
    hasher.update(if request.include_narratives {
        b"1"
    } else {
        b"0"
    });
    let digest = hasher.finalize();
    u64::from_le_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

fn deterministic_run_id(request: &MockAnalyticsRequestV1, seed: u64) -> String {
    let serialized = serde_json::to_string(request).unwrap_or_else(|_| "{}".to_string());
    let mut hasher = Sha256::new();
    hasher.update(MOCK_ANALYTICS_SCHEMA_VERSION_V1.as_bytes());
    hasher.update(serialized.as_bytes());
    hasher.update(seed.to_le_bytes());
    let digest = hasher.finalize();
    format!("mockrun-{:x}", digest)[..24].to_string()
}

fn rows_to_report(
    rows: Vec<GoogleAdsRow>,
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
) -> AnalyticsReport {
    type CampaignAgg = (String, String, ReportMetrics);
    type AdGroupAgg = (String, String, String, ReportMetrics);
    type KeywordAgg = (String, String, String, String, ReportMetrics, Option<u32>);

    let mut total = ReportMetrics::default();
    let mut campaigns: BTreeMap<String, CampaignAgg> = BTreeMap::new();
    let mut ad_groups: BTreeMap<String, AdGroupAgg> = BTreeMap::new();
    let mut keywords: BTreeMap<String, KeywordAgg> = BTreeMap::new();

    for row in rows {
        let Some(metrics) = row.metrics else { continue };
        let campaign = row.campaign.unwrap_or(CampaignResource {
            resourceName: "".to_string(),
            id: "".to_string(),
            name: "".to_string(),
            status: "UNKNOWN".to_string(),
        });
        let ad_group = row.adGroup.unwrap_or(AdGroupResource {
            resourceName: "".to_string(),
            id: "".to_string(),
            name: "".to_string(),
            status: "UNKNOWN".to_string(),
            campaignResourceName: "".to_string(),
        });
        let criterion = row.adGroupCriterion.unwrap_or(AdGroupCriterionResource {
            resourceName: "".to_string(),
            criterionId: "".to_string(),
            status: "UNKNOWN".to_string(),
            keyword: None,
            qualityScore: None,
            adGroupResourceName: "".to_string(),
        });

        let line = from_metrics_data(&metrics);
        total = sum_metrics(&total, &line);

        let campaign_entry = campaigns.entry(campaign.id.clone()).or_insert((
            campaign.name.clone(),
            campaign.status.clone(),
            ReportMetrics::default(),
        ));
        campaign_entry.0 = campaign.name.clone();
        campaign_entry.1 = campaign.status.clone();
        campaign_entry.2 = sum_metrics(&campaign_entry.2, &line);

        let ad_group_entry = ad_groups.entry(ad_group.id.clone()).or_insert((
            campaign.id.clone(),
            ad_group.name.clone(),
            ad_group.status.clone(),
            ReportMetrics::default(),
        ));
        ad_group_entry.0 = campaign.id.clone();
        ad_group_entry.1 = ad_group.name.clone();
        ad_group_entry.2 = ad_group.status.clone();
        ad_group_entry.3 = sum_metrics(&ad_group_entry.3, &line);

        let keyword_entry = keywords.entry(criterion.criterionId.clone()).or_insert((
            campaign.id.clone(),
            ad_group.id.clone(),
            criterion
                .keyword
                .as_ref()
                .map(|k| k.text.clone())
                .unwrap_or_default(),
            criterion
                .keyword
                .as_ref()
                .map(|k| k.matchType.clone())
                .unwrap_or_else(|| "EXACT".to_string()),
            ReportMetrics::default(),
            criterion.qualityScore,
        ));
        keyword_entry.0 = campaign.id.clone();
        keyword_entry.1 = ad_group.id.clone();
        keyword_entry.4 = sum_metrics(&keyword_entry.4, &line);
        keyword_entry.5 = criterion.qualityScore;
    }

    let campaign_data = campaigns
        .into_iter()
        .map(|(id, (name, status, metrics))| CampaignReportRow {
            date: "".to_string(),
            campaign_id: id,
            campaign_name: name,
            campaign_status: status,
            metrics: round_metrics(metrics),
        })
        .collect::<Vec<_>>();

    let ad_group_data = ad_groups
        .into_iter()
        .map(|(id, (campaign_id, name, status, metrics))| {
            let campaign_name = campaign_data
                .iter()
                .find(|c| c.campaign_id == campaign_id)
                .map(|c| c.campaign_name.clone())
                .unwrap_or_default();
            AdGroupReportRow {
                date: "".to_string(),
                campaign_id,
                campaign_name,
                ad_group_id: id,
                ad_group_name: name,
                ad_group_status: status,
                metrics: round_metrics(metrics),
            }
        })
        .collect::<Vec<_>>();

    let keyword_data = keywords
        .into_iter()
        .map(
            |(id, (campaign_id, ad_group_id, text, match_type, metrics, quality_score))| {
                let campaign_name = campaign_data
                    .iter()
                    .find(|c| c.campaign_id == campaign_id)
                    .map(|c| c.campaign_name.clone())
                    .unwrap_or_default();
                let ad_group_name = ad_group_data
                    .iter()
                    .find(|ag| ag.ad_group_id == ad_group_id)
                    .map(|ag| ag.ad_group_name.clone())
                    .unwrap_or_default();
                KeywordReportRow {
                    date: "".to_string(),
                    campaign_id,
                    campaign_name,
                    ad_group_id,
                    ad_group_name,
                    keyword_id: id,
                    keyword_text: text,
                    match_type,
                    quality_score,
                    metrics: round_metrics(metrics),
                }
            },
        )
        .collect::<Vec<_>>();

    AnalyticsReport {
        report_name: format!(
            "Mock Analytics Report: {} to {}",
            request.start_date, request.end_date
        ),
        date_range: format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d")),
        total_metrics: round_metrics(total),
        campaign_data,
        ad_group_data,
        keyword_data,
    }
}

fn from_metrics_data(data: &MetricsData) -> ReportMetrics {
    let cost = data.costMicros as f64 / 1_000_000.0;
    derived_metrics(
        data.impressions,
        data.clicks,
        cost,
        data.conversions,
        data.conversionsValue,
    )
}

fn sum_metrics(a: &ReportMetrics, b: &ReportMetrics) -> ReportMetrics {
    derived_metrics(
        a.impressions + b.impressions,
        a.clicks + b.clicks,
        a.cost + b.cost,
        a.conversions + b.conversions,
        a.conversions_value + b.conversions_value,
    )
}

fn round_metrics(mut m: ReportMetrics) -> ReportMetrics {
    m.cost = round4(m.cost);
    m.conversions = round4(m.conversions);
    m.conversions_value = round4(m.conversions_value);
    m.ctr = round4(m.ctr);
    m.cpc = round4(m.cpc);
    m.cpa = round4(m.cpa);
    m.roas = round4(m.roas);
    m
}

fn derived_metrics(
    impressions: u64,
    clicks: u64,
    cost: f64,
    conversions: f64,
    conversions_value: f64,
) -> ReportMetrics {
    let ctr = if impressions > 0 {
        (clicks as f64 / impressions as f64) * 100.0
    } else {
        0.0
    };
    let cpc = if clicks > 0 {
        cost / clicks as f64
    } else {
        0.0
    };
    let cpa = if conversions > 0.0 {
        cost / conversions
    } else {
        0.0
    };
    let roas = if cost > 0.0 {
        conversions_value / cost
    } else {
        0.0
    };
    ReportMetrics {
        impressions,
        clicks,
        cost,
        conversions,
        conversions_value,
        ctr,
        cpc,
        cpa,
        roas,
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn build_evidence_and_guidance(
    report: &AnalyticsReport,
    include_narratives: bool,
) -> (Vec<EvidenceItem>, Vec<GuidanceItem>, Vec<String>) {
    let mut evidence = Vec::new();
    evidence.push(EvidenceItem {
        evidence_id: "ev_total_impressions".to_string(),
        label: "Total Impressions".to_string(),
        value: report.total_metrics.impressions.to_string(),
        source_class: "simulated".to_string(),
        metric_key: Some("impressions".to_string()),
        observed_window: Some(report.date_range.clone()),
        comparator_value: None,
        notes: vec!["Deterministic mock aggregation across selected date window.".to_string()],
    });
    evidence.push(EvidenceItem {
        evidence_id: "ev_total_clicks".to_string(),
        label: "Total Clicks".to_string(),
        value: report.total_metrics.clicks.to_string(),
        source_class: "simulated".to_string(),
        metric_key: Some("clicks".to_string()),
        observed_window: Some(report.date_range.clone()),
        comparator_value: None,
        notes: vec!["Includes all simulated campaigns/ad groups after filters.".to_string()],
    });

    let mut guidance = Vec::new();
    if include_narratives {
        guidance.push(GuidanceItem {
            guidance_id: "gd_budget_focus".to_string(),
            text: "Prioritize campaigns with above-median ROAS in next optimization pass."
                .to_string(),
            confidence_label: "medium".to_string(),
            evidence_refs: vec!["ev_total_clicks".to_string()],
            attribution_basis: Some("roas_vs_cost_distribution".to_string()),
            calibration_bps: Some(6500),
            calibration_band: Some("medium".to_string()),
        });
        guidance.push(GuidanceItem {
            guidance_id: "gd_quality_improve".to_string(),
            text: "Review ad groups with low CTR for creative/keyword alignment.".to_string(),
            confidence_label: "medium".to_string(),
            evidence_refs: vec!["ev_total_impressions".to_string()],
            attribution_basis: Some("ctr_vs_impressions_mix".to_string()),
            calibration_bps: Some(6200),
            calibration_band: Some("medium".to_string()),
        });
    }

    let uncertainty = vec![
        "Dataset is simulated and intended for tool integration validation only.".to_string(),
        "Attribution assumptions are simplified for deterministic replay.".to_string(),
    ];
    (evidence, guidance, uncertainty)
}

fn build_mock_provenance(
    connector_id: &str,
    seed: u64,
    cleaning_note_count_by_source: &BTreeMap<String, u32>,
) -> Vec<SourceProvenance> {
    // Keep provenance byte-stable for replay while still simulating source-specific lag.
    let ga4_freshness = 30 + ((seed % 40) as u32);
    let ads_freshness = 60 + ((seed % 50) as u32);
    let wix_freshness = 90 + ((seed % 60) as u32);
    vec![
        SourceProvenance {
            connector_id: connector_id.to_string(),
            source_class: SourceClassLabel::Simulated,
            source_system: "google_ads".to_string(),
            collected_at_utc: "deterministic-simulated".to_string(),
            freshness_minutes: ads_freshness,
            validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
            rejected_rows_count: 0,
            cleaning_note_count: *cleaning_note_count_by_source
                .get("google_ads")
                .unwrap_or(&0),
        },
        SourceProvenance {
            connector_id: connector_id.to_string(),
            source_class: SourceClassLabel::Simulated,
            source_system: "ga4".to_string(),
            collected_at_utc: "deterministic-simulated".to_string(),
            freshness_minutes: ga4_freshness,
            validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
            rejected_rows_count: 0,
            cleaning_note_count: *cleaning_note_count_by_source.get("ga4").unwrap_or(&0),
        },
        SourceProvenance {
            connector_id: connector_id.to_string(),
            source_class: SourceClassLabel::Simulated,
            source_system: "wix_storefront".to_string(),
            collected_at_utc: "deterministic-simulated".to_string(),
            freshness_minutes: wix_freshness,
            validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
            rejected_rows_count: 0,
            cleaning_note_count: *cleaning_note_count_by_source
                .get("wix_storefront")
                .unwrap_or(&0),
        },
    ]
}

fn build_cross_source_checks(
    report: &AnalyticsReport,
    seed: u64,
    reconciliation_policy: &ReconciliationPolicyV1,
) -> Vec<QualityCheckV1> {
    let attributed_revenue = report.total_metrics.conversions_value;
    let wix_revenue_multiplier = 0.99 + ((seed % 3) as f64 * 0.005);
    let wix_gross_revenue = round4(attributed_revenue * wix_revenue_multiplier);

    let ga4_sessions = round4((report.total_metrics.clicks as f64) * 0.92);
    let clicks = report.total_metrics.clicks as f64;

    let revenue_check_code = "cross_source_attributed_revenue_within_wix_gross";
    let (revenue_passed, revenue_severity, revenue_expected) =
        if let Some(tolerance) = reconciliation_policy.tolerance_for(revenue_check_code) {
            let rel_tol = tolerance.max_relative_delta.unwrap_or(0.0).max(0.0);
            let max_allowed = wix_gross_revenue * (1.0 + rel_tol);
            (
                attributed_revenue <= max_allowed,
                tolerance.severity.clone(),
                format!("attributed_revenue <= wix_gross * (1 + {:.2})", rel_tol),
            )
        } else {
            (
                false,
                "high".to_string(),
                "reconciliation policy must define revenue tolerance".to_string(),
            )
        };

    let sessions_check_code = "cross_source_ga4_sessions_within_click_bound";
    let (sessions_passed, sessions_severity, sessions_expected) =
        if let Some(tolerance) = reconciliation_policy.tolerance_for(sessions_check_code) {
            let rel_tol = tolerance.max_relative_delta.unwrap_or(0.0).max(0.0);
            let max_allowed = clicks * (1.0 + rel_tol);
            (
                ga4_sessions <= max_allowed,
                tolerance.severity.clone(),
                format!("ga4_sessions <= ad_clicks * (1 + {:.2})", rel_tol),
            )
        } else {
            (
                false,
                "high".to_string(),
                "reconciliation policy must define GA4 session tolerance".to_string(),
            )
        };

    vec![
        QualityCheckV1 {
            code: revenue_check_code.to_string(),
            passed: revenue_passed,
            severity: revenue_severity,
            observed: format!(
                "attributed_revenue={:.4}, wix_gross={:.4}",
                attributed_revenue, wix_gross_revenue
            ),
            expected: revenue_expected,
        },
        QualityCheckV1 {
            code: sessions_check_code.to_string(),
            passed: sessions_passed,
            severity: sessions_severity,
            observed: format!("ga4_sessions={:.4}, ad_clicks={:.4}", ga4_sessions, clicks),
            expected: sessions_expected,
        },
    ]
}

fn build_mock_observed_window_inputs(
    start: NaiveDate,
    end: NaiveDate,
) -> (
    BTreeMap<String, Vec<DateTime<Utc>>>,
    BTreeMap<String, TimeGranularity>,
) {
    let mut by_source = BTreeMap::new();
    let mut granularity_by_source = BTreeMap::new();
    for source in ["google_ads", "ga4", "wix_storefront"] {
        let mut points = Vec::new();
        let mut current = start;
        while current <= end {
            if let Some(midday) = current.and_hms_opt(12, 0, 0) {
                points.push(midday.and_utc());
            }
            let Some(next) = current.checked_add_signed(Duration::days(1)) else {
                break;
            };
            current = next;
        }
        by_source.insert(source.to_string(), points);
        granularity_by_source.insert(source.to_string(), TimeGranularity::Day);
    }
    (by_source, granularity_by_source)
}

fn resolve_source_window_inputs(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<
    (
        BTreeMap<String, Vec<DateTime<Utc>>>,
        BTreeMap<String, TimeGranularity>,
        Option<BTreeSet<String>>,
    ),
    AnalyticsError,
> {
    let (mut observed_by_source, mut granularity_by_source) =
        build_mock_observed_window_inputs(start, end);
    if request.source_window_observations.is_empty() {
        return Ok((observed_by_source, granularity_by_source, None));
    }

    let start_utc = start
        .and_hms_opt(0, 0, 0)
        .expect("start date should support midnight")
        .and_utc();
    let end_utc = end
        .and_hms_opt(23, 59, 59)
        .expect("end date should support second boundary")
        .and_utc();

    let mut provided_sources = BTreeSet::new();
    for (idx, observation) in request.source_window_observations.iter().enumerate() {
        let source = observation.source_system.trim();
        if source.is_empty() {
            return Err(AnalyticsError::new(
                "invalid_source_window_observation_source",
                "source_window_observations.source_system cannot be empty",
                vec![format!("source_window_observations[{idx}].source_system")],
                None,
            ));
        }
        let granularity = match observation.granularity {
            SourceWindowGranularityV1::Day => TimeGranularity::Day,
            SourceWindowGranularityV1::Hour => TimeGranularity::Hour,
        };
        let mut parsed_points = Vec::new();
        for (ts_idx, raw) in observation.observed_timestamps_utc.iter().enumerate() {
            let parsed = DateTime::parse_from_rfc3339(raw).map_err(|_| {
                AnalyticsError::new(
                    "invalid_source_window_observation_timestamp",
                    "source_window_observations timestamps must be RFC3339",
                    vec![format!(
                        "source_window_observations[{idx}].observed_timestamps_utc[{ts_idx}]"
                    )],
                    None,
                )
            })?;
            let utc = parsed.with_timezone(&Utc);
            if utc >= start_utc && utc <= end_utc {
                parsed_points.push(utc);
            }
        }
        provided_sources.insert(source.to_string());
        observed_by_source.insert(source.to_string(), parsed_points);
        granularity_by_source.insert(source.to_string(), granularity);
    }
    Ok((
        observed_by_source,
        granularity_by_source,
        Some(provided_sources),
    ))
}

fn build_quality_controls(
    report: &AnalyticsReport,
    provenance: &[SourceProvenance],
    budget_checks: Vec<QualityCheckV1>,
    cross_source_checks: Vec<QualityCheckV1>,
    freshness_policy: &FreshnessSlaPolicyV1,
    reconciliation_policy: &ReconciliationPolicyV1,
    start: NaiveDate,
    end: NaiveDate,
    observed_units_by_source: &BTreeMap<String, Vec<DateTime<Utc>>>,
    granularity_by_source: &BTreeMap<String, TimeGranularity>,
    provided_observation_sources: Option<&BTreeSet<String>>,
) -> AnalyticsQualityControlsV1 {
    let keyword_coverage_ratio = if report.keyword_data.is_empty() {
        1.0
    } else {
        report
            .keyword_data
            .iter()
            .filter(|row| !row.ad_group_id.trim().is_empty())
            .count() as f64
            / report.keyword_data.len() as f64
    };
    let ad_group_coverage_ratio = if report.ad_group_data.is_empty() {
        1.0
    } else {
        report
            .ad_group_data
            .iter()
            .filter(|row| !row.campaign_id.trim().is_empty())
            .count() as f64
            / report.ad_group_data.len() as f64
    };
    let sum_campaign_spend = report
        .campaign_data
        .iter()
        .map(|row| row.metrics.cost)
        .sum::<f64>();
    let sum_campaign_revenue = report
        .campaign_data
        .iter()
        .map(|row| row.metrics.conversions_value)
        .sum::<f64>();

    let schema_drift_checks = vec![
        QualityCheckV1 {
            code: "schema_campaign_required_fields".to_string(),
            passed: report.campaign_data.iter().all(|row| {
                !row.campaign_id.trim().is_empty() && !row.campaign_name.trim().is_empty()
            }),
            severity: "high".to_string(),
            observed: "campaign rows contain id/name".to_string(),
            expected: "all campaign rows include stable id and name".to_string(),
        },
        QualityCheckV1 {
            code: "schema_keyword_required_fields".to_string(),
            passed: report.keyword_data.iter().all(|row| {
                !row.keyword_id.trim().is_empty() && !row.keyword_text.trim().is_empty()
            }),
            severity: "high".to_string(),
            observed: "keyword rows contain id/text".to_string(),
            expected: "all keyword rows include criterion id and keyword text".to_string(),
        },
        QualityCheckV1 {
            code: "schema_report_metrics_finite".to_string(),
            passed: report.total_metrics.cost.is_finite()
                && report.total_metrics.conversions.is_finite()
                && report.total_metrics.conversions_value.is_finite()
                && report.total_metrics.ctr.is_finite()
                && report.total_metrics.cpc.is_finite()
                && report.total_metrics.cpa.is_finite()
                && report.total_metrics.roas.is_finite(),
            severity: "high".to_string(),
            observed: "all report metrics are finite".to_string(),
            expected: "no NaN or +/-inf values".to_string(),
        },
        QualityCheckV1 {
            code: "schema_ingest_contract_version_present".to_string(),
            passed: provenance.iter().all(|item| {
                item.validated_contract_version
                    .as_ref()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            }),
            severity: "high".to_string(),
            observed: "connector provenance declares validated contract version".to_string(),
            expected: "every provenance entry has validated_contract_version".to_string(),
        },
    ];
    let reconciliation_code = "identity_campaign_rollup_reconciliation";
    let reconciliation_tol = reconciliation_policy.tolerance_for(reconciliation_code);
    let spend_delta = (sum_campaign_spend - report.total_metrics.cost).abs();
    let revenue_delta = (sum_campaign_revenue - report.total_metrics.conversions_value).abs();
    let (reconciliation_passed, reconciliation_severity, reconciliation_expected) =
        if let Some(tol) = reconciliation_tol {
            let epsilon = tol.max_abs_delta.unwrap_or(0.0).max(0.0);
            (
                spend_delta <= epsilon && revenue_delta <= epsilon,
                tol.severity.clone(),
                format!("abs(delta) <= {:.4}", epsilon),
            )
        } else {
            (
                false,
                "high".to_string(),
                "reconciliation policy must define absolute tolerance".to_string(),
            )
        };

    let identity_resolution_checks = vec![
        QualityCheckV1 {
            code: "identity_ad_group_linked_to_campaign".to_string(),
            passed: ad_group_coverage_ratio >= MIN_IDENTITY_COVERAGE_RATIO,
            severity: "high".to_string(),
            observed: format!("coverage={:.3}", ad_group_coverage_ratio),
            expected: format!("coverage >= {:.2}", MIN_IDENTITY_COVERAGE_RATIO),
        },
        QualityCheckV1 {
            code: "identity_keyword_linked_to_ad_group".to_string(),
            passed: keyword_coverage_ratio >= MIN_IDENTITY_COVERAGE_RATIO,
            severity: "high".to_string(),
            observed: format!("coverage={:.3}", keyword_coverage_ratio),
            expected: format!("coverage >= {:.2}", MIN_IDENTITY_COVERAGE_RATIO),
        },
        QualityCheckV1 {
            code: reconciliation_code.to_string(),
            passed: reconciliation_passed,
            severity: reconciliation_severity,
            observed: format!(
                "campaign_spend={:.4}, campaign_rev={:.4}, total_spend={:.4}, total_rev={:.4}",
                sum_campaign_spend,
                sum_campaign_revenue,
                report.total_metrics.cost,
                report.total_metrics.conversions_value
            ),
            expected: reconciliation_expected,
        },
    ];
    let start_utc = start
        .and_hms_opt(0, 0, 0)
        .expect("start date should support midnight")
        .and_utc();
    let end_utc = end
        .and_hms_opt(23, 0, 0)
        .expect("end date should support hour boundary")
        .and_utc();
    let mut freshness_sla_checks = Vec::new();
    for item in provenance {
        let maybe_threshold = freshness_policy.threshold_for(&item.source_system);
        let (freshness_passed, freshness_severity, freshness_expected) =
            if let Some(threshold) = maybe_threshold {
                (
                    item.freshness_minutes <= threshold.max_freshness_minutes,
                    threshold.severity.clone(),
                    format!("freshness <= {} minutes", threshold.max_freshness_minutes),
                )
            } else {
                (
                    false,
                    "high".to_string(),
                    "source_system must have an explicit freshness SLA threshold".to_string(),
                )
            };
        freshness_sla_checks.push(QualityCheckV1 {
            code: format!("freshness_sla_{}", item.source_system),
            passed: freshness_passed,
            severity: freshness_severity.clone(),
            observed: format!("freshness={}m", item.freshness_minutes),
            expected: freshness_expected,
        });

        let (
            completeness_passed,
            completeness_severity,
            completeness_observed,
            completeness_expected,
        ) = if let Some(threshold) = maybe_threshold {
            let tz_result = threshold.timezone.parse::<chrono_tz::Tz>();
            if let Ok(timezone) = tz_result {
                let observed = observed_units_by_source
                    .get(&item.source_system)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let window = window_completeness(
                    start_utc,
                    end_utc,
                    granularity_by_source
                        .get(&item.source_system)
                        .copied()
                        .unwrap_or(TimeGranularity::Day),
                    timezone,
                    observed,
                );
                (
                    window.completeness_ratio >= threshold.min_completeness_ratio,
                    threshold.severity.clone(),
                    format!(
                        "observed={}/{} ratio={:.3}",
                        window.observed_units, window.expected_units, window.completeness_ratio
                    ),
                    format!("ratio >= {:.2}", threshold.min_completeness_ratio),
                )
            } else {
                (
                    false,
                    "high".to_string(),
                    format!("invalid_timezone={}", threshold.timezone),
                    "timezone must parse to a valid IANA name".to_string(),
                )
            }
        } else {
            (
                false,
                "high".to_string(),
                "no observed source window policy".to_string(),
                "source_system must have completeness threshold policy".to_string(),
            )
        };
        freshness_sla_checks.push(QualityCheckV1 {
            code: format!("completeness_sla_{}", item.source_system),
            passed: completeness_passed,
            severity: completeness_severity,
            observed: completeness_observed,
            expected: completeness_expected,
        });
        if let Some(provided_sources) = provided_observation_sources {
            freshness_sla_checks.push(QualityCheckV1 {
                code: format!("source_window_observation_present_{}", item.source_system),
                passed: provided_sources.contains(&item.source_system),
                severity: "medium".to_string(),
                observed: format!(
                    "provided_sources={}",
                    provided_sources
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                expected: "source observation provided for each provenance source".to_string(),
            });
        }
    }

    let is_healthy = schema_drift_checks
        .iter()
        .chain(identity_resolution_checks.iter())
        .chain(freshness_sla_checks.iter())
        .chain(cross_source_checks.iter())
        .chain(budget_checks.iter())
        .all(|c| c.passed);

    AnalyticsQualityControlsV1 {
        schema_drift_checks,
        identity_resolution_checks,
        freshness_sla_checks,
        cross_source_checks,
        budget_checks,
        is_healthy,
    }
}

fn build_data_quality_summary(
    quality_controls: &AnalyticsQualityControlsV1,
) -> DataQualitySummaryV1 {
    let completeness_ratio =
        pass_ratio_filtered(&quality_controls.freshness_sla_checks, "completeness_sla_");
    let identity_join_coverage_ratio = pass_ratio(&quality_controls.identity_resolution_checks);
    let freshness_pass_ratio =
        pass_ratio_filtered(&quality_controls.freshness_sla_checks, "freshness_sla_");
    let reconciliation_pass_ratio = quality_controls
        .identity_resolution_checks
        .iter()
        .find(|check| check.code == "identity_campaign_rollup_reconciliation")
        .map(|check| if check.passed { 1.0 } else { 0.0 })
        .unwrap_or(0.0);
    let cross_source_pass_ratio = pass_ratio(&quality_controls.cross_source_checks);
    let budget_pass_ratio = pass_ratio(&quality_controls.budget_checks);

    let quality_score = round4(
        completeness_ratio * 0.25
            + identity_join_coverage_ratio * 0.20
            + freshness_pass_ratio * 0.15
            + reconciliation_pass_ratio * 0.15
            + cross_source_pass_ratio * 0.15
            + budget_pass_ratio * 0.10,
    );
    assert!((0.0..=1.0).contains(&quality_score));

    DataQualitySummaryV1 {
        completeness_ratio,
        identity_join_coverage_ratio,
        freshness_pass_ratio,
        reconciliation_pass_ratio,
        cross_source_pass_ratio,
        budget_pass_ratio,
        quality_score,
    }
}

fn build_budget_checks(budget: &BudgetSummaryV1) -> Vec<QualityCheckV1> {
    let blocked_events = budget
        .events
        .iter()
        .filter(|event| event.outcome.eq_ignore_ascii_case("blocked"))
        .count();
    vec![
        QualityCheckV1 {
            code: "budget_no_blocked_spend".to_string(),
            passed: blocked_events == 0,
            severity: "high".to_string(),
            observed: format!("blocked_events={blocked_events}"),
            expected: "blocked_events=0".to_string(),
        },
        QualityCheckV1 {
            code: "budget_retrieval_within_cap".to_string(),
            passed: budget.actuals.retrieval_units <= budget.envelope.max_retrieval_units,
            severity: "high".to_string(),
            observed: format!(
                "{}/{}",
                budget.actuals.retrieval_units, budget.envelope.max_retrieval_units
            ),
            expected: "actual <= cap".to_string(),
        },
        QualityCheckV1 {
            code: "budget_analysis_within_cap".to_string(),
            passed: budget.actuals.analysis_units <= budget.envelope.max_analysis_units,
            severity: "high".to_string(),
            observed: format!(
                "{}/{}",
                budget.actuals.analysis_units, budget.envelope.max_analysis_units
            ),
            expected: "actual <= cap".to_string(),
        },
        QualityCheckV1 {
            code: "budget_total_cost_within_cap".to_string(),
            passed: budget.actuals.total_cost_micros <= budget.envelope.max_total_cost_micros,
            severity: "high".to_string(),
            observed: format!(
                "{}/{}",
                budget.actuals.total_cost_micros, budget.envelope.max_total_cost_micros
            ),
            expected: "actual <= cap".to_string(),
        },
        QualityCheckV1 {
            code: "budget_daily_hard_cap_within_limit".to_string(),
            passed: budget.daily_spent_after_micros <= budget.hard_daily_cap_micros,
            severity: "high".to_string(),
            observed: format!(
                "{}/{}",
                budget.daily_spent_after_micros, budget.hard_daily_cap_micros
            ),
            expected: "daily_spent_after <= hard_daily_cap".to_string(),
        },
    ]
}

fn pass_ratio(checks: &[QualityCheckV1]) -> f64 {
    if checks.is_empty() {
        return 1.0;
    }
    let passed = checks.iter().filter(|check| check.passed).count() as f64;
    round4((passed / checks.len() as f64).clamp(0.0, 1.0))
}

fn pass_ratio_filtered(checks: &[QualityCheckV1], code_prefix: &str) -> f64 {
    let mut total = 0usize;
    let mut passed = 0usize;
    for check in checks
        .iter()
        .filter(|check| check.code.starts_with(code_prefix))
    {
        total += 1;
        if check.passed {
            passed += 1;
        }
    }
    if total == 0 {
        return 1.0;
    }
    round4((passed as f64 / total as f64).clamp(0.0, 1.0))
}

fn build_operator_summary(
    report: &AnalyticsReport,
    evidence: &[EvidenceItem],
) -> OperatorSummaryV1 {
    let evidence_ids = evidence
        .iter()
        .map(|item| item.evidence_id.clone())
        .collect::<Vec<_>>();
    OperatorSummaryV1 {
        attribution_narratives: vec![
            KpiAttributionNarrativeV1 {
                kpi: "ctr".to_string(),
                narrative: format!(
                    "CTR is {:.2}% from {} clicks on {} impressions.",
                    report.total_metrics.ctr,
                    report.total_metrics.clicks,
                    report.total_metrics.impressions
                ),
                evidence_ids: evidence_ids.clone(),
                confidence_label: "medium".to_string(),
            },
            KpiAttributionNarrativeV1 {
                kpi: "roas".to_string(),
                narrative: format!(
                    "ROAS is {:.2} with conversion value {:.2} against cost {:.2}.",
                    report.total_metrics.roas,
                    report.total_metrics.conversions_value,
                    report.total_metrics.cost
                ),
                evidence_ids,
                confidence_label: "medium".to_string(),
            },
        ],
    }
}

struct IngestNormalizationAudit {
    notes: Vec<IngestCleaningNoteV1>,
    note_counts: BTreeMap<String, u32>,
}

fn collect_ingest_cleaning_notes(
    ga4_events: &[Ga4EventRawV1],
    wix_orders: &[WixOrderRawV1],
) -> Result<IngestNormalizationAudit, AnalyticsError> {
    let mut notes = Vec::new();
    let mut note_counts = BTreeMap::new();

    for event in ga4_events {
        let raw_json = serde_json::to_string(event).map_err(|err| {
            AnalyticsError::internal(
                "ingest_serialization_failed",
                format!("failed to serialize GA4 raw event: {err}"),
            )
        })?;
        let parsed = parse_ga4_event(&raw_json).map_err(|err| {
            AnalyticsError::new(
                "ingest_validation_failed",
                format!("failed to parse/normalize ga4 event: {}", err.reason),
                vec![err.field],
                None,
            )
        })?;
        increment_source_count(&mut note_counts, "ga4", parsed.notes.len() as u32);
        notes.extend(map_ingest_notes("ga4", &parsed.notes));
    }

    for order in wix_orders {
        let raw_json = serde_json::to_string(order).map_err(|err| {
            AnalyticsError::internal(
                "ingest_serialization_failed",
                format!("failed to serialize Wix raw order: {err}"),
            )
        })?;
        let parsed = parse_wix_order(&raw_json).map_err(|err| {
            AnalyticsError::new(
                "ingest_validation_failed",
                format!("failed to parse/normalize wix order: {}", err.reason),
                vec![err.field],
                None,
            )
        })?;
        increment_source_count(
            &mut note_counts,
            "wix_storefront",
            parsed.notes.len() as u32,
        );
        notes.extend(map_ingest_notes("wix_storefront", &parsed.notes));
    }

    Ok(IngestNormalizationAudit { notes, note_counts })
}

fn map_ingest_notes(source_system: &str, notes: &[CleaningNote]) -> Vec<IngestCleaningNoteV1> {
    notes
        .iter()
        .map(|note| IngestCleaningNoteV1 {
            source_system: source_system.to_string(),
            rule_id: note.rule_id.clone(),
            severity: format!("{:?}", note.severity).to_lowercase(),
            affected_field: note.affected_field.clone(),
            raw_value: note.raw_value.clone(),
            clean_value: note.clean_value.clone(),
            message: note.message.clone(),
        })
        .collect()
}

fn increment_source_count(counts: &mut BTreeMap<String, u32>, source_system: &str, count: u32) {
    let entry = counts.entry(source_system.to_string()).or_insert(0);
    *entry = entry.saturating_add(count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    #[tokio::test]
    async fn same_request_and_seed_is_byte_stable() {
        let svc = DefaultMarketAnalysisService::new();
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-03".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(42),
            profile_id: "small".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let mut a = svc
            .run_mock_analysis(req.clone())
            .await
            .expect("run should succeed");
        let mut b = svc
            .run_mock_analysis(req)
            .await
            .expect("run should succeed");
        for artifact in [&mut a, &mut b] {
            for check in &mut artifact.quality_controls.budget_checks {
                if check.code == "budget_daily_hard_cap_within_limit" {
                    check.observed = format!("0/{}", artifact.budget.hard_daily_cap_micros);
                }
            }
        }
        a.budget.daily_spent_before_micros = 0;
        a.budget.daily_spent_after_micros = 0;
        b.budget.daily_spent_before_micros = 0;
        b.budget.daily_spent_after_micros = 0;
        let sa = serde_json::to_string(&a).expect("serialize");
        let sb = serde_json::to_string(&b).expect("serialize");
        assert_eq!(sa, sb);
    }

    #[tokio::test]
    async fn derived_seed_is_stable_when_seed_omitted() {
        let svc = DefaultMarketAnalysisService::new();
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: Some("Summer".to_string()),
            ad_group_filter: None,
            seed: None,
            profile_id: "small".to_string(),
            include_narratives: false,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };
        let first = svc
            .run_mock_analysis(req.clone())
            .await
            .expect("run should succeed");
        let second = svc
            .run_mock_analysis(req)
            .await
            .expect("run should succeed");
        assert_eq!(first.metadata.seed, second.metadata.seed);
        assert_eq!(first.metadata.run_id, second.metadata.run_id);
    }

    #[tokio::test]
    async fn property_impressions_are_always_gte_clicks() {
        let svc = DefaultMarketAnalysisService::new();
        for seed in 1..=16_u64 {
            let req = MockAnalyticsRequestV1 {
                start_date: "2026-01-01".to_string(),
                end_date: "2026-01-03".to_string(),
                campaign_filter: None,
                ad_group_filter: None,
                seed: Some(seed),
                profile_id: "small".to_string(),
                include_narratives: true,
                source_window_observations: Vec::new(),
                budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
            };
            let artifact = svc
                .run_mock_analysis(req)
                .await
                .expect("run should succeed");
            assert!(
                artifact.report.total_metrics.impressions >= artifact.report.total_metrics.clicks
            );
            assert!((0.0..=1.0).contains(&artifact.data_quality.quality_score));
            assert!((0.0..=1.0).contains(&artifact.data_quality.completeness_ratio));
            assert!((0.0..=1.0).contains(&artifact.data_quality.identity_join_coverage_ratio));
            assert!((0.0..=1.0).contains(&artifact.data_quality.cross_source_pass_ratio));
            assert!(
                artifact.budget.estimated.retrieval_units
                    >= artifact.budget.actuals.retrieval_units
            );
            assert!(
                artifact.budget.estimated.analysis_units >= artifact.budget.actuals.analysis_units
            );
            assert!(
                artifact.budget.estimated.total_cost_micros
                    >= artifact.budget.actuals.total_cost_micros
            );
        }
    }

    #[tokio::test]
    async fn source_window_observations_can_drive_completeness_fail_closed() {
        let svc = DefaultMarketAnalysisService::new();
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(99),
            profile_id: "small".to_string(),
            include_narratives: false,
            source_window_observations: vec![
                super::super::contracts::SourceWindowObservationV1 {
                    source_system: "google_ads".to_string(),
                    granularity: super::super::contracts::SourceWindowGranularityV1::Day,
                    observed_timestamps_utc: Vec::new(),
                },
                super::super::contracts::SourceWindowObservationV1 {
                    source_system: "ga4".to_string(),
                    granularity: super::super::contracts::SourceWindowGranularityV1::Day,
                    observed_timestamps_utc: Vec::new(),
                },
                super::super::contracts::SourceWindowObservationV1 {
                    source_system: "wix_storefront".to_string(),
                    granularity: super::super::contracts::SourceWindowGranularityV1::Day,
                    observed_timestamps_utc: Vec::new(),
                },
            ],
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let err = svc
            .run_mock_analysis(req)
            .await
            .expect_err("artifact should fail closed when completeness SLAs are unmet");
        assert_eq!(err.code, "artifact_invariant_violation");
    }

    #[test]
    fn data_quality_summary_uses_completeness_checks_not_schema_checks() {
        let controls = AnalyticsQualityControlsV1 {
            schema_drift_checks: vec![QualityCheckV1 {
                code: "schema_ok".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            identity_resolution_checks: vec![QualityCheckV1 {
                code: "identity_campaign_rollup_reconciliation".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            freshness_sla_checks: vec![
                QualityCheckV1 {
                    code: "freshness_sla_ga4".to_string(),
                    passed: true,
                    severity: "high".to_string(),
                    observed: "ok".to_string(),
                    expected: "ok".to_string(),
                },
                QualityCheckV1 {
                    code: "completeness_sla_ga4".to_string(),
                    passed: false,
                    severity: "high".to_string(),
                    observed: "missing".to_string(),
                    expected: "ratio >= 0.98".to_string(),
                },
            ],
            cross_source_checks: vec![QualityCheckV1 {
                code: "cross_source_ok".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            budget_checks: vec![QualityCheckV1 {
                code: "budget_ok".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            is_healthy: false,
        };

        let summary = build_data_quality_summary(&controls);
        assert_eq!(summary.completeness_ratio, 0.0);
        assert_eq!(summary.freshness_pass_ratio, 1.0);
        assert_eq!(summary.reconciliation_pass_ratio, 1.0);
        assert_eq!(summary.quality_score, 0.75);
    }

    #[test]
    fn ingest_cleaning_audit_includes_wix_notes() {
        let ga4 = vec![Ga4EventRawV1 {
            event_name: " purchase ".to_string(),
            event_timestamp_utc: "2026-02-01T12:00:00Z".to_string(),
            user_pseudo_id: " user_1 ".to_string(),
            session_id: Some("session-1".to_string()),
            campaign: Some("spring_launch".to_string()),
        }];
        let wix = vec![WixOrderRawV1 {
            order_id: " wix-1 ".to_string(),
            placed_at_utc: "2026-02-01T18:00:00Z".to_string(),
            gross_amount: "123.45".to_string(),
            currency: " usd ".to_string(),
        }];

        let audit = collect_ingest_cleaning_notes(&ga4, &wix).expect("ingest audit");
        assert!(!audit.notes.is_empty());
        assert_eq!(audit.note_counts.get("ga4").copied().unwrap_or(0), 2);
        assert_eq!(
            audit
                .note_counts
                .get("wix_storefront")
                .copied()
                .unwrap_or(0),
            2
        );
        assert!(audit
            .notes
            .iter()
            .any(|note| note.source_system == "wix_storefront"));
    }

    struct FailingConnector;

    #[async_trait]
    impl super::super::connector_v2::AnalyticsConnectorContractV2 for FailingConnector {
        fn capabilities(&self) -> super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
            super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
                connector_id: "failing_connector".to_string(),
                contract_version: "analytics_connector_contract.v2".to_string(),
                supports_healthcheck: true,
                sources: Vec::new(),
            }
        }

        async fn healthcheck(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
        ) -> Result<super::super::connector_v2::ConnectorHealthStatusV1, AnalyticsError> {
            Ok(super::super::connector_v2::ConnectorHealthStatusV1 {
                connector_id: "failing_connector".to_string(),
                ok: true,
                mode: "simulated".to_string(),
                source_status: Vec::new(),
                blocking_reasons: Vec::new(),
                warning_reasons: Vec::new(),
            })
        }

        async fn fetch_ga4_events(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<Ga4EventRawV1>, AnalyticsError> {
            Err(AnalyticsError::internal(
                "connector_fetch_failed",
                "ga4 unavailable",
            ))
        }

        async fn fetch_google_ads_rows(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _request: &MockAnalyticsRequestV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
            Err(AnalyticsError::internal(
                "connector_fetch_failed",
                "google ads unavailable",
            ))
        }

        async fn fetch_wix_orders(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::ingest::WixOrderRawV1>, AnalyticsError> {
            Err(AnalyticsError::internal(
                "connector_fetch_failed",
                "wix unavailable",
            ))
        }

        async fn fetch_wix_sessions(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::connector_v2::WixSessionRawV1>, AnalyticsError> {
            Err(AnalyticsError::internal(
                "connector_fetch_failed",
                "wix unavailable",
            ))
        }
    }

    #[tokio::test]
    async fn simulated_mode_falls_back_when_connector_fetch_fails() {
        let svc = DefaultMarketAnalysisService::with_connector(Arc::new(FailingConnector));
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(7),
            profile_id: "small".to_string(),
            include_narratives: false,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let artifact = svc
            .run_mock_analysis(req)
            .await
            .expect("simulated mode should fallback to local deterministic generator");
        assert!(!artifact.report.campaign_data.is_empty());
        assert_eq!(artifact.metadata.connector_id, "failing_connector");
    }

    #[test]
    fn data_quality_summary_rejects_out_of_bound_ratio_via_artifact_validator() {
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
                source_window_observations: Vec::new(),
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
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["sim".to_string()],
            provenance: vec![SourceProvenance {
                connector_id: "simulated".to_string(),
                source_class: SourceClassLabel::Simulated,
                source_system: "mock".to_string(),
                collected_at_utc: "now".to_string(),
                freshness_minutes: 0,
                validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
                rejected_rows_count: 0,
                cleaning_note_count: 0,
            }],
            ingest_cleaning_notes: Vec::new(),
            validation: super::super::contracts::AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: DataQualitySummaryV1 {
                quality_score: 1.4,
                ..Default::default()
            },
            freshness_policy: FreshnessSlaPolicyV1::default(),
            reconciliation_policy: ReconciliationPolicyV1::default(),
            budget: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };
        artifact.report.total_metrics.impressions = 10;
        artifact.report.total_metrics.clicks = 5;
        let validation = validate_mock_analytics_artifact_v1(&artifact);
        assert!(!validation.is_valid);
    }
}
