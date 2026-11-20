//! Tests of the new `spec-dna.toml` of `upgrade` (step 5, slice 5). The golden
//! pairs of the plan are in `tests/golden/upgrade/config/`, and a unit test of
//! `src/upgrade_config.rs` checks that the plan gives them. This file checks
//! the property that the plan promises: the 0.6 binary loads the new file. The
//! documents of a fixture still have their 0.5 types, because the plan of the
//! documents is another slice, so `check --all` finds errors in the documents,
//! but it finds no error in the configuration.

mod common;

use common::*;
use std::fs;

/// A case: the golden name, the fixture and the overlays of its repository.
const CASES: &[(&str, &str, &[&str])] = &[
    ("v04", "v04", &[]),
    ("v05", "v05", &[]),
    ("brownfield", "brownfield", &[]),
    ("harness-lean", "v05", &["harness-lean"]),
    ("harness-full", "v05", &["harness-full"]),
];

fn golden(name: &str, side: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/upgrade/config")
        .join(format!("{name}.{side}.toml"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The golden file pins the version 0.6.0. The test pins the version of the
/// binary, so that a later release does not change the golden files.
fn pinned(text: &str) -> String {
    assert!(text.starts_with("spec-dna = \"0.6.0\"\n"), "{text}");
    text.replacen("0.6.0", VERSION, 1)
}

#[test]
fn the_binary_loads_the_new_configuration_of_each_fixture() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        repo.write("spec-dna.toml", &pinned(&golden(name, "after")));
        let (code, out, err) = repo.run(&["check", "--all"]);
        // The documents have 0.5 types, so the exit code is 1. It is never 3,
        // the code of an error in the configuration or the pin.
        assert_eq!(code, 1, "{name}: {out}{err}");
        assert!(!err.contains("spec-dna.toml"), "{name}: {err}");
        // A command that reads the layers and the policy loads it too.
        let (code, out, err) = repo.run(&["layer", "list"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(out.contains("requirements"), "{name}: {out}");
    }
}

#[test]
fn the_old_configuration_of_each_fixture_stops_the_binary() {
    // The control: with the file before the plan, the exit code is 3.
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        repo.write("spec-dna.toml", &golden(name, "before"));
        let (code, _, err) = repo.run(&["check", "--all"]);
        assert_eq!(code, 3, "{name}: {err}");
        assert!(err.contains("spec-dna upgrade"), "{name}: {err}");
    }
}

#[test]
fn the_new_configuration_keeps_the_roots_of_the_old_one() {
    for (name, _, _) in CASES {
        let paths = |text: &str| -> Vec<String> {
            text.lines()
                .filter_map(|l| l.strip_prefix("path = "))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(
            paths(&golden(name, "before")),
            paths(&golden(name, "after")),
            "{name}"
        );
    }
}

// --- the skills that are new in 0.7.0 (ADR 0023, amendment of 2026-11-19) ---

/// The skills that 0.7.0 adds, in the order of D2.
const ADDED: [&str; 7] = [
    "execute",
    "tdd",
    "verify",
    "accept",
    "finish",
    "debug",
    "post-mortem",
];

/// The skills that are on by default in this build, from `skill list --json`.
fn default_skills() -> Vec<String> {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    out.split("{\"name\":\"")
        .skip(1)
        .filter(|item| item.contains("\"default\":\"on\""))
        .map(|item| item.split('"').next().unwrap().to_string())
        .collect()
}

/// A project of 0.6.1 whose `skills` line is `skills`.
fn project_0_6_1(profile: &str, skills: &str) -> Repo {
    let repo = Repo::from_fixture("v061", &[]);
    let text = repo.read("spec-dna.toml");
    assert!(text.contains("spec-dna = \"0.6.1\""), "{text}");
    let text = text
        .replace(
            "skills = [\"intent\", \"spec\", \"plan\", \"review\"]",
            &format!("skills = {skills}"),
        )
        .replace(
            "profile = \"standard\"",
            &format!("profile = \"{profile}\""),
        );
    repo.write("spec-dna.toml", &text);
    repo.commit("variant");
    repo
}

fn skills_of(repo: &Repo) -> Vec<String> {
    let t: toml::Table = repo.read("spec-dna.toml").parse().unwrap();
    t["harness"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn upgrade_adds_the_skills_that_are_new_in_0_7_0_for_the_profile() {
    let on = default_skills();
    for (profile, only_finish) in [("standard", false), ("lite", true)] {
        let repo = project_0_6_1(profile, "[\"review\"]");
        let (code, out, err) = repo.run(&["upgrade"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        // `review` keeps its place, and the new names follow in the order of D2.
        let added: Vec<String> = ADDED
            .iter()
            .filter(|n| on.iter().any(|d| d == *n))
            .filter(|n| !only_finish || **n == "finish")
            .map(|n| n.to_string())
            .collect();
        let mut want = vec!["review".to_string()];
        want.extend(added.clone());
        assert_eq!(skills_of(&repo), want, "{profile}: {out}");
        // `accept` and its agent come together, and `lite` gets neither
        // (ADR 0025).
        for path in [
            ".claude/skills/spec-dna-accept/SKILL.md",
            ".claude/agents/acceptor.md",
        ] {
            assert_eq!(
                repo.dir.join(path).exists(),
                !only_finish,
                "{profile}: {path}: {out}"
            );
        }
        if !added.is_empty() {
            assert!(
                out.contains(", new in 0.7.0")
                    && out.contains(" and on by default for the profile `")
                    && out.contains("remove a name to turn its skill off"),
                "{profile}: {out}"
            );
        }
    }
}

#[test]
fn an_empty_list_of_skills_stays_empty_and_a_list_of_many_lines_does_not_stop_upgrade() {
    let repo = project_0_6_1("standard", "[]");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(skills_of(&repo), Vec::<String>::new(), "{out}");
    // A list of many lines with no old name: the upgrade goes on, and a
    // warning names the skills to add by hand.
    let repo = project_0_6_1("standard", "[\n  \"review\",  # mine\n]");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(skills_of(&repo), ["review"], "{out}");
    if default_skills().iter().any(|n| n == "finish") {
        assert!(
            out.contains(
                "warning: `skills` of `[harness]` is a list of many lines, so upgrade did not add "
            ),
            "{out}"
        );
    }
}

#[test]
fn a_second_upgrade_after_the_commit_adds_no_skill() {
    let repo = project_0_6_1("standard", "[\"review\"]");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    let first = skills_of(&repo);
    repo.commit("upgrade");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
    assert_eq!(skills_of(&repo), first);
}

#[test]
fn a_list_of_many_lines_with_a_name_of_0_6_takes_the_rename_and_gives_a_warning() {
    // The rename keeps the length of the list, so the splice works, as in
    // 0.7.0. The new skills do not fit, so a warning names them and the
    // upgrade does not stop.
    let repo = project_0_6_1("standard", "[\n  \"intent\",  # mine\n  \"review\",\n]");
    let (code, out, err) = repo.run(&["upgrade", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let on = default_skills();
    assert!(
        out.contains(
            "warning: `skills` of `[harness]` is a list of many lines, so upgrade did not add `"
        ),
        "{out}"
    );
    assert!(on.iter().any(|n| n == "finish"));
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    let want: Vec<String> = if on.iter().any(|n| n == "brainstorm") {
        vec!["brainstorm".to_string(), "review".to_string()]
    } else {
        vec!["review".to_string()]
    };
    assert_eq!(skills_of(&repo), want, "{out}");
    assert!(repo.read("spec-dna.toml").contains("# mine"));
}

#[test]
fn the_change_policy_none_adds_no_skill_at_the_upgrade() {
    let repo = Repo::from_fixture("v061", &[]);
    let text = repo.read("spec-dna.toml");
    assert!(text.contains("policy = \"three-class\""), "{text}");
    let new = text.replacen("policy = \"three-class\"", "policy = \"none\"", 1);
    repo.write("spec-dna.toml", &new);
    repo.commit("policy none");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    // The rename of 0.7.0 stays: no new skill joins the list.
    let list = skills_of(&repo);
    for n in ["tdd", "execute", "verify", "finish", "debug", "post-mortem"] {
        assert!(!list.iter().any(|l| l == n), "{n} in {list:?}");
    }
}
