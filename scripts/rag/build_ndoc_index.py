#!/usr/bin/env python3
"""Build a declaration index from NDOC structured Rust doc comments."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

DECL_RE = re.compile(
    r"^\s*(pub(?:\([^)]+\))?\s+)?(?:async\s+)?(fn|struct|enum|trait|mod|type)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def parse_rust_file(path: Path) -> list[dict[str, Any]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    out: list[dict[str, Any]] = []
    pending_docs: list[str] = []

    for i, line in enumerate(lines, start=1):
        stripped = line.strip()

        if stripped.startswith("///"):
            pending_docs.append(stripped[3:].strip())
            continue

        # Keep doc comments attached across Rust attributes like #[derive(...)].
        if stripped.startswith("#[") and pending_docs:
            continue

        decl = DECL_RE.match(line)
        if decl:
            vis, kind, name = decl.groups()
            doc_text = "\n".join(pending_docs).strip()
            has_ndoc = "NDOC" in doc_text
            out.append(
                {
                    "file": str(path),
                    "line": i,
                    "kind": kind,
                    "name": name,
                    "visibility": "public" if vis else "private",
                    "has_ndoc": has_ndoc,
                    "doc": doc_text,
                }
            )
            pending_docs = []
            continue

        if stripped and not stripped.startswith("//"):
            pending_docs = []

    return out


def collect_rust_files(roots: list[Path]) -> list[Path]:
    files: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        files.extend(sorted(root.rglob("*.rs")))
    return files


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--roots",
        nargs="+",
        default=["rustBotNetwork/app_core/src", "src-tauri/src"],
        help="Root directories to scan for Rust source files.",
    )
    parser.add_argument(
        "--output",
        default="planning/reports/ndoc_index.json",
        help="Path to write JSON index output.",
    )
    args = parser.parse_args()

    roots = [Path(p) for p in args.roots]
    files = collect_rust_files(roots)

    index: list[dict[str, Any]] = []
    for path in files:
        index.extend(parse_rust_file(path))

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(index, indent=2), encoding="utf-8")
    print(f"Wrote {len(index)} declarations to {output_path}")


if __name__ == "__main__":
    main()
