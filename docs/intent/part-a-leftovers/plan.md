---
status: draft
---
# The open points of part A of 0.8.1: plan

## Files

- `docs/adr/0017-profiles.md` The amendment of 2026-11-25: SD646 in `strict` (written with this plan).
- `docs/adr/0032-acceptance-in-one-pass.md` The amendment of 2026-11-25: a team with no advisor (written with this plan).
- `tests/release_check.rs` L1: the copy of the release script runs with `bash`.
- `tests/common_helpers.rs` L1: the stub of the test of `stub_bin` runs with `sh`.
- `src/harness.rs` L2: the check list of the team page. L4: the removal of `advisorModel`, and the unit tests of both.
- `src/init.rs` L3: no move of the role `advisor` under `--reconfigure`.
- `tests/harness_skills.rs` L2: the team page of `lite` with and with no advisor.
- `tests/init_profiles.rs` L3 and L4: the role and `advisorModel` under `--reconfigure`.
- `tests/harness.rs` L4: the level `none` removes `advisorModel`.
- `src/diag.rs` L5: the code SD942. L6: the fix text of SD646.
- `src/config.rs` L5: the profile row of SD942 and the group `acceptance`. L6: the profile row of SD646. The unit tests of both rows.
- `src/change.rs` L5: the mark of a repeated id on an item of `Deviations`.
- `src/lifecycle.rs` L5: SD942 in `deviation_diags` and in the rule of the pin.
- `src/acceptance.rs` L5: SD942 in `sections_of` and in `section_stop`.
- `src/stage.rs` L5: the text of `next` for a record that `HEAD` does not have.
- `tests/acceptance_checks.rs` L5: SD942 in `check`.
- `tests/accept_next.rs` L5: SD942 and a refused record in `next`.
- `tests/accept_cmd.rs` L5: the refusal of `accept`.
- `src/rollup.rs` L6: the message of SD646.
- `tests/rollup_checks.rs` L6: SD646 in `strict`, and `finish` after the removal of the last requirement of an area.
- `docs/harness.md` L2 and L4: the text of `lite`, the merge of the settings and the removal.
- `docs/init.md` L3 and L4: `--reconfigure` and a smaller configuration.
- `docs/profiles.md` L2, L3, L5 and L6: the text of `lite`, `--reconfigure`, the rows of SD942 and SD646.
- `docs/diagnostics.md` L5 and L6: the row of SD942, the rule of the pin, the row and the notes of SD646.
- `docs/config.md` L5: the group `acceptance` holds SD927 to SD942.
- `docs/commands.md` L5: the sections `accept` and `next`.
- `docs/guides/change.md` L5: the list of the stops before the acceptor and the table of the codes.
- `bench/results/retrieval-synthetic-medium.json` L7: the result of the current binary.
- `bench/results/retrieval-synthetic-large.json` L7: the result of the current binary.
- `bench/retrieval/README.md` L7: the current table, the history and the rule of a rerun.
- `CHANGELOG.md` The entries of the change.

## Order

- [ ] t1: Run the copy of `scripts/verify-release.sh` in `run_with_cross` as `Command::new("bash").arg(...)` (`release-engineer`). Run the stub of `stub_bin_writes_an_executable_script` with `sh`, and keep the check of its mode (`test-engineer`). Files: `tests/release_check.rs`, `tests/common_helpers.rs`. Verify: `cargo test --test release_check --test common_helpers`, then in a Linux container (`docker run` of `rust:1.99.1-slim-bookworm` with the checkout mounted) build the tests and run the test binary of `release_check` 300 times with `--test-threads 4` and 300 times with `--test-threads 16`: no `Text file busy` (before: 25 of 600 runs failed, measured on 2026-11-25). Hours: 1.
- [ ] t2: Write the steps on the advisor in the check list of the team page only when the role `advisor` is on, also in `lite` (L2). Change the last assertion of `the_check_list_names_the_advisor_only_where_it_is_on` to the list with no advisor, and add the case of a new `lite` project with no advisor to `tests/harness_skills.rs` (`init-engineer`). Files: `src/harness.rs`, `tests/harness_skills.rs`, `docs/harness.md`. Verify: `cargo test --bin spec-dna harness::` and `cargo test --test harness_skills`; `lite_writes_the_team_page_of_0_7_1` passes with no edit. Hours: 1.
- [ ] t3: Under `--reconfigure`, move no role `advisor` of a harness that the file has when question 1 gets a new answer (L3). A harness that the run adds keeps the role of the new profile. Change `reconfigure_moves_the_advisor_with_the_profile` to `reconfigure_keeps_the_advisor_with_a_new_profile` (`init-engineer`). Files: `src/init.rs`, `tests/init_profiles.rs`, `docs/init.md`. Verify: `cargo test --test init_profiles`; `reconfigure_gives_a_new_harness_the_advisor_of_the_profile` passes with no edit. Hours: 2.
- [ ] t4: In `settings_change`, remove `advisorModel` when `--reconfigure` turns the role `advisor` off and the value is the model of the old configuration, and name the removal in the note of the file (L4). A different value keeps its `kept` line. Change the assertion of `advisorModel` in `reconfigure_moves_between_the_levels_and_removes_what_it_wrote` (`init-engineer`). Files: `src/harness.rs`, `tests/init_profiles.rs`, `tests/harness.rs`, `docs/harness.md`, `docs/init.md`. Verify: `cargo test --bin spec-dna harness::` and `cargo test --test init_profiles --test harness`; `lean_no_advisor_has_no_advisor_model_and_no_fallback_agent_and_lean_has_both` of `tests/eval_ab.rs` passes with no edit. Hours: 2.
- [ ] t5: Add the code SD942, its profile row (off, warning, warning) and its place in the group `acceptance` (L5). Mark a repeated id on an item of `Deviations`, give it as SD939 and as SD942 in `deviation_diags`, and keep SD942 only on a plan that the rule of the pin does not read, in the one function that `check` and `sections_of` use (`diagnostics-engineer`; `core-engineer` for `src/config.rs` and `src/change.rs`). Files: `src/diag.rs`, `src/config.rs`, `src/change.rs`, `src/lifecycle.rs`, `src/acceptance.rs`, `tests/acceptance_checks.rs`. Verify: `cargo test --bin spec-dna config::` and `cargo test --test acceptance_checks`. Hours: 4.
- [ ] t6: Stop `next` and `accept` on SD942 before the acceptor through `section_stop`, and give the text of L5 to `next` for a record that `HEAD` does not have (`core-engineer`; `init-engineer` for `tests/accept_next.rs`). Files: `src/acceptance.rs`, `src/stage.rs`, `tests/accept_next.rs`, `tests/accept_cmd.rs`. Verify: `cargo test --test accept_next --test accept_cmd`. Hours: 3.
- [ ] t7: Make SD646 a warning in `strict`, and give it the message and the fix of L6 (`core-engineer` for the row of `src/config.rs`, `diagnostics-engineer` for `src/diag.rs`, `hygiene-engineer` for `src/rollup.rs` and its tests). Files: `src/config.rs`, `src/diag.rs`, `src/rollup.rs`, `tests/rollup_checks.rs`. Verify: `cargo test --bin spec-dna config::` and `cargo test --test rollup_checks`. Hours: 2.
- [ ] t8: Document SD942, the new severity of SD646, the stops of `accept` and `next`, `lite` and the group `acceptance` (`diagnostics-engineer` for `docs/diagnostics.md` and `docs/profiles.md`, `core-engineer` for `docs/config.md` and `docs/commands.md`; `architect` gives the facts of `docs/guides/change.md`, and `docs-editor` the language). Files: `docs/diagnostics.md`, `docs/profiles.md`, `docs/config.md`, `docs/commands.md`, `docs/guides/change.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [ ] t9: After the change `path-summaries` merges into `master`, build the release binary of `master` and, for each size `medium` and `large`: write the corpus with `python3 -I bench/gen/gen.py --model spec-dna --size <size> --out <dir>`, set its line `spec-dna = "..."` to the version of the binary, commit the corpus in a new git repository, and run `python3 -I bench/retrieval/run.py --repo <dir> --tasks <dir>/tasks.json --budgets 4000,8000,12000 --out <file>`. Write the results in place of the 2 files, and write the current table, the history and the rule of a rerun in the README (L7). If `path-summaries` wrote the 2 files at its merge commit, close the task with `Skip:` and that commit (`context-engineer`). Files: `bench/results/retrieval-synthetic-medium.json`, `bench/results/retrieval-synthetic-large.json`, `bench/retrieval/README.md`. Verify: at 8,000 the medium file no longer has 5,227 characters, recall 0.995 and precision 0.756. A value other than 5,271, 0.998 and 0.759 (the binary of `475d8a8`) has a line in the README that names the commit that changed it. Each row of the new README table equals the `summary` of its file. Hours: 1.
- [ ] t10: Write the entries of the change: SD942 under "Added"; the severity of SD646 in the 2 current lines on SD646; the team page of `lite`, the role under `--reconfigure` and `advisorModel` under "Changed"; one line under "Upgrade from 0.8.0" for SD942 and the page of `lite` (each responsible agent writes its entry, and `release-engineer` edits the file). Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t11: Run the review and the acceptance pass, then finish the change. Files: none: `finish` deletes the folder of the change. Verify: `spec-dna finish part-a-leftovers --dry-run`. Hours: 2.

## Change sets

The rows of `.claude/agents/ownership.md` that each fix touches:

| Fix | Change set or work area | A | R |
| --- | --- | --- | --- |
| L1 | "CI or hooks" (`tests/release_check.rs`), and the work area "Test strategy, shared test code" (`tests/common_helpers.rs`) | `architect`, `test-engineer` | `release-engineer`, `test-engineer` |
| L2, L3, L4 | "Harness role or generated file" | `architect` | `init-engineer` |
| L5 | "Diagnostic code: new" | `architect` | `diagnostics-engineer`, `core-engineer` for `src/config.rs`, `src/change.rs` and `src/stage.rs` |
| L6 | "Diagnostic code: new severity" | `architect` | `diagnostics-engineer`, `core-engineer` for `src/config.rs`, `hygiene-engineer` for `src/rollup.rs` |
| L7 | "Context plan or rank" (`bench/results/retrieval-*.json`) | `architect` | `context-engineer` |
| The amendments of ADR 0017 and ADR 0032 | "Information model" (the owner accepts the text) | the owner | `architect` |

## Texts that agents read

The texts of the skills and of the agent files are frozen before the paid reruns of this session. This change edits none of them: no skill text, no agent file of `init`, no line of `CLAUDE.md` and no file of `.claude/` of this repository. These texts change, and the spec gives each one in full:

- The message and the fix of SD942 (new), and the message and the fix of SD646 (`check`).
- 2 texts of the record item of `next`: a finding before the acceptor with an uncommitted record, and a record that is not valid.
- The check list of `.claude/agents/model-team.md` of a `lite` team with no advisor. The page is for people, and agents do not read it.

## Risks

- t9 waits for `path-summaries`. The code of t1 to t8 can merge first, and `finish` follows in a later pull request when t9 is closed (D87).
- The changes `harness-reports` and `harness-tools` also edit `src/harness.rs`, `src/init.rs`, `docs/harness.md`, `docs/init.md`, `docs/profiles.md` and `CHANGELOG.md`. The change that merges second rebases, and runs each check again.
- SD942 is the provisional next free code. If another change of this part takes SD942 first, this change takes the next free code in each file of the list.
- A `lite` project with no advisor gets SD502 on `.claude/agents/model-team.md` after the upgrade, until `upgrade` or `init` writes the page again, as after each release.
- L1 has no test that can fail first: the race needs load. The loop of t1 is the proof.

## Proof

Each test fails before the fix, unless it says "guard".

- L2: `the_check_list_names_the_advisor_only_where_it_is_on`, `lite` with no advisor: the list of 4 steps (before: the list with `/advisor`). `tests/harness_skills.rs`: `init` of a new `lite` project writes a team page with no `/advisor` and no `Advising` (before: both). `lite_writes_the_team_page_of_0_7_1` is a guard.
- L3: `standard` to `strict` keeps `advisor = { claude-code = "none", ... }` and writes no `advisorModel` (before: `fable`). `strict` to `standard` keeps `fable` (before: `none` and a `kept` line). The answer to question 13 wins (guard). `reconfigure_gives_a_new_harness_the_advisor_of_the_profile` (guard).
- L4: `keys_that_the_new_configuration_drops_are_kept_with_a_note`: `advisorModel` is gone, with no `kept` line, and the other `kept` lines stay (before: `kept advisorModel`). `tests/init_profiles.rs`: `advisor=fable`, then `advisor=none`: no `advisorModel`, and the preview shows the removal (before: the key and a `kept` line). `harness = "none"` removes it too (`tests/harness.rs`, before: the key stays). An `advisorModel` of `opus` against a configuration of `fable` stays, with its `kept` line (guard).
- L5: `tests/acceptance_checks.rs`: a plan that the branch added at the pin 0.7.1, then the pin of the binary, with 2 items `t1`: `check` gives the warning SD942 at the second item in `standard`, and no SD939 (before: no finding). The same plan added at the pin of the binary gives SD939 and no SD942 (guard). With `[rules] SD939 = "off"`, the early plan still gives SD942. `tests/accept_next.rs`: the item names the builder and SD942 (before: the acceptor). An uncommitted record with a `pending` value: the item names SD928 and the problem (before: "commit the record alone"). An uncommitted record and the early plan with 2 items `t1`: the item names SD942 and "delete the uncommitted record" (before: "commit the record alone"). A plan of 0.7.1 in the base that needs no record: SD942 is not new against the base (guard), and `accept --brief` gives exit code 1. `tests/accept_cmd.rs`: `accept` and `accept --brief` give exit code 1 and write no file (before: exit code 0 and a skeleton).
- L6: the unit test of the roll-up rows: SD646 is a warning in `strict` (before: an error). `tests/rollup_checks.rs`: the fixture of L6 in `strict`: `finish` exits with 0, and `check` gives the warning SD646 (before: exit code 1 at step 5). `a_summary_with_no_pin_and_no_item_that_system_pins_gives_sd646` expects a warning in `strict`.
- L7: no test reads the files. The proof is the compare of t9.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result `accepted`.
