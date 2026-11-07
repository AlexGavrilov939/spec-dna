# AGENTS.md

Rules for AI coding agents (Claude Code, Codex and others) working in this repository. `CLAUDE.md` imports this file.

## Project

spec-dna: spec-driven development for AI coding agents.

Early stage: there is a skeleton and conventions, no code yet. Do not add structure, languages or tooling that the task does not ask for.

## Done means checked

Run before every commit:

```sh
pre-commit run --all-files
```

CI runs the same hooks (`.github/workflows/ci.yml`). A change is not done until they pass.

## Conventions

- Text files: UTF-8, LF, final newline, no trailing whitespace, 2-space indent. `.editorconfig` is the source of truth.
- Docs, comments and commit messages in English.
- Commits and PR titles follow Conventional Commits, see `CONTRIBUTING.md`.
- Work on a branch. Never commit to `master` directly.
- GitHub Actions: pin every `uses:` to a full commit SHA with the version tag in a trailing comment. Set `permissions: {}` at workflow level and grant the minimum per job.
- Never commit secrets or local agent state: `.env*`, `.claude/settings.local.json`, `CLAUDE.local.md`.
- If you change a convention, update `CONTRIBUTING.md` and this file in the same pull request.
