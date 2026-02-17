// provenance: decision_id=DEC-0014; change_request_id=CR-QA_FIXER-0031
use super::contracts::{AnalyticsError, PersistedAnalyticsRunV1};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const DEFAULT_STORE_PATH: &str = "data/analytics_runs/mock_analytics_runs_v1.jsonl";

/// # NDOC
/// component: `subsystems::marketing_data_analysis::persistence`
/// purpose: File-backed persistence for analytics artifact runs.
/// invariants:
///   - Every line in storage file is one valid `PersistedAnalyticsRunV1` JSON object.
///   - New writes are append-only.
#[derive(Debug, Clone)]
pub struct AnalyticsRunStore {
    path: PathBuf,
}

impl AnalyticsRunStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        std::env::var("ANALYTICS_RUN_STORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_STORE_PATH))
    }

    pub fn append_run(
        &self,
        mut run: PersistedAnalyticsRunV1,
    ) -> Result<PersistedAnalyticsRunV1, AnalyticsError> {
        ensure_parent_dir(&self.path)?;
        run.stored_at_utc = Utc::now().to_rfc3339();

        let line = serde_json::to_string(&run).map_err(|err| {
            AnalyticsError::internal(
                "persistence_serialize_failed",
                format!("failed to serialize persisted run: {err}"),
            )
        })?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|err| {
                AnalyticsError::internal(
                    "persistence_open_failed",
                    format!("failed to open analytics run store: {err}"),
                )
            })?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|err| {
                AnalyticsError::internal(
                    "persistence_write_failed",
                    format!("failed to append analytics run: {err}"),
                )
            })?;
        Ok(run)
    }

    pub fn list_recent(
        &self,
        profile_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PersistedAnalyticsRunV1>, AnalyticsError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.path).map_err(|err| {
            AnalyticsError::internal(
                "persistence_open_failed",
                format!("failed to read analytics run store: {err}"),
            )
        })?;
        let reader = BufReader::new(file);
        let mut runs = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|err| {
                AnalyticsError::internal(
                    "persistence_read_failed",
                    format!("failed to read analytics run line: {err}"),
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let parsed: PersistedAnalyticsRunV1 = serde_json::from_str(&line).map_err(|err| {
                AnalyticsError::internal(
                    "persistence_parse_failed",
                    format!("failed to parse persisted analytics run: {err}"),
                )
            })?;
            if let Some(profile_id) = profile_id {
                if parsed.request.profile_id != profile_id {
                    continue;
                }
            }
            runs.push(parsed);
        }

        runs.sort_by(|a, b| b.stored_at_utc.cmp(&a.stored_at_utc));
        if runs.len() > limit {
            runs.truncate(limit);
        }
        Ok(runs)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Default for AnalyticsRunStore {
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
            "persistence_parent_dir_failed",
            format!("failed to create run store directory: {err}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::marketing_data_analysis::contracts::{
        AnalyticsRunMetadataV1, AnalyticsValidationReportV1, MockAnalyticsArtifactV1,
        MockAnalyticsRequestV1, MOCK_ANALYTICS_SCHEMA_VERSION_V1,
    };
    use tempfile::tempdir;

    fn sample_run(run_id: &str, profile_id: &str) -> PersistedAnalyticsRunV1 {
        let request = MockAnalyticsRequestV1 {
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            campaign_filter: None,
            ad_group_filter: None,
            seed: Some(7),
            profile_id: profile_id.to_string(),
            include_narratives: true,
            budget_envelope:
                crate::subsystems::marketing_data_analysis::contracts::BudgetEnvelopeV1::default(),
        };
        let artifact = MockAnalyticsArtifactV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request: request.clone(),
            metadata: AnalyticsRunMetadataV1 {
                run_id: run_id.to_string(),
                connector_id: "mock".to_string(),
                profile_id: profile_id.to_string(),
                seed: 7,
                schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
                date_span_days: 2,
                requested_at_utc: None,
            },
            report: Default::default(),
            observed_evidence: Vec::new(),
            inferred_guidance: Vec::new(),
            uncertainty_notes: vec!["simulated".to_string()],
            provenance: Vec::new(),
            ingest_cleaning_notes: Vec::new(),
            validation: AnalyticsValidationReportV1 {
                is_valid: true,
                checks: Vec::new(),
            },
            quality_controls: Default::default(),
            data_quality: Default::default(),
            budget: Default::default(),
            historical_analysis: Default::default(),
            operator_summary: Default::default(),
            persistence: None,
        };
        PersistedAnalyticsRunV1 {
            schema_version: MOCK_ANALYTICS_SCHEMA_VERSION_V1.to_string(),
            request,
            metadata: artifact.metadata.clone(),
            validation: artifact.validation.clone(),
            artifact,
            stored_at_utc: String::new(),
        }
    }

    #[test]
    fn append_and_list_recent_runs() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("runs.jsonl");
        let store = AnalyticsRunStore::new(path);
        store.append_run(sample_run("r1", "p1")).expect("append");
        store.append_run(sample_run("r2", "p1")).expect("append");
        store.append_run(sample_run("r3", "p2")).expect("append");

        let p1_runs = store.list_recent(Some("p1"), 10).expect("list");
        assert_eq!(p1_runs.len(), 2);
        assert_eq!(p1_runs[0].request.profile_id, "p1");
    }
}
