//! Integration tests of the place rule (docs/types.md, "Place gives the
//! type"). A mixed root gives a document its type from the front matter, then
//! from a top file, then from the folder of a layer, then from the file name
//! in the folder of a layer with more types, then from `default-type`. The
//! helpers are small copies of the ones in tests/cli.rs.

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
    /// A repository with a configuration. `roots` is the text of the
    /// `[[roots]]` table and what follows it.
    fn with(head: &str, roots: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-place-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let r = Repo { dir };
        r.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{VERSION}\"\n{head}[[roots]]\npath = \"docs\"\n{roots}"),
        );
        r
    }

    /// A mixed root with no `default-type`.
    fn mixed() -> Repo {
        Repo::with("", "type = \"mixed\"\n")
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", "x"]);
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

    /// The output of `check --all`.
    fn check(&self) -> String {
        self.run(&["check", "--all"]).1
    }

    /// The type of the document at a path, from `check --all --json`.
    fn type_of(&self, path: &str) -> String {
        let json = self.run(&["check", "--all", "--json"]).1;
        let key = format!("\"path\":\"{path}\",\"type\":\"");
        let at = json
            .find(&key)
            .unwrap_or_else(|| panic!("{path} is not a document: {json}"))
            + key.len();
        json[at..].split('"').next().unwrap().to_string()
    }
}

/// A document of a type: with the status and the section that the type needs.
fn doc(ty: &str, with_type: bool) -> String {
    let (status, body) = match ty {
        "rule" => ("active", "## Rules\n\nText.\n"),
        "requirements" | "interface" | "procedure" => ("active", "Text.\n"),
        "decision" => ("accepted", "## Decision Outcome\n\nText.\n"),
        "incident" => (
            "",
            "## Summary\n\nText.\n\n## Root cause\n\nText. See [[x]].\n",
        ),
        "intent" => ("draft", "## Intent\n\nText.\n"),
        "spec" => ("draft", "## Delta\n\nText.\n"),
        "plan" => ("draft", "## Files\n\nText.\n"),
        _ => ("", "Text.\n"),
    };
    let mut fm = String::new();
    if with_type {
        fm.push_str(&format!("type: {ty}\n"));
    }
    if !status.is_empty() {
        fm.push_str(&format!("status: {status}\n"));
    }
    let fm = if fm.is_empty() {
        String::new()
    } else {
        format!("---\n{fm}---\n")
    };
    format!("{fm}# Title\n\n{body}")
}

const PLACES: [(&str, &str); 15] = [
    ("docs/product.md", "product"),
    ("docs/system.md", "system"),
    ("docs/terms.md", "terms"),
    ("docs/roadmap.md", "roadmap"),
    ("docs/questions.md", "questions"),
    ("docs/rules/water.md", "rule"),
    ("docs/requirements/valves/closing.md", "requirements"),
    ("docs/interfaces/http.md", "interface"),
    ("docs/decisions/pump.md", "decision"),
    ("docs/procedures/restart.md", "procedure"),
    ("docs/incidents/flood.md", "incident"),
    ("docs/research/pumps.md", "research"),
    ("docs/intent/drip/intent.md", "intent"),
    ("docs/intent/drip/spec.md", "spec"),
    ("docs/intent/drip/plan.md", "plan"),
];

#[test]
fn each_place_gives_its_type_with_and_without_a_type_line() {
    for with_type in [false, true] {
        let r = Repo::mixed();
        for (path, ty) in PLACES {
            r.write(path, &doc(ty, with_type));
        }
        let out = r.check();
        assert!(!out.contains("SD303"), "type line {with_type}: {out}");
        assert!(!out.contains("SD304"), "type line {with_type}: {out}");
        assert!(!out.contains("SD305"), "type line {with_type}: {out}");
        for (path, ty) in PLACES {
            assert_eq!(r.type_of(path), ty, "{path}, type line {with_type}");
        }
    }
}

#[test]
fn a_document_outside_every_place_has_no_type_without_a_default() {
    let r = Repo::mixed();
    r.write("docs/loans.md", "# Loans\n\nText.\n");
    r.write("docs/valves/closing.md", "# Closing\n\nText.\n");
    let out = r.check();
    for p in ["docs/loans.md", "docs/valves/closing.md"] {
        assert!(
            out.contains(&format!(
                "{p}:1: error SD303 type-invalid: the document has no type"
            )),
            "{out}"
        );
    }
    // One SD303 for each document.
    assert_eq!(out.matches("error SD303").count(), 2, "{out}");
    // A `default-type` gives the type of such a document, and the place
    // still has priority over it.
    let r = Repo::with("", "type = \"mixed\"\ndefault-type = \"research\"\n");
    r.write("docs/loans.md", "# Loans\n\nText.\n");
    r.write("docs/rules/water.md", &doc("rule", false));
    let out = r.check();
    assert!(!out.contains("SD303"), "{out}");
    assert_eq!(r.type_of("docs/loans.md"), "research");
    assert_eq!(r.type_of("docs/rules/water.md"), "rule");
}

#[test]
fn the_file_name_gives_the_type_in_the_intent_layer() {
    let r = Repo::mixed();
    r.write("docs/intent/drip/notes.md", "# Notes\n\nText.\n");
    r.write("docs/intent/lonely.md", "# Lonely\n\nText.\n");
    let out = r.check();
    assert!(
        out.contains(
            "docs/intent/drip/notes.md:1: error SD303 type-invalid: the document has no type"
        ),
        "{out}"
    );
    assert!(
        out.contains("docs/intent/lonely.md:1: error SD303 type-invalid: the document has no type"),
        "{out}"
    );
    // A file with the name of a type, but at another depth of the folder,
    // is not a document of that type. The type that its place gives
    // otherwise (none here) takes its place.
    let r = Repo::mixed();
    r.write("docs/intent/spec.md", &doc("spec", false));
    r.write("docs/intent/a/b/plan.md", &doc("plan", false));
    let out = r.check();
    assert!(
        out.contains("docs/intent/spec.md:1: error SD303 type-invalid: type `spec` is valid only in the `intent` layer, as `intent/<slug>/spec.md`"),
        "{out}"
    );
    assert!(
        out.contains(
            "docs/intent/a/b/plan.md:1: error SD303 type-invalid: type `plan` is valid only"
        ),
        "{out}"
    );
    assert_eq!(out.matches("error SD303").count(), 2, "{out}");
    assert_eq!(r.type_of("docs/intent/spec.md"), "research");
}

#[test]
fn a_spec_outside_the_intent_layer_gives_sd303_and_names_requirements() {
    let r = Repo::mixed();
    r.write("docs/rules/water.md", &doc("spec", true));
    r.write("docs/spec.md", &doc("spec", true));
    r.write("docs/intent/spec.md", &doc("spec", true));
    r.write("docs/intent/a/b/spec.md", &doc("spec", true));
    let out = r.check();
    for p in [
        "docs/rules/water.md",
        "docs/spec.md",
        "docs/intent/spec.md",
        "docs/intent/a/b/spec.md",
    ] {
        let line = out
            .lines()
            .find(|l| l.starts_with(&format!("{p}:1: error SD303")))
            .unwrap_or_else(|| panic!("{p}: {out}"));
        assert!(
            line.contains(
                "type `spec` is valid only in the `intent` layer, as `intent/<slug>/spec.md`"
            ) && line.contains("the 0.5 type `spec` is the 0.6 type `requirements`"),
            "{line}"
        );
    }
    assert_eq!(out.matches("error SD303").count(), 4, "{out}");
    // `intent` and `plan` outside their place give SD303 too, but they have
    // no 0.5 type to report.
    let r = Repo::mixed();
    r.write("docs/rules/a.md", &doc("intent", true));
    r.write("docs/rules/b.md", &doc("plan", true));
    let out = r.check();
    assert_eq!(out.matches("error SD303").count(), 2, "{out}");
    assert!(!out.contains("0.5 type"), "{out}");
    // The place that the document has gives its type in this case.
    assert_eq!(r.type_of("docs/rules/a.md"), "rule");
}

#[test]
fn the_front_matter_type_overrides_the_place_only_in_a_mixed_root() {
    let r = Repo::mixed();
    r.write("docs/rules/water.md", &doc("requirements", true));
    r.write("docs/intent/a/spec.md", &doc("research", true));
    let out = r.check();
    assert!(!out.contains("SD303"), "{out}");
    assert_eq!(r.type_of("docs/rules/water.md"), "requirements");
    assert_eq!(r.type_of("docs/intent/a/spec.md"), "research");
    // A root of one type gives the type, and a front matter that differs is
    // an error. The folder of a layer in that root gives nothing.
    let r = Repo::with("", "type = \"requirements\"\nstatus = \"active\"\n");
    r.write("docs/rules/water.md", "# Water\n\nText.\n");
    r.write("docs/loans.md", &doc("rule", true));
    let out = r.check();
    assert!(
        out.contains("docs/loans.md:2: error SD303 type-invalid: type `rule` disagrees with root `docs` of type `requirements`"),
        "{out}"
    );
    assert_eq!(out.matches("error SD303").count(), 1, "{out}");
    assert_eq!(r.type_of("docs/rules/water.md"), "requirements");
}

#[test]
fn a_layer_with_dir_and_files_keys_places_its_documents() {
    let r = Repo::with(
        "[layers.policies]\ntypes = [\"rule\"]\ndir = \"policy\"\n\n[layers.base]\ntypes = [\"terms\", \"product\"]\nfiles = [\"glossary.md\", \"product.md\"]\n\n[layers.tales]\ntypes = [\"research\", \"incident\"]\n",
        "type = \"mixed\"\n",
    );
    r.write("docs/policy/water.md", &doc("rule", false));
    // A top file of a layer with more types gives the type by its name, and
    // other names give none.
    r.write("docs/product.md", &doc("product", false));
    r.write("docs/glossary.md", "# Glossary\n\nText.\n");
    // A layer with more types: the file name gives the type.
    r.write("docs/tales/research.md", "# R\n\nText.\n");
    r.write("docs/tales/incident.md", &doc("incident", false));
    r.write("docs/tales/other.md", "# Other\n\nText.\n");
    let out = r.check();
    assert_eq!(r.type_of("docs/policy/water.md"), "rule");
    assert_eq!(r.type_of("docs/product.md"), "product");
    assert_eq!(r.type_of("docs/tales/research.md"), "research");
    assert_eq!(r.type_of("docs/tales/incident.md"), "incident");
    for p in ["docs/glossary.md", "docs/tales/other.md"] {
        assert!(
            out.contains(&format!(
                "{p}:1: error SD303 type-invalid: the document has no type"
            )),
            "{out}"
        );
    }
    assert_eq!(out.matches("error SD303").count(), 2, "{out}");
}

#[test]
fn a_type_that_no_layer_has_gives_sd303_with_the_layer_add_hint() {
    let r = Repo::with(
        "[layers.rules]\ntypes = [\"rule\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n",
        "type = \"mixed\"\n",
    );
    r.write("docs/rules/water.md", &doc("rule", false));
    r.write("docs/research/pumps.md", &doc("research", true));
    r.write("docs/loans.md", &doc("requirements", true));
    let out = r.check();
    for ty in ["research", "requirements"] {
        let path = if ty == "research" {
            "docs/research/pumps.md"
        } else {
            "docs/loans.md"
        };
        assert!(
            out.contains(&format!(
                "{path}:1: error SD303 type-invalid: no layer of spec-dna.toml has type `{ty}`; run `spec-dna layer add <name> --types {ty}` to add one"
            )),
            "{out}"
        );
    }
    assert_eq!(out.matches("error SD303").count(), 2, "{out}");
    // Without a `[layers]` table, the implied graph has a layer for each type.
    let r = Repo::mixed();
    r.write("docs/research/pumps.md", &doc("research", true));
    assert!(!r.check().contains("SD303"));
}

#[test]
fn a_type_of_one_change_is_not_a_type_of_a_root() {
    for (key, ty) in [
        ("type", "intent"),
        ("type", "spec"),
        ("type", "plan"),
        ("default-type", "intent"),
        ("default-type", "spec"),
        ("default-type", "plan"),
    ] {
        let extra = if key == "type" {
            format!("type = \"{ty}\"\n")
        } else {
            format!("type = \"mixed\"\ndefault-type = \"{ty}\"\n")
        };
        let r = Repo::with("", &extra);
        r.write("docs/a.md", "# A\n");
        let (code, _, err) = r.run(&["check"]);
        assert_eq!(code, 3, "{key} {ty}: {err}");
        assert!(
            err.contains(&format!("roots[0].{key}"))
                && err.contains(&format!("type `{ty}` is valid only in the `intent` layer")),
            "{err}"
        );
        assert_eq!(
            err.contains("the 0.5 type `spec` is the 0.6 type `requirements`"),
            ty == "spec",
            "{err}"
        );
    }
}

#[test]
fn a_root_status_needs_a_default_type() {
    let r = Repo::with("", "type = \"mixed\"\nstatus = \"active\"\n");
    r.write("docs/a.md", "# A\n");
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(
        err.contains("`roots[0].status` needs `roots[0].default-type`"),
        "{err}"
    );
}

#[test]
fn was_type_is_read_and_checked() {
    let r = Repo::mixed();
    r.write(
        "docs/research/old.md",
        "---\nwas-type: note\n---\n# Old\n\nText.\n",
    );
    r.write(
        "docs/research/done.md",
        "---\nwas-type: change\n---\n# Done\n\nText.\n",
    );
    r.write(
        "docs/research/bad.md",
        "---\nwas-type: rule\n---\n# Bad\n\nText.\n",
    );
    let out = r.check();
    assert!(
        out.contains("docs/research/bad.md:2: error SD302 front-matter-field: `was-type` must be `note` or `change`, not `rule`"),
        "{out}"
    );
    assert_eq!(out.matches("SD302").count(), 2, "{out}");
}

#[test]
fn a_stub_needs_no_type() {
    let r = Repo::mixed();
    r.write(
        "docs/loans.md",
        "---\nmoved-to: requirements/loans\n---\n# Loans\n",
    );
    r.write("docs/requirements/loans.md", &doc("requirements", false));
    let out = r.check();
    assert!(!out.contains("SD303"), "{out}");
}
