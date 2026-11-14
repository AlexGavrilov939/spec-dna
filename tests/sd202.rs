//! Integration tests of SD202 (`id-removed`). The layer of the base document
//! decides: a layer with `keep = "until-finished"` allows a deletion. In a
//! `forever` layer, only `was-type: note` or `was-type: change` allows it.
//! The helpers are small copies of the ones in tests/new_place.rs.

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
    /// A repository with one mixed root `docs`. `layers` is the text of the
    /// tables `[layers.*]`: it is empty for the default graph.
    fn with(layers: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-sd202-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let config = format!(
            "spec-dna = \"{VERSION}\"\n{layers}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
        );
        fs::write(dir.join("spec-dna.toml"), config).unwrap();
        Repo { dir }
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
        git(&self.dir, &["commit", "-q", "-m", "start"]);
    }

    fn check(&self) -> (i32, String) {
        let out = Command::new(BIN)
            .arg("check")
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }
}

/// A document with a front matter. `extra` is more front matter lines.
fn doc(ty: &str, status: &str, extra: &str, body: &str) -> String {
    format!("---\ntype: {ty}\nstatus: {status}\n{extra}---\n# Title\n\n{body}\n")
}

const INTENT_BODY: &str = "## Intent\n\nText.";
const PLAIN: &str = "Text.";

#[test]
fn a_deleted_intent_folder_gives_no_sd202_in_any_status() {
    let r = Repo::with("");
    for status in ["draft", "accepted", "closed"] {
        r.write(
            &format!("docs/intent/{status}/intent.md"),
            &doc("intent", status, "", INTENT_BODY),
        );
    }
    r.write(
        "docs/intent/closed/plan.md",
        &doc("plan", "draft", "", "## Files\n\nText."),
    );
    r.commit();
    fs::remove_dir_all(r.dir.join("docs/intent")).unwrap();
    let (code, out) = r.check();
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD202"), "{out}");
}

#[test]
fn a_deleted_research_document_gives_sd202_unless_it_has_was_type() {
    let r = Repo::with("");
    r.write(
        "docs/research/plain.md",
        &doc("research", "active", "", PLAIN),
    );
    r.write(
        "docs/research/former-note.md",
        &doc("research", "active", "was-type: note\n", PLAIN),
    );
    r.write(
        "docs/research/former-change.md",
        &doc("research", "active", "was-type: change\n", PLAIN),
    );
    r.commit();
    for f in ["plain", "former-note", "former-change"] {
        r.remove(&format!("docs/research/{f}.md"));
    }
    let (code, out) = r.check();
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("SD202 id-removed: `research/plain` was removed"),
        "{out}"
    );
    assert_eq!(out.matches("error SD202").count(), 1, "{out}");
}

#[test]
fn a_deleted_rule_or_requirements_document_gives_sd202() {
    let r = Repo::with("");
    r.write(
        "docs/rules/logging.md",
        &doc(
            "rule",
            "active",
            "",
            "- **log**: The code SHALL log errors.",
        ),
    );
    r.write(
        "docs/requirements/store.md",
        &doc(
            "requirements",
            "active",
            "",
            "- **keep**: The store SHALL keep a record.",
        ),
    );
    // A final status does not allow the deletion: the layer is `forever`.
    r.write(
        "docs/rules/old.md",
        &doc("rule", "retired", "", "- **old**: The code SHALL be old."),
    );
    r.commit();
    for f in ["rules/logging", "requirements/store", "rules/old"] {
        r.remove(&format!("docs/{f}.md"));
    }
    let (code, out) = r.check();
    assert_eq!(code, 1, "{out}");
    assert_eq!(out.matches("error SD202").count(), 3, "{out}");
    for id in ["rules/logging", "requirements/store", "rules/old"] {
        assert!(
            out.contains(&format!("SD202 id-removed: `{id}` was removed")),
            "{out}"
        );
    }
}

#[test]
fn a_custom_until_finished_layer_allows_a_deletion() {
    let layers = [
        "[layers.requirements]\ntypes = [\"requirements\"]\n",
        "[layers.sketches]\ntypes = [\"research\"]\nkeep = \"until-finished\"\n",
        "[layers.scratch]\npaths = [\"scratch/**\"]\nkeep = \"until-finished\"\n",
        "[layers.notes]\ndir = \"notes\"\ntypes = [\"procedure\"]\n",
    ];
    let r = Repo::with(&layers.join("\n"));
    // The layer `sketches` has the type. The layer `scratch` has the path,
    // also for a document of a type in another layer.
    r.write(
        "docs/sketches/idea.md",
        &doc("research", "active", "", PLAIN),
    );
    r.write(
        "docs/scratch/draft.md",
        &doc(
            "requirements",
            "active",
            "",
            "- **tmp**: The code SHALL be temporary.",
        ),
    );
    r.write(
        "docs/requirements/store.md",
        &doc(
            "requirements",
            "active",
            "",
            "- **keep**: The store SHALL keep a record.",
        ),
    );
    r.write("docs/notes/how.md", &doc("procedure", "active", "", PLAIN));
    r.commit();
    let (code, out) = r.check();
    assert_eq!(code, 0, "{out}");
    for f in [
        "sketches/idea",
        "scratch/draft",
        "requirements/store",
        "notes/how",
    ] {
        r.remove(&format!("docs/{f}.md"));
    }
    let (code, out) = r.check();
    assert_eq!(code, 1, "{out}");
    assert_eq!(out.matches("error SD202").count(), 2, "{out}");
    for id in ["requirements/store", "notes/how"] {
        assert!(
            out.contains(&format!("SD202 id-removed: `{id}` was removed")),
            "{out}"
        );
    }
}

#[test]
fn a_deleted_document_with_an_alias_in_the_head_gives_no_sd202() {
    let r = Repo::with("");
    r.write(
        "docs/rules/logging.md",
        &doc(
            "rule",
            "active",
            "",
            "- **log**: The code SHALL log errors.",
        ),
    );
    r.commit();
    r.remove("docs/rules/logging.md");
    r.write(
        "docs/rules/logs.md",
        &doc(
            "rule",
            "active",
            "aliases: [rules/logging]\n",
            "- **log**: The code SHALL log errors.",
        ),
    );
    let (code, out) = r.check();
    assert_eq!(code, 0, "{out}");
}
