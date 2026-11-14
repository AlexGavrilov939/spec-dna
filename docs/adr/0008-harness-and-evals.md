---
status: accepted
---

# 0008. Harness and evals for 0.6.0

## Context and Problem Statement

[ADR 0004](0004-harness.md) installs a team of models, hooks and a drift check for Claude Code. 0.6.0 adds a lifecycle for each change ([ADR 0006](0006-change-lifecycle.md)). The owner asks for a skill for each stage, a generated `REVIEW.md`, and a hook that protects the tests during a fix. The owner also asks for role names from the playbook and the Claude Code documentation, and for evals with a pass-rate gate. How can spec-dna add these parts and keep the constraints of ADR 0004?

Constraints:

- The harness works in any repository, with any stack. It names or copies no consumer project.
- Claude Code is the only agent. The repository is the only source of truth, so the harness needs no service.
- A name comes from the playbook or the Claude Code documentation. Where they name nothing, a plain STE word applies that does not conflict with them.
- Hooks and the runner need no second runtime. spec-dna itself makes no network call, and keeps no prompt, command, tool or output text.
- A rename keeps no runtime alias ([ADR 0009](0009-upgrade-from-0-5.md)). A hand edit stays. The default harness level at `init` stays `lean`.

Evidence:

- Earlier measurements on anonymized corpora showed that the gate gave the measured gain. Retrieval and the split of large documents did not. The owner keeps the data. Agents search the repository and seldom follow a tree of links. Large files and process text in each request cost tokens and gave no gain. Hooks at the time of the problem changed agent behavior, and prose rules did not ([ADR 0004](0004-harness.md)).
- The [playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook) names `verifier.md`, "a researcher", "a code simplifier", `REVIEW.md` and `agent-evals.yml`. It says: "Each production incident gets an eval". It names no advisor. The [Terminology](../design/0.6.0.md#terminology) table gives the source of each role and term.

## Considered Options

The open question is the eval runner. The other parts follow the pattern of ADR 0004, and "Rejected" gives their alternatives.

1. No runner: each project writes its own eval scripts.
2. A runner script in Python or shell that spec-dna ships.
3. A runner on the Agent SDK packages, in Python or Node.
4. A third-party action (`claude-code-action`) with the checks in the workflow.
5. `spec-dna eval run`: a subcommand that starts headless Claude Code and runs the checks. It needs only the binary and `claude`, runs on a laptop, has a cost cap before the start, and `check` reads its files (SD506).

## Decision Outcome

Option 5, with the harness parts below. The design gives the tables and the forms, in [Harness](../design/0.6.0.md#harness) and [Evals](../design/0.6.0.md#evals).

The default level at `init` stays `lean`. `lean` adds the stage skills, `REVIEW.md` and `protect-tests`. `full` adds `implementer` and `code-simplifier`. For a new project, `init` proposes the evals `skeleton`: it writes the files only, and CI runs nothing until the owner sets the repository variable `SPEC_DNA_EVALS`. An upgraded project keeps `none`. The role table keeps the key `claude-code`, so a later adapter can add its own key.

### Role names

- Neither the playbook nor Claude Code names `orchestrator`, `implementer` or `helper`, so these are plain STE words.
- `verifier` (`verifier.md`) replaces `reviewer` (`checker.md`). Its text follows the playbook: "Do not fix anything; report only."
- `researcher` keeps the file `Explore.md`, because only an agent with the name `Explore` replaces the built-in subagent.
- `code-simplifier` is new at the level `full`.
- `advisor` keeps the Claude Code name, because the playbook names no advisor.
- `spec-writer` ends. The stage skills give its steps at the time of use.
- An agent file gives no source URL, because its description is in each request.

### Rename, `upgrade` and `check`

[ADR 0009](0009-upgrade-from-0-5.md) refers to this section. Before `upgrade`, spec-dna 0.6.0 stops with exit code 3 on a 0.5 configuration, because the pin differs.

Configuration:

- The old keys `[harness.roles] reviewer` and `spec-writer` give exit code 3. The message names `verifier`, or says that the role ended. `upgrade` renames `reviewer` to `verifier` with its values. It removes `spec-writer`, and its report names the model of that role.
- `upgrade` writes each new key with a value that adds nothing: `code-simplifier = "none"`, `skills = []`, `review-md = false` and `protect-tests = "off"`. Thus the team stays the 0.5 team under the new names, and `check` gives no SD504 for a new part. `init --reconfigure` offers the new parts.

Files. The marker of ADR 0004 gives the state of each file:

- `checker.md` or `spec-writer.md`: `upgrade` writes `verifier.md`, and deletes the old file if it is unedited. It keeps an edited one and names it, and `check` reports SD501.
- An unedited `model-team.md`, model-team line or other agent file: `upgrade` writes the 0.6 text, which names `verifier`. An edited generated file stays, and `check` reports SD501, as in 0.5.
- A file with no marker at the path of a new part, or an agent file with the name `verifier`: spec-dna never replaces it. `upgrade` and `init` set the role to `none`, remove the skill from `skills`, or set `review-md = false`, and name the file.

Codes. No code changes its number, its name or its purpose:

- SD501 also covers an edited file with a harness marker that the configuration does not generate. 0.5 gave SD502 for it, with the advice to run `init --reconfigure`, but `init` keeps an edited file.
- SD502 covers an unedited file with a marker that the configuration does not generate, and `upgrade` and `init --reconfigure` remove it. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill of ADR 0003, which has its own marker.
- SD504 also covers the stage skills and `REVIEW.md`.

### Stage skills

spec-dna generates the 4 [stage skills](../design/0.6.0.md#stage-skills) as whole files with the harness marker. `[harness] skills` alone selects them: `init` proposes all 4 when `[change] policy` is not `none`, and `[]` when it is `none`. Each skill runs spec-dna commands for its steps, as `spec-writer` did. Each description is short, because it is in each request.

### `REVIEW.md` and the review skill

- With `review-md = true`, spec-dna writes [`REVIEW.md`](../design/0.6.0.md#reviewmd) with the harness marker. Compliance compares the diff with `spec.md`, `plan.md` and `rules/`, so a project puts its own policy in `rules/`. The "Do not report" list names only the findings of `spec-dna check` and the generated files, because spec-dna cannot know the other CI checks.
- The service reads `REVIEW.md` only from the base or default branch, so a change applies after the merge. The local command does not read it. Thus the skill `spec-dna-review` carries the same passes. spec-dna writes both from one source text, so they agree in each version.
- Claude Code reads `CLAUDE.md`, not `REVIEW.md`, for each task. Thus `REVIEW.md` adds no bytes to an agent request.

### `protect-tests`

- The [hook](../design/0.6.0.md#protect-tests-during-a-fix) answers `ask` before an edit of a tracked test file. A new test file does not trigger it, because a fix can add a test. A project with a fix prefix other than `fix/` uses `"always"`.
- The generated command holds the globs and the mode, so the hook reads no configuration. It runs `git` only for a path that matches a glob. This amends the hook rule of `docs/commands.md` that a hook reads at most one more file.
- The hook never fails, and it does not see an edit through `Bash`. The eval check `tests_unchanged` proves the result.

### Evals

- An eval is a JSON [file](../design/0.6.0.md#an-eval-file) in `[evals] dir`. It must give `max-budget-usd`, because the cost cap needs it. The `[evals]` table enables `eval run`, SD506 and SD918. The repository variable `SPEC_DNA_EVALS` enables the CI runs.
- [The runner](../design/0.6.0.md#the-runner) uses the standard library and the git command line, so it adds no dependency. It never passes `--bare`.
- The pass rate is the share of evals that pass, not of runs, because a person adds an eval after an incident and repairs it after a failure.
- The runner checks `max-total-usd` before the first run, from `runs × max-budget-usd`, and again after each run, from the sum of `total_cost_usd`, because the real spend can be higher than the estimate.
- Exit code 3 is for a configuration error, a stop by the cap, or a `claude` that is absent or cannot start. The runner does not use exit code 4, because `docs/commands.md` gives it to git and file system errors.
- `eval run` is the only spec-dna command that starts a program that uses the network. The runner is the binary, and its result has no prompt text and no output text. A check is a command of the project, as a test command is.

SD506 (`evals-invalid`) is an error in each profile from the start. ADR 0003 starts a new rule as a warning in a minor release. But SD506 applies only to a project with `[evals]`, which no older configuration has, so no current corpus gets it. ADR 0004 set this precedent for the harness rules.

### Rejected

- Option 1: no shared form, no cost cap and no SD506.
- Option 2: a second runtime on each machine. ADR 0004 rejected script hooks for the same reason.
- Option 3: a new language and a new dependency for one command. The CLI gives the same agent loop.
- Option 4: a third-party action holds the key and asks for a write scope, in a public repository. The job needs only `contents: read`.
- A workflow with `workflow_dispatch` only, and the other triggers as a documented edit: the playbook triggers are then a comment, and a stop of the spend needs a pull request. A test of the secret needs a step, so each trigger starts a runner.
- LLM-judged checks: the verdict changes from run to run, and each verdict costs a model call.
- `--bare` in the runner, or a fixture with only its own harness files: the eval then does not test the setup of the project.
- `deny` or exit code 2 in `protect-tests`: a test can be wrong, and only a person can decide that. A deny gives the person no choice.
- `REVIEW.md` as the only review channel: the local command does not read it, and the service reads it only after the merge.
- An `@` include of the review skill in `REVIEW.md`: Code Review ignores a Nit cap or an exclusion in a skill, and the include changes some characters.
- The passes in `CLAUDE.md`: each request would carry the review text.
- The name `reviewer`: the playbook says `verifier`, and "review" names the passes of `REVIEW.md`.
- The name `spec-writer`: "spec" now means the `spec.md` of one change.
- A playbook word for the advisor: the playbook has no advisor, and its words name other roles.
- `researcher.md`: the built-in Explore subagent would return, on the model of the main session.
- A `check.sh` that spec-dna writes again at each run: it would delete the checks of the project. A marker on `check.sh` or on the workflow would give SD501 for each project edit or pin update.
- Eval transcripts in the result: they can hold secrets and code.
- Evals on fork pull requests: a fork gets no secrets. `pull_request_target` would give the key to the code of the fork, and its hooks run with no trust prompt.

### Consequences

- Good: one `init` adds the stage skills, the review policy, the hook and the eval skeleton. `check` keeps the generated parts true.
- Good: the role names match the playbook and Claude Code. An eval tests the harness of the project, with the same command on a laptop and in CI.
- Good: an upgrade changes names, not behavior. Each new part stays off until the owner selects it.
- Bad: each eval run costs money. The last run before a stop can take the spend above `max-total-usd`. The cap limits one command, not a month. `runs` and `pass-at` reduce the noise, and add cost.
- Bad: a local run also loads `~/.claude` of the person, so its result can differ from CI. When `--bare` becomes the default of `-p`, the runner needs a new flag and a spec-dna release.
- Bad: if an allow rule overrides the hook `ask`, the hook gives no guard in an eval. A probe must settle this first.
- Bad: a generated agent, such as `code-simplifier`, has priority over a user or plugin agent of the same name.
- Bad: an edited `checker.md` or `spec-writer.md` gives SD501 until the owner deletes the file or its marker.
- Bad: `protect-tests` needs spec-dna 0.6.0 on the `PATH`. The checks need a POSIX shell, as the hooks do on Windows.
- Bad: `check.sh` and the workflow do not follow a later release. `stats --sdlc` reads only the eval results that the project commits.

## Amends

ADR 0004 is accepted, and its text is locked. This ADR changes these sections of it:

- "Roles": `verifier` (`verifier.md`) replaces `reviewer` (`checker.md`). `spec-writer` ends. `code-simplifier` is new at the level `full`.
- "Configuration": `[harness]` gets `skills`, `review-md` and `protect-tests`. The table `[evals]` is new.
- "Generated files": the stage skills and `REVIEW.md` are generated files with the marker. This amends the "one skill" rule of the ADR 0003 amendment of 2026-11-08, for a project with a harness.
- "Hooks" and "Consequences": `protect-tests` is the third hook, and it needs spec-dna 0.6.0 or later. It can run `git` for a test path. The rule "no stored prompt, command or tool text" also applies to the eval result.
- "`init` and `check`": `upgrade` also removes an unedited generated file that the configuration no longer generates. SD501 covers an edited marked file outside the configuration. The scan covers `.claude/skills/` and `REVIEW.md`. SD506 is new.

## Amendments
