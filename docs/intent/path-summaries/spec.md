---
status: draft
---
# `context` for a source file serves the summaries of its areas: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the packing order and the share of the tier `summary` of `context` for a request with a code path and no document target. The text and the `--json` output keep their form, and only the set of the units in full and in stubs changes. The [design](../../design/0.8.1-b.md) states the contract in D101, and [ADR 0035](../../adr/0035-summaries-for-a-source-file.md) holds the rules. ADR 0035 amends [ADR 0028](../../adr/0028-roll-up-and-summaries.md).

## Interfaces

- None: no interface document describes this command.

## Decisions

- [[adr/0028-roll-up-and-summaries]]
- [[adr/0035-summaries-for-a-source-file]]

## Concerns

- [x] The tier takes room of the citing sections after the best one. On fx-s7, the citing sections in full go from 3 to 1 at 8,000 and from 16 to 4 at 20,000 (a simulation). Resolved: the task puts the summaries before the detail sections. Each such section stays listed while the map has room, and `show` reads it.
- [x] The rules pay for the best detail section and for the floor at small budgets: fx serves 2 rule sections in full at 4,000 in place of 7 (4 with no floor). Resolved: the floor is the room of the title sections only, not a flat quarter, and a rule that is not in full stays listed while the map has room. The design gives the cost of a flat quarter for comparison.
- [x] The sizes of the design come from a simulation of the packing, not from a build. Resolved: the simulation matches the binary of 475d8a8 in 16 of 16 runs, and the build measures the sizes in t1 and puts them in the pull request.
- [x] A title section larger than a quarter of the budget gives only the stub lines of the titles under the floor. Resolved: the docs state the limit, and a test of t4 covers it.
- [x] In one depth, the tier orders the summaries by id, so the summary of an area that cites the file weakly can come first. Resolved: the order of D83 stays, because the fixtures give no evidence for another order. Section 2 of the design keeps the question open.
- [x] `src/context.rs` can change in another change of part B. Resolved: the change that merges second rebases.

## Open questions

- [x] Where do the rules go? ADR 0028 has 13 characters of room under its hard budget of 14,000, so a dated amendment there gives SD702, an error. Answer of the owner (2026-11-27): a new ADR, ADR 0035, amends ADR 0028. The old decision stays as it is, and the budgets do not change.
