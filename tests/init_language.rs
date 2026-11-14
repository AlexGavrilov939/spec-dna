//! Integration tests of question 7 of `spec-dna init`: the key `language`
//! (design 0.6.0, "Init questions"), the language that detection proposes,
//! and `--reconfigure`, which keeps `language`, `[change]` and `[rules] lint`.
//! The helpers are small copies of the ones in tests/init_graph.rs. The
//! neutral domains are irrigation and library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
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
    fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-language-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        Repo { dir }
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        self.dir.join(path).exists()
    }

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// Runs `init` with an answers file. The harness is off unless the
    /// answers say otherwise, so that the output is short.
    fn init_with(&self, answers: &str, args: &[&str]) -> (i32, String, String) {
        let file = PathBuf::from(format!("{}-answers.toml", self.dir.display()));
        let text = if answers.contains("harness") {
            answers.to_string()
        } else {
            format!("harness = \"none\"\n{answers}")
        };
        fs::write(&file, text).unwrap();
        let mut a = vec!["init", "--answers", file.to_str().unwrap()];
        a.extend_from_slice(args);
        let out = self.run(&a);
        let _ = fs::remove_file(&file);
        out
    }
}

/// The text of `spec-dna.toml` before its first table, for the top keys.
fn top_keys(cfg: &str) -> &str {
    cfg.split("\n[").next().unwrap()
}

/// A decision of a project that writes in German.
fn german(r: &Repo) {
    for i in 1..=3 {
        r.write(
            &format!("docs/adr/ADR-{i:03}-ventil-{i}.md"),
            &format!(
                "---\nid: ADR-{i:03}\ntype: decision\nstatus: accepted\n---\n\n# Ventil {i}\n\n## Kontext\n\nDas Ventil {i} bleibt often.\n\n## Entscheidung\n\nDie Steuerung oeffnet das Ventil.\n\n## Nachtrag\n\nKeiner.\n"
            ),
        );
    }
    r.commit("german decisions");
}

#[test]
fn the_answer_writes_the_key_language_and_no_lint_rule() {
    let r = Repo::empty();
    // The preview names the key.
    let (code, out, err) = r.init_with("language = \"de\"\n", &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("-> language = \"de\""), "{out}");
    let (code, out, err) = r.init_with("language = \"de\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        top_keys(&cfg).contains("\nlanguage = \"de\""),
        "language is a top key:\n{cfg}"
    );
    assert!(!cfg.contains("lint"), "no lint rule:\n{cfg}");
    assert!(!cfg.contains("[rules]"), "{cfg}");
    r.commit("init");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_answer_en_writes_nothing() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("language = \"en\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("language"), "{cfg}");
    assert!(!cfg.contains("lint"), "{cfg}");
}

#[test]
fn a_language_tag_with_a_region_stays_as_written() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("language = \"pt-BR\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("\nlanguage = \"pt-BR\""));
}

#[test]
fn a_bad_tag_stops_init_and_writes_nothing() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("language = \"German\"\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("BCP 47"), "{err}");
    assert!(err.contains("`German`"), "{err}");
    assert!(!r.exists("spec-dna.toml"));
}

#[test]
fn a_non_english_corpus_gets_its_language_and_check_passes() {
    let r = Repo::empty();
    german(&r);
    // The detected language is the default of question 7.
    let (code, out, err) = r.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("7. normative language and heading aliases: de; outcome: Entscheidung; amendments: Nachtrag"),
        "{out}"
    );
    let (code, out, err) = r.init_with("", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(top_keys(&cfg).contains("\nlanguage = \"de\""), "{cfg}");
    assert!(!cfg.contains("lint"), "{cfg}");
    r.commit("init");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn an_english_corpus_proposes_en() {
    let r = Repo::empty();
    for i in 1..=3 {
        r.write(
            &format!("docs/adr/ADR-{i:03}-valve-{i}.md"),
            &format!(
                "---\nid: ADR-{i:03}\ntype: decision\nstatus: accepted\n---\n\n# Valve {i}\n\n## Context\n\nValve {i} stays open.\n\n## Decision Outcome\n\nThe controller opens the valve.\n"
            ),
        );
    }
    r.commit("english decisions");
    let (code, out, err) = r.init_with("", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains("language"));
}

#[test]
fn headings_in_one_other_script_give_the_language_of_that_script() {
    let r = Repo::empty();
    for i in 1..=3 {
        r.write(
            &format!("docs/specs/zone-{i}.md"),
            &format!("# Λειτουργία {i}\n\n## Απαιτήσεις\n\n- **open-{i}**: The controller SHALL open valve {i}.\n"),
        );
    }
    r.commit("greek headings");
    let (code, out, err) = r.init_with("", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("\nlanguage = \"el\""));
}

#[test]
fn reconfigure_keeps_language_the_change_table_and_the_lint_rule() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("language = \"de\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    // A project edits the file by hand: a rule and the change policy.
    let mut cfg = r.read("spec-dna.toml");
    cfg.push_str("\n[rules]\nlint = \"warning\"\n\n[change]\npolicy = \"two-class\"\n");
    r.write("spec-dna.toml", &cfg);
    r.commit("init and edits");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(top_keys(&cfg).contains("\nlanguage = \"de\""), "{cfg}");
    assert!(cfg.contains("[change]\npolicy = \"two-class\"\n"), "{cfg}");
    assert!(cfg.contains("lint = \"warning\""), "{cfg}");
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // The answer `en` removes the key. The table and the rule stay.
    let (code, out, err) = r.init_with("language = \"en\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("language"), "{cfg}");
    assert!(cfg.contains("[change]\npolicy = \"two-class\"\n"), "{cfg}");
    assert!(cfg.contains("lint = \"warning\""), "{cfg}");
}

#[test]
fn a_new_project_has_no_change_table() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains("[change]"));
}
