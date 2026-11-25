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
        out.contains("note: the profile `strict` uses the verdicts of the skills; run `spec-dna eval pack` to write the 62 evals of the pack into `evals/pack/` before `spec-dna eval run --ab` (docs/evals.md)"),
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
fn full_proposes_the_11_skills_in_each_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"full\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        // The skill `accept` merged into `review` (design 0.8.0, D71), so each
        // profile has the same 11 skills.
        let want: Vec<String> = all_skills();
        assert_eq!(want.len(), 11, "{want:?}");
        assert!(!want.iter().any(|s| s == "accept"), "{want:?}");
        assert_eq!(skills_of(&r), want, "{profile}");
        assert_eq!(skill_files(&r), want, "{profile}");
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
        out.contains("note: the profile `standard` sets other defaults only for a new project; answer questions 4, 5, 12, 13 and 14 to change them (docs/profiles.md)"),
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

/// The key `[evidence] reports` (design 0.8.0, D1): `standard` and `strict`
/// write `reports = []` with a comment, and `lite` writes none. `init` knows
/// no runner, so it writes no glob.
#[test]
fn standard_and_strict_write_the_key_reports_and_lite_writes_none() {
    for (profile, writes) in [("lite", false), ("standard", true), ("strict", true)] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let text = r.read("spec-dna.toml");
        let t = table(&r);
        let reports = t.get("evidence").and_then(|e| e.get("reports"));
        if writes {
            assert_eq!(
                reports.and_then(toml::Value::as_array).map(Vec::len),
                Some(0),
                "{profile}: {text}"
            );
            assert!(text.contains("\n[evidence]\nreports = []  # "), "{text}");
            // One table `[evidence]`, and `tree-exclude` is not written.
            assert_eq!(text.matches("[evidence]").count(), 1, "{text}");
            assert!(!text.contains("tree-exclude"), "{text}");
        } else {
            assert!(t.get("evidence").is_none(), "{profile}: {text}");
        }
        // `check` accepts the project, with and without the flag.
        r.commit("adopt spec-dna");
        for args in [&["check"][..], &["check", "--reports"][..]] {
            let (code, out, err) = run(&r, args);
            assert_eq!(code, 0, "{profile} {args:?}: {out}{err}");
        }
    }
}

#[test]
fn reconfigure_keeps_the_reports_and_the_tree_exclude() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"standard\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("spec-dna.toml").replace(
        "reports = []",
        "reports = [\"out/*.xml\"]\ntree-exclude = [\"**/*.txt\"]\ntests = [\"checks/**\"]",
    );
    r.write("spec-dna.toml", &text);
    r.commit("adopt spec-dna");
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    let list = |key: &str| -> Vec<String> {
        t["evidence"][key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(list("reports"), ["out/*.xml"]);
    assert_eq!(list("tree-exclude"), ["**/*.txt"]);
    assert_eq!(list("tests"), ["checks/**"]);
    // A profile of `lite` keeps a list that is not empty.
    let (code, out, err) = init_with(
        &r,
        "profile = \"lite\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        table(&r)["evidence"]["reports"].as_array().map(Vec::len),
        Some(1)
    );
}

/// The key `[evidence] reliable-for` (design 0.8.0, D2): `strict` writes it as a
/// comment, because the key needs `reports`. `standard` and `lite` write none.
#[test]
fn strict_writes_reliable_for_as_a_comment_and_the_other_profiles_write_none() {
    for (profile, writes) in [("lite", false), ("standard", false), ("strict", true)] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let text = r.read("spec-dna.toml");
        assert_eq!(
            text.contains("# reliable-for = \"higher-risk\""),
            writes,
            "{profile}: {text}"
        );
        // It is a comment: the table has no key, and `reliable-runs` is absent.
        assert!(!text.contains("\nreliable-for"), "{profile}: {text}");
        assert!(!text.contains("reliable-runs"), "{profile}: {text}");
        let t = table(&r);
        assert!(
            t.get("evidence")
                .and_then(|e| e.get("reliable-for"))
                .is_none(),
            "{profile}: {text}"
        );
        // The comment is in the table `[evidence]`, under `reports`.
        if writes {
            let at = text.find("reports = []").expect("reports");
            assert!(text[at..].contains("# reliable-for"), "{text}");
            assert!(
                text.contains("\"Proof levels and the proof debt\""),
                "{text}"
            );
        }
        r.commit("adopt spec-dna");
        for args in [&["check"][..], &["check", "--reports"][..]] {
            let (code, out, err) = run(&r, args);
            assert_eq!(code, 0, "{profile} {args:?}: {out}{err}");
        }
    }
}

#[test]
fn reconfigure_keeps_reliable_for_and_reliable_runs() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("spec-dna.toml").replace(
        "reports = []",
        "reports = [\"out/*.xml\"]\nreliable-runs = 5\nreliable-for = \"all\"",
    );
    r.write("spec-dna.toml", &text);
    r.commit("adopt spec-dna");
    for profile in ["strict", "standard"] {
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &["--reconfigure"],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = table(&r);
        let ev = &t["evidence"];
        assert_eq!(ev["reliable-runs"].as_integer(), Some(5), "{profile}");
        assert_eq!(ev["reliable-for"].as_str(), Some("all"), "{profile}");
        assert_eq!(r.read("spec-dna.toml").matches("reliable-for").count(), 1);
    }
}

/// The key `[change] higher-risk-acceptors` (design 0.8.0, D3 item 7): `strict`
/// writes 2, `standard` and `lite` write none.
#[test]
fn strict_writes_two_higher_risk_acceptors_and_the_other_profiles_write_none() {
    for (profile, want) in [("lite", None), ("standard", None), ("strict", Some(2))] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = table(&r);
        let got = t
            .get("change")
            .and_then(|c| c.get("higher-risk-acceptors"))
            .and_then(toml::Value::as_integer);
        assert_eq!(got, want, "{profile}: {}", r.read("spec-dna.toml"));
        r.commit("adopt spec-dna");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
    }
}

#[test]
fn reconfigure_keeps_higher_risk_acceptors() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r
        .read("spec-dna.toml")
        .replace("higher-risk-acceptors = 2", "higher-risk-acceptors = 3");
    r.write("spec-dna.toml", &text);
    r.commit("adopt spec-dna");
    for profile in ["strict", "standard"] {
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &["--reconfigure"],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = table(&r);
        assert_eq!(
            t["change"]["higher-risk-acceptors"].as_integer(),
            Some(3),
            "{profile}"
        );
        assert_eq!(
            r.read("spec-dna.toml")
                .matches("higher-risk-acceptors")
                .count(),
            1
        );
    }
}

/// The key `[change] invariants` (design 0.8.0, D6): `strict` writes `true`,
/// `standard` and `lite` write none.
#[test]
fn strict_writes_the_key_invariants_and_the_other_profiles_write_none() {
    for (profile, want) in [("lite", None), ("standard", None), ("strict", Some(true))] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let got = table(&r)
            .get("change")
            .and_then(|c| c.get("invariants"))
            .and_then(toml::Value::as_bool);
        assert_eq!(got, want, "{profile}: {}", r.read("spec-dna.toml"));
        r.commit("adopt spec-dna");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
    }
}

#[test]
fn reconfigure_keeps_the_key_invariants_as_the_file_sets_it() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    // `standard` with the key that `strict` wrote: the key stays.
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(table(&r)["change"]["invariants"].as_bool(), Some(true));
    assert_eq!(r.read("spec-dna.toml").matches("invariants").count(), 1);
    // A file that sets `false` keeps `false` in `strict`.
    let text = r
        .read("spec-dna.toml")
        .replace("invariants = true", "invariants = false");
    r.write("spec-dna.toml", &text);
    r.commit("turn the key off");
    let (code, out, err) = init_with(
        &r,
        "profile = \"strict\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(table(&r)["change"]["invariants"].as_bool(), Some(false));
}

/// The area budget (design 0.8.0, D55) of a file: its `[budgets.area]` as 2
/// numbers.
fn area_budget(r: &Repo) -> Option<(i64, i64)> {
    let t = table(r);
    let a = t.get("budgets")?.get("area")?;
    Some((a.get("soft")?.as_integer()?, a.get("hard")?.as_integer()?))
}

/// `standard` writes `[budgets.area]` with 60,000 and 120,000, `strict` with
/// 40,000 and 80,000, and `lite` writes none. The budget level scales it as it
/// scales the budget of a type, and `off` writes none.
#[test]
fn a_new_project_gets_the_area_budget_of_its_profile_and_budget_level() {
    for (profile, level, want) in [
        ("lite", "default", None),
        ("standard", "default", Some((60_000, 120_000))),
        ("strict", "default", Some((40_000, 80_000))),
        ("standard", "relaxed", Some((120_000, 240_000))),
        ("standard", "tight", Some((30_000, 60_000))),
        ("standard", "off", None),
        ("strict", "off", None),
    ] {
        let r = repo();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\nbudgets = \"{level}\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile} {level}: {out}{err}");
        let text = r.read("spec-dna.toml");
        assert_eq!(area_budget(&r), want, "{profile} {level}: {text}");
        assert_eq!(
            text.matches("[budgets.area]").count(),
            usize::from(want.is_some()),
            "{profile} {level}: {text}"
        );
        // `check` accepts the new project.
        r.commit("adopt spec-dna");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{profile} {level}: {out}{err}");
    }
}

/// `--reconfigure` keeps the table of the file with its values, writes none
/// for a file that has none, and never writes it twice.
#[test]
fn reconfigure_keeps_the_area_budget_and_adds_none() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"standard\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r
        .read("spec-dna.toml")
        .replace("hard = 120000", "hard = 99000");
    r.write("spec-dna.toml", &text);
    r.commit("adopt spec-dna");
    // The budget level scales the tables of the types. It never changes the
    // table of the area.
    for level in ["default", "default", "relaxed", "tight", "off", "default"] {
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"standard\"\nharness = \"lean\"\nbudgets = \"{level}\"\n"),
            &["--reconfigure"],
        );
        assert_eq!(code, 0, "{level}: {out}{err}");
        assert_eq!(area_budget(&r), Some((60_000, 99_000)), "{level}");
        assert_eq!(
            r.read("spec-dna.toml").matches("[budgets.area]").count(),
            1,
            "{level}"
        );
    }
    // A file of another profile keeps it too.
    let (code, out, err) = init_with(
        &r,
        "profile = \"lite\"\nharness = \"lean\"\nbudgets = \"relaxed\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(area_budget(&r), Some((60_000, 99_000)));
    // A file with no table gets none: the table is for a new project.
    let text = r.read("spec-dna.toml");
    let at = text.find("\n[budgets.area]").unwrap();
    let end = text[at + 1..]
        .find("\n\n")
        .map_or(text.len(), |i| at + 1 + i);
    r.write("spec-dna.toml", &format!("{}{}", &text[..at], &text[end..]));
    r.commit("drop the area budget");
    assert_eq!(area_budget(&r), None);
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(area_budget(&r), None, "{}", r.read("spec-dna.toml"));
}

/// The key `[evals] min-free-mib` (D100). `init` writes none, so `--reconfigure`
/// keeps a value that is not the default, and writes the key at most once.
#[test]
fn reconfigure_keeps_min_free_mib_when_it_is_not_the_default() {
    let r = repo();
    let answers = "profile = \"standard\"\nharness = \"lean\"\nevals = \"skeleton\"\n";
    let (code, out, err) = init_with(&r, answers, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let fresh = r.read("spec-dna.toml");
    assert!(fresh.contains("\n[evals]"), "{fresh}");
    assert!(!fresh.contains("min-free-mib"), "{fresh}");
    r.commit("adopt spec-dna");
    for value in [0_i64, 512, 20_000] {
        let text = r.read("spec-dna.toml");
        let at = text.find("\n[evals]").unwrap() + 1;
        let line_end = at + text[at..].find('\n').unwrap() + 1;
        let rest: String = text[line_end..]
            .split_inclusive('\n')
            .filter(|l| !l.starts_with("min-free-mib"))
            .collect();
        r.write(
            "spec-dna.toml",
            &format!("{}min-free-mib = {value}\n{rest}", &text[..line_end]),
        );
        r.commit("set the floor");
        for round in 1..=2 {
            let (code, out, err) = init_with(&r, answers, &["--reconfigure"]);
            assert_eq!(code, 0, "{value} {round}: {out}{err}");
            let new = r.read("spec-dna.toml");
            assert_eq!(
                table(&r)["evals"]
                    .get("min-free-mib")
                    .and_then(|v| v.as_integer()),
                Some(value),
                "{value} {round}: {new}"
            );
            assert_eq!(new.matches("min-free-mib").count(), 1, "{new}");
        }
        r.commit("reconfigure");
    }
    // The default value needs no line.
    let text = r
        .read("spec-dna.toml")
        .replace("min-free-mib = 20000", "min-free-mib = 2048");
    r.write("spec-dna.toml", &text);
    r.commit("the default");
    let (code, out, err) = init_with(&r, answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let new = r.read("spec-dna.toml");
    assert!(!new.contains("min-free-mib"), "{new}");
}

/// The key `[change] acceptance-for` (design 0.8.0, D70). `init` writes none,
/// so `--reconfigure` keeps the value that the file sets, also a value that is
/// the default of the profile. It never writes the key twice.
#[test]
fn reconfigure_keeps_the_key_acceptance_for_in_each_profile() {
    for profile in ["lite", "standard", "strict"] {
        for value in ["none", "higher-risk", "all"] {
            let r = repo();
            let answers = format!("profile = \"{profile}\"\nharness = \"lean\"\n");
            let (code, out, err) = init_with(&r, &answers, &[]);
            assert_eq!(code, 0, "{profile} {value}: {out}{err}");
            assert!(!r.read("spec-dna.toml").contains("acceptance-for"));
            let text = r.read("spec-dna.toml").replace(
                "\nmax-fanout = 8",
                &format!("\nacceptance-for = \"{value}\"\nmax-fanout = 8"),
            );
            r.write("spec-dna.toml", &text);
            r.commit("set the key");
            for round in 1..=2 {
                let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
                assert_eq!(code, 0, "{profile} {value} {round}: {out}{err}");
                assert_eq!(
                    table(&r)["change"]["acceptance-for"].as_str(),
                    Some(value),
                    "{profile} {value} {round}: {}",
                    r.read("spec-dna.toml")
                );
                assert_eq!(
                    r.read("spec-dna.toml").matches("acceptance-for").count(),
                    1,
                    "{profile} {value} {round}"
                );
            }
        }
    }
}

/// Reads the value at a dotted path of a TOML table. No key of the path has a
/// dot.
fn at<'a>(t: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
    let mut keys = path.split('.');
    let mut v = t.get(keys.next()?)?;
    for k in keys {
        v = v.get(k)?;
    }
    Some(v)
}

/// Each key that 0.8.0 added and that `init` does not ask for (the list is
/// `schema/spec-dna.schema.json` against the tag `v0.7.1`, and the groups of
/// `[rules]`, `[layers.summaries]` and `[owners]`). A value that a person set
/// survives `init --reconfigure` in each profile, and a second run changes
/// nothing.
#[test]
fn reconfigure_keeps_each_key_that_0_8_0_added() {
    use toml::Value;
    let s = |x: &str| Value::String(x.to_string());
    let list = |xs: &[&str]| Value::Array(xs.iter().map(|x| s(x)).collect());
    let keys: Vec<(&str, Value)> = vec![
        ("change.acceptance-for", s("none")),
        ("change.higher-risk-acceptors", Value::Integer(3)),
        ("change.invariants", Value::Boolean(true)),
        ("change.max-task-files", Value::Integer(5)),
        ("owners.src/**", list(&["security"])),
        ("budgets.area.soft", Value::Integer(111)),
        ("budgets.area.hard", Value::Integer(222)),
        ("budgets.summary.soft", Value::Integer(33)),
        ("budgets.summary.hard", Value::Integer(44)),
        ("evidence.reports", list(&["reports/**/*.xml"])),
        ("evidence.reliable-runs", Value::Integer(4)),
        ("evidence.reliable-for", s("all")),
        ("evidence.tree-exclude", list(&["**/*.txt"])),
        ("rules.acceptance", s("off")),
        ("rules.rollup", s("warning")),
        ("rules.area-budget", s("off")),
        ("rules.proof", s("off")),
        ("rules.reports", s("off")),
        ("layers.summaries.types", list(&["summary"])),
        ("harness.roles.acceptor.claude-code", s("sonnet")),
        ("harness.roles.acceptor.effort", s("low")),
        ("harness.roles.acceptor.advisor", Value::Boolean(true)),
    ];
    let mut report = Vec::new();
    for profile in ["lite", "standard", "strict"] {
        let r = repo();
        let answers = format!("profile = \"{profile}\"\nharness = \"lean\"\n");
        r.write("docs/a.md", "# A\n\nText.\n");
        r.write(
            "spec-dna.toml",
            &format!(
                "spec-dna = \"{VERSION}\"\nprofile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"warn\"\nacceptance-for = \"none\"\nhigher-risk-acceptors = 3\ninvariants = true\nmax-task-files = 5\n\n[owners]\n\"src/**\" = [\"security\"]\n\n[budgets.area]\nsoft = 111\nhard = 222\n\n[budgets.summary]\nsoft = 33\nhard = 44\n\n[evidence]\nreports = [\"reports/**/*.xml\"]\nreliable-runs = 4\nreliable-for = \"all\"\ntree-exclude = [\"**/*.txt\"]\n\n[rules]\nacceptance = \"off\"\nrollup = \"warning\"\narea-budget = \"off\"\nproof = \"off\"\nreports = \"off\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n[harness]\nlevel = \"lean\"\n\n[harness.roles]\nacceptor = {{ claude-code = \"sonnet\", effort = \"low\", advisor = true }}\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
            ),
        );
        r.commit("set the keys");
        let mut first = String::new();
        for round in 1..=2 {
            let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
            assert_eq!(code, 0, "{profile} {round}: {out}{err}");
            let got = table(&r);
            let lost: Vec<&str> = keys
                .iter()
                .filter(|(path, v)| at(&got, path) != Some(v))
                .map(|(path, _)| *path)
                .collect();
            if !lost.is_empty() {
                report.push(format!("{profile} {round}: {lost:?}"));
            }
            if round == 1 {
                first = r.read("spec-dna.toml");
            } else {
                assert_eq!(r.read("spec-dna.toml"), first, "{profile}: second run");
            }
        }
    }
    assert_eq!(
        report,
        Vec::<String>::new(),
        "keys that `--reconfigure` lost"
    );
}

/// A value of the role `acceptor` that a person set stays when the configuration
/// has no such role: the profile `lite`, or a skill list with no `review`. It
/// comes back into use with the skill. A new project writes no entry then.
#[test]
fn reconfigure_keeps_the_acceptor_entry_where_the_role_is_not_in_use() {
    for (profile, skills) in [
        ("standard", "[\"brainstorm\"]"),
        ("strict", "[]"),
        ("lite", "[\"brainstorm\"]"),
    ] {
        let r = repo();
        let answers = format!("profile = \"{profile}\"\nharness = \"lean\"\n");
        r.write("docs/a.md", "# A\n\nText.\n");
        r.write(
            "spec-dna.toml",
            &format!(
                "spec-dna = \"{VERSION}\"\nprofile = \"{profile}\"\n\n[harness]\nlevel = \"lean\"\nskills = {skills}\n\n[harness.roles]\nacceptor = {{ claude-code = \"sonnet\", effort = \"low\" }}\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
            ),
        );
        r.commit("set the role");
        let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = table(&r);
        assert_eq!(
            at(&t, "harness.roles.acceptor.claude-code").and_then(toml::Value::as_str),
            Some("sonnet"),
            "{profile}: {}",
            r.read("spec-dna.toml")
        );
        assert!(
            !r.dir.join(".claude/agents/acceptor.md").exists(),
            "{profile}: the role is not in use, so it has no file"
        );
        r.commit("reconfigure");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
    }
}

// ---- The key `advisor` of the acceptor (design 0.8.0, D81) ----

/// The line that `acceptor.md` has with `advisor = false` (D81 item 4).
const NO_ADVISOR: &str = "Advisor: do not consult the advisor in this task. This rule has priority over each project instruction that tells you to consult the advisor.";

const ACCEPTOR_FILE: &str = ".claude/agents/acceptor.md";

/// `[harness.roles] acceptor.advisor` of `spec-dna.toml`.
fn advisor_key(r: &Repo) -> Option<bool> {
    at(&table(r), "harness.roles.acceptor.advisor").and_then(toml::Value::as_bool)
}

/// The number of lines of `acceptor.md` that are the line of the key, or
/// `None` if the file does not exist.
fn advisor_lines(r: &Repo) -> Option<usize> {
    fs::read_to_string(r.dir.join(ACCEPTOR_FILE))
        .ok()
        .map(|t| t.matches(NO_ADVISOR).count())
}

/// The profile answers of a project at the level `lean`.
fn answers_of(profile: &str) -> String {
    format!("profile = \"{profile}\"\nharness = \"lean\"\n")
}

/// The test of the owner (Q54). `init` writes the default of each profile: the
/// key and the text of the agent `acceptor`.
#[test]
fn each_profile_writes_the_advisor_of_the_acceptor() {
    assert_eq!(NO_ADVISOR.len(), 141);
    for (profile, key, lines) in [
        ("lite", None, None),
        ("standard", Some(false), Some(1)),
        ("strict", Some(true), Some(0)),
    ] {
        let r = repo();
        let (code, out, err) = init_with(&r, &answers_of(profile), &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        assert_eq!(
            advisor_key(&r),
            key,
            "{profile}: {}",
            r.read("spec-dna.toml")
        );
        assert_eq!(advisor_lines(&r), lines, "{profile}");
        if profile == "lite" {
            // The role does not exist: no entry and no file, as in 0.7.1.
            assert!(
                at(&table(&r), "harness.roles.acceptor").is_none(),
                "{}",
                r.read("spec-dna.toml")
            );
        }
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
    }
}

/// `--reconfigure` keeps the key, also a value that is the default of the
/// profile, and it never writes the key twice. A file with the entry and no key
/// gets the default of its profile.
#[test]
fn reconfigure_keeps_the_advisor_of_the_acceptor_in_each_profile() {
    for profile in ["standard", "strict"] {
        let default = profile == "strict";
        for value in [true, false] {
            let r = repo();
            let answers = answers_of(profile);
            let (code, out, err) = init_with(&r, &answers, &[]);
            assert_eq!(code, 0, "{profile} {value}: {out}{err}");
            let text = r.read("spec-dna.toml").replace(
                &format!("advisor = {default}"),
                &format!("advisor = {value}"),
            );
            r.write("spec-dna.toml", &text);
            r.commit("set the key");
            for round in 1..=2 {
                let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
                assert_eq!(code, 0, "{profile} {value} {round}: {out}{err}");
                assert_eq!(advisor_key(&r), Some(value), "{profile} {value} {round}");
                assert_eq!(
                    r.read("spec-dna.toml").matches(", advisor = ").count(),
                    1,
                    "{profile} {value} {round}"
                );
                assert_eq!(
                    advisor_lines(&r),
                    Some(usize::from(!value)),
                    "{profile} {value} {round}"
                );
            }
        }
        // A file of the release candidate: the entry has no key.
        let r = repo();
        let answers = answers_of(profile);
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let text = r
            .read("spec-dna.toml")
            .replace(&format!(", advisor = {default}"), "");
        r.write("spec-dna.toml", &text);
        r.commit("remove the key");
        let mut first = String::new();
        for round in 1..=2 {
            let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
            assert_eq!(code, 0, "{profile} {round}: {out}{err}");
            assert_eq!(advisor_key(&r), Some(default), "{profile} {round}");
            if round == 1 {
                first = r.read("spec-dna.toml");
            } else {
                assert_eq!(r.read("spec-dna.toml"), first, "{profile}: second run");
            }
        }
    }
}

/// A new answer to question 1 moves a value that is the default of the old
/// profile to the default of the new profile (D81 item 7). Another value stays.
#[test]
fn reconfigure_with_another_profile_moves_a_default_advisor() {
    let r = repo();
    let (code, out, err) = init_with(&r, &answers_of("standard"), &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(false));
    r.commit("adopt spec-dna");
    // `standard` to `strict`: the default `false` becomes `true`.
    let (code, out, err) = init_with(&r, &answers_of("strict"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(true), "{}", r.read("spec-dna.toml"));
    assert_eq!(advisor_lines(&r), Some(0));
    // A second run with the same profile changes nothing.
    let first = r.read("spec-dna.toml");
    let (code, out, err) = init_with(&r, &answers_of("strict"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read("spec-dna.toml"), first);
    r.commit("strict");
    // The way back: `true` becomes `false`, and the line returns.
    let (code, out, err) = init_with(&r, &answers_of("standard"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(false), "{}", r.read("spec-dna.toml"));
    assert_eq!(advisor_lines(&r), Some(1));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");

    // A value that is not the default of the old profile stays: `true` in a
    // `standard` file stays in `strict`.
    let r = repo();
    let (code, out, err) = init_with(&r, &answers_of("standard"), &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r
        .read("spec-dna.toml")
        .replace("advisor = false", "advisor = true");
    r.write("spec-dna.toml", &text);
    r.commit("set the key");
    let (code, out, err) = init_with(&r, &answers_of("strict"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(true));
    assert_eq!(advisor_lines(&r), Some(0));
    // The file is now the same as a `strict` file with the default, so the way
    // back moves the value: a file holds no mark of the person (D81 item 7).
    let fresh = repo();
    let (code, out, err) = init_with(&fresh, &answers_of("strict"), &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        at(&table(&r), "harness.roles.acceptor"),
        at(&table(&fresh), "harness.roles.acceptor")
    );
    assert_eq!(r.read(ACCEPTOR_FILE), fresh.read(ACCEPTOR_FILE));
    r.commit("strict");
    let (code, out, err) = init_with(&r, &answers_of("standard"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(false));

    // `false` in a `strict` file is not the default of `strict`: it stays.
    let r = repo();
    let (code, out, err) = init_with(&r, &answers_of("strict"), &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r
        .read("spec-dna.toml")
        .replace("advisor = true", "advisor = false");
    r.write("spec-dna.toml", &text);
    r.commit("set the key");
    let (code, out, err) = init_with(&r, &answers_of("standard"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), Some(false));
    assert_eq!(advisor_lines(&r), Some(1));
}

/// A `lite` file with a kept entry (a model of a person) and `advisor = true`
/// keeps the value when the answer to question 1 is `standard` (D81 item 7: `lite`
/// has no default to move). It keeps it also when the skill `review` returns.
#[test]
fn reconfigure_from_lite_keeps_the_advisor_of_a_kept_entry() {
    let file = |skills: &str| {
        format!(
            "spec-dna = \"{VERSION}\"\nprofile = \"lite\"\n\n[harness]\nlevel = \"lean\"\nskills = {skills}\n\n[harness.roles]\nacceptor = {{ claude-code = \"sonnet\", effort = \"high\", advisor = true }}\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
        )
    };
    let answers = answers_of("standard");
    let want = Some(true);
    // Without `review`, the role does not exist: the entry stays, with no file.
    let r = repo();
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write("spec-dna.toml", &file("[\"brainstorm\"]"));
    r.commit("set the role");
    let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), want, "{}", r.read("spec-dna.toml"));
    assert_eq!(
        at(&table(&r), "harness.roles.acceptor.claude-code").and_then(toml::Value::as_str),
        Some("sonnet")
    );
    assert_eq!(advisor_lines(&r), None);
    // The skill `review` returns: the file has no line, and `check` is clean.
    r.write(
        "spec-dna.toml",
        &r.read("spec-dna.toml")
            .replace("skills = [\"brainstorm\"]", "skills = [\"review\"]"),
    );
    r.commit("add review");
    let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), want, "{}", r.read("spec-dna.toml"));
    assert_eq!(advisor_lines(&r), Some(0));
    r.commit("reconfigure");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // With `review` in the list at once, the role appears with the answer
    // `standard`, and the value is not the default of `lite` to move.
    let r = repo();
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write("spec-dna.toml", &file("[\"review\"]"));
    r.commit("set the role");
    let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(advisor_key(&r), want, "{}", r.read("spec-dna.toml"));
    assert_eq!(advisor_lines(&r), Some(0));
}

/// Where the role `acceptor` does not exist, the key `advisor` alone never
/// keeps the entry (D81 item 8). A model or an effort of a person keeps it, with
/// its key.
#[test]
fn reconfigure_drops_an_entry_with_no_effect_that_holds_only_the_advisor() {
    let skills_of_file = |r: &Repo, list: &str| {
        let text = r.read("spec-dna.toml");
        let line = text
            .lines()
            .find(|l| l.starts_with("skills = "))
            .unwrap()
            .to_string();
        r.write(
            "spec-dna.toml",
            &text.replace(&line, &format!("skills = {list}")),
        );
        r.commit("set the skills");
    };
    // The skill `review` goes: the default entry goes too, with its key.
    for (profile, value) in [("standard", false), ("standard", true), ("strict", true)] {
        let r = repo();
        let answers = answers_of(profile);
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let text = r.read("spec-dna.toml").replace(
            &format!("advisor = {}", profile == "strict"),
            &format!("advisor = {value}"),
        );
        r.write("spec-dna.toml", &text);
        skills_of_file(&r, "[\"brainstorm\"]");
        let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
        assert_eq!(code, 0, "{profile} {value}: {out}{err}");
        let t = table(&r);
        assert!(
            at(&t, "harness.roles.acceptor").is_none(),
            "{profile} {value}: {}",
            r.read("spec-dna.toml")
        );
        assert_eq!(advisor_lines(&r), None, "{profile} {value}");
    }
    // A model of a person keeps the entry with both values.
    let r = repo();
    let answers = answers_of("standard");
    let (code, out, err) = init_with(&r, &answers, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("spec-dna.toml").replace(
        "claude-code = \"opus\", effort = \"high\", advisor = false",
        "claude-code = \"sonnet\", effort = \"high\", advisor = true",
    );
    r.write("spec-dna.toml", &text);
    skills_of_file(&r, "[\"brainstorm\"]");
    let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = table(&r);
    assert_eq!(
        at(&t, "harness.roles.acceptor.claude-code").and_then(toml::Value::as_str),
        Some("sonnet")
    );
    assert_eq!(advisor_key(&r), Some(true));
    // A `strict` file that moves to `lite` loses the entry.
    let r = repo();
    let (code, out, err) = init_with(&r, &answers_of("strict"), &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    let (code, out, err) = init_with(&r, &answers_of("lite"), &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        at(&table(&r), "harness.roles.acceptor").is_none(),
        "{}",
        r.read("spec-dna.toml")
    );
}

/// The warning of D97: the advisor is on, and no change has a class.
const NO_CLASS_WARNING: &str = "warning: the advisor is on, and the policy `none` has no change of class `higher-risk`, so the model-team line asks for no consult; write advisor=none, or choose the policy three-class or two-class";

#[test]
fn the_advisor_with_the_policy_none_gives_a_warning() {
    let r = repo();
    let answers = "change-policy = \"none\"\nharness-models = [\"advisor=fable\"]\n";
    let (code, out, err) = init_with(&r, answers, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(NO_CLASS_WARNING), "{out}");
    let (code, out, err) = init_with(&r, answers, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(NO_CLASS_WARNING), "{out}");
    // A plain `init` on the configuration gives the warning too.
    let (code, out, err) = run(&r, &["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(NO_CLASS_WARNING), "{out}");
}

#[test]
fn no_advisor_or_a_change_policy_gives_no_warning_of_the_advisor() {
    for answers in [
        "change-policy = \"three-class\"\nharness-models = [\"advisor=fable\"]\n",
        "change-policy = \"two-class\"\nharness-models = [\"advisor=fable\"]\n",
        "change-policy = \"none\"\nharness-models = [\"advisor=none\"]\n",
        "change-policy = \"none\"\nharness = \"none\"\n",
    ] {
        let r = repo();
        let (code, out, err) = init_with(&r, answers, &[]);
        assert_eq!(code, 0, "{answers}: {out}{err}");
        assert!(!out.contains("has no change of class"), "{answers}: {out}");
        let (code, out, err) = run(&r, &["init"]);
        assert_eq!(code, 0, "{answers}: {out}{err}");
        assert!(!out.contains("has no change of class"), "{answers}: {out}");
    }
}

// ---- The advisor of a new project (design 0.8.1, D97) ----

/// The row of the role `advisor` of a new project of `lite` or `standard`.
const NO_ADVISOR_ROW: &str =
    "\nadvisor = { claude-code = \"none\", agent = \"opus\", effort = \"xhigh\" }\n";

/// The row of the role `advisor` of a new project of `strict`.
const ADVISOR_ROW: &str =
    "\nadvisor = { claude-code = \"fable\", agent = \"opus\", effort = \"xhigh\" }\n";

const FALLBACK: &str = ".claude/agents/fallback-advisor.md";

fn harness_golden(name: &str) -> String {
    let p = format!("{}/tests/golden/harness/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// The project has no row, no setting, no file and no sentence of the advisor.
fn assert_no_advisor(r: &Repo, profile: &str) {
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains(NO_ADVISOR_ROW), "{profile}:\n{cfg}");
    assert!(!r.dir.join(FALLBACK).exists(), "{profile}: {FALLBACK}");
    let settings = r.read(".claude/settings.json");
    assert!(!settings.contains("advisorModel"), "{profile}:\n{settings}");
    let claude = r.read("CLAUDE.md");
    for gone in ["advisor", "consult"] {
        assert!(!claude.contains(gone), "{profile}: {gone}\n{claude}");
    }
}

#[test]
fn a_new_standard_project_has_no_advisor() {
    let r = repo();
    let (code, out, err) = run(&r, &["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_no_advisor(&r, "standard");
    assert_eq!(
        r.read(".claude/settings.json"),
        harness_golden("settings-standard.json")
    );
    let page = harness_golden("model-team-no-advisor.md");
    assert_eq!(
        normalize_marker(&r.read(".claude/agents/model-team.md")),
        page
    );
    assert_eq!(md_lint(&page), [], "model-team-no-advisor.md");
    r.commit("adopt spec-dna");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // The same in `lite`.
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"lite\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_no_advisor(&r, "lite");
}

#[test]
fn a_new_strict_project_has_the_advisor() {
    let r = repo();
    let (code, out, err) = init_with(&r, "profile = \"strict\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains(ADVISOR_ROW), "{cfg}");
    assert!(r.dir.join(FALLBACK).exists(), "{out}");
    let settings = r.read(".claude/settings.json");
    assert!(
        settings.contains("\"advisorModel\": \"fable\""),
        "{settings}"
    );
    let claude = r.read("CLAUDE.md");
    assert!(claude.contains("Consult the advisor only"), "{claude}");
    // With the advisor, the restart line is the line of 0.8.0.
    assert!(
        out.contains(
            "\nRestart Claude Code. Then check: the header shows the main model, /advisor shows the advisor, and @ lists the agents.\n"
        ),
        "{out}"
    );
}

// ---- `lite` keeps its files (harness additions of 0.8.1) ----

/// The adapter line of 0.8.0, in `AGENTS.md` and `CLAUDE.md`. `{V}` stands for
/// the version of the binary.
const LITE_ADAPTER_LINE: &str = "If a task edits code or docs, run `spec-dna context <files>` once. Before a commit, run `spec-dna check`. Rules: test first (tdd), cause first (debug), proof before done (verify), review is data (review), isolated work (execute). Detail: `spec-dna skill show <name>`. An agent writes an approval commit only when the owner says so explicitly in the chat.<!-- spec-dna v{V} h:b9b4ed5ebfbbd534 -->";

/// The model-team lines of 0.8.0 for a new project of `lite`, by level.
const LITE_TEAM_LINES: [(&str, &str); 2] = [
    (
        "lean",
        "Model team: opus builds. `verifier` (opus) tests finished work. `Explore` (haiku) searches. Run `verifier` before you say that a task is done. A trivial task needs only the one command that proves it. Send a search that needs more than one command, and a docs search, to `Explore`. Run a one-command search yourself. Give a subagent that writes code or reviews work the model `sonnet` or `opus`, not `haiku`. For a small task that fits in this session, use no subagent. <!-- spec-dna harness v{V} h:3d5380c9b5698d17 -->",
    ),
    (
        "full",
        "Model team: opus builds. `verifier` (opus) tests finished work. `Explore` (haiku) searches. Run `verifier` before you say that a task is done. A trivial task needs only the one command that proves it. Send a search that needs more than one command, and a docs search, to `Explore`. Run a one-command search yourself. Give a code change to `implementer`. After the checks pass, give the diff to `code-simplifier`. Give a subagent that writes code or reviews work the model `sonnet` or `opus`, not `haiku`. For a small task that fits in this session, use no subagent. <!-- spec-dna harness v{V} h:025cb55ceb82cd0b -->",
    ),
];

/// The hash of the team page that 0.8.0 wrote for a new project of `lite`, by
/// level. The page of a new project has no advisor, so it is not the file
/// `model-team-0.7.1.md`.
const LITE_PAGE_HASHES: [(&str, &str); 2] =
    [("lean", "ab92f591110b9695"), ("full", "c52a50a56812fa89")];

/// The names of the entries of a folder, in order.
fn names_in(r: &Repo, dir: &str) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(r.dir.join(dir))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    out.sort();
    out
}

/// Guard: each generated file of a new project of `lite` keeps its bytes. Only
/// `spec-dna.toml` gets new lines, and this test does not read it.
#[test]
fn a_new_lite_project_keeps_the_files_of_0_8_0() {
    let version = |line: &str| line.replace("{V}", VERSION);
    for (level, team_line) in LITE_TEAM_LINES {
        let r = repo();
        let answers = format!("profile = \"lite\"\nharness = \"{level}\"\n");
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{level}: {out}{err}");
        assert_eq!(
            r.read(".claude/settings.json"),
            harness_golden("settings-lite.json"),
            "{level}"
        );
        assert_eq!(
            r.read("AGENTS.md"),
            format!("{}\n", version(LITE_ADAPTER_LINE)),
            "{level}"
        );
        assert_eq!(
            r.read("CLAUDE.md"),
            format!(
                "{}\n\n{}\n",
                version(LITE_ADAPTER_LINE),
                version(team_line)
            ),
            "{level}"
        );
        let agents: &[&str] = match level {
            "lean" => &["Explore.md", "model-team.md", "verifier.md"],
            _ => &[
                "Explore.md",
                "code-simplifier.md",
                "implementer.md",
                "model-team.md",
                "verifier.md",
            ],
        };
        assert_eq!(names_in(&r, ".claude/agents"), agents, "{level}");
        for name in agents.iter().filter(|n| **n != "model-team.md") {
            assert_eq!(
                normalize_marker(&r.read(&format!(".claude/agents/{name}"))),
                harness_golden(&format!("agents-0.7.1/{name}")),
                "{level}: {name}"
            );
        }
        let page = r.read(".claude/agents/model-team.md");
        let hash = LITE_PAGE_HASHES.iter().find(|h| h.0 == level).unwrap().1;
        assert!(
            page.ends_with(&format!(
                "\n<!-- spec-dna harness v{VERSION} h:{hash} -->\n"
            )),
            "{level}:\n{page}"
        );
        let skills: Vec<String> = names_in(&r, ".claude/skills")
            .into_iter()
            .filter(|s| s != "spec-dna")
            .collect();
        assert!(skills.len() >= 3, "{level}: {skills:?}");
        for skill in &skills {
            assert_eq!(
                normalize_marker(&r.read(&format!(".claude/skills/{skill}/SKILL.md"))),
                harness_golden(&format!("skills-0.7.1/{skill}.md")),
                "{level}: {skill}"
            );
        }
        // A clean tree for `check`.
        r.commit("adopt spec-dna");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 0, "{level}: {out}{err}");
    }
    // With the parts of the page of 0.7.1, a new project of `lite` writes that
    // page.
    let r = repo();
    let answers = format!(
        "profile = \"lite\"\nharness = \"full\"\nharness-models = [\"advisor=fable\"]\nharness-skills = [{}]\nharness-review-md = true\nharness-protect-tests = \"fix-branches\"\n",
        all_skills()
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let (code, out, err) = init_with(&r, &answers, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        normalize_marker(&r.read(".claude/agents/model-team.md")),
        harness_golden("model-team-0.7.1.md")
    );
}

/// A new project of `profile` at the level `lean`, in a commit.
fn project_of(profile: &str) -> Repo {
    let r = repo();
    let (code, out, err) = init_with(&r, &format!("profile = \"{profile}\"\n"), &[]);
    assert_eq!(code, 0, "{profile}: {out}{err}");
    r.commit("adopt spec-dna");
    r
}

/// `init --reconfigure` with the answers, then a commit. Returns the output.
fn reconfigure(r: &Repo, answers: &str) -> String {
    let (code, out, err) = init_with(r, answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{answers}: {out}{err}");
    r.commit("reconfigure");
    out
}

/// Asserts the row of the role `advisor` and the file of `fallback-advisor`.
fn assert_row(r: &Repo, row: &str, what: &str) {
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains(row), "{what}:\n{cfg}");
    assert_eq!(r.dir.join(FALLBACK).exists(), row == ADVISOR_ROW, "{what}");
}

/// D97, item 3: the role stays, and a new answer to question 1 moves only a
/// value that is the default of the old profile.
#[test]
fn reconfigure_moves_the_advisor_with_the_profile() {
    let strict = "profile = \"strict\"\n";
    let standard = "profile = \"standard\"\n";
    let lite = "profile = \"lite\"\n";
    let r = project_of("standard");
    let out = reconfigure(&r, strict);
    assert_row(&r, ADVISOR_ROW, "standard to strict");
    assert!(
        r.read(".claude/settings.json")
            .contains("\"advisorModel\": \"fable\""),
        "{out}"
    );
    assert!(
        out.contains("answer questions 4, 5, 12, 13 and 14 to change them (docs/profiles.md)"),
        "{out}"
    );
    // And back. `init` leaves the setting, as for each role that goes.
    let out = reconfigure(&r, standard);
    assert_row(&r, NO_ADVISOR_ROW, "strict to standard");
    assert!(out.contains("kept advisorModel: "), "{out}");
    // `lite` has the default of `standard`.
    let r = project_of("lite");
    reconfigure(&r, strict);
    assert_row(&r, ADVISOR_ROW, "lite to strict");
    reconfigure(&r, lite);
    assert_row(&r, NO_ADVISOR_ROW, "strict to lite");
    // A value that a person set stays.
    let r = project_of("standard");
    reconfigure(&r, "harness-models = [\"advisor=opus\"]\n");
    let opus = "\nadvisor = { claude-code = \"opus\", agent = \"opus\", effort = \"xhigh\" }\n";
    for profile in [strict, standard] {
        reconfigure(&r, profile);
        let cfg = r.read("spec-dna.toml");
        assert!(cfg.contains(opus), "{profile}:\n{cfg}");
    }
    // The answer of question 13 wins over the move.
    let r = project_of("standard");
    reconfigure(
        &r,
        "profile = \"strict\"\nharness-models = [\"advisor=none\"]\n",
    );
    assert_row(&r, NO_ADVISOR_ROW, "the answer of question 13");
    // The same profile moves nothing: `none` of `strict` stays.
    reconfigure(&r, strict);
    assert_row(&r, NO_ADVISOR_ROW, "strict to strict");
    // With no answer, each project keeps its role.
    let r = project_of("strict");
    reconfigure(&r, "");
    assert_row(&r, ADVISOR_ROW, "no answer");
}

/// Guard: under `--reconfigure`, a harness from the level `none`, or from a
/// file with no table `[harness]`, is new. It takes the preset of the profile.
#[test]
fn reconfigure_gives_a_new_harness_the_advisor_of_the_profile() {
    for (profile, row) in [("standard", NO_ADVISOR_ROW), ("strict", ADVISOR_ROW)] {
        // From the level `none`.
        let r = repo();
        let answers = format!("profile = \"{profile}\"\nharness = \"none\"\n");
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        r.commit("adopt spec-dna");
        reconfigure(&r, "harness = \"lean\"\n");
        assert_row(&r, row, &format!("{profile}: from none"));
        // From a file with no table.
        let r = repo();
        r.write(
            "spec-dna.toml",
            &format!(
                "spec-dna = \"{VERSION}\"\nprofile = \"{profile}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
            ),
        );
        r.write("docs/a.md", "# A\n\nText.\n");
        r.commit("start");
        reconfigure(&r, "harness = \"lean\"\n");
        assert_row(&r, row, &format!("{profile}: no table"));
    }
    // A new harness and a new profile in one run: the new profile decides.
    let r = repo();
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    reconfigure(&r, "profile = \"strict\"\nharness = \"lean\"\n");
    assert_row(&r, ADVISOR_ROW, "from none to strict");
}

/// Guard (D97, item 2): `upgrade` applies no preset. A project keeps its
/// advisor, also with a table that has no row `advisor`, which still reads as
/// `fable`.
#[test]
fn upgrade_keeps_the_advisor_of_a_project_of_0_8_0() {
    const ROW: &str =
        "advisor = { claude-code = \"fable\", agent = \"opus\", effort = \"xhigh\" }\n";
    for name in ["v061", "v070", "v071", "v071-lite", "v071-strict"] {
        for keep_row in [true, false] {
            let what = format!("{name}, row {keep_row}");
            let r = Repo::from_fixture(name, &[]);
            if !keep_row {
                let cfg = r.read("spec-dna.toml");
                assert_eq!(cfg.matches(ROW).count(), 1, "{what}\n{cfg}");
                r.write("spec-dna.toml", &cfg.replace(ROW, ""));
                r.commit("no row of the advisor");
            }
            let (code, out, err) = run(&r, &["upgrade"]);
            assert_eq!(code, 0, "{what}: {out}{err}");
            let cfg = r.read("spec-dna.toml");
            assert!(cfg.contains(&format!("spec-dna = \"{VERSION}\"")), "{cfg}");
            assert_eq!(cfg.contains(ROW), keep_row, "{what}\n{cfg}");
            assert_eq!(cfg.contains("advisor = {"), keep_row, "{what}\n{cfg}");
            let settings = r.read(".claude/settings.json");
            assert!(
                settings.contains("\"advisorModel\": \"fable\""),
                "{what}\n{settings}"
            );
            assert!(r.dir.join(FALLBACK).exists(), "{what}");
            let claude = r.read("CLAUDE.md");
            assert!(claude.contains("The advisor (fable)"), "{what}\n{claude}");
        }
    }
}
