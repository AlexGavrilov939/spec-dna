---
status: approved
---
# Hours and a "not needed" state for each task, and a long version in the line: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the task grammar of a plan, the output of `next`, `status`, `finish`, `log` and `stats --sdlc`, the problems of SD923, the cache data and the generated line. The section 15 bullets of [the design](../../design/0.7.0.md) state the rules, and the docs of the build give them.

## Interfaces

None. No interface document describes the CLI or a plan. The surface changes in this way.

A task has the form `- [ ] <id>: <text>. Files: <paths>. Verify: <check>. Hours: <hours>. Skip: <reason>.` The labels `Hours:` and `Skip:` follow the rules of `Files:` and `Verify:`. The value of `Hours:` is `<n>` (up to 4 digits, up to 2 decimals, an optional `h`) or `<a>-<b>` with `a <= b`. Another value is invalid.

A task with a non-empty `Skip:` is skipped: it is closed, whatever its box. A done task has `- [x]` and no `Skip:`. `next` names the first task that is not closed. The `next` line of the tasks is ``tasks of `Order`: <done> of <total> done, <skipped> not needed`` when a task is skipped, and as before when none is. `status` adds the token `skipped <n>` after `tasks <done>/<total>` and before `next-task`, and `status --json` adds `tasks.skipped`.

`finish` adds the hours of each task that is not a duplicate and not skipped, as a range. It prints `Hours: <low>` or `Hours: <low>-<high>`, a note for each invalid value (and then no total), and one note for the tasks with no `Hours:`. `hours` of `finish --json` is `null` or `{"low", "high"}`. `log` reads `Hours: <n>` and `Hours: <a>-<b>` in a commit message: `hours 5-9` in the text, `{"low", "high"}` in `--json`. `stats --sdlc` counts a range as its midpoint.

SD923 adds the problem "its `Skip:` has no reason". The generated line of `AGENTS.md` and `CLAUDE.md` is `<text><!-- spec-dna v<version> h:<hash> -->`, with no space before `<!--`.

## Decisions

- [[adr/0006-change-lifecycle]]
- [[adr/0001-information-model]]
- [[adr/0003-distribution]]

## Concerns

- [x] A new label `Hours:` ends the text of `Verify:`. Resolved: no task line of the tests, the fixtures or the plans of this repository has a plain `Hours:` or `Skip:`, and a code span does not hold a label.
- [x] A new box such as `- [-]` makes the item no task, for `pulldown-cmark` and for a 0.6.1 binary. Resolved: the state is a label, so the box and the parser stay. The guide writes `- [x]`.
- [x] `finish-1` and `finish-3` check `^Hours: 4\.5` in the commit of a real agent. Resolved: their fixtures move the hours to the tasks with the same total, and `finish-3` gets a skipped task whose hours do not count.
- [x] The line of `AGENTS.md` is in each eval run. Resolved: only the space before the marker goes. The text and its hash stay, and `find_line` of 0.6.1 reads both forms.
- [x] A commit that adds `Skip:` to an approved plan could need a new approval in the branch. Resolved: it adds no path to `Files`, so reason 8 of SD910 does not apply. A test fixes it.

## Open questions

- [x] Which source of hours wins when a plan has both? Answer: only the labels. The section `## Hours` is plain text, and nothing reads it. No release had the section.
- [x] What does `done` mean in `status --json`? Answer: the tasks with `- [x]` and no `Skip:`. The new key `skipped` counts the skipped tasks. A 0.6 plan has no `Skip:`, so its counts do not change.
- [x] What is the margin of the line? Answer: 0 bytes at `99.99.99` and 3 bytes at `0.7.0`. A test with the version `99.99.99` keeps it.
