#!/usr/bin/env bash
set -euo pipefail

die() {
  local code="$1"
  shift
  echo "FAIL[$code] $*" >&2
  exit "$code"
}

pass() {
  echo "PASS $*"
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    die 20 "required file missing: $path"
  fi
}

