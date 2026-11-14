# spec-dna agent team

This folder defines 14 Claude Code agents, with one file for each agent. An agent file gives the scope of the agent, its invariants, its limits, and the agents that it delegates to. It links to the docs and does not copy them. This README and `ownership.md` apply to all agents.

- [ownership.md](ownership.md) gives the responsible agent of each task and each path. Read it before you delegate.
- `CLAUDE.md` tells the main session when to use an agent, the advisor or no agent.
- `AGENTS.md` gives the rules that every agent follows: rule zero, the checks before a commit, and a pointer to the rules for prose in `CONTRIBUTING.md`. Each agent loads `AGENTS.md` through `CLAUDE.md`, except `Explore`.

## Pick an agent

| You have | Use |
| --- | --- |
| A small task that fits in this session | No agent. Do it in the main session. |
| A change to a known path | The responsible agent in [Quick lookup by path](ownership.md#quick-lookup-by-path) |
| A change to a contract, or a change to 2 or more modules | `architect` |
| A failure with no known cause | `failure-analyst`, before any fix |
| A change to parse, the model, config, hashes, the cache, git access, `new` or the CLI | `core-engineer` |
| A new or changed diagnostic, or a change to the gate | `diagnostics-engineer` |
| A change to what `context` or `show` serves | `context-engineer` |
| A change that moves, splits or generates document text | `hygiene-engineer` |
| A change to `init`, the adapters or the skill text | `init-engineer` |
| A change to the transcript analysis | `transcript-engineer` |
| A test design, a slow or unstable test, or a speed measurement | `test-engineer` |
| Prose to bring to ASD-STE100 | `docs-editor` |
| A release, or a change to CI or the hooks | `release-engineer` |
| Finished work, before you say that it is done | `checker` |
| A wide search or a docs lookup | `Explore` |
| A decision point while the advisor tool is off or fails | `fallback-advisor` |

## Roster

### Decision

| Agent | Owns | Model | Effort | Edits files |
| --- | --- | :-: | :-: | :-: |
| [architect](architect.md) | The information model, ADRs, the contracts between modules, the conventions, the agent team | opus | xhigh | Docs and configs only |

### Engineering

| Agent | Owns | Model | Effort | Edits files |
| --- | --- | :-: | :-: | :-: |
| [core-engineer](core-engineer.md) | Parse, front matter, model, config and schema, hashes, cache, git access, JSON writer, templates, `new`, the CLI | sonnet | high | yes |
| [diagnostics-engineer](diagnostics-engineer.md) | `check`: controls, gating, drift, citations, evidence, lint, the SD codes | sonnet | high | yes |
| [context-engineer](context-engineer.md) | `context` and `show`: the section plan, budgets, the BM25 rank, the retrieval benchmark | sonnet | high | yes |
| [hygiene-engineer](hygiene-engineer.md) | `mv`, the split, `history`, `index`, `--stub` files, the preservation check | sonnet | high | yes |
| [init-engineer](init-engineer.md) | `init`: detection, the adapters, the skill text, the quickstart and the demo | sonnet | high | yes |
| [transcript-engineer](transcript-engineer.md) | `transcript`: formats, counts, the privacy rule | sonnet | high | yes |

### Quality

| Agent | Owns | Model | Effort | Edits files |
| --- | --- | :-: | :-: | :-: |
| [test-engineer](test-engineer.md) | The test strategy, the shared test code, golden and fixture policy, the tool benchmark | sonnet | high | yes |
| [failure-analyst](failure-analyst.md) | The proved cause of a failure and a small repro. Never fixes | opus | xhigh | no |
| [checker](checker.md) | An independent check of finished work. Never edits | opus | high | no |

### Docs and release

| Agent | Owns | Model | Effort | Edits files |
| --- | --- | :-: | :-: | :-: |
| [docs-editor](docs-editor.md) | The language of all prose: ASD-STE100, with no change of meaning | sonnet | high | yes |
| [release-engineer](release-engineer.md) | The version, `CHANGELOG.md`, cross-builds, workflows, hooks, Dependabot. Never pushes | sonnet | high | yes |

### Support

| Agent | Owns | Model | Effort | Edits files |
| --- | --- | :-: | :-: | :-: |
| [Explore](Explore.md) | A read-only search of the repository and the web. It replaces the built-in `Explore` | haiku | high | no |
| [fallback-advisor](fallback-advisor.md) | Advice at a decision point while the advisor tool is off or fails | opus | xhigh | no |

There is no product role. The owner decides what to build and when.

## Model team

`.claude/settings.json` sets the models of the main session and the advisor:

| Role | Model and effort | Source |
| --- | --- | --- |
| Main session: writes code and runs tests | Sonnet, `high` | `model` and `modelSettings` |
| Advisor: gives advice and writes no code | Fable, no effort setting | `advisorModel` |
| A subagent without a `model` field, for example `general-purpose` | Haiku | `CLAUDE_CODE_SUBAGENT_MODEL` in `env` |

Each agent file in this folder sets the `model` and the `effort` of its agent. Exceptions:

- The built-in `Plan` subagent uses the model of the main session. `CLAUDE_CODE_SUBAGENT_MODEL` does not change it.
- A `model` parameter in an Agent tool call overrides the `model` field of an agent file. Do not pass a `model` parameter to an agent of this folder. Give `sonnet` only to a subagent without a `model` field that must write code.
- Subagents also get the advisor tool. Each agent can call Fable, and each call costs tokens.

### Fallbacks

- `fallbackModel` is `["opus", "sonnet"]`. When a model is overloaded or not available, Claude Code tries Opus and then Sonnet for that turn. Subagents use the same chain.
- Billing, authentication and rate-limit errors do not start a fallback.
- No setting moves the advisor to a different model. When the advisor tool is not available or fails, the main session briefs `fallback-advisor`.
- On some plans, Fable usage bills to usage credits and needs a one-time consent. Until you give it, the session has no advisor and shows a notice. To give consent, run `/model fable`, accept, and then run `/model sonnet`.

### What overrides these settings

- Managed settings and an `availableModels` allowlist override all settings files.
- For one session, these override the project values: `--model`, `--advisor`, `--effort`, `--fallback-model`, `ANTHROPIC_MODEL` and `CLAUDE_CODE_EFFORT_LEVEL`.
- A session that you resume with `--resume` or `--continue` keeps the model of its transcript.
- `/model`, `/advisor` and `/effort` save a choice in your user settings. The project file overrides the user file, so the project values apply again in the next session. If a launcher sets `CLAUDE_CONFIG_DIR`, your user settings file is in that directory.
- The advisor needs the Anthropic API or Claude Platform on AWS. It also needs feature flags from Anthropic. `DISABLE_TELEMETRY`, `DO_NOT_TRACK`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` and `CLAUDE_CODE_DISABLE_FEATURE_FLAGS` stop the fetch of these flags. `CLAUDE_CODE_DISABLE_ADVISOR_TOOL` removes the advisor.

A session that runs reloads `env`, `advisorModel`, `fallbackModel` and the files in this folder. A change to `model` or `modelSettings` applies after a restart.

### Check the team

1. Start a new session. Do not resume an old session. The header shows Sonnet and names `.claude/settings.json`.
2. Run `/effort status`. It shows `high`.
3. Find the notice `Advisor Tool (experimental) is on`. Run `/advisor`. It shows Fable.
4. Type `@`. The list shows the 14 agents of the roster. Since v2.1.198, `/agents` prints only a reminder.
5. While a subagent runs, run `/tasks`. The row of the subagent shows its model.
6. When Claude calls the advisor, an `Advising` line shows. Press `Ctrl+O` to read the advice. Run `/usage` to see the cost.

## Rules for delegation

- Delegate a task only if it needs more context than this session can hold, needs an independent view, or can run in parallel.
- Only one agent edits a given file at a given time.
- Give a brief. State the goal, the files and the change sets of the task. Also state the constraints and the check that proves the result. Do not give the history of the session.
- Check the diff and the command output. Do not accept the summary of an agent as proof.
- Do not limit the findings of a reviewer.
- Remove each temporary file that you made.
- Other sessions can work in this checkout. Stage only the files that you changed. Never discard a change that you did not make.

## Change the team

The owner accepts each change of the team. `architect` makes the change:

1. Edit or add the agent file. Set `name` to the file name without `.md`.
2. Write `description` on one line. Do not put a colon and a space in it. A strict YAML parser refuses that text.
3. Set `model`, `effort` and `tools`. Do not set `effort` to `max`. Do not list `Agent`, `Skill` or `ToolSearch`.
4. Update "Pick an agent" and the roster in this file.
5. Update the roles, the rows and the quick lookup in [ownership.md](ownership.md).
6. Run `pre-commit run --files` with the changed files.
7. Start a new session and type `@`. Make sure that the list shows each agent of the roster.

`claude plugin validate .claude/agents` is not a check of this folder. In Claude Code v2.1.323 it reads no file of a project `agents` folder, and it reports a pass.
