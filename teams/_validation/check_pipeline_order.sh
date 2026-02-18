#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0001; change_request_id=RQ-029; change_request_id=RQ-MGR-002

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
HANDOFF_LOG="$ROOT/data/team_ops/handoff_log.csv"
TEAM_REGISTRY="$ROOT/data/team_ops/team_registry.csv"
DECISION_LOG="$ROOT/data/team_ops/decision_log.csv"
DOCTRINE_REF="teams/shared/OPERATING_DOCTRINE.md"
PIPELINE_MODE="${1:-${TEAMS_PIPELINE_MODE:-lite}}"

if [[ "$PIPELINE_MODE" != "full" && "$PIPELINE_MODE" != "lite" ]]; then
  echo "FAIL[RQ-029] invalid pipeline mode '$PIPELINE_MODE' (expected full|lite)" >&2
  exit 29
fi

if [[ ! -f "$HANDOFF_LOG" ]]; then
  echo "FAIL[RQ-029] missing handoff log: $HANDOFF_LOG" >&2
  exit 29
fi

python3 - "$HANDOFF_LOG" "$TEAM_REGISTRY" "$DECISION_LOG" "$DOCTRINE_REF" "$PIPELINE_MODE" <<'PY'
import csv
import sys

handoff_log = sys.argv[1]
team_registry = sys.argv[2]
decision_log = sys.argv[3]
doctrine_ref = sys.argv[4]
pipeline_mode = sys.argv[5]

rows = []
with open(handoff_log, "r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        if not row.get("run_id") or not row.get("from_team") or not row.get("to_team"):
            continue
        rows.append(row)

if not rows:
    print(f"FAIL[RQ-029] no handoff rows found; doctrine={doctrine_ref}", file=sys.stderr)
    sys.exit(29)

phase_map = {}
with open(team_registry, "r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        team = (row.get("team_id") or "").strip()
        phase = (row.get("phase_order") or "").strip()
        if not team or not phase:
            continue
        phase_map[team] = int(phase)

if not phase_map:
    print(f"FAIL[RQ-MGR-002] empty team phase registry: {team_registry}", file=sys.stderr)
    sys.exit(29)

phase_to_team = {v: k for k, v in phase_map.items()}

if pipeline_mode == "full":
    required_teams = {"blue", "red", "green", "black", "white", "grey", "qa_fixer"}
else:
    required_teams = {"blue", "red", "white", "qa_fixer"}

required_phases = {phase_map[t] for t in required_teams if t in phase_map}

def transition_valid(from_team: str, to_team: str):
    from_phase = phase_map[from_team]
    to_phase = phase_map[to_team]
    if to_phase <= from_phase:
        return False, from_phase, to_phase, "non-forward transition"
    if pipeline_mode == "full":
        return (to_phase - from_phase == 1), from_phase, to_phase, "full mode requires phase delta 1"

    # lite mode: allow skipping only optional teams (green/black/grey)
    skipped = range(from_phase + 1, to_phase)
    skipped_required = [p for p in skipped if p in required_phases]
    if skipped_required:
        return (
            False,
            from_phase,
            to_phase,
            "lite mode cannot skip required teams: "
            + ",".join(phase_to_team.get(p, str(p)) for p in skipped_required),
        )
    return True, from_phase, to_phase, "ok"

by_run = {}
for row in rows:
    by_run.setdefault(row["run_id"], []).append(row)

decision_rows = []
with open(decision_log, "r", encoding="utf-8", newline="") as f:
    for row in csv.DictReader(f):
        decision_rows.append(row)

for run_id, run_rows in by_run.items():
    run_rows.sort(key=lambda r: r.get("timestamp_utc", ""))
    latest_phase = None
    for row in run_rows:
        from_team = row["from_team"].strip()
        to_team = row["to_team"].strip()

        if from_team not in phase_map or to_team not in phase_map:
            print(
                f"FAIL[RQ-MGR-002] run_id={run_id} unknown team in handoff: "
                f"{from_team}->{to_team}; doctrine={doctrine_ref}",
                file=sys.stderr,
            )
            sys.exit(29)

        is_valid, from_phase, to_phase, reason = transition_valid(from_team, to_team)
        observed_pair = f"{from_team}->{to_team}"

        # Allow append-only duplicates/supersedes for the same phase pair.
        if latest_phase is not None and from_phase == latest_phase - 1 and to_phase == latest_phase:
            continue

        expected_next_from = (latest_phase if latest_phase is not None else from_phase)
        expected_pair = None
        if latest_phase is None:
            expected_pair = f"{from_team}->{to_team}"
        else:
            # Expected progression is exactly one phase forward from current latest phase.
            expected_from_team = next((k for k, v in phase_map.items() if v == latest_phase), None)
            expected_to_team = next((k for k, v in phase_map.items() if v == latest_phase + 1), None)
            expected_pair = f"{expected_from_team}->{expected_to_team}"

        if not is_valid:
            has_block_decision = any(
                (d.get("run_id") or "").strip() == run_id
                and "hard_fail_pipeline_order_violation" in (d.get("decision") or "")
                for d in decision_rows
            )
            block_diag = "logged" if has_block_decision else "missing"
            # If legacy out-of-order rows are already explicitly blocked and logged,
            # treat them as contained and continue so future compliant rows can pass.
            if has_block_decision:
                continue
            observed_pair = f"{from_team}->{to_team}"
            print(
                f"FAIL[RQ-MGR-002] run_id={run_id} out-of-order handoff; "
                f"mode={pipeline_mode}; expected_rule={reason}; observed_phase_delta={to_phase - from_phase}; "
                f"expected_pair={expected_pair}; observed={observed_pair}; "
                f"block_decision_log={block_diag}; doctrine={doctrine_ref}",
                file=sys.stderr,
            )
            sys.exit(29)

        latest_phase = to_phase

print(f"PASS[RQ-029|RQ-MGR-002] pipeline order valid via team_registry phases; mode={pipeline_mode}; doctrine={doctrine_ref}")
PY
