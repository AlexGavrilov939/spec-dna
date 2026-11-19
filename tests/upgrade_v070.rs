//! End-to-end tests of `spec-dna upgrade` on the fixture of 0.7.0 (0.7.1,
//! part B, task t6). The fixture `tests/fixtures/upgrade/v070/` is a project
//! that the real binary 0.7.0 wrote: the profile `standard`, the harness
//! `lean`, a change with an approved spec and an approved plan (its tasks have
//! `Hours:`), a decision, a roadmap, a questions file, and a pre-commit file
//! with the hook `typos`. `tests/fixtures/upgrade/make.sh` records how it was
//! made. A test here runs no old binary.
//!
//! The pin of the fixture is 0.7.0. The tree binary has the version 0.7.0 until
//! the release sets 0.7.1, so no pin exists that is before the binary and
//! after 0.7.0. `pin_before_binary` gives a stand-in: the pin `0.6.9`, while
//! the binary is 0.7.0. After the release, it gives the pin 0.7.0, so the
//! tests then move the pin of the fixture from 0.7.0 to the new version. The
//! tests assert structure and not a list of paths: after the release, each
//! marker of a generated file moves.
//!
//! The last test runs `upgrade` twice on a copy of this repository.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The skills that 0.7.0 added. A project of 0.6 had no way to choose them.
const ADDED_NAMES: [&str; 6] = ["execute", "tdd", "verify", "finish", "debug", "post-mortem"];
/// The pin that the fixture records.
const RECORDED_PIN: &str = "0.7.0";
/// The pin of the stand-in, while the binary is not later than 0.7.0.
const STAND_IN_PIN: &str = "0.6.9";
/// The line of `_typos.toml` that the note of `upgrade` names.
const TYPOS_GUARD: &str = "[default]\nextend-ignore-re = ['(?:h:|@)[0-9a-f]{16}\\b']\n";

fn parse_version(text: &str) -> (u64, u64, u64) {
    let mut parts = text.split('.').map(|p| p.parse::<u64>().unwrap());
    (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    )
}

/// True if the binary is later than the release of the fixture.
fn binary_is_later() -> bool {
    parse_version(VERSION) > parse_version(RECORDED_PIN)
}

/// A pin that is before the binary. The pin of the fixture itself when the
/// binary is later than 0.7.0, and a stand-in before it if not.
fn pin_before_binary() -> &'static str {
    if binary_is_later() {
        RECORDED_PIN
    } else {
        STAND_IN_PIN
    }
}

/// Sets the pin line of `spec-dna.toml` (the first line that starts with
/// `spec-dna = "`) and returns true if it changed the text.
fn set_pin(repo: &Repo, pin: &str) -> bool {
    let text = repo.read("spec-dna.toml");
    let mut done = false;
    let new: String = text
        .split_inclusive('\n')
        .map(|line| {
            if !done && line.starts_with("spec-dna = \"") {
                done = true;
                format!("spec-dna = \"{pin}\"\n")
            } else {
                line.to_string()
            }
        })
        .collect();
    assert!(done, "no pin line in spec-dna.toml");
    repo.write("spec-dna.toml", &new);
    new != text
}

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

/// The fixture on `main`, with a pin that is before the binary.
fn project() -> Repo {
    let repo = Repo::from_fixture("v070", &[]);
    let changed = set_pin(&repo, pin_before_binary());
    // The stand-in changes the text. The recorded pin changes nothing.
    assert_eq!(changed, !binary_is_later());
    if changed {
        repo.commit("pin");
    }
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
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

/// The default skills of this build, from `skill list --json`, run in a
/// folder with no `spec-dna.toml`. `on` is the state to select.
fn skills_by_default(default: &str) -> Vec<String> {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    out.split("{\"name\":\"")
        .skip(1)
        .filter(|item| item.contains(&format!("\"default\":\"{default}\"")))
        .map(|item| item.split('"').next().unwrap().to_string())
        .collect()
}

fn skill_file(name: &str) -> String {
    format!(".claude/skills/spec-dna-{name}/SKILL.md")
}

/// The paths that git reports as changed or new, in a working tree.
fn changed_paths(repo: &Repo) -> BTreeSet<String> {
    repo.git_out(&["status", "--porcelain", "-uall"])
        .lines()
        .map(|l| l[3..].to_string())
        .collect()
}

/// True if `path` is a file that `upgrade` writes: the configuration, an
/// adapter, a harness file or a skill file.
fn is_generated(path: &str) -> bool {
    path == "spec-dna.toml"
        || path == "AGENTS.md"
        || path == "CLAUDE.md"
        || path == "REVIEW.md"
        || path.starts_with(".claude/")
}

/// The paths after `would write`, `wrote`, `would remove` or `removed`.
fn written_paths(out: &str) -> Vec<String> {
    out.lines()
        .filter_map(|l| {
            ["would write ", "would remove ", "wrote ", "removed "]
                .iter()
                .find_map(|verb| l.strip_prefix(verb))
        })
        .map(str::to_string)
        .collect()
}

/// The lines of the diff of `spec-dna.toml` that add or remove text.
fn config_diff_lines(out: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in out.lines() {
        if line.starts_with("would write ") || line.starts_with("wrote ") {
            inside = line.ends_with(" spec-dna.toml");
        } else if line.starts_with("proof:") {
            inside = false;
        } else if inside
            && (line.starts_with('-') || line.starts_with('+'))
            && !line.starts_with("---")
            && !line.starts_with("+++")
        {
            lines.push(line.to_string());
        }
    }
    lines
}

#[test]
fn the_dry_run_names_the_pin_move_and_no_other_edit_of_the_configuration() {
    let repo = project();
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = run(&repo, &["upgrade", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // The diff of the configuration is the pin line.
    assert_eq!(
        config_diff_lines(&out),
        [
            format!("-spec-dna = \"{}\"", pin_before_binary()),
            format!("+spec-dna = \"{VERSION}\""),
        ],
        "{out}"
    );
    // The report names no edit beyond the pin, and the line `next:` says
    // that only the pin changes.
    assert!(
        !out.contains("edits of spec-dna.toml beyond the pin"),
        "{out}"
    );
    let next = out
        .lines()
        .find(|l| l.starts_with("next:"))
        .unwrap_or_else(|| panic!("no next line: {out}"));
    assert!(
        next.contains("Only the pin of spec-dna.toml changes"),
        "{next}"
    );
    // The other paths are generated files. No document, script or test is
    // among them.
    let paths = written_paths(&out);
    assert!(paths.contains(&"spec-dna.toml".to_string()), "{out}");
    for path in &paths {
        assert!(is_generated(path), "{path} is no generated file: {out}");
    }
    // A dry run writes nothing.
    assert!(out.contains("dry run; nothing written"), "{out}");
    assert_eq!(before, tree_bytes(&repo.dir));
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
}

#[test]
fn the_upgraded_project_passes_check_and_the_documents_stay() {
    let repo = project();
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        table(&repo)["spec-dna"].as_str(),
        Some(VERSION),
        "the pin is the version of the binary"
    );
    // Only generated files changed: no document, script or test.
    for path in changed_paths(&repo) {
        assert!(is_generated(&path), "{path} changed: {out}");
    }
    let (code, out, err) = run(&repo, &["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("0 errors"), "{out}");
    // The check finds the 7 documents of the fixture.
    assert!(out.contains("7 documents"), "{out}");
}

#[test]
fn a_second_upgrade_changes_nothing() {
    let repo = project();
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    let before = tree_bytes(&repo.dir);
    for args in [&["upgrade"][..], &["upgrade", "--dry-run"]] {
        let (code, out, err) = run(&repo, args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(out.contains("nothing to change"), "{args:?}: {out}");
        assert!(written_paths(&out).is_empty(), "{args:?}: {out}");
    }
    assert_eq!(before, tree_bytes(&repo.dir), "the files stay");
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
    // `init` also changes nothing in the upgraded project.
    let (code, out, err) = run(&repo, &["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        repo.git_out(&["status", "--porcelain"]),
        "",
        "init changed a file: {out}"
    );
}

#[test]
fn the_skills_of_the_project_stay() {
    let repo = project();
    let list = skills_of(&repo);
    let files_before: BTreeSet<String> = repo
        .git_out(&["ls-files", ".claude/skills"])
        .lines()
        .map(str::to_string)
        .collect();
    // The fixture has the skills that `init` of 0.7.0 chose: each skill that
    // is on by default for the profile and the level is in the list. The
    // test reads the default from this build, and names no skill.
    let on = skills_by_default("on");
    let off = skills_by_default("off");
    assert!(!list.is_empty());
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(skills_of(&repo), list, "the list `skills` stays");
    // No skill file is added or removed, and the report names no rename.
    let kinds = repo.git_out(&["status", "--porcelain", "-uall", ".claude/skills"]);
    for line in kinds.lines() {
        assert!(line.starts_with(" M"), "a skill file moved: {line}");
    }
    let files_after: BTreeSet<String> = repo
        .git_out(&["ls-files", ".claude/skills"])
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(files_before, files_after);
    for name in &list {
        assert!(files_after.contains(&skill_file(name)), "{name}");
    }
    assert!(!out.contains("is off by default in 0.7.0"), "{out}");
    assert!(!out.contains("`skills` of `[harness]`"), "{out}");
    // Each skill that is on by default and that the list has, stays on.
    for name in on.iter().filter(|n| list.contains(n)) {
        assert!(skills_of(&repo).contains(name), "{name}");
    }
    // A skill that is off by default and that the list has, stays too.
    for name in off.iter().filter(|n| list.contains(n)) {
        assert!(skills_of(&repo).contains(name), "{name}");
    }
}

/// A person turned a skill that is on by default off, and another skill that
/// is off by default on. The upgrade keeps both choices. This case needs the
/// recorded pin as the pin of the project: with a pin before 0.7.0, the rule
/// of 0.6 adds each default skill that the list lacks. So it runs only when
/// the binary is later than 0.7.0.
#[test]
fn a_skill_that_a_person_turned_off_or_on_stays() {
    if !binary_is_later() {
        eprintln!("skipped: the binary is {VERSION}, so the pin of 0.7.0 is not before it");
        eprintln!("the test runs when the release sets a later version");
        return;
    }
    let repo = project();
    let on = skills_by_default("on");
    let off = skills_by_default("off");
    let list = skills_of(&repo);
    // A skill that 0.7.0 added: the rule of 0.6 would add it again.
    let dropped = on
        .iter()
        .find(|n| list.contains(n) && ADDED_NAMES.contains(&n.as_str()))
        .expect("a new skill, on by default, in the list")
        .clone();
    let added = off
        .iter()
        .find(|n| !list.contains(n))
        .expect("a skill off by default, not in the list")
        .clone();
    let mut new_list: Vec<String> = list.iter().filter(|n| **n != dropped).cloned().collect();
    new_list.push(added.clone());
    let text = repo.read("spec-dna.toml");
    let old_line = format!(
        "skills = [{}]",
        list.iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(text.contains(&old_line), "the list of the fixture");
    let new_line = format!(
        "skills = [{}]",
        new_list
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    repo.write("spec-dna.toml", &text.replace(&old_line, &new_line));
    remove_skill_file(&repo, &dropped);
    repo.commit("a person chose the skills");
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(skills_of(&repo), new_list, "{out}");
    assert!(
        !repo.dir.join(skill_file(&dropped)).exists(),
        "{dropped} came back"
    );
    assert!(
        repo.dir.join(skill_file(&added)).exists(),
        "{added} has no file"
    );
}

fn remove_skill_file(repo: &Repo, name: &str) {
    fs::remove_file(repo.dir.join(skill_file(name))).unwrap();
}

#[test]
fn the_typos_note_prints_and_a_setting_for_the_hashes_ends_it() {
    let repo = project();
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    let note: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("note: the pre-commit hook `typos`"))
        .collect();
    assert_eq!(note.len(), 1, "{out}");
    assert!(
        note[0].contains("docs/init.md#spell-checkers"),
        "{}",
        note[0]
    );
    // The same project with the setting of `typos` gets no note.
    let repo = project();
    repo.write("_typos.toml", TYPOS_GUARD);
    repo.commit("guard the hashes");
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("note: the pre-commit hook"), "{out}");
}

/// The branch `up` with the commit of the upgrade and a change of the script
/// in the same commit. `extra` is more work, as `(path, text)`.
fn upgrade_with_work(extra: &[(&str, &str)]) -> Repo {
    let repo = project();
    repo.git(&["switch", "-q", "-c", "up"]);
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    let script = repo.read("src/schedule.sh");
    repo.write("src/schedule.sh", &format!("{script}# a comment\n"));
    for (path, text) in extra {
        repo.write(path, text);
    }
    repo.commit("upgrade and work");
    repo
}

/// The lines of `check --base main` that report `code`. The line `<code> fix:`
/// that follows a report is not one.
fn gate_lines(repo: &Repo, code: &str) -> Vec<String> {
    let (_, out, err) = run(repo, &["check", "--base", "main"]);
    assert!(out.contains("documents"), "{out}{err}");
    out.lines()
        .filter(|l| l.contains(code) && !l.starts_with(&format!("{code} fix:")))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_upgrade_and_a_code_change_in_one_commit_give_no_sd503_and_no_sd913() {
    let repo = upgrade_with_work(&[]);
    // The commit holds the written files and the script.
    let paths: BTreeSet<String> = repo
        .git_out(&["diff", "--name-only", "main", "HEAD"])
        .lines()
        .map(str::to_string)
        .collect();
    assert!(paths.contains("src/schedule.sh"));
    assert!(paths.contains("spec-dna.toml"));
    assert!(paths.iter().any(|p| p.starts_with(".claude/")), "{paths:?}");
    let (code, out, err) = run(&repo, &["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(gate_lines(&repo, "SD503"), Vec::<String>::new());
    assert_eq!(gate_lines(&repo, "SD913"), Vec::<String>::new());
}

/// The negative cases of the test above: they show that the same setup gives
/// SD503 and SD913 when the commit does hold what they report.
#[test]
fn the_same_setup_gives_sd503_for_another_edit_and_sd913_for_a_stray_file() {
    // Another edit of the configuration, with the pin.
    let repo = project();
    repo.git(&["switch", "-q", "-c", "up"]);
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = repo.read("spec-dna.toml");
    assert!(text.contains("max-fanout = 8"));
    repo.write(
        "spec-dna.toml",
        &text.replace("max-fanout = 8", "max-fanout = 9"),
    );
    repo.write("src/schedule.sh", "#!/usr/bin/env bash\necho changed\n");
    repo.commit("upgrade, another edit and work");
    assert!(
        !gate_lines(&repo, "SD503").is_empty(),
        "another edit of spec-dna.toml must give SD503"
    );
    // A file that is outside the plan and that is not generated.
    let repo = upgrade_with_work(&[("notes.txt", "A note.\n")]);
    let lines = gate_lines(&repo, "SD913");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("notes.txt"), "{lines:?}");
}

// --- This repository ---

/// Copies each file that git tracks in `from` into `to`, with its mode. The
/// git call ignores the configuration of the machine.
fn copy_tracked(from: &Path, to: &Path) {
    let list = git_out(from, &["ls-files", "-z"]);
    for path in list.split('\0').filter(|p| !p.is_empty()) {
        let src = from.join(path);
        // A file that the working tree has deleted is not copied.
        if !src.is_file() {
            continue;
        }
        let dst = to.join(path);
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        fs::copy(&src, &dst).unwrap();
    }
}

/// The proof of the plan of 0.7.1 for this repository: a copy of its tracked
/// files, with a pin before the binary, gets one upgrade, and the second
/// upgrade changes nothing. The pin of the copy is the pin before the binary,
/// so the proof stays meaningful after the version is 0.7.1: the pin is 0.7.0
/// then, and the upgrade moves it to the version of the binary.
#[test]
fn this_repository_gets_no_change_at_the_second_upgrade() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !root.join(".git").exists() {
        eprintln!("skipped: {} has no git repository", root.display());
        return;
    }
    let repo = Repo::new();
    copy_tracked(root, &repo.dir);
    assert!(repo.dir.join("spec-dna.toml").is_file());
    let pin = pin_before_binary();
    assert!(set_pin(&repo, pin), "the pin of the copy moves");
    repo.commit("copy of the repository");
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("proof: ok"), "{out}");
    assert_eq!(
        table(&repo)["spec-dna"].as_str(),
        Some(VERSION),
        "the first upgrade moves the pin: {out}"
    );
    repo.commit("upgrade");
    let (code, out, err) = run(&repo, &["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "", "{out}");
}
