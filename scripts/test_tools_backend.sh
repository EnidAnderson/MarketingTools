#!/usr/bin/env bash
set -euo pipefail

# Runs backend-only tool execution tests that use the same registry/dispatch
# path as runtime jobs, without launching the Tauri GUI shell.
echo "[tool-harness] running stable tool registry and E2E backend tests"
cargo test -p app_core test_tool_registry_stable_tools_have_runnable_defaults
cargo test -p app_core test_tool_registry_maturity_filtering
cargo test -p app_core test_tool_e2e_echo_tool_produces_actionable_artifact
cargo test -p app_core test_tool_e2e_seo_analyzer_produces_actionable_artifact
cargo test -p app_core test_stable_defaults_execute_non_empty_artifact
cargo test -p app_core test_tool_audit_report_covers_all_tool_modules
cargo test -p app_core test_tool_audit_stable_recommendations_meet_thresholds
cargo test -p app_core test_execute_unknown_tool_returns_typed_envelope
cargo test -p app_core test_runtime_panic_converts_to_internal_error_with_trace_id
echo "[tool-harness] complete"
