---
status: approved
---
# Build of 0.8.0, slice S4: plan

## Files

- `src/record.rs`, `src/accept.rs`, `src/change.rs` The record, its rules, its writer, the command with the brief, and the rule that reads the scope patterns of a plan.
- `src/finish.rs`, `src/lifecycle.rs`, `src/approval.rs` Steps 5.2 and 5.5, the archive and the check of the diff.
- `src/acceptance.rs`, `src/tree.rs` The rule "a change that needs a record", the compare of the brief, the checks of the record that `finish` and `check` share, and the code tree of the index.
- `src/check.rs`, `src/diag.rs`, `src/config.rs`, `src/git.rs`, `src/main.rs`, `src/init.rs` The codes, the group `acceptance`, the keys, the lookup of the pin, the CLI and the key that `init` writes.
- `src/stage.rs`, `src/harness.rs`, `src/harness_text.rs`, `src/hook.rs` The item `accept` of `next`, the agent `acceptor`, the skill and the hook.
- `schema/spec-dna.schema.json` The keys.
- `tests/*.rs`, `tests/golden/**` The tests and the golden files.
- `.claude/agents/ownership.md` The file rows.
- `src/upgrade_config.rs`, `src/upgrade.rs`, `src/eval_ab.rs`, `src/eval_pack.rs`, `src/eval_records.rs`, `src/verdict.rs` The upgrade of the skill list, the arm `without` of `eval run --ab`, the pack and the verdict names for `accept`.
- `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/harness.md`, `docs/glossary.md`, `docs/guides/change.md`, `docs/guides/upgrade-0.6.md`, `docs/init.md`, `docs/quickstart.md`, `docs/evals.md`, `docs/concepts.md`, `docs/agents.md` The reference pages.
- `docs/design/0.8.0.md` The sentences of D3 that name the commits of the branch.
- `docs/intent/build-080-s4/acceptance.toml` The record of this slice.

## Order

- [x] t1: Build the record, its reader and writer, `spec-dna accept` with `--brief`, and `finish` 5.2 and 5.5 with SD927 to SD931, SD933 and SD935 (`core-engineer`, `hygiene-engineer`, `diagnostics-engineer`). Files: `src/record.rs`, `src/accept.rs`, `src/finish.rs`. Verify: `cargo test --test accept_cmd --test finish` passes, and each new test failed before the change. Hours: 12.
- [x] t2: Build the skill `accept`, the agent `acceptor`, the item of `next`, the hook rule and the `APPROVAL_RULE` sentence (`init-engineer`). Files: `src/harness_text.rs`, `src/harness.rs`, `src/hook.rs`. Verify: `cargo test --test harness_skills --test harness_roles` passes with the golden files. Hours: 4.
- [ ] t3: Accept this slice with its own mechanism: run `spec-dna accept build-080-s4` from the brief in a subagent, and commit the record (`acceptor`). Files: `docs/intent/build-080-s4/acceptance.toml`. Verify: the tree binary gives no SD927 to SD931 for the record. Hours: 2.

## Risks

- The brief must be a function of the tree of one commit. The design lists the cases that it cannot prove.
- The `toml` crate of the build has no writer, so the writer is code of the slice.
- S1 did not build the removal of the agent `acceptor` under `eval run --ab --harness`, because the skill and the role come in this slice; build it here (design D57, D62).

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the slice is valid. `spec-dna finish build-080-s4` runs in the pull request of the slice.
