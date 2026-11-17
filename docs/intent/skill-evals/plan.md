---
status: approved
---
# Evals of each skill against no skill: plan

## Files

- `tests/eval_ab.rs` The tests of `eval run --ab` and `eval verdict` with a fake `claude`.
- `tests/eval_pack.rs` The tests of `eval pack` and of the 55 evals.
- `tests/eval_run.rs` A guard test: a refused run without `--ab` counts as a failed run, as in ADR 0008.
- `tests/evals_checks.rs` The tests of SD507 and of the key `skill` in SD506.
- `tests/stats_evals.rs` A guard test: `stats --sdlc` does not count a result of `--ab`.
- `tests/cli.rs` The help, the usage errors and the exit codes of `eval pack`, `eval verdict` and `--ab`.
- `src/verdict.rs` SplitMix64, the paired bootstrap, the rules of D33, the simulation, the table of the release and the reader of the results.
- `src/eval_ab.rs` The arms, the pairs and the result file of `--ab`.
- `src/eval_run.rs` The excluded runs of `--ab`, the back-off, the argument `--disallowedTools` and the entry of `--ab`.
- `src/evals.rs` The key `skill` of an eval file.
- `src/eval_pack.rs` The 55 evals, their base fixtures and the command `eval pack`.
- `src/adapters.rs` The adapter skill and the line of `AGENTS.md` without one discipline, and with no line.
- `src/main.rs` The modules, the commands `eval pack` and `eval verdict`, and the option `--ab`.
- `src/diag.rs` The code SD507.
- `src/check.rs` The check of SD507.
- `src/config.rs` The severity of SD507 in each profile.
- `docs/evals.md` The pack, the arms, the result file, the excluded runs, the back-off, the verdict and the result of the simulation.
- `docs/commands.md` The section `eval` and its exit codes.
- `docs/diagnostics.md` The row of SD507.
- `docs/profiles.md` The severity of SD507.
- `docs/glossary.md` The terms arm, verdict and cost per correct outcome.
- `CHANGELOG.md` The entry of the change.
- `.claude/agents/ownership.md` The owners of the new modules and test files.
- `src/eval_pack_pilot.rs` The 20 evals of `execute`, `tdd`, `verify` and `debug`, with their fixture texts.
- `src/eval_pack_rest.rs` The 35 evals of the 7 other skills.
- `src/eval_pack_fixtures_rest.rs` The fixture texts of those 35 evals.
- `tests/eval_pack_pilot/mod.rs` The proof of the pilot evals: the untouched fixture fails, the reference solution passes.
- `tests/eval_pack_rest.rs` The same proof for the 35 other evals.
- `tests/fixtures/eval-solutions/**` The reference solutions and the wrong solutions of the 55 evals.

## Order

- [x] t1: Write the tests that fail before the code for `eval run --ab` and `eval verdict`, with a fake `claude` (`core-engineer`). They cover the 2 arms (the folder, the lines, the denied command), the pairs and the result file. They also cover an excluded run of each class with `SPEC_DNA_EVAL_BACKOFF=0`, the cap before and during a run, the target `agents-line` and the usage errors. Files: `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab` fails on each case.
- [x] t2: Write the tests that fail before the code for `eval pack` (`init-engineer`). They cover 55 valid evals with the key `skill`, a kept file, checks that name their codes, and an estimate of `--ab --dry-run`. Files: `tests/eval_pack.rs`. Verify: `cargo test --test eval_pack` fails on each case.
- [x] t3: Write the tests that fail before the code for SD507 and for the key `skill` (`diagnostics-engineer`). Add the guard tests of `eval run` without `--ab` and of `stats --sdlc` (`core-engineer`). Files: `tests/evals_checks.rs`, `tests/eval_run.rs`, `tests/stats_evals.rs`. Verify: `cargo test --test evals_checks` fails on the new cases.
- [x] t4: Add SplitMix64, the paired bootstrap, the rules of D33 with unit tests on 5 fixed inputs, and an empty table of the release (`core-engineer`). Files: `src/verdict.rs`, `src/main.rs`. Verify: `cargo test --bin spec-dna verdict` passes.
- [x] t5: Add the simulation of D33 as an ignored test, run it, and record the shares of rule 1, rule 2 and `on` (`core-engineer`). Files: `src/verdict.rs`, `docs/evals.md`. Verify: `cargo test --release --bin spec-dna verdict -- --ignored --nocapture` prints the 3 shares that `docs/evals.md` holds.
- [x] t6: Add the key `skill` of an eval file (`core-engineer`). Files: `src/evals.rs`. Verify: the cases of the key in `cargo test --test evals_checks` pass.
- [x] t7: Retry an attempt of `--ab` that the API refused, with the back-off, the charged cost and the counts `excluded` (`core-engineer`). Add a unit test for each class of the signal. Files: `src/eval_run.rs`. Verify: `cargo test --bin spec-dna eval_run && cargo test --test eval_run` passes.
- [x] t8: Give the adapter skill and the line of `AGENTS.md` without one discipline, and with no line (`init-engineer`). Files: `src/adapters.rs`. Verify: `cargo test --bin spec-dna adapters` passes.
- [x] t9: Add the option `--ab`, the arms, the pairs, the denied command, the pin of the copy and the result file (`core-engineer`). Files: `src/eval_ab.rs`, `src/eval_run.rs`, `src/main.rs`. Verify: the cases of the arms and of the result file in `cargo test --test eval_ab` pass.
- [x] t10: Add the commands `eval pack` and `eval verdict` and their help (`core-engineer`). Files: `src/main.rs`, `tests/cli.rs`. Verify: `cargo test --test eval_ab --test cli` passes.
- [x] t11: Add the writer of `eval pack` and the base fixtures (`init-engineer`). Files: `src/eval_pack.rs`, `src/main.rs`. Verify: the cases of the writer in `cargo test --test eval_pack` pass.
- [x] t12: Write the 5 evals of `execute` and of `tdd` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: the cases of the 2 skills in `cargo test --test eval_pack` pass.
- [x] t13: Write the 5 evals of `verify` and of `debug` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: the cases of the 2 skills in `cargo test --test eval_pack` pass.
- [x] t14: Write the 5 evals of `brainstorm` and of `specify` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: the cases of the 2 skills in `cargo test --test eval_pack` pass.
- [x] t15: Write the 5 evals of `clarify` and of `write-plan` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: the cases of the 2 skills in `cargo test --test eval_pack` pass.
- [x] t16: Write the 5 evals of `review` and of `finish` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: the cases of the 2 skills in `cargo test --test eval_pack` pass.
- [x] t17: Write the 5 evals of `post-mortem` (`init-engineer`). Files: `src/eval_pack.rs`. Verify: `cargo test --test eval_pack` passes.
- [x] t18: Add SD507, its check and its severity in each profile (`diagnostics-engineer`; `core-engineer` for the profile table). Files: `src/diag.rs`, `src/check.rs`, `src/config.rs`. Verify: `cargo test --test evals_checks --test stats_evals` passes.
- [x] t19: Update the docs of the evals and of the commands (`core-engineer`; `init-engineer` for the pack). Files: `docs/evals.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` passes.
- [x] t20: Update the docs of the codes and the profiles, and the glossary (`diagnostics-engineer`; `architect` for the glossary). Files: `docs/diagnostics.md`, `docs/profiles.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files` passes.
- [x] t21: Write the owners of the new files and the entry of the change (`architect`; the owner accepts). Files: `.claude/agents/ownership.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- The design gives 3 hours for this slice, and the 55 evals are most of the work. The evals of the pilot come first (t12, t13), so the estimate of the pilot is ready early.
- The signal of a refused run is text. A later Claude Code can change it, and a missed refusal then counts as a failed run. The markers are in one function with a test for each class, and the build confirms them with one real output.
- A usage limit can last for hours. The waits end after about 63 minutes, and the command stops with `api-error`. The person runs the targets again.
- `SPEC_DNA_EVAL_BACKOFF` is a new contract. `docs/evals.md` gives it, and a value out of range is a usage error.
- The fake `claude` cannot prove that Claude Code denies `Bash(spec-dna skill:*)`. The pilot shows it: a `without` run that reads the skill text is a defect of the arm.
- Slices 2, 3a and 3b merge before slice 4, and this change starts after slice 4. It rebases on slice 4, because the arms use the skill text and the line of `AGENTS.md`.
- A check of the pack can be weak, such as the order of a test and its code. The check of `tdd` reads the git history of the copy, so a test commit must come before the code commit.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t3 failed before its task. In a scratch project, `spec-dna eval pack` writes 55 evals. Then `spec-dna eval run --ab execute --dry-run` and `spec-dna eval run --ab agents-line --dry-run` each print 40 runs and an estimate. `docs/evals.md` holds the 3 shares of the simulation. `spec-dna check --base origin/master` with the tree binary gives no error on this repository.
