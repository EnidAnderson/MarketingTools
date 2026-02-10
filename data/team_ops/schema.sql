-- Optional SQLite schema for team-ops logs.

CREATE TABLE IF NOT EXISTS team_registry (
  team_id TEXT PRIMARY KEY,
  team_name TEXT NOT NULL,
  phase_order INTEGER NOT NULL,
  can_edit_code INTEGER NOT NULL,
  can_edit_config INTEGER NOT NULL,
  can_edit_schema INTEGER NOT NULL,
  primary_authority TEXT NOT NULL,
  output_file TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS run_registry (
  run_id TEXT NOT NULL,
  created_utc TEXT NOT NULL,
  status TEXT NOT NULL,
  current_phase TEXT NOT NULL,
  owner TEXT NOT NULL,
  summary TEXT,
  supersedes_run_id TEXT
);

CREATE TABLE IF NOT EXISTS handoff_log (
  entry_id INTEGER,
  run_id TEXT NOT NULL,
  from_team TEXT NOT NULL,
  to_team TEXT NOT NULL,
  timestamp_utc TEXT NOT NULL,
  input_refs TEXT,
  output_ref TEXT NOT NULL,
  change_request_ids TEXT,
  blocking_flags TEXT,
  notes TEXT,
  supersedes_entry_id INTEGER
);

CREATE TABLE IF NOT EXISTS change_request_queue (
  request_id TEXT,
  run_id TEXT NOT NULL,
  source_team TEXT NOT NULL,
  priority TEXT NOT NULL,
  status TEXT NOT NULL,
  statement TEXT NOT NULL,
  acceptance_criteria_refs TEXT,
  constraint_refs TEXT,
  evidence_refs TEXT,
  assignee TEXT,
  supersedes_request_id TEXT
);

CREATE TABLE IF NOT EXISTS decision_log (
  decision_id TEXT,
  run_id TEXT NOT NULL,
  phase TEXT NOT NULL,
  decision_owner TEXT NOT NULL,
  decision TEXT NOT NULL,
  timestamp_utc TEXT NOT NULL,
  reasoning_ref TEXT,
  impact_level TEXT,
  supersedes_decision_id TEXT
);
