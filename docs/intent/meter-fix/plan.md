---
status: approved
---
# The context meter reads the last executor iteration, and the briefs name sections: plan

## Files

- `docs/design/0.8.1-a-lifecycle-and-harness.md` The design of the meter and of the briefs (D94).
- `src/hook.rs` The reading of a transcript line in the hook context-meter, and its unit tests.
- `tests/hook.rs` The 3 tests of the meter of D94.
- `docs/harness.md` The section of the hook context-meter in the section Hooks.
- `.claude/agents/core-engineer.md` The line Read first and the line Reads.
- `.claude/agents/diagnostics-engineer.md` The line Read first and the line Reads.
- `.claude/agents/context-engineer.md` The line Read first and the line Reads.
- `.claude/agents/hygiene-engineer.md` The line Read first and the line Reads.
- `.claude/agents/init-engineer.md` The line Read first and the line Reads.
- `.claude/agents/transcript-engineer.md` The line Read first and the line Reads.
- `.claude/agents/test-engineer.md` The line Read first and the line Reads.
- `.claude/agents/release-engineer.md` The line Read first and the line Reads.
- `.claude/agents/docs-editor.md` The line Read first and the line Reads.
- `.claude/agents/failure-analyst.md` The line Read first and the line Reads.
- `.claude/agents/checker.md` The line Reads only.
- `.claude/agents/architect.md` The line Reads only.
- `CHANGELOG.md` The entry of the change.

## Order

- [x] t1: Add the guard `meter_is_not_cumulative`. Then make the reading of a line take the sum of the 3 token fields of the last iteration of type message, else of usage, and add `meter_counts_the_last_executor_iteration` and a unit test for an iteration of another type (D94 item 2). Files: `src/hook.rs`, `tests/hook.rs`. Verify: `cargo test --test hook` and `cargo test --bin spec-dna hook::`. Hours: 2.
- [x] t2: Make the reading skip a line whose model is `<synthetic>`, so the backward scan reads the line before it, and add `meter_skips_a_synthetic_line` (D94 item 1). Files: `src/hook.rs`, `tests/hook.rs`. Verify: `cargo test --test hook` and `cargo test --bin spec-dna hook::`. Hours: 1.
- [x] t3: Write the reading of the meter in the section of the hook: the last iteration of type message, and no count for a synthetic line. Files: `docs/harness.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t4: Write the new lines Read first of D94 and the line Reads after them. Files: `.claude/agents/core-engineer.md`, `.claude/agents/diagnostics-engineer.md`, `.claude/agents/context-engineer.md`. Verify: `pre-commit run --all-files` and `grep -c '^Reads: ' .claude/agents/core-engineer.md .claude/agents/diagnostics-engineer.md .claude/agents/context-engineer.md` gives 1 for each file. Hours: 1.
- [ ] t5: Write the new lines Read first of D94 and the line Reads after them. Files: `.claude/agents/hygiene-engineer.md`, `.claude/agents/init-engineer.md`, `.claude/agents/transcript-engineer.md`. Verify: `pre-commit run --all-files` and the same grep on the 3 files. Hours: 1.
- [ ] t6: Write the new lines Read first of D94 and the line Reads after them. Files: `.claude/agents/test-engineer.md`, `.claude/agents/release-engineer.md`, `.claude/agents/docs-editor.md`. Verify: `pre-commit run --all-files` and the same grep on the 3 files. Hours: 1.
- [ ] t7: Write the new line Read first of `failure-analyst` and the line Reads after it. Add the line Reads to `checker` after its paragraph Scope, and to `architect` after its line Read first. Files: `.claude/agents/failure-analyst.md`, `.claude/agents/checker.md`, `.claude/agents/architect.md`. Verify: `pre-commit run --all-files` and the same grep on the 3 files. Hours: 1.
- [ ] t8: Write the entry of the change. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t9: Run the independent review, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish meter-fix --dry-run`. Hours: 1.

## Risks

- The shape of `usage.iterations` comes from one lab transcript with one iteration. A later Claude Code can write another shape. Then the reading falls back to the top-level usage, which is the rule of 0.8.0.
- The fix of H1 can hide a real context of 250,000 tokens only if the last iteration of type message is smaller than the context. The API gives no other number, and the test `meter_is_not_cumulative` pins the rule of one request.
- The new lines Read first name sections, so an agent can miss a fact of another section. Each line names the sections of the task, and the line Reads tells the agent to find the place with `grep -n`.
- The agent files are the change set "Agent team". The owner accepts them in the pull request, and `pre-commit` lints them.

## Proof

Each test fails before the fix, unless it says "guard".

- `tests/hook.rs`, `meter_counts_the_last_executor_iteration`: a line with a top-level sum of 260,000 and 2 iterations of type message of 130,000 each, and the level 250,000. Before the fix: the notice. After: no notice.
- `meter_skips_a_synthetic_line`: a line of 260,000, then a `<synthetic>` line with a usage of 0. Before the fix: the second event deletes the state, and the notice speaks again. After: one notice only.
- `meter_is_not_cumulative`: 2 lines of 100,000 and 120,000 read as 120,000 (guard).
- A unit test of `src/hook.rs`: a line whose last iteration has another type reads the last iteration of type message. A line with no iterations reads the top-level usage (guard).
- The tests of 0.8.0 in `tests/hook.rs` stay green with no change (guard), for example `meter_higher_level_and_reset_after_a_drop` and `meter_reads_a_long_last_line`.
- `grep -L '^Reads: '` on the 12 agent files of t4 to t7 prints nothing. `Explore.md`, `fallback-advisor.md`, `acceptor.md`, `ownership.md` and `README.md` keep their bytes.

The cost, measured on 2026-11-24 on the files of the tree: the line Reads has 175 bytes with its line end. The body of each of the 10 engineer and analyst files grows by 214 to 361 bytes, 2,735 bytes in total, and only the context of that agent holds it. The pages that the old lines named whole are no longer read whole: for example `docs/config.md` has 92,872 bytes and `docs/diagnostics.md` 107,775. No request of the main session grows, because it holds only the description of each agent.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker.
