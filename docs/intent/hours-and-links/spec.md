---
status: draft
---
# Hours of a change and links into a change: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the code SD612, the line `Hours:`, 3 JSON fields and a row of `stats --sdlc`, and it changes the info line of a locked link. The rules are in section 15 of [the design](../../design/0.7.0.md), and the docs of the build state them.

## Interfaces

None. No interface document describes `finish`, `log`, `stats` or the output of `check`.

## Decisions

- [[adr/0001-information-model]]
- [[adr/0006-change-lifecycle]]

## Concerns

- [x] After a squash merge, the parent tree of the commit that deletes the folder has the plan of the earlier pull request, with no `Hours`. Resolved: `log` reads the commit message, not the tree. The lab of session 25 proved the parent tree has no `Hours` section.
- [x] A host can write the commit messages of a squash as a list, so the line is not always in the last paragraph. Resolved: `log` reads the last line of the whole message that matches `Hours: <number>`, in any paragraph. With no such line, the value is absent.
- [x] ADR 0012 rejects a trailer `Approved-by:`, because any author can write it. Resolved: the line `Hours:` is a record, and nothing gates on it.
- [x] One link can give SD612 and also SD611 or SD901. Resolved: SD612 does not report a reference that SD611 or SD901 reports in the same run.
- [x] The pointer of a locked link needs the history, and `check --json` always has `locked_links`. Resolved: `check` reads the history only when a dead locked link names the folder of a change. With no such commit (a shallow clone, or a deletion with no commit), the pointer keeps the text of 0.6.1.

## Open questions

- [x] What is the form of an item of `Hours`? Answer: a number of hours (up to 4 digits, and up to 2 decimals after a `.`), an optional `h`, then the end of the item, or `:` or a space and free text. An item with another form gives a note of `finish` and no total.
- [x] What does `stats --sdlc` show? Answer: one row, play `Build`, kind `lagging`, name "Hours of work per finished change", unit `hours`. The value is the median of the finished changes that have a line, and the count is their number. The note gives the total.
- [x] What clears the info line of a locked link? Answer: an amendment section of the same decision with a code span or a prose word that equals the target as written (without its fragment), or the id or the path of the place that is gone. For a target in the folder of a change, the slug of the change in a code span also clears it.
- [x] What are the severities of SD612? Answer: a warning in each profile. It is in no rule group, as SD611, and `[rules] SD612` changes it.
