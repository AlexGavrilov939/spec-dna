---
status: approved
---
# Feedback of a second project: plan

## Files

- `src/lifecycle.rs`
- `src/evidence.rs`
- `src/upgrade.rs`
- `src/finish.rs`
- `src/check.rs`
- `src/diag.rs`
- `src/stage.rs`
- `src/context.rs`
- `src/sdlc.rs`
- `src/main.rs`
- `src/config.rs`
- `schema/spec-dna.schema.json`
- `tests/lifecycle_base.rs`
- `tests/branch_approval.rs`
- `tests/finish.rs`
- `tests/cli.rs`
- `tests/lifecycle_finished.rs`
- `tests/lifecycle_commands.rs`
- `tests/stats_evals.rs`
- `tests/lifecycle_log.rs`
- `tests/lifecycle_checks.rs`
- `tests/golden/show-requirement.json`
- `tests/golden/stats-sdlc.txt`
- `tests/golden/stats-sdlc-evals.txt`
- `docs/diagnostics.md`
- `docs/config.md`
- `docs/types.md`
- `docs/commands.md`
- `docs/guides/change.md`
- `docs/glossary.md`
- `CHANGELOG.md`

## Order

- [ ] t1: Write the failing tests of SD913 before a plan, and replace `a_change_with_no_plan_has_the_folder_as_its_scope` (`diagnostics-engineer`). Files: `tests/lifecycle_base.rs`. Verify: `cargo test --test lifecycle_base` fails on the new tests only.
- [ ] t2: Run SD913 only when a change of the diff has a `plan.md` (`diagnostics-engineer`). Files: `src/lifecycle.rs`. Verify: `cargo test --test lifecycle_base --test finish_sd913` passes.
- [ ] t3: Add the guards of a delta edit after a spike: no new approval in the first pull request, a plan approved again, a stop of `finish` (`diagnostics-engineer`, `hygiene-engineer`). Files: `tests/branch_approval.rs`, `tests/finish.rs`. Verify: `cargo test --test branch_approval --test finish` passes.
- [ ] t4: Write the failing tests of the partial tag: SD608, SD105, `show`, SD916, `next` and `status` (`diagnostics-engineer`, `context-engineer`, `core-engineer`). Files: `tests/cli.rs`, `tests/lifecycle_finished.rs`, `tests/lifecycle_commands.rs`. Verify: `cargo test --test cli --test lifecycle_finished --test lifecycle_commands partial` fails on the new tests only.
- [ ] t5: Write the failing test of the row of `stats --sdlc`, and add the row to the goldens (`core-engineer`). Files: `tests/stats_evals.rs`, `tests/golden/stats-sdlc.txt`, `tests/golden/stats-sdlc-evals.txt`. Verify: `cargo test --test stats_evals --test lifecycle_log` fails on the new test and the goldens only.
- [ ] t6: Read `covers: partial`, and add the flag `partial` to `Cover` and to its 2 other literals (`diagnostics-engineer`; C `init-engineer`, `hygiene-engineer`). Files: `src/evidence.rs`, `src/upgrade.rs`, `src/finish.rs`. Verify: `cargo test --bin spec-dna evidence::` passes.
- [ ] t7: Keep the partial tags out of the evidence, and name them in SD608, SD105 and the fix texts (`diagnostics-engineer`). Files: `src/evidence.rs`, `src/check.rs`, `src/diag.rs`. Verify: `cargo test --test cli partial_tag` passes.
- [ ] t8: Name the partial tag in SD916 (`diagnostics-engineer`). Files: `src/lifecycle.rs`. Verify: `cargo test --test lifecycle_finished` passes.
- [ ] t9: Add the field `partial` to `show` (`context-engineer`). Files: `src/context.rs`, `tests/golden/show-requirement.json`. Verify: `cargo test --test cli show_` passes.
- [ ] t10: Count a partial tag as missing evidence at the stage `test`, and add the token and the field `partial` to `status` (`core-engineer`). Files: `src/stage.rs`. Verify: `cargo test --test lifecycle_commands` passes.
- [ ] t11: Add the row of `stats --sdlc` and its help text, and count 16 rows (`core-engineer`). Files: `src/sdlc.rs`, `src/main.rs`, `tests/lifecycle_log.rs`. Verify: `cargo test --test stats_evals --test lifecycle_log` passes.
- [ ] t12: Write the failing test of a plan of 40 tasks, and the guards of `[budgets.plan]` (`diagnostics-engineer`). Files: `tests/lifecycle_checks.rs`. Verify: `cargo test --test lifecycle_checks budget` fails on the new test only.
- [ ] t13: Remove the default budget of `plan`, and state that `soft = 0, hard = 0` gives a type no budget (`core-engineer`). Files: `src/config.rs`, `schema/spec-dna.schema.json`. Verify: `cargo test --test lifecycle_checks` passes.
- [ ] t14: Update the docs of SD913, SD608, SD916, the evidence and the budgets (`diagnostics-engineer`, `core-engineer`). Files: `docs/diagnostics.md`, `docs/config.md`, `docs/types.md`. Verify: `pre-commit run --files docs/diagnostics.md docs/config.md docs/types.md` passes.
- [ ] t15: Update `show`, `next`, `status` and `stats`, the guide (a delta that depends on a spike) and the glossary (`docs-editor`). Files: `docs/commands.md`, `docs/guides/change.md`, `docs/glossary.md`. Verify: `pre-commit run --files docs/commands.md docs/guides/change.md docs/glossary.md` passes.
- [ ] t16: Write the entry, with the migration note of `[budgets.plan]`. Files: `CHANGELOG.md`. Verify: `pre-commit run --files CHANGELOG.md` passes.

## Hours

- 1: t1, t2
- 0.5: t3
- 4.5: t4 to t11
- 0.5: t12, t13
- 1.5: t14 to t16

## Risks

- Other changes of part B edit `src/lifecycle.rs`, `src/config.rs`, `tests/cli.rs`, the guide and `CHANGELOG.md`. Rebase after each merge.
- `stats --json` gets 16 rows. A script must read a row by `name`.
- The row of `stats` parses the tree of `HEAD` once more, and runs one more `git grep`.
- A project with its own `[budgets.plan]` keeps it.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each new test of t1, t4, t5 and t12 failed before its code. `spec-dna check --base origin/master` gives no error on this repository.
