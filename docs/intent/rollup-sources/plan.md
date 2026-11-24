---
status: approved
---
# The roll-up reads interfaces, rules, decisions and research, and an old plan gets one note: plan

## Files

- `docs/design/0.8.1-a-evidence-and-context.md` The design of the source layers (D84) and of the old plan (D86).
- `docs/adr/0028-roll-up-and-summaries.md` The amendment of 2026-11-24 (slice A0 of the design).
- `src/layer.rs` The key rolls-up, its default and its 3 errors of the configuration.
- `schema/spec-dna.schema.json` The key rolls-up.
- `src/model.rs` The hash of the text in force of a whole document.
- `src/diag.rs` The codes SD646 and SD647, and the texts of SD643 and SD644.
- `src/config.rs` The profile rows of SD646 and SD647 and their group.
- `src/rollup.rs` The source layers, the area, the items of the frontier, SD646, SD647, the guard of refresh and the pins of a new summary.
- `src/check.rs` SD646 and SD647 in the check of the roll-up.
- `src/main.rs` The area of a new summary, if the command refuses a project with no layer of requirements before the roll-up.
- `src/mv.rs` The summaries that a move of a document of a source layer changes.
- `src/split.rs` The summaries after a move, if the area of a source document needs it.
- `src/context.rs` The area of a served document of a source layer in the chain.
- `src/acceptance.rs` The opt-in of the need rule by a record in the folder, and the text of the note.
- `src/finish.rs` The note of finish, and step 5.2 for a change that opted in.
- `src/stage.rs` The note of next, and its entry in the notes of the JSON output.
- `tests/config_layers.rs` The errors of the key.
- `tests/rollup_checks.rs` The tests of the sources, of SD646 and of SD647.
- `tests/rollup_refresh.rs` The tests of refresh, of a new summary and of a move.
- `tests/context_layers.rs` The test of the chain of a decision.
- `tests/acceptance_need.rs` The tests of the note and of the opt-in at finish.
- `tests/acceptance_checks.rs` The text of the note of finish, and the record of an old plan.
- `tests/accept_next.rs` The tests of the note of next.
- `docs/config.md` The key rolls-up, and the opt-in in the section of the acceptance record.
- `docs/types.md` The summary of decisions and its example, and the link of the heading of the roll-up.
- `docs/diagnostics.md` SD646, SD647, the texts of SD643 and SD644, and the heading of the roll-up.
- `docs/profiles.md` The rows of SD646 and SD647.
- `docs/commands.md` The sections of refresh, next and finish, and the link of the heading of the roll-up.
- `CHANGELOG.md` The entry of the change.

## Order

- [x] t1: Add the key rolls-up to the layer of the type summary, with the default of the layer of the type requirements. Make each of the 3 errors of D84 item 1 exit code 3: a name of a layer of another type, the key on a layer with no type summary, and a source folder inside another. Files: `src/layer.rs`, `schema/spec-dna.schema.json`, `tests/config_layers.rs`. Verify: `cargo test --test config_layers`, `cargo test --bin spec-dna layer::` and `cargo test --bin spec-dna config::`. Hours: 3.
- [x] t2: Add the hash of the text in force: the frozen hashes of the chunks outside the history and live sections, with the amendment sections, as `frozen_hash` combines them (D84 item 4). Add its unit tests. Files: `src/model.rs`. Verify: `cargo test --bin spec-dna model::`. Hours: 2.
- [x] t3: Add the codes SD646 and SD647, their profile rows in the group rollup and their rows in the profiles page. Files: `src/diag.rs`, `src/config.rs`, `docs/profiles.md`. Verify: `cargo test --bin spec-dna diag::` and `cargo test --bin spec-dna config::`. Hours: 1.
- [x] t4: Read the source layers of the types requirements, interface and rule: the area under the folder of each source layer, one item for each requirement of an active document, and SD643 and SD644 with their new texts (D84 items 2, 3, 5 and 6). Files: `src/rollup.rs`, `src/diag.rs`, `tests/rollup_checks.rs`. Verify: `cargo test --test rollup_checks` and `cargo test --bin spec-dna rollup::`. Hours: 6.
- [x] t5: Add the decisions and the research documents as whole items: an accepted decision and each research document of a source layer are in the frontier, their pin matches the hash of the text in force, SD641 names the successor of a superseded decision, and a changed note gives SD647 (D84 item 3). Files: `src/rollup.rs`, `src/check.rs`, `tests/rollup_checks.rs`. Verify: `cargo test --test rollup_checks` and `cargo test --bin spec-dna rollup::`. Hours: 6.
- [x] t6: Report SD646 on a roll-up document with no pin and an empty frontier that another roll-up document pins, only where SD641 and SD642 are silent, and compare it with the base by the fingerprint (D84 item 7). Files: `src/rollup.rs`, `src/check.rs`, `tests/rollup_checks.rs`. Verify: `cargo test --test rollup_checks`. Hours: 3.
- [x] t7: Make refresh write the frontier of the source layers and refuse a document that has SD647 and the text of the base. Make `new summary` write the pins of an area in a project with only decisions and research (D84 item 9). Files: `src/rollup.rs`, `src/main.rs`, `tests/rollup_refresh.rs`. Verify: `cargo test --test rollup_refresh`. Hours: 3.
- [x] t8: Make `mv` name the summaries whose frontier a move of a document of a source layer changes (D84 item 11). Files: `src/mv.rs`, `src/split.rs`, `tests/rollup_refresh.rs`. Verify: `cargo test --test rollup_refresh`. Hours: 2.
- [x] t9: Give the chain of context the area of each served document of a source layer: an active requirements, interface or rule document, an accepted decision, and a research document that a target names (D84 item 8). Files: `src/context.rs`, `tests/context_layers.rs`. Verify: `cargo test --test context_layers`. Hours: 3.
- [x] t10: Make a record in the folder of a change of an old plan end the exemption of the need rule, and give the note the text of D86 with the prompt of the acceptor. The note is only for a change that would need a record if its plan were new, while its folder has no record. Files: `src/acceptance.rs`, `tests/acceptance_need.rs`, `tests/acceptance_checks.rs`. Verify: `cargo test --test acceptance_need --test acceptance_checks`. Hours: 3.
- [x] t11: Make `finish` print the note in place of the note of 0.8.0, and check the record of a change that opted in at step 5.2 as for a change of 0.8.0. Files: `src/finish.rs`, `tests/acceptance_need.rs`. Verify: `cargo test --test acceptance_need --test acceptance_checks`. Hours: 2.
- [x] t12: Make `next` print the note after the checklist, and put it in the notes of `next --json` before the note of the roll-up. Files: `src/stage.rs`, `tests/accept_next.rs`. Verify: `cargo test --test accept_next`. Hours: 2.
- [x] t13: Document the key and its rules, the summary of decisions with the example of D84, the section of refresh, and the note and the opt-in in the sections of next and finish and of the acceptance record. Files: `docs/config.md`, `docs/types.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 3.
- [ ] t14: Document SD646, SD647 and the new texts of SD643 and SD644, rename the heading of the roll-up to SD641 to SD647, and change its links in the 3 pages. Files: `docs/diagnostics.md`, `docs/commands.md`, `docs/types.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [ ] t15: Write the entry of the change. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t16: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish rollup-sources --dry-run`. Hours: 3.

## Risks

- `src/context.rs` and `tests/context_layers.rs` also change in the change `summary-tier` (the fill order and the share of the tier), which merges first. This change rebases on it, and t9 adds the area of a source document to the chain of the new tier.
- `src/stage.rs`, `src/acceptance.rs`, `src/finish.rs`, `src/check.rs`, `src/diag.rs`, `src/config.rs`, `tests/accept_next.rs` and some pages also change in the changes `lifecycle-fixes` and `test-tag-bind`. The change that merges second rebases and runs each check again.
- A project that opts in rewrites its summaries once, and each pin adds about 60 bytes to the front matter of a summary. No byte goes into `context`.
- A summary of decisions with current pins can still describe a decision wrongly, and a pin to a research note says only that the author saw the note. The docs state both limits.
- A change of 0.7.1 that already has a record in its folder now gets the full check of step 5.2 at `finish`, and the record can fail it. That is the opt-in: an agent wrote the record, and `finish` names the code of each problem.
- Q3 of the design is open. If the owner chooses no opt-in, t10 and t11 keep only the note.

## Proof

Each test fails before the fix, unless it says "guard". The "before" results come from the code of 0.8.0.

- `tests/rollup_checks.rs`, with `rolls-up = ["requirements", "decisions"]`: `summaries/storage.md` pins the accepted `decisions/storage/use-sqlite`, and no finding. Before: SD644 and SD643.
- The decision becomes superseded by a new accepted decision of the same area: SD641 names the successor, and SD642 names the new decision. Before: SD643 only.
- An amendment section added to the accepted decision: SD641, "the text in force changed". Before: SD643, and with `frozen_hash` there would be no finding. An edit of its live section Status: no finding.
- A research note edited: SD647, a warning in `standard`, and no SD641.
- An interface source `interfaces/valves/api.md` with the key `open-close`: SD642 until a pin exists, then SD641 after an edit of the key. Before: SD643 for the pin.
- SD646: a summary of an area with only draft requirements, pinned by `system`: a warning in `standard`, an error in `strict`, and nothing in `lite`. Before: no finding.
- `tests/rollup_refresh.rs`: `refresh summaries/storage` after an edit of the note and with the text of the base is refused with exit code 1, and with `--holds` it is held with SD645. Before: written with no claim. A project with only the layers `decisions` and `research` and both in `rolls-up`: `new summary storage` writes the pins, and `check` gives no finding. Before: exit code 2 and SD644. A move of a decision of a source layer names the summary of its area. Before: no summary.
- `tests/context_layers.rs`: a project with decisions only, `context decisions/storage/use-sqlite`: the tier has `summaries/storage`. Before: no tier. `a_corpus_with_no_summary_keeps_the_old_output` stays green (guard).
- `tests/config_layers.rs`: `rolls-up` on the layer `rules`, a name of a layer of the type `procedure`, and a source folder inside another each give exit code 3.
- The unit tests of `src/model.rs`: the hash of the text in force changes with an amendment section and not with an edit of a live section or of a link destination. `frozen_hash` keeps its value (guard).
- Each test of `tests/rollup_checks.rs` and `tests/rollup_refresh.rs` of 0.8.0 passes with no key (guard).
- `tests/accept_next.rs`: a `higher-risk` change in `standard` whose plan the base added at the pin 0.7.1: `next` prints the note, byte for byte, and `next --json` has it in `notes`. Before: no line. The test of the output of 0.7.1 gets the note for the old plan.
- `tests/acceptance_need.rs`: `finish` of the same change prints the new text on stderr. Before: the text of 0.8.0. The same change with an accepted record whose code changed after the acceptance stops at step 5.2 with SD929. Before: `finish` archived the record. The same change in `lite`, or a `routine` change in `standard`: no note (guard).
- `tests/acceptance_checks.rs`: the test of the note of an old pin gets the new text. In `finish_archives_a_record_that_is_present_also_in_lite_and_for_an_old_pin`, the record of an old plan goes through step 5.2, and no note prints. The part of `lite` stays (guard).

The retrieval benchmark has no corpus with a roll-up document (E16 of the design), so the change does not run it again: a corpus with no key and no summary gives the output of 0.8.0.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
