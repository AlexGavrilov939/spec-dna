---
status: proposed
---

# 0008. Harness and evals for 0.6.0

## Context and Problem Statement

[ADR 0004](0004-harness.md) installs a team of models, hooks and a drift check for Claude Code. 0.6.0 adds a lifecycle for each change ([ADR 0006](0006-change-lifecycle.md)). The owner asks for a skill for each stage, a generated `REVIEW.md`, and a hook that protects the tests during a fix. The owner also asks for role names from the playbook and the Claude Code documentation, and for evals with a pass-rate gate. How can spec-dna add these parts and keep the constraints of ADR 0004?

Constraints:

- The harness must work in any repository, with any stack. It must not name or copy a consumer project.
- Claude Code is the only agent. The repository is the only source of truth, so the harness needs no service.
- A name comes from the playbook or the Claude Code documentation. Where they name nothing, a plain STE word applies that does not conflict with them.
- Hooks and the runner must not need a second runtime. spec-dna itself makes no network call, and keeps no prompt, command, tool or output text.
- A rename keeps no runtime alias ([ADR 0009](0009-upgrade-from-0-5.md)). A hand edit must stay. The default harness level at `init` stays `lean`.

Evidence:

- Earlier measurements on anonymized corpora showed that the gate gave the measured gain. Retrieval and the split of large documents did not. The owner keeps the data.
- Agents search the repository. They seldom follow a tree of links. Large files and process text in each request cost tokens and gave no gain. Prose rules did not change agent behavior, but hooks at the time of the problem did ([ADR 0004](0004-harness.md)).
- The [playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook) names `verifier.md` ("Do not fix anything; report only."), "a researcher" and "a code simplifier". It names no advisor. Claude Code names the "advisor tool", with the key `advisorModel` and the command `/advisor` ([advisor](https://code.claude.com/docs/en/advisor.md)).
- The playbook names `REVIEW.md` with the passes Bugs, Security and Compliance, and the severities Important and Nit. It names `agent-evals.yml`, which "runs non-interactively in CI on a schedule and on any change to CLAUDE.md, skills or hooks". It says: "Each production incident gets an eval".
- The built-in subagents are `Explore`, `Plan` and `general-purpose`. None has the name `verifier` or `code-simplifier`. A project agent has priority over a user or plugin agent of the same name ([subagents](https://code.claude.com/docs/en/sub-agents.md)).
- The Code Review service reads `REVIEW.md` from the base or default branch, which is protected. The local `/code-review` command does not read it. Both read `CLAUDE.md`. `REVIEW.md` can include a skill file, but Code Review ignores a Nit cap or an exclusion in a skill ([Code Review](https://code.claude.com/docs/en/code-review-customize.md)).
- A `PreToolUse` hook can block a call, or answer `allow`, `deny` or `ask`. In a `-p` run where nobody can answer, Claude Code denies the call, and Claude reads the reason ([hooks](https://code.claude.com/docs/en/hooks.md)). A hook decision does not bypass a deny rule or an ask rule. The page does not say whether an allow rule overrides the `ask` answer of a hook ([permissions](https://code.claude.com/docs/en/permissions.md)).
- `claude -p` has `--allowedTools`, `--output-format json` with `total_cost_usd`, `--max-turns`, `--max-budget-usd` and `--permission-mode`. In `dontAsk` mode, Claude Code denies each call that would prompt. The cost is an estimate of the client ([headless](https://code.claude.com/docs/en/headless.md)).
- `--bare` skips hooks, skills, `CLAUDE.md` and subagents. Without it, a `-p` run also loads `~/.claude`, and runs the project hooks with no trust prompt. The page says that `--bare` will become the default of `-p`.

## Considered Options

The open question is the eval runner. The other parts follow the pattern of ADR 0004, and "Rejected" gives their alternatives.

| Option | Runtime | Local run | Exposure of the key | Cost cap before the start | `check` reads the eval files |
| --- | --- | --- | --- | --- | --- |
| 1. No runner: each project writes its own eval scripts | Varies | Varies | Varies | None | No |
| 2. A runner script in Python or shell that spec-dna ships | Python or a shell beside the binary | Yes | Low | In the script | No |
| 3. A runner on the Agent SDK packages | Python or Node, and an SDK package | Yes | Low | In the code | No |
| 4. A third-party action (`claude-code-action`) with the checks in the workflow | A third-party action | No | The action holds the key and asks for a write scope | Action inputs | No |
| 5. `spec-dna eval run`: a subcommand that starts headless Claude Code and runs the checks | The spec-dna binary and `claude` | Yes | The key only in the job, never for a fork | `max-total-usd` | Yes (SD506) |

## Decision Outcome

Option 5, with the harness parts below. The tables are in the [design of 0.6.0](../design/0.6.0.md), sections [Harness](../design/0.6.0.md#harness), [Evals](../design/0.6.0.md#evals) and [Terminology](../design/0.6.0.md#terminology). This ADR gives the decisions and their reasons.

The default level at `init` stays `lean`. `lean` adds the stage skills, `REVIEW.md` and `protect-tests`. `full` adds `implementer` and `code-simplifier`. The evals are off by default. The role table keeps the key `claude-code`, so a later adapter can add its own key.

### Role names

- A role takes the name of the playbook or of the Claude Code documentation. A role that neither source names takes a plain STE word: `orchestrator`, `implementer` and `helper`.
- `verifier` replaces the role `reviewer`, and `verifier.md` replaces `checker.md`. Its text follows the [playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook): "Do not fix anything; report only."
- `researcher` ([playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook)) keeps the file `Explore.md`. Only an agent with the name `Explore` replaces the built-in subagent ([subagents](https://code.claude.com/docs/en/sub-agents.md)).
- `code-simplifier` ([playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook)) is new, at the level `full`. After the main session finishes, it removes needless complexity from the diff and keeps the behavior.
- `advisor` keeps its name. The playbook names no advisor, and the Claude Code name is "[advisor tool](https://code.claude.com/docs/en/advisor.md)".
- The role `spec-writer` ends. The stage skills give its steps at the time of use.
- `model-team.md` and `docs/harness.md` give the source URL of each role. An agent file gives no URL, because its description is in each request.

### Rename, `upgrade` and `check`

[ADR 0009](0009-upgrade-from-0-5.md) refers to this section. Before `upgrade`, spec-dna 0.6.0 stops with exit code 3 on a 0.5 configuration, because the pin differs.

Configuration:

- `[harness.roles] reviewer` and `spec-writer` are old keys, and each is a configuration error (exit code 3). The message names `verifier`, or says that the role ended. `upgrade` renames `reviewer` to `verifier` with its values. It removes `spec-writer`, and its report names the model of that role.
- `upgrade` writes each new key with a value that adds nothing: `code-simplifier = "none"`, `skills = []`, `review-md = false` and `protect-tests = "off"`.
- Thus the upgraded team is the 0.5 team under the new names, and `check` gives no SD504 for a new part. `init --reconfigure` offers the new parts, as ADR 0004 does for an upgrade.

Files. The marker of ADR 0004 gives the state of each file. `--dry-run` shows each change first, and `upgrade` never commits. `upgrade` does this for each state:

- An unedited `checker.md` or `spec-writer.md`: `upgrade` deletes it and writes `verifier.md`. `check` then reports nothing.
- An edited `checker.md` or `spec-writer.md`: `upgrade` keeps it, writes `verifier.md`, and names the kept file in its report. `check` reports SD501.
- An unedited `model-team.md`, model-team line or other agent file: `upgrade` writes the 0.6 text, which names `verifier`.
- An edited file that the configuration generates: `upgrade` keeps it. `check` reports SD501, as in 0.5.
- A file with no marker at the path of a new part, for example `verifier.md`, a stage skill or `REVIEW.md`: spec-dna never replaces it. An agent file with the name `verifier` counts too. `upgrade` and `init` set the role to `none`, remove the skill from `skills`, or set `review-md = false`. The report names the file.

Codes. No code changes its number, its name or its purpose:

- SD501: a generated harness file or line has a hand edit. It now also covers an edited file with a harness marker that the configuration does not generate. 0.5 gave SD502 for it, with the advice to run `init --reconfigure`, but `init` keeps an edited file. To clear it, delete the file, or delete its marker line to keep it as an agent file of the project. The names `checker` and `spec-writer` belong to no 0.6 role.
- SD502: an old or different text. It covers an unedited file with a marker that the configuration does not generate. `upgrade` and `init --reconfigure` remove that file. The scan for marked files outside the configuration covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`. The adapter skill of ADR 0003 has its own marker, so the scan ignores it.
- SD504: a needed file is absent, or a file at a role path has no marker. It now also covers the stage skills and `REVIEW.md`.

### Stage skills

spec-dna generates 4 skills, each as a whole file `.claude/skills/<name>/SKILL.md` with the harness marker. The design gives the names and the stages. `[harness] skills` alone selects the skills: `init` proposes all 4 when `[change] policy` is not `none`, and `[]` when it is `none`. Each skill runs spec-dna commands for its steps, as `spec-writer` did. Each description is short, because it is in each request, and the test plan measures the bytes.

### `REVIEW.md` and the review skill

- When `review-md = true`, spec-dna writes `REVIEW.md` at the root as a whole file with the harness marker. Its text follows the playbook: the passes Bugs, Security and Compliance, the severities Important and Nit, and at most 5 Nits.
- Compliance compares the diff with `spec.md`, `plan.md` and the `rules/` documents. Thus a project puts its own policy in `rules/`, not in `REVIEW.md`. The "Do not report" list names only the findings of `spec-dna check` and the generated files. spec-dna cannot know the other CI checks of a repository.
- The service reads `REVIEW.md` only from the base or default branch, so a change applies after the merge. The local command does not read it. Thus the skill `spec-dna-review` carries the same passes for a local review.
- spec-dna writes both texts from one source, so they agree in each version. The [Code Review page](https://code.claude.com/docs/en/code-review-customize.md) names `CLAUDE.md`, not `REVIEW.md`, as the file that Claude Code reads for each task. Thus `REVIEW.md` adds no bytes to an agent request.

### `protect-tests`

- `spec-dna hook protect-tests` runs on `PreToolUse` with the matcher `Edit|Write`. It answers `permissionDecision: "ask"`, with a reason, when the path matches a glob of `[evidence] tests` and git tracks the file. A new test file does not trigger the hook, because a fix can add a test.
- With `protect-tests = "fix-branches"`, the branch must also start with `fix/`. With `"always"`, any branch applies. A project with another prefix for fixes uses `"always"`.
- The hook reads no configuration. The generated command holds the globs and the mode. An empty `[evidence] tests` list gives no hook.
- For a path that matches a glob, the hook runs `git` to get the branch and the tracked state. `git` also resolves a worktree. Any other path starts no process. This amends the hook rule of `docs/commands.md` that a hook reads at most one more file.
- The hook never fails. An error gives no output and exit code 0, so the edit continues. The command ends with `2>/dev/null || true`. The hook does not see an edit through `Bash`. The eval check `tests_unchanged` proves the result.

### Evals

Files:

- An eval is a JSON file in `[evals] dir` (default `evals`). It gives the task, the fixture, the allowed tools, the limits and the checks. The design gives the form. Each eval must give `max-budget-usd`, because the cost cap needs it.
- The `[evals]` table enables the evals. `check` applies SD506 only when the table exists.
- `<dir>/check.sh` is a file of the project. `init` writes a starter once. It has no marker, and `check` does not compare its text. `init` also writes `.github/workflows/agent-evals.yml` once, with no marker. Dependabot changes its pins, and a marker would then give SD501.

The runner. `spec-dna eval run` uses the standard library and the git command line, so it adds no dependency. For each run, it does these steps:

1. Copy the fixture into a new temporary directory.
2. Copy the harness files of the project over it: `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and `.claude/`. Never copy `.claude/settings.local.json`, because it is local state of one person.
3. Start a git repository, and commit the tree as the base of each check. Use a fixed author and no signature. Switch to the `branch` of the eval, if it has one.
4. Start `claude -p` with the task, `--allowedTools`, `--output-format json`, `--max-turns`, `--max-budget-usd` and `--permission-mode dontAsk`. Never pass `--bare`.
5. Run each check with `sh -c` in the directory. Source `<dir>/check.sh` from the project first, so the agent cannot change the helper of its own checks.
6. Delete the directory.

Rules of the runner:

- A run passes when `claude` exits with code 0 and each check exits with code 0. An eval passes when at least `pass-at` of its runs pass. `--runs` overrides the `runs` of an eval, which overrides `[evals] runs`.
- The pass rate is the share of evals that pass. An eval is the unit that a person adds after an incident and repairs after a failure.
- Before the first run, the runner adds `runs × max-budget-usd` for each selected eval. If the sum is more than `max-total-usd`, it does not start.
- Claude Code checks `--max-budget-usd` against an estimate of the client, so the spend of a run can be higher. Thus the runner also adds the `total_cost_usd` of each run. When the sum passes `max-total-usd`, it starts no more runs.
- Exit code 0 means that the pass rate is at least `min-pass-rate`, and 1 means that it is lower. Exit code 3 is for a configuration error, such as an invalid eval, or for a stop by the cap.
- A `claude` that is absent or cannot start is also exit code 3, with a message that names `claude`. The runner does not use exit code 4, because `docs/commands.md` gives it to git and file system errors.
- `--json` prints the result, and `--out <dir>` writes it to a file. The result has the eval ids, the check names, the counts, the sum of `total_cost_usd` and the versions. It has no prompt text and no output text. The task text is in the eval file, which the project commits. Claude Code can keep its own session files on the machine, and spec-dna does not read or copy them.
- `eval run` is the only spec-dna command that starts a program that uses the network. The runner is the binary. A check is a command of the project, as a test command is, and `sh` runs it.

The workflow:

- Triggers: `pull_request` and `push` on the harness files and `[evals] dir`, a weekly `schedule`, and `workflow_dispatch`.
- `permissions: {}` is at the workflow level. The job has only `contents: read`. Each `uses:` is a full commit SHA, with the tag in a comment. A fork pull request gets no secrets. The job runs only when the head repository is the base repository. The workflow never uses `pull_request_target`.
- The job installs spec-dna and Claude Code, then runs `spec-dna eval run` with the secret `ANTHROPIC_API_KEY`. The exit code of the runner is the gate.

SD506 (`evals-invalid`) is an error in each profile. It reports an eval file that is not valid JSON or does not follow the form. It also reports an eval with no check, an absent fixture, and an absent `<dir>/check.sh`. ADR 0003 says that a new rule starts as a warning in a minor release. SD506 applies only to a project with `[evals]`, which no older configuration has, so no current corpus gets it. Thus SD506 is an error from the start, as the harness rules of ADR 0004 are.

### Rejected

- Option 1, no runner: each project writes its own scripts, with no shared form, no cost cap and no SD506.
- A Python or shell eval runner: it needs a second runtime on each machine. ADR 0004 rejected script hooks for the same reason.
- The Agent SDK packages: a new language and a new dependency for one command. The CLI gives the same agent loop.
- `claude-code-action` for the eval job: a third-party action that holds the key and asks for a write scope, in a public repository. The job needs only `contents: read`.
- LLM-judged checks: the verdict changes from run to run, and each verdict costs a model call. A check is a command with an exit code.
- `--bare` in the runner, or a fixture with only its own harness files: the eval then does not test the setup of the project. `--bare` skips the hooks, the skills, `CLAUDE.md` and the subagents.
- `deny` or exit code 2 in `protect-tests`: a test can be wrong, and only a person can decide that. A deny gives the person no choice.
- `REVIEW.md` as the only review channel: the local command does not read it, and the service reads it only after the merge.
- A `REVIEW.md` that includes the review skill with an `@` line: Code Review ignores a Nit cap or an exclusion in a skill. It also changes some characters of the text.
- The passes in `CLAUDE.md`, which both review tools read: `CLAUDE.md` is in each request, so each request would carry the review text.
- The role name `reviewer`: the playbook calls the role `verifier`. "Review" already names the passes of `REVIEW.md` and the review skill.
- The role `spec-writer`: "spec" now means the `spec.md` of one change, so the name would mislead. The stage skills give its steps at the stage where they apply.
- A playbook word for the advisor: the playbook has no advisor, and its words name other roles. Claude Code calls it the "advisor tool".
- `researcher.md` as the file name: the built-in Explore subagent would return, on the model of the main session.
- A generated `check.sh` that spec-dna writes again at each run: it would delete the checks of the project. A marker on `check.sh` or on the workflow would give SD501 for each project edit or pin update.
- Eval transcripts in the result: they can hold secrets and code. A person runs one eval again to see the session.
- Evals on each pull request, forks included: a fork gets no secrets. `pull_request_target` would give the key to the code of the fork, and its hooks run with no trust prompt.

### Consequences

- Good: one `init` adds the stage skills, the review policy, the hook and the eval skeleton. `check` keeps the generated parts true.
- Good: the role names match the playbook and Claude Code. An eval tests the harness of the project, with the same command on a laptop and in CI.
- Good: an upgrade changes names, not behavior. Each new part stays off until the owner selects it.
- Bad: each eval run costs money, and `total_cost_usd` is an estimate. The last run before a stop can take the spend above `max-total-usd`. The cap limits one command, not a month. `runs` and `pass-at` reduce the noise, and add cost.
- Bad: a local run also loads `~/.claude` of the person, so a local result can differ from the CI result. When `--bare` becomes the default of `-p`, the runner must disable it with a new flag, and a spec-dna release must follow.
- Bad: if an allow rule overrides the hook `ask`, the hook gives no guard in an eval. A probe must settle this before an eval relies on it. `tests_unchanged` proves the outcome in both cases.
- Bad: a generated agent has priority over a user or plugin agent of the same name, for example `code-simplifier`.
- Bad: an edited `checker.md` or `spec-writer.md` gives SD501 until the owner deletes the file or its marker.
- Bad: `protect-tests` needs spec-dna 0.6.0 on the `PATH`. The checks need a POSIX shell, as the hooks do on Windows.
- Bad: `check.sh` and the workflow do not follow a later release. `stats --sdlc` reads only the eval results that the project commits, and the workflow commits nothing.

## Amends

ADR 0004 is accepted, and its text is locked. This ADR changes these sections of it:

- "Roles": `verifier` (`verifier.md`) replaces `reviewer` (`checker.md`). `spec-writer` ends. `code-simplifier` is new at the level `full`.
- "Configuration": `[harness]` gets `skills`, `review-md` and `protect-tests`. The table `[evals]` is new.
- "Generated files": the stage skills and `REVIEW.md` are generated files with the marker. This amends the "one skill" rule of the ADR 0003 amendment of 2026-11-08, for a project with a harness.
- "Hooks" and "Consequences": `protect-tests` is the third hook, and it needs spec-dna 0.6.0 or later. It can run `git` for a test path. The rule "no stored prompt, command or tool text" also applies to the eval result.
- "`init` and `check`": `upgrade` also removes an unedited generated file that the configuration no longer generates. SD501 covers an edited marked file outside the configuration. The scan covers `.claude/skills/` and `REVIEW.md`. SD506 is new.

## Amendments
