---
status: approved
---
# Limits of 0.6.1: index, area names and a plain move: plan

## Files

- `src/registry.rs` The status of a generated index.
- `src/main.rs` The handler of `index` and its stop, and the stop of `new`.
- `src/finish.rs` The refresh of an index with the same status rule.
- `src/config.rs` The compare of the area names, and the area of a path.
- `src/mv.rs` The stop of `mv`.
- `src/upgrade_docs.rs` The stop of a pair in the plan of `--areas`.
- `src/detect.rs` The areas that `init` proposes.
- `src/init.rs` The note of the preview.
- `src/git.rs` The 3 conditions of a move out.
- `src/sdlc.rs` The comments of the move out.
- `tests/cli.rs` The cases of `index` and `mv`.
- `tests/finish.rs` The case of the refresh.
- `tests/new_place.rs` The cases of the reader and of `new`.
- `tests/upgrade_cli.rs` The case of `upgrade --layout --areas`.
- `tests/init_graph.rs` The case of `init`.
- `tests/lifecycle_log.rs` The cases of `log` and `stats --sdlc`.
- `docs/commands.md` The sections `index`, `new`, `mv`, `log` and `stats`, and the exit codes.
- `docs/config.md` The key `areas`.
- `docs/init.md` The note of the areas.
- `schema/spec-dna.schema.json` The description of `areas`.
- `CHANGELOG.md` The entry of the change.

## Order

- [x] t1: Write the failing tests of `index` and of the refresh of `finish`, from the reproduction of D29 (`hygiene-engineer`). Files: `tests/cli.rs`, `tests/finish.rs`. Verify: `cargo test --test cli --test finish` fails on the new cases.
- [x] t2: Write the failing tests of the reader and of `new` for a pair of areas (`core-engineer`). Files: `tests/new_place.rs`. Verify: `cargo test --test new_place` fails on the new cases.
- [x] t3: Write the failing test of `mv` into an area that differs only in case (`hygiene-engineer`). Files: `tests/cli.rs`. Verify: `cargo test --test cli mv` fails on the new case.
- [x] t4: Write the failing tests of `upgrade --layout --areas` and of `init` for a pair (`init-engineer`). Files: `tests/upgrade_cli.rs`, `tests/init_graph.rs`. Verify: `cargo test --test upgrade_cli --test init_graph` fails on the new cases.
- [x] t5: Write the failing tests of a plain `git mv`, the near copies, the whole folder and the case that stays. Change the test that expects a plain `git mv` to end the change (`core-engineer`). Files: `tests/lifecycle_log.rs`. Verify: `cargo test --test lifecycle_log` fails on the new cases.
- [x] t6: Give a generated index the status of its place, and stop for a place with no type (`hygiene-engineer`). Files: `src/registry.rs`, `src/main.rs`, `src/finish.rs`. Verify: `cargo test --test cli --test finish`.
- [x] t7: Compare the area names in lowercase ASCII in the reader, and stop `new` (`core-engineer`). Files: `src/config.rs`, `src/main.rs`. Verify: `cargo test --test new_place`.
- [x] t8: Stop `mv` for an area that differs only in case (`hygiene-engineer`). Files: `src/mv.rs`. Verify: `cargo test --test cli mv`.
- [x] t9: Stop the plan of `upgrade --layout --areas` for a pair, with exit code 1 (`hygiene-engineer`). Files: `src/upgrade_docs.rs`. Verify: `cargo test --test upgrade_cli`.
- [x] t10: Propose one name of a pair in `init`, with a note (`init-engineer`). Files: `src/detect.rs`, `src/init.rs`. Verify: `cargo test --test init_graph`.
- [x] t11: Read a rename out of the intent folder by the 3 conditions of D31 (`core-engineer`). Files: `src/git.rs`, `src/sdlc.rs`. Verify: `cargo test --test lifecycle_log`.
- [ ] t12: Update the sections of the commands and the exit codes. `hygiene-engineer` writes `index` and `mv`, `init-engineer` writes `upgrade`, and `core-engineer` writes the rest. Files: `docs/commands.md`. Verify: `pre-commit run --all-files`.
- [ ] t13: Update the key `areas` and the note of `init` (`core-engineer`; `init-engineer` for `docs/init.md`). Files: `docs/config.md`, `schema/spec-dna.schema.json`, `docs/init.md`. Verify: `pre-commit run --all-files`.
- [ ] t14: Write the entry of the change (`hygiene-engineer`, `core-engineer` and `init-engineer`, each for its part). Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`.

## Risks

- Merge order: this change merges before the change `import`, because the plan of `import --areas` uses the compare of D30.
- A project with 2 area names that differ only in case stops with exit code 3 at the first run. The person renames one area and moves its documents with `spec-dna mv`.
- A script that expects exit code 1 from `index` for a place with no type gets exit code 2.
- A `git mv` and a large edit in one commit still end the change, as in 0.6.1.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t5 failed before its task. The script of the reproduction of D29 gives exit code 0 and a clean `check --all`. `spec-dna check --base origin/master` with the tree binary gives no error on this repository.
