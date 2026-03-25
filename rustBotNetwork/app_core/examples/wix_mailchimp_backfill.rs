use app_core::subsystems::mailchimp_backfill::{
    run_wix_mailchimp_backfill_v1, BackfillPhaseV1, BackfillRunOptionsV1,
    MailchimpBackfillConfigV1, SourceModeV1,
};
use dotenv::dotenv;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let options = parse_args(env::args().skip(1).collect())?;
    let config = MailchimpBackfillConfigV1::from_env()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let artifacts = runtime.block_on(run_wix_mailchimp_backfill_v1(&config, &options))?;
    println!("Dry-run manifest: {}", artifacts.manifest_path.display());
    if let Some(path) = artifacts.reconciliation_path {
        println!("Reconciliation: {}", path.display());
    }
    if let Some(path) = artifacts.idempotency_path {
        println!("Idempotency report: {}", path.display());
    }
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<BackfillRunOptionsV1, Box<dyn std::error::Error>> {
    let mut options = BackfillRunOptionsV1::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--phase" => {
                index += 1;
                options.phase = match required_arg(&args, index, "--phase")? {
                    "dry-run" => BackfillPhaseV1::DryRun,
                    "pilot" => BackfillPhaseV1::Pilot,
                    "full" => BackfillPhaseV1::Full,
                    other => return Err(format!("unsupported --phase value: {other}").into()),
                };
            }
            "--source" => {
                index += 1;
                options.source_mode = match required_arg(&args, index, "--source")? {
                    "api" => SourceModeV1::WixApi,
                    "csv" => SourceModeV1::WixCsv,
                    other => return Err(format!("unsupported --source value: {other}").into()),
                };
            }
            "--csv" => {
                index += 1;
                options.csv_path = Some(PathBuf::from(required_arg(&args, index, "--csv")?));
            }
            "--out-dir" => {
                index += 1;
                options.out_dir = PathBuf::from(required_arg(&args, index, "--out-dir")?);
            }
            "--pilot-limit" => {
                index += 1;
                options.pilot_limit = required_arg(&args, index, "--pilot-limit")?.parse()?;
            }
            "--max-orders" => {
                index += 1;
                options.max_orders = Some(required_arg(&args, index, "--max-orders")?.parse()?);
            }
            "--allow-existing-store-write" => {
                options.allow_existing_store_write = true;
            }
            "--not-all-history" => {
                options.all_history = false;
            }
            _ => {}
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
        .map(|value| value.as_str())
        .ok_or_else(|| format!("{flag} requires a value").into())
}
