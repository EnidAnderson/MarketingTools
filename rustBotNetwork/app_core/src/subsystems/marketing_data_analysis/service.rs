use super::contracts::{
    AnalyticsError, AnalyticsQualityControlsV1, AnalyticsRunMetadataV1, BudgetSummaryV1,
    DataQualitySummaryV1, EvidenceItem, GuidanceItem, IngestCleaningNoteV1,
    KpiAttributionNarrativeV1, MockAnalyticsArtifactV1, MockAnalyticsRequestV1, OperatorSummaryV1,
    QualityCheckV1, MOCK_ANALYTICS_SCHEMA_VERSION_V1,
};
use super::budget::{build_budget_plan, enforce_daily_hard_cap, BudgetCategory, BudgetGuard};
use super::ingest::{parse_ga4_event, CleaningNote};
use super::validators::{validate_mock_analytics_artifact_v1, validate_mock_analytics_request_v1};
use crate::data_models::analytics::{
    AdGroupCriterionResource, AdGroupReportRow, AdGroupResource, AnalyticsReport,
    CampaignReportRow, CampaignResource, Ga4NormalizedEvent, GoogleAdsRow, KeywordData,
    KeywordReportRow, MetricsData, ReportMetrics, SegmentsData, SourceClassLabel, SourceProvenance,
};
use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const CAMPAIGN_NAMES: &[&str] = &[
    "Summer Pet Food Promo",
    "New Puppy Essentials",
    "Senior Dog Health",
    "Organic Cat Treats",
];
const AD_GROUP_NAMES: &[&str] = &["Dry Food", "Wet Food", "Treats", "Supplements"];
const KEYWORD_TEXTS: &[&str] = &[
    "healthy dog food",
    "grain-free cat food",
    "best puppy treats",
    "senior pet vitamins",
];
const MIN_IDENTITY_COVERAGE_RATIO: f64 = 0.98;
const RECONCILIATION_EPSILON: f64 = 0.01;
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
pub struct DefaultMarketAnalysisService;

impl DefaultMarketAnalysisService {
    pub fn new() -> Self {
        Self
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

        budget_guard.spend(
            BudgetCategory::Retrieval,
            budget_plan.estimated.retrieval_units,
            "mock_analytics.fetch",
        )?;
        let rows = generate_rows(&request, start, budget_plan.effective_end, seed);
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
        let ingest_cleaning_notes = collect_ingest_cleaning_notes(seed)?;
        let provenance = vec![SourceProvenance {
            connector_id: "mock_analytics_connector_v1".to_string(),
            source_class: SourceClassLabel::Simulated,
            source_system: "mock_analytics".to_string(),
            collected_at_utc: "deterministic-simulated".to_string(),
            freshness_minutes: 0,
            validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
            rejected_rows_count: 0,
            cleaning_note_count: ingest_cleaning_notes.len() as u32,
        }];
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
        let quality_controls = build_quality_controls(&report, &provenance, budget_checks);
        let data_quality = build_data_quality_summary(&quality_controls);
        let operator_summary = build_operator_summary(&report, &observed_evidence);

        let metadata = AnalyticsRunMetadataV1 {
            run_id: deterministic_run_id(&request, seed),
            connector_id: "mock_analytics_connector_v1".to_string(),
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
            ingest_cleaning_notes,
            validation: super::contracts::AnalyticsValidationReportV1 {
                is_valid: false,
                checks: Vec::new(),
            },
            quality_controls,
            data_quality,
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

fn generate_rows(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
    seed: u64,
) -> Vec<GoogleAdsRow> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut rows = Vec::new();
    let mut current = start;

    while current <= end {
        let date_str = current.format("%Y-%m-%d").to_string();
        for (campaign_idx, campaign_name) in CAMPAIGN_NAMES.iter().enumerate() {
            if let Some(filter) = &request.campaign_filter {
                if !campaign_name.contains(filter) {
                    continue;
                }
            }
            let campaign_id = format!("{}", campaign_idx + 1);
            let campaign_resource = format!("customers/123/campaigns/{}", campaign_id);
            let campaign_status = if rng.gen_bool(0.9) {
                "ENABLED"
            } else {
                "PAUSED"
            };
            let campaign = CampaignResource {
                resourceName: campaign_resource.clone(),
                id: campaign_id.clone(),
                name: (*campaign_name).to_string(),
                status: campaign_status.to_string(),
            };

            for (ad_group_idx, ad_group_name) in AD_GROUP_NAMES.iter().enumerate() {
                if let Some(filter) = &request.ad_group_filter {
                    if !ad_group_name.contains(filter) {
                        continue;
                    }
                }
                let ad_group_id = format!("{}.{}", campaign_id, ad_group_idx + 1);
                let ad_group_resource = format!("customers/123/adGroups/{}", ad_group_id);
                let ad_group_status = if rng.gen_bool(0.9) {
                    "ENABLED"
                } else {
                    "PAUSED"
                };
                let ad_group = AdGroupResource {
                    resourceName: ad_group_resource.clone(),
                    id: ad_group_id.clone(),
                    name: (*ad_group_name).to_string(),
                    status: ad_group_status.to_string(),
                    campaignResourceName: campaign_resource.clone(),
                };

                for (kw_idx, keyword_text) in KEYWORD_TEXTS.iter().enumerate() {
                    let impressions: u64 = rng.gen_range(100..1200);
                    let max_clicks = (impressions / 2).max(1);
                    let clicks: u64 = rng.gen_range(1..=max_clicks);
                    let cost_micros = clicks * rng.gen_range(200_000..1_300_000);
                    let conversions = round4(rng.gen_range(0.0..(clicks as f64 / 5.0)));
                    let conversions_value = round4(conversions * rng.gen_range(10.0..60.0));

                    let metrics = MetricsData {
                        clicks,
                        impressions,
                        costMicros: cost_micros,
                        conversions,
                        conversionsValue: conversions_value,
                        ctr: round4((clicks as f64 / impressions as f64) * 100.0),
                        averageCpc: round4(cost_micros as f64 / clicks as f64 / 1_000_000.0),
                    };

                    let criterion_id =
                        format!("{}{}{}", campaign_idx + 1, ad_group_idx + 1, kw_idx + 1);
                    let criterion = AdGroupCriterionResource {
                        resourceName: format!(
                            "customers/123/adGroupCriteria/{}.{}",
                            ad_group_id, criterion_id
                        ),
                        criterionId: criterion_id,
                        status: "ENABLED".to_string(),
                        keyword: Some(KeywordData {
                            text: (*keyword_text).to_string(),
                            matchType: "EXACT".to_string(),
                        }),
                        qualityScore: Some(rng.gen_range(1..=10)),
                        adGroupResourceName: ad_group_resource.clone(),
                    };

                    rows.push(GoogleAdsRow {
                        campaign: Some(campaign.clone()),
                        adGroup: Some(ad_group.clone()),
                        keywordView: None,
                        adGroupCriterion: Some(criterion),
                        metrics: Some(metrics),
                        segments: Some(SegmentsData {
                            date: Some(date_str.clone()),
                            device: Some("DESKTOP".to_string()),
                        }),
                    });
                }
            }
        }
        let Some(next) = current.checked_add_signed(Duration::days(1)) else {
            break;
        };
        current = next;
    }

    rows
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

fn build_quality_controls(
    report: &AnalyticsReport,
    provenance: &[SourceProvenance],
    budget_checks: Vec<QualityCheckV1>,
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
            code: "identity_campaign_rollup_reconciliation".to_string(),
            passed: (sum_campaign_spend - report.total_metrics.cost).abs()
                <= RECONCILIATION_EPSILON
                && (sum_campaign_revenue - report.total_metrics.conversions_value).abs()
                    <= RECONCILIATION_EPSILON,
            severity: "high".to_string(),
            observed: format!(
                "campaign_spend={:.4}, campaign_rev={:.4}, total_spend={:.4}, total_rev={:.4}",
                sum_campaign_spend,
                sum_campaign_revenue,
                report.total_metrics.cost,
                report.total_metrics.conversions_value
            ),
            expected: format!("abs(delta) <= {:.2}", RECONCILIATION_EPSILON),
        },
    ];
    let freshness_sla_checks = provenance
        .iter()
        .map(|item| QualityCheckV1 {
            code: format!("freshness_sla_{}", item.connector_id),
            passed: item.freshness_minutes <= 60,
            severity: "medium".to_string(),
            observed: format!("freshness={}m", item.freshness_minutes),
            expected: "freshness <= 60 minutes".to_string(),
        })
        .collect::<Vec<_>>();

    let is_healthy = schema_drift_checks
        .iter()
        .chain(identity_resolution_checks.iter())
        .chain(freshness_sla_checks.iter())
        .chain(budget_checks.iter())
        .all(|c| c.passed);

    AnalyticsQualityControlsV1 {
        schema_drift_checks,
        identity_resolution_checks,
        freshness_sla_checks,
        budget_checks,
        is_healthy,
    }
}

fn build_data_quality_summary(
    quality_controls: &AnalyticsQualityControlsV1,
) -> DataQualitySummaryV1 {
    let completeness_ratio = pass_ratio(&quality_controls.schema_drift_checks);
    let identity_join_coverage_ratio = pass_ratio(&quality_controls.identity_resolution_checks);
    let freshness_pass_ratio = pass_ratio(&quality_controls.freshness_sla_checks);
    let reconciliation_pass_ratio = quality_controls
        .identity_resolution_checks
        .iter()
        .find(|check| check.code == "identity_campaign_rollup_reconciliation")
        .map(|check| if check.passed { 1.0 } else { 0.0 })
        .unwrap_or(0.0);
    let budget_pass_ratio = pass_ratio(&quality_controls.budget_checks);

    let quality_score = round4(
        completeness_ratio * 0.30
            + identity_join_coverage_ratio * 0.25
            + freshness_pass_ratio * 0.15
            + reconciliation_pass_ratio * 0.15
            + budget_pass_ratio * 0.15,
    );

    DataQualitySummaryV1 {
        completeness_ratio,
        identity_join_coverage_ratio,
        freshness_pass_ratio,
        reconciliation_pass_ratio,
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

#[allow(dead_code)]
fn build_ga4_events(seed: u64) -> Vec<Ga4NormalizedEvent> {
    vec![Ga4NormalizedEvent {
        event_name: "purchase".to_string(),
        event_timestamp_utc: "deterministic-simulated".to_string(),
        session_id: format!("sess_{seed}"),
        user_pseudo_id: format!("user_{seed}"),
        traffic_source: Some("google".to_string()),
        medium: Some("cpc".to_string()),
        campaign: Some("spring_launch".to_string()),
        revenue_micros: Some(1_000_000),
        provenance: SourceProvenance {
            connector_id: "mock_analytics_connector_v1".to_string(),
            source_class: SourceClassLabel::Simulated,
            source_system: "mock_analytics".to_string(),
            collected_at_utc: "deterministic-simulated".to_string(),
            freshness_minutes: 0,
            validated_contract_version: Some(INGEST_CONTRACT_VERSION.to_string()),
            rejected_rows_count: 0,
            cleaning_note_count: 0,
        },
    }]
}

fn collect_ingest_cleaning_notes(seed: u64) -> Result<Vec<IngestCleaningNoteV1>, AnalyticsError> {
    let ga4_json = format!(
        r#"{{"event_name":" purchase ","event_timestamp_utc":"2026-02-16T00:00:00Z","user_pseudo_id":" user_{seed} ","session_id":"sess_{seed}","campaign":"spring_launch"}}"#
    );
    let parsed = parse_ga4_event(&ga4_json).map_err(|err| {
        AnalyticsError::new(
            "ingest_validation_failed",
            format!("failed to parse/normalize ga4 event: {}", err.reason),
            vec![err.field],
            None,
        )
    })?;
    Ok(map_ingest_notes("ga4", &parsed.notes))
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

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(
                artifact.budget.estimated.retrieval_units >= artifact.budget.actuals.retrieval_units
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
