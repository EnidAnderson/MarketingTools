#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "Not inside a git repository." >&2
  exit 2
fi

MIN_FREE_GB="${MIN_FREE_GB:-4}"
target_path="${1:-$ROOT/target}"

mkdir -p "$target_path"
avail_kb="$(df -Pk "$target_path" | awk 'NR==2 {print $4}')"
avail_gb="$((avail_kb / 1024 / 1024))"

if (( avail_gb < MIN_FREE_GB )); then
  echo "FAIL[disk_preflight] path=$target_path free_gb=$avail_gb required_gb=$MIN_FREE_GB"
  exit 12
fi

echo "PASS[disk_preflight] path=$target_path free_gb=$avail_gb required_gb=$MIN_FREE_GB"
