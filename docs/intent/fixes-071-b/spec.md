---
status: approved
---
# Fixes of 0.7.1, part B: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the scope of the diagnostics SD503, SD913, SD923 and SD924, the behavior of `finish`, `log`, `next`, `status`, `upgrade` and `check --base`, one config key, and the text of 4 skills. The [design](../../design/0.7.1-b.md) states each contract, and the docs of the build state them.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0019-limits-of-0-7-0-part-a]]
- [[adr/0020-hours-and-state-of-a-task]]
- [[adr/0023-default-skills-from-the-release-table]]

## Concerns

- [ ] The rule of B1 needs the slug. A real task that names `spec-dna finish <slug>` closes with `finish`. The skill tells agents to write no such task.
- [ ] B3 lets a pin move share a pull request with other work. The pin stays exact, and a binary of an older version stops with exit code 3.
- [ ] B5 reads git history. A shallow clone finds no commit, and SD923 reports each task there.
- [ ] The text of `write-plan`, `specify`, `execute` and `finish` changes, so the verdicts of the release table for them describe the older text. Only `write-plan` has a new run.
- [ ] The line `Hours: <estimate>, actual <n>` is unreadable for `log` of 0.7.0.

## Open questions

- [x] Does the review task of the owner stay the default? Answer: no. It is optional (design B1), and this change has none.
- [x] Does the text of `execute` have 1 sentence? Answer: it has 2. The trap of `execute-6` needs the step "ask the person" (design B11).
