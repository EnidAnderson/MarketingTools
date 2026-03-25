use chrono::Utc;
use md5::compute as md5_compute;
use reqwest::{Client, Method, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
const DEFAULT_WRITE_BATCH_SIZE: usize = 100;
const DEFAULT_FALLBACK_STORE_ID: &str = "wix_backfill_v1";
const DEFAULT_OUTPUT_DIR: &str = "exports/mailchimp_backfill";

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
            fallback_store_id: env::var("MAILCHIMP_FALLBACK_STORE_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_FALLBACK_STORE_ID.to_string()),
            wix_site_id: required_env("WIX_SITE_ID")?,
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
        .timeout(std::time::Duration::from_secs(DEFAULT_MAILCHIMP_TIMEOUT_SECS))
        .build()?;

    let source_orders = match options.source_mode {
        SourceModeV1::WixApi => fetch_wix_orders_api(&client, config, options).await?,
        SourceModeV1::WixCsv => {
            let path = options
                .csv_path
                .as_ref()
                .ok_or_else(|| BackfillError::Message("--csv is required for csv source mode".to_string()))?;
            parse_wix_orders_csv(path)?
        }
    };

    let valid = filter_valid_orders(source_orders);
    let manifest_prefix = phase_label(&options.phase);
    let store_decision = discover_store_target(&client, config, options).await?;
    let manifest = build_manifest(&valid.kept, &valid.skipped, options, config, &store_decision);
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

fn filter_valid_orders(
    orders: Vec<WixOrderBackfillV1>,
) -> FilteredOrdersV1 {
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
    let mut skipped_by_reason: BTreeMap<String, usize> = BTreeMap::new();
    for order in kept {
        customers.insert(normalize_email(&order.customer_email));
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
                effective_store_mode: "fallback_dedicated_store_required".to_string(),
                write_block_reasons: vec![
                    "requested_mailchimp_store_appears_connected_or_non_custom".to_string(),
                ],
                notes: vec![format!(
                    "store {} has platform {:?}; rerun with a dedicated fallback store or explicitly allow existing store writes",
                    store.id, store.platform
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
    ensure_store_exists(client, config, effective_store_id).await?;
    let mut member_status_cache: HashMap<String, Option<String>> = HashMap::new();
    let mut rows = Vec::with_capacity(orders.len());
    for chunk in orders.chunks(config.budget.max_orders_per_write_batch) {
        for order in chunk {
            upsert_mailchimp_customer(client, config, effective_store_id, order, &mut member_status_cache).await?;
            upsert_mailchimp_products(client, config, effective_store_id, order).await?;
            upsert_mailchimp_order(client, config, effective_store_id, order).await?;
            rows.push(ReconciliationRowV1 {
                order_id: order.order_id.clone(),
                customer_email: order.customer_email.clone(),
                currency: order.currency.clone(),
                source_total: order.total.round_dp(2).to_string(),
                imported: true,
                reason: "imported".to_string(),
                effective_store_id: effective_store_id.to_string(),
            });
        }
    }
    Ok(rows)
}

async fn ensure_store_exists(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    store_id: &str,
) -> Result<(), BackfillError> {
    let path = format!("/ecommerce/stores/{store_id}");
    let response = mailchimp_request(client, config, Method::GET, &path, None).await?;
    if response.status() == StatusCode::NOT_FOUND {
        let body = json!({
            "id": store_id,
            "list_id": config.audience_id,
            "name": format!("Wix Backfill {}", config.wix_site_id),
            "platform": "Custom",
            "currency_code": "USD",
            "is_syncing": false,
        });
        mailchimp_request_expect_success(client, config, Method::POST, "/ecommerce/stores", Some(body)).await?;
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

async fn upsert_mailchimp_customer(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    store_id: &str,
    order: &WixOrderBackfillV1,
    member_status_cache: &mut HashMap<String, Option<String>>,
) -> Result<(), BackfillError> {
    let email = normalize_email(&order.customer_email);
    let subscriber_hash = format!("{:x}", md5_compute(email.as_bytes()));
    let existing_status = if let Some(value) = member_status_cache.get(&subscriber_hash) {
        value.clone()
    } else {
        let path = format!("/lists/{}/members/{}", config.audience_id, subscriber_hash);
        let response = mailchimp_request(client, config, Method::GET, &path, None).await?;
        let status = if response.status() == StatusCode::NOT_FOUND {
            None
        } else if response.status().is_success() {
            let payload: Value = response.json().await?;
            payload
                .get("status")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        } else {
            return Err(BackfillError::Message(format!(
                "mailchimp member lookup failed for {}: {}",
                email,
                response.status()
            )));
        };
        member_status_cache.insert(subscriber_hash.clone(), status.clone());
        status
    };
    let audience_payload = json!({
        "email_address": email,
        "status_if_new": existing_status.clone().unwrap_or_else(|| "transactional".to_string()),
        "status": existing_status.clone().unwrap_or_else(|| "transactional".to_string()),
        "merge_fields": {
            "FNAME": order.customer_first_name.clone().unwrap_or_default(),
            "LNAME": order.customer_last_name.clone().unwrap_or_default(),
        }
    });
    let list_path = format!("/lists/{}/members/{}", config.audience_id, subscriber_hash);
    mailchimp_request_expect_success(client, config, Method::PUT, &list_path, Some(audience_payload)).await?;

    let customer_id = stable_customer_id(&email);
    let customer_payload = json!({
        "id": customer_id,
        "email_address": email,
        "opt_in_status": false,
        "first_name": order.customer_first_name.clone().unwrap_or_default(),
        "last_name": order.customer_last_name.clone().unwrap_or_default(),
    });
    let customer_path = format!("/ecommerce/stores/{store_id}/customers/{customer_id}");
    mailchimp_request_expect_success(client, config, Method::PUT, &customer_path, Some(customer_payload)).await?;
    Ok(())
}

async fn upsert_mailchimp_products(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    store_id: &str,
    order: &WixOrderBackfillV1,
) -> Result<(), BackfillError> {
    for line in &order.line_items {
        let product_payload = json!({
            "id": line.product_id,
            "title": line.title,
            "variants": [{
                "id": line.variant_id,
                "title": line.title,
                "sku": line.sku.clone().unwrap_or_default(),
                "price": line.unit_price.round_dp(2).to_string(),
                "inventory_quantity": 0
            }]
        });
        let product_path = format!("/ecommerce/stores/{store_id}/products/{}", line.product_id);
        mailchimp_request_expect_success(client, config, Method::PUT, &product_path, Some(product_payload)).await?;
    }
    Ok(())
}

async fn upsert_mailchimp_order(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    store_id: &str,
    order: &WixOrderBackfillV1,
) -> Result<(), BackfillError> {
    let order_payload = json!({
        "id": stable_order_id(&order.order_id),
        "customer": {"id": stable_customer_id(&normalize_email(&order.customer_email))},
        "currency_code": order.currency,
        "order_total": order.total.round_dp(2).to_string(),
        "processed_at_foreign": order.paid_at_utc.clone().unwrap_or_else(|| order.created_at_utc.clone()),
        "financial_status": "paid",
        "fulfillment_status": "fulfilled",
        "lines": order.line_items.iter().map(|line| json!({
            "id": line.line_id,
            "product_id": line.product_id,
            "product_variant_id": line.variant_id,
            "quantity": line.quantity,
            "price": line.unit_price.round_dp(2).to_string(),
        })).collect::<Vec<_>>()
    });
    let path = format!("/ecommerce/stores/{store_id}/orders/{}", stable_order_id(&order.order_id));
    mailchimp_request_expect_success(client, config, Method::PUT, &path, Some(order_payload)).await?;
    Ok(())
}

async fn fetch_wix_orders_api(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    options: &BackfillRunOptionsV1,
) -> Result<Vec<WixOrderBackfillV1>, BackfillError> {
    let token = config
        .wix_api_token
        .clone()
        .ok_or_else(|| BackfillError::Message("WIX_API_TOKEN is required for api source mode".to_string()))?;
    let mut cursor: Option<String> = None;
    let mut out = Vec::new();
    loop {
        let mut body = json!({
            "paging": {
                "limit": config.budget.max_source_page_size.min(DEFAULT_WIX_PAGE_LIMIT)
            },
            "sort": [{"fieldName":"createdDate","order":"ASC"}]
        });
        if let Some(cursor_value) = cursor.as_ref() {
            body["cursorPaging"] = json!({ "cursor": cursor_value, "limit": config.budget.max_source_page_size });
        }
        let response = client
            .post("https://www.wixapis.com/ecom/v1/orders/search")
            .bearer_auth(token.clone())
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
            out.push(parse_wix_api_order(item)?);
            if out.len() as u64 >= config.budget.max_orders_per_run {
                return Ok(out);
            }
        }
        cursor = payload
            .get("metadata")
            .and_then(|value| value.get("cursor"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .or_else(|| {
                payload
                    .get("pagingMetadata")
                    .and_then(|value| value.get("cursor"))
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            });
        if cursor.is_none() || (!options.all_history && !out.is_empty()) {
            break;
        }
    }
    Ok(out)
}

fn parse_wix_api_order(value: Value) -> Result<WixOrderBackfillV1, BackfillError> {
    let order_id = string_path(&value, &["id"])
        .or_else(|| string_path(&value, &["number"]))
        .ok_or_else(|| BackfillError::Message("wix order missing id".to_string()))?;
    let created_at_utc = string_path(&value, &["createdDate"])
        .or_else(|| string_path(&value, &["_createdDate"]))
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let paid_at_utc = string_path(&value, &["paymentStatus", "lastUpdated"]);
    let buyer_email = string_path(&value, &["buyerInfo", "email"])
        .or_else(|| string_path(&value, &["billingInfo", "contactDetails", "email"]))
        .unwrap_or_default();
    let order_status = string_path(&value, &["status"])
        .or_else(|| string_path(&value, &["fulfillmentStatus"]))
        .unwrap_or_else(|| "unknown".to_string());
    let financial_status = string_path(&value, &["paymentStatus", "status"]);
    let currency = string_path(&value, &["currency"])
        .or_else(|| string_path(&value, &["priceSummary", "currency"]))
        .unwrap_or_else(|| "USD".to_string());
    let total = decimal_path(&value, &["priceSummary", "total", "amount"])
        .or_else(|| decimal_path(&value, &["priceSummary", "totalPrice", "amount"]))
        .unwrap_or(Decimal::ZERO);
    let subtotal = decimal_path(&value, &["priceSummary", "subtotal", "amount"]).unwrap_or(total);
    let tax_total = decimal_path(&value, &["priceSummary", "tax", "amount"]).unwrap_or(Decimal::ZERO);
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

fn parse_wix_api_line_item(index: usize, value: Value) -> Result<WixOrderLineItemBackfillV1, BackfillError> {
    let title = string_path(&value, &["productName"])
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
        .unwrap_or(Decimal::ZERO);
    let line_total = decimal_path(&value, &["priceData", "totalPrice", "amount"])
        .unwrap_or(unit_price * Decimal::from(quantity));
    Ok(WixOrderLineItemBackfillV1 {
        line_id: string_path(&value, &["id"]).unwrap_or_else(|| format!("{product_id}:{index}")),
        product_id,
        variant_id,
        title,
        sku: string_path(&value, &["sku"]),
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
    let required = ["order_id", "customer_email", "currency", "line_title", "quantity", "unit_price"];
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
        let first = rows.first().ok_or_else(|| BackfillError::Message("grouped csv order had no rows".to_string()))?;
        let currency = csv_value(first, &index, "currency").unwrap_or_else(|| "USD".to_string());
        let line_items = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let title = csv_value(row, &index, "line_title").unwrap_or_else(|| format!("line-{idx}"));
                let quantity = csv_value(row, &index, "quantity")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(1);
                let unit_price = csv_value(row, &index, "unit_price")
                    .and_then(|value| Decimal::from_str_exact(&value).ok())
                    .unwrap_or(Decimal::ZERO);
                Ok(WixOrderLineItemBackfillV1 {
                    line_id: csv_value(row, &index, "line_id").unwrap_or_else(|| format!("{order_id}:{idx}")),
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
        let total = line_items.iter().fold(Decimal::ZERO, |sum, line| sum + line.line_total);
        out.push(WixOrderBackfillV1 {
            order_id: order_id.clone(),
            created_at_utc: csv_value(first, &index, "created_at_utc")
                .or_else(|| csv_value(first, &index, "created_at"))
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            paid_at_utc: csv_value(first, &index, "paid_at_utc"),
            order_status: csv_value(first, &index, "order_status").unwrap_or_else(|| "paid".to_string()),
            financial_status: csv_value(first, &index, "financial_status"),
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

fn write_reconciliation_csv(path: &Path, rows: &[ReconciliationRowV1]) -> Result<(), BackfillError> {
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
        notes: vec!["deterministic order ids are wix:<order_id>; reruns should upsert same ids".to_string()],
    }
}

async fn mailchimp_get_json(
    client: &Client,
    config: &MailchimpBackfillConfigV1,
    path: &str,
) -> Result<Value, BackfillError> {
    let response = mailchimp_request_expect_success(client, config, Method::GET, path, None).await?;
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

fn csv_value(
    row: &csv::StringRecord,
    index: &HashMap<String, usize>,
    key: &str,
) -> Option<String> {
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
    current.as_str().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
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
        name: current.get("fullName").and_then(|value| value.as_str()).map(str::to_string),
        address1: current.get("addressLine1").and_then(|value| value.as_str()).map(str::to_string),
        address2: current.get("addressLine2").and_then(|value| value.as_str()).map(str::to_string),
        city: current.get("city").and_then(|value| value.as_str()).map(str::to_string),
        province: current.get("subdivision").and_then(|value| value.as_str()).map(str::to_string),
        postal_code: current.get("postalCode").and_then(|value| value.as_str()).map(str::to_string),
        country_code: current.get("country").and_then(|value| value.as_str()).map(str::to_string),
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
        assert_eq!(stable_customer_id(" Test@Example.com "), "wixcust:test@example.com");
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
}
