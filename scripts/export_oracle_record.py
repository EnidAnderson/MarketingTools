#!/usr/bin/env python3
"""Build a comprehensive, machine-readable Oracle handoff record for team activity."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Tuple


INCLUDE_GLOBS = [
    "teams/**/*.md",
    "pipeline/**/*.md",
    "data/team_ops/**/*.csv",
    "data/team_ops/**/*.json",
    "planning/reports/TEAM_LEAD*.md",
    "planning/reports/HARDENING_CONTROL_MATRIX_*.md",
    "planning/invariants/**/*.md",
    "planning/invariants/**/*.csv",
    "planning/RAPID_REVIEW_CELL/**/*.md",
    "planning/RAPID_REVIEW_CELL/**/*.csv",
    "AGENTS.md",
]

CSV_EVENT_FILES = [
    "data/team_ops/run_registry.csv",
    "data/team_ops/change_request_queue.csv",
    "data/team_ops/handoff_log.csv",
    "data/team_ops/decision_log.csv",
    "data/team_ops/team_registry.csv",
    "data/team_ops/qa_edit_authority.csv",
    "data/team_ops/archive/archive_index.csv",
]

JSON_LIST_FIELDS = {
    "input_refs",
    "change_request_ids",
    "blocking_flags",
    "acceptance_criteria_refs",
    "constraint_refs",
    "evidence_refs",
}

ACTIVE_REQUEST_STATUSES = {"open", "in_progress", "blocked", "pending", "ready", "todo"}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def read_csv(path: Path) -> Tuple[List[str], List[dict]]:
    if not path.exists():
        return [], []
    with path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        return reader.fieldnames or [], list(reader)


def gather_files(root: Path) -> List[Path]:
    files = set()
    for pattern in INCLUDE_GLOBS:
        for p in root.glob(pattern):
            if p.is_file():
                files.add(p.resolve())
    return sorted(files)


def parse_json_list(value: str) -> Tuple[List[str] | None, str | None]:
    raw = (value or "").strip()
    if raw == "":
        return [], None
    if not raw.startswith("["):
        return None, f"expected JSON array string, got: {raw[:80]}"
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as e:
        return None, f"invalid JSON array syntax: {e.msg}"
    if not isinstance(parsed, list):
        return None, "value is valid JSON but not an array"
    return parsed, None


def run_cmd(root: Path, args: List[str]) -> Tuple[bool, str]:
    try:
        p = subprocess.run(
            args,
            cwd=root,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        if p.returncode != 0:
            return False, (p.stderr or p.stdout or "").strip()
        return True, p.stdout
    except Exception as e:
        return False, str(e)


def parse_qa_fix_log(md_text: str) -> List[dict]:
    entries: List[dict] = []
    blocks = [b.strip() for b in md_text.split("\n---\n") if b.strip()]
    for b in blocks:
        if "- run_id:" not in b:
            continue
        current: dict = {
            "run_id": "",
            "timestamp_utc": "",
            "request_ids_implemented": [],
            "decision_and_change_refs": [],
            "files_changed": [],
            "rationale": "",
            "verification_evidence": [],
            "residual_risks": [],
        }
        mode = None
        for raw in b.splitlines():
            line = raw.rstrip()
            if line.startswith("- run_id:"):
                current["run_id"] = line.split(":", 1)[1].strip()
                mode = None
            elif line.startswith("- timestamp_utc:"):
                current["timestamp_utc"] = line.split(":", 1)[1].strip()
                mode = None
            elif line.startswith("- request_ids_implemented:"):
                mode = "request_ids_implemented"
            elif line.startswith("- decision_and_change_refs:"):
                mode = "decision_and_change_refs"
            elif line.startswith("- files_changed:"):
                mode = "files_changed"
            elif line.startswith("- rationale:"):
                mode = "rationale"
            elif line.startswith("- verification_evidence:"):
                mode = "verification_evidence"
            elif line.startswith("- residual_risks:"):
                mode = "residual_risks"
            elif mode == "rationale":
                if line.strip().startswith("|"):
                    continue
                if line.strip().startswith("- "):
                    mode = None
                elif line.strip():
                    current["rationale"] += (line.strip() + " ")
            elif mode in {"request_ids_implemented", "decision_and_change_refs", "files_changed", "verification_evidence", "residual_risks"}:
                s = line.strip()
                if s.startswith("- "):
                    current[mode].append(s[2:].strip())
        current["rationale"] = current["rationale"].strip()
        entries.append(current)
    return entries


def collect_git_change_evidence(root: Path) -> dict:
    evidence = {
        "git_available": False,
        "branch": "",
        "commit_history": [],
        "worktree_status_porcelain": [],
        "worktree_changed_files": [],
        "worktree_untracked_files": [],
    }

    ok, out = run_cmd(root, ["git", "rev-parse", "--is-inside-work-tree"])
    if not ok or out.strip() != "true":
        return evidence

    evidence["git_available"] = True

    ok, out = run_cmd(root, ["git", "branch", "--show-current"])
    if ok:
        evidence["branch"] = out.strip()

    ok, out = run_cmd(
        root,
        ["git", "log", "--date=iso-strict", "--name-status", "--pretty=format:@@%H|%ad|%an|%s"],
    )
    if ok:
        commits = []
        current = None
        for line in out.splitlines():
            if line.startswith("@@"):
                if current:
                    commits.append(current)
                parts = line[2:].split("|", 3)
                current = {
                    "commit": parts[0] if len(parts) > 0 else "",
                    "date_utc": parts[1] if len(parts) > 1 else "",
                    "author": parts[2] if len(parts) > 2 else "",
                    "subject": parts[3] if len(parts) > 3 else "",
                    "files": [],
                }
            elif current and line.strip():
                # examples: M\tpath, A\tpath, R100\told\tnew
                parts = line.split("\t")
                if len(parts) >= 2:
                    status = parts[0].strip()
                    if status.startswith("R") and len(parts) >= 3:
                        current["files"].append(
                            {"status": status, "path_old": parts[1], "path_new": parts[2]}
                        )
                    else:
                        current["files"].append({"status": status, "path": parts[1]})
        if current:
            commits.append(current)
        evidence["commit_history"] = commits

    ok, out = run_cmd(root, ["git", "status", "--porcelain", "-uall"])
    if ok:
        rows = [ln.rstrip() for ln in out.splitlines() if ln.strip()]
        evidence["worktree_status_porcelain"] = rows
        changed = []
        untracked = []
        for r in rows:
            if len(r) < 4:
                continue
            status = r[:2]
            path = r[3:].strip()
            if status == "??":
                untracked.append(path)
            else:
                changed.append(path)
        evidence["worktree_changed_files"] = changed
        evidence["worktree_untracked_files"] = untracked

    return evidence


def normalize_rows(file_key: str, rows: List[dict], issues: List[dict]) -> List[dict]:
    out: List[dict] = []
    for i, row in enumerate(rows, start=1):
        nr = dict(row)
        nr["_row_number"] = i
        for k, v in list(nr.items()):
            if k in JSON_LIST_FIELDS:
                parsed, err = parse_json_list(str(v or ""))
                if err is None:
                    nr[k] = parsed
                else:
                    nr[f"{k}_raw"] = v
                    issues.append(
                        {
                            "severity": "error",
                            "code": "JSON_FIELD_INVALID",
                            "file": file_key,
                            "row_number": i,
                            "field": k,
                            "message": err,
                        }
                    )
        out.append(nr)
    return out


def make_uid(file_key: str, row: dict) -> str:
    r = row.get("_row_number", "?")
    if file_key.endswith("run_registry.csv"):
        return f"run|{row.get('run_id','')}|{row.get('created_utc','')}|{row.get('status','')}|row:{r}"
    if file_key.endswith("change_request_queue.csv"):
        return f"cr|{row.get('run_id','')}|{row.get('request_id','')}|row:{r}"
    if file_key.endswith("handoff_log.csv"):
        return (
            f"handoff|{row.get('run_id','')}|{row.get('timestamp_utc','')}|"
            f"{row.get('from_team','')}->{row.get('to_team','')}|entry:{row.get('entry_id','')}|row:{r}"
        )
    if file_key.endswith("decision_log.csv"):
        return (
            f"decision|{row.get('run_id','')}|{row.get('timestamp_utc','')}|"
            f"{row.get('decision_id','')}|row:{r}"
        )
    return f"event|{file_key}|row:{r}"


def validate_duplicates(file_key: str, rows: List[dict], id_field: str, issues: List[dict]) -> None:
    seen: Dict[str, int] = {}
    for row in rows:
        value = str(row.get(id_field, "") or "").strip()
        if not value:
            continue
        if value in seen:
            issues.append(
                {
                    "severity": "error",
                    "code": "DUPLICATE_ID",
                    "file": file_key,
                    "field": id_field,
                    "row_number": row.get("_row_number"),
                    "message": f"duplicate {id_field}='{value}' (first seen row {seen[value]})",
                }
            )
        else:
            seen[value] = int(row.get("_row_number", 0))


def validate_supersedes(
    file_key: str,
    rows: List[dict],
    current_field: str,
    supersedes_field: str,
    issues: List[dict],
) -> None:
    seen_prior: set[str] = set()
    for row in rows:
        current = str(row.get(current_field, "") or "").strip()
        supers = str(row.get(supersedes_field, "") or "").strip()
        rn = row.get("_row_number")
        if supers:
            if supers == current:
                issues.append(
                    {
                        "severity": "error",
                        "code": "SUPERSEDES_SELF",
                        "file": file_key,
                        "field": supersedes_field,
                        "row_number": rn,
                        "message": f"{supersedes_field} cannot equal {current_field} ('{supers}')",
                    }
                )
            elif supers not in seen_prior:
                issues.append(
                    {
                        "severity": "error",
                        "code": "SUPERSEDES_TARGET_NOT_PRIOR",
                        "file": file_key,
                        "field": supersedes_field,
                        "row_number": rn,
                        "message": f"{supersedes_field}='{supers}' does not reference a prior {current_field}",
                    }
                )
        if current:
            seen_prior.add(current)


def materialize_latest(rows: List[dict], key_field: str) -> Dict[str, dict]:
    latest: Dict[str, dict] = {}
    for row in rows:
        key = str(row.get(key_field, "") or "").strip()
        if not key:
            continue
        latest[key] = row
    return latest


def build_summary(current: dict, events: dict, issues: List[dict]) -> dict:
    current_runs = current.get("runs", {})
    current_requests = current.get("change_requests", {})
    current_handoffs = current.get("handoffs", {})

    run_status_counts = Counter()
    for row in current_runs.values():
        run_status_counts[str(row.get("status", "") or "unknown")] += 1

    stage_completion = {}
    for run_id in current_runs.keys():
        completion = {
            "blue": False,
            "red": False,
            "green": False,
            "black": False,
            "white": False,
            "grey": False,
            "qa_fixer": False,
        }
        for row in events.get("data/team_ops/handoff_log.csv", []):
            if str(row.get("run_id", "") or "").strip() != run_id:
                continue
            src = str(row.get("from_team", "") or "").strip()
            if src in completion:
                completion[src] = True
        stage_completion[run_id] = completion

    open_p1_by_assignee = defaultdict(list)
    for req_id, row in current_requests.items():
        status = str(row.get("status", "") or "").strip().lower()
        priority = str(row.get("priority", "") or "").strip().upper()
        if status in ACTIVE_REQUEST_STATUSES and priority == "P1":
            assignee = str(row.get("assignee", "") or "unassigned").strip() or "unassigned"
            open_p1_by_assignee[assignee].append(req_id)

    active_blocking_flags = []
    for row in current_handoffs.values():
        flags = row.get("blocking_flags", [])
        if isinstance(flags, list) and flags:
            active_blocking_flags.append(
                {
                    "run_id": row.get("run_id"),
                    "entry_id": row.get("entry_id"),
                    "from_team": row.get("from_team"),
                    "to_team": row.get("to_team"),
                    "flags": flags,
                }
            )

    severity_counts = Counter(i.get("severity", "unknown") for i in issues)

    return {
        "run_summary": {
            "total_runs": len(current_runs),
            "status_counts": dict(run_status_counts),
            "active_runs": [rid for rid, r in current_runs.items() if str(r.get("status", "")).lower() in {"active", "in_progress", "blocked", "blocked_missing_stages", "initialized", "ready", "awaiting_input"}],
        },
        "stage_completion_map": stage_completion,
        "open_p1_by_assignee": dict(open_p1_by_assignee),
        "active_blocking_flags": active_blocking_flags,
        "integrity_overview": {
            "issue_count": len(issues),
            "severity_counts": dict(severity_counts),
        },
    }


def build_markdown_summary(summary: dict, generated_utc: str, execution_changes: dict) -> str:
    lines = [
        "# Oracle Record Summary",
        "",
        f"Generated UTC: {generated_utc}",
        "",
        "## Run Summary",
        f"- Total runs: {summary['run_summary']['total_runs']}",
        f"- Status counts: {json.dumps(summary['run_summary']['status_counts'], sort_keys=True)}",
        f"- Active runs: {', '.join(summary['run_summary']['active_runs']) if summary['run_summary']['active_runs'] else 'none'}",
        "",
        "## Stage Completion",
    ]
    for run_id, stages in summary["stage_completion_map"].items():
        lines.append(f"- {run_id}: {json.dumps(stages, sort_keys=True)}")

    lines.extend(["", "## Open P1 By Assignee"])
    if summary["open_p1_by_assignee"]:
        for assignee, ids in summary["open_p1_by_assignee"].items():
            lines.append(f"- {assignee}: {', '.join(sorted(ids))}")
    else:
        lines.append("- none")

    lines.extend(["", "## Active Blocking Flags"])
    if summary["active_blocking_flags"]:
        for item in summary["active_blocking_flags"]:
            lines.append(
                f"- run={item['run_id']} entry={item['entry_id']} {item['from_team']}->{item['to_team']} flags={json.dumps(item['flags'])}"
            )
    else:
        lines.append("- none")

    lines.extend([
        "",
        "## Integrity Overview",
        f"- issue_count: {summary['integrity_overview']['issue_count']}",
        f"- severity_counts: {json.dumps(summary['integrity_overview']['severity_counts'], sort_keys=True)}",
        "",
        "## Implementation Change Evidence",
        f"- qa_fixer_entries: {len(execution_changes.get('qa_fixer_log_entries', []))}",
        f"- qa_fixer_files_changed_unique: {len(execution_changes.get('derived', {}).get('qa_fixer_files_changed_unique', []))}",
        f"- git_available: {execution_changes.get('git', {}).get('git_available', False)}",
        f"- git_commit_count: {len(execution_changes.get('git', {}).get('commit_history', []))}",
        f"- worktree_changed_files: {len(execution_changes.get('git', {}).get('worktree_changed_files', []))}",
        f"- worktree_untracked_files: {len(execution_changes.get('git', {}).get('worktree_untracked_files', []))}",
        "",
    ])
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Export Oracle-ready team history record.")
    p.add_argument("--strict", action="store_true", help="Fail export when integrity issues are found.")
    return p.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[1]
    now = datetime.now(timezone.utc)
    generated_utc = now.replace(microsecond=0).isoformat()
    stamp = now.strftime("%Y%m%dT%H%M%SZ")

    export_dir = root / "exports" / f"oracle_record_{stamp}"
    export_dir.mkdir(parents=True, exist_ok=True)

    files = gather_files(root)

    inventory = []
    file_contents: Dict[str, str] = {}
    for p in files:
        rel = p.relative_to(root).as_posix()
        digest = sha256_file(p)
        stat = p.stat()
        inventory.append(
            {
                "path": rel,
                "size_bytes": stat.st_size,
                "mtime_utc": datetime.fromtimestamp(stat.st_mtime, timezone.utc).replace(microsecond=0).isoformat(),
                "sha256": digest,
            }
        )
        file_contents[rel] = read_text(p)

    issues: List[dict] = []
    events: Dict[str, List[dict]] = {}
    headers: Dict[str, List[str]] = {}

    for rel in CSV_EVENT_FILES:
        h, r = read_csv(root / rel)
        headers[rel] = h
        normalized = normalize_rows(rel, r, issues)
        for row in normalized:
            row["_event_uid"] = make_uid(rel, row)
        events[rel] = normalized

    validate_duplicates("data/team_ops/handoff_log.csv", events.get("data/team_ops/handoff_log.csv", []), "entry_id", issues)
    validate_duplicates("data/team_ops/decision_log.csv", events.get("data/team_ops/decision_log.csv", []), "decision_id", issues)

    validate_supersedes("data/team_ops/run_registry.csv", events.get("data/team_ops/run_registry.csv", []), "run_id", "supersedes_run_id", issues)
    validate_supersedes("data/team_ops/change_request_queue.csv", events.get("data/team_ops/change_request_queue.csv", []), "request_id", "supersedes_request_id", issues)
    validate_supersedes("data/team_ops/handoff_log.csv", events.get("data/team_ops/handoff_log.csv", []), "entry_id", "supersedes_entry_id", issues)
    validate_supersedes("data/team_ops/decision_log.csv", events.get("data/team_ops/decision_log.csv", []), "decision_id", "supersedes_decision_id", issues)

    current = {
        "runs": materialize_latest(events.get("data/team_ops/run_registry.csv", []), "run_id"),
        "change_requests": materialize_latest(events.get("data/team_ops/change_request_queue.csv", []), "request_id"),
        "handoffs": materialize_latest(events.get("data/team_ops/handoff_log.csv", []), "entry_id"),
        "decisions": materialize_latest(events.get("data/team_ops/decision_log.csv", []), "decision_id"),
    }

    summary = build_summary(current, events, issues)

    qa_fix_log_path = root / "pipeline" / "07_qa_fix_log.md"
    qa_fix_log_entries = parse_qa_fix_log(read_text(qa_fix_log_path)) if qa_fix_log_path.exists() else []
    qa_files = sorted(
        {
            item.strip()
            for e in qa_fix_log_entries
            for item in e.get("files_changed", [])
            if item.strip()
        }
    )
    git_evidence = collect_git_change_evidence(root)

    execution_changes = {
        "qa_fixer_log_entries": qa_fix_log_entries,
        "git": git_evidence,
        "derived": {
            "qa_fixer_files_changed_unique": qa_files,
            "qa_fixer_request_ids": sorted(
                {
                    rid.strip()
                    for e in qa_fix_log_entries
                    for rid in e.get("request_ids_implemented", [])
                    if rid.strip()
                }
            ),
        },
    }

    record = {
        "record_type": "oracle_team_history_export",
        "generated_utc": generated_utc,
        "repo_root": str(root),
        "scope": {
            "description": "Comprehensive machine-readable record of teams work since inception",
            "included_globs": INCLUDE_GLOBS,
            "event_csv_files": CSV_EVENT_FILES,
        },
        "integrity": {
            "strict_mode": bool(args.strict),
            "issue_count": len(issues),
            "issues": issues,
            "integrity_ok": len(issues) == 0,
        },
        "summary": summary,
        "execution_changes": execution_changes,
        "events": events,
        "current": current,
        "inventory": inventory,
        "files": file_contents,
    }

    record_path = export_dir / "oracle_record.json"
    record_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

    summary_md_path = export_dir / "oracle_record.md"
    summary_md_path.write_text(
        build_markdown_summary(summary, generated_utc, execution_changes) + "\n",
        encoding="utf-8",
    )

    manifest = {
        "generated_utc": generated_utc,
        "record_path": str(record_path.relative_to(root)),
        "summary_path": str(summary_md_path.relative_to(root)),
        "record_sha256": sha256_file(record_path),
        "summary_sha256": sha256_file(summary_md_path),
        "included_file_count": len(files),
        "inventory_entries": len(inventory),
        "integrity_ok": len(issues) == 0,
        "issue_count": len(issues),
    }
    manifest_path = export_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    checksum_path = export_dir / "oracle_record.sha256"
    checksum_path.write_text(f"{manifest['record_sha256']}  {record_path.name}\n", encoding="utf-8")

    output = {
        "export_dir": str(export_dir),
        "record": str(record_path),
        "summary_md": str(summary_md_path),
        "manifest": str(manifest_path),
        "checksum": str(checksum_path),
        "included_file_count": len(files),
        "integrity_ok": manifest["integrity_ok"],
        "issue_count": manifest["issue_count"],
    }

    if args.strict and issues:
        print(json.dumps(output, indent=2))
        print("strict mode failed: integrity issues detected", flush=True)
        return 1

    print(json.dumps(output, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
