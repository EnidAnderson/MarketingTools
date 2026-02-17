#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi
cd "$ROOT"

if ! command -v cargo-audit >/dev/null 2>&1; then
  echo "SKIP[cargo_audit] cargo-audit not installed"
  exit 0
fi

cargo audit
echo "PASS[cargo_audit]"
