---
status: draft
---
# The open points of part A of 0.8.1: spec

## Delta

None. This repository has no layer of requirements. The change adds the code SD942, changes the severity of SD646 in `strict`, and changes 3 rules of `init` and 1 text of `next`. The amendments of 2026-11-25 of [ADR 0017](../../adr/0017-profiles.md) and [ADR 0032](../../adr/0032-acceptance-in-one-pass.md) state the contracts. The sections L1 to L7 below give the design. A path and line, such as `src/stage.rs:541`, refer to the tree `bec2377`.

## L1. The copy of the release script runs with `bash`

Cause: `run_with_cross` copies `scripts/verify-release.sh` (`tests/release_check.rs:137`) and runs the copy at once (`:145`). On Linux, another thread of the test binary can fork while the copy is open for write. The child keeps a copy of the descriptor until its own exec, so the exec of the script fails with `ETXTBSY` ("Text file busy"). In a Linux container (`rust:1.99.1-slim-bookworm`), 25 of 600 runs of the test binary failed with that error: 17 of 300 runs with 4 threads, and 8 of 300 with 16. 3 of the 6 failed `Test` jobs since pull request #91 have the same error. The analysis also read the failure of `tests/upgrade_v070.rs:549` on `956b27bc`: it has a code cause. The code of that commit had the rule of D92, and the test still expected SD913 for a stray file. The next commit of the branch changed the test.

Change: the test runs `Command::new("bash").arg(scripts.join("verify-release.sh"))`. bash opens the script for read, and a read does not fail while a writer exists. With this change, 0 of 600 runs failed. `scripts/verify-release.sh` does not change.

`stub_bin_writes_an_executable_script` (`tests/common_helpers.rs:195`) has the same pattern: it writes a stub and runs it at once. No failure of it is recorded. It runs the stub with `sh` and keeps the check of the mode `0o755`.

The job "Run aarch64-apple-darwin" that GitHub cancelled with 0 steps has no cause in the repository: the runner was not acquired.

## L2. A `lite` team with no advisor has no step on the advisor

Cause: `team_page` (`src/harness.rs:1445-1451`) writes the steps `/advisor` and `Advising` for each `lite` team, because the page of `lite` keeps the bytes of 0.7.1 (D76 of 0.8.0), and 0.7.1 wrote them for each team. Since the amendment of 2026-11-24 of ADR 0004, a new `lite` project has no advisor. Its page has no advisor row and no section "Advisor", but it names `/advisor`.

Change: the condition is `h.on(Role::Advisor)` only. A `lite` team with no advisor gets the list of `standard` with no advisor:

```md
## Check after a restart

1. Read the header. It shows the main model.
2. Type `@`. The list shows the agents.
3. Run `/tasks`. It lists the running agents.
4. Run `/usage`. It shows the cost of each model.
```

A `lite` team with the advisor keeps the page of 0.7.1, byte for byte (`tests/golden/harness/model-team-0.7.1.md`). The restart line of `init` follows the advisor already (`RESTART_NO_ADVISOR`, `src/init.rs:3016`). Upgrade: a `lite` project with no advisor gets SD502 on `.claude/agents/model-team.md` until `upgrade` or `init` writes it again, as after each release. Cost: none in a request, because agents do not read the page.

## L3. A new profile keeps the role `advisor` of the file

Cause: `Plan::apply_preset` (`src/init.rs:419-428`) moves a role `advisor` whose model is the default of the old profile to the default of the new profile (D97, item 3). The file has no record of who set a value. Thus the answer `advisor=none` of a person in `standard` becomes `fable` in `strict`, and the answer `advisor=fable` in `strict` becomes `none` in `standard`.

Change: with `--reconfigure`, a new answer to question 1 moves no role `advisor` of a harness that the file has. Question 13 changes the role, and the note of `--reconfigure` names question 13 already. A harness that the run adds (from the level `none`, or from a file with no table `[harness]`) is new, and it gets the role of the new profile, as now. A new project keeps the preset. Thus a project that `--reconfigure` moves from `standard` to `strict` has no advisor until question 13 turns it on. D97 says that `strict` keeps the advisor: that stays true for a new project of `strict`.

Rejected: a key that records the origin of a value (a new contract for one value); a move in one direction only (it still loses a value of a person); a move with a note (the value still changes with no answer).

## L4. `advisorModel` goes when the advisor goes

Cause: `settings_change` (`src/harness.rs:2499`) keeps each key that the old configuration set and the new one does not (`kept_keys`, `:2136`). After the answer `advisor=none`, `advisorModel` stays. Claude Code then gives the session an advisor, but the model-team line, the team page and the deleted `fallback-advisor.md` say that the team has none.

Change: in `init --reconfigure`, when the old configuration has the role `advisor` on with the model `<m>` and the new one has it off (the model `none`, or the level `none`), `settings_change` removes `advisorModel` if its value is `<m>`. The preview shows the removal in the diff of `.claude/settings.json`, and the note of the file names `advisorModel removed`. A different value stays with its `kept` line, because a person set it. A key that `init` took from an existing settings file when it wrote the configuration (`docs/init.md`, "Harness defaults") has the model of the configuration, so it goes too. `init` with no `--reconfigure` and `upgrade` give `plan` the same configuration as old and new (`src/upgrade.rs:210`), so they remove nothing. `eval run` gives no old configuration (`src/eval_ab.rs:1300`), so the copies of the harness arms do not change for the paid reruns.

Not covered: a hand edit of the role to `none` before `init` (no old configuration, so the key stays with no line), and `advisorModel` in the user settings.

## L5. SD942: a repeated deviation id in an early plan

Cause, proved in a lab fixture (the code and the message of the failed commit of item 6 of part A were not recorded):

1. A plan that a commit added when `spec-dna.toml` pinned a version before 0.8.0 keeps the meaning of 0.7.1 (`acceptance::section_is_read`). SD939 does not read its section `Deviations`, also when a later commit at the pin 0.8.0 wrote that section.
2. The plan has 2 items with the id `t1`. `section_stop` sees no finding, so `next` names the acceptor.
3. The brief has the text of the plan, and its step 7 says: "Answer each invariant and each deviation that this text lists." The acceptor writes 2 `[[deviation]]` entries with the id `t1`.
4. The commit of the record fails with SD928, whose fix says "Fix the record". After the refusal, `next` says "commit the record alone, then run `spec-dna finish`" (`src/stage.rs:541`), so the next commit fails the same way.

A plan that a commit at the pin 0.8.0 added gets SD939 for the repeated id, and the hook refuses the commit of the builder first.

Change:

1. The new code SD942 `deviation-id-repeated`: in a plan whose sections the rule of the pin does not read, a later item of `Deviations` in the form `- <id> (<kind>): <text>` has the id of an earlier item. It is off in `lite`, a warning in `standard` and `strict`, and in the group `acceptance`. It is a new rule, so it starts as a warning (ADR 0003). It reads the ids only. Each other item of such a plan keeps the meaning of 0.7.1.
2. A plan that the rule of the pin reads keeps SD939 for a repeated id and gets no SD942, so each item has one finding. The rule of the pin does not change.
3. `deviation_diags` gives a repeated id both as SD939 and as SD942, also when only SD942 is on. The one function of the rule of the pin, which `check` (`lifecycle::skip_early_sections`) and `next`, `accept` and `finish` (`acceptance::sections_of`) use, keeps SD942 only on a plan that it does not read, and SD939 only on a plan that it reads. Thus SD942 does not need SD939 to be on.
4. `acceptance::section_stop` stops on SD942 after SD940. `next` names the builder with the text of D88, and `accept` and `accept --brief` refuse with exit code 1. `first_stop` (step 5.2 of `finish`) does not stop on SD942: a valid record of an early plan needs no `[[deviation]]` entry.
5. `next`, for a record that has an edit that `HEAD` does not have: first `section_stop`, then the rules of a valid record (`acceptance::read_record`), then the text of 0.8.0.

The message of SD942:

```text
the id `<id>` is used already in `Deviations`; SD939 reads this plan as 0.7.1 does, because a commit added it before the pin 0.8.0, but an acceptor answers each item and the record takes one `[[deviation]]` entry for each id: give each item its own id, for example `<id>-2`
```

The record item of `next` with an uncommitted record, when `section_stop` finds an item, and when the record is not valid:

```text
the plan of the change `<slug>` has an item that is not valid (<code> at <path>:<line>: <message>); delete the uncommitted record `<path of the record>`; the builder fixes the item and commits, and then starts the acceptor
the record `<path>` has an edit that `HEAD` does not have, and it is not valid (SD928: <first problem>); fix the record, then commit it alone
```

The fix text of SD942 in `src/diag.rs`:

```text
A plan that a commit added before the pin 0.8.0 repeats the id of an item of `Deviations`. SD939 keeps the meaning of 0.7.1 for such a plan, but the brief gives the acceptor each item, and the record takes one `[[deviation]]` entry for each id (SD928). The builder gives each item its own id and commits. The acceptor does not edit the plan.
```

SD942 also reads an early plan that needs no record (the note of an old plan in `next`): a person can still start an acceptor, and `accept --brief` then stops on it. Upgrade: an open change whose early plan repeats a deviation id gets the warning SD942. A plan of the base has it in the base too, so it is not new. No other project gets a finding.

## L6. SD646 is a warning in `strict`

Cause, measured with the binary of `475d8a8` in a lab fixture: a `strict` project has the area `zones` with 1 requirement, its summary, and a `system` document that pins the summary. A change removes the requirement and edits the text of the summary. `finish --dry-run` and `finish` stop at step 5 with SD646: "the result would add 1 error(s)". Each other way out gives an error: `system` with no pin to the summary gives SD642, and a deleted summary gives SD202. In `standard`, `finish` exits with 0, and `check` then gives the warning SD646.

Change: SD646 is a warning in `strict` (amendment of ADR 0017). `finish` then passes, and `check` gives the warning. The message and the fix of SD646 no longer tell a person to delete the summary, because the delete gives SD202 in a layer with `keep = "forever"`. The message, then the fix:

```text
`<id>` rests on nothing: it has no pin and no item of a source layer is in its area, and `<pinning id>` pins it; add the layer of its sources to `rolls-up`, or add a source to its area; after a change that removed its last item, keep its text true
A roll-up document has no pin and an empty frontier, and another roll-up document pins it, so the chain rests on nothing. The message names the document that pins it. Add the layer of the sources of its area to `rolls-up`, or add the sources. After a change that removed the last item of the area, the warning needs no other action: keep the text of the summary true.
```

Rejected: an exception of SD202 for a summary (a change of the information model for one case); no count of SD646 in step 5 of `finish` (ADR 0028 says that `finish` stops on each new error, and `check` of the pull request still fails); a frontier with no empty summary (SD641, an error in `standard`, on each current pin to such a summary).

## L7. The retrieval benchmark of the synthetic corpora

Cause: `bench/results/retrieval-synthetic-medium.json` comes from pull request #1 (`7dfc01d`) and never changed. At 8,000 characters it has 5,227 characters, recall 0.995 and precision 0.756. The binary of `475d8a8` gives 5,271, 0.998 and 0.759: #5 (`a5f234e`) changed the recall, and #27 (`83e4abc`) added 8 characters to each section header. The design of part A compares with this file (`docs/design/0.8.1-a-evidence-and-context.md:301` and `:555`). No test and no CI step reads `bench/results/retrieval-*`.

Change: after the change `path-summaries` merges, because it changes what `context` serves, `context-engineer` runs the synthetic corpora `medium` and `large` at 4,000, 8,000 and 12,000 with the binary of `master`. The results replace `retrieval-synthetic-medium.json` and `retrieval-synthetic-large.json`, so the 2 lines of the design name a current file with no edit. `bench/retrieval/README.md` gets the new table, with its date and commit, as the current one. The table of 2026-11-08 and its `-s3` files stay as history. The history table of 2026-11-07 names `7dfc01d` for its raw rows. One sentence says: a change of the change set "Context plan or rank" writes the new result files. If `path-summaries` writes the 2 files at its merge commit, the task closes with `Skip:` and that commit.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0017-profiles]]
- [[adr/0032-acceptance-in-one-pass]]
- [[adr/0004-harness]]
- [[adr/0028-roll-up-and-summaries]]

## Concerns

- [x] ADR 0004 and ADR 0028 hold the rules that L4 and L6 change, and both are at their hard budget. Resolved: ADR 0032 and ADR 0017 hold the amendments, and each names the ADR that it changes, as ADR 0019 does for ADR 0012.
- [x] SD942 is a new finding for an upgraded project. Resolved: it is a warning, it needs an open change with a repeated id, and the same plan already fails at the commit of the acceptor.
- [x] L3 leaves the key `advisor` of the entry `acceptor` with its move. Resolved: no question asks for that key, so the move is the only way that `init` gives it the default of the new profile. `docs/config.md` states the limit.
- [x] L7 waits for another change. Resolved: the code of the other tasks can merge first, and `finish` follows in a later pull request when each task is closed (D87).
- [x] D97 item 3 of `docs/design/0.8.1-a-lifecycle-and-harness.md` and the severity of SD646 in D84 of `docs/design/0.8.1-a-evidence-and-context.md` become false. Resolved: this spec replaces both, and the design pages of part A stay as the record of part A, with no edit.
- [x] The checks of `next` and `accept` read the plan. Resolved: `sections_of` reads it already, and SD942 reads the same items.

## Open questions

- [x] Is the cause of L5 the cause of the failed commit of the acceptor of item 6 of part A? Answer of the design: the code and the message of that commit were not recorded, so this spec fixes the cause that a lab fixture proves. The report of this design asks the owner for the message (OWNER QUESTION), and L5 does not depend on the answer.
