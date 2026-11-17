//! Integration tests of the preset of each profile in `spec-dna init`
//! (design 0.7.0, D37, and ADR 0023). A new project gets the keys of its
//! profile. `--reconfigure` keeps the current values. No test here names the
//! default skills: each reads them from `spec-dna skill list --json`, so a new
//! table of the release or a new value of the constant changes no test.

mod common;

use common::*;
use std::fs;
use std::path::PathBuf;

/// The e-mail of the project, for the key `approvers`.
const OWNER: &str = "owner@example.test";

/// The folder `.claude/skills/spec-dna-<name>/SKILL.md` of a skill.
fn skill_file(name: &str) -> String {
    format!(".claude/skills/spec-dna-{name}/SKILL.md")
}

/// A new git repository with a `user.email`.
fn repo() -> Repo {
    let r = Repo::new();
    r.git(&["config", "user.email", OWNER]);
    r
}

/// Runs `init` with an answers file.
fn init_with(r: &Repo, answers: &str, args: &[&str]) -> (i32, String, String) {
    let file = PathBuf::from(format!("{}-answers.toml", r.dir.display()));
    fs::write(&file, answers).unwrap();
    let mut a = vec!["init", "--answers", file.to_str().unwrap()];
    a.extend_from_slice(args);
    let out = run(r, &a);
    let _ = fs::remove_file(&file);
    out
}

/// Runs the binary with no git configuration of the machine.
fn run(r: &Repo, args: &[&str]) -> (i32, String, String) {
    r.run_env(
        args,
        &[
            ("HOME", r.dir.to_str().unwrap()),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ],
        &[],
    )
}

/// The skills that are on by default in this build, in the order of
/// `skill list`. The folder has no `spec-dna.toml`.
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

/// The names of the 11 skills, in the order of `skill list`.
fn all_skills() -> Vec<String> {
    let r = Repo::new();
    let (_, out, _) = r.run(&["skill", "list", "--json"]);
    out.split("{\"name\":\"")
        .skip(1)
        .map(|item| item.split('"').next().unwrap().to_string())
        .collect()
}

fn table(r: &Repo) -> toml::Table {
    r.read("spec-dna.toml").parse().unwrap()
}

fn skills_of(r: &Repo) -> Vec<String> {
    table(r)
        .get("harness")
        .and_then(|h| h.get("skills"))
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

/// The skill files that the repository has.
fn skill_files(r: &Repo) -> Vec<String> {
    all_skills()
        .into_iter()
        .filter(|s| r.dir.join(skill_file(s)).exists())
        .collect()
}

fn str_key<'a>(t: &'a toml::Table, section: &str, key: &str) -> Option<&'a str> {
    t.get(section)?.get(key)?.as_str()
}

#[test]
fn lite_writes_two_class_warn_and_no_review_md_no_protect_tests_no_evals() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"lite\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    assert_eq!(str_key(&t, "change", "policy"), Some("two-class"));
    assert_eq!(str_key(&t, "change", "gate"), Some("warn"));
    // `lite` keeps the approval in the branch, with the e-mail of git.
    assert_eq!(str_key(&t, "change", "approval"), Some("branch"));
    let harness = t.get("harness").unwrap();
    assert_eq!(harness.get("review-md"), Some(&toml::Value::Boolean(false)));
    assert_eq!(
        harness.get("protect-tests").and_then(toml::Value::as_str),
        Some("off")
    );
    assert!(harness.get("protect-approvals").is_none());
    assert!(!r.dir.join("REVIEW.md").exists());
    assert!(t.get("evals").is_none(), "{t:?}");
    assert!(!r.dir.join("evals").exists());
    assert!(!out.contains("eval pack"), "{out}");
    // `check` accepts the project.
    r.commit("adopt spec-dna");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn lite_proposes_only_its_3_skills_with_default_on() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"lite\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let want: Vec<String> = default_skills()
        .into_iter()
        .filter(|s| ["brainstorm", "write-plan", "finish"].contains(&s.as_str()))
        .collect();
    assert_eq!(skills_of(&r), want);
    assert_eq!(skill_files(&r), want);
}

#[test]
fn standard_writes_three_class_block_and_skeleton() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"standard\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    assert_eq!(str_key(&t, "change", "policy"), Some("three-class"));
    assert_eq!(str_key(&t, "change", "gate"), Some("block"));
    assert_eq!(str_key(&t, "change", "approval"), Some("branch"));
    assert!(t.get("evals").is_some());
    assert!(r.dir.join("evals/check.sh").exists());
    let harness = t.get("harness").unwrap();
    assert_eq!(harness.get("review-md"), Some(&toml::Value::Boolean(true)));
    assert_eq!(
        harness.get("protect-tests").and_then(toml::Value::as_str),
        Some("fix-branches")
    );
    assert!(harness.get("protect-approvals").is_none());
    assert!(r.dir.join("REVIEW.md").exists());
    assert_eq!(skills_of(&r), default_skills());
    assert_eq!(skill_files(&r), default_skills());
    // No note on the eval pack for `standard`.
    assert!(!out.contains("eval pack"), "{out}");
}

#[test]
fn strict_writes_base_and_protect_approvals() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    assert_eq!(str_key(&t, "change", "policy"), Some("three-class"));
    assert_eq!(str_key(&t, "change", "gate"), Some("block"));
    assert_eq!(str_key(&t, "change", "approval"), Some("base"));
    let harness = t.get("harness").unwrap();
    assert_eq!(
        harness.get("protect-approvals"),
        Some(&toml::Value::Boolean(true))
    );
    assert_eq!(harness.get("review-md"), Some(&toml::Value::Boolean(true)));
    assert_eq!(skills_of(&r), default_skills());
}

#[test]
fn strict_prints_the_note_of_eval_pack_and_writes_no_pack() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("note: the profile `strict` uses the verdicts of the skills; run `spec-dna eval pack` to write the 57 evals of the pack into `evals/pack/` before `spec-dna eval run --ab` (docs/evals.md)"),
        "{out}"
    );
    assert!(!r.dir.join("evals/pack").exists());
    assert!(r.dir.join("evals/check.sh").exists());
    // `--json` has the note in `notes`.
    let r = repo();
    let (code, out, err) = init_with(
        &r,
        "profile = \"strict\"\nharness = \"lean\"\n",
        &["--dry-run", "--json"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let notes = out.split("\"notes\":[").nth(1).expect("notes").to_string();
    let notes = notes.split("],\"warnings\"").next().unwrap();
    assert!(notes.contains("spec-dna eval pack"), "{notes}");
    // The answer `none` for the evals gives no note.
    let r = repo();
    let (_, out, _) = init_with(
        &r,
        "profile = \"strict\"\nharness = \"lean\"\nevals = \"none\"\n",
        &[],
    );
    assert!(!out.contains("eval pack"), "{out}");
}

#[test]
fn full_proposes_the_11_skills() {
    for profile in ["lite", "standard", "strict"] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"full\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        assert_eq!(skills_of(&r), all_skills(), "{profile}");
        assert_eq!(skill_files(&r), all_skills(), "{profile}");
    }
}

#[test]
fn the_skills_of_init_equal_the_default_of_skill_list() {
    let r = repo();
    let (code, out, err) = init_with(&r, "harness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(skills_of(&r), default_skills());
    // The files are the skills of the list, and no other.
    assert_eq!(skill_files(&r), default_skills());
}

#[test]
fn an_answer_of_a_question_replaces_the_preset() {
    let r = repo();
    let (code, out, err) = init_with(
        &r,
        "profile = \"lite\"\nharness = \"lean\"\nchange-policy = \"three-class\"\ngate = \"block\"\nharness-review-md = true\nharness-protect-tests = \"always\"\nevals = \"skeleton\"\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    assert_eq!(str_key(&t, "change", "policy"), Some("three-class"));
    assert_eq!(str_key(&t, "change", "gate"), Some("block"));
    assert_eq!(
        t.get("harness")
            .unwrap()
            .get("protect-tests")
            .unwrap()
            .as_str(),
        Some("always")
    );
    assert!(t.get("evals").is_some());
    assert!(r.dir.join("REVIEW.md").exists());
}

#[test]
fn reconfigure_gives_a_missing_part_the_default_of_the_profile() {
    // A `lite` file with a `[harness]` table and no `review-md`,
    // `protect-tests` or `skills`.
    let r = Repo::with_config(
        "profile = \"lite\"\n\n[change]\npolicy = \"two-class\"\ngate = \"warn\"\n\n[harness]\nlevel = \"lean\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.git(&["config", "user.email", OWNER]);
    r.commit("start");
    let (code, out, err) = run(&r, &["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    let harness = t.get("harness").unwrap();
    assert_eq!(harness.get("review-md"), Some(&toml::Value::Boolean(false)));
    assert_eq!(
        harness.get("protect-tests").and_then(toml::Value::as_str),
        Some("off")
    );
    let want: Vec<String> = default_skills()
        .into_iter()
        .filter(|s| ["brainstorm", "write-plan", "finish"].contains(&s.as_str()))
        .collect();
    assert_eq!(skills_of(&r), want);
    assert!(!r.dir.join("REVIEW.md").exists());
}

#[test]
fn reconfigure_with_another_profile_keeps_the_values_and_prints_a_note() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"lite\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let before = table(&r);
    r.commit("adopt spec-dna");
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("note: the profile `standard` sets other defaults only for a new project; answer questions 4, 5, 12 and 14 to change them (docs/profiles.md)"),
        "{out}"
    );
    let after = table(&r);
    // Only the profile changed.
    assert_eq!(
        after.get("profile").and_then(toml::Value::as_str),
        Some("standard")
    );
    assert_eq!(after.get("change"), before.get("change"));
    assert_eq!(after.get("evals"), before.get("evals"));
    assert_eq!(after.get("harness"), before.get("harness"));
    assert!(!r.dir.join("REVIEW.md").exists());
    // The same profile gives no note.
    let (_, out, _) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert!(!out.contains("sets other defaults"), "{out}");
}

#[test]
fn reconfigure_to_strict_turns_branch_into_base_with_a_note_and_keeps_the_approvers() {
    let r = repo();
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\napprovers = [\"owner@example.test\"]\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        str_key(&table(&r), "change", "approval"),
        Some("branch"),
        "the start is a project with the approval in the branch"
    );
    r.commit("adopt spec-dna");
    let (code, out, err) = init_with(
        &r,
        "profile = \"strict\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    // `strict` reads the approvals of the base only, so `branch` cannot stay.
    let t = table(&r);
    assert_eq!(str_key(&t, "change", "approval"), Some("base"));
    assert_eq!(
        t["change"]["approvers"].as_array().unwrap()[0].as_str(),
        Some(OWNER),
        "the list stays"
    );
    assert_eq!(
        t["harness"].get("protect-approvals"),
        Some(&toml::Value::Boolean(true))
    );
    assert!(
        out.contains("note: the profile `strict` reads the approvals of the base only, so `init` changes `approval = \"branch\"` to `\"base\"`"),
        "{out}"
    );
    // A second run has nothing to change, so it prints no such note.
    r.commit("strict");
    let (_, out, _) = init_with(
        &r,
        "profile = \"strict\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert!(!out.contains("changes `approval = \"branch\"`"), "{out}");
}

#[test]
fn the_note_of_eval_pack_goes_when_the_pack_folder_exists() {
    let r = repo();
    let answers = "profile = \"strict\"\nharness = \"lean\"\n";
    let (code, out, err) = init_with(&r, answers, &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    // No pack yet: `--reconfigure` repeats the note.
    let (_, out, _) = init_with(&r, answers, &["--reconfigure"]);
    assert!(out.contains("run `spec-dna eval pack`"), "{out}");
    // After `eval pack`, the folder exists and the note goes.
    r.write("evals/pack/placeholder.json", "{}\n");
    let (_, out, _) = init_with(&r, answers, &["--reconfigure"]);
    assert!(!out.contains("spec-dna eval pack"), "{out}");
}
