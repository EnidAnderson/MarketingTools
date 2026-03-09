use app_core::subsystems::marketing_data_analysis::{
    analytics_connector_config_fingerprint_v1, analytics_connector_config_from_env,
    build_analytics_connector_v2, build_executive_dashboard_snapshot, build_historical_analysis,
    evaluate_analytics_connectors_preflight, load_attestation_key_registry_from_env_or_file,
    resolve_attestation_policy_v1, verify_connector_attestation_with_registry_v1,
    AnalyticsConnectorModeV1, AnalyticsRunStore, BudgetEnvelopeV1, ConnectorConfigAttestationV1,
    DashboardExportAuditRecordV1, DashboardExportAuditStore, DefaultMarketAnalysisService,
    Ga4ReadBackendV1, MarketAnalysisService, MockAnalyticsRequestV1, PersistedAnalyticsRunV1,
    SnapshotBuildOptions, CONNECTOR_CONFIG_FINGERPRINT_ALG_V1,
    CONNECTOR_CONFIG_FINGERPRINT_SCHEMA_V1,
};
use chrono::{Duration, NaiveDate, Utc};
use dotenv::dotenv;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_WINDOW_DAYS: i64 = 14;
const DEFAULT_TARGET_ROAS: f64 = 6.0;

#[derive(Debug)]
struct CliOptions {
    profile_id: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    window_days: i64,
    checked_by: String,
    release_id: Option<String>,
    target_roas: Option<f64>,
    monthly_revenue_target: Option<f64>,
    use_data_api: bool,
}

#[derive(Debug, Serialize)]
struct MaterializationManifestV1 {
    schema_version: String,
    source: String,
    profile_id: String,
    run_id: String,
    stored_at_utc: String,
    date_range: String,
    ga4_session_rollup_count: usize,
    export_id: String,
    export_payload_ref: String,
    export_artifact_dir: String,
    ui_capture_output_dir: String,
    run_store_path: String,
    export_audit_store_path: String,
    release_id: String,
    checked_by: String,
    materialized_at_utc: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let options = parse_args(env::args().skip(1).collect())?;

    let mut config = analytics_connector_config_from_env().map_err(render_err)?;
    if let Some(profile_id) = &options.profile_id {
        config.profile_id = profile_id.clone();
    }
    if options.use_data_api {
        config.ga4.read_backend = Ga4ReadBackendV1::DataApiRunReport;
    } else {
        config.ga4.read_backend = Ga4ReadBackendV1::BigqueryExport;
        config.ga4.bigquery_max_rows = config.ga4.bigquery_max_rows.max(100_000);
    }

    if config.mode != AnalyticsConnectorModeV1::ObservedReadOnly {
        return Err(format!(
            "expected ANALYTICS_CONNECTOR_MODE=observed_read_only, got {:?}",
            config.mode
        )
        .into());
    }

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
    let request = MockAnalyticsRequestV1 {
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        campaign_filter: None,
        ad_group_filter: None,
        seed: None,
        profile_id: config.profile_id.clone(),
        include_narratives: true,
        source_window_observations: Vec::new(),
        budget_envelope: BudgetEnvelopeV1 {
            provenance_ref: "budget.live_dashboard_materialization.v1".to_string(),
            ..BudgetEnvelopeV1::default()
        },
    };

    let service =
        DefaultMarketAnalysisService::with_connector_and_config(connector, config.clone())
            .map_err(render_err)?;
    let mut artifact = service
        .run_mock_analysis(request)
        .await
        .map_err(render_err)?;
    let config_fingerprint =
        analytics_connector_config_fingerprint_v1(&config).map_err(render_err)?;
    artifact.metadata.connector_attestation = ConnectorConfigAttestationV1 {
        connector_mode_effective: connector_mode_label(&config.mode).to_string(),
        connector_config_fingerprint: config_fingerprint,
        fingerprint_alg: CONNECTOR_CONFIG_FINGERPRINT_ALG_V1.to_string(),
        fingerprint_input_schema: CONNECTOR_CONFIG_FINGERPRINT_SCHEMA_V1.to_string(),
        fingerprint_created_at: None,
        runtime_build: runtime_build_label(),
        fingerprint_salt_id: None,
        fingerprint_signature: None,
        fingerprint_key_id: None,
    };

    if artifact.ga4_session_rollups.is_empty() {
        return Err("observed artifact did not produce ga4_session_rollups; use BigQuery export backend with landing/session fields".into());
    }

    let run_store = AnalyticsRunStore::default();
    let history_before = run_store
        .list_recent(Some(&config.profile_id), 64)
        .map_err(render_err)?;
    artifact.historical_analysis = build_historical_analysis(&artifact, &history_before);
    apply_confidence_calibration(
        &mut artifact.inferred_guidance,
        &artifact
            .historical_analysis
            .confidence_calibration
            .recommended_confidence_cap,
    );

    let persisted = run_store
        .append_run(PersistedAnalyticsRunV1 {
            schema_version: artifact.schema_version.clone(),
            request: artifact.request.clone(),
            metadata: artifact.metadata.clone(),
            validation: artifact.validation.clone(),
            artifact,
            stored_at_utc: String::new(),
        })
        .map_err(render_err)?;

    let history = run_store
        .list_recent(Some(&config.profile_id), 64)
        .map_err(render_err)?;
    let latest = history
        .first()
        .ok_or("expected persisted analytics history after append")?;
    if latest.metadata.run_id != persisted.metadata.run_id {
        return Err("persisted run was not returned as latest history row".into());
    }
    if latest.artifact.ga4_session_rollups.is_empty() {
        return Err("persisted run is missing ga4_session_rollups".into());
    }

    let snapshot = build_executive_dashboard_snapshot(
        &config.profile_id,
        &history,
        SnapshotBuildOptions {
            compare_window_runs: 1,
            target_roas: options.target_roas.or(Some(DEFAULT_TARGET_ROAS)),
            monthly_revenue_target: options.monthly_revenue_target,
        },
    )
    .ok_or("failed to build executive dashboard snapshot from stored history")?;

    if !snapshot.publish_export_gate.export_ready {
        return Err(format!(
            "dashboard export gate blocked: {}",
            serde_json::to_string_pretty(&snapshot.publish_export_gate)?
        )
        .into());
    }

    let policy = resolve_attestation_policy_v1(&config.profile_id).map_err(render_err)?;
    let signature_present = latest
        .metadata
        .connector_attestation
        .fingerprint_signature
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if policy.require_signed_attestations {
        let registry = load_attestation_key_registry_from_env_or_file()
            .map_err(render_err)?
            .ok_or("signed attestation policy requires a configured key registry")?;
        verify_connector_attestation_with_registry_v1(
            &latest.metadata.run_id,
            &latest.metadata.run_id,
            &latest.metadata.connector_attestation,
            &registry,
        )
        .map_err(render_err)?;
    }

    let export_id = format!("exp-{}-{}", now_millis(), latest.metadata.run_id);
    let payload_bytes = serde_json::to_vec(&snapshot)?;
    let payload_ref = persist_dashboard_export_payload(&export_id, &payload_bytes)?;
    let audit_store = DashboardExportAuditStore::default();
    let release_id = options
        .release_id
        .unwrap_or_else(|| format!("live-dashboard-{}", Utc::now().format("%Y%m%dT%H%M%SZ")));
    let audit_record = audit_store
        .append_record(DashboardExportAuditRecordV1 {
            schema_version: String::new(),
            export_id: export_id.clone(),
            profile_id: config.profile_id.clone(),
            run_id: latest.metadata.run_id.clone(),
            exported_at_utc: String::new(),
            export_format: "json".to_string(),
            target_ref: "operator_download".to_string(),
            gate_status: snapshot.publish_export_gate.gate_status.clone(),
            publish_ready: snapshot.publish_export_gate.publish_ready,
            export_ready: snapshot.publish_export_gate.export_ready,
            blocking_reasons: snapshot.publish_export_gate.blocking_reasons.clone(),
            warning_reasons: snapshot.publish_export_gate.warning_reasons.clone(),
            attestation_policy_required: policy.require_signed_attestations,
            attestation_verified: if policy.require_signed_attestations {
                true
            } else {
                signature_present
            },
            attestation_key_id: latest
                .metadata
                .connector_attestation
                .fingerprint_key_id
                .clone(),
            export_payload_checksum_alg: "sha256".to_string(),
            export_payload_checksum: checksum_sha256_hex(&payload_bytes),
            export_payload_ref: payload_ref.clone(),
            checked_by: options.checked_by.clone(),
            release_id: release_id.clone(),
        })
        .map_err(render_err)?;

    let artifact_dir = PathBuf::from("exports/live_dashboard").join(&export_id);
    let ui_capture_output_dir = artifact_dir.join("images");
    fs::create_dir_all(&ui_capture_output_dir)?;
    fs::write(artifact_dir.join("governed_snapshot.json"), &payload_bytes)?;

    let manifest = MaterializationManifestV1 {
        schema_version: "live_dashboard_materialization_manifest.v1".to_string(),
        source: "live_stored_analytics_history".to_string(),
        profile_id: config.profile_id.clone(),
        run_id: latest.metadata.run_id.clone(),
        stored_at_utc: latest.stored_at_utc.clone(),
        date_range: snapshot.date_range.clone(),
        ga4_session_rollup_count: latest.artifact.ga4_session_rollups.len(),
        export_id: export_id.clone(),
        export_payload_ref: payload_ref,
        export_artifact_dir: artifact_dir.to_string_lossy().into_owned(),
        ui_capture_output_dir: ui_capture_output_dir.to_string_lossy().into_owned(),
        run_store_path: run_store.path().to_string_lossy().into_owned(),
        export_audit_store_path: DashboardExportAuditStore::default_path()
            .to_string_lossy()
            .into_owned(),
        release_id,
        checked_by: options.checked_by,
        materialized_at_utc: audit_record.exported_at_utc,
    };
    let manifest_path = artifact_dir.join("materialization_manifest.json");
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "run_id": manifest.run_id,
            "export_id": manifest.export_id,
            "date_range": manifest.date_range,
            "ga4_session_rollup_count": manifest.ga4_session_rollup_count,
            "run_store_path": manifest.run_store_path,
            "export_payload_ref": manifest.export_payload_ref,
            "manifest_path": manifest_path,
            "source": manifest.source
        }))?
    );

    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<CliOptions, String> {
    let mut options = CliOptions {
        profile_id: None,
        start_date: None,
        end_date: None,
        window_days: DEFAULT_WINDOW_DAYS,
        checked_by: "qa_fixer".to_string(),
        release_id: None,
        target_roas: None,
        monthly_revenue_target: None,
        use_data_api: false,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" => {
                i += 1;
                options.profile_id = Some(required_value(&args, i, "--profile")?.to_string());
            }
            "--start" => {
                i += 1;
                options.start_date =
                    Some(parse_date(required_value(&args, i, "--start")?, "--start")?);
            }
            "--end" => {
                i += 1;
                options.end_date = Some(parse_date(required_value(&args, i, "--end")?, "--end")?);
            }
            "--days" => {
                i += 1;
                options.window_days = required_value(&args, i, "--days")?
                    .parse::<i64>()
                    .map_err(|_| "--days must be an integer".to_string())?;
            }
            "--checked-by" => {
                i += 1;
                options.checked_by = required_value(&args, i, "--checked-by")?.to_string();
            }
            "--release-id" => {
                i += 1;
                options.release_id = Some(required_value(&args, i, "--release-id")?.to_string());
            }
            "--target-roas" => {
                i += 1;
                options.target_roas = Some(
                    required_value(&args, i, "--target-roas")?
                        .parse::<f64>()
                        .map_err(|_| "--target-roas must be numeric".to_string())?,
                );
            }
            "--monthly-revenue-target" => {
                i += 1;
                options.monthly_revenue_target = Some(
                    required_value(&args, i, "--monthly-revenue-target")?
                        .parse::<f64>()
                        .map_err(|_| "--monthly-revenue-target must be numeric".to_string())?,
                );
            }
            "--data-api" => options.use_data_api = true,
            "--bigquery" => options.use_data_api = false,
            flag => return Err(format!("unknown flag: {flag}")),
        }
        i += 1;
    }

    if options.window_days <= 0 {
        return Err("--days must be > 0".to_string());
    }
    if options.start_date.is_some() ^ options.end_date.is_some() {
        return Err("--start and --end must be provided together".to_string());
    }
    Ok(options)
}

fn required_value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_date(value: &str, flag: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| format!("{flag} must be YYYY-MM-DD"))
}

fn resolve_date_window(options: &CliOptions) -> Result<(NaiveDate, NaiveDate), String> {
    if let (Some(start), Some(end)) = (options.start_date, options.end_date) {
        if start > end {
            return Err("--start must be <= --end".to_string());
        }
        return Ok((start, end));
    }

    let end = Utc::now().date_naive() - Duration::days(1);
    let start = end - Duration::days(options.window_days - 1);
    Ok((start, end))
}

fn persist_dashboard_export_payload(
    export_id: &str,
    payload_bytes: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    let dir = env::var("ANALYTICS_EXPORT_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/analytics_runs/exports"));
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{export_id}.json"));
    fs::write(&path, payload_bytes)?;
    Ok(path.to_string_lossy().into_owned())
}

fn checksum_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn apply_confidence_calibration(
    guidance: &mut [app_core::subsystems::marketing_data_analysis::GuidanceItem],
    cap: &str,
) {
    for item in guidance.iter_mut() {
        let normalized = item.confidence_label.to_ascii_lowercase();
        let adjusted = match cap {
            "low" => "low".to_string(),
            "medium" => {
                if normalized == "high" {
                    "medium".to_string()
                } else {
                    normalized
                }
            }
            _ => normalized,
        };
        item.confidence_label = adjusted;
        item.calibration_band = Some(cap.to_string());
    }
}

fn connector_mode_label(mode: &AnalyticsConnectorModeV1) -> &'static str {
    match mode {
        AnalyticsConnectorModeV1::Simulated => "simulated",
        AnalyticsConnectorModeV1::ObservedReadOnly => "observed_read_only",
    }
}

fn runtime_build_label() -> Option<String> {
    env::var("ANALYTICS_RUNTIME_BUILD")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("GIT_SHA")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .or_else(|| Some(env!("CARGO_PKG_VERSION").to_string()))
}

fn now_millis() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i128)
        .unwrap_or(0)
}

fn render_err(err: app_core::subsystems::marketing_data_analysis::AnalyticsError) -> String {
    let details = err.context.unwrap_or_else(|| json!({}));
    serde_json::to_string_pretty(&json!({
        "code": err.code,
        "message": err.message,
        "field_paths": err.field_paths,
        "context": details
    }))
    .unwrap_or_else(|_| "analytics error".to_string())
}
