---
status: draft
---
# Hours of a change and links into a change: plan

## Files

- `tests/lifecycle_log.rs` The tests of the line `Hours:` in `log` and `stats --sdlc`.
- `tests/finish.rs` The test of the total that `finish` prints.
- `tests/layers_checks.rs` The tests of SD612.
- `tests/lifecycle_finished.rs` The tests of the locked links.
- `tests/golden/stats-sdlc.txt` The new row of `stats --sdlc`.
- `tests/golden/stats-sdlc-evals.txt` The new row of `stats --sdlc`.
- `src/git.rs` The messages of the listed commits, in one git process.
- `src/finish.rs` The total of `Hours`, and the search of step 0 as a function that `check` also calls.
- `src/main.rs` The line `Hours:` and the field `hours` of `finish`, and the field `commit` of `locked_links`.
- `src/sdlc.rs` The field `hours` of `log`, the row of `stats --sdlc`, and the list of the ended changes.
- `src/check.rs` SD612, and the locked links.
- `src/diag.rs` The code SD612.
- `docs/diagnostics.md` SD612 and the section "Locked references".
- `docs/profiles.md` SD612 is a code in no group.
- `docs/commands.md` `finish`, `log`, `stats --sdlc` and `check --json`.
- `docs/types.md` The section `Hours` of a plan.
- `docs/guides/change.md` The line `Hours:` in the commit message.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Write the failing tests of the hours: `finish` prints `Hours: 3.5`, and `log` and `stats --sdlc` read the line from a squash commit with the line in a middle paragraph (`core-engineer`, `hygiene-engineer`). Files: `tests/lifecycle_log.rs`, `tests/finish.rs`. Verify: `cargo test --test lifecycle_log --test finish` fails on the new tests only.
- [ ] t2: Write the failing tests of SD612 in `standard` and `lite`, and of the locked links: an amendment that names the target, and the pointer with the commit (`diagnostics-engineer`). Files: `tests/layers_checks.rs`, `tests/lifecycle_finished.rs`. Verify: `cargo test --test layers_checks --test lifecycle_finished` fails on the new tests only.
- [ ] t3: Read the messages of a list of commits with one git process (`core-engineer`). Files: `src/git.rs`. Verify: `cargo test --lib git` passes.
- [ ] t4: Add the total of `Hours`, the line `Hours: <total>`, the note of an item with no number, and the field `hours` to `finish` (`hygiene-engineer`). Files: `src/finish.rs`, `src/main.rs`. Verify: `cargo test --test finish` passes.
- [ ] t5: Read the line `Hours:` in `log` and `stats --sdlc` (`core-engineer`). Files: `src/sdlc.rs`, `tests/golden/stats-sdlc.txt`, `tests/golden/stats-sdlc-evals.txt`. Verify: `cargo test --test lifecycle_log --test stats_evals` passes.
- [ ] t6: Make the search of step 0 of `finish` a function that takes the corpus and the change folders, with no change of its result (`hygiene-engineer`). Files: `src/finish.rs`. Verify: `cargo test --test finish` passes.
- [ ] t7: Add SD612 with the order of SD611 and SD901 (`diagnostics-engineer`). Files: `src/diag.rs`, `src/check.rs`. Verify: `cargo test --test layers_checks` passes.
- [ ] t8: Clear a locked link that an amendment names, and name the commit of a finished change in the pointer and in `locked_links` (`diagnostics-engineer`; `core-engineer` for `sdlc`). Files: `src/check.rs`, `src/sdlc.rs`, `src/main.rs`. Verify: `cargo test --test lifecycle_finished` passes.
- [ ] t9: Update the docs of SD612, the locked links and the commands. Files: `docs/diagnostics.md`, `docs/profiles.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t10: Update the docs of the plan type and the guide, and the change log. Files: `docs/types.md`, `docs/guides/change.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- A person who does not copy the line `Hours:` into the squash message loses the hours of the change. The skill `finish` and the guide say where the line goes.
- The text line of `log` gets 2 tokens (`hours <n>` or `hours -`). A script that reads the line by position must change.
- `check` reads the history for the pointer of a locked link. On a long history this costs one `git log` for each intent folder, and only when such a link exists.
- Slice 7 also edits `src/sdlc.rs` and `src/git.rs` (D31), so it starts after this change.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 and t2 failed before its task. `spec-dna log --json` on a squash commit with the line gives `"hours":3.5`. `spec-dna check --base origin/master` gives no error on this repository.
