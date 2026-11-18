---
status: approved
---
# Fixes of 0.7.1, part B: plan

## Files

- `docs/design/0.7.1-b.md` The decisions of the change.
- `docs/adr/0019-limits-of-0-7-0-part-a.md` The amendment of B3.
- `docs/adr/0020-hours-and-state-of-a-task.md` The amendment of B1 and B7.
- `docs/adr/0023-default-skills-from-the-release-table.md` The amendment of B4.
- `src/change.rs` The label `Actual:` and the task that closes with `finish`.
- `src/stage.rs` The open task that `finish` closes.
- `src/finish.rs` Step 0, and the actual hours in the line `Hours:`.
- `src/sdlc.rs` The pair `actual` in `log`.
- `src/main.rs` The `check`, `finish` and `status` handlers and their help.
- `src/lifecycle.rs` SD913, SD923, SD924 and the base of the change checks.
- `src/approval.rs` The sentence of reason 9.
- `src/git.rs` The lookups of B5 and B10.
- `src/config.rs` The key `shared-paths` and the pin rule of SD503.
- `src/detect.rs` The detection of a typos hook and of the shared documents.
- `src/init.rs` The notes, the `[change]` text, `is_tool_file` and the skill preset.
- `src/upgrade.rs` The `next:` line, the stash text and `no_status`.
- `src/upgrade_config.rs` The skills that are new in 0.7.0.
- `src/eval_run.rs` The adapter line of the eval copy.
- `src/eval_ab.rs` The rules of the eval copy.
- `src/harness_text.rs` The text of `write-plan`, `specify`, `execute` and `finish`.
- `src/verdict.rs` The table of the release.
- `schema/spec-dna.schema.json` The key `shared-paths`.
- `tests/*.rs` The tests of each item.
- `tests/fixtures/upgrade/**` The fixture project of 0.7.0.
- `tests/golden/harness/skills/*.md` The golden files of the skills.
- `tests/golden/upgrade/config/*.toml` The golden files of `upgrade`.
- `bench/results/evals/*.jsonl` The records of the run of `write-plan`.
- `docs/commands.md` The sections of `check`, `next`, `status`, `finish`, `log` and `upgrade`.
- `docs/config.md` The key `shared-paths`, the order of the base and the version.
- `docs/diagnostics.md` SD503, SD910, SD913, SD923 and SD924.
- `docs/profiles.md` The default skills and SD923.
- `docs/init.md` The spell checkers and the version.
- `docs/evals.md` The eval copy and the run of `write-plan`.
- `docs/types.md` The label `Actual:`.
- `docs/quickstart.md` The version.
- `docs/guides/change.md` The rules of B1, B6 and B7 and the path outside `Files`.
- `docs/guides/upgrade-0.6.md` The upgrade in one commit.
- `CHANGELOG.md` The section of 0.7.1.
- `Cargo.toml` The version.
- `Cargo.lock` The version.
- `spec-dna.toml` The pin.
- `README.md` The version.

## Order

- [ ] t1: Skip SD923 and SD924 for a plan that was added under a pin before 0.7.0, and give the change checks of an older `--base` the merge base with `origin/HEAD` (B5 and B10, `diagnostics-engineer`). Files: `src/lifecycle.rs`, `src/git.rs`, `src/main.rs`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/commands.md`, `docs/config.md`, `docs/guides/upgrade-0.6.md`, `tests/task_checks.rs`, `tests/lifecycle_base.rs`. Verify: `cargo test --test task_checks --test lifecycle_base` passes, and the new cases failed before the change. Hours: 4.
- [ ] t2: Let SD503 and SD913 skip a pin that moves to a later version, and add the key `[change] shared-paths` (B3 checks and B8, `core-engineer`, `diagnostics-engineer`, `init-engineer`). Files: `src/config.rs`, `src/lifecycle.rs`, `src/main.rs`, `src/init.rs`, `src/detect.rs`, `schema/spec-dna.schema.json`, `docs/diagnostics.md`, `docs/config.md`, `docs/init.md`, `docs/adr/0019-limits-of-0-7-0-part-a.md`, `tests/upgrade_pin.rs`, `tests/lifecycle_checks.rs`, `tests/change_config.rs`, `tests/init_graph.rs`. Verify: `cargo test --test upgrade_pin --test lifecycle_checks --test change_config --test init_graph` passes, and the new cases failed before the change. Hours: 4.5.
- [ ] t3: Print the typos note, the `next:` line and the stash text, keep the default skills at the upgrade, fix `no_status`, and turn SD501 and SD502 off in the eval copy (B2, B3 text, B4 and B11 runner, `init-engineer`, `core-engineer`). Files: `src/upgrade.rs`, `src/upgrade_config.rs`, `src/init.rs`, `src/detect.rs`, `src/eval_run.rs`, `src/eval_ab.rs`, `docs/init.md`, `docs/profiles.md`, `docs/commands.md`, `docs/evals.md`, `docs/guides/upgrade-0.6.md`, `docs/adr/0023-default-skills-from-the-release-table.md`, `tests/cli.rs`, `tests/upgrade_cli.rs`, `tests/upgrade_config.rs`, `tests/init_profiles.rs`, `tests/eval_ab.rs`, `tests/golden/upgrade/config/harness-full.after.toml`. Verify: `cargo test --test cli --test upgrade_cli --test upgrade_config --test init_profiles --test eval_ab` passes, and the new cases failed before the change. Hours: 4.5.
- [ ] t4: Close a task that names `spec-dna finish <slug>` with `finish`, add the label `Actual:`, give the step back to `draft` a documented way, and write the new skill text (B1, B6, B7 and B11 text, `hygiene-engineer`, `core-engineer`, `init-engineer`, `diagnostics-engineer`). Files: `src/change.rs`, `src/stage.rs`, `src/finish.rs`, `src/sdlc.rs`, `src/main.rs`, `src/approval.rs`, `src/harness_text.rs`, `docs/types.md`, `docs/commands.md`, `docs/diagnostics.md`, `docs/guides/change.md`, `docs/adr/0020-hours-and-state-of-a-task.md`, `tests/finish.rs`, `tests/lifecycle_commands.rs`, `tests/lifecycle_log.rs`, `tests/branch_approval.rs`, `tests/harness_skills.rs`, `tests/golden/harness/skills/spec-dna-write-plan.md`, `tests/golden/harness/skills/spec-dna-specify.md`, `tests/golden/harness/skills/spec-dna-execute.md`, `tests/golden/harness/skills/spec-dna-finish.md`. Verify: `cargo test --test finish --test lifecycle_commands --test lifecycle_log --test branch_approval --test harness_skills --test eval_pack` passes, and the new cases failed before the change. Hours: 7.
- [ ] t5: Run the eval of `write-plan` again with 15 runs for each arm, commit the records, and rebuild the table of the release (`test-engineer`). Files: `src/verdict.rs`, `docs/evals.md`. Verify: `cargo test --lib verdict` passes, so the records and the table agree. Hours: 2.
- [ ] t6: Add the fixture project of 0.7.0 and the proof that `upgrade` on it and on this repository leaves no change at the second run (`test-engineer`). Files: `tests/fixtures/upgrade/make.sh`, `tests/upgrade_fixtures.rs`, `tests/upgrade_v070.rs`. Verify: `cargo test --test upgrade_fixtures --test upgrade_v070` passes. Hours: 3.
- [ ] t7: Set the version 0.7.1 and the pin, and write the section of 0.7.1 in `CHANGELOG.md` with the notes of the upgrade from 0.7.0 (`release-engineer`). Files: `Cargo.toml`, `Cargo.lock`, `spec-dna.toml`, `CHANGELOG.md`, `README.md`, `docs/quickstart.md`, `docs/config.md`, `docs/init.md`. Verify: `cargo test` passes and `spec-dna check` exits 0 with the tree binary. Hours: 3.

## Risks

- A rerun of `write-plan` can pass the cap of 160 USD, or stop at an account limit. The runner stops before a run that could pass the cap.
- The 4 slices edit `docs/commands.md`, `docs/diagnostics.md` and `src/main.rs` in different places. The main session merges them in the order t1, t2, t3, t4, and each later slice rebases first.
- The text of `execute`, `specify` and `finish` changes without a new run of their evals. Their verdicts describe the older text, and the notes of the release say so.
- The paired benchmark against v0.7.0 can exceed 1.10. Then the tag waits.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` and `pre-commit run --all-files` pass. `spec-dna check --base origin/master` with the tree binary gives no new error. The test that each new case fails before its fix is in the commit history of each pull request. `spec-dna upgrade` on the fixture of 0.7.0 and on this repository changes nothing at its second run. The paired benchmark against v0.7.0 gives a median ratio of 1.10 or less.
