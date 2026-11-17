---
status: draft
---
# Import of a Spec Kit or OpenSpec project: plan

## Files

- `src/import*.rs` The command, the configuration, the plan of the documents, the 2 readers and the proof.
- `src/main.rs` The arguments, the help, the exit codes and the modules.
- `tests/common/import.rs` The 2 fixture projects.
- `tests/common/mod.rs` The module of the fixtures.
- `tests/import*.rs` The cases of the command and of each source.
- `tests/cli.rs` The help and the usage errors.
- `docs/commands.md` The section `import`, the summary and the exit codes.
- `docs/glossary.md` The term import.
- `CHANGELOG.md` The entry of the change.
- `.claude/agents/ownership.md` The owners of the new files.

## Order

- [ ] t1: Write the 2 fixture projects in the public layouts, in the words of spec-dna, and one case `fixtures_` that writes each project (`test-engineer`). Files: `tests/common/import.rs`, `tests/common/mod.rs`, `tests/import_cli.rs`. Verify: `cargo test --test import_cli fixtures_`.
- [ ] t2: Write the failing tests of the help and of the usage errors (`core-engineer`). Files: `tests/cli.rs`. Verify: `cargo test --test cli import` fails on the new cases.
- [ ] t3: Add the failing tests of the command (`init-engineer`). Files: `tests/import_cli.rs`. Verify: `cargo test --test import_cli` fails on each new case.
- [ ] t4: Write the failing tests of Spec Kit and of the proof (`hygiene-engineer`). Files: `tests/import_speckit.rs`. Verify: `cargo test --test import_speckit` fails on each case.
- [ ] t5: Write the failing tests of OpenSpec (`hygiene-engineer`). Files: `tests/import_openspec.rs`. Verify: `cargo test --test import_openspec` fails on each case.
- [ ] t6: Add the command, its help, its exit codes and the module lines (`core-engineer`). Files: `src/main.rs`. Verify: `cargo test --test cli import`.
- [ ] t7: Plan the legacy key, the areas and the types of `spec-dna.toml`. Read the file of `--areas`, and stop a pair of areas (`core-engineer`; the unit tests come first). Files: `src/import_config.rs`. Verify: `cargo test --bin spec-dna import_config`.
- [ ] t8: Add the normal form and the 5 steps of the proof (`hygiene-engineer`; the unit tests come first). Files: `src/import_proof.rs`. Verify: `cargo test --bin spec-dna import_proof`.
- [ ] t9: Plan the targets, the moves, the writes, the aliases, the links and the clashes (`hygiene-engineer`; the unit tests come first). Files: `src/import_docs.rs`. Verify: `cargo test --bin spec-dna import_docs`.
- [ ] t10: Read the constitution and the done features of Spec Kit (`hygiene-engineer`; the unit tests come first). Files: `src/import_speckit.rs`. Verify: `cargo test --bin spec-dna import_speckit`.
- [ ] t11: Read the open features of Spec Kit: the intent, the delta, the questions and the tasks (`hygiene-engineer`; the unit tests come first). Files: `src/import_speckit.rs`. Verify: `cargo test --bin spec-dna import_speckit`.
- [ ] t12: Read the domain specs of OpenSpec (`hygiene-engineer`; the unit tests come first). Files: `src/import_openspec.rs`. Verify: `cargo test --bin spec-dna import_openspec`.
- [ ] t13: Read the open changes, the delta specs and the archive of OpenSpec (`hygiene-engineer`; the unit tests come first). Files: `src/import_openspec.rs`. Verify: `cargo test --bin spec-dna import_openspec`.
- [ ] t14: Run the import: the project, the clean tree, D25, the first run of the configuration, the proof and the write (`init-engineer`). Files: `src/import.rs`. Verify: `cargo test --test import_speckit --test import_openspec`.
- [ ] t15: Add `--keep research`, the report, the text output and `--json` (`init-engineer`). Files: `src/import.rs`. Verify: `cargo test --test import_cli`.
- [ ] t16: Write the section `import`, the summary, the exit codes and the entry of the change (`init-engineer`; `core-engineer` for the summary and the exit codes). Files: `docs/commands.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`.
- [ ] t17: Add the term import and the owners of the new files (`architect`; the owner accepts). Files: `docs/glossary.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files`.

## Risks

- Merge order: the change `limits-061` merges first, because t7 uses the compare of D30 for a pair of areas.
- The cases of t4 and t5 need the whole command, so they pass at t14, and the cases of t3 pass at t15. The unit tests of t7 to t13 come first, on trees in memory.
- The fixtures follow the public templates, and no real project proves the rules yet. A real feature can have an `SC-nnn` line with `MUST`, or a broken link, and the proof then stops. The message names the line.
- `git mv` with a large edit, as for the constitution, can lose the rename in git. t4 measures the similarity on the fixture.
- With `strict`, SD608 is an error for each imported requirement, and with `--skip-open` SD917 and SD921 are errors too (probe of 2026-11-17). The report names them.
- t10 to t13 edit 2 files. One agent builds each reader, in the order of the tasks.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t2 to t5 failed before its task. In a scratch project from each fixture, `spec-dna import <source> --dry-run` and `spec-dna import <source>` print the same plan, and a second run prints `spec-dna import: nothing to change`. `spec-dna check --base origin/master` with the tree binary gives no error on this repository.
