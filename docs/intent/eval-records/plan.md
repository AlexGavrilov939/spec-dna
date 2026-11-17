---
status: approved
---
# Eval records, the release table and harder evals for the full run: plan

## Files

- `tests/eval_ab.rs`, `tests/eval_records.rs`, `tests/fixtures/eval-records/*.json` The tests of the runner and of `eval records`.
- `tests/eval_pack.rs`, `tests/evals_checks.rs`, `tests/eval_pack_pilot/mod.rs` The tests of the pack.
- `tests/fixtures/eval-solutions/verify-*/**`, `tests/fixtures/eval-solutions/agents-line-*/**` The solutions of the new evals.
- `src/eval_run.rs`, `src/eval_ab.rs` The runner.
- `src/evals.rs` The key `skill`.
- `src/eval_records.rs`, `src/main.rs`, `src/verdict.rs` `eval records`, the release table and `eval verdict --release`.
- `src/eval_pack.rs`, `src/eval_pack_pilot.rs`, `src/eval_pack_harder.rs` The pack.
- `bench/results/evals/*.jsonl` The records of the pilot.
- `docs/evals.md`, `docs/commands.md`, `docs/diagnostics.md`, `docs/glossary.md`, `.claude/agents/ownership.md`, `CHANGELOG.md` The docs and the owners.

## Order

- [x] t1: Write the failing tests of the runner: `tokens`, `models` and `checks` of a side, `--resume` with more runs, and the start below the cap (`core-engineer`). Files: `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab` fails on the new tests only.
- [x] t2: Write the failing tests of `eval records`: the 9 keys, a poisoned file, a bad model name, no verdict, a failed self-check and the usage errors (`core-engineer`). Files: `tests/eval_records.rs`, `tests/fixtures/eval-records/poisoned.json`, `tests/fixtures/eval-records/small.json`. Verify: `cargo test --test eval_records` fails on each test.
- [x] t3: Write the failing tests of the 57 evals, the set of `agents-line` and the key `skill = "agents-line"` (`init-engineer`; `core-engineer`). Files: `tests/eval_pack.rs`, `tests/evals_checks.rs`. Verify: `cargo test --test eval_pack --test evals_checks` fails on the new tests only.
- [x] t4: Read the tokens and the models of each run from `modelUsage` (`core-engineer`). Files: `src/eval_run.rs`. Verify: `cargo test --bin spec-dna eval_run` passes.
- [x] t5: Write `tokens`, `models` and `checks` in each side, and read them with `--resume` (`core-engineer`). Files: `src/eval_ab.rs`. Verify: `cargo test --test eval_ab` passes on the cases of the sides.
- [x] t6: Let `--resume` take a higher `--runs`, and let `--ab` start below its estimate (`core-engineer`). Files: `src/eval_ab.rs`. Verify: `cargo test --test eval_ab` passes.
- [x] t7: Accept `agents-line` in the key `skill` (`core-engineer`). Files: `src/evals.rs`. Verify: `cargo test --test evals_checks` passes.
- [ ] t8: Add `eval records` with the allowlist and the self-check (`core-engineer`). Files: `src/eval_records.rs`, `src/main.rs`. Verify: `cargo test --test eval_records` passes.
- [ ] t9: Write `verify-6` to `verify-10`, and remove `verify-1` to `verify-5` (`init-engineer`). Files: `src/eval_pack_harder.rs`, `src/eval_pack_pilot.rs`, `src/eval_pack.rs`. Verify: `cargo test --test eval_pack` passes on the cases of `verify`.
- [ ] t10: Write `agents-line-1` and `agents-line-2`, and set the set of `agents-line` (`init-engineer`; `core-engineer`). Files: `src/eval_pack_harder.rs`, `src/eval_ab.rs`. Verify: `cargo test --test eval_pack --test eval_ab` passes.
- [ ] t11: Write the solutions of the 7 new evals, and remove those of `verify-1` to `verify-5` (`init-engineer`). Files: `tests/eval_pack_pilot/mod.rs`. Verify: `cargo test --test eval_pack` passes.
- [ ] t12: Import the records of the pilot of `execute`, `tdd` and `verify` (the main session). Files: `bench/results/evals/20261117T082858Z-execute.jsonl`, `bench/results/evals/20261117T082902Z-tdd.jsonl`, `bench/results/evals/20261117T082906Z-verify.jsonl`. Verify: `cat bench/results/evals/*.jsonl | wc -l` prints 120.
- [ ] t13: Import the records of the pilot of `debug` and `agents-line` (the main session). Files: `bench/results/evals/20261117T082910Z-debug.jsonl`, `bench/results/evals/20261117T082914Z-agents-line.jsonl`. Verify: `cat bench/results/evals/*.jsonl | wc -l` prints 200.
- [ ] t14: Write the release table and its 3 tests: the agreement with the records, the forms of the records and the numbers of the pilot (`core-engineer`). Files: `src/verdict.rs`. Verify: `cargo test --bin spec-dna verdict` passes.
- [ ] t15: Add `eval verdict --release` and its JSON (`core-engineer`). Files: `src/main.rs`, `src/verdict.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab` passes.
- [ ] t16: Update the docs of the evals and of the commands (`core-engineer`; `init-engineer`). Files: `docs/evals.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t17: Update SD506, the glossary ("record", "batch") and the owners of the new paths (`diagnostics-engineer`; `architect`). Files: `docs/diagnostics.md`, `docs/glossary.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t18: Write the entry of the change (`release-engineer`). Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t19: Raise `max_turns` from 25 to 40 for each eval of `execute` and for `tdd-1`, `debug-1` and `execute-1` of `agents-line`, and give the fixture of `execute-1` a requirement key for its `covers:` tag. The proof is in the lab file of session 26: 4 of 4 runs with the line stopped at the cap with correct work. Write first a failing test that no eval of `execute` has a cap under 40 (`init-engineer`). Files: `tests/eval_pack.rs`, `src/eval_pack_pilot.rs`, `src/eval_pack_harder.rs`. Verify: `cargo test --test eval_pack` passes.

## Risks

- The runner changes before an expensive run. Each change has a test with a fake `claude`, and a smoke with `--runs 1` runs first.
- `--ab all` with the default cap now gives a partial result in place of a refusal. The note names `--resume`.
- The budget fits the run with a margin of about 2%. `agents-line` runs last, because it sets no default.
- The cause of `execute-1` with the line is not known. `failure-analyst` reads its checks before the run.
- `tasks-hours` merges first. Both changes edit the pack.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t3 failed before its task. `spec-dna eval records` on each pilot result file gives its committed file byte for byte. `spec-dna eval verdict --release` prints `tdd` as `on` (rule 1) and the 4 other names as `inconclusive`. `spec-dna eval pack` writes 57 evals. `spec-dna check --base origin/master` gives no error.
