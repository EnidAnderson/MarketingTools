# Invariant Test Suite

This folder contains executable invariant tests requested by Team Lead.

## Naming scheme

Invariant IDs follow:
- `INV-GLOBAL-<DOMAIN>-<NNN>`

Request IDs follow:
- `RQ-INV-<NNN>`

Test files follow:
- `test_inv_global_<domain>_<nnn>.sh`

Domains:
- `sec` security
- `bud` budget
- `gov` governance
- `rol` role authority
- `evd` evidence quality
- `aud` auditability
- `chg` change control
- `ops` operational resilience

## Harness

Run all invariant tests:

```bash
scripts/tests/invariants/run_all_invariant_tests.sh
```

Machine-readable output:
- `scripts/tests/invariants/results/invariant_test_report.json`

