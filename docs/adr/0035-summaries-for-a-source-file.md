---
status: proposed
---

# 0035. Summaries in `context` for a source file

## Context and Problem Statement

[ADR 0028](0028-roll-up-and-summaries.md) gives `context` the tier `summary`. The main use of `context` is a file that an agent edits. For such a request, `context <file>` serves no section of the tier in a project with a mature layer graph, at each budget from 4,000 to 20,000 ([D101 of part B of 0.8.1](../design/0.8.1-b.md#d101-the-best-detail-section-then-the-tier-then-the-rest)). A request with no document target has no entry. Thus the tier comes after the rules and after each citing section in the packing order, and these sections fill the budget.

ADR 0028 has 13 characters of room under its hard budget of 14,000. On 2026-11-27 the owner chose a new ADR for the new rules. Thus ADR 0028 keeps its text, and the budgets do not change.

## Decision Outcome

The rules of the section "Amends" apply to a request that has a code path and no document target, when the chain of the tier is not empty. A request with a document target, also with `--path`, keeps the rules of ADR 0028 and of its amendment of 2026-11-24. A tree with no roll-up document has no tier, and its output does not change.

### Rejected

- A tier after the rules: with 40 rule sections, the share is 39 characters, and the tier serves nothing.
- No floor: the same result.
- A floor of a flat quarter of the budget: it takes more room of the rules than the title sections need.
- A tier before the best detail section: the agent can lose the requirement of the file that it edits.

### Consequences

- Good: a project with summaries gets the summary of the area of a file, also with many rules.
- Bad: the tier takes room of the citing sections after the best one. At a small budget, the rules lose the room of the best detail section and of the floor. Each section that is not in full stays listed while the map has room.
- Neutral: the output order, the form of the text and the form of `--json` do not change.

## Amends

ADR 0028 is accepted, so this ADR does not edit it. These rules apply in place of the section "`context`" of ADR 0028 and of its amendment of 2026-11-24, for the requests of the section "Decision Outcome":

- The output order does not change. In the packing order, the tier comes after the best detail section and before all other sections, the first tier included. The best detail section is the first section after the first tier that `context` can serve in full. Usually it is the best section that cites the path.
- The share of the tier is at most a half of the budget, and it never takes the room of the best detail section. In these limits, the share is the room that the first tier and the best detail section leave, but it is never less than the floor.
- The floor is the room of the title sections of the chain: the first title section of the fill order in full, and one stub line for each other title section. It is at most a quarter of the budget. If the first title section is larger than a quarter of the budget, the floor is the stub lines of the title sections only. Thus a project with many rules still gets the summary of an area. Its rules lose at most the room of the best detail section and of the floor.
- The fill order of the amendment of 2026-11-24 of ADR 0028 does not change.

## Amendments
