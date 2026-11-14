# Agent harness

This page is the reference of the Claude Code adapter of the harness. The harness is a team of models, 4 stage skills, a review policy and 3 hooks. It is for people who configure or check the files that `spec-dna init` writes for Claude Code. [Agents](agents.md) describes the other agents.

One model builds. A stronger model advises. A verifier checks finished work. A small model searches. The stage skills carry the steps of a change ([change guide](guides/change.md)). `spec-dna check` reports a generated file that differs from the configuration.

The decisions are [ADR 0004](adr/0004-harness.md) and [ADR 0008](adr/0008-harness-and-evals.md).

The harness is optional. `init` asks for a level.

| Level | What `init` writes |
| --- | --- |
| `none` | Nothing |
| `lean` | The main model and its effort, the advisor, `verifier`, `Explore`, `fallback-advisor`, the default model of other subagents, the stage skills, `REVIEW.md`, 3 hooks, a model-team line in `CLAUDE.md` and a page for people |
| `full` | `lean`, and `implementer` and `code-simplifier` |

A new configuration gets `lean` by default. A configuration from spec-dna 0.4 or older has no `[harness]` table, so an upgrade adds no harness. Run `spec-dna init --reconfigure` to add it.

## Roles

| Role | Purpose | Default model, effort | Claude Code file | Levels |
| --- | --- | --- | --- | --- |
| `orchestrator` | The main session. It builds, delegates and runs checks | `opus`, `xhigh`; fallback `opus`, `sonnet` | `.claude/settings.json` | `lean`, `full` |
| `advisor` | Advice at a decision point. It writes no code | `fable` | `advisorModel` in `.claude/settings.json` | `lean`, `full` |
| | The advisor when the advisor tool is not available | `opus`, `xhigh` | `.claude/agents/fallback-advisor.md` | `lean`, `full` |
| `verifier` | Runs the work and reports defects. It never fixes | `opus`, `high` | `.claude/agents/verifier.md` | `lean`, `full` |
| `researcher` | Read-only search of the repository and the web | `haiku`, `high` | `.claude/agents/Explore.md` | `lean`, `full` |
| `code-simplifier` | Removes needless complexity from the diff and keeps the behavior | `sonnet`, `high` | `.claude/agents/code-simplifier.md` | `full` |
| `implementer` | Code and tests for one delegated task | `sonnet`, `high` | `.claude/agents/implementer.md` | `full` |
| `helper` | Each subagent without a model of its own, such as `general-purpose` | `haiku` | `env.CLAUDE_CODE_SUBAGENT_MODEL` | `lean`, `full` |

Notes:

- `Explore.md` replaces the built-in Explore subagent of Claude Code. The built-in Explore uses the model of the main session. The replacement uses `haiku`, has 6 tools and does not load `CLAUDE.md`, so its first request is small.
- The `verifier` file has the tools `Bash` and `Read`. Its text says "Do not fix anything; report only."
- The model of a role is a default. An Agent tool call with a `model` parameter overrides it.
- The `helper` default is `haiku`. The `CLAUDE.md` line tells the main session to give `sonnet` or `opus` to a subagent that writes code or verifies work.
- 0.5 had the role `reviewer` (file `checker.md`) and the role `spec-writer`. [Upgrade from 0.5](#upgrade-from-05) gives the rename.

## Stage skills

Each skill is a folder `.claude/skills/<name>/SKILL.md`. spec-dna generates the whole file, with the harness marker. A stage skill runs spec-dna commands for its steps.

| Skill | Stage | What it does |
| --- | --- | --- |
| `spec-dna-intent` | Plan | Writes `intent.md` from a request, then runs `spec-dna next` |
| `spec-dna-spec` | Design | Writes `spec.md`: the delta with hashes from `spec-dna show`, the interfaces, the decisions and the concerns |
| `spec-dna-plan` | Build | Writes `plan.md` in plan mode: the files, the order, the risks and the proof |
| `spec-dna-review` | Deploy | Reviews the diff against `spec.md` and `plan.md`, with the passes and the severities of `REVIEW.md` |

A description is part of each request, so each skill has a short description. The key `[harness] skills` decides which skills spec-dna writes. `init` proposes all 4 when `[change] policy` is not `none`, and `[]` when it is `none`.

The adapter skill `.claude/skills/spec-dna/SKILL.md` is separate. It has its own marker and exists with or without a harness ([init.md](init.md)).

## `REVIEW.md`

With `review-md = true`, spec-dna generates `REVIEW.md` at the root of the repository. The harness marker is its last line. The text has these parts:

- 3 passes: Bugs, Security and Compliance. Compliance compares the change with `spec.md`, `plan.md` and the documents of `rules/`. A project puts its own policy in `rules/`.
- 2 severities: Important and Nit. Important is for a finding that breaks behavior, leaks data or breaches a policy.
- At most 5 Nits for each review.
- A `Do not report` list. It names the findings of `spec-dna check` and the generated files.

The Claude Code Code Review service reads `REVIEW.md` from the base branch or the default branch. A change of the file applies after the merge, and the repository must protect that branch. The local `/code-review` command does not read `REVIEW.md`. Thus the skill `spec-dna-review` has the same passes for local use. spec-dna writes both from one source text, so they agree in each version.

The `finish` pull request deletes `spec.md` and `plan.md`. The deleted text is in the diff, so the Compliance pass can read it.

Claude Code reads `CLAUDE.md` for each task, but it does not read `REVIEW.md`. Thus `REVIEW.md` adds no bytes to a request.

## Configuration

```toml
[harness]
level = "lean"
deny = ["NotebookEdit"]
skills = ["intent", "spec", "plan", "review"]
review-md = true
protect-tests = "fix-branches"
context-meter = [300000, 600000]
subagent-meter = 250000
fanout-cap = 10

[harness.roles]
orchestrator = { claude-code = "opus", effort = "xhigh", fallback = ["opus", "sonnet"] }
advisor = { claude-code = "fable", agent = "opus", effort = "xhigh" }
verifier = { claude-code = "opus", effort = "high" }
researcher = { claude-code = "haiku", effort = "high" }
helper = { claude-code = "haiku" }

[harness.efforts]
sonnet = "high"
```

| Key | Meaning |
| --- | --- |
| `level` | `none`, `lean` or `full` |
| `deny` | Tools that no agent of the project gets. A list that removes each tool of a generated agent is an error. `init` proposes `NotebookEdit` if git tracks no notebook. A deny rule removes the tool from each request, so it makes each request smaller |
| `skills` | The stage skills that spec-dna writes: `intent`, `spec`, `plan` and `review`. `[]` writes none |
| `review-md` | `true` writes `REVIEW.md`. `false` writes none |
| `protect-tests` | `fix-branches`, `always` or `off`. See [`spec-dna hook protect-tests`](#spec-dna-hook-protect-tests) |
| `context-meter` | The context sizes, in tokens, at which the main session gets a message. `[]` disables the meter for the main session |
| `subagent-meter` | The context size at which a subagent gets a message. `0` disables it |
| `fanout-cap` | The largest number of agents that a workflow can start without a question to the person. `0` disables the cap |
| `[harness.roles]` | The model (`claude-code`) and the effort of each role. `none` disables a role. `inherit` writes no model, so Claude Code uses its own default. The orchestrator also has `fallback`, the models that Claude Code tries when the main model is overloaded. The advisor also has `agent`, the model of `fallback-advisor` (`none`: no file) |
| `[harness.efforts]` | The effort of a model when a person selects it with `/model` |

A model is an alias (`fable`, `opus`, `sonnet`, `haiku`) or a full model id. The advisor model cannot be haiku. The fallback agent can. Efforts are `low`, `medium`, `high`, `xhigh`, `max` and `inherit`. The main session cannot use `max`. The role table keeps the key `claude-code`, so a later adapter can add its own key.

`init` writes an effort as `modelSettings.<model id>.effortLevel`, so it needs the model id of each alias. Each spec-dna release has a table of the aliases and their model ids. When Anthropic changes the model of an alias, a new spec-dna release updates the table. Until you install that release, `check` passes, and the effort applies to the old id only.

## Generated files

| File | How spec-dna writes it |
| --- | --- |
| `.claude/agents/<name>.md` | The whole file. The last line is a marker with the version and a hash: `<!-- spec-dna harness v<version> h:<hash> -->` |
| `.claude/agents/model-team.md` | The whole file, with the same marker. It has no front matter, so Claude Code does not load it as an agent. It tells people what the team is and how to check it |
| `.claude/skills/spec-dna-<stage>/SKILL.md` | The whole file, with the same marker |
| `REVIEW.md` | The whole file, with the same marker as its last line |
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
| `hooks.PreToolUse` (matcher `Edit\|Write`) | `spec-dna hook protect-tests ...` |

spec-dna writes the settings file again with a 2-space indent. A number form such as `1e3` becomes `1000`, and an integer above 2^53 loses precision.

A key that spec-dna writes into can have a value of the wrong type, for example `permissions.deny` as a string. Then `init` refuses and names the JSON path, and `check` reports SD505. spec-dna never deletes a key or a deny entry of the owner. If a new configuration no longer sets a key, `init` reports it. Remove the key manually.

spec-dna finds its hook handlers by the command prefix `spec-dna hook <name>`. It changes or removes only these handlers.

A file of your own in `.claude/agents` stays. If its agent name is the name of a role file, `init` disables that role by default. `init` never replaces your file. A file with no marker at the path of a stage skill or of `REVIEW.md` stays too. `init` removes the skill from `skills`, or sets `review-md = false`, and names the file.

## Hooks

Each hook is a subcommand of the spec-dna binary. A hook reads the event of Claude Code from stdin. It does not read `spec-dna.toml`, so the generated command contains the values. It never fails: an error gives no output and exit code 0. The command ends with `2>/dev/null || true`, so a teammate with an older spec-dna, or without spec-dna, gets no error and no hook.

Each teammate needs spec-dna on `PATH` for the hooks. `context-meter` and `fanout-cap` need version 0.5.0 or later. `protect-tests` needs version 0.6.0 or later. The command uses POSIX shell syntax. On Windows, Claude Code must run hooks in a POSIX shell, such as Git Bash.

### `spec-dna hook context-meter`

An agent cannot see the size of its context, and each request sends the whole context again. After each tool call, the hook reads the last token count of the transcript. When the context reaches a level for the first time, the hook adds a message to the context of the agent:

- The first level: delegate long searches and long reads to a subagent.
- A higher level: save the state of the task in a file, and tell the user that a new session can continue from it.
- The subagent level: complete the current step and return the findings to the caller.

The default levels (300,000 and 600,000 tokens) suit a model with a context window of 1,000,000 tokens. For a window of 200,000 tokens, set lower levels, for example `[100000, 150000]`.

The hook keeps one empty file for each level that it reported, in the temporary directory of the system. It stores no text. After a compaction, the context falls, and the hook can report the levels again.

### `spec-dna hook fanout-cap`

A workflow script can start many agents, and each agent uses its own context. Before a workflow runs, the hook computes an upper limit of the agents that the script can start. It multiplies each `agent(` call by the size of each loop around it. A loop over a list that the run makes has no limit. If the limit is above the cap, or if there is no limit, Claude Code asks the person before the run.

The upper limit is a text scan. It does not see a regex literal with a quote or a bracket. It does not see an alias of `agent`. Such a script runs without a question.

To change the cap for one session, start Claude Code with `SPEC_DNA_FANOUT_CAP=<n>`.

### `spec-dna hook protect-tests`

A fix must not change a test to make it pass. The hook runs on `PreToolUse` for `Edit` and `Write`. It asks the person before an edit of a test file that git tracks, when the branch name starts with `fix/`. The answer of the hook is `permissionDecision: "ask"` with a reason. A new test file does not trigger the hook, because a fix can add a test.

| `protect-tests` | Result |
| --- | --- |
| `fix-branches` | The hook asks on a branch that starts with `fix/` |
| `always` | The hook asks on each branch. Use it if the project has a fix prefix other than `fix/` |
| `off` | spec-dna writes no hook |

The generated command contains the test globs of `[evidence] tests` and the mode. The hook runs `git` only for a path that matches a test glob. An empty `[evidence] tests` list writes no hook.

The hook does not see an edit through `Bash`. In a headless run, Claude Code denies a call that needs approval. The eval check `tests_unchanged` proves the result ([evals.md](evals.md)).

## Check

`spec-dna check` reads the generated files when `[harness] level` is not `none`. It reports these findings:

| Code | Finding |
| --- | --- |
| SD504 | A generated file or the `CLAUDE.md` line is missing, or a file at a role path has no marker. This includes the stage skills and `REVIEW.md` |
| SD501 | A generated file or line was edited by hand, or has text after its marker. A file with a harness marker that the configuration does not generate and that you edited also gives SD501. Delete the file or the line. Then run `spec-dna init` |
| SD502 | `CLAUDE.md` has a second model-team line. A generated file, the line or a settings entry differs from what this version generates for the configuration. This includes a second `spec-dna hook <name>` handler with another command. An unedited file with a marker that the configuration does not generate also gives SD502, and `init --reconfigure` removes it. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill |
| SD505 | `.claude/settings.json` is not valid JSON, or a key that spec-dna writes into has a value of the wrong type |

As for the other adapters, only findings that are new against the base fail the gate. The check of the base uses the `[harness]` table of the base. Thus a change of the table, without a change of the files, gives new findings. To change the team, change `[harness]` and run `spec-dna init --reconfigure`. Do not edit the generated files.

## Evals

An eval tests the agent setup of a project: its instruction files, skills, agents and hooks. `spec-dna eval run` starts Claude Code in headless mode, and the checks of the eval decide the result. `init` question 14 writes the starter files. [evals.md](evals.md) gives the eval file, the runner, the cost cap and the CI workflow.

## Cost and requirements

- The main model sets the cost of each request. `opus` at `xhigh` costs more than `sonnet` at `high`.
- The advisor sends the whole context to the advisor model at each consult, at the price of that model. The default advisor (Fable) is an experimental feature. It needs the Anthropic API or Claude Platform on AWS, access to Fable and, on some plans, a one-time approval of billing to usage credits. To get access, run `/model fable` and approve. Then run `/model opus` to select the main model again. Without access, Claude Code shows a notice, and the main session briefs `fallback-advisor`.
- `verifier`, `Explore`, `implementer` and `code-simplifier` cost nothing until the main session uses them. Their descriptions are part of each request. The descriptions of the stage skills are part of each request too.
- The `CLAUDE.md` line is part of each request (about 200 tokens). `REVIEW.md` is not.
- `init` prints the bytes that the harness adds to each request of the main session.

These options make each request smaller. `init` does not write them, because they remove features that some teams use:

- Add large tools that the team does not use to `deny`, for example `Artifact`, `Workflow`, `SendFeedback` or `ScheduleWakeup`.
- Set `skillListingMaxDescChars` in `.claude/settings.json` to a small value, such as 400, if the team has many skills.

## Check the team after a restart

1. Start a new session. Do not resume an old session: it keeps the model of its transcript.
2. Read the header. It shows the main model.
3. Run `/advisor`. It shows the advisor model.
4. Type `@`. The list shows `verifier`, `Explore` and `fallback-advisor`.
5. While a subagent runs, run `/tasks`. Its row shows its model.
6. When Claude consults the advisor, an `Advising` line appears. Press `Ctrl+O` to read the advice. Run `/usage` to see the cost.

## Change, upgrade and removal

- Change: edit `[harness]`, or run `spec-dna init --reconfigure`. The preview shows each file before `init` writes it. `init --reconfigure` also offers the parts that a project does not have yet.
- Upgrade: install the new version. Run `spec-dna upgrade` to move the pin. Run `spec-dna init`. It writes the generated files that you did not edit and the settings entries that changed. It keeps a file that you edited, and `check` reports it.
- Removal: run `spec-dna init --reconfigure`. Choose `none`. `init` removes the generated agent files and skills that you did not edit, `REVIEW.md`, the `CLAUDE.md` line and the hook handlers. It keeps the other settings keys and tells you which ones it no longer sets.

### Upgrade from 0.5

A 0.5 configuration stops with exit code 3 under the 0.6 binary, because the pin differs. Run `spec-dna upgrade` first ([upgrade guide](guides/upgrade-0.5.md)). For the harness, `upgrade` does these steps:

| 0.5 | 0.6 |
| --- | --- |
| Role `reviewer` | Role `verifier`, with the same model and effort |
| File `checker.md` | File `verifier.md`. `upgrade` deletes `checker.md` if you did not edit it |
| Role `spec-writer` and file `spec-writer.md` | The role ends. The stage skills replace it. `upgrade` deletes the file if you did not edit it, and its report names the model of the role |
| A 0.5 project at level `full` | `code-simplifier = "none"`, so the project gets no new agent |
| New keys | `skills = []`, `review-md = false` and `protect-tests = "off"`. The team stays the 0.5 team under the new names |

The old keys `reviewer` and `spec-writer` in `[harness.roles]` give exit code 3. The message names `verifier`, or says that the role ended.

`upgrade` writes the 0.6 text of an unedited `model-team.md`, model-team line or agent file. It keeps an edited generated file, and `check` reports SD501. After the upgrade, `check` reports nothing for a deleted `checker.md` or `spec-writer.md`. To clear SD501 for an edited file of an old role, delete the file. To keep it as an agent file of your project, delete its marker line. The names `checker` and `spec-writer` belong to no 0.6 role.

`upgrade` never replaces a file that has no marker, at the path of a new part or of an agent with the name `verifier`. It sets the role to `none`, removes the skill from `skills`, or sets `review-md = false`, and it names the file. A generated agent has priority over a user or plugin agent of the same name.

Run `spec-dna init --reconfigure` to enable the new parts: the stage skills, `REVIEW.md`, `protect-tests` and `code-simplifier`.
