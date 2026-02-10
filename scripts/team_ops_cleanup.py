#!/usr/bin/env python3
"""Archive non-active team-ops tickets/log entries and keep living logs focused on active work."""

from __future__ import annotations

import argparse
import csv
import json
import shutil
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Sequence, Set, Tuple


ACTIVE_RUN_STATUSES_DEFAULT = {
    "initialized",
    "active",
    "in_progress",
    "blocked",
    "blocked_missing_stages",
    "awaiting_input",
    "ready",
}

ACTIVE_REQUEST_STATUSES_DEFAULT = {
    "open",
    "in_progress",
    "blocked",
    "ready",
    "todo",
    "pending",
}


@dataclass
class SplitResult:
    keep: List[dict]
    archive: List[dict]


def _read_csv(path: Path) -> Tuple[List[str], List[dict]]:
    if not path.exists():
        raise FileNotFoundError(f"Missing required file: {path}")
    with path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        header = reader.fieldnames or []
        rows = list(reader)
    return header, rows


def _write_csv(path: Path, header: Sequence[str], rows: Sequence[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with tmp.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(header), extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
    tmp.replace(path)


def _normalize(value: str | None) -> str:
    return (value or "").strip().lower()


def _latest_run_rows(rows: Sequence[dict]) -> Dict[str, dict]:
    latest: Dict[str, dict] = {}
    for row in rows:
        run_id = (row.get("run_id") or "").strip()
        if not run_id:
            continue
        ts = (row.get("created_utc") or "").strip()
        prev = latest.get(run_id)
        if prev is None or ts >= (prev.get("created_utc") or ""):
            latest[run_id] = row
    return latest


def _split_run_registry(
    rows: Sequence[dict],
    active_run_statuses: Set[str],
    keep_run_history: bool,
) -> Tuple[SplitResult, Set[str]]:
    latest = _latest_run_rows(rows)
    active_runs = {
        run_id
        for run_id, row in latest.items()
        if _normalize(row.get("status")) in active_run_statuses
    }

    if keep_run_history:
        keep = [r for r in rows if (r.get("run_id") or "").strip() in active_runs]
    else:
        keep = [latest[run_id] for run_id in sorted(active_runs)]

    keep_keys = {id(r) for r in keep}
    archive = [r for r in rows if id(r) not in keep_keys]
    return SplitResult(keep=keep, archive=archive), active_runs


def _split_by_active_runs(rows: Sequence[dict], active_runs: Set[str]) -> SplitResult:
    keep: List[dict] = []
    archive: List[dict] = []
    for row in rows:
        run_id = (row.get("run_id") or "").strip()
        if run_id and run_id in active_runs:
            keep.append(row)
        else:
            archive.append(row)
    return SplitResult(keep=keep, archive=archive)


def _split_change_requests(
    rows: Sequence[dict],
    active_runs: Set[str],
    active_request_statuses: Set[str],
) -> SplitResult:
    superseded_ids = {
        (r.get("supersedes_request_id") or "").strip()
        for r in rows
        if (r.get("supersedes_request_id") or "").strip()
    }

    keep: List[dict] = []
    archive: List[dict] = []
    for row in rows:
        request_id = (row.get("request_id") or "").strip()
        run_id = (row.get("run_id") or "").strip()
        status = _normalize(row.get("status"))

        is_active_status = status in active_request_statuses
        is_active_run = run_id in active_runs if run_id else False
        is_superseded = request_id in superseded_ids if request_id else False

        if is_active_status and is_active_run and not is_superseded:
            keep.append(row)
        else:
            archive.append(row)

    return SplitResult(keep=keep, archive=archive)


def _copy_as_backup(src: Path, backup_dir: Path) -> None:
    backup_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, backup_dir / src.name)


def _archive_rows(
    archive_dir: Path,
    file_name: str,
    header: Sequence[str],
    rows: Sequence[dict],
) -> Path | None:
    if not rows:
        return None
    out = archive_dir / file_name
    _write_csv(out, header, rows)
    return out


def _parse_set(value: str, default: Set[str]) -> Set[str]:
    if not value.strip():
        return set(default)
    return {_normalize(v) for v in value.split(",") if v.strip()}


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="Archive non-active team-ops tickets/log entries and keep living logs focused on active work."
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be archived without writing files.",
    )
    p.add_argument(
        "--keep-run-history",
        action="store_true",
        help="Keep all rows for active runs in run_registry.csv (default keeps only latest per active run).",
    )
    p.add_argument(
        "--active-run-statuses",
        default=",".join(sorted(ACTIVE_RUN_STATUSES_DEFAULT)),
        help="Comma-separated statuses considered active for run_registry.",
    )
    p.add_argument(
        "--active-request-statuses",
        default=",".join(sorted(ACTIVE_REQUEST_STATUSES_DEFAULT)),
        help="Comma-separated statuses considered active in change_request_queue.",
    )
    p.add_argument(
        "--root",
        default=".",
        help="Repo root path (default: current directory).",
    )
    return p


def main() -> int:
    args = _build_parser().parse_args()
    root = Path(args.root).resolve()
    data_dir = root / "data" / "team_ops"

    required_files = {
        "run_registry.csv": data_dir / "run_registry.csv",
        "change_request_queue.csv": data_dir / "change_request_queue.csv",
        "handoff_log.csv": data_dir / "handoff_log.csv",
        "decision_log.csv": data_dir / "decision_log.csv",
    }

    for path in required_files.values():
        if not path.exists():
            print(f"error: required file not found: {path}", file=sys.stderr)
            return 2

    active_run_statuses = _parse_set(args.active_run_statuses, ACTIVE_RUN_STATUSES_DEFAULT)
    active_request_statuses = _parse_set(args.active_request_statuses, ACTIVE_REQUEST_STATUSES_DEFAULT)

    headers: Dict[str, List[str]] = {}
    rows: Dict[str, List[dict]] = {}
    for name, path in required_files.items():
        h, r = _read_csv(path)
        headers[name] = h
        rows[name] = r

    run_split, active_runs = _split_run_registry(
        rows["run_registry.csv"],
        active_run_statuses=active_run_statuses,
        keep_run_history=args.keep_run_history,
    )
    req_split = _split_change_requests(
        rows["change_request_queue.csv"],
        active_runs=active_runs,
        active_request_statuses=active_request_statuses,
    )
    handoff_split = _split_by_active_runs(rows["handoff_log.csv"], active_runs)
    decision_split = _split_by_active_runs(rows["decision_log.csv"], active_runs)

    split_map = {
        "run_registry.csv": run_split,
        "change_request_queue.csv": req_split,
        "handoff_log.csv": handoff_split,
        "decision_log.csv": decision_split,
    }

    summary = {
        "timestamp_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
        "dry_run": args.dry_run,
        "active_run_ids": sorted(active_runs),
        "active_run_statuses": sorted(active_run_statuses),
        "active_request_statuses": sorted(active_request_statuses),
        "counts": {
            name: {
                "before": len(rows[name]),
                "keep": len(split_map[name].keep),
                "archive": len(split_map[name].archive),
            }
            for name in split_map
        },
    }

    if args.dry_run:
        print(json.dumps(summary, indent=2))
        return 0

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    archive_dir = data_dir / "archive" / timestamp
    backup_dir = archive_dir / "backup"
    archive_csv_dir = archive_dir / "archived_rows"

    archive_manifest = {
        **summary,
        "archive_dir": str(archive_dir),
        "archived_files": [],
    }

    for name, path in required_files.items():
        _copy_as_backup(path, backup_dir)

    for name, path in required_files.items():
        split = split_map[name]
        _write_csv(path, headers[name], split.keep)
        archived = _archive_rows(archive_csv_dir, name, headers[name], split.archive)
        if archived:
            archive_manifest["archived_files"].append(str(archived))

    manifest_path = archive_dir / "manifest.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(archive_manifest, indent=2) + "\n", encoding="utf-8")

    index_path = data_dir / "archive" / "archive_index.csv"
    index_header = [
        "timestamp_utc",
        "archive_dir",
        "active_run_ids",
        "run_rows_archived",
        "request_rows_archived",
        "handoff_rows_archived",
        "decision_rows_archived",
    ]

    index_rows: List[dict] = []
    if index_path.exists():
        ih, ir = _read_csv(index_path)
        if ih == index_header:
            index_rows = ir

    index_rows.append(
        {
            "timestamp_utc": archive_manifest["timestamp_utc"],
            "archive_dir": str(archive_dir),
            "active_run_ids": ";".join(summary["active_run_ids"]),
            "run_rows_archived": summary["counts"]["run_registry.csv"]["archive"],
            "request_rows_archived": summary["counts"]["change_request_queue.csv"]["archive"],
            "handoff_rows_archived": summary["counts"]["handoff_log.csv"]["archive"],
            "decision_rows_archived": summary["counts"]["decision_log.csv"]["archive"],
        }
    )
    _write_csv(index_path, index_header, index_rows)

    print(json.dumps(archive_manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
