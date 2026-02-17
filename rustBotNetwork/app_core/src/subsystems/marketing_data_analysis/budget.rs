use super::contracts::{
    AnalyticsError, BudgetActualsV1, BudgetEnvelopeV1, BudgetEventV1, BudgetPolicyModeV1,
    BudgetSummaryV1, MockAnalyticsRequestV1,
};
use chrono::NaiveDate;

const COST_PER_RETRIEVAL_UNIT_MICROS: u64 = 200;
const COST_PER_ANALYSIS_UNIT_MICROS: u64 = 100;
const COST_PER_LLM_TOKEN_IN_MICROS: u64 = 3;
const COST_PER_LLM_TOKEN_OUT_MICROS: u64 = 6;
const RETRIEVAL_UNITS_PER_DAY: u64 = 128;
const ANALYSIS_UNITS_PER_DAY: u64 = 64;
const LLM_IN_WITH_NARRATIVES: u64 = 600;
const LLM_OUT_WITH_NARRATIVES: u64 = 380;

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
    total_cost_micros =
        total_cost_micros.saturating_add(analysis_units.saturating_mul(COST_PER_ANALYSIS_UNIT_MICROS));
    total_cost_micros =
        total_cost_micros.saturating_add(llm_tokens_in.saturating_mul(COST_PER_LLM_TOKEN_IN_MICROS));
    total_cost_micros =
        total_cost_micros.saturating_add(llm_tokens_out.saturating_mul(COST_PER_LLM_TOKEN_OUT_MICROS));

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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
}
