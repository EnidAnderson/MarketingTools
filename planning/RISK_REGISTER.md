# Program Risk Register

Append-only operating register.

| Risk ID | Opened UTC | Risk statement | Owner | Probability | Impact | Mitigation | Trigger | Status | Target close UTC | Supersedes risk ID |
|---|---|---|---|---|---|---|---|---|---|---|
| RISK-001 | 2026-02-10T00:00:00Z | Claims could ship without complete evidence mapping. | Product Steward | medium | high | enforce release/evidence gate | non-green HC-10 | open | 2026-02-28T00:00:00Z | |
| RISK-002 | 2026-02-10T00:00:00Z | Spend can exceed intended envelope without explicit stop path. | Team Lead | medium | high | enforce budget caps + exception expiry | non-green HC-04/HC-05 | open | 2026-02-21T00:00:00Z | |
| RISK-003 | 2026-02-10T00:00:00Z | Role conflicts can delay safety-critical decisions. | Team Lead | medium | medium | escalation protocol with 24h safety block | unresolved role conflict >24h | open | 2026-02-21T00:00:00Z | |

