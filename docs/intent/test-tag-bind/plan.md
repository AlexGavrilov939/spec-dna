---
status: approved
---
# A tag binds the test under it: plan

## Files

- `docs/design/0.8.1-a-evidence-and-context.md` The design of the bind (D82) and of the code tree (D85).
- `docs/design/0.8.1-a-lifecycle-and-harness.md` The design of the hook of the tests (D89).
- `docs/adr/0024-test-report-as-evidence.md` The amendment of 2026-11-24 (slice A0 of the design).
- `src/evidence.rs` The test under a tag, the forms of a test and of a name, the file rule, the shared cases, the strength of a bind and the reasons of a tag with no passed case.
- `src/report.rs` The paths of the code tree for the bind, and the text of SD614 for changed bytes.
- `src/diag.rs` The code SD621.
- `src/config.rs` The profile row of SD621 and its group.
- `src/check.rs` SD621 as a state code, and the bind of each full tag in the report of check.
- `src/lifecycle.rs` SD621 as a new finding in the diff that deletes the folder of a change.
- `src/finish.rs` SD621 at step 5.4 of finish.
- `src/context.rs` The test under each tag in the output of show, and the field binds of show.
- `src/main.rs` The option of show, and the field binds of the JSON output of check.
- `src/json.rs` The writer of the field binds, if the field needs a new helper.
- `src/detect.rs` The scan of the pre-commit configuration for a hook that runs the tests.
- `src/init.rs` The note of init for a test hook that does not stamp.
- `src/upgrade.rs` The note of upgrade for a test hook that does not stamp.
- `tests/evidence_reports.rs` The tests of the bind, of the runners, of the code tree, of SD621 and of the stamp.
- `tests/accept_cmd.rs` The proof states of the skeleton of accept.
- `tests/cli.rs` The tests of show and of the JSON output of check.
- `tests/golden/show-requirement.json` The field binds with no report.
- `tests/golden/show-binds.json` The output of show with the reports (new).
- `tests/golden/check-binds.json` The field binds of the JSON output of check (new).
- `tests/init_change.rs` The note of init.
- `tests/upgrade_cli.rs` The note of upgrade.
- `docs/test-report.md` The new reference page of the test reports.
- `docs/config.md` The sections of the test reports and of the proof levels move to the new page.
- `docs/guides/change.md` A link to the new page replaces the copy of the bind rule.
- `docs/diagnostics.md` SD621, the reasons of SD616 and SD617, the text of SD614, and the heading of the test reports.
- `docs/profiles.md` The row of SD621.
- `docs/commands.md` The sections of check and show.
- `docs/init.md` The note of a test hook.
- `docs/README.md` The line of the new page in the map. The main session adds it.
- `.claude/agents/ownership.md` The row of the new page.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Bind a tag to the test directly under it: the scan, the forms of a test, the name forms, a case that names 2 tests, and the reasons no-test, group and template (D82 items 1 to 5). Add the test of the defect and the proof state of the skeleton of accept. Files: `src/evidence.rs`, `tests/evidence_reports.rs`, `tests/accept_cmd.rs`. Verify: `cargo test --bin spec-dna evidence::` and `cargo test --test evidence_reports --test accept_cmd`. Hours: 6.
- [ ] t2: Fit a case to the file of the tag by its file or classname, with the precedence, the shared cases, the exact `case:` and the strength of a bind (D82 items 6 to 10). Add the fixtures of the runners and their twins. The Jest case of the shared runner list takes the form with a file. Files: `src/evidence.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --bin spec-dna evidence::` and `cargo test --test evidence_reports`. Hours: 8.
- [ ] t3: Bind no case to a tag in a file outside the code tree, with the reason outside-tree (D85). Change the text of SD614 for changed bytes so that it names the hook, and test the setup of the hook (D89). Files: `src/evidence.rs`, `src/report.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports`. Hours: 3.
- [ ] t4: Add the code SD621, its profile row in the group reports and its row in the profiles page. Files: `src/diag.rs`, `src/config.rs`, `docs/profiles.md`. Verify: `cargo test --bin spec-dna diag::` and `cargo test --bin spec-dna config::`. Hours: 1.
- [ ] t5: Report SD621 as a state code in `check --reports`, and keep the bind of each full tag in the report of check. Test the defaults of jest-junit, the form with a file and the exact `case:`. Files: `src/check.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports`. Hours: 3.
- [ ] t6: Report SD621 as new where SD617 runs: the diff that deletes the folder of a change, and step 5.4 of `finish`. Files: `src/lifecycle.rs`, `src/finish.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports`. Hours: 3.
- [ ] t7: Show the test under each full tag in the evidence line of `show`, and add the field binds to `show --json` with the result not-read. Files: `src/context.rs`, `tests/cli.rs`, `tests/golden/show-requirement.json`. Verify: `cargo test --test cli`. Hours: 3.
- [ ] t8: Add the option `--reports` to `show`: read the reports as `check --reports` does, and make `--reports` with `--conventions` a usage error. Files: `src/main.rs`, `tests/cli.rs`, `tests/golden/show-binds.json`. Verify: `cargo test --test cli` and `cargo test --test evidence_reports`. Hours: 3.
- [ ] t9: Add the top-level field binds to `check --reports --json`, after the field approvals. Files: `src/main.rs`, `tests/cli.rs`, `tests/golden/check-binds.json`. Verify: `cargo test --test cli`. Hours: 2.
- [ ] t10: Make `init` print the note for a pre-commit hook that runs the tests and does not stamp. Files: `src/detect.rs`, `src/init.rs`, `tests/init_change.rs`. Verify: `cargo test --test init_change`. Hours: 3.
- [ ] t11: Make `upgrade` print the same note, and document the note. Files: `src/upgrade.rs`, `tests/upgrade_cli.rs`, `docs/init.md`. Verify: `cargo test --test upgrade_cli`. Hours: 2.
- [ ] t12: Write the page of the test reports with the rule of D82, the code tree of D85 and the hook of D89. Move the 2 sections of the configuration page to it, link the section stats of the commands page from the section of the proof levels, and link the page from the change guide. Files: `docs/test-report.md`, `docs/config.md`, `docs/guides/change.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 4.
- [ ] t13: Document SD621, the reasons of SD616 and SD617, the text of SD614, the new heading of the test reports and its links, and the sections of check and show. Files: `docs/diagnostics.md`, `docs/commands.md`, `docs/test-report.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [ ] t14: Add the line of the page to the map and its row to the ownership file, and write the entry of the change. Files: `docs/README.md`, `.claude/agents/ownership.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t15: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish test-tag-bind --dry-run`. Hours: 3.

## Risks

- A layout of a runner that the design did not read binds by name only, or gives no-case. SD621 and the reasons make each such tag visible, and `case:` is the escape.
- The new rule leaves requirements of a current corpus unproven. That is the fix of a false proof. The entry of `CHANGELOG.md` gives the steps, and only the `finish` of a change that touches such a requirement stops.
- 3 changes of wave 1 edit `src/lifecycle.rs`, `src/finish.rs`, `src/check.rs`, `src/diag.rs`, `src/context.rs`, `src/upgrade.rs` and some pages. The change that merges second rebases and runs each check again.
- The name forms of item 4 can give a false bind, for example a parameter part that holds the name of another test. The shared set of item 7 and the unit tests of the table limit it.

## Proof

Each test fails before the fix, unless it says "guard". The design pages give the fixtures, and `tests/evidence_reports.rs` holds the fixtures of 7 runners.

- `tests/evidence_reports.rs`, the defect: a tag on line 1 of a Vitest file above an import line, and only the passed case `Pumps > stops it` of another file. `check --reports` gives SD617 with the reason no-test. Before the fix: no finding, measured (E22).
- A `classname` that is the path of another file: SD617 no-case. The precedence: a skipped case of the file of the tag and a passed line with no file give SD617 "was skipped".
- The runners pytest `xunit2`, cargo-nextest (`app::valves`), Vitest, Go (go-junit-report), JUnit 5 (Surefire) and PHPUnit: a passed case proves the requirement with no SD621 (guard), and a skipped case gives SD617 "was skipped" (guard). A passed twin of the same name in a second file gives SD617: it fails before the fix for nextest, Vitest and Go, and it is a guard for pytest, JUnit 5 and PHPUnit.
- jest-junit with its defaults: the bind counts and gives SD621, a warning in `standard` and an error in `strict`. The same report with `file`, and an exact `case:` with one such case: no SD621 (guard).
- Shared names in one file (2 classes), and jest-junit twins in 2 files: SD617 with the shared reason.
- `case:closes` with the case `Valves closes`: SD617 no-case, and the message names `Valves closes`. `case:"TestTimeout/no_reply"` and `case:"testOne with data set #0"`: bound, and no SD105.
- A tag above `describe(...)`: the reason group. A tag above `it.each(rows)(...)`: the reason template.
- Guards: a multi-line `@pytest.mark.parametrize(...)`, the attributes `#[test]` and `#[should_panic(...)]`, the annotations `@Test` and `@DisplayName(...)`, a Kotlin and a C# test on one line, and a tag on the line of the definition are bound. A tag in the body of a test gives no-test.
- The code tree: an untracked test file with a passed case and a stamp gives SD617 outside-tree. An edit of that file after the stamp still gives SD617 (issue 113). A tracked file that `tree-exclude` matches gives SD617 outside-tree. After `git add`, a new report and a stamp: no SD617 (guard). An untracked twin keeps its own file (guard).
- The hook: a report written again after the stamp gives SD614 with "one hook entry". The entry of the setup keeps the stamp fresh (guard).
- `tests/accept_cmd.rs`: the skeleton for the tag of the defect has the state tagged and no case.
- `tests/cli.rs`: `show` gives `[test <name>]` and the result not-read. The golden files of `show --reports --json` and `check --reports --json` hold the field binds of the examples of D82.
- `tests/init_change.rs` and `tests/upgrade_cli.rs`: the note names the id of a test hook that does not stamp. A hook with `evidence stamp` in its entry gives no note (guard).
- The unit tests of `src/evidence.rs` hold the table of the name forms: each example of item 4 names its test, and `test_x_slow` does not name `test_x`.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
