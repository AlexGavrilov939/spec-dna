---
status: accepted
---

# 0020. The hours and the state of a task

## Context and Problem Statement

[ADR 0006](0006-change-lifecycle.md) gives the steps of a change in `Order` of `plan.md`. Slice 3b of 0.7.0 added the section `## Hours` to a plan, and `finish` prints its total as one line for the commit message. No release has the section. The feedback of a second project asks for 2 more things:

- The hours of each task, in the form `Hours: a-b`.
- A state "not needed" for the branches of a spike, which `next`, `status` and `finish` treat as closed, with a reason.

Constraints:

- The task line already has labels (`Files:`, `Verify:`) with a parser, and `execute` ticks the task on the same line.
- `pulldown-cmark` knows only the task boxes `[ ]`, `[x]` and `[X]`. Any other box makes the item "not a task", also for a 0.6.1 binary.
- ADR 0006 is at its hard budget. Thus this ADR holds the new rules.

## Considered Options

1. Keep the section `## Hours` as the one source of the hours.
2. Read the section and the labels, and add them.
3. A label `Hours:` on each task as the one source of the hours, and a label `Skip:` for a task that is not needed.
4. A new task box, such as `- [-]` or `- [~]`, for a task that is not needed.

## Decision Outcome

Option 3.

- A task of `Order` can have the labels `Hours: <n>` or `Hours: <a>-<b>`, and `Skip: <reason>`. They follow the rules of `Files:` and `Verify:`: the labels are case sensitive, the first instance of a label counts, and each label ends at the next label. The order of the labels is free.
- `<n>` is 1 to 4 digits, then optionally `.` and 1 or 2 digits, then optionally `h`. In `<a>-<b>`, `a` is not more than `b`. Another value is invalid.
- The labels are the one source of the hours of a change. The section `## Hours` is not read. `PARSER_VERSION` is 15, because the parser no longer keeps the items of that section.
- `finish` adds the hours of each task that is not skipped. Done tasks and open tasks both count. A number `n` counts as the range `n-n`. `finish` prints `Hours: <low>` or `Hours: <low>-<high>`. A task with no `Hours:` gives a note, and the total leaves it out. An invalid value gives a note and no total. `hours` in `finish --json` and `log --json` is `null` or `{"low", "high"}`. `stats --sdlc` counts a range as its midpoint.
- A task with a non-empty `Skip:` is not needed. `next`, `status` and the counts treat it as closed, and its box does not matter. The guide and the skill write `- [x]`, so a 0.6.1 binary and a Markdown viewer also read the task as closed. `status --json` has `tasks.skipped`.
- SD923 reports an empty `Skip:`. A skipped task still needs `Files:` and `Verify:`, and its paths stay in the scope of SD913. A commit that adds `Skip:` or `Hours:` to an approved plan adds no path, so it needs no new approval.
- [Section 15 of the design](../design/0.7.0.md#15-deviations-during-the-build) gives the detail.

### Rejected

- Option 1: it does not give the hours of each task.
- Option 2: 2 places for one number, so a number can count 2 times. A rule that lets the labels win is a hidden rule.
- Option 4: a 0.6.1 binary and SD923 lose the task, and the cache needs new data.
- A number only in `Hours:`: it refuses the range that the request names.
- 2 labels, `Estimate:` and `Hours:`: more syntax, and `finish` needs a rule for which one counts.
- Delete a task that is not needed: the plan loses the record of the branch and its reason.

### Consequences

- Good: the hours have one source, on the line that `execute` ticks.
- Good: a spike keeps a record of each branch and of its reason.
- Bad: the time of the plan or of a review has no task. The author writes a task for it.
- Bad: the cache is built again once (`PARSER_VERSION` 15).
- Neutral: a plan that still has `## Hours` gives no diagnostic. Nothing reads the section.

## Amends

This ADR amends no locked text. ADR 0006 stays as it is: `Order` in `plan.md` gives the steps. This ADR adds 2 labels to a task of `Order`. The labels replace the section `Hours` of slice 3b (design 0.7.0, section 15) as the source of the hours.

## Amendments
