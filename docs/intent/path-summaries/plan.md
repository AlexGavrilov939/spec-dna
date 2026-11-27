---
status: draft
---
# `context` for a source file serves the summaries of its areas: plan

## Files

- `docs/design/0.8.1-b.md` The design of the tier for a code path (D101).
- `docs/adr/0035-summaries-for-a-source-file.md` The new ADR that amends ADR 0028.
- `docs/README.md` The entry of the new design page.
- `src/context.rs` The first tier, the best detail section, the packing order, the share and the floor of the tier for a code path, and the comments that state the old rule.
- `tests/context_layers.rs` The fixture with a mature layer graph, the new tests, and the split of the test of the room of a citing section.
- `docs/commands.md` The paragraphs of the section of context on the tier.
- `CHANGELOG.md` The entry of the tier `summary` under "Changed" of the unreleased version.

## Order

The responsible agent of each task is `context-engineer`.

- [ ] t1: Add the regression fixture with the layer graph of a mature project, with made-up names, in the shape of fx-s7 of the design: `[layers]` with `foundation` (`product`, `system`, `terms`), `rules`, `requirements`, `decisions` and `summaries`; 4 project-wide rules; 14 active requirement documents in 5 areas and 1 sub-area, each with 7 requirement subsections, 10 of them cite one source file by its path or by an identifier of the file; 2 accepted decisions; 6 summaries written with `new summary`; `system` and `product` with `rests-on` and `refresh --holds`. The sizes of fx-s7, so that the expected modes hold: the title sections of the summaries have 495 to 898 characters (the deepest, `mounts`, 729), the titles of `system` and `product` 359 and 339, the 8 rule sections 161 to 448 (2,459 in total), and the 43 citing sections 19,643 in total (the 8 sections that cite the file 469 to 527 each). The fixture function has 3 options: 10 rules of 4 sections (40 rule sections, 138 to 466 characters, 9,066 in total) and 2 citing documents in the deepest area, each with one requirement section of about 915 characters (the shape of fx-r10c2), a title section of 3,000 characters in the deepest summary, and one more document whose `applies-to` matches a second source file that no document cites. Add one test that prints, for `context --path <file>` at 4,000, 8,000, 12,000 and 20,000, the bytes, the used characters, the tokens of the head, the modes of the tier, and the rule and citing sections in full and in stubs. Run it on the tree before t2 and after t3, and put both tables in the pull request. Files: `tests/context_layers.rs`. Verify: `cargo test --test context_layers mature_graph -- --nocapture` prints 4 lines for each shape, and the "before" lines of the shape of fx-s7 show no section of the tier at the 4 budgets. Hours: 3.
- [ ] t2: Add the tests that fail before the fix: the tier has a section at each of the 4 budgets and the first title of the fill order is in full; with 40 rule sections, the first title is in full at 4,000, 8,000 and 12,000 and each title section is listed at 8,000 and 12,000; for the file that only the document with `applies-to` reaches, the first section of that document is in full at 8,000 and the tier has a section. Files: `tests/context_layers.rs`. Verify: `cargo test --test context_layers` fails in these 3 tests and passes in each other test. Hours: 1.
- [ ] t3: For a request with a code path and no document target, and a chain that is not empty: record the sections of the first tier (step 0 of the plan); take the best detail section (the first section of the packing order of D83 that is not in the first tier, not a convention, and can be in full, after the rule for an inactive document); pack the best detail section, then the tier in its fill order, then the other sections in their order; set the share to `min(budget / 2, budget - r1, max(budget - r2, floor))` with the floor of D101 item 6. Keep the code of D83 for each other request. Update the comments of the module, of `SUMMARY_SHARE`, of the packing order, of `summary_after_entry` and of `room_without_tier`. Files: `src/context.rs`. Verify: `cargo test --test context_layers` passes, also the tests of D83 and `the_tier_never_pushes_out_the_full_entry_that_a_tree_with_no_summary_serves` with no change. Hours: 4.
- [ ] t4: Add the tests of the limits and the guards. A title section of 3,000 characters with 40 rule sections: at 8,000 the tier lists the 4 title sections and serves none in full (fails before). Split `the_tier_never_takes_the_room_of_the_entry_or_of_a_citing_section`: the document arm keeps the name, and the path arm becomes `the_tier_never_takes_the_room_of_the_best_detail_section`, with a sweep of the budgets from 1,000 to 24,000 (step 250) on the mature graph: the best detail section is in full at each budget where the same tree with no roll-up serves it in full, also under `--query` and with 2 code paths (guard). `context <file>`, `context ./<file>` and `context --path <file>` give the same bytes (guard). A request with a document target and `--path`: the entry and the best citing section are in full at each budget of a sweep where the tree with no roll-up serves them in full (guard). Files: `tests/context_layers.rs`. Verify: `cargo test --test context_layers`. Hours: 3.
- [ ] t5: Write the rule for a code path in the section of context: the sentence "The tier never takes the room that the entry and the citing sections need in full" holds for a document target only; a new paragraph gives the packing order, the share, the floor and the 2 limits of D101 (a title section over a quarter of the budget, and no tier where the best detail section just fits). Files: `docs/commands.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t6: Edit the entry of the tier `summary` under "Changed" of the unreleased version: the half is a cap, the entry keeps its room, and for a code path the tier comes after the best detail section and before the rules and the other sections, with the floor of the titles. Give the sizes of t1 at 8,000. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t7: Run the retrieval benchmark on the synthetic corpora `medium` and `large` at 4,000, 8,000 and 12,000, once with a release build of the base of the change and once with a release build of the change, and compare `chars_median`, `recall_full_mean` and `precision_mean`. This change writes no result file: t9 of the change `part-a-leftovers` writes them again after the merge. Files: none: the results stay in the pull request. Verify: for each size, `python3 -I bench/gen/gen.py --model spec-dna --size <size> --out <dir>`, the corpus committed in a new git repository, then `python3 -I bench/retrieval/run.py --repo <dir> --tasks <dir>/tasks.json --budgets 4000,8000,12000 --out <file>` with each build, and the 3 numbers equal at each budget. Hours: 1.
- [ ] t8: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish path-summaries --dry-run`. Hours: 2.

## Risks

- The tier takes room of the other citing sections and, at small budgets, of the rules. Each such section stays listed while the map has room, so the agent can read it with `show`.
- The sizes of the design come from a simulation. If a measured cell of t1 differs from the design by more than one section, the build stops and reports it to `architect` before t5.
- The build of the change can give other numbers than the build of its base in t7, also for a corpus with no roll-up document. Then the cause is a defect of the change: the build stops and reports it.
- `src/context.rs` can change in another change of part B. The change that merges second rebases.

## Proof

Each test fails before the fix, unless it says "guard". All are in `tests/context_layers.rs`.

- The mature graph, `context --path <file>` at 4,000, 8,000, 12,000 and 20,000: the tier has a section in full, and the first title of the fill order is in full. Before the fix: no section of the tier.
- The same with 40 rule sections and 2 citing documents: the first title in full at 4,000, 8,000 and 12,000, each title listed at 8,000 and 12,000. Before the fix: no section of the tier at 4,000 and 8,000, one stub at 12,000.
- The file that only a document with `applies-to` reaches: the first section of that document in full at 8,000. Before the fix: a stub.
- A title section of 3,000 characters with 40 rule sections: at 8,000, the 4 title sections listed and none in full. Before the fix: no section of the tier.
- `the_tier_never_takes_the_room_of_the_best_detail_section`: the sweep with no loss against the tree with no roll-up (guard).
- The 3 forms of a path give the same bytes (guard). A document target with `--path` keeps its entry and its best citing section (guard). The tests of D83, `the_tier_never_pushes_out_the_full_entry_that_a_tree_with_no_summary_serves` and `a_corpus_with_no_summary_keeps_the_old_output` stay green with no change (guard). No golden file of `tests/golden/` changes.

The sizes of fx-s7 of the design (`context --path src/mount/slew.rs`). "Before" is measured with the binary of 475d8a8. "After" is a simulation of the packing that matches that binary in 16 of 16 runs, not a build: t1 measures it. `1+2` is 1 section in full and 2 stubs.

| Budget | Bytes before | Used before and after | Tier before and after | Rules in full, before and after | Citing sections in full, before and after |
| --- | --- | --- | --- | --- | --- |
| 4,000 | 4,118 | 3,853 and 3,978 | none and 1+2 | 6 and 3 | 0 and 1 |
| 8,000 | 8,179 | 7,912 and 7,949 | none and 2+0 | 8 and 7 | 3 and 1 |
| 12,000 | 12,194 | 11,925 and 11,896 | none and 4+5 | 8 and 8 | 7 and 3 |
| 20,000 | 20,190 | 19,921 and 19,901 | none and 10+6 | 8 and 8 | 16 and 4 |

The retrieval benchmark: at 8,000 the binary of 475d8a8 gives 5,271, 0.998 and 0.759 on the corpus `medium` (t9 of `part-a-leftovers`), and the result file of an older binary gives 5,227, 0.995 and 0.756. The build of the change gives the same numbers as the build of its base at each budget, because the corpora have no roll-up document.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
