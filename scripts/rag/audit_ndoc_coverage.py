#!/usr/bin/env python3
"""Audit NDOC coverage for public Rust declarations."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def is_public_decl(doc_row: dict) -> bool:
    return doc_row.get("visibility") == "public"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--index",
        default="planning/reports/ndoc_index.json",
        help="Path to NDOC index JSON",
    )
    parser.add_argument(
        "--output",
        default="planning/reports/ndoc_coverage.md",
        help="Path to coverage report output.",
    )
    args = parser.parse_args()

    rows = json.loads(Path(args.index).read_text(encoding="utf-8"))
    considered = [row for row in rows if is_public_decl(row)]
    documented = [row for row in considered if row.get("has_ndoc")]
    undocumented = [row for row in considered if not row.get("has_ndoc")]

    pct = 0.0
    if considered:
        pct = (len(documented) / len(considered)) * 100.0

    lines = [
        "# NDOC Coverage Audit",
        "",
        f"- Declarations considered: {len(considered)}",
        f"- With NDOC: {len(documented)}",
        f"- Without NDOC: {len(undocumented)}",
        f"- Coverage: {pct:.2f}%",
        "",
        "## Missing NDOC",
        "",
    ]

    for row in undocumented[:300]:
        lines.append(f"- `{row['kind']} {row['name']}` at `{row['file']}:{row['line']}`")

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"Wrote coverage report to {out}")


if __name__ == "__main__":
    main()
