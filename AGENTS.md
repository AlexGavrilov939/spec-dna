# AGENTS.md

This file gives the rules for AI coding agents (Claude Code, Codex and others) that work in this repository. `CLAUDE.md` imports this file.

## Project

spec-dna is a small CLI for any project. It keeps engineering specifications small, linked and machine-checked. It gives AI coding agents the minimum sufficient context. It must work for any repository: greenfield or mature, any stack, any agent.

Rule zero: do not put the name, paths, ids, text, metrics or conventions of a consumer project into this repository. This rule applies to code, docs, ADRs, tests, fixtures and commit messages. Refer to a consumer corpus only as an anonymized profile, for example "corpus A: mature product, about 2.3k docs".

- Standards and prior art: `docs/research/matrix.md`
- Decisions: `docs/adr/` (information model, language, distribution, harness, layout, change lifecycle, templates, evals, upgrade)
- User documentation: `docs/README.md` is the map
- Language benchmark prototype, not the tool: `bench/`

The project is at an early stage. It has one Rust crate at the repository root (`src/`, `tests/`). `Cargo.toml` gives the version. Releases are public (`.github/workflows/release.yml`). Do not add structure, languages or tools that the task does not ask for. Give each new dependency a one-line reason in the dependency table of `CONTRIBUTING.md`.

## Done means checked

Before each commit, run:

```sh
pre-commit run --all-files
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

CI runs the same hooks (`.github/workflows/ci.yml`). A change is complete only when these checks pass.

## Conventions

- Text files use UTF-8, LF, a final newline, no trailing whitespace and a 2-space indent. Python, TOML and Rust use 4 spaces. Go uses tabs, as gofmt writes them. `.editorconfig` is the source of truth.
- Write docs, comments and commit messages in English.
- Write specifications, docs and code comments in ASD-STE100 Simplified Technical English. Make them clear and concise. The rules are in `CONTRIBUTING.md`, section "Writing".
- Commits and PR titles follow Conventional Commits. See `CONTRIBUTING.md`.
- Work on a branch. Do not commit to `master` directly.
- Start each change that is not a small fix with `spec-dna new intent <slug>`. `CONTRIBUTING.md`, section "Changes", gives the steps.
- Approval commits: an agent writes an approval commit (a commit that sets `status: accepted` or `status: approved`) only when the owner says so explicitly in the chat. Do not set such a status on your own initiative. `spec-dna.toml` sets `[change] approval = "branch"`, and `approvers` lists the 2 addresses of the owner. The owner approves with `spec-dna approve <slug> <artifact>` in a terminal of their own, or tells the agent in the chat.
- Acceptance records: a change of class `higher-risk` needs an acceptance record before `spec-dna finish`. The agent `acceptor` writes the record, `acceptance.toml` in the folder of the change, and commits it alone. This commit is no approval commit. It sets no `status`, so it needs no go-ahead of the owner. The agent that built the change does not write its record. `spec-dna next <slug>` prints the line that starts the agent. If the record has a `rejected` result, fix the change and run the acceptor again. After 2 `rejected` records, stop and tell the owner.
- GitHub Actions: pin each `uses:` to a full commit SHA, with the version tag in a trailing comment. Set `permissions: {}` at the workflow level, and give each job only the permissions that it needs.
- Do not commit secrets or local agent state: `.env*`, `.claude/settings.local.json`, `CLAUDE.local.md`.
- Docs: follow `CONTRIBUTING.md`, section "Docs layout". Give each page one kind. Write each detail in one reference page. Add each new page to `docs/README.md`.
- Claude Code: `.claude/settings.json` and `.claude/agents/` set a team of models and agents, and `CLAUDE.md` gives its rule. `.claude/agents/README.md` describes the team. `.claude/agents/ownership.md` gives the responsible agent of each task and each path.
- If you change a convention, update `CONTRIBUTING.md` and this file in the same pull request.
