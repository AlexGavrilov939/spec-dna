---
status: proposed
---

# 0024. The test report as evidence, and the proof levels

## Context and Problem Statement

A `covers:` tag links a test to a requirement. `check` and `finish` read the tag, and they never see if the test ran or passed ([ADR 0001](0001-information-model.md): "spec-dna still does not claim that tests pass"). A skipped test or a failed test clears SD608 and SD916. A test that passes once can be flaky. A requirement whose proof is owed for a known reason looks the same as a requirement with no proof. So an author adds a false tag to clear SD608.

How does a requirement count as proven only when its test ran and passed, with a deterministic core and no network in `check`?

Constraints:

- `check` has no network and no model. The same input gives the same result on each machine.
- No new dependency without the owner. A JUnit XML reader is the one candidate.
- An absent key keeps the behavior of 0.7.1 in each profile. A current corpus passes `check` with no edit.
- SD608 and SD916 keep their meaning: no tag and no `verified-by`.
- The core has no flaky-test machinery: no retry, no quarantine and no statistics.

Evidence ([design 0.8.0](../design/0.8.0.md#1-evidence-and-sources)):

- A tag is a path, a line and an id (F1). Only the markers `aliases` and `verified-by` leave the hash of a requirement (F3).
- pytest writes the attribute `file` on a test case only with `junit_family = xunit1`. Many runners write only `classname` and `name` (F20). A bind by file alone fails for them.
- `git ls-files -s -z` gives the object id of each tracked file at no cost of a file read (F5).

## Considered Options

1. Keep the tag as the only evidence.
2. spec-dna runs the tests itself.
3. A test report file, bound to the code tree by a stamp, read by `finish` and by `check --reports`.
4. Committed reports, read from the git tree.

For "passes reliably":

- (a) N passes of each bound case in the reports of one tree.
- (b) N archived acceptance records on different trees that list the requirement as passed.
- (c) A ledger file in the repository, or a cache of CI.
- (d) Git notes.

## Decision Outcome

Option 3, and option (a) for "passes reliably". The design page gives the detail in [D1](../design/0.8.0.md#d1-the-test-report-as-evidence) and [D2](../design/0.8.0.md#d2-the-proof-levels-and-the-proof-debt).

### The code tree

- The input is each entry of the git index. A tracked file that differs from the index gives the mode and the object id of the file on disk. An untracked file does not count.
- The code paths exclude each path that `[evidence] tree-exclude` matches (default: each Markdown file). They also exclude the folders of the `intent` layer, `.spec-dna/`, the tool files of spec-dna, and the reports with their stamps. Thus a change of documents never changes the tree.
- The hash covers `spec-dna-tree 1`, then one line `<mode> <object id>\t<path>` for each entry in the byte order of the paths. It is the first 8 bytes of SHA-256, as 16 hexadecimal digits: the form of the pinned hash of ADR 0001. A new compare function serves the tree, because `hash_matches` compares a requirement.
- A report binds to all code paths, because a test runs on the whole tree. [ADR 0025](0025-independent-acceptance.md) uses the same definition for the record.

### The report

- `[evidence] reports` lists globs of report files. spec-dna reads them from the disk, even when git ignores them.
- The list of formats is closed: `junit` (a subset of JUnit XML) and `lines`. The `lines` format is `spec-dna-report 1`, then one line for each case: `<pass|fail|skip>`, a tab, a path or `-`, a tab and the case name. Any runner can write the line format with a small script. A new format is a new reader and one more name.
- The XML reader is own code. It reads UTF-8 only, and it rejects a DTD, so it never loads an external entity.
- An unknown format, a parse error, a `testsuite` whose `tests` count differs from its `testcase` elements (a cut file), a file over 64 MiB, or a report or stamp that git tracks gives SD615. A committed report can be a hand-written one, so the gate refuses option 4.

### The stamp

- `spec-dna evidence stamp` writes `<report>.stamp` with the tree hash and the hash of the report bytes. CI runs it after the tests in the same job, so the stamp is exact.
- A stamp of another tree, or a report whose bytes changed, gives SD614. No report or no stamp gives SD613.

### The bind of a tag

- An explicit name follows the ids: `case:<name>` or `case:"<name>"`. A binary of 0.7.1 ends the tag at that word, so it reads the same ids.
- Else the tag binds to the first word or string literal of the first code block after the tag, when it names a case of the reports. The block ends before the first later line whose indent is not deeper than its first line, after a deeper line. It also ends before a line with less indent than the tag, at the next tag, or after 20 lines. Thus, in the usual layout, the tag of a test that did not run does not bind to the next test. A name matches when it equals the word, ends with it after a separator, or starts with it and `/`, after the removal of a final `[...]` or `(...)`.
- A case with a file binds only a tag of that file. A case with no file binds only a tag of the test file whose dotted path ends with its `classname`, or with the part before the last `.` (the form of pytest). When no file has such a path, it binds a tag of any test file.
- The limits: a test in a comment after the tag, one case name in 2 files with no `file` and no such `classname`, and a test on one line. `case:` removes each doubt.
- A tag with no case is not bound. It proves nothing, and the message names the fix.

### The proof and its states

- A requirement passed when at least one bound case passed and no bound case failed. A skipped case is no pass. A pass after a rerun is a fail.
- The states, from best to worst: `reliable`, `passed-once`, `verified` (`verified-by`), `tagged`, `owed`, `none`.
- `reliable`: each bound case that passed has at least `[evidence] reliable-runs` passes (default 3) in the stamped reports of the current tree, with no fail. The runner repeats the tests. `[evidence] reliable-for` (`none`, `higher-risk`, `all`) selects the changes that need it at `finish` (SD620). The level is a level of the test report, so SD620 runs only with `reports` set.
- `verified-by` keeps its meaning, as a check by a person. The report does not apply to it.

### Where the report counts

- `finish` reads the reports when `[evidence] reports` is set (step 5).
- `check` reads them only with `--reports`. `init` writes no flag, because its CI job runs no tests. The change guide gives the steps for the job that runs the tests: the tests, `spec-dna evidence stamp`, and `check --reports`. The hook `check --staged` keeps the rule of 0.7.1, so a commit needs no fresh report.
- New codes carry the new facts: SD613 to SD615 for the report, SD616 (an active requirement, a state code) and SD617 (a requirement of a change) for a tag with no passed case.

### The proof debt

- A requirement can end with `<!-- proof-owed: <reason> -->`. The reasons are a closed list: `manual`, `environment`, `external`, `pending`.
- The hash of a requirement does not cover the marker. `PARSER_VERSION` goes to 17.
- A debt never counts as proof. SD916, SD617 and SD620 stop `finish` as before. For an active requirement with no evidence, SD618 replaces SD608. A bad reason, or a debt on a requirement with evidence, gives SD619.
- `status` and `stats` show the count of debts and their reasons.

### The profiles

On 2026-11-19 the owner decided the profiles of 0.8.0. In `lite`, each new code of this ADR is off. In `standard` and `strict`, the test reports are on only by `[evidence] reports`, and the reliable level only by `[evidence] reliable-for`. In `strict`, `init` writes `reliable-for = "higher-risk"`, and the path of `reports` only for a runner that it detects. The proof debt has no key, because a marker on one requirement is the switch. In `standard` its codes are warnings, so a marker never stops a change there.

### Rejected

- Option 1: a skipped test proves a requirement, and a false tag costs nothing.
- Option 2: a test runner for each stack. The core becomes a build tool, and a check depends on the machine and the clock.
- Option 4: a report in git changes with each run and gives a diff in each pull request. A stale committed report looks fresh.
- A bind by file only: many runners write no file. A bind by a language parser: one parser for each language.
- A wrapper command that runs the tests and stamps: more code (signals, exit codes, Windows). Its only gain is a local edit between the run and the stamp, and the acceptor runs the tests again.
- Option (b): a requirement that no change touches stays at `passed-once`, and different trees mix in code change. Repetition on one tree is the definition of a flaky test.
- Options (c) and (d): a committed ledger gives conflicts (ADR 0001, "Concurrency"), a cache of CI is not in a fresh checkout, and git does not fetch notes by default.
- A date or an expiry on a debt: a check reads no clock ([ADR 0006](0006-change-lifecycle.md)).
- A crate for XML: a new dependency for a small subset.

### Consequences

- Good: with the key set, a skipped or failed test no longer proves a requirement, and `finish` stops.
- Good: one tree definition serves the report and the acceptance record.
- Good: an honest debt costs less than a false tag, and `status` shows it.
- Bad: `check --reports` reads files outside git. A person or an agent can write a report by hand. The stamp binds bytes to a tree, not to a run.
- Bad: a tag is still a claim that the test checks the requirement.
- Bad: the bind by name can miss a case. The tag then proves nothing until the author adds `case:`.
- Bad: `reliable` costs N runs of the tests.
- Neutral: with no `reports` key, nothing changes.

## Amends

ADR 0001 and ADR 0006 are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0001, "Layers", the sentences "A gate MUST run from the repository with only spec-dna and git" and "Every gate needs only two trees": `finish`, and `check` with `--reports`, also read the report files of `[evidence] reports` from the disk. Without the key or the flag, the rule stays.
- ADR 0001, the amendment "profiles, knowledge types and evidence (0.2.0)", the item "Evidence", the sentence "spec-dna still does not claim that tests pass": with `[evidence] reports`, a tag proves a requirement only when a bound case passed in a fresh report.
- ADR 0001, "Change-risk classes", the sentence "The hash of a requirement covers its sentence without the key and without the alias marker": the hash also excludes the markers `verified-by` and `proof-owed`.
- ADR 0006, "Context and Problem Statement", the constraint "A gate MUST run from the repository with only spec-dna and git": as in the first item.

## Amendments
