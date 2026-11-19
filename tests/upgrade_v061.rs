//! End-to-end tests of `spec-dna upgrade` on the fixture of 0.6.1 (design
//! 0.7.0, D41, and ADR 0023). The fixture `tests/fixtures/upgrade/v061/` is a
//! project that the real binary 0.6.1 wrote, with a change that has an
//! accepted intent, an approved spec and an approved plan.
//! `tests/fixtures/upgrade/make.sh` records how it was made, and
//! `recorded/v061.txt` holds the result of `check --all` of 0.6.1. A test here
//! runs no old binary.
//!
//! No test names the skills that are on by default: each reads them from
//! `spec-dna skill list --json`, so a new table of the release or a new value
//! of the constant changes no test.

mod common;

use common::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// The 4 new names that replace the skills of 0.6 (D4), and the old names.
const NEW_NAMES: [&str; 4] = ["brainstorm", "specify", "clarify", "write-plan"];
const OLD_SKILL_FILES: [&str; 3] = [
    ".claude/skills/spec-dna-intent/SKILL.md",
    ".claude/skills/spec-dna-plan/SKILL.md",
    ".claude/skills/spec-dna-spec/SKILL.md",
];
const ANSWERS_OWNER: &str = "owner@example.test";

/// A case: the profile line of the file and the pin.
struct Case {
    name: &'static str,
    profile: &'static str,
    /// The pin that the file has before the upgrade.
    pin: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "standard",
        profile: "standard",
        pin: FIXTURE_PIN,
    },
    Case {
        name: "lite",
        profile: "lite",
        pin: FIXTURE_PIN,
    },
    Case {
        name: "strict",
        profile: "strict",
        pin: FIXTURE_PIN,
    },
    // The pin moves while `Cargo.toml` has the version of the binary.
    Case {
        name: "pin-0.6.0",
        profile: "standard",
        pin: "0.6.0",
    },
];

fn env_of(repo: &Repo) -> [(&'static str, String); 3] {
    [
        ("HOME", repo.dir.to_string_lossy().into_owned()),
        ("GIT_CONFIG_GLOBAL", "/dev/null".to_string()),
        ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
    ]
}

/// Runs the binary with no git configuration of the machine.
fn run(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let env = env_of(repo);
    let set: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    repo.run_env(args, &set, &[])
}

/// The default skills of this build, from `skill list --json`, run in a folder
/// with no `spec-dna.toml`.
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

/// The skills that 0.7.0 adds, in the order of D2.
const ADDED_NAMES: [&str; 6] = ["execute", "tdd", "verify", "finish", "debug", "post-mortem"];

/// The names of `skills` that an upgrade of 0.6 gives in this build: each new
/// name of the rename that is on by default, `review` (the list of 0.6.1 has
/// it), and each skill that is new in 0.7.0, is on by default and is proposed
/// for the profile (`lite` proposes `finish` only), in the order of D2.
fn renamed_skills(profile: &str) -> Vec<String> {
    let on = default_skills();
    let is_on = |n: &str| on.iter().any(|d| d == n);
    let mut names: Vec<&str> = NEW_NAMES.into_iter().filter(|n| is_on(n)).collect();
    names.push("review");
    names.extend(
        ADDED_NAMES
            .into_iter()
            .filter(|n| is_on(n))
            .filter(|n| profile != "lite" || *n == "finish"),
    );
    let order = [
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
    ];
    names.sort_by_key(|n| order.iter().position(|o| o == n).unwrap());
    names.iter().map(|n| n.to_string()).collect()
}

fn skill_file(name: &str) -> String {
    format!(".claude/skills/spec-dna-{name}/SKILL.md")
}

/// The pin of the fixture, as the binary 0.6.1 wrote it.
const FIXTURE_PIN: &str = "0.6.1";

/// The fixture on `main`, with the profile and the pin of the case.
fn project(case: &Case) -> Repo {
    let repo = Repo::from_fixture("v061", &[]);
    let toml = repo.read("spec-dna.toml");
    let toml = toml
        .replace(
            "profile = \"standard\"",
            &format!("profile = \"{}\"", case.profile),
        )
        .replacen(
            &format!("spec-dna = \"{FIXTURE_PIN}\""),
            &format!("spec-dna = \"{}\"", case.pin),
            1,
        );
    assert!(
        toml.contains(&format!("spec-dna = \"{}\"", case.pin)),
        "the pin of the case is in the file"
    );
    repo.write("spec-dna.toml", &toml);
    // The case `standard` changes nothing.
    if !repo.git_out(&["status", "--porcelain"]).is_empty() {
        repo.commit("variant");
    }
    repo
}

fn table(repo: &Repo) -> toml::Table {
    repo.read("spec-dna.toml").parse().unwrap()
}

fn skills_of(repo: &Repo) -> Vec<String> {
    table(repo)["harness"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// The paths that git reports as changed, by kind: `M`, `A` (with the
/// untracked files) and `D`.
fn changed(repo: &Repo) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in repo.git_out(&["status", "--porcelain", "-uall"]).lines() {
        let (status, path) = line.split_at(3);
        let kind = match status.trim() {
            "M" => "M",
            "D" => "D",
            "??" | "A" => "A",
            other => panic!("status `{other}` of {path}"),
        };
        out.entry(kind.to_string())
            .or_default()
            .insert(path.to_string());
    }
    out
}

/// `spec-dna.toml` without the pin and the list `skills`.
fn without_skills_and_pin(repo: &Repo) -> toml::Table {
    let mut t = table(repo);
    t.remove("spec-dna");
    t["harness"].as_table_mut().unwrap().remove("skills");
    t
}

/// The number of ids in the proof line of the upgrade.
fn proof_ids(out: &str) -> usize {
    let line = out
        .lines()
        .find(|l| l.starts_with("proof: ok ("))
        .unwrap_or_else(|| panic!("no proof line: {out}"));
    line["proof: ok (".len()..]
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// The findings of `check --all --json` with the severity `error`.
fn errors_of(repo: &Repo) -> Vec<String> {
    let (_, out, err) = run(repo, &["check", "--all", "--json"]);
    assert!(out.starts_with('{'), "{out}{err}");
    out.split("\"severity\":\"error\"")
        .skip(1)
        .map(|rest| {
            let at = rest.find("\"code\":\"").expect("a code") + 8;
            rest[at..].split('"').next().unwrap().to_string()
        })
        .collect()
}

/// The errors that 0.6.1 recorded for the fixture.
fn recorded_errors() -> Vec<String> {
    let text = fs::read_to_string(fixture_dir("recorded").join("v061.txt")).unwrap();
    text.lines()
        .filter_map(|l| l.strip_prefix("finding "))
        .filter(|l| l.split(' ').nth(1) == Some("error"))
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect()
}

#[test]
fn the_recording_is_of_these_bytes() {
    let repo = Repo::from_fixture("v061", &[]);
    let text = fs::read_to_string(fixture_dir("recorded").join("v061.txt")).unwrap();
    assert!(text.starts_with("# case v061, recorded with spec-dna 0.6.1\nexit "));
    let recorded: BTreeMap<String, String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("file "))
        .map(|l| {
            let (blob, path) = l.split_once(' ').unwrap();
            (path.to_string(), blob.to_string())
        })
        .collect();
    let actual: BTreeMap<String, String> = repo
        .git_out(&["ls-files", "-s"])
        .lines()
        .map(|line| {
            let (meta, path) = line.split_once('\t').unwrap();
            (
                path.to_string(),
                meta.split(' ').nth(1).unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        recorded, actual,
        "run tests/fixtures/upgrade/make.sh v061 again"
    );
    // The change and its documents are in the fixture.
    assert!(
        text.lines()
            .any(|l| l.starts_with("doc docs/intent/drip-schedule/plan.md "))
    );
}

#[test]
fn upgrade_writes_the_new_skills_and_changes_no_other_key() {
    for case in CASES {
        let repo = project(case);
        let before = without_skills_and_pin(&repo);
        let (code, out, err) = run(&repo, &["upgrade"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        // The pin is the version of the binary.
        assert_eq!(
            table(&repo)["spec-dna"].as_str(),
            Some(VERSION),
            "{}",
            case.name
        );
        // The list is the rename and the new skills of the profile, by the
        // default of this build.
        assert_eq!(
            skills_of(&repo),
            renamed_skills(case.profile),
            "{}",
            case.name
        );
        // No preset: no other key changed.
        assert_eq!(without_skills_and_pin(&repo), before, "{}", case.name);
        // The paths: the 3 old skills go, each new name with the default on
        // comes, and the other generated files change.
        let kinds = changed(&repo);
        let new: BTreeSet<String> = renamed_skills(case.profile)
            .iter()
            .filter(|n| *n != "review")
            .map(|n| skill_file(n))
            .collect();
        let removed: BTreeSet<String> = OLD_SKILL_FILES.iter().map(|p| p.to_string()).collect();
        assert_eq!(
            kinds.get("A").cloned().unwrap_or_default(),
            new,
            "{}: added",
            case.name
        );
        assert_eq!(
            kinds.get("D").cloned().unwrap_or_default(),
            removed,
            "{}: removed",
            case.name
        );
        // The version mark of each generated file moves from 0.6.1 to the
        // version of the binary, so `Explore.md`, `fallback-advisor.md`,
        // `verifier.md` and `REVIEW.md` change by their mark only.
        let modified: BTreeSet<String> = [
            ".claude/agents/Explore.md",
            ".claude/agents/fallback-advisor.md",
            ".claude/agents/model-team.md",
            ".claude/agents/verifier.md",
            ".claude/skills/spec-dna-review/SKILL.md",
            ".claude/skills/spec-dna/SKILL.md",
            "AGENTS.md",
            "CLAUDE.md",
            "REVIEW.md",
            "spec-dna.toml",
        ]
        .iter()
        .map(|p| p.to_string())
        .collect();
        assert_eq!(
            kinds.get("M").cloned().unwrap_or_default(),
            modified,
            "{}: modified",
            case.name
        );
        // The proof counts the ids of the fixture.
        assert!(proof_ids(&out) > 0, "{}: {out}", case.name);
        // The report names each name that the rule leaves out.
        let on = default_skills();
        for n in NEW_NAMES.iter().filter(|n| !on.iter().any(|d| d == *n)) {
            assert!(
                out.contains(&format!("`{n}` is off by default in 0.7.0")),
                "{}: {n}: {out}",
                case.name
            );
        }
    }
}

#[test]
fn the_result_has_no_new_error_and_the_first_pull_request_passes() {
    for case in CASES {
        let repo = project(case);
        let base = repo.git_out(&["rev-parse", "HEAD"]);
        repo.git(&["switch", "-q", "-c", "up"]);
        let (code, out, err) = run(&repo, &["upgrade"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        // Each error of the tree is an error that 0.6.1 recorded.
        let recorded = recorded_errors();
        for code in errors_of(&repo) {
            assert!(recorded.contains(&code), "{}: new error {code}", case.name);
        }
        repo.commit("upgrade");
        let (code, out, err) = run(&repo, &["check", "--base", &base]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        for id in ["SD503", "SD502"] {
            assert!(!out.contains(id), "{}: {id} in {out}", case.name);
        }
    }
}

#[test]
fn a_second_upgrade_and_init_change_nothing() {
    for case in CASES {
        let repo = project(case);
        let (code, out, err) = run(&repo, &["upgrade"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        repo.commit("upgrade");
        let (code, out, err) = run(&repo, &["upgrade"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        assert!(out.contains("nothing to change"), "{}: {out}", case.name);
        assert_eq!(
            repo.git_out(&["status", "--porcelain"]),
            "",
            "{}",
            case.name
        );
        let (code, out, err) = run(&repo, &["init"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        assert_eq!(
            repo.git_out(&["status", "--porcelain"]),
            "",
            "{}: init changed a file: {out}",
            case.name
        );
    }
}

#[test]
fn reconfigure_adds_the_approvers_and_keeps_the_skills() {
    for case in CASES {
        let repo = project(case);
        let (code, out, err) = run(&repo, &["upgrade"]);
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        repo.commit("upgrade");
        let skills = skills_of(&repo);
        let answers = repo.dir.join("answers.toml");
        fs::write(&answers, format!("approvers = [\"{ANSWERS_OWNER}\"]\n")).unwrap();
        let (code, out, err) = run(
            &repo,
            &[
                "init",
                "--reconfigure",
                "--answers",
                answers.to_str().unwrap(),
            ],
        );
        assert_eq!(code, 0, "{}: {out}{err}", case.name);
        let change = table(&repo)["change"].as_table().unwrap().clone();
        if case.profile == "strict" {
            // `strict` reads the approvals of the base only.
            assert_eq!(change["approval"].as_str(), Some("base"), "{}", case.name);
            assert!(!change.contains_key("approvers"), "{}", case.name);
        } else {
            assert_eq!(change["approval"].as_str(), Some("branch"), "{}", case.name);
            assert_eq!(
                change["approvers"].as_array().unwrap()[0].as_str(),
                Some(ANSWERS_OWNER),
                "{}",
                case.name
            );
        }
        assert_eq!(skills_of(&repo), skills, "{}: the skills stay", case.name);
        let _ = fs::remove_file(&answers);
    }
}

#[test]
fn an_edited_skill_stays_and_gives_sd501_after_the_upgrade() {
    let case = &CASES[0];
    let repo = project(case);
    let path = ".claude/skills/spec-dna-spec/SKILL.md";
    let text = repo.read(path);
    // One more line before the marker: its hash no longer matches.
    let marker_at = text.rfind("<!-- spec-dna harness").expect("a marker");
    let edited = format!(
        "{}Run the linter after the tests.\n\n{}",
        &text[..marker_at],
        &text[marker_at..]
    );
    repo.write(path, &edited);
    repo.commit("edit a skill");
    let sd501 = |repo: &Repo| -> bool {
        let (_, out, _) = run(repo, &["check", "--all"]);
        out.lines().any(|l| l.contains("SD501") && l.contains(path))
    };
    // The configuration of 0.6.1 has names that the binary does not know, so
    // `check` stops with exit code 3 before the upgrade. The file stays, and
    // `check` reports it after.
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(repo.read(path), edited, "the edited file stays");
    assert!(sd501(&repo), "SD501 after the upgrade");
    // The report names the file that the upgrade keeps.
    assert!(out.contains(path), "{out}");
}
