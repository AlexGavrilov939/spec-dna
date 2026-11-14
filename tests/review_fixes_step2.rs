//! Tests of the fixes of the review of step 2 of 0.6.0: `layer list` and a
//! document with a type that SD303 rejects, an autolink with a date in SD903,
//! and the placeholder row of the `terms` template in SD904. The helpers are
//! small copies of the ones in tests/layers_checks.rs.

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
    /// A repository with one mixed root `docs`. `top` is the top-level keys,
    /// `tables` is the text of the tables before the root, and `root` is more
    /// keys of the root.
    fn new(profile: &str, top: &str, tables: &str, root: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-review2-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let config = format!(
            "spec-dna = \"{VERSION}\"\nprofile = \"{profile}\"\n{top}\n\n{tables}\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n{root}\n"
        );
        fs::write(dir.join("spec-dna.toml"), config).unwrap();
        Repo { dir }
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn run(&self, args: &[&str]) -> String {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// `check --all`: every diagnostic, new or not.
    fn check(&self) -> String {
        self.run(&["check", "--all"])
    }
}

/// A document with a front matter. `extra` is more front matter lines.
fn doc(ty: &str, status: &str, extra: &str, body: &str) -> String {
    format!("---\ntype: {ty}\nstatus: {status}\n{extra}---\n# Title\n\n{body}\n")
}

/// A `requirements` document with one requirement, `a-1`.
fn req_doc(text: &str) -> String {
    doc(
        "requirements",
        "active",
        "",
        &format!("## Requirements\n\n- **a-1**: {text}"),
    )
}

/// The severity of a code in the output of `check`, or `None`.
fn severity(out: &str, code: &str) -> Option<&'static str> {
    ["error", "warning"]
        .into_iter()
        .find(|s| out.contains(&format!("{s} {code} ")))
}

// Fix 1: `layer list` and SD303

#[test]
fn layer_list_places_no_document_with_a_type_that_sd303_rejects() {
    let r = Repo::new("standard", "", "", "");
    r.write("docs/rules/bogus.md", &doc("bogus", "active", "", "Text."));
    r.write("docs/rules/spec.md", &doc("spec", "active", "", "Text."));
    r.write(
        "docs/rules/pump.md",
        &doc(
            "rule",
            "active",
            "",
            "## Rules\n\n- **pump-1**: The controller SHALL stop the pump.",
        ),
    );
    let out = r.run(&["layer", "list", "--json"]);
    // The valid rule is placed by its type. The two others are in no layer.
    assert!(
        out.contains("\"layer\":\"rules\",\"rule\":\"type\""),
        "{out}"
    );
    assert_eq!(
        out.matches("\"layer\":\"rules\",\"rule\":\"type\"").count(),
        1,
        "{out}"
    );
    let check = r.check();
    assert!(check.contains(" SD303 "), "{check}");
}

// Fix 2: an autolink in SD903 and SD904

#[test]
fn sd903_a_date_in_an_autolink_or_a_bare_url_is_not_history() {
    for text in [
        "The pump SHALL read <https://x.org/?d=2024-03-05> now.",
        "The pump SHALL read https://x.org/?d=2024-03-05 now.",
        "The pump SHALL read <mailto:a@x.org?subject=2024-03-05> now.",
    ] {
        let r = Repo::new("standard", "", "", "");
        r.write("docs/requirements/a.md", &req_doc(text));
        let out = r.check();
        assert_eq!(severity(&out, "SD903"), None, "{text}: {out}");
    }
    // A date in the prose still gives SD903, also next to an autolink.
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/requirements/a.md",
        &req_doc("The pump SHALL read <https://x.org/> since 2024-03-05."),
    );
    assert_eq!(severity(&r.check(), "SD903"), Some("warning"));
}

#[test]
fn sd904_a_synonym_in_an_autolink_is_not_a_match() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/terms.md",
        "---\ntype: terms\n---\n# Terms\n\n| Term | Meaning | Forbidden synonyms |\n| --- | --- | --- |\n| valve | A part. | tap |\n",
    );
    r.write(
        "docs/requirements/a.md",
        &req_doc("The pump SHALL read <https://x.org/tap> now."),
    );
    assert_eq!(severity(&r.check(), "SD904"), None);
}

// Fix 3: the placeholder row of the `terms` template

#[test]
fn sd904_the_placeholder_row_of_the_terms_template_gives_nothing() {
    let r = Repo::new("standard", "", "", "");
    let out = r.run(&["new", "terms"]);
    assert!(out.contains("wrote "), "{out}");
    r.write(
        "docs/requirements/a.md",
        &req_doc("The pump SHALL show the synonyms and the term."),
    );
    let out = r.check();
    assert_eq!(severity(&out, "SD904"), None, "{out}");
    assert!(!out.contains(" SD904 "), "{out}");
}
