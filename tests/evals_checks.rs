//! Integration tests of SD506 (`evals-invalid`), SD918
//! (`incident-eval-missing`) and the SD503 exemption of the starter files
//! (step 4, slice S7).
//!
//! SD506 reads the `*.json` files directly in `[evals] dir` and the file
//! `<dir>/check.sh`. It has no profile row, so it is an error in each
//! profile. SD918 needs `[evals]` and a `[change] policy` other than `none`,
//! and it runs on the lifecycle base, so its fixtures have `origin/HEAD`.

mod common;

use common::*;
use std::fs;

const PROFILES: [&str; 3] = ["lite", "standard", "strict"];

const CHECK_SH: &str = "file_has() {\n  grep -q -F -- \"$2\" \"$1\"\n}\n";
const NOTES: &str = "status: draft\n";
const INCIDENT: &str = "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n## Root cause\n\nA worn seal.\n";

/// The text of an eval file. `over` replaces a key (`Some`), removes it
/// (`None`) or adds it, and the value is raw JSON.
fn eval(id: &str, over: &[(&str, Option<&str>)]) -> String {
    let mut pairs: Vec<(String, String)> = vec![
        ("id".into(), format!("\"{id}\"")),
        ("task".into(), "\"Change draft to done.\"".into()),
        ("fixture".into(), "\"evals/fixtures/example\"".into()),
        ("max-budget-usd".into(), "1".into()),
        ("runs".into(), "1".into()),
        (
            "checks".into(),
            "[{\"name\":\"done\",\"run\":\"file_has notes.txt done\"}]".into(),
        ),
    ];
    for (k, v) in over {
        pairs.retain(|(pk, _)| pk != k);
        if let Some(v) = v {
            pairs.push(((*k).to_string(), (*v).to_string()));
        }
    }
    let body: Vec<String> = pairs.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect();
    format!("{{\n  {}\n}}\n", body.join(",\n  "))
}

fn config(profile: &str, change: &str, extra: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n{change}\n[evals]\ndir = \"evals\"\n\n[rules]\nSD608 = \"off\"\n{extra}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
    )
}

/// A committed repository with `[evals]`, a valid eval and its fixture.
fn repo(profile: &str, extra: &str) -> Repo {
    let r = Repo::with_config(&config(profile, "", extra));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/check.sh", CHECK_SH);
    r.write("evals/example.json", &eval("example", &[]));
    r.write("evals/fixtures/example/notes.txt", NOTES);
    r.commit("base");
    r
}

fn check_all(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check", "--all"]);
    (code, out)
}

/// The lines of the diagnostics of `code`.
fn lines<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines()
        .filter(|l| {
            l.contains(&format!(": error {code} ")) || l.contains(&format!(": warning {code} "))
        })
        .collect()
}

// SD506

#[test]
fn a_valid_eval_gives_no_sd506_in_each_profile() {
    for profile in PROFILES {
        let r = repo(profile, "");
        let (code, out) = check_all(&r);
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(lines(&out, "SD506").is_empty(), "{profile}: {out}");
    }
}

#[test]
fn an_invalid_eval_gives_an_error_in_each_profile() {
    for profile in PROFILES {
        let r = repo(profile, "");
        r.write("evals/bad.json", "{\n  \"id\": \"bad\"\n}\n");
        let (code, out) = check_all(&r);
        assert_eq!(code, 1, "{profile}: {out}");
        assert_eq!(
            lines(&out, "SD506"),
            ["evals/bad.json:1: error SD506 evals-invalid: `task` is required"],
            "{profile}: {out}"
        );
        assert!(out.contains("SD506 fix: "), "{out}");
    }
}

#[test]
fn each_rule_of_the_eval_file_gives_its_key_as_the_target() {
    // The stem of the file, the text, and the key that the rule gives.
    let cases: Vec<(&str, String, &str)> = vec![
        ("not-json", "{".into(), "file"),
        ("an-array", "[]\n".into(), "file"),
        ("unknown", eval("unknown", &[("extra", Some("1"))]), "extra"),
        ("other", eval("someone", &[]), "id"),
        ("no-task", eval("no-task", &[("task", None)]), "task"),
        (
            "dash-task",
            eval("dash-task", &[("task", Some("\"-p x\""))]),
            "task",
        ),
        (
            "no-fixture",
            eval(
                "no-fixture",
                &[("fixture", Some("\"evals/fixtures/absent\""))],
            ),
            "fixture",
        ),
        (
            "result-fixture",
            eval(
                "result-fixture",
                &[("fixture", Some("\"evals/results/example\""))],
            ),
            "fixture",
        ),
        (
            "bad-branch",
            eval("bad-branch", &[("branch", Some("\"a..b\""))]),
            "branch",
        ),
        (
            "bad-tools",
            eval("bad-tools", &[("allowed-tools", Some("[\"\"]"))]),
            "allowed-tools",
        ),
        (
            "dash-tool",
            eval(
                "dash-tool",
                &[("allowed-tools", Some("[\"Read\", \"--bare\"]"))],
            ),
            "allowed-tools",
        ),
        (
            "no-turns",
            eval("no-turns", &[("max-turns", Some("0"))]),
            "max-turns",
        ),
        (
            "no-budget",
            eval("no-budget", &[("max-budget-usd", None)]),
            "max-budget-usd",
        ),
        (
            "big-budget",
            eval("big-budget", &[("max-budget-usd", Some("21"))]),
            "max-budget-usd",
        ),
        ("no-runs", eval("no-runs", &[("runs", Some("0"))]), "runs"),
        (
            "bad-pass",
            eval("bad-pass", &[("pass-at", Some("2"))]),
            "pass-at",
        ),
        (
            "no-checks",
            eval("no-checks", &[("checks", None)]),
            "checks",
        ),
        (
            "empty-checks",
            eval("empty-checks", &[("checks", Some("[]"))]),
            "checks",
        ),
        (
            "dup-checks",
            eval(
                "dup-checks",
                &[(
                    "checks",
                    Some("[{\"name\":\"a\",\"run\":\"true\"},{\"name\":\"a\",\"run\":\"true\"}]"),
                )],
            ),
            "checks",
        ),
    ];
    let r = repo("standard", "");
    for (stem, text, _) in &cases {
        r.write(&format!("evals/{stem}.json"), text);
    }
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 1, "{out}");
    for (stem, _, key) in &cases {
        let path = format!("\"path\":\"evals/{stem}.json\"");
        let found = out
            .split("{\"code\":")
            .filter(|d| d.starts_with("\"SD506\"") && d.contains(&path))
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{stem}: {out}");
        assert!(
            found[0].contains(&format!("\"target\":\"{key}\"")),
            "{stem}: {}",
            found[0]
        );
        assert!(found[0].contains("\"line\":1"), "{stem}: {}", found[0]);
        assert!(found[0].contains("\"severity\":\"error\""), "{}", found[0]);
    }
    // The valid eval gives nothing.
    assert!(!out.contains("evals/example.json"), "{out}");
}

#[test]
fn a_file_that_is_not_utf8_gives_sd506() {
    let r = repo("standard", "");
    fs::write(r.dir.join("evals/binary.json"), [0xff, 0xfe, b'{', b'}']).unwrap();
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    assert_eq!(
        lines(&out, "SD506"),
        ["evals/binary.json:1: error SD506 evals-invalid: the file is not UTF-8"],
        "{out}"
    );
}

#[test]
fn a_file_over_64_kib_gives_sd506() {
    let r = repo("standard", "");
    let big = format!("{{\"id\": \"big\", \"task\": \"{}\"}}", "x".repeat(70_000));
    r.write("evals/big.json", &big);
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    assert_eq!(
        lines(&out, "SD506"),
        ["evals/big.json:1: error SD506 evals-invalid: the file has more than 64 KiB"],
        "{out}"
    );
    // A committed file too: the base tree is read from a blob.
    r.commit("big");
    let (code, out, _) = r.run(&["check", "--all", "--staged"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
}

#[test]
fn an_absent_check_file_gives_one_sd506_on_the_check_file() {
    for profile in PROFILES {
        let r = repo(profile, "");
        fs::remove_file(r.dir.join("evals/check.sh")).unwrap();
        // The file is deleted in the working tree and not committed.
        let (code, out) = check_all(&r);
        assert_eq!(code, 1, "{profile}: {out}");
        assert_eq!(
            lines(&out, "SD506"),
            [
                "evals/check.sh:1: error SD506 evals-invalid: the check file is absent; `eval run` sources it"
            ],
            "{profile}: {out}"
        );
        let (_, json, _) = r.run(&["check", "--all", "--json"]);
        assert!(json.contains("\"target\":\"check.sh\""), "{json}");
    }
}

#[test]
fn sd506_reads_no_file_in_a_subfolder_and_no_other_file() {
    let r = repo("standard", "");
    r.write("evals/results/eval-20260101T000000Z.json", "not json");
    r.write("evals/fixtures/example/data.json", "{");
    r.write("evals/notes.txt", "{");
    r.write("evals/README.md", "# Evals\n");
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
}

#[test]
fn sd506_needs_the_evals_table_and_ignores_the_change_policy() {
    // No `[evals]`: no SD506, whatever the files are.
    let r = Repo::with_config(
        "profile = \"standard\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
    );
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/bad.json", "{");
    r.commit("base");
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
    // `[evals]` with the policy `none` still gives SD506.
    let r = Repo::with_config(&config("standard", "[change]\npolicy = \"none\"\n", ""));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/check.sh", CHECK_SH);
    r.commit("base");
    r.write("evals/bad.json", "{");
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
}

#[test]
fn a_rule_changes_the_severity_of_sd506() {
    let r = repo("standard", "SD506 = \"off\"\n");
    r.write("evals/bad.json", "{");
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
    let r = repo("standard", "SD506 = \"warning\"\n");
    r.write("evals/bad.json", "{");
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
    assert!(
        lines(&out, "SD506")[0].contains(": warning SD506 "),
        "{out}"
    );
}

#[test]
fn an_untracked_eval_file_counts_in_the_working_tree_only() {
    let r = repo("standard", "");
    origin_head(&r.dir, "main");
    r.write("evals/bad.json", "{");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
    // A gate against `--base` reads tracked files only (as CI does).
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
}

#[test]
fn a_staged_eval_file_counts_and_an_unstaged_one_does_not() {
    let r = repo("standard", "");
    r.write("evals/bad.json", "{");
    r.git(&["add", "evals/bad.json"]);
    let (code, out, _) = r.run(&["check", "--staged"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
    let r = repo("standard", "");
    r.write("evals/bad.json", "{");
    let (code, out, _) = r.run(&["check", "--staged"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_finding_of_the_base_is_not_new() {
    let r = repo("standard", "");
    r.write("evals/old.json", &eval("old", &[("task", None)]));
    r.commit("an invalid eval");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/valves.md", "# Valves\n\nMore text.\n");
    r.commit("an edit");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
    // `--all` lists it.
    let (_, out) = check_all(&r);
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
    // Another file that fails the same rule is a new finding.
    r.write("evals/newer.json", &eval("newer", &[("task", None)]));
    r.commit("a second invalid eval");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(
        lines(&out, "SD506"),
        ["evals/newer.json:1: error SD506 evals-invalid: `task` is required"],
        "{out}"
    );
}

#[test]
fn the_base_findings_use_the_evals_table_of_the_base() {
    // The base has no `[evals]`, so the head findings are all new, also for a
    // file that the base already had.
    let r = Repo::with_config(
        "profile = \"standard\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
    );
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/bad.json", "{");
    r.write("evals/check.sh", CHECK_SH);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{VERSION}\"\nprofile = \"standard\"\n[evals]\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
        ),
    );
    r.commit("add the evals table");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD506").len(), 1, "{out}");
}

// SD918

const CHANGE: &str = "[change]\npolicy = \"three-class\"\n";

/// A fixture with a lifecycle base. `extra` is added to `[rules]`, and
/// `change` is the `[change]` table.
fn change_repo(profile: &str, change: &str, extra: &str) -> Repo {
    let r = Repo::with_config(&config(profile, change, extra));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("docs/incidents/old-leak.md", INCIDENT);
    r.write("evals/check.sh", CHECK_SH);
    r.write("evals/example.json", &eval("example", &[]));
    r.write("evals/fixtures/example/notes.txt", NOTES);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

fn sd918(r: &Repo) -> Vec<String> {
    let (_, out, _) = r.run(&["check"]);
    lines(&out, "SD918").iter().map(|l| l.to_string()).collect()
}

#[test]
fn an_added_incident_with_no_eval_gives_sd918_by_profile() {
    for (profile, want) in [
        ("lite", None),
        ("standard", Some("warning")),
        ("strict", Some("warning")),
    ] {
        let r = change_repo(profile, CHANGE, "");
        r.write("docs/incidents/hose-leak.md", INCIDENT);
        r.commit("incident");
        let found = sd918(&r);
        match want {
            Some(sev) => {
                assert_eq!(found.len(), 1, "{profile}: {found:?}");
                assert!(
                    found[0].starts_with(&format!(
                        "docs/incidents/hose-leak.md:1: {sev} SD918 incident-eval-missing: "
                    )),
                    "{found:?}"
                );
                assert!(found[0].contains("`incidents/hose-leak`"), "{found:?}");
                let (code, ..) = r.run(&["check"]);
                assert_eq!(code, 0, "a warning does not fail check");
                let (_, json, _) = r.run(&["check", "--json"]);
                assert!(
                    json.contains("\"target\":\"incidents/hose-leak\""),
                    "{json}"
                );
                assert!(json.contains("\"new\":true"), "{json}");
            }
            None => assert!(found.is_empty(), "{profile}: {found:?}"),
        }
    }
}

#[test]
fn an_untracked_incident_gives_sd918() {
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    assert_eq!(sd918(&r).len(), 1);
}

#[test]
fn each_added_incident_gives_one_diagnostic() {
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.write("docs/incidents/pump-stall.md", INCIDENT);
    r.commit("incidents");
    assert_eq!(sd918(&r).len(), 2);
}

#[test]
fn sd918_is_off_with_the_policy_none_and_with_no_evals_table() {
    let r = change_repo("standard", "[change]\npolicy = \"none\"\n", "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert!(sd918(&r).is_empty());
    // A rule cannot turn it on with the policy `none`.
    let r = change_repo(
        "standard",
        "[change]\npolicy = \"none\"\n",
        "SD918 = \"error\"\n",
    );
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert!(sd918(&r).is_empty());
    // No `[evals]`.
    let r = Repo::with_config(&format!(
        "profile = \"standard\"\n{CHANGE}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
    ));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert!(sd918(&r).is_empty());
}

#[test]
fn a_rule_changes_the_severity_of_sd918() {
    let r = change_repo("standard", CHANGE, "SD918 = \"error\"\n");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert!(sd918(&r)[0].contains(": error SD918 "));
    let (code, ..) = r.run(&["check"]);
    assert_eq!(code, 1);
    // The group `changes` turns it off.
    let r = change_repo("standard", CHANGE, "changes = \"off\"\n");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert!(sd918(&r).is_empty());
    // And `lite` with a rule turns it on.
    let r = change_repo("lite", CHANGE, "SD918 = \"warning\"\n");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    assert_eq!(sd918(&r).len(), 1);
}

#[test]
fn an_added_or_changed_eval_file_satisfies_sd918() {
    // A new eval, in each profile.
    for profile in PROFILES {
        let r = change_repo(profile, CHANGE, "");
        r.write("docs/incidents/hose-leak.md", INCIDENT);
        r.write("evals/hose-leak.json", &eval("hose-leak", &[]));
        r.commit("incident and eval");
        assert!(sd918(&r).is_empty(), "{profile}");
        let (code, out, _) = r.run(&["check"]);
        assert_eq!(code, 0, "{profile}: {out}");
    }
    // A changed eval (D53).
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.write(
        "evals/example.json",
        &eval("example", &[("runs", Some("2"))]),
    );
    r.commit("incident and a changed eval");
    assert!(sd918(&r).is_empty());
    // An untracked eval in the working tree.
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.write("evals/hose-leak.json", &eval("hose-leak", &[]));
    assert!(sd918(&r).is_empty());
}

#[test]
fn only_a_json_file_directly_in_the_evals_dir_satisfies_sd918() {
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.write("evals/results/eval-20260101T000000Z.json", "{}");
    r.write("evals/fixtures/example/notes.txt", "status: done\n");
    r.write("evals/fixtures/other/data.json", "{}");
    r.write("evals/notes.txt", "text\n");
    r.write("other/hose-leak.json", &eval("hose-leak", &[]));
    r.commit("incident and files that are not evals");
    assert_eq!(sd918(&r).len(), 1);
    // The deletion of an eval does not satisfy it.
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    fs::remove_file(r.dir.join("evals/example.json")).unwrap();
    r.commit("incident and a deleted eval");
    assert_eq!(sd918(&r).len(), 1);
}

#[test]
fn a_changed_or_a_moved_incident_gives_no_sd918() {
    // A change of an incident that the base has.
    let r = change_repo("standard", CHANGE, "");
    r.write(
        "docs/incidents/old-leak.md",
        &INCIDENT.replace("A worn seal.", "A cracked seal."),
    );
    r.commit("edit");
    assert!(sd918(&r).is_empty());
    // A move, staged. The new path has another id, so only the rename
    // detection of git tells that it is a move.
    let r = change_repo("standard", CHANGE, "");
    r.git(&[
        "mv",
        "docs/incidents/old-leak.md",
        "docs/incidents/archived-leak.md",
    ]);
    assert!(sd918(&r).is_empty());
    // A move, committed.
    r.commit("move");
    assert!(sd918(&r).is_empty());
    // A move with a small edit still counts as a move.
    let r = change_repo("standard", CHANGE, "");
    r.git(&[
        "mv",
        "docs/incidents/old-leak.md",
        "docs/incidents/archived-leak.md",
    ]);
    r.write(
        "docs/incidents/archived-leak.md",
        &INCIDENT.replace("A worn seal.", "A worn seal. It was old."),
    );
    r.commit("move and edit");
    assert!(sd918(&r).is_empty());
}

#[test]
fn a_moved_incident_that_keeps_its_old_id_as_an_alias_gives_no_sd918() {
    // The move is not staged, so git cannot detect the rename. The new file
    // keeps the old id as an alias, as `spec-dna mv` writes it.
    let r = change_repo("standard", CHANGE, "");
    fs::remove_file(r.dir.join("docs/incidents/old-leak.md")).unwrap();
    r.write(
        "docs/incidents/archived-leak.md",
        &INCIDENT.replace(
            "type: incident\n",
            "type: incident\naliases: incidents/old-leak\n",
        ),
    );
    assert!(sd918(&r).is_empty());
    // Without the alias, the file is a new incident.
    r.write("docs/incidents/archived-leak.md", INCIDENT);
    assert_eq!(sd918(&r).len(), 1);
}

#[test]
fn a_document_that_is_not_an_incident_gives_no_sd918() {
    let r = change_repo("standard", CHANGE, "");
    r.write("docs/notes.md", "# Notes\n\nText.\n");
    r.commit("a document");
    assert!(sd918(&r).is_empty());
}

#[test]
fn sd918_needs_a_lifecycle_base() {
    let r = Repo::with_config(&config("standard", CHANGE, ""));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/check.sh", CHECK_SH);
    r.commit("base");
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD918").is_empty(), "{out}");
    assert!(err.contains("no base for the change checks"), "{err}");
    // With `--base`, it runs.
    r.commit("incident");
    let (_, out, _) = r.run(&["check", "--base", "HEAD~1"]);
    assert_eq!(lines(&out, "SD918").len(), 1, "{out}");
}

// SD503

#[test]
fn the_starter_files_of_skeleton_pass_sd503_and_pass_sd506() {
    for dir in ["evals", ".spec-dna/evals"] {
        let r = Repo::with_config(
            "profile = \"standard\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
        );
        r.write("docs/valves.md", "# Valves\n\nText.\n");
        r.commit("base");
        // `init --reconfigure` to `skeleton`: the table and the 4 files.
        let toml = r.read("spec-dna.toml");
        r.write(
            "spec-dna.toml",
            &format!("{toml}\n[evals]\ndir = \"{dir}\"\n"),
        );
        r.write(&format!("{dir}/check.sh"), CHECK_SH);
        r.write(
            &format!("{dir}/example.json"),
            &eval(
                "example",
                &[("fixture", Some(&format!("\"{dir}/fixtures/example\"")))],
            ),
        );
        r.write(&format!("{dir}/fixtures/example/notes.txt"), NOTES);
        r.write(
            ".github/workflows/agent-evals.yml",
            "name: Agent evals\non: workflow_dispatch\n",
        );
        let (code, out, _) = r.run(&["check"]);
        assert_eq!(code, 0, "{dir}: {out}");
        assert!(lines(&out, "SD503").is_empty(), "{dir}: {out}");
        assert!(lines(&out, "SD506").is_empty(), "{dir}: {out}");
        // Another file in the same change gives SD503.
        r.write("src/x.py", "x = 1\n");
        let (code, out, _) = r.run(&["check"]);
        assert_eq!(code, 1, "{dir}: {out}");
        assert_eq!(lines(&out, "SD503").len(), 1, "{dir}: {out}");
        assert!(out.contains("(first: `src/x.py`)"), "{dir}: {out}");
        assert!(out.contains("1 other file"), "{dir}: {out}");
    }
}

#[test]
fn the_starter_paths_are_not_exempt_without_an_evals_table() {
    let r = Repo::with_config(
        "profile = \"standard\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
    );
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.commit("base");
    let toml = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &format!("{toml}\n# a comment\n"));
    r.write("evals/check.sh", CHECK_SH);
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(lines(&out, "SD503").len(), 1, "{out}");
}

#[test]
fn a_path_in_the_evals_dir_that_is_not_a_starter_file_gives_sd503() {
    let r = repo("standard", "");
    let toml = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &format!("{toml}\n# a comment\n"));
    r.write("evals/second.json", &eval("second", &[]));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("(first: `evals/second.json`)"), "{out}");
}

// The key `skill` (ADR 0016).

#[test]
fn the_key_skill_takes_the_name_of_a_skill_of_the_pack() {
    let r = repo("standard", "");
    for (i, skill) in [
        "brainstorm",
        "specify",
        "clarify",
        "write-plan",
        "execute",
        "tdd",
        "review",
        "verify",
        "finish",
        "debug",
        "post-mortem",
    ]
    .iter()
    .enumerate()
    {
        let id = format!("skill-{i}");
        r.write(
            &format!("evals/{id}.json"),
            &eval(&id, &[("skill", Some(&format!("\"{skill}\"")))]),
        );
    }
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
}

#[test]
fn the_key_skill_accepts_agents_line() {
    // `agents-line` is no skill, but the pair `agents-line` of `eval run --ab`
    // has evals of its own, and they name this group.
    let r = repo("standard", "");
    r.write(
        "evals/line.json",
        &eval("line", &[("skill", Some("\"agents-line\""))]),
    );
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
    assert!(lines(&out, "SD506").is_empty(), "{out}");
}

#[test]
fn a_bad_value_of_the_key_skill_gives_sd506_with_the_key_as_the_target() {
    let r = repo("standard", "");
    for (stem, value) in [
        ("old-name", "\"intent\""),
        ("adapter", "\"spec-dna\""),
        ("prefix", "\"spec-dna-tdd\""),
        ("upper", "\"TDD\""),
        ("empty", "\"\""),
        ("number", "1"),
        ("list", "[\"tdd\"]"),
        ("agents-line-prefix", "\"agents-line-1\""),
    ] {
        r.write(
            &format!("evals/{stem}.json"),
            &eval(stem, &[("skill", Some(value))]),
        );
    }
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 1, "{out}");
    let found: Vec<&str> = out
        .split("{\"code\":")
        .filter(|d| d.starts_with("\"SD506\""))
        .collect();
    assert_eq!(found.len(), 8, "{out}");
    for d in found {
        assert!(d.contains("\"target\":\"skill\""), "{d}");
        assert!(d.contains("`skill`"), "{d}");
    }
}

// SD507 `skill-unproven` (ADR 0016).

fn sd507_repo(profile: &str, skills: &[&str]) -> Repo {
    let list: Vec<String> = skills.iter().map(|s| format!("\"{s}\"")).collect();
    let r = Repo::with_config(&format!(
        "profile = \"{profile}\"\n\n[evals]\ndir = \"evals\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n\n[harness]\nlevel = \"lean\"\nskills = [{}]\n",
        list.join(", ")
    ));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.write("evals/check.sh", CHECK_SH);
    r.commit("start");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("init");
    r
}

fn skills_file(items: &[(&str, &str)], stopped: &str) -> String {
    let list: Vec<String> = items
        .iter()
        .map(|(n, v)| format!("{{\"name\":\"{n}\",\"verdict\":{v},\"rule\":null}}"))
        .collect();
    format!(
        "{{\"spec_dna\":\"0.7.0\",\"started\":\"2026-11-20T10:00:00Z\",\"stopped\":{stopped},\"skills\":[{}]}}\n",
        list.join(",")
    )
}

/// The skills that the lines of SD507 name.
fn unproven(out: &str) -> Vec<String> {
    let mut v: Vec<String> = lines(out, "SD507")
        .iter()
        .map(|l| {
            let msg = l.split("skill-unproven: ").nth(1).unwrap();
            msg.split('`').nth(1).unwrap().to_string()
        })
        .collect();
    v.sort();
    v
}

#[test]
fn sd507_is_off_in_lite_and_standard_and_a_warning_in_strict() {
    for (profile, want) in [
        ("lite", vec![]),
        ("standard", vec![]),
        ("strict", vec!["execute".to_string(), "verify".to_string()]),
    ] {
        // `execute` and `verify` are `inconclusive` in the table of the release.
        let r = sd507_repo(profile, &["brainstorm", "execute", "verify"]);
        let (_, out) = check_all(&r);
        assert_eq!(unproven(&out), want, "{profile}: {out}");
        if profile == "strict" {
            assert!(lines(&out, "SD507")[0].contains("spec-dna.toml:"), "{out}");
            assert!(
                lines(&out, "SD507")[0].contains(": warning SD507 "),
                "{out}"
            );
            assert!(out.contains("SD507 fix: "), "{out}");
        }
    }
}

#[test]
fn a_skill_that_the_table_of_the_release_holds_on_is_not_reported() {
    // The release holds `tdd` as `on` (rule 1, the records of the pilot).
    let r = sd507_repo("strict", &["brainstorm", "tdd"]);
    let (_, out) = check_all(&r);
    assert!(unproven(&out).is_empty(), "{out}");
}

#[test]
fn a_skill_that_is_on_in_the_design_is_exempt() {
    let r = sd507_repo(
        "strict",
        &[
            "brainstorm",
            "specify",
            "clarify",
            "write-plan",
            "review",
            "finish",
            "post-mortem",
        ],
    );
    let (_, out) = check_all(&r);
    assert!(unproven(&out).is_empty(), "{out}");
}

#[test]
fn only_the_verdict_on_of_a_committed_result_clears_sd507() {
    let r = sd507_repo("strict", &["tdd", "verify", "debug", "execute"]);
    r.write(
        "evals/results/skills-20261120T100000Z.json",
        &skills_file(
            &[
                ("tdd", "\"on\""),
                ("verify", "\"inconclusive\""),
                ("debug", "\"off\""),
            ],
            "null",
        ),
    );
    r.commit("result");
    let (_, out) = check_all(&r);
    assert_eq!(unproven(&out), ["debug", "execute", "verify"], "{out}");
}

#[test]
fn the_newest_file_that_holds_the_skill_decides_and_a_stopped_file_does_not_count() {
    let r = sd507_repo("strict", &["tdd", "verify"]);
    r.write(
        "evals/results/skills-20261120T100000Z.json",
        &skills_file(&[("tdd", "\"on\""), ("verify", "\"on\"")], "null"),
    );
    // Newer: `tdd` is now inconclusive, and a file of a stopped run follows.
    r.write(
        "evals/results/skills-20261121T100000Z.json",
        &skills_file(&[("tdd", "\"inconclusive\"")], "null"),
    );
    r.write(
        "evals/results/skills-20261122T100000Z.json",
        &skills_file(&[("tdd", "null"), ("verify", "null")], "\"api-error\""),
    );
    r.commit("results");
    let (_, out) = check_all(&r);
    assert_eq!(unproven(&out), ["tdd"], "{out}");
}

#[test]
fn sd507_reads_the_result_files_of_the_working_tree_and_ignores_other_files() {
    let r = sd507_repo("strict", &["tdd"]);
    r.write(
        "evals/results/skills-20261120T100000Z.json",
        &skills_file(&[("tdd", "\"on\"")], "null"),
    );
    // Not a result of `--ab`, and not valid JSON: no effect.
    r.write(
        "evals/results/eval-20261121T100000Z.json",
        "{\"pass_rate\":1}\n",
    );
    r.write("evals/results/skills-bad.json", "{ not json\n");
    let (_, out) = check_all(&r);
    assert!(unproven(&out).is_empty(), "{out}");
}

#[test]
fn sd507_needs_no_evals_table_and_a_rule_changes_its_severity() {
    let r = sd507_repo("strict", &["execute"]);
    let toml = r
        .read("spec-dna.toml")
        .replace("[evals]\ndir = \"evals\"\n\n", "");
    r.write("spec-dna.toml", &toml);
    r.commit("no table");
    let (_, out) = check_all(&r);
    assert_eq!(unproven(&out), ["execute"], "{out}");
    let toml = r
        .read("spec-dna.toml")
        .replace("[rules]\n", "[rules]\nSD507 = \"off\"\n");
    r.write("spec-dna.toml", &toml);
    let (_, out) = check_all(&r);
    assert!(unproven(&out).is_empty(), "{out}");
}
