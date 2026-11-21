---
status: approved
---
# Build of 0.8.0 part B, slice S4c: one pass for the agent: plan

## Files

- `src/harness.rs`, `src/harness_text.rs`, `src/hook.rs`, `src/init.rs`, `src/upgrade_config.rs`, `src/eval_ab.rs`, `src/eval_pack.rs` The skill, the role, the hook and the profile text.
- `src/stage.rs`, `src/change.rs`, `src/finish.rs`, `src/main.rs` The items of `next` and the open task.
- `tests/golden/harness/**` The golden files, with the new folder `skills-0.7.1`.
- `tests/*.rs` The tests of D71, D75 and D76.
- `docs/harness.md`, `docs/commands.md`, `docs/init.md`, `docs/profiles.md`, `docs/evals.md` The reference pages.

## Order

- [ ] t1: Freeze the skill texts of 0.7.1 for `lite`, give `body` the profile, and add the golden test against the files of `v0.7.1` (D76). Files: `src/harness_text.rs`, `src/harness.rs`, `src/main.rs`, `src/eval_ab.rs`, `tests/golden/harness/skills-0.7.1`. Verify: each of the 11 skills rendered with `lite` equals its file of 0.7.1. Hours: 8.
- [ ] t2: Merge `accept` into `review`, write the agent `acceptor` with `review`, and give `next` the one item (D71). Files: `src/harness.rs`, `src/harness_text.rs`, `src/stage.rs`, `src/init.rs`, `src/upgrade_config.rs`. Verify: `cargo test --test harness_skills --test harness_roles --test accept_next` passes. Hours: 8.
- [ ] t3: Build the prompt line, the item of a person after 2 rejected records, the open task of D75 item 6 and rule D of the hook. Files: `src/stage.rs`, `src/change.rs`, `src/finish.rs`, `src/hook.rs`. Verify: `cargo test --test finish --test hook_approvals --test accept_next` passes. Hours: 6.
- [ ] t4: Update the reference pages. Files: `docs/harness.md`, `docs/commands.md`, `docs/init.md`, `docs/profiles.md`, `docs/evals.md`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- A golden file of a skill changes for every profile that is not `lite`: the test must name each change.
- Rule D reads the text of the command, so it cannot enforce a write outside the worktree.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s4c` runs in the pull request of the slice.
