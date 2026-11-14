# Demo: a new project

This page shows spec-dna on a small project that starts without it: a book-loan HTTP API in Python with its tests. It gives a replay that needs no agent, and 4 stages that need one. Read it to see what a new project gets, or to run the same stages with your agent.

The seed is in `tests/fixtures/demo/`: the application (`app/`) and the documents that a user writes on adoption (`docs/`). The documents are in the folders of their layers: `requirements/loans.md`, `decisions/` and `interfaces/http.md`.

## Replay without an agent

`scripts/demo-replay.sh` replays the setup, an upgrade and the uninstall. CI runs it in the `Demo replay` job.

```sh
scripts/demo-replay.sh [work directory]
```

The script does these steps:

1. It builds the current version and the next patch version as an unreleased `-dev` version, for example `0.1.2-dev` for `0.1.1`.
2. It makes a git repository from the application and runs the "Start" block of [the quickstart](quickstart.md) as it is written. It checks that `init` added 2 lines to `CLAUDE.md`: the spec-dna line and the model-team line of the lean harness. It also checks that `init` wrote the agent files, no import and no `AGENTS.md`. The prompt prefix of the adapters must be 300 bytes or less.
3. The small graph of `init` has no `interfaces` layer, and the seed has an interface document. The script runs `spec-dna layer add interfaces --types interface --moved-from intent` and `spec-dna init` (the skill lists the types of the layers). It runs `check` and commits `spec-dna.toml` and the skill, as a change of their own (SD503). Then it adds the documents and uses `context`, `show`, `mv` and `check`. The ids have the folder of the layer, for example `requirements/loans#loan-period`. A change that removes a cited identifier must give SD607 in `check --base`, as in CI.
4. It runs the upgrade of the quickstart to the `-dev` version. The pin mismatch gives exit code 3. `init` writes the adapters and the harness files of the new version. Then the script rolls back to the current version with `git revert`.
5. It disables the harness with `init --reconfigure`. Then it runs the "Uninstall" block of the quickstart. The documents must not change. The uninstall must remove only the files of spec-dna. `.claude/settings.json` stays, because it can hold keys of the project.

## Agent stages

The agent stages have no script, because they need a model and a budget. The stages ran with Claude Code. Each stage gives a task to a new headless session (`claude -p`). Any agent with a headless mode can run the same stages.

The session gets only the task text and the entry points that `init` wrote. These are the line in `AGENTS.md` or `CLAUDE.md` and, for Claude Code, the skill. The task text does not name spec-dna.

| Stage | Task | What it shows |
| --- | --- | --- |
| a | A small requirement change that starts from a test that fails (a second renewal) | `context <code path>` finds the requirement. The agent updates it |
| b | A new capability (book holds) with an HTTP interface and a decision | New documents of the right types, linked to the code |
| c | A change of policy (capped fines): update the requirements and supersede the old decision | `supersedes`, the locked decision, SD605 |
| d | A large policy text that puts the loans requirements over their hard budget | SD702, then a split with `spec-dna mv` that keeps the old ids as aliases |

For each stage, a control session runs the same task on a copy of the same commit. The copy has no `spec-dna.toml`, no adapters and no `spec-dna` on `PATH`. The ratio of total input tokens (treatment over control) is the fixed overhead of spec-dna on a small project.

After each stage, these conditions must be true:

- `spec-dna check --base <stage start>` passes on a clean clone.
- The tests pass.
- `spec-dna transcript` reports the tokens, the document reads, the tool calls, the `spec-dna` commands and the time to the first expected document.

`transcript` reads the sessions of Claude Code with its `claude-code` adapter. For another agent, it needs an adapter for the transcript format of that agent ([transcript.md](transcript.md#adapters)).

Use the same model and effort in both arms. Run the control with a `PATH` that has no `spec-dna`. Deny the session access to other projects, so that it reads only its own copy.
