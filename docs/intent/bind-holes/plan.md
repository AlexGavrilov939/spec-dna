---
status: draft
---
# A failed test case never proves a requirement, a case of another language never fails a test, and 2,000 tags on one title add at most 1 s: plan

## Files

- `tests/fixtures/runners/**` The 72 files of the folder: the 6 folders of sources, the 24 reports, `generate.sh`, the files of the versions and the `.editorconfig` of the folder, copied with no edit from the work tree `pb-fix`, where they are not committed.
- `tests/evidence_reports.rs` The module `runners` with the tests on the real reports, the tests of the level of SD621 in `strict`, and the bound of the stall test.
- `src/evidence.rs` The locator of a case, the rule of a word, the forms of the name of a call, the maps by shape, the judgment of a signature, the reference binder, the twin of a template, the second `case:`, `struct` and `enum` in the class index, the flag of a fixable case, the new cells, the counts and the clock bounds.
- `src/check.rs` The level of each line of SD621, and its hint texts.
- `src/config.rs` The function that tells a rule for SD621 from the default of the profile, and its unit test.
- `src/finish.rs` Step 5.4 reads the level of each line of SD621.
- `tests/finish.rs` The stop of `finish` in `strict` on a fixable weak bind, and no stop on another weak bind.
- `src/diag.rs` The explain text of SD621.
- `tests/golden/check-binds.json` The hint text of SD621 in the golden output of `check --reports --json`.
- `bench/tool/bind.py` The loads L8 to L10 of the bind (new).
- `bench/tool/README.md` The section of the loads of the bind.
- `bench/results/tool-bind.json` The result of the bench (new).
- `docs/test-report.md` The rule, the shapes of the runners, the new section of the settings, and limits 10, 12, 15, 16 and 17.
- `docs/diagnostics.md` The row of SD621 and the level in `strict`.
- `docs/profiles.md` The row of the group `reports`.
- `CHANGELOG.md` The entry of the change.
- `docs/design/0.8.1-b-binding.md` The design, in the pull request of the plans, and the deviations of the build.
- `docs/adr/0017-profiles.md` The amendment of 2026-11-25, in the pull request of the plans.
- `docs/adr/0034-the-identity-of-a-test-case.md` The new ADR, in the pull request of the plans.
- `docs/design/0.8.1-a-lifecycle-and-harness.md` The note on D89 item 3, in the pull request of the plans.
- `docs/README.md` The row of the design page, in the pull request of the plans.

## Change sets

The change sets of `.claude/agents/ownership.md` that apply. `architect` is A of each, except where the row names the owner.

- Information model: ADR 0034, a new ADR that amends ADR 0024 (answer of the owner to Q1, 2026-11-27). It comes in with `status: proposed`, and an approval commit of the owner changes only that line. The amendment of ADR 0017 of 2026-11-27 is accepted under the word of the owner of 2026-11-25.
- Diagnostic code, new severity: SD621 in `strict`. `src/diag.rs`, `src/check.rs`, `src/evidence.rs`, `src/config.rs`, `docs/diagnostics.md`, `docs/profiles.md`, the `check` tests and the golden files, and `CHANGELOG.md`. R: `diagnostics-engineer`, and `core-engineer` for `src/config.rs`. `hygiene-engineer` is R for `src/finish.rs`.
- Convention: the `.editorconfig` of `tests/fixtures/runners/`, which turns off the indent check for `*.xml` only. The owner accepted it on 2026-11-27 (Q3). R: `test-engineer`.
- The bind (no row): `src/evidence.rs`, `tests/evidence_reports.rs` and `docs/test-report.md`. R: `diagnostics-engineer`. `test-engineer` is R for `tests/fixtures/runners/`, the module `runners` and `bench/tool/`.

## Texts

Each text below is final. A task that writes it uses these bytes.

The fix of SD621 `by-name` for a fixable case of the row of the defaults of jest-junit:

```text
make the runner write the path of the test file (jest-junit: `addFileAttribute: "true"`; Vitest: the default `classname`)
```

The fix of SD621 `by-name` or `not-unique` for a fixable case of the row of Go:

```text
make gotestsum write the import path (the default `--junitfile-testcase-classname=full`)
```

The fix of SD621 `by-name` for each other case:

```text
no setting of the runner writes that field; keep the `Cargo.toml` or `go.mod` above the test file, or see docs/test-report.md#the-settings-that-give-a-strong-bind
```

The message of a `case:` that is not valid (it replaces the current text, and it also names the second `case:`):

```text
the `case:` of the tag at {at} is not valid: write one `case:"<name>"` in the tag, with the whole name of a case between 2 double quotes and no `"` inside the name
```

## Order

- [ ] t1: Copy the folder `tests/fixtures/runners/` of the work tree `pb-fix` into this repository, with no edit: 72 files, with its `.editorconfig`. The main session gives the full path of the work tree. Keep `generate.sh`: it writes the reports again with the pinned versions, in a folder outside the repository. It keeps the machine out of each report: it writes each path of the run folder as a relative path, gives `time`, `timestamp`, `hostname` and `uuid` fixed values (`0.001`, `2026-01-01T00:00:00` in the form of the runner, `host` and zeros), gives the duration of the Rust harness the value `0.00s`, and runs with `TZ=UTC` and with the settings of the user unset. It then fails when a report still holds the run folder, the repository path, the home folder, `/Users/`, `/home/`, `/private/`, the user name or the host name. Agent: `test-engineer`. Files: `tests/fixtures/runners/**`. The `Files:` is a glob, so SD923 stays on this task: the task copies 72 files with no edit, and `diff -r` checks each of them, so a list of 72 paths gives no better review. Verify: the folder has 72 files; `diff -r` of the 2 folders prints nothing; no report holds `/Users/`, `/home/`, `/private/`, the user name or the host name; each `time` is `0.001`, each `timestamp` starts with `2026-01-01T00:00:00`, and each `hostname` is `host`; `pre-commit run --all-files` passes. Hours: 1.
- [ ] t2: Write the module `runners` of `tests/evidence_reports.rs`, as a red step. A helper makes a git repository from a folder of sources: it gives each `*.fixture` its real name, writes one requirement for each key of a tag, `[evidence] reports`, the report and its stamp, and runs `check --reports --all --json`. One table holds the expected bind of each tag of each report: the result, the case, the fit, `shared` and the cause of SD621. Record the table from the binary of the unchanged tree first (the code of the bind of bec2377 equals 475d8a8), then apply the 15 binds of the 9 rows of the table of section 1 of the design. Add the assertions A1 and A2, and the variants of section 5 of the design: `src/Gate.js` with `jest/reports/classname-describe.xml`, the 3 reports of `namesake/` together, the same with no `py/test_gate.py`, and a copy of `go/reports/relative.xml` with the case `.` `TestAlarms` passed. Make the commit as the skill `tdd` says for a red test. Agent: `test-engineer`, with `diagnostics-engineer` as C. Files: `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports runners::` fails only in these 15 binds and the 3 variants of E2c, E2e and E2f. Hours: 3.
- [ ] t3: Add the named cells of the matrix, as a red step: B2 of part A (`Button` `Button renders` failed, `src/Button.tsx`, and a passed case `Button renders` of the defaults: failed with SD621), B3 of part A (the nextest case `closesv1` passed and the Kotlin case `com.example.ValveV1Test` `closesv1` failed, with no folder of the package: passed, no SD621), E2c in memory (2 `go.mod` files and the case `.`: SD621 `not-unique`), a test outside a group in jest-junit (`" alarms"`), the join ` > ` with a `classname` that is a title, and one pytest stem in 2 rootdirs (`not-unique`). Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: `cargo test --bin spec-dna evidence::matrix` fails only in the new cells. Hours: 2.
- [ ] t4: Add the locator of a case and rule 1 of section 1 of the design (B3): a full locator that names no reading of the tag file makes the case another test in each row. The condition "names a file of the code tree" goes. Rule 1 applies only where the scanner holds the tag side of each reading in full, and not where the row ties the case by the built identity (`ties`). Add the guard A6 of part A as named cells: no cell classifies the own case as another test, with a Go root package with no `go.mod` and an import path as the `classname`, a .NET tag whose namespace the scanner cannot read, and a JVM tag whose chain it cannot read. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the cell B3, the cells of A6 and the variant with no `py/test_gate.py` pass; `cargo test --bin spec-dna evidence::` and `cargo test --test evidence_reports`. Hours: 3.
- [ ] t5: Add rule 2 (a word, B2): a word and `.` never make a case another test by rule 1. In the row of Go, the readings `relative` and `short` (the root package too), and `.` counts the root of each module of the code tree. In the row of pytest, a word counts each module with that stem. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the cells E2c and the pytest stem, and the rows of `go/relative.xml` and `go/short.xml`, pass; `cargo test --bin spec-dna evidence::`. Hours: 3.
- [ ] t6: Add rule 4 (the name of a call): with no group, the head with one space before the title; in the row of the defaults of jest-junit, the joins ` > ` and ` › `; and the title readings, which keep a case with such a `classname` in the set. The reader trims no name. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the cells B2, `" alarms"` and the join pass, and the rows of the jest and Vitest reports and the variant E2e pass; `cargo test --test evidence_reports runners::`. Hours: 2.
- [ ] t7: Add the reference binder of `cfg(test)`: it judges each case of the wide union of 475d8a8 with the rule of t4 to t6. Add the test of the same result (`Proof` and `Bind`: `cases`, `shared`, `case`, `fit`, `why`, `reason`, `near` and the passes) on each cell of the matrix, on each real report and on L8 to L10 with 60,000 cases. Add the counts of the judgments of section 2 of the design, as a red step: they fail with the current read. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the test of the same result passes; the 3 counts fail. Hours: 3.
- [ ] t8: Split the head map and the segment map by the shape class of the case. A tag reads the segment map only for the shape classes of section 2, step 3, of the design, and for a case with a locator only the bucket of the base name of the tag file (B4). Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the counts of L8 and L9 pass, the test of the same result passes, and `index_cost::` passes (at most 67 keys for a name, at most 6 entries for each case). Hours: 4.
- [ ] t9: Judge the cases with no locator once for each signature of the test, and keep their summary for the worst of the set (section 2, step 5, of the design). Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: the count of L10 passes, and the test of the same result passes. Hours: 3.
- [ ] t10: Remove the clock bounds of section 2 of the design: the matrix gets at most 4 judgments for each bind, the long name gets its count of keys, and the bind of 300 files gets at most 2 judgments for each tag. The test with `--ignored` stays. In `tests/evidence_reports.rs`, the bound of the stall test goes from 10 s to 30 s. Agent: `diagnostics-engineer`, with `test-engineer` as C. Files: `src/evidence.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --bin spec-dna evidence::` and `cargo test --test evidence_reports`, and no `Instant` in `src/evidence.rs` outside the test with `--ignored`. Hours: 1.
- [ ] t11: Note 1 of section 3 of the design: a template of the file is a twin when a full title that it can give equals a full title of the tag. The test fails first: the cell of the acceptor (`it.skip('cold opens')` in `describe('Valves')`, and `it.each(['opens'])('%s')` in a nested `describe('cold')`, with only the case `Valves cold opens` passed) gives passed with SD621 `not-unique`. A template in `describe('hot')` stays silent (guard). Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: `cargo test --bin spec-dna evidence::`. Hours: 2.
- [ ] t12: Note 3: a second `case:` word before the next `covers:` of the line makes the `case:` not valid, with the text of the section "Texts". The test fails first: `case:test_x case:test_y` with a passed case `test_x` gives unbound and the text. The quoted form stays not valid, and 2 tags of one line with one `case:` each bind both (guards). Agent: `diagnostics-engineer`. Files: `src/evidence.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --bin spec-dna evidence::` and `cargo test --test evidence_reports`. Hours: 1.
- [ ] t13: Note 4: the class index reads `struct` (C#, also after `record`, `readonly` and `ref`) and `enum` (Java and C#) as declarations. The test fails first: a C# twin `struct ValveTest` and a Java twin `enum ValveTest` in the namespace of the tag give SD621 `not-unique`. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: `cargo test --bin spec-dna evidence::`. Hours: 1.
- [ ] t14: Add the function that tells a rule for a code (in a root or in `[rules]`, by the code or by its group) from the default of the profile. The unit test fails first: `strict` with no rule gives the default, and a rule `SD621 = "error"` or `reports = "warning"` gives the rule. Agent: `core-engineer`. Files: `src/config.rs`. Verify: `cargo test --bin spec-dna config::`. Hours: 1.
- [ ] t15: Item 7: give each weak case the flag fixable of section 4, rule 1, of the design: a weak case of the row of the defaults of jest-junit with the cause `by-name`, or of the row of Go with a word or `.`, whose identity is built and unique. The unit tests fail first: these 2 cases are fixable, and a case of the line format, a case of a pair that no row lists, and a case of the defaults of jest-junit for a test with a twin are not. Agent: `diagnostics-engineer`. Files: `src/evidence.rs`. Verify: `cargo test --bin spec-dna evidence::`. Hours: 2.
- [ ] t16: Write the tests of the level of SD621 in `strict`, as a red step: `jest/reports/default.xml` gives SD621 errors and a warning for `lane-lamp`; `jest/reports/classname-describe.xml` gives a warning for `alarms` (an empty `classname`); `go/reports/relative.xml` gives 2 errors; a weak bind of nextest with no `Cargo.toml` gives a warning, and `check` exits 0 for it; a rule `SD621 = "error"` makes it an error, and a rule `reports = "warning"` makes each line a warning. Make the commit as the skill `tdd` says for a red test. Agent: `diagnostics-engineer`. Files: `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports` fails only in these tests. Hours: 1.
- [ ] t17: Give each line of SD621 its level in `check`: in `strict` with no rule (t14), an error when each weak case of the set is fixable (t15), else a warning. Write the 3 fixes of the section "Texts" and the explain text of SD621. The golden file `check-binds.json` holds the old hint of SD621, so it changes. Agent: `diagnostics-engineer`. Files: `src/check.rs`, `src/diag.rs`, `tests/golden/check-binds.json`. Verify: the tests of t16 pass; `cargo test --test evidence_reports` and `cargo test --test cli`. Hours: 2.
- [ ] t18: Make step 5.4 of `finish` read the level of each line of SD621 from the function of t17. The test fails first: `finish` in `strict` stops on the weak bind of the defaults of jest-junit, and not on the weak bind of nextest with no `Cargo.toml`. Agent: `hygiene-engineer`, with `diagnostics-engineer` as C. Files: `src/finish.rs`, `tests/finish.rs`. Verify: `cargo test --test finish`. Hours: 1.
- [ ] t19: Write `bench/tool/bind.py`: the loads L8, L9 and L10 of section 2 of the design with 600,000 cases, and the paired measure of the design. In each of 5 rounds after one warm-up, it runs the release build of the branch with no tag and with 2,000 tags, one run after the other, in an order that changes from round to round. Each run records with `wait4` the wall time, the CPU time (user and system) and the peak RSS, and the load average before and after the run. The release build of the base runs one round of each load with 2,000 tags, with a limit of 600 s for each run. The result file holds each run, the medians, the load averages and the target. Run it once and commit the result. Add the section to the README. Agent: `test-engineer`. Files: `bench/tool/bind.py`, `bench/tool/README.md`, `bench/results/tool-bind.json`. Verify: for each load, the median over the 5 rounds of the difference of the CPU time (2,000 tags against no tag) is at most 1 s; each round whose load average stays below 2 holds the same limit in wall time; the peak RSS with 2,000 tags is at most 1.25 times the peak RSS with no tag; `pre-commit run --all-files`. Hours: 3.
- [ ] t20: Document the rule in `docs/test-report.md`: the identity of a case with the table of the locators, rules 1, 2 and 4 of section 1 of the design, the new section "The settings that give a strong bind" with the table of section 4, the shapes table (the `-` of nextest is confirmed, the hint of gotestsum `relative` goes), and limits 10, 12, 15, 16 and 17. Write the level of SD621 in `docs/diagnostics.md` and `docs/profiles.md`. Agent: `diagnostics-engineer`. Files: `docs/test-report.md`, `docs/diagnostics.md`, `docs/profiles.md`. Verify: `pre-commit run --all-files`, and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 3.
- [ ] t21: Write the entry of the change, with a part for a project of `strict`: a weak bind that no setting fixes is now a warning, and the results of the reports of gotestsum `relative` and `short` in a tree of 2 modules change. Agent: `diagnostics-engineer`; `release-engineer` edits the file. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t22: Add the deviations of the build to the design page: each row of the table of the real reports that differs from the prediction, each current test that changed with its reason, and the result of the bench. Agent: `architect`. Files: `docs/design/0.8.1-b-binding.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 1.

## Risks

- A current test of the bind can pin a rule that this change removes: a word that reads as another file, or a dotted name of another language that stays a weak link. The task that changes the rule (t4, t5 or t6) gives each such test the new result and names it with its reason in its commit, and t22 lists it. A test that fails for another reason stops the task, and `failure-analyst` finds the cause before a fix.
- The prediction of the real reports is from a lab run of one binary. A row that differs in t2 to t6 stops the build before the table changes.
- Rule 1 makes a case of a runner that no row lists another test when its full locator names no reading of the tag file. A project with such a runner gets `no-case` in place of a weak pass. The entry of the change says so, and `case:` stays the way out.
- The fast binder of t8 and t9 can differ from the reference in a shape that no test has. The test of the same result runs on each cell, each real report and each load, and a difference is fixed in the fast binder. A new shape of a later change must add a cell.
- The machine of the bench is shared, and its load average is often 20 to 80. The wall time of one run is then no proof. t19 checks the target on the CPU time of paired runs, and it writes the load of each run. Contention for the caches can still add to the CPU time under load, which makes the measure stricter, not weaker. The counts of the judgments of t7 to t9 do not depend on the load.
- The level of each line of SD621 changes the stop of `finish` in `strict`. A project with a weak bind that no setting fixes no longer stops there. This is the decision of the owner (item 7).
- The Go reports keep the property `go.version` of the run (the Go version, the system and the processor). It is no path, user, host or time, so `generate.sh` keeps it as the runner wrote it.
- The reports of real runners are fixed bytes of one version of each runner. `generate.sh` writes them again. A new version can change a shape, and the test then fails with the diff of the bind.

## Proof

Each test fails before the code, unless it says "guard".

- `tests/evidence_reports.rs`, module `runners`: each tag of each of the 24 reports has the expected bind, and A1 and A2 hold. The variants: B2 on `jest/reports/classname-describe.xml` with `src/Gate.js`, B3 on the 3 reports of `namesake/` with no `py/test_gate.py`, and E2c on a copy of `go/reports/relative.xml`. In `strict`, the levels of SD621 of section 4 of the design.
- `src/evidence.rs`: the cells B2, B3, E2c, `" alarms"`, the join ` > `, the pytest stem, the template in a nested group, the second `case:`, `struct` and `enum`. A2 on each cell (guard). The test of the same result (guard after t7). The counts of the judgments of L8, L9 and L10.
- `src/config.rs`: the function that tells a rule from the default.
- `tests/evidence_reports.rs` (t16): the levels of SD621 in `strict`, and a rule that gives one level to each line.
- `tests/finish.rs` (t18): `finish` in `strict` stops on a fixable weak bind, and not on another one.
- `bench/results/tool-bind.json`: 2,000 tags on one title add at most 1 s to the load with no tag, for L8, L9 and L10: the median of the paired difference of the CPU time, and the wall time of each round with a load average below 2.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result `accepted`.
