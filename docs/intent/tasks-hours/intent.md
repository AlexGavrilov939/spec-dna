---
status: draft
class: higher-risk
---
# Hours and a "not needed" state for each task, and a long version in the line

## Intent

Build items 9 and 6 of part B of 0.7.0, as the section 15 bullets of [the design](../../design/0.7.0.md) and [ADR 0020](../../adr/0020-hours-and-state-of-a-task.md) state them.

- Item 9: a task of `Order` can have the label `Hours: <n>` or `Hours: <a>-<b>`. These labels are the one source of the hours of a change. `finish` adds the hours of the tasks and prints `Hours: <low>` or `Hours: <low>-<high>`. The section `## Hours` of slice 3b is not read, and `PARSER_VERSION` is 15.
- Item 9: a task with the label `Skip: <reason>` is not needed. `next`, `status` and the counts treat it as closed, and `finish` leaves its hours out. SD923 reports a `Skip:` with no reason.
- Item 6: the line of `AGENTS.md` has no space before its marker, so a version up to `99.99.99` fits in 400 bytes. The text of the line does not change.

## Why now

A second project plans the branches of a spike as tasks, and it records the hours of each task. With the section `## Hours`, the hours and the tasks are in 2 places. The full eval run measures the text of `write-plan` and the line of `AGENTS.md`, so both must be final before the run.

## Out of scope

- A stop of `finish` for an open task. `finish` reads no task state today.
- A check of the value of `Hours:`. The note of `finish` is the only report.
- A partial form of `covers:` (item 11) and the edit of a delta line after a spike (item 10).
- A change of the text of the line, of `approval_sentence!` or of the bodies of `execute`, `tdd`, `verify`, `debug` and `finish`.
