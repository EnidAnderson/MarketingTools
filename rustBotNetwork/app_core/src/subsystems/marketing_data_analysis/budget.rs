use super::contracts::{
    AnalyticsError, BudgetActualsV1, BudgetEnvelopeV1, BudgetEventV1, BudgetPolicyModeV1,
    BudgetSummaryV1, MockAnalyticsRequestV1,
};
use chrono::NaiveDate;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

const COST_PER_RETRIEVAL_UNIT_MICROS: u64 = 200;
const COST_PER_ANALYSIS_UNIT_MICROS: u64 = 100;
const COST_PER_LLM_TOKEN_IN_MICROS: u64 = 3;
const COST_PER_LLM_TOKEN_OUT_MICROS: u64 = 6;
const RETRIEVAL_UNITS_PER_DAY: u64 = 128;
const ANALYSIS_UNITS_PER_DAY: u64 = 64;
const LLM_IN_WITH_NARRATIVES: u64 = 600;
const LLM_OUT_WITH_NARRATIVES: u64 = 380;
pub const HARD_DAILY_SPEND_CAP_MICROS: u64 = 10_000_000;
const DAILY_LEDGER_DEFAULT_PATH: &str = "data/analytics_runs/daily_spend_ledger_v1.json";

#[derive(Debug, Clone, Default)]
pub struct BudgetLedger {
    pub retrieval_units: u64,
    pub analysis_units: u64,
    pub llm_tokens_in: u64,
    pub llm_tokens_out: u64,
    pub total_cost_micros: u64,
}

impl BudgetLedger {
    pub fn into_actuals(self) -> BudgetActualsV1 {
        BudgetActualsV1 {
            retrieval_units: self.retrieval_units,
            analysis_units: self.analysis_units,
            llm_tokens_in: self.llm_tokens_in,
            llm_tokens_out: self.llm_tokens_out,
            total_cost_micros: self.total_cost_micros,
        }
    }
}

pub struct BudgetGuard {
    envelope: BudgetEnvelopeV1,
    ledger: BudgetLedger,
    events: Vec<BudgetEventV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetCategory {
    Retrieval,
    Analysis,
    LlmTokensIn,
    LlmTokensOut,
    CostMicros,
}

#[derive(Debug, Clone)]
pub struct BudgetEstimate {
    pub retrieval_units: u64,
    pub analysis_units: u64,
    pub llm_tokens_in: u64,
    pub llm_tokens_out: u64,
    pub total_cost_micros: u64,
}

impl BudgetEstimate {
    pub fn as_actuals(&self) -> BudgetActualsV1 {
        BudgetActualsV1 {
            retrieval_units: self.retrieval_units,
            analysis_units: self.analysis_units,
            llm_tokens_in: self.llm_tokens_in,
            llm_tokens_out: self.llm_tokens_out,
            total_cost_micros: self.total_cost_micros,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BudgetPlan {
    pub estimated: BudgetEstimate,
    pub effective_end: NaiveDate,
    pub include_narratives: bool,
    pub clipped: bool,
    pub sampled: bool,
    pub skipped_modules: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct DailyHardCapStatus {
    pub cap_micros: u64,
    pub spent_before_micros: u64,
    pub spent_after_micros: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DailySpendLedgerV1 {
    #[serde(default)]
    by_date: BTreeMap<String, u64>,
}

impl BudgetGuard {
    pub fn new(envelope: BudgetEnvelopeV1) -> Self {
        Self {
            envelope,
            ledger: BudgetLedger::default(),
            events: Vec::new(),
        }
    }

    pub fn policy(&self) -> BudgetPolicyModeV1 {
        self.envelope.policy.clone()
    }

    pub fn spend(
        &mut self,
        category: BudgetCategory,
        units: u64,
        subsystem: &str,
    ) -> Result<(), AnalyticsError> {
        let (remaining_before, exceeded, code) = match category {
            BudgetCategory::Retrieval => {
                let remaining = self
                    .envelope
                    .max_retrieval_units
                    .saturating_sub(self.ledger.retrieval_units);
                (
                    remaining,
                    self.ledger
                        .retrieval_units
                        .checked_add(units)
                        .map(|v| v > self.envelope.max_retrieval_units)
                        .unwrap_or(true),
                    "max_retrieval_units",
                )
            }
            BudgetCategory::Analysis => {
                let remaining = self
                    .envelope
                    .max_analysis_units
                    .saturating_sub(self.ledger.analysis_units);
                (
                    remaining,
                    self.ledger
                        .analysis_units
                        .checked_add(units)
                        .map(|v| v > self.envelope.max_analysis_units)
                        .unwrap_or(true),
                    "max_analysis_units",
                )
            }
            BudgetCategory::LlmTokensIn => {
                let remaining = self
                    .envelope
                    .max_llm_tokens_in
                    .saturating_sub(self.ledger.llm_tokens_in);
                (
                    remaining,
                    self.ledger
                        .llm_tokens_in
                        .checked_add(units)
                        .map(|v| v > self.envelope.max_llm_tokens_in)
                        .unwrap_or(true),
                    "max_llm_tokens_in",
                )
            }
            BudgetCategory::LlmTokensOut => {
                let remaining = self
                    .envelope
                    .max_llm_tokens_out
                    .saturating_sub(self.ledger.llm_tokens_out);
                (
                    remaining,
                    self.ledger
                        .llm_tokens_out
                        .checked_add(units)
                        .map(|v| v > self.envelope.max_llm_tokens_out)
                        .unwrap_or(true),
                    "max_llm_tokens_out",
                )
            }
            BudgetCategory::CostMicros => {
                let remaining = self
                    .envelope
                    .max_total_cost_micros
                    .saturating_sub(self.ledger.total_cost_micros);
                (
                    remaining,
                    self.ledger
                        .total_cost_micros
                        .checked_add(units)
                        .map(|v| v > self.envelope.max_total_cost_micros)
                        .unwrap_or(true),
                    "max_total_cost_micros",
                )
            }
        };

        if exceeded {
            self.events.push(BudgetEventV1 {
                subsystem: subsystem.to_string(),
                category: format!("{:?}", category).to_lowercase(),
                attempted_units: units,
                remaining_units_before: remaining_before,
                outcome: "blocked".to_string(),
                message: format!("budget cap exceeded for {}", code),
            });
            return Err(AnalyticsError::new(
                "budget_exceeded",
                format!("budget cap exceeded for {}", code),
                vec!["request.budget_envelope".to_string()],
                None,
            ));
        }

        match category {
            BudgetCategory::Retrieval => {
                self.ledger.retrieval_units = self.ledger.retrieval_units.saturating_add(units)
            }
            BudgetCategory::Analysis => {
                self.ledger.analysis_units = self.ledger.analysis_units.saturating_add(units)
            }
            BudgetCategory::LlmTokensIn => {
                self.ledger.llm_tokens_in = self.ledger.llm_tokens_in.saturating_add(units)
            }
            BudgetCategory::LlmTokensOut => {
                self.ledger.llm_tokens_out = self.ledger.llm_tokens_out.saturating_add(units)
            }
            BudgetCategory::CostMicros => {
                self.ledger.total_cost_micros = self.ledger.total_cost_micros.saturating_add(units)
            }
        }
        self.events.push(BudgetEventV1 {
            subsystem: subsystem.to_string(),
            category: format!("{:?}", category).to_lowercase(),
            attempted_units: units,
            remaining_units_before: remaining_before,
            outcome: "applied".to_string(),
            message: "budget spend accepted".to_string(),
        });
        Ok(())
    }

    pub fn summary(
        self,
        estimated: &BudgetEstimate,
        daily_status: DailyHardCapStatus,
        clipped: bool,
        sampled: bool,
        skipped_modules: Vec<String>,
        incomplete_output: bool,
    ) -> BudgetSummaryV1 {
        let actuals = self.ledger.into_actuals();
        let remaining = BudgetActualsV1 {
            retrieval_units: self
                .envelope
                .max_retrieval_units
                .saturating_sub(actuals.retrieval_units),
            analysis_units: self
                .envelope
                .max_analysis_units
                .saturating_sub(actuals.analysis_units),
            llm_tokens_in: self
                .envelope
                .max_llm_tokens_in
                .saturating_sub(actuals.llm_tokens_in),
            llm_tokens_out: self
                .envelope
                .max_llm_tokens_out
                .saturating_sub(actuals.llm_tokens_out),
            total_cost_micros: self
                .envelope
                .max_total_cost_micros
                .saturating_sub(actuals.total_cost_micros),
        };
        BudgetSummaryV1 {
            envelope: self.envelope,
            actuals,
            remaining,
            estimated: estimated.as_actuals(),
            hard_daily_cap_micros: daily_status.cap_micros,
            daily_spent_before_micros: daily_status.spent_before_micros,
            daily_spent_after_micros: daily_status.spent_after_micros,
            clipped,
            sampled,
            incomplete_output,
            skipped_modules,
            events: self.events,
        }
    }
}

pub fn estimate_budget_upper_bound(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
    include_narratives: bool,
) -> BudgetEstimate {
    let span_days = (end - start).num_days().max(0) as u64 + 1;
    let retrieval_units = span_days.saturating_mul(RETRIEVAL_UNITS_PER_DAY);
    let analysis_units = span_days.saturating_mul(ANALYSIS_UNITS_PER_DAY);
    let llm_tokens_in = if include_narratives {
        LLM_IN_WITH_NARRATIVES
    } else {
        0
    };
    let llm_tokens_out = if include_narratives {
        LLM_OUT_WITH_NARRATIVES
    } else {
        0
    };

    let mut total_cost_micros = retrieval_units.saturating_mul(COST_PER_RETRIEVAL_UNIT_MICROS);
    total_cost_micros = total_cost_micros
        .saturating_add(analysis_units.saturating_mul(COST_PER_ANALYSIS_UNIT_MICROS));
    total_cost_micros = total_cost_micros
        .saturating_add(llm_tokens_in.saturating_mul(COST_PER_LLM_TOKEN_IN_MICROS));
    total_cost_micros = total_cost_micros
        .saturating_add(llm_tokens_out.saturating_mul(COST_PER_LLM_TOKEN_OUT_MICROS));

    if request.campaign_filter.is_some() {
        total_cost_micros = total_cost_micros.saturating_sub(500);
    }
    if request.ad_group_filter.is_some() {
        total_cost_micros = total_cost_micros.saturating_sub(300);
    }

    BudgetEstimate {
        retrieval_units,
        analysis_units,
        llm_tokens_in,
        llm_tokens_out,
        total_cost_micros,
    }
}

pub fn build_budget_plan(
    request: &MockAnalyticsRequestV1,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<BudgetPlan, AnalyticsError> {
    let envelope = &request.budget_envelope;
    let full = estimate_budget_upper_bound(request, start, end, request.include_narratives);
    if estimate_fits_envelope(&full, envelope) {
        return Ok(BudgetPlan {
            estimated: full,
            effective_end: end,
            include_narratives: request.include_narratives,
            clipped: false,
            sampled: false,
            skipped_modules: Vec::new(),
        });
    }

    match envelope.policy {
        BudgetPolicyModeV1::FailClosed => Err(AnalyticsError::new(
            "budget_estimate_exceeds",
            "estimated run cost exceeds budget envelope",
            vec!["request.budget_envelope".to_string()],
            None,
        )),
        BudgetPolicyModeV1::Degrade => {
            let degraded = estimate_budget_upper_bound(request, start, end, false);
            if estimate_fits_envelope(&degraded, envelope) {
                return Ok(BudgetPlan {
                    estimated: degraded,
                    effective_end: end,
                    include_narratives: false,
                    clipped: false,
                    sampled: false,
                    skipped_modules: vec!["narratives".to_string()],
                });
            }
            let span_days = (end - start).num_days().max(0) as u64 + 1;
            let allowed_days = envelope.max_retrieval_units / RETRIEVAL_UNITS_PER_DAY;
            if allowed_days == 0 {
                return Err(AnalyticsError::new(
                    "budget_estimate_exceeds",
                    "degrade mode cannot fit retrieval floor within budget envelope",
                    vec!["request.budget_envelope.max_retrieval_units".to_string()],
                    None,
                ));
            }
            let clip_days = span_days.min(allowed_days);
            let effective_end = start
                .checked_add_days(chrono::Days::new(clip_days.saturating_sub(1)))
                .unwrap_or(end);
            let clipped = estimate_budget_upper_bound(request, start, effective_end, false);
            if !estimate_fits_envelope(&clipped, envelope) {
                return Err(AnalyticsError::new(
                    "budget_estimate_exceeds",
                    "degrade mode still exceeds budget envelope after clipping",
                    vec!["request.budget_envelope".to_string()],
                    None,
                ));
            }
            Ok(BudgetPlan {
                estimated: clipped,
                effective_end,
                include_narratives: false,
                clipped: clip_days < span_days,
                sampled: false,
                skipped_modules: vec!["narratives".to_string()],
            })
        }
        BudgetPolicyModeV1::Sample => {
            let allowed_days = envelope.max_retrieval_units / RETRIEVAL_UNITS_PER_DAY;
            if allowed_days == 0 {
                return Err(AnalyticsError::new(
                    "budget_estimate_exceeds",
                    "sample mode cannot fit retrieval floor within budget envelope",
                    vec!["request.budget_envelope.max_retrieval_units".to_string()],
                    None,
                ));
            }
            let span_days = (end - start).num_days().max(0) as u64 + 1;
            let sample_days = span_days.min(allowed_days);
            let effective_end = start
                .checked_add_days(chrono::Days::new(sample_days.saturating_sub(1)))
                .unwrap_or(end);
            let sampled = estimate_budget_upper_bound(request, start, effective_end, false);
            if !estimate_fits_envelope(&sampled, envelope) {
                return Err(AnalyticsError::new(
                    "budget_estimate_exceeds",
                    "sample mode still exceeds budget envelope",
                    vec!["request.budget_envelope".to_string()],
                    None,
                ));
            }
            Ok(BudgetPlan {
                estimated: sampled,
                effective_end,
                include_narratives: false,
                clipped: true,
                sampled: true,
                skipped_modules: vec!["narratives".to_string(), "full_window".to_string()],
            })
        }
    }
}

pub fn estimate_fits_envelope(estimate: &BudgetEstimate, envelope: &BudgetEnvelopeV1) -> bool {
    estimate.retrieval_units <= envelope.max_retrieval_units
        && estimate.analysis_units <= envelope.max_analysis_units
        && estimate.llm_tokens_in <= envelope.max_llm_tokens_in
        && estimate.llm_tokens_out <= envelope.max_llm_tokens_out
        && estimate.total_cost_micros <= envelope.max_total_cost_micros
}

pub fn enforce_daily_hard_cap(
    run_cost_micros: u64,
    run_day: NaiveDate,
) -> Result<DailyHardCapStatus, AnalyticsError> {
    let resolved = std::env::var("ANALYTICS_DAILY_BUDGET_LEDGER_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_daily_ledger_path());
    enforce_daily_hard_cap_at_path(run_cost_micros, run_day, &resolved)
}

fn default_daily_ledger_path() -> PathBuf {
    #[cfg(test)]
    {
        let exe_tag = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
            })
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("pid{}", std::process::id()));
        return std::env::temp_dir()
            .join(format!("nd_marketing_daily_spend_ledger_{}.json", exe_tag));
    }
    #[cfg(not(test))]
    {
        PathBuf::from(DAILY_LEDGER_DEFAULT_PATH)
    }
}

fn enforce_daily_hard_cap_at_path(
    run_cost_micros: u64,
    run_day: NaiveDate,
    ledger_path: &Path,
) -> Result<DailyHardCapStatus, AnalyticsError> {
    let lock_path = PathBuf::from(format!("{}.lock", ledger_path.display()));
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                AnalyticsError::internal(
                    "daily_budget_lock_parent_failed",
                    format!("failed to create daily budget lock directory: {err}"),
                )
            })?;
        }
    }
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|err| {
            AnalyticsError::internal(
                "daily_budget_lock_open_failed",
                format!("failed to open daily budget lock file: {err}"),
            )
        })?;
    lock_file.lock_exclusive().map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_lock_acquire_failed",
            format!("failed to acquire daily budget lock: {err}"),
        )
    })?;

    let mut ledger = read_daily_ledger(ledger_path)?;
    let key = run_day.format("%Y-%m-%d").to_string();
    let spent_before = ledger.by_date.get(&key).copied().unwrap_or(0);
    let spent_after = spent_before.saturating_add(run_cost_micros);
    if spent_after > HARD_DAILY_SPEND_CAP_MICROS {
        return Err(AnalyticsError::new(
            "daily_budget_hard_cap_exceeded",
            "daily hard cap of $10.00 reached; run blocked",
            vec!["budget.hard_daily_cap".to_string()],
            Some(serde_json::json!({
                "day": key,
                "cap_micros": HARD_DAILY_SPEND_CAP_MICROS,
                "spent_before_micros": spent_before,
                "attempted_additional_micros": run_cost_micros,
                "would_be_spent_after_micros": spent_after
            })),
        ));
    }

    ledger.by_date.insert(key, spent_after);
    write_daily_ledger(ledger_path, &ledger)?;
    let _ = lock_file.unlock();
    Ok(DailyHardCapStatus {
        cap_micros: HARD_DAILY_SPEND_CAP_MICROS,
        spent_before_micros: spent_before,
        spent_after_micros: spent_after,
    })
}

fn read_daily_ledger(path: &Path) -> Result<DailySpendLedgerV1, AnalyticsError> {
    if !path.exists() {
        return Ok(DailySpendLedgerV1::default());
    }
    let bytes = fs::read(path).map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_ledger_read_failed",
            format!("failed to read daily budget ledger: {err}"),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_ledger_parse_failed",
            format!("failed to parse daily budget ledger: {err}"),
        )
    })
}

fn write_daily_ledger(path: &Path, ledger: &DailySpendLedgerV1) -> Result<(), AnalyticsError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| {
                AnalyticsError::internal(
                    "daily_budget_ledger_parent_failed",
                    format!("failed to create daily budget ledger directory: {err}"),
                )
            })?;
        }
    }
    let tmp_path = PathBuf::from(format!("{}.tmp", path.display()));
    let payload = serde_json::to_vec_pretty(ledger).map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_ledger_serialize_failed",
            format!("failed to serialize daily budget ledger: {err}"),
        )
    })?;
    fs::write(&tmp_path, payload).map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_ledger_write_failed",
            format!("failed to write temporary daily budget ledger: {err}"),
        )
    })?;
    fs::rename(&tmp_path, path).map_err(|err| {
        AnalyticsError::internal(
            "daily_budget_ledger_rename_failed",
            format!("failed to commit daily budget ledger update: {err}"),
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn fail_closed_errors_when_estimate_exceeds() {
        let request = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-31".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(1),
            profile_id: "p1".to_string(),
            include_narratives: true,
            budget_envelope: BudgetEnvelopeV1 {
                max_retrieval_units: 10,
                max_analysis_units: 10,
                max_llm_tokens_in: 10,
                max_llm_tokens_out: 10,
                max_total_cost_micros: 10,
                policy: BudgetPolicyModeV1::FailClosed,
                provenance_ref: "unit".to_string(),
            },
        };
        let start = NaiveDate::parse_from_str(&request.start_date, "%Y-%m-%d").expect("date");
        let end = NaiveDate::parse_from_str(&request.end_date, "%Y-%m-%d").expect("date");
        let err = build_budget_plan(&request, start, end).expect_err("must fail");
        assert_eq!(err.code, "budget_estimate_exceeds");
    }

    proptest! {
        #[test]
        fn no_overrun_property_spend_never_exceeds(
            max_r in 1u64..10_000,
            max_a in 1u64..10_000,
            max_in in 1u64..10_000,
            max_out in 1u64..10_000,
            max_cost in 1u64..50_000_000,
            ops in prop::collection::vec((0u8..5, 0u64..20_000), 1..100),
        ) {
            let envelope = BudgetEnvelopeV1 {
                max_retrieval_units: max_r,
                max_analysis_units: max_a,
                max_llm_tokens_in: max_in,
                max_llm_tokens_out: max_out,
                max_total_cost_micros: max_cost,
                policy: BudgetPolicyModeV1::FailClosed,
                provenance_ref: "proptest".to_string(),
            };
            let mut guard = BudgetGuard::new(envelope.clone());
            for (cat, units) in ops {
                let category = match cat {
                    0 => BudgetCategory::Retrieval,
                    1 => BudgetCategory::Analysis,
                    2 => BudgetCategory::LlmTokensIn,
                    3 => BudgetCategory::LlmTokensOut,
                    _ => BudgetCategory::CostMicros,
                };
                let _ = guard.spend(category, units, "prop");
            }
            let summary = guard.summary(
                &BudgetEstimate {
                    retrieval_units: 0,
                    analysis_units: 0,
                    llm_tokens_in: 0,
                    llm_tokens_out: 0,
                    total_cost_micros: 0,
                },
                DailyHardCapStatus {
                    cap_micros: HARD_DAILY_SPEND_CAP_MICROS,
                    spent_before_micros: 0,
                    spent_after_micros: 0,
                },
                false,
                false,
                Vec::new(),
                false,
            );
            prop_assert!(summary.actuals.retrieval_units <= summary.envelope.max_retrieval_units);
            prop_assert!(summary.actuals.analysis_units <= summary.envelope.max_analysis_units);
            prop_assert!(summary.actuals.llm_tokens_in <= summary.envelope.max_llm_tokens_in);
            prop_assert!(summary.actuals.llm_tokens_out <= summary.envelope.max_llm_tokens_out);
            prop_assert!(summary.actuals.total_cost_micros <= summary.envelope.max_total_cost_micros);
        }
    }

    #[test]
    fn daily_hard_cap_blocks_when_over_ten_dollars() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ledger_path = temp.path().join("daily.json");
        let day = NaiveDate::parse_from_str("2026-02-17", "%Y-%m-%d").expect("day");

        let first = enforce_daily_hard_cap_at_path(6_000_000, day, &ledger_path).expect("first");
        assert_eq!(first.spent_after_micros, 6_000_000);

        let blocked = enforce_daily_hard_cap_at_path(5_000_000, day, &ledger_path)
            .expect_err("second should block");
        assert_eq!(blocked.code, "daily_budget_hard_cap_exceeded");
    }

    #[test]
    fn daily_hard_cap_resets_by_day() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ledger_path = temp.path().join("daily.json");
        let day1 = NaiveDate::parse_from_str("2026-02-17", "%Y-%m-%d").expect("day");
        let day2 = NaiveDate::parse_from_str("2026-02-18", "%Y-%m-%d").expect("day");

        let d1 = enforce_daily_hard_cap_at_path(9_000_000, day1, &ledger_path).expect("day1");
        let d2 = enforce_daily_hard_cap_at_path(9_000_000, day2, &ledger_path).expect("day2");
        assert_eq!(d1.spent_after_micros, 9_000_000);
        assert_eq!(d2.spent_after_micros, 9_000_000);
    }

    #[test]
    fn daily_hard_cap_fails_closed_on_corrupt_ledger() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ledger_path = temp.path().join("daily.json");
        fs::write(&ledger_path, "{bad json").expect("write corrupt");
        let day = NaiveDate::parse_from_str("2026-02-17", "%Y-%m-%d").expect("day");
        let err =
            enforce_daily_hard_cap_at_path(1_000_000, day, &ledger_path).expect_err("must fail");
        assert_eq!(err.code, "daily_budget_ledger_parse_failed");
    }

    #[test]
    fn daily_hard_cap_is_atomic_under_parallel_attempts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ledger_path = temp.path().join("daily.json");
        let day = NaiveDate::parse_from_str("2026-02-17", "%Y-%m-%d").expect("day");
        let workers = 5usize;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::new();
        for _ in 0..workers {
            let barrier = Arc::clone(&barrier);
            let ledger_path = ledger_path.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                enforce_daily_hard_cap_at_path(3_000_000, day, &ledger_path).is_ok()
            }));
        }
        let mut success_count = 0usize;
        for handle in handles {
            if handle.join().expect("thread join") {
                success_count += 1;
            }
        }
        assert_eq!(
            success_count, 3,
            "only three 3M reservations fit within 10M"
        );
        let ledger = read_daily_ledger(&ledger_path).expect("read ledger");
        let spent = ledger.by_date.get("2026-02-17").copied().unwrap_or(0);
        assert_eq!(spent, 9_000_000);
    }
}
