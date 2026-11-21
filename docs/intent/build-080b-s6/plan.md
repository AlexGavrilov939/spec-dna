---
status: approved
---
# Build of 0.8.0 part B, slice S6: the agent roles of [owners]: plan

## Files

- `src/owners.rs` (new), `src/config.rs`, `schema/spec-dna.schema.json` The table and its warning.
- `src/acceptance.rs`, `src/record.rs`, `src/accept.rs`, `src/approval.rs`, `src/approve.rs`, `src/lifecycle.rs`, `src/diag.rs` The roles of the record and of the brief, SD932, SD933 and SD934.
- `tests/owners.rs` (new), `tests/branch_approval.rs`, `tests/accept_cmd.rs` The tests.
- `docs/config.md`, `docs/guides/change.md`, `docs/diagnostics.md`, `docs/commands.md`, `.claude/agents/ownership.md` The reference pages and the rows of the new files.
- `src/main.rs`, `src/finish.rs`, `src/stage.rs`, `src/repo.rs`, `src/init.rs`, `src/upgrade.rs`, `tests/cli.rs` The help of `approve`, the roles and the acknowledgement in `finish` and `next`, the warning for a person, the kept table `[owners]`, and the help test.
- `docs/profiles.md`, `docs/glossary.md` The rows of SD932 and SD934, and the terms "acknowledgement" and "owner role".

## Order

- [x] t1: Build `src/owners.rs`, the table in the configuration with the warning for a person, and the match of D70 item 7. Files: `src/owners.rs`, `src/config.rs`, `schema/spec-dna.schema.json`. Verify: `cargo test --test owners` passes, and each test failed before the change. Hours: 6.
- [x] t2: Build the roles in the record, the brief and SD933, with one entry for each pass (D72 item 4, D77). Files: `src/acceptance.rs`, `src/record.rs`, `src/accept.rs`. Verify: `cargo test --test owners --test accept_cmd` passes. Hours: 5.
- [x] t3: Build the acknowledgement with `branch`: `approve <slug> acceptance`, SD932 and SD934, and the follow-up case of D73 item 7. Files: `src/approval.rs`, `src/approve.rs`, `src/lifecycle.rs`, `src/diag.rs`. Verify: `cargo test --test branch_approval` passes. Hours: 6.
- [x] t4: Update the reference pages and the rows of `ownership.md`. Files: `docs/config.md`, `docs/guides/change.md`, `docs/diagnostics.md`, `docs/commands.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- The walk of the approval commits is the most complex code of the record. A new rule can make an honest history fail.
- The rows of `ownership.md` need the acceptance of the owner.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s6` runs in the pull request of the slice.
