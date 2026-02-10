#!/usr/bin/env python3
"""Generate a Markdown summary from NDOC declaration index."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def extract_component(doc: str) -> str:
    for line in doc.splitlines():
        if line.startswith("component:"):
            return line.split(":", 1)[1].strip().strip("`")
    return "undocumented"


def extract_purpose(doc: str) -> str:
    for line in doc.splitlines():
        if line.startswith("purpose:"):
            return line.split(":", 1)[1].strip()
    return ""


def extract_invariants(doc: str) -> list[str]:
    lines = doc.splitlines()
    out: list[str] = []
    in_block = False
    for line in lines:
        if line.startswith("invariants:"):
            in_block = True
            continue
        if in_block:
            if line.startswith("-"):
                out.append(line.lstrip("-").strip())
            elif line.startswith("  -"):
                out.append(line.lstrip(" -").strip())
            elif line and ":" in line and not line.startswith("  "):
                break
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--index",
        default="planning/reports/ndoc_index.json",
        help="Path to NDOC index JSON produced by build_ndoc_index.py",
    )
    parser.add_argument(
        "--output",
        default="planning/reports/ndoc_summary.md",
        help="Path to Markdown summary output.",
    )
    args = parser.parse_args()

    index_path = Path(args.index)
    rows: list[dict[str, Any]] = json.loads(index_path.read_text(encoding="utf-8"))

    by_component: dict[str, list[dict[str, Any]]] = defaultdict(list)
    ndoc_count = 0
    for row in rows:
        doc = row.get("doc", "")
        if row.get("has_ndoc"):
            ndoc_count += 1
        by_component[extract_component(doc)].append(row)

    lines: list[str] = []
    lines.append("# NDOC Summary Report")
    lines.append("")
    lines.append(f"- Total declarations indexed: {len(rows)}")
    lines.append(f"- Declarations with NDOC: {ndoc_count}")
    lines.append("")

    for component in sorted(by_component):
        entries = by_component[component]
        lines.append(f"## {component}")
        lines.append("")
        for entry in entries:
            purpose = extract_purpose(entry.get("doc", ""))
            invariants = extract_invariants(entry.get("doc", ""))
            file_ref = f"{entry['file']}:{entry['line']}"
            lines.append(
                f"- `{entry['kind']} {entry['name']}` ({file_ref})"
                + (f" - {purpose}" if purpose else "")
            )
            for inv in invariants:
                lines.append(f"  - invariant: {inv}")
        lines.append("")

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"Wrote summary to {output_path}")


if __name__ == "__main__":
    main()
