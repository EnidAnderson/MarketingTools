# Security Control Baseline

Effective date: 2026-02-10
Owner: Security Steward

## Controls

| Control ID | Threat coverage | Prevention | Detection | Response owner | Recovery step | Repo mapping |
|---|---|---|---|---|---|---|
| SC-01 | TH-01 | ignore secrets, no direct commit/push | `./scripts/secret_scan.sh`, `.githooks/pre-commit`, `.githooks/pre-push` | Security Steward | rotate exposed secrets, purge artifact, rerun scan | `AGENTS.md`, `scripts/secret_scan.sh`, `.githooks/` |
| SC-02 | TH-02 | treat external text as untrusted, constrain prompts | review logs for unsupported/caveated claims | Product Steward | quarantine run and re-review with Rapid Review Cell | `planning/RAPID_REVIEW_CELL/` |
| SC-03 | TH-03 | mandatory release gates | `planning/reports/RELEASE_GATE_LOG.csv` red status | Team Lead | block publish and open remediation ticket | `planning/RELEASE_GATES_POLICY.md` |
| SC-04 | TH-04 | ADR requirement for architecture-impacting changes | weekly ADR compliance check | Platform Architect | rollback risky integration path; patch with reviewed ADR | `planning/ADR_TRIGGER_RULES.md`, `planning/adrs/` |
| SC-05 | TH-05 | explicit role contracts and escalation protocol | role conflict/approval mismatch in logs | Team Lead | enforce escalation protocol and reassign authority | `planning/AGENT_ROLE_CONTRACTS.md`, `planning/ROLE_ESCALATION_PROTOCOL.md` |

