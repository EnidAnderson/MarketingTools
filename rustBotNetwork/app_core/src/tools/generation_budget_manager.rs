//! Spend governor for all paid provider API calls.
//!
//! Policy:
//! 1. Hard daily cap is enforced at `$10.00` (`HARD_DAILY_BUDGET_USD`).
//! 2. Any configured daily cap above hard cap is rejected (fail-closed).
//! 3. Paid calls must reserve budget before network requests.
//! 4. Preferred API for callers is `PaidCallPermit::reserve(...)`.
//! 5. `PaidCallPermit` auto-refunds on drop unless explicitly committed.

use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const DEFAULT_API_COSTS_FILE: &str = "src/data/api_costs.json";
const BUDGET_FILE_PATH_ENV: &str = "GENERATION_BUDGET_FILE_PATH";
const RUNTIME_DIR_ENV: &str = "ND_RUNTIME_DIR";
const DEFAULT_BUDGET_FILENAME: &str = "generation_budget_v1.json";
const DAILY_BUDGET_ENV: &str = "DAILY_BUDGET_USD";

pub const HARD_DAILY_BUDGET_USD: f64 = 10.0;
pub const DEFAULT_DAILY_BUDGET_USD: f64 = 10.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetState {
    pub daily_spend: f64,
    pub daily_resets_on: String,
    pub generations: Vec<GenerationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationRecord {
    pub timestamp: String,
    pub tool: String,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ApiCosts {
    #[serde(flatten)]
    costs: HashMap<String, ApiCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiCost {
    #[serde(default)]
    pub input: Option<f64>,
    #[serde(default)]
    pub output: Option<f64>,
    #[serde(default)]
    pub per_image: Option<f64>,
    #[serde(default)]
    pub per_video: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendRequest {
    pub tool: String,
    pub provider: String,
    pub model: String,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendReservation {
    pub timestamp_utc: String,
    pub tool: String,
    pub provider: String,
    pub model: String,
    pub reserved_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendGuardError {
    pub code: String,
    pub message: String,
}

impl SpendGuardError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SpendGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SpendGuardError {}

/// # NDOC
/// component: `tools::generation_budget_manager::paid_call_permit`
/// purpose: Fail-safe reservation guard for paid API calls.
/// invariants:
///   - Reservation is created before network call.
///   - If guard is dropped without commit, reservation is automatically refunded.
///   - Commit is explicit and one-way.
pub struct PaidCallPermit {
    reservation: Option<SpendReservation>,
    committed: bool,
}

impl PaidCallPermit {
    pub fn reserve(
        cost_usd: f64,
        tool: &str,
        provider: &str,
        model: &str,
    ) -> Result<Self, SpendGuardError> {
        let reservation = reserve_for_paid_call(cost_usd, tool, provider, model)?;
        Ok(Self {
            reservation: Some(reservation),
            committed: false,
        })
    }

    pub fn commit(mut self) -> Result<(), SpendGuardError> {
        let reservation = self.reservation.take().ok_or_else(|| {
            SpendGuardError::new("invalid_paid_call_permit", "missing reservation")
        })?;
        commit_paid_call(&reservation)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for PaidCallPermit {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(reservation) = self.reservation.take() {
            let _ = refund_paid_call(&reservation);
        }
    }
}

/// # NDOC
/// component: `tools::generation_budget_manager::spend_governor`
/// purpose: Shared trait for reserving and recording paid provider usage.
/// invariants:
///   - Reserve must fail closed when daily budget policy is invalid or exceeded.
///   - Configured daily budget cap may not exceed HARD_DAILY_BUDGET_USD.
pub trait SpendGovernor: Send + Sync {
    fn reserve(&self, request: SpendRequest) -> Result<SpendReservation, SpendGuardError>;
    fn commit(&self, reservation: &SpendReservation) -> Result<(), SpendGuardError>;
    fn refund(&self, reservation: &SpendReservation) -> Result<(), SpendGuardError>;
    fn status(&self) -> Result<BudgetState, SpendGuardError>;
}

#[derive(Debug, Clone)]
pub struct FileSpendGovernor {
    budget_file_path: PathBuf,
}

impl Default for FileSpendGovernor {
    fn default() -> Self {
        Self {
            budget_file_path: get_budget_file_path(),
        }
    }
}

impl FileSpendGovernor {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            budget_file_path: path.into(),
        }
    }
}

impl SpendGovernor for FileSpendGovernor {
    fn reserve(&self, request: SpendRequest) -> Result<SpendReservation, SpendGuardError> {
        validate_spend_request(&request)?;
        let cap = configured_daily_budget_usd()?;
        with_locked_budget_state(&self.budget_file_path, |state| {
            ensure_budget_day(state);
            if state.daily_spend + request.estimated_cost_usd > cap {
                return Err(SpendGuardError::new(
                    "daily_budget_exceeded",
                    format!(
                        "daily cap ${cap:.2} would be exceeded by spend ${:.4} + ${:.4}",
                        state.daily_spend, request.estimated_cost_usd
                    ),
                ));
            }
            state.daily_spend += request.estimated_cost_usd;
            let reservation = SpendReservation {
                timestamp_utc: Utc::now().to_rfc3339(),
                tool: request.tool.clone(),
                provider: request.provider.clone(),
                model: request.model.clone(),
                reserved_cost_usd: request.estimated_cost_usd,
            };
            state.generations.push(GenerationRecord {
                timestamp: reservation.timestamp_utc.clone(),
                tool: format!(
                    "reserve:{}:{}:{}",
                    reservation.tool, reservation.provider, reservation.model
                ),
                cost: reservation.reserved_cost_usd,
            });
            Ok(reservation)
        })
    }

    fn commit(&self, reservation: &SpendReservation) -> Result<(), SpendGuardError> {
        with_locked_budget_state(&self.budget_file_path, |state| {
            ensure_budget_day(state);
            state.generations.push(GenerationRecord {
                timestamp: Utc::now().to_rfc3339(),
                tool: format!(
                    "commit:{}:{}:{}",
                    reservation.tool, reservation.provider, reservation.model
                ),
                cost: reservation.reserved_cost_usd,
            });
            Ok(())
        })
    }

    fn refund(&self, reservation: &SpendReservation) -> Result<(), SpendGuardError> {
        with_locked_budget_state(&self.budget_file_path, |state| {
            ensure_budget_day(state);
            state.daily_spend = (state.daily_spend - reservation.reserved_cost_usd).max(0.0);
            state.generations.push(GenerationRecord {
                timestamp: Utc::now().to_rfc3339(),
                tool: format!(
                    "refund:{}:{}:{}",
                    reservation.tool, reservation.provider, reservation.model
                ),
                cost: reservation.reserved_cost_usd,
            });
            Ok(())
        })
    }

    fn status(&self) -> Result<BudgetState, SpendGuardError> {
        with_locked_budget_state(&self.budget_file_path, |state| {
            ensure_budget_day(state);
            Ok(state.clone())
        })
    }
}

#[cfg(test)]
thread_local! {
    static TEST_BUDGET_FILE_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    static TEST_API_COSTS_FILE_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn set_test_budget_file_path(path: Option<PathBuf>) {
    TEST_BUDGET_FILE_PATH.with(|slot| *slot.borrow_mut() = path);
}

#[cfg(test)]
pub fn set_test_api_costs_file_path(path: Option<PathBuf>) {
    TEST_API_COSTS_FILE_PATH.with(|slot| *slot.borrow_mut() = path);
}

fn get_budget_file_path() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(path) = TEST_BUDGET_FILE_PATH.with(|slot| slot.borrow().clone()) {
            return path;
        }
    }
    if let Ok(path) = std::env::var(BUDGET_FILE_PATH_ENV) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    runtime_state_root().join(DEFAULT_BUDGET_FILENAME)
}

fn get_api_costs_file_path() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(path) = TEST_API_COSTS_FILE_PATH.with(|slot| slot.borrow().clone()) {
            return path;
        }
    }
    PathBuf::from(DEFAULT_API_COSTS_FILE)
}

fn runtime_state_root() -> PathBuf {
    if let Ok(path) = std::env::var(RUNTIME_DIR_ENV) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join(".natures_diet_runtime");
        }
    }
    PathBuf::from(".natures_diet_runtime")
}

fn configured_daily_budget_usd() -> Result<f64, SpendGuardError> {
    let raw =
        std::env::var(DAILY_BUDGET_ENV).unwrap_or_else(|_| DEFAULT_DAILY_BUDGET_USD.to_string());
    let parsed = raw.parse::<f64>().map_err(|_| {
        SpendGuardError::new(
            "invalid_daily_budget",
            format!("{DAILY_BUDGET_ENV} must be a numeric USD value"),
        )
    })?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(SpendGuardError::new(
            "invalid_daily_budget",
            format!("{DAILY_BUDGET_ENV} must be > 0"),
        ));
    }
    if parsed > HARD_DAILY_BUDGET_USD {
        return Err(SpendGuardError::new(
            "hard_daily_budget_violation",
            format!(
                "{DAILY_BUDGET_ENV}=${parsed:.2} exceeds enforced hard cap ${HARD_DAILY_BUDGET_USD:.2}"
            ),
        ));
    }
    Ok(parsed)
}

fn validate_spend_request(request: &SpendRequest) -> Result<(), SpendGuardError> {
    if request.tool.trim().is_empty()
        || request.provider.trim().is_empty()
        || request.model.trim().is_empty()
    {
        return Err(SpendGuardError::new(
            "invalid_spend_request",
            "tool/provider/model are required",
        ));
    }
    if !request.estimated_cost_usd.is_finite() || request.estimated_cost_usd < 0.0 {
        return Err(SpendGuardError::new(
            "invalid_spend_request",
            "estimated_cost_usd must be finite and >= 0",
        ));
    }
    Ok(())
}

fn default_budget_state() -> BudgetState {
    BudgetState {
        daily_spend: 0.0,
        daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
        generations: Vec::new(),
    }
}

fn ensure_budget_day(state: &mut BudgetState) {
    let today = Utc::now().date_naive().to_string();
    if today >= state.daily_resets_on {
        state.daily_spend = 0.0;
        state.daily_resets_on = (Utc::now().date_naive() + Duration::days(1)).to_string();
        let one_month_ago = Utc::now() - Duration::days(30);
        state.generations.retain(|record| {
            DateTime::parse_from_rfc3339(&record.timestamp)
                .map(|dt| dt.with_timezone(&Utc) > one_month_ago)
                .unwrap_or(false)
        });
    }
}

fn ensure_parent(path: &Path) -> Result<(), SpendGuardError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| {
        SpendGuardError::new(
            "budget_dir_create_failed",
            format!("failed to create budget directory: {err}"),
        )
    })
}

fn with_locked_budget_state<T>(
    path: &Path,
    f: impl FnOnce(&mut BudgetState) -> Result<T, SpendGuardError>,
) -> Result<T, SpendGuardError> {
    ensure_parent(path)?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(|err| {
            SpendGuardError::new(
                "budget_file_open_failed",
                format!("failed to open budget file: {err}"),
            )
        })?;
    file.lock_exclusive().map_err(|err| {
        SpendGuardError::new(
            "budget_file_lock_failed",
            format!("failed to lock budget file: {err}"),
        )
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|err| {
        SpendGuardError::new(
            "budget_file_read_failed",
            format!("failed to read budget file: {err}"),
        )
    })?;
    let mut state = if contents.trim().is_empty() {
        default_budget_state()
    } else {
        serde_json::from_str::<BudgetState>(&contents).map_err(|err| {
            SpendGuardError::new(
                "budget_file_parse_failed",
                format!("failed to parse budget file: {err}"),
            )
        })?
    };

    let result = f(&mut state)?;

    let serialized = serde_json::to_string_pretty(&state).map_err(|err| {
        SpendGuardError::new(
            "budget_file_serialize_failed",
            format!("failed to serialize budget state: {err}"),
        )
    })?;
    file.set_len(0)
        .and_then(|_| file.seek(SeekFrom::Start(0)))
        .and_then(|_| file.write_all(serialized.as_bytes()))
        .map_err(|err| {
            SpendGuardError::new(
                "budget_file_write_failed",
                format!("failed to write budget file: {err}"),
            )
        })?;

    Ok(result)
}

fn load_api_costs() -> Result<HashMap<String, ApiCost>, SpendGuardError> {
    let path = get_api_costs_file_path();
    let contents = fs::read_to_string(&path).map_err(|err| {
        SpendGuardError::new(
            "api_costs_read_failed",
            format!("failed to read API costs file '{}': {err}", path.display()),
        )
    })?;
    let parsed: ApiCosts = serde_json::from_str(&contents).map_err(|err| {
        SpendGuardError::new(
            "api_costs_parse_failed",
            format!("failed to parse API costs file '{}': {err}", path.display()),
        )
    })?;
    Ok(parsed.costs)
}

pub fn estimate_llm_cost(model_name: &str, input_text: &str, output_text: &str) -> f64 {
    let Ok(costs) = load_api_costs() else {
        return 0.0;
    };
    let Some(model) = costs.get(model_name) else {
        return 0.0;
    };
    let input_rate = model.input.unwrap_or(0.0);
    let output_rate = model.output.unwrap_or(0.0);
    let input_tokens = input_text.len() as f64 / 4.0;
    let output_tokens = output_text.len() as f64 / 4.0;
    (input_tokens * input_rate) + (output_tokens * output_rate)
}

pub fn estimate_llm_cost_strict(
    model_name: &str,
    input_text: &str,
    output_text: &str,
) -> Result<f64, SpendGuardError> {
    let costs = load_api_costs()?;
    let model = costs.get(model_name).ok_or_else(|| {
        SpendGuardError::new(
            "unknown_model_cost",
            format!("model '{model_name}' not found in api_costs.json"),
        )
    })?;
    let input_rate = model.input.ok_or_else(|| {
        SpendGuardError::new(
            "missing_model_input_rate",
            format!("model '{model_name}' missing input token rate"),
        )
    })?;
    let output_rate = model.output.ok_or_else(|| {
        SpendGuardError::new(
            "missing_model_output_rate",
            format!("model '{model_name}' missing output token rate"),
        )
    })?;
    let input_tokens = input_text.len() as f64 / 4.0;
    let output_tokens = output_text.len() as f64 / 4.0;
    Ok((input_tokens * input_rate) + (output_tokens * output_rate))
}

pub fn estimate_embedding_cost(model_name: &str, text: &str) -> f64 {
    let Ok(costs) = load_api_costs() else {
        return 0.0;
    };
    let Some(model) = costs.get(model_name) else {
        return 0.0;
    };
    let input_rate = model.input.unwrap_or(0.0);
    (text.len() as f64 / 4.0) * input_rate
}

pub fn estimate_image_cost_strict(model_name: &str) -> Result<f64, SpendGuardError> {
    let costs = load_api_costs()?;
    let model = costs.get(model_name).ok_or_else(|| {
        SpendGuardError::new(
            "unknown_model_cost",
            format!("model '{model_name}' not found in api_costs.json"),
        )
    })?;
    model.per_image.ok_or_else(|| {
        SpendGuardError::new(
            "missing_model_image_rate",
            format!("model '{model_name}' missing per_image rate"),
        )
    })
}

pub fn reserve_for_paid_call(
    cost_usd: f64,
    tool: &str,
    provider: &str,
    model: &str,
) -> Result<SpendReservation, SpendGuardError> {
    FileSpendGovernor::default().reserve(SpendRequest {
        tool: tool.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        estimated_cost_usd: cost_usd,
    })
}

pub fn commit_paid_call(reservation: &SpendReservation) -> Result<(), SpendGuardError> {
    FileSpendGovernor::default().commit(reservation)
}

pub fn refund_paid_call(reservation: &SpendReservation) -> Result<(), SpendGuardError> {
    FileSpendGovernor::default().refund(reservation)
}

pub fn can_generate(cost: f64) -> bool {
    let cap = match configured_daily_budget_usd() {
        Ok(v) => v,
        Err(_) => return false,
    };
    match FileSpendGovernor::default().status() {
        Ok(state) => state.daily_spend + cost <= cap,
        Err(_) => false,
    }
}

pub fn record_generation(cost: f64, tool_name: &str) {
    if let Ok(reservation) = reserve_for_paid_call(cost, tool_name, "unknown", "unknown") {
        let _ = commit_paid_call(&reservation);
    }
}

pub fn get_budget_state() -> BudgetState {
    FileSpendGovernor::default()
        .status()
        .unwrap_or_else(|_| default_budget_state())
}

pub fn save_budget_state(state: &BudgetState) {
    let path = get_budget_file_path();
    if ensure_parent(&path).is_err() {
        return;
    }
    if let Ok(mut file) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
    {
        if file.lock_exclusive().is_ok() {
            if let Ok(payload) = serde_json::to_string_pretty(state) {
                let _ = file
                    .set_len(0)
                    .and_then(|_| file.seek(SeekFrom::Start(0)))
                    .and_then(|_| file.write_all(payload.as_bytes()));
            }
        }
    }
}

pub fn get_budget_status() -> String {
    let cap = match configured_daily_budget_usd() {
        Ok(v) => v,
        Err(err) => {
            return format!("Budget unavailable: {}", err.message);
        }
    };
    let state = get_budget_state();
    format!("Daily spend: ${:.4} / ${:.2}", state.daily_spend, cap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_api_costs(path: &Path, content: &str) {
        let mut file = fs::File::create(path).expect("create api costs");
        file.write_all(content.as_bytes()).expect("write api costs");
    }

    #[test]
    fn hard_budget_cap_rejects_values_above_ten_dollars() {
        std::env::set_var("DAILY_BUDGET_USD", "10.01");
        let err = configured_daily_budget_usd().expect_err("must reject cap above hard limit");
        assert_eq!(err.code, "hard_daily_budget_violation");
        std::env::remove_var("DAILY_BUDGET_USD");
    }

    #[test]
    fn reserve_commit_and_refund_round_trip() {
        let dir = tempdir().expect("tempdir");
        set_test_budget_file_path(Some(dir.path().join("budget.json")));
        std::env::set_var("DAILY_BUDGET_USD", "10.0");

        let reservation =
            reserve_for_paid_call(2.5, "image_tool", "openai", "gpt-image").expect("reserve");
        commit_paid_call(&reservation).expect("commit");
        let state = get_budget_state();
        assert_eq!(state.daily_spend, 2.5);

        refund_paid_call(&reservation).expect("refund");
        let state_after_refund = get_budget_state();
        assert_eq!(state_after_refund.daily_spend, 0.0);

        std::env::remove_var("DAILY_BUDGET_USD");
        set_test_budget_file_path(None);
    }

    #[test]
    fn reserve_fails_when_daily_budget_exceeded() {
        let dir = tempdir().expect("tempdir");
        set_test_budget_file_path(Some(dir.path().join("budget.json")));
        std::env::set_var("DAILY_BUDGET_USD", "10.0");
        save_budget_state(&BudgetState {
            daily_spend: 9.5,
            daily_resets_on: (Utc::now().date_naive() + Duration::days(1)).to_string(),
            generations: vec![],
        });

        let err = reserve_for_paid_call(0.6, "text_tool", "openai", "gpt-mini")
            .expect_err("reserve should fail");
        assert_eq!(err.code, "daily_budget_exceeded");

        std::env::remove_var("DAILY_BUDGET_USD");
        set_test_budget_file_path(None);
    }

    #[test]
    fn budget_file_path_uses_env_override_when_set() {
        std::env::set_var(BUDGET_FILE_PATH_ENV, "/tmp/nd-budget-test.json");
        let path = get_budget_file_path();
        assert_eq!(path, PathBuf::from("/tmp/nd-budget-test.json"));
        std::env::remove_var(BUDGET_FILE_PATH_ENV);
    }

    #[test]
    fn estimate_image_cost_strict_reads_per_image_rates() {
        let dir = tempdir().expect("tempdir");
        let costs_path = dir.path().join("api_costs.json");
        write_api_costs(
            &costs_path,
            r#"{"img-model":{"per_image":0.009},"llm-model":{"input":0.001,"output":0.002}}"#,
        );
        set_test_api_costs_file_path(Some(costs_path));

        let rate = estimate_image_cost_strict("img-model").expect("image rate");
        assert!((rate - 0.009).abs() < 1e-12);

        set_test_api_costs_file_path(None);
    }

    #[test]
    fn estimate_llm_cost_strict_requires_model_rates() {
        let dir = tempdir().expect("tempdir");
        let costs_path = dir.path().join("api_costs.json");
        write_api_costs(
            &costs_path,
            r#"{"llm-model":{"input":0.001,"output":0.002}}"#,
        );
        set_test_api_costs_file_path(Some(costs_path));

        let value = estimate_llm_cost_strict("llm-model", "abcd", "abcd").expect("llm cost");
        assert!(value > 0.0);

        set_test_api_costs_file_path(None);
    }

    #[test]
    fn paid_api_modules_must_reference_spend_guard() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let files = [
            "image_generator.rs",
            "llm_client.rs",
            "tools/image_generation.rs",
        ];
        for file in files {
            let full = root.join(file);
            let text = fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", full.display()));
            assert!(
                text.contains("PaidCallPermit::reserve(")
                    || text.contains("reserve_for_paid_call("),
                "paid API module '{}' must reserve spend via PaidCallPermit::reserve",
                file
            );
        }
    }

    #[test]
    fn any_module_with_paid_api_key_usage_must_reference_spend_guard() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let key_markers = [
            "OPENAI_API_KEY",
            "GEMINI_API_KEY",
            "GOOGLE_API_KEY",
            "STABILITY_API_KEY",
        ];
        let mut stack = vec![root.clone()];
        while let Some(path) = stack.pop() {
            let read_dir = fs::read_dir(&path)
                .unwrap_or_else(|e| panic!("failed to read dir {}: {e}", path.display()));
            for entry in read_dir {
                let entry = entry.expect("dir entry");
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                let text = fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", p.display()));
                let has_paid_key = key_markers.iter().any(|marker| text.contains(marker));
                if has_paid_key {
                    assert!(
                        text.contains("PaidCallPermit::reserve(")
                            || text.contains("reserve_for_paid_call("),
                        "module '{}' references paid API keys but does not reserve spend",
                        p.display()
                    );
                }
            }
        }
    }

    #[test]
    fn permit_auto_refunds_when_not_committed() {
        let dir = tempdir().expect("tempdir");
        set_test_budget_file_path(Some(dir.path().join("budget.json")));
        std::env::set_var("DAILY_BUDGET_USD", "10.0");

        {
            let _permit = PaidCallPermit::reserve(1.0, "test_tool", "openai", "gpt-test")
                .expect("permit reserve");
            let mid = get_budget_state();
            assert_eq!(mid.daily_spend, 1.0);
        }

        let after = get_budget_state();
        assert_eq!(after.daily_spend, 0.0);

        std::env::remove_var("DAILY_BUDGET_USD");
        set_test_budget_file_path(None);
    }

    #[test]
    fn permit_commit_persists_spend() {
        let dir = tempdir().expect("tempdir");
        set_test_budget_file_path(Some(dir.path().join("budget.json")));
        std::env::set_var("DAILY_BUDGET_USD", "10.0");

        let permit =
            PaidCallPermit::reserve(1.25, "test_tool", "openai", "gpt-test").expect("reserve");
        permit.commit().expect("commit");

        let state = get_budget_state();
        assert_eq!(state.daily_spend, 1.25);

        std::env::remove_var("DAILY_BUDGET_USD");
        set_test_budget_file_path(None);
    }
}
