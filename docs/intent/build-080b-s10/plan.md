---
status: approved
---
# Build of 0.8.0 part B, slice S10: the freeze of the skill texts and the reruns: plan

## Files

- `bench/results/evals/` The records.
- `src/verdict.rs` The constant `RELEASE`.
- `tests/verdict_release.rs` The test that compares `RELEASE` with the records.
- `docs/evals.md`, `docs/design/0.8.0.md` The report.

## Order

- [ ] t1: Freeze the skill texts and record the hash of each text. Files: `docs/evals.md`. Verify: the golden tests of the skills pass. Hours: 1.
- [ ] t2: Run the reruns with the caps and commit the records. Files: `bench/results/evals/`. Verify: each batch has a verdict. Hours: 6.
- [ ] t3: Rebuild `RELEASE` from the records. Files: `src/verdict.rs`, `tests/verdict_release.rs`. Verify: `cargo test` passes. Hours: 2.

## Risks

- The 4 skills may not fit in 140 USD. The report names what was cut.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s10` runs in the pull request of the slice.
