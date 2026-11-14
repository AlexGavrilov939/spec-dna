//! Integration tests of the agent harness (ADR 0004): `check` on the harness
//! files. The helpers are small copies of the ones in tests/cli.rs.

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

impl Repo {
    /// A repository with one document and the given harness table.
    fn with_harness(harness: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-harness-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let r = Repo { dir };
        r.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n\n{harness}"),
        );
        r.write("docs/a.md", "# A\n\nText.\n");
        r.commit("start");
        r
    }

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
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
}

#[test]
fn check_reports_each_missing_harness_file() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    for file in [
        ".claude/agents/fallback-advisor.md",
        ".claude/agents/checker.md",
        ".claude/agents/Explore.md",
        ".claude/agents/model-team.md",
    ] {
        assert!(
            out.contains(&format!("{file}:1: error SD504 harness-missing")),
            "{file}: {out}"
        );
    }
    assert!(
        out.contains("CLAUDE.md:1: error SD504 harness-missing"),
        "{out}"
    );
    assert!(out.contains("SD504 fix: Run `spec-dna init`"), "{out}");
    // Without `[harness]`, with level none, and with the rule off: no finding.
    for harness in [
        "",
        "[harness]\nlevel = \"none\"\n",
        "[harness]\nlevel = \"lean\"\n[rules]\nSD504 = \"off\"\n",
    ] {
        let r = Repo::with_harness(harness);
        let (code, out) = r.run(&["check"]);
        assert!(!out.contains("SD504"), "{harness}: {code} {out}");
    }
}

#[test]
fn check_reads_untracked_files_and_reports_a_file_that_spec_dna_did_not_write() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    r.write(".claude/agents/checker.md", "my own agent\n");
    let (_, out) = r.run(&["check"]);
    assert!(
        out.contains("checker.md:1: error SD504 harness-missing: the file has no harness marker"),
        "{out}"
    );
    assert!(
        !out.contains("checker.md:1: error SD504 harness-missing: the generated file is absent")
    );
    // The gate against a base reads tracked files only.
    let (_, out) = r.run(&["check", "--base", "HEAD"]);
    assert!(
        out.contains("checker.md:1: error SD504 harness-missing: the generated file is absent"),
        "{out}"
    );
}

#[test]
fn check_reports_a_settings_file_that_is_not_json() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    r.write(".claude/settings.json", "{\"model\": ");
    let (code, out) = r.run(&["check", "--json"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("\"code\":\"SD505\",\"name\":\"harness-settings-invalid\""),
        "{out}"
    );
    assert!(!out.contains("\"code\":\"SD502\""), "{out}");
    r.write(".claude/settings.json", "{}");
    let (_, out) = r.run(&["check", "--json"]);
    assert!(!out.contains("SD505"), "{out}");
    assert!(out.contains("\"target\":\"advisorModel\""), "{out}");
}

#[test]
fn harness_files_do_not_make_a_configuration_change_impure() {
    let r = Repo::with_harness("");
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n"
        ),
    );
    r.write(".claude/settings.json", "{}\n");
    r.write(".claude/agents/checker.md", "text\n");
    r.write("CLAUDE.md", "# Rules\n");
    let (_, out) = r.run(&["check"]);
    assert!(!out.contains("SD503"), "{out}");
    r.write("src/new.py", "x = 1\n");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error SD503 config-not-alone: the change edits spec-dna.toml and 1 other file (first: `src/new.py`)"),
        "{out}"
    );
}
