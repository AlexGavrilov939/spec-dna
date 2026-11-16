---
status: approved
---
# 0.6.1 upgrade feedback

## Files

- `docs/design/0.6.1.md` the design: one decision for each item
- `docs/intent/0-6-1-upgrade-feedback/**` the intent and this plan, which `finish` deletes
- `src/*.rs` the fixes of the 16 items
- `tests/**` the tests of the fixes, the golden files and the fixtures
- `schema/spec-dna.schema.json` the new key `[change] base`
- `docs/*.md` the pages of the commands, the configuration, the types, the diagnostics, init and the harness
- `docs/guides/*.md` the guides of the upgrade and of the change
- `.claude/agents/ownership.md` the change sets, if a path of a change set moves
- `.github/workflows/bench.yml` the benchmark workflow (item 16)
- `bench/tool/README.md` the paragraph on the workflow
- `CHANGELOG.md` the section of 0.6.1
- `Cargo.toml` the version
- `Cargo.lock` the version

## Order

- [ ] t1: Add the benchmark workflow and merge it first, because a workflow runs only from the default branch. Files: `.github/workflows/bench.yml`, `bench/tool/README.md`. Verify: `pre-commit run --all-files` passes, and one run on the final branch gives the numbers.
- [ ] t2: Fix `mv`: the path of its own alias (item 1), the `type` and `status` lines of the new place (item 2), and the empty folders (item 11). Files: `src/mv.rs`, `src/main.rs`, `tests/cli.rs`, `docs/commands.md`, `docs/types.md`. Verify: each new test fails before the fix and passes after it.
- [ ] t3: Make `log` and `stats` keep a move out of the intent layer as not closed (item 3). Files: `src/git.rs`, `src/sdlc.rs`, `tests/lifecycle_log.rs`, `tests/git_lifecycle.rs`. Verify: the new tests fail before the fix.
- [ ] t4: Fix `layer add --types` (item 4) and the links to the `pending` layer (item 12). Files: `src/main.rs`, `src/layer.rs`, `tests/layer_cmd.rs`, `tests/layers_checks.rs`, `tests/golden/`. Verify: the new tests fail before the fix.
- [ ] t5: Let a routine change with no files reach `finish` (item 9). Files: `src/change.rs`, `src/stage.rs`, `docs/types.md`, `docs/guides/change.md`, `tests/lifecycle_commands.rs`. Verify: the new test fails before the fix.
- [ ] t6: Fix `upgrade` for the types of the policy, the empty folders and the notes (items 5 and 11). Files: `src/upgrade.rs`, `src/upgrade_config.rs`, `tests/upgrade_cli.rs`. Verify: the new tests fail before the fix.
- [ ] t7: Fix `init --reconfigure` for `high-risk-layers` and the warning on `types` (items 5 and 6). Files: `src/init.rs`, `src/config.rs`, `tests/init_change.rs`. Verify: the new tests fail before the fix.
- [ ] t8: Make the generated files pass markdownlint and test each one (item 7). Files: `src/harness.rs`, `src/adapters.rs`, `tests/common/`, `tests/generated_lint.rs`, `tests/golden/harness/`. Verify: the new test fails before the fix, and pre-commit lints the golden files.
- [ ] t9: Document the files that SD503 counts (item 8), and name a dead link in locked text (item 10). Files: `docs/diagnostics.md`, `src/check.rs`, `src/main.rs`, `tests/lifecycle_finished.rs`, `tests/golden/check-all.json`. Verify: the new tests fail before the fix.
- [ ] t10: Add the key `[change] base` (item 13). Files: `src/config.rs`, `src/git.rs`, `src/lifecycle.rs`, `src/harness_text.rs`, `schema/spec-dna.schema.json`, `docs/config.md`, `docs/guides/upgrade-0.5.md`, `tests/lifecycle_base.rs`, `tests/change_config.rs`. Verify: the new tests fail before the fix.
- [ ] t11: Map the common file names to top files and add `--areas` (items 14 and 15). Files: `src/upgrade_docs.rs`, `src/upgrade_proof.rs`, `src/upgrade_config.rs`, `src/upgrade.rs`, `src/main.rs`, `tests/upgrade_cli.rs`, `tests/fixtures/upgrade/`. Verify: the new tests fail before the fix, and the proof of `upgrade` stays ok.
- [ ] t12: Write the docs, the CHANGELOG and the version, run the benchmark, and finish the change. Files: `docs/*.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`. Verify: `pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass.

## Risks

- Items 14 and 15 touch the proof of `upgrade`, which is the safety net. A bug there can move a document with no alias. The slice comes last, and `checker` checks it alone. If the check finds a risk that the tests do not cover, the slice is cut back to the guide text.
- Item 3 can hide a change that `finish` closes if the commit adds a near copy of the intent with its id as an alias. The design (D10) accepts this limit, and a test covers the near copy.
- The version moves to 0.6.1 while the pin of this repository stays 0.6.0 until the pin pull request. `check` and `finish` stop in that state, so `finish` runs before the version commit.
- The generated files change, so each consumer gets new files at the next `init`.

## Proof

- Each fix has a test that fails before the fix. The red run of each test is in the report of its step.
- `pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass.
- The binary of `cargo install --root <a temporary folder>` runs the steps of the upgrade guide on a repository with no remote.
- One run of the workflow `Benchmark` on the final branch gives the numbers for the CHANGELOG.
- `checker` reviews each slice and then the whole diff.
