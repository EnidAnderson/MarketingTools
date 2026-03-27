use app_core::subsystems::marketing_data_analysis::{
    analytics_connector_config_from_env, build_analytics_connector_v2,
    build_purchase_truth_audit_v1, evaluate_analytics_connectors_preflight,
    AnalyticsConnectorModeV1, Ga4ReadBackendV1,
};
use chrono::{Duration, NaiveDate, Utc};
use dotenv::dotenv;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

const DEFAULT_WINDOW_DAYS: i64 = 7;

#[derive(Debug)]
struct CliOptions {
    profile_id: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    window_days: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseTruthAuditMaterializationV1 {
    schema_version: String,
    generated_at_utc: String,
    profile_id: String,
    start_date: String,
    end_date: String,
    ga4_read_backend: String,
    total_ga4_events: usize,
    purchase_truth_audit: app_core::subsystems::marketing_data_analysis::PurchaseTruthAuditReportV1,
    orphan_event_samples: Vec<OrphanPurchaseSampleV1>,
}

#[derive(Debug, Serialize)]
struct OrphanPurchaseSampleV1 {
    event_timestamp_utc: String,
    user_pseudo_id: String,
    session_key: String,
    device_category: String,
    source_medium: String,
    page_location: Option<String>,
    transaction_id: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let options = parse_args(env::args().skip(1).collect())?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main(options))
}

async fn async_main(options: CliOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = analytics_connector_config_from_env().map_err(render_err)?;
    if let Some(profile_id) = &options.profile_id {
        config.profile_id = profile_id.clone();
    }
    if config.mode != AnalyticsConnectorModeV1::ObservedReadOnly {
        return Err(format!(
            "expected ANALYTICS_CONNECTOR_MODE=observed_read_only, got {:?}",
            config.mode
        )
        .into());
    }
    config.ga4.read_backend = Ga4ReadBackendV1::BigqueryExport;
    config.ga4.bigquery_max_rows = config.ga4.bigquery_max_rows.max(100_000);

    let connector = build_analytics_connector_v2(&config);
    let preflight = evaluate_analytics_connectors_preflight(connector.as_ref(), &config).await;
    if !preflight.ok {
        return Err(format!(
            "analytics preflight blocked: {}",
            serde_json::to_string_pretty(&preflight)?
        )
        .into());
    }

    let (start_date, end_date) = resolve_date_window(&options)?;
    let ga4_events = connector
        .fetch_ga4_events(&config, start_date, end_date, 0)
        .await
        .map_err(render_err)?;
    let audit = build_purchase_truth_audit_v1(&ga4_events, 30);

    let payload = PurchaseTruthAuditMaterializationV1 {
        schema_version: "purchase_truth_audit_materialization.v1".to_string(),
        generated_at_utc: Utc::now().to_rfc3339(),
        profile_id: config.profile_id.clone(),
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        ga4_read_backend: "bigquery_export".to_string(),
        total_ga4_events: ga4_events.len(),
        purchase_truth_audit: audit,
        orphan_event_samples: collect_orphan_purchase_samples(&ga4_events, 30, 25),
    };

    let export_id = format!(
        "purchase-truth-{}-{}",
        now_millis(),
        config.profile_id.replace('/', "_")
    );
    let output_dir = PathBuf::from("exports/live_dashboard").join(&export_id);
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("purchase_truth_audit.json");
    fs::write(&output_path, serde_json::to_vec_pretty(&payload)?)?;
    println!("{}", output_path.display());
    Ok(())
}

fn collect_orphan_purchase_samples(
    ga4_events: &[app_core::subsystems::marketing_data_analysis::Ga4EventRawV1],
    tolerance_seconds: i64,
    limit: usize,
) -> Vec<OrphanPurchaseSampleV1> {
    use app_core::subsystems::marketing_data_analysis::purchase_truth::{
        ga4_canonical_purchase_truth_key_v1, ga4_event_epoch_seconds_v1, ga4_session_key_v1,
        ga4_transaction_id_v1, has_canonical_purchase_within_window_v1,
        is_ga4_canonical_purchase_event_v1, is_ga4_custom_purchase_event_v1,
    };

    let mut canonical_purchase_seconds: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    let mut seen_truth_keys = BTreeSet::new();
    for event in ga4_events {
        if !is_ga4_canonical_purchase_event_v1(&event.event_name) {
            continue;
        }
        let Some(truth_key) = ga4_canonical_purchase_truth_key_v1(event) else {
            continue;
        };
        if !seen_truth_keys.insert(truth_key) {
            continue;
        }
        let Some(event_second) = ga4_event_epoch_seconds_v1(event) else {
            continue;
        };
        canonical_purchase_seconds
            .entry((
                event.user_pseudo_id.trim().to_string(),
                ga4_session_key_v1(event),
            ))
            .or_default()
            .push(event_second);
    }
    for seconds in canonical_purchase_seconds.values_mut() {
        seconds.sort_unstable();
    }

    let mut orphan_rows = ga4_events
        .iter()
        .filter(|event| is_ga4_custom_purchase_event_v1(&event.event_name))
        .filter_map(|event| {
            let event_second = ga4_event_epoch_seconds_v1(event)?;
            if has_canonical_purchase_within_window_v1(
                &canonical_purchase_seconds,
                event.user_pseudo_id.trim(),
                &ga4_session_key_v1(event),
                event_second,
                tolerance_seconds,
            ) {
                return None;
            }
            Some(OrphanPurchaseSampleV1 {
                event_timestamp_utc: event.event_timestamp_utc.clone(),
                user_pseudo_id: event.user_pseudo_id.clone(),
                session_key: ga4_session_key_v1(event),
                device_category: event
                    .device_category
                    .clone()
                    .unwrap_or_else(|| "unknown_device".to_string()),
                source_medium: event
                    .source_medium
                    .clone()
                    .unwrap_or_else(|| "unknown_source_medium".to_string()),
                page_location: event.page_location.clone(),
                transaction_id: ga4_transaction_id_v1(event),
            })
        })
        .collect::<Vec<_>>();
    orphan_rows.sort_by(|left, right| left.event_timestamp_utc.cmp(&right.event_timestamp_utc));
    orphan_rows.truncate(limit);
    orphan_rows
}

fn resolve_date_window(
    options: &CliOptions,
) -> Result<(NaiveDate, NaiveDate), Box<dyn std::error::Error>> {
    let today = Utc::now().date_naive();
    let end = options
        .end_date
        .unwrap_or_else(|| today - Duration::days(1));
    let start = options
        .start_date
        .unwrap_or_else(|| end - Duration::days(options.window_days.saturating_sub(1)));
    if start > end {
        return Err("start_date must be <= end_date".into());
    }
    Ok((start, end))
}

fn parse_args(args: Vec<String>) -> Result<CliOptions, Box<dyn std::error::Error>> {
    let mut options = CliOptions {
        profile_id: None,
        start_date: None,
        end_date: None,
        window_days: DEFAULT_WINDOW_DAYS,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                options.profile_id = Some(required_arg(&args, index, "--profile")?.to_string());
            }
            "--start-date" => {
                index += 1;
                options.start_date = Some(parse_date(required_arg(&args, index, "--start-date")?)?);
            }
            "--end-date" => {
                index += 1;
                options.end_date = Some(parse_date(required_arg(&args, index, "--end-date")?)?);
            }
            "--window-days" => {
                index += 1;
                options.window_days =
                    required_arg(&args, index, "--window-days")?.parse::<i64>()?;
            }
            flag => return Err(format!("unrecognized flag: {flag}").into()),
        }
        index += 1;
    }
    Ok(options)
}

fn required_arg<'a>(
    args: &'a [String],
    index: usize,
    flag: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn parse_date(value: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    Ok(NaiveDate::parse_from_str(value, "%Y-%m-%d")?)
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn render_err(err: app_core::subsystems::marketing_data_analysis::AnalyticsError) -> String {
    format!("{}: {}", err.code, err.message)
}
