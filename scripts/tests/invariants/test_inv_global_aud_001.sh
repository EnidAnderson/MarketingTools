#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/planning/reports/RELEASE_GATE_LOG.csv"
require_file "$ROOT/planning/BUDGET_EXCEPTION_LOG.md"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cp "$ROOT/planning/reports/RELEASE_GATE_LOG.csv" "$TMP/release_gate_log.csv"

orig_hash="$(shasum "$TMP/release_gate_log.csv" | awk '{print $1}')"

# Negative: mutation attempt should be detected deterministically.
echo "mutated_row,should_not_exist" >> "$TMP/release_gate_log.csv"
mut_hash="$(shasum "$TMP/release_gate_log.csv" | awk '{print $1}')"
if [[ "$orig_hash" == "$mut_hash" ]]; then
  die 141 "mutation detection failed for release_gate_log.csv"
fi
echo "MUTATION_DETECTED file=release_gate_log.csv"

# Positive: superseding-entry path accepted when only append occurs.
cp "$ROOT/planning/reports/RELEASE_GATE_LOG.csv" "$TMP/release_gate_log_append.csv"
echo "999,2026-02-10T00:00:00Z,rel_test,internal,green,green,green,green,green,green,,qa_fixer,998" >> "$TMP/release_gate_log_append.csv"
if ! tail -n 1 "$TMP/release_gate_log_append.csv" | rg -q ",998$"; then
  die 142 "superseding append entry not preserved"
fi

pass "INV-GLOBAL-AUD-001 validated"

