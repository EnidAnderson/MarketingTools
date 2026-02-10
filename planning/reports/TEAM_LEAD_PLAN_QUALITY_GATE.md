# Team Lead Plan Quality Gate

Date: 2026-02-10

## Purpose

Define minimum quality requirements before any plan/spec is accepted into implementation backlog.

## Gate checks (must all pass)

1. Control binding check
- Plan maps required controls to sections, owners, verification artifacts.

2. Quantitative acceptance check
- All success criteria are measurable and bounded.

3. Budget hardening check
- Plan includes budget envelope, warning threshold, hard stop, and fallback behavior.

4. Security hardening check
- Plan includes abuse cases, detection signals, response owners, and blocked-state rules.

5. Role hardening check
- Plan includes signoff matrix with approve/block authority and escalation path.

6. Rollback check
- Plan includes explicit red-state containment and rollback actions.

7. Change-control check
- ADR trigger checkpoints are defined for architecture-impacting milestones.

8. Terminology consistency check
- Canonical glossary references included; ambiguous terms resolved.

## Scoring

- Pass: 8/8 checks green.
- Conditional: 6-7 green with no failures in checks 1-5.
- Fail: <6 green or any failure in checks 1-5.

## Enforcement

1. Only `Pass` plans may enter active implementation.
2. `Conditional` plans require Team Lead waiver with expiry.
3. `Fail` plans are returned with mandatory revision requests.
