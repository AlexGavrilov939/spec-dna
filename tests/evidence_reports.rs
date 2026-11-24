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

// The runners: the shapes of the reports of 7 runners (design 0.8.1, D82). The
// names of the projects are made up.

/// One runner: the test file with its tag, a twin file that has a test of the
/// same name, and the reports that the runner writes.
struct Runner {
    name: &'static str,
    path: &'static str,
    test: &'static str,
    /// A second test file with a test of the same name, and no tag. It is in
    /// another class, module or package, as the runner needs.
    twin_path: &'static str,
    twin_test: &'static str,
    /// The reports: the child of the case under the tag, then the child of the
    /// case of the twin, or `None` for a report with no twin.
    report: fn(&str, Option<&str>) -> Reports,
}

/// The reports of a runner as (file, text).
type Reports = Vec<(&'static str, String)>;

/// The test file of a Vitest or Jest test with the tag above the test.
const JS_TEST: &str = "describe('Valves', () => {\n  // covers: requirements/valves#flush\n  it('closes on timeout', () => {\n    expect(1).toBe(1);\n  });\n});\n";
const JS_TWIN: &str = "describe('Valves', () => {\n  it('closes on timeout', () => {\n    expect(1).toBe(1);\n  });\n});\n";

fn runners() -> Vec<Runner> {
    vec![
        Runner {
            name: "pytest xunit2",
            path: "tests/test_valves.py",
            test: PY_TEST,
            twin_path: "tests/test_pumps.py",
            twin_test: "def test_closes_on_timeout():\n    assert True\n",
            report: |c, t| {
                let mut cases = vec![("tests.test_valves", "test_closes_on_timeout", c)];
                cases.extend(t.map(|t| ("tests.test_pumps", "test_closes_on_timeout", t)));
                vec![("r.xml", junit("pytest", &cases))]
            },
        },
        Runner {
            name: "cargo-nextest",
            path: "tests/valves.rs",
            test: "// covers: requirements/valves#flush\n#[test]\nfn closes() {\n    assert!(true);\n}\n",
            twin_path: "tests/pumps.rs",
            twin_test: "#[test]\nfn closes() {\n    assert!(true);\n}\n",
            report: |c, t| {
                let suite = |bin: &str, child: &str| {
                    format!(
                        "<testsuite name=\"app::{bin}\" tests=\"1\" disabled=\"0\" errors=\"0\" failures=\"0\"><testcase name=\"closes\" classname=\"app::{bin}\" timestamp=\"2026-01-01T00:00:00.000+00:00\" time=\"0.01\">{child}</testcase></testsuite>"
                    )
                };
                let twin = t.map(|t| suite("pumps", t)).unwrap_or_default();
                vec![(
                    "r.xml",
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites name=\"nextest-run\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"0.1\">{}{twin}</testsuites>",
                        suite("valves", c)
                    ),
                )]
            },
        },
        Runner {
            name: "Jest with addFileAttribute",
            path: "tests/valves.test.js",
            test: JS_TEST,
            twin_path: "tests/pumps.test.js",
            twin_test: JS_TWIN,
            report: |c, t| {
                let case = |file: &str, child: &str| {
                    format!(
                        "<testsuite name=\"Valves\" errors=\"0\" failures=\"0\" skipped=\"0\" time=\"0.3\" tests=\"1\"><testcase classname=\"Valves closes on timeout\" name=\"Valves closes on timeout\" time=\"0.002\" file=\"{file}\">{child}</testcase></testsuite>"
                    )
                };
                let twin = t
                    .map(|t| case("tests/pumps.test.js", t))
                    .unwrap_or_default();
                vec![(
                    "r.xml",
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites name=\"jest tests\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"0.5\">{}{twin}</testsuites>",
                        case("tests/valves.test.js", c)
                    ),
                )]
            },
        },
        Runner {
            name: "Vitest",
            path: "tests/valves.test.ts",
            test: JS_TEST,
            twin_path: "tests/pumps.test.ts",
            twin_test: JS_TWIN,
            report: |c, t| {
                let suite = |file: &str, child: &str| {
                    format!(
                        "<testsuite name=\"{file}\" timestamp=\"2026-01-01T00:00:00.000Z\" hostname=\"ci\" tests=\"1\" failures=\"0\" errors=\"0\" skipped=\"0\" time=\"0.001\"><testcase classname=\"{file}\" name=\"Valves &gt; closes on timeout\" time=\"0.001\">{child}</testcase></testsuite>"
                    )
                };
                let twin = t
                    .map(|t| suite("tests/pumps.test.ts", t))
                    .unwrap_or_default();
                vec![(
                    "r.xml",
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?><testsuites name=\"vitest tests\" tests=\"2\" failures=\"0\" errors=\"0\" time=\"0.01\">{}{twin}</testsuites>",
                        suite("tests/valves.test.ts", c)
                    ),
                )]
            },
        },
        Runner {
            name: "Go, go-junit-report",
            path: "valves/sensor_test.go",
            test: "package valves\n\n// covers: requirements/valves#flush\nfunc TestTimeout(t *testing.T) {\n\tt.Run(\"no_reply\", func(t *testing.T) {})\n}\n",
            twin_path: "pumps/sensor_test.go",
            twin_test: "package pumps\n\nfunc TestTimeout(t *testing.T) {\n}\n",
            report: |c, t| {
                let suite = |pkg: &str, child: &str| {
                    format!(
                        "<testsuite name=\"example.org/app/{pkg}\" tests=\"2\" failures=\"0\" errors=\"0\" id=\"0\" hostname=\"ci\" time=\"0.000\" timestamp=\"2026-01-01T00:00:00Z\"><properties><property name=\"go.version\" value=\"go1.22\"></property></properties><testcase name=\"TestTimeout\" classname=\"example.org/app/{pkg}\" time=\"0.000\">{child}</testcase><testcase name=\"TestTimeout/no_reply\" classname=\"example.org/app/{pkg}\" time=\"0.000\">{child}</testcase></testsuite>"
                    )
                };
                let twin = t.map(|t| suite("pumps", t)).unwrap_or_default();
                vec![(
                    "r.xml",
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites tests=\"4\">{}{twin}</testsuites>",
                        suite("valves", c)
                    ),
                )]
            },
        },
        Runner {
            name: "JUnit 5, Surefire",
            path: "src/test/java/com/example/ValveTest.java",
            test: "package com.example;\n\nclass ValveTest {\n    // covers: requirements/valves#flush\n    @Test\n    void flushesFirst() {\n        assertTrue(true);\n    }\n}\n",
            twin_path: "src/test/java/com/example/PumpTest.java",
            twin_test: "package com.example;\n\nclass PumpTest {\n    @Test\n    void flushesFirst() {\n        assertTrue(true);\n    }\n}\n",
            report: |c, t| {
                let suite = |class: &str, child: &str| {
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuite version=\"3.0\" name=\"com.example.{class}\" time=\"0.01\" tests=\"1\" errors=\"0\" skipped=\"0\" failures=\"0\"><properties><property name=\"java.version\" value=\"21\"/></properties><testcase name=\"flushesFirst\" classname=\"com.example.{class}\" time=\"0.01\">{child}</testcase></testsuite>"
                    )
                };
                let mut out = vec![("TEST-com.example.ValveTest.xml", suite("ValveTest", c))];
                out.extend(t.map(|t| ("TEST-com.example.PumpTest.xml", suite("PumpTest", t))));
                out
            },
        },
        Runner {
            name: "PHPUnit",
            path: "tests/ValveTest.php",
            test: "<?php\n\nclass ValveTest extends TestCase\n{\n    // covers: requirements/valves#flush\n    public function testClosesOnTimeout()\n    {\n        $this->assertTrue(true);\n    }\n}\n",
            twin_path: "tests/PumpTest.php",
            twin_test: "<?php\n\nclass PumpTest extends TestCase\n{\n    public function testClosesOnTimeout()\n    {\n        $this->assertTrue(true);\n    }\n}\n",
            report: |c, t| {
                let suite = |class: &str, child: &str| {
                    format!(
                        "<testsuite name=\"Tests\\{class}\" file=\"/work/app/tests/{class}.php\" tests=\"1\" assertions=\"1\" errors=\"0\" failures=\"0\" skipped=\"0\" time=\"0.01\"><testcase name=\"testClosesOnTimeout with data set #0\" file=\"/work/app/tests/{class}.php\" line=\"9\" class=\"Tests\\{class}\" classname=\"Tests.{class}\" assertions=\"1\" time=\"0.01\">{child}</testcase></testsuite>"
                    )
                };
                let twin = t.map(|t| suite("PumpTest", t)).unwrap_or_default();
                let n = 1 + usize::from(t.is_some());
                vec![(
                    "r.xml",
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites><testsuite name=\"Valves\" tests=\"{n}\" assertions=\"{n}\" errors=\"0\" failures=\"0\" skipped=\"0\" time=\"0.02\">{}{twin}</testsuite></testsuites>",
                        suite("ValveTest", c)
                    ),
                )]
            },
        },
    ]
}

/// A pull request with the test file of `run`, and the test file of its twin
/// when `twin` is set. The reports hold the cases as the runner writes them.
fn runner_pr(run: &Runner, child: &str, twin: Option<&str>) -> Repo {
    let mut files = vec![(run.path, run.test)];
    if twin.is_some() {
        files.push((run.twin_path, run.twin_test));
    }
    let r = pr("strict", REPORTS, &files);
    for (file, text) in (run.report)(child, twin) {
        r.write(&format!("reports/{file}"), &text);
    }
    stamp(&r);
    r
}

#[test]
fn a_tag_above_the_test_binds_to_its_case_in_the_report_of_each_runner() {
    for run in runners() {
        // Passed: the tag proves the requirement.
        let r = runner_pr(&run, "", None);
        let (code, out) = check(&r, &[]);
        assert!(!has(&out, "SD617"), "{}: {out}", run.name);
        assert_eq!(code, 0, "{}: {out}", run.name);
        // Each shape names its test file, so no bind is by name only.
        let (code, out) = check(&r, &["--all"]);
        assert!(!has(&out, "SD621"), "{}: {out}", run.name);
        assert_eq!(code, 0, "{}: {out}", run.name);
        // Skipped: the tag is bound, so the message names the skipped case.
        let r = runner_pr(&run, SKIPPED, None);
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{}: {out}", run.name);
        assert!(found[0].contains("was skipped"), "{}: {out}", run.name);
        assert_eq!(code, 1, "{}: {out}", run.name);
    }
}

#[test]
fn a_passed_twin_of_the_same_name_in_a_second_file_does_not_prove_the_tag() {
    let mut failed = Vec::new();
    for run in runners() {
        let r = runner_pr(&run, SKIPPED, Some(""));
        let (code, out) = check(&r, &[]);
        let found = lines(&out, "SD617");
        if found.len() != 1 || !found[0].contains("was skipped") || code != 1 {
            failed.push(format!("{}: {out}", run.name));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
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
        found[0].contains(
            "names the test `test_closes_on_timeout` of the tag at tests/test_valves.py:1"
        ),
        "{out}"
    );
    assert!(found[0].contains("case:\"<name>\""), "{out}");
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

/// The fixture of an untracked twin: the tag is in `tests/test_a.py` (skipped),
/// and `tests/test_b.py` (passed) has a case of the same name. Both are new and
/// never added.
fn untracked_twins(evidence: &str, args: &[&str]) -> (i32, String) {
    let tag = "# covers: requirements/valves#flush\ndef test_init():\n    assert True\n";
    let plain = "def test_init():\n    assert True\n";
    let r = pr("strict", evidence, &[]);
    r.write("tests/test_a.py", tag);
    r.write("tests/test_b.py", plain);
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
    let mut all = vec!["check", "--reports"];
    all.extend(args);
    let (code, out, err) = r.run(&all);
    (code, format!("{out}{err}"))
}

/// The tag is in a file that git does not track: SD617 gives the reason
/// `outside-tree` (design 0.8.1, D85).
fn assert_outside_tree(code: i32, out: &str) {
    let found = lines(out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("tests/test_a.py:1 is in a file that is not in the code tree"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_tag_of_a_test_file_that_git_does_not_track_proves_nothing() {
    // Both test files are new and never added, so the tag of `test_a.py` is in
    // no file of the code tree.
    let (code, out) = untracked_twins(REPORTS, &["--all"]);
    assert_outside_tree(code, &out);
}

#[test]
fn an_unusable_test_glob_does_not_turn_the_untracked_file_into_a_proof() {
    // A glob with `..` is no pathspec: the list of files is not limited, and
    // the filter keeps the test files.
    let evidence = format!("{REPORTS}tests = [\"tests/test_*.py\", \"../none/*.py\"]\n");
    let (code, out) = untracked_twins(&evidence, &["--all"]);
    assert_outside_tree(code, &out);
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

#[test]
fn finish_reads_untracked_test_files_too_so_a_twin_case_does_not_prove_a_skipped_one() {
    // `finish` reads the working tree, also the files that git does not track.
    // The tagged test is skipped. An untracked file has a passed case of the
    // same name. The case of the untracked file must not bind to the tag.
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
    // The twin: never added to git.
    r.write(
        "tests/test_twin.py",
        "def test_closes_on_timeout():\n    assert True\n",
    );
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[
                ("tests.test_valves", "test_closes_on_timeout", SKIPPED),
                ("tests.test_twin", "test_closes_on_timeout", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("SD617") && out.contains("was skipped"),
        "{out}{err}"
    );
    assert!(r.dir.join(FOLDER).exists());
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
    // `test_x[1]` passed in 3 reports and `test_x[2]` in 2: both bind to the
    // tag above `test_x`, and the case with 2 passes is named.
    let r = pr_class_in(
        "higher-risk",
        &config_with("strict", "", &reliable(3, "all")),
        &[(
            "tests/test_valves.py",
            "# covers: requirements/valves#flush\ndef test_x():\n    assert True\n",
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

// The test under the tag (design 0.8.1, D82 items 1 to 5)

/// One `strict` check of one tagged test file with one report. The report
/// holds `cases` as (classname, name, child). Returns the exit code and the
/// output.
fn bind_check(path: &str, test: &str, cases: &[(&str, &str, &str)]) -> (i32, String) {
    let r = pr("strict", REPORTS, &[(path, test)]);
    r.write("reports/r.xml", &junit("runner", cases));
    stamp(&r);
    check(&r, &[])
}

/// The tag of the examples, as a line comment of JavaScript and Rust.
const SLASH_TAG: &str = "// covers: requirements/valves#flush\n";

#[test]
fn a_tag_above_an_import_line_binds_no_case_of_another_file() {
    // The defect (E22): the line below the tag is an import with the word
    // `it`, and the only passed case is `Pumps > stops it` of another file.
    let valves = format!(
        "{SLASH_TAG}import {{ describe, it, expect }} from 'vitest'\n\ndescribe('Valves', () => {{\n  it('closes on timeout', () => {{\n    expect(1).toBe(1);\n  }});\n}});\n"
    );
    let (code, out) = bind_check(
        "tests/valves.test.ts",
        &valves,
        &[("tests/pumps.test.ts", "Pumps > stops it", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("stands above no test"), "{out}");
    assert!(found[0].contains("tests/valves.test.ts:1"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_above_a_group_binds_no_test_of_the_group() {
    let text = format!(
        "{SLASH_TAG}describe('Valves', () => {{\n  it('closes on timeout', () => {{\n    expect(1).toBe(1);\n  }});\n}});\n"
    );
    let (code, out) = bind_check(
        "tests/valves.test.ts",
        &text,
        &[("tests/valves.test.ts", "Valves > closes on timeout", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("stands above a group"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_above_a_test_with_a_template_name_binds_no_case() {
    let text = format!(
        "{SLASH_TAG}it.each(rows)('closes %s', (row) => {{\n  expect(row).toBe(1);\n}});\n"
    );
    let (code, out) = bind_check(
        "tests/valves.test.ts",
        &text,
        &[("tests/valves.test.ts", "closes a", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("has a template name"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_case_that_names_2_tests_belongs_to_the_test_with_the_nearer_name() {
    // The case `Valves > never closes` ends with both names. It belongs to
    // `never closes`, so the tag above `closes` has no case.
    let text = format!(
        "describe('Valves', () => {{\n  {SLASH_TAG}  it('closes', () => {{\n    expect(1).toBe(1);\n  }});\n\n  it('never closes', () => {{\n    expect(1).toBe(1);\n  }});\n}});\n"
    );
    let (code, out) = bind_check(
        "tests/valves.test.ts",
        &text,
        &[("tests/valves.test.ts", "Valves > never closes", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("names the test `closes`"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_in_the_body_of_a_test_stands_above_no_test() {
    let text = "def test_a():\n    # covers: requirements/valves#flush\n    assert True\n\n\ndef test_b():\n    assert True\n";
    let (code, out) = bind_check(
        "tests/test_valves.py",
        text,
        &[
            ("tests.test_valves", "test_a", ""),
            ("tests.test_valves", "test_b", ""),
        ],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("stands above no test"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_above_a_multi_line_decorator_binds_the_test_below_it() {
    // The 0.8.0 block ends at the line `)`, before the definition.
    let text = "# covers: requirements/valves#flush\n@pytest.mark.parametrize(\n    \"a, b\",\n    [(1, 2), (3, 4)],\n)\ndef test_x(a, b):\n    assert a < b\n";
    let case = |child| [("tests.test_valves", "test_x[1-2]", child)];
    let (code, out) = bind_check("tests/test_valves.py", text, &case(""));
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    let (code, out) = bind_check("tests/test_valves.py", text, &case(SKIPPED));
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_x[1-2]` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_tag_on_the_line_of_the_definition_binds_that_test() {
    // The 0.8.0 block starts at the line below the tag.
    let text = "def test_x():  # covers: requirements/valves#flush\n    assert True\n\n\ndef test_y():\n    assert True\n";
    let case = |child| [("tests.test_valves", "test_x", child)];
    let (code, out) = bind_check("tests/test_valves.py", text, &case(""));
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    let (code, out) = bind_check("tests/test_valves.py", text, &case(SKIPPED));
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_x` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

/// Guard: each layout binds to its case, and a skipped case names itself.
#[test]
fn a_tag_binds_the_test_under_its_attributes_and_annotations() {
    let layouts: [(&str, &str, &str, &str); 5] = [
        // Rust attributes.
        (
            "tests/valves.rs",
            "// covers: requirements/valves#flush\n#[test]\n#[should_panic(expected = \"x\")]\nfn closes() {\n    panic!(\"x\");\n}\n",
            "app::valves",
            "closes",
        ),
        // Java annotations.
        (
            "src/test/java/com/example/ValveTest.java",
            "class ValveTest {\n    // covers: requirements/valves#flush\n    @Test\n    @DisplayName(\"closes\")\n    void flushesFirst() {\n        assertTrue(true);\n    }\n}\n",
            "com.example.ValveTest",
            "flushesFirst",
        ),
        // Kotlin: the annotation and the function on one line.
        (
            "src/test/kotlin/com/example/ValveTest.kt",
            "class ValveTest {\n    // covers: requirements/valves#flush\n    @Test fun closes() {\n        assertTrue(true)\n    }\n}\n",
            "com.example.ValveTest",
            "closes",
        ),
        // C#: the attribute and the method on one line.
        (
            "Valves/Tests/ValveTest.cs",
            "public class ValveTest {\n    // covers: requirements/valves#flush\n    [Fact] public void ClosesOnTimeout() {\n    }\n}\n",
            "Valves.Tests.ValveTest",
            "ClosesOnTimeout",
        ),
        // A Go test with a comment line between the tag and the function.
        (
            "valves/sensor_test.go",
            "package valves\n\n// covers: requirements/valves#flush\n// More text.\nfunc TestTimeout(t *testing.T) {\n}\n",
            "example.org/valves",
            "TestTimeout",
        ),
    ];
    for (path, text, classname, name) in layouts {
        let (code, out) = bind_check(path, text, &[(classname, name, "")]);
        assert!(!has(&out, "SD617"), "{path}: {out}");
        assert_eq!(code, 0, "{path}: {out}");
        let (code, out) = bind_check(path, text, &[(classname, name, SKIPPED)]);
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{path}: {out}");
        assert!(found[0].contains("was skipped"), "{path}: {out}");
        assert_eq!(code, 1, "{path}: {out}");
    }
}

// The file of a case, the shared cases and the exact `case:` (design 0.8.1, D82
// items 6 to 8)

/// A line-format report: `spec-dna-report 1`, then one line for each case.
fn lines_report(cases: &[(&str, &str, &str)]) -> String {
    let mut out = String::from("spec-dna-report 1\n");
    for (outcome, file, name) in cases {
        out.push_str(&format!("{outcome}\t{file}\t{name}\n"));
    }
    out
}

#[test]
fn a_classname_that_is_the_path_of_another_file_binds_no_case() {
    // The report has the case `Valves > closes` of `tests/b.test.ts`, and no
    // case of `tests/a.test.ts`, which has the tag.
    let tag = format!(
        "describe('Valves', () => {{\n  {SLASH_TAG}  it('closes', () => {{\n    expect(1).toBe(1);\n  }});\n}});\n"
    );
    let plain =
        "describe('Valves', () => {\n  it('closes', () => {\n    expect(1).toBe(1);\n  });\n});\n";
    let r = pr(
        "strict",
        REPORTS,
        &[("tests/a.test.ts", &tag), ("tests/b.test.ts", plain)],
    );
    r.write(
        "reports/r.xml",
        &junit("vitest", &[("tests/b.test.ts", "Valves > closes", "")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("names the test `closes` of the tag at tests/a.test.ts:2"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_skipped_case_of_the_file_hides_a_passed_case_of_no_file() {
    // The precedence: the case of `tests.test_a` fits the file by its
    // `classname`, so the line of the line format, which fits by its name only,
    // does not bind.
    let tag = "# covers: requirements/valves#flush\ndef test_init():\n    assert True\n";
    let r = pr("strict", REPORTS, &[("tests/test_a.py", tag)]);
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_a", "test_init", SKIPPED)]),
    );
    r.write(
        "reports/lines.txt",
        &lines_report(&[("pass", "-", "test_init")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_init` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn two_tests_of_one_name_in_one_file_share_their_cases() {
    // `test_init` is in 2 classes. The tag is above the first, whose case was
    // skipped. The case of the second passed.
    let text = "class TestA:\n    # covers: requirements/valves#flush\n    def test_init(self):\n        assert True\n\n\nclass TestB:\n    def test_init(self):\n        assert True\n";
    let (code, out) = bind_check(
        "tests/test_valves.py",
        text,
        &[
            ("tests.test_valves.TestA", "test_init", SKIPPED),
            ("tests.test_valves.TestB", "test_init", ""),
        ],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains(
            "the tests of the name `test_init` in tests/test_valves.py share their cases"
        ),
        "{out}"
    );
    assert!(
        found[0].contains("the case `test_init` was skipped"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // Each case passed: the requirement is proven.
    let (code, out) = bind_check(
        "tests/test_valves.py",
        text,
        &[
            ("tests.test_valves.TestA", "test_init", ""),
            ("tests.test_valves.TestB", "test_init", ""),
        ],
    );
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn jest_twins_in_2_files_share_their_cases() {
    // 2 files have the same titles. The default of jest-junit writes no file,
    // so the 2 cases fit both files by their name only.
    let text = format!(
        "describe('Valves', () => {{\n  {SLASH_TAG}  it('closes', () => {{\n    expect(1).toBe(1);\n  }});\n}});\n"
    );
    let twin =
        "describe('Valves', () => {\n  it('closes', () => {\n    expect(1).toBe(1);\n  });\n});\n";
    let r = pr(
        "standard",
        REPORTS,
        &[("tests/a.test.js", &text), ("tests/b.test.js", twin)],
    );
    r.write(
        "reports/r.xml",
        &junit(
            "jest",
            &[
                ("Valves closes", "Valves closes", SKIPPED),
                ("Valves closes", "Valves closes", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("the tests of the name `closes` in tests/a.test.js share their cases"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn case_is_exact_and_the_message_names_the_case_that_ends_with_the_value() {
    let text = "# covers: requirements/valves#flush case:closes\nVALUE = 1\n";
    let (code, out) = bind_check(
        "tests/test_valves.py",
        text,
        &[("tests.test_valves", "Valves closes", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`closes`"), "{out}");
    assert!(found[0].contains("`Valves closes`"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn case_binds_a_name_with_a_slash_a_space_and_a_hash() {
    // Guard: the explicit name binds, and the `#` of a data set is no id.
    let go = "package valves\n\n// covers: requirements/valves#flush case:\"TestTimeout/no_reply\"\nVALUE = 1\n";
    let (code, out) = bind_check(
        "valves/sensor_test.go",
        go,
        &[("example.org/valves", "TestTimeout/no_reply", "")],
    );
    assert!(!has(&out, "SD617") && !has(&out, "SD105"), "{out}");
    assert_eq!(code, 0, "{out}");
    let php =
        "<?php\n// covers: requirements/valves#flush case:\"testOne with data set #0\"\n$x = 1;\n";
    let (code, out) = bind_check(
        "tests/ValveTest.php",
        php,
        &[("Tests.ValveTest", "testOne with data set #0", "")],
    );
    assert!(!has(&out, "SD617") && !has(&out, "SD105"), "{out}");
    assert_eq!(code, 0, "{out}");
}

// A test file outside the code tree (design 0.8.1, D85)

/// `check --reports` in the working tree, which reads the files that git does
/// not track. A check with `--base` reads the tracked files only.
fn check_tree(r: &Repo, extra: &[&str]) -> (i32, String) {
    let mut args = vec!["check", "--reports"];
    args.extend(extra);
    let (code, out, err) = r.run(&args);
    (code, format!("{out}{err}"))
}

/// A pull request with a new test file that git does not track, its passed
/// case and a stamp.
fn untracked_new_file() -> Repo {
    let r = pr("strict", REPORTS, &[]);
    r.write(
        "tests/test_new.py",
        "# covers: requirements/valves#flush\ndef test_new():\n    assert True\n",
    );
    r.write(
        "reports/junit.xml",
        &junit("pytest", &[("tests.test_new", "test_new", "")]),
    );
    stamp(&r);
    r
}

#[test]
fn a_tag_in_an_untracked_test_file_gives_sd617_with_the_reason_outside_tree() {
    let r = untracked_new_file();
    let (code, out) = check_tree(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0]
            .contains("the tag at tests/test_new.py:1 is in a file that is not in the code tree"),
        "{out}"
    );
    assert!(found[0].contains("`git add` the file"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_edit_of_the_untracked_file_after_the_stamp_still_gives_sd617() {
    // Issue 113: the edit left the report fresh, and the requirement stayed
    // proven by a test that no longer had its form.
    let r = untracked_new_file();
    r.write(
        "tests/test_new.py",
        "# covers: requirements/valves#flush\ndef test_new():\n    assert False\n",
    );
    let (code, out) = check_tree(&r, &[]);
    assert_eq!(lines(&out, "SD617").len(), 1, "{out}");
    assert!(out.contains("not in the code tree"), "{out}");
    assert!(!has(&out, "SD614"), "{out}");
    assert_eq!(code, 1, "{out}");
    // `--all` reports the state of the requirement.
    let (_, out) = check_tree(&r, &["--all"]);
    assert_eq!(lines(&out, "SD616").len(), 1, "{out}");
}

#[test]
fn a_new_report_and_a_stamp_after_git_add_prove_the_requirement() {
    // Guard: the file is in the index, so it is in the code tree.
    let r = untracked_new_file();
    r.git(&["add", "tests/test_new.py"]);
    stamp(&r);
    let (code, out) = check_tree(&r, &[]);
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_tracked_test_file_that_tree_exclude_matches_gives_sd617_outside_tree() {
    let evidence = format!("{REPORTS}tree-exclude = [\"tests/test_valves.py\", \"**/*.md\"]\n");
    let r = pr("strict", &evidence, &[("tests/test_valves.py", PY_TEST)]);
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("not in the code tree"), "{out}");
    assert!(found[0].contains("tree-exclude"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_untracked_twin_keeps_its_own_file_and_does_not_prove_a_tracked_tag() {
    // Guard: the tag is in the tracked `test_a.py` (skipped). The untracked
    // `test_b.py` has the same test name, and its case passed. Its
    // `classname` names its own file, so the case does not fit `test_a.py`.
    let tag = "# covers: requirements/valves#flush\ndef test_init():\n    assert True\n";
    let r = pr("strict", REPORTS, &[("tests/test_a.py", tag)]);
    r.write("tests/test_b.py", "def test_init():\n    assert True\n");
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
    let (code, out) = check_tree(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`test_init` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

// The hook that runs the tests (design 0.8.1, D89)

#[test]
fn a_report_written_again_after_the_stamp_gives_the_hook_text() {
    let r = pytest_pr("strict", "");
    let (code, out) = check(&r, &[]);
    assert_eq!(code, 0, "{out}");
    // A commit hook runs the tests again: new bytes, no stamp.
    r.write(
        "reports/junit.xml",
        &junit(
            "pytest",
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        )
        .replace("time=\"0.1\"", "time=\"0.2\""),
    );
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD614");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("the bytes of the report `reports/junit.xml` changed after its stamp"),
        "{out}"
    );
    assert!(
        found[0].contains("a commit hook or a second test run"),
        "{out}"
    );
    assert!(found[0].contains("one hook entry"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_setup_of_the_hook_keeps_the_stamp_fresh() {
    // The entry of the setup: the test command, then the stamp, in one
    // `sh -c`. The stub test command writes new bytes at each run.
    let r = pr("strict", REPORTS, &[("tests/test_valves.py", PY_TEST)]);
    let entry = format!(
        "mkdir -p reports && printf '<testsuites><testsuite name=\"s\" tests=\"1\" time=\"%s\"><testcase classname=\"tests.test_valves\" name=\"test_closes_on_timeout\" time=\"0.01\"/></testsuite></testsuites>' \"$$\" > reports/junit.xml && \"{BIN}\" evidence stamp"
    );
    let mut seen = Vec::new();
    for _ in 0..2 {
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&entry)
            .current_dir(&r.dir)
            .output()
            .unwrap();
        assert!(status.status.success(), "{status:?}");
        seen.push(r.read("reports/junit.xml"));
        let (code, out) = check(&r, &[]);
        assert!(!has(&out, "SD614"), "{out}");
        assert_eq!(code, 0, "{out}");
    }
    // The 2 runs wrote different bytes.
    assert_ne!(seen[0], seen[1]);
}

// A bind by name: SD621 (design 0.8.1, D82 items 6 and 9)

/// The lines of a state code: they say that the requirement "is active". The
/// check of a change gives its own lines of the same code.
fn state_lines<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    lines(out, code)
        .into_iter()
        .filter(|l| l.contains("` is active, and "))
        .collect()
}

/// The report of jest-junit with its defaults: the `classname` and the `name`
/// are the same title, and the case has no `file`.
const JEST_DEFAULTS: [(&str, &str, &str); 1] =
    [("Valves closes on timeout", "Valves closes on timeout", "")];

#[test]
fn jest_junit_with_its_defaults_binds_by_name_and_gives_sd621() {
    for (profile, severity, code) in [("standard", "warning", 0), ("strict", "error", 1)] {
        let r = pr(profile, REPORTS, &[("tests/valves.test.js", JS_TEST)]);
        r.write("reports/r.xml", &junit("jest", &JEST_DEFAULTS));
        stamp(&r);
        // The bind counts: the requirement is proven.
        let (_, out) = check(&r, &[]);
        assert!(!has(&out, "SD617"), "{profile}: {out}");
        // SD621 is a state code, so `--all` shows it.
        let (exit, out) = check(&r, &["--all"]);
        let found = state_lines(&out, "SD621");
        assert_eq!(found.len(), 1, "{profile}: {out}");
        assert!(found[0].contains(&format!(": {severity} SD621 ")), "{out}");
        assert!(
            found[0].contains(
                "the tag at tests/valves.test.js:2 binds the case `Valves closes on timeout`"
            ),
            "{out}"
        );
        assert!(found[0].contains("the test `closes on timeout`"), "{out}");
        assert!(
            found[0].contains("case:\"Valves closes on timeout\""),
            "{out}"
        );
        assert!(found[0].contains("addFileAttribute: \"true\""), "{out}");
        assert_eq!(exit, code, "{profile}: {out}");
        assert!(!has(&out, "SD616") && !has(&out, "SD617"), "{out}");
    }
    // `lite` turns the report codes off.
    let r = pr(
        "lite",
        "reports = [\"reports/*\"]\n",
        &[("tests/valves.test.js", JS_TEST)],
    );
    r.write("reports/r.xml", &junit("jest", &JEST_DEFAULTS));
    stamp(&r);
    let (_, out) = check(&r, &["--all"]);
    assert!(!has(&out, "SD621"), "{out}");
}

#[test]
fn the_report_of_jest_junit_with_the_file_gives_no_sd621() {
    let r = pr("strict", REPORTS, &[("tests/valves.test.js", JS_TEST)]);
    r.write(
        "reports/r.xml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites><testsuite name=\"Valves\" tests=\"1\"><testcase classname=\"Valves closes on timeout\" name=\"Valves closes on timeout\" file=\"tests/valves.test.js\"/></testsuite></testsuites>",
    );
    stamp(&r);
    let (code, out) = check(&r, &["--all"]);
    assert!(!has(&out, "SD621") && !has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn an_exact_case_name_that_one_case_has_gives_no_sd621() {
    let test =
        "// covers: requirements/valves#flush case:\"Valves closes on timeout\"\nVALUE = 1\n";
    let r = pr("strict", REPORTS, &[("tests/valves.test.js", test)]);
    r.write("reports/r.xml", &junit("jest", &JEST_DEFAULTS));
    stamp(&r);
    let (code, out) = check(&r, &["--all"]);
    assert!(!has(&out, "SD621") && !has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The same name in 2 cases (2 files with the same titles): a bind by name,
    // and a shared set.
    let r = pr("strict", REPORTS, &[("tests/valves.test.js", test)]);
    r.write(
        "reports/r.xml",
        &junit(
            "jest",
            &[
                ("Valves closes on timeout", "Valves closes on timeout", ""),
                ("Valves closes on timeout", "Valves closes on timeout", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &["--all"]);
    let found = state_lines(&out, "SD621");
    assert_eq!(found.len(), 1, "{out}");
    // The tag has `case:` already, so the message does not tell the author to
    // add it.
    assert!(
        found[0].contains("names the case `Valves closes on timeout`"),
        "{out}"
    );
    assert!(
        found[0].contains("more than one case of that name"),
        "{out}"
    );
    assert!(!found[0].contains("add `case:"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_bind_by_name_of_a_requirement_of_the_change_gives_sd621_where_sd617_runs() {
    // The diff deletes the folder of the change `dose`, which adds `flush`.
    for (profile, severity, code) in [("standard", "warning", 0), ("strict", "error", 1)] {
        let r = pr(profile, REPORTS, &[("tests/valves.test.js", JS_TEST)]);
        r.write("reports/r.xml", &junit("jest", &JEST_DEFAULTS));
        stamp(&r);
        let (exit, out) = check(&r, &[]);
        let found = lines(&out, "SD621");
        assert_eq!(found.len(), 1, "{profile}: {out}");
        assert!(found[0].contains(&format!(": {severity} SD621 ")), "{out}");
        assert!(found[0].contains("the change `dose`"), "{out}");
        assert!(found[0].contains("`requirements/valves#flush`"), "{out}");
        assert_eq!(exit, code, "{profile}: {out}");
        assert!(!has(&out, "SD617"), "{out}");
    }
}

#[test]
fn a_bind_by_name_of_a_requirement_that_no_change_touches_shows_only_with_all() {
    // `close` has a tag in a Jest file (by name), and `flush`, the requirement
    // of the change, has an exact tag.
    let close = "describe('Valves', () => {\n  // covers: requirements/valves#close\n  it('closes on timeout', () => {\n    expect(1).toBe(1);\n  });\n});\n";
    let r = pr(
        "strict",
        REPORTS,
        &[
            ("tests/valves.test.js", close),
            ("tests/test_valves.py", PY_TEST),
        ],
    );
    r.write(
        "reports/r.xml",
        &junit(
            "mixed",
            &[
                ("Valves closes on timeout", "Valves closes on timeout", ""),
                ("tests.test_valves", "test_closes_on_timeout", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(!has(&out, "SD621"), "{out}");
    assert_eq!(code, 0, "{out}");
    let (code, out) = check(&r, &["--all"]);
    let found = lines(&out, "SD621");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`requirements/valves#close`"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn finish_stops_on_sd621_in_strict_and_goes_on_in_standard() {
    // The report names a `classname` that names no test file, so the case fits
    // the tag by its name only.
    let setup = |profile: &str| {
        let r = Repo::with_config(&config(profile, REPORTS));
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
        r.write(
            "reports/junit.xml",
            &junit("pytest", &[("pytest.valves", "test_closes_on_timeout", "")]),
        );
        stamp(&r);
        r
    };
    let r = setup("strict");
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5"), "{err}");
    // Step 5.4 names the cause, as it does for SD617 and SD620.
    assert!(err.contains("SD621"), "{err}");
    assert!(err.contains("binds a case by its name only"), "{err}");
    assert!(out.contains("SD621"), "{out}");
    assert!(out.contains("by its name only"), "{out}");
    assert!(r.dir.join(FOLDER).exists());
    // In `standard` SD621 is a warning, and it never stops `finish`.
    let r = setup("standard");
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
}

// Regressions against 0.8.0 that the review found

#[test]
fn a_kotlin_test_name_with_spaces_binds_the_case_of_the_report() {
    let text = "class ValveTest {\n    // covers: requirements/valves#flush\n    @Test fun `closes on timeout`() {\n    }\n}\n";
    let path = "src/test/kotlin/com/example/ValveTest.kt";
    for name in ["closes on timeout()", "closes on timeout"] {
        let (code, out) = bind_check(path, text, &[("com.example.ValveTest", name, "")]);
        assert!(!has(&out, "SD617"), "{name}: {out}");
        assert_eq!(code, 0, "{name}: {out}");
        let (code, out) = bind_check(path, text, &[("com.example.ValveTest", name, SKIPPED)]);
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{name}: {out}");
        assert!(found[0].contains("was skipped"), "{name}: {out}");
        assert_eq!(code, 1, "{name}: {out}");
    }
}

#[test]
fn a_go_method_of_a_testify_suite_binds_the_case_of_its_suite_function() {
    let text = "package valves\n\ntype ValveSuite struct{ suite.Suite }\n\nfunc TestValveSuite(t *testing.T) {\n\tsuite.Run(t, new(ValveSuite))\n}\n\n// covers: requirements/valves#flush\nfunc (s *ValveSuite) TestCloses() {\n}\n\nfunc (s *ValveSuite) TestOther() {\n}\n";
    let path = "valves/suite_test.go";
    let pkg = "example.org/app/valves";
    let report = |closes: &'static str| {
        [
            (pkg, "TestValveSuite", ""),
            (pkg, "TestValveSuite/TestCloses", closes),
            (pkg, "TestValveSuite/TestOther", ""),
        ]
    };
    let (code, out) = bind_check(path, text, &report(""));
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
    let (code, out) = bind_check(path, text, &report(SKIPPED));
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`TestValveSuite/TestCloses` was skipped"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // A case of the suite function alone, or of another method, binds nothing.
    let (code, out) = bind_check(
        path,
        text,
        &[
            (pkg, "TestValveSuite", ""),
            (pkg, "TestValveSuite/TestOther", ""),
        ],
    );
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A case of a suite in another package does not fit the file.
    let (code, out) = bind_check(
        path,
        text,
        &[
            ("example.org/app/pumps", "TestValveSuite/TestCloses", ""),
            (pkg, "TestValveSuite/TestCloses", SKIPPED),
        ],
    );
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_byte_order_mark_before_the_tag_of_line_1_does_not_hide_the_test() {
    let text = "\u{feff}# covers: requirements/valves#flush\ndef test_closes_on_timeout():\n    assert True\n";
    let (code, out) = bind_check(
        "tests/test_valves.py",
        text,
        &[("tests.test_valves", "test_closes_on_timeout", SKIPPED)],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_broken_quote_in_case_binds_no_case_and_never_the_test_under_the_tag() {
    // The test under the tag passed. The tag has a `"` inside its name, or a
    // quote that is not closed, so it must not bind that test.
    for tag in [
        "# covers: requirements/valves#flush case:\"say \"hi\" now\"",
        "# covers: requirements/valves#flush case:\"test_closes_on_timeout",
    ] {
        let text = format!("{tag}\ndef test_closes_on_timeout():\n    assert True\n");
        let (code, out) = bind_check(
            "tests/test_valves.py",
            &text,
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        );
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{tag}: {out}");
        assert!(
            found[0].contains("`case:` of the tag at tests/test_valves.py:1 is not valid"),
            "{tag}: {out}"
        );
        assert!(found[0].contains("no `\"` inside the name"), "{tag}: {out}");
        assert_eq!(code, 1, "{tag}: {out}");
    }
}

// The review of 0.8.1: false proofs of the new rule

#[test]
fn a_testify_method_binds_only_the_cases_of_the_suite_function_that_runs_its_type() {
    // The tag is above `ValveSuite.TestCloses`. `BSuite` has a method of the
    // same name in another file of the package.
    let a = "package valves\n\nfunc TestValveSuite(t *testing.T) {\n\tsuite.Run(t, new(ValveSuite))\n}\n\n// covers: requirements/valves#flush\nfunc (s *ValveSuite) TestCloses() {\n}\n";
    let b = "package valves\n\nfunc TestBSuite(t *testing.T) {\n\tsuite.Run(t, &BSuite{})\n}\n\nfunc (s *BSuite) TestCloses() {\n}\n";
    let pkg = "example.org/app/valves";
    let r = pr(
        "strict",
        REPORTS,
        &[("valves/suite_test.go", a), ("valves/b_test.go", b)],
    );
    r.write(
        "reports/r.xml",
        &junit(
            "go",
            &[
                (pkg, "TestValveSuite/TestCloses", SKIPPED),
                (pkg, "TestBSuite/TestCloses", ""),
            ],
        ),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`TestValveSuite/TestCloses` was skipped"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // With no case of the suite of the tag, nothing binds.
    let r = pr(
        "strict",
        REPORTS,
        &[("valves/suite_test.go", a), ("valves/b_test.go", b)],
    );
    r.write(
        "reports/r.xml",
        &junit("go", &[(pkg, "TestBSuite/TestCloses", "")]),
    );
    stamp(&r);
    let (code, out) = check(&r, &[]);
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A suite method binds no top-level test of the same name.
    let (code, out) = bind_check("valves/suite_test.go", a, &[(pkg, "TestCloses", "")]);
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
    // No suite function runs the type: the tag fails closed.
    let lone = "package valves\n\n// covers: requirements/valves#flush\nfunc (s *ValveSuite) TestCloses() {\n}\n";
    let (code, out) = bind_check(
        "valves/suite_test.go",
        lone,
        &[(pkg, "TestValveSuite/TestCloses", "")],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("names the test `TestCloses`"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_javascript_name_binds_the_exact_case_and_not_the_cases_of_a_table() {
    let text = "describe('Valves', () => {\n  // covers: requirements/valves#flush\n  it('flushes', () => {\n  });\n\n  it.each(['slow', 'fast'])('flushes (%s)', (m) => {\n  });\n});\n";
    let cl = "tests/valves.test.js";
    let (code, out) = bind_check(
        "tests/valves.test.js",
        text,
        &[
            (cl, "Valves > flushes", SKIPPED),
            (cl, "Valves > flushes (slow)", ""),
            (cl, "Valves > flushes (fast)", ""),
        ],
    );
    let found = lines(&out, "SD617");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`Valves > flushes` was skipped"), "{out}");
    assert_eq!(code, 1, "{out}");
    // With no case of the test, the cases of the table prove nothing.
    let (code, out) = bind_check(
        "tests/valves.test.js",
        text,
        &[
            (cl, "Valves > flushes (slow)", ""),
            (cl, "Valves > flushes (fast)", ""),
        ],
    );
    assert!(has(&out, "SD617"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_kotlin_name_in_backticks_binds_a_parameterized_case() {
    let text = "class ValveTest {\n    // covers: requirements/valves#flush\n    @Test fun `closes`(n: Int) {\n    }\n}\n";
    let (code, out) = bind_check(
        "src/test/kotlin/com/example/ValveTest.kt",
        text,
        &[("com.example.ValveTest", "closes(int) [1] 3", "")],
    );
    assert!(!has(&out, "SD617"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_case_with_a_space_after_the_colon_or_a_stray_quote_is_not_valid() {
    for tag in [
        "# covers: requirements/valves#flush case:",
        "# covers: requirements/valves#flush case: \"test_closes_on_timeout\"",
        "# covers: requirements/valves#flush case:\"test_closes_on_timeout\"-x\"",
        "# covers: requirements/valves#flush case:\"test_closes_on_timeout\" x\"",
    ] {
        let text = format!("{tag}\ndef test_closes_on_timeout():\n    assert True\n");
        let (code, out) = bind_check(
            "tests/test_valves.py",
            &text,
            &[("tests.test_valves", "test_closes_on_timeout", "")],
        );
        let found = lines(&out, "SD617");
        assert_eq!(found.len(), 1, "{tag}: {out}");
        assert!(found[0].contains("is not valid"), "{tag}: {out}");
        assert_eq!(code, 1, "{tag}: {out}");
    }
}
