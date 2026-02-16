#!/usr/bin/env python3
"""
Append-only queue migration helper.

Creates superseding rows that:
1) normalize legacy request IDs (CR-####-TEAM -> CR-TEAM-####),
2) backfill missing supersedes_request_id on duplicate IDs.

This script never mutates/deletes existing rows.
"""

from __future__ import annotations

import csv
import re
from pathlib import Path

ROOT = Path("/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam")
QUEUE = ROOT / "data/team_ops/change_request_queue.csv"

LEGACY_RE = re.compile(r"^CR-([0-9]{4})-(BLUE|RED|GREEN|BLACK|WHITE|GREY)$")


def normalize_request_id(request_id: str) -> str:
    m = LEGACY_RE.match(request_id.strip())
    if not m:
        return request_id
    return f"CR-{m.group(2)}-{m.group(1)}"


def main() -> None:
    rows = []
    with QUEUE.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames or []
        for row in reader:
            rows.append(row)

    if not rows:
        print("No rows found.")
        return

    by_id = {}
    for row in rows:
        rid = (row.get("request_id") or "").strip()
        if rid:
            by_id.setdefault(rid, []).append(row)

    append_rows = []

    # 1) Canonicalize legacy IDs via append-only superseding row.
    for rid, group in sorted(by_id.items()):
        canonical = normalize_request_id(rid)
        if canonical == rid:
            continue
        latest = group[-1]
        new_row = dict(latest)
        new_row["request_id"] = canonical
        new_row["statement"] = (
            "Append-only migration row: canonicalized legacy request_id format to CR-<TEAM>-NNNN."
        )
        new_row["supersedes_request_id"] = rid
        append_rows.append(new_row)

    # 2) Backfill unresolved duplicate supersedes pointers.
    for rid, group in sorted(by_id.items()):
        if len(group) <= 1:
            continue
        latest = group[-1]
        sup = (latest.get("supersedes_request_id") or "").strip()
        if sup == rid:
            continue
        new_row = dict(latest)
        new_row["statement"] = (
            "Append-only migration row: backfilled supersedes_request_id for duplicate request_id lineage."
        )
        new_row["supersedes_request_id"] = rid
        append_rows.append(new_row)

    if not append_rows:
        print("No migration rows required.")
        return

    with QUEUE.open("a", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        for row in append_rows:
            writer.writerow(row)

    print(f"Appended {len(append_rows)} migration row(s) to {QUEUE}")


if __name__ == "__main__":
    main()

