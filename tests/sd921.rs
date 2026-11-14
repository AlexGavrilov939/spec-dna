//! Integration tests of SD921 (`truth-changed-without-change`). The check runs
//! only with `--base`, and only with a `[change] policy` other than `none`. It
//! fires when a requirement of a `forever` layer changed and no file of the
//! intent folder did. The helpers are small copies of the ones in
//! tests/sd202.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

const PUMP: &str = "---\ntype: requirements\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **pump-1**: The controller SHALL stop the pump at 90 percent.\n- **pump-2**: The controller SHALL log each stop, as [the log](../research/log.md) says.\n";

const INTENT: &str = "---\ntype: intent\nstatus: draft\n---\n# Pump limit\n\n## Intent\n\nText.\n";

impl Repo {
    /// A repository on the branch `main` with the base commit: the document
    /// `requirements/pump` and the intent `intent/limit`. `policy` is the
    /// value of `[change] policy`, or empty for no `[change]` table.
    fn new(profile: &str, policy: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-sd921-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let change = if policy.is_empty() {
            String::new()
        } else {
            format!("\n[change]\npolicy = \"{policy}\"\n")
        };
        let config = format!(
            "spec-dna = \"{VERSION}\"\nprofile = \"{profile}\"\n{change}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
        );
        let r = Repo { dir };
        r.write("spec-dna.toml", &config);
        r.write("docs/requirements/pump.md", PUMP);
        r.write("docs/intent/limit/intent.md", INTENT);
        for name in ["log", "log-2"] {
            r.write(
                &format!("docs/research/{name}.md"),
                "---\ntype: research\n---\n# Log\n\nText.\n",
            );
        }
        git(&r.dir, &["add", "-A"]);
        git(&r.dir, &["commit", "-q", "-m", "base"]);
        git(&r.dir, &["checkout", "-q", "-b", "feature"]);
        r
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn remove(&self, path: &str) {
        fs::remove_file(self.dir.join(path)).unwrap();
    }

    fn commit(&self) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "-m", "change"]);
    }

    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }

    /// `check --base main`.
    fn check(&self) -> (i32, String) {
        self.run(&["check", "--base", "main"])
    }
}

fn has_sd921(out: &str) -> bool {
    out.contains(" SD921 truth-changed-without-change")
}

fn edit_a_key(r: &Repo) {
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace("90 percent", "80 percent"),
    );
    r.commit();
}

#[test]
fn an_edited_key_without_an_intent_fires_in_standard_as_a_warning() {
    let r = Repo::new("standard", "three-class");
    edit_a_key(&r);
    let (code, out) = r.check();
    assert_eq!(code, 0, "a warning does not fail: {out}");
    assert!(
        out.contains("docs/requirements/pump.md:9: warning SD921 truth-changed-without-change: requirements changed `pump-1`, and no file of the intent folder changed"),
        "{out}"
    );
}

#[test]
fn an_edited_key_fires_as_an_error_in_strict_and_never_in_lite() {
    let r = Repo::new("strict", "three-class");
    edit_a_key(&r);
    let (code, out) = r.check();
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD921 "), "{out}");

    let r = Repo::new("lite", "three-class");
    edit_a_key(&r);
    let (code, out) = r.check();
    assert_eq!(code, 0, "{out}");
    assert!(!has_sd921(&out), "{out}");
}

#[test]
fn an_added_key_fires() {
    let r = Repo::new("standard", "three-class");
    r.write(
        "docs/requirements/pump.md",
        &format!("{PUMP}- **pump-3**: The controller SHALL report the level.\n"),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(
        out.contains("SD921 truth-changed-without-change: requirements added `pump-3`"),
        "{out}"
    );
}

#[test]
fn a_removed_key_fires() {
    let r = Repo::new("standard", "three-class");
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace(
            "- **pump-2**: The controller SHALL log each stop, as [the log](../research/log.md) says.\n",
            "",
        ),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(
        out.contains("SD921 truth-changed-without-change: requirements removed `pump-2`"),
        "{out}"
    );
}

#[test]
fn a_removed_document_and_a_added_document_fire() {
    let r = Repo::new("standard", "three-class");
    r.remove("docs/requirements/pump.md");
    r.write(
        "docs/requirements/valve.md",
        "---\ntype: requirements\nstatus: active\n---\n# Valve\n\n## Requirements\n\n- **valve-1**: The valve SHALL close.\n",
    );
    r.commit();
    let (_, out) = r.check();
    assert!(out.contains("requirements/valve"), "{out}");
    assert!(out.contains("requirements added `valve-1`"), "{out}");
    assert!(
        out.contains("requirements removed `pump-1`, `pump-2`"),
        "{out}"
    );
}

#[test]
fn a_changed_intent_folder_clears_it() {
    // An added intent file, a changed file (the approval), and a deleted file.
    for step in ["add", "approve", "delete"] {
        let r = Repo::new("standard", "three-class");
        match step {
            "add" => r.write("docs/intent/other/intent.md", INTENT),
            "approve" => r.write(
                "docs/intent/limit/intent.md",
                &INTENT.replace("draft", "accepted"),
            ),
            _ => r.remove("docs/intent/limit/intent.md"),
        }
        edit_a_key(&r);
        let (_, out) = r.check();
        assert!(!has_sd921(&out), "{step}: {out}");
    }
}

#[test]
fn an_uncommitted_edit_of_an_intent_file_counts_as_a_change() {
    let r = Repo::new("standard", "three-class");
    edit_a_key(&r);
    r.write(
        "docs/intent/limit/intent.md",
        &INTENT.replace("Text.", "More text."),
    );
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
}

#[test]
fn a_text_change_of_another_document_gives_nothing() {
    // The prose around the requirements is not a requirement.
    let r = Repo::new("standard", "three-class");
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace(
            "# Pump\n",
            "# Pump\n\nThe pump belongs to the irrigation line.\n",
        ),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
}

#[test]
fn a_link_rewrite_keeps_the_hash() {
    // Only the destination of a link changes: the hash of the requirement
    // does not include link destinations, as `mv` rewrites them.
    let r = Repo::new("standard", "three-class");
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace("research/log.md", "research/log-2.md"),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
    // The text of the link is part of the requirement.
    let r = Repo::new("standard", "three-class");
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace("[the log]", "[the journal]"),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(has_sd921(&out), "{out}");
}

#[test]
fn a_document_move_with_mv_keeps_the_hashes() {
    let r = Repo::new("standard", "three-class");
    let (code, out) = r.run(&["mv", "requirements/pump", "requirements/pump-station"]);
    assert_eq!(code, 0, "{out}");
    r.commit();
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
    assert!(
        r.dir.join("docs/requirements/pump-station.md").exists(),
        "mv moved the file"
    );
}

#[test]
fn moving_a_key_to_a_new_document_with_mv_keeps_the_hash() {
    let r = Repo::new("standard", "three-class");
    let (code, out) = r.run(&["mv", "requirements/pump#pump-2", "requirements/pump-log"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        r.dir.join("docs/requirements/pump-log.md").exists(),
        "{out}"
    );
    r.commit();
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
}

#[test]
fn moving_a_section_with_mv_keeps_the_hashes() {
    // A split: the section with both keys goes to a new document.
    let r = Repo::new("standard", "three-class");
    let (code, out) = r.run(&[
        "mv",
        "requirements/pump#requirements",
        "requirements/pump-body",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(
        r.dir.join("docs/requirements/pump-body.md").exists(),
        "{out}"
    );
    r.commit();
    let (_, out) = r.check();
    assert!(!has_sd921(&out), "{out}");
    // A later edit of a moved key still gives SD921.
    let body = fs::read_to_string(r.dir.join("docs/requirements/pump-body.md")).unwrap();
    r.write(
        "docs/requirements/pump-body.md",
        &body.replace("90 percent", "80 percent"),
    );
    r.commit();
    let (_, out) = r.check();
    assert!(
        out.contains("requirements/pump-body") && out.contains("requirements changed `pump-1`"),
        "{out}"
    );
}

#[test]
fn policy_none_and_no_table_give_nothing() {
    for policy in ["none", ""] {
        let r = Repo::new("strict", policy);
        edit_a_key(&r);
        let (code, out) = r.check();
        assert_eq!(code, 0, "{policy}: {out}");
        assert!(!has_sd921(&out), "{policy}: {out}");
    }
}

#[test]
fn two_class_has_the_same_trigger() {
    let r = Repo::new("standard", "two-class");
    edit_a_key(&r);
    let (_, out) = r.check();
    assert!(has_sd921(&out), "{out}");
}

#[test]
fn without_base_it_does_not_run() {
    let r = Repo::new("strict", "three-class");
    edit_a_key(&r);
    // The working tree equals HEAD: no diagnostic is new.
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!has_sd921(&out), "{out}");
    let (_, out) = r.run(&["check", "--all"]);
    assert!(!has_sd921(&out), "{out}");
    // An uncommitted edit also gives nothing.
    r.write(
        "docs/requirements/pump.md",
        &PUMP.replace("90 percent", "70 percent"),
    );
    let (_, out) = r.run(&["check", "--all"]);
    assert!(!has_sd921(&out), "{out}");
}

#[test]
fn an_unchanged_requirement_gives_nothing_in_each_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = Repo::new(profile, "three-class");
        r.write(
            "docs/notes.md",
            "---\ntype: research\n---\n# Notes\n\nText.\n",
        );
        r.commit();
        let (code, out) = r.check();
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(!has_sd921(&out), "{profile}: {out}");
    }
}

#[test]
fn the_json_has_the_new_name_and_the_diagnostic_is_new() {
    let r = Repo::new("standard", "three-class");
    edit_a_key(&r);
    let (_, out) = r.run(&["check", "--base", "main", "--json"]);
    assert!(
        out.contains("\"code\":\"SD921\",\"name\":\"truth-changed-without-change\",\"severity\":\"warning\",\"new\":true"),
        "{out}"
    );
}
