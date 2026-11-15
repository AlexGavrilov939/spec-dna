//! Integration tests of the layer graph and the areas in `spec-dna init`
//! (questions 2 and 3), and of the layout of a new project: one mixed root
//! with no `default-type`. The helpers are small copies of the ones in
//! tests/layer_cmd.rs. The neutral domains are irrigation and library.

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
        let dir = std::env::temp_dir().join(format!("spec-dna-graph-{}-{n}", std::process::id()));
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

    /// The names of the layers in `spec-dna.toml`, in file order.
    fn layers(&self) -> Vec<String> {
        self.read("spec-dna.toml")
            .lines()
            .filter_map(|l| l.strip_prefix("[layers.")?.strip_suffix(']'))
            .map(str::to_string)
            .collect()
    }
}

/// The text of `spec-dna.toml` before its first table, for the top keys.
fn top_keys(cfg: &str) -> &str {
    cfg.split("\n[").next().unwrap()
}

const SMALL: [&str; 7] = [
    "foundation",
    "pending",
    "rules",
    "requirements",
    "decisions",
    "intent",
    "incidents",
];

const DEFAULT: [&str; 10] = [
    "foundation",
    "rules",
    "requirements",
    "interfaces",
    "decisions",
    "procedures",
    "research",
    "intent",
    "incidents",
    "pending",
];

/// `n` plain documents in a folder that detection reads as a root of
/// requirements.
fn specs(r: &Repo, folder: &str, n: usize) {
    for i in 0..n {
        r.write(
            &format!("docs/specs/{folder}zone-{i:02}.md"),
            &format!("# Zone {i}\n\n## Requirements\n\n- **open-{i}**: The controller SHALL open valve {i}.\n"),
        );
    }
}

#[test]
fn a_new_project_gets_the_small_graph_and_a_root_without_a_default_type() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.layers(), SMALL);
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.ends_with("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"),
        "{cfg}"
    );
    assert!(!cfg.contains("default-type"), "{cfg}");
    assert!(!cfg.contains("status ="), "{cfg}");
    assert!(!cfg.contains("areas"), "{cfg}");
    assert!(
        top_keys(&cfg).contains(&format!("spec-dna = \"{}\"", env!("CARGO_PKG_VERSION"))),
        "{cfg}"
    );
    // `check` exits 0 on the result, and the adapters stay small.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let bytes = prefix_bytes(&r.run(&["init"]).1);
    assert!(bytes <= 300, "{bytes} bytes");
    // The preview names the questions with the numbers of the design.
    let (_, out, _) = r.run(&["init", "--reconfigure", "--dry-run", "--non-interactive"]);
    for n in 1..=13 {
        assert!(out.contains(&format!("\n  {n}. ")), "question {n}:\n{out}");
    }
    // Question 14 (the evals) comes in step 4.
    assert!(!out.contains("\n  14. "), "question 14:\n{out}");
}

/// The bytes that the adapters add to the prefix of each request, from the
/// output of `init`.
fn prefix_bytes(out: &str) -> usize {
    out.lines()
        .find_map(|l| l.strip_prefix("per-request prefix: "))
        .and_then(|l| l.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no prefix line:\n{out}"))
}

#[test]
fn the_number_of_documents_chooses_small_or_default() {
    let r = Repo::empty();
    specs(&r, "", 29);
    r.commit("docs");
    let (code, out, err) = r.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("2. layer graph: small (default)"), "{out}");
    let r = Repo::empty();
    specs(&r, "", 30);
    r.commit("docs");
    // The answer wins over the size.
    let (code, out, err) = r.init_with("graph = \"small\"\n", &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("2. layer graph: small (changed from default)"),
        "{out}"
    );
    let (code, out, err) = r.init_with("graph = \"huge\"\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("use small, default or custom"), "{err}");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.layers(), DEFAULT);
}

#[test]
fn the_default_graph_is_written_in_the_order_of_the_design() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.layers(), DEFAULT);
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // The default graph has a layer for each type, so each type is a type of
    // the project, and `new` writes a procedure into its folder.
    let (code, out, err) = r.run(&["new", "procedure", "flush-lines"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("wrote docs/procedures/flush-lines.md"),
        "{out}"
    );
    let (code, out, _) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("procedures"), "{out}");
}

#[test]
fn custom_keeps_the_layers_of_the_current_file() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    // A layer of the project, with its own folder and a limit.
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "pumps",
        "--types",
        "interface",
        "--dir",
        "hardware",
        "--max-docs",
        "12",
        "--moved-from",
        "intent",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("layers");
    let before = r.layers();
    assert_eq!(before.len(), 8);
    // `--reconfigure` offers `custom`, and it keeps the layers in their order.
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.layers(), before);
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(
            "[layers.pumps]\ntypes = [\"interface\"]\ndir = \"hardware\"\nmax-docs = 12\n"
        ),
        "{cfg}"
    );
    assert!(
        cfg.contains("moves-to = [\"rules\", \"requirements\", \"decisions\", \"pumps\"]"),
        "{cfg}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // The preview says that the answer is the default.
    let (_, out, _) = r.run(&["init", "--reconfigure", "--dry-run", "--non-interactive"]);
    assert!(out.contains("2. layer graph: custom (default)"), "{out}");
    // Another answer replaces the layers.
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.layers(), DEFAULT);
    // A new project has no layers to keep.
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"custom\"\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("keeps the layers of the current spec-dna.toml"),
        "{err}"
    );
    assert!(!r.exists("spec-dna.toml"));
}

#[test]
fn a_file_with_no_layers_table_stays_without_it_on_reconfigure() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{}\"\ntypes = [\"requirements\", \"decision\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"draft\"\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    let (code, out, err) = r.init_with("", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("[layers"), "{cfg}");
    // The key `types` stays, and it narrows the types of the skill.
    assert!(
        top_keys(&cfg).contains("types = [\"requirements\", \"decision\"]"),
        "{cfg}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_old_types_answer_names_graph() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("types = [\"requirements\", \"decision\"]\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("`types`") && err.contains("`graph`"), "{err}");
    assert!(!r.exists("spec-dna.toml"));
}

#[test]
fn a_root_of_a_type_without_a_layer_is_not_written() {
    let r = Repo::empty();
    for i in 0..3 {
        r.write(
            &format!("docs/runbooks/flush-{i}.md"),
            "# Flush\n\n## Steps\n\n1. Open the valve.\n",
        );
    }
    for i in 0..3 {
        r.write(&format!("rfcs/0{i}-drip.md"), "# Drip\n\nA proposal.\n");
    }
    r.write("docs/product-notes.md", "# Notes\n");
    r.commit("docs");
    let (code, out, err) = r.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("note: root `docs/runbooks` of type `procedure` is not written: the small graph has no layer with that type"),
        "{out}"
    );
    assert!(
        out.contains("note: root `rfcs` of type `research` is not written"),
        "{out}"
    );
    assert!(!out.contains("path = \"docs/runbooks\""), "{out}");
    // The note about proposals comes also when the graph drops the root.
    assert!(
        out.contains("To start an open proposal, run `spec-dna new intent <slug>`"),
        "{out}"
    );
    // The default graph keeps them. A flat folder of proposals is a
    // `research` root, and the note says how to start an open proposal.
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("path = \"docs/runbooks\"\ntype = \"procedure\""),
        "{out}"
    );
    assert!(
        out.contains("path = \"rfcs\"\ntype = \"research\""),
        "{out}"
    );
    assert!(
        out.contains("note: a folder of proposals or changes is a `research` root")
            && out.contains("To start an open proposal, run `spec-dna new intent <slug>`"),
        "{out}"
    );
    // Written, the default graph passes `check`.
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    // `--json` has the notes, and no root has a default type that is not there.
    let (code, out, _) = r.run(&[
        "init",
        "--reconfigure",
        "--dry-run",
        "--non-interactive",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\"notes\":["), "{out}");
}

#[test]
fn the_answers_can_leave_no_root_and_init_writes_the_root_of_a_new_project() {
    let r = Repo::empty();
    r.write(
        ".claude/agents/mine.md",
        "---\nname: mine\ndescription: mine\n---\n\nMine.\n",
    );
    r.commit("agent");
    // The agent folder is a root of procedures. The small graph has none.
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "no detected root fits the small graph: `init` writes the root `docs` of a new project"
        ),
        "{out}"
    );
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.ends_with("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"),
        "{cfg}"
    );
}

/// A project with folders of requirements: the areas.
fn project_with_areas(folders: &[(&str, usize)]) -> Repo {
    let r = Repo::empty();
    for (folder, n) in folders {
        for i in 0..*n {
            r.write(
                &format!("docs/specs/{folder}/note-{i}.md"),
                "# Note\n\n## Requirements\n\n- **open**: The controller SHALL open the valve.\n",
            );
        }
    }
    r.commit("docs");
    r
}

#[test]
fn two_folders_of_specs_are_areas_at_every_graph_size() {
    let r = project_with_areas(&[("valves", 1), ("sensors", 2), ("reports", 1)]);
    for graph in ["small", "default"] {
        let (code, out, err) = r.init_with(&format!("graph = \"{graph}\"\n"), &[]);
        assert_eq!(code, 0, "{graph}: {out}{err}");
        let cfg = r.read("spec-dna.toml");
        assert!(
            top_keys(&cfg).contains("areas = [\"reports\", \"sensors\", \"valves\"]"),
            "{graph}:\n{cfg}"
        );
        // The preview lists the answer.
        let (_, out, _) = r.run(&["init", "--reconfigure", "--dry-run", "--non-interactive"]);
        assert!(
            out.contains("3. areas of requirements: reports, sensors, valves (default)"),
            "{out}"
        );
        let (code, out, _) = r.run(&["check"]);
        assert_eq!(code, 0, "{out}");
        fs::remove_file(r.dir.join("spec-dna.toml")).unwrap();
    }
    // `none` gives one folder, so `areas` is absent. A list of one name is
    // the choice of the user, and it stays.
    for (answer, want) in [
        ("areas = \"none\"\n", None),
        ("areas = []\n", None),
        ("areas = [\"valves\"]\n", Some("areas = [\"valves\"]")),
    ] {
        let (code, out, err) = r.init_with(answer, &[]);
        assert_eq!(code, 0, "{answer}: {out}{err}");
        let cfg = r.read("spec-dna.toml");
        match want {
            Some(w) => assert!(top_keys(&cfg).contains(w), "{answer}:\n{cfg}"),
            None => assert!(!cfg.contains("areas"), "{answer}:\n{cfg}"),
        }
        fs::remove_file(r.dir.join("spec-dna.toml")).unwrap();
    }
    // A bad name is a usage error.
    let (code, out, err) = r.init_with("areas = [\"a/b\"]\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("one folder"), "{err}");
    let (code, _, err) = r.init_with("areas = [\"\"]\n", &[]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = r.init_with("areas = [\" a\"]\n", &[]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = r.init_with("areas = [\"a\", \"a\"]\n", &[]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("listed twice"), "{err}");
}

#[test]
fn one_folder_of_specs_is_no_area() {
    let r = project_with_areas(&[("valves", 3)]);
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains("areas"));
}

#[test]
fn at_most_8_areas_are_proposed_and_the_folders_with_the_most_documents_win() {
    let mut folders: Vec<(String, usize)> = (1..=10).map(|i| (format!("area-{i:02}"), 1)).collect();
    folders[8].1 = 2;
    folders[9].1 = 2;
    let list: Vec<(&str, usize)> = folders.iter().map(|f| (f.0.as_str(), f.1)).collect();
    let r = project_with_areas(&list);
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    let line = top_keys(&cfg)
        .lines()
        .find(|l| l.starts_with("areas = "))
        .unwrap_or_else(|| panic!("{cfg}"));
    assert_eq!(
        line.split('#').next().unwrap().trim(),
        "areas = [\"area-01\", \"area-02\", \"area-03\", \"area-04\", \"area-05\", \"area-06\", \"area-09\", \"area-10\"]"
    );
}

#[test]
fn the_folders_of_the_apps_are_areas_if_the_specs_have_none() {
    let r = Repo::empty();
    r.write("services/billing/package.json", "{}\n");
    r.write("services/orders/package.json", "{}\n");
    r.write("services/orders/src/index.js", "// orders\n");
    r.write("docs/specs/overview.md", "# Overview\n");
    r.commit("apps");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        top_keys(&cfg).contains("areas = [\"billing\", \"orders\"]"),
        "{cfg}"
    );
}

#[test]
fn reconfigure_keeps_the_areas_the_groups_and_the_templates() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("areas = [\"valves\", \"sensors\"]\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &format!(
            "{}\n[groups]\noperations = [\"valves\", \"sensors\"]\n\n[templates]\ndir = \"templates\"\n",
            cfg.trim_end()
        ),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        top_keys(&cfg).contains("areas = [\"valves\", \"sensors\"]"),
        "{cfg}"
    );
    assert!(
        cfg.contains("[groups]\noperations = [\"valves\", \"sensors\"]\n"),
        "{cfg}"
    );
    assert!(cfg.contains("[templates]\ndir = \"templates\"\n"), "{cfg}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn reconfigure_keeps_a_root_with_no_default_type_and_the_live_sections() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {{ decision = [\"Status\"] }}\n\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nlive-sections = {{ decision = [] }}\n\n[[roots]]\npath = \"log\"\ntype = \"decision\"\nlive-sections = {{ decision = [\"Status\", \"Progress\"] }}\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    let (code, out, err) = r.init_with("", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    // The root keeps no `default-type`, and it does not become `research`.
    assert!(
        cfg.contains("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n"),
        "{cfg}"
    );
    assert!(!cfg.contains("default-type"), "{cfg}");
    // The default `Status` is not written. A choice of the file is.
    assert!(
        !cfg.contains("live-sections = { decision = [\"Status\"] }"),
        "{cfg}"
    );
    assert!(cfg.contains("live-sections = { decision = [] }"), "{cfg}");
    assert!(
        cfg.contains("live-sections = { decision = [\"Status\", \"Progress\"] }"),
        "{cfg}"
    );
}

#[test]
fn a_new_project_follows_the_demo_layout_with_a_layer_added_later() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("", &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt");
    // The place gives the type: the documents have no `type:` line.
    r.write(
        "docs/requirements/loans.md",
        "---\nstatus: draft\n---\n# Loans\n\n## Checkout\n\n- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.\n",
    );
    r.write(
        "docs/interfaces/http.md",
        "---\ntype: interface\nstatus: draft\n---\n# HTTP API\n\nThe rules are in the [loans](../requirements/loans.md) document.\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    // The small graph has no layer for an interface.
    assert!(
        out.contains("docs/interfaces/http.md:1: error SD303 type-invalid: no layer of spec-dna.toml has type `interface`"),
        "{out}"
    );
    // A change of `spec-dna.toml` is a change of its own (SD503): park the
    // documents while the layer is added.
    let parked = PathBuf::from(format!("{}-parked", r.dir.display()));
    fs::rename(r.dir.join("docs"), &parked).unwrap();
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "interfaces",
        "--types",
        "interface",
        "--moved-from",
        "intent",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    // The skill lists the types of the layers, so it is outdated now.
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("SD502"), "{out}");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("wrote .claude/skills/spec-dna/SKILL.md"),
        "{out}"
    );
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    r.commit("build: add the interfaces layer");
    fs::rename(&parked, r.dir.join("docs")).unwrap();
    assert!(
        r.read(".claude/skills/spec-dna/SKILL.md").contains(
            "Types of this project: product, system, terms, roadmap, questions, rule, requirements, interface, decision, incident, intent, spec, plan."
        )
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["show", "requirements/loans#loan-period"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("due date 14 days later"), "{out}");
}

#[test]
fn the_skill_describes_the_places() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let skill = r.read(".claude/skills/spec-dna/SKILL.md");
    for want in [
        "The folder of a layer gives the type, so write no `type:` line there.",
        "`spec-dna new terms`",
        "`spec-dna new intent <slug>`",
    ] {
        assert!(skill.contains(want), "{want}\n{skill}");
    }
    // The `--json` preview has a null default type for the new root.
    let r2 = Repo::empty();
    let (_, out, _) = r2.run(&["init", "--dry-run", "--non-interactive", "--json"]);
    assert!(out.contains("\"default_type\":null"), "{out}");
}
