//! Integration tests of `check` on the lifecycle base (step 3, slice S4):
//! SD910, SD913, SD917, SD905 (the part for a change with no delta), SD912 (the
//! base part) and SD921 on the merge base with `origin/HEAD`.
//!
//! Each fixture has a `main` branch with the base commit, a `refs/remotes/origin/HEAD`
//! that names it, and a `feature` branch. A test changes the `feature` branch.
//! The fixture has these layers: `requirements`, `interfaces` and `rules` hold
//! the truth, and `intent` moves into them.

mod common;

use common::*;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n";
const PUMPS: &str = "---\nstatus: active\n---\n# Pumps\n\n## Requirements\n\n- **stop**: The controller SHALL stop the pump at 90 percent.\n";
const TANKS: &str = "---\nstatus: active\n---\n# Tanks\n\n## Requirements\n\n- **fill**: The controller SHALL fill the tank to 80 percent.\n";
const HTTP: &str = "---\nstatus: active\n---\n# Http\n\n## Requirements\n\n- **status**: The interface SHALL return 200 for a valid call.\n";
const SAFETY: &str = "---\nstatus: active\n---\n# Safety\n\n## Requirements\n\n- **alarm**: The controller SHALL sound the alarm at 95 percent.\n";

const INTENT: &str = "docs/intent/dose/intent.md";
const SPEC: &str = "docs/intent/dose/spec.md";
const PLAN: &str = "docs/intent/dose/plan.md";

fn intent(status: &str, class: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: {class}\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n"
    )
}

fn spec(status: &str, delta: &[&str]) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Spec\n\n## Delta\n\n{}\n",
        delta.join("\n")
    )
}

fn plan(status: &str, files: &[&str]) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("- `{f}`")).collect();
    format!(
        "---\nstatus: {status}\n---\n# Plan\n\n## Files\n\n{}\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        items.join("\n")
    )
}

fn config(profile: &str, change: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.interfaces]\ntypes = [\"interface\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\", \"interfaces\", \"rules\"]\nkeep = \"until-finished\"\n"
    )
}

/// The base commit on `main`, `origin/HEAD` on it, and the branch `feature`.
/// `base` adds the files of the change before the commit.
fn start_with(profile: &str, change: &str, base: impl Fn(&Repo)) -> Repo {
    let r = Repo::with_config(&config(profile, change));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/requirements/pumps.md", PUMPS);
    r.write("docs/requirements/tanks.md", TANKS);
    r.write("docs/interfaces/http.md", HTTP);
    r.write("docs/rules/safety.md", SAFETY);
    r.write("src/dose.py", "x = 1\n");
    r.write("src/other.py", "y = 1\n");
    r.write("src/payments/pay.py", "z = 1\n");
    base(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

/// A fixture with the gate `block` and the profile `standard`.
fn start(base: impl Fn(&Repo)) -> Repo {
    start_with("standard", "gate = \"block\"\n", base)
}

fn nothing(_: &Repo) {}

/// `check`: the exit code and the text.
fn check(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check"]);
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

fn has(out: &str, code: &str) -> bool {
    !lines(out, code).is_empty()
}

// SD910: the order of artifacts

#[test]
fn an_intent_is_the_first_artifact_and_needs_no_approval() {
    let r = start(nothing);
    r.write(INTENT, &intent("draft", "routine"));
    let (code, out) = check(&r);
    assert!(!has(&out, "SD910"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_spec_needs_an_accepted_intent_in_the_base() {
    for (status, fires) in [("draft", true), ("accepted", false), ("closed", true)] {
        let r = start(|r| r.write(INTENT, &intent(status, "higher-risk")));
        r.write(SPEC, &spec("draft", &[]));
        let (code, out) = check(&r);
        let found = lines(&out, "SD910");
        if fires {
            assert_eq!(found.len(), 1, "{status}: {out}");
            assert!(
                found[0].starts_with(&format!("{SPEC}:1: error SD910 ")),
                "{out}"
            );
            assert!(found[0].contains("no `accepted` `intent.md`"), "{out}");
            assert_eq!(code, 1, "{status}: {out}");
        } else {
            assert!(found.is_empty(), "{status}: {out}");
            assert_eq!(code, 0, "{status}: {out}");
        }
    }
}

#[test]
fn a_plan_of_a_routine_change_needs_an_accepted_intent() {
    for (status, fires) in [("draft", true), ("accepted", false)] {
        let r = start(|r| r.write(INTENT, &intent(status, "routine")));
        r.write(PLAN, &plan("draft", &["src/dose.py"]));
        let (_, out) = check(&r);
        assert_eq!(has(&out, "SD910"), fires, "{status}: {out}");
        if fires {
            assert!(lines(&out, "SD910")[0].contains(PLAN), "{out}");
        }
    }
}

#[test]
fn a_plan_of_a_higher_risk_change_needs_an_approved_spec() {
    // The intent is accepted in both runs. Only the spec decides.
    for (spec_status, fires) in [("draft", true), ("approved", false)] {
        let r = start(|r| {
            r.write(INTENT, &intent("accepted", "higher-risk"));
            r.write(SPEC, &spec(spec_status, &[]));
        });
        r.write(PLAN, &plan("draft", &["src/dose.py"]));
        let (_, out) = check(&r);
        assert_eq!(has(&out, "SD910"), fires, "{spec_status}: {out}");
        if fires {
            let l = lines(&out, "SD910")[0];
            assert!(
                l.contains(PLAN) && l.contains("no `approved` `spec.md`"),
                "{out}"
            );
        }
    }
}

#[test]
fn code_needs_an_approved_plan_in_the_base() {
    for (status, fires) in [("draft", true), ("approved", false)] {
        let r = start(|r| {
            r.write(INTENT, &intent("accepted", "routine"));
            r.write(PLAN, &plan(status, &["src/dose.py"]));
        });
        r.write("src/dose.py", "x = 2\n");
        let (code, out) = check(&r);
        let found = lines(&out, "SD910");
        if fires {
            assert_eq!(found.len(), 1, "{status}: {out}");
            assert!(
                found[0].starts_with("src/dose.py:1: error SD910 ")
                    && found[0].contains("no `approved` plan"),
                "{out}"
            );
            assert_eq!(code, 1, "{out}");
        } else {
            assert!(found.is_empty(), "{status}: {out}");
            assert_eq!(code, 0, "{status}: {out}");
        }
    }
}

#[test]
fn code_under_an_unapproved_plan_counts_with_no_change_in_the_diff() {
    // The plan is `draft` in the base, and the diff touches no file of the
    // folder. The path matches `Files`, so it is an artifact (K1). A path that
    // no `Files` item matches is a small fix.
    let r = start(|r| {
        r.write(INTENT, &intent("accepted", "routine"));
        r.write(PLAN, &plan("draft", &["src/dose.py"]));
    });
    r.write("src/other.py", "y = 2\n");
    let (code, out) = check(&r);
    assert!(!has(&out, "SD910") && !has(&out, "SD913"), "{out}");
    assert_eq!(code, 0, "{out}");
    r.write("src/dose.py", "x = 2\n");
    let (code, out) = check(&r);
    assert_eq!(lines(&out, "SD910").len(), 1, "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_approval_in_the_pull_request_itself_does_not_count() {
    // The branch accepts the intent, approves the plan and edits the code. The
    // base tree has the intent as `draft`, so the plan and the code give SD910.
    let r = start(|r| {
        r.write(INTENT, &intent("draft", "routine"));
        r.write(PLAN, &plan("draft", &["src/dose.py"]));
    });
    r.write(INTENT, &intent("accepted", "routine"));
    r.commit("accept");
    r.write(PLAN, &plan("approved", &["src/dose.py"]));
    r.commit("approve");
    r.write("src/dose.py", "x = 2\n");
    r.commit("code");
    let (code, out) = check(&r);
    let found = lines(&out, "SD910");
    assert_eq!(found.len(), 2, "{out}");
    assert!(
        found.iter().any(|l| l.starts_with(&format!("{PLAN}:1:"))),
        "{out}"
    );
    assert!(
        found.iter().any(|l| l.starts_with("src/dose.py:1:")),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_gate_sets_the_severity_of_sd910() {
    let base = |r: &Repo| r.write(INTENT, &intent("draft", "routine"));
    for (gate, code, word) in [("block", 1, "error"), ("warn", 0, "warning")] {
        let r = start_with("standard", &format!("gate = \"{gate}\"\n"), base);
        r.write(PLAN, &plan("draft", &["src/dose.py"]));
        let (exit, out) = check(&r);
        let found = lines(&out, "SD910");
        assert_eq!(found.len(), 1, "{gate}: {out}");
        assert!(
            found[0].contains(&format!(": {word} SD910 ")),
            "{gate}: {out}"
        );
        assert_eq!(exit, code, "{gate}: {out}");
    }
    let r = start_with("standard", "gate = \"off\"\n", base);
    r.write(PLAN, &plan("draft", &["src/dose.py"]));
    let (exit, out) = check(&r);
    assert!(!has(&out, "SD910"), "{out}");
    assert_eq!(exit, 0, "{out}");
}

#[test]
fn a_stacked_pull_request_trusts_its_base() {
    // The branch `a` accepts the intent. The branch `b` starts from `a` and
    // adds the spec. Against `main`, the base tree has no accepted intent.
    // Against `a`, it has.
    let r = start(|r| r.write(INTENT, &intent("draft", "higher-risk")));
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.commit("accept");
    r.git(&["branch", "a"]);
    r.git(&["checkout", "-q", "-b", "b"]);
    r.write(SPEC, &spec("draft", &[]));
    r.commit("spec");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(lines(&out, "SD910").len(), 1, "{out}");
    assert_eq!(code, 1, "{out}");
    let (code, out, _) = r.run(&["check", "--base", "a"]);
    assert!(!has(&out, "SD910"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_diff_with_no_change_is_a_small_fix() {
    let r = start(nothing);
    r.write("src/other.py", "y = 2\n");
    r.write("README.md", "Read me.\n");
    let (code, out) = check(&r);
    for c in ["SD910", "SD913", "SD917"] {
        assert!(!has(&out, c), "{c}: {out}");
    }
    assert_eq!(code, 0, "{out}");
}

// B5: a change that the branch added and finished

fn born(r: &Repo, files: &[&str]) {
    r.write(INTENT, &intent("accepted", "routine"));
    r.write(PLAN, &plan("approved", files));
    r.commit("add the change");
    r.write("src/dose.py", "x = 2\n");
    r.write(
        "docs/requirements/valves.md",
        &VALVES.replace("5 seconds", "4 seconds"),
    );
    r.commit("code");
    std::fs::remove_dir_all(r.dir.join("docs/intent/dose")).unwrap();
    r.commit("finish");
}

#[test]
fn a_branch_born_change_gives_sd910_with_block() {
    let r = start(nothing);
    born(&r, &["src/dose.py", "docs/requirements/valves.md"]);
    let (code, out) = check(&r);
    let found = lines(&out, "SD910");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("was added and finished in this branch"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // The change is a change of the diff: no SD921. Its `Files` cover the
    // diff, so the artifacts were read, and there is no SD913.
    assert!(!has(&out, "SD921") && !has(&out, "SD913"), "{out}");
}

#[test]
fn a_branch_born_change_passes_with_warn() {
    let r = start_with("standard", "gate = \"warn\"\n", nothing);
    born(&r, &["src/dose.py", "docs/requirements/valves.md"]);
    let (code, out) = check(&r);
    let found = lines(&out, "SD910");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains(": warning SD910 "), "{out}");
    assert!(!has(&out, "SD921"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_branch_born_change_with_no_gate_gives_no_sd921() {
    let r = start_with("standard", "gate = \"off\"\n", nothing);
    born(&r, &["src/dose.py", "docs/requirements/valves.md"]);
    let (code, out) = check(&r);
    assert!(!has(&out, "SD910") && !has(&out, "SD921"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_files_of_a_branch_born_change_are_its_scope() {
    // The plan lists the code, and not the requirement document.
    let r = start_with("standard", "gate = \"warn\"\n", nothing);
    born(&r, &["src/dose.py"]);
    let (_, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("docs/requirements/valves.md:1:"),
        "{out}"
    );
}

// SD913

fn approved_change(r: &Repo, files: &[&str]) {
    r.write(INTENT, &intent("accepted", "routine"));
    r.write(PLAN, &plan("approved", files));
}

#[test]
fn a_path_that_no_files_item_matches_gives_sd913() {
    let r = start(|r| approved_change(r, &["src/dose.py", "docs/requirements/"]));
    r.write("src/dose.py", "x = 2\n");
    r.write("src/other.py", "y = 2\n");
    r.write(
        "docs/requirements/pumps.md",
        &PUMPS.replace("90 percent", "85 percent"),
    );
    r.write(
        INTENT,
        &intent("accepted", "routine").replace("too high", "too big"),
    );
    let (code, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("src/other.py:1: warning SD913 outside-plan: "),
        "{out}"
    );
    // A warning does not fail the check.
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_scope_has_the_folder_and_the_documents_of_the_delta() {
    let r = start(|r| {
        r.write(INTENT, &intent("accepted", "higher-risk"));
        r.write(
            SPEC,
            &spec(
                "approved",
                &["- ADD requirements/valves#open: The controller SHALL open the valve."],
            ),
        );
        r.write(PLAN, &plan("approved", &["src/dose.py"]));
    });
    r.write(
        SPEC,
        &spec(
            "approved",
            &["- ADD requirements/valves#open: The controller SHALL open the valve in 5 seconds."],
        ),
    );
    r.write(
        "docs/requirements/valves.md",
        &format!("{VALVES}- **open**: The controller SHALL open the valve in 5 seconds.\n"),
    );
    r.write("src/dose.py", "x = 2\n");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD913"), "{out}");
    // A document that the delta does not name is outside the scope.
    r.write(
        "docs/requirements/tanks.md",
        &TANKS.replace("80 percent", "70 percent"),
    );
    let (_, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("docs/requirements/tanks.md:1:"),
        "{out}"
    );
}

#[test]
fn a_routine_change_needs_its_requirement_document_in_files() {
    let r = start(|r| approved_change(r, &["src/dose.py"]));
    r.write("src/dose.py", "x = 2\n");
    r.write(
        "docs/requirements/valves.md",
        &VALVES.replace("5 seconds", "4 seconds"),
    );
    let (_, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("docs/requirements/valves.md:1:"),
        "{out}"
    );
}

#[test]
fn a_change_with_no_plan_has_the_folder_as_its_scope() {
    let r = start(nothing);
    r.write(INTENT, &intent("draft", "routine"));
    r.write("src/dose.py", "x = 2\n");
    let (_, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("src/dose.py:1:"), "{out}");
}

#[test]
fn the_changes_that_files_match_are_changes_of_the_diff() {
    // The folder is not in the diff. The code matches `Files`, so the change
    // is a change of the diff, and the file outside `Files` is outside its scope.
    let r = start(|r| approved_change(r, &["src/dose.py"]));
    r.write("src/dose.py", "x = 2\n");
    r.write("README.md", "Read me.\n");
    let (code, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("README.md:1:"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn each_path_outside_all_scopes_is_reported_once_for_two_changes() {
    let r = start(|r| {
        approved_change(r, &["src/dose.py"]);
        r.write("docs/intent/flow/intent.md", &intent("accepted", "routine"));
        r.write(
            "docs/intent/flow/plan.md",
            &plan("approved", &["src/other.py"]),
        );
    });
    r.write("src/dose.py", "x = 2\n");
    r.write("src/other.py", "y = 2\n");
    r.write("README.md", "Read me.\n");
    let (_, out) = check(&r);
    let found = lines(&out, "SD913");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("README.md:1:"), "{out}");
}

#[test]
fn sd913_is_off_in_lite_and_for_a_gate_that_is_off_it_stays_on() {
    let setup = |r: &Repo| approved_change(r, &["src/dose.py"]);
    let r = start_with("lite", "gate = \"block\"\n", setup);
    r.write("src/dose.py", "x = 2\n");
    r.write("README.md", "Read me.\n");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD913"), "{out}");
    // The gate does not set SD913.
    let r = start_with("standard", "gate = \"off\"\n", setup);
    r.write("src/dose.py", "x = 2\n");
    r.write("README.md", "Read me.\n");
    let (_, out) = check(&r);
    assert!(has(&out, "SD913"), "{out}");
}

// SD917

#[test]
fn a_delta_line_for_a_high_risk_layer_needs_a_higher_risk_class() {
    for (class, fires) in [("routine", true), ("higher-risk", false)] {
        let r = start(|r| {
            r.write(INTENT, &intent("accepted", class));
            r.write(
                SPEC,
                &spec(
                    "draft",
                    &["- ADD interfaces/http#timeout: The interface SHALL time out after 30 seconds."],
                ),
            );
        });
        r.write(
            INTENT,
            &intent("accepted", class).replace("too high", "too big"),
        );
        let (_, out) = check(&r);
        let found = lines(&out, "SD917");
        assert_eq!(found.len(), usize::from(fires), "{class}: {out}");
        if fires {
            assert!(
                found[0].starts_with(&format!("{INTENT}:1: warning SD917 class-too-low: ")),
                "{out}"
            );
            assert!(found[0].contains("the layer `interfaces`"), "{out}");
        }
    }
}

#[test]
fn a_routine_change_that_changes_a_high_risk_requirement_gives_sd917() {
    let r = start(|r| approved_change(r, &["docs/interfaces/http.md"]));
    r.write("docs/interfaces/http.md", &HTTP.replace("200", "204"));
    let (_, out) = check(&r);
    let found = lines(&out, "SD917");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`interfaces/http#status`"), "{out}");
    // A requirement of a layer that is not high risk gives nothing.
    let r = start(|r| approved_change(r, &["docs/requirements/valves.md"]));
    r.write(
        "docs/requirements/valves.md",
        &VALVES.replace("5 seconds", "4 seconds"),
    );
    let (_, out) = check(&r);
    assert!(!has(&out, "SD917"), "{out}");
}

#[test]
fn a_path_of_high_risk_paths_gives_sd917() {
    let change = "gate = \"block\"\nhigh-risk-paths = [\"src/payments/**\"]\n";
    // The diff has the path.
    let r = start_with("standard", change, |r| {
        approved_change(r, &["src/payments/"])
    });
    r.write("src/payments/pay.py", "z = 2\n");
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD917").len(), 1, "{out}");
    // The plan has the path, and the diff has not.
    let r = start_with("standard", change, |r| {
        approved_change(r, &["src/payments/pay.py", "src/dose.py"])
    });
    r.write("src/dose.py", "x = 2\n");
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD917").len(), 1, "{out}");
    // A higher class has no SD917.
    let r = start_with("standard", change, |r| {
        r.write(INTENT, &intent("accepted", "higher-risk"));
        r.write(PLAN, &plan("approved", &["src/payments/"]));
    });
    r.write("src/payments/pay.py", "z = 2\n");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD917"), "{out}");
}

#[test]
fn a_small_fix_gives_sd917_only_for_a_high_risk_path_or_requirement() {
    let change = "gate = \"block\"\nhigh-risk-paths = [\"src/payments/**\"]\n";
    let r = start_with("standard", change, nothing);
    // A path of `high-risk-paths`.
    r.write("src/payments/pay.py", "z = 2\n");
    let (_, out) = check(&r);
    let found = lines(&out, "SD917");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("src/payments/pay.py:1:"), "{out}");
    assert!(!out.contains("small-fix"), "{out}");
    git_clean(&r);
    // A requirement of a high-risk layer. A small fix to a rule also gives
    // SD921, because it has no intent.
    r.write("docs/rules/safety.md", &SAFETY.replace("95", "90"));
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD917").len(), 1, "{out}");
    assert_eq!(lines(&out, "SD921").len(), 1, "{out}");
    git_clean(&r);
    // A requirement of another layer, and other code.
    r.write(
        "docs/requirements/valves.md",
        &VALVES.replace("5 seconds", "4 seconds"),
    );
    r.write("src/other.py", "y = 2\n");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD917"), "{out}");
    assert_eq!(lines(&out, "SD921").len(), 1, "{out}");
}

/// Resets the working tree to `HEAD`.
fn git_clean(r: &Repo) {
    r.git(&["checkout", "-q", "--", "."]);
    r.git(&["clean", "-q", "-fd"]);
}

#[test]
fn sd917_is_an_error_in_strict_and_off_in_lite() {
    for (profile, want) in [("lite", None), ("strict", Some("error"))] {
        let r = start_with(profile, "gate = \"block\"\n", |r| {
            approved_change(r, &["docs/interfaces/http.md"])
        });
        r.write("docs/interfaces/http.md", &HTTP.replace("200", "204"));
        let (_, out) = check(&r);
        let found = lines(&out, "SD917");
        match want {
            Some(w) => assert!(found[0].contains(&format!(": {w} SD917 ")), "{out}"),
            None => assert!(found.is_empty(), "{out}"),
        }
    }
}

// SD905

#[test]
fn a_routine_change_that_touches_many_documents_gives_sd905() {
    let change = "gate = \"block\"\nmax-fanout = 2\n";
    let edit = |r: &Repo, n: usize| {
        let docs = [
            ("valves", VALVES, "5 seconds", "4 seconds"),
            ("pumps", PUMPS, "90 percent", "85 percent"),
            ("tanks", TANKS, "80 percent", "70 percent"),
        ];
        for (name, text, from, to) in docs.iter().take(n) {
            r.write(
                &format!("docs/requirements/{name}.md"),
                &text.replace(from, to),
            );
        }
    };
    let r = start_with("standard", change, |r| {
        approved_change(r, &["docs/requirements/"])
    });
    edit(&r, 3);
    let (code, out) = check(&r);
    let found = lines(&out, "SD905");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with(&format!("{INTENT}:1: warning SD905 fan-out: ")),
        "{out}"
    );
    assert!(found[0].contains("3 requirement documents"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The limit is not above.
    let r = start_with("standard", change, |r| {
        approved_change(r, &["docs/requirements/"])
    });
    edit(&r, 2);
    let (_, out) = check(&r);
    assert!(!has(&out, "SD905"), "{out}");
}

// SD912

#[test]
fn sd912_is_new_when_the_head_has_more_started_changes_than_the_base() {
    // `max-started` is 1. The base has 1 started change. The branch commits
    // the second one. The base of the gate (`HEAD`) has the second one, so only
    // the lifecycle base shows that the diagnostic is new.
    let cfg = "gate = \"block\"\n";
    let r = Repo::with_config(&config("standard", cfg).replace(
        "keep = \"until-finished\"\n",
        "keep = \"until-finished\"\nmax-started = 1\n",
    ));
    r.write(INTENT, &intent("accepted", "routine"));
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/intent/flow/intent.md", &intent("accepted", "routine"));
    r.commit("second");
    let (code, out) = check(&r);
    assert_eq!(lines(&out, "SD912").len(), 1, "{out}");
    assert_eq!(code, 0, "{out}");
    // A branch that adds no started change does not make the diagnostic new,
    // although the tree is over the limit.
    git_clean(&r);
    r.git(&["checkout", "-q", "main"]);
    r.write("docs/intent/flow/intent.md", &intent("accepted", "routine"));
    r.commit("main has two");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "other"]);
    r.write("src/other.py", "y = 2\n");
    r.commit("code");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD912"), "{out}");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert_eq!(lines(&out, "SD912").len(), 1, "{out}");
}

// SD921

const EDIT: &str = "5 seconds";

fn edit_valves(r: &Repo) {
    r.write(
        "docs/requirements/valves.md",
        &VALVES.replace(EDIT, "4 seconds"),
    );
}

#[test]
fn sd921_runs_on_the_merge_base_with_origin_head() {
    let r = start(nothing);
    edit_valves(&r);
    r.commit("edit");
    let (code, out) = check(&r);
    let found = lines(&out, "SD921");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0]
            .contains("warning SD921 truth-changed-without-change: requirements changed `close`"),
        "{out}"
    );
    assert_eq!(code, 0, "{out}");
    // `--base` keeps its behavior.
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(lines(&out, "SD921").len(), 1, "{out}");
}

#[test]
fn sd921_uses_the_merge_base_and_not_head() {
    // The branch adds its intent in an earlier commit, and edits the
    // requirement later. The base `HEAD` would give SD921, and the merge base
    // sees the intent folder in the diff.
    let r = start(nothing);
    r.write(INTENT, &intent("draft", "routine"));
    r.commit("intent");
    edit_valves(&r);
    r.commit("edit");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD921"), "{out}");
}

#[test]
fn the_files_of_an_approved_plan_in_the_base_clear_sd921() {
    // The plan is approved in the base and lists the document. The pull request
    // edits the document and does not touch the folder.
    let r = start(|r| approved_change(r, &["docs/requirements/valves.md", "src/dose.py"]));
    edit_valves(&r);
    r.write(
        "docs/requirements/tanks.md",
        &TANKS.replace("80 percent", "70 percent"),
    );
    let (_, out) = check(&r);
    let found = lines(&out, "SD921");
    // The document that the plan does not list still gives SD921.
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].starts_with("docs/requirements/tanks.md:"), "{out}");
}

#[test]
fn a_plan_that_is_not_approved_in_the_base_gives_no_exemption() {
    let r = start(|r| {
        r.write(INTENT, &intent("accepted", "routine"));
        r.write(PLAN, &plan("draft", &["docs/requirements/valves.md"]));
    });
    edit_valves(&r);
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD921").len(), 1, "{out}");
    // The intent must be `accepted` too.
    let r = start(|r| {
        r.write(INTENT, &intent("draft", "routine"));
        r.write(PLAN, &plan("approved", &["docs/requirements/valves.md"]));
    });
    edit_valves(&r);
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD921").len(), 1, "{out}");
}

// SD914 reads the lifecycle base

#[test]
fn a_remove_line_that_the_branch_applied_in_an_earlier_commit_gives_no_sd914() {
    // The base has the key. The first commit adds the spec with the `REMOVE`
    // line, and the second commit removes the key. Against `HEAD` the key is
    // absent in both trees, so the line would give SD914. Against the merge
    // base with `origin/HEAD`, the line is applied.
    let two = format!("{VALVES}- **open**: The controller SHALL open the valve in 5 seconds.\n");
    let r = start(|r| r.write("docs/requirements/valves.md", &two));
    let (code, out, err) = r.run(&["show", "requirements/valves#close", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out
        .split("\"hash\":\"")
        .nth(1)
        .and_then(|t| t.split('"').next())
        .expect("a hash");
    r.write(INTENT, &intent("draft", "higher-risk"));
    r.write(
        SPEC,
        &spec(
            "draft",
            &[&format!("- REMOVE requirements/valves#close@{hash}")],
        ),
    );
    r.commit("spec");
    r.write(
        "docs/requirements/valves.md",
        &two.replace(
            "- **close**: The controller SHALL close the valve in 5 seconds.\n",
            "",
        ),
    );
    r.commit("apply");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD914"), "{out}");
    // With the earlier commit as the base, the key is absent in both trees.
    let (_, out, _) = r.run(&["check", "--all", "--base", "HEAD"]);
    assert_eq!(lines(&out, "SD914").len(), 1, "{out}");
}

// The base itself

#[test]
fn with_no_base_the_note_is_printed_and_only_the_checks_of_one_tree_run() {
    let r = Repo::with_config(&config("standard", "gate = \"block\"\n"));
    r.write("docs/requirements/valves.md", VALVES);
    r.write(INTENT, &intent("draft", "routine"));
    r.commit("base");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(SPEC, &spec("draft", &[]));
    r.write(PLAN, &plan("draft", &["src/dose.py"]));
    edit_valves(&r);
    r.write("src/other.py", "y = 2\n");
    let (code, out, err) = r.run(&["check"]);
    for c in ["SD905", "SD910", "SD913", "SD917", "SD921"] {
        assert!(!has(&out, c), "{c}: {out}");
    }
    assert_eq!(code, 0, "{out}");
    assert!(
        err.contains("note: no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>"),
        "{err}"
    );
    let (_, out, _) = r.run(&["check", "--json"]);
    assert!(out.contains("\"change_base\":null"), "{out}");
    // With the base, the note goes away, and the codes run.
    origin_head(&r.dir, "main");
    let (code, out, err) = r.run(&["check"]);
    assert!(!err.contains("no base for the change checks"), "{err}");
    assert!(has(&out, "SD910"), "{out}");
    assert_eq!(code, 1, "{out}");
    let (_, out, _) = r.run(&["check", "--json"]);
    let sha = r.git_out(&["rev-parse", "main"]);
    assert!(out.contains(&format!("\"change_base\":\"{sha}\"")), "{out}");
}

#[test]
fn staged_reads_the_index() {
    let r = start(|r| r.write(INTENT, &intent("draft", "higher-risk")));
    r.write(SPEC, &spec("draft", &[]));
    // The file is not staged: the index has no spec.
    let (code, out, _) = r.run(&["check", "--staged"]);
    assert!(!has(&out, "SD910"), "{out}");
    assert_eq!(code, 0, "{out}");
    r.git(&["add", SPEC]);
    let (code, out, _) = r.run(&["check", "--staged"]);
    assert_eq!(lines(&out, "SD910").len(), 1, "{out}");
    assert_eq!(code, 1, "{out}");
    // Code that is edited and not staged is not in the diff of the index.
    git_clean(&r);
    r.git(&["reset", "-q"]);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.git(&["add", INTENT]);
    r.write(SPEC, &spec("draft", &[]));
    let (_, out, _) = r.run(&["check", "--staged"]);
    assert!(!has(&out, "SD910"), "{out}");
}

#[test]
fn the_policy_none_runs_none_of_the_codes() {
    let r = Repo::with_config(
        &config("strict", "gate = \"block\"\n")
            .replace("policy = \"three-class\"", "policy = \"none\""),
    );
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/interfaces/http.md", HTTP);
    r.write(INTENT, &intent("draft", "routine"));
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(SPEC, &spec("draft", &[]));
    r.write(PLAN, &plan("draft", &["src/dose.py"]));
    edit_valves(&r);
    r.write("docs/interfaces/http.md", &HTTP.replace("200", "204"));
    r.write("src/other.py", "y = 2\n");
    r.commit("all");
    let (code, out, err) = r.run(&["check"]);
    for c in [
        "SD905", "SD910", "SD912", "SD913", "SD914", "SD917", "SD921",
    ] {
        assert!(!has(&out, c), "{c}: {out}");
    }
    assert_eq!(code, 0, "{out}");
    assert!(!err.contains("no base for the change checks"), "{err}");
    let (_, out, _) = r.run(&["check", "--json"]);
    assert!(!out.contains("change_base"), "{out}");
}
