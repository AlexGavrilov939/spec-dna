# Agent harness

This page is the reference of the Claude Code adapter of the harness: models, 4 stage skills, a review policy and up to 3 hooks. It is for people who configure or check the files that `spec-dna init` writes for Claude Code. [Agents](agents.md) describes the other agents.

One model builds. A stronger model advises. A verifier checks finished work. A small model searches. The stage skills carry the steps of a change ([change guide](guides/change.md)). `spec-dna check` reports a generated file that differs from the configuration.

The decisions are [ADR 0004](adr/0004-harness.md) and [ADR 0008](adr/0008-harness-and-evals.md).

The harness is optional. `init` asks for a level.

| Level | What `init` writes |
| --- | --- |
| `none` | Nothing |
| `lean` | The main model and its effort, the advisor, `verifier`, `Explore`, `fallback-advisor`, the default model of other subagents, 2 hooks, a model-team line in `CLAUDE.md` and a page for people. The keys `skills`, `review-md` and `protect-tests` add the stage skills, `REVIEW.md` and a third hook. A new configuration sets all 3 keys |
| `full` | `lean`, and `implementer` and `code-simplifier` |

A new configuration gets `lean` by default. A configuration from spec-dna 0.4 or older has no `[harness]` table, so an upgrade adds no harness. Run `spec-dna init --reconfigure` to add it.

## Roles

| Role | Purpose | Default model, effort | Claude Code file | Levels |
| --- | --- | --- | --- | --- |
| `orchestrator` | The main session. It builds, delegates and runs checks | `opus`, `xhigh`; fallback `opus`, `sonnet` | `.claude/settings.json` | `lean`, `full` |
| `advisor` | Advice at a decision point. It writes no code | `fable` | `advisorModel` in `.claude/settings.json` | `lean`, `full` |
| | The advisor when the advisor tool is not available | `opus`, `xhigh` | `.claude/agents/fallback-advisor.md` | `lean`, `full` |
| `verifier` | Runs the tests and checks on finished work and reports defects. It fixes nothing | `opus`, `high` | `.claude/agents/verifier.md` | `lean`, `full` |
| `researcher` | Read-only search of the repository and the web | `haiku`, `high` | `.claude/agents/Explore.md` | `lean`, `full` |
| `implementer` | Code and tests for one delegated task | `sonnet`, `high` | `.claude/agents/implementer.md` | `full` |
| `code-simplifier` | Removes needless complexity from the diff of a task and keeps the behavior | `sonnet`, `high` | `.claude/agents/code-simplifier.md` | `full` |
| `helper` | Each subagent without a model of its own, such as `general-purpose` | `haiku` | `env.CLAUDE_CODE_SUBAGENT_MODEL` | `lean`, `full` |

Notes:

- `.claude/agents/Explore.md` replaces the built-in `Explore` subagent of Claude Code, with its model, tools and text. This is on purpose: only an agent with the name `Explore` replaces the built-in one. `init` says this when it writes the file.
- The built-in `Explore` uses the model of the main session. The replacement uses `haiku`, has 6 tools and does not load `CLAUDE.md`, so its first request is small.
- To keep the built-in `Explore`, set `researcher = { claude-code = "none" }` in `[harness.roles]` and run `spec-dna init --reconfigure`. `init` removes the file if you did not edit it.
- To use an agent of your own with the name `Explore`, set the same value and write your file. Without it, `init` refuses to replace your file.
- The `verifier` file has the tools `Read` and `Bash`. Its text says "Do not fix anything; report only."
- The model of a role is a default. An Agent tool call with a `model` parameter overrides it.
- The `helper` default is `haiku`. The `CLAUDE.md` line tells the main session to give `sonnet` or `opus` to a subagent that writes code or reviews work.
- spec-dna 0.6 renames the role `reviewer` to `verifier`, and its file `checker.md` to `verifier.md`. The role `spec-writer` ended, and `code-simplifier` is new at level `full`. [Upgrade from 0.5](#upgrade-from-05) gives the steps.

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

A key that the file does not have adds nothing: no skill, no `REVIEW.md` and no `protect-tests` hook. A new configuration from `spec-dna init` has all 3 keys. It has the 4 skills if the change policy is not `none`, `review-md = true` and `protect-tests = "fix-branches"`. [init.md](init.md#parts-of-the-harness) gives the defaults.

| Key | Meaning |
| --- | --- |
| `level` | `none`, `lean` or `full` |
| `deny` | Tools that no agent of the project gets. A list that removes each tool of a generated agent is an error. `init` proposes `NotebookEdit` if git tracks no notebook. A deny rule removes the tool from each request, so it makes each request smaller |
| `skills` | The stage skills that `init` writes: `intent`, `spec`, `plan` and `review`, each once. `[]` writes none. `init` writes them in this order, also if the list has another order. See [Stage skills and `REVIEW.md`](#stage-skills-and-reviewmd) |
| `review-md` | `true` writes `REVIEW.md`. `false` writes no file |
| `protect-tests` | `fix-branches`, `always` or `off`. It selects the hook that asks before an edit of a test file. See [`spec-dna hook protect-tests`](#spec-dna-hook-protect-tests) |
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
| `.claude/skills/spec-dna-<skill>/SKILL.md` | The whole file, with the same marker, for each skill of `skills`. The adapter skill `.claude/skills/spec-dna/SKILL.md` has its own marker and is not part of the harness |
| `REVIEW.md` | The whole file, with the same marker as its last line, if `review-md` is `true` |
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
| `hooks.PreToolUse` (matcher `Edit\|Write`) | `spec-dna hook protect-tests ...`, only if `protect-tests` is not `off` and `[evidence] tests` is not empty |

spec-dna writes the settings file again with a 2-space indent. A number form such as `1e3` becomes `1000`, and an integer above 2^53 loses precision.

A key that spec-dna writes into can have a value of the wrong type, for example `permissions.deny` as a string. Then `init` refuses and names the JSON path, and `check` reports SD505. spec-dna never deletes a key or a deny entry of the owner. If a new configuration no longer sets a key, `init` reports it. Remove the key manually.

spec-dna finds its hook handlers by the command prefix `spec-dna hook <name>`. It changes or removes only these handlers.

A file of your own in `.claude/agents` stays. If its agent name is the name of a role file, `init` disables that role by default. `init` never replaces your file.

`init` stops with exit code 2 for a file with no marker at the path of a stage skill or of `REVIEW.md`. This applies when the configuration generates that path. Move the file, remove the skill from `skills`, or set `review-md = false`. For a new configuration, `init` leaves such a skill or `REVIEW.md` out of the defaults, and it names the file.

## Stage skills and `REVIEW.md`

The 4 stage skills help an agent through the stages of a change ([ADR 0008](adr/0008-harness-and-evals.md)). Each skill is a folder `.claude/skills/<name>/SKILL.md`. Each skill runs spec-dna commands for its steps.

| Skill | Use | What it does |
| --- | --- | --- |
| `spec-dna-intent` | A request starts a change | `spec-dna new intent <slug>`, then the class, then `spec-dna next` |
| `spec-dna-spec` | The intent is `accepted`, and the change is `higher-risk` | `spec-dna new spec <slug>`: `Delta` with the hash of each `CHANGE` and `REMOVE` line from `spec-dna show`, `Interfaces`, `Decisions`, `Concerns` and `Open questions` |
| `spec-dna-plan` | Before the code | `spec-dna new plan <slug>`, in plan mode if the session has it: `Files`, the tasks of `Order`, `Risks` and `Proof` |
| `spec-dna-review` | Before a merge | The diff against the base, `spec.md` and `plan.md`, then the review policy |

- The text of a skill is static. It holds no value of the configuration, so the file is the same in each project.
- A skill never sets `accepted`. It sets `approved` only on the plan of a `routine` change, which the agent approves ([change guide](guides/change.md#choose-the-class)). A person approves each other document in a pull request.
- A description is part of each request. Each description has at most 160 characters, and the 4 descriptions together have 588 bytes in the listing of Claude Code. The body has at most 2,500 bytes.
- The 4 names do not change the skill names that `spec-dna next` prints.
- The adapter skill `.claude/skills/spec-dna/SKILL.md` is separate. It has its own marker and exists with or without a harness ([init.md](init.md)).

`REVIEW.md` is the review policy for the Code Review service of Claude Code. With `review-md = true`, spec-dna generates it at the root of the repository. The text has these parts:

- 3 passes: Bugs, Security and Compliance. Compliance compares the diff with `spec.md`, `plan.md` and the documents of the layer `rules`. A project puts its own policy in `rules`.
- 2 severities: Important and Nit. Important is for a finding that breaks behavior, leaks data or breaches a policy.
- At most 5 Nits for each review.
- A list "Do not report". It names the findings of `spec-dna check`, which CI runs, and the files that spec-dna generates.

The service reads `REVIEW.md` from the base branch or the default branch. A change of the file applies after the merge, and the repository must protect that branch. The local `/code-review` command does not read the file. Thus `spec-dna-review` has the same passes for local use. spec-dna writes both from one text, so they agree in each version.

The `finish` pull request deletes `spec.md` and `plan.md`. The deleted text is in the diff, so the Compliance pass can read it.

Claude Code does not load `REVIEW.md` for a task, so the file adds no bytes to a request.

## Hooks

Each hook is a subcommand of the spec-dna binary. A hook reads the event of Claude Code from stdin. It does not read `spec-dna.toml`, so the generated command contains the values. It never fails: an error gives no output and exit code 0. The command ends with `2>/dev/null || true`, so a teammate with an older spec-dna, or without spec-dna, gets no error and no hook.

Each teammate needs spec-dna 0.5.0 or later on `PATH` for the hooks. The hook `protect-tests` needs 0.6.0 or later. The command uses POSIX shell syntax. On Windows, Claude Code must run hooks in a POSIX shell, such as Git Bash.

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

A fix must change the code, not the test that finds the defect. An agent can edit the test to make it pass. The hook runs on `PreToolUse`, before an `Edit` or `Write` call. It asks the person when the file is a test file that git tracks and the branch starts with `fix/`. The answer of the hook is `permissionDecision: "ask"` with a reason.

The hook does not ask for a new test file, because a fix can add a test. The person approves the edit only if the test is wrong.

Set `protect-tests` in `[harness]`:

| Value | The hook asks |
| --- | --- |
| `fix-branches` | On a branch that starts with `fix/` |
| `always` | On each branch. Use it if your fix branches have another prefix |
| `off` | Never. `init` writes no entry. A file with no `protect-tests` key has the same effect as `off`, and a new configuration has `fix-branches` |

The test files are the globs of `[evidence] tests`. The generated command has one `--test '<glob>'` for each glob and the mode. An empty `[evidence] tests` list writes no entry.

The hook runs `git` only for a path that matches a glob, and at most 2 times. The first call finds the branch. The second call finds whether git tracks the file. A path that is not below `CLAUDE_PROJECT_DIR` (else the `cwd` of the event) gets no question. An error gives no output and exit code 0, so the edit continues.

- In a run with `claude -p`, nobody can answer the question. Claude Code denies the edit, and the model reads the reason. This is also true if `--allowedTools` lists `Edit`, so the hook guards an eval ([evals.md](evals.md)).
- The hook does not see an edit through `Bash`, such as `sed -i`. The check `tests_unchanged` of an eval proves that no test changed.
- If the agent enters a worktree, `CLAUDE_PROJECT_DIR` stays at the main checkout, and only the `cwd` moves. A file of a worktree that the agent enters gets no question, also if the worktree is inside the main checkout. Start the session in the worktree to protect it.
- A change of `[evidence] tests` changes the command, and `check` reports SD502 for the entry until you run `spec-dna init`.
- To remove the entry, set `protect-tests = "off"` and run `spec-dna init --reconfigure`. Plain `init` removes no entry.

## Check

`spec-dna check` reads the generated files when `[harness] level` is not `none`. It reports these findings:

| Code | Finding |
| --- | --- |
| SD504 | A generated file or the `CLAUDE.md` line is absent. A generated file is an agent file, a stage skill, `REVIEW.md` or the team page. A file with no marker at the path of a generated file also gives SD504, because spec-dna does not replace it. The message gives the ways to fix it: move the file, set the role of an agent file to `none`, remove the skill from `skills`, or set `review-md = false` |
| SD501 | A generated file or line was edited by hand, or has text after its marker. Delete the file or the line and run `spec-dna init`. The same code covers a file with a marker that the configuration does not generate (an old `checker.md`, or a skill or `REVIEW.md` that you turned off) and that has an edit. Delete the file, or delete its marker line to keep it as a file of the project |
| SD502 | `CLAUDE.md` has a second model-team line. Delete it. A generated file, the line or a settings entry differs from what this version generates for the configuration. This includes a second `spec-dna hook <name>` handler with another command. A file with a marker that the configuration does not generate, and that has no edit, also gives SD502. Run `spec-dna init --reconfigure` to remove it. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill |
| SD505 | `.claude/settings.json` is not valid JSON, or a key that spec-dna writes into has a value of the wrong type |

As for the other adapters, only findings that are new against the base fail the gate. The check of the base uses the `[harness]` table of the base. Thus a change of the table, without a change of the files, gives new findings. To change the team, change `[harness]` and run `spec-dna init --reconfigure`. Do not edit the generated files.

## Evals

An eval tests the agent setup of a project: its instruction files, skills, agents and hooks. `spec-dna eval run` starts Claude Code in headless mode, and the checks of the eval decide the result. `init` question 14 writes the starter files. [evals.md](evals.md) gives the eval file, the runner, the cost cap and the CI workflow.

## Cost and requirements

- The main model sets the cost of each request. `opus` at `xhigh` costs more than `sonnet` at `high`.
- The advisor sends the whole context to the advisor model at each consult, at the price of that model. The default advisor (Fable) is an experimental feature. It needs the Anthropic API or Claude Platform on AWS, access to Fable and, on some plans, a one-time approval of billing to usage credits. To get access, run `/model fable` and approve. Then run `/model opus` to select the main model again. Without access, Claude Code shows a notice, and the main session briefs `fallback-advisor`.
- `verifier`, `Explore`, `implementer` and `code-simplifier` cost nothing until the main session uses them. Their descriptions are part of each request.
- The `CLAUDE.md` line is part of each request (about 200 tokens).
- The descriptions of the stage skills are part of each request: 588 bytes for the 4 skills. The body of a skill loads only when Claude Code uses the skill. `REVIEW.md` adds nothing to a request.
- `init` prints the bytes that the harness adds to each request, with the skill descriptions in the sum.

These options make each request smaller. `init` does not write them, because they remove features that some teams use:

- Add large tools that the team does not use to `deny`, for example `Artifact`, `Workflow`, `SendFeedback` or `ScheduleWakeup`.
- Set `skillListingMaxDescChars` in `.claude/settings.json` to a small value, such as 400, if the team has many skills.

## Check the team after a restart

1. Start a new session. Do not resume an old session: it keeps the model of its transcript.
2. Read the header. It shows the main model.
3. Run `/advisor`. It shows the advisor model.
4. Type `@`. The list shows `verifier`, `Explore` and `fallback-advisor`. If the configuration has skills, type `/`. The list shows the skills `spec-dna-...`.
5. While a subagent runs, run `/tasks`. Its row shows its model.
6. When Claude consults the advisor, an `Advising` line appears. Press `Ctrl+O` to read the advice. Run `/usage` to see the cost.

## Change, upgrade and removal

- Change: edit `[harness]`, or run `spec-dna init --reconfigure`. The preview shows each file before `init` writes it. `init --reconfigure` also offers the parts that a project does not have yet.
- Upgrade: install the new version, set the pin, and run `spec-dna init`. It writes the generated files that you did not edit and the settings entries that changed. It keeps a file that you edited, and `check` reports it.
- Removal: run `spec-dna init --reconfigure` and choose `none`. `init` removes the generated agent files, skill files and `REVIEW.md` that you did not edit, the `CLAUDE.md` line and the hook handlers. It keeps the other settings keys and tells you which ones it no longer sets.
- Fewer skills: `spec-dna init --reconfigure` also removes an unedited skill file or `REVIEW.md` that the configuration no longer generates. It keeps an edited file and names it. Plain `init` removes no file.

### Upgrade from 0.5

A 0.5 configuration stops with exit code 3 under the 0.6 binary, because the pin differs. Run `spec-dna upgrade` first ([upgrade guide](guides/upgrade-0.5.md)). For the harness, `upgrade` does these steps:

| 0.5 | 0.6 |
| --- | --- |
| Role `reviewer` | Role `verifier`, with the same model and effort |
| File `checker.md` | File `verifier.md`. `upgrade` deletes `checker.md` if you did not edit it |
| Role `spec-writer` and file `spec-writer.md` | The role ends. The stage skills replace it. `upgrade` deletes the file if you did not edit it, and its report names the model of the role |
| A 0.5 project at level `full` | `code-simplifier = "none"`, so the project gets no new agent |
| New keys | `skills = []`, `review-md = false` and `protect-tests = "off"`. The team stays the 0.5 team under the new names |

A table with the key `reviewer` or `spec-writer` gives exit code 3. The same word in the answer to question 13 also gives exit code 3. The message names `verifier`, or says that the role ended.

Rename the key or remove it, and run `spec-dna init --reconfigure`. The command writes `verifier.md` and removes an unedited `checker.md` and `spec-writer.md`. It keeps an edited file.

`upgrade` writes the 0.6 text of an unedited `model-team.md`, model-team line or agent file. It keeps an edited generated file, and `check` reports SD501. After the upgrade, `check` reports nothing for a deleted `checker.md` or `spec-writer.md`. The row SD501 of [Check](#check) gives the fix for an edited file of an old role. The names `checker` and `spec-writer` belong to no 0.6 role.

`upgrade` never replaces a file that has no marker, at the path of a new part or of an agent with the name `verifier`. It sets the role to `none`, removes the skill from `skills`, or sets `review-md = false`, and it names the file. A generated agent has priority over a user or plugin agent of the same name.

Run `spec-dna init --reconfigure` to enable the new parts: the stage skills, `REVIEW.md`, `protect-tests` and `code-simplifier`.
