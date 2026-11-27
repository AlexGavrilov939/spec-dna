---
status: proposed
---

# 0034. The identity of a test case

## Context and Problem Statement

[ADR 0024](0024-test-report-as-evidence.md) binds a `covers:` tag to the cases of a test report. Its amendment of 2026-11-25 makes the rule one invariant: a bind is silent only when each case has the identity that the scanner builds from the test under the tag, and a case leaves the set only by a field that the report and that identity both hold in full. The check of part A of 0.8.1 left 3 holes open, and reports of real runners showed 2 more:

- B2: a `classname` of one word read as another file, so the failed case of the test left the set, and a passed case of another test proved the tag. gotestsum `relative` writes `.` for the root of each module, so a tag of one module bound the case of another module with no SD621.
- B3: a failed case of another language, whose `classname` names no file of the code tree, failed a tag whose own case passed.
- B4: 2,000 tags on one title took 171 s.
- jest-junit writes a space before the title of a test outside a group, and Vitest can write a title as the `classname`. The own case then left the set by its name.

ADR 0024 has 9 characters of room under the hard budget of a decision. On 2026-11-27 the owner decided that this new ADR holds the rules, and that ADR 0024 and the budgets stay as they are. How does a case of the test under a tag never leave its set, while a case of another test never fails it?

## Decision Outcome

The identity of a case is its locator and its head. The head is the name without the row tail. The locator is the first of these that the case has: the `file`, a `classname` that is a path, a binary id with `::`, a folder path, a dotted name of 2 or more segments, and a word. A `classname` with a space, or equal to the name, is a title and no locator. The identity takes the `file` and the `classname` of the test case itself, and the `classname` is the suite of the case. spec-dna does not read the `name` of the `testsuite` element (answer of the owner of 2026-11-27). The rule:

- A full locator (each kind but the word) that names no reading of the tag file makes the case another test, also in a pair that no row lists. It does not need to name another file of the code tree. Where the row of the tag ties the case by the built identity (nextest, Go, and a class chain of the JVM or .NET), the fields of the row decide, as in part A.
- A word is relative to a root that the report does not give. It never makes a case another test by the step "another file". In the row of the tag, it ties the case only when its readings name one file or folder of the code tree, and it makes the case another test only when no reading names the tag file. `.` is the root of each Go module of the code tree.
- The built name of a call has the forms that the runners write with their defaults: a space before the title of a test outside a group, and the 3 joins of the titles. A `classname` that equals a title reading of the test is a title.
- A failed case never proves a requirement, in any profile. A case leaves the set only by one verdict of a closed list, and for each documented shape a runner writes the case of the test under the tag so that no verdict applies. The design gives the list and the proof.

The work of a tag does not grow with the cases of another file or another full name: 2,000 tags on one title add at most 1 s to a check. The detail is in [the design of part B](../design/0.8.1-b-binding.md).

## Consequences

- Good: a failed or skipped case of the test always counts, also through a word or a template of the runner.
- Good: a case of another language with a full locator never fails a passed test.
- Bad: a case of a runner that no row lists, whose full locator names no reading of the tag file, no longer proves a tag. The tag gets `no-case`, and `case:` binds it as a weak link.
- Bad: gotestsum `relative` and `short` give SD621 where part A was silent, when 2 Go modules share `.` or the name of a folder.
- Neutral: no new code, reason, key or JSON field.

## Amends

ADR 0024 is accepted, and its amendments name the design of part A. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0024, the amendment of 2026-11-25, "the rule of the identity": the step of another file and the row of a case read the locator of this ADR. A word and a title never make a case another test by the step of another file.
- ADR 0024, "The proof and its states", the sentence "A requirement passed when at least one bound case passed and no bound case failed": a bound case is each case that no verdict of the closed list leaves out.
