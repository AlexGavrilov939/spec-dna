---
status: approved
---
# Tree binary

## Files

- `src/git.rs` The diff of one file for the SD503 pin rule.
- `src/check.rs` The tree binary test and the SD503 rule.
- `src/config.rs` The tree binary test, next to the pin check.
- `docs/diagnostics.md` The section of SD503.
- `docs/config.md` The text about the pin and the tree binary.
- `tests/tree_binary.rs` The tests of the tree binary and SD503.
- `.github/workflows/ci.yml` The job `Self-check`.
- `CONTRIBUTING.md` The section "Releases": the pin moves with the version.
- `.claude/agents/ownership.md` The rows "Release" and `spec-dna.toml`.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Write the tests that fail before the code. Files: `tests/tree_binary.rs`. Verify: `cargo test --test tree_binary` fails on each case before the fix.
- [ ] t2: Add the tree binary test and the SD503 pin rule. Files: `src/config.rs`, `src/check.rs`, `src/git.rs`. Verify: `cargo test --test tree_binary` passes.
- [ ] t3: Add the job `Self-check`. Files: `.github/workflows/ci.yml`. Verify: `pre-commit run --all-files` passes, and the job passes in the pull request.
- [ ] t4: Update the docs, the conventions and the entry of the change log. Files: `docs/diagnostics.md`, `docs/config.md`, `CONTRIBUTING.md`, `.claude/agents/ownership.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- A local run stops with exit code 3 between a commit of the version and a commit of the pin. A release makes both in one commit.
- The job runs the binary of the tree, so a bug in it can make the job pass on a wrong rule. The tests of t1 cover the rule.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. `spec-dna check --base origin/master` with the tree binary gives no error on this repository. The job `Self-check` passes in the pull request.
