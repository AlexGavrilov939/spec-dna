---
status: approved
---
# The tier `summary` of `context` serves the summary of the area: plan

## Files

- `docs/design/0.8.1-a-evidence-and-context.md` The design of the tier (D83).
- `docs/adr/0028-roll-up-and-summaries.md` The amendment of 2026-11-24 (slice A0 of the design).
- `src/context.rs` The fill order of the tier, its share and the fill of its sections.
- `tests/context_layers.rs` The fixture with a real layer graph, and the tests of the tier.
- `docs/commands.md` The 2 sentences of the section of context on the tier.
- `bench/results/retrieval-synthetic-medium.json` The rerun of the retrieval benchmark. The file changes only if the numbers differ.
- `CHANGELOG.md` The entry of the change.

## Order

- [x] t1: Add the fixture of D83 with a real layer graph, the guard of the output order at each budget, and one line of the sizes for each budget in the test log. Files: `tests/context_layers.rs`. Verify: `cargo test --test context_layers -- --nocapture`. Hours: 3.
- [x] t2: Fill the tier in the new order: the title section of each document first, from the deepest summary up, then the other sections. The served sections are a prefix of the fill order. The test of the prefix of the output order becomes the test of the prefix of the fill order. Files: `src/context.rs`, `tests/context_layers.rs`. Verify: `cargo test --test context_layers`. Hours: 4.
- [x] t3: Give the tier at most a half of the budget, and update the 3 tests of the share and of the modes at 8,000. Files: `src/context.rs`, `tests/context_layers.rs`. Verify: `cargo test --test context_layers -- --nocapture`. Hours: 2.
- [x] t4: Write the fill order and the share in the section of context. Files: `docs/commands.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t5: Run the retrieval benchmark on the synthetic medium corpus at the budget 8,000 again, and compare the 3 numbers of the proof. Files: `bench/results/retrieval-synthetic-medium.json`. Verify: `python3 bench/retrieval/run.py --budgets 8000` on the corpus of `python3 bench/gen/gen.py --model spec-dna --size medium`, with a release build, and the 3 numbers equal. Hours: 1.
- [ ] t6: Write the entry of the change. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t7: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish summary-tier --dry-run`. Hours: 2.

## Risks

- The half share gives fewer sections of `product` and `system` in a project with summaries. Each such section is a stub or is counted in the line of the sections that did not fit, so the agent can read it with `show`.
- The rerun can give other numbers than the result file of 0.8.0, also for a corpus with no roll-up document. Then the cause is a defect of the change, not of the benchmark: the build stops and reports it before the result file changes.
- `src/context.rs` also changes in the change `test-tag-bind` (`show`) and later in D84 (the chain). The change that merges second rebases.

## Proof

Each test fails before the fix, unless it says "guard".

- `tests/context_layers.rs`, the fixture of D83 with the summary of `valves` of 600 bytes, `context src/motor.rs` at 8,000: `summaries/valves/drives` and `summaries/valves` in full. Before the fix: neither is listed.
- The same with the summary of 3,000 bytes: in full at 8,000, and a listed stub at 4,000. Before the fix: not listed at both budgets.
- The output order stays `product`, `system`, `summaries/valves`, `summaries/valves/drives` at each budget (guard).
- `the_tier_is_a_prefix_of_its_fill_order_at_each_budget` replaces the test of the prefix of the output order: for each rule size and each budget from 600 to 4,600, no section of the fill order is in full after a stub.
- `the_summary_tier_uses_at_most_a_half_of_the_budget` replaces the test of a quarter: the summary of 3,200 characters is in full at 8,000, and the tier has at most 4,000 characters. The test of the share takes a budget of 3,000 for its stub case, and the test of the modes expects both summaries in full at 8,000.
- `a_corpus_with_no_summary_keeps_the_old_output` stays green with no change (guard).

The sizes of `context src/motor.rs` before the fix, measured on 2026-11-24 with a debug build of 0.8.0 (E15 of the design). The output is the same for both sizes of the summary of `valves`:

| Budget | Bytes | Characters of the budget | Tokens (estimate) | Tier |
| --- | --- | --- | --- | --- |
| 4,000 | 1,574 | 1,319 | 361 | `product`: the title in full, 7 stubs. No `system`, no summary |
| 8,000 | 2,530 | 2,277 | 600 | `product`: the title and 2 sections in full, 6 stubs. No `system`, no summary |
| 12,000 | 3,572 | 3,321 | 860 | `product`: the title and 4 sections in full, 4 stubs. `system`: 2 stubs. No summary |
| 20,000 | 5,549 | 5,299 | 1,354 | `product`: all 9 sections in full. `system`: 5 stubs. No summary |

After the fix, the build measures the same 4 rows with the test log of t3, and puts the table in the pull request. Expected at 8,000 with the summary of 3,000 bytes: `summaries/valves/drives` and `summaries/valves` in full, and the titles of `system` and `product` as stubs or not listed. This plan gives no measured number after the fix, because no build exists yet.

The retrieval benchmark: `bench/results/retrieval-synthetic-medium.json` gives at 8,000 a `chars_median` of 5227, a `recall_full_mean` of 0.995 and a `precision_mean` of 0.756. The rerun of t5 gives the same 3 numbers, because the corpus has no roll-up document.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
