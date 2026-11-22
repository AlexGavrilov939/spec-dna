---
status: approved
---
# Build of 0.8.0 part B, slice S8b: the roll-up with finish, next, context and the area budget: plan

## Files

- `src/finish.rs`, `src/stage.rs`, `src/lifecycle.rs`, `src/split.rs`, `src/mv.rs`, `src/rollup.rs`, `src/main.rs`, `src/context.rs`, `src/check.rs`, `src/config.rs`, `src/diag.rs`, `src/init.rs`, `src/harness_text.rs` The flow and the tier.
- `tests/finish.rs`, `tests/lifecycle_commands.rs`, `tests/lifecycle_checks.rs`, `tests/layers_checks.rs`, `tests/context_layers.rs`, `tests/upgrade_cli.rs`, `tests/init_profiles.rs`, `tests/cli.rs`, `tests/harness_skills.rs`, `tests/golden/harness/skills/**` The tests.
- `docs/commands.md`, `docs/guides/change.md`, `docs/config.md`, `docs/profiles.md`, `docs/diagnostics.md`, `docs/init.md`, `docs/concepts.md`, `docs/types.md`, `docs/design/0.8.0.md`, `schema/spec-dna.schema.json` The reference pages, the design and the schema.

## Order

- [x] t1: Build `finish::rollup()` and `finish --holds`, the note of `next`, the SD913 exception and `mv` (D54). Files: `src/finish.rs`, `src/stage.rs`, `src/lifecycle.rs`, `src/split.rs`, `src/main.rs`. Verify: `cargo test --test finish --test lifecycle_commands --test lifecycle_checks` passes. Hours: 10.
- [x] t2: Build the area budget SD705 to SD707 (D55). Files: `src/check.rs`, `src/config.rs`, `src/init.rs`. Verify: `cargo test --test layers_checks --test init_profiles` passes. Hours: 5.
- [x] t3: Build the tier `summary` of `context` (D56). Files: `src/context.rs`. Verify: `cargo test --test context_layers` passes, and the golden files of a corpus with no summary do not change. Hours: 5.
- [x] t4: Add the equality test of `upgrade` with `[budgets.area]`. This repository gets no summary: D50 allows only requirements and summaries as the source of a pin, and the repository has decisions and research pages (decision of `architect`, D50 to D56). The fixtures of the tests prove the roll-up. Files: `tests/upgrade_cli.rs`. Verify: `cargo test --test upgrade_cli` passes. Hours: 6.
- [x] t5: Update the reference pages and the text of the skills. Files: `docs/commands.md`, `docs/guides/change.md`, `docs/config.md`, `docs/profiles.md`, `docs/diagnostics.md`, `src/harness_text.rs`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- The cascade of `--holds` must not hide a stale summary.
- This repository has no summary, so the proof of the roll-up is in the fixtures of the tests: each rule has a test that failed before the change.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s8b` runs in the pull request of the slice.
