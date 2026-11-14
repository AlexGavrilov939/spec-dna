//! Integration tests of the fixes after the review of step 1 of 0.6.0:
//! `history` in a new project, a document with no type in `layer list`, the
//! checks of SD303, the line endings of `layer add`, and the entries of
//! `files`. The helpers are small copies of the ones in tests/new_place.rs.

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
    /// An empty repository.
    fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-rev-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        Repo { dir }
    }

    /// A repository with a configuration: the head (keys and tables before
    /// the roots) and the text of the roots.
    fn with(head: &str, roots: &str) -> Repo {
        let r = Repo::empty();
        r.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{VERSION}\"\n{head}{roots}"),
        );
        r
    }

    /// One mixed root `docs`, with the implied graph.
    fn mixed() -> Repo {
        Repo::with("", "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n")
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

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        git(&self.dir, &["add", "-A"]);
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
}

const DECISION: &str = "---\nstatus: accepted\n---\n# Use y\n\n## Context and Problem Statement\n\nThe team needs a store.\n\n## Considered Options\n\n- Store y.\n- Store z.\n\n## Decision Outcome\n\nChosen option: store y, because it is small.\n\n## Status\n\nDone.\n\n## Amendments\n\n### Valve timeout\n\nThe controller MUST close a valve after 30 minutes.\n";

#[test]
fn history_in_a_new_project_names_the_command_that_adds_the_layer() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    // The small graph has no `research` layer.
    assert!(!r.read("spec-dna.toml").contains("[layers.research]"));
    r.write("docs/decisions/use-y.md", DECISION);
    let before = r.read("docs/decisions/use-y.md");
    let (code, out, err) = r.run(&["history", "decisions/use-y"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("spec-dna layer add research --types research"),
        "{err}"
    );
    assert!(!err.contains("SD303 for"), "{err}");
    // Nothing changed.
    assert_eq!(r.read("docs/decisions/use-y.md"), before);
    assert!(!r.exists("docs/decisions/use-y-history.md"));
    // Run the command of the message. `init` updates the skill.
    let (code, out, err) = r.run(&["layer", "add", "research", "--types", "research"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["history", "decisions/use-y"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists("docs/decisions/use-y-history.md"));
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn mv_to_a_type_with_no_layer_stops_before_the_proof() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write("docs/decisions/use-y.md", DECISION);
    let (code, out, err) = r.run(&["mv", "decisions/use-y#valve-timeout", "notes-y"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("spec-dna layer add research --types research"),
        "{err}"
    );
    assert!(!r.exists("docs/notes-y.md"));
}

#[test]
fn layer_list_places_no_document_that_has_no_type() {
    let r = Repo::mixed();
    r.write("docs/p2/q.md", "# Q\n\nText.\n");
    r.write("docs/requirements/loans.md", "# Loans\n\nText.\n");
    let (code, out, err) = r.run(&["layer", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("{\"path\":\"docs/p2/q.md\",\"id\":\"p2/q\",\"type\":\"research\",\"layer\":null,\"rule\":null}"),
        "{out}"
    );
    assert!(
        out.contains("\"path\":\"docs/requirements/loans.md\""),
        "{out}"
    );
    assert!(out.ends_with("\"unplaced\":1}\n"), "{out}");
    let (code, out, _) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 document"), "{out}");
}

#[test]
fn a_type_that_sd303_rejects_gives_no_check_of_another_type() {
    let r = Repo::mixed();
    // `spec` is valid only as `intent/<slug>/spec.md`. The place of the
    // file is a `plan`, which needs a `Files` section.
    r.write(
        "docs/intent/add-x/plan.md",
        "---\ntype: spec\nstatus: draft\n---\n# Plan\n\nText.\n",
    );
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("SD303"), "{out}");
    assert!(!out.contains("SD305"), "{out}");
    assert!(!out.contains("needs a section"), "{out}");
    // A type with a status that the fallback does not have: SD303 only.
    r.write(
        "docs/intent/add-y/plan.md",
        "---\ntype: spec\nstatus: accepted\n---\n# Plan\n\nText.\n",
    );
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD304"), "{out}");
    assert!(!out.contains("SD305"), "{out}");
    // A document with no type gives SD303 only.
    r.write("docs/p2/q.md", "# Q\n\nText.\n");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(out.contains("docs/p2/q.md:1: error SD303"), "{out}");
    assert!(!out.contains("docs/p2/q.md:1: error SD304"), "{out}");
    assert!(!out.contains("docs/p2/q.md:1: error SD305"), "{out}");
    assert!(out.contains("3 errors"), "{out}");
}

/// True if each `\n` of a text follows a `\r`.
fn all_crlf(text: &str) -> bool {
    text.matches('\n').count() == text.matches("\r\n").count()
}

#[test]
fn layer_add_keeps_crlf_in_a_file_with_a_layers_table() {
    let r = Repo::mixed();
    let (code, out, err) = r.run(&["layer", "add", "refs", "--paths", "docs/refs/**"]);
    assert_eq!(code, 0, "{out}{err}");
    // The file now has the `[layers]` tables. Make all line endings CRLF.
    let crlf = r.read("spec-dna.toml").replace('\n', "\r\n");
    r.write("spec-dna.toml", &crlf);
    // The first form of the edit: a new `moves-to` line below a header
    // (`intent` has the list, `refs` has none), then a new table.
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "more",
        "--paths",
        "docs/more/**",
        "--moved-from",
        "intent,refs",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("spec-dna.toml");
    assert!(text.contains("[layers.more]\r\n"), "{text:?}");
    assert!(text.contains("moves-to = [\"more\"]\r\n"), "{text:?}");
    assert!(all_crlf(&text), "{text:?}");
}

#[test]
fn layer_add_keeps_crlf_in_a_file_with_no_layers_table() {
    let r = Repo::mixed();
    let head = r.read("spec-dna.toml").replace('\n', "\r\n");
    r.write("spec-dna.toml", &head);
    let (code, out, err) = r.run(&["layer", "add", "refs", "--paths", "docs/refs/**"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("spec-dna.toml");
    assert!(text.contains("[layers.refs]\r\n"), "{text:?}");
    assert!(all_crlf(&text), "{text:?}");
    // A file with LF stays LF.
    let r = Repo::mixed();
    let (code, out, err) = r.run(&["layer", "add", "refs", "--paths", "docs/refs/**"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains('\r'));
}

#[test]
fn layer_add_refuses_a_bad_entry_of_files() {
    let r = Repo::mixed();
    let before = r.read("spec-dna.toml");
    for bad in [
        "../../etc/passwd.md",
        "/etc/passwd.md",
        "a\\b.md",
        "c:x.md",
        "a/../b.md",
        "a//b.md",
    ] {
        let (code, out, err) = r.run(&[
            "layer",
            "add",
            "refs",
            "--types",
            "interface",
            "--files",
            bad,
        ]);
        assert_eq!(code, 2, "{bad}: {out}{err}");
        assert!(err.contains("--files"), "{bad}: {err}");
        assert_eq!(r.read("spec-dna.toml"), before, "{bad}");
    }
    // `foundation` has `terms.md` already.
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "refs",
        "--paths",
        "docs/refs/**",
        "--files",
        "terms.md",
    ]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("foundation"), "{err}");
    assert_eq!(r.read("spec-dna.toml"), before);
    // A good entry works.
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "refs",
        "--paths",
        "docs/refs/**",
        "--files",
        "refs.md",
    ]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_configuration_refuses_a_bad_entry_of_files_and_a_file_of_two_layers() {
    for bad in ["../x.md", "/x.md", "a\\\\b.md", "c:x.md"] {
        let r = Repo::with(
            &format!("[layers.refs]\ntypes = [\"research\"]\nfiles = [\"{bad}\"]\n"),
            "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
        );
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 3, "{bad}: {out}{err}");
        assert!(err.contains("layers.refs.files"), "{bad}: {err}");
    }
    let r = Repo::with(
        "[layers.a]\ntypes = [\"terms\"]\nfiles = [\"terms.md\"]\n[layers.b]\ntypes = [\"research\"]\nfiles = [\"./terms.md\"]\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("layers.b.files"), "{err}");
    assert!(err.contains("`a`"), "{err}");
}
