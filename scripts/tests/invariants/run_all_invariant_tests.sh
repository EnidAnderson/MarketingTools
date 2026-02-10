#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$ROOT_DIR/results"
REPORT_PATH="$RESULTS_DIR/invariant_test_report.json"
mkdir -p "$RESULTS_DIR"

python3 - "$ROOT_DIR" "$REPORT_PATH" <<'PY'
import datetime
import glob
import json
import os
import subprocess
import sys
import time

root_dir = sys.argv[1]
report_path = sys.argv[2]
tests = sorted(glob.glob(os.path.join(root_dir, "test_inv_global_*.sh")))

results = []
for test_path in tests:
    started = time.time()
    proc = subprocess.run([test_path], capture_output=True, text=True)
    ended = time.time()
    output = (proc.stdout + proc.stderr).strip()
    results.append(
        {
            "test_file": os.path.basename(test_path),
            "status": "pass" if proc.returncode == 0 else "fail",
            "exit_code": proc.returncode,
            "duration_ms": int((ended - started) * 1000),
            "diagnostic": output.splitlines()[-1] if output else "",
        }
    )

failed = [r for r in results if r["status"] == "fail"]
report = {
    "generated_at_utc": datetime.datetime.utcnow().replace(microsecond=0).isoformat() + "Z",
    "total": len(results),
    "passed": len(results) - len(failed),
    "failed": len(failed),
    "results": results,
}

with open(report_path, "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2)

print(f"Wrote report: {report_path}")
print(f"Total={report['total']} Passed={report['passed']} Failed={report['failed']}")
if failed:
    for r in failed:
        print(f"FAILED: {r['test_file']} exit={r['exit_code']} diagnostic={r['diagnostic']}")
    sys.exit(1)
PY

