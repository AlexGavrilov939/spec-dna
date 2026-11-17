# Agent harness

This page is the reference of the Claude Code adapter of the harness: models, the 11 skills of the workflow pack, a review policy and up to 3 hooks. It is for people who configure or check the files that `spec-dna init` writes for Claude Code. [Agents](agents.md) describes the other agents.

One model builds. A stronger model advises. A verifier checks finished work. A small model searches. The skills of the workflow pack carry the steps of a change ([change guide](guides/change.md)). `spec-dna check` reports a generated file that differs from the configuration.

The decisions are [ADR 0004](adr/0004-harness.md), [ADR 0008](adr/0008-harness-and-evals.md), [ADR 0010](adr/0010-workflow-pack.md) and [ADR 0011](adr/0011-entry-points.md).

The harness is optional. `init` asks for a level.

| Level | What `init` writes |
| --- | --- |
| `none` | Nothing |
| `lean` | The main model and its effort, the advisor, `verifier`, `Explore`, `fallback-advisor`, the default model of other subagents, 2 hooks, a model-team line in `CLAUDE.md` and a page for people. The keys `skills`, `review-md` and `protect-tests` add the skills, `REVIEW.md` and a third hook. A new configuration sets all 3 keys. It proposes the 7 skills that are on by default |
| `full` | `lean`, and `implementer` and `code-simplifier`. It also proposes the 4 skills that are off by default: `execute`, `tdd`, `verify` and `debug` |

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
skills = ["brainstorm", "specify", "clarify", "write-plan", "review", "finish", "post-mortem"]
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

A key that the file does not have adds nothing: no skill, no `REVIEW.md` and no `protect-tests` hook. A new configuration from `spec-dna init` has all 3 keys. It has the 7 skills that are on by default if the change policy is not `none` (the level `full` has all 11), `review-md = true` and `protect-tests = "fix-branches"`. [init.md](init.md#parts-of-the-harness) gives the defaults.

| Key | Meaning |
| --- | --- |
| `level` | `none`, `lean` or `full` |
| `deny` | Tools that no agent of the project gets. A list that removes each tool of a generated agent is an error. `init` proposes `NotebookEdit` if git tracks no notebook. A deny rule removes the tool from each request, so it makes each request smaller |
| `skills` | The skills of the workflow pack that `init` writes. Each value is one of the 11 names, and each name is listed once. `[]` writes none. `init` writes them in the order of the table in [The workflow pack](#the-workflow-pack-and-reviewmd), also if the list has another order. The names `intent`, `spec` and `plan` of 0.6 give exit code 3 ([Upgrade from 0.6](#upgrade-from-06)) |
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

`init` stops with exit code 2 for a file with no marker at the path of a skill or of `REVIEW.md`. This applies when the configuration generates that path. Move the file, remove the skill from `skills`, or set `review-md = false`. For a new configuration, `init` leaves such a skill or `REVIEW.md` out of the defaults, and it names the file.

## The workflow pack and `REVIEW.md`

The workflow pack has 11 skills ([ADR 0010](adr/0010-workflow-pack.md)). A stage skill binds to a step that `spec-dna next` names. An event skill binds to an event: a failure or an incident. Each skill is a folder `.claude/skills/spec-dna-<name>/SKILL.md`. `spec-dna next` prints the bare name, for example `write-plan`.

| Skill | Kind and stage | Use | Artifact | Floor | Default |
| --- | --- | --- | --- | --- | --- |
| `brainstorm` | Stage `plan` | A request starts a change | `intent.md` with its class | SD306 | On |
| `specify` | Stage `design` | The intent is `accepted`, and the class is `higher-risk` | `spec.md` with the delta | SD914, SD920 | On |
| `clarify` | Stage `design` | `spec.md` has an open concern or question | The sections `Concerns` and `Open questions`, answered | SD919, SD922, SD925 | On |
| `write-plan` | Stage `build` | Before the code | `plan.md` with `Files` and small tasks | SD913, SD923, SD924 | On |
| `execute` | Stage `build` | The plan is `approved`, and a task is open | The code, the tests and the checked task | SD913, the hook `protect-tests` | Off |
| `tdd` | Stage `test` | A requirement has no evidence | A test with a `covers:` tag, red before the code | SD608, SD916 | Off |
| `review` | Stage `deploy` | Before a merge | The review of the diff, and the answer to review comments | None | On |
| `verify` | Stage `deploy` | Before the word "done" | The output of the `Proof` commands and of `check` | `finish` stops on SD915 and SD916 | Off |
| `finish` | Stage `closed` | The change is ready to apply | The result of `spec-dna finish` | `finish` | On |
| `debug` | Event | A test or a command fails, and the cause is not known | A reproduction, a regression test and one fix | None | Off |
| `post-mortem` | Event | An `incident` has an open follow-up | A new `intent.md` with `from:`, and an eval | SD918 | On |

The floor is a check that fails when the artifact is wrong. It does not check what the skill adds, such as the order of a test and its code. Where the floor is `None`, the text of the skill is the only control.

- The 7 skills that are on by default are the 2 that replace `spec-dna-intent` and `spec-dna-review` of 0.6, and 5 whose artifact a check reads. The 4 skills that are off have no check of what they add. `init` writes them only when `skills` lists them, and the level `full` proposes them.
- Each body has 5 parts in this order: when to use the skill, the commands, the artifact, the check, and the action when the check fails.
- The text of a skill is static. It holds no value of the configuration, so the file is the same in each project. The binary holds the text, so the pin fixes it.
- A skill never sets `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat, and that commit changes only the `status` line. The line of `AGENTS.md` has the same words. [The change guide](guides/change.md#choose-the-class) gives who approves each document.
- `execute` starts one fresh subagent for each task. Then 2 reviews follow: compliance with the task and `spec.md`, then quality. At the level `full`, `implementer` builds and `code-simplifier` runs the quality review. At the level `lean`, a general subagent builds and `verifier` runs both reviews.
- The adapter skill `.claude/skills/spec-dna/SKILL.md` is separate. It has its own marker and exists with or without a harness ([init.md](init.md)).

### Read the text of a skill

`spec-dna skill list` prints the 11 names with their kind, stage, state and default. `spec-dna skill show <name>` prints the body of one skill, with no front matter and no marker. It works for a skill that is off, and for an agent that has no skill folder. Such an agent runs `spec-dna skill show <name>` for the step that `spec-dna next` names. [commands.md](commands.md#skill) gives the output.

### Disciplines

5 rules apply to each task. Each rule is one line of 140 bytes or fewer, and it names the skill that holds the detail.

| Rule | Detail in |
| --- | --- |
| Test first: write a test that fails before the code | `tdd` |
| Cause first: find the cause of a failure before the fix. After 3 failed fixes, stop and ask | `debug` |
| Proof before done: run the proof and read its output before you say "done" | `verify` |
| Review is data: check each review comment against the code before you act on it | `review` |
| Isolated work: do each change in its own git worktree or branch | `execute` |

The adapter skill holds the 5 lines in one block of 700 bytes or fewer. A discipline works when its skill is off, because the line is the rule, and `spec-dna skill show <name>` prints the detail. The line of `AGENTS.md` and `CLAUDE.md` names the 5 rules for each other agent ([agents.md](agents.md)).

### Another workflow plugin

If the project also has a plugin such as Superpowers, 2 skills of the same stage compete.

- The description of each spec-dna skill starts with "In a spec-dna project".
- The adapter skill states that a spec-dna skill replaces a general skill of the same stage. For `write-plan`, it replaces `writing-plans`.
- If `enabledPlugins` of `.claude/settings.json` names a plugin with the word `superpowers`, `init` prints a note with these rules. `init` reads this file of the project only, and never a file of the home folder.
- `init` does not move a file that a plugin writes, such as `docs/superpowers/specs/`. Exclude that folder from the roots.

Nothing proves that Claude Code selects the spec-dna skill first. No eval has the plugin installed, so a project with the plugin can still get 2 answers.

### Budgets

- A description has 160 characters or fewer, and a body has 2,500 bytes or fewer.
- The listing of Claude Code (`- spec-dna-<name>: <description>` and a line feed for each skill) has 1,900 bytes or fewer for the 11 skills, and 1,250 bytes or fewer for the 7 skills that are on.
- The block of disciplines has 700 bytes or fewer. The line of `AGENTS.md`, with its marker, has 400 bytes or fewer.
- A test fixes each limit.

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
| SD504 | A generated file or the `CLAUDE.md` line is absent. A generated file is an agent file, a skill, `REVIEW.md` or the team page. A file with no marker at the path of a generated file also gives SD504, because spec-dna does not replace it. The message gives the ways to fix it: move the file, set the role of an agent file to `none`, remove the skill from `skills`, or set `review-md = false` |
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
- The `CLAUDE.md` line is part of each request (about 200 tokens). The line of `AGENTS.md` has 400 bytes or fewer, so it adds about 100 tokens.
- The descriptions of the skills are part of each request: 1,250 bytes or fewer for the 7 skills that are on, and 1,900 bytes or fewer for the 11 skills. The body of a skill loads only when Claude Code uses the skill. `REVIEW.md` adds nothing to a request.
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
- Upgrade: install the new version, and run `spec-dna upgrade`. It moves the pin. It writes the generated files that you did not edit and the settings entries that changed. It keeps a file that you edited, and `check` reports it.
- Removal: run `spec-dna init --reconfigure` and choose `none`. `init` removes the generated agent files, skill files and `REVIEW.md` that you did not edit, the `CLAUDE.md` line and the hook handlers. It keeps the other settings keys and tells you which ones it no longer sets.
- Fewer skills: `spec-dna init --reconfigure` also removes an unedited skill file or `REVIEW.md` that the configuration no longer generates. It keeps an edited file and names it. Plain `init` removes no file, except an unedited skill file of 0.6 for a name that ended (see [Upgrade from 0.6](#upgrade-from-06)).

### Upgrade from 0.5

A 0.5 configuration stops with exit code 3 under the 0.6 binary, because the pin differs. Run `spec-dna upgrade` first ([upgrade guide](guides/upgrade-0.5.md)). For the harness, `upgrade` does these steps:

| 0.5 | 0.6 |
| --- | --- |
| Role `reviewer` | Role `verifier`, with the same model and effort |
| File `checker.md` | File `verifier.md`. `upgrade` deletes `checker.md` if you did not edit it |
| Role `spec-writer` and file `spec-writer.md` | The role ends. The stage skills replace it. `upgrade` deletes the file if you did not edit it, and its report names the model of the role |
| A 0.5 project at level `full` | `code-simplifier = { claude-code = "none" }` in `[harness.roles]`, so the project gets no new agent |
| New keys | `skills = []`, `review-md = false` and `protect-tests = "off"`. The team stays the 0.5 team under the new names |

A table with the key `reviewer` or `spec-writer` gives exit code 3. The same word in the answer to question 13 also gives exit code 3. The message names `verifier`, or says that the role ended.

Rename the key or remove it, and run `spec-dna init --reconfigure`. The command writes `verifier.md` and removes an unedited `checker.md` and `spec-writer.md`. It keeps an edited file.

`upgrade` writes the 0.6 text of an unedited `model-team.md`, model-team line or agent file. It keeps an edited generated file, and `check` reports SD501. After the upgrade, `check` reports nothing for a deleted `checker.md` or `spec-writer.md`. The row SD501 of [Check](#check) gives the fix for an edited file of an old role. The names `checker` and `spec-writer` belong to no 0.6 role.

`upgrade` never replaces a file that has no marker, at the path of an agent file that the configuration generates. For a 0.5 project, `upgrade` sets the role to `none` and names the file. For a project that pins 0.6.0 or later, `upgrade` stops with exit code 1. The message says to rename the agent, or to set the role to `none` in `[harness.roles]`. A generated agent has priority over a user or plugin agent of the same name.

Run `spec-dna init --reconfigure` to enable the new parts: the skills, `REVIEW.md`, `protect-tests` and `code-simplifier`.

### Upgrade from 0.6

A 0.6 project that lists the skills `intent`, `spec` or `plan` in `[harness] skills` gets exit code 3 from each command under the 0.7 binary. The message names the new names and `spec-dna upgrade`. `spec-dna` keeps no alias, because agents copy the old name.

| 0.6 | 0.7 |
| --- | --- |
| Skill `intent` | `brainstorm` |
| Skill `spec` | `specify` and `clarify` |
| Skill `plan` | `write-plan` |
| Skill `review` | `review`, with a new text |

`spec-dna upgrade` renames these names in `[harness] skills` before it reads the file. It changes only the value of `skills`, not the type names `intent`, `spec` and `plan` in other keys. It puts the names in the order of the table, and it adds a name only once. A second run changes nothing. Only `upgrade` renames: `init` gives exit code 3 for an old name in `skills`, and exit code 2 for an old name in an answer, and its message names the new name. A list of many lines that the rename would change in length stays. Then `upgrade` stops with exit code 3 and prints the list that you write by hand.

`init` and `upgrade` remove the unedited file of each old skill: `.claude/skills/spec-dna-intent/SKILL.md`, `.claude/skills/spec-dna-spec/SKILL.md` and `.claude/skills/spec-dna-plan/SKILL.md`. They keep an edited file and name it, and `check` reports SD501. `spec-dna upgrade` writes the files of the new names that the list has. The 4 skills that are off stay off until `skills` lists them.
