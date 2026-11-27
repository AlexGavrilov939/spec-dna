# Agent harness

This page is the reference of the Claude Code adapter of the harness: models, the 11 skills of the workflow pack, a review policy and up to 7 hooks. It is for people who configure or check the files that `spec-dna init` writes for Claude Code. [Agents](agents.md) describes the other agents.

One model builds. A stronger model advises. A verifier checks finished work. A small model searches. The skills of the workflow pack carry the steps of a change ([change guide](guides/change.md)). `spec-dna check` reports a generated file that differs from the configuration.

The decisions are [ADR 0004](adr/0004-harness.md), [ADR 0008](adr/0008-harness-and-evals.md), [ADR 0010](adr/0010-workflow-pack.md) and [ADR 0011](adr/0011-entry-points.md).

The harness is optional. `init` asks for a level.

| Level | What `init` writes |
| --- | --- |
| `none` | Nothing |
| `lean` | The main model and its effort, `verifier`, `Explore`, the default model of other subagents, 2 hooks, a model-team line in `CLAUDE.md` and a page for people. With [the advisor](#the-advisor-of-the-team) on, also the advisor and `fallback-advisor`. The keys `skills`, `review-md` and `protect-tests` add the skills, `REVIEW.md` and a third hook. A new configuration sets all 3 keys. The keys `subagent-card` and `fix-limit` add 3 more hooks, and a new configuration of `standard` or `strict` turns them on. It proposes the skills that are on by default (`spec-dna skill list`; [ADR 0023](adr/0023-default-skills-from-the-release-table.md)) |
| `full` | `lean`, and `implementer` and `code-simplifier`. It proposes the 11 skills, also the skills that are off by default. The profile `lite` writes no agent `acceptor` at any level |

A new configuration gets `lean` by default. A configuration from spec-dna 0.4 or older has no `[harness]` table, so an upgrade adds no harness. Run `spec-dna init --reconfigure` to add it.

## Roles

| Role | Purpose | Default model, effort | Claude Code file | Levels |
| --- | --- | --- | --- | --- |
| `orchestrator` | The main session. It builds, delegates and runs checks | `opus`, `xhigh`; fallback `opus`, `sonnet` | `.claude/settings.json` | `lean`, `full` |
| `advisor` | Advice at a decision point. It writes no code | `fable`. A new project of `lite` or `standard` has `none` ([The advisor of the team](#the-advisor-of-the-team)) | `advisorModel` in `.claude/settings.json` | `lean`, `full` |
| | The advisor when the advisor tool is not available | `opus`, `xhigh` | `.claude/agents/fallback-advisor.md` | `lean`, `full` |
| `verifier` | Runs the tests and checks on finished work and reports defects. It fixes nothing | `opus`, `high` | `.claude/agents/verifier.md` | `lean`, `full` |
| `acceptor` | Tries to break a finished change from the brief of `spec-dna accept --brief`, and writes its acceptance record. It edits no code | `opus`, `high` | `.claude/agents/acceptor.md`, only with the skill `review` | `lean`, `full` |
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
- The `acceptor` file has the tools `Read`, `Grep`, `Glob`, `Bash` and `Write`. Its prompt is one line that `next` prints: `cd '<root>' && spec-dna accept <slug> --brief` (or the same with `--follow-up`). The agent runs it and reads its output as its brief. The text of the file gives these rules: start each shell call with that `cd '<root>' &&`, stop when the `cd` fails, put each test repository under the scratch folder, commit with the path of the record, write only `acceptance.toml` and fix no defect. `init` and `upgrade` write the file only with the skill `review`, where the acceptance is on, so the profile `lite` gets neither. Without `review` in `skills`, `[harness.roles]` has no row `acceptor`, and `init` writes none. The key `advisor` of that row sets one more rule of the file: see [The advisor of the acceptor](#the-advisor-of-the-acceptor).
- The model of a role is a default. An Agent tool call with a `model` parameter overrides it.
- The `helper` default is `haiku`. The `CLAUDE.md` line tells the main session to give `sonnet` or `opus` to a subagent that writes code or reviews work.
- spec-dna 0.6 renames the role `reviewer` to `verifier`, and its file `checker.md` to `verifier.md`. The role `spec-writer` ended, and `code-simplifier` is new at level `full`. [Upgrade from 0.5](#upgrade-from-05) gives the steps.

### The advisor of the team

A new project of `lite` or `standard` has no advisor. A new project of `strict` has it ([init.md](init.md#the-preset-of-a-profile)). To turn it on later, run `spec-dna init --reconfigure` and answer `advisor=fable` to question 13. A person can set `advisorModel` in the user settings. Thus a session of a project with no advisor can still have one.

If the role is on, the model-team line of `CLAUDE.md` has these sentences:

```text
Consult the advisor only in a change of class `higher-risk` (the `class` of its `intent.md`): before you choose an approach, before you say that a task is done, and when the same error occurs twice. In other work, do not consult it. This rule is text: no gate checks it. Each consult sends the whole context at the advisor price, so consult before the context grows large.
```

This is a rule of text: no hook and no check enforces it. Claude Code 2.1.330 runs no hook on the advisor tool ([The advisor of the acceptor](#the-advisor-of-the-acceptor)). Each subagent gets the advisor of its session. In `standard` and `strict`, the section "Advisor" of `model-team.md` says the same. In `lite`, that page keeps the text of 0.7.1, and the line of `CLAUDE.md` has the sentences above. A project that has the older line gets SD502 until `init` or `upgrade` writes the line again.

### The advisor of the acceptor

The key `advisor` of the entry `acceptor` decides if `acceptor.md` has a rule on the advisor. [config.md](config.md#the-key-advisor-of-the-entry-acceptor) gives the key, its default for each profile and its rules.

With `advisor = false`, the file has this line. It comes after "Write only `acceptance.toml`. Report each defect as a finding. Do not fix it." and before "Scratch folder:".

```text
Advisor: do not consult the advisor in this task. This rule has priority over each project instruction that tells you to consult the advisor.
```

The acceptor reads `CLAUDE.md`. With the role `advisor` on, the line of `CLAUDE.md` tells an agent to consult the advisor in a change of class `higher-risk`. The second sentence gives the line of the agent file the priority over it. With `advisor = true`, the file has no sentence on the advisor, and the acceptor follows the instructions of the project. The line depends on the key only, not on the role `advisor`.

The line is a soft rule. It asks the agent not to consult the advisor, and the agent can still do it. A subagent gets the advisor of its session, and Claude Code has no switch of the advisor for one subagent. A test with Claude Code 2.1.330, in a lab folder, gave these results:

- A `tools` list with no `advisor`, and `disallowedTools: advisor`, did not remove the tool.
- A `PreToolUse` hook with the matcher `advisor` never ran, because the advisor is a tool of the server.
- A sentence in the body of an agent file that forbids the call was obeyed in 1 run of 1. The run was a task of one line, in a folder with no `CLAUDE.md`.

No run tested a real change, or a context that has the sentence of `CLAUDE.md`. Thus the line is no proof that the acceptor makes no call.

Only the whole session can turn the advisor off: run `/advisor off`, or set `CLAUDE_CODE_DISABLE_ADVISOR_TOOL=1`. For a pass with no advisor, run the acceptor in a session of its own, with the prompt line that `spec-dna next` prints:

```sh
CLAUDE_CODE_DISABLE_ADVISOR_TOOL=1 claude --agent acceptor '<prompt>'
```

With `--agent`, rule D of the hook sees its calls ([`protect-approvals`](#spec-dna-hook-protect-approvals)). [evals.md](evals.md#the-measured-cost-of-one-acceptance-pass) gives the cost of one pass with and with no advisor, and the limits of that measure.

## Configuration

```toml
[harness]
level = "lean"
deny = ["NotebookEdit"]
skills = ["brainstorm", "specify", "clarify", "write-plan", "tdd", "review", "finish", "post-mortem"]
review-md = true
protect-tests = "fix-branches"
protect-approvals = true
subagent-card = true
fix-limit = 3
context-meter = [300000, 600000]
subagent-meter = 250000
fanout-cap = 10

[harness.roles]
orchestrator = { claude-code = "opus", effort = "xhigh", fallback = ["opus", "sonnet"] }
advisor = { claude-code = "fable", agent = "opus", effort = "xhigh" }
verifier = { claude-code = "opus", effort = "high" }
acceptor = { claude-code = "opus", effort = "high", advisor = false }
researcher = { claude-code = "haiku", effort = "high" }
helper = { claude-code = "haiku" }

[harness.efforts]
sonnet = "high"
```

A key that the file does not have adds nothing: no skill, no `REVIEW.md` and no `protect-tests` hook. A new configuration from `spec-dna init` has all 3 keys. It has the skills that are on by default (`spec-dna skill list`; [ADR 0023](adr/0023-default-skills-from-the-release-table.md)) if the change policy is not `none` (the level `full` has all 11), `review-md = true` and `protect-tests = "fix-branches"`. The profile `lite` has fewer skills, no `REVIEW.md` and `protect-tests = "off"`. [init.md](init.md#parts-of-the-harness) gives the defaults.

The keys `subagent-card` and `fix-limit` follow the same rule: a file with no key gets no hook of them. `init` writes the 2 keys in a new configuration only: `true` and `3` for `standard` and `strict`, and `false` and `0` for `lite`. `init --reconfigure` and `spec-dna upgrade` keep the keys of the file, and add none. In a new `lite` project, `spec-dna.toml` has the 2 lines, and each other generated file keeps its bytes. A key that you set to `true` in `lite` writes the hooks, because the key decides and not the profile.

| Key | Meaning |
| --- | --- |
| `level` | `none`, `lean` or `full` |
| `deny` | Tools that no agent of the project gets. A list that removes each tool of a generated agent is an error. `init` proposes `NotebookEdit` if git tracks no notebook. A deny rule removes the tool from each request, so it makes each request smaller |
| `skills` | The skills of the workflow pack that `init` writes. Each value is one of the 11 names, and each name is listed once. The name `accept` is no skill, and it gives exit code 3. `[]` writes none. `init` writes them in the order of the table in [The workflow pack](#the-workflow-pack-and-reviewmd), also if the list has another order. The names `intent`, `spec` and `plan` of 0.6 give exit code 3 ([Upgrade from 0.6](#upgrade-from-06)) |
| `review-md` | `true` writes `REVIEW.md`. `false` writes no file |
| `protect-tests` | `fix-branches`, `always` or `off`. It selects the hook that asks before an edit of a test file. See [`spec-dna hook protect-tests`](#spec-dna-hook-protect-tests) |
| `protect-approvals` | `true` or `false`. `true` adds the hook that denies an approval by an agent. See [`spec-dna hook protect-approvals`](#spec-dna-hook-protect-approvals). `init` writes `true` for the profile `strict`. A file with no key has the same effect as `false` |
| `subagent-card` | `true` or `false`. `true` adds the hooks [`spec-dna hook subagent-card`](#spec-dna-hook-subagent-card) and [`spec-dna hook return-state`](#spec-dna-hook-return-state), and the section "Subagent card" of the team page. A file with no key has the same effect as `false` |
| `fix-limit` | A number of failed fixes. A number above `0` adds the hook [`spec-dna hook fix-limit`](#spec-dna-hook-fix-limit), and `0` adds no hook. A file with no key has the same effect as `0` |
| `context-meter` | The context sizes, in tokens, at which the main session gets a message. `[]` disables the meter for the main session |
| `subagent-meter` | The context size at which a subagent gets a message. `0` disables it |
| `fanout-cap` | The largest number of agents that a workflow can start without a question to the person. `0` disables the cap |
| `[harness.roles]` | The model (`claude-code`) and the effort of each role. `none` disables a role. `inherit` writes no model, so Claude Code uses its own default. The orchestrator also has `fallback`, the models that Claude Code tries when the main model is overloaded. The advisor also has `agent`, the model of `fallback-advisor` (`none`: no file). The acceptor also has `advisor`, a boolean ([The advisor of the acceptor](#the-advisor-of-the-acceptor)) |
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
| `hooks.PreToolUse` (matcher `Bash`) | `spec-dna hook protect-approvals ...`, only if `protect-approvals` is `true` and the project has a layer of the type `intent` |
| `hooks.SubagentStart` (matcher `.+`) | `spec-dna hook subagent-card`, only if `subagent-card` is `true` |
| `hooks.SubagentStop` (matcher `.+`) | `spec-dna hook return-state`, only if `subagent-card` is `true` |
| `hooks.PostToolUseFailure` (matcher `Bash`) | `spec-dna hook fix-limit --limit <n> ...`, only if `fix-limit` is above `0` |
| `hooks.PostToolUse` (matcher `Bash\|Edit\|Write`) | The same command as in `hooks.PostToolUseFailure` |

spec-dna writes the settings file again with a 2-space indent. A number form such as `1e3` becomes `1000`, and an integer above 2^53 loses precision.

A key that spec-dna writes into can have a value of the wrong type, for example `permissions.deny` as a string. Then `init` refuses and names the JSON path, and `check` reports SD505. spec-dna never deletes a key or a deny entry of the owner. If a new configuration no longer sets a key, `init` reports it. Remove the key manually.

spec-dna finds its hook handlers by the command prefix `spec-dna hook <name>`. It changes or removes only these handlers.

Each agent file ends with one shared paragraph, after its role text. It starts with `Scratch folder:` and has these rules:

- The rules cover temporary files only. They never limit an edit of the repository that the task of the role needs. A temporary file in the scratchpad is the one write that a read-only role (`verifier`, `Explore`, `fallback-advisor`) may make.
- Make your own new subfolder in the scratchpad folder that the session names. If the session names none, make a new folder with `mktemp -d`.
- Put each temporary file in that subfolder. Put there also each screenshot and each file of a browser tool. Never write them into the repository.
- Delete only the files that you put in the scratchpad, and the subfolder that you made. Never delete or change another file of the scratchpad. Never delete the scratchpad folder itself.

The scratchpad folder is shared with the parent session, so a subagent must not remove it. `init` does not change `.gitignore`. The paragraph is text only. The paragraph adds no bytes to the request of the main session, because that request holds only the descriptions of the agents.

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
| `tdd` | Stage `test` | A requirement has no evidence | A test with a `covers:` tag, red before the code | SD608, SD916 | On |
| `review` | Stage `deploy` | The tasks of a change are done, or a person asks for a review | The findings of the review, and `acceptance.toml` for a change that needs a record | SD927 to SD931, SD933, SD936 to SD940. `finish` stops on them | On |
| `verify` | Stage `deploy` | Before the word "done" | The output of the `Proof` commands and of `check` | `finish` stops on SD915 and SD916 | Off |
| `finish` | Stage `closed` | The change is ready to apply | The result of `spec-dna finish` | `finish` | On |
| `debug` | Event | A test or a command fails, and the cause is not known | A reproduction, a regression test and one fix | None | Off |
| `post-mortem` | Event | An `incident` has an open follow-up | A new `intent.md` with `from:`, and an eval | SD918 | On |

The floor is a check that fails when the artifact is wrong. It does not check what the skill adds, such as the order of a test and its code. Where the floor is `None`, the text of the skill is the only control.

- The Default column is the default of 0.8.0, from the table of the release ([ADR 0023](adr/0023-default-skills-from-the-release-table.md)). The 7 skills of the design (D2) are the 2 that replace `spec-dna-intent` and `spec-dna-review` of 0.6, and 5 whose artifact a check reads. The acceptance is part of the skill `review` ([ADR 0032](adr/0032-acceptance-in-one-pass.md)), so it has no skill of its own. A skill of D2 is on unless its verdict is `off`, and no verdict is `off` in 0.8.0. The 4 other skills have no check of what they add, so each needs the verdict `on`: only `tdd` has it. `execute`, `verify` and `debug` have no verdict `on`, so they are off by default. `init` writes a skill only when `skills` lists it, and the level `full` proposes the 11. `spec-dna skill show <name>` prints the text of a skill that is off.
- `tdd` gives one way for the commit of the red test: the subject `test: <what the test checks> (red)`, and a skip of only the hook that runs the tests, named in the body of the commit. The green commit runs all hooks. `write-plan` tells an agent to write no task for `spec-dna finish`. If the owner reviews the code before `finish`, the plan ends with the task `Review by the owner` (`Files: none`), which a person ticks after the review and before `spec-dna finish` runs. Only the review of the owner ends this task: `write-plan` and `execute` tell an agent never to tick it and never to write `Skip:` for it.
- Each body has 5 parts in this order: when to use the skill, the commands, the artifact, the check, and the action when the check fails.
- The text of a skill is static. It holds no value of the configuration, so the file is the same in each project of one profile. The binary holds the text, so the pin fixes it.
- The profile `lite` keeps the text of 0.7.1. Each generated skill file of a `lite` project has the bytes of 0.7.1 above the marker line, and `spec-dna skill show` prints that text. The skill `review` of `lite` has no step of the record, and `lite` writes no agent `acceptor`. The same holds for the agent files: a `lite` project gets the agent files of 0.7.1 at each level, with the body of `code-simplifier` of 0.7.1. The other profiles get the text of this release. A golden test fixes each text to the file of 0.7.1 ([ADR 0032](adr/0032-acceptance-in-one-pass.md)).
- A skill never sets `accepted` or `approved`. The skill `review` limits the first sentence to the `status` line of an artifact, because its acceptor sets `result = "accepted"` in the record. An agent writes an approval commit only when the owner says so explicitly in the chat, and that commit changes only the `status` line. A skill also never adds an entry to `acknowledged-by` of an acceptance record. The line of `AGENTS.md` has the first 2 sentences of the rule, with the same words, and not the third sentence. [The change guide](guides/change.md#choose-the-class) gives who approves each document.
- `review` tells the builder to run `spec-dna next <slug>` and to read the item `review`:
  - With a prompt, the builder starts one agent `acceptor` with that prompt (`cd '<root>' && spec-dna accept <slug> --brief`). It adds no word of its own.
  - With the prompt of a follow-up, the builder starts an acceptor with that prompt.
  - With a wait for the owner, the builder starts no third run.
  - With no prompt, the builder reviews the diff itself.
- The acceptor runs the prompt and works from its output only. It runs the `Proof`, makes the 3 passes of the review and tries to break the change. Then it writes the record and commits it alone, in a call of its own.
- For a `higher-risk` change in `strict`, the skill tells the builder to start the second acceptor in a new session with `claude --agent acceptor '<prompt>'`. Then rule D of the hook sees its calls. The gate cannot check a session, so this advice is no rule.
- A person can write the record by hand, also with the harness level `none`. A skill is no precondition of the gate ([ADR 0006](adr/0006-change-lifecycle.md)).
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
- The listing of Claude Code (`- spec-dna-<name>: <description>` and a line feed for each skill) has 1,900 bytes or fewer for the 11 skills. It has 1,250 bytes or fewer for the 7 skills of the design (D2).
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

Each teammate needs spec-dna 0.5.0 or later on `PATH` for the hooks. The hook `protect-tests` needs 0.6.0 or later, and the hook `protect-approvals` needs 0.7.0 or later. The hooks `subagent-card`, `return-state` and `fix-limit` need 0.8.1 or later. The command uses POSIX shell syntax. On Windows, Claude Code must run hooks in a POSIX shell, such as Git Bash.

A live check on Claude Code 2.1.332 tested some facts that the hooks `subagent-card`, `return-state` and `fix-limit` need. It used 2 headless sessions (`claude -p`) with the model `haiku`. Its hooks did not run spec-dna. They recorded names, the presence of keys and the length of a report, and they printed nothing. In the first session, the command `false` failed and the command `true` passed. The session also started an agent of the project and the built-in `general-purpose`. The live check recorded no tool call of these 2 agents. The second session started with `--agent <name>` and ran the command `true`. The live check showed nothing about the effect of an output of a hook.

A live test on the same version then ran the 3 hooks of spec-dna in headless sessions with the model `haiku`, with no `--agent` and with `--agent <name>`. The settings had the entries that `init` writes for these hooks, with the path of the build in place of the name `spec-dna`. The test removed the generated agent files, skills, team page and `REVIEW.md`. It also removed the file `CLAUDE.md`, which held only generated lines. So only a hook could give the card or the message of the fix limit. The sections of the 3 hooks say what the live check and the live test showed. Another version or mode of Claude Code can differ.

### `spec-dna hook context-meter`

An agent cannot see the size of its context, and each request sends the whole context again. After each tool call, the hook reads the last token count of the transcript. The count of a line is the sum of the 3 input token fields of the last iteration of type `message` in `usage.iterations` that has one of them. If the line has no such iteration, the count is the sum of the same fields of `usage`. The hook does not count a line of the model `<synthetic>`: it reads the line before it. When the context reaches a level for the first time, the hook adds a message to the context of the agent:

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

### `spec-dna hook protect-approvals`

Only a person approves a document. An agent that works in the repository commits with the e-mail and the key of the person, so `check` cannot tell its approval commit from a commit of the person. The default rule of each project is the sentence of the generated line of `AGENTS.md`: an agent writes an approval commit only when the owner says so explicitly in the chat. The hook makes the rule stricter. It runs on `PreToolUse` of `Bash`, and it denies the call.

Set `protect-approvals = true` in `[harness]` (the profile `strict` has it). A person then runs `spec-dna approve <slug> <artifact>` in a terminal of their own ([commands.md](commands.md#approve)). The hook denies these calls of an agent:

- a call that runs `spec-dna approve`, also on the word of the owner, and also inside `sh -c`;
- a call that writes the word `accepted` or `approved` and runs `git commit`. The hook cannot read the edit in that case, so the reason says to run the edit and the commit as 2 calls;
- a `git commit` while the working tree or the index holds an artifact of a change (`intent.md`, `spec.md` or `plan.md`) with its approval status (`accepted` for an intent, `approved` for a spec or a plan), and the old tree does not. The old tree is `HEAD`, or `HEAD~1` for `--amend`. A new file counts;
- a `git commit` while the working tree or the index holds the record `acceptance.toml` of a change folder with an entry in `acknowledged-by` that the old tree does not have ([commands.md](commands.md#accept)). An agent never acknowledges a record. Only a person does. The commit of `finish` passes, because it deletes the record of the folder and adds the same content under `.spec-dna/accepted/`.
- a `git commit` in a call of the agent `acceptor` (rule D, below);
- a `git commit` while the working tree or the index holds a new file under `.spec-dna/accepted/` whose content is not that of a record that the same commit deletes from a change folder (line ends CRLF and LF count the same, so `core.autocrlf` changes no result), and a `git commit` that changes a file under `.spec-dna/accepted/` that the old tree has. A delete of such a file passes, so a person can revert the commit of `finish`.

The test is that a status moves to the approval value, not that the commit changes only the status line. With `strict`, `approval` is `base`, and a commit that sets the status and edits the text becomes an approval at the merge. The hook has no exception for a routine plan: a person approves each artifact.

The command that `init` writes has one `--intent '<dir>'` for each folder of the `intent` layer, from the root of the repository, for example `docs/intent`. The hook reads no `spec-dna.toml`. It runs `git` only for a command whose `git` has the subcommand `commit` (`git log --grep commit` is no commit; options such as `-C <dir>` and `-c <name=value>` can come before the subcommand), and then 3 times, and 2 more times for each artifact file that the tree changes (3 more times for each record). Bad JSON, no git, a git error and a path that it cannot read give no output and exit code 0.

Rule D (design 0.8.0, D75 item 5) runs for a `Bash` call whose input has `agent_type = "acceptor"`. This is a subagent of that name, or a session that starts with `claude --agent acceptor`. A session that starts without `--agent` has no `agent_type`, so rule D does not run for it. The rule does not interpret a shell. It reads a call when the text of the call has the word `commit`, or the word `git` and a dynamic construct outside single quotes (`$`, a backtick, `*`, `?`, `[` or `{`). The scan ignores the case, and it removes a backslash before a line feed, then `'`, `"` and `\`, so `GIT`, `g''it`, `c\ommit` and `comm\<line feed>it` count. A word counts also inside a quote, a `bash -c` script, an `eval` or a here-document. A call with neither (for example `git log`, `git diff -- src/a.rs` or `git status`) is not read, and rule D allows it. Then the whole call that the hook reads must match one fixed grammar, and the hook denies each other call. The reason names the one allowed shape:

```text
call := [ cd <absolute path> && ] [ git [-C <path>] add <record> && ]
        git [-C <path>] commit <options> [<record>]
```

- Only `&&` joins the parts. A `;`, a line feed, `||`, `|`, `&`, `(`, `)`, `{`, `}`, `<`, `>`, `$`, a backtick, `#`, `~`, `*`, `?`, `[`, `]` or `\` outside a quoted word is denied, and so is a `$`, a backtick or a `\` inside a double-quoted word. A word in single quotes is plain text. The one use of a backslash that passes is the sequence `'\''` between 2 single-quoted parts: it puts a `'` in the word, and `spec-dna next` writes a root with a `'` so. So a message is one word, and it is never read as an option, a path or a pattern.
- A part that is not `cd`, `git add` or `git commit` is denied: another command (`pushd`, `bash`, `sh`, `eval`, `xargs`, `env`), a second `cd` or a `cd` that is not first, a `cd` option or a relative `cd`, any `git` option other than `-C <path>` (`--git-dir`, `--work-tree`, `-c`), another `git` verb (`git stage`, `git status`), and more than one `git add` or `git commit`.
- `git add` names the literal path of one `acceptance.toml` of a change folder (the `--intent` folders), with no option.
- `git commit` allows only the options `-m`, `--message`, `-s`, `--signoff`, `-n`, `--no-verify` and `--` (git accepts an abbreviated long option, so only the exact names pass), and at most one path: the literal path of the record. A glob, a pathspec with `:`, a folder or another file is denied.
- The index must not hold a path other than the record, and not two records. This applies also when the commit has no path argument. The hook reads the index at the time of the call.
- The `cd` folder and each `-C` folder must be literal paths (a glob character is denied, also in a quoted folder), exist and be in a git repository. The rule guards the repository of the project (the repository of the `cwd` of the event and of `CLAUDE_PROJECT_DIR`, with its linked worktrees). If the folder is in another repository, such as a scratch repository of the agent, rule D reads no part of the call there (the grammar still applies). A folder that is in no repository is denied.

A commit of the record alone, with its path, passes. For a new record, the call is `git add <path> && git commit -m '<text>' <path>`, because `git commit <path>` refuses a path that git does not track. The commit of the record is part of the task of the acceptor. It is no approval commit, so the rule of the status line does not apply to it.

Rule D reads the text of the command only, with this grammar. It denies what it cannot read. A command that is built only from variables or escapes, with neither the literal word `git` nor the literal word `commit` in the text, is not read: this is a residual limit, because the hook is a text filter and no proof. It does not see a commit in a script file, through an alias or a git hook, a commit of another tool, an index that `git update-index` or another command changes, a commit in another repository, or a write of a file. Nothing in spec-dna limits the folder that a shell call or the tool `Write` of the acceptor writes to. A script can write into another checkout. The `cd '<root>' &&` of the prompt lowers the risk, and so does the rule that each test repository goes under the scratch folder of the agent. Neither enforces the limit.

Without the key `protect-approvals`, no hook runs, and the text of the brief is the only rule. In CI, SD932 checks each record commit with `approval = "branch"`, also with no hook. With `base`, nothing in CI checks that a record commit is alone.

These limits apply:

- The hook reads the text of the command. It is not a shell parser. It does not see an edit and a commit in one call that has no word `accepted` or `approved` (a variable, a script file). It does not see git plumbing (`commit-tree`, `update-ref`), `cherry-pick`, `rebase`, `merge`, `am` or a git alias.
- The same scan can deny a call that only names the words, for example `grep "spec-dna approve" docs/` or a commit message with the line `spec-dna approve` in a heredoc. The agent then runs the call in another form.
- The hook expands `~`, `$PWD` and `$CLAUDE_PROJECT_DIR` (also with braces) at the start of the dir of a `cd` or `git -C`. It cannot read another dir with `$` or `~`. The hook then fails closed: it inspects the repository of the `cwd` of the event and of `CLAUDE_PROJECT_DIR` too, and it denies the call if one of them has a pending approval. A commit in a third repository, behind such a dir, passes.
- The word test of the second rule is on each word before `git commit` in the call. A file or a folder name with `accepted` or `approved`, such as `git add x-approved.md && git commit`, gives a deny. The path `.spec-dna/accepted/`, where `finish` writes the archive file of an acceptance record, is the one exception. The test ignores it, so `git add .spec-dna/accepted/<slug>.toml docs/intent && git commit` passes in one call. The state test of the first rule then checks the archive file against the record. For any other `git add` that names such a word, run the `git add` and the `git commit` as 2 calls. The acceptor commits `acceptance.toml` alone, in a call of its own after the write. This commit is part of its task and is no approval commit, so it needs no go-ahead of the owner. The skill `review` and the file `acceptor.md` say so.
- The hook reads at most 1 MiB of the event. A longer event gives no output (it fails open).
- It denies a commit that has nothing to do with an approval while the working tree holds a status that a person set by hand. The person commits that status, or runs `spec-dna approve`.
- It denies a move of a change folder whose artifacts are `accepted` or `approved`, because the new path has an approval status. The person makes the move.
- The command ends with `|| true`. A teammate with no spec-dna, or with spec-dna before 0.7.0, gets no hook.
- Only the Claude Code adapter has the hook. For another agent, the generated line of `AGENTS.md` is the rule.
- The Claude Code documentation does not say if a command that the person starts with a `!` prefix passes the hooks. The input of a hook does not tell a person from an agent. So run `spec-dna approve` in a terminal of your own. With the hook, `!spec-dna approve` can be denied.
- The hook guards against an agent that acts on its own initiative. It is not a security control. The code host stays the control: [ADR 0012](adr/0012-approval-in-the-branch.md) states the limit, and [ADR 0018](adr/0018-approval-by-a-person.md) holds the rules.

To remove the entry, set `protect-approvals = false` and run `spec-dna init --reconfigure`.

### `spec-dna hook subagent-card`

A prompt in free form lets a subagent grow the scope, commit and return a long text with no clear result. The hook runs on `SubagentStart` and adds one fixed card to the context of each subagent, as `additionalContext`. Thus the card also reaches a subagent with no agent file, and an agent file of the project. The agent file and the task brief have priority over the card, and its first line says so. The card has 788 bytes:

```text
Subagent card. Your agent file and your brief have priority over this card.
1. Do only the task of the brief. Change only the files of `Files`. If the task needs more, stop and report it.
2. Read the brief, then the files that it names. Run `spec-dna context <file>` before an edit. Search only for a fact that these do not give.
3. Do not commit, push or change the git state.
4. Report the result, the changed files, each command that you ran with its result, and the open issues.
5. For a rule that the project files do not state, add `Lessons:` before the last line, with one line for each: `- Target: <file>. Text: <sentence>. Evidence: <what happened>.`
6. End the report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.
```

For the agent `acceptor`, line 3 is ``3. Commit only `acceptance.toml`, and do not push.``, and the card has 791 bytes. The team page prints the card, so a pull request shows each change of it. The card adds nothing to the output of `spec-dna accept --brief`.

The hook reads `agent_type` of the input, and it gives no output for an empty or absent one. In the [live check](#hooks), the input of `SubagentStart` had the keys `agent_type` and `agent_id`. The event occurred for the agent of the project and for `general-purpose`, and `agent_type` was the name of the agent. A hook with the matcher `.+` ran for both. In the live test, a subagent copied a sentence and the form of a lesson line from the card. Its task, its agent file and the files of the project did not hold them, and the test recorded no tool call of that subagent. So the card reached its context. The hooks reference of Claude Code states that a `SubagentStart` hook cannot block, and that it can add `additionalContext`.

The hook cannot prove these things:

- That a subagent obeys the card. The card is text in its context.
- That an agent other than Claude Code gets the card. That agent gets it only when the main session pastes it into the task brief.
- That the main agent of a session with `--agent <name>` gets the card. In the live check and in the live test, such a session gave no `SubagentStart` and no `SubagentStop` event for its main agent. So do not rely on the card, or on `return-state`, for an agent that you start in this way, such as an acceptor in a session of its own.
- That it skips each internal agent of Claude Code. The live check and the live test saw no event of an internal agent, also not with `--agent`. Only the hooks reference states the `agent_type` of an internal agent. It is empty, which the matcher `.+` skips, or it is the name of the `--agent` of the session. So an internal agent of a session that starts with `--agent <name>` can get the card, and one block from `return-state`.

Each subagent gets 800 bytes or fewer in its context, and each of its requests sends them again from the cache. The request of the main session gets no byte.

### `spec-dna hook return-state`

The hook runs on `SubagentStop`. It reads the last line of the report that is not empty. A state line is that line, after the removal of white space and of `*` and `` ` `` at its 2 ends. It is the word `State:`, one space and one of the [4 return states](#return-states), with this case. If the line is no state line, the hook gives `decision: "block"` with this reason, and the subagent gets it as its next instruction:

```text
spec-dna return state: end your report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.
```

The hook blocks only when `stop_hook_active` is `false`. Thus it adds one request at most. It also gives no output for an empty `agent_type`, or for an input that it cannot read. It stores nothing and prints no text of the report.

In the [live check](#hooks), the input of `SubagentStop` had the keys `agent_type`, `agent_id`, `agent_transcript_path`, `last_assistant_message` and `stop_hook_active`. `last_assistant_message` was a text that was not empty. The event occurred for the agent of the project and for `general-purpose`, and a hook with the matcher `.+` ran for both.

In the live test, a subagent gave a report with no state line. At its first `SubagentStop`, `stop_hook_active` was the boolean `false`, and the hook blocked. The subagent then gave one more report, which ended with a state line. At the second `SubagentStop`, `stop_hook_active` was the boolean `true`, the hook printed nothing, and the subagent ended. So the block gave the subagent one more instruction.

The hooks reference of Claude Code states that with the tool `SubagentHandback`, the report is the `message` input of that tool. After such a call, the hook reads the `message` of the last call in the transcript of the subagent. It reads from the end of the file, 4 MiB at most. A subagent can give a later report in plain text. So if an assistant text follows the last call in the transcript, a state line at the end of `last_assistant_message` also passes. A final text with no state line does not replace the message of the call. The live check and the live test showed no call of `SubagentHandback`, so no live run confirmed this form of the transcript.

The hook cannot prove these things:

- That it blocks on each version of Claude Code. It blocks only when `stop_hook_active` is the boolean `false`. If a version sends another form, the hook never blocks, and it gives no message.
- That the second report has a state line. A subagent can give none again, and the hook then lets it end.
- That the state is true. A subagent can write `COMPLETE` for work that fails. The verifier and the acceptance record stay the controls.
- That one more request is useful. A subagent with a structured output, such as an agent of a workflow with a schema, can get it, because its last text has no state line.

`Explore`, `verifier`, `fallback-advisor` and `acceptor` keep the report form of their agent files, which have priority over the card. Line 6 of the card still asks each of them for a state line. A report with no state line costs one more request over the whole context of the subagent.

### `spec-dna hook fix-limit`

An agent can fix a failure by guesses, one after the other, and each try makes its context larger. The skill `debug` tells an agent to stop at 3 failed fixes. This hook counts them and speaks at the time of the failure.

- The same problem is the same shell command that fails again. Each run of white space in the command counts as one space.
- A failed fix is a failure of that command after the same agent made an edit since its last failure. The first failure is the problem and not a fix, so a red test run does not count.
- One edit counts for one failure only. A run with no edit between 2 failures counts nothing.
- An input of `PostToolUseFailure` with `is_interrupt` set to the boolean `true` is no failure, because an abort tells nothing about a fix: the hook changes no state, so the edit mark stays, and it gives no message.
- When the command passes, the hook deletes its count, and the file of its message.
- An edit below a folder of the layer `intent` is no fix. So the record of the acceptor and an edit of `plan.md` count nothing.
- The main session and each subagent have counts of their own.

At the limit, the hook adds this message to the context of the agent, once for each command:

```text
spec-dna fix limit: this command failed after 3 fixes. Stop the fixes of this problem. Tell the person the command, the cause that each fix assumed, the error now, and the next step that you propose. A subagent ends its report with `State: BLOCKED`.
```

The command is `spec-dna hook fix-limit --limit <n> --intent '<dir>'...`, with one `--intent` for each folder of the layer `intent`. It has 2 settings entries: `PostToolUseFailure` of `Bash` counts a failure, and `PostToolUse` of `Bash`, `Edit` and `Write` sees a pass or an edit. The hooks reference of Claude Code states that a `Bash` call with an exit code other than 0 gives `PostToolUseFailure` and no `PostToolUse`. It also states that the hook cannot block this event.

In the [live check](#hooks), the command `false` gave `PostToolUseFailure` and no `PostToolUse`. The command `true` gave `PostToolUse` and no `PostToolUseFailure`. Both inputs of the first session had `session_id`, `tool_name` and `tool_input.command`, and no `agent_id`. In the session with `--agent`, the input of `PostToolUse` had `agent_type` and no `agent_id`. So for that input, the hook uses the counts of the main session.

In the live test, one command failed again and again, with a `Write` call between 2 failures. The hook printed the message at the failure after the third `Write`, and not at the next failure. The last reply of the agent held 2 phrases that only the message has, so the message reached its context. Each of these failures had `is_interrupt` with the boolean `false`. In a subagent, the inputs of `PreToolUse` and `PostToolUse` of a `Bash` call had `agent_id` and `agent_type`.

The state is one file for each count, one for each edit mark and one for each message, in `<temporary directory>/spec-dna-fix/`. A count file holds a number, and the other files are empty. The file of a message keeps the message single when calls run at the same time. The name of a file has the session id and the agent. The name of a count also has the first 8 bytes of SHA-256 of the session id, a zero byte and the command. The files hold no text of a command. The session id salts the hash, but a person who has the session id can test a guess of a short command. The hook removes a file that is older than 3 days ([ADR 0004](adr/0004-harness.md)). It uses the folder only if it is a folder, and no symlink, that your user owns. No write follows a symlink.

The hook cannot prove these things:

- That it runs. In a shared temporary directory, another user can make the folder `spec-dna-fix` first. Then the hook counts nothing, and it gives no message.
- That the agent stops. The message is context, and the hook cannot block.
- That 2 subagents have 2 counts in Claude Code. The hook takes the agent from `agent_id` of a tool event. The live test saw `agent_id` on a call of a subagent that passed, but no call of a subagent failed. With no `agent_id`, the hook uses the counts of the main session.
- That an `Edit` call sets the edit mark in Claude Code. The live test used only `Write`.
- That an interrupted call has `is_interrupt` set to `true`. No live run gave such a failure.
- That it sees each fix. A fix through `Bash`, such as `sed -i`, sets no edit mark.
- That it sees each failure. A command in a pipe, such as `cargo test | tail`, has the exit code of the last command.
- That 2 commands are 2 problems. A changed command, such as `pytest -x` after `pytest`, starts a new count.
- That a project stops at 3 fixes in total. 3 fixes by 3 subagents in sequence count as 1 fix for each. The text rule of the skill `debug` covers this case.
- That one count is one problem. A command that runs the whole test suite counts different defects as one problem, until the command passes once.

## Task brief, return states and lessons

These 3 forms tie the work of a subagent to the main session. The skill `spec-dna-execute` of `standard` and `strict` has a short form of the task brief and of the actions. The skill of `lite` has none.

### Task brief

The task brief is the text that the main session writes for a subagent:

```text
Task: <the result, in one sentence>
Files: <the paths to change>
Out of scope: <the paths and the work to leave>
Read first: <the documents and the files>
Done when: <the command, and its expected result>
```

`spec-dna next <slug> --json` gives the next task of the plan in `next_task`. 3 lines of the task brief come from it.

| Line | What it tells the subagent | Source |
| --- | --- | --- |
| `Task` | The result that it must give | `next_task.text` |
| `Files` | The only paths that it changes | `next_task.files` |
| `Out of scope` | The paths and the work that it leaves | The main session |
| `Read first` | What it reads before an edit | The main session, with `spec-dna context <file>...` for the files of `Files` |
| `Done when` | The command that proves the task, and its expected result | `next_task.verify` |

The acceptor gets no task brief. Its prompt stays the line that `spec-dna next` prints, and the main session adds no word. An agent other than Claude Code gets the card only when the main session pastes [it](#spec-dna-hook-subagent-card) into the task brief.

### Return states

A subagent ends its report with one line: `State: <state>`.

| State | Meaning | The main session |
| --- | --- | --- |
| `COMPLETE` | The task is done, and the check of `Done when` passes | Verifies the work as usual |
| `COMPLETE_WITH_CONCERNS` | The task is done. The report lists each concern: a risk, a skipped check or a doubt | Reads each concern before the verification |
| `NEEDS_CONTEXT` | The subagent stopped, because it does not have a fact. The report names the fact | Gives the fact, then starts the task again |
| `BLOCKED` | The subagent cannot complete the task. For example, a check fails outside the scope, the agent host denies a tool, the [fix limit](#spec-dna-hook-fix-limit) occurs, or the task needs more scope | Does not send the same task brief again. Removes the cause, or asks the person |

A report with no state line gets the same review as each other report. A state is never an input of a gate: `spec-dna next` and the acceptance record stay the source of truth. An acceptor that commits a record with `result = "rejected"` ends with `State: COMPLETE`.

### Lessons

A lesson is one fact that a rule, an instruction file or a skill of the project does not state yet. Its form in a report:

```text
Lessons:
- Target: <file>. Text: <the sentence to add>. Evidence: <what happened>.
```

A subagent that has a lesson adds this part before its state line. At the end of a change, the main session lists the lessons of the change, with the lessons of its subagents, and puts them in the body of the pull request. After a success, `spec-dna finish` prints [a note](commands.md#finish) that asks for this list in `standard` and `strict`. Thus no lesson file is in the repository.

The person accepts or rejects each lesson in the chat. No lesson enters the repository before that. An agent never edits a target on its own initiative. An accepted lesson enters by the class of its target:

| Target | Example | How the lesson enters |
| --- | --- | --- |
| An instruction file or a skill that a person writes | `AGENTS.md`, the text of `CLAUDE.md` outside the generated lines, a skill of the project | The agent edits the file in a commit of its own, after the word of the person in the chat |
| A document with a status | A rule or a decision | The change process of the project: `spec-dna new intent <slug>` |
| A generated file | A file with a spec-dna marker line | No edit, because `check` reports SD501. The person sends the lesson to the maintainers of spec-dna |

The lessons cannot prove these things:

- That a lesson is true or general. The person decides.
- That an agent writes its lessons. The note and the card ask for them, and no check reads a report.
- That a lesson stays. A lesson that the person does not answer stays only in the pull request.

## Check

`spec-dna check` reads the generated files when `[harness] level` is not `none`. It reports these findings:

| Code | Finding |
| --- | --- |
| SD504 | A generated file or the `CLAUDE.md` line is absent. A generated file is an agent file, a skill, `REVIEW.md` or the team page. A file with no marker at the path of a generated file also gives SD504, because spec-dna does not replace it. The message gives the ways to fix it: move the file, set the role of an agent file to `none`, remove the skill from `skills`, or set `review-md = false` |
| SD501 | A generated file or line was edited by hand, or has text after its marker. Delete the file or the line and run `spec-dna init`. The same code covers a file with a marker that the configuration does not generate (an old `checker.md`, or a skill or `REVIEW.md` that you turned off) and that has an edit. Delete the file, or delete its marker line to keep it as a file of the project |
| SD502 | `CLAUDE.md` has a second model-team line. Delete it. A generated file, the line or a settings entry differs from what this version generates for the configuration. This includes a second `spec-dna hook <name>` handler with another command. A file with a marker that the configuration does not generate, and that has no edit, also gives SD502. Run `spec-dna init --reconfigure` to remove it. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill |
| SD505 | `.claude/settings.json` is not valid JSON, or a key that spec-dna writes into has a value of the wrong type |

The entries of the hooks `subagent-card`, `return-state` and `fix-limit` follow the same rules. An absent entry, a changed command, or a second handler of the name with another command gives SD502. The message names the event and the hook. `check` reports nothing for the entry of a key that is off.

As for the other adapters, only findings that are new against the base fail the gate. The check of the base uses the `[harness]` table of the base. Thus a change of the table, without a change of the files, gives new findings. To change the team, change `[harness]` and run `spec-dna init --reconfigure`. Do not edit the generated files.

## Evals

An eval tests the agent setup of a project: its instruction files, skills, agents and hooks. `spec-dna eval run` starts Claude Code in headless mode, and the checks of the eval decide the result. `init` question 14 writes the starter files. [evals.md](evals.md) gives the eval file, the runner, the cost cap and the CI workflow.

## Cost and requirements

- The main model sets the cost of each request. `opus` at `xhigh` costs more than `sonnet` at `high`.
- With [the advisor](#the-advisor-of-the-team) on, it sends the whole context to the advisor model at each consult, at the price of that model. The advisor model `fable` is an experimental feature. It needs the Anthropic API or Claude Platform on AWS, access to Fable and, on some plans, a one-time approval of billing to usage credits. To get access, run `/model fable` and approve. Then run `/model opus` to select the main model again. Without access, Claude Code shows a notice, and the main session briefs `fallback-advisor`.
- `verifier`, `acceptor`, `Explore`, `implementer` and `code-simplifier` cost nothing until the main session uses them. Their descriptions are part of each request. A run of the `acceptor` is the review of a change that needs a record. A run on a slice of this repository took 13 to 25 minutes and about 190,000 to 250,000 tokens, and a `routine` change in `standard` has none. [evals.md](evals.md#the-measured-cost-of-one-acceptance-pass) gives the measured cost of one pass and its limits.
- The `CLAUDE.md` line is part of each request. In a `standard` project at the level `lean`, the model-team line has about 500 bytes with no advisor and about 1,100 bytes with it. The line of `AGENTS.md` has 400 bytes or fewer, so it adds about 100 tokens.
- The descriptions of the skills are part of each request. They have 1,250 bytes or fewer for the 7 skills of the design (D2), and 1,900 bytes or fewer for the 11 skills. The body of a skill loads only when Claude Code uses the skill. `REVIEW.md` adds nothing to a request.
- `init` prints the bytes that the harness adds to each request, with the skill descriptions in the sum.

These options make each request smaller. `init` does not write them, because they remove features that some teams use:

- Add large tools that the team does not use to `deny`, for example `Artifact`, `Workflow`, `SendFeedback` or `ScheduleWakeup`.
- Set `skillListingMaxDescChars` in `.claude/settings.json` to a small value, such as 400, if the team has many skills.

## Check the team after a restart

1. Start a new session. Do not resume an old session: it keeps the model of its transcript.
2. Read the header. It shows the main model.
3. With the advisor on, run `/advisor`. It shows the advisor model.
4. Type `@`. The list shows `verifier` and `Explore`, and `fallback-advisor` with the advisor on. If the configuration has skills, type `/`. The list shows the skills `spec-dna-...`.
5. While a subagent runs, run `/tasks`. Its row shows its model.
6. With the advisor on, an `Advising` line appears when Claude consults it. Press `Ctrl+O` to read the advice.
7. Run `/usage` to see the cost.

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

`spec-dna upgrade` renames these names in `[harness] skills` before it reads the file. It changes only the value of `skills`, not the type names `intent`, `spec` and `plan` in other keys. It puts the names in the order of the table, and it adds a name only once. It adds a new name only if the name is on by default (`spec-dna skill list`; [ADR 0023](adr/0023-default-skills-from-the-release-table.md)). For each name that it leaves out, the report has a line with the name and the fix: add the name to `skills`. A valid name of the list, such as `review`, stays, also when it is off by default. A list that is not empty also gets each skill that is new in 0.7.0 (`tdd`, `execute`, `verify`, `finish`, `debug` and `post-mortem`) and that `init` proposes for the profile and the level of the project. 0.8.0 adds no skill name. The agent file `acceptor.md` comes with the skill `review`, and `lite` gets none. After `upgrade` alone, a project of 0.7.1 gets no new diagnostic from the skill or the agent. An empty list stays empty, and the report names each skill that `upgrade` adds. `upgrade` applies no other part of the preset of a profile: use `init --reconfigure` for the other parts. A second run changes nothing. Only `upgrade` renames: `init` gives exit code 3 for an old name in `skills`, and exit code 2 for an old name in an answer. Its message names the new name. A list of many lines that the rename would change in length stays. Then `upgrade` stops with exit code 3 and prints the list that you write by hand.

`init` and `upgrade` remove the unedited file of each old skill: `.claude/skills/spec-dna-intent/SKILL.md`, `.claude/skills/spec-dna-spec/SKILL.md` and `.claude/skills/spec-dna-plan/SKILL.md`. They keep an edited file and name it, and `check` reports SD501. `spec-dna upgrade` writes the files of the new names that the list has. A skill that is off by default stays off until `skills` lists it.
