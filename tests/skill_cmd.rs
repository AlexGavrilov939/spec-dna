//! Tests of `spec-dna skill list` and `spec-dna skill show` (ADR 0011, D4 and
//! D10). The command reads no document. Its text is a constant in the binary.
//! Each test uses a temporary repository.

mod common;

use common::Repo;

/// The 12 names, with the kind, the stage and a flag for the 8 skills of D2
/// (the 7 and `accept`), in the order of D2. The default of a skill follows the rule of ADR 0023,
/// so the tests read it with `expected_defaults`.
const SKILLS: [(&str, &str, &str, bool); 12] = [
    ("brainstorm", "stage", "plan", true),
    ("specify", "stage", "design", true),
    ("clarify", "stage", "design", true),
    ("write-plan", "stage", "build", true),
    ("execute", "stage", "build", false),
    ("tdd", "stage", "test", false),
    ("review", "stage", "deploy", true),
    ("verify", "stage", "deploy", false),
    ("accept", "stage", "deploy", true),
    ("finish", "stage", "closed", true),
    ("debug", "event", "-", false),
    ("post-mortem", "event", "-", true),
];

/// The default of each skill (`on` or `off`), in the order of D2, for each
/// value of the constant of ADR 0023: the first list is for `true`, and the
/// second is for `false`. The verdicts come from the table of this build
/// (`eval verdict --release`), so a new table changes no test.
fn expected_defaults() -> [Vec<&'static str>; 2] {
    let r = Repo::new();
    let (code, out, err) = r.run(&["eval", "verdict", "--release", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let verdict = |name: &str| -> Option<String> {
        let key = format!("{{\"name\":\"{name}\",\"verdict\":\"");
        out.find(&key)
            .map(|at| out[at + key.len()..].split('"').next().unwrap().to_string())
    };
    let word = |on: bool| if on { "on" } else { "off" };
    let list = |needs_on: bool| -> Vec<&'static str> {
        SKILLS
            .iter()
            .map(|(name, _, _, in_d2)| {
                let v = verdict(name);
                let on = v.as_deref() == Some("on")
                    || (!needs_on && *in_d2 && v.as_deref() != Some("off"));
                word(on)
            })
            .collect()
    };
    [list(true), list(false)]
}

/// The `default` of each item of the JSON of `skill list`.
fn defaults_of_json(out: &str) -> Vec<String> {
    out.split("{\"name\":\"")
        .skip(1)
        .map(|item| {
            let at = item.find("\"default\":\"").expect("a default") + 11;
            item[at..].split('"').next().unwrap().to_string()
        })
        .collect()
}

fn project(harness: &str) -> Repo {
    let r = Repo::with_config(&format!("\n[[roots]]\npath = \"docs\"\n\n{harness}"));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    r
}

/// The text of a skill file between its front matter and its marker.
fn body_of(text: &str) -> &str {
    let rest = text.strip_prefix("---\n").expect("front matter");
    let (_, rest) = rest.split_once("\n---\n").expect("front matter end");
    let end = rest.rfind("\n<!-- spec-dna harness v").expect("marker");
    rest[..end].trim_matches('\n')
}

#[test]
fn list_prints_one_line_for_each_skill_with_no_configuration() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 0, "{out}{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(lines.len(), 12, "{out}");
    for (l, (name, kind, stage, _)) in lines.iter().zip(SKILLS) {
        // The name, the kind, the stage and the state. The default is checked
        // below.
        assert_eq!(l[..4], [name, kind, stage, "-"], "{out}");
    }
    // The default column is the default of one value of the constant.
    let defaults: Vec<&str> = lines.iter().map(|l| l[4]).collect();
    assert!(
        expected_defaults().contains(&defaults),
        "{defaults:?} is the default of no value of the constant: {:?}\n{out}",
        expected_defaults()
    );
}

#[test]
fn list_takes_the_state_from_the_skills_of_the_configuration() {
    let r =
        project("[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\", \"review\", \"tdd\"]\n");
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 0, "{out}{err}");
    for (l, (name, _, _, _)) in out.lines().zip(SKILLS) {
        let state = l.split_whitespace().nth(3).unwrap();
        let want = if ["brainstorm", "review", "tdd"].contains(&name) {
            "on"
        } else {
            "off"
        };
        assert_eq!(state, want, "{name}\n{out}");
    }
    // A project with no `[harness]` table has no skill on.
    let r = project("");
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.lines()
            .all(|l| l.split_whitespace().nth(3) == Some("off")),
        "{out}"
    );
}

#[test]
fn list_json_has_the_fields_of_the_contract() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with(
            "{\"skills\":[{\"name\":\"brainstorm\",\"kind\":\"stage\",\"stage\":\"plan\",\"state\":null,\"default\":\""
        ),
        "{out}"
    );
    // An event skill has no stage.
    assert!(
        out.contains(
            "{\"name\":\"debug\",\"kind\":\"event\",\"stage\":null,\"state\":null,\"default\":\""
        ),
        "{out}"
    );
    // The field `default` is the default of one value of the constant.
    let defaults = defaults_of_json(&out);
    assert!(
        expected_defaults().iter().any(|e| *e == defaults),
        "{defaults:?} is the default of no value of the constant: {:?}",
        expected_defaults()
    );
    assert_eq!(out.matches("\"name\":").count(), 12, "{out}");
    let r = project("[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\"]\n");
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "\"name\":\"brainstorm\",\"kind\":\"stage\",\"stage\":\"plan\",\"state\":\"on\""
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "\"name\":\"specify\",\"kind\":\"stage\",\"stage\":\"design\",\"state\":\"off\""
        ),
        "{out}"
    );
}

#[test]
fn a_configuration_error_gives_exit_code_3() {
    let r = project("[harness]\nlevel = \"lean\"\nskills = [\"intent\"]\n");
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("spec-dna upgrade"), "{err}");
}

#[test]
fn show_prints_the_body_that_init_writes_with_no_front_matter_and_no_marker() {
    let names: Vec<String> = SKILLS.iter().map(|s| format!("\"{}\"", s.0)).collect();
    let r = project(&format!(
        "[harness]\nlevel = \"lean\"\nskills = [{}]\n",
        names.join(", ")
    ));
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for (name, kind, stage, _) in SKILLS {
        let file = r.read(&format!(".claude/skills/spec-dna-{name}/SKILL.md"));
        let body = body_of(&file);
        let (code, out, err) = r.run(&["skill", "show", name]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(out, format!("{body}\n"), "{name}");
        assert!(
            !out.starts_with("---") && !out.contains("<!-- spec-dna"),
            "{name}"
        );
        // The command also works for a skill that is off in the project.
        let (code, json, err) = r.run(&["skill", "show", name, "--json"]);
        assert_eq!(code, 0, "{name}: {json}{err}");
        let stage = if stage == "-" {
            "null".to_string()
        } else {
            format!("\"{stage}\"")
        };
        assert!(
            json.starts_with(&format!(
                "{{\"name\":\"{name}\",\"kind\":\"{kind}\",\"stage\":{stage},\"body\":\""
            )),
            "{name}: {json}"
        );
        assert!(json.trim_end().ends_with("\"}"), "{name}: {json}");
    }
}

#[test]
fn show_works_for_a_skill_that_is_off_and_with_no_configuration() {
    let r = Repo::new();
    for (name, ..) in SKILLS {
        let (code, out, err) = r.run(&["skill", "show", name]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(out.contains("Commands:"), "{name}: {out}");
    }
}

#[test]
fn an_unknown_name_or_a_bad_call_gives_exit_code_2() {
    let r = Repo::new();
    // A name of the pack works, so the exit code 2 below is not a missing
    // command.
    let (code, out, err) = r.run(&["skill", "show", "brainstorm"]);
    assert_eq!(code, 0, "{out}{err}");
    for args in [
        &["skill", "show", "nope"][..],
        // The adapter skill and the 3 old names are no name of the pack.
        &["skill", "show", "spec-dna"][..],
        &["skill", "show", "intent"][..],
        &["skill", "show", "spec"][..],
        &["skill", "show", "plan"][..],
        &["skill", "show", "spec-dna-review"][..],
        &["skill", "show"][..],
        &["skill"][..],
        &["skill", "frob"][..],
    ] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(out.is_empty(), "{args:?}: {out}");
    }
}

#[test]
fn each_skill_that_next_names_has_a_text() {
    // A new project has a change policy, so `next` names a skill.
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["new", "intent", "x"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("intent");
    let (code, out, err) = r.run(&["next", "x", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let names: Vec<&str> = out
        .split("\"skill\":\"")
        .skip(1)
        .map(|p| &p[..p.find('"').unwrap()])
        .collect();
    assert!(!names.is_empty(), "{out}");
    for n in names {
        let (code, text, err) = r.run(&["skill", "show", n]);
        assert_eq!(code, 0, "{n}: {text}{err}");
        assert!(!text.is_empty(), "{n}");
    }
}

#[test]
fn skill_changes_no_file() {
    let r = project("[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n");
    let before = common::tree_bytes(&r.dir);
    for args in [
        &["skill", "list"][..],
        &["skill", "list", "--json"][..],
        &["skill", "show", "write-plan"][..],
    ] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
    }
    assert_eq!(before, common::tree_bytes(&r.dir));
}
