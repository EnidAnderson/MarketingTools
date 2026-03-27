use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::time::{sleep, Duration};

/// # NDOC
/// component: `subsystems::mailchimp_backfill`
/// purpose: End-to-end Wix -> Mailchimp historical commerce backfill with dry-run and resumable write phases.
/// invariants:
///   - Dry-run never performs Mailchimp writes.
///   - Pilot/full runs require `MAILCHIMP_ALLOW_WRITES=true`.
///   - Contact creation preserves stronger existing Mailchimp audience status.
///   - Order ids, customer ids, product ids, and variant ids are deterministic and rerunnable.

const DEFAULT_MAILCHIMP_TIMEOUT_SECS: u64 = 60;
const DEFAULT_WIX_PAGE_LIMIT: u64 = 100;
const DEFAULT_WRITE_BATCH_SIZE: usize = 500;
const DEFAULT_FALLBACK_STORE_ID: &str = "wix_backfill_v1";
const DEFAULT_OUTPUT_DIR: &str = "exports/mailchimp_backfill";
const MAILCHIMP_BATCH_POLL_INTERVAL_SECS: u64 = 2;

#[derive(Debug, Error)]
pub enum BackfillError {
    #[error("{0}")]
    Message(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackfillPhaseV1 {
    DryRun,
    Pilot,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceModeV1 {
    WixApi,
    WixCsv,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackfillBudgetEnvelopeV1 {
    pub max_source_page_size: u64,
    pub max_orders_per_run: u64,
    pub max_orders_per_write_batch: usize,
}

impl Default for BackfillBudgetEnvelopeV1 {
    fn default() -> Self {
        Self {
            max_source_page_size: DEFAULT_WIX_PAGE_LIMIT,
            max_orders_per_run: 500_000,
            max_orders_per_write_batch: DEFAULT_WRITE_BATCH_SIZE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailchimpBackfillConfigV1 {
    pub api_key_env_var: String,
    pub api_key: String,
    pub server_prefix: String,
    pub audience_id: String,
    pub store_id: String,
    pub allow_writes: bool,
    pub confirm_automations_paused: bool,
    pub fallback_store_id: String,
    pub wix_site_id: String,
    pub wix_api_token: Option<String>,
    pub budget: BackfillBudgetEnvelopeV1,
}

impl MailchimpBackfillConfigV1 {
    pub fn from_env() -> Result<Self, BackfillError> {
        let api_key = required_env("MAILCHIMP_API_KEY")?;
        let server_prefix = env::var("MAILCHIMP_SERVER_PREFIX")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                api_key
                    .split('-')
                    .nth(1)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            });
        if server_prefix.is_empty() {
            return Err(BackfillError::Message(
                "MAILCHIMP_SERVER_PREFIX is required when it cannot be derived from MAILCHIMP_API_KEY"
                    .to_string(),
            ));
        }
        Ok(Self {
            api_key_env_var: "MAILCHIMP_API_KEY".to_string(),
            api_key,
            server_prefix,
            audience_id: required_env("MAILCHIMP_AUDIENCE_ID")?,
            store_id: required_env("MAILCHIMP_STORE_ID")?,
            allow_writes: env_bool("MAILCHIMP_ALLOW_WRITES", false)?,
            confirm_automations_paused: env_bool("MAILCHIMP_CONFIRM_AUTOMATIONS_PAUSED", false)?,
            fallback_store_id: env::var("MAILCHIMP_FALLBACK_STORE_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_FALLBACK_STORE_ID.to_string()),
            wix_site_id: env::var("WIX_SITE_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown_wix_site".to_string()),
            wix_api_token: env::var("WIX_API_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            budget: BackfillBudgetEnvelopeV1::default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillRunOptionsV1 {
    pub phase: BackfillPhaseV1,
    pub source_mode: SourceModeV1,
    pub csv_path: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub all_history: bool,
    pub max_orders: Option<usize>,
    pub pilot_limit: usize,
    pub allow_existing_store_write: bool,
}

impl Default for BackfillRunOptionsV1 {
    fn default() -> Self {
        Self {
            phase: BackfillPhaseV1::DryRun,
            source_mode: SourceModeV1::WixApi,
            csv_path: None,
            out_dir: PathBuf::from(DEFAULT_OUTPUT_DIR),
            all_history: true,
            max_orders: None,
            pilot_limit: 100,
            allow_existing_store_write: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WixOrderBackfillV1 {
    pub order_id: String,
    pub created_at_utc: String,
    pub paid_at_utc: Option<String>,
    pub order_status: String,
    pub financial_status: Option<String>,
    pub fulfillment_status: Option<String>,
    pub customer_email: String,
    pub customer_first_name: Option<String>,
    pub customer_last_name: Option<String>,
    pub currency: String,
    pub total: Decimal,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub shipping_total: Decimal,
    pub discount_total: Decimal,
    pub line_items: Vec<WixOrderLineItemBackfillV1>,
    pub shipping_address: Option<BackfillAddressV1>,
    pub billing_address: Option<BackfillAddressV1>,
    pub source_payload_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WixOrderLineItemBackfillV1 {
    pub line_id: String,
    pub product_id: String,
    pub variant_id: String,
    pub title: String,
    pub sku: Option<String>,
    pub quantity: u32,
    pub unit_price: Decimal,
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillAddressV1 {
    pub name: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunManifestV1 {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub phase: BackfillPhaseV1,
    pub source_mode: SourceModeV1,
    pub target_audience_id: String,
    pub requested_store_id: String,
    pub effective_store_id: String,
    pub effective_store_mode: String,
    pub order_count: usize,
    pub unique_customers: usize,
    pub unique_products: usize,
    pub unique_variants: usize,
    pub gross_revenue_by_currency: BTreeMap<String, String>,
    pub status_breakdown: BTreeMap<String, usize>,
    pub skipped_by_reason: BTreeMap<String, usize>,
    pub write_block_reasons: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationRowV1 {
    pub order_id: String,
    pub customer_email: String,
    pub currency: String,
    pub source_total: String,
    pub imported: bool,
    pub reason: String,
    pub effective_store_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyReportV1 {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub effective_store_id: String,
    pub checked_orders: usize,
    pub duplicate_source_order_ids: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct StoreTargetDecision {
    effective_store_id: String,
    effective_store_mode: String,
    write_block_reasons: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MailchimpStoreSummary {
    id: String,
    #[serde(default)]
    platform: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackfillArtifactsV1 {
    pub manifest_path: PathBuf,
    pub reconciliation_path: Option<PathBuf>,
    pub idempotency_path: Option<PathBuf>,
}

pub async fn run_wix_mailchimp_backfill_v1(
    config: &MailchimpBackfillConfigV1,
    options: &BackfillRunOptionsV1,
) -> Result<BackfillArtifactsV1, BackfillError> {
    fs::create_dir_all(&options.out_dir)?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(
            DEFAULT_MAILCHIMP_TIMEOUT_SECS,
        ))
        .build()?;

    let source_orders = match options.source_mode {
        SourceModeV1::WixApi => fetch_wix_orders_api(&client, config, options).await?,
        SourceModeV1::WixCsv => {
            let path = options.csv_path.as_ref().ok_or_else(|| {
                BackfillError::Message("--csv is required for csv source mode".to_string())
            })?;
            parse_wix_orders_csv(path)?
        }
    };

    let valid = filter_valid_orders(source_orders);
    let manifest_prefix = phase_label(&options.phase);
    let store_decision = discover_store_target(&client, config, options).await?;
    let mut manifest = build_manifest(
        &valid.kept,
        &valid.skipped,
        options,
        config,
        &store_decision,
    );
    if !config.allow_writes {
        manifest
            .write_block_reasons
            .push("MAILCHIMP_ALLOW_WRITES=false".to_string());
    }
    if !matches!(options.phase, BackfillPhaseV1::DryRun) && !config.confirm_automations_paused {
        manifest
            .write_block_reasons
            .push("MAILCHIMP_CONFIRM_AUTOMATIONS_PAUSED=false".to_string());
    }
    let manifest_path = options
        .out_dir
        .join(format!("{manifest_prefix}_dry_run_manifest.json"));
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    let write_blocked = !manifest.write_block_reasons.is_empty()
        || matches!(options.phase, BackfillPhaseV1::DryRun)
        || !config.allow_writes;
    if write_blocked {
        return Ok(BackfillArtifactsV1 {
            manifest_path,
            reconciliation_path: None,
            idempotency_path: None,
        });
    }

    let mut orders = valid.kept;
    if matches!(options.phase, BackfillPhaseV1::Pilot) {
        orders.truncate(options.pilot_limit);
    }
    if let Some(max_orders) = options.max_orders {
        orders.truncate(max_orders);
    }

    let reconciliation_rows =
        execute_mailchimp_backfill(&client, config, &store_decision.effective_store_id, &orders)
            .await?;
    let reconciliation_name = match options.phase {
        BackfillPhaseV1::Pilot => "pilot_reconciliation.csv",
        BackfillPhaseV1::Full => "final_reconciliation.csv",
        BackfillPhaseV1::DryRun => "dry_run_reconciliation.csv",
    };
    let reconciliation_path = options.out_dir.join(reconciliation_name);
    write_reconciliation_csv(&reconciliation_path, &reconciliation_rows)?;

    let idempotency = build_idempotency_report(&orders, &store_decision.effective_store_id);
    let idempotency_path = options.out_dir.join("idempotency_report.json");
    fs::write(&idempotency_path, serde_json::to_vec_pretty(&idempotency)?)?;

    Ok(BackfillArtifactsV1 {
        manifest_path,
        reconciliation_path: Some(reconciliation_path),
        idempotency_path: Some(idempotency_path),
    })
}

fn filter_valid_orders(orders: Vec<WixOrderBackfillV1>) -> FilteredOrdersV1 {
    let mut kept = Vec::new();
    let mut skipped = Vec::new();
    for order in orders {
        if order.customer_email.trim().is_empty() {
            skipped.push(("missing_customer_email".to_string(), order));
            continue;
        }
        if order.line_items.is_empty() {
            skipped.push(("missing_line_items".to_string(), order));
            continue;
        }
        let normalized_status = order.order_status.trim().to_ascii_lowercase();
        if matches!(
            normalized_status.as_str(),
            "initialized" | "cancelled" | "canceled" | "test"
        ) {
            skipped.push((format!("excluded_status:{normalized_status}"), order));
            continue;
        }
        if order.total <= Decimal::ZERO {
            skipped.push(("non_positive_total".to_string(), order));
            continue;
        }
        kept.push(order);
    }
    FilteredOrdersV1 { kept, skipped }
}

struct FilteredOrdersV1 {
    kept: Vec<WixOrderBackfillV1>,
    skipped: Vec<(String, WixOrderBackfillV1)>,
}

fn build_manifest(
    kept: &[WixOrderBackfillV1],
    skipped: &[(String, WixOrderBackfillV1)],
    options: &BackfillRunOptionsV1,
    config: &MailchimpBackfillConfigV1,
    decision: &StoreTargetDecision,
) -> DryRunManifestV1 {
    let mut customers = BTreeSet::new();
    let mut products = BTreeSet::new();
    let mut variants = BTreeSet::new();
    let mut gross_by_currency: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut status_breakdown: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped_by_reason: BTreeMap<String, usize> = BTreeMap::new();
    for order in kept {
        customers.insert(normalize_email(&order.customer_email));
        let normalized_status = order.order_status.trim().to_ascii_lowercase();
        *status_breakdown
            .entry(if normalized_status.is_empty() {
                "unknown".to_string()
            } else {
                normalized_status
            })
            .or_insert(0) += 1;
        gross_by_currency
            .entry(order.currency.clone())
            .and_modify(|value| *value += order.total)
            .or_insert(order.total);
        for line in &order.line_items {
            products.insert(line.product_id.clone());
            variants.insert(line.variant_id.clone());
        }
    }
    for (reason, _) in skipped {
        *skipped_by_reason.entry(reason.clone()).or_insert(0) += 1;
    }

    DryRunManifestV1 {
        schema_version: "mailchimp_wix_backfill_manifest.v1".to_string(),
        generated_at_utc: Utc::now().to_rfc3339(),
        phase: options.phase.clone(),
        source_mode: options.source_mode.clone(),
        target_audience_id: config.audience_id.clone(),
        requested_store_id: config.store_id.clone(),
        effective_store_id: decision.effective_store_id.clone(),
        effective_store_mode: decision.effective_store_mode.clone(),
        order_count: kept.len(),
        unique_customers: customers.len(),
        unique_products: products.len(),
        unique_variants: variants.len(),
        gross_revenue_by_currency: gross_by_currency
            .into_iter()
            .map(|(currency, amount)| (currency, amount.round_dp(2).to_string()))
            .collect(),
        status_breakdown,
        skipped_by_reason,
        write_block_reasons: decision.write_block_reasons.clone(),
        notes: decision.notes.clone(),
    }
}

async fn discover_store_target(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    options: &BackfillRunOptionsV1,
) -> Result<StoreTargetDecision, BackfillError> {
    let stores = mailchimp_get_json(client, config, "/ecommerce/stores?count=1000").await?;
    let existing = stores
        .get("stores")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| serde_json::from_value::<MailchimpStoreSummary>(value).ok())
        .collect::<Vec<_>>();

    let requested = existing.iter().find(|store| store.id == config.store_id);
    if let Some(store) = requested {
        let looks_connected = store
            .platform
            .as_deref()
            .map(|value| !matches!(value.trim().to_ascii_lowercase().as_str(), "" | "custom"))
            .unwrap_or(false);
        if looks_connected && !options.allow_existing_store_write {
            return Ok(StoreTargetDecision {
                effective_store_id: config.fallback_store_id.clone(),
                effective_store_mode: "fallback_dedicated_store".to_string(),
                write_block_reasons: Vec::new(),
                notes: vec![format!(
                    "store {} has platform {:?}; using fallback store {} instead of writing into the connected store",
                    store.id, store.platform
                    , config.fallback_store_id
                )],
            });
        }
        return Ok(StoreTargetDecision {
            effective_store_id: config.store_id.clone(),
            effective_store_mode: "existing_store".to_string(),
            write_block_reasons: Vec::new(),
            notes: vec!["existing requested store found".to_string()],
        });
    }

    Ok(StoreTargetDecision {
        effective_store_id: config.store_id.clone(),
        effective_store_mode: "store_creation_required".to_string(),
        write_block_reasons: Vec::new(),
        notes: vec!["requested store not found; it will be created on write phase".to_string()],
    })
}

async fn execute_mailchimp_backfill(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    effective_store_id: &str,
    orders: &[WixOrderBackfillV1],
) -> Result<Vec<ReconciliationRowV1>, BackfillError> {
    ensure_single_currency(orders)?;
    ensure_store_exists(client, config, effective_store_id, orders).await?;
    let customer_operations = build_customer_batch_operations(effective_store_id, orders);
    for chunk in customer_operations.chunks(config.budget.max_orders_per_write_batch) {
        submit_mailchimp_batch_operations(client, config, chunk).await?;
    }
    let product_operations = build_product_batch_operations(effective_store_id, orders);
    for chunk in product_operations.chunks(config.budget.max_orders_per_write_batch) {
        submit_mailchimp_batch_operations(client, config, chunk).await?;
    }
    let mut rows = Vec::with_capacity(orders.len());
    for chunk in orders.chunks(config.budget.max_orders_per_write_batch) {
        let order_operations = chunk
            .iter()
            .map(|order| build_mailchimp_order_operation(effective_store_id, order))
            .collect::<Vec<_>>();
        submit_mailchimp_batch_operations(client, config, &order_operations).await?;
        rows.extend(chunk.iter().map(|order| ReconciliationRowV1 {
            order_id: order.order_id.clone(),
            customer_email: order.customer_email.clone(),
            currency: order.currency.clone(),
            source_total: order.total.round_dp(2).to_string(),
            imported: true,
            reason: "imported".to_string(),
            effective_store_id: effective_store_id.to_string(),
        }));
    }
    Ok(rows)
}

async fn ensure_store_exists(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    store_id: &str,
    orders: &[WixOrderBackfillV1],
) -> Result<(), BackfillError> {
    let path = format!("/ecommerce/stores/{store_id}");
    let response = mailchimp_request(client, config, Method::GET, &path, None).await?;
    if response.status() == StatusCode::NOT_FOUND {
        let currency_code = dominant_currency(orders)?;
        let body = json!({
            "id": store_id,
            "list_id": config.audience_id,
            "name": format!("Wix Backfill {}", config.wix_site_id),
            "platform": "Custom",
            "currency_code": currency_code,
            "is_syncing": false,
        });
        mailchimp_request_expect_success(
            client,
            config,
            Method::POST,
            "/ecommerce/stores",
            Some(body),
        )
        .await?;
        return Ok(());
    }
    if !response.status().is_success() {
        return Err(BackfillError::Message(format!(
            "mailchimp store probe failed: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }
    Ok(())
}

fn build_mailchimp_order_payload(order: &WixOrderBackfillV1) -> Value {
    json!({
        "id": stable_order_id(&order.order_id),
        "customer": {"id": stable_customer_id(&normalize_email(&order.customer_email))},
        "currency_code": order.currency,
        "order_total": order.total.round_dp(2).to_string(),
        "processed_at_foreign": order.paid_at_utc.clone().unwrap_or_else(|| order.created_at_utc.clone()),
        "financial_status": mailchimp_financial_status(order),
        "fulfillment_status": mailchimp_fulfillment_status(order),
        "tax_total": order.tax_total.round_dp(2).to_string(),
        "shipping_total": order.shipping_total.round_dp(2).to_string(),
        "discount_total": order.discount_total.round_dp(2).to_string(),
        "lines": order.line_items.iter().map(|line| json!({
            "id": line.line_id,
            "product_id": line.product_id,
            "product_variant_id": line.variant_id,
            "quantity": line.quantity,
            "price": line.unit_price.round_dp(2).to_string(),
        })).collect::<Vec<_>>(),
    })
}

fn build_mailchimp_order_operation(store_id: &str, order: &WixOrderBackfillV1) -> Value {
    json!({
        "method": "PUT",
        "path": format!(
            "/ecommerce/stores/{store_id}/orders/{}",
            stable_order_id(&order.order_id)
        ),
        "body": serde_json::to_string(&build_mailchimp_order_payload(order)).unwrap_or_default(),
    })
}

fn build_customer_batch_operations(store_id: &str, orders: &[WixOrderBackfillV1]) -> Vec<Value> {
    let mut customers = BTreeMap::<String, Value>::new();
    for order in orders {
        let email = normalize_email(&order.customer_email);
        let customer_id = stable_customer_id(&email);
        customers.entry(customer_id.clone()).or_insert_with(|| {
            json!({
                "method": "PUT",
                "path": format!("/ecommerce/stores/{store_id}/customers/{customer_id}"),
                "body": serde_json::to_string(&json!({
                    "id": customer_id,
                    "email_address": email,
                    "opt_in_status": false,
                    "first_name": order.customer_first_name.clone().unwrap_or_default(),
                    "last_name": order.customer_last_name.clone().unwrap_or_default(),
                })).unwrap_or_default(),
            })
        });
    }
    customers.into_values().collect()
}

fn build_product_batch_operations(store_id: &str, orders: &[WixOrderBackfillV1]) -> Vec<Value> {
    let mut products = BTreeMap::<String, Value>::new();
    for order in orders {
        for line in &order.line_items {
            products.entry(line.product_id.clone()).or_insert_with(|| {
                json!({
                    "method": "PUT",
                    "path": format!("/ecommerce/stores/{store_id}/products/{}", line.product_id),
                    "body": serde_json::to_string(&json!({
                        "id": line.product_id,
                        "title": line.title,
                        "variants": [{
                            "id": line.variant_id,
                            "title": line.title,
                            "sku": line.sku.clone().unwrap_or_default(),
                            "price": line.unit_price.round_dp(2).to_string(),
                            "inventory_quantity": 0
                        }]
                    })).unwrap_or_default(),
                })
            });
        }
    }
    products.into_values().collect()
}

async fn submit_mailchimp_batch_operations(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    operations: &[Value],
) -> Result<(), BackfillError> {
    if operations.is_empty() {
        return Ok(());
    }
    let response = mailchimp_request_expect_success(
        client,
        config,
        Method::POST,
        "/batches",
        Some(json!({ "operations": operations })),
    )
    .await?;
    let payload: Value = response.json().await?;
    let batch_id = string_path(&payload, &["id"])
        .ok_or_else(|| BackfillError::Message("mailchimp batch response missing id".to_string()))?;
    loop {
        sleep(Duration::from_secs(MAILCHIMP_BATCH_POLL_INTERVAL_SECS)).await;
        let status_payload =
            mailchimp_get_json(client, config, &format!("/batches/{batch_id}")).await?;
        let status = string_path(&status_payload, &["status"])
            .unwrap_or_else(|| "unknown".to_string())
            .to_ascii_lowercase();
        if status == "finished" {
            let errored = status_payload
                .get("errored_operations")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            if errored > 0 {
                let response_body_url = string_path(&status_payload, &["response_body_url"])
                    .unwrap_or_default();
                return Err(BackfillError::Message(format!(
                    "mailchimp batch {batch_id} finished with {errored} errored operations; response_body_url={response_body_url}"
                )));
            }
            return Ok(());
        }
        if matches!(
            status.as_str(),
            "pending" | "preprocessing" | "started" | "finalizing"
        ) {
            continue;
        }
        return Err(BackfillError::Message(format!(
            "mailchimp batch {batch_id} entered unexpected status {status}"
        )));
    }
}

async fn fetch_wix_orders_api(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    options: &BackfillRunOptionsV1,
) -> Result<Vec<WixOrderBackfillV1>, BackfillError> {
    let wix_api_credential = config.wix_api_token.clone().ok_or_else(|| {
        BackfillError::Message("WIX_API_TOKEN is required for api source mode".to_string())
    })?;
    let page_limit = config
        .budget
        .max_source_page_size
        .min(DEFAULT_WIX_PAGE_LIMIT);
    let mut out = Vec::new();
    let mut seen_order_ids = std::collections::HashSet::new();
    let mut seen_cursors = BTreeSet::new();
    let mut cursor: Option<String> = None;

    loop {
        if !options.all_history && !out.is_empty() {
            break;
        }
        let mut search = json!({
            "sort": [{"fieldName":"createdDate","order":"ASC"}],
            "cursorPaging": {
                "limit": page_limit
            }
        });
        if let Some(cursor_value) = cursor.as_ref() {
            search["cursorPaging"] = json!({
                "limit": page_limit,
                "cursor": cursor_value
            });
        }
        let body = json!({ "search": search });
        let response = client
            .post("https://www.wixapis.com/ecom/v1/orders/search")
            .bearer_auth(wix_api_credential.clone())
            .header("wix-site-id", config.wix_site_id.clone())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(BackfillError::Message(format!(
                "wix orders search failed: {} {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }
        let payload: Value = response.json().await?;
        let items = payload
            .get("orders")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        for item in items {
            let order = parse_wix_api_order(item)?;
            if !seen_order_ids.insert(order.order_id.clone()) {
                continue;
            }
            out.push(order);
            if let Some(max_orders) = options.max_orders {
                if out.len() >= max_orders {
                    out.truncate(max_orders);
                    return Ok(out);
                }
            }
            if out.len() as u64 >= config.budget.max_orders_per_run {
                return Ok(out);
            }
        }
        let next_cursor = payload
            .get("metadata")
            .and_then(|value| value.get("cursors"))
            .and_then(|value| value.get("next"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let has_next = payload
            .get("metadata")
            .and_then(|value| value.get("hasNext"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if !has_next || next_cursor.is_none() {
            break;
        }
        let next_cursor = next_cursor.expect("checked is_some");
        if !seen_cursors.insert(next_cursor.clone()) {
            return Err(BackfillError::Message(
                "wix orders search cursor cycle detected".to_string(),
            ));
        }
        cursor = Some(next_cursor);
    }

    out.sort_by(|left, right| {
        left.created_at_utc
            .cmp(&right.created_at_utc)
            .then_with(|| left.order_id.cmp(&right.order_id))
    });
    Ok(out)
}

fn parse_wix_api_order(value: Value) -> Result<WixOrderBackfillV1, BackfillError> {
    let order_id = string_path(&value, &["id"])
        .or_else(|| string_path(&value, &["number"]))
        .ok_or_else(|| BackfillError::Message("wix order missing id".to_string()))?;
    let created_at_utc = string_path(&value, &["createdDate"])
        .or_else(|| string_path(&value, &["_createdDate"]))
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let paid_at_utc = string_path(&value, &["paymentStatus", "lastUpdated"])
        .or_else(|| string_path(&value, &["updatedDate"]));
    let buyer_email = string_path(&value, &["buyerInfo", "email"])
        .or_else(|| string_path(&value, &["billingInfo", "contactDetails", "email"]))
        .unwrap_or_default();
    let order_status = string_path(&value, &["status"])
        .or_else(|| string_path(&value, &["fulfillmentStatus"]))
        .unwrap_or_else(|| "unknown".to_string());
    let financial_status = string_path(&value, &["paymentStatus", "status"])
        .or_else(|| string_path(&value, &["paymentStatus"]));
    let fulfillment_status = string_path(&value, &["fulfillmentStatus"]);
    let currency = string_path(&value, &["currency"])
        .or_else(|| string_path(&value, &["priceSummary", "currency"]))
        .unwrap_or_else(|| "USD".to_string());
    let total = decimal_path(&value, &["priceSummary", "total", "amount"])
        .or_else(|| decimal_path(&value, &["priceSummary", "totalPrice", "amount"]))
        .unwrap_or(Decimal::ZERO);
    let subtotal = decimal_path(&value, &["priceSummary", "subtotal", "amount"]).unwrap_or(total);
    let tax_total =
        decimal_path(&value, &["priceSummary", "tax", "amount"]).unwrap_or(Decimal::ZERO);
    let shipping_total =
        decimal_path(&value, &["priceSummary", "shipping", "amount"]).unwrap_or(Decimal::ZERO);
    let discount_total =
        decimal_path(&value, &["priceSummary", "discount", "amount"]).unwrap_or(Decimal::ZERO);
    let line_items = value
        .get("lineItems")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(idx, line)| parse_wix_api_line_item(idx, line))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WixOrderBackfillV1 {
        order_id,
        created_at_utc,
        paid_at_utc,
        order_status,
        financial_status,
        fulfillment_status,
        customer_email: buyer_email,
        customer_first_name: string_path(&value, &["buyerInfo", "firstName"]),
        customer_last_name: string_path(&value, &["buyerInfo", "lastName"]),
        currency,
        total,
        subtotal,
        tax_total,
        shipping_total,
        discount_total,
        line_items,
        shipping_address: parse_wix_address(&value, &["recipientInfo", "address"]),
        billing_address: parse_wix_address(&value, &["billingInfo", "address"]),
        source_payload_ref: None,
    })
}

fn parse_wix_api_line_item(
    index: usize,
    value: Value,
) -> Result<WixOrderLineItemBackfillV1, BackfillError> {
    let title = string_path(&value, &["productName", "original"])
        .or_else(|| string_path(&value, &["productName", "translated"]))
        .or_else(|| string_path(&value, &["productName"]))
        .or_else(|| string_path(&value, &["name"]))
        .unwrap_or_else(|| format!("line-{index}"));
    let product_id = string_path(&value, &["catalogReference", "catalogItemId"])
        .or_else(|| string_path(&value, &["productId"]))
        .unwrap_or_else(|| stable_id_from_text(&title));
    let variant_id = string_path(&value, &["catalogReference", "options", "variantId"])
        .or_else(|| string_path(&value, &["variantId"]))
        .unwrap_or_else(|| format!("{product_id}:default"));
    let quantity = string_path(&value, &["quantity"])
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let unit_price = decimal_path(&value, &["price", "amount"])
        .or_else(|| decimal_path(&value, &["priceData", "price", "amount"]))
        .or_else(|| decimal_path(&value, &["lineItemPrice", "amount"]))
        .unwrap_or(Decimal::ZERO);
    let line_total = decimal_path(&value, &["priceData", "totalPrice", "amount"])
        .or_else(|| decimal_path(&value, &["totalPriceAfterTax", "amount"]))
        .or_else(|| decimal_path(&value, &["totalPriceBeforeTax", "amount"]))
        .unwrap_or(unit_price * Decimal::from(quantity));
    Ok(WixOrderLineItemBackfillV1 {
        line_id: string_path(&value, &["id"]).unwrap_or_else(|| format!("{product_id}:{index}")),
        product_id,
        variant_id,
        title,
        sku: string_path(&value, &["sku"])
            .or_else(|| string_path(&value, &["physicalProperties", "sku"])),
        quantity,
        unit_price,
        line_total,
    })
}

fn parse_wix_orders_csv(path: &Path) -> Result<Vec<WixOrderBackfillV1>, BackfillError> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let index = headers
        .iter()
        .enumerate()
        .map(|(idx, header)| (header.trim().to_ascii_lowercase(), idx))
        .collect::<HashMap<_, _>>();
    let required = [
        "order_id",
        "customer_email",
        "currency",
        "line_title",
        "quantity",
        "unit_price",
    ];
    for field in required {
        if !index.contains_key(field) {
            return Err(BackfillError::Message(format!(
                "wix csv missing required column: {field}"
            )));
        }
    }

    let mut grouped: BTreeMap<String, Vec<csv::StringRecord>> = BTreeMap::new();
    for row in reader.records() {
        let row = row?;
        let order_id = csv_value(&row, &index, "order_id").unwrap_or_default();
        grouped.entry(order_id).or_default().push(row);
    }

    let mut out = Vec::new();
    for (order_id, rows) in grouped {
        let first = rows
            .first()
            .ok_or_else(|| BackfillError::Message("grouped csv order had no rows".to_string()))?;
        let currency = csv_value(first, &index, "currency").unwrap_or_else(|| "USD".to_string());
        let line_items = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let title =
                    csv_value(row, &index, "line_title").unwrap_or_else(|| format!("line-{idx}"));
                let quantity = csv_value(row, &index, "quantity")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(1);
                let unit_price = csv_value(row, &index, "unit_price")
                    .and_then(|value| Decimal::from_str_exact(&value).ok())
                    .unwrap_or(Decimal::ZERO);
                Ok(WixOrderLineItemBackfillV1 {
                    line_id: csv_value(row, &index, "line_id")
                        .unwrap_or_else(|| format!("{order_id}:{idx}")),
                    product_id: csv_value(row, &index, "product_id")
                        .unwrap_or_else(|| stable_id_from_text(&title)),
                    variant_id: csv_value(row, &index, "variant_id")
                        .unwrap_or_else(|| format!("{}:default", stable_id_from_text(&title))),
                    title,
                    sku: csv_value(row, &index, "sku"),
                    quantity,
                    unit_price,
                    line_total: unit_price * Decimal::from(quantity),
                })
            })
            .collect::<Result<Vec<_>, BackfillError>>()?;
        let total = line_items
            .iter()
            .fold(Decimal::ZERO, |sum, line| sum + line.line_total);
        out.push(WixOrderBackfillV1 {
            order_id: order_id.clone(),
            created_at_utc: csv_value(first, &index, "created_at_utc")
                .or_else(|| csv_value(first, &index, "created_at"))
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            paid_at_utc: csv_value(first, &index, "paid_at_utc"),
            order_status: csv_value(first, &index, "order_status")
                .unwrap_or_else(|| "paid".to_string()),
            financial_status: csv_value(first, &index, "financial_status"),
            fulfillment_status: csv_value(first, &index, "fulfillment_status"),
            customer_email: csv_value(first, &index, "customer_email").unwrap_or_default(),
            customer_first_name: csv_value(first, &index, "customer_first_name"),
            customer_last_name: csv_value(first, &index, "customer_last_name"),
            currency,
            total,
            subtotal: total,
            tax_total: Decimal::ZERO,
            shipping_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            line_items,
            shipping_address: None,
            billing_address: None,
            source_payload_ref: Some(path.display().to_string()),
        });
    }
    Ok(out)
}

fn write_reconciliation_csv(
    path: &Path,
    rows: &[ReconciliationRowV1],
) -> Result<(), BackfillError> {
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "order_id",
        "customer_email",
        "currency",
        "source_total",
        "imported",
        "reason",
        "effective_store_id",
    ])?;
    for row in rows {
        writer.write_record([
            row.order_id.as_str(),
            row.customer_email.as_str(),
            row.currency.as_str(),
            row.source_total.as_str(),
            if row.imported { "true" } else { "false" },
            row.reason.as_str(),
            row.effective_store_id.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn build_idempotency_report(
    orders: &[WixOrderBackfillV1],
    effective_store_id: &str,
) -> IdempotencyReportV1 {
    let mut seen = HashSetCompat::default();
    let mut duplicates = 0usize;
    for order in orders {
        if !seen.insert(order.order_id.clone()) {
            duplicates += 1;
        }
    }
    IdempotencyReportV1 {
        schema_version: "mailchimp_wix_backfill_idempotency.v1".to_string(),
        generated_at_utc: Utc::now().to_rfc3339(),
        effective_store_id: effective_store_id.to_string(),
        checked_orders: orders.len(),
        duplicate_source_order_ids: duplicates,
        notes: vec![
            "deterministic order ids are wix:<order_id>; reruns should upsert same ids".to_string(),
        ],
    }
}

async fn mailchimp_get_json(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    path: &str,
) -> Result<Value, BackfillError> {
    let response =
        mailchimp_request_expect_success(client, config, Method::GET, path, None).await?;
    Ok(response.json().await?)
}

async fn mailchimp_request_expect_success(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<reqwest::Response, BackfillError> {
    let response = mailchimp_request(client, config, method, path, body).await?;
    if !response.status().is_success() {
        return Err(BackfillError::Message(format!(
            "mailchimp request failed: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }
    Ok(response)
}

async fn mailchimp_request(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<reqwest::Response, BackfillError> {
    let url = format!(
        "https://{}.api.mailchimp.com/3.0{}",
        config.server_prefix, path
    );
    let mut request = client
        .request(method, url)
        .basic_auth("natures-diet", Some(config.api_key.as_str()));
    if let Some(payload) = body {
        request = request.json(&payload);
    }
    Ok(request.send().await?)
}

fn stable_customer_id(email: &str) -> String {
    format!("wixcust:{}", normalize_email(email))
}

fn stable_order_id(order_id: &str) -> String {
    format!("wix:{}", order_id.trim())
}

fn stable_id_from_text(value: &str) -> String {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    format!("wixitem:{}", normalized.trim_matches('-'))
}

fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn dominant_currency(orders: &[WixOrderBackfillV1]) -> Result<String, BackfillError> {
    ensure_single_currency(orders)?;
    orders
        .first()
        .map(|order| order.currency.clone())
        .ok_or_else(|| {
            BackfillError::Message("no orders available to infer store currency".to_string())
        })
}

fn ensure_single_currency(orders: &[WixOrderBackfillV1]) -> Result<(), BackfillError> {
    let currencies = orders
        .iter()
        .map(|order| order.currency.trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if currencies.len() > 1 {
        return Err(BackfillError::Message(format!(
            "mixed currencies are not supported for a single Mailchimp store: {:?}",
            currencies
        )));
    }
    Ok(())
}

fn mailchimp_financial_status(order: &WixOrderBackfillV1) -> &'static str {
    let status = order
        .financial_status
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if status.contains("refund") {
        "refunded"
    } else if status.contains("paid") {
        "paid"
    } else if status.contains("pending") || status.contains("unpaid") {
        "pending"
    } else {
        "paid"
    }
}

fn mailchimp_fulfillment_status(order: &WixOrderBackfillV1) -> &'static str {
    let status = order
        .fulfillment_status
        .as_deref()
        .unwrap_or(order.order_status.as_str())
        .trim()
        .to_ascii_lowercase();
    if status.contains("fulfilled") || status.contains("completed") || status.contains("shipped") {
        "shipped"
    } else if status.contains("cancel") {
        "cancelled"
    } else {
        "pending"
    }
}

fn required_env(name: &str) -> Result<String, BackfillError> {
    env::var(name)
        .map_err(|_| BackfillError::Message(format!("{name} is required")))
        .and_then(|value| {
            if value.trim().is_empty() {
                Err(BackfillError::Message(format!("{name} is required")))
            } else {
                Ok(value)
            }
        })
}

fn env_bool(name: &str, default: bool) -> Result<bool, BackfillError> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" => Ok(true),
            "0" | "false" | "no" | "n" => Ok(false),
            _ => Err(BackfillError::Message(format!("{name} must be a boolean"))),
        },
        _ => Ok(default),
    }
}

fn phase_label(phase: &BackfillPhaseV1) -> &'static str {
    match phase {
        BackfillPhaseV1::DryRun => "dry_run",
        BackfillPhaseV1::Pilot => "pilot",
        BackfillPhaseV1::Full => "full",
    }
}

fn csv_value(row: &csv::StringRecord, index: &HashMap<String, usize>, key: &str) -> Option<String> {
    index
        .get(key)
        .and_then(|idx| row.get(*idx))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn string_path(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn decimal_path(value: &Value, path: &[&str]) -> Option<Decimal> {
    string_path(value, path).and_then(|value| Decimal::from_str_exact(&value).ok())
}

fn parse_wix_address(root: &Value, path: &[&str]) -> Option<BackfillAddressV1> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    Some(BackfillAddressV1 {
        name: current
            .get("fullName")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        address1: current
            .get("addressLine1")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        address2: current
            .get("addressLine2")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        city: current
            .get("city")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        province: current
            .get("subdivision")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        postal_code: current
            .get("postalCode")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        country_code: current
            .get("country")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

#[derive(Default)]
struct HashSetCompat<T: std::hash::Hash + Eq>(std::collections::HashSet<T>);

impl<T: std::hash::Hash + Eq> HashSetCompat<T> {
    fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_are_deterministic() {
        assert_eq!(
            stable_customer_id(" Test@Example.com "),
            "wixcust:test@example.com"
        );
        assert_eq!(stable_order_id("123"), "wix:123");
    }

    #[test]
    fn filter_valid_orders_excludes_cancelled_and_missing_email() {
        let base = WixOrderBackfillV1 {
            order_id: "1".to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            paid_at_utc: None,
            order_status: "paid".to_string(),
            financial_status: Some("paid".to_string()),
            fulfillment_status: None,
            customer_email: "test@example.com".to_string(),
            customer_first_name: None,
            customer_last_name: None,
            currency: "USD".to_string(),
            total: Decimal::from(10),
            subtotal: Decimal::from(10),
            tax_total: Decimal::ZERO,
            shipping_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            line_items: vec![WixOrderLineItemBackfillV1 {
                line_id: "l1".to_string(),
                product_id: "p1".to_string(),
                variant_id: "v1".to_string(),
                title: "Product".to_string(),
                sku: Some("SKU".to_string()),
                quantity: 1,
                unit_price: Decimal::from(10),
                line_total: Decimal::from(10),
            }],
            shipping_address: None,
            billing_address: None,
            source_payload_ref: None,
        };
        let mut cancelled = base.clone();
        cancelled.order_status = "cancelled".to_string();
        let mut missing_email = base.clone();
        missing_email.customer_email.clear();
        let filtered = filter_valid_orders(vec![base, cancelled, missing_email]);
        assert_eq!(filtered.kept.len(), 1);
        assert_eq!(filtered.skipped.len(), 2);
    }

    #[test]
    fn csv_parser_requires_line_item_columns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orders.csv");
        fs::write(&path, "order_id,customer_email\n1,test@example.com\n").unwrap();
        let err = parse_wix_orders_csv(&path).unwrap_err().to_string();
        assert!(err.contains("missing required column"));
    }

    #[test]
    fn financial_and_fulfillment_status_mapping_is_conservative() {
        let refunded = WixOrderBackfillV1 {
            order_id: "1".to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            paid_at_utc: None,
            order_status: "processing".to_string(),
            financial_status: Some("REFUNDED".to_string()),
            fulfillment_status: Some("NOT_FULFILLED".to_string()),
            customer_email: "test@example.com".to_string(),
            customer_first_name: None,
            customer_last_name: None,
            currency: "USD".to_string(),
            total: Decimal::from(10),
            subtotal: Decimal::from(10),
            tax_total: Decimal::ZERO,
            shipping_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            line_items: vec![WixOrderLineItemBackfillV1 {
                line_id: "l1".to_string(),
                product_id: "p1".to_string(),
                variant_id: "v1".to_string(),
                title: "Product".to_string(),
                sku: Some("SKU".to_string()),
                quantity: 1,
                unit_price: Decimal::from(10),
                line_total: Decimal::from(10),
            }],
            shipping_address: None,
            billing_address: None,
            source_payload_ref: None,
        };
        assert_eq!(mailchimp_financial_status(&refunded), "refunded");
        assert_eq!(mailchimp_fulfillment_status(&refunded), "shipped");

        let shipped = WixOrderBackfillV1 {
            order_status: "approved".to_string(),
            financial_status: Some("PAID".to_string()),
            fulfillment_status: Some("FULFILLED".to_string()),
            ..refunded
        };
        assert_eq!(mailchimp_financial_status(&shipped), "paid");
        assert_eq!(mailchimp_fulfillment_status(&shipped), "shipped");
    }

    #[test]
    fn single_currency_guard_rejects_mixed_currency_runs() {
        let usd = WixOrderBackfillV1 {
            order_id: "1".to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            paid_at_utc: None,
            order_status: "paid".to_string(),
            financial_status: Some("paid".to_string()),
            fulfillment_status: None,
            customer_email: "test@example.com".to_string(),
            customer_first_name: None,
            customer_last_name: None,
            currency: "USD".to_string(),
            total: Decimal::from(10),
            subtotal: Decimal::from(10),
            tax_total: Decimal::ZERO,
            shipping_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            line_items: vec![WixOrderLineItemBackfillV1 {
                line_id: "l1".to_string(),
                product_id: "p1".to_string(),
                variant_id: "v1".to_string(),
                title: "Product".to_string(),
                sku: Some("SKU".to_string()),
                quantity: 1,
                unit_price: Decimal::from(10),
                line_total: Decimal::from(10),
            }],
            shipping_address: None,
            billing_address: None,
            source_payload_ref: None,
        };
        let eur = WixOrderBackfillV1 {
            order_id: "2".to_string(),
            currency: "EUR".to_string(),
            ..usd.clone()
        };
        let err = ensure_single_currency(&[usd, eur]).unwrap_err().to_string();
        assert!(err.contains("mixed currencies"));
    }

    #[test]
    fn parse_wix_api_order_handles_live_shape_fields() {
        let payload = json!({
            "id": "order-1",
            "number": "41331",
            "createdDate": "2026-03-27T18:05:20.234Z",
            "updatedDate": "2026-03-27T18:05:29.671Z",
            "paymentStatus": "PAID",
            "fulfillmentStatus": "NOT_FULFILLED",
            "buyerInfo": {
                "email": "customer@example.com"
            },
            "currency": "USD",
            "priceSummary": {
                "subtotal": {"amount": "89.98"},
                "total": {"amount": "89.98"}
            },
            "lineItems": [{
                "id": "line-1",
                "productName": {
                    "original": "Simply Raw® All Flavors Mix"
                },
                "catalogReference": {
                    "catalogItemId": "catalog-1",
                    "options": {
                        "variantId": "variant-1"
                    }
                },
                "quantity": 1,
                "physicalProperties": {
                    "sku": "NDP-04140031B1C1T"
                },
                "price": {"amount": "89.98"},
                "totalPriceAfterTax": {"amount": "89.98"}
            }]
        });
        let order = parse_wix_api_order(payload).unwrap();
        assert_eq!(order.financial_status.as_deref(), Some("PAID"));
        assert_eq!(order.fulfillment_status.as_deref(), Some("NOT_FULFILLED"));
        assert_eq!(order.paid_at_utc.as_deref(), Some("2026-03-27T18:05:29.671Z"));
        assert_eq!(order.line_items.len(), 1);
        assert_eq!(order.line_items[0].title, "Simply Raw® All Flavors Mix");
        assert_eq!(
            order.line_items[0].sku.as_deref(),
            Some("NDP-04140031B1C1T")
        );
    }

    #[test]
    fn batch_builders_dedupe_customers_and_products() {
        let shared_line = WixOrderLineItemBackfillV1 {
            line_id: "l1".to_string(),
            product_id: "p1".to_string(),
            variant_id: "v1".to_string(),
            title: "Product".to_string(),
            sku: Some("SKU".to_string()),
            quantity: 1,
            unit_price: Decimal::from(10),
            line_total: Decimal::from(10),
        };
        let base = WixOrderBackfillV1 {
            order_id: "1".to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            paid_at_utc: None,
            order_status: "paid".to_string(),
            financial_status: Some("PAID".to_string()),
            fulfillment_status: Some("FULFILLED".to_string()),
            customer_email: "test@example.com".to_string(),
            customer_first_name: Some("Test".to_string()),
            customer_last_name: Some("User".to_string()),
            currency: "USD".to_string(),
            total: Decimal::from(10),
            subtotal: Decimal::from(10),
            tax_total: Decimal::ZERO,
            shipping_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            line_items: vec![shared_line.clone()],
            shipping_address: None,
            billing_address: None,
            source_payload_ref: None,
        };
        let second = WixOrderBackfillV1 {
            order_id: "2".to_string(),
            line_items: vec![shared_line],
            ..base.clone()
        };
        let customers =
            build_customer_batch_operations("wix_backfill_v1", &[base.clone(), second.clone()]);
        let products = build_product_batch_operations("wix_backfill_v1", &[base, second]);
        assert_eq!(customers.len(), 1);
        assert_eq!(products.len(), 1);
    }
}
