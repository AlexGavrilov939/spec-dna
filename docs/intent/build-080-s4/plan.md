---
status: draft
---
# Build of 0.8.0, slice S4: plan

## Files

- `src/record.rs`, `src/accept.rs` The record, its rules, its writer, and the command with the brief.
- `src/finish.rs`, `src/lifecycle.rs`, `src/approval.rs` Steps 5.2 and 5.5, the archive and the check of the diff.
- `src/check.rs`, `src/diag.rs`, `src/config.rs`, `src/git.rs`, `src/main.rs` The codes, the group `acceptance`, the keys, the lookup of the pin and the CLI.
- `src/stage.rs`, `src/harness.rs`, `src/harness_text.rs`, `src/hook.rs` The item `accept` of `next`, the agent `acceptor`, the skill and the hook.
- `schema/spec-dna.schema.json` The keys.
- `tests/*.rs`, `tests/golden/**` The tests and the golden files.
- `.claude/agents/ownership.md` The file rows.
- `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/harness.md`, `docs/glossary.md`, `docs/guides/change.md` The reference pages.
- `docs/intent/build-080-s4/acceptance.toml` The record of this slice.

## Order

- [ ] t1: Build the record, its reader and writer, `spec-dna accept` with `--brief`, and `finish` 5.2 and 5.5 with SD927 to SD931, SD933 and SD935 (`core-engineer`, `hygiene-engineer`, `diagnostics-engineer`). Files: `src/record.rs`, `src/accept.rs`, `src/finish.rs`. Verify: `cargo test --test accept_cmd --test finish` passes, and each new test failed before the change. Hours: 12.
- [ ] t2: Build the skill `accept`, the agent `acceptor`, the item of `next`, the hook rule and the `APPROVAL_RULE` sentence (`init-engineer`). Files: `src/harness_text.rs`, `src/harness.rs`, `src/hook.rs`. Verify: `cargo test --test harness_skills --test harness_roles` passes with the golden files. Hours: 4.
- [ ] t3: Accept this slice with its own mechanism: run `spec-dna accept build-080-s4` from the brief in a subagent, and commit the record (`acceptor`). Files: `docs/intent/build-080-s4/acceptance.toml`. Verify: the tree binary gives no SD927 to SD931 for the record. Hours: 2.

## Risks

- The brief must be a function of the tree of one commit. The design lists the cases that it cannot prove.
- The `toml` crate of the build has no writer, so the writer is code of the slice.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the slice is valid. `spec-dna finish build-080-s4` runs in the pull request of the slice.
