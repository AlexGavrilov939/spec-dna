---
status: draft
---
# Fixes of 0.7.1, part A: plan

## Files

- `src/stage.rs` The stage `build` while a task is open.
- `src/finish.rs` The stop at step 0 for an open task.
- `src/main.rs` The help text of `finish` and `next`.
- `src/drift.rs` The layer folder in SD606.
- `src/harness.rs` The scratch rule of the agent files.
- `src/harness_text.rs` The text of the skills `execute`, `finish`, `tdd` and `write-plan`.
- `src/eval_pack_harder.rs` The `verify` evals.
- `src/eval_pack_rest.rs` The `brainstorm` and `execute` evals.
- `src/eval_pack_fixtures_rest.rs` The fixtures of those evals.
- `src/verdict.rs` The table of the release.
- `tests/*.rs` The tests of each item.
- `tests/fixtures/eval-solutions/**` The reference and wrong solutions of the evals.
- `tests/golden/**` The golden files of the skills and the agent files.
- `bench/results/evals/*.jsonl` The records of the pilot and of the reruns.
- `docs/commands.md` The stage table and `finish`.
- `docs/diagnostics.md` SD606.
- `docs/evals.md` The records, the verify budget and the pack.
- `docs/harness.md` The scratch rule and the skills.
- `docs/guides/change.md` The red commit and the review task.
- `CHANGELOG.md` The section `Unreleased`.

## Order

- [ ] t1: Make an open task keep the stage `build` for each plan, and stop `finish` at step 0 with the list of the open tasks (`core-engineer`). Files: `src/stage.rs`, `src/finish.rs`, `tests/lifecycle_commands.rs`. Verify: `cargo test --test lifecycle_commands --test finish` passes, and the new cases failed before the change. Hours: 2.
- [ ] t2: Find the cause of SD606 on ADR 0012 after the last change folder went, and skip a layer folder that held only change folders (`diagnostics-engineer`). Files: `src/drift.rs`, `tests/drift_layer.rs`, `docs/diagnostics.md`. Verify: `cargo test --test drift_layer` passes, and `spec-dna check --base a275019~1` gives no SD606. Hours: 1.
- [ ] t3: Fill the tokens of the 200 pilot records from the transcripts of the pilot runs (`test-engineer`). Files: `bench/results/evals/20261117T082858Z-execute.jsonl`, `tests/eval_records.rs`, `docs/evals.md`. Verify: `cargo test --test eval_records` passes. Hours: 1.
- [ ] t4: Fix the proven causes of issues 67 and 68: the faults of the `verify` evals and the stop sentences of the `finish` skill (`init-engineer`, proof by `failure-analyst`). Files: `src/harness_text.rs`, `src/eval_pack_harder.rs`, `tests/eval_pack_pilot/mod.rs`. Verify: `cargo test --test eval_pack` passes, and the new scenarios failed before the change. Hours: 3.
- [ ] t5: Make the 5 evals of `brainstorm` and of `execute` harder, each with a trap and a check proved on the stub agent (`init-engineer`). Files: `src/eval_pack_rest.rs`, `src/eval_pack_fixtures_rest.rs`, `tests/eval_pack_rest.rs`. Verify: `cargo test --test eval_pack_rest` passes. Hours: 4.
- [ ] t6: Write the 4 harness rules of the second project: the red commit, the scratchpad of a subagent, the place of browser files and the review task of the owner (`init-engineer`). Files: `src/harness.rs`, `src/harness_text.rs`, `docs/guides/change.md`. Verify: `cargo test --test harness_skills --test harness_roles` passes. Hours: 3.
- [ ] t7: Run each target that t4 to t6 touch, plus `finish`, `brainstorm` and `execute`, with 15 runs for each arm, commit the records and rebuild the table of the release (`test-engineer`). Files: `src/verdict.rs`, `docs/evals.md`, `CHANGELOG.md`. Verify: `cargo test --lib verdict` passes, so the records and the table agree. Hours: 8.
- [ ] t8: Write the report in Russian on one page, and the archive of `origin/master` (the main session). Files: none. Verify: the file `spec-dna-0.7.1-A.md` is on the Desktop. Hours: 1.
- [ ] t9: Review by the owner of the whole part. Files: none. Verify: the owner approves the pull requests of the part. Hours: 1.

## Risks

- A rerun of the evals can cost more than the cap of 400 USD, or stop at an account limit. The runner stops before a run that could pass the cap, and `--resume` goes on.
- The effect of the new text of `finish` cannot be proved on the stub agent. Only the rerun shows it.
- The text of `tdd` and `write-plan` changes in t6, so the rerun must start after its merge.
- The gate of t1 stops the plans of other projects that tick no task. The CHANGELOG tells it.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` and `pre-commit run --all-files` pass. `spec-dna check --base origin/master` with the tree binary gives no new error. The test that each new case fails before its fix is in the commit history of each pull request.
