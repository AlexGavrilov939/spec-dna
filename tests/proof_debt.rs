//! Integration tests of the proof debt (design 0.8.0, D2): the marker
//! `<!-- proof-owed: <reason> -->`, SD618 and SD619, the counts of `status` and
//! `stats`, and the rule that a debt is no proof for `finish`.

mod common;

use common::*;

const FOLDER: &str = "docs/intent/dose";

/// A document with 3 requirements. `first` and `second` are the text at the end
/// of `close` and `open`.
fn valves(first: &str, second: &str) -> String {
    format!(
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.{first}\n- **open**: The controller SHALL open the valve in 5 seconds.{second}\n- **flush**: The controller SHALL flush the line.\n"
    )
}

fn config(profile: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\nstale-days = 14\n"
    )
}

/// A repository on `main` with the document `valves` and the given tests.
fn repo(profile: &str, rules: &str, first: &str, second: &str) -> Repo {
    let r = Repo::with_config(&config(profile, rules));
    r.write("docs/requirements/valves.md", &valves(first, second));
    r.write("tests/test_other.py", "# nothing\n");
    r.commit_base_of_0_7_1("base");
    r
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

const MANUAL: &str = " <!-- proof-owed: manual -->";
const ENV: &str = " <!-- proof-owed: environment -->";

// SD618: a debt on a requirement with no evidence

#[test]
fn a_debt_replaces_sd608_with_the_warning_sd618_in_standard_and_strict() {
    for profile in ["standard", "strict"] {
        // `flush` has no marker, so SD608 stays for it.
        let r = repo(profile, "", MANUAL, ENV);
        let (code, out, err) = r.run(&["check", "--all"]);
        let owed = lines(&out, "SD618");
        assert_eq!(owed.len(), 2, "{profile}: {out}{err}");
        assert!(
            owed.iter()
                .all(|l| l.contains(": warning SD618 proof-owed")),
            "{out}"
        );
        assert!(owed[0].contains("`requirements/valves#close`"), "{out}");
        assert!(owed[0].contains("`manual`"), "{out}");
        assert!(owed[1].contains("`environment`"), "{out}");
        let unverified = lines(&out, "SD608");
        assert_eq!(unverified.len(), 1, "{profile}: {out}{err}");
        assert!(
            unverified[0].contains("`requirements/valves#flush`"),
            "{out}"
        );
        // SD608 is an error in `strict` and a warning in `standard`.
        let want = if profile == "strict" { 1 } else { 0 };
        assert_eq!(code, want, "{profile}: {out}{err}");
    }
}

#[test]
fn a_debt_makes_a_strict_project_with_no_other_finding_pass() {
    let r = Repo::with_config(&config("strict", ""));
    r.write(
        "docs/requirements/valves.md",
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds. <!-- proof-owed: environment -->\n",
    );
    r.commit_base_of_0_7_1("base");
    // Without the marker, SD608 would be an error in `strict`.
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(has(&out, "SD618") && !has(&out, "SD608"), "{out}");
}

#[test]
fn sd618_is_a_state_code_and_it_is_off_in_lite() {
    let r = repo("standard", "", MANUAL, "");
    // Never new: `check` against the base shows it with `--all` only.
    let (_, out, err) = r.run(&["check"]);
    assert!(!has(&out, "SD618"), "{out}{err}");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/requirements/valves.md", &valves(MANUAL, ENV));
    r.commit("more");
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    assert!(!has(&out, "SD618"), "{out}{err}");
    // In `lite` the codes are off, and a rule for the group turns them on.
    let r = repo("lite", "", MANUAL, "");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD618") && !has(&out, "SD619"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
    let r = repo(
        "lite",
        "proof = \"warning\"\nevidence = \"warning\"\n",
        MANUAL,
        "",
    );
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD618"), "{out}{err}");
}

#[test]
fn a_rule_for_the_group_evidence_does_not_change_the_proof_codes() {
    let r = repo(
        "standard",
        "evidence = \"off\"\n",
        MANUAL,
        " <!-- proof-owed: later -->",
    );
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD618") && has(&out, "SD619"), "{out}{err}");
    let r = repo(
        "standard",
        "proof = \"off\"\n",
        MANUAL,
        " <!-- proof-owed: later -->",
    );
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD618") && !has(&out, "SD619"), "{out}{err}");
}

#[test]
fn a_marker_keeps_sd608_where_sd618_is_off() {
    // `lite` turns SD618 off: the marker must not hide SD608, which a rule
    // for the group `evidence` turned on.
    let r = repo("lite", "evidence = \"warning\"\n", MANUAL, "");
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD618"), "{out}{err}");
    let found = lines(&out, "SD608");
    assert!(
        found
            .iter()
            .any(|l| l.contains("`requirements/valves#close`")),
        "{out}"
    );
    // The same in `strict` with a rule that turns the group `proof` off.
    let r = repo("strict", "proof = \"off\"\n", MANUAL, "");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD618"), "{out}{err}");
    let found = lines(&out, "SD608");
    assert!(
        found
            .iter()
            .any(|l| l.contains(": error SD608") && l.contains("`requirements/valves#close`")),
        "{out}"
    );
    assert_eq!(code, 1, "{out}{err}");
}

#[test]
fn a_rule_for_the_group_proof_alone_reads_the_tags_in_lite() {
    // The requirement has a tag and a marker. With only SD618 and SD619 on, the
    // tags must still load: SD619 (stale), and no SD618.
    let r = repo("lite", "proof = \"warning\"\n", MANUAL, "");
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\ndef test_closes():\n    assert True\n",
    );
    r.commit("tag");
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD619") && !has(&out, "SD618"), "{out}{err}");
}

// SD619: a reason that is not in the list, or a debt that is stale

#[test]
fn a_reason_that_is_not_in_the_list_gives_sd619_and_keeps_sd608() {
    for (profile, severity, code) in [("standard", "warning", 0), ("strict", "error", 1)] {
        let r = repo(profile, "", " <!-- proof-owed: later -->", "");
        let (got, out, err) = r.run(&["check", "--all"]);
        let found = lines(&out, "SD619");
        assert_eq!(found.len(), 1, "{profile}: {out}{err}");
        assert!(
            found[0].contains(&format!(": {severity} SD619 proof-owed-invalid")),
            "{out}"
        );
        assert!(found[0].contains("`later`"), "{out}");
        assert!(found[0].contains("`requirements/valves#close`"), "{out}");
        assert!(
            found[0].contains("manual, environment, external, pending"),
            "{out}"
        );
        // The invalid marker is no debt, so SD608 names each requirement.
        assert_eq!(lines(&out, "SD608").len(), 3, "{profile}: {out}");
        assert!(!has(&out, "SD618"), "{out}");
        // SD619 and SD608 both have the severity of the profile.
        assert_eq!(got, code, "{profile}: {out}{err}");
    }
    // An empty reason is not in the list either.
    let r = repo("standard", "", " <!-- proof-owed: -->", "");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD619"), "{out}");
}

#[test]
fn a_debt_on_a_requirement_with_a_tag_gives_sd619() {
    let r = repo("standard", "", MANUAL, "");
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\ndef test_closes():\n    assert True\n",
    );
    r.commit("tag");
    let (code, out, err) = r.run(&["check", "--all"]);
    let found = lines(&out, "SD619");
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].contains("has evidence"), "{out}");
    assert!(!has(&out, "SD618"), "{out}");
    assert_eq!(code, 0, "a warning in `standard`: {out}{err}");
}

#[test]
fn a_debt_on_a_requirement_with_a_verified_by_marker_gives_sd619() {
    let r = repo(
        "strict",
        "",
        " <!-- verified-by: tests/test_other.py --> <!-- proof-owed: manual -->",
        "",
    );
    let (code, out, err) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD619"), "{out}{err}");
    assert_eq!(code, 1, "an error in `strict`: {out}{err}");
}

#[test]
fn lite_gives_no_sd619() {
    let r = repo("lite", "", " <!-- proof-owed: later -->", "");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD619"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_marker_changes_no_hash_so_it_gives_no_sd914_and_no_sd921() {
    // The base has the requirement with no marker. The branch adds the marker
    // and changes nothing else: the hash is the same.
    let r = repo("strict", "SD608 = \"off\"\n", "", "");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/requirements/valves.md", &valves(MANUAL, ENV));
    r.commit("debt");
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert!(!has(&out, "SD921") && !has(&out, "SD914"), "{out}{err}");
    assert!(!has(&out, "SD619"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
}

// `status` and `stats`

#[test]
fn status_prints_one_line_for_the_debts_and_status_json_counts_them() {
    let r = repo("standard", "", MANUAL, ENV);
    let (code, out, err) = r.run(&["status"]);
    assert_eq!(code, 0, "{out}{err}");
    let proof: Vec<&str> = out.lines().filter(|l| l.starts_with("proof:")).collect();
    assert_eq!(proof, ["proof: 2 owed (manual 1, environment 1)"], "{out}");
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains(
            "\"proof\":{\"owed\":2,\"by_reason\":{\"manual\":1,\"environment\":1,\"external\":0,\"pending\":0}}"
        ),
        "{out}"
    );
    // One requirement with a tag is no debt: SD619 reports it.
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\ndef test_closes():\n    assert True\n",
    );
    r.commit("tag");
    let (_, out, _) = r.run(&["status"]);
    assert!(out.contains("proof: 1 owed (environment 1)"), "{out}");
}

#[test]
fn a_project_with_no_marker_gets_the_status_and_stats_of_0_7_1() {
    for profile in ["lite", "standard", "strict"] {
        let r = repo(profile, "", "", "");
        let (_, out, _) = r.run(&["status"]);
        assert!(!out.contains("proof"), "{profile}: {out}");
        let (_, out, _) = r.run(&["status", "--json"]);
        assert!(!out.contains("proof"), "{profile}: {out}");
        let (_, out, _) = r.run(&["stats", "--sdlc"]);
        assert!(!out.contains("proof"), "{profile}: {out}");
        let (_, out, _) = r.run(&["stats", "--sdlc", "--json"]);
        assert!(!out.contains("proof"), "{profile}: {out}");
    }
}

#[test]
fn in_lite_status_and_stats_print_no_proof_line_and_read_no_report() {
    // A marker, a tag and a fresh report in `lite`: the codes are off.
    let r = Repo::with_config(&format!(
        "{}[evidence]\nreports = [\"reports/*\"]\n",
        config("lite", "")
    ));
    r.write(".gitignore", "reports/\n");
    r.write("docs/requirements/valves.md", &valves(MANUAL, ""));
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#open\ndef test_opens():\n    assert True\n",
    );
    r.commit_base_of_0_7_1("base");
    r.write(
        "reports/junit.xml",
        "<testsuites><testsuite name=\"p\" tests=\"1\"><testcase classname=\"tests.test_valves\" name=\"test_opens\"/></testsuite></testsuites>",
    );
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, out, _) = r.run(&["status"]);
    assert!(!out.contains("proof:"), "{out}");
    let (_, out, _) = r.run(&["stats", "--sdlc"]);
    assert!(!out.contains("proof:"), "{out}");
    // The JSON has the counts of a project that has a marker, and the passed
    // case is not read: the tag is `tagged`.
    let (_, out, _) = r.run(&["stats", "--sdlc", "--json"]);
    assert!(
        out.contains("\"proof\":{\"reliable\":0,\"passed_once\":0,\"verified\":0,\"tagged\":1,\"owed\":1,\"none\":1,\"reports_read\":false}"),
        "{out}"
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(out.contains("\"proof\":{\"owed\":1,"), "{out}");
    // A rule for the group `proof` turns the line and the reports on.
    let r2 = Repo::with_config(&format!(
        "{}[evidence]\nreports = [\"reports/*\"]\n",
        config("lite", "proof = \"warning\"\n")
    ));
    r2.write(".gitignore", "reports/\n");
    r2.write("docs/requirements/valves.md", &valves(MANUAL, ""));
    r2.commit_base_of_0_7_1("base");
    let (_, out, _) = r2.run(&["status"]);
    assert!(out.contains("proof: 1 owed (manual 1)"), "{out}");
    let (_, out, _) = r2.run(&["stats", "--sdlc"]);
    assert!(out.contains("proof: reliable 0,"), "{out}");
}

const OPEN_REPORT: &str = "<testsuites><testsuite name=\"p\" tests=\"1\"><testcase classname=\"tests.test_valves\" name=\"test_opens\"/></testsuite></testsuites>";

/// A project with the tag of `open` committed, and a fresh report of its test.
fn tagged_with_a_report() -> Repo {
    let r = Repo::with_config(&format!(
        "{}[evidence]\nreports = [\"reports/*\"]\n",
        config("standard", "")
    ));
    r.write(".gitignore", "reports/\n");
    r.write("docs/requirements/valves.md", &valves("", ""));
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#open\ndef test_opens():\n    assert True\n",
    );
    r.commit_base_of_0_7_1("base");
    r.write("reports/junit.xml", OPEN_REPORT);
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    r
}

#[test]
fn stats_reads_no_report_while_the_code_tree_differs() {
    // A clean tree binds as before (guard).
    let r = tagged_with_a_report();
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"passed_once\":1,"), "{out}");
    let (_, text, _) = r.run(&["stats", "--sdlc"]);
    assert!(
        text.contains("proof: reliable 0, passed-once 1,") && !text.contains("no report read"),
        "{text}"
    );
    // An edit of a test file that `HEAD` does not have, and a report that is
    // fresh for the working tree: the tags read `HEAD`, so no report is read.
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#open\ndef test_opens():\n    assert True\n# edited\n",
    );
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"passed_once\":0,") && out.contains("\"tagged\":1,"),
        "{out}"
    );
    assert!(out.contains("\"reports_read\":false}"), "{out}");
    let (_, text, _) = r.run(&["stats", "--sdlc"]);
    assert!(
        text.contains("; no report read: the working tree has code edits that HEAD does not have"),
        "{text}"
    );
    // The commit of the edit makes the tree clean again: the reports bind.
    r.commit("edit the test");
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, out, _) = r.run(&["stats", "--sdlc", "--json"]);
    assert!(
        out.contains("\"passed_once\":1,") && out.contains("\"reports_read\":true}"),
        "{out}"
    );
}

#[test]
fn stats_with_an_unreadable_folder_of_reports_counts_no_report() {
    use std::os::unix::fs::PermissionsExt;
    let r = Repo::with_config(&format!(
        "{}[evidence]\nreports = [\"reports/*\"]\n",
        config("standard", "")
    ));
    r.write("docs/requirements/valves.md", &valves(MANUAL, ""));
    r.write("reports/keep", "x");
    r.commit_base_of_0_7_1("base");
    let dir = r.dir.join("reports");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Running as root can read the folder: then the test proves nothing.
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"proof\":{\"reliable\":0,"), "{out}");
}

#[test]
fn stats_json_has_one_count_for_each_state() {
    let r = repo("standard", "", MANUAL, "");
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#open\ndef test_opens():\n    assert True\n",
    );
    r.commit("tag");
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "\"proof\":{\"reliable\":0,\"passed_once\":0,\"verified\":0,\"tagged\":1,\"owed\":1,\"none\":1,\"reports_read\":false}"
        ),
        "{out}"
    );
}

// A debt is no proof for `finish`

/// The base: the change `dose` with a delta that adds `flush` with a debt. The
/// branch has no test.
fn finish_repo(profile: &str, delta_line: &str) -> Repo {
    let r = Repo::with_config(&config(profile, ""));
    r.write("docs/requirements/valves.md", &valves("", ""));
    r.write("src/valves.py", "x = 1\n");
    r.write(
        &format!("{FOLDER}/intent.md"),
        "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nFlush the line.\n",
    );
    r.write(
        &format!("{FOLDER}/plan.md"),
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
    );
    r.write(
        &format!("{FOLDER}/spec.md"),
        &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- {delta_line}\n"),
    );
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/valves.py", "x = 2\n");
    r.commit("code");
    r
}

#[test]
fn finish_stops_with_sd916_for_a_requirement_that_has_a_debt_and_no_tag() {
    let line = "ADD requirements/valves#purge: The controller SHALL purge the line. <!-- proof-owed: environment -->";
    for profile in ["standard", "strict"] {
        let r = finish_repo(profile, line);
        let (code, out, err) = r.run(&["finish", "dose"]);
        assert_eq!(code, 1, "{profile}: {out}{err}");
        assert!(err.contains("step 5"), "{err}");
        assert!(out.contains("SD916"), "{out}");
        assert!(r.dir.join(FOLDER).exists());
    }
}

#[test]
fn finish_goes_on_when_the_requirement_has_a_tag_and_a_stale_debt_only_warns() {
    let line = "ADD requirements/valves#purge: The controller SHALL purge the line. <!-- proof-owed: environment -->";
    let r = finish_repo("standard", line);
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#purge\ndef test_purges():\n    assert True\n",
    );
    r.commit("test");
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
}
