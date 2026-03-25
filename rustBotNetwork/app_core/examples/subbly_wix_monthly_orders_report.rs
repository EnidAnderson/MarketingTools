use app_core::subsystems::marketing_data_analysis::{
    analytics_connector_config_from_env, build_subbly_wix_monthly_report,
    build_subbly_wix_monthly_report_with_bigquery, default_report_paths,
    default_wix_unmapped_path, write_conflicts_csv, write_monthly_report_csv,
    write_unresolved_csv, write_wix_unmapped_csv, default_suggestions_path,
    write_suggestions_csv,
    ObservedReadOnlyAnalyticsConnectorV2,
};
use chrono::Utc;
use std::env;
use std::path::{Path, PathBuf};
use tokio::runtime::Builder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let args: Vec<String> = env::args().collect();
    let mut subbly_csv: Option<PathBuf> = None;
    let mut wix_csv: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from("exports");
    let mut start_date: Option<String> = None;
    let mut end_date: Option<String> = None;
    let mut use_bigquery = false;
    let mut mapping_csv: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--subbly-csv" => {
                if let Some(value) = args.get(i + 1) {
                    subbly_csv = Some(PathBuf::from(value));
                    i += 1;
                }
            }
            "--wix-csv" => {
                if let Some(value) = args.get(i + 1) {
                    wix_csv = Some(PathBuf::from(value));
                    i += 1;
                }
            }
            "--out-dir" => {
                if let Some(value) = args.get(i + 1) {
                    out_dir = PathBuf::from(value);
                    i += 1;
                }
            }
            "--start-date" => {
                if let Some(value) = args.get(i + 1) {
                    start_date = Some(value.to_string());
                    i += 1;
                }
            }
            "--end-date" => {
                if let Some(value) = args.get(i + 1) {
                    end_date = Some(value.to_string());
                    i += 1;
                }
            }
            "--use-bigquery" => {
                use_bigquery = true;
            }
            "--mapping-csv" => {
                if let Some(value) = args.get(i + 1) {
                    mapping_csv = Some(PathBuf::from(value));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let subbly_csv = subbly_csv.ok_or("--subbly-csv is required")?;
    let wix_csv_ref = wix_csv.as_deref();

    let report = if use_bigquery {
        let start_date = start_date.ok_or("--start-date is required with --use-bigquery")?;
        let end_date = end_date.ok_or("--end-date is required with --use-bigquery")?;
        std::env::set_var("ANALYTICS_ENABLE_GOOGLE_ADS", "false");
        std::env::set_var("ANALYTICS_ENABLE_WIX", "false");
        let config = analytics_connector_config_from_env()?;
        let connector = ObservedReadOnlyAnalyticsConnectorV2::new();
        let runtime = Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(build_subbly_wix_monthly_report_with_bigquery(
            &subbly_csv,
            &connector,
            &config,
            &start_date,
            &end_date,
            mapping_csv.as_deref(),
        ))?
    } else {
        build_subbly_wix_monthly_report(&subbly_csv, wix_csv_ref, mapping_csv.as_deref())?
    };

    let tag = Utc::now().format("%Y-%m-%d").to_string();
    let (report_path, unresolved_path, conflicts_path) = default_report_paths(&out_dir, &tag);
    let wix_unmapped_path = default_wix_unmapped_path(&out_dir, &tag);
    let suggestions_path = default_suggestions_path(&out_dir, &tag);

    ensure_parent(&report_path)?;
    ensure_parent(&unresolved_path)?;
    ensure_parent(&conflicts_path)?;
    ensure_parent(&wix_unmapped_path)?;
    ensure_parent(&suggestions_path)?;

    write_monthly_report_csv(&report_path, &report.rows)?;
    write_unresolved_csv(&unresolved_path, &report.unresolved)?;
    write_conflicts_csv(&conflicts_path, &report.conflicts)?;
    write_wix_unmapped_csv(&wix_unmapped_path, &report.wix_unmapped)?;
    write_suggestions_csv(&suggestions_path, &report.suggestions)?;

    println!("Monthly SKU report: {}", report_path.display());
    println!("Unresolved Mix & Match items: {}", unresolved_path.display());
    println!("SKU mapping conflicts: {}", conflicts_path.display());
    if !report.wix_unmapped.is_empty() {
        println!("Unmapped Wix items: {}", wix_unmapped_path.display());
    }
    if !report.suggestions.is_empty() {
        println!("Mapping suggestions: {}", suggestions_path.display());
    }

    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
