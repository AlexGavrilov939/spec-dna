---
status: draft
---
# Build of 0.8.0 part B, slice S8b: the roll-up with finish, next, context and the area budget: plan

## Files

- `src/finish.rs`, `src/stage.rs`, `src/lifecycle.rs`, `src/split.rs`, `src/context.rs`, `src/check.rs`, `src/config.rs`, `src/init.rs`, `src/harness_text.rs` The flow and the tier.
- `tests/finish.rs`, `tests/lifecycle_commands.rs`, `tests/lifecycle_checks.rs`, `tests/layers_checks.rs`, `tests/context_layers.rs`, `tests/upgrade_cli.rs` The tests.
- `docs/commands.md`, `docs/guides/change.md`, `docs/config.md`, `docs/profiles.md`, `docs/diagnostics.md` The reference pages.

## Order

- [ ] t1: Build `finish::rollup()` and `finish --holds`, the note of `next`, the SD913 exception and `mv` (D54). Files: `src/finish.rs`, `src/stage.rs`, `src/lifecycle.rs`, `src/split.rs`, `src/main.rs`. Verify: `cargo test --test finish --test lifecycle_commands --test lifecycle_checks` passes. Hours: 10.
- [ ] t2: Build the area budget SD705 to SD707 (D55). Files: `src/check.rs`, `src/config.rs`, `src/init.rs`. Verify: `cargo test --test layers_checks --test init_profiles` passes. Hours: 5.
- [ ] t3: Build the tier `summary` of `context` (D56). Files: `src/context.rs`. Verify: `cargo test --test context_layers` passes, and the golden files of a corpus with no summary do not change. Hours: 5.
- [ ] t4: Add the summaries of this repository and the equality test of `upgrade` with `[budgets.area]`. Files: `docs/`, `tests/upgrade_cli.rs`, `spec-dna.toml`. Verify: `spec-dna check` gives no SD641 or SD642. Hours: 6.
- [ ] t5: Update the reference pages and the text of the skills. Files: `docs/commands.md`, `docs/guides/change.md`, `docs/config.md`, `docs/profiles.md`, `docs/diagnostics.md`, `src/harness_text.rs`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- The cascade of `--holds` must not hide a stale summary.
- The summaries of this repository must stay true through the rest of part B.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s8b` runs in the pull request of the slice.
