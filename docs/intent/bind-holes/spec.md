---
status: approved
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

- [x] The invariant. "A failed case never proves a requirement" holds only when each verdict of the closed list of section 1 of the design is decisive. Resolved: the design keeps the closed list of 5 verdicts, and rule 1 does not apply where the row of the tag ties the case by the built identity. t3 adds the cells B2, B3 and E2c, which fail first. t4 adds the cells of A6: no cell makes the own case another test. t2 runs A1 and A2 on each real report, and A2 runs on each cell of the matrix.
- [x] The prediction of section 1 of the design. Resolved: t2 records the table from the binary of the unchanged tree, then applies the 15 changed binds of the 9 rows of the table of section 1. A bind of t4 to t6 that differs from the table stops the task. `architect` then decides the row, and t22 records it in the design page as a deviation. No task edits a row of the table only to make a test pass.
- [x] Current tests that pin the old rule. Resolved: the task that changes a rule (t4, t5 or t6) gives the new result to each current test that pins the old rule: a word that reads as another file (limit 10), and a dotted name of another language that stays a weak link (limits 12 and 16). The commit of the task names each such test with its reason, and t22 lists them. A test that fails for another reason stops the task, and `failure-analyst` finds the cause before a fix.
- [x] The fast binder and the reference binder. Resolved: t7 adds the reference binder and the test of the same result before any change of the read. t8 and t9 are done only when that test passes on each cell of the matrix, on each real report and on L8 to L10 with 60,000 cases. A difference is fixed in the fast binder, never in the reference or in the expected result.
- [x] The level of SD621 in `strict`. Resolved: one function gives the level of each line of SD621. t14 tells a rule for SD621 or for the group `reports` from the default of the profile, t15 gives each weak case the flag fixable, t17 makes `check` read the function, and t18 makes step 5.4 of `finish` read the same function. A rule gives one level to each line (the unit test of t14, and t16).
- [x] The fixtures. Resolved: t1 copies the 72 files with no edit, and it checks them with `diff -r`, the count of the files, and a search for the paths, the user, the host and the clock values of a machine. The owner accepted the `.editorconfig` of the folder on 2026-11-27, so `pre-commit run --all-files` passes on the reports. `generate.sh` stays in the folder, and it fails on a report that holds the machine. The property `go.version` of the Go reports stays (section "Risks" of the plan).
- [x] The target of B4 on a shared machine. Its load average is often 20 to 80, so the time of one run is no proof. Resolved: the target stays: 2,000 tags on one title add at most 1 s. t19 measures in pairs. In each of 5 rounds, it runs the new binary with no tag and with 2,000 tags, one run after the other, in an order that changes from round to round. Each run records the wall time, the CPU time (user and system, from `wait4`) and the load average before and after it. The target holds when the median over the 5 rounds of the difference of the CPU time is at most 1 s, because the load of other processes does not add to the CPU time of a process. A round whose load average stays below 2 must also hold the target in wall time. The base binary runs one round of each load with 2,000 tags, with a limit of 600 s for each run, as the comparison. The counts of the judgments of t7 to t9 are the proof that no load changes.

## Open questions

- [x] Q1 of the design: the place of the new rules. Answer of the owner (2026-11-27, the recommended option): 2 new ADRs. ADR 0034 on the bind of a test amends ADR 0024, and the budgets do not change.
- [x] Q2 of the design: the second part of the identity of a case. Answer of the owner (2026-11-27, the recommended option): the attributes `file` and `classname` of the test case itself. The `name` of the `testsuite` element is not read.
- [x] Q3 of the design: the `.editorconfig` of `tests/fixtures/runners/`, which turns off the indent check for `*.xml` only. Answer of the owner (2026-11-27, the recommended option): add it.
