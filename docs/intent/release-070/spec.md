---
status: approved
---
# Release 0.7.0: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the version, the pin of this repository, the release notes, the docs, the table of the release and the constant of [ADR 0023](../../adr/0023-default-skills-from-the-release-table.md).

## Interfaces

- None: no interface document describes the release.

## Decisions

- [[adr/0003-distribution]]
- [[adr/0013-self-hosting]]

## Concerns

- [x] The run gives the names of the record files, so the plan cannot name them before the run. Resolved: `Files` of the plan has the glob `bench/results/evals/*.jsonl`, and the task names `src/verdict.rs`, which holds the table that a test compares with the records.
- [x] Slice 9 of the design has no code but the version, and this change sets 2 constants of `src/verdict.rs`. Resolved: the 2 constants are data of the release. The test of ADR 0021 compares the table with the records, and the tests of ADR 0023 cover both values of the constant. Section 15 records the deviation.
- [x] The benchmark runs on shared runners, and 10% of about 80 ms is near the noise. Resolved: the owner set the rule on 2026-11-18. Both refs run in one job on one runner, in turns, and the median of the paired ratios of `full` is at most 1.10. The rule is fixed before the run.
- [x] The speed targets (a full `check` in 1 s or less with a warm cache, an incremental `check` in 100 ms or less, `context` in 200 ms or less) are for corpus A (mature product, about 2.3k docs), and the medium synthetic corpus has 1,194 documents. Resolved: the medium corpus is the gate, as for 0.6.1. A run on the large corpus gives a second point, and the release notes give both sizes.
- [x] `upgrade` on this repository writes the adapters, which this repository does not use. Resolved: the proof runs on a clone, and the commit of the release moves only the version and the pin.
- [x] The report of `upgrade` on this repository gives `SD702 x2`, and `check` gives no error. Resolved: ADR 0001 and ADR 0003 are over the hard budget. The report reads the new tree with no base, and `check` with a base gives SD704. The proof passes when the report names only these 2 errors.
- [x] A release condition fails. Resolved: no tag. The report names the condition.
- [x] ADR 0021 and ADR 0023 are `proposed`, and SD920 stops `finish` for a needed decision that is not `accepted`. Resolved: the spec links ADR 0023 in its text and not in `Decisions`. The owner accepted ADR 0018 to ADR 0023 by the word of 2026-11-18, and the release notes link them.

## Open questions

- [x] Who sets the constant of ADR 0023? Answer: the owner, after the full run. The main session writes the answer in the commit of task t1.
- [x] Does the pin move in a pull request of its own? Answer: no. The commit of the version moves it (ADR 0013).
- [x] Which commit gets the tag? Answer: the squash merge of this change on `master`, after its checks pass. The benchmark measured the head of the branch, and the 2 trees are the same.
- [x] Which Claude Code version does the eval workflow of `init` install? Answer: the value of `src/skeleton.rs` stays. The release notes name the version of the full run (D36).
