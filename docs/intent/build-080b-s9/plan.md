---
status: approved
---
# Build of 0.8.0 part B, slice S9: the evals of the review with the record and the cost of one pass: plan

## Files

- `src/eval_pack.rs`, `src/eval_pack_*.rs`, `src/eval_ab.rs`, `src/eval_run.rs` The evals and the target `review`.
- `tests/eval_pack.rs`, `tests/eval_ab.rs`, `tests/fixtures/eval-solutions/` The reference and the wrong record of each eval.
- `bench/results/evals/` The records of the run.
- `docs/evals.md`, `docs/design/0.8.0.md` The report of the cost.

## Order

- [ ] t1: Build the 5 evals, the oracle and the reference and wrong records, with no paid run. Files: `src/eval_pack*.rs`, `tests/eval_pack.rs`, `tests/fixtures/eval-solutions/`. Verify: the reference record passes and each wrong record fails, with no model. Hours: 10.
- [ ] t2: Run the 5 evals with the cap of 100 USD, and commit the records. Files: `bench/results/evals/`. Verify: the records hold the cost, the turns and the seconds of each run. Hours: 4.
- [ ] t3: Write the cost of one pass into the design and `docs/evals.md`. Files: `docs/evals.md`, `docs/design/0.8.0.md`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- The paid run needs the text freeze of S4c, S6 and S8b.
- 4 planted defects give a wide interval.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s9` runs in the pull request of the slice.
