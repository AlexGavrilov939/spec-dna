//! Integration tests of the place that `spec-dna new` gives a document. A top
//! file takes no id. `intent` writes `intent/<slug>/intent.md`, and `spec`
//! and `plan` need that folder. Another type writes `<layer dir>/<id>.md` in a
//! mixed root, and `<id>.md` in a root of one type. The file has no `type:`
//! line where the place gives the type. The helpers are small copies of the
//! ones in tests/cli.rs.

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
    /// A repository with a configuration: the head (keys and tables before
    /// the roots) and the text of the roots.
    fn with(head: &str, roots: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-new-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        fs::write(
            dir.join("spec-dna.toml"),
            format!("spec-dna = \"{VERSION}\"\n{head}{roots}"),
        )
        .unwrap();
        Repo { dir }
    }

    /// One mixed root `docs`, with the implied graph.
    fn mixed() -> Repo {
        Repo::with("", "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n")
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

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        self.dir.join(path).exists()
    }

    /// The type of the document at a path, from `check --all --json`.
    fn type_of(&self, path: &str) -> String {
        git(&self.dir, &["add", "-A"]);
        let json = self.run(&["check", "--all", "--json"]).1;
        let key = format!("\"path\":\"{path}\",\"type\":\"");
        let at = json
            .find(&key)
            .unwrap_or_else(|| panic!("{path} is not a document: {json}"))
            + key.len();
        json[at..].split('"').next().unwrap().to_string()
    }
}

#[test]
fn a_top_file_type_takes_no_id() {
    let r = Repo::mixed();
    for ty in ["product", "system", "terms", "roadmap", "questions"] {
        let path = format!("docs/{ty}.md");
        let (code, out, err) = r.run(&["new", ty]);
        assert_eq!(code, 0, "{ty}: {out}{err}");
        assert!(
            out.starts_with(&format!("wrote {path} ({ty} template)")),
            "{out}"
        );
        // The place gives the type: the file has no `type:` line.
        assert!(!r.read(&path).contains("type:"), "{ty}");
        assert_eq!(r.type_of(&path), ty);
        // A second run stops, and so does an id.
        let (code, _, err) = r.run(&["new", ty]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains("exists"), "{err}");
        let (code, _, err) = r.run(&["new", ty, "x"]);
        assert_eq!(code, 2, "{err}");
        assert!(err.contains(&format!("`new {ty}` takes no id")), "{err}");
    }
    assert_eq!(r.read("docs/terms.md").lines().next(), Some("# Terms"));
    // The JSON output has the path, the type and the id.
    let r = Repo::mixed();
    let (code, out, _) = r.run(&["new", "terms", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.trim(),
        "{\"path\":\"docs/terms.md\",\"type\":\"terms\",\"id\":\"terms\"}"
    );
}

#[test]
fn intent_writes_its_folder_and_spec_and_plan_need_it() {
    let r = Repo::mixed();
    // Without the folder, `spec` and `plan` stop with exit code 5.
    for ty in ["spec", "plan"] {
        let (code, out, err) = r.run(&["new", ty, "add-drip-lines"]);
        assert_eq!(code, 5, "{ty}: {out}{err}");
        assert!(
            err.contains("the folder `docs/intent/add-drip-lines` does not exist")
                && err.contains("new intent add-drip-lines"),
            "{err}"
        );
    }
    assert!(!r.exists("docs"));
    // An intent writes the folder, and then the 2 other files follow.
    let (code, out, err) = r.run(&["new", "intent", "add-drip-lines"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("wrote docs/intent/add-drip-lines/intent.md (intent template)"),
        "{out}"
    );
    for ty in ["spec", "plan"] {
        let (code, out, err) = r.run(&["new", ty, "add-drip-lines"]);
        assert_eq!(code, 0, "{ty}: {out}{err}");
    }
    for ty in ["intent", "spec", "plan"] {
        let path = format!("docs/intent/add-drip-lines/{ty}.md");
        let text = r.read(&path);
        assert!(!text.contains("type:"), "{text}");
        // The title is a placeholder: no rule makes a title from a slug.
        assert!(
            text.contains("\n# {{The title of the change.}}\n"),
            "{text}"
        );
        assert_eq!(r.type_of(&path), ty);
    }
    // A file that exists stops the command. A slug has one name.
    let (code, _, err) = r.run(&["new", "intent", "add-drip-lines"]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = r.run(&["new", "intent", "a/b"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("is not a slug"), "{err}");
    let (code, _, err) = r.run(&["new", "intent"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("`new intent` needs an id"), "{err}");
}

#[test]
fn another_type_goes_into_the_folder_of_its_layer() {
    let r = Repo::mixed();
    for (ty, id, path) in [
        ("rule", "water", "docs/rules/water.md"),
        ("requirements", "loans", "docs/requirements/loans.md"),
        (
            "requirements",
            "valves/closing",
            "docs/requirements/valves/closing.md",
        ),
        ("interface", "http", "docs/interfaces/http.md"),
        ("decision", "pump-choice", "docs/decisions/pump-choice.md"),
        ("procedure", "restart", "docs/procedures/restart.md"),
        ("incident", "flooded-bed", "docs/incidents/flooded-bed.md"),
        ("research", "pumps", "docs/research/pumps.md"),
    ] {
        let (code, out, err) = r.run(&["new", ty, id]);
        assert_eq!(code, 0, "{ty}: {out}{err}");
        assert!(
            out.starts_with(&format!("wrote {path} ({ty} template)")),
            "{out}"
        );
        assert!(!r.read(path).contains("type:"), "{ty}");
        assert_eq!(r.type_of(path), ty);
    }
    let (code, _, err) = r.run(&["new", "rule"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("`new rule` needs an id"), "{err}");
    // The first status of the type stays in the file.
    assert!(
        r.read("docs/decisions/pump-choice.md")
            .contains("status: proposed\n")
    );
}

#[test]
fn a_root_of_one_type_takes_id_md_and_gives_the_type() {
    let r = Repo::with(
        "",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[[roots]]\npath = \"docs/adr\"\ntype = \"decision\"\n",
    );
    let (code, out, err) = r.run(&["new", "decision", "pump-choice"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("wrote docs/adr/pump-choice.md (decision template)"),
        "{out}"
    );
    let text = r.read("docs/adr/pump-choice.md");
    assert!(
        text.starts_with("---\nstatus: proposed\n---\n# Pump choice\n"),
        "{text}"
    );
    assert_eq!(r.type_of("docs/adr/pump-choice.md"), "decision");
    // An id with a folder stays under the root.
    let (code, out, err) = r.run(&["new", "decision", "pumps/second"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists("docs/adr/pumps/second.md"));
    // A root of one type with a top-file type needs an id: there is no top
    // file in such a root.
    let r = Repo::with("", "[[roots]]\npath = \"docs/words\"\ntype = \"terms\"\n");
    let (code, _, err) = r.run(&["new", "terms"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("`new terms` needs an id"), "{err}");
    let (code, out, err) = r.run(&["new", "terms", "kitchen"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists("docs/words/kitchen.md"));
    // A type that the root does not hold stops.
    let (code, _, err) = r.run(&["new", "rule", "x", "--root", "docs/words"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("holds only `terms` documents"), "{err}");
}

#[test]
fn the_layers_table_gives_the_folder() {
    let r = Repo::with(
        "[layers.policies]\ntypes = [\"rule\"]\ndir = \"policy\"\n\n[layers.base]\ntypes = [\"terms\"]\nfiles = [\"glossary.md\", \"terms.md\"]\n\n[layers.words]\ntypes = [\"questions\"]\nfiles = [\"asks.md\"]\n\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    let (code, out, err) = r.run(&["new", "rule", "water"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("wrote docs/policy/water.md"), "{out}");
    assert_eq!(r.type_of("docs/policy/water.md"), "rule");
    let (code, out, err) = r.run(&["new", "terms"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("wrote docs/terms.md"), "{out}");
    // A layer with top files, but none with the name of the type.
    let (code, _, err) = r.run(&["new", "questions"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("layer `words` has no top file `questions.md`"),
        "{err}"
    );
    // A type that no layer has.
    let (code, _, err) = r.run(&["new", "research", "x"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("no layer of spec-dna.toml has type `research`; run `spec-dna layer add <name> --types research`"),
        "{err}"
    );
    assert!(!r.exists("docs/research"));
}

#[test]
fn the_type_line_stays_where_the_place_gives_no_type() {
    // A layer with 2 types gives a type by the file name only, so a new
    // document there needs its `type:` line.
    let r = Repo::with(
        "[layers.law]\ntypes = [\"rule\", \"requirements\"]\n\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    let (code, out, err) = r.run(&["new", "rule", "water"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("wrote docs/law/water.md (rule template)"),
        "{out}"
    );
    let text = r.read("docs/law/water.md");
    assert!(
        text.starts_with("---\ntype: rule\nstatus: draft\n"),
        "{text}"
    );
    assert_eq!(r.type_of("docs/law/water.md"), "rule");
    // The file name of a type gives the place, so `requirements.md` needs no line.
    let (code, out, err) = r.run(&["new", "requirements", "requirements"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("docs/law/requirements.md").contains("type:"));
    assert_eq!(r.type_of("docs/law/requirements.md"), "requirements");
}

#[test]
fn a_root_at_the_repository_root_and_the_types_list_limit_new() {
    let r = Repo::with(
        "types = [\"requirements\", \"decision\"]\n",
        "[[roots]]\npath = \".\"\ntype = \"mixed\"\nexclude = [\"README.md\"]\n",
    );
    let (code, out, err) = r.run(&["new", "requirements", "loans"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists("requirements/loans.md"));
    assert_eq!(r.type_of("requirements/loans.md"), "requirements");
    let (code, _, err) = r.run(&["new", "rule", "water"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("not a type of this project"), "{err}");
}

#[test]
fn history_writes_the_type_line_when_the_folder_gives_another_type() {
    // The folder `decisions/` gives the type `decision`. The history file is a
    // `research` document, so it needs its own `type:` line, also when the
    // root has the default type `research`.
    let r = Repo::with(
        "",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"research\"\nhistory-sections = { decision = [\"Discussion\"] }\n",
    );
    fs::create_dir_all(r.dir.join("docs/decisions")).unwrap();
    fs::write(
        r.dir.join("docs/decisions/pump.md"),
        "---\nstatus: accepted\n---\n# Pump\n\n## Decision Outcome\n\nThe pump runs at night.\n\n## Discussion\n\nRound 1 asked for the day.\n",
    )
    .unwrap();
    git(&r.dir, &["add", "-A"]);
    git(&r.dir, &["commit", "-q", "-m", "init"]);
    let (code, out, err) = r.run(&["history", "decisions/pump"]);
    assert_eq!(code, 0, "{out}{err}");
    let h = r.read("docs/decisions/pump-history.md");
    assert!(h.contains("type: research"), "{h}");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}
