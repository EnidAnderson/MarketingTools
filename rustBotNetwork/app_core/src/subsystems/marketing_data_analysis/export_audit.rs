use super::contracts::{AnalyticsError, DashboardExportAuditRecordV1};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const DEFAULT_EXPORT_AUDIT_STORE_PATH: &str = "data/analytics_runs/dashboard_export_audit_v1.jsonl";
const EXPORT_AUDIT_SCHEMA_VERSION_V1: &str = "dashboard_export_audit.v1";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::export_audit`
/// purpose: Append-only persistence for governed dashboard export audit records.
/// invariants:
///   - Each line is one valid `DashboardExportAuditRecordV1` JSON object.
///   - Writes are append-only and timestamps are generated at write time.
#[derive(Debug, Clone)]
pub struct DashboardExportAuditStore {
    path: PathBuf,
}

impl DashboardExportAuditStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        std::env::var("ANALYTICS_EXPORT_AUDIT_STORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_EXPORT_AUDIT_STORE_PATH))
    }

    pub fn append_record(
        &self,
        mut record: DashboardExportAuditRecordV1,
    ) -> Result<DashboardExportAuditRecordV1, AnalyticsError> {
        ensure_parent_dir(&self.path)?;
        record.schema_version = EXPORT_AUDIT_SCHEMA_VERSION_V1.to_string();
        record.exported_at_utc = Utc::now().to_rfc3339();

        let line = serde_json::to_string(&record).map_err(|err| {
            AnalyticsError::internal(
                "export_audit_serialize_failed",
                format!("failed to serialize dashboard export audit record: {err}"),
            )
        })?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|err| {
                AnalyticsError::internal(
                    "export_audit_open_failed",
                    format!("failed to open dashboard export audit store: {err}"),
                )
            })?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|err| {
                AnalyticsError::internal(
                    "export_audit_write_failed",
                    format!("failed to append dashboard export audit record: {err}"),
                )
            })?;

        Ok(record)
    }

    pub fn list_recent(
        &self,
        profile_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DashboardExportAuditRecordV1>, AnalyticsError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.path).map_err(|err| {
            AnalyticsError::internal(
                "export_audit_open_failed",
                format!("failed to read dashboard export audit store: {err}"),
            )
        })?;
        let reader = BufReader::new(file);
        let mut rows = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|err| {
                AnalyticsError::internal(
                    "export_audit_read_failed",
                    format!("failed to read dashboard export audit line: {err}"),
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let parsed: DashboardExportAuditRecordV1 =
                serde_json::from_str(&line).map_err(|err| {
                    AnalyticsError::internal(
                        "export_audit_parse_failed",
                        format!("failed to parse dashboard export audit record: {err}"),
                    )
                })?;
            if let Some(profile_id) = profile_id {
                if parsed.profile_id != profile_id {
                    continue;
                }
            }
            rows.push(parsed);
        }

        rows.sort_by(|a, b| b.exported_at_utc.cmp(&a.exported_at_utc));
        if rows.len() > limit {
            rows.truncate(limit);
        }
        Ok(rows)
    }
}

impl Default for DashboardExportAuditStore {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), AnalyticsError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|err| {
        AnalyticsError::internal(
            "export_audit_parent_dir_failed",
            format!("failed to create export audit directory: {err}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_record(export_id: &str, profile_id: &str) -> DashboardExportAuditRecordV1 {
        DashboardExportAuditRecordV1 {
            schema_version: String::new(),
            export_id: export_id.to_string(),
            profile_id: profile_id.to_string(),
            run_id: "run-1".to_string(),
            exported_at_utc: String::new(),
            export_format: "json".to_string(),
            target_ref: "operator_download".to_string(),
            gate_status: "ready".to_string(),
            publish_ready: true,
            export_ready: true,
            blocking_reasons: Vec::new(),
            warning_reasons: Vec::new(),
            attestation_policy_required: true,
            attestation_verified: true,
            attestation_key_id: Some("key-2026-03".to_string()),
            export_payload_checksum_alg: "sha256".to_string(),
            export_payload_checksum: "abc123".to_string(),
            checked_by: "qa_fixer".to_string(),
            release_id: "rel-1".to_string(),
        }
    }

    #[test]
    fn append_and_list_export_audit_records() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("exports.jsonl");
        let store = DashboardExportAuditStore::new(path);
        store
            .append_record(sample_record("exp-1", "p1"))
            .expect("append");
        store
            .append_record(sample_record("exp-2", "p1"))
            .expect("append");
        store
            .append_record(sample_record("exp-3", "p2"))
            .expect("append");

        let p1 = store.list_recent(Some("p1"), 10).expect("list");
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].profile_id, "p1");
        assert_eq!(p1[0].schema_version, "dashboard_export_audit.v1");
        assert!(!p1[0].exported_at_utc.is_empty());
    }
}
