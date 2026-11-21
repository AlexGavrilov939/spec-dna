---
status: approved
---
# Build of 0.8.0 part B, slice S5n: plan

## Files

- `src/record.rs`, `src/acceptance.rs`, `src/finish.rs` Unique ids and part 5.3 of `finish`.
- `src/change.rs`, `src/lifecycle.rs` The code spans and the sections `Invariants` and `Deviations`, and the doc comment of `skip_early_sections`.
- `tests/*.rs` The tests of the cases of D78.
- `docs/commands.md`, `docs/diagnostics.md`, `docs/guides/change.md` The reference pages that name these rules.

## Order

- [ ] t1: Fix the 5 notes of D78 with a test for each that fails before the fix. Files: `src/record.rs`, `src/acceptance.rs`, `src/finish.rs`, `src/change.rs`, `src/lifecycle.rs`. Verify: `cargo test` passes, and each new test failed before the change. Hours: 3.
- [ ] t2: Update the reference pages. Files: `docs/commands.md`, `docs/diagnostics.md`, `docs/guides/change.md`. Verify: `pre-commit run --all-files` passes. Hours: 1.

## Risks

- A change of the code span rule can change the text of a message that a golden file holds.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s5n` runs in the pull request of the slice.
