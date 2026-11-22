//! Integration tests of the test report as evidence (design 0.8.0, D1):
//! `spec-dna evidence stamp`, `check --reports`, the bind of a `covers:` tag to
//! a test case, SD613 to SD617, and the step 5 of `finish`.
//!
//! Each fixture is a pull request. The base commit is on `main`, and it holds
//! the folder of the change `dose`, with a delta line that adds the requirement
//! `requirements/valves#flush`. The branch `feature` adds the requirement, the
//! tests and deletes the folder, as the pull request of `finish` does. The
//! reports are in `reports/`, which git ignores. The 5 reports of the runners
//! are small neutral examples of the files that the runners write.

mod common;

use common::*;
use sha2::{Digest, Sha256};

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const FLUSH: &str = "- **flush**: The controller SHALL flush the line.\n";
const FOLDER: &str = "docs/intent/dose";
const TAG: &str = "requirements/valves#flush";

fn config(profile: &str, evidence: &str) -> String {
    config_with(profile, "", evidence)
}

/// The configuration, with the lines `rules` added to `[rules]`.
fn config_with(profile: &str, rules: &str, evidence: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n{rules}\n[evidence]\n{evidence}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\nstale-days = 14\n"
    )
}

const REPORTS: &str = "reports = [\"reports/*\"]\n";

/// The pull request: the base on `main`, then the branch `feature` with the
/// requirement, the test files and no folder of the change. The result has
/// no report yet.
fn pr(profile: &str, evidence: &str, tests: &[(&str, &str)]) -> Repo {
    pr_with(profile, "", evidence, tests)
}

/// Like `pr`, with the lines `rules` added to `[rules]`.
fn pr_with(profile: &str, rules: &str, evidence: &str, tests: &[(&str, &str)]) -> Repo {
    pr_in(&config_with(profile, rules, evidence), tests)
}

/// Like `pr`, with the text of the configuration.
fn pr_in(config: &str, tests: &[(&str, &str)]) -> Repo {
    pr_class_in("higher-risk", config, tests)
}

/// Like `pr_in`, with the `class` of the intent.
fn pr_class_in(class: &str, config: &str, tests: &[(&str, &str)]) -> Repo {
    let r = Repo::with_config(config);
    r.write(".gitignore", "reports/\n");
    r.write("docs/requirements/valves.md", VALVES);
    r.write("src/valves.py", "x = 1\n");
    r.write("tests/test_other.py", "# nothing\n");
    r.write(
        &format!("{FOLDER}/intent.md"),
        &format!(
            "---\nstatus: accepted\nclass: {class}\n---\n# Dose\n\n## Intent\n\nFlush the line.\n"
        ),
    );
    r.write(
        &format!("{FOLDER}/plan.md"),
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
    );
    r.write(
        &format!("{FOLDER}/spec.md"),
        &format!(
            "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD {TAG}: The controller SHALL flush the line.\n"
        ),
    );
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/requirements/valves.md", &format!("{VALVES}{FLUSH}"));
    for (path, text) in tests {
        r.write(path, text);
    }
    std::fs::remove_dir_all(r.dir.join(FOLDER)).unwrap();
    r.commit("finish");
    r
}

/// The text of a pytest report (`junit_family = xunit2`): a `classname` and a
/// `name`, with no `file`. `child` is the element in the case.
fn junit(suite: &str, cases: &[(&str, &str, &str)]) -> String {
    let body: String = cases
        .iter()
        .map(|(cn, name, child)| {
            format!("<testcase classname=\"{cn}\" name=\"{name}\" time=\"0.01\">{child}</testcase>")
        })
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><testsuites><testsuite name=\"{suite}\" errors=\"0\" failures=\"0\" skipped=\"0\" tests=\"{}\" time=\"0.1\">{body}</testsuite></testsuites>",
        cases.len()
    )
}

const SKIPPED: &str = "<skipped type=\"skip\" message=\"later\"/>";
const FAILED: &str = "<failure message=\"assert False\">trace</failure>";

/// The test of a pytest file with the tag above `test_closes_on_timeout`.
const PY_TEST: &str =
    "# covers: requirements/valves#flush\ndef test_closes_on_timeout():\n    assert True\n";

fn stamp(r: &Repo) {
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
}

fn check(r: &Repo, extra: &[&str]) -> (i32, String) {
    let mut args = vec!["check", "--reports", "--base", "main"];
    args.extend(extra);
    let (code, out, err) = r.run(&args);
    (code, format!("{out}{err}"))
}

fn lines<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines()
        .filter(|l| {
            l.contains(&format!(": error {code} ")) || l.contains(&format!(": warning {code} "))
        })
        .collect()
}

fn has(out: &str, code: &str) -> bool {
    !lines(out, code).is_empty()
}

/// A pull request with one pytest report that holds `child` for the case.
fn pytest_pr(profile: &str, child: &str) -> Repo {
    pytest_pr_with(profile, "", child)
}

/// Like `pytest_pr`, with the lines `rules` added to `[rules]`.
fn pytest_pr_with(profile: &str, rules: &str, child: &str) -> Repo {
    let r = pr_with(
        profile,
        rules,
        REPORTS,
        &[("tests/test_valves.py", PY_TEST)],
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", child)],
        ),
    );
    stamp(&r);
    r
}

// The proof of a requirement of a change (SD617)

#[test]
fn a_skipped_case_gives_sd617_and_a_passed_case_gives_none() {
    let r = pytest_pr("strict", SKIPPED);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(": error SD617 "), "{out}");
    assert!(found[0].contains("`requirements/valves#flush`"), "{out}");
    assert!(
        found[0].contains("`test_closes_on_timeout` was skipped"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // The same tree with the case passed.
    let r = pytest_pr("strict", "");
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_failed_case_gives_sd617_and_names_the_case() {
    let r = pytest_pr("standard", FAILED);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`test_closes_on_timeout` failed"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_case_that_passed_after_a_rerun_counts_as_a_fail() {
    let r = pytest_pr("standard", "<flakyFailure message=\"x\"/>");
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
}

#[test]
fn without_the_flag_the_tag_is_the_evidence_as_in_0_7_1() {
    let r = pytest_pr("strict", SKIPPED);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD617") && !has(&out, "SD613"), "{out}");
}

#[test]
fn a_verified_by_marker_keeps_its_meaning_and_needs_no_report() {
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "docs/requirements/valves.md",
        &format!("{VALVES}- **flush**: The controller SHALL flush the line. <!-- verified-by: tests/test_other.py -->\n"),
    );
    r.commit("verified");
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &["--all"]);
    assert!(!has(&out, "SD617") && !has(&out, "SD616"), "{out}");
    assert_eq!(code, 0, "{out}");
}

// The 5 runners: the first proof of the bind rule

/// One runner: the test file with its tag, the report text for a case that
/// ran in the outcome `child`, and the name that a skipped case has.
struct Runner {
    name: &'static str,
    path: &'static str,
    test: &'static str,
    report: fn(&str) -> String,
}

fn runners() -> Vec<Runner> {
    vec![
        Runner {
            name: "pytest xunit2",
            path: "tests/test_valves.py",
            test: PY_TEST,
            report: |c| {
                junit(
                    "pytest",
                    &[("tests.test_valves", "test_closes_on_timeout", c)],
                )
            },
        },
        Runner {
            name: "cargo-nextest",
            path: "tests/valves.rs",
            test: "// covers: requirements/valves#flush\n#[test]\nfn closes() {\n    assert!(true);\n}\n",
            report: |c| {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites name=\"nextest-run\" tests=\"1\" failures=\"0\" errors=\"0\" time=\"0.1\"><testsuite name=\"valves\" tests=\"1\" disabled=\"0\" errors=\"0\" failures=\"0\"><testcase name=\"tests::closes\" classname=\"valves\" time=\"0.01\">{c}</testcase></testsuite></testsuites>"
                )
            },
        },
        Runner {
            name: "Jest",
            path: "tests/valves.test.js",
            test: "// covers: requirements/valves#flush\nit('closes on timeout', () => {\n  expect(1).toBe(1);\n});\n",
            report: |c| {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites name=\"jest tests\" tests=\"1\" failures=\"0\" errors=\"0\" time=\"0.5\"><testsuite name=\"Valves\" errors=\"0\" failures=\"0\" skipped=\"0\" time=\"0.3\" tests=\"1\"><testcase classname=\"Valves closes on timeout\" name=\"Valves closes on timeout\" time=\"0.002\">{c}</testcase></testsuite></testsuites>"
                )
            },
        },
        Runner {
            name: "Go",
            path: "valves/sensor_test.go",
            test: "package valves\n\n// covers: requirements/valves#flush\nfunc TestTimeout(t *testing.T) {\n\tt.Run(\"no_reply\", func(t *testing.T) {})\n}\n",
            report: |c| {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites tests=\"2\"><testsuite name=\"example.org/valves\" tests=\"2\" failures=\"0\" errors=\"0\" id=\"0\"><testcase name=\"TestTimeout\" classname=\"example.org/valves\" time=\"0.00\">{c}</testcase><testcase name=\"TestTimeout/no_reply\" classname=\"example.org/valves\" time=\"0.00\">{c}</testcase></testsuite></testsuites>"
                )
            },
        },
        Runner {
            name: "JUnit Java",
            path: "src/test/java/com/example/ValveTest.java",
            test: "package com.example;\n\nclass ValveTest {\n    // covers: requirements/valves#flush\n    @Test\n    void flushesFirst() {\n        assertTrue(true);\n    }\n}\n",
            report: |c| {
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuite name=\"com.example.ValveTest\" tests=\"1\" errors=\"0\" skipped=\"0\" failures=\"0\" time=\"0.01\"><testcase name=\"flushesFirst\" classname=\"com.example.ValveTest\" time=\"0.01\">{c}</testcase></testsuite>"
                )
            },
        },
    ]
}

#[test]
fn a_tag_above_the_test_binds_to_its_case_in_the_report_of_each_runner() {
    for run in runners() {
        // Passed: the tag proves the requirement.
        let r = pr("strict", REPORTS, &[(run.path, run.test)]);
        r.write("reports/r.xml", &(run.report)(""));
        stamp(&r);
        let (code, out) = check(&r, &[]);
        assert!(!has(&out, "SD617"), "{}: {out}", run.name);
        assert_eq!(code, 0, "{}: {out}", run.name);
        // Skipped: the tag is bound, so the message names the skipped case.
        let r = pr("strict", REPORTS, &[(run.path, run.test)]);
        r.write("reports/r.xml", &(run.report)(SKIPPED));
        stamp(&r);
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{}: {out}", run.name);
        assert!(found[0].contains("was skipped"), "{}: {out}", run.name);
        assert_eq!(code, 1, "{}: {out}", run.name);
    }
}

// The bind

#[test]
fn a_tag_with_no_case_in_the_report_is_not_bound_and_the_message_names_case() {
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_valves", "test_unrelated", "")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("binds to the tag at tests/test_valves.py:1"),
        "{out}"
    );
    assert!(found[0].contains("case:<name>"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_with_case_binds_to_the_case_that_it_names() {
    // The block of the tag has no test word, so only `case:` binds.
    let test = "# covers: requirements/valves#flush case:\"closes on timeout\"\nVALUE = 1\n";
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", test)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_valves", "closes on timeout", "")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD617") && !has(&out, "SD105"), "{out}");
    assert_eq!(code, 0, "{out}");
    // Without the name, the same report binds nothing.
    let test = "# covers: requirements/valves#flush\nVALUE = 1\n";
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", test)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_valves", "closes on timeout", "")]),
    );
    stamp(&r);
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
}

#[test]
fn a_tag_above_a_function_that_is_no_test_does_not_bind_to_the_next_test() {
    let test = "# covers: requirements/valves#flush\ndef check_timeout():\n    assert True\n\ndef test_next():\n    assert True\n";
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", test)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_valves", "test_next", "")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_in_the_body_of_a_skipped_test_does_not_bind_to_the_next_test() {
    let test = "@pytest.mark.skip\ndef test_a():\n    # covers: requirements/valves#flush\n    assert True\n\ndef test_b():\n    assert True\n";
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", test)]);
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[
                ("tests.test_valves", "test_a", SKIPPED),
                ("tests.test_valves", "test_b", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_classname_binds_a_case_to_its_own_test_file() {
    // `test_a.py` and `test_b.py` both have `test_init`. Only the first has
    // the tag. Its case is skipped, and the case of the other file passed.
    let tag = "# covers: requirements/valves#flush\ndef test_init():\n    assert True\n";
    let plain = "def test_init():\n    assert True\n";
    let r = pr(
        "strict",
        REPORTS,
        &[("tests/test_a.py", tag), ("tests/test_b.py", plain)],
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[
                ("tests.test_a", "test_init", SKIPPED),
                ("tests.test_b", "test_init", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_init` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

// The reports: SD613 to SD615

#[test]
fn a_committed_report_and_stamp_give_sd615() {
    let r = pr(
        "strict",
        "reports = [\"out/*.xml\"]\n",
        &[("tests/test_valves.py", PY_TEST)],
    );
    r.write(
        "out/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        ),
    );
    stamp(&r);
    // The report and its stamp are in a commit: a hand-written report can be.
    r.commit("report");
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD615");
    assert!(
        found
            .iter()
            .any(|l| l.contains("git tracks `out/junit.xml`")),
        "{out}"
    );
    assert!(
        found
            .iter()
            .any(|l| l.contains("git tracks `out/junit.xml.stamp`")),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // `evidence stamp` refuses it too.
    let (code, _, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("git tracks"), "{err}");
}

#[test]
fn no_report_and_no_stamp_give_sd613() {
    let r = pr("standard", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD613");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("no file matches"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A report with no stamp.
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        ),
    );
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD613");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("has no stamp"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_code_edit_after_the_stamp_gives_sd614_and_a_markdown_edit_does_not() {
    // The file of the archive has no record text, so the group `acceptance` is off.
    let r = pytest_pr_with("standard", "acceptance = \"off\"\n", "");
    let (code, out) = check(&r, &[]);
    assert_eq!(code, 0, "{out}");
    // A Markdown edit, and an edit of the record: the tree is the same.
    r.write("README.md", "# A new readme\n");
    r.write(
        "docs/requirements/notes.md",
        "---\nstatus: active\n---\n# Notes\n",
    );
    r.write(".spec-dna/accepted/x.toml", "a = 1\n");
    r.commit("docs");
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD614"), "{out}");
    assert_eq!(code, 0, "{out}");
    // A code edit, committed or not.
    r.write("src/valves.py", "x = 2\n");
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD614");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("names the code tree"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A stamp again makes it fresh.
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert_eq!(code, 0, "{out}");
    // A change of the report bytes after the stamp.
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", FAILED)],
        ),
    );
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD614");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("bytes of the report"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_cut_report_and_a_report_with_a_dtd_give_sd615_and_no_stamp() {
    let whole = junit(
        "pytest",
        &[("tests.test_valves", "test_closes_on_timeout", "")],
    );
    let cut = &whole[..whole.len() - 40];
    let dtd = format!("<!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]>{whole}");
    for (what, text) in [("cut", cut.to_string()), ("dtd", dtd)] {
        let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
        r.write("reports/junit.xml", &text);
        let (code, out, err) = r.run(&["evidence", "stamp"]);
        assert_eq!(code, 1, "{what}: {out}{err}");
        assert!(!r.dir.join("reports/junit.xml.stamp").exists(), "{what}");
        // A stamp that a person wrote does not help: the report stays unreadable.
        r.write(
            "reports/junit.xml.stamp",
            "spec-dna-stamp 1\ntree 0000000000000000\nreport 0000000000000000\n",
        );
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD615");
        assert_eq!(found.len(), 1, "{what}: {out}");
        assert!(
            found[0].contains("`reports/junit.xml` is unreadable"),
            "{what}: {out}"
        );
        assert_eq!(code, 1, "{what}: {out}");
    }
}

#[test]
fn a_report_of_an_unknown_format_gives_sd615() {
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write("reports/out.txt", "ok 1 - something\n");
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("unknown format"), "{err}");
}

// The line format

#[test]
fn the_line_format_is_a_report_for_any_runner() {
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "reports/cases.txt",
        "spec-dna-report 1\n# a comment\nskip\ttests/test_valves.py\ttest_closes_on_timeout\n",
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
    r.write(
        "reports/cases.txt",
        "spec-dna-report 1\npass\ttests/test_valves.py\ttest_closes_on_timeout\n",
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    // A case of another file does not bind.
    r.write(
        "reports/cases.txt",
        "spec-dna-report 1\npass\ttests/test_other.py\ttest_closes_on_timeout\n",
    );
    stamp(&r);
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
}

// The state code SD616

#[test]
fn sd616_is_a_state_code_of_check_all() {
    // The requirement is active in the base, and it has a skipped test.
    let r = Repo::with_config(&config("standard", REPORTS));
    r.write(".gitignore", "reports/\n");
    r.write("docs/requirements/valves.md", &format!("{VALVES}{FLUSH}"));
    r.write("tests/test_valves.py", PY_TEST);
    r.commit_base_of_0_7_1("base");
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    // Never new: `check` prints it with `--all` only.
    let (code, out, err) = r.run(&["check", "--reports"]);
    assert!(!has(&out, "SD616"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["check", "--reports", "--all"]);
    let found = lines(&out, "SD616");
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(
        found[0].contains(": warning SD616 requirement-unproven"),
        "{out}"
    );
    assert!(found[0].contains("`requirements/valves#flush`"), "{out}");
    assert_eq!(code, 0, "a warning in `standard`: {out}{err}");
}

#[test]
fn sd616_is_an_error_in_strict_and_off_in_lite() {
    for (profile, want) in [("strict", Some("error")), ("lite", None)] {
        let r = Repo::with_config(&config(profile, REPORTS));
        r.write(".gitignore", "reports/\n");
        r.write("docs/requirements/valves.md", &format!("{VALVES}{FLUSH}"));
        r.write("tests/test_valves.py", PY_TEST);
        r.commit_base_of_0_7_1("base");
        r.write(
            "reports/junit.xml",
            &junit(
                "pytest",
                &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
            ),
        );
        stamp(&r);
        let (code, out, err) = r.run(&["check", "--reports", "--all"]);
        match want {
            Some(sev) => {
                assert!(
                    out.contains(&format!(": {sev} SD616 ")),
                    "{profile}: {out}{err}"
                );
                assert_eq!(code, 1, "{profile}: {out}{err}");
            }
            None => {
                assert!(!has(&out, "SD616"), "{profile}: {out}{err}");
                assert_eq!(code, 0, "{profile}: {out}{err}");
            }
        }
    }
}

// The profiles and the key

#[test]
fn lite_turns_each_code_off_and_a_rule_for_the_group_turns_them_on() {
    let r = pr("lite", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    // No report and no stamp: no SD613 in `lite`.
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD613"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The group `reports` turns the codes on. A rule of the group `evidence`
    // of 0.7.1 does not.
    let on = |rules: &str| {
        let text = r
            .read("spec-dna.toml")
            .replace("[rules]\n", &format!("[rules]\n{rules}"));
        r.write("spec-dna.toml", &text);
    };
    on("evidence = \"error\"\n");
    let (_, out) = check(&r, &[]);
    assert!(!has(&out, "SD613"), "{out}");
    on("reports = \"error\"\n");
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD613"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn with_no_reports_key_the_flag_changes_nothing_in_each_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = pr(profile, "", &[("tests/test_valves.py", PY_TEST)]);
        let plain = r.run(&["check", "--base", "main", "--all"]);
        let flagged = r.run(&["check", "--reports", "--base", "main", "--all"]);
        assert_eq!(plain, flagged, "{profile}");
        assert!(!plain.1.contains("SD61"), "{profile}: {}", plain.1);
        // An empty list is the same.
        let r = pr(
            profile,
            "reports = []\n",
            &[("tests/test_valves.py", PY_TEST)],
        );
        let flagged = r.run(&["check", "--reports", "--base", "main", "--all"]);
        assert_eq!(plain.0, flagged.0, "{profile}");
        assert!(!flagged.1.contains("SD61"), "{profile}: {}", flagged.1);
    }
}

#[test]
fn the_flag_does_not_combine_with_staged_and_belongs_to_check() {
    let r = pytest_pr("standard", "");
    let (code, out, err) = r.run(&["check", "--reports", "--staged"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("--staged"), "{err}");
    let (code, _, err) = r.run(&["context", "requirements/valves", "--reports"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("option of `check`"), "{err}");
}

#[test]
fn the_config_keys_are_checked() {
    let r = Repo::with_config(
        "[evidence]\nreports = \"junit.xml\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.commit("x");
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("evidence.reports"), "{err}");
    let r = Repo::with_config(
        "[evidence]\ntree-exclude = [1]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.commit("x");
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("evidence.tree-exclude"), "{err}");
}

// `evidence stamp`

#[test]
fn evidence_stamp_writes_the_stamp_and_prints_the_counts() {
    let r = pr("standard", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[
                ("tests.test_valves", "test_a", ""),
                ("tests.test_valves", "test_b", ""),
                ("tests.test_valves", "test_c", FAILED),
                ("tests.test_valves", "test_d", SKIPPED),
            ],
        ),
    );
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    let stamp_text = r.read("reports/junit.xml.stamp");
    let mut it = stamp_text.lines();
    assert_eq!(it.next(), Some("spec-dna-stamp 1"));
    let tree = it
        .next()
        .unwrap()
        .strip_prefix("tree ")
        .unwrap()
        .to_string();
    assert!(it.next().unwrap().starts_with("report "), "{stamp_text}");
    assert_eq!(tree.len(), 16);
    assert_eq!(
        out.trim(),
        format!("stamped 1 report(s) at tree {tree}: 2 pass, 1 fail, 1 skip")
    );
    // JSON.
    let (code, out, err) = r.run(&["evidence", "stamp", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"stamped\":1") && out.contains("\"fail\":1"),
        "{out}"
    );
    assert!(out.contains(&format!("\"tree\":\"{tree}\"")), "{out}");
    assert!(out.contains("\"format\":\"junit\""), "{out}");
}

#[test]
fn the_tree_of_a_clean_checkout_equals_the_hash_of_ls_tree() {
    let r = pr("standard", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_valves", "test_a", "")]),
    );
    let (_, out, _) = r.run(&["evidence", "stamp"]);
    let tree = out
        .split("at tree ")
        .nth(1)
        .unwrap()
        .split(':')
        .next()
        .unwrap()
        .to_string();
    // The paths of the code tree: no Markdown, no tool file, no folder of an
    // intent, and no file of `.spec-dna`.
    let listing = r.git_out(&["ls-tree", "-r", "HEAD"]);
    let mut entries: Vec<(String, String, String)> = listing
        .lines()
        .map(|l| {
            let (meta, path) = l.split_once('\t').unwrap();
            let mut m = meta.split(' ');
            (
                m.next().unwrap().to_string(),
                m.nth(1).unwrap().to_string(),
                path.to_string(),
            )
        })
        .filter(|(_, _, p)| {
            !p.ends_with(".md")
                && !p.starts_with(".spec-dna/")
                && !p.starts_with("docs/intent/")
                && !p.starts_with("spec-dna")
        })
        .collect();
    entries.sort_by(|a, b| a.2.as_bytes().cmp(b.2.as_bytes()));
    let mut text = String::from("spec-dna-tree 1\n");
    for (mode, oid, path) in &entries {
        text.push_str(&format!("{mode} {oid}\t{path}\n"));
    }
    let digest = Sha256::digest(text.as_bytes());
    let want: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(tree, want, "{text}");
}

#[test]
fn evidence_stamp_has_exit_codes_for_each_failure() {
    // 3: no key.
    let r = pr("standard", "", &[("tests/test_valves.py", PY_TEST)]);
    let (code, _, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("[evidence] reports"), "{err}");
    // 1: no file matches.
    let r = pr("standard", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    let (code, _, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("no file matches"), "{err}");
    // 2: no subcommand.
    let (code, _, err) = r.run(&["evidence"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn a_stamp_file_is_never_a_report() {
    // The glob `reports/*` matches the stamp too. The stamp is no report.
    let r = pytest_pr("standard", "");
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("stamped 1 report(s)"), "{out}");
}

// `finish`

#[test]
fn finish_stops_at_step_5_without_a_stamp_and_with_a_skipped_case() {
    let setup = || {
        let r = Repo::with_config(&config("standard", REPORTS));
        r.write(".gitignore", "reports/\n");
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/valves.py", "x = 1\n");
        r.write(
            &format!("{FOLDER}/intent.md"),
            "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nFlush the line.\n",
        );
        r.write(
            &format!("{FOLDER}/plan.md"),
            "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        );
        r.write(
            &format!("{FOLDER}/spec.md"),
            &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD {TAG}: The controller SHALL flush the line.\n"),
        );
        r.commit_base_of_0_7_1("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r.write("tests/test_valves.py", PY_TEST);
        r.commit("test");
        r
    };
    // No report and no stamp: SD613 at step 5. Nothing changes.
    let r = setup();
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5"), "{err}");
    assert!(out.contains("SD613"), "{out}");
    assert!(r.dir.join(FOLDER).exists());
    // A skipped case: SD617 at step 5.
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5"), "{err}");
    assert!(
        out.contains("SD617") && out.contains("was skipped"),
        "{out}"
    );
    assert!(r.dir.join(FOLDER).exists());
    // A code edit after the stamp: SD614.
    r.write("src/valves.py", "x = 3\n");
    let (code, out, _) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("SD614"), "{out}");
    r.git(&["checkout", "-q", "src/valves.py"]);
    // The case passed: `finish` goes on.
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        ),
    );
    stamp(&r);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
}

// `init`

#[test]
fn the_ci_job_of_init_has_no_reports_flag() {
    // The job runs no tests, and `init` does not know the test command.
    for profile in ["standard", "strict"] {
        let r = Repo::new();
        r.git(&["config", "user.email", "owner@example.test"]);
        let answers = r.dir.join("../answers-reports-ci.toml");
        std::fs::write(
            &answers,
            format!("profile = \"{profile}\"\nci = \"github\"\nbase = \"main\"\n"),
        )
        .unwrap();
        let (code, out, err) = r.run(&["init", "--answers", answers.to_str().unwrap()]);
        let _ = std::fs::remove_file(&answers);
        assert_eq!(code, 0, "{out}{err}");
        let job = r.read(".github/workflows/spec-dna.yml");
        assert!(job.contains("spec-dna check --base"), "{job}");
        assert!(!job.contains("--reports"), "{job}");
        assert!(!job.contains("evidence stamp"), "{job}");
    }
}

// SD916 and SD617 are 2 checks of one function: each follows its own code

#[test]
fn a_rule_that_turns_sd916_off_does_not_turn_sd617_off_and_the_reverse() {
    // With no key, `changes = "off"` keeps the output of 0.7.1: no SD916, and
    // no SD617 (no report is read).
    let r = pr_with("standard", "changes = \"off\"\n", "", &[]);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD916") && !has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The requirement has no tag, and the code is on: SD916, and no SD617.
    let r = pr("standard", REPORTS, &[]);
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD916") && !has(&out, "SD617"), "{out}");
    // With the key and SD916 off, a skipped case still gives SD617.
    let r = pr_with(
        "standard",
        "SD916 = \"off\"\n",
        REPORTS,
        &[("tests/test_valves.py", PY_TEST)],
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD617") && !has(&out, "SD916"), "{out}");
    assert_eq!(code, 1, "{out}");
    // SD617 off: the skipped case proves the requirement again.
    let r = pr_with(
        "standard",
        "SD617 = \"off\"\n",
        REPORTS,
        &[("tests/test_valves.py", PY_TEST)],
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD617") && !has(&out, "SD916"), "{out}");
    assert_eq!(code, 0, "{out}");
}

// The local flow: the stamp comes before the commit

#[test]
fn a_stamp_of_an_uncommitted_edit_stays_fresh_after_the_commit() {
    let r = pytest_pr("standard", "");
    // The agent edits the code, runs the tests, stamps, and then commits.
    r.write("src/valves.py", "x = 2\n");
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD614"), "{out}");
    assert_eq!(code, 0, "{out}");
    r.commit("code");
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD614"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_file_that_git_does_not_track_at_stamp_time_makes_the_report_stale_after_git_add() {
    // An untracked file is not in the code tree. After `git add`, it is.
    let r = pytest_pr("standard", "");
    r.write("tests/test_new.py", "def test_new():\n    assert True\n");
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD614"), "{out}");
    assert_eq!(code, 0, "{out}");
    r.git(&["add", "tests/test_new.py"]);
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD614"), "{out}");
}

// A setting of the report codes cannot hide an unproven requirement

#[test]
fn a_warning_or_off_setting_of_sd613_to_sd615_does_not_hide_an_unproven_requirement() {
    // A stale report, with a skipped case: SD614 is a warning, SD617 stays.
    let r = pr_with(
        "standard",
        "SD614 = \"warning\"\n",
        REPORTS,
        &[("tests/test_valves.py", PY_TEST)],
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    r.write("src/valves.py", "x = 2\n");
    let (code, out) = check(&r, &[]);
    assert!(out.contains(": warning SD614 "), "{out}");
    assert!(out.contains(": error SD617 "), "{out}");
    assert_eq!(code, 1, "{out}");
    // No report, and the 3 codes off.
    let r = pr_with(
        "standard",
        "SD613 = \"off\"\nSD614 = \"off\"\nSD615 = \"off\"\n",
        REPORTS,
        &[("tests/test_valves.py", PY_TEST)],
    );
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD613") && !has(&out, "SD614") && !has(&out, "SD615"));
    assert!(out.contains(": error SD617 "), "{out}");
    assert!(out.contains("no case of a fresh report"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_report_outside_every_root_does_not_turn_the_root_profile_off() {
    // The project is `lite`, the root is `standard`, and the report is in a
    // folder that no root holds, so SD614 is off for it. The requirement of
    // the root is still unproven.
    let text = config_with("lite", "", "reports = [\"out/*.xml\"]\n").replace(
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nprofile = \"standard\"\n",
    );
    let r = pr_in(&text, &[("tests/test_valves.py", PY_TEST)]);
    r.write(".gitignore", "out/\n");
    r.write(
        "out/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
        ),
    );
    stamp(&r);
    r.write("src/valves.py", "x = 2\n");
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD614"), "{out}");
    assert!(out.contains(": error SD617 "), "{out}");
    assert_eq!(code, 1, "{out}");
}

// The code tree

/// The hash that `evidence stamp` writes now.
fn tree_of(r: &Repo) -> String {
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    out.split("at tree ")
        .nth(1)
        .unwrap()
        .split(':')
        .next()
        .unwrap()
        .to_string()
}

#[cfg(unix)]
#[test]
fn two_paths_that_are_not_utf8_both_count_in_the_tree() {
    use std::os::unix::ffi::OsStrExt;
    let r = pytest_pr("standard", "");
    let oid = |p: &str| r.git_out(&["rev-parse", &format!("HEAD:{p}")]);
    let (one, two) = (oid("src/valves.py"), oid("tests/test_other.py"));
    // Both names give the same text when a reader replaces the bad byte.
    let names: [&[u8]; 2] = [b"bin/a\xff.dat", b"bin/a\xfe.dat"];
    let index = |name: &[u8], id: &str| {
        // The path is part of the argument: build it from bytes.
        let mut arg = format!("100644,{id},").into_bytes();
        arg.extend_from_slice(name);
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&r.dir)
            .args(["update-index", "--add", "--cacheinfo"])
            .arg(std::ffi::OsStr::from_bytes(&arg))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&r.dir)
            .args(["update-index", "--assume-unchanged", "--"])
            .arg(std::ffi::OsStr::from_bytes(name))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    index(names[0], &one);
    index(names[1], &one);
    let before = tree_of(&r);
    // A change of the blob of the first name in the index changes the tree.
    index(names[0], &two);
    let after = tree_of(&r);
    assert_ne!(before, after);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_that_changes_on_disk_changes_the_tree() {
    let r = pytest_pr("standard", "");
    std::os::unix::fs::symlink("valves.py", r.dir.join("src/link")).unwrap();
    r.commit("link");
    let committed = tree_of(&r);
    // The link points to another file. Nothing is staged.
    std::fs::remove_file(r.dir.join("src/link")).unwrap();
    std::os::unix::fs::symlink("../tests/test_other.py", r.dir.join("src/link")).unwrap();
    let moved = tree_of(&r);
    assert_ne!(committed, moved);
    // The same link text as the commit: the same tree.
    std::fs::remove_file(r.dir.join("src/link")).unwrap();
    std::os::unix::fs::symlink("valves.py", r.dir.join("src/link")).unwrap();
    assert_eq!(tree_of(&r), committed);
    // A file that replaces the link is in the tree as a file.
    std::fs::remove_file(r.dir.join("src/link")).unwrap();
    r.write("src/link", "plain\n");
    let file = tree_of(&r);
    assert!(file != committed && file != moved);
}

// The globs of `reports`

#[test]
fn a_glob_that_leaves_the_repository_is_a_configuration_error() {
    for glob in [
        "/tmp/out/*.xml",
        "../out/*.xml",
        "reports/../../x.xml",
        "C:/out/*.xml",
    ] {
        let r = Repo::with_config(&format!(
            "[evidence]\nreports = [\"{glob}\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
        ));
        r.commit("x");
        let (code, _, err) = r.run(&["check"]);
        assert_eq!(code, 3, "{glob}: {err}");
        assert!(err.contains("evidence.reports"), "{glob}: {err}");
    }
    // A name with `..` inside it is fine.
    let r = Repo::with_config(
        "[evidence]\nreports = [\"out/a..b/*.xml\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.commit("x");
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{err}");
}

#[cfg(unix)]
#[test]
fn a_folder_of_reports_that_cannot_be_read_is_an_io_error() {
    use std::os::unix::fs::PermissionsExt;
    let r = pytest_pr("standard", "");
    let dir = r.dir.join("reports");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    // A process of the super user reads the folder: nothing to test.
    if std::fs::read_dir(&dir).is_ok() {
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let stamp_run = r.run(&["evidence", "stamp"]);
    let check_run = r.run(&["check", "--reports", "--base", "main"]);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(stamp_run.0, 4, "{}{}", stamp_run.1, stamp_run.2);
    assert!(stamp_run.2.contains("reports"), "{}", stamp_run.2);
    assert_eq!(check_run.0, 4, "{}{}", check_run.1, check_run.2);
    // A folder that does not exist is no error: SD613 says it.
    std::fs::remove_dir_all(&dir).unwrap();
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD613"), "{out}");
    assert_eq!(code, 1, "{out}");
}

// The reliable level (design 0.8.0, D2)

/// The keys of `[evidence]` for the reliable level.
fn reliable(runs: u32, for_: &str) -> String {
    format!("{REPORTS}reliable-runs = {runs}\nreliable-for = \"{for_}\"\n")
}

/// A pull request with the tag above `test_closes_on_timeout`, and one report
/// file for each item of `children`: the child element of the case. The
/// reports are stamped.
fn runs_pr(class: &str, profile: &str, rules: &str, evidence: &str, children: &[&str]) -> Repo {
    let r = pr_class_in(
        class,
        &config_with(profile, rules, evidence),
        &[("tests/test_valves.py", PY_TEST)],
    );
    for (i, child) in children.iter().enumerate() {
        r.write(
            &format!("reports/run{i}.xml"),
            &junit(
                "pytest",
                &[("tests.test_valves", "test_closes_on_timeout", child)],
            ),
        );
    }
    stamp(&r);
    r
}

fn proof_counts(r: &Repo) -> String {
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let at = out.find("\"proof\":").unwrap_or_else(|| panic!("{out}"));
    out[at..].split('}').next().unwrap().to_string()
}

#[test]
fn three_passes_of_the_case_on_one_tree_make_the_requirement_reliable() {
    let r = runs_pr(
        "higher-risk",
        "strict",
        "",
        &reliable(3, "all"),
        &["", "", ""],
    );
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD620") && !has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    let counts = proof_counts(&r);
    assert!(
        counts.contains("\"reliable\":1,\"passed_once\":0"),
        "{counts}"
    );
}

#[test]
fn two_passes_are_passed_once_and_reliable_for_all_gives_sd620() {
    for profile in ["standard", "strict"] {
        let r = runs_pr("higher-risk", profile, "", &reliable(3, "all"), &["", ""]);
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD620");
        assert_eq!(found.len(), 1, "{profile}: {out}");
        assert!(
            found[0].contains(": error SD620 proof-not-reliable"),
            "{out}"
        );
        assert!(found[0].contains("`requirements/valves#flush`"), "{out}");
        assert!(found[0].contains("`test_closes_on_timeout`"), "{out}");
        assert!(found[0].contains("passed 2 times"), "{out}");
        assert!(found[0].contains("`reliable-runs` is 3"), "{out}");
        assert!(!has(&out, "SD617"), "{out}");
        assert_eq!(code, 1, "{out}");
        let counts = proof_counts(&r);
        assert!(
            counts.contains("\"reliable\":0,\"passed_once\":1"),
            "{counts}"
        );
    }
}

#[test]
fn sd620_needs_the_flag_the_key_and_the_class_that_reliable_for_names() {
    // No flag: the reports are not read.
    let r = runs_pr("higher-risk", "strict", "", &reliable(3, "all"), &[""]);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert!(!has(&out, "SD620"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
    // `none` (the default): no SD620 with one pass.
    let r = runs_pr("higher-risk", "strict", "", &reliable(3, "none"), &[""]);
    let (_, out) = check(&r, &[]);
    assert!(!has(&out, "SD620"), "{out}");
    // `higher-risk` covers a higher-risk change and no routine change.
    for (class, evidence, want) in [
        ("higher-risk", reliable(3, "higher-risk"), true),
        ("routine", reliable(3, "higher-risk"), false),
        ("routine", reliable(3, "all"), true),
    ] {
        let r = runs_pr(class, "strict", "", &evidence, &[""]);
        let (_, out) = check(&r, &[]);
        assert_eq!(has(&out, "SD620"), want, "{class} {evidence}: {out}");
    }
}

#[test]
fn three_passes_and_one_fail_of_one_case_are_not_proven() {
    let r = runs_pr(
        "higher-risk",
        "strict",
        "",
        &reliable(3, "all"),
        &["", "", "", FAILED],
    );
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("failed"), "{out}");
    // A fail is not "passed once": SD620 is silent, SD617 stops the change.
    assert!(!has(&out, "SD620"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_reliable_level_is_off_in_lite_and_a_rule_for_the_group_proof_turns_it_on() {
    let r = runs_pr("higher-risk", "lite", "", &reliable(3, "all"), &[""]);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD620"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The reports are read for SD620 alone, with each report code off.
    let r = runs_pr(
        "higher-risk",
        "lite",
        "proof = \"error\"\n",
        &reliable(3, "all"),
        &[""],
    );
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD620"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A rule for the group `reports` does not change SD620.
    let r = runs_pr(
        "higher-risk",
        "standard",
        "reports = \"off\"\n",
        &reliable(3, "all"),
        &[""],
    );
    let (_, out) = check(&r, &[]);
    assert!(has(&out, "SD620"), "{out}");
}

#[test]
fn finish_stops_at_step_5_with_sd620_until_the_case_passed_n_times() {
    let setup = |children: &[&str]| {
        let r = Repo::with_config(&config("standard", &reliable(3, "all")));
        r.write(".gitignore", "reports/\n");
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/valves.py", "x = 1\n");
        r.write(
            &format!("{FOLDER}/intent.md"),
            "---\nstatus: accepted\nclass: routine\n---\n# Dose\n\n## Intent\n\nFlush the line.\n",
        );
        r.write(
            &format!("{FOLDER}/plan.md"),
            "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        );
        r.write(
            &format!("{FOLDER}/spec.md"),
            &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD {TAG}: The controller SHALL flush the line.\n"),
        );
        r.commit_base_of_0_7_1("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r.write("tests/test_valves.py", PY_TEST);
        r.commit("test");
        for (i, child) in children.iter().enumerate() {
            r.write(
                &format!("reports/run{i}.xml"),
                &junit(
                    "pytest",
                    &[("tests.test_valves", "test_closes_on_timeout", child)],
                ),
            );
        }
        stamp(&r);
        r
    };
    let r = setup(&["", ""]);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5"), "{err}");
    assert!(
        out.contains("SD620") && out.contains("passed 2 times"),
        "{out}"
    );
    assert!(r.dir.join(FOLDER).exists());
    let r = setup(&["", "", ""]);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
}

/// D70 item 3: SD620 reads the class of the acceptance, so a branch that sets
/// `routine` on an intent that the base has as `higher-risk` does not skip the
/// level `reliable` (before: the class of the head, and `finish` exited 0).
#[test]
fn a_branch_that_lowers_the_class_of_the_base_does_not_skip_sd620_in_finish() {
    let setup = |class_in_branch: &str| {
        let r = Repo::with_config(&config("standard", &reliable(3, "higher-risk")));
        r.write(".gitignore", "reports/\n");
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/valves.py", "x = 1\n");
        r.write(
            &format!("{FOLDER}/intent.md"),
            "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nFlush the line.\n",
        );
        r.write(
            &format!("{FOLDER}/plan.md"),
            "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        );
        r.write(
            &format!("{FOLDER}/spec.md"),
            &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD {TAG}: The controller SHALL flush the line.\n"),
        );
        r.commit_base_of_0_7_1("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        if !class_in_branch.is_empty() {
            r.write(
                &format!("{FOLDER}/intent.md"),
                &format!("---\nstatus: accepted\nclass: {class_in_branch}\n---\n# Dose\n\n## Intent\n\nFlush the line.\n"),
            );
            r.commit("set the class");
        }
        r.write("tests/test_valves.py", PY_TEST);
        r.commit("test");
        for i in 0..2 {
            r.write(
                &format!("reports/run{i}.xml"),
                &junit(
                    "pytest",
                    &[("tests.test_valves", "test_closes_on_timeout", "")],
                ),
            );
        }
        stamp(&r);
        r
    };
    // The base has `higher-risk`, the branch sets `routine`, and the case passed
    // 2 times: SD620.
    let r = setup("routine");
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5"), "{err}");
    assert!(
        out.contains("SD620") && out.contains("passed 2 times"),
        "{out}"
    );
    assert!(r.dir.join(FOLDER).exists());
}

#[test]
fn a_requirement_with_no_pass_gives_sd620_when_sd617_is_off() {
    // With SD617 on, the requirement is SD617 and no SD620.
    for child in [SKIPPED, FAILED] {
        let r = runs_pr("higher-risk", "strict", "", &reliable(3, "all"), &[child]);
        let (_, out) = check(&r, &[]);
        assert!(has(&out, "SD617") && !has(&out, "SD620"), "{out}");
        // SD617 off: a skipped or failed case is fewer than N passes, so it
        // must not pass the gate while a case that passed once stops it.
        for (profile, rules) in [
            ("lite", "proof = \"error\"\n"),
            ("strict", "SD617 = \"off\"\n"),
        ] {
            let r = runs_pr("higher-risk", profile, rules, &reliable(3, "all"), &[child]);
            let (code, out) = check(&r, &[]);
            let found = lines(&out, "SD620");
            assert_eq!(found.len(), 1, "{profile} {child}: {out}");
            assert!(!has(&out, "SD617"), "{out}");
            assert!(
                found[0].contains("skipped") || found[0].contains("failed"),
                "{out}"
            );
            assert_eq!(code, 1, "{profile} {child}: {out}");
        }
    }
}

#[test]
fn the_message_of_sd620_names_the_case_with_the_fewest_passes() {
    // `test_a` passed in 3 reports and `test_b` in 2: both bind to the tag by
    // `case:` with the same word, and the case with 2 passes is named.
    let r = pr_class_in(
        "higher-risk",
        &config_with("strict", "", &reliable(3, "all")),
        &[(
            "tests/test_valves.py",
            "# covers: requirements/valves#flush case:test_x\ndef test_x():\n    assert True\n",
        )],
    );
    for (i, names) in [
        ["test_x[1]", "test_x[2]"],
        ["test_x[1]", "test_x[2]"],
        ["test_x[1]", ""],
    ]
    .iter()
    .enumerate()
    {
        let cases: Vec<(&str, &str, &str)> = names
            .iter()
            .filter(|n| !n.is_empty())
            .map(|n| ("tests.test_valves", *n, ""))
            .collect();
        r.write(&format!("reports/run{i}.xml"), &junit("pytest", &cases));
    }
    stamp(&r);
    let (_, out) = check(&r, &[]);
    let found = lines(&out, "SD620");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_x[2]` passed 2 times"), "{out}");
}

#[test]
fn a_verified_by_record_does_not_hide_a_tag_that_is_below_reliable() {
    let verified = format!(
        "{VALVES}- **flush**: The controller SHALL flush the line. <!-- verified-by: tests/test_other.py -->\n"
    );
    let setup = |tests: &[(&str, &str)], children: &[&str]| {
        let r = pr_class_in(
            "higher-risk",
            &config_with("strict", "", &reliable(3, "all")),
            tests,
        );
        r.write("docs/requirements/valves.md", &verified);
        r.commit("verified");
        for (i, child) in children.iter().enumerate() {
            r.write(
                &format!("reports/run{i}.xml"),
                &junit(
                    "pytest",
                    &[("tests.test_valves", "test_closes_on_timeout", child)],
                ),
            );
        }
        stamp(&r);
        r
    };
    let tag = [("tests/test_valves.py", PY_TEST)];
    // A tag with a bound case below `reliable` gives SD620, whether the case
    // failed, was skipped or passed too few times.
    for (children, want) in [
        (&[FAILED][..], "failed"),
        (&[SKIPPED][..], "skipped"),
        (&["", ""][..], "passed 2 times"),
    ] {
        let r = setup(&tag, children);
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD620");
        assert_eq!(found.len(), 1, "{want}: {out}");
        assert!(found[0].contains(want), "{want}: {out}");
        assert!(!has(&out, "SD617"), "{out}");
        assert_eq!(code, 1, "{want}: {out}");
    }
    let r = setup(&tag, &["", "", ""]);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD620"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The `verified-by` alone, with no tag, stays exempt.
    let r = setup(&[], &["", ""]);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD620") && !has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn finish_names_both_causes_when_sd617_and_sd620_stop_it() {
    let r = Repo::with_config(&config("standard", &reliable(3, "all")));
    r.write(".gitignore", "reports/\n");
    r.write("docs/requirements/valves.md", VALVES);
    r.write("src/valves.py", "x = 1\n");
    r.write(
        &format!("{FOLDER}/intent.md"),
        "---\nstatus: accepted\nclass: routine\n---\n# Dose\n\n## Intent\n\nFlush the line.\n",
    );
    r.write(
        &format!("{FOLDER}/plan.md"),
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
    );
    r.write(
        &format!("{FOLDER}/spec.md"),
        &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD {TAG}: The controller SHALL flush the line.\n- ADD requirements/valves#purge: The controller SHALL purge the line.\n"),
    );
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush\ndef test_flushes():\n    assert True\n\n# covers: requirements/valves#purge\ndef test_purges():\n    assert True\n",
    );
    r.commit("test");
    for i in 0..3 {
        let purge: &[(&str, &str, &str)] = if i < 2 {
            &[("tests.test_valves", "test_purges", "")]
        } else {
            &[]
        };
        let mut cases = vec![("tests.test_valves", "test_flushes", SKIPPED)];
        cases.extend_from_slice(purge);
        r.write(&format!("reports/run{i}.xml"), &junit("pytest", &cases));
    }
    stamp(&r);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("SD617") && out.contains("SD620"), "{out}");
    assert!(err.contains("SD617") && err.contains("SD620"), "{err}");
}
