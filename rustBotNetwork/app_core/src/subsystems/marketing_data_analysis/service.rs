use super::analytics_config::{
    validate_analytics_connector_config_v1, AnalyticsConnectorConfigV1, AnalyticsConnectorModeV1,
    AnalyticsSourceTopologyV1,
};
use super::budget::{build_budget_plan, enforce_daily_hard_cap, BudgetCategory, BudgetGuard};
use super::connector_v2::{
    generate_simulated_ga4_events, generate_simulated_google_ads_rows,
    generate_simulated_wix_orders, AnalyticsConnectorContractV2, SimulatedAnalyticsConnectorV2,
};
use super::contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1, BudgetSummaryV1,
    DailyRevenuePointV1, DataQualitySummaryV1, EvidenceItem, FreshnessSlaPolicyV1, GuidanceItem,
    IngestCleaningNoteV1, KpiAttributionNarrativeV1, MockAnalyticsArtifactV1,
    MockAnalyticsRequestV1, OperatorSummaryV1, QualityCheckApplicabilityV1, QualityCheckV1,
    ReconciliationPolicyV1, SourceCoverageV1, SourceWindowGranularityV1,
    MOCK_ANALYTICS_SCHEMA_VERSION_V1,
};
use super::ga4_sessions::rollup_ga4_sessions_v1;
use super::ingest::{
    parse_ga4_event, parse_google_ads_row, parse_wix_order, window_completeness, CleaningNote,
    Ga4EventRawV1, GoogleAdsRowRawV1, TimeGranularity, WixOrderRawV1,
};
use super::validators::{validate_mock_analytics_artifact_v1, validate_mock_analytics_request_v1};
use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupReportRow, AdGroupResource, AnalyticsReport,
    CampaignReportRow, CampaignResource, GoogleAdsRow, KeywordReportRow, MetricsData,
    ReportMetrics, SourceClassLabel, SourceProvenance,
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
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
        let ga4_unified_topology =
            self.connector_config.source_topology == AnalyticsSourceTopologyV1::Ga4Unified;
        let ads_enabled = self.connector_config.google_ads.enabled && !ga4_unified_topology;
        let ga4_enabled = self.connector_config.ga4.enabled;
        let wix_enabled = self.connector_config.wix.enabled && !ga4_unified_topology;

        let rows = if ads_enabled {
            match self
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
                Ok(rows) => rows,
                Err(_) if self.connector_config.mode.is_simulated() => {
                    generate_simulated_google_ads_rows(
                        &request,
                        start,
                        budget_plan.effective_end,
                        seed,
                    )
                }
                Err(err) => return Err(err),
            }
        } else {
            Vec::new()
        };
        let ga4_events = if ga4_enabled {
            match self
                .connector
                .fetch_ga4_events(
                    &self.connector_config,
                    start,
                    budget_plan.effective_end,
                    seed,
                )
                .await
            {
                Ok(events) => events,
                Err(_) if self.connector_config.mode.is_simulated() => {
                    generate_simulated_ga4_events(start, budget_plan.effective_end, seed)
                }
                Err(err) => return Err(err),
            }
        } else {
            Vec::new()
        };
        let wix_orders = if wix_enabled {
            match self
                .connector
                .fetch_wix_orders(
                    &self.connector_config,
                    start,
                    budget_plan.effective_end,
                    seed,
                )
                .await
            {
                Ok(orders) => orders,
                Err(_) if self.connector_config.mode.is_simulated() => {
                    generate_simulated_wix_orders(start, budget_plan.effective_end, seed)
                }
                Err(err) => return Err(err),
            }
        } else {
            Vec::new()
        };
        let wix_sessions = if wix_enabled {
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
                Ok(sessions) => sessions,
                Err(_) if self.connector_config.mode.is_simulated() => Vec::new(),
                Err(err) => return Err(err),
            }
        } else {
            Vec::new()
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

        let google_ads_row_count = rows.len() as u64;
        let ga4_event_count = ga4_events.len() as u64;
        let wix_row_count = wix_orders.len().max(wix_sessions.len()) as u64;
        let source_coverage = build_source_coverage(
            &self.connector_config,
            google_ads_row_count,
            ga4_event_count,
            wix_row_count,
        );

        let google_ads_raw_rows = google_ads_rows_to_raw_v1(&rows)?;
        let mut report_from_ga4 = false;
        let mut report = rows_to_report(rows, &request, start, budget_plan.effective_end);
        if report.campaign_data.is_empty() && !ga4_events.is_empty() {
            report = ga4_events_to_report(&ga4_events, &request, start, budget_plan.effective_end);
            report_from_ga4 = true;
        }
        let daily_revenue_series = build_daily_revenue_series(
            report_from_ga4,
            &ga4_events,
            &google_ads_raw_rows,
            start,
            budget_plan.effective_end,
        );
        let ga4_session_rollups = rollup_ga4_sessions_v1(&ga4_events);
        let ingest_audit =
            collect_ingest_cleaning_notes(&ga4_events, &google_ads_raw_rows, &wix_orders)?;
        let freshness_policy = FreshnessSlaPolicyV1::default();
        let reconciliation_policy = ReconciliationPolicyV1::default();
        let connector_id = self.connector.capabilities().connector_id;
        let provenance = build_provenance(
            &connector_id,
            &self.connector_config.mode,
            &source_coverage,
            &ingest_audit.note_counts,
        );
        let (observed_units_by_source, granularity_by_source, provided_observation_sources) =
            resolve_source_window_inputs(
                &request,
                start,
                budget_plan.effective_end,
                &google_ads_raw_rows,
                &ga4_events,
                &wix_orders,
            )?;
        let cross_source_checks = build_cross_source_checks(
            &report,
            seed,
            &reconciliation_policy,
            &source_coverage,
            &self.connector_config.source_topology,
        );
        let source_class_label =
            if self.connector_config.mode == AnalyticsConnectorModeV1::ObservedReadOnly {
                "observed"
            } else {
                "simulated"
            };
        let no_data_window = source_coverage
            .iter()
            .filter(|item| item.enabled)
            .all(|item| item.row_count == 0);
        let (observed_evidence, inferred_guidance, mut uncertainty_notes) =
            build_evidence_and_guidance(
                &report,
                budget_plan.include_narratives,
                source_class_label,
                no_data_window,
            );
        if no_data_window {
            uncertainty_notes.push(
                "No observed rows were returned for the selected date window; artifact represents a valid zero-activity interval."
                    .to_string(),
            );
        }
        if ga4_unified_topology {
            uncertainty_notes.push(
                "source_topology=ga4_unified: Google Ads and Wix connectors are intentionally disabled; cross-source reconciliation checks are reported as not applicable."
                    .to_string(),
            );
        }
        if !ga4_events.is_empty() && ga4_session_rollups.is_empty() {
            uncertainty_notes.push(
                "GA4 session rollups are unavailable for this run; funnel and storefront panels require landing/session fields from the GA4 BigQuery export."
                    .to_string(),
            );
        }
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
            &ga4_events,
            &provenance,
            budget_checks,
            cross_source_checks,
            &freshness_policy,
            &reconciliation_policy,
            start,
            budget_plan.effective_end,
            &observed_units_by_source,
            &granularity_by_source,
            &source_coverage,
            provided_observation_sources.as_ref(),
        );
        if let Some(check) = quality_controls.schema_drift_checks.iter().find(|check| {
            check.code == "ga4_custom_purchase_ndp_overlap_rate"
                && check.applicability == QualityCheckApplicabilityV1::Applies
                && !check.passed
        }) {
            uncertainty_notes.push(format!(
                "Custom purchase stream `purchase_ndp` overlaps canonical `purchase` events; duplicate instrumentation remains active ({}) but is excluded from truth KPIs.",
                check.observed
            ));
        }
        if let Some(check) = quality_controls.schema_drift_checks.iter().find(|check| {
            check.code == "ga4_custom_purchase_ndp_orphan_rate"
                && check.applicability == QualityCheckApplicabilityV1::Applies
                && !check.passed
        }) {
            uncertainty_notes.push(format!(
                "Custom purchase stream `purchase_ndp` emitted rows without nearby canonical purchases ({}); investigate potential checkout undercount before treating revenue as complete.",
                check.observed
            ));
        }
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
            connector_attestation: Default::default(),
        };

        let mut artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request,
            metadata,
            report,
            daily_revenue_series,
            observed_evidence,
            inferred_guidance,
            uncertainty_notes,
            provenance,
            source_coverage,
            ga4_session_rollups,
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
            let failed_quality_checks = artifact
                .quality_controls
                .schema_drift_checks
                .iter()
                .chain(artifact.quality_controls.identity_resolution_checks.iter())
                .chain(artifact.quality_controls.freshness_sla_checks.iter())
                .chain(artifact.quality_controls.cross_source_checks.iter())
                .chain(artifact.quality_controls.budget_checks.iter())
                .filter(|check| !check.passed)
                .cloned()
                .collect::<Vec<_>>();
            return Err(AnalyticsError::new(
                "artifact_invariant_violation",
                "generated artifact failed invariant checks",
                Vec::new(),
                Some(serde_json::json!({
                    "validation": serde_json::to_value(&artifact.validation).unwrap_or_else(|_| serde_json::json!({
                        "is_valid": false,
                        "checks": []
                    })),
                    "quality_controls": artifact.quality_controls,
                    "failed_quality_checks": failed_quality_checks,
                    "data_quality": artifact.data_quality
                })),
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
            resource_name: "".to_string(),
            id: "".to_string(),
            name: "".to_string(),
            status: "UNKNOWN".to_string(),
        });
        let ad_group = row.ad_group.unwrap_or(AdGroupResource {
            resource_name: "".to_string(),
            id: "".to_string(),
            name: "".to_string(),
            status: "UNKNOWN".to_string(),
            campaign_resource_name: "".to_string(),
        });
        let criterion = row.ad_group_criterion.unwrap_or(AdGroupCriterionResource {
            resource_name: "".to_string(),
            criterion_id: "".to_string(),
            status: "UNKNOWN".to_string(),
            keyword: None,
            quality_score: None,
            ad_group_resource_name: "".to_string(),
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

        let keyword_entry = keywords.entry(criterion.criterion_id.clone()).or_insert((
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
                .map(|k| k.match_type.clone())
                .unwrap_or_else(|| "EXACT".to_string()),
            ReportMetrics::default(),
            criterion.quality_score,
        ));
        keyword_entry.0 = campaign.id.clone();
        keyword_entry.1 = ad_group.id.clone();
        keyword_entry.4 = sum_metrics(&keyword_entry.4, &line);
        keyword_entry.5 = criterion.quality_score;
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

fn ga4_events_to_report(
    ga4_events: &[Ga4EventRawV1],
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
) -> AnalyticsReport {
    let mut total = ReportMetrics::default();
    let mut campaigns: BTreeMap<String, ReportMetrics> = BTreeMap::new();
    let mut event_buckets: BTreeMap<(String, String), ReportMetrics> = BTreeMap::new();
    let mut canonical_purchase_seconds: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    for event in ga4_events {
        if !is_ga4_canonical_purchase_event(&event.event_name) {
            continue;
        }
        let Some(tx_id) = ga4_transaction_id(event) else {
            continue;
        };
        let Some(event_second) = ga4_event_epoch_seconds(event) else {
            continue;
        };
        let user = event.user_pseudo_id.trim().to_string();
        let session = ga4_session_key(event);
        canonical_purchase_seconds
            .entry((user, session))
            .or_default()
            .push(event_second);
        debug_assert!(!tx_id.trim().is_empty());
    }
    for seconds in canonical_purchase_seconds.values_mut() {
        seconds.sort_unstable();
    }
    let mut seen_purchase_transaction_ids = BTreeSet::new();
    let mut seen_custom_purchase_second_keys = BTreeSet::new();

    for event in ga4_events {
        let event_name = event.event_name.trim().to_string();
        if event_name.is_empty() {
            continue;
        }
        let event_name_lower = event_name.to_ascii_lowercase();
        let campaign = event
            .campaign
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "GA4 (Campaign Unavailable)".to_string());
        let count = ga4_event_count_hint(event);
        let mut include_in_kpi_rollup = true;
        let mut effective_count = count;
        let event_second = ga4_event_epoch_seconds(event);
        let session_key = ga4_session_key(event);

        if is_ga4_canonical_purchase_event(&event_name_lower) {
            if let Some(tx_id) = ga4_transaction_id(event) {
                if !seen_purchase_transaction_ids.insert(tx_id) {
                    include_in_kpi_rollup = false;
                } else {
                    effective_count = 1;
                }
            }
        } else if is_ga4_custom_purchase_event(&event_name_lower) {
            if let Some(second) = event_second {
                let second_key = (
                    event.user_pseudo_id.trim().to_string(),
                    session_key.clone(),
                    second,
                );
                if !seen_custom_purchase_second_keys.insert(second_key) {
                    // Track repeated emission signature even though custom purchase is excluded from KPI rollups.
                }
                if has_canonical_purchase_within_window(
                    &canonical_purchase_seconds,
                    event.user_pseudo_id.trim(),
                    &session_key,
                    second,
                    30,
                ) {
                    // Canonical purchase exists nearby; keep custom purchase event out of KPI rollups.
                }
            }
            // Never allow custom purchase without transaction_id to drive KPI totals.
            include_in_kpi_rollup = false;
        }
        if !include_in_kpi_rollup {
            continue;
        }

        let impressions = effective_count;
        let clicks = if is_ga4_click_event(&event_name) {
            effective_count
        } else {
            0
        };
        let conversions = if is_ga4_conversion_event(&event_name) {
            effective_count as f64
        } else {
            0.0
        };
        let conversions_value = if is_ga4_canonical_purchase_event(&event_name_lower) {
            ga4_purchase_revenue(event).unwrap_or(0.0)
        } else {
            0.0
        };
        let metrics = derived_metrics(impressions, clicks, 0.0, conversions, conversions_value);
        total = sum_metrics(&total, &metrics);

        let campaign_entry = campaigns.entry(campaign.clone()).or_default();
        *campaign_entry = sum_metrics(campaign_entry, &metrics);

        let bucket_entry = event_buckets.entry((campaign, event_name)).or_default();
        *bucket_entry = sum_metrics(bucket_entry, &metrics);
    }

    let campaign_data = campaigns
        .into_iter()
        .enumerate()
        .map(|(idx, (name, metrics))| CampaignReportRow {
            date: "".to_string(),
            campaign_id: format!("ga4_campaign_{:03}", idx + 1),
            campaign_name: name,
            campaign_status: "ENABLED".to_string(),
            metrics: round_metrics(metrics),
        })
        .collect::<Vec<_>>();

    let campaign_id_by_name = campaign_data
        .iter()
        .map(|row| (row.campaign_name.clone(), row.campaign_id.clone()))
        .collect::<BTreeMap<_, _>>();

    let ad_group_data = campaign_data
        .iter()
        .map(|row| AdGroupReportRow {
            date: "".to_string(),
            campaign_id: row.campaign_id.clone(),
            campaign_name: row.campaign_name.clone(),
            ad_group_id: format!("{}_aggregate", row.campaign_id),
            ad_group_name: "GA4 Aggregate".to_string(),
            ad_group_status: "ENABLED".to_string(),
            metrics: row.metrics.clone(),
        })
        .collect::<Vec<_>>();

    let keyword_data = event_buckets
        .into_iter()
        .enumerate()
        .map(|(idx, ((campaign_name, event_name), metrics))| {
            let campaign_id = campaign_id_by_name
                .get(&campaign_name)
                .cloned()
                .unwrap_or_else(|| "ga4_campaign_unmapped".to_string());
            let ad_group_id = format!("{}_aggregate", campaign_id);
            KeywordReportRow {
                date: "".to_string(),
                campaign_id: campaign_id.clone(),
                campaign_name,
                ad_group_id: ad_group_id.clone(),
                ad_group_name: "GA4 Aggregate".to_string(),
                keyword_id: format!("ga4_event_{:03}", idx + 1),
                keyword_text: event_name,
                match_type: "EXACT".to_string(),
                quality_score: None,
                metrics: round_metrics(metrics),
            }
        })
        .collect::<Vec<_>>();

    AnalyticsReport {
        report_name: format!(
            "GA4 Observed Report: {} to {}",
            request.start_date, request.end_date
        ),
        date_range: format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d")),
        total_metrics: round_metrics(total),
        campaign_data,
        ad_group_data,
        keyword_data,
    }
}

fn build_daily_revenue_series(
    report_from_ga4: bool,
    ga4_events: &[Ga4EventRawV1],
    google_ads_rows: &[GoogleAdsRowRawV1],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<DailyRevenuePointV1> {
    if report_from_ga4 && !ga4_events.is_empty() {
        return build_daily_revenue_series_from_ga4(ga4_events, start, end);
    }
    if !google_ads_rows.is_empty() {
        return build_daily_revenue_series_from_google_ads(google_ads_rows, start, end);
    }
    Vec::new()
}

fn build_daily_revenue_series_from_ga4(
    ga4_events: &[Ga4EventRawV1],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<DailyRevenuePointV1> {
    let mut by_day = seeded_daily_revenue_map(start, end);
    let mut seen_purchase_transaction_ids = BTreeSet::new();

    for event in ga4_events {
        if !is_ga4_canonical_purchase_event(&event.event_name) {
            continue;
        }
        let first_transaction_occurrence = if let Some(tx_id) = ga4_transaction_id(event) {
            if !seen_purchase_transaction_ids.insert(tx_id) {
                continue;
            }
            true
        } else {
            false
        };
        let Some(day) = ga4_event_date_utc(event) else {
            continue;
        };
        if day < start || day > end {
            continue;
        }
        let revenue = ga4_purchase_revenue(event).unwrap_or(0.0).max(0.0);
        let conversions = if first_transaction_occurrence {
            1.0
        } else {
            ga4_event_count_hint(event) as f64
        };
        if let Some((day_revenue, day_conversions)) = by_day.get_mut(&day) {
            *day_revenue += revenue;
            *day_conversions += conversions.max(0.0);
        }
    }

    by_day
        .into_iter()
        .map(|(date, (revenue, conversions))| DailyRevenuePointV1 {
            date: date.format("%Y-%m-%d").to_string(),
            revenue: round4(revenue.max(0.0)),
            conversions: round4(conversions.max(0.0)),
            source_system: "ga4".to_string(),
        })
        .collect()
}

fn build_daily_revenue_series_from_google_ads(
    rows: &[GoogleAdsRowRawV1],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<DailyRevenuePointV1> {
    let mut by_day = seeded_daily_revenue_map(start, end);
    for row in rows {
        let Ok(day) = NaiveDate::parse_from_str(row.date.trim(), "%Y-%m-%d") else {
            continue;
        };
        if day < start || day > end {
            continue;
        }
        let revenue = row.conversions_micros as f64 / 1_000_000.0;
        if let Some((day_revenue, _)) = by_day.get_mut(&day) {
            *day_revenue += revenue.max(0.0);
        }
    }

    by_day
        .into_iter()
        .map(|(date, (revenue, conversions))| DailyRevenuePointV1 {
            date: date.format("%Y-%m-%d").to_string(),
            revenue: round4(revenue.max(0.0)),
            conversions: round4(conversions.max(0.0)),
            source_system: "google_ads".to_string(),
        })
        .collect()
}

fn seeded_daily_revenue_map(start: NaiveDate, end: NaiveDate) -> BTreeMap<NaiveDate, (f64, f64)> {
    let mut out = BTreeMap::new();
    let mut day = start;
    while day <= end {
        out.insert(day, (0.0, 0.0));
        let Some(next_day) = day.checked_add_signed(chrono::Duration::days(1)) else {
            break;
        };
        day = next_day;
    }
    out
}

fn ga4_event_count_hint(event: &Ga4EventRawV1) -> u64 {
    event
        .session_id
        .as_ref()
        .and_then(|value| value.strip_prefix("ga4_count:"))
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|count| *count > 0)
        .unwrap_or(1)
}

fn ga4_dimension(event: &Ga4EventRawV1, key: &str) -> Option<String> {
    event
        .dimensions
        .get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn ga4_transaction_id(event: &Ga4EventRawV1) -> Option<String> {
    ga4_dimension(event, "transaction_id")
}

fn ga4_purchase_revenue(event: &Ga4EventRawV1) -> Option<f64> {
    ga4_dimension(event, "purchase_revenue")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or_else(|| {
            ga4_dimension(event, "purchase_revenue_in_usd")
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value >= 0.0)
        })
}

fn ga4_event_epoch_seconds(event: &Ga4EventRawV1) -> Option<i64> {
    ga4_dimension(event, "event_timestamp_micros")
        .and_then(|value| value.parse::<i64>().ok())
        .map(|micros| micros.div_euclid(1_000_000))
        .or_else(|| {
            DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim())
                .ok()
                .map(|value| value.timestamp())
        })
}

fn ga4_event_date_utc(event: &Ga4EventRawV1) -> Option<NaiveDate> {
    ga4_event_epoch_seconds(event)
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        .map(|timestamp| timestamp.date_naive())
        .or_else(|| {
            DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim())
                .ok()
                .map(|value| value.with_timezone(&Utc).date_naive())
        })
}

fn ga4_session_key(event: &Ga4EventRawV1) -> String {
    ga4_dimension(event, "ga_session_id")
        .or_else(|| {
            event
                .session_id
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn has_canonical_purchase_within_window(
    canonical_purchase_seconds: &BTreeMap<(String, String), Vec<i64>>,
    user_pseudo_id: &str,
    session_key: &str,
    event_second: i64,
    tolerance_seconds: i64,
) -> bool {
    canonical_purchase_seconds
        .get(&(user_pseudo_id.to_string(), session_key.to_string()))
        .map(|seconds| {
            seconds
                .iter()
                .any(|candidate| (candidate - event_second).abs() <= tolerance_seconds)
        })
        .unwrap_or(false)
}

fn is_ga4_canonical_purchase_event(event_name: &str) -> bool {
    event_name.trim().eq_ignore_ascii_case("purchase")
}

fn is_ga4_custom_purchase_event(event_name: &str) -> bool {
    event_name.trim().eq_ignore_ascii_case("purchase_ndp")
}

fn ga4_custom_purchase_value(event: &Ga4EventRawV1) -> Option<f64> {
    ga4_dimension(event, "value")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn ga4_custom_purchase_schema_stats(ga4_events: &[Ga4EventRawV1]) -> (usize, usize, usize) {
    let custom_purchase_events = ga4_events
        .iter()
        .filter(|event| is_ga4_custom_purchase_event(&event.event_name))
        .collect::<Vec<_>>();
    let total_rows = custom_purchase_events.len();
    let rows_with_transaction_id = custom_purchase_events
        .iter()
        .filter(|event| ga4_transaction_id(event).is_some())
        .count();
    let rows_with_value = custom_purchase_events
        .iter()
        .filter(|event| {
            ga4_purchase_revenue(event).is_some() || ga4_custom_purchase_value(event).is_some()
        })
        .count();
    (total_rows, rows_with_transaction_id, rows_with_value)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Ga4CustomPurchaseMatchStats {
    total_rows: usize,
    rows_with_canonical_purchase: usize,
    orphan_rows: usize,
    overlap_ratio: f64,
    orphan_ratio: f64,
}

fn ga4_custom_purchase_match_stats(
    ga4_events: &[Ga4EventRawV1],
    tolerance_seconds: i64,
) -> Ga4CustomPurchaseMatchStats {
    let mut canonical_purchase_seconds: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    for event in ga4_events {
        if !is_ga4_canonical_purchase_event(&event.event_name) {
            continue;
        }
        let Some(event_second) = ga4_event_epoch_seconds(event) else {
            continue;
        };
        let user = event.user_pseudo_id.trim().to_string();
        let session = ga4_session_key(event);
        canonical_purchase_seconds
            .entry((user, session))
            .or_default()
            .push(event_second);
    }
    for seconds in canonical_purchase_seconds.values_mut() {
        seconds.sort_unstable();
    }

    let mut stats = Ga4CustomPurchaseMatchStats::default();
    for event in ga4_events {
        if !is_ga4_custom_purchase_event(&event.event_name) {
            continue;
        }
        stats.total_rows += 1;
        let Some(event_second) = ga4_event_epoch_seconds(event) else {
            stats.orphan_rows += 1;
            continue;
        };
        if has_canonical_purchase_within_window(
            &canonical_purchase_seconds,
            event.user_pseudo_id.trim(),
            &ga4_session_key(event),
            event_second,
            tolerance_seconds,
        ) {
            stats.rows_with_canonical_purchase += 1;
        } else {
            stats.orphan_rows += 1;
        }
    }

    if stats.total_rows > 0 {
        stats.overlap_ratio = stats.rows_with_canonical_purchase as f64 / stats.total_rows as f64;
        stats.orphan_ratio = stats.orphan_rows as f64 / stats.total_rows as f64;
    }
    stats
}

fn is_ga4_click_event(event_name: &str) -> bool {
    let name = event_name.to_ascii_lowercase();
    name.contains("click") || name.contains("select") || name.contains("cta") || name == "outbound"
}

fn is_ga4_conversion_event(event_name: &str) -> bool {
    let name = event_name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "purchase"
            | "generate_lead"
            | "begin_checkout"
            | "add_to_cart"
            | "add_payment_info"
            | "subscribe"
            | "sign_up"
    )
}

fn ga4_duplicate_signature_stats(ga4_events: &[Ga4EventRawV1]) -> (usize, usize, f64) {
    if ga4_events.is_empty() {
        return (0, 0, 0.0);
    }
    let mut signature_counts: BTreeMap<(String, String, String, String, String), usize> =
        BTreeMap::new();
    for event in ga4_events {
        let event_name = event.event_name.trim().to_ascii_lowercase();
        let user_pseudo_id = event.user_pseudo_id.trim().to_string();
        let timestamp_key = event
            .dimensions
            .get("event_timestamp_micros")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| event.event_timestamp_utc.trim().to_string());
        let session_key = event
            .dimensions
            .get("ga_session_id")
            .map(|value| value.trim().to_string())
            .or_else(|| event.session_id.clone())
            .unwrap_or_default();
        let bundle_key = [
            event
                .dimensions
                .get("event_bundle_sequence_id")
                .map(|value| value.trim())
                .unwrap_or(""),
            event
                .dimensions
                .get("batch_event_index")
                .map(|value| value.trim())
                .unwrap_or(""),
            event
                .dimensions
                .get("event_server_timestamp_offset")
                .map(|value| value.trim())
                .unwrap_or(""),
        ]
        .join("|");
        let signature = (
            event_name,
            user_pseudo_id,
            timestamp_key,
            session_key,
            bundle_key,
        );
        *signature_counts.entry(signature).or_insert(0) += 1;
    }
    let duplicate_rows = signature_counts
        .values()
        .map(|count| count.saturating_sub(1))
        .sum::<usize>();
    let duplicate_ratio = duplicate_rows as f64 / ga4_events.len() as f64;
    (duplicate_rows, signature_counts.len(), duplicate_ratio)
}

fn ga4_event_second_key(event: &Ga4EventRawV1) -> String {
    if let Some(seconds) = event
        .dimensions
        .get("event_timestamp_micros")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
        .map(|micros| micros.div_euclid(1_000_000))
    {
        return seconds.to_string();
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim()) {
        return parsed.timestamp().to_string();
    }

    event.event_timestamp_utc.trim().to_string()
}

fn ga4_near_duplicate_second_stats(ga4_events: &[Ga4EventRawV1]) -> (usize, usize, f64) {
    if ga4_events.is_empty() {
        return (0, 0, 0.0);
    }
    let mut signature_counts: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    for event in ga4_events {
        let event_name = event.event_name.trim().to_ascii_lowercase();
        let user_pseudo_id = event.user_pseudo_id.trim().to_string();
        let second_key = ga4_event_second_key(event);
        let session_key = event
            .dimensions
            .get("ga_session_id")
            .map(|value| value.trim().to_string())
            .or_else(|| event.session_id.clone())
            .unwrap_or_default();
        let signature = (event_name, user_pseudo_id, session_key, second_key);
        *signature_counts.entry(signature).or_insert(0) += 1;
    }
    let duplicate_groups = signature_counts
        .values()
        .filter(|count| **count > 1)
        .count();
    let duplicate_rows = signature_counts
        .values()
        .map(|count| count.saturating_sub(1))
        .sum::<usize>();
    let duplicate_ratio = duplicate_rows as f64 / ga4_events.len() as f64;
    (duplicate_rows, duplicate_groups, duplicate_ratio)
}

fn google_ads_rows_to_raw_v1(
    rows: &[GoogleAdsRow],
) -> Result<Vec<GoogleAdsRowRawV1>, AnalyticsError> {
    rows.iter()
        .filter_map(|row| {
            row.metrics.as_ref().map(|metrics| {
                let campaign_id = row
                    .campaign
                    .as_ref()
                    .map(|item| item.id.clone())
                    .unwrap_or_default();
                let ad_group_id = row
                    .ad_group
                    .as_ref()
                    .map(|item| item.id.clone())
                    .unwrap_or_default();
                let date = row
                    .segments
                    .as_ref()
                    .and_then(|segments| segments.date.clone())
                    .unwrap_or_default();
                let conversion_value = metrics.conversions_value;
                let conversions_micros_float = conversion_value * 1_000_000.0;
                if !conversions_micros_float.is_finite() || conversions_micros_float < 0.0 {
                    return Err(AnalyticsError::new(
                        "ads_conversion_value_non_finite",
                        "google ads conversion value must be finite and non-negative",
                        vec!["rows[].metrics.conversions_value".to_string()],
                        None,
                    ));
                }
                if conversions_micros_float > u64::MAX as f64 {
                    return Err(AnalyticsError::new(
                        "ads_conversion_value_overflow",
                        "google ads conversion value exceeds representable micros range",
                        vec!["rows[].metrics.conversions_value".to_string()],
                        None,
                    ));
                }
                Ok(GoogleAdsRowRawV1 {
                    campaign_id,
                    ad_group_id,
                    date,
                    impressions: metrics.impressions,
                    clicks: metrics.clicks,
                    cost_micros: metrics.cost_micros,
                    conversions_micros: conversions_micros_float.round() as u64,
                    currency: "USD".to_string(),
                })
            })
        })
        .collect()
}

fn from_metrics_data(data: &MetricsData) -> ReportMetrics {
    let cost = data.cost_micros as f64 / 1_000_000.0;
    derived_metrics(
        data.impressions,
        data.clicks,
        cost,
        data.conversions,
        data.conversions_value,
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
    source_class: &str,
    no_data_window: bool,
) -> (Vec<EvidenceItem>, Vec<GuidanceItem>, Vec<String>) {
    let mut evidence = Vec::new();
    evidence.push(EvidenceItem {
        evidence_id: "ev_total_impressions".to_string(),
        label: "Total Impressions".to_string(),
        value: report.total_metrics.impressions.to_string(),
        source_class: source_class.to_string(),
        metric_key: Some("impressions".to_string()),
        observed_window: Some(report.date_range.clone()),
        comparator_value: None,
        notes: vec!["Aggregated across enabled sources for selected date window.".to_string()],
    });
    evidence.push(EvidenceItem {
        evidence_id: "ev_total_clicks".to_string(),
        label: "Total Clicks".to_string(),
        value: report.total_metrics.clicks.to_string(),
        source_class: source_class.to_string(),
        metric_key: Some("clicks".to_string()),
        observed_window: Some(report.date_range.clone()),
        comparator_value: None,
        notes: vec!["Includes all enabled source rows after request filters.".to_string()],
    });

    let mut guidance = Vec::new();
    if include_narratives && !no_data_window {
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

    let uncertainty = if source_class == "simulated" {
        vec![
            "Dataset is simulated and intended for tool integration validation only.".to_string(),
            "Attribution assumptions are simplified for deterministic replay.".to_string(),
        ]
    } else if no_data_window {
        vec![
            "Observed mode returned zero rows in the selected window; no directional guidance generated."
                .to_string(),
            "Check date range, property activity, and source enablement before drawing conclusions."
                .to_string(),
        ]
    } else {
        vec![
            "Observed mode is read-only and reflects connector-available source data.".to_string(),
            "Attribution and joins remain bounded by enabled source coverage.".to_string(),
        ]
    };
    (evidence, guidance, uncertainty)
}

fn build_source_coverage(
    config: &AnalyticsConnectorConfigV1,
    google_ads_rows: u64,
    ga4_events: u64,
    wix_rows: u64,
) -> Vec<SourceCoverageV1> {
    let ga4_unified_topology = config.source_topology == AnalyticsSourceTopologyV1::Ga4Unified;
    let google_ads_enabled = config.google_ads.enabled && !ga4_unified_topology;
    let wix_enabled = config.wix.enabled && !ga4_unified_topology;
    vec![
        SourceCoverageV1 {
            source_system: "google_ads".to_string(),
            enabled: google_ads_enabled,
            observed: google_ads_enabled && google_ads_rows > 0,
            row_count: google_ads_rows,
            unavailable_reason: if ga4_unified_topology {
                Some("disabled_by_ga4_unified_topology".to_string())
            } else if !config.google_ads.enabled {
                Some("source_disabled".to_string())
            } else if google_ads_rows == 0 {
                Some("no_rows_in_requested_window".to_string())
            } else {
                None
            },
        },
        SourceCoverageV1 {
            source_system: "ga4".to_string(),
            enabled: config.ga4.enabled,
            observed: config.ga4.enabled && ga4_events > 0,
            row_count: ga4_events,
            unavailable_reason: if !config.ga4.enabled {
                Some("source_disabled".to_string())
            } else if ga4_events == 0 {
                Some("no_rows_in_requested_window".to_string())
            } else {
                None
            },
        },
        SourceCoverageV1 {
            source_system: "wix_storefront".to_string(),
            enabled: wix_enabled,
            observed: wix_enabled && wix_rows > 0,
            row_count: wix_rows,
            unavailable_reason: if ga4_unified_topology {
                Some("disabled_by_ga4_unified_topology".to_string())
            } else if !config.wix.enabled {
                Some("source_disabled".to_string())
            } else if wix_rows == 0 {
                Some("no_rows_in_requested_window".to_string())
            } else {
                None
            },
        },
    ]
}

fn build_provenance(
    connector_id: &str,
    mode: &AnalyticsConnectorModeV1,
    source_coverage: &[SourceCoverageV1],
    cleaning_note_count_by_source: &BTreeMap<String, u32>,
) -> Vec<SourceProvenance> {
    let (source_class, collected_at_utc) = if mode.is_simulated() {
        (
            SourceClassLabel::Simulated,
            "deterministic-simulated".to_string(),
        )
    } else {
        (SourceClassLabel::Observed, Utc::now().to_rfc3339())
    };

    source_coverage
        .iter()
        .filter(|item| item.enabled)
        .map(|item| {
            let freshness_minutes = if mode.is_simulated() {
                match item.source_system.as_str() {
                    "ga4" => 45,
                    "google_ads" => 90,
                    "wix_storefront" => 120,
                    _ => 60,
                }
            } else {
                0
            };
            SourceProvenance {
                connector_id: connector_id.to_string(),
                source_class: source_class.clone(),
                source_system: item.source_system.clone(),
                collected_at_utc: collected_at_utc.clone(),
                freshness_minutes,
                validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
                rejected_rows_count: 0,
                cleaning_note_count: *cleaning_note_count_by_source
                    .get(&item.source_system)
                    .unwrap_or(&0),
            }
        })
        .collect()
}

fn build_cross_source_checks(
    report: &AnalyticsReport,
    seed: u64,
    reconciliation_policy: &ReconciliationPolicyV1,
    source_coverage: &[SourceCoverageV1],
    source_topology: &AnalyticsSourceTopologyV1,
) -> Vec<QualityCheckV1> {
    if *source_topology == AnalyticsSourceTopologyV1::Ga4Unified {
        return vec![
            QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::NotApplicable,
                code: "cross_source_attributed_revenue_within_wix_gross".to_string(),
                passed: true,
                severity: "low".to_string(),
                observed: "source_topology=ga4_unified".to_string(),
                expected:
                    "independent google_ads and wix_storefront streams required for reconciliation"
                        .to_string(),
            },
            QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::NotApplicable,
                code: "cross_source_ga4_sessions_within_click_bound".to_string(),
                passed: true,
                severity: "low".to_string(),
                observed: "source_topology=ga4_unified".to_string(),
                expected: "independent google_ads and ga4 streams required for reconciliation"
                    .to_string(),
            },
        ];
    }

    let source_ready = |name: &str| -> bool {
        source_coverage
            .iter()
            .find(|item| item.source_system == name)
            .map(|item| item.enabled && item.observed)
            .unwrap_or(false)
    };
    let ads_ready = source_ready("google_ads");
    let ga4_ready = source_ready("ga4");
    let wix_ready = source_ready("wix_storefront");

    let attributed_revenue = report.total_metrics.conversions_value;
    let wix_revenue_multiplier = 0.99 + ((seed % 3) as f64 * 0.005);
    let wix_gross_revenue = round4(attributed_revenue * wix_revenue_multiplier);

    let ga4_sessions = round4((report.total_metrics.clicks as f64) * 0.92);
    let clicks = report.total_metrics.clicks as f64;

    let revenue_check_code = "cross_source_attributed_revenue_within_wix_gross";
    let revenue_applicable = ads_ready && wix_ready;
    let (revenue_passed, revenue_severity, revenue_expected, revenue_applicability) =
        if !revenue_applicable {
            (
                true,
                "low".to_string(),
                "requires observed google_ads and wix_storefront coverage".to_string(),
                QualityCheckApplicabilityV1::NotApplicable,
            )
        } else if let Some(tolerance) = reconciliation_policy.tolerance_for(revenue_check_code) {
            let rel_tol = tolerance.max_relative_delta.unwrap_or(0.0).max(0.0);
            let max_allowed = wix_gross_revenue * (1.0 + rel_tol);
            (
                attributed_revenue <= max_allowed,
                tolerance.severity.clone(),
                format!("attributed_revenue <= wix_gross * (1 + {:.2})", rel_tol),
                QualityCheckApplicabilityV1::Applies,
            )
        } else {
            (
                false,
                "high".to_string(),
                "reconciliation policy must define revenue tolerance".to_string(),
                QualityCheckApplicabilityV1::Applies,
            )
        };

    let sessions_check_code = "cross_source_ga4_sessions_within_click_bound";
    let sessions_applicable = ads_ready && ga4_ready;
    let (sessions_passed, sessions_severity, sessions_expected, sessions_applicability) =
        if !sessions_applicable {
            (
                true,
                "low".to_string(),
                "requires observed google_ads and ga4 coverage".to_string(),
                QualityCheckApplicabilityV1::NotApplicable,
            )
        } else if let Some(tolerance) = reconciliation_policy.tolerance_for(sessions_check_code) {
            let rel_tol = tolerance.max_relative_delta.unwrap_or(0.0).max(0.0);
            let max_allowed = clicks * (1.0 + rel_tol);
            (
                ga4_sessions <= max_allowed,
                tolerance.severity.clone(),
                format!("ga4_sessions <= ad_clicks * (1 + {:.2})", rel_tol),
                QualityCheckApplicabilityV1::Applies,
            )
        } else {
            (
                false,
                "high".to_string(),
                "reconciliation policy must define GA4 session tolerance".to_string(),
                QualityCheckApplicabilityV1::Applies,
            )
        };

    vec![
        QualityCheckV1 {
            applicability: revenue_applicability,
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
            applicability: sessions_applicability,
            code: sessions_check_code.to_string(),
            passed: sessions_passed,
            severity: sessions_severity,
            observed: format!("ga4_sessions={:.4}, ad_clicks={:.4}", ga4_sessions, clicks),
            expected: sessions_expected,
        },
    ]
}

fn derive_observed_window_inputs_from_raw(
    google_ads_rows: &[GoogleAdsRowRawV1],
    ga4_events: &[Ga4EventRawV1],
    wix_orders: &[WixOrderRawV1],
) -> (
    BTreeMap<String, Vec<DateTime<Utc>>>,
    BTreeMap<String, TimeGranularity>,
) {
    let mut by_source = BTreeMap::new();
    let mut granularity_by_source = BTreeMap::new();
    by_source.insert("google_ads".to_string(), Vec::new());
    by_source.insert("ga4".to_string(), Vec::new());
    by_source.insert("wix_storefront".to_string(), Vec::new());
    granularity_by_source.insert("google_ads".to_string(), TimeGranularity::Day);
    // GA4 runReport rows are aggregated; evaluate completeness at day-level.
    granularity_by_source.insert("ga4".to_string(), TimeGranularity::Day);
    granularity_by_source.insert("wix_storefront".to_string(), TimeGranularity::Day);

    for row in google_ads_rows {
        if let Ok(day) = NaiveDate::parse_from_str(row.date.trim(), "%Y-%m-%d") {
            if let Some(ts) = day.and_hms_opt(12, 0, 0) {
                by_source
                    .entry("google_ads".to_string())
                    .or_default()
                    .push(ts.and_utc());
            }
        }
    }
    for event in ga4_events {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(event.event_timestamp_utc.trim()) {
            by_source
                .entry("ga4".to_string())
                .or_default()
                .push(parsed.with_timezone(&Utc));
        }
    }
    for order in wix_orders {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(order.placed_at_utc.trim()) {
            by_source
                .entry("wix_storefront".to_string())
                .or_default()
                .push(parsed.with_timezone(&Utc));
        }
    }

    (by_source, granularity_by_source)
}

fn resolve_source_window_inputs(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
    google_ads_rows: &[GoogleAdsRowRawV1],
    ga4_events: &[Ga4EventRawV1],
    wix_orders: &[WixOrderRawV1],
) -> Result<
    (
        BTreeMap<String, Vec<DateTime<Utc>>>,
        BTreeMap<String, TimeGranularity>,
        Option<BTreeSet<String>>,
    ),
    AnalyticsError,
> {
    let (mut observed_by_source, mut granularity_by_source) =
        derive_observed_window_inputs_from_raw(google_ads_rows, ga4_events, wix_orders);
    if request.source_window_observations.is_empty() {
        let start_utc = start
            .and_hms_opt(0, 0, 0)
            .expect("start date should support midnight")
            .and_utc();
        let end_utc = end
            .and_hms_opt(23, 59, 59)
            .expect("end date should support second boundary")
            .and_utc();
        for points in observed_by_source.values_mut() {
            points.retain(|point| *point >= start_utc && *point <= end_utc);
        }
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
    ga4_events: &[Ga4EventRawV1],
    provenance: &[SourceProvenance],
    budget_checks: Vec<QualityCheckV1>,
    cross_source_checks: Vec<QualityCheckV1>,
    freshness_policy: &FreshnessSlaPolicyV1,
    reconciliation_policy: &ReconciliationPolicyV1,
    start: NaiveDate,
    end: NaiveDate,
    observed_units_by_source: &BTreeMap<String, Vec<DateTime<Utc>>>,
    granularity_by_source: &BTreeMap<String, TimeGranularity>,
    source_coverage: &[SourceCoverageV1],
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
    let source_by_name = |name: &str| -> Option<&SourceCoverageV1> {
        source_coverage
            .iter()
            .find(|item| item.source_system == name)
    };
    let google_ads_enabled = source_by_name("google_ads")
        .map(|item| item.enabled)
        .unwrap_or(false);
    let google_ads_observed = source_by_name("google_ads")
        .map(|item| item.row_count > 0)
        .unwrap_or(false);
    let google_ads_applicability = if google_ads_enabled && google_ads_observed {
        QualityCheckApplicabilityV1::Applies
    } else {
        QualityCheckApplicabilityV1::NotApplicable
    };

    let schema_drift_checks = vec![
        QualityCheckV1 {
            applicability: google_ads_applicability.clone(),
            code: "schema_campaign_required_fields".to_string(),
            passed: google_ads_applicability == QualityCheckApplicabilityV1::NotApplicable
                || report.campaign_data.iter().all(|row| {
                    !row.campaign_id.trim().is_empty() && !row.campaign_name.trim().is_empty()
                }),
            severity: "high".to_string(),
            observed: "campaign rows contain id/name".to_string(),
            expected: "all campaign rows include stable id and name".to_string(),
        },
        QualityCheckV1 {
            applicability: google_ads_applicability.clone(),
            code: "schema_keyword_required_fields".to_string(),
            passed: google_ads_applicability == QualityCheckApplicabilityV1::NotApplicable
                || report.keyword_data.iter().all(|row| {
                    !row.keyword_id.trim().is_empty() && !row.keyword_text.trim().is_empty()
                }),
            severity: "high".to_string(),
            observed: "keyword rows contain id/text".to_string(),
            expected: "all keyword rows include criterion id and keyword text".to_string(),
        },
        QualityCheckV1 {
            applicability: QualityCheckApplicabilityV1::Applies,
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
            applicability: QualityCheckApplicabilityV1::Applies,
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
    let ga4_enabled = source_by_name("ga4")
        .map(|item| item.enabled)
        .unwrap_or(false);
    let ga4_observed = source_by_name("ga4")
        .map(|item| item.row_count > 0)
        .unwrap_or(false);
    let ga4_applicability = if ga4_enabled && ga4_observed {
        QualityCheckApplicabilityV1::Applies
    } else {
        QualityCheckApplicabilityV1::NotApplicable
    };
    let (ga4_duplicate_rows, ga4_unique_signatures, ga4_duplicate_ratio) =
        ga4_duplicate_signature_stats(ga4_events);
    let (ga4_near_duplicate_rows, ga4_near_duplicate_groups, ga4_near_duplicate_ratio) =
        ga4_near_duplicate_second_stats(ga4_events);
    let (
        ga4_custom_purchase_rows,
        ga4_custom_purchase_rows_with_tx,
        ga4_custom_purchase_rows_with_value,
    ) = ga4_custom_purchase_schema_stats(ga4_events);
    let ga4_custom_purchase_match_stats = ga4_custom_purchase_match_stats(ga4_events, 30);
    let ga4_custom_purchase_applies =
        ga4_applicability == QualityCheckApplicabilityV1::Applies && ga4_custom_purchase_rows > 0;
    let ga4_custom_purchase_applicability = if ga4_custom_purchase_applies {
        QualityCheckApplicabilityV1::Applies
    } else {
        QualityCheckApplicabilityV1::NotApplicable
    };
    let mut schema_drift_checks = schema_drift_checks;
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_applicability.clone(),
        code: "ga4_duplicate_event_signature_rate".to_string(),
        passed: ga4_applicability == QualityCheckApplicabilityV1::NotApplicable
            || ga4_duplicate_ratio <= 0.02,
        severity: "medium".to_string(),
        observed: format!(
            "duplicate_rows={}, total_rows={}, unique_signatures={}, ratio={:.4}",
            ga4_duplicate_rows,
            ga4_events.len(),
            ga4_unique_signatures,
            ga4_duplicate_ratio
        ),
        expected: "duplicate ratio <= 0.02".to_string(),
    });
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_applicability.clone(),
        code: "ga4_near_duplicate_second_rate".to_string(),
        passed: ga4_applicability == QualityCheckApplicabilityV1::NotApplicable
            || ga4_near_duplicate_ratio <= 0.05,
        severity: "medium".to_string(),
        observed: format!(
            "extra_rows={}, total_rows={}, duplicate_groups={}, ratio={:.4}",
            ga4_near_duplicate_rows,
            ga4_events.len(),
            ga4_near_duplicate_groups,
            ga4_near_duplicate_ratio
        ),
        expected: "near-duplicate ratio <= 0.05".to_string(),
    });
    let purchase_ndp_rows_in_report = report
        .keyword_data
        .iter()
        .filter(|row| is_ga4_custom_purchase_event(&row.keyword_text))
        .count();
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_applicability.clone(),
        code: "ga4_custom_purchase_ndp_excluded_from_truth_kpis".to_string(),
        passed: ga4_applicability == QualityCheckApplicabilityV1::NotApplicable
            || purchase_ndp_rows_in_report == 0,
        severity: "high".to_string(),
        observed: format!("purchase_ndp_rows_in_report={purchase_ndp_rows_in_report}"),
        expected: "purchase_ndp must not appear in KPI rollups/report tables".to_string(),
    });
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_custom_purchase_applicability.clone(),
        code: "ga4_custom_purchase_ndp_schema_integrity".to_string(),
        passed: !ga4_custom_purchase_applies
            || (ga4_custom_purchase_rows_with_tx == ga4_custom_purchase_rows
                && ga4_custom_purchase_rows_with_value == ga4_custom_purchase_rows),
        severity: "medium".to_string(),
        observed: format!(
            "purchase_ndp_rows={}, with_transaction_id={}, with_value={}",
            ga4_custom_purchase_rows,
            ga4_custom_purchase_rows_with_tx,
            ga4_custom_purchase_rows_with_value
        ),
        expected:
            "all purchase_ndp rows include transaction_id and value; otherwise keep excluded from truth KPIs"
                .to_string(),
    });
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_custom_purchase_applicability.clone(),
        code: "ga4_custom_purchase_ndp_overlap_rate".to_string(),
        passed: !ga4_custom_purchase_applies || ga4_custom_purchase_match_stats.overlap_ratio <= 0.20,
        severity: "medium".to_string(),
        observed: format!(
            "purchase_ndp_rows={}, rows_with_canonical_purchase={}, orphan_rows={}, overlap_ratio={:.4}, orphan_ratio={:.4}",
            ga4_custom_purchase_match_stats.total_rows,
            ga4_custom_purchase_match_stats.rows_with_canonical_purchase,
            ga4_custom_purchase_match_stats.orphan_rows,
            ga4_custom_purchase_match_stats.overlap_ratio,
            ga4_custom_purchase_match_stats.orphan_ratio
        ),
        expected:
            "custom purchase overlap ratio <= 0.20 after duplicate-stream cleanup; keep excluded from truth KPIs"
                .to_string(),
    });
    schema_drift_checks.push(QualityCheckV1 {
        applicability: ga4_custom_purchase_applicability,
        code: "ga4_custom_purchase_ndp_orphan_rate".to_string(),
        passed: !ga4_custom_purchase_applies || ga4_custom_purchase_match_stats.orphan_ratio <= 0.05,
        severity: "medium".to_string(),
        observed: format!(
            "purchase_ndp_rows={}, rows_with_canonical_purchase={}, orphan_rows={}, overlap_ratio={:.4}, orphan_ratio={:.4}",
            ga4_custom_purchase_match_stats.total_rows,
            ga4_custom_purchase_match_stats.rows_with_canonical_purchase,
            ga4_custom_purchase_match_stats.orphan_rows,
            ga4_custom_purchase_match_stats.overlap_ratio,
            ga4_custom_purchase_match_stats.orphan_ratio
        ),
        expected:
            "custom purchase orphan ratio <= 0.05; otherwise investigate missing canonical purchase coverage"
                .to_string(),
    });
    let reconciliation_code = "identity_campaign_rollup_reconciliation";
    let reconciliation_tol = reconciliation_policy.tolerance_for(reconciliation_code);
    let spend_delta = (sum_campaign_spend - report.total_metrics.cost).abs();
    let revenue_delta = (sum_campaign_revenue - report.total_metrics.conversions_value).abs();
    let (
        reconciliation_passed,
        reconciliation_severity,
        reconciliation_expected,
        reconciliation_applicability,
    ) = if google_ads_applicability == QualityCheckApplicabilityV1::NotApplicable {
        (
            true,
            "low".to_string(),
            "requires observed google_ads coverage".to_string(),
            QualityCheckApplicabilityV1::NotApplicable,
        )
    } else if let Some(tol) = reconciliation_tol {
        let epsilon = tol.max_abs_delta.unwrap_or(0.0).max(0.0);
        (
            spend_delta <= epsilon && revenue_delta <= epsilon,
            tol.severity.clone(),
            format!("abs(delta) <= {:.4}", epsilon),
            QualityCheckApplicabilityV1::Applies,
        )
    } else {
        (
            false,
            "high".to_string(),
            "reconciliation policy must define absolute tolerance".to_string(),
            QualityCheckApplicabilityV1::Applies,
        )
    };

    let identity_resolution_checks = vec![
        QualityCheckV1 {
            applicability: google_ads_applicability.clone(),
            code: "identity_ad_group_linked_to_campaign".to_string(),
            passed: google_ads_applicability == QualityCheckApplicabilityV1::NotApplicable
                || ad_group_coverage_ratio >= MIN_IDENTITY_COVERAGE_RATIO,
            severity: "high".to_string(),
            observed: format!("coverage={:.3}", ad_group_coverage_ratio),
            expected: format!("coverage >= {:.2}", MIN_IDENTITY_COVERAGE_RATIO),
        },
        QualityCheckV1 {
            applicability: google_ads_applicability.clone(),
            code: "identity_keyword_linked_to_ad_group".to_string(),
            passed: google_ads_applicability == QualityCheckApplicabilityV1::NotApplicable
                || keyword_coverage_ratio >= MIN_IDENTITY_COVERAGE_RATIO,
            severity: "high".to_string(),
            observed: format!("coverage={:.3}", keyword_coverage_ratio),
            expected: format!("coverage >= {:.2}", MIN_IDENTITY_COVERAGE_RATIO),
        },
        QualityCheckV1 {
            applicability: reconciliation_applicability,
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
        let source_has_rows = source_by_name(&item.source_system)
            .map(|source| source.row_count > 0)
            .unwrap_or(true);
        let source_applicability = if source_has_rows {
            QualityCheckApplicabilityV1::Applies
        } else {
            QualityCheckApplicabilityV1::NotApplicable
        };
        let maybe_threshold = freshness_policy.threshold_for(&item.source_system);
        let (freshness_passed, freshness_severity, freshness_expected) =
            if source_applicability == QualityCheckApplicabilityV1::NotApplicable {
                (
                    true,
                    "low".to_string(),
                    "not applicable for zero-row source window".to_string(),
                )
            } else if let Some(threshold) = maybe_threshold {
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
            applicability: source_applicability.clone(),
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
        ) = if source_applicability == QualityCheckApplicabilityV1::NotApplicable {
            (
                true,
                "low".to_string(),
                "no rows in selected window".to_string(),
                "not applicable for zero-row source window".to_string(),
            )
        } else if let Some(threshold) = maybe_threshold {
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
            applicability: source_applicability.clone(),
            code: format!("completeness_sla_{}", item.source_system),
            passed: completeness_passed,
            severity: completeness_severity,
            observed: completeness_observed,
            expected: completeness_expected,
        });
        if let Some(provided_sources) = provided_observation_sources {
            freshness_sla_checks.push(QualityCheckV1 {
                applicability: source_applicability.clone(),
                code: format!("source_window_observation_present_{}", item.source_system),
                passed: source_applicability == QualityCheckApplicabilityV1::NotApplicable
                    || provided_sources.contains(&item.source_system),
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
        .all(|c| c.applicability == QualityCheckApplicabilityV1::NotApplicable || c.passed);

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
    let identity_applicability_ratio =
        applicability_ratio(&quality_controls.identity_resolution_checks);
    let freshness_pass_ratio =
        pass_ratio_filtered(&quality_controls.freshness_sla_checks, "freshness_sla_");
    let reconciliation_pass_ratio = quality_controls
        .identity_resolution_checks
        .iter()
        .find(|check| check.code == "identity_campaign_rollup_reconciliation")
        .map(|check| if check.passed { 1.0 } else { 0.0 })
        .unwrap_or(0.0);
    let cross_source_pass_ratio = pass_ratio(&quality_controls.cross_source_checks);
    let cross_source_applicability_ratio =
        applicability_ratio(&quality_controls.cross_source_checks);
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
        identity_applicability_ratio,
        freshness_pass_ratio,
        reconciliation_pass_ratio,
        cross_source_pass_ratio,
        cross_source_applicability_ratio,
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
            applicability: QualityCheckApplicabilityV1::Applies,
            code: "budget_no_blocked_spend".to_string(),
            passed: blocked_events == 0,
            severity: "high".to_string(),
            observed: format!("blocked_events={blocked_events}"),
            expected: "blocked_events=0".to_string(),
        },
        QualityCheckV1 {
            applicability: QualityCheckApplicabilityV1::Applies,
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
            applicability: QualityCheckApplicabilityV1::Applies,
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
            applicability: QualityCheckApplicabilityV1::Applies,
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
            applicability: QualityCheckApplicabilityV1::Applies,
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
    let applicable = checks
        .iter()
        .filter(|check| check.applicability == QualityCheckApplicabilityV1::Applies)
        .collect::<Vec<_>>();
    if applicable.is_empty() {
        return 1.0;
    }
    let passed = applicable.iter().filter(|check| check.passed).count() as f64;
    round4((passed / applicable.len() as f64).clamp(0.0, 1.0))
}

fn pass_ratio_filtered(checks: &[QualityCheckV1], code_prefix: &str) -> f64 {
    let mut total = 0usize;
    let mut passed = 0usize;
    for check in checks.iter().filter(|check| {
        check.code.starts_with(code_prefix)
            && check.applicability == QualityCheckApplicabilityV1::Applies
    }) {
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

fn applicability_ratio(checks: &[QualityCheckV1]) -> f64 {
    if checks.is_empty() {
        return 1.0;
    }
    let applicable = checks
        .iter()
        .filter(|check| check.applicability == QualityCheckApplicabilityV1::Applies)
        .count() as f64;
    round4((applicable / checks.len() as f64).clamp(0.0, 1.0))
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
    google_ads_rows: &[GoogleAdsRowRawV1],
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

    for row in google_ads_rows {
        let raw_json = serde_json::to_string(row).map_err(|err| {
            AnalyticsError::internal(
                "ingest_serialization_failed",
                format!("failed to serialize Google Ads raw row: {err}"),
            )
        })?;
        let parsed = parse_google_ads_row(&raw_json).map_err(|err| {
            AnalyticsError::new(
                "ingest_validation_failed",
                format!("failed to parse/normalize google ads row: {}", err.reason),
                vec![err.field],
                None,
            )
        })?;
        increment_source_count(&mut note_counts, "google_ads", parsed.notes.len() as u32);
        notes.extend(map_ingest_notes("google_ads", &parsed.notes));
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
            assert!((0.0..=1.0).contains(&artifact.data_quality.identity_applicability_ratio));
            assert!((0.0..=1.0).contains(&artifact.data_quality.cross_source_pass_ratio));
            assert!((0.0..=1.0).contains(&artifact.data_quality.cross_source_applicability_ratio));
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
        let context = err.context.expect("validation report context");
        assert_eq!(
            context
                .get("validation")
                .and_then(|value| value.get("is_valid"))
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn data_quality_summary_uses_completeness_checks_not_schema_checks() {
        let controls = AnalyticsQualityControlsV1 {
            schema_drift_checks: vec![QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::Applies,
                code: "schema_ok".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            identity_resolution_checks: vec![QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::Applies,
                code: "identity_campaign_rollup_reconciliation".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            freshness_sla_checks: vec![
                QualityCheckV1 {
                    applicability: QualityCheckApplicabilityV1::Applies,
                    code: "freshness_sla_ga4".to_string(),
                    passed: true,
                    severity: "high".to_string(),
                    observed: "ok".to_string(),
                    expected: "ok".to_string(),
                },
                QualityCheckV1 {
                    applicability: QualityCheckApplicabilityV1::Applies,
                    code: "completeness_sla_ga4".to_string(),
                    passed: false,
                    severity: "high".to_string(),
                    observed: "missing".to_string(),
                    expected: "ratio >= 0.98".to_string(),
                },
            ],
            cross_source_checks: vec![QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::Applies,
                code: "cross_source_ok".to_string(),
                passed: true,
                severity: "high".to_string(),
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            }],
            budget_checks: vec![QualityCheckV1 {
                applicability: QualityCheckApplicabilityV1::Applies,
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
        assert_eq!(summary.identity_applicability_ratio, 1.0);
        assert_eq!(summary.freshness_pass_ratio, 1.0);
        assert_eq!(summary.reconciliation_pass_ratio, 1.0);
        assert_eq!(summary.quality_score, 0.75);
    }

    #[test]
    fn ga4_duplicate_signature_stats_detects_duplicate_rows() {
        let mut dimensions = BTreeMap::new();
        dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1709500000000000".to_string(),
        );
        dimensions.insert("ga_session_id".to_string(), "1234".to_string());
        dimensions.insert("event_bundle_sequence_id".to_string(), "10".to_string());
        dimensions.insert("batch_event_index".to_string(), "1".to_string());
        let base = Ga4EventRawV1 {
            event_name: "purchase".to_string(),
            event_timestamp_utc: "2026-03-01T12:00:00Z".to_string(),
            user_pseudo_id: "user-1".to_string(),
            session_id: Some("ga_session:1234".to_string()),
            campaign: Some("spring".to_string()),
            device_category: Some("mobile".to_string()),
            source_medium: Some("google / cpc".to_string()),
            dimensions: dimensions.clone(),
            metrics: BTreeMap::new(),
            ..Default::default()
        };
        let events = vec![
            base.clone(),
            base,
            Ga4EventRawV1 {
                event_name: "page_view".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("ga_session:1234".to_string()),
                campaign: Some("spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
        ];
        let (duplicate_rows, unique_signatures, ratio) = ga4_duplicate_signature_stats(&events);
        assert_eq!(duplicate_rows, 1);
        assert_eq!(unique_signatures, 2);
        assert!(ratio > 0.0);
    }

    #[test]
    fn ga4_near_duplicate_second_stats_detects_same_second_replays() {
        let mut base_dimensions = BTreeMap::new();
        base_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1709500000000000".to_string(),
        );
        base_dimensions.insert("ga_session_id".to_string(), "1234".to_string());
        base_dimensions.insert("event_bundle_sequence_id".to_string(), "10".to_string());
        base_dimensions.insert("batch_event_index".to_string(), "1".to_string());

        let mut second_dimensions = base_dimensions.clone();
        second_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1709500000200000".to_string(),
        );
        second_dimensions.insert("event_bundle_sequence_id".to_string(), "11".to_string());
        second_dimensions.insert("batch_event_index".to_string(), "2".to_string());

        let mut third_dimensions = base_dimensions.clone();
        third_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1709500000400000".to_string(),
        );
        third_dimensions.insert("event_bundle_sequence_id".to_string(), "12".to_string());
        third_dimensions.insert("batch_event_index".to_string(), "3".to_string());

        let events = vec![
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:00Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("1234".to_string()),
                campaign: Some("spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: base_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:00Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("1234".to_string()),
                campaign: Some("spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: second_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:00Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("1234".to_string()),
                campaign: Some("spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: third_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "page_view".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:00Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("1234".to_string()),
                campaign: Some("spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: BTreeMap::new(),
                metrics: BTreeMap::new(),
                ..Default::default()
            },
        ];

        let (duplicate_rows, duplicate_groups, ratio) = ga4_near_duplicate_second_stats(&events);
        assert_eq!(duplicate_rows, 2);
        assert_eq!(duplicate_groups, 1);
        assert!((ratio - 0.5).abs() < 0.0001);
    }

    #[test]
    fn ga4_custom_purchase_match_stats_detect_overlap_and_orphans() {
        let mut canonical_dimensions = BTreeMap::new();
        canonical_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1772500001000000".to_string(),
        );
        canonical_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());
        canonical_dimensions.insert("transaction_id".to_string(), "tx-123".to_string());
        canonical_dimensions.insert("purchase_revenue".to_string(), "57.25".to_string());

        let mut matching_custom_dimensions = BTreeMap::new();
        matching_custom_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1772500003000000".to_string(),
        );
        matching_custom_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());

        let mut orphan_custom_dimensions = BTreeMap::new();
        orphan_custom_dimensions.insert(
            "event_timestamp_micros".to_string(),
            "1772503600000000".to_string(),
        );
        orphan_custom_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());

        let events = vec![
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: canonical_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:03Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: matching_custom_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T13:00:00Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: orphan_custom_dimensions,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
        ];

        let stats = ga4_custom_purchase_match_stats(&events, 30);
        assert_eq!(stats.total_rows, 2);
        assert_eq!(stats.rows_with_canonical_purchase, 1);
        assert_eq!(stats.orphan_rows, 1);
        assert!((stats.overlap_ratio - 0.5).abs() < 0.0001);
        assert!((stats.orphan_ratio - 0.5).abs() < 0.0001);
    }

    #[test]
    fn ga4_report_uses_canonical_purchase_and_ignores_purchase_ndp_duplicates() {
        let request = MockAnalyticsRequestV1 {
            start_date: "2026-03-01".to_string(),
            end_date: "2026-03-01".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(1),
            profile_id: "qa".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };
        let mut purchase_dims = BTreeMap::new();
        purchase_dims.insert(
            "event_timestamp_micros".to_string(),
            "1772500001000000".to_string(),
        );
        purchase_dims.insert("ga_session_id".to_string(), "session-1".to_string());
        purchase_dims.insert("transaction_id".to_string(), "tx-123".to_string());
        purchase_dims.insert("purchase_revenue".to_string(), "57.25".to_string());

        let mut purchase_dup_dims = purchase_dims.clone();
        purchase_dup_dims.insert("batch_event_index".to_string(), "2".to_string());

        let mut purchase_ndp_dims = BTreeMap::new();
        purchase_ndp_dims.insert(
            "event_timestamp_micros".to_string(),
            "1772500001000000".to_string(),
        );
        purchase_ndp_dims.insert("ga_session_id".to_string(), "session-1".to_string());
        purchase_ndp_dims.insert("event_bundle_sequence_id".to_string(), "44".to_string());
        purchase_ndp_dims.insert("batch_event_index".to_string(), "1".to_string());

        let mut purchase_ndp_dup_dims = purchase_ndp_dims.clone();
        purchase_ndp_dup_dims.insert("batch_event_index".to_string(), "2".to_string());

        let events = vec![
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_ndp_dims,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase_ndp".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_ndp_dup_dims,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_dims,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_dup_dims,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
        ];

        let (custom_rows, custom_rows_with_tx, custom_rows_with_value) =
            ga4_custom_purchase_schema_stats(&events);
        assert_eq!(custom_rows, 2);
        assert_eq!(custom_rows_with_tx, 0);
        assert_eq!(custom_rows_with_value, 0);

        let report = ga4_events_to_report(
            &events,
            &request,
            NaiveDate::from_ymd_opt(2026, 3, 1).expect("date"),
            NaiveDate::from_ymd_opt(2026, 3, 1).expect("date"),
        );

        assert_eq!(report.total_metrics.conversions, 1.0);
        assert!((report.total_metrics.conversions_value - 57.25).abs() < 0.0001);
        assert!(!report
            .keyword_data
            .iter()
            .any(|row| row.keyword_text.eq_ignore_ascii_case("purchase_ndp")));
    }

    #[test]
    fn daily_revenue_series_tracks_canonical_purchase_revenue_per_day() {
        let mut purchase_day_one = BTreeMap::new();
        purchase_day_one.insert("ga_session_id".to_string(), "session-1".to_string());
        purchase_day_one.insert("transaction_id".to_string(), "tx-1".to_string());
        purchase_day_one.insert("purchase_revenue".to_string(), "57.25".to_string());

        let mut purchase_day_one_dup = purchase_day_one.clone();
        purchase_day_one_dup.insert("batch_event_index".to_string(), "2".to_string());

        let mut purchase_day_two = BTreeMap::new();
        purchase_day_two.insert("ga_session_id".to_string(), "session-2".to_string());
        purchase_day_two.insert("transaction_id".to_string(), "tx-2".to_string());
        purchase_day_two.insert("purchase_revenue".to_string(), "42.75".to_string());

        let events = vec![
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:01Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_day_one,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-01T12:00:02Z".to_string(),
                user_pseudo_id: "user-1".to_string(),
                session_id: Some("session-1".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("mobile".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_day_one_dup,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
            Ga4EventRawV1 {
                event_name: "purchase".to_string(),
                event_timestamp_utc: "2026-03-02T12:00:01Z".to_string(),
                user_pseudo_id: "user-2".to_string(),
                session_id: Some("session-2".to_string()),
                campaign: Some("Spring".to_string()),
                device_category: Some("desktop".to_string()),
                source_medium: Some("google / cpc".to_string()),
                dimensions: purchase_day_two,
                metrics: BTreeMap::new(),
                ..Default::default()
            },
        ];

        let series = build_daily_revenue_series_from_ga4(
            &events,
            NaiveDate::from_ymd_opt(2026, 3, 1).expect("date"),
            NaiveDate::from_ymd_opt(2026, 3, 3).expect("date"),
        );

        assert_eq!(series.len(), 3);
        assert_eq!(series[0].date, "2026-03-01");
        assert!((series[0].revenue - 57.25).abs() < 0.0001);
        assert_eq!(series[1].date, "2026-03-02");
        assert!((series[1].revenue - 42.75).abs() < 0.0001);
        assert_eq!(series[2].date, "2026-03-03");
        assert!(series[2].revenue.abs() < 0.0001);
    }

    #[test]
    fn ingest_cleaning_audit_includes_ads_and_wix_notes() {
        let ga4 = vec![Ga4EventRawV1 {
            event_name: " purchase ".to_string(),
            event_timestamp_utc: "2026-02-01T12:00:00Z".to_string(),
            user_pseudo_id: " user_1 ".to_string(),
            session_id: Some("session-1".to_string()),
            campaign: Some("spring_launch".to_string()),
            device_category: Some("mobile".to_string()),
            source_medium: Some("google / cpc".to_string()),
            dimensions: BTreeMap::new(),
            metrics: BTreeMap::new(),
            ..Default::default()
        }];
        let ads = vec![GoogleAdsRowRawV1 {
            campaign_id: " camp-1 ".to_string(),
            ad_group_id: " adg-1 ".to_string(),
            date: "2026-02-01".to_string(),
            impressions: 100,
            clicks: 20,
            cost_micros: 500_000,
            conversions_micros: 900_000,
            currency: " usd ".to_string(),
        }];
        let wix = vec![WixOrderRawV1 {
            order_id: " wix-1 ".to_string(),
            placed_at_utc: "2026-02-01T18:00:00Z".to_string(),
            gross_amount: "123.45".to_string(),
            currency: " usd ".to_string(),
        }];

        let audit = collect_ingest_cleaning_notes(&ga4, &ads, &wix).expect("ingest audit");
        assert!(!audit.notes.is_empty());
        assert_eq!(audit.note_counts.get("ga4").copied().unwrap_or(0), 2);
        assert_eq!(audit.note_counts.get("google_ads").copied().unwrap_or(0), 3);
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
        assert!(audit
            .notes
            .iter()
            .any(|note| note.source_system == "google_ads"));
    }

    #[test]
    fn google_ads_raw_conversion_rejects_non_finite_values() {
        let rows = vec![GoogleAdsRow {
            campaign: Some(CampaignResource {
                resource_name: "customers/1/campaigns/1".to_string(),
                id: "1".to_string(),
                name: "c".to_string(),
                status: "ENABLED".to_string(),
            }),
            ad_group: Some(AdGroupResource {
                resource_name: "customers/1/adGroups/1".to_string(),
                id: "1".to_string(),
                name: "a".to_string(),
                status: "ENABLED".to_string(),
                campaign_resource_name: "customers/1/campaigns/1".to_string(),
            }),
            keyword_view: None,
            ad_group_criterion: None,
            metrics: Some(MetricsData {
                impressions: 100,
                clicks: 10,
                cost_micros: 100_000,
                conversions: 1.0,
                conversions_value: f64::NAN,
                ctr: 10.0,
                average_cpc: 1.0,
            }),
            segments: Some(crate::data_models::analytics::SegmentsData {
                date: Some("2026-02-01".to_string()),
                device: Some("DESKTOP".to_string()),
            }),
        }];

        let err = google_ads_rows_to_raw_v1(&rows).expect_err("non-finite values must fail");
        assert_eq!(err.code, "ads_conversion_value_non_finite");
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

    struct EmptyObservedConnector;

    #[async_trait]
    impl super::super::connector_v2::AnalyticsConnectorContractV2 for EmptyObservedConnector {
        fn capabilities(&self) -> super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
            super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
                connector_id: "empty_observed_connector".to_string(),
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
                connector_id: "empty_observed_connector".to_string(),
                ok: true,
                mode: "observed_read_only".to_string(),
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
            Ok(Vec::new())
        }

        async fn fetch_google_ads_rows(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _request: &MockAnalyticsRequestV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_orders(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::ingest::WixOrderRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_sessions(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::connector_v2::WixSessionRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn observed_mode_allows_zero_row_window_with_valid_artifact() {
        let mut cfg =
            super::super::analytics_config::AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = super::super::analytics_config::AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        cfg.ga4.enabled = true;

        let svc = DefaultMarketAnalysisService::with_connector_and_config(
            Arc::new(EmptyObservedConnector),
            cfg,
        )
        .expect("service config should be valid");
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(11),
            profile_id: "staging-observed".to_string(),
            include_narratives: false,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let artifact = svc
            .run_mock_analysis(req)
            .await
            .expect("observed mode should emit valid no-data artifact for empty window");
        let ga4_coverage = artifact
            .source_coverage
            .iter()
            .find(|item| item.source_system == "ga4")
            .expect("ga4 coverage");
        assert!(ga4_coverage.enabled);
        assert_eq!(ga4_coverage.row_count, 0);
        assert!(artifact
            .uncertainty_notes
            .iter()
            .any(|note| note.contains("zero rows")));
    }

    struct Ga4OnlyObservedConnector;

    #[async_trait]
    impl super::super::connector_v2::AnalyticsConnectorContractV2 for Ga4OnlyObservedConnector {
        fn capabilities(&self) -> super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
            super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
                connector_id: "ga4_observed_connector".to_string(),
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
                connector_id: "ga4_observed_connector".to_string(),
                ok: true,
                mode: "observed_read_only".to_string(),
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
            Ok(vec![
                Ga4EventRawV1 {
                    event_name: "page_view".to_string(),
                    event_timestamp_utc: "2026-01-01T12:00:00Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("ga4_count:20".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("desktop".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: BTreeMap::new(),
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
                Ga4EventRawV1 {
                    event_name: "purchase".to_string(),
                    event_timestamp_utc: "2026-01-01T13:00:00Z".to_string(),
                    user_pseudo_id: "user-2".to_string(),
                    session_id: Some("ga4_count:3".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: BTreeMap::new(),
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
                Ga4EventRawV1 {
                    event_name: "cta_click".to_string(),
                    event_timestamp_utc: "2026-01-01T14:00:00Z".to_string(),
                    user_pseudo_id: "user-3".to_string(),
                    session_id: Some("ga4_count:5".to_string()),
                    campaign: Some("Brand Search".to_string()),
                    device_category: Some("tablet".to_string()),
                    source_medium: Some("direct / none".to_string()),
                    dimensions: BTreeMap::new(),
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
            ])
        }

        async fn fetch_google_ads_rows(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _request: &MockAnalyticsRequestV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_orders(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::ingest::WixOrderRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_sessions(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::connector_v2::WixSessionRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn ga4_only_observed_mode_populates_report_tables() {
        let mut cfg =
            super::super::analytics_config::AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = super::super::analytics_config::AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.source_topology = super::super::analytics_config::AnalyticsSourceTopologyV1::Ga4Unified;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        cfg.ga4.enabled = true;

        let svc = DefaultMarketAnalysisService::with_connector_and_config(
            Arc::new(Ga4OnlyObservedConnector),
            cfg,
        )
        .expect("service config should be valid");
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-01".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(12),
            profile_id: "staging-ga4-only".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let artifact = svc
            .run_mock_analysis(req)
            .await
            .expect("ga4-only observed run should produce non-empty report tables");
        assert!(!artifact.report.campaign_data.is_empty());
        assert!(!artifact.report.ad_group_data.is_empty());
        assert!(!artifact.report.keyword_data.is_empty());
        assert_eq!(artifact.report.total_metrics.impressions, 28);
        assert_eq!(artifact.report.total_metrics.clicks, 5);
        assert!((artifact.report.total_metrics.conversions - 3.0).abs() < 0.001);
        assert!(artifact
            .quality_controls
            .cross_source_checks
            .iter()
            .all(|check| check.applicability == QualityCheckApplicabilityV1::NotApplicable));
        let google_ads = artifact
            .source_coverage
            .iter()
            .find(|item| item.source_system == "google_ads")
            .expect("google_ads coverage");
        assert_eq!(
            google_ads.unavailable_reason.as_deref(),
            Some("disabled_by_ga4_unified_topology")
        );
        let wix = artifact
            .source_coverage
            .iter()
            .find(|item| item.source_system == "wix_storefront")
            .expect("wix coverage");
        assert_eq!(
            wix.unavailable_reason.as_deref(),
            Some("disabled_by_ga4_unified_topology")
        );
    }

    struct Ga4ObservedCustomPurchaseConnector;

    #[async_trait]
    impl super::super::connector_v2::AnalyticsConnectorContractV2
        for Ga4ObservedCustomPurchaseConnector
    {
        fn capabilities(&self) -> super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
            super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
                connector_id: "ga4_observed_custom_purchase_connector".to_string(),
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
                connector_id: "ga4_observed_custom_purchase_connector".to_string(),
                ok: true,
                mode: "observed_read_only".to_string(),
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
            let mut purchase_dimensions = BTreeMap::new();
            purchase_dimensions.insert("transaction_id".to_string(), "tx-1".to_string());
            purchase_dimensions.insert("purchase_revenue".to_string(), "57.25".to_string());

            let mut purchase_ndp_dimensions = BTreeMap::new();
            purchase_ndp_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());

            Ok(vec![
                Ga4EventRawV1 {
                    event_name: "purchase_ndp".to_string(),
                    event_timestamp_utc: "2026-01-01T12:00:00Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: purchase_ndp_dimensions,
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
                Ga4EventRawV1 {
                    event_name: "purchase".to_string(),
                    event_timestamp_utc: "2026-01-01T13:00:00Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: purchase_dimensions,
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
            ])
        }

        async fn fetch_google_ads_rows(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _request: &MockAnalyticsRequestV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_orders(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::ingest::WixOrderRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_sessions(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::connector_v2::WixSessionRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }
    }

    struct Ga4ObservedCustomPurchaseDuplicateConnector;

    #[async_trait]
    impl super::super::connector_v2::AnalyticsConnectorContractV2
        for Ga4ObservedCustomPurchaseDuplicateConnector
    {
        fn capabilities(&self) -> super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
            super::super::connector_v2::AnalyticsConnectorCapabilitiesV1 {
                connector_id: "ga4_observed_custom_purchase_duplicate_connector".to_string(),
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
                connector_id: "ga4_observed_custom_purchase_duplicate_connector".to_string(),
                ok: true,
                mode: "observed_read_only".to_string(),
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
            let mut purchase_dimensions = BTreeMap::new();
            purchase_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());
            purchase_dimensions.insert("transaction_id".to_string(), "tx-1".to_string());
            purchase_dimensions.insert("purchase_revenue".to_string(), "57.25".to_string());

            let mut matched_custom_dimensions = BTreeMap::new();
            matched_custom_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());

            let mut orphan_custom_dimensions = BTreeMap::new();
            orphan_custom_dimensions.insert("ga_session_id".to_string(), "session-1".to_string());

            Ok(vec![
                Ga4EventRawV1 {
                    event_name: "purchase_ndp".to_string(),
                    event_timestamp_utc: "2026-01-01T12:00:10Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: matched_custom_dimensions,
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
                Ga4EventRawV1 {
                    event_name: "purchase_ndp".to_string(),
                    event_timestamp_utc: "2026-01-01T13:00:00Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: orphan_custom_dimensions,
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
                Ga4EventRawV1 {
                    event_name: "purchase".to_string(),
                    event_timestamp_utc: "2026-01-01T12:00:00Z".to_string(),
                    user_pseudo_id: "user-1".to_string(),
                    session_id: Some("session-1".to_string()),
                    campaign: Some("Spring Launch".to_string()),
                    device_category: Some("mobile".to_string()),
                    source_medium: Some("google / cpc".to_string()),
                    dimensions: purchase_dimensions,
                    metrics: BTreeMap::new(),
                    ..Default::default()
                },
            ])
        }

        async fn fetch_google_ads_rows(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _request: &MockAnalyticsRequestV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<GoogleAdsRow>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_orders(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::ingest::WixOrderRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }

        async fn fetch_wix_sessions(
            &self,
            _config: &super::super::analytics_config::AnalyticsConnectorConfigV1,
            _start: NaiveDate,
            _end: NaiveDate,
            _seed: u64,
        ) -> Result<Vec<super::super::connector_v2::WixSessionRawV1>, AnalyticsError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn ga4_custom_purchase_schema_integrity_check_is_emitted() {
        let mut cfg =
            super::super::analytics_config::AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = super::super::analytics_config::AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.source_topology = super::super::analytics_config::AnalyticsSourceTopologyV1::Ga4Unified;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        cfg.ga4.enabled = true;

        let svc = DefaultMarketAnalysisService::with_connector_and_config(
            Arc::new(Ga4ObservedCustomPurchaseConnector),
            cfg,
        )
        .expect("service config should be valid");
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-01".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(44),
            profile_id: "staging-ga4-custom-purchase".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let artifact = svc
            .run_mock_analysis(req)
            .await
            .expect("run should succeed while marking custom purchase schema warning");

        let exclusion_check = artifact
            .quality_controls
            .schema_drift_checks
            .iter()
            .find(|check| check.code == "ga4_custom_purchase_ndp_excluded_from_truth_kpis")
            .expect("custom purchase exclusion check");
        assert!(exclusion_check.passed);
        assert_eq!(exclusion_check.severity, "high");

        let schema_check = artifact
            .quality_controls
            .schema_drift_checks
            .iter()
            .find(|check| check.code == "ga4_custom_purchase_ndp_schema_integrity")
            .expect("custom purchase schema integrity check");
        assert_eq!(
            schema_check.applicability,
            QualityCheckApplicabilityV1::Applies
        );
        assert!(!schema_check.passed);
        assert_eq!(schema_check.severity, "medium");
    }

    #[tokio::test]
    async fn ga4_custom_purchase_overlap_and_orphan_checks_are_emitted() {
        let mut cfg =
            super::super::analytics_config::AnalyticsConnectorConfigV1::simulated_defaults();
        cfg.mode = super::super::analytics_config::AnalyticsConnectorModeV1::ObservedReadOnly;
        cfg.source_topology = super::super::analytics_config::AnalyticsSourceTopologyV1::Ga4Unified;
        cfg.google_ads.enabled = false;
        cfg.wix.enabled = false;
        cfg.ga4.enabled = true;

        let svc = DefaultMarketAnalysisService::with_connector_and_config(
            Arc::new(Ga4ObservedCustomPurchaseDuplicateConnector),
            cfg,
        )
        .expect("service config should be valid");
        let req = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-01".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(55),
            profile_id: "staging-ga4-custom-purchase-duplicate".to_string(),
            include_narratives: true,
            source_window_observations: Vec::new(),
            budget_envelope: super::super::contracts::BudgetEnvelopeV1::default(),
        };

        let artifact = svc
            .run_mock_analysis(req)
            .await
            .expect("run should succeed while emitting duplicate-stream diagnostics");

        let overlap_check = artifact
            .quality_controls
            .schema_drift_checks
            .iter()
            .find(|check| check.code == "ga4_custom_purchase_ndp_overlap_rate")
            .expect("custom purchase overlap check");
        assert_eq!(
            overlap_check.applicability,
            QualityCheckApplicabilityV1::Applies
        );
        assert!(!overlap_check.passed);
        assert!(overlap_check
            .observed
            .contains("rows_with_canonical_purchase=1"));

        let orphan_check = artifact
            .quality_controls
            .schema_drift_checks
            .iter()
            .find(|check| check.code == "ga4_custom_purchase_ndp_orphan_rate")
            .expect("custom purchase orphan check");
        assert_eq!(
            orphan_check.applicability,
            QualityCheckApplicabilityV1::Applies
        );
        assert!(!orphan_check.passed);
        assert!(orphan_check.observed.contains("orphan_rows=1"));
        assert!(artifact
            .uncertainty_notes
            .iter()
            .any(|note| note.contains("duplicate instrumentation remains active")));
        assert!(artifact
            .uncertainty_notes
            .iter()
            .any(|note| note.contains("potential checkout undercount")));
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
                connector_attestation: Default::default(),
            },
            report: AnalyticsReport::default(),
            daily_revenue_series: Vec::new(),
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
            source_coverage: Vec::new(),
            ga4_session_rollups: Vec::new(),
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
