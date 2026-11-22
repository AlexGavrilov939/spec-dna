---
status: approved
---
# Build of 0.8.0 part B, slice S10: the freeze of the skill texts and the reruns: plan

## Files

- `bench/results/evals/` The records and the result files below the floor.
- `src/verdict.rs`, `src/eval_pack_fixtures_rest.rs` The constant `RELEASE` and the fixtures of the `finish` evals.
- `tests/eval_ab.rs` The pin of the released names. The test that compares `RELEASE` with the records is in `src/verdict.rs`.
- `docs/evals.md`, `docs/design/0.8.0.md` The report.

## Order

- [x] t1: Freeze the skill texts and record the hash of each text. Files: `docs/evals.md`. Verify: the golden tests of the skills pass. Hours: 1.
- [x] t2: Run the reruns with the caps and commit the records. Files: `bench/results/evals/`. Verify: each batch has records or a result file, and a batch at the floor of 15 pairs has a verdict. Hours: 6.
- [x] t3: Rebuild `RELEASE` from the records. Files: `src/verdict.rs`, `tests/eval_ab.rs`. Verify: `cargo test` passes. Hours: 2.

## Risks

- The 4 skills may not fit in 140 USD. The report names what was cut: `write-plan` and `finish` stopped below the floor of 15 pairs.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s10` runs in the pull request of the slice.
