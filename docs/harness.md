# Agent harness

The agent harness is a small team of models for a coding agent. One model builds. A stronger model advises. A reviewer checks finished work. A small model searches. `spec-dna init` writes the files of the team for Claude Code. `spec-dna check` reports a generated file that differs from the configuration. The decision is [ADR 0004](adr/0004-harness.md).

The harness is optional. `init` asks for a level.

| Level | What `init` writes |
| --- | --- |
| `none` | Nothing |
| `lean` | The main model and its effort, the advisor, `checker`, `Explore`, `fallback-advisor`, the default model of other subagents, 2 hooks, a model-team line in `CLAUDE.md` and a page for people |
| `full` | `lean`, and `implementer` and `spec-writer` |

A new configuration gets `lean` by default. A configuration from spec-dna 0.4 or older has no `[harness]` table, so an upgrade adds no harness. Run `spec-dna init --reconfigure` to add it.

## Roles

| Role | Purpose | Default model, effort | Claude Code file |
| --- | --- | --- | --- |
| `orchestrator` | The main session. It builds, delegates and runs checks | `opus`, `xhigh`; fallback `opus`, `sonnet` | `.claude/settings.json` |
| `advisor` | Advice at a decision point. It writes no code | `fable` | `advisorModel` in `.claude/settings.json` |
| | The advisor when the advisor tool is not available | `opus`, `xhigh` | `.claude/agents/fallback-advisor.md` |
| `reviewer` | Checks finished work and looks for defects. It never edits | `opus`, `high` | `.claude/agents/checker.md` |
| `researcher` | Read-only search of the repository and the web | `haiku`, `high` | `.claude/agents/Explore.md` |
| `implementer` (full) | Code and tests for one delegated task | `sonnet`, `high` | `.claude/agents/implementer.md` |
| `spec-writer` (full) | Specs with spec-dna | `sonnet`, `high` | `.claude/agents/spec-writer.md` |
| `helper` | Each subagent without a model of its own, such as `general-purpose` | `haiku` | `env.CLAUDE_CODE_SUBAGENT_MODEL` |

Notes:

- `Explore.md` replaces the built-in Explore subagent of Claude Code. The built-in Explore uses the model of the main session. The replacement uses `haiku`, has 6 tools and does not load `CLAUDE.md`, so its first request is small.
- The model of a role is a default. An Agent tool call with a `model` parameter overrides it.
- The `helper` default is `haiku`. The `CLAUDE.md` line tells the main session to give `sonnet` or `opus` to a subagent that writes code or reviews work.

## Configuration

```toml
[harness]
level = "lean"
deny = ["NotebookEdit"]
context-meter = [300000, 600000]
subagent-meter = 250000
fanout-cap = 10

[harness.roles]
orchestrator = { claude-code = "opus", effort = "xhigh", fallback = ["opus", "sonnet"] }
advisor = { claude-code = "fable", agent = "opus", effort = "xhigh" }
reviewer = { claude-code = "opus", effort = "high" }
researcher = { claude-code = "haiku", effort = "high" }
helper = { claude-code = "haiku" }

[harness.efforts]
sonnet = "high"
```

| Key | Meaning |
| --- | --- |
| `level` | `none`, `lean` or `full` |
| `deny` | Tools that no agent of the project gets. A list that removes each tool of a generated agent is an error. `init` proposes `NotebookEdit` if git tracks no notebook. A deny rule removes the tool from each request, so it makes each request smaller |
| `context-meter` | The context sizes, in tokens, at which the main session gets a message. `[]` disables the meter for the main session |
| `subagent-meter` | The context size at which a subagent gets a message. `0` disables it |
| `fanout-cap` | The largest number of agents that a workflow can start without a question to the person. `0` disables the cap |
| `[harness.roles]` | The model (`claude-code`) and the effort of each role. `none` disables a role. `inherit` writes no model, so Claude Code uses its own default. The orchestrator also has `fallback`, the models that Claude Code tries when the main model is overloaded. The advisor also has `agent`, the model of `fallback-advisor` (`none`: no file) |
| `[harness.efforts]` | The effort of a model when a person selects it with `/model` |

A model is an alias (`fable`, `opus`, `sonnet`, `haiku`) or a full model id. The advisor model cannot be haiku. The fallback agent can. Efforts are `low`, `medium`, `high`, `xhigh`, `max` and `inherit`. The main session cannot use `max`.

`init` writes an effort as `modelSettings.<model id>.effortLevel`, so it needs the model id of each alias. Each spec-dna release has a table of the aliases and their model ids. When Anthropic changes the model of an alias, a new spec-dna release updates the table. Until you install that release, `check` passes, and the effort applies to the old id only.

## Generated files

| File | How spec-dna writes it |
| --- | --- |
| `.claude/agents/<name>.md` | The whole file. The last line is a marker with the version and a hash: `<!-- spec-dna harness v<version> h:<hash> -->` |
| `.claude/agents/model-team.md` | The whole file, with the same marker. It has no front matter, so Claude Code does not load it as an agent. It tells people what the team is and how to check it |
| `CLAUDE.md` | One line: the model-team rule, with the same marker. It is the second spec-dna line in this file |
| `.claude/settings.json` | A merge. spec-dna sets its entries and keeps every other key |

Entries in `.claude/settings.json`:

| Path | Value |
| --- | --- |
| `model` | The orchestrator model |
| `modelSettings.<id>.effortLevel` | The orchestrator effort, and each entry of `[harness.efforts]` |
| `fallbackModel` | The orchestrator `fallback` |
| `advisorModel` | The advisor model |
| `env.CLAUDE_CODE_SUBAGENT_MODEL` | The helper model |
| `permissions.deny` | Each `deny` entry |
| `hooks.PostToolUse` | `spec-dna hook context-meter ...` |
| `hooks.PreToolUse` (matcher `Workflow`) | `spec-dna hook fanout-cap ...` |

spec-dna writes the settings file again with a 2-space indent, so a number form such as `1e3` becomes `1000`, and an integer above 2^53 loses precision. If a key that spec-dna writes into has a value of the wrong type, for example `permissions.deny` as a string, `init` refuses and names the JSON path, and `check` reports SD505. spec-dna never deletes a key or a deny entry of the owner. If a new configuration no longer sets a key, `init` reports it. Remove the key by hand. spec-dna finds its hook handlers by the command prefix `spec-dna hook <name>`. It changes or removes only these handlers.

A file of your own in `.claude/agents` stays. If its agent name is the name of a role file, `init` disables that role by default. `init` never replaces your file.

## Hooks

Each hook is a subcommand of the spec-dna binary. A hook reads the event of Claude Code from stdin. It does not read `spec-dna.toml`, so the generated command contains the values. It never fails: an error gives no output and exit code 0. The command ends with `2>/dev/null || true`, so a teammate with an older spec-dna, or without spec-dna, gets no error and no hook.

Each teammate needs spec-dna 0.5.0 or later on `PATH` for the hooks. The command uses POSIX shell syntax. On Windows, Claude Code must run hooks in a POSIX shell, such as Git Bash.

### `spec-dna hook context-meter`

An agent cannot see the size of its context, and each request sends the whole context again. After each tool call, the hook reads the last token count of the transcript. When the context reaches a level for the first time, the hook adds a message to the context of the agent:

- The first level: delegate long searches and long reads to a subagent.
- A higher level: save the state of the task in a file, and tell the user that a new session can continue from it.
- The subagent level: complete the current step and return the findings to the caller.

The default levels (300,000 and 600,000 tokens) suit a model with a context window of 1,000,000 tokens. For a window of 200,000 tokens, set lower levels, for example `[100000, 150000]`.

The hook keeps one empty file for each level that it reported, in the temporary directory of the system. It stores no text. After a compaction, the context falls, and the hook can report the levels again.

### `spec-dna hook fanout-cap`

A workflow script can start many agents, and each agent uses its own context. Before a workflow runs, the hook computes an upper limit of the agents that the script can start. It multiplies each `agent(` call by the size of each loop around it. A loop over a list that the run makes has no limit. If the limit is above the cap, or if there is no limit, Claude Code asks the person before the run. The bound is a text scan. It does not see a regex literal with a quote or a bracket, or an alias of `agent`, so such a script runs without a question.

To change the cap for one session, start Claude Code with `SPEC_DNA_FANOUT_CAP=<n>`.

## Check

`spec-dna check` reads the generated files when `[harness] level` is not `none`. It reports these findings:

| Code | Finding |
| --- | --- |
| SD504 | A generated file or the `CLAUDE.md` line is missing, or a file at a role path has no marker |
| SD501 | A generated file or line was edited by hand, or has text after its marker. Delete the file or the line and run `spec-dna init` |
| SD502 | A generated file, the line or a settings entry differs from what this version generates for the configuration. This includes a second `spec-dna hook <name>` handler with another command |
| SD505 | `.claude/settings.json` is not valid JSON, or a key that spec-dna writes into has a value of the wrong type |

As for the other adapters, only findings that are new against the base fail the gate. The check of the base uses the `[harness]` table of the base. Thus a change of the table, without a change of the files, gives new findings. To change the team, change `[harness]` and run `spec-dna init --reconfigure`. Do not edit the generated files.

## Cost and requirements

- The main model sets the cost of each request. `opus` at `xhigh` costs more than `sonnet` at `high`.
- The advisor sends the whole context to the advisor model at each consult, at the price of that model. The default advisor (Fable) is an experimental feature. It needs the Anthropic API or Claude Platform on AWS, access to Fable and, on some plans, a one-time approval of billing to usage credits. To get access, run `/model fable` and approve. Then run `/model opus` to select the main model again. Without access, Claude Code shows a notice, and the main session briefs `fallback-advisor`.
- `checker`, `Explore`, `implementer` and `spec-writer` cost nothing until the main session uses them. Their descriptions are part of each request.
- The `CLAUDE.md` line is part of each request (about 200 tokens).

These options make each request smaller. `init` does not write them, because they remove features that some teams use:

- Add large tools that the team does not use to `deny`, for example `Artifact`, `Workflow`, `SendFeedback` or `ScheduleWakeup`.
- Set `skillListingMaxDescChars` in `.claude/settings.json` to a small value, such as 400, if the team has many skills.

## Check the team after a restart

1. Start a new session. Do not resume an old session: it keeps the model of its transcript.
2. Read the header. It shows the main model.
3. Run `/advisor`. It shows the advisor model.
4. Type `@`. The list shows `checker`, `Explore` and `fallback-advisor`.
5. While a subagent runs, run `/tasks`. Its row shows its model.
6. When Claude consults the advisor, an `Advising` line shows. Press `Ctrl+O` to read the advice. Run `/usage` to see the cost.

## Change, upgrade and removal

- Change: edit `[harness]`, or run `spec-dna init --reconfigure`. The preview shows each file before `init` writes it.
- Upgrade: install the new version, set the pin, and run `spec-dna init`. It writes the generated files that you did not edit and the settings entries that changed. It keeps a file that you edited, and `check` reports it.
- Removal: run `spec-dna init --reconfigure` and choose `none`. `init` removes the generated agent files that you did not edit, the `CLAUDE.md` line and the hook handlers. It keeps the other settings keys and tells you which ones it no longer sets.
