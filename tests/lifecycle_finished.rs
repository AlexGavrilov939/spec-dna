//! Integration tests of the checks of a finished change and of staleness (step
//! 3, slice S5): SD915, SD916, SD920 (a decision that is not `accepted`), SD914
//! by touch and SD911.
//!
//! Each fixture has a `main` branch with the base commit, a
//! `refs/remotes/origin/HEAD` that names it, and a `feature` branch. The base
//! commit holds the folder of the change `dose`. A test acts as the pull
//! request of the feature branch: it edits the truth, and it deletes the folder.

mod common;

use common::*;
use std::process::Command;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const DECISION: &str =
    "---\nstatus: {status}\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer.\n";
const COVERS: &str = "# covers: requirements/valves#close\n";

const FOLDER: &str = "docs/intent/dose";
const SPEC: &str = "docs/intent/dose/spec.md";
const INTENT: &str = "docs/intent/dose/intent.md";
const CLOSE_4: &str = "The controller SHALL close the valve in 4 seconds.";

fn config(profile: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\nstale-days = 14\n"
    )
}

fn intent(status: &str, class: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: {class}\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n"
    )
}

fn plan(files: &[&str]) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("- `{f}`")).collect();
    format!(
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n{}\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        items.join("\n")
    )
}

/// What the base commit holds for the change `dose`.
struct Fx<'a> {
    profile: &'a str,
    rules: &'a str,
    /// The status of the intent.
    status: &'a str,
    /// The delta lines of the spec. `{close}` and `{open}` stand for the hash
    /// of the requirement. No line and no decision: no spec.
    delta: &'a [&'a str],
    /// The `[[id]]` items of `Decisions` of the spec.
    decisions: &'a [&'a str],
    /// The status of the decision `decisions/timer`, or `None` for no document.
    decision_doc: Option<&'a str>,
    files: &'a [&'a str],
}

impl Default for Fx<'_> {
    fn default() -> Self {
        Fx {
            profile: "standard",
            rules: "",
            status: "accepted",
            delta: &[],
            decisions: &[],
            decision_doc: None,
            files: &["src/dose.py"],
        }
    }
}

fn hash(r: &Repo, id: &str) -> String {
    let (code, out, err) = r.run(&["show", id, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let rest = out.split("\"hash\":\"").nth(1).expect("a hash");
    rest.split('"').next().unwrap().to_string()
}

impl Fx<'_> {
    /// The base commit on `main`, `origin/HEAD` on it, and the branch
    /// `feature`.
    fn start(&self) -> Repo {
        let r = Repo::with_config(&config(self.profile, self.rules));
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/dose.py", "x = 1\n");
        r.write("tests/test_other.py", "# nothing\n");
        if let Some(s) = self.decision_doc {
            r.write("docs/decisions/timer.md", &DECISION.replace("{status}", s));
        }
        let close = hash(&r, "requirements/valves#close");
        let open = hash(&r, "requirements/valves#open");
        let class = if self.delta.is_empty() && self.decisions.is_empty() {
            "routine"
        } else {
            "higher-risk"
        };
        r.write(INTENT, &intent(self.status, class));
        r.write(
            "docs/intent/dose/plan.md",
            &plan(
                &self
                    .files
                    .iter()
                    .copied()
                    .chain(["tests/test_valves.py"])
                    .collect::<Vec<_>>(),
            ),
        );
        if !self.delta.is_empty() || !self.decisions.is_empty() {
            let mut body = String::new();
            if !self.delta.is_empty() {
                body.push_str("## Delta\n\n");
                for l in self.delta {
                    body.push_str(&format!(
                        "{}\n",
                        l.replace("{close}", &close).replace("{open}", &open)
                    ));
                }
            }
            if !self.decisions.is_empty() {
                body.push_str("\n## Decisions\n\n");
                for d in self.decisions {
                    body.push_str(&format!("- [[{d}]]\n"));
                }
            }
            r.write(
                SPEC,
                &format!("---\nstatus: approved\n---\n# Spec\n\n{body}"),
            );
        }
        r.commit("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r
    }
}

fn valves_with(close: &str) -> String {
    VALVES.replace("The controller SHALL close the valve in 5 seconds.", close)
}

/// The pull request of `finish`: it deletes the folder of the change.
fn finish(r: &Repo) {
    std::fs::remove_dir_all(r.dir.join(FOLDER)).unwrap();
    r.commit("finish");
}

fn check(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check"]);
    (code, out)
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

/// The 3 codes of a finished change.
fn finished_codes(out: &str) -> Vec<&str> {
    ["SD915", "SD916", "SD920"]
        .into_iter()
        .filter(|c| has(out, c))
        .collect()
}

const CHANGE_CLOSE: &str = "- CHANGE requirements/valves#close@{close}: The controller SHALL close the valve in 4 seconds.";

// SD915: a delta that the head does not apply

#[test]
fn a_finished_delta_change_gives_none_of_the_codes() {
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    r.write("tests/test_valves.py", COVERS);
    finish(&r);
    let (code, out) = check(&r);
    assert!(finished_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_delta_that_the_head_does_not_apply_gives_sd915() {
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    // The head has another text than the line.
    r.write(
        "docs/requirements/valves.md",
        &valves_with("The controller SHALL close the valve in 3 seconds."),
    );
    finish(&r);
    let (code, out) = check(&r);
    let found = lines(&out, "SD915");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with(&format!("{SPEC}:")) && found[0].contains(": error SD915 "),
        "{out}"
    );
    assert!(
        found[0].contains("`requirements/valves#close`") && found[0].contains("`accepted`"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // The target of the diagnostic is the target of the line.
    let (_, json, _) = r.run(&["check", "--json"]);
    assert!(
        json.contains("\"code\":\"SD915\"")
            && json.contains("\"target\":\"requirements/valves#close\""),
        "{json}"
    );
}

#[test]
fn a_correction_in_the_finish_pull_request_has_no_effect() {
    // The pull request edits the line of the spec to the text that it writes,
    // and then it deletes the folder. The check reads the line of the base.
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    let three = "The controller SHALL close the valve in 3 seconds.";
    r.write("docs/requirements/valves.md", &valves_with(three));
    let text = r.read(SPEC).replace(CLOSE_4, three);
    r.write(SPEC, &text);
    r.commit("correct the line");
    finish(&r);
    let (code, out) = check(&r);
    assert_eq!(lines(&out, "SD915").len(), 1, "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_remove_line_needs_the_key_to_be_absent() {
    let remove = "- REMOVE requirements/valves#open@{open}";
    let r = Fx {
        delta: &[remove],
        ..Fx::default()
    }
    .start();
    finish(&r);
    let (code, out) = check(&r);
    let found = lines(&out, "SD915");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`requirements/valves#open`"), "{out}");
    assert_eq!(code, 1, "{out}");
    // The head does not have the key: the line is applied.
    let r = Fx {
        delta: &[remove],
        ..Fx::default()
    }
    .start();
    let gone = VALVES.replace(
        "- **open**: The controller SHALL open the valve in 5 seconds.\n",
        "",
    );
    r.write("docs/requirements/valves.md", &gone);
    finish(&r);
    let (code, out) = check(&r);
    assert!(finished_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn an_add_line_needs_the_text_in_the_head() {
    let add = "- ADD requirements/valves#flush: The controller SHALL flush the line.";
    let r = Fx {
        delta: &[add],
        ..Fx::default()
    }
    .start();
    finish(&r);
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD915").len(), 1, "{out}");
    let r = Fx {
        delta: &[add],
        ..Fx::default()
    }
    .start();
    r.write(
        "docs/requirements/valves.md",
        &format!("{VALVES}- **flush**: The controller SHALL flush the line.\n"),
    );
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush\n",
    );
    finish(&r);
    let (code, out) = check(&r);
    assert!(finished_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

// SD915: a change with no delta

#[test]
fn a_routine_change_needs_a_changed_files_path_or_a_changed_hash() {
    // The test deletes the folder, and nothing else changes.
    let r = Fx::default().start();
    finish(&r);
    let (code, out) = check(&r);
    let found = lines(&out, "SD915");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with(&format!("{INTENT}:1: error SD915 "))
            && found[0].contains("no `Files` path and no requirement hash"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // A path of `Files` changes.
    let r = Fx::default().start();
    r.write("src/dose.py", "x = 2\n");
    finish(&r);
    let (code, out) = check(&r);
    assert!(finished_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
    // A path outside `Files` does not count.
    let r = Fx::default().start();
    r.write("src/other.py", "y = 2\n");
    finish(&r);
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD915").len(), 1, "{out}");
    // A requirement hash changes, and `Files` does not list the document.
    let r = Fx::default().start();
    // The test file is in `Files`, so the fixture does not write it here: the
    // hash is the only change.
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    finish(&r);
    let (_, out) = check(&r);
    assert!(!has(&out, "SD915"), "{out}");
}

// A close and a finish in one pull request

#[test]
fn a_close_and_a_finish_in_one_pull_request_give_sd915() {
    // The base status is `accepted`. The pull request sets `closed` and then
    // deletes the folder, so the base status does not count as `closed`.
    for (name, fx) in [
        (
            "delta",
            Fx {
                delta: &[CHANGE_CLOSE],
                ..Fx::default()
            },
        ),
        ("routine", Fx::default()),
    ] {
        let r = fx.start();
        let class = if name == "delta" {
            "higher-risk"
        } else {
            "routine"
        };
        r.write(INTENT, &intent("closed", class));
        r.commit("close");
        finish(&r);
        let (code, out) = check(&r);
        let found = lines(&out, "SD915");
        assert_eq!(found.len(), 1, "{name}: {out}");
        assert!(found[0].contains("its base status is `accepted`"), "{out}");
        assert_eq!(code, 1, "{name}: {out}");
    }
}

#[test]
fn the_deletion_of_a_closed_change_gives_none_of_the_codes() {
    // The base status is `closed`, so the change is dropped, not finished.
    for fx in [
        Fx {
            status: "closed",
            delta: &[CHANGE_CLOSE],
            decisions: &["decisions/timer", "decisions/none"],
            decision_doc: Some("proposed"),
            ..Fx::default()
        },
        Fx {
            status: "closed",
            ..Fx::default()
        },
    ] {
        let r = fx.start();
        finish(&r);
        let (code, out) = check(&r);
        assert!(finished_codes(&out).is_empty(), "{out}");
        assert_eq!(code, 0, "{out}");
    }
}

#[test]
fn the_codes_follow_the_profile() {
    for (profile, word, exit) in [
        ("lite", "warning", 0),
        ("standard", "error", 1),
        ("strict", "error", 1),
    ] {
        let r = Fx {
            profile,
            delta: &[CHANGE_CLOSE],
            decisions: &["decisions/timer"],
            decision_doc: Some("proposed"),
            ..Fx::default()
        }
        .start();
        r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
        finish(&r);
        let (code, out) = check(&r);
        for c in ["SD915", "SD916", "SD920"] {
            let found = lines(&out, c);
            let applied = c == "SD915";
            // The line is applied, so SD915 stays silent.
            assert_eq!(found.len(), usize::from(!applied), "{profile} {c}: {out}");
            if !applied {
                assert!(
                    found[0].contains(&format!(": {word} {c} ")),
                    "{profile}: {out}"
                );
            }
        }
        assert_eq!(code, exit, "{profile}: {out}");
    }
}

// SD916

#[test]
fn a_requirement_that_the_delta_changes_needs_evidence() {
    let fx = || Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    };
    // No tag and no marker.
    let r = fx().start();
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    finish(&r);
    let (code, out) = check(&r);
    let found = lines(&out, "SD916");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("docs/requirements/valves.md:") && found[0].contains(": error SD916 "),
        "{out}"
    );
    assert!(!has(&out, "SD915"), "{out}");
    assert_eq!(code, 1, "{out}");
    // A `covers:` tag.
    let r = fx().start();
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    r.write("tests/test_valves.py", COVERS);
    finish(&r);
    let (_, out) = check(&r);
    assert!(!has(&out, "SD916"), "{out}");
    // A `verified-by` marker.
    let r = fx().start();
    r.write(
        "docs/requirements/valves.md",
        &valves_with(&format!(
            "{CLOSE_4} <!-- verified-by: tests/test_other.py -->"
        )),
    );
    finish(&r);
    let (code, out) = check(&r);
    assert!(finished_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_requirement_that_a_routine_change_edits_needs_evidence() {
    let r = Fx::default().start();
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    finish(&r);
    let (_, out) = check(&r);
    let found = lines(&out, "SD916");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("`requirements/valves#close`"), "{out}");
    // The requirement `open` did not change, so it needs nothing.
    assert!(!found[0].contains("valves#open"), "{out}");
    r.write("tests/test_valves.py", COVERS);
    r.commit("tag");
    let (_, out) = check(&r);
    assert!(!has(&out, "SD916"), "{out}");
}

#[test]
fn sd916_reads_the_tags_when_sd105_and_sd608_are_off() {
    let r = Fx {
        rules: "SD105 = \"off\"\n",
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    r.write("docs/requirements/valves.md", &valves_with(CLOSE_4));
    r.write("tests/test_valves.py", COVERS);
    finish(&r);
    let (_, out) = check(&r);
    assert!(!has(&out, "SD916"), "{out}");
}

// SD920

#[test]
fn a_decision_that_is_not_accepted_gives_sd920() {
    for (doc, why) in [
        (Some("proposed"), Some("is `proposed`")),
        (Some("accepted"), None),
        (None, Some("names no document")),
    ] {
        let r = Fx {
            decisions: &["decisions/timer"],
            decision_doc: doc,
            ..Fx::default()
        }
        .start();
        r.write("src/dose.py", "x = 2\n");
        finish(&r);
        let (code, out) = check(&r);
        let found = lines(&out, "SD920");
        match why {
            Some(w) => {
                assert_eq!(found.len(), 1, "{doc:?}: {out}");
                assert!(
                    found[0].starts_with(&format!("{SPEC}:"))
                        && found[0].contains(": error SD920 ")
                        && found[0].contains(w),
                    "{doc:?}: {out}"
                );
                assert_eq!(code, 1, "{doc:?}: {out}");
            }
            None => {
                assert!(found.is_empty(), "{doc:?}: {out}");
                assert_eq!(code, 0, "{doc:?}: {out}");
            }
        }
    }
}

// SD914 by touch

fn bad_line(doc: &str) -> String {
    format!(
        "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- CHANGE {doc}#close@0123456789abcdef: Text.\n"
    )
}

#[test]
fn sd914_is_an_error_for_a_spec_that_the_diff_touches_and_a_warning_for_another() {
    let build = |origin: bool| {
        let r = Repo::with_config(&config("standard", ""));
        r.write("docs/requirements/valves.md", VALVES);
        r.write(
            "docs/requirements/pumps.md",
            &VALVES.replace("Valves", "Pumps"),
        );
        for (slug, doc) in [
            ("dose", "requirements/valves"),
            ("flow", "requirements/pumps"),
        ] {
            r.write(
                &format!("docs/intent/{slug}/intent.md"),
                &intent("accepted", "routine"),
            );
            r.write(&format!("docs/intent/{slug}/spec.md"), &bad_line(doc));
        }
        r.commit("base");
        if origin {
            origin_head(&r.dir, "main");
        }
        r.git(&["checkout", "-q", "-b", "feature"]);
        r.write(
            SPEC,
            &format!("{}\nA note.\n", bad_line("requirements/valves")),
        );
        r.commit("touch dose");
        r
    };
    let r = build(true);
    let (code, out, _) = r.run(&["check", "--all"]);
    let found = lines(&out, "SD914");
    assert_eq!(found.len(), 2, "{out}");
    let word = |path: &str| {
        let l = found.iter().find(|l| l.contains(path)).expect("a line");
        l.contains(": error SD914 ")
    };
    assert!(word("docs/intent/dose/spec.md"), "{out}");
    assert!(!word("docs/intent/flow/spec.md"), "{out}");
    assert_eq!(code, 1, "{out}");
    // With no lifecycle base, no spec is touched.
    let r = build(false);
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(lines(&out, "SD914").len(), 2, "{out}");
    assert!(!out.contains(": error SD914 "), "{out}");
    assert_eq!(code, 0, "{out}");
}

// SD911

const D1: &str = "2001-01-01T00:00:00+00:00";
const D2: &str = "2001-03-01T00:00:00+00:00";

/// A repository with an `accepted` change. The base commit is at `D1`, and a
/// commit that touches nothing of the change is at `late`. `origin/HEAD` names
/// the last commit.
fn dated(late: &str) -> Repo {
    let r = Repo::with_config(&config("standard", ""));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("src/dose.py", "x = 1\n");
    r.write(INTENT, &intent("accepted", "routine"));
    r.write("docs/intent/dose/plan.md", &plan(&["src/dose.py"]));
    r.commit_at("base", D1);
    r.write("README.md", "Read me.\n");
    r.commit_at("later", late);
    origin_head(&r.dir, "main");
    r
}

#[test]
fn a_change_without_a_commit_for_stale_days_is_stale() {
    let r = dated(D2);
    // `check` shows a state code only with `--all`.
    let (code, out, _) = r.run(&["check"]);
    assert!(!has(&out, "SD911"), "{out}");
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["check", "--all"]);
    let found = lines(&out, "SD911");
    assert_eq!(found.len(), 1, "{out}");
    assert_eq!(code, 0, "a warning does not fail the check: {out}");
    assert!(
        found[0].starts_with(&format!("{INTENT}:1: warning SD911 "))
            && found[0].contains("59 days"),
        "{out}"
    );
    // The check reads the dates of the commits, so the age is the age at the
    // base commit. 9 days are fewer than 14.
    let r = dated("2001-01-10T00:00:00+00:00");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD911"), "{out}");
    // 15 days are more than 14.
    let r = dated("2001-01-16T00:00:00+00:00");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD911"), "{out}");
    let (_, json, _) = r.run(&["check", "--all", "--json"]);
    assert!(
        json.contains("\"code\":\"SD911\"") && json.contains("\"new\":false"),
        "{json}"
    );
}

#[test]
fn a_commit_to_a_files_path_keeps_the_change_alive() {
    let r = dated(D2);
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/dose.py", "x = 2\n");
    r.commit_at("code", "2001-02-25T00:00:00+00:00");
    // The merge base is the commit at `D2`, and the code is 4 days older.
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD911"), "{out}");
    // A path outside `Files` does not.
    let r = dated(D2);
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/other.py", "y = 2\n");
    r.commit_at("other", "2001-02-25T00:00:00+00:00");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(has(&out, "SD911"), "{out}");
}

#[test]
fn sd911_is_off_in_lite_and_follows_a_rule() {
    let r = dated(D2);
    let cfg = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &cfg.replace("profile = \"standard\"", "profile = \"lite\""),
    );
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD911"), "{out}");
    // A rule makes it an error. It is never new, so only `--all` fails.
    r.write(
        "spec-dna.toml",
        &cfg.replace("SD608 = \"off\"", "SD608 = \"off\"\nSD911 = \"error\""),
    );
    let (code, out, _) = r.run(&["check", "--all"]);
    assert!(lines(&out, "SD911")[0].contains(": error SD911 "), "{out}");
    assert_eq!(code, 1, "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert!(!has(&out, "SD911"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_depth_1_clone_gives_no_sd911_where_the_full_clone_gives_it() {
    let src = dated(D2);
    let url = format!(
        "file://{}",
        std::fs::canonicalize(&src.dir).unwrap().display()
    );
    // The full clone has both commits.
    let full = Repo::empty();
    git(&src.dir, &["clone", "-q", &url, full.dir.to_str().unwrap()]);
    let (_, out, _) = full.run(&["check", "--all"]);
    assert_eq!(lines(&out, "SD911").len(), 1, "{out}");
    // The depth-1 clone has the last commit only. Each file of the change
    // shows as added there, so the clone cannot tell how old the change is.
    let shallow = Repo::empty();
    shallow_clone(&src.dir, &shallow.dir, 1);
    assert_eq!(
        shallow.git_out(&["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    let (code, out, _) = shallow.run(&["check", "--all"]);
    assert!(!has(&out, "SD911"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The clone has a lifecycle base, so the silence is not a missing base.
    let (_, json, _) = shallow.run(&["check", "--all", "--json"]);
    assert!(!json.contains("\"change_base\":null"), "{json}");
    assert!(json.contains("\"change_base\":\""), "{json}");
}

/// `check --all` in a given time zone.
fn check_in_zone(r: &Repo, zone: &str) -> (i32, String, String) {
    let out = Command::new(BIN)
        .args(["check", "--all"])
        .current_dir(&r.dir)
        .env("TZ", zone)
        .output()
        .expect("binary runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn sd911_reads_the_dates_of_the_commits_and_not_the_clock_or_the_time_zone() {
    // The commits are from 2001. A check that reads the clock would find
    // 25 years, in any case. The result is the same in 2 time zones.
    let stale = dated(D2);
    let near = dated("2001-01-10T00:00:00+00:00");
    for zone in ["Pacific/Auckland", "America/Los_Angeles"] {
        let (_, out, _) = check_in_zone(&stale, zone);
        let found = lines(&out, "SD911");
        assert_eq!(found.len(), 1, "{zone}: {out}");
        assert!(found[0].contains("59 days"), "{zone}: {out}");
        let (_, out, _) = check_in_zone(&near, zone);
        assert!(!has(&out, "SD911"), "{zone}: {out}");
    }
    // A date that is near a day change in a time zone gives the same count.
    let edge = dated("2001-03-01T23:59:59-11:00");
    let a = check_in_zone(&edge, "Pacific/Auckland");
    let b = check_in_zone(&edge, "America/Los_Angeles");
    assert_eq!(a, b);
    assert!(lines(&a.1, "SD911")[0].contains("60 days"), "{}", a.1);
}

#[test]
fn sd911_is_silent_with_no_merge_base() {
    // `--base solo` names an unrelated history, so the command uses the tip
    // for the gate, and it has no lifecycle base.
    let r = dated(D2);
    r.git(&["checkout", "-q", "--orphan", "solo"]);
    r.git(&["rm", "-rfq", "."]);
    r.write("solo.txt", "solo\n");
    r.commit_at("solo", D1);
    r.git(&["checkout", "-q", "main"]);
    let (_, out, err) = r.run(&["check", "--all", "--base", "solo"]);
    assert!(!has(&out, "SD911"), "{out}");
    assert!(err.contains("no base for the change checks"), "{err}");
    // With no base at all, it is silent too.
    r.git(&["update-ref", "-d", "refs/remotes/origin/main"]);
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(!has(&out, "SD911"), "{out}");
    assert!(err.contains("no base for the change checks"), "{err}");
}

#[test]
fn a_closed_or_draft_change_is_never_stale() {
    for status in ["draft", "closed"] {
        let r = dated(D2);
        r.write(INTENT, &intent(status, "routine"));
        r.commit_at("status", D1);
        let (_, out, _) = r.run(&["check", "--all"]);
        assert!(!has(&out, "SD911"), "{status}: {out}");
    }
}
