# Security Daily Checklist

Goal: complete in less than 10 minutes.

1. Run `./scripts/secret_scan.sh staged` and confirm pass.
2. Confirm no unresolved critical security findings for active release scope.
3. Confirm `planning/reports/RELEASE_GATE_LOG.csv` latest row has non-red security gate.
4. Confirm no external artifact is marked `unsupported` in Rapid Review evidence logs.
5. Confirm new dependencies (if any) have ADR check.
6. If any check fails, block publish and open/assign remediation ticket.

