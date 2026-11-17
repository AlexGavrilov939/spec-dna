---
status: draft
class: higher-risk
---
# Feedback of a second project: SD913, a delta after a spike, a partial tag and the plan budget

## Intent

Build items 8, 10, 11 and 12 of the feedback of a second project that uses 0.6.1. A team then gets no false SD913 during `clarify`, a path for a delta that waits for a spike, a way to mark partial evidence, and no budget diagnostic on a long plan.

- Item 8: SD913 runs only when a change of the diff has a `plan.md`. Before that, an edit of the questions, the roadmap or a spike gives no SD913.
- Item 10: no new syntax. `docs/guides/change.md` gives the steps for a delta that depends on a spike. Guard tests pin them.
- Item 11: the tag `covers: partial <doc-id>#<key>`. Alone, it does not clear SD608, SD916 or the stage `test`. `show`, `next`, `status` and `stats --sdlc` show it.
- Item 12: the type `plan` has no default budget. `[budgets.plan]` sets one.

## Why now

These items are false alarms or a wrong signal in daily use of 0.6.1. The small tasks of 0.7.0 (D40) make each plan longer, so SD701 and SD702 fire more often on a plan.

## Out of scope

- A marker for a delta line that waits for a spike, and a change of step 3 of `finish`.
- A partial form of `verified-by`.
- A budget change for `intent` or `spec`.
- The text of the skills and of the line of `AGENTS.md`, because the eval run freezes it.
- The "not needed" state of a spike task: item 9 is another change.
