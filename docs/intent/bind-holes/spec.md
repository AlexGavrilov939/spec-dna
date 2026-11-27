---
status: draft
---
# A failed test case never proves a requirement, a case of another language never fails a test, and 2,000 tags on one title add at most 1 s: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the rule of the bind of a `covers:` tag, the work of the bind, the class index, the parse of `case:`, the level of SD621 in `strict` and the hint text of SD621. It adds the fixtures of real runners, their tests and a bench. The [design of part B](../../design/0.8.1-b-binding.md) states the contract. The amendment of [ADR 0017](../../adr/0017-profiles.md) of 2026-11-25 records the level of SD621, and [ADR 0034](../../adr/0034-the-identity-of-a-test-case.md) records the identity of a case.

## Interfaces

- None: no interface document describes the bind.

## Decisions

- [[adr/0024-test-report-as-evidence]]
- [[adr/0017-profiles]]
- [[adr/0034-the-identity-of-a-test-case]]

## Concerns

- [ ] The invariant. "A failed case never proves a requirement" holds only when each verdict of the closed list of section 1 of the design is decisive. Assertion A2 runs on each cell of the matrix and on each real report, and the named cells B2, B3 and E2c fail first.
- [ ] The prediction of section 1 of the design. The table of the expected binds of the real reports is the table of the lab run of 475d8a8 with the 15 changed binds of the 9 rows of its table. A row of the build that differs stops the build, and `architect` decides before the table changes.
- [ ] Current tests that pin the old rule. The build lists each test that fails after the rule of section 1, with the reason. A test that pins a fault (a word that reads as another file, a dotted name of another language that stays a weak link) gets the new result, and the plan records it in `Deviations`. A test that fails for another reason stops the build.
- [ ] The fast binder and the reference binder. They give the same `Proof` and `Bind` on each cell of the matrix, on each real report and on the 3 scaled loads. A difference is a fault of the fast binder, not of the reference.
- [ ] The level of SD621 in `strict`. The level is now that of each line. `finish` (step 5.4) and `check` read the same function, and an explicit rule for SD621 or for the group `reports` keeps one level for each line.
- [ ] The fixtures. The 24 reports and the sources come from the work tree of the agent that made them, with no edit. `pre-commit run --all-files` passes on them with the `.editorconfig` of the folder. No report holds a path, a user or a host of a machine (`generate.sh` checks it, and the first task checks it again).
- [ ] The target of B4. The bench measures it on a machine with a load average below 2. A result on a loaded machine is no result: the bench writes the load, and the build reports it.

## Open questions

- [x] Q1 of the design: the place of the new rules. Answer of the owner (2026-11-27, the recommended option): 2 new ADRs. ADR 0034 on the bind of a test amends ADR 0024, and the budgets do not change.
- [x] Q2 of the design: the second part of the identity of a case. Answer of the owner (2026-11-27, the recommended option): the attributes `file` and `classname` of the test case itself. The `name` of the `testsuite` element is not read.
- [x] Q3 of the design: the `.editorconfig` of `tests/fixtures/runners/`, which turns off the indent check for `*.xml` only. Answer of the owner (2026-11-27, the recommended option): add it.
