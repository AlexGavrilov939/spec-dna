---
status: approved
---
# Release 0.7.0: plan

## Files

- `bench/results/evals/*.jsonl` The records of the full run. The run gives the names.
- `src/verdict.rs` The table of the release and the constant of ADR 0023.
- `tests/cli.rs` The test of the verdict list, with no value of the table.
- `tests/eval_ab.rs` The tests of `eval verdict`, with the values of the table.
- `tests/upgrade_cli.rs` The test of a skills list that the upgrade cannot edit.
- `tests/upgrade_v061.rs` The files that change by their version mark when the binary is 0.7.0.
- `Cargo.toml`, `Cargo.lock`, `spec-dna.toml` The version and the pin, in one commit.
- `src/init.rs`, `tests/golden/evals/agent-evals.yml`, `docs/quickstart.md` The checkout SHA that `init` writes, only if it differs from CI.
- `CHANGELOG.md` The section 0.7.0.
- `docs/guides/upgrade-0.6.md` The guide of the upgrade from 0.6 (new).
- `docs/guides/upgrade-0.5.md`, `docs/README.md` The links to the new guide.
- `README.md`, `docs/config.md` The version, and the text of the README on the import and the profiles.
- `docs/research/matrix.md` The versions of the 3 tools on the date of the release.

## Order

- [ ] t1: Write the records of each result file of the full run with `spec-dna eval records`. Set the table of the release from the records, and the constant `DEFAULT_ON_NEEDS_VERDICT_ON` from the word of the owner (the main session; `core-engineer` for the 2 constants). The glob of `Files` covers the records. Files: `src/verdict.rs`. Verify: `cargo test --bin spec-dna verdict`. Hours: 1.
- [ ] t2: Set the version 0.7.0, and move the pin of this repository in the same commit (`release-engineer`). Files: `Cargo.toml`, `Cargo.lock`, `spec-dna.toml`. Verify: `cargo build && target/debug/spec-dna check --base origin/master`. Hours: 0.25.
- [ ] t3: Run the tests of the fixture of 0.6.1 with the version 0.7.0, so that the pin of the fixture moves (`release-engineer`). Files: none. Verify: `cargo test --test upgrade_v061`. Hours: 0.25.
- [ ] t4: Prove `upgrade` with the binary of t2 on a new project that the binary 0.6.1 writes, and on a clone of this repository at the commit before t2 (the main session). Files: none. Verify: in each project, `spec-dna upgrade` prints `proof: ok`, `spec-dna check --base <the commit before>` exits with code 0, and a second `spec-dna upgrade` prints `spec-dna upgrade: nothing to change`. Hours: 0.75.
- [ ] t5: Compare the checkout SHA that `init` writes with the one of CI. Change it only if they differ (`release-engineer`). Files: `src/init.rs`, `tests/golden/evals/agent-evals.yml`, `docs/quickstart.md`. Verify: `git grep -h -o 'actions/checkout@[0-9a-f]*' -- src/init.rs .github/workflows/ci.yml | sort -u | wc -l` prints 1. Hours: 0.25.
- [ ] t6: Run the benchmark in Actions on the head of the branch: 2 pairs of `v0.6.1` and the head on the medium corpus, in turns, then the head on the large corpus (the main session). Files: none. Verify: the mean of the 2 medians of `full` of the head is 1.10 times the mean of `v0.6.1` or less, and on the medium corpus `warm` is 1,000 ms or less, `incremental` 100 ms or less and `context_0` 200 ms or less. Hours: 0.75.
- [ ] t7: Write the section 0.7.0 of the changelog: the date, the upgrade from 0.6, the verdict of each skill and the skills with no verdict `on`, the benchmark with the sizes of the corpus, and the known limits (`release-engineer`). Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t8: Write the guide of the upgrade from 0.6, and link it from the map and from the guide of 0.5 (`docs-editor`; `architect` for the facts). Files: `docs/guides/upgrade-0.6.md`, `docs/README.md`, `docs/guides/upgrade-0.5.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t9: Write the text of the README on the import, the profiles and the date of the test of the 2 tools (`architect`; `docs-editor` for the language). Files: `README.md`. Verify: `pre-commit run --all-files`. Hours: 0.25.
- [ ] t10: Set the version 0.7.0 in the install lines and the examples of the docs (`release-engineer`). Files: `README.md`, `docs/quickstart.md`, `docs/config.md`. Verify: `git grep -n '0\.6\.1' -- README.md docs/quickstart.md docs/config.md` prints nothing. Hours: 0.25.
- [ ] t11: Check the versions of Spec Kit, OpenSpec and Superpowers on the date of the release, and name the date (`architect`). Files: `docs/research/matrix.md`. Verify: `pre-commit run --all-files`. Hours: 0.5.
- [ ] t12: Run the checks of `AGENTS.md` and the demo on the branch (`release-engineer`). Files: none. Verify: `pre-commit run --all-files && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && scripts/demo-replay.sh`. Hours: 0.5.
- [ ] t13: Run the final review of `checker` on the whole diff since `v0.6.1` (`checker`). Files: none. Verify: the report of `checker` has no open item of high severity. Hours: 1.

## Risks

- The full run ends late, or below the budget. Then t1 waits, and no tag comes before the records and the table exist.
- The owner can set the constant to `false`. Then `init` proposes the 7 skills of D2 again, and the changelog of t7 names them.
- The squash merge of another pull request after t6 changes the tree of the tag. Then t6 runs again on the new head.
- A defect that t3, t4, t12 or t13 finds stops the release. A change of its own fixes it, and this change waits.
- Shared runners give noisy times. The rule of t6 is fixed before the first run, and the changelog gives each number.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass with the version 0.7.0 and the pin 0.7.0. `spec-dna eval verdict --release` prints the table of the release, and its test agrees with the records. `spec-dna upgrade` passes on a project of 0.6.1 and on a clone of this repository. The benchmark meets the rule of t6. `spec-dna check --base origin/master` with the tree binary gives no error. `spec-dna finish release-070` runs before the merge.
