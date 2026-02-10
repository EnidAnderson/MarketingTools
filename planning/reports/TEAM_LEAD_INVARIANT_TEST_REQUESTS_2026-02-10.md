# Team Lead Invariant Test Requests

Date: 2026-02-10  
Mode: Blue Team directive (one request per invariant)

## Standard test-suite structure for implementer

1. Create invariant test harness index:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/scripts/tests/invariants/run_all_invariant_tests.sh`
2. Add one test script per invariant using naming:
- `scripts/tests/invariants/test_inv_global_<domain>_<nnn>.sh`
3. Emit machine-readable result summary:
- `scripts/tests/invariants/results/invariant_test_report.json`

## Request format

For each request:
- implement test file,
- include positive + negative case,
- include explicit fail code and diagnostic text,
- update harness index.

---

## RQ-INV-001 -> INV-GLOBAL-SEC-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_sec_001.sh`

### Must verify
1. staged secret detection blocks ship.
2. tracked secret detection blocks push.
3. clean state passes both checks.

### Acceptance
1. Negative cases exit non-zero.
2. Positive case exits zero.

---

## RQ-INV-002 -> INV-GLOBAL-SEC-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_sec_002.sh`

### Must verify
1. untrusted external-content sample cannot be promoted without evidence/caveat path.
2. bypass attempt is rejected.

### Acceptance
1. validation fails when review-cell evidence binding is missing.

---

## RQ-INV-003 -> INV-GLOBAL-GOV-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_gov_001.sh`

### Must verify
1. any red mandatory gate causes publish-block result.
2. all-green gate state allows publish path.

### Acceptance
1. red-gate simulation exits non-zero and prints blocking gate ID.

---

## RQ-INV-004 -> INV-GLOBAL-BUD-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_bud_001.sh`

### Must verify
1. run without budget envelope is rejected.
2. run with complete envelope is accepted.

### Acceptance
1. missing field detection identifies exact missing keys.

---

## RQ-INV-005 -> INV-GLOBAL-BUD-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_bud_002.sh`

### Must verify
1. cap exceedance transitions to blocked state.
2. approved exception with valid expiry can temporarily unblock.

### Acceptance
1. expired exception does not unblock.

---

## RQ-INV-006 -> INV-GLOBAL-ROL-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_rol_001.sh`

### Must verify
1. safety-critical decision record with ambiguous owner fails validation.
2. single explicit authorized owner passes.

### Acceptance
1. role-authority mismatch is caught with actionable message.

---

## RQ-INV-007 -> INV-GLOBAL-ROL-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_rol_002.sh`

### Must verify
1. unresolved role conflict beyond SLA forces blocked status.
2. resolved conflict within SLA allows progression.

### Acceptance
1. SLA breach path includes escalation reference.

---

## RQ-INV-008 -> INV-GLOBAL-EVD-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_evd_001.sh`

### Must verify
1. every externally-facing claim links to evidence or caveat.
2. missing linkage causes failure.

### Acceptance
1. outputs include offending claim IDs.

---

## RQ-INV-009 -> INV-GLOBAL-AUD-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_aud_001.sh`

### Must verify
1. append-only logs reject mutation/delete patterns.
2. superseding-entry correction path is accepted.

### Acceptance
1. mutation detection is deterministic and file-specific.

---

## RQ-INV-010 -> INV-GLOBAL-CHG-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_chg_001.sh`

### Must verify
1. architecture-triggered change without ADR fails.
2. architecture-triggered change with ADR reference passes.

### Acceptance
1. failure output cites missing ADR path/id.

---

## RQ-INV-011 -> INV-GLOBAL-OPS-001

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_ops_001.sh`

### Must verify
1. safe mode blocks external publish.
2. safe mode blocks budget/security exception approvals.
3. cleared safe mode re-enables permitted path.

### Acceptance
1. blocked operations produce explicit reason code.

---

## RQ-INV-012 -> INV-GLOBAL-OPS-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_ops_002.sh`

### Must verify
1. critical incident timeline records containment <= 60 min.
2. breach of 60-min SLA fails check.

### Acceptance
1. report includes elapsed time and owner.

---

## RQ-INV-013 -> INV-GLOBAL-GOV-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_gov_002.sh`

### Must verify
1. external publish fails without two-role signoff.
2. publish passes with technical + business signoff.

### Acceptance
1. missing-signoff output names required missing role.

---

## RQ-INV-014 -> INV-GLOBAL-EVD-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_evd_002.sh`

### Must verify
1. unsupported claims appear only in “Do not claim yet”.
2. unsupported claim in “Safe to say” fails validation.

### Acceptance
1. failing output includes claim ID and summary section.

---

## RQ-INV-015 -> INV-GLOBAL-AUD-002

Status: `OPEN`

### Build
1. `scripts/tests/invariants/test_inv_global_aud_002.sh`

### Must verify
1. artifact lineage record includes inputs, spec, run metadata, decision.
2. missing lineage element fails validation.

### Acceptance
1. failure output lists missing lineage dimensions.

---

## Execution order

1. Implement all critical-severity invariant tests first:
- `RQ-INV-001`, `003`, `004`, `005`, `006`, `008`, `011`, `013`, `014`
2. Then implement high-severity tests:
- `RQ-INV-002`, `007`, `009`, `010`, `012`, `015`
