use crate::subsystems::marketing_data_analysis::{
    AnalyticsConnectorConfigV1, AnalyticsError, ObservedReadOnlyAnalyticsConnectorV2, WixItemRowV1,
};
use chrono::{Datelike, NaiveDate};
use csv::StringRecord;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// # NDOC
/// component: `subsystems::marketing_data_analysis::subbly_wix_report`
/// purpose: Build monthly SKU unit reports by combining Subbly exports with Wix BigQuery exports.
/// invariants:
///   - Month keys use `YYYY-MM` derived from `order_date` or `shipping_date`.
///   - Subbly rows with explicit `SKUs` are treated as authoritative.
///   - Subbly rows without `SKUs` attempt reconstruction from Mix & Match selections.
///   - Reconstruction uses shipping-item-derived SKU mapping and records unresolved items.
#[derive(Debug, Clone)]
pub struct MonthlySkuSalesRow {
    pub month: String,
    pub sku: String,
    pub subbly_units: u64,
    pub subbly_reconstructed_units: u64,
    pub wix_units: u64,
    pub combined_units: u64,
    pub subbly_revenue: Decimal,
    pub subbly_reconstructed_revenue: Decimal,
    pub wix_revenue: Decimal,
    pub combined_revenue: Decimal,
}

#[derive(Debug, Clone)]
struct BundleRecipeItem {
    sku: &'static str,
    quantity: u64,
}

#[derive(Debug, Clone)]
pub struct UnresolvedMixMatchItem {
    pub order_id: String,
    pub shipping_date: String,
    pub selection_text: String,
    pub item_name: String,
}

#[derive(Debug, Clone)]
pub struct WixUnmappedItem {
    pub month: String,
    pub raw_sku: String,
    pub quantity: u64,
    pub item_name: Option<String>,
    pub classification: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkuMappingConflict {
    pub item_name: String,
    pub existing_sku: String,
    pub new_sku: String,
}

#[derive(Debug, Clone)]
pub struct SkuMappingSuggestion {
    pub wix_item_id: String,
    pub wix_item_name: String,
    pub subbly_sku: String,
    pub subbly_name: String,
    pub similarity: f64,
}

#[derive(Debug, Clone)]
pub struct SubblyWixReportOutput {
    pub rows: Vec<MonthlySkuSalesRow>,
    pub unresolved: Vec<UnresolvedMixMatchItem>,
    pub conflicts: Vec<SkuMappingConflict>,
    pub wix_unmapped: Vec<WixUnmappedItem>,
    pub suggestions: Vec<SkuMappingSuggestion>,
}

pub fn build_subbly_wix_monthly_report(
    subbly_csv_path: &Path,
    wix_csv_path: Option<&Path>,
    mapping_csv_path: Option<&Path>,
) -> Result<SubblyWixReportOutput, AnalyticsError> {
    let wix_units = match wix_csv_path {
        Some(path) => parse_wix_bigquery_csv(path)?,
        None => BTreeMap::new(),
    };
    let wix_revenues: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let (mapping, _mapping_enabled) = load_mapping_csv(mapping_csv_path)?;
    let _suggestions: Vec<SkuMappingSuggestion> = Vec::new();
    build_subbly_report_core(
        subbly_csv_path,
        wix_units,
        wix_revenues,
        Some(mapping),
        None,
        HashMap::new(),
    )
}

pub async fn build_subbly_wix_monthly_report_with_bigquery(
    subbly_csv_path: &Path,
    connector: &ObservedReadOnlyAnalyticsConnectorV2,
    config: &AnalyticsConnectorConfigV1,
    start_date: &str,
    end_date: &str,
    mapping_csv_path: Option<&Path>,
) -> Result<SubblyWixReportOutput, AnalyticsError> {
    let rows = connector
        .fetch_wix_item_rows_bigquery(config, start_date, end_date)
        .await?;
    let (mapping, mapping_enabled) = load_mapping_csv(mapping_csv_path)?;
    let (wix_units, wix_revenues, wix_unmapped, wix_items) =
        wix_units_from_bigquery_rows(&rows, &mapping);
    let wix_unmapped = if mapping_enabled {
        wix_unmapped
    } else {
        Vec::new()
    };
    build_subbly_report_core(
        subbly_csv_path,
        wix_units,
        wix_revenues,
        Some(mapping),
        Some(wix_unmapped),
        wix_items,
    )
}

pub fn write_monthly_report_csv(
    path: &Path,
    rows: &[MonthlySkuSalesRow],
) -> Result<(), AnalyticsError> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| {
        AnalyticsError::new(
            "monthly_report_write_failed",
            format!("failed to open output csv: {err}"),
            vec![path.display().to_string()],
            None,
        )
    })?;

    writer
        .write_record([
            "month",
            "sku",
            "subbly_units",
            "subbly_reconstructed_units",
            "wix_units",
            "combined_units",
            "subbly_revenue",
            "subbly_reconstructed_revenue",
            "wix_revenue",
            "combined_revenue",
        ])
        .map_err(|err| {
            AnalyticsError::internal(
                "monthly_report_write_failed",
                format!("failed to write csv header: {err}"),
            )
        })?;

    for row in rows {
        writer
            .write_record([
                row.month.as_str(),
                row.sku.as_str(),
                row.subbly_units.to_string().as_str(),
                row.subbly_reconstructed_units.to_string().as_str(),
                row.wix_units.to_string().as_str(),
                row.combined_units.to_string().as_str(),
                row.subbly_revenue.round_dp(2).to_string().as_str(),
                row.subbly_reconstructed_revenue
                    .round_dp(2)
                    .to_string()
                    .as_str(),
                row.wix_revenue.round_dp(2).to_string().as_str(),
                row.combined_revenue.round_dp(2).to_string().as_str(),
            ])
            .map_err(|err| {
                AnalyticsError::internal(
                    "monthly_report_write_failed",
                    format!("failed to write csv row: {err}"),
                )
            })?;
    }

    writer.flush().map_err(|err| {
        AnalyticsError::internal(
            "monthly_report_write_failed",
            format!("failed to flush csv: {err}"),
        )
    })
}

pub fn write_unresolved_csv(
    path: &Path,
    rows: &[UnresolvedMixMatchItem],
) -> Result<(), AnalyticsError> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| {
        AnalyticsError::new(
            "unresolved_report_write_failed",
            format!("failed to open unresolved csv: {err}"),
            vec![path.display().to_string()],
            None,
        )
    })?;

    writer
        .write_record(["order_id", "shipping_date", "selection_text", "item_name"])
        .map_err(|err| {
            AnalyticsError::internal(
                "unresolved_report_write_failed",
                format!("failed to write csv header: {err}"),
            )
        })?;

    for row in rows {
        writer
            .write_record([
                row.order_id.as_str(),
                row.shipping_date.as_str(),
                row.selection_text.as_str(),
                row.item_name.as_str(),
            ])
            .map_err(|err| {
                AnalyticsError::internal(
                    "unresolved_report_write_failed",
                    format!("failed to write csv row: {err}"),
                )
            })?;
    }

    writer.flush().map_err(|err| {
        AnalyticsError::internal(
            "unresolved_report_write_failed",
            format!("failed to flush csv: {err}"),
        )
    })
}

pub fn write_conflicts_csv(path: &Path, rows: &[SkuMappingConflict]) -> Result<(), AnalyticsError> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| {
        AnalyticsError::new(
            "conflicts_report_write_failed",
            format!("failed to open conflicts csv: {err}"),
            vec![path.display().to_string()],
            None,
        )
    })?;

    writer
        .write_record(["item_name", "existing_sku", "new_sku"])
        .map_err(|err| {
            AnalyticsError::internal(
                "conflicts_report_write_failed",
                format!("failed to write csv header: {err}"),
            )
        })?;

    for row in rows {
        writer
            .write_record([
                row.item_name.as_str(),
                row.existing_sku.as_str(),
                row.new_sku.as_str(),
            ])
            .map_err(|err| {
                AnalyticsError::internal(
                    "conflicts_report_write_failed",
                    format!("failed to write csv row: {err}"),
                )
            })?;
    }

    writer.flush().map_err(|err| {
        AnalyticsError::internal(
            "conflicts_report_write_failed",
            format!("failed to flush csv: {err}"),
        )
    })
}

pub fn write_suggestions_csv(
    path: &Path,
    rows: &[SkuMappingSuggestion],
) -> Result<(), AnalyticsError> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| {
        AnalyticsError::new(
            "suggestions_report_write_failed",
            format!("failed to open suggestions csv: {err}"),
            vec![path.display().to_string()],
            None,
        )
    })?;

    writer
        .write_record([
            "wix_item_id",
            "wix_item_name",
            "subbly_sku",
            "subbly_name",
            "similarity",
        ])
        .map_err(|err| {
            AnalyticsError::internal(
                "suggestions_report_write_failed",
                format!("failed to write csv header: {err}"),
            )
        })?;

    for row in rows {
        writer
            .write_record([
                row.wix_item_id.as_str(),
                row.wix_item_name.as_str(),
                row.subbly_sku.as_str(),
                row.subbly_name.as_str(),
                format!("{:.3}", row.similarity).as_str(),
            ])
            .map_err(|err| {
                AnalyticsError::internal(
                    "suggestions_report_write_failed",
                    format!("failed to write csv row: {err}"),
                )
            })?;
    }

    writer.flush().map_err(|err| {
        AnalyticsError::internal(
            "suggestions_report_write_failed",
            format!("failed to flush csv: {err}"),
        )
    })
}

pub fn write_wix_unmapped_csv(path: &Path, rows: &[WixUnmappedItem]) -> Result<(), AnalyticsError> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| {
        AnalyticsError::new(
            "wix_unmapped_report_write_failed",
            format!("failed to open wix unmapped csv: {err}"),
            vec![path.display().to_string()],
            None,
        )
    })?;

    writer
        .write_record([
            "month",
            "raw_sku",
            "quantity",
            "item_name",
            "classification",
        ])
        .map_err(|err| {
            AnalyticsError::internal(
                "wix_unmapped_report_write_failed",
                format!("failed to write csv header: {err}"),
            )
        })?;

    for row in rows {
        writer
            .write_record([
                row.month.as_str(),
                row.raw_sku.as_str(),
                row.quantity.to_string().as_str(),
                row.item_name.as_deref().unwrap_or(""),
                row.classification.as_deref().unwrap_or(""),
            ])
            .map_err(|err| {
                AnalyticsError::internal(
                    "wix_unmapped_report_write_failed",
                    format!("failed to write csv row: {err}"),
                )
            })?;
    }

    writer.flush().map_err(|err| {
        AnalyticsError::internal(
            "wix_unmapped_report_write_failed",
            format!("failed to flush csv: {err}"),
        )
    })
}

fn header_index_map(headers: &StringRecord) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.to_string(), idx))
        .collect()
}

fn get_value<'a>(
    record: &'a StringRecord,
    index: &HashMap<String, usize>,
    key: &str,
) -> Option<String> {
    let idx = index.get(key)?;
    record.get(*idx).map(|value| value.trim().to_string())
}

fn normalize_name(value: &str) -> String {
    value
        .replace('®', "")
        .replace('™', "")
        .replace('\u{00a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn parse_sku_quantities(value: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut pieces = part.splitn(2, 'x');
        let qty_part = pieces.next().unwrap_or("").trim();
        let sku_part = pieces.next().unwrap_or("").trim();
        if let Ok(qty) = qty_part.parse::<u32>() {
            if !sku_part.is_empty() {
                out.push((sku_part.to_string(), qty));
            }
        }
    }
    out
}

fn parse_shipping_items(value: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((name_part, rest)) = part.split_once("- SKU:") {
            let name = name_part.trim().to_string();
            let sku = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c: char| c == '(' || c == ')')
                .to_string();
            if !name.is_empty() && !sku.is_empty() {
                out.push((name, sku));
            }
        }
    }
    out
}

fn parse_mix_match_selection(value: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut pieces = part.splitn(2, 'x');
        let qty_part = pieces.next().unwrap_or("").trim();
        let name_part = pieces.next().unwrap_or("").trim();
        if let Ok(qty) = qty_part.parse::<u32>() {
            if !name_part.is_empty() {
                out.push((name_part.to_string(), qty));
            }
        }
    }
    out
}

fn parse_named_quantity_list(value: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((qty_raw, name_raw)) = part.split_once('x') {
            if let Ok(qty) = qty_raw.trim().parse::<u32>() {
                let name = name_raw.trim().to_string();
                if !name.is_empty() {
                    out.push((name, qty));
                    continue;
                }
            }
        }
        out.push((part.to_string(), 1));
    }
    out
}

fn month_key(value: &str) -> String {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return format!("{:04}-{:02}", date.year(), date.month());
    }
    if value.len() == 8 {
        if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
            return format!("{:04}-{:02}", date.year(), date.month());
        }
    }
    String::from("unknown")
}

fn parse_decimal(value: &str) -> Option<Decimal> {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }
    Decimal::from_str_exact(trimmed).ok()
}

fn extract_bag_plan_name(products: &str) -> Option<String> {
    products
        .split(',')
        .map(|part| part.trim())
        .find(|part| part.contains("Bag Plan"))
        .map(|part| part.to_string())
}

fn extract_product_extras(products: &str) -> Vec<(String, u32)> {
    let mut extras = Vec::new();
    for part in products
        .split(',')
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        if part.contains("Bag Plan") {
            continue;
        }
        extras.push((part.to_string(), 1));
    }
    extras
}

fn median_decimal(values: &mut [Decimal]) -> Option<Decimal> {
    if values.is_empty() {
        return None;
    }
    values.sort();
    Some(values[values.len() / 2])
}

fn resolve_sku_from_name(
    item_name: &str,
    mapping: &SkuMappingRegistry,
    name_to_sku: &HashMap<String, String>,
) -> Option<String> {
    let normalized = normalize_name(item_name);
    if let Some(mapped) = name_to_sku.get(&normalized) {
        return Some(mapped.clone());
    }
    if let Some(mapped) = mapping.lookup("subbly", &normalized) {
        return Some(mapped);
    }
    let input_tokens = tokenize(item_name);
    let mut best: Option<(String, f64)> = None;
    for (candidate_name, sku) in name_to_sku.iter() {
        let score = jaccard_similarity(&input_tokens, &tokenize(candidate_name));
        if score >= 0.55
            && best
                .as_ref()
                .map(|(_, best_score)| score > *best_score)
                .unwrap_or(true)
        {
            best = Some((sku.clone(), score));
        }
    }
    best.map(|(sku, _)| sku)
}

fn parse_wix_bigquery_csv(path: &Path) -> Result<BTreeMap<(String, String), u64>, AnalyticsError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|err| {
            AnalyticsError::new(
                "wix_csv_unreadable",
                format!("failed to open Wix BigQuery CSV: {err}"),
                vec!["wix_csv_path".to_string()],
                None,
            )
        })?
        .read_to_end(&mut bytes)
        .map_err(|err| {
            AnalyticsError::new(
                "wix_csv_unreadable",
                format!("failed to read Wix BigQuery CSV: {err}"),
                vec!["wix_csv_path".to_string()],
                None,
            )
        })?;

    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader
        .headers()
        .map_err(|err| {
            AnalyticsError::new(
                "wix_csv_invalid",
                format!("failed to read Wix CSV headers: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?
        .clone();

    let date_col = find_column(
        &headers,
        &["order_date", "order_date_utc", "event_date", "date"],
    );
    let sku_col = find_column(&headers, &["sku", "item_sku", "item_id", "product_sku"]);
    let qty_col = find_column(&headers, &["quantity", "item_quantity", "qty"]);

    let (date_col, sku_col, qty_col) = match (date_col, sku_col, qty_col) {
        (Some(d), Some(s), Some(q)) => (d, s, q),
        _ => {
            return Err(AnalyticsError::new(
                "wix_csv_missing_columns",
                "Wix BigQuery CSV must include date, sku, and quantity columns",
                vec![path.display().to_string()],
                Some(serde_json::json!({
                    "expected_date_columns": ["order_date", "order_date_utc", "event_date", "date"],
                    "expected_sku_columns": ["sku", "item_sku", "item_id", "product_sku"],
                    "expected_quantity_columns": ["quantity", "item_quantity", "qty"],
                })),
            ));
        }
    };

    let mut out: BTreeMap<(String, String), u64> = BTreeMap::new();
    for result in reader.records() {
        let record = result.map_err(|err| {
            AnalyticsError::new(
                "wix_csv_invalid",
                format!("failed to parse Wix CSV row: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?;
        let date = record.get(date_col).unwrap_or("").trim();
        let sku = record.get(sku_col).unwrap_or("").trim();
        let qty_raw = record.get(qty_col).unwrap_or("0").trim();
        if sku.is_empty() || date.is_empty() {
            continue;
        }
        let qty: u64 = qty_raw.parse::<f64>().unwrap_or(0.0) as u64;
        if qty == 0 {
            continue;
        }
        let month = month_key(date);
        *out.entry((month, sku.to_string())).or_insert(0) += qty;
    }

    Ok(out)
}

fn wix_units_from_bigquery_rows(
    rows: &[WixItemRowV1],
    mapping: &SkuMappingRegistry,
) -> (
    BTreeMap<(String, String), u64>,
    BTreeMap<(String, String), Decimal>,
    Vec<WixUnmappedItem>,
    HashMap<String, String>,
) {
    let mut out: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut revenues: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let mut unmapped: BTreeMap<(String, String), (u64, Option<String>)> = BTreeMap::new();
    let mut wix_items: HashMap<String, String> = HashMap::new();
    for row in rows {
        let month = month_key(&row.event_date_utc);
        let item_name = row.item_name.clone().unwrap_or_else(|| row.sku.clone());
        let normalized = normalize_name(&row.sku);
        let mapped = mapping.lookup("wix", &normalized);
        let is_unmapped = mapped.is_none();
        let final_sku = mapped.unwrap_or_else(|| row.sku.clone());
        if let Some(recipe) = bundle_recipe_for_name(&item_name) {
            let row_revenue = row.item_revenue.unwrap_or(Decimal::ZERO);
            let recipe_total_units: u64 = recipe.iter().map(|component| component.quantity).sum();
            let per_unit_revenue = if recipe_total_units == 0 {
                Decimal::ZERO
            } else {
                row_revenue / Decimal::from(recipe_total_units)
            };
            for component in recipe {
                let component_units = row.quantity * component.quantity;
                let key = (month.clone(), component.sku.to_string());
                *out.entry(key.clone()).or_insert(0) += component_units;
                *revenues.entry(key).or_insert(Decimal::ZERO) +=
                    per_unit_revenue * Decimal::from(component_units);
            }
        } else {
            let key = (month.clone(), final_sku.clone());
            *out.entry(key.clone()).or_insert(0) += row.quantity;
            *revenues.entry(key).or_insert(Decimal::ZERO) +=
                row.item_revenue.unwrap_or(Decimal::ZERO);
        }
        if is_unmapped && bundle_recipe_for_name(&item_name).is_none() {
            let entry = unmapped
                .entry((month, row.sku.clone()))
                .or_insert((0, row.item_name.clone()));
            entry.0 += row.quantity;
        }
        if let Some(item_name) = row.item_name.as_ref() {
            wix_items
                .entry(row.sku.clone())
                .or_insert_with(|| item_name.clone());
        }
    }
    let unmapped_rows = unmapped
        .into_iter()
        .map(|((month, raw_sku), (quantity, item_name))| {
            let classification = item_name
                .as_deref()
                .map(classify_unmapped_wix_item_name)
                .map(str::to_string);
            WixUnmappedItem {
                month,
                raw_sku,
                quantity,
                item_name,
                classification,
            }
        })
        .collect();
    (out, revenues, unmapped_rows, wix_items)
}

fn build_subbly_report_core(
    subbly_csv_path: &Path,
    wix_units: BTreeMap<(String, String), u64>,
    wix_revenues: BTreeMap<(String, String), Decimal>,
    mapping: Option<SkuMappingRegistry>,
    wix_unmapped: Option<Vec<WixUnmappedItem>>,
    wix_items: HashMap<String, String>,
) -> Result<SubblyWixReportOutput, AnalyticsError> {
    let mut subbly_bytes = Vec::new();
    File::open(subbly_csv_path)
        .map_err(|err| {
            AnalyticsError::new(
                "subbly_csv_unreadable",
                format!("failed to open Subbly CSV: {err}"),
                vec!["subbly_csv_path".to_string()],
                None,
            )
        })?
        .read_to_end(&mut subbly_bytes)
        .map_err(|err| {
            AnalyticsError::new(
                "subbly_csv_unreadable",
                format!("failed to read Subbly CSV: {err}"),
                vec!["subbly_csv_path".to_string()],
                None,
            )
        })?;

    let mut reader = csv::Reader::from_reader(subbly_bytes.as_slice());
    let headers = reader
        .headers()
        .map_err(|err| {
            AnalyticsError::new(
                "subbly_csv_invalid",
                format!("failed to read Subbly CSV headers: {err}"),
                vec!["subbly_csv_path".to_string()],
                None,
            )
        })?
        .clone();

    let header_index = header_index_map(&headers);
    let mix_match_columns = headers
        .iter()
        .filter(|name| name.starts_with("Mix & Match"))
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    let mapping = mapping.unwrap_or_default();
    let mut records = Vec::new();
    for result in reader.records() {
        records.push(result.map_err(|err| {
            AnalyticsError::new(
                "subbly_csv_invalid",
                format!("failed to parse Subbly CSV row: {err}"),
                vec!["subbly_csv_path".to_string()],
                None,
            )
        })?);
    }

    let mut name_to_sku: HashMap<String, String> = HashMap::new();
    let mut subbly_names: HashMap<String, String> = HashMap::new();
    let mut conflicts: Vec<SkuMappingConflict> = Vec::new();
    let mut bag_plan_baselines: HashMap<String, Decimal> = HashMap::new();
    let mut clean_plan_samples: HashMap<String, Vec<Decimal>> = HashMap::new();

    for record in records.iter() {
        let shipping_items = get_value(record, &header_index, "Shipping Items").unwrap_or_default();
        if !shipping_items.is_empty() {
            for (item_name, sku) in parse_shipping_items(&shipping_items) {
                let key = normalize_name(&item_name);
                subbly_names.entry(sku.clone()).or_insert(item_name.clone());
                if let Some(existing) = name_to_sku.get(&key) {
                    if existing != &sku {
                        conflicts.push(SkuMappingConflict {
                            item_name,
                            existing_sku: existing.clone(),
                            new_sku: sku.clone(),
                        });
                    }
                } else {
                    name_to_sku.insert(key, sku);
                }
            }
        }

        let products = get_value(record, &header_index, "Products").unwrap_or_default();
        let add_supplements =
            get_value(record, &header_index, "Add Supplements").unwrap_or_default();
        let add_treats = get_value(record, &header_index, "Add Treats").unwrap_or_default();
        let order_subtotal = get_value(record, &header_index, "Order Subtotal")
            .and_then(|value| parse_decimal(&value))
            .unwrap_or(Decimal::ZERO);
        let has_clean_plan_shape = extract_bag_plan_name(&products).is_some()
            && extract_product_extras(&products).is_empty()
            && add_supplements.trim().is_empty()
            && add_treats.trim().is_empty()
            && !order_subtotal.is_zero();
        if has_clean_plan_shape {
            if let Some(plan_name) = extract_bag_plan_name(&products) {
                clean_plan_samples
                    .entry(plan_name)
                    .or_default()
                    .push(order_subtotal);
            }
        }
    }

    for (plan_name, mut samples) in clean_plan_samples {
        if let Some(median) = median_decimal(&mut samples) {
            bag_plan_baselines.insert(plan_name, median);
        }
    }

    let mut subbly_direct: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut subbly_reconstructed: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut subbly_direct_revenue: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let mut subbly_reconstructed_revenue: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let mut unresolved: Vec<UnresolvedMixMatchItem> = Vec::new();
    let wix_unmapped = wix_unmapped.unwrap_or_default();

    for record in records.into_iter() {
        let order_id = get_value(&record, &header_index, "Order ID").unwrap_or_default();
        let shipping_date = get_value(&record, &header_index, "Shipping Date").unwrap_or_default();
        let month = month_key(&shipping_date);
        let sku_field = get_value(&record, &header_index, "SKUs").unwrap_or_default();
        let order_subtotal = get_value(&record, &header_index, "Order Subtotal")
            .and_then(|value| parse_decimal(&value))
            .unwrap_or(Decimal::ZERO);
        let products = get_value(&record, &header_index, "Products").unwrap_or_default();
        let add_supplements =
            get_value(&record, &header_index, "Add Supplements").unwrap_or_default();
        let add_treats = get_value(&record, &header_index, "Add Treats").unwrap_or_default();

        let mut direct_items: Vec<(String, u64)> = Vec::new();
        let mut reconstructed_items: Vec<(String, u64)> = Vec::new();

        if !sku_field.is_empty() {
            for (sku, qty) in parse_sku_quantities(&sku_field) {
                let normalized = normalize_name(&sku);
                let final_sku = mapping
                    .lookup("subbly", &normalized)
                    .unwrap_or_else(|| sku.clone());
                direct_items.push((final_sku, qty as u64));
            }
        } else if !mix_match_columns.is_empty() {
            let selections = mix_match_columns
                .iter()
                .filter_map(|col| get_value(&record, &header_index, col))
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>();
            for selection in selections {
                for (item_name, qty) in parse_mix_match_selection(&selection) {
                    if let Some(sku) = resolve_sku_from_name(&item_name, &mapping, &name_to_sku) {
                        reconstructed_items.push((sku, qty as u64));
                    } else {
                        unresolved.push(UnresolvedMixMatchItem {
                            order_id: order_id.clone(),
                            shipping_date: shipping_date.clone(),
                            selection_text: selection.clone(),
                            item_name,
                        });
                    }
                }
            }
        }

        let mut extra_item_counts: HashMap<String, u64> = HashMap::new();
        for (name, qty) in extract_product_extras(&products) {
            if let Some(sku) = resolve_sku_from_name(&name, &mapping, &name_to_sku) {
                *extra_item_counts.entry(sku).or_insert(0) += qty as u64;
            }
        }
        for (name, qty) in parse_named_quantity_list(&add_supplements) {
            if let Some(sku) = resolve_sku_from_name(&name, &mapping, &name_to_sku) {
                *extra_item_counts.entry(sku).or_insert(0) += qty as u64;
            }
        }
        for (name, qty) in parse_named_quantity_list(&add_treats) {
            if let Some(sku) = resolve_sku_from_name(&name, &mapping, &name_to_sku) {
                *extra_item_counts.entry(sku).or_insert(0) += qty as u64;
            }
        }

        let mut order_counts: HashMap<String, u64> = HashMap::new();
        for (sku, qty) in direct_items.iter().chain(reconstructed_items.iter()) {
            *order_counts.entry(sku.clone()).or_insert(0) += *qty;
        }
        let total_units: u64 = order_counts.values().sum();
        if total_units == 0 {
            continue;
        }

        let bag_plan_name = extract_bag_plan_name(&products);
        let bag_plan_baseline = bag_plan_name
            .as_ref()
            .and_then(|plan_name| bag_plan_baselines.get(plan_name))
            .copied();
        let product_extras_present = !extract_product_extras(&products).is_empty()
            || !add_supplements.trim().is_empty()
            || !add_treats.trim().is_empty();

        let mut plan_counts: HashMap<String, u64> = order_counts.clone();
        if bag_plan_name.is_some() {
            for (sku, qty) in extra_item_counts.iter() {
                if let Some(count) = plan_counts.get_mut(sku) {
                    *count = count.saturating_sub(*qty);
                }
            }
        }
        let plan_units: u64 = plan_counts.values().sum();
        let extra_units: u64 = extra_item_counts.values().sum();

        let use_plan_aware_allocation = bag_plan_baseline.is_some()
            && plan_units > 0
            && (!product_extras_present || extra_units > 0);

        let fallback_revenue_per_unit = order_subtotal / Decimal::from(total_units);
        let plan_revenue_budget = if use_plan_aware_allocation && extra_units > 0 {
            bag_plan_baseline
                .unwrap_or(order_subtotal)
                .min(order_subtotal)
        } else {
            order_subtotal
        };
        let extra_revenue_budget = if use_plan_aware_allocation && extra_units > 0 {
            (order_subtotal - plan_revenue_budget).max(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        let plan_revenue_per_unit = if use_plan_aware_allocation {
            if plan_units == 0 {
                Decimal::ZERO
            } else {
                plan_revenue_budget / Decimal::from(plan_units)
            }
        } else {
            fallback_revenue_per_unit
        };
        let extra_revenue_per_unit = if use_plan_aware_allocation && extra_units > 0 {
            extra_revenue_budget / Decimal::from(extra_units)
        } else {
            fallback_revenue_per_unit
        };

        let mut remaining_plan_counts = plan_counts.clone();
        for (sku, qty) in direct_items {
            let key = (month.clone(), sku.clone());
            *subbly_direct.entry(key.clone()).or_insert(0) += qty;
            let plan_qty = remaining_plan_counts
                .get(&sku)
                .copied()
                .unwrap_or(0)
                .min(qty);
            if let Some(remaining) = remaining_plan_counts.get_mut(&sku) {
                *remaining = remaining.saturating_sub(plan_qty);
            }
            let extra_qty = qty.saturating_sub(plan_qty);
            let revenue = if use_plan_aware_allocation {
                (plan_revenue_per_unit * Decimal::from(plan_qty))
                    + (extra_revenue_per_unit * Decimal::from(extra_qty))
            } else {
                fallback_revenue_per_unit * Decimal::from(qty)
            };
            *subbly_direct_revenue.entry(key).or_insert(Decimal::ZERO) += revenue;
        }

        let mut remaining_plan_counts = plan_counts;
        for (sku, qty) in reconstructed_items {
            let key = (month.clone(), sku.clone());
            *subbly_reconstructed.entry(key.clone()).or_insert(0) += qty;
            let plan_qty = remaining_plan_counts
                .get(&sku)
                .copied()
                .unwrap_or(0)
                .min(qty);
            if let Some(remaining) = remaining_plan_counts.get_mut(&sku) {
                *remaining = remaining.saturating_sub(plan_qty);
            }
            let extra_qty = qty.saturating_sub(plan_qty);
            let revenue = if use_plan_aware_allocation {
                (plan_revenue_per_unit * Decimal::from(plan_qty))
                    + (extra_revenue_per_unit * Decimal::from(extra_qty))
            } else {
                fallback_revenue_per_unit * Decimal::from(qty)
            };
            *subbly_reconstructed_revenue
                .entry(key)
                .or_insert(Decimal::ZERO) += revenue;
        }
    }

    let mut combined_keys: BTreeMap<(String, String), ()> = BTreeMap::new();
    for key in subbly_direct.keys() {
        combined_keys.insert(key.clone(), ());
    }
    for key in subbly_reconstructed.keys() {
        combined_keys.insert(key.clone(), ());
    }
    for key in wix_units.keys() {
        combined_keys.insert(key.clone(), ());
    }
    for key in wix_revenues.keys() {
        combined_keys.insert(key.clone(), ());
    }

    let mut rows = Vec::new();
    for (key, _) in combined_keys {
        let subbly = *subbly_direct.get(&key).unwrap_or(&0);
        let recon = *subbly_reconstructed.get(&key).unwrap_or(&0);
        let wix = *wix_units.get(&key).unwrap_or(&0);
        let combined = subbly + recon + wix;
        let subbly_revenue = *subbly_direct_revenue.get(&key).unwrap_or(&Decimal::ZERO);
        let subbly_reconstructed_revenue = *subbly_reconstructed_revenue
            .get(&key)
            .unwrap_or(&Decimal::ZERO);
        let wix_revenue = *wix_revenues.get(&key).unwrap_or(&Decimal::ZERO);
        let combined_revenue = subbly_revenue + subbly_reconstructed_revenue + wix_revenue;
        rows.push(MonthlySkuSalesRow {
            month: key.0,
            sku: key.1,
            subbly_units: subbly,
            subbly_reconstructed_units: recon,
            wix_units: wix,
            combined_units: combined,
            subbly_revenue,
            subbly_reconstructed_revenue,
            wix_revenue,
            combined_revenue,
        });
    }

    Ok(SubblyWixReportOutput {
        rows,
        unresolved,
        conflicts,
        wix_unmapped,
        suggestions: suggest_mappings_from_names(&wix_items, &subbly_names),
    })
}

fn find_column(headers: &StringRecord, candidates: &[&str]) -> Option<usize> {
    for (idx, header) in headers.iter().enumerate() {
        let normalized = header.trim().to_lowercase();
        if candidates.iter().any(|c| normalized == *c) {
            return Some(idx);
        }
    }
    None
}

pub fn default_report_paths(out_dir: &Path, tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let report_path = out_dir.join(format!("subbly_wix_monthly_orders_{tag}.csv"));
    let unresolved_path = out_dir.join(format!("subbly_wix_monthly_orders_unresolved_{tag}.csv"));
    let conflicts_path = out_dir.join(format!("subbly_wix_monthly_orders_conflicts_{tag}.csv"));
    (report_path, unresolved_path, conflicts_path)
}

pub fn default_wix_unmapped_path(out_dir: &Path, tag: &str) -> PathBuf {
    out_dir.join(format!("subbly_wix_monthly_orders_wix_unmapped_{tag}.csv"))
}

pub fn default_suggestions_path(out_dir: &Path, tag: &str) -> PathBuf {
    out_dir.join(format!(
        "subbly_wix_monthly_orders_mapping_suggestions_{tag}.csv"
    ))
}

#[derive(Debug, Clone, Default)]
struct SkuMappingRegistry {
    global: HashMap<String, String>,
    by_source: HashMap<String, HashMap<String, String>>,
}

impl SkuMappingRegistry {
    fn lookup(&self, source: &str, normalized_value: &str) -> Option<String> {
        if let Some(source_map) = self.by_source.get(source) {
            if let Some(mapped) = source_map.get(normalized_value) {
                return Some(mapped.clone());
            }
        }
        self.global.get(normalized_value).cloned()
    }
}

fn suggest_mappings_from_names(
    wix_items: &HashMap<String, String>,
    subbly_names: &HashMap<String, String>,
) -> Vec<SkuMappingSuggestion> {
    let mut subbly_candidates = Vec::new();
    for (sku, name) in subbly_names.iter() {
        subbly_candidates.push((sku.clone(), name.clone()));
    }
    let mut suggestions = Vec::new();
    for (wix_id, wix_name) in wix_items.iter() {
        let wix_tokens = tokenize(wix_name);
        if wix_tokens.is_empty() {
            continue;
        }
        let mut best = Vec::new();
        for (subbly_sku, subbly_name) in subbly_candidates.iter() {
            let subbly_tokens = tokenize(subbly_name);
            let score = jaccard_similarity(&wix_tokens, &subbly_tokens);
            if score >= 0.35 {
                best.push(SkuMappingSuggestion {
                    wix_item_id: wix_id.clone(),
                    wix_item_name: wix_name.clone(),
                    subbly_sku: subbly_sku.clone(),
                    subbly_name: subbly_name.clone(),
                    similarity: score,
                });
            }
        }
        best.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        suggestions.extend(best.into_iter().take(3));
    }
    suggestions
}

fn bundle_recipe_for_name(value: &str) -> Option<Vec<BundleRecipeItem>> {
    let normalized = normalize_name(value);
    match normalized.as_str() {
        "simply raw all flavors mix" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0414001",
                quantity: 1,
            },
        ]),
        "simply raw : 2 beef 1 turkey" | "simply raw 2 beef 1 turkey" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414001",
                quantity: 1,
            },
        ]),
        "simply raw : 2 beef 1 chicken" | "simply raw 2 beef 1 chicken" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 1,
            },
        ]),
        "simply raw 2 turkey 1 beef" | "simply raw : 2 turkey 1 beef" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414001",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 1,
            },
        ]),
        "simply raw : 2 chicken 1 turkey" | "simply raw 2 chicken 1 turkey" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414001",
                quantity: 1,
            },
        ]),
        "simply raw : 2 chicken 1 beef" | "simply raw 2 chicken 1 beef" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 1,
            },
        ]),
        "simply raw: 2 turkey 1 chicken" | "simply raw : 2 turkey 1 chicken" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0414001",
                quantity: 2,
            },
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 1,
            },
        ]),
        "bone broth assortment value pack" | "bone broth assortment" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0733009",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0733008",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0733007",
                quantity: 1,
            },
        ]),
        "ready raw value assortment (for dogs)" => Some(vec![
            BundleRecipeItem {
                sku: "1-1",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0414002",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0414003",
                quantity: 1,
            },
        ]),
        "wellness value bundle" => Some(vec![
            BundleRecipeItem {
                sku: "NDP-0733010",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0733011",
                quantity: 1,
            },
            BundleRecipeItem {
                sku: "NDP-0733002",
                quantity: 1,
            },
        ]),
        "hip & joint value bundle" => Some(vec![BundleRecipeItem {
            sku: "NDP-0733009",
            quantity: 3,
        }]),
        _ => None,
    }
}

fn classify_unmapped_wix_item_name(value: &str) -> &'static str {
    let normalized = normalize_name(value);
    match normalized.as_str() {
        "ready raw value assortment (for dogs)"
        | "bone broth assortment value pack"
        | "bone broth assortment"
        | "wellness value bundle"
        | "hip & joint value bundle" => "bundle_unknown_recipe",
        _ if normalized.contains("bundle") || normalized.contains("assortment") => {
            "bundle_unknown_recipe"
        }
        _ if normalized.contains("trial size") => "trial_size_wix_only",
        _ => "wix_only_product",
    }
}

fn tokenize(value: &str) -> Vec<String> {
    let normalized = normalize_name(value);
    normalized
        .split_whitespace()
        .filter(|token| !is_stopword(token))
        .map(|token| token.to_string())
        .collect()
}

fn is_stopword(value: &str) -> bool {
    matches!(
        value,
        "for"
            | "dogs"
            | "dog"
            | "cats"
            | "cat"
            | "bag"
            | "bags"
            | "plan"
            | "assortment"
            | "value"
            | "lb"
            | "lbs"
            | "raw"
            | "simply"
            | "ready"
            | "months"
            | "month"
    )
}

fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn load_mapping_csv(path: Option<&Path>) -> Result<(SkuMappingRegistry, bool), AnalyticsError> {
    let Some(path) = path else {
        return Ok((SkuMappingRegistry::default(), false));
    };
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|err| {
            AnalyticsError::new(
                "sku_mapping_csv_unreadable",
                format!("failed to open mapping CSV: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?
        .read_to_end(&mut bytes)
        .map_err(|err| {
            AnalyticsError::new(
                "sku_mapping_csv_unreadable",
                format!("failed to read mapping CSV: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?;

    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader
        .headers()
        .map_err(|err| {
            AnalyticsError::new(
                "sku_mapping_csv_invalid",
                format!("failed to read mapping CSV headers: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?
        .clone();
    let header_index = header_index_map(&headers);
    let mut registry = SkuMappingRegistry::default();
    for result in reader.records() {
        let record = result.map_err(|err| {
            AnalyticsError::new(
                "sku_mapping_csv_invalid",
                format!("failed to parse mapping CSV row: {err}"),
                vec![path.display().to_string()],
                None,
            )
        })?;
        let source = get_value(&record, &header_index, "source").unwrap_or_default();
        let value = get_value(&record, &header_index, "value").unwrap_or_default();
        let canonical = get_value(&record, &header_index, "canonical_sku").unwrap_or_default();
        if value.is_empty() || canonical.is_empty() {
            continue;
        }
        let normalized = normalize_name(&value);
        if source.is_empty() {
            registry.global.insert(normalized, canonical);
        } else {
            let entry = registry.by_source.entry(source.to_lowercase()).or_default();
            entry.insert(normalized, canonical);
        }
    }
    Ok((registry, true))
}
