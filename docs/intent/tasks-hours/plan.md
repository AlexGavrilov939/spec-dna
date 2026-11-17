---
status: approved
---
# Hours and a "not needed" state for each task, and a long version in the line: plan

## Files

- `tests/lifecycle_commands.rs`, `tests/finish.rs`, `tests/lifecycle_log.rs` The tests of `next`, `status`, `finish`, `log` and `stats`.
- `tests/task_checks.rs`, `tests/branch_approval.rs` The tests of SD923 and SD910.
- `src/change.rs` The labels `Hours:` and `Skip:`, and the state of a task.
- `src/finish.rs`, `src/main.rs` The hours of `finish`.
- `src/parse.rs`, `src/index.rs` No section `Hours`, and `PARSER_VERSION` 15.
- `src/stage.rs` `next` and `status`.
- `src/sdlc.rs` `log` and `stats --sdlc`.
- `src/lifecycle.rs` SD923.
- `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-write-plan.md` The skill `write-plan`.
- `src/eval_pack_fixtures_rest.rs`, `tests/fixtures/eval-solutions/finish-*/*.sh` The plans and the solutions of the `finish` evals.
- `src/adapters.rs` The line of `AGENTS.md`.
- `docs/types.md`, `docs/commands.md`, `docs/diagnostics.md`, `docs/guides/change.md`, `docs/glossary.md`, `CHANGELOG.md` The docs.

## Order

- [x] t1: Write the failing tests of a skipped task in `next` and `status` (`core-engineer`). Files: `tests/lifecycle_commands.rs`. Verify: `cargo test --test lifecycle_commands` fails on the new tests only.
- [x] t2: Write the failing tests of the hours of the tasks in `finish`: a sum, a range, a skipped task, a task with no `Hours:`, a bad value and the JSON (`hygiene-engineer`). Files: `tests/finish.rs`. Verify: `cargo test --test finish` fails on the new tests only.
- [x] t3: Write the failing tests of a range in `log` and `stats --sdlc` (`core-engineer`). Files: `tests/lifecycle_log.rs`. Verify: `cargo test --test lifecycle_log` fails on the new tests only.
- [x] t4: Write the failing tests of SD923 for an empty `Skip:`, and of no SD910 for a `Skip:` edit (`diagnostics-engineer`). Files: `tests/task_checks.rs`, `tests/branch_approval.rs`. Verify: `cargo test --test task_checks --test branch_approval` fails on the new tests only.
- [ ] t5: Read the labels `Hours:` and `Skip:`, and give the state of a task (`core-engineer`). Files: `src/change.rs`. Verify: `cargo test --bin spec-dna change::` passes.
- [ ] t6: Add the hours of the tasks to `finish`, with the notes and `hours` as `{"low", "high"}` (`hygiene-engineer`). Files: `src/finish.rs`, `src/main.rs`. Verify: `cargo test --test finish` passes.
- [ ] t7: Remove `ItemSection::Hours`, and set `PARSER_VERSION` to 15 (`core-engineer`). Files: `src/parse.rs`, `src/index.rs`. Verify: `cargo test --bin spec-dna` passes.
- [ ] t8: Count a skipped task as closed in `next` and `status` (`core-engineer`). Files: `src/stage.rs`. Verify: `cargo test --test lifecycle_commands` passes.
- [ ] t9: Read a range of hours in `log` and `stats --sdlc` (`core-engineer`). Files: `src/sdlc.rs`. Verify: `cargo test --test lifecycle_log --test stats_evals` passes.
- [ ] t10: Add the problem of an empty `Skip:` to SD923 (`diagnostics-engineer`). Files: `src/lifecycle.rs`. Verify: `cargo test --test task_checks --test branch_approval` passes.
- [ ] t11: Write step 6 of `write-plan` with the labels (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-write-plan.md`. Verify: `cargo test --test harness_skills` passes.
- [ ] t12: Put the hours of the `finish` evals on the tasks (total 4.5), give `finish-3` a skipped task, and remove the fallback line of the reference solutions (`init-engineer`). Files: `src/eval_pack_fixtures_rest.rs`, `tests/fixtures/eval-solutions/finish-1/good.sh`, `tests/fixtures/eval-solutions/finish-3/good.sh`. Verify: `cargo test --test eval_pack` passes.
- [ ] t13: Write the line with no space before its marker, and test the version `99.99.99` (`init-engineer`). Files: `src/adapters.rs`. Verify: `cargo test --bin spec-dna adapters` passes.
- [ ] t14: Update the docs of the plan type and of the commands (`core-engineer`; `hygiene-engineer` for `finish`). Files: `docs/types.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t15: Update SD923, the change guide and the glossary with the terms "skipped task" and "spike" (`diagnostics-engineer`; `architect`). Files: `docs/diagnostics.md`, `docs/guides/change.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t16: Write the entry of the change (`release-engineer`). Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- `hours` of `finish --json` and `log --json` becomes an object. No release has the field.
- The line has a margin of 0 bytes at `99.99.99`. A test stops a longer text.
- `done` of `status --json` does not count a skipped task.
- The full eval run measures `write-plan`, the line and the `finish` fixtures. This change merges before the smoke, and `eval-records` rebases on it.
- `change-feedback` also edits `src/stage.rs`, `src/sdlc.rs` and their tests (the token `partial` after the tokens of the tasks). The second change to merge rebases.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t4 failed before its task. A plan with `Hours: 1-2`, `Hours: 1.5` and a skipped task gives `Hours: 2.5-3.5` from `finish`. `cargo test --test eval_pack` proves `Hours: 4.5` for `finish-1` and `finish-3`. `spec-dna check --base origin/master` gives no error.
