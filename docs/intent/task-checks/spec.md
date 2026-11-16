---
status: draft
---
# Task checks and the parse of a change: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the codes SD923, SD924 and SD925, the key `[change] max-task-files`, a new item form of `Interfaces`, and new parsed data. The rules are in [the design](../../design/0.7.0.md), D28, D40 and section 15, and the docs of the build state them.

## Interfaces

None. No interface document describes the parser, the codes or the templates.

## Decisions

- [[adr/0010-workflow-pack]]
- [[adr/0015-limits-of-0-6-1]]
- [[adr/0017-profiles]]

## Concerns

- [x] The paragraph under `Files` is a new field of the parsed data, and the cache stores the parsed data. Resolved: `PARSER_VERSION` is 14 and the cache `FORMAT` is 2 (D28). The change set "Cache layout" applies, and `architect` approves it.
- [x] SD923, SD924 and SD925 are new warnings in `standard` and new errors in `strict`. Resolved: a warning never fails `check`, and the gate reports only a new error (D41).
- [x] The H1 placeholder gives SD306 for each new document of a change until the author writes a title. Resolved: the templates already have placeholders, and SD306 is the floor of the skill `brainstorm`.
- [x] An `ADD` item of `Interfaces` names a document that does not exist yet. Resolved: when `finish` runs, the document must exist, or a delta line must add it. Else SD914 stops `finish`.

## Open questions

- [x] Which text states "nothing"? Answer: the form of D28 applies: the first word is `none`, then `:`, `.` or the end. It applies to the paragraph under `Files`. It also applies to the single item of `Interfaces`, `Concerns` and `Open questions`, with or without a task box. A list item under `Files` keeps the rule of 0.6.1.
- [x] Which line does `next` show first when a plan has a paragraph under `Files` that is not `none`? Answer: the line of D28 comes first. If the plan is not `approved`, the approval line follows it and stays the last line. If the plan is `approved`, the line of D28 is the last line, in the place of the line "it lists none".
- [x] What does an `ADD` item of `Interfaces` give when the diff deletes the folder? Answer: SD914 when the head has no such document and no delta line adds it. It gives nothing when the head has the document.
- [x] Does SD923 or SD924 read a draft plan? Answer: no. They read an `approved` plan only (D40). SD925 reads a spec in each status, except the spec of a `closed` change.
- [x] What is a glob or a folder in the label `Files:` of a task (SD923)? Answer: a path with `*`, `?` or `[`, a path that ends with `/`, or a path that names a folder of the tree.
- [x] What are the severities? Answer: SD923, SD924 and SD925 are in the rule group `changes`: off in `lite`, a warning in `standard` and an error in `strict`.
