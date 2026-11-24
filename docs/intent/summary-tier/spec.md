---
status: draft
---
# The tier `summary` of `context` serves the summary of the area: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the fill order and the share of the tier `summary` of `context`. The text and the `--json` output keep their form, and only the set of the units in full and in stubs changes. The [design](../../design/0.8.1-a-evidence-and-context.md) states the contract in D83, and [ADR 0028](../../adr/0028-roll-up-and-summaries.md) holds the amendment of 2026-11-24.

## Interfaces

- None: no interface document describes this command.

## Decisions

- [[adr/0028-roll-up-and-summaries]]

## Concerns

- [x] A half of the budget gives fewer sections of `product` and `system`. Resolved: the summary of the area is the document that the agent needs, and a stub line tells the agent that each other section exists. The tier is a limit, not a reserve, and it comes after the sections of the entry and of the citing documents.
- [x] No retrieval task set has summaries, so no benchmark measures the gain of a summary. Resolved: the fixture of D83 proves the tier, and the rerun of `retrieval-synthetic-medium` proves that a corpus with no roll-up document gives the same numbers. The task set stays open in section 10 of the design.
- [x] A summary at its hard budget of 6,000 characters is still a stub at the default budget. Resolved: the docs state the limit. Such a summary is in full from a budget of about 12,200 characters.
- [x] The numbers after the fix do not exist before the build. Resolved: the build measures the fixture at the 4 budgets, and puts the table in the pull request and in the test log.
- [x] The changes `test-tag-bind` and `summary-tier` both edit `src/context.rs` (`show` and the tier), and D84 edits the chain later. Resolved: the functions are apart, and the change that merges second rebases.

## Open questions

- [x] Which share does the tier get? Answer of the owner (Q1 of the design, 2026-11-24): (a) a half of the budget, one constant.
