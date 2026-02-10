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

## Exceptions

1. Emergency bypasses (`--no-verify`) are prohibited unless the repository owner explicitly approves in-thread.
