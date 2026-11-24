# Test reports

This page is the reference of the test reports. It gives the rule that binds a `covers:` tag to a test case, the code tree and the stamp, the setup of a hook, the report formats and the proof levels. It is for the author of a test and of `spec-dna.toml`.

A tag is a claim of the author. With `[evidence] reports`, a tag proves a requirement only when a test case that binds to it passed in a fresh report. Without the key, nothing changes: a tag alone is evidence, as in 0.7.1. An empty list is the same as no key. The `lite` profile turns the codes of the reports off. `[evidence] reports` and the group `reports` of `[rules]` are the only switches.

## Turn the reports on

Make 3 steps in the job that runs the tests ([the change guide](guides/change.md#test-reports-in-ci) gives them with an example):

1. Run the tests so that they write a report. spec-dna reads 2 formats ([The report formats](#the-report-formats)).
2. Run `spec-dna evidence stamp` after the tests, in the same job. It writes `<report>.stamp` next to each report. The stamp holds the hash of the code tree and the hash of the report bytes.
3. Run `spec-dna check --reports`. `finish` reads the reports when the key is set, with no option.

`check --reports` and `finish` read the reports from the disk, also when git ignores them. A report counts when it is readable, git does not track it, it has a stamp, and the stamp names the current code tree and the bytes of the report. Add the reports to `.gitignore`.

A glob of `reports` is a path from the repository root: an absolute path, a drive letter and a `..` segment are errors of the configuration (exit code 3). A folder that does not exist has no report (SD613). A folder that exists and cannot be read is an input error (exit code 4), because it must not look like a project with no report.

With no fresh report, each full tag is unbound, so SD616 or SD617 still tell that the requirement is unproven, also when a setting makes SD613 to SD615 a warning or turns them off. A report that is in no root (its path is below no `[[roots]]` path) takes the severity of the project for SD613 to SD615, but the requirement keeps the severity of its own root.

## How a tag binds to a test case

A tag binds to the test that it stands directly above, or to the case that `case:` names in full. The rule reads lines, not a syntax tree, so it needs no language detection. The sections below give the rule.

### The test under the tag

The scan starts at the line after the tag and reads at most 50 lines. It skips these lines:

- a blank line;
- a comment line: `//`, `/*`, `*`, `--`, `<!--`, and `#` that does not start `#[`;
- a decorator, an annotation or an attribute: `@<name>` with an optional group `(...)`, `#[...]` or `[...]` at the start of a line. An item goes on in the next lines while one of its brackets stays open. The count of the brackets ignores the text in quotes. When text stays after the items on a line, that text is the definition line: `@Test fun closes() {` and `[Fact] public void ClosesOnTimeout()` are tests with an attribute.

The first other line is the definition line. A definition on the line of the tag, before the text `covers:`, is the test of the tag: `def test_x():  # covers: ...`. A tag that stands in a comment on its own line has no definition before it.

A definition line with less indent than the tag ends the scope of the tag. A tag in the body of a test thus stands above no test, also when the next test follows.

The definition line is a test when one form of this closed list matches it. The first form that matches decides.

| Form | Languages | The definition line | The name of the test |
| --- | --- | --- | --- |
| `def` | Python | `def <name>(` or `async def <name>(` | `<name>` |
| `fn` | Rust | `fn <name>(`, after any of `pub`, `pub(crate)`, `async`, `const` and `unsafe` | `<name>` |
| `func` | Go | `func <Name>(`, or `func (<receiver>) <Name>(` | `<Name>` |
| call | JavaScript, TypeScript, Ruby, Kotlin | `it`, `test`, `specify`, `example` or `scenario`, with any of `.only`, `.skip`, `.todo`, `.concurrent` and `.failing`, then `(` or a space, then a literal in `'`, `"` or a backtick. In Kotlin, `fun`, a text in backticks, then `(` | the text of the literal, or the text in the backticks |
| method | Java, Kotlin, C#, PHP, Scala | a word and `(`, where an annotation or an attribute came before, or where the line has `void`, `fun`, `function` or `def` before the word. | the word |
| explicit | each | `case:<name>` in the tag | `<name>` |

A tag binds no case in these 3 cases:

| Reason | The tag | Fix |
| --- | --- | --- |
| `no-test` | The definition line matches no form: an import, a statement in the body of a test, or a helper function that a runner does not collect | Put the tag on the line above one test, or add `case:"<name>"` |
| `group` | The definition line opens a group: `describe`, `context` or `suite`, or `class`, `object`, `mod`, `impl`, `module` or `namespace` with a space. A tag above a group binds no test of the group | Put the tag above one test, or add `case:"<name>"` |
| `template` | The literal of a call has a placeholder (`%`, `$` or `#{`), or the call has `.each`. One literal gives many names | Add `case:"<name>"` with one name of the report |

### The name of the case

A case names the test when the name of the test is a whole segment of the name of the case. The separators of the segments are ` > `, ` › `, `::`, `/`, `.` and `#`. A space inside a name is no separator, so the name `Closes` is no segment of the name `Closes when cold`. These forms are closed:

- The forms `def`, `fn`, `func` and method. The name of the test is a segment of the name of the case before its parameters. The parameters start right after the name with `(` or `[`, or with the words `with data set`. Examples: `test_closes[1-2]` (pytest parameters), `tests::closes` (a module path of nextest), `closes::case_1` (the cases of a Rust macro), `TestTimeout/no_reply` (a Go subtest), `test1(String) [1] a` (JUnit 5 parameters), `testOne with data set #0` (PHPUnit data) and `ValveTest.ClosesOnTimeout` (.NET). The name `test_x_slow` is not the name `test_x`, and a display name such as `Closes when cold` is not the name `Closes`.
- A Go method of a suite (`func (s *ValveSuite) TestCloses()`, testify) has the case `<Test function>/<method>`, for example `TestValveSuite/TestCloses`, and a subtest adds more segments. The function is the one whose lines have `suite.Run(t, new(ValveSuite))` or `suite.Run(t, &ValveSuite{...})`, in the test files of the same folder. The case of another suite function, a case with the method name alone, and a type that no function runs bind no case (the reason `no-case`). A function that runs 2 suites that both have a method of this name binds neither, because the 2 cases have one name.
- The form call. The name of the test is the literal. A segment that equals it, or the whole case name, is a strong match. A case name that only ends with the literal after a space is a weak match, because jest-junit and RSpec join the titles with a space. A weak match is the expected name when the titles of the groups above the test (`describe`) and the literal, joined with a space, give the whole case name. A case name that has a segment of ` > ` never matches by its last word. Examples: `closes on timeout`, `Valves closes on timeout` (jest-junit, RSpec) and `Valves > closes on timeout` (Vitest). A Kotlin test with a name in backticks has this form, and its name is whole: parentheses and brackets are part of it. The report can add the parameters of the method after it: the cases `closes on timeout()` and `closes on timeout(int) [1] 3` have the name `closes on timeout`. For another language the name is the literal, so `it('flushes')` does not bind the case `flushes (slow)` of a table.
- A word of the line that is not the name of the test never binds. In particular, the word `it` of an import line binds no case.

No case hides another case. A tag binds each case of the reports that fits its test file and that the name rules above link to the name of the test, and the worst of them decides. These tests have more than one case: a test of one name that the file defines twice, a template with the same literal (`it.each(rows)('closes')`), a group with the title of the test, a Go test with a suite method and a subtest of one name, and the cases of the rows of a table. A shared example (`it_behaves_like`) or a template (`it.each(rows)('%s opens')`) gives cases that end with the name of the test: they are weak matches, so they bind too. Thus the failed or skipped case of the test under the tag is always in the set, and a case of another test never takes its place. RSpec reads `RSpec.describe` as a group, and it joins the title of a group that starts with `#` or `.` to the title before it with no space (`Valve#open closes`).

A case can name 2 tests of the file. Example: `Valves never closes` ends with `closes` and with `never closes`. The case binds to the tag above `it('closes', ...)` as a weak match. If it failed, the requirement is unproven, and SD621 says that the bind is by the end of a name.

### The file of the case

A case fits the test file of the tag by the first row that applies:

| The case has | It fits the test file of the tag when | Runners |
| --- | --- | --- |
| `file` | the 2 paths are equal, or one ends with `/` and the other. `./` at the start goes, and `\` reads as `/` | pytest `xunit1`, PHPUnit, RSpec, jest-junit with `addFileAttribute`, the line format |
| a `classname` with `::` | the part after the last `::`, and after its last `/`, is the stem of the test file, or the name of its folder when the stem is `main` | cargo-nextest |
| a `classname` with `/`, whose last segment has a `.` | the `classname` is the path of the test file, by the rule of `file` | Vitest |
| a `classname` with `/` | the folder of the test file ends the `classname` in whole segments. When several folders do, only the longest one fits | go-junit-report, gotestsum `full` and `relative` |
| another `classname` | `$` reads as `.`. The dotted path of the test file (the path without its extension, with `.` for `/`) ends with the whole `classname`, in whole segments. A class after the module (pytest) or an inner class (Java) is a trailing segment that starts with an upper case letter, and it can go: `tests.test_valves.TestValves` fits `tests/test_valves.py`. The part before the last `.` of another name is no key: `tests.test_close` does not fit `app/tests/tests.py`. A `classname` of one segment also fits a test file whose folder ends with it | pytest `xunit2`, JUnit, .NET, gotestsum `short` |

When the row names no test file of the tree, the case fits by its name only. A case with no `file` and no `classname` also fits by its name only (a line of the line format with `-`). A case with a `file` that is not the file of the tag does not fit.

A Rust file is special. A file of `tests/` is a binary of its own, so only the binary id (`<crate>::<name>`) fits it. The name is the stem of the file, or the name of its folder when the stem is `main`. A file in a folder of `tests/` (`tests/it/valves.rs`) is a module of the binary of that folder, so it fits the id `<crate>::it` too, and the case name has the module path (`valves::closes`). The `classname` of the unit tests of the library (the crate) fits a file of `tests/` by its name only (SD621), unless the crate has the name of a test file: then it does not fit. A file of `src/` fits the unit tests of the library by the module path of its name: the case `valves::tests::closes` fits `src/valves.rs`, as a bind by name only.

The precedence: when a case fits by `file` or by `classname`, the cases that fit by their name only do not bind the tag. Thus a passed case of an unknown file never hides a skipped case of the file of the tag. No other rule leaves a case out of the set: a case with a strong name never hides a case with a weak name.

### Shared cases

The worst case decides in each set that is not one case. A set is not one case when 2 tests of the file have the name of the test, or more than one case fits. One case has one name, one `classname` and one `file`, and it appears once in each report. A set of more than one case proves the requirement only when each of its cases passed. Thus a skipped twin never hides behind a passed twin, and a skipped row of a table or a failed subtest leaves the requirement unproven. The message names the tests and the case: ``the tests of the name `<name>` in <path> share their cases, and the case `<case>` was skipped``.

### The name of a case with `case:`

`case:<name>` follows the ids of the tag: `# covers: loans#renew case:test_renew_on_time`. Use `case:"<name>"` when the name has a space or a `#`. The value is the whole `name` of the case as the report writes it, and it equals the name byte for byte. A part of a name binds nothing: `case:closes` does not bind the case `Valves closes`, and the message names the cases of the report that end with the value. A tag with `case:` needs no test under it. These make the `case:` not valid: no name, a space after the colon, a quote that is not closed, an empty name in quotes, a `"` inside the name, and another `"` in the rest of the line. Such a tag binds no case (the reason `no-case`, and the message says that the `case:` is not valid), and it never binds the test under it. Write the name without a `"`, with no space after the colon.

A case with a `file` binds only when the file is the file of the tag. For a case with no `file`, the `classname` does not exclude the case: it decides the strength, and the precedence of the file rule applies.

### The strength of a bind

A bind is exact when one case fits by `file` or by `classname`, and its name is a strong match (a whole segment of the name of the case, or the expected name), or the cases are the subtests and the rows of one test (they differ only after the name of the test), or when `case:` names it and the fresh reports have one case of that name. Else SD621 reports the tag: a case fits by its name only, a case names the test only by the end of its name, or more than one test of the reports fits (the cases differ before the end of the name of the test, or in `classname` or `file`, or one name is in one report more than once). The cases of a test that the file defines twice are no cause. The worst of all cases decides: a failed case, then a skipped case, then a passed case. A bind by name counts: the requirement is proven when its cases passed. SD621 makes the weak link visible. The fix is `case:"<name>"` with the name of the case, or a runner that writes the `file` of each case: for jest-junit, set `addFileAttribute: "true"` once.

### The proof

A requirement passes when at least one bound case passed and no bound case failed. A skipped case is no pass and no fail. A case that passed after a rerun (`flakyFailure`, `rerunFailure`) counts as a fail. A shared set needs a pass of each case. A `verified-by` record is a check by a person: the report does not apply to it.

When a tag binds no passed case, SD616 or SD617 gives one reason. The list is closed:

| Reason | Meaning |
| --- | --- |
| `no-test` | The tag stands above no test |
| `group` | The tag stands above a group |
| `template` | The test under the tag has a template name |
| `no-case` | No case of a fresh report names the test in the file of the tag. For `case:`, no case has the name |
| `outside-tree` | The test file of the tag is not in the code tree ([The code tree and the stamp](#the-code-tree-and-the-stamp)) |
| `long-name` | No case names the test, and a case of the reports has a name with more than 256 characters or more than 32 separators. Such a case has no key but its whole name: add `case:"<name>"` |
| `no-report` | No report is fresh |

A failed case, a skipped case and a shared set give their own text, with the name of the case.

### The report shapes of the runners

The shapes below come from the documentation or the source of each runner. The tests of spec-dna run the reports of 7 runners (pytest, cargo-nextest, Jest, Vitest, go-junit-report, Surefire and PHPUnit), and a table of unit tests covers the file rule for the other rows. A layout that this table does not read binds by name only, or gives the reason `no-case`. `case:` is the way out.

| Runner and producer | `classname` | `name` | `file` | Name form | Result of the file rule |
| --- | --- | --- | --- | --- | --- |
| pytest, `junit_family = xunit2` (the default) | `tests.test_valves`, or `tests.test_valves.TestValves` in a class | `test_closes_on_timeout`, `test_closes[1-2]` | none | `def`, parameters in `[...]` | `classname`: exact |
| pytest, `junit_family = xunit1` | the same | the same | `tests/test_valves.py` | `def` | `file`: exact |
| cargo-nextest, an integration test `tests/valves.rs` | `<crate>::valves` | `closes`, or `<module>::closes` | none | `fn` | `classname` (binary id): exact. A file in a folder of `tests/` (`tests/it/valves.rs`) fits the id of the folder (`<crate>::it`) too |
| cargo-nextest, a unit test of a library | `<crate>` | `<module>::tests::closes` | none | `fn` | a file of `src/` by the module path of the name, as a bind by name only (SD621). A file of `tests/` fits it by its name only, and gives SD621, unless the crate has the name of a test file (it does not fit then). A `src/lib.rs` fits any case by the weak end of its name, and gives SD621. Add `case:` for another layout |
| Jest, jest-junit, the defaults | `Valves closes on timeout` | `Valves closes on timeout` | none | call | by name only (SD621) |
| Jest, jest-junit, `addFileAttribute: "true"` | as above | as above | `tests/valves.test.js` | call | `file`: exact |
| Vitest, `--reporter=junit` | `tests/valves.test.ts` | `Valves > closes on timeout` | none | call | `classname` (path): exact |
| Go, go-junit-report | `example.org/app/valves` | `TestTimeout`, `TestTimeout/no_reply` | none | `func` | `classname` (folder): exact. The root package of a module: by name only |
| Go, gotestsum, `--junitfile-testcase-classname=relative` | `valves` | as go-junit-report | none | `func` | `classname` (folder): exact |
| Java, Maven Surefire with JUnit 5, the phrased names off | `com.example.ValveTest` | `flushesFirst`, `flushesFirst(int) [1] 3` | none | method | `classname`: exact in the layout of Java. A Kotlin file with another name than its class: by name only |
| PHP, PHPUnit `--log-junit` | `Tests.ValveTest` | `testClosesOnTimeout`, `testClosesOnTimeout with data set #0` | the absolute path | method | `file`: exact |
| Ruby, rspec_junit_formatter | `spec.valve_spec` | `Valve closes on timeout` | `./spec/valve_spec.rb` | call | `file`: exact |
| .NET, JunitXml.TestLogger | `Valves.Tests.ValveTest` | `ClosesOnTimeout`, `ClosesOnTimeout(seconds: 30)` | none | method | `classname`: exact when the folders follow the namespace. Else by name only |

Two limits. A nextest report has no skipped test by default, so the tag of a skipped test finds no case of its own (`no-case`). A case of another test that ends with the name can bind by name only, and SD621 reports it. Surefire with the phrased names on writes the display names, so the name of the method is not a segment of the case name, and the tag needs `case:`.

### The limits of the bind

- The forms read lines. An unusual layout, such as a test on 2 lines before its `(`, gives `no-test`, and `case:` is the way out.
- A helper function with a name that no case has gives `no-case`.
- A bind by name can bind a case of another file with the same name. A shared set catches 2 such cases in one report, but not a case of a file that the runner did not run.
- A tag above a group (`describe`, `class`) binds no test of the group. A class line in a file whose name is not the class (Kotlin), and a Rust unit test in `src/`, bind by name only.
- A case with a name of more than 256 characters or more than 32 separators has no key but its whole name, so only `case:` binds it (the reason `long-name`).
- Not read, and not changed in 0.8.1: 2 test files with one name in 2 packages of pytest (the `classname` of one segment binds both, and SD621 shows it when a case has no `file`); .NET names that start with an upper case letter after the namespace (the rule strips a trailing class by its case); a nextest crate whose name is not the stem of a test file; the weak end of the name for a root package of Go and for an RSpec file in the root. Each of them gives SD621 where the bind is by the end of a name, and `case:` is the way out.

## The code tree and the stamp

The code tree is one hash of the code as it was when the tests ran. It covers each file that git tracks, with the bytes on the disk. A symbolic link counts with the text that it names, as git stores it, and a submodule counts with its entry in the git index (the tree does not look into it). It leaves out these paths: the files that `tree-exclude` matches (default: each Markdown file), the folders of the `intent` layer, `.spec-dna/`, the files that `init` writes with `spec-dna.toml` and the report files with their stamps. Thus a change of documents never makes a report stale, and a change of code does.

An untracked file does not count. A file that git does not track yet is not in the tree, so a `git add` of a new file after the stamp makes the report stale. A test file that git does not track yet is not in the tree hash, but `check`, `finish` and `accept` read it for its `classname`: more test files only narrow a fit. A test file that git ignores and does not track stays unread.

A tag proves a requirement only when its test file is in the code tree: git has the file in its index, and `tree-exclude` does not match it. A tag in another file binds no test case, and SD616 or SD617 names the file with the reason `outside-tree`. Run `git add`, then the tests and `spec-dna evidence stamp`. `check` without `--reports` keeps the rule of 0.7.1: a tag alone is evidence, also in an untracked file.

SD614 reports a report that is stale. The stamp of a report names another code tree, or the bytes of the report changed after the stamp. A change of code makes a report stale. A change of Markdown does not.

## A hook that runs the tests

A pre-commit hook that runs the tests and writes a report writes new bytes at each commit, for example new times. The stamp holds the hash of the bytes, so the report is stale after the commit, and `check --reports` gives SD614. The message says so: the bytes of the report changed after its stamp, for example a commit hook or a second test run wrote it again.

The one setup: the hook that runs the tests runs `spec-dna evidence stamp` in the same entry, after the tests. Then each new report gets its stamp.

```yaml
  - repo: local
    hooks:
      - id: tests
        name: tests
        entry: sh -c '<the test command that writes the report> && spec-dna evidence stamp'
        language: system
        pass_filenames: false
```

This setup works because pre-commit stashes the edits that are not staged before it runs the hooks. The tests and the stamp thus read the tree of the commit. The report and the stamp are ignored files, so pre-commit sees no edit of a tracked file. A code edit that is not staged makes the stamp stale after the commit, which is correct.

`init` and `upgrade` print a note for a hook that runs the tests and does not stamp ([A hook that runs the tests](init.md#a-hook-that-runs-the-tests) in the page of `init`). `sh -c` needs a POSIX shell. On Windows, Git for Windows has one.

## The report formats

spec-dna reads 2 formats:

- `junit` is a subset of JUnit XML: the elements `testsuites`, `testsuite` and `testcase`, the attributes `name`, `classname` and `file`, and the children `failure`, `error`, `skipped`, `flakyFailure`, `flakyError`, `rerunFailure` and `rerunError`. spec-dna reads UTF-8 only, and it rejects a DTD.
- `lines` is a format for any runner. The first line is `spec-dna-report 1`, and each other line is `pass`, `fail` or `skip`, a tab, a path from the repository root or `-`, a tab and the name of the case. A blank line and a line that starts with `#` do not count.

A report is unreadable (SD615) in these cases: an unknown format, a parse error (also a `testcase` inside a `testcase`), a `testsuite` whose `tests` count is not its number of `testcase` elements (a cut file), a file over 64 MiB, a `<!DOCTYPE`, and a report or a stamp that git tracks. A report in git can be a hand-written one. Add the reports to `.gitignore`.

## Proof levels and the proof debt

Each requirement of an active document has one proof state. The states go from the best to the worst:

| State | Meaning |
| --- | --- |
| `reliable` | The state `passed-once`, and each bound case that passed has at least `reliable-runs` passes in the reports of the current code tree, with no fail |
| `passed-once` | A bound case passed in a fresh report, and no bound case failed |
| `verified` | A `verified-by` record. A person checked the requirement |
| `tagged` | A full `covers:` tag that no fresh report proves: no `reports` key, no fresh report, or a case that failed, was skipped or was not found |
| `owed` | A proof debt and no evidence |
| `none` | No evidence and no debt |

A tag that a bound case proves is above a `verified-by` record. `status` and `stats` show the states ([status](commands.md#status), [stats](commands.md#stats)). In `lite`, they show no line of the proof and `stats` reads no report, unless a rule turns a code of the group `proof` on.

The level `reliable`. The runner runs the tests `reliable-runs` times, and it writes one report for each run. Each report has its stamp (`spec-dna evidence stamp`). A pass of a case is one report in which it passed. A case that appears twice in one report counts once. spec-dna keeps no ledger and no retry. With `reliable-for` set, `finish` and `check --reports` stop on a requirement of a covered change that passed fewer times than `reliable-runs` (SD620). A requirement that no bound case proves gives SD617. When SD617 is off, it gives SD620, because 0 passes is fewer than `reliable-runs`. The values of `reliable-for` are:

- `none`: no change needs `reliable`. This is the default.
- `higher-risk`: a change of the class `higher-risk` needs it.
- `all`: each change needs it.

`reliable-for` needs `reports`. A value other than `none` with an empty or absent `reports` is an error of the configuration (exit code 3): ``[evidence] reliable-for needs [evidence] reports: set reports, or set reliable-for = "none"``. The `lite` profile turns SD620 off. A requirement that has only a `verified-by` record, with no tag, is `verified`, and SD620 does not apply to it. A requirement that has a tag is still below `reliable` when its bound case failed, was skipped or passed fewer times than `reliable-runs`, also when it has a `verified-by` record. The record does not replace the passes of a tag.

The proof debt. A requirement can end with the marker `<!-- proof-owed: <reason> -->`. The reason is one word of a closed list:

| Reason | Use it when |
| --- | --- |
| `manual` | A person must look |
| `environment` | CI has no such device, account or data |
| `external` | A third party must act |
| `pending` | The test is not written yet |

The marker is an act of the author on one requirement, as `verified-by` is. It has no key and no profile switch: in `lite` its codes are off, in `standard` they are warnings, and in `strict` SD618 is a warning and SD619 is an error. The hash of the requirement does not cover the marker, so a marker gives no SD914 and no SD921. The marker can stand next to `aliases` and `verified-by` markers. A debt never counts as proof:

- An active requirement with a debt and no evidence gives SD618, in place of SD608. SD618 is a state code, so `check --all` shows it.
- A marker with a reason that is not in the list, or a marker on a requirement that has evidence, gives SD619. A requirement with an invalid marker keeps SD608.
- `finish` and `check --reports` stop on a requirement with a debt and no tag with SD916, as they stop on a requirement with no evidence. A debt does not clear SD617 and SD620 either.

A debt has no date and no expiry, because a check reads no clock. `status` and `stats` show the count ([the section `stats`](commands.md#stats) gives the counts). The reason is a claim of the author: a person reads it in the review. A copy of a report counts as one more pass. The stamp binds bytes to a tree, not to a run, so the level `reliable` does not prove that the runs were real.

## What the reports do not prove

- A test can pass and still not check the requirement. The tag is a claim of the author.
- A person or an agent can write a report by hand, because the stamp binds bytes to a tree, not to a run. `check --reports` reads files outside git.
- A copy of a report, with its stamp, counts as one more pass of the level `reliable`.
- A definition line is not proof that the runner collects the test. A helper function with a name that no case has gives `no-case`, which is the correct result.
- Without `--reports`, `check` keeps the rule of 0.7.1, so the hook `check --staged` needs no report.

[Diagnostics](diagnostics.md#test-reports-sd613-to-sd617-and-sd621) lists the codes, and [commands](commands.md) gives `check --reports`, `show --reports` and the field `binds`.
