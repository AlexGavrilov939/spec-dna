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

A tag binds to the test that it stands directly above, or to the case that `case:` names in full. [The rule of a bind](#the-rule-of-a-bind) says which cases of the reports prove that test. The sections below give the details.

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
| call | JavaScript, TypeScript, Ruby, Kotlin | `it`, `test`, `specify`, `example` or `scenario`, with any of `.only`, `.skip`, `.todo`, `.concurrent`, `.failing`, `.fails` and `.sequential`, and `.runIf(<condition>)` and `.skipIf(<condition>)` with the condition on the same line, then `(` or a space, then a literal in `'`, `"` or a backtick. In Kotlin, `fun`, a text in backticks, then `(` | the text of the literal, or the text in the backticks |
| method | Java, Kotlin, C#, PHP, Scala | a word and `(`, where an annotation or an attribute came before, or where the line has `void`, `fun`, `function` or `def` before the word. | the word |
| explicit | each | `case:<name>` in the tag | `<name>` |

A tag binds no case in these 3 cases:

| Reason | The tag | Fix |
| --- | --- | --- |
| `no-test` | The definition line matches no form: an import, a statement in the body of a test, or a helper function that a runner does not collect | Put the tag on the line above one test, or add `case:"<name>"` |
| `group` | The definition line opens a group: `describe`, `context` or `suite` (also `RSpec.describe` and `test.describe`), or `class`, `object`, `mod`, `impl`, `module` or `namespace` with a space. A tag above a group binds no test of the group | Put the tag above one test, or add `case:"<name>"` |
| `template` | The literal of a call has a placeholder (`%`, `$` or `#{`), or the call has `.each`. One literal gives many names | Add `case:"<name>"` with one name of the report |

### The rule of a bind

A bind is strong only when each case that it binds is an own case. A strong bind is silent. Each other bind is weak, and SD621 reports it.

- The identity of a test is the test file, the chain of the module, class or group titles around the definition, and the name. A report writes the test file as the `file` of a case, or as its whole `classname`. The row tail of a runner is no part of the identity: parameters, a data set, a subtest or the case of a macro.
- spec-dna builds the identity from the definition under the tag. An own case has this identity in the report, and no other definition of the project builds it.
- A case that names the test is another test only when a field differs that the report and the built identity both hold in full, or when its field names [another file](#a-case-of-another-file). Such a case leaves the set of the tag. No case leaves the set because another case fits better.
- Each other case that names the test is a weak link. It stays in the set, and it takes part in [the proof](#the-proof).
- When spec-dna cannot build the identity, or the identity is not unique, the bind is weak.

Thus a report that holds the full fields tells 2 tests of one name apart: a failed case of the other test does not touch the tag, and a passed case of the other test cannot prove it. A report that holds fewer fields puts the 2 cases in one set. SD621 then says so, and a fail or a skip of one of them leaves the requirement unproven.

### The cases that spec-dna reads

spec-dna reads each case whose name can name the test. This list is wide on purpose, and the identity then decides what each case is.

- The forms `def`, `fn`, `func` and method. The name of the test is a whole segment of the name of the case before its parameters. The separators of the segments are ` > `, ` › `, `::`, `/`, `.` and `#`. A space inside a name is no separator. The parameters start right after the name with `(` or `[`, or with the words `with data set`. Thus `test_x_slow` is not `test_x`.
- A Go method of a suite (`func (s *ValveSuite) TestCloses()`, testify) has the case `<Test function>/<method>`, for example `TestValveSuite/TestCloses`. The function is the one whose lines have `suite.Run(t, new(ValveSuite))` or `suite.Run(t, &ValveSuite{...})`, in the `_test.go` files of the same folder. The case of another function, a case with the method name alone, and a type that no function runs bind nothing (`no-case`). A function that runs 2 suites with a method of this name binds neither, because the 2 cases have one name.
- The form call. The literal is a whole segment of the name of the case, or the end of that name after a space. A name that has a segment of ` > ` never matches by its last word. A Kotlin test with a name in backticks has its whole text as its name, with its parentheses and brackets, and the report can add the parameters of the method.
- A case whose name is the built name of the test, with or without a row tail.
- For the end after a space, spec-dna looks only at the cases whose last word is the last word of the title.
- A word of the line that is not the name of the test never binds. In particular, the word `it` of an import line binds no case.

### A case of another file

A case can name the test and belong to another file, for example a test of another language in the same report. spec-dna leaves such a case out of the set when 2 facts hold: its `file` or its `classname` names a file of the code tree in full, and neither the field nor the identity ties the case to the tag file. This applies to each row, and also to a pair that no row lists. A case that can be the case of the tag never leaves this way.

| The field | It names a file of the code tree when |
| --- | --- |
| `file` | a case with the `file` of another file is another test, with or without such a file in the tree |
| a `classname` with `::` | the part after the last `::`, and after its last `/`, is the stem of the file, or the name of its folder when the stem is `main` |
| a `classname` that is a path of a script, or with `/` and a `.` in its last segment | the paths are equal, or one ends with `/` and the other |
| another `classname` with `/` | the folder of the file ends it |
| another `classname` with no space | `$`, `+` and `\` read as `.`. The dotted path of the file ends with it, or with its part before the last `.`. A name of one segment also names a file of a folder with this name |

A `classname` with a space is a title, and the `classname` of jest-junit that equals the name is a title too. Such a text names no file. The identity ties a case to the tag file in 3 cases: the class chain ends the `classname` of a JVM or .NET case (or spec-dna cannot read the chain), the row is the one of cargo-nextest, or the row is the one of Go. In these 2 rows, only a field that differs leaves a case out. `case:` reads the `file` only: a case whose `classname` names another file stays as a weak link.

### The chain of groups

spec-dna reads the chain from the indent of the lines. Each line above the definition with less indent must open a group: `describe`, `context` or `suite` (with `.only`, `.skip` or another suffix that keeps one title), `test.describe`, `RSpec.describe`, `class`, `object`, `mod`, `module` or `namespace`. A line `{` alone and a line that starts with `)` do not count.

The title of a group of a call is a literal with nothing added, with no escape and no interpolation, or a bare constant of Ruby such as `describe Valve do`. The chain is unreadable in these cases:

- a line with less indent is no group, for example an `if`, a function or an `impl`;
- a title is not such a literal or constant: a variable, `Valves::Valve`, a second title or `describe.each`;
- the file has no indent: a group at the indent of the definition stays open above it.

The kinds of the groups must fit the form. The form `def` takes classes, `fn` takes `mod` blocks, `func` takes no group, and call takes the groups of a call. The form method needs a class, and it also takes namespaces. For the form method, spec-dna reads the line `package` or a file-scoped `namespace` too. The scanner reads lines, not a syntax tree, so it needs no language detection.

### The row of a case

The form of the definition and the shape of the case give the row. The extension of the test file does not choose the row. A path field is equal when the 2 paths are equal, or when one ends with `/` and the other, because runners write paths from their own root. `./` at the start goes, and `\` reads as `/`. A case with the `file` of another file is another test in each row.

| Row | Form and shape of the case | The identity that the case must have | Full fields | Weak when |
| --- | --- | --- | --- | --- |
| pytest, `xunit2` | `def`; a dotted `classname`, no `file` | `classname`: the dotted path of the file and the class chain, where one ends the other at a `.`. Name: the function, then `[...]` | `classname`, name | the chain is unreadable |
| pytest, `xunit1` | `def`; `file` and `classname` | the `file`, and the `classname` and name of `xunit2` | `file`, `classname`, name | the chain is unreadable |
| cargo-nextest, integration test | `fn`; a `classname` with `::` | `classname`: `<package>::<binary>`. The binary is `<name>` for `tests/<name>.rs`, or `<dir>` for `tests/<dir>/main.rs` and its modules. Name: the module path under the binary, the `mod` chain and the function, then `::<row>` | `classname`, name | with no `Cargo.toml`, 2 crates have the binary. The scanner cannot build the identity when `tests/<dir>/main.rs` is not in the code tree and is no untracked test file |
| cargo-nextest, unit test | `fn`; a `classname` `<package>` or `<package>::bin/<name>` | `<package>` for the library, and `<package>::bin/<name>` for a binary. A module file of `src/` is in the library when the crate has only `src/lib.rs`, and in the binary `bin/<package>` when it has only `src/main.rs`. With both, a root file tells the target when it has the line `mod <name>;` for the first module segment with no indent, and the other root file has no such line at any indent. `src/bin/<name>.rs` is `bin/<name>`. Name: the module path of the file, the `mod` chain and the function, then `::<row>` | `classname`, name | there is no `Cargo.toml`. Also a module file when no root file is there, or when the crate has both roots and the lines do not tell the target: both have the line at some indent, neither has it, or only one has it, with an indent |
| Go | `func`; any `classname` with no `::` | the import path (the `module` line of `go.mod` and the folder below it), or the folder below the module (gotestsum `relative`). With no `go.mod`, the folder ends the `classname`. Name: the function, then `/<subtest>`. A testify method: `<function>/<method>` | `classname`, name | gotestsum `short`. With no `go.mod`: the root package, or 2 folders end the `classname` |
| JUnit and xUnit | method; a dotted `classname`, no `file` | `classname`: the full class name, that is the package or namespace and the class chain. 2 classes of one name in 2 namespaces are 2 identities. `$` and `+` read as `.`. With no `package` or `namespace` line, the class chain ends the `classname`, and the `classname` names the tag file. Name: `M`, `C.M` or `N.C.M` (`MethodFormat`), then `(...)` or `[...]` | `classname`, name | a method with `@DisplayName` or `DisplayName =`. A test of a nested class whose case has the outer class as its `classname`. No `package` or `namespace` line, and a `classname` that does not name the tag file |
| PHPUnit | method; `file` and `classname` | the `file`. The `classname` ends with the class chain. Name: the method, then the words `with data set` and the data set | `file`, `classname`, name | the chain is unreadable |
| RSpec, jest-junit with `addFileAttribute` | call; `file` and `classname` | the `file`. Name: the titles of the groups and the literal, joined with a space. RSpec writes no space between a group of a bare constant and a group below it whose title starts with `#`, `.` or `::`. The joins of Vitest and Playwright count too | `file`, name | a title is not a plain literal |
| Vitest, Playwright | call; a `classname` that is a path of a script | `classname`: the path. Name: the titles and the literal, joined with ` > ` (Vitest) or ` › ` (Playwright). A `[<project>]` prefix of Playwright goes | `classname`, name | a title is not a plain literal |
| jest-junit, the defaults | call; a `classname` that is no path of a script | the name is the join with a space | name | always: with no `file`, no field names the test file |
| Line format | any form; no `classname` | the name | none | always |
| Another pair | for example `fn` with a dotted `classname` that is no package name | none | none | always |

The package and the import path come from the nearest `Cargo.toml` (`[package] name`) and `go.mod` (the line `module`) at or above a Rust or Go test file. A bind reads these files from the working tree, each one once. Beside the test file of the tag, it reads these files, `src/lib.rs` and `src/main.rs` of the crate of a module file, and the `_test.go` files of the folder of a Go test file. It also reads the list of the paths of the code tree, with the test files that git does not track. A tag in a JVM or .NET file also reads [the class index](#the-class-index).

### A unique identity

The identity is not unique when another definition can build it:

- the same name 2 times in one chain, 2 methods of one name (an overload), or 2 RSpec examples with one full title;
- a call with any modifier of the form table: `test.skip('closes')` beside `test.runIf(true)('closes')` is 2 definitions;
- a template with a literal and no placeholder, such as `it.each(rows)('closes')`, that has the full title of the test;
- a template in the same chain whose literal can give the title, when each placeholder (`%s`, `$name`, `${...}`, `#{...}`) reads as any text. The title can follow a table on more than one line;
- a line `include_examples` or `include_context` in a group of the chain, because RSpec writes the examples of another file with the title and the `file` of this group;
- a `func` of the same name and receiver in another `_test.go` file of the folder, for example for other build constraints;
- a method of the same name in a class inside the class of the test, because a newer Surefire writes the outer class for such a method;
- a `mod` with the name of a Rust test, for a case that its rows share;
- a method whose display name is the name of the case;
- a definition of the same name whose chain is unreadable.

It is also not unique when its file field names more than one file of the code tree with the extension of the tag file. Each file of the code tree counts, also outside `[evidence] tests`, and each test file that git does not track. For the JVM and .NET, the full class name counts, never the stem of the file: spec-dna counts the files whose dotted path ends with the full class name, and it asks [the class index](#the-class-index). Examples: `tests.test_api` in 2 packages of pytest with no root, one binary in 2 crates with no `Cargo.toml`, one `spec.ts` path in 2 projects, one Java class in 2 modules, and a `partial` class of C#.

### The class index

For a tag in a `.java`, `.kt` or `.cs` file, the identity is also not unique when another file of one of these 3 kinds declares the class. The extension counts in any letter case, for the file of the tag and for each other file. The folders and the names of the files do not count. This applies to the test under the tag and to `case:`.

spec-dna builds the index one time in each run, when the first tag needs it. It reads each such file of the code tree, and each such test file that git does not track. It keeps a hash of each class name that a file declares, with the namespace lines of the file, and no text of a file. If 2 names have one hash, a bind can be weak, and no bind becomes silent.

- A declaration is the word `class` or `interface`, or `object` in a `.java` or `.kt` file, and the name after it. Modifiers, annotations and attributes can stand before the word, on the same line. Thus `data class`, `record class` and `partial class` count.
- In a `.java` or `.cs` file, the word `record` and a name on the same line are also a declaration, when `(`, `<`, `{`, `:`, `;` or the end of the line comes after the name. Thus `record ValveTest(int id)` counts, and `record.close()`, `Record record)` and `void record()` do not. In a `.kt` file, `record` declares nothing.
- The namespace of a `.java` or `.kt` file is its `package` line. A `.cs` file has the name of each of its `namespace` lines, and each name that some of them give when joined in their order. Thus `namespace A {` and `namespace B {` also give `A.B`.
- For a nested class, the index compares the outermost class. It records no nesting, so each class with that name counts, also a nested one.
- With no `package` or `namespace` line in the tag file, 2 namespaces count: the default one, and the part of the `classname` before the class chain.
- For `case:`, the last part of a dotted `classname` is a class. So is each part before it that the tag file declares.
- spec-dna reads lines. It does not read the rest of a line after `//`, or after a `/*` with no `*/` on the line. It does not read a line that starts with `*` before its `*/`, or a `#` line of a `.cs` file. It also does not read a comment or a string that starts and ends on one line. Another line of a comment or of a string reads as code. A `package` or `namespace` counts only when the line ends after its name, or goes on with `;` or `{`. A line that reads as code can make a bind weak without a twin. Limit 15 gives the one case where it can hide a twin.

### The proof

The worst of all identities of the set decides: a failed case, then a skipped case, then a passed case. An empty set gives `no-case`. A case that passed after a rerun (`flakyFailure`, `rerunFailure`) counts as a fail. A skipped case is no pass and no fail. Each row of a table and each subtest is a case of its own, so a skipped row leaves the requirement unproven. Each weak link is a case of its own too.

An own case that comes more than once is one test that ran again. It passed when a run passed and no run failed. This applies to a case once in each of several reports. In one report, it applies only to the projects of Playwright: a title joined with ` › `, or a title with a `[<project>]` prefix, in a `classname` that is a path. Each other repeat is a case of its own, so a skipped repeat leaves the requirement unproven. This includes each repeat of an identity that is not unique, and each repeat in one report of jest-junit or Vitest. [Limit 14](#the-limits-of-the-bind) tells which test of Playwright is no such run.

A requirement passes when a tag of it is proven and no case of its tags failed. A `verified-by` record is a check by a person: the report does not apply to it.

When a tag binds no passed case, SD616 or SD617 gives one reason. The list is closed:

| Reason | Meaning |
| --- | --- |
| `no-test` | The tag stands above no test |
| `group` | The tag stands above a group |
| `template` | The test under the tag has a template name |
| `no-case` | No case of a fresh report names the test in the file of the tag. For `case:`, no case has the name |
| `outside-tree` | The test file of the tag is not in the code tree ([The code tree and the stamp](#the-code-tree-and-the-stamp)) |
| `long-name` | No case names the test, and a case that can be of the test file holds the name of the test in a name with more than 256 characters or more than 32 separators. spec-dna does not read each part of such a name: add `case:"<name>"` |
| `no-report` | No report is fresh |

A failed case and a skipped case give their own text, with the name of the case. When more than one identity or repeat of the set decided (a shared set), the message names the test and the case: ``the tests of the name `<name>` in <path> share their cases, and the case `<case>` was skipped``.

### The name of a case with `case:`

`case:<name>` follows the ids of the tag: `# covers: loans#renew case:test_renew_on_time`. Use `case:"<name>"` when the name has a space or a `#`. The value is the whole `name` of the case as the report writes it, and it equals the name byte for byte. A part of a name binds nothing: `case:closes` does not bind the case `Valves closes`, and the message names the cases of the report that end with the value. A tag with `case:` needs no test under it. These make the `case:` not valid: no name, a space after the colon, a quote that is not closed, an empty name in quotes, a `"` inside the name, and another `"` in the rest of the line. Such a tag binds no case (the reason `no-case`, and the message says that the `case:` is not valid), and it never binds the test under it. Write the name without a `"`, with no space after the colon.

The value takes the place of the built name, and spec-dna builds no name. A case with the `file` of another file is another test. Each other case of the name is in the set. Such a case is an own case when a full field ties it to the tag file:

- its `file`;
- a `classname` that is the path of the file;
- for a Rust file, the binary id of the file with the package of `Cargo.toml`, and for a Go file, the import path or the folder;
- a dotted `classname` that names the file. Classes that the tag file declares can follow the file. They can also follow the package or namespace of the file.

A case whose `classname` names another file is weak, never another test, because `case:` exists for a layout that spec-dna reads wrong. Thus [the step of another file](#a-case-of-another-file) reads only the `file` for `case:`. A case with no full field is weak, also when it is the one case of the name. 2 own cases of 2 identities are not unique. Thus `case:` ends only the SD621 of the cause `no-identity`.

### The weak bind (SD621)

A bind is weak when a case of its set is weak. SD621 names one cause, the first of this list that a case of the set has:

| Cause | The weak case | The way out |
| --- | --- | --- |
| `no-identity` | spec-dna cannot build the identity from the lines: an unreadable chain, a title that is no plain literal, a layout of a file that the path does not tell, or a display name | Add `case:"<name>"` if the case is the test under the tag |
| `not-unique` | The identity is [not unique](#a-unique-identity) | Rename one of the tests or files, or set the runner so that its report tells them apart. `case:` cannot help, because the names are equal |
| `by-name` | No full field ties the case to the test: the report or the project does not hold the field | Make the runner write it (jest-junit: `addFileAttribute: "true"`; pytest: `junit_family = xunit1`; gotestsum: `--junitfile-testcase-classname=relative`), or keep the `Cargo.toml` or `go.mod` above the test file |

A weak bind still counts. With no case of the test and a passed case of its twin, the result is passed with SD621. In `standard` SD621 is a warning. In `strict` it is an error, so it stops `finish` for a requirement that the change adds or changes.

In [the field `binds`](commands.md#show), `fit` is the full field that tied the own cases (`name` when none did), `cases` is the number of bound cases, and `shared` is true when the worst of more than one identity or repeat decided.

### The report shapes of the runners

The shapes below come from the documentation or the source of each runner. A table test of spec-dna binds each row of this page in memory against neighbors of the same name. Tests with report files run the shapes of 7 runners (pytest, cargo-nextest, Jest, Vitest, go-junit-report, Surefire and PHPUnit). A layout that this page does not read gives a weak bind, or the reason `no-case`.

| Runner and producer | `classname` | `name` | `file` |
| --- | --- | --- | --- |
| pytest, `junit_family = xunit2` (the default) | `tests.test_valves`, or `tests.test_valves.TestValves` in a class | `test_closes_on_timeout`, `test_closes[1-2]` | none |
| pytest, `junit_family = xunit1` | the same | the same | `tests/test_valves.py` |
| cargo-nextest, an integration test `tests/valves.rs` | `<package>::valves`, with the `-` of the package | `closes`, or `<module>::closes` | none |
| cargo-nextest, a unit test | `<package>`, or `<package>::bin/<name>` | `<module>::tests::closes` | none |
| Jest, jest-junit, the defaults | `Valves closes on timeout` | `Valves closes on timeout` | none |
| Jest, jest-junit, `addFileAttribute: "true"` | as above | as above | `tests/valves.test.js` |
| Vitest, `--reporter=junit` | `tests/valves.test.ts` | `Valves > closes on timeout` | none |
| Playwright, the JUnit reporter | the path from its `rootDir`, for example `valves.spec.ts` | `Valves › closes on timeout`, with a `[<project>]` prefix only with `includeProjectInTestName` | none by default |
| Go, go-junit-report | `example.org/app/valves` | `TestTimeout`, `TestTimeout/no_reply` | none |
| Go, gotestsum, `--junitfile-testcase-classname=relative` | `valves` | as go-junit-report | none |
| Java and Kotlin, Maven Surefire with JUnit 5, the phrased names off | `com.example.ValveTest`. For a nested class, Surefire 3.2.5 writes `com.example.ValveTest$Nested`, and a newer Surefire can write the outer class | `flushesFirst`, `flushesFirst(int)[1]` | none |
| PHP, PHPUnit `--log-junit` | `Tests.ValveTest` | `testClosesOnTimeout`, `testClosesOnTimeout with data set #0` | the absolute path |
| Ruby, rspec_junit_formatter | `spec.valve_spec` | `Valve closes on timeout` | `./spec/valve_spec.rb` |
| .NET, JunitXml.TestLogger | `Valves.Tests.ValveTest` | `ClosesOnTimeout`, `ClosesOnTimeout(seconds: 30)` (with `MethodFormat`, the class or the namespace comes first) | none |

No report that a real runner wrote confirms these shapes: the base folder of the path of Playwright, the default display name of xUnit, the `$` of a `@Nested` class of JUnit, the rows `name(int)[1]` of Kotlin, and the `-` of a package name in the binary id of nextest.

A nextest report has no skipped test by default, so the tag of a skipped test finds no case (`no-case`). With the phrased names on, Surefire writes the display names, so the name of the method is no part of the name of the case, and the tag needs `case:`. JunitXml.TestLogger takes the name from the method in its current source. Other versions and other runners can write a display name, so a method with a display name gives a weak bind.

### The limits of the bind

A report cannot tell these cases apart, or the scanner cannot read them. Each one is a limit of the bind, not an error. When the scanner sees the clash, the bind is weak (SD621).

1. One `classname` for 2 files in a repository of several projects: 2 `rootdir` folders of pytest, gotestsum `short`, the default package of Java or one full class name in 2 modules, one C# class in 2 projects or a `partial` class, and projects of Playwright with one relative path. The scanner sees the clash when the paths show it. For the JVM and .NET, [the class index](#the-class-index) also sees each `.java`, `.kt` or `.cs` file of the code tree that declares the class. So for them, a silent bind needs a twin outside the code tree (limit 2), or one of limit 15.
2. A clash with a file outside the code tree: an untracked file outside `[evidence] tests`, or a file that `tree-exclude` matches. The stamp does not cover such a file, so the bind does not read it.
3. jest-junit with no `addFileAttribute`: always weak.
4. A Rust or Go tree with no `Cargo.toml` or `go.mod` above the tag file, and a binary, a module path or a folder that 2 files share.
5. Inherited and shared tests: pytest base classes, abstract bases and interface defaults of JUnit, abstract cases of PHPUnit, shared examples of RSpec. A tag on the base gives a weak bind or `no-case`. A tag in a group with `include_examples` or `include_context` is weak.
6. Generated names: templates, `.each`, `@TestFactory`, a display name that is not a literal, the phrased names of Surefire. A tag above a template has no name to compare. Use `case:`, which stays weak where [The name of a case with `case:`](#the-name-of-a-case-with-case) says. A template that can give the title of a plain test makes that test weak. spec-dna does not see the title of a table in a template literal of more than one line, or a `.runIf` whose condition takes more than one line.
7. Lines that the scanner cannot read: no indent, a title from a variable or a constant path, a group head on 2 lines, a test on 2 lines before its `(` (`no-test`).
8. Custom Rust targets (`[[test]] path`, `harness = false`, `#[path]`, a binary of `[[bin]]` with another name): the mapping is wrong, so the bind is weak or `no-case`.
9. A runner that does not write a skipped test (nextest by default): `no-case`, which is the correct result.
10. A custom `classNameTemplate` or `titleTemplate` of jest-junit that changes the name: the case has another title, so the tag gets `no-case`. A `classNameTemplate` of one word can also name another file of the code tree. Then the case is another test. This also applies to a report whose `classname` is the bare title. For example, the case of the tagged test has the name `Auth login` and the `classname` `login`, and the code tree has a folder `src/login/`. The case then leaves the set, also when it failed. With no other case of the test, the result is `no-case`. When a weak case of the test stays in the set and passed, the result is passed with SD621.
11. A Scala test with `def` reads as the form `def`, so its JUnit `classname` is another test. Use `case:`.
12. A report of 2 languages: a case of another language with the same name is another test when its field names a file of the code tree ([A case of another file](#a-case-of-another-file)). When it names no file, and no row reads its shape, it is a weak link (`by-name`) of the tag.
13. A name of more than 256 characters or more than 32 separators has not each part as a key. It binds by the built name, also with its row tail, or by `case:` (the reason `long-name`).
14. A Playwright test with no `describe` and no project prefix has the shape of a Vitest test. When it runs in 2 projects, its 2 cases in one report count as 2 cases, not as 2 runs, and the worst result of them decides.
15. [The class index](#the-class-index) does not read a `struct` or an `enum`, also not a C# `record struct`. It does not read a Java or C# `record` when the name is not on the line of the word `record`. It does not read a file of Scala or Groovy, or a `.kts` file. A twin class in one of these gives a silent bind when no path shows the clash. A tag in a file of Scala or Groovy, or in a `.kts` file, does not ask the index. A line of a comment or a string that reads as a `package` or `namespace` line also takes the classes below it out of the default namespace. This matters only for a tag in the default namespace.
16. A case of another language with the same name, whose dotted `classname` names no file and no folder of the code tree, is a case of limit 12 that stays in the set. This occurs when the folders do not show the package. For example, a tag is on the Rust test `closesv1`, and `src/test/kotlin/ValveV1Test.kt` holds the Kotlin class `com.example.ValveV1Test`. When the nextest case `closesv1` passed and the Kotlin case `closesv1` of that class failed, the result is failed with SD621.

## The code tree and the stamp

The code tree is one hash of the code as it was when the tests ran. It covers each file that git tracks, with the bytes on the disk. A symbolic link counts with the text that it names, as git stores it, and a submodule counts with its entry in the git index (the tree does not look into it). It leaves out these paths: the files that `tree-exclude` matches (default: each Markdown file), the folders of the `intent` layer, `.spec-dna/`, the files that `init` writes with `spec-dna.toml` and the report files with their stamps. Thus a change of documents never makes a report stale, and a change of code does.

An untracked file does not count. A file that git does not track yet is not in the tree, so a `git add` of a new file after the stamp makes the report stale. A test file that git does not track yet is not in the tree hash, but `check`, `finish` and `accept` count its path for [a unique identity](#a-unique-identity) and for [a case of another file](#a-case-of-another-file). A test file that git ignores and does not track stays unread.

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
