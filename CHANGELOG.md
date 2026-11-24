# Changelog

This file lists the changes of each version. The project uses [Semantic Versioning](https://semver.org). Diagnostic codes are in [docs/diagnostics.md](docs/diagnostics.md).

## Unreleased

### Changed

- The tier `summary` of `context` serves the summary of the area. It fills the title section of each document first, from the deepest summary up to `product`, and then the other sections in the same order. In 0.8.0, it filled `product` first, so a project with large top documents got no summary. The tier gets at most a half of the budget in place of a quarter. The half is a cap: the tier leaves the room that the entry and the citing sections need in full. At a small budget, it can leave fewer stub lines of linked documents (in one test, at the budget 3,000, 4 of 6). In a test project with a long entry section and 10 linked documents, the entry stays in full at each budget from 6,000 to 10,000 characters (step 50) where the same project with no roll-up serves it in full. The output keeps its order from the top down, and `--json` keeps its form. In a test project with a `product` and a `system` document of about 4,100 bytes each, `context` gave no summary at the budgets 4,000 to 20,000. Now it gives the 2 summaries of the area in full at 8,000. A tree with no roll-up document gets the same output as before.

## 0.8.0 - 2026-11-23

0.8.0 adds an independent acceptance of a change in one pass, test reports as evidence, a roll-up of requirements into summaries, and agent roles in `[owners]`. It adds 31 diagnostic codes in 5 new rule groups, 5 commands or command forms and 5 options of commands. A project of 0.7.1 moves with `spec-dna upgrade`. In `lite`, each new mechanism is off, and each generated file keeps its text of 0.7.1. In `standard`, only the acceptance of a `higher-risk` change and the roll-up are on. The other mechanisms need a key, or a marker for the proof debt.

### Upgrade from 0.7.1

- Install the 0.8.0 binary. Then run `spec-dna upgrade --pin-file <spec-dna-pin.toml>` on a clean tree. If you have work that is not committed, run `git stash -u` first, and `git stash pop` after the commit. Commit the paths that `upgrade` prints in one commit. [docs/guides/upgrade-0.6.md](docs/guides/upgrade-0.6.md#run-the-upgrade) gives the report and the steps. Before the upgrade, the commands `check`, `finish`, `accept`, `approve`, `refresh`, `evidence stamp`, `eval run`, `import` and `init` stop with exit code 3 on the old pin. The other commands print a warning.
- `upgrade` sets no key. In `spec-dna.toml`, it changes the pin line only, and it adds no skill, no table and no list. In `lite`, it adds only an adapter file that the project lacks, as `upgrade` of 0.7.1 does, and it changes only the version in the marker of each generated file, also at the harness level `full`. The text above each marker stays the text of 0.7.1. In `standard` and `strict`, it writes the new text of each workflow skill file and of `model-team.md`, and it changes only the version in the marker of the other generated files. It adds the agent file `.claude/agents/acceptor.md` when the harness level is `lean` or `full` and the list `skills` has `review`. It adds no other file. The tests ran it on a project that the real binary 0.7.1 wrote, in each profile. A second run changes nothing.
- A hook that runs `spec-dna check` refuses each commit while the pin is older than the binary, so the upgrade is the first commit of the branch. SD503 does not count `spec-dna.toml` when it differs from the base only in the pin line and in `[spec-dna-sha256]`, so the pull request of the upgrade can hold other work. The pin stays exact, so each teammate needs 0.8.0. The release commit of this repository moves the pin with the version ([ADR 0013](docs/adr/0013-self-hosting.md)).
- The cache has a new parser version, so each project rebuilds it once.
- In the tests, `check --all` of an upgraded project gives 0 errors and 0 warnings in each profile. No new error comes without a key that the project set. The one exception is the acceptance of a change whose plan a branch adds after the upgrade. A plan that is on the base from 0.7.1 needs no record, and `finish` prints a note for it in `standard` and `strict`. For a plan that a branch adds, a record is needed in these cases:
  - `standard`: a `higher-risk` change, an intent with no valid `class`, and a `routine` change whose `Files` or diff match `[change] high-risk-paths` or whose delta names a requirement of a layer in `high-risk-layers`.
  - `strict`: each change that has a plan.
  - `lite`: none.
- To turn the record off for a project, set `[change] acceptance-for = "none"`, or `[rules] acceptance = "off"`. A matched role of `[owners]` still needs a record with `acceptance-for = "none"`. Make the edit in a pull request of its own (SD503). A branch cannot lower the rule with its own commits: the rule reads the stricter of the branch and the base. A shallow clone cuts the history of the branch, so `check` and `finish` then need a record for each change with a plan and name `fetch-depth: 0`. A workflow that `init` wrote has `fetch-depth: 0`.
- A `standard` project sees no new warning after the upgrade. Each new warning needs an act of the project: SD616 (`[evidence] reports`), SD618 and SD619 (a marker `proof-owed`), SD645 (`refresh --holds`), SD705 and SD707 (`[budgets.area]`). A rule for the group of the code turns it off, for example `[rules] area-budget = "off"`. The new groups are `reports`, `proof`, `acceptance`, `rollup` and `area-budget`. A rule for a group of 0.7.1, such as `budgets`, `evidence` or `changes`, changes no new code.
- `upgrade` does not set these keys. Add the key to turn the mechanism on: `[evidence] reports` (test reports), `[evidence] reliable-for` and `reliable-runs`, `[change] invariants`, `[change] higher-risk-acceptors`, `[owners]`, `[budgets.area]` and `[layers.summaries]`. `init` writes some of them for a new project, as the profile table below says. For the roll-up, run `spec-dna layer add summaries`, then `spec-dna init` to write the skill again, because SD502 reports the old skill after the new layer.
- A person entry in `[owners]` (`@<login>` or an e-mail) gives a warning, not exit code 3. It needs nothing, because 0.8.0 reads agent roles only.
- `finish --json` has the new key `deviations` in a project that pins 0.8.0, an empty list when the plan has no deviation. It has the keys `rollup` and `acceptance_runs` only when the change has the data. `next --json` has the new field `person`. `status --json` and `stats --json` can have the new object `proof` ([docs/commands.md](docs/commands.md#status) gives the cases).
- A result file of `eval run --ab` of 0.7.1 that has fewer than 15 pairs or fewer than 5 evals reads as `inconclusive` now (the floor of the verdict). The committed records of 0.7.x keep their 9 keys and stay valid. `eval pack` writes the 5 evals `accept-1` to `accept-5` into a project that runs it again, and a file that exists stays.

### Added

- Acceptance in one pass. A change that needs a record has one independent run, which is also its review. `spec-dna accept <slug>` writes the skeleton of the record `acceptance.toml` in the folder of the change, `accept <slug> --brief` prints the brief, and `finish` moves the record to `.spec-dna/accepted/<slug>.toml`. The agent `acceptor` (the model `opus`, the effort `high`, with the role line `acceptor` in `[harness.roles]` that `init` writes for a new project) works from the prompt line that `spec-dna next` prints, `cd '<root>' && spec-dna accept <slug> --brief`. The skill `review` holds the steps of the acceptance in `standard` and `strict`, so there is no new skill, and `next --json` has the 9 skill names of 0.7.1. The record has the format 2: each finding is a `blocker` or a `note`, and an entry has `roles`. A project with the policy `none` needs no record.
- Who needs a record. The new key `[change] acceptance-for` is `none`, `higher-risk` or `all`. The default is `higher-risk` in `standard` and `lite`, and `all` in `strict`. An intent with no valid `class` counts as `higher-risk`, and so does a change on a path of `high-risk-paths` or in a layer of `high-risk-layers`. A matched role of `[owners]` also needs a record. `[change] higher-risk-acceptors` (1 to 3, default 1) is the number of acceptor entries of a `higher-risk` change. `init` writes 2 in `strict`.
- The stop rule. Only a `blocker` gives the result `rejected`. A fix of a note needs a follow-up, `spec-dna accept <slug> --follow-up --brief` and `accept <slug> --follow-up`, not a new full run. After 2 rejected records of one change, `next` names the owner and starts no third run. With `approval = "branch"`, SD941 stops a third record until an approver approves the plan again. With `base`, SD941 is not used. `finish` prints `Acceptance runs: <n>, <r> rejected` when a change has 2 or more records.
- `[owners]` with agent roles. The table maps a glob to role names. A change that edits a matched path needs a record, and the record has an entry for each matched role. One entry can answer more than one role. SD933 reports a missing role.
- The acknowledgement with `branch`. `spec-dna approve <slug> acceptance` adds the e-mail of an approver to a record that has a deviation (SD934). SD932 checks that each commit of the record changes nothing else. With `base`, the merge of the pull request acknowledges the deviation.
- Hook rule D. With `[harness] protect-approvals = true`, the hook `protect-approvals` limits a `git commit` in a call of the agent `acceptor` to one `acceptance.toml`, in one fixed form. `init` writes the key in `strict` only, and `upgrade` writes it nowhere. The rule reads the text of a command, so it is no sandbox: [docs/harness.md](docs/harness.md#spec-dna-hook-protect-approvals) lists what stays open.
- Deviations and invariants. A plan can have the section `Deviations`. The record answers each item, and `finish` prints them as `Deviations: <n>` (SD939, SD940). With `[change] invariants = true`, a spec lists the rules that the change must keep true, and the record answers each one (SD936, SD937, SD938).
- Test reports and proof levels. With `[evidence] reports`, a `covers:` tag proves a requirement only when a bound test case passed in a report. `spec-dna evidence stamp` binds a report to the code tree, `check --reports` and `finish` read the reports (SD613 to SD617), and `case:` in a tag names the case. `[evidence] reliable-for` and `reliable-runs` select the level `reliable` (SD620). The marker `<!-- proof-owed: <reason> -->` records a proof debt (SD618, SD619). `status` and `stats` count the proof states. `reliable-for` with an empty `reports` is exit code 3. `tree-exclude` lists the files that the code tree leaves out.
- Roll-up. The new type `summary` and the layer `summaries` hold the short text of an area. The field `rests-on` pins the requirements that a summary covers, in the form `<doc-id>#<key>@<hash>`. SD641 to SD645 report a stale, incomplete, invalid or re-pinned summary. `spec-dna new summary <area>` writes a summary, and `spec-dna refresh <id>` writes its pins again. The guard of `refresh` refuses a document whose text did not change, unless `--holds` says that the text still holds. `finish` re-pins a summary that the change edited, and `finish --holds <id>` claims another one. `next` prints one note for the summaries that a delta makes due. `mv` prints the summaries that a move makes stale. SD913 does not report a summary. A project with no summary sees no change.
- The area budget. `[budgets.area]` with `soft` and `hard` (SD705 to SD707) bounds the size of an area. A summary counts in place of its sub-area. `init` writes the table in `standard` (60,000 and 120,000 characters) and `strict` (40,000 and 80,000), and `upgrade` writes none.
- The `summary` tier of `context`. The tier serves the chain of summaries and top documents of a request, after the conventions and before the rules, with at most a quarter of the budget. A stale document gets the word `stale`. The tier is new in `--json`, and a tree with no summary gets the output of 0.7.1.
- The 5 rule groups. `reports` (SD613 to SD617), `proof` (SD618 to SD620), `acceptance` (SD927 to SD941), `rollup` (SD641 to SD645) and `area-budget` (SD705 to SD707). A rule for a group of 0.7.1 changes no code of them. [docs/profiles.md](docs/profiles.md) gives the severity of each code.
- Evals. `eval run --arms` compares 2 or 3 harness arms (`none`, `lean`, `lean-no-advisor`, `full` and `project`), and `--harness` sets the team of a run. `eval verdict --compare` gives the verdict of 2 arms. A verdict below 15 pairs of 5 evals is `inconclusive`. A record has the 3 new keys `harness`, `turns` and `seconds`. An eval file can have the key `change`. `eval run --ab accept` runs the 5 evals `accept-1` to `accept-5` of the skill `review`.
- The cost of one pass. The 5 evals ran 3 times in each arm, 15 pairs in each of 2 batches, with `claude` 2.1.330 and the effort `xhigh`. With the advisor, the arm with the new text had a mean of 1.74 USD and 4.6 minutes for a run, and it found each planted defect and accepted the control in 15 of 15 runs. Without the advisor, it had 0.53 USD and 2.2 minutes, and it passed 13 of 15 runs. Both verdicts are `inconclusive`. Both costs are of whole `claude -p` sessions, not of the subagent. The fixtures are small changes, so a real slice costs more. The records are in `bench/results/evals/` (`20261122T064254Z-accept.jsonl` and `20261122T090136Z-accept-no-advisor.jsonl`).
- The advisor of the acceptor. The new key `[harness.roles] acceptor.advisor` is `false` in `standard` and `true` in `strict`. `init` writes it by profile, `init --reconfigure` keeps it, and `upgrade` writes no key. With `false`, the generated `acceptor.md` has one more line that asks the agent not to consult the advisor. The line is a request: Claude Code has no setting of the advisor for one subagent, and a `tools` list, `disallowedTools` and a hook did not remove it in a probe on 2.1.330. For a pass with no advisor, run the acceptor in a session of its own with `CLAUDE_CODE_DISABLE_ADVISOR_TOOL=1`. In `lite`, there is no agent and no key.

The profiles. Each new mechanism is off in `lite`. A cell says when the mechanism is on:

| Mechanism | `lite` | `standard` | `strict` |
| --- | --- | --- | --- |
| Acceptance, deviations (SD927 to SD935, SD938 to SD941) | off | a `higher-risk` change, a matched role, or `acceptance-for = "all"` | each change with a plan |
| Invariants (SD936, SD937) | off | `[change] invariants = true` | `init` writes the key |
| Test reports (SD613 to SD617) | off | `[evidence] reports` | `[evidence] reports` |
| The level `reliable` (SD620) | off | `reports` and `reliable-for` | the same |
| Proof debt (SD618, SD619) | off | a marker, as warnings | a marker, SD619 an error |
| Roll-up (SD641 to SD645) | off | on, with a summary | on, with a summary |
| Area budget (SD705 to SD707) | off | `[budgets.area]` | `[budgets.area]`, as errors |
| Skill text and the agent `acceptor` | text of 0.7.1, no agent | text of 0.8.0, agent, `advisor = false` | text of 0.8.0, agent, `advisor = true` |

### Changed

- The skill `review` holds the steps of the acceptance in `standard` and `strict`. The description of each skill does not change. Each skill file of `standard` and `strict` has new text, so SD502 reports it until `upgrade` writes it again. `skill show` prints the text of the profile of the project.
- `next` has one item `review` for each change. For a change that needs a record, it holds the prompt line, the follow-up line or the line for the owner. For a change that needs none, it is the item of 0.7.1.
- A task whose text names `spec-dna accept <slug>` closes with `finish` where the acceptance is on, as a task that names `spec-dna finish <slug>` does.
- `finish` has the parts 5.1 to 5.3 and 5.5 in its step 5: the reports, the record, the deviations and the archive. A stop keeps a step number from 0 to 5.
- A plan that a commit added when `spec-dna.toml` pinned a version before 0.8.0 needs no record. A plan that a branch adds always needs one when the rule asks for it.
- The parser version is 18, so the cache rebuilds once.
- The workflow of the evals that `init` writes (`.github/workflows/agent-evals.yml`) installs Claude Code 2.1.330, the version of the paid runs of this release. It was 2.1.325, the version of the probes of 0.6.0. `init` writes the file once, so a project that has it keeps its version.
- `eval run --ab` without `--harness` keeps the copy of 0.7.1. The arm `without` of the target `accept` has the `review` text of `lite`.

### Fixed

- The check of a release. `gh attestation verify` prints nothing on success when its output goes to a file or a pipe, and the check of 0.7.1 said "ok" on that silence. The script `scripts/verify-release.sh` checks the assets of a release (the 7 binaries, `SHA256SUMS` and `spec-dna-pin.toml`), keeps the exit code of each `gh` call, and prints `<n> of <n> assets verified`. The workflow `release` creates a draft, runs the script, and publishes the draft only when it passes.
- A task of a person. `next` and `status` end the line of a task with `[person]`, not with a skill, for a task with the label `By: person` or with the title `Review by the owner`. `--json` adds `"person": true`. When only such tasks are open, `next` says that it waits for a person. The skill `write-plan` writes the label.
- The report of `upgrade` counts the errors of the planned tree with that tree as its own base, as `check --all` does after the commit. An old excess of a budget (SD702) is no error of the report now.
- The 5 open notes of part A: a repeated id in `[[deviation]]` or `[[invariant]]` gives SD928, part 5.3 prints the deviations of the record, a code span in `Delta:`, `Follow-up:` and `Invariants` reads as its text, `none` in backticks is `none`, and an empty `Invariants` section with the key gives SD936.

### Moved to 0.8.1

- The persons of `[owners]` (`@<login>` and e-mails). 0.8.0 gives a warning and reads agent roles only.
- The value `review` of `[change] approval`, and the command `spec-dna reviews`, which would read the reviews of the code host.

### What the checks of 0.8.0 do not prove

- The acceptance record. `check` and `finish` show that a record exists, that its form is right, and that it is bound to the code tree. They do not show that the acceptor is not the builder (`session` and `agent` are claims), that it read the brief or tried hard, or that the list of deviations is complete. The independent reviews of the release (3 final reviews, then a review of each commit) found 2 serious and about 10 minor defects, and a review cannot show that none is left.
- The hook rule of the acceptor (rule D of `protect-approvals`). It reads the text of a command in a fixed grammar. It is no sandbox: it does not stop a write outside the worktree, or a script that does the same as a refused command. [docs/harness.md](docs/harness.md#spec-dna-hook-protect-approvals) lists what stays open.
- The advisor of the acceptor. The key `advisor` sets a line in the agent file. Claude Code has no switch for one subagent, so the line is a request, and a hard off is a setting of the whole session. The 2 costs of one pass come from whole sessions, not from the subagent.
- A test file that git does not track can prove a requirement while it is outside the tree hash (issue 113). Stage each test file that a tag uses.
- The roll-up has no use on a real corpus yet. It pins active requirements only, so it has no summary for documents of decisions and research (issue 111). The tests and the example use a training project.

### Verdicts of the skills, rerun for 0.8.0

Three skills have a new run on the text of 0.8.0, with 15 pairs each and `--harness lean` (Claude Code 2.1.330). The table of the release (`RELEASE` in `src/verdict.rs`) follows these records.

| Skill | Pairs | With the skill | Without it | Verdict | Text |
| --- | --- | --- | --- | --- | --- |
| `specify` | 15 | 15 of 15 | 9 of 15 | `on` (rule 1) | 0.8.0 |
| `write-plan` | 15 | 15 of 15 | 11 of 15 | `on` (rule 2) | 0.8.0 |
| `finish` | 15 | 14 of 15 | 14 of 15 | `inconclusive` | 0.8.0 |
| `execute` | not rerun | | | older row | 0.7.1 |
| `review` | not rerun | | | older row | 0.7.1 |

What these verdicts do not prove:

- The `on` of `specify` rests on 3 runs of the arm without the skill that stopped at the turn limit with each check passed. A run counts as a pass only when the agent exits normally. If these 3 runs counted as passes, the arm without the skill would have 12 of 15, and the verdict would not be `on`. The field `turns` of the records does not match the limit (issue 109).
- `finish` moves from `on` (an older text) to `inconclusive`. It stays on by default, because a skill of D2 stays on unless its verdict is `off` (ADR 0023). Both arms are near the ceiling of the oracle, so the run shows no gain and no loss.
- `execute` and `review` have no run on the text of 0.8.0 (issue 110). The 5 evals `accept-1` to `accept-5` measure the acceptance pass, and they decide no row.
- The 15 pairs show a large gain only. They give no verdict for another project or another model.

The costs come from small training fixtures of 1 or 2 files. A real change costs more: part A measured 13 to 25 minutes and about 190k to 250k tokens for each acceptance run on slices of this repository.

## 0.7.1 - 2026-11-19

0.7.1 fixes faults that the first projects found in 0.7.0: a task that only `finish` can do, the pin move of `upgrade`, the checks of a plan of 0.6 and an older `--base`. It adds the real hours of a task and the key `[change] shared-paths`, and it reruns the evals that part A of this release changed. A project of 0.7.0 moves with `spec-dna upgrade`.

### Upgrade from 0.7.0

- Install the 0.7.1 binary. Then run `spec-dna upgrade --pin-file <spec-dna-pin.toml>` on a clean tree. If you have work that is not committed, run `git stash -u` first (`upgrade` also refuses an untracked file), and `git stash pop` after the commit. Commit the paths that `upgrade` prints in one commit. [docs/guides/upgrade-0.6.md](docs/guides/upgrade-0.6.md#run-the-upgrade) gives the report and the steps. The command moves the pin and the sums, and it writes the generated files of 0.7.1. It changes no other key.
- The pin move can share the pull request with other work. SD503 does not count `spec-dna.toml` when it differs from the base only in the pin line and in `[spec-dna-sha256]`, and the base pins an earlier version. SD913 does not report the files that `init` and `upgrade` write. A hook that runs `spec-dna check` refuses each commit while the pin is older than the binary, so the upgrade is the first commit of the branch. The pin stays exact. The release commit of this repository also moves the pin with the version and other files, under the rule of its own checkout ([ADR 0013](docs/adr/0013-self-hosting.md)).
- Each teammate needs 0.7.1, because the pin is exact.
- A task whose text names `spec-dna finish <slug>` with the slug of its change now closes with `finish`, and `next`, `status` and `finish` do not wait for it. A plan of 0.7.0 that has such a task needs no edit. `write-plan` tells agents to write no such task.
- A task can have the label `Actual:`. The line `Hours:` of a commit message can be `Hours: <estimate>, actual <n>`, and the text line of `log` has a new pair `actual <n>` after `hours`. A script that splits this line on spaces must read the new pair. A `log` of 0.7.0 reads no hours from a line that has the actual part, so run `log` with 0.7.1 in a project that uses the label.
- The new key `[change] shared-paths` is a list of globs. `upgrade` does not add it. Add it by hand to keep SD913 from reporting a roadmap or a list of open questions, for example `shared-paths = ["docs/roadmap.md", "docs/questions.md"]`. `init` writes it for a new project.
- A project with the pre-commit hook `typos` needs an exclude pattern, because `typos` can rewrite the 16-digit hash of a spec-dna marker, and a marker with a changed hash gives SD501. Add this under `[default]` in `_typos.toml`, `typos.toml` or `.typos.toml`: `extend-ignore-re = ['(?:h:|@)[0-9a-f]{16}\b']`. `init` and `upgrade` print a note for such a project. [docs/init.md](docs/init.md#spell-checkers) gives the detail.
- A project that moves from 0.6 straight to 0.7.1 gets the skills that are new in 0.7.0 in a `skills` list that is not empty (see "Changed"). A project that is at 0.7.0 keeps its list, because `upgrade` adds them only when the old pin is before 0.7.0. To turn on a skill that your list lacks, add its name to `skills` in `[harness]` and run `spec-dna init`.
- The skills `write-plan`, `specify`, `execute` and `finish` have new text in part B. No eval run came after the text of `specify`, `execute` and `finish`, so their verdicts describe the older text. `write-plan` has a run after the new text (see "Verdicts of the skills, rerun for 0.7.1").

### Fixed

- An open task now gates each change. Before, only a plan that lists no files kept the stage `build` while a task of `Order` was open. With a `covers:` tag for each delta requirement and no task done, `next` gave the stage `deploy` and `finish --dry-run` passed. Now the stage stays `build` for each plan while a task is open (not `[x]` and with no reason in `Skip:`), and `next` names it. `finish` stops at step 0 and lists the open tasks. A change with the intent status `closed` is not checked. The skills `execute` and `finish`, [docs/commands.md](docs/commands.md) and the change guide say the same.
- The 200 records of the pilot of 0.7.0 have token counts. They had `"tokens": null`, because the result files of the pilot had none. The transcripts of the pilot runs hold the `modelUsage` of each run, so the counts come from them, matched by cost inside the transcripts of the same batch. The same match reproduces the counts of the 330 records of the later batches. The verdicts and the table of the release do not change.
- SD606 no longer warns on a document that names the layer folder of the changes (`docs/intent/`) after `finish` deleted the last change folder. Git has no empty folder, so the layer folder left the tree with its last change, and the warning was false: the layer exists, and the next change goes into it. A folder that held another file, the change folder itself and a folder that is no layer folder still give SD606.
- The evals `verify-8` and `verify-9` no longer give SD913 for the right work, except for `RESULT.md`, which the task asks for. The plan of `verify-9` lists `src/invoice.py`, and the plan of `verify-8` lists `docs/interfaces/invoices.md` and `docs/rules/collections.md`. In 4 of 4 runs of `verify-9`, in both arms, the run failed. The check `feature-done` of `verify-8` tests the behavior: a callable that `fees` defines gives 80 for 1,000 and 160 for 2,000, and no such callable gives 50 for 1,000. A run that renames `late_fee` to `overdue_fee` passes. A helper with the old rate kept, and an `overdue_fee` next to a `late_fee` at 5 percent, fail. A `fees.py` that imports `exit` or `sleep` is not a fault. The scenarios `wrong-renamed-old-fee` and `honest-stop` are guards: they give the same result with and without the fix. The evals `verify-6` to `verify-10` have `max-budget-usd` 2.5, because 2 runs with all checks green stopped at 1.55 and 1.54 USD against 1.5. The estimate of `verify` is 100 USD. Issue #68.
- The skill `finish` tells the agent to run `spec-dna finish <slug> --dry-run` alone, with no other command after it, and says that a command that the tool denies is no stop of `finish`. A stop is a line `stopped at step N` that `spec-dna finish` prints. SD910 reason 7 belongs to `spec-dna check --base origin/HEAD`, not to a stop. In the full run, 2 of the 3 failed runs with the skill gave up after a denied command that had `; echo` after it. The stub agent cannot prove this change: the effect is measured by the rerun of the evals. Issue #67.
- The evals `verify-6` to `verify-10` have `max-turns` 40, not 20. In the rerun of 0.7.1 each of 9 runs with the skill stopped at 20 turns: the arm with the skill reads the skills `tdd` and `execute` and makes 1 or 2 commits, so it needs about 2 turns more than the arm without it, and a cap with no headroom scored the turns, not the skill. The 0.7.0 runs had stopped at 19 turns at most.
- A task that only `finish` can do no longer stops `finish`. In 0.7.0, a task such as ``Run `spec-dna finish demo` and commit the result`` kept the stage `build` and stopped `finish` at step 0, and only `finish` could close it. Now a task whose text names `spec-dna finish <slug>` with the slug of its own change closes with `finish`: `next`, `status` and step 0 of `finish` do not wait for it, and its `Hours:` still counts. `status --json` has the new field `tasks.finish`, the number of such open tasks. A task that names `spec-dna finish` with no slug, or with another slug, still stops step 0, and the message says how to write the task. The skill `write-plan` tells agents to write no task for `finish`. The review task of the owner is optional.
- `spec-dna check --base <rev>` with a `<rev>` older than the merge base of `HEAD` and `origin/HEAD` now uses that newer merge base for the change checks and for SD503, and it prints a note on stderr. A squash merge keeps no approval commit, so the older base gave SD910 (reason 4) for work that was approved. The document checks and the choice of the new diagnostics keep `--base`. `next`, `status` and `finish` follow the same rule, with no note. Only `origin/HEAD` starts the rule, not `[change] base`.
- SD923 and SD924 skip an `approved` plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0, and they print one note for each such plan. A later edit of such a plan, also a new task, does not change this, because the check reads the commit that added the plan. A plan that is not committed, and a shallow clone (which has no commit that added the plan), keep the result of 0.7.0. To check such a plan, give each task exact paths in `Files:` and one `Verify:`.
- An upgrade can share its pull request with other work. SD503 does not count `spec-dna.toml` when it differs from the base only in the pin line and in `[spec-dna-sha256]`, and the base pins an earlier version. SD913 does not report the files that `init` and `upgrade` write. The pin stays exact: a pin that differs from the version of the binary still stops `check` with exit code 3. Before, such a pull request gave SD503 and one SD913 warning for each written path. [docs/diagnostics.md](docs/diagnostics.md#files-that-sd503-counts) gives the rule.
- The report of `upgrade` lists a decision with no `status` in its front matter. It listed each decision with no heading `Status`, also when the front matter had `status:`, and spec-dna reads no `Status` section.
- The copy of a project that `eval run --ab` makes has the adapter line of the binary, and `SD501` and `SD502` are `off` in the `[rules]` of the copy, in both arms. Before, a line of another version or build gave SD502 in the copy.
- The check `task-paths-in-files` of the evals `write-plan-1` to `write-plan-5` no longer reads the word `none.` of a task `Files: none.` as a path. Its pattern allowed an empty extension, so a correct plan with the review task of the skill failed the check, and `spec-dna check` accepted the same plan. The pattern needs an extension now, and it no longer ends on a full stop. A project that has an old pack keeps the old check until `spec-dna eval pack` writes the pack again (it keeps a file that exists).

### Added

- The skill `tdd` gives one way to commit the red test when a hook of the project runs the tests: the subject `test: <what the test checks> (red)`, and a skip of only that hook, named in the body of the commit. The commit with the code runs all hooks. The skill `write-plan` can end a plan with a task for the review by the owner (`Files: none`). The task is optional. A person ticks it after the review and before `spec-dna finish` runs. [docs/guides/change.md](docs/guides/change.md) says the same.
- The task label `Actual:` holds the real hours of a task. `finish` prints `Hours: <estimate>, actual <n>`, or `Hours: actual <n>` if no task has an estimate. `finish --json` and `log --json` have the new field `actual` (a number or `null`). The text line of `log` has the new pair `actual <n>` after `hours`, so a script that splits it on spaces must change. A `log` of 0.7.0 cannot read the new line. `stats --sdlc` reads the estimate only. `finish` prints a note for an `Actual:` that is no number and for a done task that has none while another task has one. The skill `write-plan` tells the agent to add `Actual:` when a task is done.
- The key `[change] shared-paths` is a list of globs from the repository root, in the form of `high-risk-paths`. SD913 does not report a path that matches one of them, so a change can edit a roadmap or a list of open questions with no `Files` item. No other check reads the key. `init` writes it for a new project (the top file of each layer of the type `roadmap` or `questions`, and each Markdown file named `roadmap`, `questions` or `open-questions`), and `init --reconfigure` keeps its value. `upgrade` does not add it. [docs/config.md](docs/config.md#the-table-change) gives the detail.
- `init` and `upgrade` print a note for a project that runs the pre-commit hook `typos` and has no exclude pattern. In a test with random hashes, `typos` 1.52.0 reported 142 to 145 of 4,000 marker hashes (`h:<hash>`), and `typos --write-changes` rewrote 34 of them. `codespell` 2.4.3 and `cspell` 8.19.4 reported none. A marker with a changed hash gives SD501. The note gives the line `extend-ignore-re = ['(?:h:|@)[0-9a-f]{16}\b']` for `[default]` in `_typos.toml`. With the pattern, the test found 0 ([docs/init.md](docs/init.md#spell-checkers)).
- `upgrade` prints a line `next:` after the paths to commit. It says whether the pull request can hold other work (when only the pin line changes) or needs a pull request of its own. For a tree that is not clean, the message tells you to run `git stash -u`, then `spec-dna upgrade`, then to commit its paths and run `git stash pop`.

### Changed

- Each generated agent file ends with one paragraph about the scratch folder. A subagent makes its own subfolder in the scratchpad of the session, puts there each temporary file and each file of a browser tool, and deletes only the files that it put there and its subfolder. It never deletes or changes another file of the scratchpad, and it never deletes the scratchpad folder. It never writes these files into the repository. The rules never limit an edit of the repository that the task needs. The text of `verifier` no longer tells it to remove a shared temporary directory. `init` does not change `.gitignore`.
- The evals of `brainstorm` and `execute` are harder. Both arms passed each run of `brainstorm-1` to `brainstorm-5` (15 of 15 against 15 of 15) and of `execute-1` to `execute-5` (15 of 15 against 15 of 15) in the full run of 0.7.0, so an eval could not show what the skill adds. `brainstorm-6` to `brainstorm-10` and `execute-6` to `execute-10` replace them, as `verify-6` to `verify-10` replaced the evals of `verify`. A task of a new eval names no rule of its skill, and each eval has one trap and a shell check that finds it ([docs/evals.md](docs/evals.md#the-traps-of-the-harder-evals)). The pair `agents-line` runs `execute-10` in place of `execute-1`. The records of 0.7.0 keep the old ids.
- `upgrade` adds the skills that are new in 0.7.0 (`tdd`, `execute`, `verify`, `finish`, `debug` and `post-mortem`) to a `skills` list that is not empty, when the old pin is before 0.7.0. It adds each skill that `init` proposes for the profile and the level of the project, so a 0.6 project gets `tdd`, `finish` and `post-mortem` in the profile `standard` at the level `lean`, and `finish` only in `lite`. An empty list stays empty, a project with the policy `none` gets none, and a list that cannot take the new names keeps the rename and gets a warning with the names to add by hand. The report names each skill that it added ([docs/guides/upgrade-0.6.md](docs/guides/upgrade-0.6.md#default-skills-that-upgrade-adds)).
- The skills `specify` and `write-plan` give a documented way back to `draft` after an approval: set `status: draft` in the commit of the edit, skip only the hook that runs `spec-dna check` for this commit, name the skip in the body of the commit, and let the approver make the approval commit with all hooks on. The message of SD910 reason 9 and [docs/guides/change.md](docs/guides/change.md) say the same. The skill `execute` asks the person before it adds a path to `Files`, and with a yes it adds the path to `plan.md` in the commit of the edit.
- The tests have a fixture of a project that the real binary 0.7.0 wrote (`tests/fixtures/upgrade/v070/`) and a proof that `upgrade` on it moves the pin and the generated files only, and that a second run changes nothing. The same proof runs on a copy of this repository. The case `pin-0.6.0` of `upgrade_v061` ran at the wrong pin, and it is fixed.

### Verdicts of the skills, rerun for 0.7.1

The evals that this part changed ran again with 15 runs for each arm (3 runs of each of 5 evals), with `claude` 2.1.327 (`write-plan` with 2.1.328). The table shows the old and the new result. The table of the release (`spec-dna eval verdict --release`) holds the new rows. The default skills do not change: `brainstorm` and `finish` are on unless a project sets them `off`, `tdd` was `on` already, and `execute` and `verify` are not `on` (D33).

| Skill | 0.7.0, with / without | 0.7.1, with / without | Difference (90% interval) | Cost per correct, with / without (USD) | Verdict |
| --- | --- | --- | --- | --- | --- |
| `finish` | 12/15, 15/15 | 15/15, 15/15 | 0.00 (0.00 to 0.00) | 0.80 / 0.99 | on (rule 2) |
| `verify` | 4/10, 7/10 | 15/15, 14/15 | +0.07 (0.00 to +0.20) | 1.93 / 1.77 | inconclusive |
| `brainstorm` | 15/15, 15/15 | 13/15, 11/15 | +0.13 (0.00 to +0.33) | 0.90 / 1.41 | on (rule 2) |
| `execute` | 15/15, 15/15 | 15/15, 15/15 | 0.00 (0.00 to 0.00) | 1.39 / 1.33 | inconclusive |
| `tdd` | 6/10, 0/10 | 11/15, 0/15 | +0.73 (+0.47 to +0.93) | 1.40 / - | on (rule 1) |
| `agents-line` | 10/10, 6/10 | 15/15, 6/15 | +0.60 (+0.20 to +1.00) | 1.14 / 3.22 | on (rule 1) |
| `write-plan` | 15/15, 8/15 | 13/15, 10/15 | +0.20 (-0.20 to +0.60) | 1.88 / 2.40 | inconclusive |

- The row of `agents-line` of 0.7.0 used `execute-1`, and the pair uses `execute-10` now, so the 2 rows do not compare one to one.
- `finish` and `verify` no longer score below no skill. The effect of the new text of `finish` is not proved: 3 runs failed in 0.7.0, none fails now, and the difference to no skill is 0.
- `execute` stays at the ceiling in both arms, as the evals of `execute-6` to `execute-10` were meant to prevent. These evals do not show what the skill adds.
- `brainstorm` is `on` by rule 2 (the cost), with a pass-rate difference of +0.13 that includes 0.
- The first run of `verify` stopped each run with the skill at the cap of 20 turns, so the evals have 40 turns now. That run is not in the table.
- `write-plan` was `on` (rule 1) in 0.7.0, with 15 of 15 against 8 of 15. It is `inconclusive` now: 13 of 15 against 10 of 15, and the interval of the difference includes 0. Both runs that failed with the skill stopped at the cap of 2 USD for a run, and each passed all its checks. The cost for a run did not grow (1.63 USD with the skill, 1.60 USD without it). The new text of the skill (the review task, `Actual:` and the step back to `draft`) is in this run, and `claude` was 2.1.328.
- A first run of the same evals gave 9 of 15 against 6 of 15. The check `task-paths-in-files` read the word `none.` of the valid task `Files: none.` as a path, and so 4 of the 6 failures with the skill came from the check, not from the skill (see "Fixed"). The records of that first run stay in `bench/results/evals/20261119T082310Z-write-plan.jsonl`. The table reads the newer batch.
- The table of the release has the new row for `write-plan`. The default skills do not change, because the constant `DEFAULT_ON_NEEDS_VERDICT_ON` is `false`.

## 0.7.0 - 2026-11-18

0.7.0 adds the workflow pack of 11 skills with evals, the approval of a person in the branch, the checks of a plan and its hours, and the import from Spec Kit and OpenSpec. A project of 0.6 moves with `spec-dna upgrade`.

### Upgrade from 0.6

- Install the 0.7.0 binary. Then run `spec-dna upgrade --pin-file <spec-dna-pin.toml>` on a clean tree, commit the files that it prints in one pull request, and let each teammate install 0.7.0. [docs/guides/upgrade-0.6.md](docs/guides/upgrade-0.6.md) gives the steps, the report and the undo. Before the upgrade, each command of 0.7.0 stops with exit code 3 on a 0.6 file, and `upgrade` is the command that reads it.
- `upgrade` moves the pin, renames the skills of 0.6 in `[harness] skills` (`intent` is `brainstorm`, `spec` is `specify` and `clarify`, `plan` is `write-plan`; `review` stays), writes the generated files of 0.7.0, removes the unedited skill files of 0.6, proves that no id and no requirement text changed, and prints a report. It applies no preset of a profile and changes no other key. It never commits.
- Skills that are on by default: `brainstorm`, `specify`, `clarify`, `write-plan`, `tdd`, `review`, `finish` and `post-mortem`, and the line of `AGENTS.md`. `execute`, `verify` and `debug` are off, because no verdict `on` in the table of the release names them (see "Verdicts of the skills"; the rule is [ADR 0023](docs/adr/0023-default-skills-from-the-release-table.md)). The 7 skills of the design are on unless their verdict is `off`, and no verdict is `off`. `upgrade` keeps a name that your list has, and it leaves out a new name that is off by default, with a line in the report. Add a name to `skills` in `[harness]` and run `spec-dna init` to turn it on. `spec-dna skill show <name>` prints the text of a skill that is off.
- A `skills` list of many lines, where the rename changes the length, stops `upgrade` with exit code 3. Write the list by hand and run `upgrade` again.
- The cache has a new format, so each project rebuilds it once. Some `--json` outputs have new fields. The field `class` can be `null`, and the text line of `log` has 2 new tokens (see "Changed").
- Each teammate needs 0.7.0 for the hook `protect-approvals`. An upgraded project keeps `approval = "base"`.
- The undo needs 1 or 2 pull requests, because 0.6 counts the skill file of a new name (SD503). The guide gives the steps. They ran on a project that the binary 0.6.1 wrote.

### Verdicts of the skills

The full run measured each skill against no skill in a fixture project, with `claude` 2.1.326 and 2.1.327 (the models `claude-fable-5-2`, `claude-opus-5-5`, and `claude-sonnet-5-5` and `claude-haiku-5-5` for some evals). A pair is one run with the skill and one run without it. The verdict follows the rule of D33: a 90% interval of a paired bootstrap (2,000 resamples, seed 24). The records are in `bench/results/evals/`, and `spec-dna eval verdict --release` prints the table.

| Skill | With | Without | Difference (90% interval) | Cost per correct, with / without (USD) | Verdict |
| --- | ---: | ---: | --- | --- | --- |
| `brainstorm` | 15/15 | 15/15 | 0.00 (0.00 to 0.00) | 0.81 / 0.90 | inconclusive |
| `specify` | 15/15 | 11/15 | +0.27 (0.00 to +0.60) | 1.40 / 2.06 | inconclusive |
| `clarify` | 15/15 | 12/15 | +0.20 (0.00 to +0.47) | 0.68 / 1.07 | on (rule 2) |
| `write-plan` | 15/15 | 8/15 | +0.47 (+0.13 to +0.80) | 1.65 / 2.79 | on (rule 1) |
| `execute` | 15/15 | 15/15 | 0.00 (0.00 to 0.00) | 1.41 / 1.46 | inconclusive |
| `tdd` | 6/10 | 0/10 | +0.60 (+0.20 to +1.00) | 1.96 / - | on (rule 1) |
| `review` | 12/15 | 10/15 | +0.13 (0.00 to +0.33) | 2.00 / 2.61 | inconclusive |
| `verify` | 4/10 | 7/10 | -0.30 (-0.60 to 0.00) | 3.14 / 1.47 | inconclusive |
| `finish` | 12/15 | 15/15 | -0.20 (-0.40 to 0.00) | 0.95 / 1.01 | inconclusive |
| `debug` | 15/20 | 13/20 | +0.10 (-0.10 to +0.30) | 0.98 / 0.96 | inconclusive (pilot) |
| `post-mortem` | 7/15 | 0/15 | +0.47 (+0.20 to +0.73) | 3.18 / - | on (rule 1) |
| `agents-line` | 10/10 | 6/10 | +0.40 (+0.10 to +0.70) | 1.18 / 1.98 | on (rule 1) |

`agents-line` is the line of `AGENTS.md`, not a skill. The first run of `specify` had a defect in its check (see "Known limits"), so the table holds the rerun with the fixed checks (15 + 15 runs, `claude` 2.1.327); the records of the first run stay in `bench/results/evals/` as history. The 530 runs of the records cost 610 US dollars. The account spent about 621 US dollars in all with the smoke runs and the attempts that a refusal cut off, below the cap of 660. `verify`, `tdd` and `agents-line` have 2 runs for each eval, and `execute` has 3, so their verdicts are weak.

### Benchmark

The rule of the release (owner decision of 2026-11-18): both binaries run in one job, on one runner, in turns, and the median of the paired ratios head / base of the mode `full` is at most 1.10. The workflow `bench` does it with the inputs `ref` and `base_ref` (`bench/tool/pair.py`). The run measured `v0.6.1` and the head `defa999` on the synthetic medium corpus (1,194 documents, `ubuntu-24.04`), with 100 pairs for each mode:

| Mode | `v0.6.1` median (ms) | 0.7.0 median (ms) | Median of the paired ratios | Mean | Range |
| --- | ---: | ---: | ---: | ---: | --- |
| `full` | 64.5 | 66.4 | 1.024 | 1.026 | 0.987 to 1.077 |
| `warm` | 40.7 | 42.8 | 1.058 | 1.056 | 0.999 to 1.126 |

The rule is met: 1.024 is below 1.10, and no pair of `full` is above 1.08. The first rule (2 pairs of separate jobs, the mean of the medians) gave 1.20 on one pair and 1.01 to 1.11 for other statistics, because a shared runner drifts between 2 jobs by more than 10%. The paired run removes that drift. A check of the owner on one machine (60 alternating runs of both binaries on the medium corpus) gave +3% for `full` and `warm`.

The same job measured the head alone on the medium corpus (15 timed runs for each mode). The speed targets hold: `warm` 76.5 ms (target 1,000 ms), `incremental` 67.7 ms (target 100 ms) and `context_0` 21.1 ms (target 200 ms). `full` is 100.6 ms, `cold` 100.9 ms and the start of the binary 0.6 ms. On the large corpus (11,820 documents, 15 runs) the head gives `full` 663.3 ms, `warm` 488.1 ms, `incremental` 423.1 ms and `context_0` 110.8 ms. The targets are for a mature corpus of about 2.3k documents, so the medium corpus is the gate, and the large corpus is a second point (a deviation that [docs/design/0.7.0.md](docs/design/0.7.0.md#15-deviations-during-the-build) records).

### Known limits

- The first run of the eval `specify` had a defect: its check `at-approval-clean` failed on open concerns and questions (SD919, SD922) that `specify` is meant to write, in 29 of 30 runs and in both arms. The checks are fixed, the rerun gave 15/15 against 11/15, and the verdict stays `inconclusive` (the interval of the difference touches 0). The skill text did not change. The fix also lets `delta-line` and `no-unaccepted-decision` of `specify-4` accept a `CHANGE` of the existing requirement and a mention of the decision that is not a link.
- `execute`, `verify` and `debug` have no verdict `on`, so they are off by default. `finish` (12/15 with the skill, 15/15 without) and `verify` (4/10 against 7/10) scored below no skill, and the evals of `brainstorm` and `execute` hit the ceiling (15/15 in both arms), so their verdicts say little. Issues for 0.7.1 track the study of the failed runs.
- The 200 records of the pilot (part A, 5 files of 2026-11-17 08:28 to 08:29) have no token counts (`tokens` is null), because the result files of the pilot did not hold them. The records of the full run have them. The pilot records stay as history.
- `import` reads the layouts of Spec Kit 1.4.1 and OpenSpec 2.0.0 only. The versions Spec Kit 1.4.2 and OpenSpec 2.1.0 were not tested. [ADR 0022](docs/adr/0022-the-plan-of-the-import.md) and [docs/commands.md](docs/commands.md#import) list the other limits.
- The hook `protect-approvals` reads the text of a command. It is a guard for an agent that follows rules, not a security control, and it lets a command pass when its input is larger than 1 MiB.
- The eval run used one machine and one account, so the costs are of that account.
- The benchmark gate ran on the synthetic corpus of the `bench` workflow and on a shared runner, not on a consumer corpus (a deviation that [docs/design/0.7.0.md](docs/design/0.7.0.md#15-deviations-during-the-build) records).
- ADRs 0018 to 0023 have the status `proposed`. The owner accepts them.

### Added

- The command `spec-dna import speckit|openspec [--dry-run] [--skip-open] [--areas <file>] [--keep none|research]` moves a project of Spec Kit 1.4.1 or OpenSpec 2.0.0 into the layout of spec-dna ([ADR 0014](docs/adr/0014-import.md)). It needs a clean tree and a project that `init` made. The constitution, a done feature and a domain spec move with `git mv` and keep their old id as an alias. An open feature or an open change becomes a draft intent, spec and plan, and its sources stay. The proof of 5 steps compares the sentence of each requirement, checks the states of the source files, the ids, the new errors of the codes SD101 to SD106, SD201 to SD204, SD301 to SD306 and SD401 to SD405, and a second run on the planned tree. The import changes no sentence, so a line that the target rejects (`SC-001` with `MUST`) stops the write. If `spec-dna.toml` needs the legacy key `FR-{n}`, an area or a type, the first run writes only that file. `--keep research` moves each Markdown file of an archive or an extra file to the research layer. A source file that is not valid UTF-8 stops the plan. No line of a source with a word of BCP 14 may vanish: the proof stops the write, and the report lists another dropped line. The import stages each file that it wrote, so a plain `git commit` holds the whole import. A source that is a document of the project gets the line `moved-to` and passes its id to its target as an alias, so `git rm` of the source is no lost id. If no root can hold a type that a layer holds, the first run adds a root `docs` of the type `mixed`. `--json` has the fields `sources`, `pairs`, `proof` and `report`. Exit codes: 0 done or nothing to change, 1 a stop or a failed proof, 2 usage, 3 an unknown layout, no project or a bad file, 4 git or write error.
- The new keys `[change] approval` (`"base"` or `"branch"`, default `"base"`) and `[change] approvers` (a list of author e-mails). With `approval = "branch"`, SD910 also counts an approval commit of an approver in the branch, if it comes before the work that it approves. The commit is not a merge commit, its author e-mail is in `approvers`, and it changes the `status` line of one artifact file only. The check needs the full git history, so use `fetch-depth: 0`. A shallow clone, and a merge of another branch into the branch, give the result of `approval = "base"`. The message of SD910 names one of 8 reasons and its fix. `approval = "branch"` with `profile = "strict"`, and with an empty `approvers`, is exit code 3. `init` writes `approval = "branch"` and the e-mail of `git config user.email` for `lite` and `standard`, and `approval = "base"` for `strict`. An upgraded project keeps `"base"` ([docs/config.md](docs/config.md#the-table-change), [docs/guides/change.md](docs/guides/change.md#the-approval-commit), [ADR 0012](docs/adr/0012-approval-in-the-branch.md)).
- `check --json` has the field `approvals`, after `locked_links`: the approval commits that cleared a gate, each with `commit`, `path` and `author`. The list is empty with `approval = "base"`.
- `status --json` has the object `approved_in` for each change, with the keys `intent`, `spec` and `plan`. A value is `"base"`, `"branch <short sha>"` or `null`. With `approval = "branch"`, the text of `status` shows the same source after each `accepted` or `approved` status. With `approval = "branch"`, a line of `next` that asks for an approval tells an approver to commit the `status` line alone.
- The codes SD923 `task-incomplete`, SD924 `task-too-large` and SD925 `checkbox-missing`, in the rule group `changes`: off in `lite`, a warning in `standard`, an error in `strict`. SD923 and SD924 read the tasks of an `approved` plan: a task needs `Files:` with exact paths (no glob, no folder) and `Verify:`, and it lists at most `[change] max-task-files` paths. A `draft` plan is not checked. SD925 reports an item of `Concerns` or `Open questions` with no task box. The spec template writes `- [ ] {{An open concern.}}`.
- The new key `[change] max-task-files`: an integer from 1 to 20, default 3. It is the limit of SD924.
- A `Files` section can state "no files" with a paragraph whose first word is `none`, followed by `:`, `.` or the end of the paragraph (`None.`, `None: this change is research.`), as with the item `- none`. Another paragraph under `Files`, with no list item, gives the line "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`" in `next`, `status` and the messages of SD913 and SD915.
- One item whose first word is `none`, followed by `:`, `.` or the end of the text, states "nothing" in `Interfaces`, `Concerns` and `Open questions`, with or without a task box: `- none`, `- None.` and `- [ ] None.` give no SD914, no SD919, no SD922 and no SD925. `- None of the risks is open` is an item.
- An item of `Interfaces` can be `ADD <interface id>: <text>` for a new interface. Its place must give the type `interface`. When `finish` runs, or the diff deletes the folder of the change, the document must exist, or a delta line `ADD <id>#<key>` must add it. Else SD914 reports it.
- The parser keeps the first paragraph of `Files` of a plan.
- A task of `Order` can have the label `Hours: <n>` or `Hours: <a>-<b>`, and the label `Skip: <reason>` for a task that is not needed. The hours of the tasks are the one source of the hours of a change. `finish` (also with `--dry-run`) adds the hours of the tasks that are not skipped, and prints the line `Hours: <low>` or `Hours: <low>-<high>`. It gives a note for a value that is no number and no range (and then no total), and a note for the tasks with no `Hours:`. `finish --json` has the field `hours`: `null`, or `{"low", "high"}`. The person or the agent that commits puts the line in the commit message, because `finish` makes no commit. The section `Hours` of a plan is not read. `log` and `stats --sdlc` read the last line `Hours: <number>` or `Hours: <a>-<b>` of the message of the commit that deletes `intent.md`: `log` has `hours <n>`, `hours <a>-<b>` or `hours -` in its text line and the field `hours` (`{"low", "high"}` or `null`) in `--json`. `stats --sdlc` has one new row, "Hours of work per finished change" (Build, lagging, unit `hours`), and a range counts as its midpoint.
- A skipped task (a task with a reason in `Skip:`) is closed, whatever its box. `next`, `status` and the counts treat it as closed. The line of the tasks of `next` ends with `, <n> not needed`, `status` has the token `skipped <n>` after `tasks <done>/<total>`, and `status --json` has `tasks.skipped`. `done` counts only the tasks with `- [x]` and no `Skip:`. SD923 reports a `Skip:` with no reason. The skill `write-plan` and the change guide name both labels.
- The code SD612 `link-into-change`: a reference from a document outside the folder of a change into the folder, that step 0 of `finish` stops on, when SD611 and SD901 do not report it. It is a warning in each profile, in no rule group.
- An amendment section that names the dead target of a locked link clears its info line. For a change, the pointer names the commit that `log` gives (`finished in commit <id> on <date>`), and `locked_links` in `check --json` has the new field `commit`.
- An open follow-up of an incident names the skill `post-mortem`: the text line of `status` ends with `[skill: post-mortem]`, and each item of `follow_ups` in `status --json` has `"skill":"post-mortem"`.
- The workflow pack: 11 skills in place of the 4 stage skills of 0.6. The names are `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify`, `finish`, `debug` and `post-mortem`. `init` writes them as `.claude/skills/spec-dna-<name>/SKILL.md`. The default of each skill follows the rule of ADR 0023 (see below): in 0.7.0, `execute`, `verify` and `debug` are off by default and the 8 other skills are on. Each description starts with "In a spec-dna project", and each body has 5 parts: when to use it, the commands, the artifact, the check and the action when the check fails. A test fixes the limits: 160 characters for a description, 2,500 bytes for a body, and 1,250 bytes (7 skills) or 1,900 bytes (11 skills) for the listing of the descriptions. [docs/harness.md](docs/harness.md) has the table.
- The command `spec-dna skill list` and `spec-dna skill show <name>`, with `--json`. `list` prints the kind, the stage, the state in `[harness] skills` and the default of each skill. `show` prints the body of a skill with no front matter and no marker, also for a skill that is off, so an agent with no skill folder reads the text of the step that `next` names. An unknown name gives exit code 2. The command reads no document and writes nothing.
- The adapter skill has the block of the 5 disciplines (test first, cause first, proof before done, review is data and isolated work), with 700 bytes or fewer. It also has the rule that a spec-dna skill replaces a general skill of the same stage.
- `init` prints a note when `enabledPlugins` of `.claude/settings.json` names a plugin with the word `superpowers`. It reads no file of the home folder, and it does not move a file that the plugin writes.
- `upgrade` renames the skills of 0.6 in `[harness] skills` before it reads the file: `intent` is `brainstorm`, `spec` is `specify` and `clarify`, and `plan` is `write-plan`. A second run changes nothing.
- The evals of the skills ([ADR 0016](docs/adr/0016-skill-evals.md)). `spec-dna eval pack` writes the pack to `<[evals] dir>/pack/`, with the fixtures and the key `skill` of an eval file. `spec-dna eval run --ab <skill|all|agents-line>` runs each eval with the skill and without it, in pairs, and writes `<[evals] dir>/results/skills-<YYYYMMDD>T<HHMMSS>Z.json`. The result has the pass rates, the costs, the difference and the cost ratio with a 90% interval (a paired bootstrap, 2,000 resamples, seed 24), and a verdict (`on`, `off` or `inconclusive`). `spec-dna eval verdict` prints the verdict of each skill, from the newest result file that holds it, then from the table of the release. It starts no agent. The table of the release is filled from the committed records.
- An attempt of `eval run --ab` that the API refuses (401, 403, 429, a 5xx status or a page of Cloudflare) is excluded, and no result counts it. The runner tries it again after 60, 120, 240, 480, 960 and 1,920 seconds. After the last wait, the command stops with exit code 3 and `"stopped": "api-error"`. The result reports the excluded attempts of each arm. `SPEC_DNA_EVAL_BACKOFF` sets the first wait in seconds (0 to 3,600). `eval run` without `--ab` is unchanged.
- `eval run --ab` details that the first run of the pack needs: `--ab` isolates the runs by default (`--no-isolate` switches it off). An account limit, a credit or a login problem (`You've hit your limit`, `Credit balance is too low`, `Not logged in`) waits with the ladder 60, 120, 240, 480, 960 and 1,920 seconds, then 1,800 seconds, up to 6 hours in total. The other refusals keep the 63-minute ladder. An empty output with a non-zero exit, and a lost connection, are refusals. `Context limit reached` is a run. After 5 counted runs in a row with an error and no cost, the command stops with `api-error`.
- `eval run --ab` writes the result after each arm of a pair, and SIGINT and SIGTERM write it too (`"stopped": "interrupted"`, exit 130 and 143). `--resume <result file>` goes on in that file, and writes one merged result. A git or file error writes the partial result before exit code 4 (`"stopped": "error"`).
- A run of the agent that takes more than 25 minutes, and a check that takes more than 5 minutes, are killed with their process group and count as failed (subtype `timeout`). `SPEC_DNA_EVAL_TIMEOUT` and `SPEC_DNA_EVAL_CHECK_TIMEOUT` set the limits in seconds.
- The agent and the checks of an eval run with the folder of the running binary first on `PATH`, and with a neutral git identity and no signature. The files `AGENTS.md` and `CLAUDE.md` of a fixture keep their text in `--ab`: the copy adds only the generated lines of the project.
- `eval run --ab` allows the tool `Skill` in both arms and records `permission_denials` for each side of a pair. The cap is checked before each run. `--dry-run` prints the estimate also above the cap.
- `eval run --ab` excludes a silent error like a refusal: a result with an error and a cost of 0, or a plain text or no output with a non-zero exit, and no marker. The charged cost is 0, 5 in a row stop the command with `api-error`, and a real run resets the row. The messages `5-hour limit reached ∙ resets 3pm` and `Weekly limit reached ∙ resets Mon 9am` are account limits. `Context limit reached` stays a run.
- The result of `eval run --ab` has the key `runs`, and `--resume` uses it: a different `--runs` is a usage error. SIGHUP is trapped like SIGTERM (exit 129). The cost of an agent run that a signal interrupts during its checks stays in `spend_usd`.
- The code SD507 `skill-unproven`: a skill that is off by default and is in `[harness] skills` has no verdict `on`. It is off in `lite` and `standard`, and a warning in `strict`.
- `spec-dna eval records <result file>` and `spec-dna eval verdict --release`. `eval records` prints one record for each run of a result file of `eval run --ab`, as JSON Lines. A record has 9 keys (`skill`, `eval`, `run`, `arm`, `pass`, `cost`, `tokens`, `models`, `claude_code`) and no path and no text. The command builds each record from 9 values, and it computes each item again from its records, so a result that the records do not repeat stops it. `bench/results/evals/` has the records of 500 runs (the pilot of part A and the full run), and the binary holds the table of the release as a constant that a test compares with them. `eval verdict --release` prints the table ([docs/evals.md](docs/evals.md#the-records-of-a-release)).
- Each side of a result of `eval run --ab` has `tokens` (the sums over `modelUsage`, or `null`), `models` and `checks` (the name and the result of each check, no command).
- The key `skill` of an eval file also takes `agents-line`. The pack has 57 evals: `verify-6` to `verify-10` replace `verify-1` to `verify-5`, which both arms passed in each run, and `agents-line-1` and `agents-line-2` join the pair `agents-line` in place of `verify-1` and `review-1`.
- The command `spec-dna approve <slug> intent|spec|plan`, for a person. It sets the `status` line of one artifact (`accepted` for an intent, `approved` for a spec or a plan) and commits that file alone, so the commit is an approval commit of the branch rule. It needs a tracked file with the text of `HEAD`, the status `draft`, the earlier artifact approved, and, with `approval = "branch"`, an author in `approvers`. If git fails, the file gets its old text back. Exit codes: 1 a refusal, 2 usage, 3 configuration, 4 git error, 5 no folder ([docs/commands.md](docs/commands.md#approve)).
- The flag `init --approver <e-mail>` (it can repeat) and the answers key `approvers`, and a second prompt of question 5 for the approvers. `init` never writes the address of an agent or a bot into `approvers`: `noreply@anthropic.com` and each address that ends with `[bot]@users.noreply.github.com`. Such an address in the flag or in the answers is exit code 2. With no person to list, `init` writes `approval = "base"` and a note. A git e-mail of an agent, as in a cloud session, no longer becomes an approver ([docs/init.md](docs/init.md#the-approvers), [ADR 0018](docs/adr/0018-approval-by-a-person.md)).
- The code SD926 `approver-agent`, in the rule group `changes`: a warning in each profile. It reports each item of `[change] approvers` that is the address of an agent or a bot, on the line of the key. A project that wants an agent identity as an approver sets `[rules] SD926 = "off"`. By default, `approvers` lists persons only, so the advice of ADR 0012 to list the e-mail of the agent for a routine change ends.
- The key `[harness] protect-approvals` (a boolean) and the hook `spec-dna hook protect-approvals`, on `PreToolUse` of `Bash`. The hook denies a call of an agent that runs `spec-dna approve`, a call that writes `accepted` or `approved` and runs `git commit`, and a `git commit` that moves the status of an artifact of a change to its approval value. `init` writes `protect-approvals = true` for `strict` in a file that has no such key. The hook needs spec-dna 0.7.0 on the machine of each teammate. [docs/harness.md](docs/harness.md#spec-dna-hook-protect-approvals) gives the limits: the hook reads the text of the command, and it is not a security control.
- The tag `covers: partial <doc-id>#<key>` records that a test proves a part of a requirement. A partial tag is not evidence: alone, it does not clear SD608, SD916, step 5 of `finish` or the stage `test` of `next` and `status`. A full tag or a `verified-by` of the same requirement is evidence, also next to a partial tag. A partial tag that names no requirement gives SD105. A binary before 0.7.0 reads no id after the word `partial`, so it never reads such a tag as full evidence. The messages of SD608 and SD916 name the partial tag.
- `show --json` gives each item of `evidence` the field `partial` (a list of `<path>:<line>`, always present), and the text of `show` adds `; partial: <tags>`. `status --json` gives each change the integer field `partial`, and the text of `status` adds the token `partial <n>` when `<n>` is above 0. `next` names the partial tags in the stage `test`.
- A 16th row of `stats --sdlc`, after "Regressions caught in CI": "Share of requirements with partial evidence only" (play Test, kind lagging, unit percent). It reads the tree of `HEAD`. A script that reads a row of `stats --json` by its place must read it by `name`.
- The guide of a change has the section "A delta that depends on a spike": the delta of an approved spec stays `approved` after an edit, and the edit merges before the pull request of `finish`.
- The default of a skill follows one rule ([ADR 0023](docs/adr/0023-default-skills-from-the-release-table.md)): the 7 skills of the design (D2), the verdict of the skill in the table of the release, and the constant `DEFAULT_ON_NEEDS_VERDICT_ON` of `src/verdict.rs`. With `true`, a skill is on by default only with the verdict `on`. With `false`, a skill of the 7 stays on unless its verdict is `off` (D33), and another skill needs the verdict `on`. The constant is `false` in 0.7.0, and `clarify`, `write-plan`, `tdd` and `post-mortem` have the verdict `on`, besides the line of `AGENTS.md`. `init`, `upgrade`, `skill list` and SD507 use the rule.
- `init` expands the profile of a new project (D37): `lite` gets `two-class`, `warn`, no `REVIEW.md`, `protect-tests = "off"`, no `[evals]` table and the skills of `brainstorm`, `write-plan` and `finish` that are on by default (all 3 in 0.7.0). `standard` and `strict` get `three-class`, `block`, `REVIEW.md`, the starter files of the evals and each skill that is on by default. `strict` also gets `approval = "base"` and `protect-approvals = true`. The level `full` proposes the 11 skills for each profile. The answers to the questions replace each default. [docs/profiles.md](docs/profiles.md) has the table.
- `init` for `strict` with the answer `skeleton` writes no eval pack, and it prints a note that names `spec-dna eval pack` (the pack has 57 evals in 812 files). `--json` has the note in `notes`.
- `init --reconfigure` with another profile than the file keeps the current values, and it prints a note: the profile sets other defaults only for a new project. A part that the file does not set (`skills`, `review-md`, `protect-tests`) gets the default of the profile of the file.
- The fixture `tests/fixtures/upgrade/v061/`: a project that the real binary 0.6.1 wrote, with a change that has an accepted intent, an approved spec and an approved plan, and the tests of its upgrade (`tests/upgrade_v061.rs`). `make.sh v061` records it.

### Fixed

- A `Files` item with more than one code span gives each span as a path. Before, it gave only the first span, so a file that a later span names got a false SD913. A span that holds a template placeholder is still no path.
- The tasks under a heading of a lower level inside `## Order` (`### Part two` under `## Order`) are tasks of the plan. Before, the heading ended the section, so `next`, `status` and `finish` did not see these tasks: a plan with `Files: none` moved to the next stage with every task open, and SD923 and SD924 skipped them. A heading groups tasks and is not a task, and the section goes on after a subsection with another name (`### Open questions`) until a heading of the level of `Order`. After the upgrade, such a plan can show open tasks, and SD923 or SD924 can report them. The version of the parser moves to 16, so the index of an old project is rebuilt once ([docs/types.md](docs/types.md#sections-that-carry-list-items)).
- The check `at-approval-clean` of the 5 evals of `specify` no longer fails on an open concern or an open question (SD919, SD922): `specify` writes them, and `clarify` closes them. The checks `delta-line` and `no-unaccepted-decision` of `specify-4` accept a `CHANGE` of the requirement `audit-log` and a mention of the decision `sqlite-later` that is not a link to it.

### Changed

- The generated line of `AGENTS.md` and `CLAUDE.md` has no space before its marker `<!-- spec-dna v... -->`, so a version up to `99.99.99` fits in 400 bytes. The text of the line and its hash do not change. `check` gives SD502 for a line with the space, and `init` and `upgrade` write the new form.
- `eval run --ab` starts when its estimate is above `max-total-usd`. It prints a note and stops before a run that could pass the cap, and `--resume` goes on. `--resume <file> --runs <n>` with `n` above the runs of the file adds runs to each eval. `eval run` without `--ab` keeps the refusal. `--dry-run` says that a real run starts.
- The evals of `execute`, and `tdd-1` and `debug-1`, have `max-turns` 40 (it was 25 or 20): the pilot runs of `execute-1` with the line of `AGENTS.md` stopped at the cap with correct work. The fixture of `execute-1` has 2 requirement keys, so a `covers:` tag can name one.
- `new intent`, `new spec` and `new plan` write the H1 `{{The title of the change.}}` in place of a title from the slug. SD306 reports it until the author writes the title.
- The cache of the parsed data has the format 2, and `PARSER_VERSION` is 15, so each project rebuilds its cache once. An older binary drops a cache of the new format.
- JSON shape: `class` in `next --json` and in each change of `status --json` is `null` when the intent has no valid class (the field is absent, empty or invalid). A reader that expects a string must accept `null`. The text of `status` shows the token `none`, and the first line of `next` says `(class not set)`. The stage of such a change is `plan`, and the last line asks for the class before the approval, because an approval commit changes the `status` line alone. `check` still reads a change with no class as `routine`.
- JSON shape: `log --json` has the field `hours` in each change (`{"low", "high"}` or `null`), and the text line of `log` has the 2 new tokens `hours <n>` before the commit. A script that reads the line by position must change. The info line of a locked link says `names <target> and points to <pointer>`.
- `next` and `status` ask to approve a spec or a plan only when the file exists. With no file, the last line asks to write it with `spec-dna new`.
- SD910 does not report a `spec.md` or a `plan.md` with the status `draft`. Code under `Files` still needs an `approved` plan. A project that used SD910 to stop a draft artifact loses that result, also with `approval = "base"`. A draft artifact can now travel in the pull request of the approval of the artifact before it ([docs/diagnostics.md](docs/diagnostics.md#checks-of-the-change-lifecycle), the amendments of 2026-11-16 of ADR 0006, ADR 0012 and ADR 0014).
- The message of SD910 names one of 9 reasons (`reason 1` to `reason 9`), also with `approval = "base"` (`reason 1`). The `fix` text of SD910 lists the 9 reasons with their fixes. A tool that matches the old text of the message or of the `fix` field must match the new one.
- SD910, reason 9 ([ADR 0019](docs/adr/0019-limits-of-0-7-0-part-a.md)). With `approval = "branch"`, an approval commit of the branch clears work only while the artifact keeps its approval status at the head. A later commit, or the working tree, that sets it back (a plan back to `draft` after the code) gives reason 9 to the work that used the approval, and to the work between the step back and the next approval commit. A new approval commit clears the work after it. An artifact that the head does not have keeps its approval. An approval from the base tree keeps the rule of 0.6, and SD401 reports the step back. A branch that sets an approved artifact back and keeps its work now fails, where it passed in earlier builds of 0.7.0. The work after a step back gave reason 2 before.
- SD503: the compare of the pin line in a tree binary reads `\r\n` as `\n`, so a working tree with CRLF line ends gives no SD503 for a pin-only change. A `Cargo.toml` that is a symbolic link makes no tree binary, in the working tree and with `--staged`.
- SD923: `[` and `]` are plain characters in a path of `Files:`. A path such as `app/[id]/page.tsx` is exact, and only `*` and `?` make a glob, as in the scope of the change.
- The branch walk of `approval = "branch"` reads the paths of all commits with one `git diff-tree --stdin` in place of one process for each commit. It also fixes the stop of the walk for a file whose name is the id of a commit of the branch. The walk of a branch of 500 commits was 5.7 to 11.9 times slower than `check` with `approval = "base"`; after the change the ratio is 1.59 on the same machine, with a target of 2 or less (`bench/git-access/walk.sh`, [results.md](bench/git-access/results.md#the-walk-of-the-branch)).
- With `approval = "branch"`, a shallow clone with a known base and no reachable merge base gives SD910 with `reason 5` for each change that needs an approval, and not "no change check". An unknown base keeps the note of 0.6.1. The note no longer asks for `--base` when it was passed. `change_base` in `check --json` then holds the tip of the base.
- A `[change] approvers` item that holds `*` or has no `@` is exit code 3 (ADR 0012 rejects a wildcard).
- In a tree binary, SD503 does not count a change of the line `spec-dna = "..."` of `spec-dna.toml`. A tree binary is a binary that the checkout of spec-dna itself builds: the file `Cargo.toml` at the top level of the git repository has `name = "spec-dna"` in `[package]`, and its `version` is equal to the version of the binary. The mode needs no key. A change of any other line of `spec-dna.toml` counts as before, and a release binary in another repository keeps SD503 for a pin change. The exact pin rule stays: a pin that differs from the version of the binary stops `check`, `finish` and `eval run` with exit code 3. The release pull request of spec-dna now moves the pin with the version in one commit ([docs/diagnostics.md](docs/diagnostics.md#files-that-sd503-counts), [ADR 0013](docs/adr/0013-self-hosting.md)).
- The line of `AGENTS.md` and `CLAUDE.md` names the 5 rules of the disciplines, the command `spec-dna skill show <name>` and the approval rule: an agent writes an approval commit only when the owner says so explicitly in the chat. It has 400 bytes or fewer, with its marker. The line grows from 155 to 394 bytes, and the bytes that the adapters add to each request grow from 267 to 506 (the line, its line feed and the description of the adapter skill). `init` prints both numbers. Run `spec-dna init` to write the new line, and commit it as a change of its own.
- `[harness] skills` takes the 11 names. The names `intent`, `spec` and `plan` of 0.6 end with no alias, and a configuration with one of them gives exit code 3 at each command. The message names the new names and `spec-dna upgrade`. `review` stays.
- Only `upgrade` renames a skill of 0.6. `init` does not: it gives exit code 3 for an old name in `[harness] skills`, and exit code 2 for an old name in an answer, with the new name in the message. The design of 0.7.0 (D4) said that both rename.
- `upgrade` of a `skills` list of many lines, where the rename changes the length (`spec` gives `clarify` only, and `intent` gives no name, so the list gets shorter), makes no edit. It stops with exit code 3 and prints the list to write by hand.
- The level `full` of `init` also proposes the skills that are off by default (all 11 names in 0.7.0). The level `lean` proposes the skills that are on by default.
- `init` removes the unedited skill files of 0.6 (`spec-dna-intent`, `spec-dna-spec` and `spec-dna-plan`) and keeps an edited file, which `check` reports as SD501. The 3 old paths do not count for SD503, so the pull request of the pin can delete them.
- `index` writes the first status of the type of its place (`draft`, or `proposed` for a decision) before the line `generated`, when the type needs a status and the old file and the root give none. Before, `index` of a folder of a type with a status stopped with exit code 1 and the message "a new error SD304 for" and an empty name, and `index --replace` wrote a file that `check --all` rejected. A place that gives no type is now exit code 2 (it was exit code 1 through SD303), and the message names the folder (for the folder of a layer with more than one type, such as `intent`, it names the layer and its types). The index that `finish` writes again follows the same rule, and for a place with no type it keeps the front matter ([docs/commands.md](docs/commands.md#index)).
- 2 names in `areas` that differ only in case are exit code 3. `new` and `mv` stop with exit code 2, and write nothing, for a path whose area differs from a name of `areas` only in case. `init` proposes one name of 2 such folders, with a note. The plan of `upgrade --layout --areas` stops a pair in its file with exit code 1 ([docs/config.md](docs/config.md#areas-and-groups)).
- `log` and `stats --sdlc` read a rename of `intent.md` out of the intent folder as a move when git finds the rename, the new file is the same text (`R100`) or has the old id in `aliases`, and the same commit removes no other file of the change folder. A plain `git mv` of an intent is thus no end. A `git mv` of a whole folder that holds a spec or a plan, and a `finish` that removes the spec and the plan, still end the change. 2 cases read as a move because the folder holds only `intent.md`: a `git mv` of the whole folder, and a `finish` that adds a near copy with the old id in its `aliases` ([docs/commands.md](docs/commands.md#log)).
- The rule "an agent writes an approval commit only when the owner says so explicitly in the chat" is the default of each project, not of this repository only. The agent approves a routine plan on the word of the owner. The change guide says how to make the rule stricter (`protect-approvals`) or less strict (a rule of the project after the generated line). The bytes of the generated line do not change.
- SD913 gives no diagnostic before a plan exists. It runs for a diff only when a change of the diff has a `plan.md` (a `draft` plan counts). An edit of the questions, the roadmap or a spike during `clarify` no longer gives a warning. Code that a branch edits before any plan exists has no signal, and the review is the control.
- The type `plan` has no default budget, as `research`: a plan of small tasks grows with the size of the change. SD701 to SD704 no longer apply to a plan. `soft = 0` turns off SD701 and SD703 for a type, and `hard = 0` turns off SD702 and SD704. Migration: a project with an explicit `[budgets.plan]` keeps it. Delete the table to take the new default. `[budgets.plan]` still sets a budget: a table with one key only takes the other key from the default of 0.6.1 (6,000 and 12,000).
- `spec-dna skill list`: the column and the field `default` give the default of ADR 0023, not the fixed list of 7 skills.
- SD507 reports an enabled skill whose default by ADR 0023 is off. The verdict of the project comes first, then the verdict of the release. A skill of the 7 of D2 is no longer exempt: with the verdict `off` it gives SD507, and with the constant `true` and no verdict `on` it gives SD507.
- `upgrade` renames a skill of 0.6 only to the new names that are on by default. A name that the list holds stays, also when it is off by default. For each name that it leaves out, `report.edits` has a line with the name and the fix (add the name to `skills`). A list of many lines gets the filtered list in the message of exit code 3. `upgrade` applies no preset of a profile.

## 0.6.1 - 2026-11-16

0.6.1 has fixes and small additions from the first upgrade of a consumer repository. It keeps compatibility with 0.6.0.

### Upgrade from 0.6.0

- Install the 0.6.1 binary. Then move the pin in `spec-dna.toml` in a pull request of its own, because SD503 rejects a change of the configuration that comes with other files. You can edit the pin by hand, or run `spec-dna upgrade`. In a 0.6 project, `upgrade` moves the pin, makes 2 more edits of `spec-dna.toml`, and names each edit in the report. `upgrade --layout` is for a 0.4 or 0.5 source. [docs/guides/upgrade-0.5.md](docs/guides/upgrade-0.5.md#upgrade-a-06-project) gives the steps.
- Run `spec-dna init` to refresh the generated agent and skill files. The text of `Explore.md`, `fallback-advisor.md` and the review skill changed. The marker hash of the review skill changed. The marker hashes of the 2 agent files stay the same, because the hash ignores the new blank line.
- The new key `[change] base` is optional. A binary of 0.6.0 stops with exit code 3 on this key, so pin 0.6.1 first.
- The JSON output has new fields. Each field is always present:
  - `locked_links` in `check --json` (an empty list if there is no locked link)
  - `edits` in `upgrade --json`, in `report`
  - `alias_dropped` in `mv --json`
  - `types_added` and `next` in `layer add --json`
  - `warnings` in `init --json`

### Added

- `spec-dna check --all` prints one info line for each dead link in the locked text of an accepted decision, such as a link to an intent that `finish` deleted: ``<path>:<line>: info: the locked text links `<target>`, which no longer exists; add an amendment that points to <pointer>``. The pointer names the change (`spec-dna log` names its commit), or says "the new place of the text, or the reason for its removal". The line is not a diagnostic: it has no code and no severity, and it does not change the counts or the exit code. `check --json` lists these links in `locked_links` with and without `--all` ([docs/commands.md](docs/commands.md#check), [docs/diagnostics.md](docs/diagnostics.md#locked-references)).
- The new key `[change] base` names a local branch. It is the base of the change checks in a repository with no remote. The order is `--base`, then `origin/HEAD`, then the branch of the key. spec-dna never guesses a branch. If the branch does not exist and there is no `origin/HEAD`, the base is unknown, and a note names the branch. It is not an error. `status`, `next`, `finish` and `check` read the key. `spec-dna init --reconfigure` keeps it ([docs/config.md](docs/config.md#the-table-change), [docs/guides/upgrade-0.5.md](docs/guides/upgrade-0.5.md#a-repository-with-no-remote)).
- `spec-dna upgrade --layout` moves a document of a mixed root to the top file of the root when its file stem is `roadmap`, `product-overview` or `overview`, `open-questions` or `questions`, `glossary` or `terms`, or `architecture` or `system`. The document is a `note`, a `reference` or a `change` that is not `done`. It has no requirement. It is directly in the root or in `notes/`, `reference/` or `changes/`. It is not over the hard budget of the type of the top file. A `change` named `roadmap` becomes the top file, not an intent. The moved file has no `type`, `status` or `was-type` line, and its old id is an alias. The plan adds the type to the key `types` and the layer `foundation` or `pending` to the graph. A document stays, with one `kept` line in the report, if 2 documents want one top file, if the top file or its id exists, or if its id names more than one document. A 0.6 project gets no such move ([docs/commands.md](docs/commands.md#upgrade)).
- `spec-dna upgrade --layout --areas <file>` moves each document that the TOML table `[areas]` of the file maps (`"specs/pumps" = "pumps"`) to `<root>/requirements/<area>/<slug>.md`, with its old id as an alias. The plan adds each area to the key `areas`, sorted. An unknown id, a document that is not a `requirements` document in 0.6, and an area that is not one folder name stop the plan with exit code 1 and no write. `--areas` without `--layout`, or for a 0.6 project, is a usage error (exit code 2). A bad file gives exit code 3 ([docs/commands.md](docs/commands.md#upgrade)).
- The new section "After the upgrade: top files and areas" of [docs/guides/upgrade-0.5.md](docs/guides/upgrade-0.5.md#after-the-upgrade-top-files-and-areas) gives the `spec-dna mv` commands and a shell loop for a 0.6 project, where `--layout` moves no document to a top file.
- `spec-dna upgrade --json` has the new field `edits` in `report`: one string for each edit of a 0.6 `spec-dna.toml` beyond the pin ([docs/commands.md](docs/commands.md#upgrade)).
- `spec-dna init --reconfigure` prints a warning when the policy is not `none` and the `types` list lacks `intent`, `spec` or `plan`. It does not edit the key. `--json` has the new field `warnings` ([docs/init.md](docs/init.md#reconfigure)).

### Changed

- `spec-dna mv` of a whole document into a mixed root fits the `type` and `status` lines to the new place. It removes a `type` line that the place gives, and a `status` line of a type with no status. It writes the first status of the new type when the old status does not fit, and it writes `type:` when the new place gives no type. The output names each change. `mv` stops with exit code 2 if the document has requirements and the new type holds none, or if an accepted decision would get another type ([docs/commands.md](docs/commands.md#mv)).
- `spec-dna mv` and `spec-dna upgrade` remove each folder that a move leaves empty, such as `docs/changes/` after the last change moved out. A folder with a file, including an ignored file, stays. The root folder of the document stays, and so does the folder of a `--stub`. `upgrade --dry-run` removes nothing.
- `spec-dna upgrade` of a 0.6 project makes 2 more edits of `spec-dna.toml`, and the report names each edit. If the file has the key `types`, the policy is not `none`, and the list lacks `intent`, `spec` or `plan`, it adds the absent types. If `linked-from` of `[layers.pending]` is exactly `["intent"]` (the value of `init` 0.6.0), it adds `requirements` and `decisions`. A list of many lines is not edited, and a warning names the hand edit.
- The report of `spec-dna upgrade` lists the changes of behavior of 0.6 only for a 0.4 or 0.5 project. A 0.6 project no longer gets the 5 lines. The field `behavior` of `--json` stays.
- `spec-dna layer add --types` appends each new type to the top-level `types` list, when the file has one. Before, `new` and `check` refused the type of the new layer. The output names the added types and ends with up to 3 `next:` lines (commit the file, run `init` when the agent skill lists the types, write the first document). `--json` has the new fields `types_added` and `next` ([docs/commands.md](docs/commands.md#layer-add)).
- In the default graph and the small graph, the `pending` layer has `linked-from = ["intent", "requirements", "decisions"]`. A requirement that names an open question and a decision that names the roadmap no longer give SD901. A file with its own `[layers.pending]` keeps its value ([docs/config.md](docs/config.md#default-graph)).
- `spec-dna next` and `spec-dna status` no longer ask for paths of a routine change whose plan lists no files. In such a plan, the `Files` section has one item only, and the item starts with the word `none` (`- none: research only`). The line `[x] the plan lists no files: the change edits no file` replaces the step of the files. The stage `build` ends when each task of `Order` is done, or when `Order` has no task. A higher-risk change keeps the step of the files ([docs/types.md](docs/types.md#files-and-paths), [docs/guides/change.md](docs/guides/change.md#work-with-no-files)).
- The row of SD503 in [docs/diagnostics.md](docs/diagnostics.md) gave no paths. The new section "Files that SD503 counts" lists each path that does not count (the adapters, the harness files, the hook files, the starter files of the evals and each file whose name starts with `spec-dna`) and says that each other path counts, such as a lint configuration, `CODEOWNERS` and a deleted file. No behavior changes.

### Fixed

- `spec-dna upgrade` of a 0.4 or 0.5 project writes the 3 types of the intent layer for a `change` in the key `types`: `change` becomes `intent`, `spec` and `plan`. Before, the list had only `intent`, and `new spec` and `new plan` stopped after a policy started. A list of many lines that needs this change is a hand edit (exit code 3) ([docs/commands.md](docs/commands.md#upgrade)).
- `spec-dna upgrade` no longer changes an empty `types` list (`types = []`), which means that every type is valid. Before, a 0.6 project got the list `["intent", "spec", "plan"]`, which limited the project to 3 types, and a 0.4 or 0.5 project with a `done` change got the list `["research"]`.
- The plan of `spec-dna upgrade --layout` compares the paths of a target in ASCII lowercase. The dry run and the write now agree on a file system that ignores case. Before, the dry run could say ok and the write could stop with exit code 4 for 2 names that differ only in case. Two area targets that differ only in case stop the plan.
- When the branch of `[change] base` exists but has no common history with `HEAD`, the note for an unknown base no longer tells the person to set the key. The note is ``no base for the change checks: the branch `<b>` of [change] base has no common history with HEAD; pass --base <rev>``.
- `spec-dna init --reconfigure` writes the default `high-risk-layers` of the new graph when the file has the default of the current graph. A project that changes from the small graph to the default graph gets `["interfaces", "rules"]`. A value that you set stays.
- A move of `intent.md` out of the intent folder is no longer the end of a change, when the move keeps the id as an alias (`spec-dna mv`, `upgrade`). `log` gives no entry for it, and `stats --sdlc` drops that change ([docs/commands.md](docs/commands.md#log)).
- `spec-dna mv` of a document to the path of one of its own aliases no longer stops with "already exists". It drops that alias and keeps the old id as an alias. The output has `(alias dropped: <alias>)`, and `--json` has the new field `alias_dropped`.
- The agent files `Explore.md` and `fallback-advisor.md` have a blank line between `Answer:` and its list, so they pass markdownlint (MD032). `spec-dna index` writes an empty cell as `| |` in place of `|  |`, so a generated index passes MD060. The golden files of the generated agent files and the skill in `tests/golden/harness/` now go through the lint of the pre-commit hook.

### Benchmark

The new workflow `Benchmark` ran once on a GitHub runner (ubuntu-24.04, run 43906896512) with the release binary 0.6.1 on the synthetic medium corpus (1,194 documents, 15 runs of each mode). `modes_agree` and `incremental_agrees_with_full` were true. A shared runner has other load and other hardware, so do not compare these numbers with the host table in `bench/tool/README.md`.

| Mode | Median (ms) | Min (ms) | Peak RSS (MB) |
| --- | ---: | ---: | ---: |
| startup | 0.8 | 0.8 | 15.1 |
| full | 80.5 | 77.0 | 43.1 |
| cold | 78.9 | 78.1 | 71.2 |
| warm | 50.2 | 49.0 | 99.2 |
| warm, base | 42.6 | 39.1 | 103.5 |
| context | 12.9 | 12.3 | 114.0 |
| incremental | 45.5 | 42.9 | 114.0 |

The peak RSS values rise in the order of the modes on this runner. The cause is not yet known, so read that column with care.

### Known limits

- `spec-dna layer add` with a `types` list on many lines stops with exit code 2 and changes nothing, because the editor cannot grow such a list. Add the types by hand.
- A paragraph under `Files` does not mean "no files". Use the list item `- none`.
- `spec-dna index` on a layer with a status stops with SD304 and a message with an empty target.
- Two area names that differ only in case, with different file names, plan two folders. A file system that ignores case merges them.
- A plain `git mv` of an intent out of the intent folder writes no alias, so `log` still reads it as a closed change.
- A commit of `finish` that adds a near copy of an intent, with its id as an alias, reads as a move. `log` then gives no entry for that change.

## 0.6.0 - 2026-11-16

0.6.0 breaks compatibility with 0.5. It renames document types, statuses, configuration keys, harness roles and 2 code names, and it keeps no old name as input. The command `spec-dna upgrade` moves a project that pins 0.4.0 or later, and it proves that no id and no requirement text changed ([ADR 0009](docs/adr/0009-upgrade-from-0-5.md)). 0.6.0 also adds the layer graph, the change lifecycle, the evals and the stage skills of the harness.

### Upgrade from 0.5 or 0.4

A project that pins 0.4.0 or later moves in one pull request. A project that pins an older version moves to 0.4.2 first. Install the 0.6.0 binary. Download `spec-dna-pin.toml` of the release, save it outside the repository (a file inside it makes the tree not clean), and verify it with `gh attestation verify`. Then run these commands:

```sh
spec-dna upgrade --dry-run --pin-file ../spec-dna-pin.toml
spec-dna upgrade --pin-file ../spec-dna-pin.toml
```

Run the `git add` command that `upgrade` prints, commit the result, and open the pull request. `upgrade` never commits and never deletes a document. In the first pull request, `check` plans the upgrade of the base in memory, so the gate stays active. [docs/guides/upgrade-0.5.md](docs/guides/upgrade-0.5.md) gives the full procedure, the stops and the rollback. After the upgrade, a project has the change policy `none`. Run `spec-dna init --reconfigure` to choose the change policy, the new parts of the harness and the evals. It keeps your values, but it writes `spec-dna.toml` again in its own order and drops the comments of the file, so commit the upgrade first.

Known limits of `upgrade`:

- If `upgrade` fails at its first write step, empty folders can stay. Git does not show them. The message says how to restore the tree.
- A link from a file that is not a document (`README.md`, an instruction file or a file outside the roots) to a moved file or folder stays as it is. The report counts these links and lists the first 5. Search for the old paths, and fix the links.
- A file with CRLF line endings that moves gets the new block of aliases with LF line endings, so the file can have both. `spec-dna mv` does the same.
- `git log --follow` can lose the history of a small change that `upgrade` moves, because git does not see the file as the same file after the edit of its front matter and links.
- `status`, `next` and `finish` read a lifecycle base with a pin before 0.6.0 with the rules of 0.6. `check` does not.
- A hand edit that the message of `upgrade` asks for must use the 0.5 names and must be committed, because `upgrade` reads `HEAD` with the 0.5 reader.

### Breaking changes

- The document types are 15. `spec` is now `requirements`, `contract` is `interface`, `invariant` is `rule`, `guide` and `runbook` are `procedure`, `note` and `reference` are `research`, and `change` is `intent`. A type name of 0.5 in `spec-dna.toml` is an error (exit code 3), and in a front matter it gives SD303. The message names the new type. The tier label `invariant` of `context` is now `rule` ([docs/types.md](docs/types.md)).
- The statuses of an `intent` are `draft`, `accepted` and `closed`. SD304 names the new status for `proposed`, `rejected` and `done`.
- A change is a folder: `intent/<slug>/intent.md`, with `spec.md` and `plan.md` next to it. The types `intent`, `spec` and `plan` are valid only there. A `spec` at another place gives SD303, and its message names `requirements`. A root `type` or `default-type` of one of these types is an error (exit code 3). `spec-dna new intent <slug>`, `new spec <slug>` and `new plan <slug>` write the files.
- The place gives the type. In a mixed root, the type of a document comes from these sources, in this order: the `type` field of the front matter, a top file such as `terms.md`, the folder of a layer, the file name in the `intent` folder, and `default-type`. `type:` is optional where the place gives the type. A mixed root has no default for `default-type`, so a document with no type gives SD303. A document with no valid type gets no SD304, no SD305 and no budget.
- `spec-dna new` writes each document at the place of its layer. `new requirements loans` writes `requirements/loans.md`, and an id is relative to the folder of the layer. A top file type takes no id (`new terms`). `new spec` and `new plan` stop with exit code 5 when the folder of the change does not exist. `new` writes no `type:` line where the place gives the type.
- The roles of `[harness.roles]` have new names. The role `reviewer` is now `verifier`, and its file `.claude/agents/checker.md` is now `verifier.md`. The role `spec-writer` ended. The role `code-simplifier` is new at level `full`, with `sonnet` at effort `high` as default. The key `reviewer` or `spec-writer` in `spec-dna.toml` gives exit code 3, and so does the word `reviewer=` or `spec-writer=` in the answer to question 13 of `init`. `upgrade` renames the key and removes an unedited `checker.md` and `spec-writer.md`. It keeps an edited file, and `check` reports SD501 for it ([docs/harness.md](docs/harness.md)).
- SD405 is `decision-locked` and SD611 is `decision-links-intent`. The codes stay. In `--json`, read `code`: the field `name` changed. The messages, the help of `mv` and `schema/spec-dna.schema.json` say "locked", where 0.5 said "frozen". The fingerprint and the keys of `[rules]` do not change.
- A command that stops on a pin mismatch gives a message that depends on the pin. A pin from 0.4.0 to before 0.6.0 says "run `spec-dna upgrade`", with `--pin-file ../spec-dna-pin.toml` if the file has `[spec-dna-sha256]`. A pin before 0.4.0 says to move to 0.4.2 first. `upgrade` runs with such a pin. `init` no longer moves a pin: `init` and `init --reconfigure` stop with exit code 3 and write nothing. The pin scan also reads a pin in single quotes and a file with a byte order mark.
- The order of `context` follows the levels of the layer graph. It lists the layers from level 0 up, and the layers of one level in the order of the configuration. In the default graph, `foundation` comes before `rules`, where 0.5 had a rule first. A top file (`product`, `system` or `terms`) is in the first tier only if its `applies-to` matches. The order of the layers also replaces the order of the types among the documents that cite a code path. The `context` order of the tiers does not change ([docs/commands.md](docs/commands.md#context)).
- A layer with `context = "on-request"` (in the default graph: `research`, `intent`, `incidents` and `pending`) does not serve its documents, unless a target names them. `show` reads them. `context` counts these documents in one line and lists no file. In 0.5 it listed the intents, the incidents and the roadmaps that cite a file. The `--json` output has no count.
- The default of `roots.live-sections` is `{ decision = ["Status"] }`. SD405 does not report a change to the `Status` section of an accepted decision, and `show --conventions decision` lists `Status`. `upgrade` writes `live-sections = { decision = [] }` into each root that can hold a decision, to keep the 0.5 behavior.
- SD202 follows the layer graph. A deleted document of a layer with `keep = "until-finished"` gives no SD202. The `intent` layer is such a layer by default. In a `forever` layer, only the field `was-type: note` or `was-type: change` allows a deletion, as 0.5 did for a `note` and a finished `change`. The status `closed` gives no exemption.
- A `research` document has the behavior of `reference`: no budget, `context` never serves it, and SD202 reports its deletion. A `procedure` has the budget of a guide and no default required section.
- `init` asks the layer graph at question 2 (`small`, `default` or `custom`) instead of the knowledge types. The answers key `types` of 0.5 is not valid any more, and an answers file with this key gives an error that names `graph`. `init` now asks 14 questions ([docs/init.md](docs/init.md)).

### Added

- `spec-dna upgrade [--dry-run] [--layout] [--pin-file <file>]` moves a project to the version of the binary ([docs/commands.md](docs/commands.md#upgrade)). It reads `HEAD`, builds the whole plan in memory, and proves that each old id, alias, requirement and section still resolves to the same text. Then it writes the new `spec-dna.toml`, the front matter, the adapters and the harness files, and it moves each `proposed`, `accepted` or `rejected` change to `intent/<slug>/intent.md` with `git mv`. With `--layout`, it also moves the folders of the other 0.5 namespaces of a mixed root.
  - It refuses a tree that is not clean (staged, unstaged, or untracked and not ignored), also with `--dry-run`. The line `to commit:` gives the `git add` command. A second run after the commit changes nothing.
  - The exit codes are 0 (a write, or nothing to change), 1 (a tree that is not clean, a stop of a move, a symbolic link in a path, a file in the place of a folder, a file that git ignores, or a failed proof), 2 (usage), 3 (configuration, pin or pin file) and 4 (a git or write error, with the commands that restore the tree).
  - `--pin-file` takes the `spec-dna-pin.toml` of the release. A project with `[spec-dna-sha256]` needs it. A project with no table gets the table.
  - `--json` has `from`, `to`, `files`, `proof` and `report`. The report lists the proposals to delete, each decision with no `Status` section, each locked decision that cites a migrated change, each kept harness file, each ended role with its model, the links that it did not rewrite, the count of each error code of `check --all` on the new tree, and the changes of behavior.
  - In the first pull request after the upgrade, `check` runs the plan of `upgrade` on the base tree in memory, with no `--layout`, and compares the head with the result. It writes nothing to git. SD503 does not apply when the base pins a version before 0.6.0, so one pull request can hold the configuration and the documents. A later change of the configuration keeps SD503.
  - The change checks that compare with a lifecycle base have no base when that base pins a version before 0.6.0, and `check` prints a note. A base that pins a version before 0.4.0, or that `upgrade` cannot plan, gives exit code 3 and the reason.
- The layer graph ([docs/config.md](docs/config.md)). A layer has the keys `types`, `dir`, `files`, `paths`, `moves-to`, `linked-from`, `keep`, `context`, `max-docs`, `max-chars`, `max-started` and `stale-days`. The configuration also has `areas` and `[groups]`. A cycle in `moves-to`, a type in 2 layers, an absent layer in an edge, an unknown key, a bad value, and a group area that is not in `areas` are errors (exit code 3). The place rule, the order of `context`, SD202, `new` and `layer` read the layers.
  - `spec-dna layer add <name>` writes a layer into `spec-dna.toml`, and `layer list` shows each layer, its level, its edges, its limits and the number of documents of each rule ([docs/commands.md](docs/commands.md#layer)). `--moved-from` adds the new layer to the `moves-to` list of the named layers. Both commands keep each comment and the line endings of the file. The skill of `init` lists the types of the layers, so after `layer add`, run `spec-dna init`, or `check --all` reports SD502. `layer add` stops with exit code 2 and changes nothing for a cycle, a name that exists, a type of another layer, an unknown layer in an edge, a bad value and a bad `files` entry.
  - `init` question 3 proposes the areas of `requirements/` (at most 8, and only 2 or more). `init` never proposes a group.
  - `spec-dna context --group <name>` serves no document of an area outside the group, and counts the held-back documents in one line. A target that names a document of another area serves it. An unknown group is exit code 2.
- The checks of the layers ([docs/diagnostics.md](docs/diagnostics.md)), in the new rule group `layers`: SD901 (`link-direction`, a reference from a more stable layer to a less stable one that does not list it in `linked-from`), SD902 (`layer-limit`, a layer over `max-docs` or `max-chars`), SD903 (`history-in-requirements`) and SD904 (`term-forbidden`, a forbidden synonym of a `terms` document). They are warnings in `standard` and `strict`, and off in `lite`. A 0.5 project gets them at the first `check` after the update, and the gate reports only the new ones.
- The change lifecycle. The table `[change]` has the keys `policy` (`three-class`, `two-class` or `none`), `gate`, `high-risk-layers`, `high-risk-paths` and `max-fanout`. Without the table the policy is `none`, and the codes SD905 and SD910 to SD922 are off. The rule group `changes` sets their severity. The intent fields are `class` (`routine` or `higher-risk`), `areas` and `from`.
  - `check` has the codes SD905, SD910 to SD917, SD919, SD920, SD921 and SD922 ([docs/diagnostics.md](docs/diagnostics.md#checks-of-the-change-lifecycle)). They check a change in one tree, in the diff against the lifecycle base (the merge base of `--base`, or of `origin/HEAD`, with `HEAD`) and in a finished change. SD910 follows `[change] gate`. `check --json` has the field `change_base`, and `check --staged` diffs the index. SD922 (`question-open`) reports an open question of an approved spec. SD921 (`truth-changed-without-change`, off in `lite`, a warning in `standard` and an error in `strict`) reports a requirement of a `forever` layer that changed while no file of the `intent` folder did. It needs a lifecycle base, and a move that keeps the text of each requirement gives nothing.
  - `spec-dna finish <slug> [--dry-run] [--base <rev>]` ends a change ([docs/commands.md](docs/commands.md#finish)). It runs 6 steps in memory and stops at the first that fails. Then it applies the delta lines of the spec to the documents, deletes the folder of the change and stages the result. It makes no commit, and it is the one command that deletes documents.
  - `spec-dna next <slug>` prints the checklist to the next gate, and `spec-dna status [--group <name>]` lists each change with its class, stage and staleness ([docs/commands.md](docs/commands.md#next)). Each line of `next` has the name of a playbook skill as data. Those skills come in 0.7.0.
  - `spec-dna log [--since <rev>]` lists the finished and the closed changes from the history of git ([docs/commands.md](docs/commands.md#log)). `spec-dna stats --sdlc [--group <name>]` prints the indicators of the playbook, from git and the committed files ([docs/commands.md](docs/commands.md#stats)).
  - The parser keeps the tasks of a plan (an item of `Order` with a task box and a task id), the open questions of a spec and the items of the other sections of a change. `PARSER_VERSION` is 13, so spec-dna rebuilds the cache once after the update.
  - `init` question 4 is the change policy and question 5 is the gate. They write the table `[change]`. With a policy other than `none`, `init` prints a note with the `CODEOWNERS` line `docs/intent/*/spec.md @OWNER`. It never writes the `CODEOWNERS` file.
- The evals ([docs/evals.md](docs/evals.md)). The table `[evals]` has the keys `dir` (default `evals`), `min-pass-rate` (default 0.8), `runs` (default 3) and `max-total-usd` (default 20). Without the table, there are no evals.
  - `spec-dna eval run [<id>...]` runs the evals of `[evals] dir` with headless Claude Code ([docs/commands.md](docs/commands.md#eval)). An eval is a JSON file with a task, a fixture, checks and a budget. Each run happens in a temporary git repository. The command refuses to start (exit code 3) when `runs x max-budget-usd` is above `max-total-usd`, and it stops when the spend is above the cap. `--out <dir>` writes the counts, the costs and the check names, and no task text or output of `claude`. spec-dna makes no network call. `claude` does, and each run costs money.
  - SD506 (`evals-invalid`, an error) reports an eval file that `eval run` rejects. SD918 (`incident-eval-missing`, a warning in `standard` and `strict`) reports an incident that the diff adds with no eval file. Both need the table `[evals]`.
  - `stats --sdlc` gives the eval pass rate and the time from an incident to its eval.
  - `init` question 14 asks for `none` or `skeleton`. A new project gets `skeleton`, and an upgraded project keeps `none`. `skeleton` writes `[evals]`, `evals/check.sh`, `evals/example.json` with its fixture and, for the CI answer `github`, `.github/workflows/agent-evals.yml`. It starts no run, and the workflow runs nothing until the repository variable `SPEC_DNA_EVALS` is set.
- The parts of the harness ([docs/harness.md](docs/harness.md#stage-skills-and-reviewmd)). `[harness]` has the keys `skills` (a list of `intent`, `spec`, `plan` and `review`), `review-md` and `protect-tests` (`fix-branches`, `always` or `off`). `init` question 12 has a second prompt for them. A file that has none of them behaves as in 0.5.
  - The 4 stage skills are `.claude/skills/spec-dna-intent/SKILL.md`, `spec-dna-spec`, `spec-dna-plan` and `spec-dna-review`. Each skill runs spec-dna commands and never sets `accepted` or `approved`. `REVIEW.md` has 3 passes (Bugs, Security and Compliance), the severities Important and Nit, and a list "Do not report".
  - The hook `spec-dna hook protect-tests` asks the person before an edit of a test file that git tracks ([docs/harness.md](docs/harness.md#spec-dna-hook-protect-tests)). With `fix-branches`, it asks only on a branch that starts with `fix/`. It needs spec-dna 0.6.0 or later on the `PATH` of each teammate.
  - A new project gets the 4 skills (if the change policy is not `none`), `REVIEW.md` and `protect-tests = "fix-branches"`. An upgraded project gets them off. `init --reconfigure` removes an unedited file that the configuration no longer generates, and keeps an edited file.
  - `check` reports SD504, SD501 and SD502 for these files, and SD503 accepts them with a change of `spec-dna.toml`.
- The embedded templates `prd`, `rfc`, `madr`, `post-mortem` and `runbook`, and `spec-dna new <type> <id> --template <name>` ([docs/types.md](docs/types.md#templates)). The table `[templates]` with the key `dir` (default `.spec-dna/templates`) holds the project templates, and a file there replaces the embedded template of the same name ([docs/config.md](docs/config.md#the-table-templates)).
- The new types `product`, `system`, `terms`, `roadmap`, `questions`, `spec` and `plan`, with their statuses, required sections, budgets and templates. The front matter field `was-type` (`note` or `change`) marks a document that `upgrade` converted.
- The configuration key `language`, a BCP 47 language tag (default `en`). A primary subtag other than `en` turns off the checks that read English word lists, and the requirement lint (SD801 to SD805). The key `[lint] history-terms` replaces the phrases of SD903.

### Changed

- A new project from `init` gets one mixed root `docs`, with no `default-type` and no status, the layer graph `small` (fewer than 30 documents) or `default`, the change policy `three-class` with the gate `block`, the 4 stage skills, `REVIEW.md`, the hook `protect-tests` and the evals `skeleton`. A file with no marker is never replaced: the default leaves it out, and a note names the file.
- `init --reconfigure` keeps the current values of `language`, `[change]`, `[evals]`, the keys `types`, `[groups]` and `[templates]`, and a mixed root with no `default-type`.
- `init` question 7 writes the top key `language = "<tag>"`, and writes nothing for `en`. A value that is not a BCP 47 tag is a usage error. Detection proposes a tag from the heading aliases and the script of the headings.
- The bytes that the harness adds to each request: for a new project, `lean` 2,189 and `full` 2,655. For an upgraded project, without the new parts, `lean` 1,601 (1,595 in 0.5) and `full` 1,797 (2,022 in 0.5, with the role `spec-writer`). The adapters still add 267 bytes.
- `check` gives SD501, not SD502, for a file with a harness marker that the configuration does not generate and that has an edit. This covers an old `checker.md`, a stage skill that you removed from `skills`, `REVIEW.md` with `review-md = false`, and an agent file of a role that you turned off. An unedited file of these kinds still gives SD502. The codes keep their number, name and purpose.
- `spec-dna init` says in a note that `.claude/agents/Explore.md` replaces the built-in `Explore` subagent of Claude Code, and how to remove it.
- `history`, and `mv` to a new document of a type that no layer has, stop with exit code 2 and change nothing. The message names the command to run, such as `spec-dna layer add research --types research`.
- SD101 and SD102 give no diagnostic for a reference in the locked text of an accepted decision. The author cannot change this text, and a finished intent leaves no document to link.
- The plan template writes the placeholder `{{path}}` in a code span, so an untouched plan lists no path.
- The demo and the quickstart use the layout of 0.6: `requirements/loans`, `interfaces/http` and `decisions/`.
- The performance benchmark of 0.6.0 was not run again, because the load of the host stayed above 4 (1-minute load average 36.80, 15-minute 63.59, from `uptime` when this section was written). This release makes no claim about speed.

### Fixed

- `spec-dna mv` keeps the final `/` of a relative link to a folder, in the document that it moves. It dropped the `/`.
- `spec-dna mv` names the first new error in the order of the diagnostics when a split would give one. It named one of them with no fixed order.
- A plan whose `Files` section lists plain paths (`- src/dose.py`) had an empty `Files`, because spec-dna read only a code span. After `finish`, each implementation file then gave SD913. Now an item of `Files`, and an entry of the label `Files:` of a task, can be a plain path ([docs/types.md](docs/types.md#files-and-paths)). A plain path is one token that has no white space and has a `/`, a `.` or a `*`. A URL, a placeholder `{{...}}` and a word with none of these (`Makefile`) are not paths: write them in a code span. A project can see new results from the same files, because the readers of `Files` now have real patterns. A plan that lists only prose gives no new result. `PARSER_VERSION` and `FORMAT` do not change for this fix.
- When the items of `Files` are not paths, the last line of `next` and `status`, and the messages of SD913 and SD915, say so and show the first such item.
- For the pull request of `finish`, which deletes the folder of a change, the scope of SD913 has the paths of the base tree and of the newest plan and spec of the branch. A plan that the pull request adds or edits no longer gives SD913 on its own files.

## 0.5.0 - 2026-11-14

An optional agent harness for Claude Code: `init` installs a small team of models, and `check` keeps its files true ([ADR 0004](docs/adr/0004-harness.md), [docs/harness.md](docs/harness.md)).

### Added

- `[harness]` in `spec-dna.toml`: the level (`none`, `lean`, `full`), the model and the effort of each generic role (orchestrator, advisor, reviewer, researcher, implementer, spec-writer, helper), the denied tools and the hook values.
- `init` asks 2 more questions: the harness level and the models of the roles. The answers file has the keys `harness`, `harness-models` and `harness-deny`. A new configuration gets `lean`.
- Generated Claude Code files:
  - Agent files `.claude/agents/checker.md`, `Explore.md` (it replaces the built-in Explore) and `fallback-advisor.md`. The level `full` also has `implementer.md` and `spec-writer.md`.
  - The page `.claude/agents/model-team.md`.
  - A model-team line in `CLAUDE.md`.
  - Entries in `.claude/settings.json`: `model`, `modelSettings`, `fallbackModel`, `advisorModel`, `env.CLAUDE_CODE_SUBAGENT_MODEL`, `permissions.deny` and 2 hooks. spec-dna keeps all other keys of the settings file.
- The harness files, with their removals, are in the `init --dry-run` diffs and in the `diff` of each entry of `--json` `files`, as the other files are. The preview starts with 10 answers: question 9 (the level) and question 10 (the models). Question 10 says `writes nothing` if the level is `none`.
- `check --staged` reads the harness files of the git index. An edit that is not staged does not count. The upgrade path of `init` changes the hook line of 0.4.1 and the harness files in one run.
- `spec-dna hook context-meter`: tells an agent its context size once at each level.
- `spec-dna hook fanout-cap`: asks the person before a workflow whose agent count can exceed the cap.
- SD504 `harness-missing` and SD505 `harness-settings-invalid`. SD501 and SD502 also apply to the harness files.

### Upgrade notes

- A configuration of 0.4 or older has no `[harness]` table, so `init` adds no harness on upgrade. Run `spec-dna init --reconfigure` to add it.
- The hooks need spec-dna 0.5.0 or later on the `PATH` of each teammate. Without it, the hooks do nothing.
- Each new `init`, also with `--non-interactive`, writes the `lean` harness: Opus at `xhigh` for the main session, the Fable advisor, the agent files, a deny rule and 2 hooks. For no harness, answer `none` to question 9, or use an answers file with `harness = "none"`.

## 0.4.2 - 2026-11-14

Fixes from the use of 0.4.1 in a second real project.

### Added

- `check --staged` checks the git index, the tree that the next commit records. Untracked files and edits that are not staged do not count. An unmerged path gives exit code 4, and a missing `spec-dna.toml` in the index gives exit code 3. The hook that `init` writes (pre-commit, husky, lefthook) now runs `spec-dna check --staged`. Before this change, a commit could pass while the committed tree failed with SD101. ADR 0001 has an amendment.
- SD611 `decision-links-change` (warning): an accepted decision links a change document in its frozen text or in `depends-on`. A link in an amendment, history or live section is allowed. [docs/types.md](docs/types.md#links-between-types) gives the link direction of each type.
- `init` and `init --dry-run` print the 8 answers before the preview. Each line says whether the answer is the default and what it writes (`writes nothing` for an answer that writes no key or file). With `--json`, `answers` has the same data.
- The preview of `init`, and `init --dry-run` for a project that has a `spec-dna.toml`, show the diff of each file: `AGENTS.md`, `CLAUDE.md`, the skill and the hook. With `--json`, each entry of `files` has a `diff`.

### Changed

- `cargo install --locked --git <url> --tag v0.4.2` works without a package name. The benchmark packages and the brownfield fixture have no binary now (they use examples and a library), so the repository has one package with a binary. The command for v0.4.1 and earlier still needs `spec-dna` at the end. The README and the quickstart give both forms.
- The skill that `init` writes lists the `types` of the project, not all 10 types. `check` compares the skill with the types of the project. After you change `types`, run `spec-dna init` to update the skill.
- `check --lint` runs the prose lint (SD801 to SD805) in every profile, even when `lint` is off. The findings are warnings, so they never fail the gate. Without `--lint`, nothing changes.
- A requirement key that is a template placeholder (`{{key}}`) gives SD306 only. It no longer gives SD203 too.
- SD804 does not report "many" after "how" ("how many").
- SD304 for `proposed` on a spec, a contract or an invariant names `draft`. `draft` is the proposal state of these types, and `proposed` is for a decision and a change. The namespace `reference/` stays singular. [docs/types.md](docs/types.md#statuses) gives the reason and the namespace of each type.
- `spec-dna new` says "replace each {{...}} placeholder". The templates use that form.
- A command that writes to a closed pipe (`spec-dna check --json | head -1`) ends with its own exit code, with no panic. A non-interactive `init` with a closed stdout now writes its files.

### Upgrade notes

- A project on 0.4.1 must change the pin to 0.4.2 and its `[spec-dna-sha256]` table (see [docs/quickstart.md](docs/quickstart.md#upgrade)), install the new binary and run `spec-dna init`. That run updates the skill. It also updates the hook that 0.4.1 wrote (`entry: spec-dna check`, or the line `spec-dna check` of `.husky/pre-commit`) to `--staged`. `init` changes only that exact line. For lefthook, change `run: spec-dna check` by hand.
- SD502 reports the old skill until `init` runs.

## 0.4.1 - 2026-11-14

One example domain, greenhouse irrigation, for the tests, the fixtures, the documentation and the texts for agents.

### Changed

- `init` no longer gives the type `reference` to the directory names `validation`, `validations` and `briefs`. It no longer excludes sibling directories with the prefix `ext-`. For such a directory, set the root type or an `exclude` glob in `spec-dna.toml`.
- The fix texts of SD405 and SD805 and the skill text of `init` use neutral examples. After the upgrade, run `spec-dna init` to update the adapters.
- The README says what spec-dna is, why a team needs it and how to use it in a project.

## 0.4.0 - 2026-11-13

The first public release: release binaries for 7 targets, with SHA-256 sums that a project pins in `spec-dna.toml` (phase 2 of ADR 0003).

### Added

- Release binaries for macOS (`aarch64-apple-darwin`, `x86_64-apple-darwin`), Linux (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`) and Windows (`x86_64-pc-windows-gnu`, `aarch64-pc-windows-gnullvm`). Each release also has `SHA256SUMS`, `spec-dna-pin.toml` and a build-provenance attestation for each file. A tag `v<version>` starts `.github/workflows/release.yml`. [docs/quickstart.md](docs/quickstart.md#install) gives the install and the verification commands.
- The table `[spec-dna-sha256]` in `spec-dna.toml`: the sums of the release binaries, copied from `spec-dna-pin.toml`. If the pin is the version of the binary, each command validates the table (exit code 3 for an error). `init --reconfigure` keeps the table.
- The CI job that `init` writes downloads the pinned Linux x86-64 binary and verifies its sum, if `spec-dna.toml` has that sum and no `install-source`. Otherwise, it builds the pin from source, as before.
- `SECURITY.md`, and [docs/evidence.md](docs/evidence.md): the method, the results and the limits of the agent experiments.

### Changed

- The message of a version mismatch gives the download URL and the sum of the pinned binary when the table has them. To move a project to the version of the binary, it names `spec-dna-pin.toml` of that release.
- If `spec-dna.toml` does not parse and its pin is a different version, `check` and `init` give the version mismatch, not the parse error. A configuration of a later version can have keys that an earlier binary does not know.
- CI builds the 7 targets on each push and pull request. It also runs each binary on a runner of its own platform, with these commands: `--version`, `init`, `check`, `show` and `context` (`scripts/smoke.sh`). Before, CI built `aarch64-apple-darwin` only.
- ADR 0003 has the design of phase 2. The Windows binaries keep the GNU ABI. There is no dispatch shim. A tool manager such as mise or aqua can keep several versions at the same time.
- The README is shorter: an example, the install, the start, the status and the links. [docs/commands.md](docs/commands.md) has the reference of each command. `CONTRIBUTING.md` has the repository layout, the setup and the dependencies.
- The documentation, the code comments, the schema descriptions and the text for agents follow ASD-STE100.
- Releases are immutable. After the publication, nobody can change the assets or the tag of a release.

### Upgrade notes

- A binary of a version before 0.4.0 does not know the table. For a configuration with the table, it reports ``unknown key `spec-dna-sha256` `` in place of the version mismatch. Install the pinned version.
- `init` does not change a CI job that exists, so the job of an earlier `init` still builds from source. To download the binary in CI, copy the job from [docs/quickstart.md](docs/quickstart.md#ci).

## 0.3.3 - 2026-11-13

Convention files outside the documents, and fixes from a review of the open defects.

### Added

- A pointer of `[conventions]` can name a Markdown file that is not a document. Such a file is outside every root, or an `exclude` glob removes it, for example the `README.md` of a decision directory. `context` serves its sections first in the convention tier (`"type": "file"` in `--json`). `show --conventions <type>` prints them. `show <path>#<anchor>` prints one section. No check, budget or id applies to the file. SD106 also reports a file that is absent.

### Changed

- `check` prints a warning code with more than 10 diagnostics as one count, as it does for the prose lint. On a mature corpus without test links, the SD608 lines no longer hide the other output. `--json` lists each diagnostic, and the exit code does not change.
- `scripts/cross-build.sh` names the zig version of CI when zig is absent. It also warns when another zig version is on `PATH`.

### Fixed

- `init --reconfigure` removed `[conventions]` and the root keys `history-sections`, `companion-sections` and `live-sections`. It now keeps them. If you ran `init --reconfigure` with 0.3.0 to 0.3.2, compare `spec-dna.toml` with its last commit.
- spec-dna reads a convention file of the working tree from disk on every run, without the parse cache. It also adds the file to the git status pathspec. Thus a changed file, or a symlink whose target changed, gives the current text. spec-dna does not read a symlink whose target is outside the repository (SD106).

## 0.3.2 - 2026-11-13

Live sections and project conventions. A replay of real changes of a corpus found 2 gaps. First, agents left the progress section of an accepted decision stale, because spec-dna froze that text. Second, agents did not see a convention for contract changes that only the project documents state.

### Added

- `live-sections` per root and type: the sections that record a current state, such as a progress section, in any language. In an accepted decision, a live section can change in place without SD405, and new text in it gives no SD605. The frozen text and the amendment sections keep their rules. [docs/config.md](docs/config.md), section "Sections of a decision", compares the 3 kinds of section.
- `[conventions]`: for each type, pointers to the sections of project documents that state the conventions of the project for documents of that type. `context` serves them first when a target is a document of that type. `show --conventions <type>` prints them, with the names of the amendment and live sections of the type.
- SD106 `convention-unresolved` (error, rule group `references`): a pointer of `conventions` names no document or section.

### Changed

- The skill tells agents to run `spec-dna show --conventions <type>` before they edit a document of that type. It also tells them to update a live section of an accepted decision in place when a change makes the section stale.
- The message and the fix of SD405 name the live sections.

## 0.3.1 - 2026-11-11

### Changed

- `mv` of a section leaves the heading of the section with a one-line pointer to its new place, for every destination type. Before, only a move into a `reference` document left a pointer. A review of 10 splits on a real corpus found a problem: a reader of the old document could not find the moved text. Prose references such as "see section 1" or "(Releases, below)" had the same problem.

## 0.3.0 - 2026-11-11

Hygiene functions: split, history extraction, generated indexes and stubs for every document type, and lower cost of code tasks.

### Added

- `mv <doc-id>#<section>... <to>` moves sections out of a document of any type into a new or an existing document of any type. `--type <type>` sets the type of a new destination. Each moved section keeps its old id as an alias (`<old-id>#<anchor>` in the `aliases` of the destination), and links follow it. The preservation check also requires that every moved section resolves at its old id.
- A move into a `reference` document leaves a one-line pointer in place of each moved section, and the new document links back to its source.
- Decisions: only the amendment sections and the history sections move, and only into a `reference` document. The frozen text stays. `mv` lists the moved amendments that state a requirement, a BCP 14 keyword or a status word as candidates for a superseding decision. It never writes a decision.
- `companion-sections` per root and type: the sections that belong to the requirements, such as a coverage table. When requirements move, their table rows and list items move with them. If a row has a key that moves and a key that stays, `mv` stops and gives that reason.
- `history-sections` per root and type. `spec-dna history <doc-id> [<to>]` moves the history sections (and the amendment sections of a decision) into a reference document. In a decision, a history section is not frozen text.
- `spec-dna index <dir> [--replace]` writes a generated index of the documents of a directory (default `<dir>/README.md`). The index has the mark `generated: spec-dna index h:<hash>`. With `--replace`, it replaces a hand-kept list, and links to a section of the old file that names an entry go to the entry. SD610 (warning): a stale index, or an index with a manual edit.
- `mv <from> <to> --stub` leaves a stub at the old path (front matter `moved-to`) for tools that read a file by its path. A stub has no id and no budget.
- SD503 (error): a change edits `spec-dna.toml` together with documents or code. A configuration change must be a change of its own.
- `[budgets] growth` (default 4,000 characters).
- A split can move text out of a document over its hard budget. The new document gives the warning SD704, not SD702, because it holds no new text.

### Changed

- A document can be over its hard budget in the base and grow by at most `[budgets] growth` characters. Then it gives the warning SD703, not the error SD702. In the code tasks of a real corpus, the error made agents split or shorten unrelated documents.
- The skill gives these rules, and the fix texts of SD701 to SD704 say the same:
  - Do not split, move or shorten a document for a size warning.
  - For a size error, put the new text into a new document, or move whole sections with one `mv`.
  - Do not edit `spec-dna.toml`.
  - Run `check` once, or run the project target that runs it.
- The ids of a root of type `reference` start with a namespace from the root path, so two reference roots never collide (D11-1). The id of 0.2 stays a fallback name when it is unique.

## 0.2.1 - 2026-11-08

Fewer agent requests and a better detection of common layouts.

### Added

- `spec-dna <command> --help` and `spec-dna help <command>` print the help of one command only.
- `init` detects more layouts. A top-level type directory such as `rfcs/`, `adr/` or `runbooks/` keeps its type. The new type words are `validation` and `validations` (`reference`), and `incident` and `postmortem` (`incident`). A directory with one of these names must be in a documentation tree, or it must be mostly Markdown. Decision files such as `0001-use-postgres.md` (adr-tools, `doc/adr`) give `{ alias = "ADR-{n}", file = "{n}-*" }`. These conditions apply:
  - each number of the root is unique;
  - no file of the root starts with a date;
  - no other root makes the same alias.

### Changed

- A directory of numbered files takes its type from its own files. These facts give the type:
  - the prefix (`ADR`, `RFC`, `INC`)
  - the front matter `type` of most files
  - the content of most files: `Decision`, or `Context` and `Consequences`; `Root cause` or `Timeline` and `Impact`; `Steps`; requirements with a BCP 14 keyword

  If the files show no type, the type is the type of the enclosing root, if that root has a type. In a mixed root, the type is `note`. Before, the directory took the default type of the enclosing root. A mixed tree takes the type of its numbered files only if it has no other documents. Files with a number and no prefix (`0001-*.md`) make a root only for decisions and changes.
- The skill: split a document only for a size error (SD702, and in the `strict` profile also SD701, SD703 and SD704). A size warning needs no split. A split document needs no second split. The fix texts of SD701 and SD703 no longer ask for a split.

## 0.2.0 - 2026-11-08

Profiles, 4 knowledge types, requirement evidence and an interactive `init`. The goal is that a corpus that gets larger stays consistent, so agents stay fast and cost little.

### Added

- `profile = "lite" | "standard" | "strict"` sets the default severity of each code. `[rules]` and the `rules` of a root override it by rule group (`references`, `ids`, `budgets`, `drift`, `evidence`, `lint`) or by code. A root can have its own `profile`. [docs/profiles.md](docs/profiles.md) gives the table.
- 4 document types. Each type has a template, required sections and budgets. [docs/types.md](docs/types.md) gives the details.
  - `invariant`: project-wide rules with stable ids and an `applies-to` scope. `context` serves a matching invariant first.
  - `runbook`: an operational procedure.
  - `incident`: it must link the requirements, decisions or runbooks that it involves (SD609).
  - `reference`: material that agents seldom read. It has no budget. `context` never serves it unless a target names it.
- `spec-dna new <type> <id>` writes a document from the template of its type. In a mixed root, each type gets its own namespace. SD306 (error): a template placeholder (`{{text}}`) is still in a document.
- `types` in `spec-dna.toml`: the document types of the project. A document of another type is an error (SD303).
- `required-sections` accepts a list of lists: several required sections for a type.
- Evidence: a requirement links to tests by `verified-by`, in the front matter or in an inline `<!-- verified-by: <path> -->` marker. A test file of any language (`[evidence] tests`) can also have a `covers: <doc-id>#<key>` tag. SD105 (error): a tag names no requirement. SD608 (by profile): an active requirement has no evidence. `show` lists the evidence of each requirement.
- `init` detects the repository and asks at most 8 questions with the detected defaults. The questions are about these items:
  - the profile, the knowledge types, and the evidence policy and link style;
  - the language and heading aliases, and the root map;
  - the budget level, the agent adapters, and the CI step, hook and base branch.
  It shows the full `spec-dna.toml` and the files to create or change, and writes them only after a confirmation. Modes: `--non-interactive`, `--answers <file>`, `--reconfigure`, `--dry-run`. It can write a GitHub Actions job and a pre-commit or husky hook. [docs/init.md](docs/init.md).

### Changed

- A configuration without `profile` uses `standard`. In it, the requirement lint is off (0.1: warnings) and SD608 is a warning. `[rules] lint = "warning"` and `[rules] evidence = "off"` keep the behavior of 0.1.
- `lint-severity` and `drift-severity` of a root are now the rules `lint` and `drift` of that root.
- The line in `AGENTS.md` and `CLAUDE.md` and the skill ask for one `context` call for a task that edits code or docs. A question needs no call. The skill names `new` and `covers:`, and its body has 1,863 characters (0.1.2: 2,392).
- The text output of `check` gives the fix of each listed code once.
- The default `context` budget is 8,000 characters (was 12,000).
- The parser reads the inline `verified-by` marker. The index format changed (parser version 9), so the first run parses all documents again.

## 0.1.2 - 2026-11-08

Fixes of the defects that a playground product of 12 agent tasks found in 0.1.1.

### Fixed

- `mv` of requirements: a list item can end after a blank line. A cut or an insert at that end joined the next paragraph to an item. Thus the preservation check stopped a valid move. The move now cuts and inserts whole lines without the blank lines after the item.
- `mv` of requirements: the preservation check compares the requirement text without link destinations, because the move rewrites the links to a moved key or section. The index format changed (parser version 8), so the first run parses all documents again.
- If the preservation check stops a move, it names each requirement whose text would change. It gives the text before and after the first difference, or it says that the requirement would be lost or new.
- `mv` of requirements records an alias only for an id of `HEAD`. A key that is new in the uncommitted change gets no alias.
- SD606 and SD607 on the frozen text of an accepted decision: an amendment section clears them if it cites the new path or identifier. Before, no valid change could clear them, except a superseding decision.

### Changed

- The text output of `check` lists the errors first, then the drift warnings (SD606, SD607), then the other warnings. It gives one line with a count for each prose lint code (SD801 to SD805). `--lint` lists each of these warnings. The JSON output did not change.
- The skill tells agents when to amend an accepted decision (the outcome stays) and when to supersede it (the outcome changes). It also tells them to name new code paths of an accepted decision in its amendment section.

## 0.1.1 - 2026-11-08

The first version for a consumer.

### Added

- `mv <doc-id>#<key or section>... <spec-id>` moves several requirements, or whole sections, into an existing spec or a new one. Each moved requirement keeps its old id as an alias, and links follow it. A preservation check runs before any file changes. If the check fails, `mv` changes nothing and exits with code 1.
- SD807 (warning): a list item starts with a bold slug key and has a BCP 14 keyword, but no colon follows the key.
- `install-source` in `spec-dna.toml`: the git source of spec-dna for the message of a version mismatch and for the CI job of the quickstart.
- `drift-severity` per root: SD606 and SD607 as `off`, `warning` (default) or `error`.
- `init` prints the `git add` command for the files that it wrote.

### Changed

- SD405 ignores link destinations. A move that rewrites a link in an accepted decision does not change its frozen text. The index format changed (parser version 7), so the first run parses all documents again.
- The message of a version mismatch says "move the project to", for an upgrade and for a downgrade.
- The skill tells agents to split a document with one `mv`.

## 0.1.0 - 2026-11-08

A local tag for the first demo. It was not published.

- `context <code path>` serves the sections that cite the file: by path, `path:line`, link, directory or identifier.
- SD606 and SD607 (warnings): a change removes or renames a path or an identifier that a document cites.
- `init` writes one line into each root instruction file, never an import, and prints the bytes that it adds to the prompt prefix. It does not run when the pin differs from the binary.
- `transcript` counts the spec-dna commands of an agent.
