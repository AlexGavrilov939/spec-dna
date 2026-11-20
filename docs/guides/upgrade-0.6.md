# Upgrade from 0.6 to 0.7

Version 0.7.0 replaces the 4 stage skills of 0.6 with a workflow pack of 11 skills. It also adds the approval in the branch and new checks of a plan. `spec-dna upgrade` moves a 0.6 project to 0.7 in one pull request, and it proves that no id and no requirement text changed. This guide gives the procedure. [commands.md](../commands.md#upgrade) gives the options and the exit codes.

## What changes

- **Skills.** The workflow pack has 11 skills ([harness.md](../harness.md#the-workflow-pack-and-reviewmd)). The skill names `intent`, `spec` and `plan` of 0.6 end with no alias. `intent` is now `brainstorm`, `spec` is now `specify` and `clarify`, and `plan` is now `write-plan`. `review` stays, with a new text.
- **Default skills.** The default of a skill comes from its verdict in the table of the release ([ADR 0023](../adr/0023-default-skills-from-the-release-table.md)). In 0.7.0, a skill of the 7 of the design (`brainstorm`, `specify`, `clarify`, `write-plan`, `review`, `finish` and `post-mortem`) is on unless its verdict is `off`, and no verdict is `off`. `tdd` is on too, because its verdict is `on`. `execute`, `verify` and `debug` have no verdict `on`, so they are off by default. `spec-dna skill list` prints the default of each skill, and `spec-dna eval verdict --release` prints the verdicts.
- **Profiles.** For a new project, `init` writes the keys of the workflow that the profile gives ([profiles.md](../profiles.md#what-init-writes-for-each-profile)). `upgrade` applies no profile, so each key of your project stays. The one exception is the list `skills`: `upgrade` adds the skills that are new in 0.7.0 ([Default skills](#default-skills-that-upgrade-adds)).
- **Approval.** With the new keys `[change] approval` and `[change] approvers`, an approval commit of a person in the branch counts ([config.md](../config.md#approval-in-the-branch)). An upgraded project has no such key, so it keeps the rule of 0.6: only the base tree gives an approval.
- **Plans.** A task of `Order` can have the labels `Hours:` and `Skip:` ([types.md](../types.md#tasks-of-a-plan)). `finish` adds the hours of the tasks that are not skipped, and prints the line `Hours: <total>`. The type `plan` has no default budget. A heading below `Order` (`### Part two`) groups tasks, and the tasks under it count, as do the code spans after the first in a `Files` item ([types.md](../types.md#sections-that-carry-list-items)). A plan that groups its tasks under headings can therefore show open tasks after the upgrade, and SD923 or SD924 can report them.
- **Evidence.** A tag `covers: partial <doc-id>#<key>` records a test that proves a part of a requirement. It is not evidence ([config.md](../config.md#a-partial-tag)).
- **Checks.** 0.7.0 adds 6 codes and changes others. [The new checks](#the-new-checks) lists them.
- **Cache and output.** The cache has a new format, so each project rebuilds its cache once. Some `--json` outputs have new fields, and the field `class` can be `null`. The section 0.7.0 of the [changelog](../../CHANGELOG.md) lists each change of the output.

The new commands:

| Command | Use |
| --- | --- |
| `spec-dna approve <slug> intent\|spec\|plan` | A person approves one artifact of a change in a commit of its own ([commands.md](../commands.md#approve)) |
| `spec-dna import speckit\|openspec` | Move a project of Spec Kit or OpenSpec into the layout of spec-dna ([commands.md](../commands.md#import)) |
| `spec-dna skill list\|show` | Print the state and the default of each skill, or the text of one skill ([commands.md](../commands.md#skill)) |
| `spec-dna eval pack`, `eval run --ab`, `eval verdict` | Measure a skill against no skill in your project, and print the verdict of each skill ([evals.md](../evals.md#the-pack-and-the-measure-of-a-skill)) |

## Before you start

- The project pins spec-dna 0.6.0 or later. For an older pin, first use [the guide of 0.5](upgrade-0.5.md).
- The repository has a commit, and the working tree is clean. `upgrade` reads `HEAD`, and it refuses a staged change, an unstaged change and an untracked file that git does not ignore.
- Install the 0.7.0 binary ([quickstart.md](../quickstart.md#install)). Download `spec-dna-pin.toml` of the release.
- Save `spec-dna-pin.toml` outside the repository, for example in the folder above it. Verify it with `gh attestation verify ../spec-dna-pin.toml --repo AlexGavrilov939/spec-dna`. The command prints nothing when its output goes to a file or a pipe, so check its exit code (0 means verified).

A project that has the table `[spec-dna-sha256]` needs this file. A project with no table can run `upgrade` without it.

Before the upgrade, each command of the 0.7.0 binary that reads `spec-dna.toml` stops with exit code 3. The pin is of 0.6, and `[harness] skills` can have a name of 0.6. `upgrade` is the command that reads such a file.

## Run the upgrade

```sh
spec-dna upgrade --dry-run --pin-file ../spec-dna-pin.toml
spec-dna upgrade --pin-file ../spec-dna-pin.toml
```

`--dry-run` prints the diff of each file and writes nothing. For a 0.6 project, `upgrade` does these steps:

1. It moves the pin to 0.7.0, with the sums of `--pin-file`.
2. It renames the skills of 0.6 in `[harness] skills`. It adds a new name only if the name is on by default. A name that the list has already stays, also when it is off by default. It then adds each skill that is new in 0.7.0 and that `init` proposes for your profile and level, if the list is not empty.
3. It makes the 2 edits of 0.6.1 for the key `types` and the layer `pending`, if they apply ([Upgrade a 0.6 project](upgrade-0.5.md#upgrade-a-06-project)).
4. It writes the generated files of 0.7.0: the adapters, the harness files and the skill file of each name in `skills`. It removes the unedited skill files of `intent`, `spec` and `plan`. It keeps an edited file, and `check` reports it (SD501).
5. It proves the result and prints the report. If the proof fails, it writes nothing. [What the proof checks](upgrade-0.5.md#what-the-proof-checks) gives the checks.

`upgrade` changes no other key. It does not apply the other defaults of a profile: `policy`, `gate`, `approval`, `review-md`, `protect-tests` and `[evals]` stay as they are. It never commits and never deletes a document. A second run after the commit prints `spec-dna upgrade: nothing to change`.

If the list `skills` has many lines and the rename changes its length, `upgrade` writes nothing and exits with code 3. The message gives the list to write and the names that the list leaves out. Write the list by hand, commit the change, and run `upgrade` again. The new skills of 0.7.0 never stop `upgrade`. If the list has many lines and the splice can take the rename but not the new names, `upgrade` makes the rename alone, as 0.7.0 did, and the report has a warning with the names to add by hand. A list of many lines with no name of 0.6 gets the same warning.

### Default skills that `upgrade` adds

0.6 had no way to choose `tdd`, `execute`, `verify`, `finish`, `debug` and `post-mortem`, so the list of a 0.6 project never has them. When the old pin is before 0.7.0 and the list is not empty, `upgrade` adds each of these skills that `init` proposes for the profile and the level of the project ([profiles.md](../profiles.md#what-init-writes-for-each-profile)). For the profile `standard` at the level `lean`, these are the skills that are on by default. The profile `lite` gets `finish` only, and the level `full` gets all 6. A project with the policy `none` (or with no `[change]` table) gets none, because `init` writes no skill for that policy. A name that the list has keeps its place, and an empty list stays empty. To turn a skill off, remove its name from `skills` and run `spec-dna init`.

## Read the report

After the file lines, `upgrade` prints the proof line and the report. The report of a 0.6 project names the rename of the skills. The lines have this form:

```text
proof: ok (<n> ids, <n> requirements, <n> sections, <n> types)
report:
  edits of spec-dna.toml beyond the pin (<n>):
    `skills` of `[harness]` got the names of 0.7.0: `<old>` is now `<new>`, `<old>` ended
    `skills`: `<old>` ended, and `<new>` is off by default in 0.7.0 (no verdict `on`); add "<new>" to `skills` of [harness] to turn it on
    `skills` of `[harness]`: added `<new>`, new in 0.7.0 and on by default for the profile `<profile>`; remove a name to turn its skill off
  errors of `check --all` on the new tree: none
to commit: git add -A -- <paths>
next: commit the paths above in one commit, in a pull request of its own: spec-dna.toml has edits beyond the pin (SD503).
```

- The proof line counts the ids, requirements, sections and types that the proof compared. A project with documents has more than 0 ids.
- The first line of `edits` names the new names of each old name. `ended` means that the list keeps no new name for it.
- Each line that names a new name of a rename says that the list leaves it out, because it is off by default. The last line of `edits` names the skills that `upgrade` added. With `--json`, these lines are in `report.edits`.
- The line `next:` after `to commit:` tells how to commit the paths. See the next section.
- A line `kept <path>: edited by hand (SD501)` names a skill file of 0.6 that you edited. Move your text to a file that spec-dna does not generate, and delete the old file.
- The last line counts each error code of `check --all` on the new tree. The proof refuses a new error of the rule groups `references` and `ids`. Another code can report an old problem, or a check of 0.7.0, such as SD923 in `strict`. The proof reads no history, so it can count SD923 or SD924 for an approved plan of 0.6. `check` skips such a plan (see below). The gate of the pull request reports only the new findings.

A skill that the list leaves out is still available. `next` names the skill of each step, and `spec-dna skill show <name>` prints its text. To get the skill file, add the name to `skills` in `[harness]`, and run `spec-dna init`. You can do this before the commit of the upgrade. `init` prints a `git add` command for the new file, so run it and the command of `upgrade`. In `strict`, SD507 then gives a warning for each skill in `skills` that is off by default.

## Open the pull request of the pin

1. If you have work that is not committed, run `git stash -u`, because `upgrade` also refuses an untracked file that git does not ignore.
2. Run `spec-dna upgrade`, then the `git add` command that it prints, and commit. Then run `git stash pop`.
3. Run `spec-dna check --base origin/main`. The gate reports only the findings that the commit adds.
4. Open a pull request.

A hook that runs `spec-dna check` refuses each commit while the pin is older than the binary, so the upgrade is the first commit of a branch. The line `next:` of `upgrade` tells what the pull request can hold:

- If only the pin, the table `[spec-dna-sha256]` and new skill names in the list `skills` change in `spec-dna.toml`, SD503 does not count the file. The same pull request can also hold other work. Only a move to a later version counts.
- If `spec-dna.toml` has other edits (the report lists them), the pull request changes `spec-dna.toml` and generated files only. A document or a code file in the same pull request gives SD503. A change of the list `skills` is such an edit.

SD503 does not count the generated files, and it does not count the 3 skill files of 0.6 that `upgrade` removes ([Files that SD503 counts](../diagnostics.md#files-that-sd503-counts)).

The CI job that `init` writes installs the version of the pin, so it runs 0.7.0 on this pull request.

## After the merge

Each teammate installs 0.7.0, because the pin is exact. Then run `spec-dna init --reconfigure` to choose the parts of 0.7.0. It asks the questions again, with the current values as the defaults ([init.md](../init.md#reconfigure)):

- For `lite` and `standard`, the second prompt of question 5 asks for the approvers, when the change policy is not `none`. A list gives `approval = "branch"` and `approvers` ([init.md](../init.md#the-approvers)). List persons only. `init` refuses the address of an agent or a bot, and `check` reports such an address (SD926).
- For `strict`, `init` writes `approval = "base"`. It also writes `protect-approvals = true` in a file that has no such key. The hook `protect-approvals` then denies an approval by an agent ([harness.md](../harness.md#spec-dna-hook-protect-approvals)). Each teammate needs 0.7.0 for the hook.
- Question 12 sets the skills. The list already has the skills that `upgrade` added. Change the answer to turn a skill on or off.

The command changes `spec-dna.toml` and generated files only. Commit its result in a pull request of its own.

The key `[change] max-task-files` sets the limit of SD924, and its default is 3. No question asks for it. To change the limit, set the key by hand in a pull request of its own ([config.md](../config.md#the-table-change)).

A person approves an artifact with `spec-dna approve <slug> <artifact>` in a terminal of their own. An agent writes an approval commit only when the owner says so explicitly in the chat ([the change guide](change.md#the-approval-commit)).

To measure a skill in your project, run `spec-dna eval pack` and `spec-dna eval run --ab <skill>`. For SD507, the verdict of the project comes before the verdict of the release ([evals.md](../evals.md#the-verdict)).

## The new checks

0.7.0 adds 6 codes ([diagnostics.md](../diagnostics.md)):

| Code | It reports |
| --- | --- |
| SD507 | A skill in `skills` that is off by default |
| SD612 | A reference from a document outside the folder of a change into that folder |
| SD923 | A task of an approved plan with no `Files:` or no `Verify:`, with a glob or a folder in `Files:`, or with an empty `Skip:` |
| SD924 | A task of an approved plan with more paths than `max-task-files` |
| SD925 | An item of `Concerns` or `Open questions` with no task box |
| SD926 | The address of an agent or a bot in `approvers` |

SD923, SD924 and SD925 are off in `lite`, warnings in `standard` and errors in `strict`. SD923 and SD924 skip an approved plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0 (a plan of 0.6). [diagnostics.md](../diagnostics.md#checks-of-the-change-lifecycle) gives the rule and the note. SD612 and SD926 are warnings in each profile, and SD507 is a warning in `strict` only. [profiles.md](../profiles.md) gives each severity. A warning never fails `check`.

The gate compares the head with the base, so a finding that the base also has fails no pull request. Run `spec-dna check --all` to list the old findings once.

These codes change:

- SD910 does not report a `spec.md` or a `plan.md` with the status `draft`. Its message names one of 9 reasons.
- SD913 gives nothing before a change has a `plan.md`.
- The messages of SD608 and SD916 name a partial tag. A partial tag alone is not evidence.
- SD502 reports a line of `AGENTS.md` or `CLAUDE.md` in the form of 0.6. `upgrade` and `init` write the line of 0.7.0.
- SD701 to SD704 no longer apply to a plan, unless `[budgets.plan]` sets a budget.

[The change guide](change.md#when-check-reports-a-code) gives the fix of each code of a change.

## Undo

Before the commit, run `git reset --hard` and `git clean -fd`.

After the merge, the undo needs 1 or 2 pull requests:

1. If `skills` has a name that 0.6 does not know (each name except `review`), remove it with the 0.7.0 binary first. Run `spec-dna init --reconfigure`, and give the word `skills=review` or `skills=none` in question 12. In an answers file, set `harness-skills = ["review"]` or `harness-skills = []`. `init` removes each unedited skill file that the configuration no longer has. Merge this change in a pull request of its own, because SD503 of 0.6 counts the skill file of a new name.
2. Install the 0.6 binary of the old pin. Write back the old configuration with `git checkout <the commit before the upgrade> -- spec-dna.toml`.
3. Run `spec-dna init`. It writes the generated files of 0.6.
4. Commit, and open a pull request with this commit only. It changes `spec-dna.toml` and the generated files of 0.6 only, so SD503 of 0.6 accepts it.
5. After the merge, each teammate installs the 0.6 binary again.

The old configuration has no key of 0.7.0. The 0.6 binary stops with exit code 3 on such a key: `approval`, `approvers`, `max-task-files` or `protect-approvals`. Make each later edit of `spec-dna.toml` that 0.6 knows again, in a pull request of its own.
