# Global Invariants System (Blue Team)

Effective date: 2026-02-10
Owner: Team Lead + Security Steward + Product Steward

## Purpose

Define global invariants that must remain true across workflows, regardless of tool maturity.

## Naming convention

Invariant ID format:
- `INV-GLOBAL-<DOMAIN>-<NNN>`

Domain codes:
- `SEC` security
- `BUD` budget
- `GOV` governance/release control
- `ROL` role and authority
- `EVD` evidence and claim safety
- `AUD` auditability and append-only lineage
- `CHG` change control
- `OPS` operational resilience

Examples:
- `INV-GLOBAL-SEC-001`
- `INV-GLOBAL-BUD-002`

## File layout

1. Registry index:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/invariants/GLOBAL_INVARIANTS_INDEX.csv`

2. Per-invariant spec file:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/invariants/global/INV-GLOBAL-<DOMAIN>-<NNN>.md`

3. Testing request queue (one request per invariant):
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/TEAM_LEAD_INVARIANT_TEST_REQUESTS_2026-02-10.md`

## Per-invariant file schema

Each invariant file must contain exactly these sections:
1. `Invariant Statement`
2. `Rationale`
3. `Scope`
4. `Enforcement Points`
5. `Evidence of Compliance`
6. `Failure State`
7. `Owner Role`
8. `Test Requirements`

## Test naming convention

Test request ID format:
- `RQ-INV-<NNN>` (maps 1:1 to invariant ID)

Recommended test file naming (for implementer):
- `scripts/tests/invariants/test_inv_global_<domain>_<nnn>.sh`
- or Rust equivalent: `rustBotNetwork/app_core/tests/invariants/inv_global_<domain>_<nnn>.rs`

## Enforcement principle

Any red result on a global invariant test blocks external publish.
