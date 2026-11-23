---
status: draft
---
# Release of 0.8.0: the last owner answers and the tag: plan

## Files

- `docs/design/0.8.0.md` The answers Q51 to Q55, D81 and the outcome of D66.
- `docs/adr/0032-acceptance-in-one-pass.md` The status `accepted` and the amendment of D81.
- `src/harness.rs` The key `advisor` of the entry `acceptor`, its default by profile and the generated file.
- `src/harness_text.rs` The line `ACCEPTOR_NO_ADVISOR`.
- `src/init.rs` The value that `init` writes by profile, and the rule of `--reconfigure`.
- `src/skeleton.rs` The version of Claude Code of the workflow of the evals.
- `src/verdict.rs` The table of the release.
- `schema/spec-dna.schema.json` The key `advisor`.
- `tests/golden/harness/acceptor-no-advisor.md` The generated file with `advisor = false`.
- `tests/golden/evals/agent-evals.yml` The version of Claude Code.
- `tests/init_profiles.rs` The tests of the default and of `--reconfigure`.
- `tests/harness_roles.rs` The tests of the key and of the generated file.
- `tests/harness_skills.rs` The golden file of the agent in the skill tests.
- `tests/upgrade_cli.rs` The test that `upgrade` writes no key.
- `tests/eval_ab.rs` The copy of the eval `accept` with the line.
- `bench/results/evals/20261122T103358Z-write-plan.jsonl` The records of the rerun of `write-plan`.
- `bench/results/evals/20261122T113622Z-finish.jsonl` The records of the rerun of `finish`.
- `bench/results/evals/below-floor/20261122T103358Z-write-plan.json` Removed: the batch reached the floor.
- `bench/results/evals/below-floor/20261122T113622Z-finish.json` Removed: the batch reached the floor.
- `docs/config.md` The key and the limit of the proof by an untracked test file.
- `docs/harness.md`, `docs/init.md`, `docs/profiles.md` The key in the generated files, the questions and the profiles.
- `docs/evals.md` The cost of one pass, the freeze exception and the version of Claude Code.
- `CHANGELOG.md` The notes of the release.
- `.claude/agents/ownership.md` The rows for the new files.
- `.claude/agents/acceptor.md` The agent `acceptor` of this repository.
- `.claude/agents/README.md` The roster.
- `AGENTS.md` The paragraph on the acceptance record.

## Order

- [ ] t1: Add the key `advisor` of the entry `acceptor`, its default by profile and the line of the generated file. Files: `src/harness.rs`, `src/harness_text.rs`, `schema/spec-dna.schema.json`. Verify: `cargo test --lib harness`. Hours: 2.
- [ ] t2: Make `init` write the key by profile and keep it on `--reconfigure`. Files: `src/init.rs`, `tests/init_profiles.rs`, `tests/harness_roles.rs`. Verify: `cargo test --test init_profiles --test harness_roles`. Hours: 2.
- [ ] t3: Add the golden file of the agent with `advisor = false` and the tests of `upgrade`. Files: `tests/golden/harness/acceptor-no-advisor.md`, `tests/harness_skills.rs`, `tests/upgrade_cli.rs`. Verify: `cargo test --test harness_skills --test upgrade_cli`. Hours: 1.
- [ ] t4: Update the test of the eval copy of `accept`. Files: `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab`. Hours: 1.
- [ ] t5: Set the version of Claude Code of the workflow of the evals to 2.1.330. Files: `src/skeleton.rs`, `tests/golden/evals/agent-evals.yml`. Verify: `cargo test --test init_evals`. Hours: 1.
- [ ] t6: Commit the records of the reruns and rebuild the table of the release. Files: `bench/results/evals/20261122T103358Z-write-plan.jsonl`, `bench/results/evals/20261122T113622Z-finish.jsonl`, `src/verdict.rs`. Verify: `cargo test --lib verdict`. Hours: 2.
- [ ] t7: Remove the result files below the floor that the new records replace. Files: `bench/results/evals/below-floor/20261122T103358Z-write-plan.json`, `bench/results/evals/below-floor/20261122T113622Z-finish.json`. Verify: `cargo test --lib verdict`. Hours: 1.
- [ ] t8: Write the docs of the key and the limit of the proof. Files: `docs/config.md`, `docs/harness.md`, `docs/init.md`. Verify: `pre-commit run --all-files`. Hours: 2.
- [ ] t9: Write the profiles, the cost of one pass and the freeze exception. Files: `docs/profiles.md`, `docs/evals.md`. Verify: `pre-commit run --all-files`. Hours: 2.
- [ ] t10: Write the notes of the release. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t11: Add the rows for the new files and the paragraph on the acceptance record. Files: `.claude/agents/ownership.md`, `AGENTS.md`, `.claude/agents/README.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t12: Add the agent `acceptor` of this repository. Files: `.claude/agents/acceptor.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t13: Run the independent review of each commit and the acceptance pass, and finish the change. Files: `docs/design/0.8.0.md`. Verify: `spec-dna finish release-080 --dry-run`. Hours: 3.

## Risks

- The key is a request in the text of an agent file, and no run measures it. The docs say so.
- A rerun can give a verdict other than `on`. The table of the release then follows the records, and the notes say what changed.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `scripts/verify-release.sh` verifies the assets of the draft release before the publication.
