---
status: draft
---
# The eval runner counts each segment, runs one arm alone, and keeps a floor of free space: plan

## Files

- `docs/design/0.8.1-a-lifecycle-and-harness.md` The design of the turns (D95) and of the runner (D100).
- `docs/adr/0030-harness-arms-time-record-and-verdict-floor.md` The amendment of 2026-11-24 (slice A10 of the design).
- `src/eval_run.rs` The arguments of claude, the reader of the stream, the sum of the turns, the key segments, the check of the free space, and the stop of a plain run.
- `src/eval_ab.rs` The key segments in the results of the pairs and the arms, one arm alone, and the check of the free space before each run.
- `src/verdict.rs` The join of the result files in the comparison.
- `src/main.rs` The help text of eval, and the files of the option compare.
- `src/config.rs` The function of the table evals with the default values, and the key min-free-mib.
- `src/init.rs` The default table evals of init uses the new function.
- `src/evals.rs` The table of the unit tests uses the new function.
- `src/eval_pack.rs` The table of the unit tests uses the new function.
- `schema/spec-dna.schema.json` The key min-free-mib.
- `tests/eval_run.rs` The stubs in the stream form, and the tests of the turns, of the stream and of the floor.
- `tests/eval_ab.rs` The tests of the segments, of one arm, of the comparison of files and of the floor.
- `docs/evals.md` The keys of a result, the stops, one arm, the comparison of files and the floor.
- `docs/config.md` The row of the key min-free-mib.
- `docs/commands.md` The section eval and the table of the exit codes.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Pass `--output-format stream-json --verbose` to claude, read the stdout line by line, and keep the events whose top-level type is result, from the lines that hold the bytes `"result"`. Read the cost, the subtype, the models and the tokens from the last result event. The stub that prints a JSON array prints its 2 events on 2 lines, and the tests of the command line get the new arguments. Files: `src/eval_run.rs`, `tests/eval_run.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_run --test eval_ab` and `cargo test --bin spec-dna eval_run::`. Hours: 4.
- [ ] t2: Make turns the sum of num_turns over the result events, and write the key segments after turns in each result of a plain run (D95 items 3 and 4). Files: `src/eval_run.rs`, `tests/eval_run.rs`. Verify: `cargo test --test eval_run`. Hours: 2.
- [ ] t3: Write the key segments in each side of a pair of `--ab` and of a wave of `--arms`, and read it back with `--resume`, as null for a result of 0.8.0. Files: `src/eval_ab.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab`. Hours: 2.
- [ ] t4: Let `--arms` take 1 to 3 arms: with 1 arm, wave r runs each eval once in that arm, and `--resume` works as with more arms. Change the help text of eval. Files: `src/eval_ab.rs`, `src/main.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab`. Hours: 2.
- [ ] t5: Let `eval verdict --compare` read 1 or more result files of `--arms` and join them by the eval and the number of the run. A run in 2 files is exit code 3, and files of 2 versions of Claude Code give a note. Files: `src/verdict.rs`, `src/main.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab` and `cargo test --bin spec-dna verdict::`. Hours: 4.
- [ ] t6: Give the table evals one function that builds it with the default values for a folder, and use it in the default table of init and in the table of the unit tests of the eval files. Files: `src/config.rs`, `src/init.rs`, `src/evals.rs`. Verify: `cargo test --bin spec-dna config::`, `cargo test --bin spec-dna init::` and `cargo test --bin spec-dna evals::`. Hours: 1.
- [ ] t7: Use the function in the table of the unit tests of the pack. Add the key min-free-mib: an integer of 0 or more, the default 2048, exit code 3 for another value, the schema, and the key in the test of the keys of the table evals. Files: `src/eval_pack.rs`, `src/config.rs`, `schema/spec-dna.schema.json`. Verify: `cargo test --bin spec-dna config::` and `cargo test --bin spec-dna eval_pack::`. Hours: 2.
- [ ] t8: Before each attempt of a plain run, read the free space of the temporary folder and of the folder of the result with `df -Pk`. Below the floor, start no run, write the result with stopped disk-space, print the message and exit with code 3. A floor of 0 turns the check off, and a df that fails or gives no value gives one warning. Files: `src/eval_run.rs`, `tests/eval_run.rs`. Verify: `cargo test --test eval_run` and `cargo test --bin spec-dna eval_run::`. Hours: 4.
- [ ] t9: Run the same check before each attempt of `--ab` and `--arms`, also after a refusal of the API. Keep the complete pairs and waves, set stopped to disk-space, and let `--resume` continue. Files: `src/eval_ab.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab`. Hours: 3.
- [ ] t10: Document the keys turns and segments, the limit of the turns for each segment, the stop disk-space, one arm, the comparison of files and the floor, and change the sentence on the records of 0.8.0. Files: `docs/evals.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [ ] t11: Document the key in the table of the keys, and the forms of `--arms` and `--compare` and the 2 new causes of exit code 3 in the section eval. Files: `docs/config.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 1.
- [ ] t12: Write the entry of the change. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t13: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish eval-runner --dry-run`. Hours: 3.

## Risks

- The stream after a wake-up is an assumption: no run kept it. If a segment writes no `result` event, `turns` counts less than the transcript. Q6 of the design asks for one repro run, and the stub pins the assumed shape.
- A machine with less than 2,048 MiB free stops each run, also in the tests of `eval run` and in the self-test of `bench/team/`. The message names the key and the folder.
- `df` is a POSIX tool, and its output can differ on a system that the tests do not cover. A value that does not parse gives one warning and no check, so the runner never stops for a wrong reading.
- The change `advisor-default` also edits `src/init.rs`, `tests/eval_run.rs` and `docs/evals.md`, and the design orders it after this change. The change that merges second rebases and runs each check again.
- The arms that run at the same time share one machine and one account, so their seconds are not those of a quiet run. The docs of `--arms` state it.

## Proof

Each test fails before the fix, unless it says "guard". No test starts a real `claude`.

- `tests/eval_run.rs`, `turns_is_the_sum_over_the_segments`: a stub prints a system event, an assistant event, a result event with num_turns 5 and a cost of 0.1, and a result event with num_turns 2 and a cost of 0.3. After: turns 7, segments 2, a cost of 0.3. Before: the reader cannot parse 4 lines as one value, so turns and the cost are null.
- `a_run_that_stops_at_the_turn_limit`: the stub reads `--max-turns` (3), checks `--output-format stream-json --verbose`, and prints one result event with the subtype error_max_turns and num_turns 3. After: turns 3, segments 1, a failed run and no refusal. Before: the stub finds no stream-json and fails.
- `the_result_after_16_mib_of_events_is_read`: the stub prints more than 16 MiB of assistant events, then the result event. Before: the cost is null. After: the cost is read.
- `the_command_line_of_claude_has_the_exact_order_and_no_bare`, and the test of the command line in `tests/eval_ab.rs`, get the new arguments.
- A stub that prints one compact result object on one line gives the result of 0.8.0, with segments 1 (guard). A result event with a space after each colon is read (guard for the stubs of `bench/`).
- `tests/eval_ab.rs`, `arms_runs_one_arm_alone`: `--arms none` gives a result file with the arm none. Before: exit code 2 and "--arms takes 2 or 3 arms, got 1". The test of the usage errors keeps the case of 4 arms with the new text, and loses the case of 1 arm.
- `compare_joins_2_files_of_one_arm`: 2 files of one arm each give the verdict that one file of 2 arms gives. `a_run_in_2_files_is_exit_code_3`. Before: the option reads one file, and a second file is a usage error.
- The segments of a pair and of a wave, and `--resume` of a file of 0.8.0 that writes segments null.
- `tests/eval_run.rs`, `a_disk_below_the_floor_stops_before_a_run`: a stub `df` on `PATH` prints 1,024 KiB free, and the key is 2048. After: exit code 3, stopped disk-space, and the stub claude did not run. Before: the run starts.
- `a_floor_of_0_turns_the_check_off` (guard: before and after, the run starts), and `a_df_that_fails_gives_one_warning`.
- `tests/eval_ab.rs`: the same stop before the second pair keeps the first pair, and `--resume` after the stub `df` reports space runs the rest.
- The unit tests of `src/config.rs`: the key in the schema and in the parser, the default 2048, and exit code 3 for -1, 1.5 and a string. The unit tests of the parse of the output of `df`: a folder name with spaces gives the right field.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
