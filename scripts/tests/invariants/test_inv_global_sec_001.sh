#!/usr/bin/env bash
set -euo pipefail

# provenance: decision_id=DEC-0003; change_request_id=RQ-INV-001

ROOT="/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam"
source "$ROOT/scripts/tests/invariants/common.sh"
require_file "$ROOT/scripts/secret_scan.sh"

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

git -C "$TMP_DIR" init -q
git -C "$TMP_DIR" config user.name "Invariant Test"
git -C "$TMP_DIR" config user.email "invariant@example.com"
printf 'seed\n' > "$TMP_DIR/seed.txt"
git -C "$TMP_DIR" add seed.txt
git -C "$TMP_DIR" commit -q -m "seed"

# Negative: staged secret should fail staged scan.
printf 'API_KEY="%s%s"\n' "abcdefghijklmnopqrstuvwxyz" "123456" > "$TMP_DIR/staged_secret.env"
git -C "$TMP_DIR" add staged_secret.env
if (cd "$TMP_DIR" && "$ROOT/scripts/secret_scan.sh" staged >/dev/null 2>&1); then
  die 111 "staged secret was not blocked"
fi
git -C "$TMP_DIR" reset -q HEAD staged_secret.env
rm -f "$TMP_DIR/staged_secret.env"

# Negative: tracked secret should fail tracked scan.
printf 'token = "%s%s"\n' "ABCDEFGHIJKLMNOPQRSTUVWXYZ" "123456" > "$TMP_DIR/tracked_secret.txt"
git -C "$TMP_DIR" add tracked_secret.txt
git -C "$TMP_DIR" commit -q -m "tracked secret fixture"
if (cd "$TMP_DIR" && "$ROOT/scripts/secret_scan.sh" tracked >/dev/null 2>&1); then
  die 112 "tracked secret was not blocked"
fi

# Positive: clean tracked and staged scopes should pass.
git -C "$TMP_DIR" rm -q tracked_secret.txt
git -C "$TMP_DIR" commit -q -m "remove tracked secret fixture"
(cd "$TMP_DIR" && "$ROOT/scripts/secret_scan.sh" tracked >/dev/null)
(cd "$TMP_DIR" && "$ROOT/scripts/secret_scan.sh" staged >/dev/null)

pass "INV-GLOBAL-SEC-001 validated"
