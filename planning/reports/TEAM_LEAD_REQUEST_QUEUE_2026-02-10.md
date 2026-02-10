# Team Lead Request Queue

Date: 2026-02-10  
Mode: Team Lead (requests only, no implementation)

Follow-on queue:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/TEAM_LEAD_REQUEST_QUEUE_STRATEGIC_2026-02-10.md`

## Execution Protocol for Implementing Bot

1. Pull highest-priority open request first.
2. For each request, submit:
- files changed,
- test/verification evidence,
- residual risks,
- status update (`DONE`, `BLOCKED`, or `PARTIAL`).
3. Do not expand scope without explicit request update.
4. Keep one commit per request unless request explicitly spans multiple commits.

## Request Status Legend

- `OPEN`: ready for implementation.
- `BLOCKED`: waiting on dependency or decision.
- `DONE`: accepted by Team Lead.

---

## RQ-001 (P0) Harden secret scanning to reduce false negatives

Status: `OPEN`  
Owner: Implementing Bot

### Why
Current secret checks are useful but still regex-based and can miss edge cases (multiline secrets, encoded blobs, atypical token formats).

### Required changes
1. Extend `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/scripts/secret_scan.sh` to support:
- configurable include/exclude globs,
- JSON/text output mode,
- scan scope options (`staged`, `tracked`, `all-files`).
2. Add a baseline allowlist file:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/.secrets.allowlist`
3. Update hook scripts to pass consistent mode flags:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/.githooks/pre-commit`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/.githooks/pre-push`
4. Document usage and maintenance:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/AGENTS.md`

### Acceptance criteria
1. Known fake secret fixtures are detected and block commit/push.
2. Placeholder/example strings (documented allowlist cases) do not fail scans.
3. `./scripts/secret_scan.sh staged`, `tracked`, and `all-files` each return clear pass/fail status and machine-readable output option.

### Verification evidence required
1. Command transcript showing fail on injected test secret and pass after removal.
2. Command transcript showing allowlisted placeholder does not fail.
3. Final `git status --short` showing only intended files changed.

---

## RQ-002 (P0) Add protected branch + safe-push guidance checks

Status: `OPEN`  
Owner: Implementing Bot

### Why
Commit/push guardrails exist locally, but no guard yet against accidental direct pushes to protected branches in repos where policy requires PR-based flow.

### Required changes
1. Update `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/scripts/git_ship.sh`:
- add optional `--require-pr` mode that blocks pushes directly to `main`.
- print next-step guidance when blocked.
2. Add branch policy section to:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/AGENTS.md`

### Acceptance criteria
1. In `--require-pr` mode, direct push to `main` exits non-zero with clear guidance.
2. Non-main branch push remains allowed.
3. Existing default mode remains backward compatible.

### Verification evidence required
1. Transcript showing `--require-pr` block on main.
2. Transcript showing successful push on feature branch in same mode.

---

## RQ-003 (P1) Wire Rapid Review Cell into campaign workflow docs

Status: `OPEN`  
Owner: Implementing Bot

### Why
The Rapid Review Cell exists but is not yet integrated into operational workflow docs, so it can be skipped during campaign execution.

### Required changes
1. Update:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/WORKFLOW.md`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/PLANNING.md`
2. Add mandatory review gate referencing:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/RAPID_REVIEW_CELL/SOP.md`
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/RAPID_REVIEW_CELL/SUMMARY_TEMPLATE.md`
3. Add explicit handoff artifact requirement: completed summary must be attached before publish.

### Acceptance criteria
1. Workflow includes a named, mandatory pre-publish claim safety step.
2. Required log files and summary output are explicitly listed.
3. Failure path is defined (`needs_revision` or `blocked` halts publish).

### Verification evidence required
1. Diff snippets for both docs.
2. Example “happy path” + “blocked path” text in workflow docs.

---

## RQ-004 (P1) Add Team Lead request intake file for recurring cycles

Status: `OPEN`  
Owner: Implementing Bot

### Why
Need a stable location for ongoing Team Lead directives beyond this dated queue.

### Required changes
1. Create:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/planning/reports/TEAM_LEAD_REQUEST_QUEUE.md`
2. Seed with sections:
- Active requests
- Blocked requests
- Done requests
- Decision log
3. Add update rule in:
- `/Users/e/Documents/GUNS/ENIDSASSETS/NaturesDietMarketingTeam/AGENTS.md`

### Acceptance criteria
1. New queue template exists with clear schema.
2. Existing dated requests are linked from new queue.

### Verification evidence required
1. File content snapshot and links.

---

## Request to Start Immediately

Start with `RQ-001`, then `RQ-003`.
