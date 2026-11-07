# AGENTS.md

Rules for AI coding agents (Claude Code, Codex and others) working in this repository. `CLAUDE.md` imports this file.

## Project

spec-dna is a small, project-agnostic CLI that keeps engineering specifications small, linked and machine-checked, and serves AI coding agents the minimum sufficient context. It must work for any repository (greenfield or mature, any stack, any agent).

Rule zero: never put a consumer project's name, paths, ids, text, metrics or conventions into this repository: not in code, docs, ADRs, tests, fixtures or commit messages. Cite a consumer corpus only as an anonymized profile, for example "corpus A: mature product, about 2.3k docs".

- Standards and prior art: `docs/research/matrix.md`
- Decisions: `docs/adr/` (information model, language, distribution)
- Language benchmark prototype, not the tool: `bench/`

Early stage: one Rust crate at the repository root (`src/`, `tests/`), version 0.1.0, not released. Do not add structure, languages or tooling that the task does not ask for. Every new dependency needs a one-line reason in the README's dependency table.

## Done means checked

Run before every commit:

```sh
pre-commit run --all-files
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

CI runs the same hooks (`.github/workflows/ci.yml`). A change is not done until they pass.

## Conventions

- Text files: UTF-8, LF, final newline, no trailing whitespace, 2-space indent (Python, TOML and Rust: 4 spaces; Go: tabs, as gofmt writes them). `.editorconfig` is the source of truth.
- Docs, comments and commit messages in English.
- Write specifications, docs and code comments in ASD-STE100 Simplified Technical English: clear and concise. The rules are in `CONTRIBUTING.md`, section "Writing".
- Commits and PR titles follow Conventional Commits, see `CONTRIBUTING.md`.
- Work on a branch. Never commit to `master` directly.
- GitHub Actions: pin every `uses:` to a full commit SHA with the version tag in a trailing comment. Set `permissions: {}` at workflow level and grant the minimum per job.
- Never commit secrets or local agent state: `.env*`, `.claude/settings.local.json`, `CLAUDE.local.md`.
- If you change a convention, update `CONTRIBUTING.md` and this file in the same pull request.
