#!/usr/bin/env bash
set -euo pipefail

# Wrapper for frequent operational use by teams.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/scripts/team_ops_cleanup.py" --root "$ROOT" "$@"
