---
status: approved
---
# Risks of part A: plan

## Files

- `src/config.rs`
- `src/repo.rs`
- `src/git.rs`
- `src/approval.rs`
- `src/lifecycle.rs`
- `src/diag.rs`
- `tests/tree_binary.rs`
- `tests/branch_approval.rs`
- `tests/task_checks.rs`
- `docs/diagnostics.md`
- `docs/types.md`
- `docs/guides/change.md`
- `bench/git-access/walk.sh`
- `bench/git-access/results.md`
- `bench/git-access/README.md`
- `docs/design/0.7.0.md`
- `docs/adr/0019-limits-of-0-7-0-part-a.md`
- `.claude/agents/ownership.md`
- `CHANGELOG.md`

## Order

- [x] t1: Write the failing tests of the CRLF compare and of a symlinked `Cargo.toml` (`core-engineer`). Files: `tests/tree_binary.rs`. Verify: `cargo test --test tree_binary` fails on the new cases.
- [x] t2: Write the failing tests of a plan and an intent set back to `draft`, with the guards (`diagnostics-engineer`). Files: `tests/branch_approval.rs`. Verify: `cargo test --test branch_approval` fails on the new cases.
- [x] t3: Write the failing test of a task path with `[` and the guard of the scope (`diagnostics-engineer`). Files: `tests/task_checks.rs`. Verify: `cargo test --test task_checks` fails on the new case.
- [ ] t4: Add the walk script and measure the walk before the change (`architect`). Files: `bench/git-access/walk.sh`, `bench/git-access/results.md`, `bench/git-access/README.md`. Verify: `bash bench/git-access/walk.sh`.
- [x] t5: Read `\r\n` as `\n` in the pin compare (`core-engineer`). Files: `src/config.rs`. Verify: `cargo test --test tree_binary`.
- [x] t6: Make a symlinked `Cargo.toml` no tree binary in both modes (`core-engineer`). Files: `src/repo.rs`, `src/git.rs`. Verify: `cargo test --test tree_binary`.
- [ ] t7: Add the head rule and reason 9 (`diagnostics-engineer`). Files: `src/approval.rs`, `src/lifecycle.rs`, `src/diag.rs`. Verify: `cargo test --test branch_approval`.
- [ ] t8: Read a glob in a task path as `crate::glob` does (`diagnostics-engineer`). Files: `src/lifecycle.rs`. Verify: `cargo test --test task_checks`.
- [ ] t9: Batch the walk, with a unit test against the paths of each commit (`core-engineer`). Files: `src/git.rs`. Verify: `cargo test --bin spec-dna git:: && cargo test --test branch_approval`.
- [ ] t10: Measure the walk after the change against the target (`architect`). Files: `bench/git-access/results.md`. Verify: `bash bench/git-access/walk.sh`.
- [ ] t11: Update the docs of SD503, SD910, SD923 and the paths (`diagnostics-engineer`; `core-engineer` for the types). Files: `docs/diagnostics.md`, `docs/types.md`, `docs/guides/change.md`. Verify: `pre-commit run --all-files`.
- [ ] t12: Write ADR 0019, which amends ADR 0012 and ADR 0013 (`architect`; the owner accepts). Files: `docs/adr/0019-limits-of-0-7-0-part-a.md`. Verify: `cargo run --quiet -- check`.
- [ ] t13: Write section 15, the owner of `tests/tree_binary.rs` and the entry of the change (`architect`, `release-engineer`). Files: `docs/design/0.7.0.md`, `.claude/agents/ownership.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`.

## Risks

- `approver-safety` also edits `src/diag.rs`, `src/git.rs`, `tests/branch_approval.rs`, `docs/diagnostics.md` and the guide. Rebase after each merge.
- The head rule gives SD910 to a branch that passed before: a branch that sets an approved artifact back and keeps its work.
- The batched walk changes the git access of each `check` with `approval = "branch"`. The guards of the merges and of the forged headers hold the old results.
- The cost of a git process differs by platform. The target is a ratio on one machine, not a time.

## Hours

- 0.5: t1 to t3 tests
- 0.5: t4 and t10 bench
- 1: t5 to t8 code
- 1.5: t9 walk
- 0.5: t11 to t13 docs

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t3 failed before its code. `bench/git-access/results.md` shows the walk of 500 commits before and after, and the ratio meets the target of the spec. `spec-dna check --base origin/master` gives no error on this repository.
