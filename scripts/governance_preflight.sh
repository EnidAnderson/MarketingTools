#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-013

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

required=(
  "planning/RELEASE_GATES_POLICY.md"
  "planning/SECURITY_THREAT_MODEL.md"
  "planning/SECURITY_CONTROL_BASELINE.md"
  "planning/BUDGET_GUARDRAILS_STANDARD.md"
  "planning/AGENT_ROLE_CONTRACTS.md"
  "planning/RISK_REGISTER.md"
)

fail=0
for rel in "${required[@]}"; do
  abs="$ROOT/$rel"
  if [[ ! -f "$abs" ]]; then
    echo "FAIL[RQ-013] missing_control_artifact path=$rel"
    fail=1
    continue
  fi
  if [[ ! -s "$abs" ]]; then
    echo "FAIL[RQ-013] empty_control_artifact path=$rel"
    fail=1
    continue
  fi
done

if [[ "$fail" -ne 0 ]]; then
  exit 13
fi

echo "PASS[RQ-013] governance_preflight status=ok required_artifacts=${#required[@]}"

