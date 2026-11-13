# Demo: a new project

The demo shows spec-dna on a small project that starts without it: a book-loan HTTP API in Python with its tests. The seed is in `tests/fixtures/demo/`: the application (`app/`) and the documents that a user writes on adoption (`docs/`).

## Replay without an agent

`scripts/demo-replay.sh` replays the setup, an upgrade and the uninstall. CI runs it in the `Demo replay` job.

```sh
scripts/demo-replay.sh [work directory]
```

The script does these steps:

1. It builds the current version and the next patch version as an unreleased `-dev` version, for example `0.1.2-dev` for `0.1.1`.
2. It makes a git repository from the application and runs the "Start" block of [the quickstart](quickstart.md) as it is written. It checks that `init` added one line to `CLAUDE.md`. It also checks that `init` wrote no import and no `AGENTS.md`, and that the prompt prefix is 300 bytes or less.
3. It adds the documents and uses `context`, `show`, `mv` and `check`. A change that removes a cited identifier must give SD607 in `check --base`, as in CI.
4. It runs the upgrade of the quickstart to the `-dev` version. The pin mismatch gives exit code 3. `init` writes the adapters of the new version. Then the script rolls back to the current version with `git revert`.
5. It runs the "Uninstall" block of the quickstart. The documents must not change. The uninstall must remove only the files of spec-dna.

## Agent stages

The agent stages have no script, because they need a model and a budget. Each stage gives a task to a new headless session (`claude -p`). The session gets only the task text and the entry points that `init` wrote. The task text does not name spec-dna.

| Stage | Task | What it shows |
| --- | --- | --- |
| a | A small requirement change that starts from a failing test (a second renewal) | `context <code path>` finds the requirement. The agent updates it |
| b | A new capability (book holds) with an HTTP contract and a decision | New documents of the right types, linked to the code |
| c | A change of policy (capped fines): update the spec and supersede the old decision | `supersedes`, the frozen decision, SD605 |
| d | A large policy text that puts the loans spec over its hard budget | SD702, then a split with `spec-dna mv` that keeps the old ids as aliases |

For each stage, a control session runs the same task on a copy of the same commit. The copy has no `spec-dna.toml`, no adapters and no `spec-dna` on `PATH`. The ratio of total input tokens (treatment over control) is the fixed overhead of spec-dna on a small project.

After each stage, these conditions must be true:

- `spec-dna check --base <stage start>` passes on a clean clone.
- The tests pass.
- `spec-dna transcript` reports the tokens, the document reads, the tool calls, the `spec-dna` commands and the time to the first expected document.

Use the same model and effort in both arms. Run the control with a `PATH` that has no `spec-dna`. Deny the session access to other projects, so that it reads only its own copy.
