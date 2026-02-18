# Agent Guardrails

## Git workflow (required)

1. Do not run `git commit` directly.
2. Do not run `git push` directly.
3. Ensure hooks are installed for this clone:
   `./scripts/setup_git_guards.sh`
4. Use only this command path for shipping changes:
   `./scripts/git_ship.sh -m "<commit message>"`
5. If push needs explicit target, use:
   `./scripts/git_ship.sh -m "<commit message>" -- <remote> <branch>`

## Secret safety (required)

1. Commits are blocked unless executed through `git_ship.sh`.
2. Secret scan runs on staged content before commit.
3. Secret scan runs on tracked content before push.
4. If a scan fails, remove or redact secrets, then rerun `git_ship.sh`.

## Git add rules (required)

1. Never use `git add .` or `git add -A` by default.
2. Stage only explicit paths for the task being shipped:
   `git add <file1> <file2> ...`
3. Do not stage dependency/vendor/build outputs unless explicitly requested:
   `node_modules/`, `target/`, `dist/`, `build/`, generated binaries, caches.
4. Before shipping, review staged content with:
   `git diff --cached --name-status` and `git diff --cached`.
5. If staged files include unrelated changes, unstage them before shipping:
   `git restore --staged <path>`.
6. Keep commits focused: one logical change per ship command.

## Git ignore rules (required)

1. Keep secrets and local config ignored:
   `.env`, `.env.*`, `*.pem`, `*.key`, `*.p12`, `*.pfx`, `*.jks`, `*.crt`, `*.cer`, `*.der`.
2. Keep example templates tracked when safe:
   `.env.example` must remain trackable.
3. Keep machine-specific and generated noise ignored:
   `.DS_Store`, `Thumbs.db`, `.idea/`, `.vscode/`, `*.log`, `*.tmp`, caches, venv folders.
4. If a new local-only or generated path appears repeatedly, add it to `.gitignore` before shipping.
5. Never commit real credentials to tracked files. Use placeholders in examples/docs.

## Pre-ship checklist (required)

1. `git status --short` and verify only intended files are staged.
2. `./scripts/secret_scan.sh staged` must pass.
3. `./scripts/git_ship.sh -m "<message>"` (or with explicit remote/branch) is the only allowed ship step.
4. Optional hardening gate (recommended before ship):
   `./scripts/governance_preflight.sh`

## Exceptions

1. Emergency bypasses (`--no-verify`) are prohibited unless the repository owner explicitly approves in-thread.

## Release gates and controls (required)

1. Follow `planning/RELEASE_GATES_POLICY.md` for all publish decisions.
2. Publish is blocked if any mandatory gate is red.
3. Append gate results to `planning/reports/RELEASE_GATE_LOG.csv` (append-only).
4. For architecture-impacting changes, ADR is mandatory per `planning/ADR_TRIGGER_RULES.md`.

## Budget controls (required)

1. No run may proceed without a declared budget envelope (per-run, daily, monthly caps).
2. Cap exceedance must transition to explicit blocked state.
3. Exceptions must be append-only in `planning/BUDGET_EXCEPTION_LOG.md` with approver role and expiry.
4. Budget envelopes must conform to `planning/BUDGET_ENVELOPE_SCHEMA.md`.
5. Use template:
   `planning/examples/budget_envelope_example.json`.

## Role contracts (required)

1. Safety-critical decisions must follow `planning/AGENT_ROLE_CONTRACTS.md`.
2. Role conflicts must follow `planning/ROLE_ESCALATION_PROTOCOL.md`.
3. Do not approve role changes without updating role contract documents.

## Colored teams pipeline authority (required)

1. Team pipeline has two allowed modes:
   `full`: `blue -> red -> green -> black -> white -> grey -> qa_fixer`
   `lite` (lean default): `blue -> red -> white -> qa_fixer`
2. In `lite`, `green`, `black`, and `grey` are optional wake-up teams and are consulted only when:
   - explicit risk/constraint/synthesis need is identified, or
   - a blocking flag requires their lane.
3. If optional teams are skipped in `lite`, White must record:
   - skip rationale,
   - residual risk note,
   - whether escalation to Black/Grey is required before publish.
4. Team prompts/specs live under `teams/<team>/prompt.md` and `teams/<team>/spec.md`.
5. Pipeline artifacts under `pipeline/` are append-only and chronological.
6. Only `qa_fixer` may edit executable artifacts (code/config/schema/scripts/hooks).
7. Non-`qa_fixer` teams are analysis/request only and must produce actionable, testable change requests.
8. Any violation of edit authority is a hard failure and must be logged in `data/team_ops/decision_log.csv`.
9. If a team disagrees with a prior stage, it must file a change request; debate-only outputs are invalid.
10. Every `qa_fixer` edit must reference at least one `decision_id` or `change_request_id`.
11. Run team-ops cleanup routinely to keep living logs focused on active work:
   `./scripts/team_ops_cleanup.sh --dry-run` then `./scripts/team_ops_cleanup.sh`.
12. New entries in `data/team_ops/change_request_queue.csv` must use team-scoped IDs with global uniqueness: `CR-<TEAM>-<NNNN>` (example: `CR-RED-0011`).
13. Pipeline-order validation must run with explicit mode:
   - `TEAMS_PIPELINE_MODE=lite teams/_validation/check_pipeline_order.sh`
   - `TEAMS_PIPELINE_MODE=full teams/_validation/check_pipeline_order.sh`
14. Every run must declare mode in `teams/shared/run_mode_registry.csv` (`pipeline_mode` must be `full` or `lite`) before handoffs are appended.
