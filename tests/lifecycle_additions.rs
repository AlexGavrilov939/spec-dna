//! Integration tests of the additions for the workflow pack (step 3, slice
//! S5b): SD922 (`question-open`), and the `Open questions` and `Order`
//! sections of the templates of a spec and a plan.
//!
//! The binary has no library, and no command prints the tasks of a plan
//! before slice S7. So the grammar of a task line, the order of `next`, the
//! duplicate ids, the byte ranges under CRLF and the cache round trip are unit
//! tests in `src/change.rs`, `src/parse.rs` and `src/index.rs`.

mod common;

use common::*;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n";

const DELTA: &str =
    "## Delta\n\n- ADD requirements/valves#flush: The controller SHALL flush the line.\n";

const PROFILES: [&str; 3] = ["lite", "standard", "strict"];

/// The configuration of a fixture. `change` is the `[change]` table, and
/// `rules` the keys of `[rules]`.
fn config(profile: &str, change: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n{change}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

const THREE: &str = "[change]\npolicy = \"three-class\"\n";

fn repo_with(cfg: &str) -> Repo {
    let r = Repo::with_config(cfg);
    r.write("docs/requirements/valves.md", VALVES);
    r
}

fn repo(profile: &str) -> Repo {
    repo_with(&config(profile, THREE, ""))
}

/// The change `dose`: its intent and a spec with `body` after the title.
fn change(r: &Repo, intent: &str, spec: &str, body: &str) {
    r.write(
        "docs/intent/dose/intent.md",
        &format!("---\nstatus: {intent}\n---\n# Change\n\n## Intent\n\nText.\n"),
    );
    r.write(
        "docs/intent/dose/spec.md",
        &format!("---\nstatus: {spec}\n---\n# Spec\n\n{body}"),
    );
}

fn check(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check", "--all"]);
    (code, out)
}

/// The severity words of the diagnostics of `code` in the text of `check`.
fn severities(out: &str, code: &str) -> Vec<&'static str> {
    let mut v = Vec::new();
    for line in out.lines() {
        if line.contains(&format!(": error {code} ")) {
            v.push("error");
        } else if line.contains(&format!(": warning {code} ")) {
            v.push("warning");
        }
    }
    v
}

fn spec_with_questions(questions: &str) -> String {
    format!("{DELTA}\n## Open questions\n\n{questions}")
}

/// Asserts the severity of SD922 in each profile (`None`: no diagnostic).
fn matrix(want: [Option<&str>; 3], setup: impl Fn(&Repo)) {
    for (profile, want) in PROFILES.iter().zip(want) {
        let r = repo(profile);
        setup(&r);
        let (_, out) = check(&r);
        let got = severities(&out, "SD922");
        match want {
            Some(w) => assert_eq!(got, [w], "SD922 in {profile}: {out}"),
            None => assert!(got.is_empty(), "SD922 in {profile}: {out}"),
        }
    }
}

fn never(setup: impl Fn(&Repo)) {
    matrix([None, None, None], setup);
}

#[test]
fn an_open_question_of_an_approved_spec_gives_sd922_in_each_profile() {
    matrix([Some("warning"), Some("error"), Some("error")], |r| {
        change(
            r,
            "accepted",
            "approved",
            &spec_with_questions("- [ ] Who owns the valve timer?\n"),
        )
    });
}

#[test]
fn sd922_fails_check_in_standard_and_names_the_question() {
    let r = repo("standard");
    change(
        &r,
        "accepted",
        "approved",
        &spec_with_questions("- [x] Which unit? Seconds.\n- [ ] Who owns the valve timer?\n"),
    );
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    // Line 13: the front matter, the title and the delta come first.
    assert!(
        out.contains(
            "docs/intent/dose/spec.md:13: error SD922 question-open: the spec is `approved`, and the question `Who owns the valve timer?` is open"
        ),
        "{out}"
    );
    assert_eq!(severities(&out, "SD922"), ["error"], "{out}");
}

#[test]
fn each_open_question_gives_one_diagnostic() {
    let r = repo("strict");
    change(
        &r,
        "accepted",
        "approved",
        &spec_with_questions("- [ ] First?\n- [ ] Second?\n- [x] Third? Yes.\n"),
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD922"), ["error", "error"], "{out}");
}

#[test]
fn an_approved_spec_with_a_resolved_question_passes() {
    for profile in PROFILES {
        let r = repo(profile);
        change(
            &r,
            "accepted",
            "approved",
            &spec_with_questions("- [x] Which unit? Seconds.\n- [X] Who owns it? The team.\n"),
        );
        let (code, out) = check(&r);
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(!out.contains("SD922"), "{profile}: {out}");
    }
}

#[test]
fn only_an_approved_spec_with_an_unchecked_box_in_open_questions_counts() {
    let open = "- [ ] Who owns the valve timer?\n";
    // A draft spec can have an open question.
    never(|r| change(r, "accepted", "draft", &spec_with_questions(open)));
    // No section, and a bullet with no box.
    never(|r| change(r, "accepted", "approved", DELTA));
    never(|r| {
        change(
            r,
            "accepted",
            "approved",
            &spec_with_questions("- Who owns the valve timer?\n"),
        )
    });
    // Another section is not `Open questions`.
    never(|r| {
        change(
            r,
            "accepted",
            "approved",
            &format!("{DELTA}\n## Questions\n\n{open}\n## Unresolved\n\n{open}"),
        )
    });
    // A nested item belongs to its parent.
    never(|r| {
        change(
            r,
            "accepted",
            "approved",
            &spec_with_questions("- [x] Which unit? Seconds.\n  - [ ] nested\n"),
        )
    });
    // A closed change does not proceed.
    never(|r| change(r, "closed", "approved", &spec_with_questions(open)));
}

#[test]
fn the_heading_level_and_the_case_do_not_matter() {
    for heading in [
        "## Open questions",
        "### OPEN QUESTIONS",
        "#### open Questions",
    ] {
        let r = repo("standard");
        change(
            &r,
            "accepted",
            "approved",
            &format!("{DELTA}\n{heading}\n\n- [ ] Who?\n"),
        );
        let (_, out) = check(&r);
        assert_eq!(severities(&out, "SD922"), ["error"], "{heading}: {out}");
    }
}

#[test]
fn sd922_and_sd919_are_independent() {
    let r = repo("standard");
    change(
        &r,
        "accepted",
        "approved",
        &format!(
            "{DELTA}\n## Concerns\n\n- [ ] A concern.\n\n## Open questions\n\n- [ ] A question?\n"
        ),
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD919"), ["error"], "{out}");
    assert_eq!(severities(&out, "SD922"), ["error"], "{out}");
    // A resolved concern does not hide an open question, and the other way.
    let r = repo("standard");
    change(
        &r,
        "accepted",
        "approved",
        &format!(
            "{DELTA}\n## Concerns\n\n- [x] A concern. Done.\n\n## Open questions\n\n- [ ] A question?\n"
        ),
    );
    let (_, out) = check(&r);
    assert!(severities(&out, "SD919").is_empty(), "{out}");
    assert_eq!(severities(&out, "SD922"), ["error"], "{out}");
}

#[test]
fn the_policy_none_and_a_missing_change_table_turn_sd922_off() {
    let body = spec_with_questions("- [ ] Who owns the valve timer?\n");
    for (change_table, rules) in [
        ("[change]\npolicy = \"none\"\n", ""),
        // A rule cannot turn the code on.
        (
            "[change]\npolicy = \"none\"\n",
            "changes = \"error\"\nSD922 = \"error\"\n",
        ),
        // No `[change]` table is the policy `none`.
        ("", ""),
    ] {
        let r = repo_with(&config("strict", change_table, rules));
        change(&r, "accepted", "approved", &body);
        let (code, out) = check(&r);
        assert_eq!(code, 0, "{change_table:?}: {out}");
        assert!(!out.contains("SD922"), "{change_table:?}: {out}");
    }
}

#[test]
fn a_rule_sets_the_severity_of_sd922() {
    let body = spec_with_questions("- [ ] Who owns the valve timer?\n");
    for (profile, rules, want) in [
        ("standard", "SD922 = \"off\"\n", None),
        ("standard", "SD922 = \"warning\"\n", Some("warning")),
        ("lite", "SD922 = \"error\"\n", Some("error")),
        ("standard", "changes = \"off\"\n", None),
        ("lite", "changes = \"error\"\n", Some("error")),
        // A code rule is more specific than a group rule.
        (
            "standard",
            "changes = \"off\"\nSD922 = \"warning\"\n",
            Some("warning"),
        ),
    ] {
        let r = repo_with(&config(profile, THREE, rules));
        change(&r, "accepted", "approved", &body);
        let (_, out) = check(&r);
        let got = severities(&out, "SD922");
        match want {
            Some(w) => assert_eq!(got, [w], "{profile} {rules:?}: {out}"),
            None => assert!(got.is_empty(), "{profile} {rules:?}: {out}"),
        }
    }
}

#[test]
fn the_json_of_sd922_has_the_name_and_the_fix() {
    let r = repo("standard");
    change(
        &r,
        "accepted",
        "approved",
        &spec_with_questions("- [ ] Who owns the valve timer?\n"),
    );
    r.git(&["add", "-A"]);
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    assert!(
        out.contains("\"code\":\"SD922\",\"name\":\"question-open\",\"severity\":\"error\""),
        "{out}"
    );
    assert!(
        out.contains("\"fix\":\"Answer each open question of the spec, or remove it, before the spec is `approved`."),
        "{out}"
    );
}

// The templates

#[test]
fn the_spec_template_has_an_open_questions_section_after_concerns() {
    let r = repo("standard");
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    let (code, out, err) = r.run(&["new", "spec", "drip"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("docs/intent/drip/spec.md");
    let headings: Vec<&str> = text.lines().filter(|l| l.starts_with("## ")).collect();
    assert_eq!(
        headings,
        [
            "## Delta",
            "## Interfaces",
            "## Decisions",
            "## Concerns",
            "## Open questions"
        ],
        "{text}"
    );
    assert!(text.contains("## Open questions\n\n- [ ] {{"), "{text}");
    assert!(text.len() < 700, "{} bytes", text.len());
}

#[test]
fn the_plan_template_shows_2_task_lines_in_the_order_section() {
    let r = repo("standard");
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    let (code, out, err) = r.run(&["new", "plan", "drip"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("docs/intent/drip/plan.md");
    let order = text
        .split("## Order\n")
        .nth(1)
        .and_then(|t| t.split("\n## ").next())
        .expect("an Order section");
    let tasks: Vec<&str> = order.lines().filter(|l| l.starts_with("- [ ] ")).collect();
    assert_eq!(tasks.len(), 2, "{text}");
    assert!(tasks[0].starts_with("- [ ] t1: "), "{text}");
    assert!(tasks[1].starts_with("- [ ] t2: "), "{text}");
    for t in tasks {
        assert!(t.contains(" Files: ") && t.contains(" Verify: "), "{t}");
    }
    assert!(text.len() < 700, "{} bytes", text.len());
}

#[test]
fn the_new_spec_and_plan_have_only_placeholders_to_fill() {
    let r = repo("standard");
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "plan", "drip"]).0, 0);
    r.git(&["add", "-A"]);
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    let mut codes: Vec<&str> = out
        .split("\"code\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    codes.sort();
    codes.dedup();
    // SD306 for the placeholders. No SD305: the new section is not required.
    assert!(codes.contains(&"SD306"), "{codes:?}");
    assert!(!codes.contains(&"SD305"), "{codes:?}");
    assert!(!codes.contains(&"SD302"), "{codes:?}");
    assert!(!codes.contains(&"SD922"), "{codes:?}");
}
