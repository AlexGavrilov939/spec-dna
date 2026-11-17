//! Tests of `spec-dna skill list` and `spec-dna skill show` (ADR 0011, D4 and
//! D10). The command reads no document. Its text is a constant in the binary.
//! Each test uses a temporary repository.

mod common;

use common::Repo;

/// The 11 names, with the kind, the stage and the default of each, in the
/// order of D2.
const SKILLS: [(&str, &str, &str, &str); 11] = [
    ("brainstorm", "stage", "plan", "on"),
    ("specify", "stage", "design", "on"),
    ("clarify", "stage", "design", "on"),
    ("write-plan", "stage", "build", "on"),
    ("execute", "stage", "build", "off"),
    ("tdd", "stage", "test", "off"),
    ("review", "stage", "deploy", "on"),
    ("verify", "stage", "deploy", "off"),
    ("finish", "stage", "closed", "on"),
    ("debug", "event", "-", "off"),
    ("post-mortem", "event", "-", "on"),
];

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
#[ignore = "workflow-pack t17"]
fn list_prints_one_line_for_each_skill_with_no_configuration() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 0, "{out}{err}");
    let lines: Vec<Vec<&str>> = out
        .lines()
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(lines.len(), 11, "{out}");
    for (l, (name, kind, stage, default)) in lines.iter().zip(SKILLS) {
        // The name, the kind, the stage, the state and the default.
        assert_eq!(l, &[name, kind, stage, "-", default], "{out}");
    }
}

#[test]
#[ignore = "workflow-pack t17"]
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
#[ignore = "workflow-pack t17"]
fn list_json_has_the_fields_of_the_contract() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with(
            "{\"skills\":[{\"name\":\"brainstorm\",\"kind\":\"stage\",\"stage\":\"plan\",\"state\":null,\"default\":\"on\"},"
        ),
        "{out}"
    );
    // An event skill has no stage.
    assert!(
        out.contains(
            "{\"name\":\"debug\",\"kind\":\"event\",\"stage\":null,\"state\":null,\"default\":\"off\"}"
        ),
        "{out}"
    );
    assert_eq!(out.matches("\"name\":").count(), 11, "{out}");
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
#[ignore = "workflow-pack t17"]
fn a_configuration_error_gives_exit_code_3() {
    let r = project("[harness]\nlevel = \"lean\"\nskills = [\"intent\"]\n");
    let (code, out, err) = r.run(&["skill", "list"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("spec-dna upgrade"), "{err}");
}

#[test]
#[ignore = "workflow-pack t17"]
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
#[ignore = "workflow-pack t17"]
fn show_works_for_a_skill_that_is_off_and_with_no_configuration() {
    let r = Repo::new();
    for (name, ..) in SKILLS {
        let (code, out, err) = r.run(&["skill", "show", name]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(out.contains("Commands:"), "{name}: {out}");
    }
}

#[test]
#[ignore = "workflow-pack t17"]
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
#[ignore = "workflow-pack t17"]
fn each_skill_that_next_names_has_a_text() {
    let r = project("");
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
#[ignore = "workflow-pack t17"]
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
