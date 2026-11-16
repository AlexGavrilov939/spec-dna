---
status: draft
class: higher-risk
---
# Task checks and the parse of a change

## Intent

Make the tasks of a plan and the sections of a spec machine-checked, as slice 3 of [the design](../../design/0.7.0.md) says (D28, D40 and section 15). An agent then gets small tasks with a check, and a spec that states each open item.

The change adds these items:

- D28: the parser keeps the first paragraph of `Files`. A paragraph that starts with `none` states "no files". `PARSER_VERSION` is 14.
- SD923 (`task-incomplete`) and SD924 (`task-too-large`), with the key `[change] max-task-files`.
- SD925 (`checkbox-missing`): an item of `Concerns` or `Open questions` with no task box. The spec template writes a box.
- The `none` rule for `Interfaces`, `Concerns` and `Open questions` (item 3 and item 4a of the feedback of a second project).
- The item form `ADD <interface id>: <text>` of `Interfaces` for a new interface (item 3).
- The H1 placeholder of `new intent`, `new spec` and `new plan` (item 4b).
- The parser part of item 5: the items of a section `Hours`, in the same `PARSER_VERSION`.

## Why now

The skills of slice 4 (`workflow-pack`) name SD923, SD924 and SD925 as their floor, so these checks come first. One change of `PARSER_VERSION` makes each project rebuild its cache once.

## Out of scope

- The commands that read `Hours`, SD612 and the locked links (slice 3b, `hours-and-links`).
- The approval in the branch (slice 2, `branch-approval`).
- A list item under `Files`: it keeps the rule of 0.6.1.
