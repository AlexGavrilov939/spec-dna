---
status: approved
---
# Build of 0.8.0 part B, slice S4b: the core of the acceptance in one pass: plan

## Files

- `src/acceptance.rs`, `src/record.rs`, `src/accept.rs`, `src/finish.rs` The need, the record, the brief, the follow-up and the count.
- `src/config.rs`, `schema/spec-dna.schema.json`, `src/diag.rs`, `src/lifecycle.rs`, `src/check.rs`, `src/main.rs`, `src/git.rs`, `src/approval.rs`, `src/stage.rs` The key, SD941 and the command line.
- `tests/*.rs`, `tests/golden/**` The tests of D70 and D72 to D74.
- `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/guides/change.md`, `docs/glossary.md` The reference pages.

## Order

- [x] t1: Build the key `acceptance-for`, the class and the config floor of the need, and the record and the brief of format 2 with the reader of format 1 (D70, D72). Files: `src/config.rs`, `src/acceptance.rs`, `src/record.rs`, `src/accept.rs`, `src/finish.rs`. Verify: `cargo test` passes, and each new test failed before the change. Hours: 8.
- [x] t2: Build `accept --follow-up`, its brief and its chain, and the rules of SD928 and SD929 for it (D73). Files: `src/accept.rs`, `src/record.rs`, `src/acceptance.rs`. Verify: `cargo test --test accept_followup` passes, and each test failed before the change. Hours: 8.
- [x] t3: Build the count of rejected records, SD941, the line `Acceptance runs:` and part 5.2 of `finish` (D74). Files: `src/acceptance.rs`, `src/finish.rs`, `src/lifecycle.rs`, `src/diag.rs`, `src/config.rs`. Verify: `cargo test --test accept_stop` passes, and each test failed before the change. Hours: 6.
- [x] t4: Update the reference pages and the limit of D79 item 1. Files: `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/guides/change.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- The walk over the commits of the branch costs time on a long branch: it shares one batch read with the compare of the brief.
- A new JSON field or a new default of a profile needs the approval of `architect`.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s4b` runs in the pull request of the slice.
