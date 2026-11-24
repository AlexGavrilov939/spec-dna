//! Integration tests of `spec-dna layer add` and `spec-dna layer list`. The
//! golden files of `layer list` are in tests/golden. To regenerate them, run
//! with `UPDATE_GOLDEN=1`. The helpers are small copies of the ones in
//! tests/new_place.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The tables of the small graph (design, "Default graph", table "Size").
const SMALL: &str = r#"[layers.foundation]
types = ["product", "system", "terms"]
files = ["product.md", "system.md", "terms.md"]
max-chars = 30000

[layers.pending]
types = ["roadmap", "questions"]
files = ["roadmap.md", "questions.md"]
moves-to = ["intent"]
linked-from = ["intent", "requirements", "decisions"]
context = "on-request"
max-docs = 2

[layers.rules]
types = ["rule"]
max-docs = 30

[layers.requirements]
types = ["requirements"]

[layers.decisions]
types = ["decision"]

[layers.intent]
types = ["intent", "spec", "plan"]
moves-to = ["rules", "requirements", "decisions"]
keep = "until-finished"
context = "on-request"
max-docs = 30
max-started = 3
stale-days = 14

[layers.incidents]
types = ["incident"]
moves-to = ["intent"]
linked-from = ["rules", "decisions", "requirements", "intent"]
context = "on-request"
"#;

/// The documents of the golden corpus: the neutral domain is irrigation.
const DOCS: [(&str, &str); 7] = [
    ("docs/terms.md", "# Terms\n"),
    ("docs/rules/valve-pressure.md", "# Valve pressure\n"),
    ("docs/requirements/zones.md", "# Zones\n"),
    (
        "docs/interfaces/sensor-bus.md",
        "---\ntype: interface\n---\n# Sensor bus\n",
    ),
    ("docs/decisions/pump-choice.md", "# Pump choice\n"),
    ("docs/intent/add-drip-lines/intent.md", "# Add drip lines\n"),
    (
        "docs/play/cleaning.md",
        "---\ntype: rule\n---\n# Cleaning\n",
    ),
];

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
    /// A repository with one mixed root `docs`. `layers` is the text after
    /// the root: the tables of the layers, or nothing.
    fn with(layers: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-layer-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let r = Repo { dir };
        r.write_config(&format!(
            "spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n{layers}"
        ));
        r
    }

    /// A repository with a configuration that the test writes after the pin.
    /// Keys and dotted keys of the root table come before the first table.
    fn raw(body: &str) -> Repo {
        let r = Repo::with("");
        r.write_config(&format!("spec-dna = \"{VERSION}\"\n{body}"));
        r
    }

    /// The same, with the documents of the golden corpus.
    fn with_docs(layers: &str) -> Repo {
        let r = Repo::with(layers);
        for (path, text) in DOCS {
            let p = r.dir.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
        git(&r.dir, &["add", "-A"]);
        r
    }

    fn write_config(&self, text: &str) {
        fs::write(self.dir.join("spec-dna.toml"), text).unwrap();
    }

    fn config(&self) -> Vec<u8> {
        fs::read(self.dir.join("spec-dna.toml")).unwrap()
    }

    fn text(&self) -> String {
        String::from_utf8(self.config()).unwrap()
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
}

/// Compares the output with a golden file.
fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(
        actual, expected,
        "golden {name} differs; run with UPDATE_GOLDEN=1 to accept"
    );
}

#[test]
fn layer_list_of_the_small_graph_matches_golden() {
    let r = Repo::with_docs(SMALL);
    let (code, out, err) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{err}");
    golden("layer-list-small.txt", &out);
    let (code, out, err) = r.run(&["layer", "list", "--json"]);
    assert_eq!(code, 0, "{err}");
    golden("layer-list-small.json", &out);
    // The small graph has no `interfaces` layer, so one document has no layer.
    assert!(out.contains("\"unplaced\":1"), "{out}");
}

#[test]
fn layer_list_of_the_default_graph_matches_golden() {
    // The file has no `[layers]` table: the graph is the default graph.
    let r = Repo::with_docs("");
    let (code, out, err) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{err}");
    golden("layer-list-default.txt", &out);
    let (code, out, err) = r.run(&["layer", "list", "--json"]);
    assert_eq!(code, 0, "{err}");
    golden("layer-list-default.json", &out);
    // `layer add` writes the same graph as tables. The list is the same, but
    // for the note and the new layer.
    let (code, _, err) = r.run(&["layer", "add", "notes"]);
    assert_eq!(code, 0, "{err}");
    let (_, written, _) = r.run(&["layer", "list"]);
    assert!(!written.contains("has no [layers] table"), "{written}");
    let golden_text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/layer-list-default.txt"),
    )
    .unwrap();
    let golden_lines: Vec<&str> = golden_text.lines().skip(1).collect();
    let written_lines: Vec<&str> = written.lines().take(golden_lines.len()).collect();
    assert_eq!(written_lines, golden_lines);
    assert!(
        written
            .lines()
            .last()
            .unwrap()
            .starts_with("notes  level 0")
    );
}

#[test]
fn a_refusal_leaves_the_file_byte_equal() {
    let roots = "\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";
    let dotted_no_edge = format!("layers.d.types = [\"decision\"]\n{roots}");
    let inline_no_edge = format!("layers = {{ d = {{ types = [\"decision\"] }} }}\n{roots}");
    let inline_header = format!("{roots}\n[layers]\nd = {{ types = [\"decision\"] }}\n");
    let cases: Vec<(&str, Vec<&str>)> = vec![
        // An existing name.
        (SMALL, vec!["add", "rules"]),
        // A type that another layer has.
        (SMALL, vec!["add", "laws", "--types", "rule"]),
        // A cycle: `intent` moves to `rules`, so `rules` cannot move to `intent`.
        (
            SMALL,
            vec![
                "add",
                "laws",
                "--moved-from",
                "rules",
                "--moves-to",
                "intent",
            ],
        ),
        (SMALL, vec!["add", "laws", "--moves-to", "laws"]),
        // An unknown layer in an edge.
        (SMALL, vec!["add", "laws", "--moves-to", "nowhere"]),
        (SMALL, vec!["add", "laws", "--linked-from", "nowhere"]),
        (SMALL, vec!["add", "laws", "--moved-from", "nowhere"]),
        // A value that is not valid.
        (SMALL, vec!["add", "laws", "--keep", "sometimes"]),
        (SMALL, vec!["add", "laws", "--context", "never"]),
        (SMALL, vec!["add", "laws", "--max-docs", "many"]),
        (SMALL, vec!["add", "laws", "--max-chars", "-1"]),
        (SMALL, vec!["add", "laws", "--types", "contract"]),
        (SMALL, vec!["add", "laws", "--types", "rule,,decision"]),
        (SMALL, vec!["add", "bad name"]),
        (SMALL, vec!["add", "laws", "--dir", "../up"]),
        (SMALL, vec!["add", "laws", "--dir", "/abs"]),
        (
            SMALL,
            vec!["add", "laws", "--files", "x.md", "--dir", "laws"],
        ),
        (SMALL, vec!["add", "laws", "--dir", "rules"]),
        (SMALL, vec!["add", "laws", "--paths", ""]),
        (SMALL, vec!["add"]),
        (SMALL, vec!["list", "--types", "rule"]),
        // A layer with no `moves-to` key, in the dotted form: edit by hand.
        (&dotted_no_edge, vec!["add", "n", "--moved-from", "d"]),
        (&inline_no_edge, vec!["add", "n", "--moved-from", "d"]),
        (&inline_header, vec!["add", "n", "--moved-from", "d"]),
        // The implied graph (no table) has no layer `nowhere`.
        ("", vec!["add", "laws", "--moved-from", "nowhere"]),
        ("", vec!["add", "decisions"]),
    ];
    for (layers, args) in cases {
        // A text with `[[roots]]` in it is a whole file.
        let r = if layers.contains("[[roots]]") {
            Repo::raw(layers)
        } else {
            Repo::with(layers)
        };
        let before = r.config();
        let mut full = vec!["layer"];
        full.extend(&args);
        let (code, out, err) = r.run(&full);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert_eq!(out, "", "{args:?}");
        assert_eq!(r.config(), before, "{args:?}");
        assert!(!r.dir.join("spec-dna.spec-dna.tmp").exists(), "{args:?}");
    }
}

#[test]
fn a_refusal_names_its_reason() {
    let r = Repo::with(SMALL);
    for (args, needle) in [
        (vec!["add", "rules"], "layer `rules` exists"),
        (
            vec!["add", "laws", "--types", "rule"],
            "type `rule` is already in layer `rules`",
        ),
        (
            vec![
                "add",
                "laws",
                "--moved-from",
                "rules",
                "--moves-to",
                "intent",
            ],
            "cycle",
        ),
        (
            vec!["add", "laws", "--moves-to", "nowhere"],
            "layer `nowhere` is not in `[layers]`",
        ),
        (vec!["add", "laws", "--types", "contract"], "interface"),
    ] {
        let mut full = vec!["layer"];
        full.extend(&args);
        let (code, _, err) = r.run(&full);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(err.contains(needle), "{args:?}: {err}");
    }
}

#[test]
fn moved_from_keeps_every_comment() {
    let layers = r#"# The layers of the irrigation docs.
[layers.rules] # stable
types = ["rule"]
# The edge below is a comment.
moves-to = [
    "decisions", # first
    "requirements", # second
]

[layers.requirements]
types = ["requirements"]

[layers.decisions] # no edge yet
types = ["decision"]
"#;
    let r = Repo::with(layers);
    let before = r.text();
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "interfaces",
        "--types",
        "interface",
        "--moved-from",
        "rules,decisions,requirements",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    // The file is the old text, with 3 insertions and the new table.
    let expected = before
        .replace(
            "    \"requirements\", # second\n]",
            "    \"requirements\", \"interfaces\", # second\n]",
        )
        .replace(
            "[layers.decisions] # no edge yet\n",
            "[layers.decisions] # no edge yet\nmoves-to = [\"interfaces\"]\n",
        )
        .replace(
            "[layers.requirements]\n",
            "[layers.requirements]\nmoves-to = [\"interfaces\"]\n",
        );
    assert_eq!(
        r.text(),
        format!("{expected}\n[layers.interfaces]\ntypes = [\"interface\"]\n")
    );
    // The levels follow: `interfaces` is the most stable layer.
    let (_, list, _) = r.run(&["layer", "list"]);
    assert!(list.contains("interfaces  level 0"), "{list}");
    assert!(list.contains("rules  level 2"), "{list}");
    // The output tells the user to commit the file alone.
    assert!(out.contains("added layer `interfaces` (level 0)"), "{out}");
    assert!(
        out.contains("next: commit spec-dna.toml as a change of its own (SD503)"),
        "{out}"
    );
    assert!(out.contains("SD503"), "{out}");
}

#[test]
fn moved_from_works_in_the_dotted_and_the_inline_form_with_an_edge() {
    let roots = "\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";
    let dotted = format!(
        "layers.a.types = [\"rule\"]\nlayers.a.moves-to = [\"b\"] # edge\nlayers.b.types = [\"decision\"]\n{roots}"
    );
    let r = Repo::raw(&dotted);
    let (code, out, err) = r.run(&["layer", "add", "n", "--moved-from", "a"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.text()
            .contains("layers.a.moves-to = [\"b\", \"n\"] # edge\n"),
        "{}",
        r.text()
    );
    let inline = format!("layers = {{ a = {{ moves-to = [\"b\"] }}, b = {{}} }}\n{roots}");
    let r = Repo::raw(&inline);
    let before = r.config();
    // The new table cannot follow an inline `layers`: edit by hand.
    let (code, out, err) = r.run(&["layer", "add", "n", "--moved-from", "a"]);
    assert_eq!(code, 2, "{out}{err}");
    assert_eq!(r.config(), before);
}

#[test]
fn a_header_layer_without_moves_to_gets_a_line() {
    let r = Repo::with("[layers.a]\ntypes = [\"rule\"]\n[layers.b]\ntypes = [\"decision\"]\n");
    let (code, out, err) = r.run(&["layer", "add", "n", "--moved-from", "b"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.text().ends_with(
            "[layers.a]\ntypes = [\"rule\"]\n[layers.b]\nmoves-to = [\"n\"]\ntypes = [\"decision\"]\n\n[layers.n]\n"
        ),
        "{}",
        r.text()
    );
}

#[test]
fn layer_add_on_a_file_without_a_layers_table_writes_the_graph_first() {
    let r = Repo::with("");
    let head = r.text();
    let (code, out, err) = r.run(&["layer", "add", "notes", "--types", "terms,roadmap"]);
    // `terms` and `roadmap` are in the implied graph.
    assert_eq!(code, 2, "{out}{err}");
    assert_eq!(r.text(), head);

    let (code, out, err) = r.run(&["layer", "add", "notes", "--keep", "until-finished"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("had no [layers] table") && out.contains("default graph"),
        "{out}"
    );
    let text = r.text();
    // The old text stays at the start. The 10 tables of the default graph
    // come next, and the new table comes last.
    assert!(text.starts_with(&head), "{text}");
    assert!(
        text.ends_with("\n\n[layers.notes]\nkeep = \"until-finished\"\n"),
        "{text}"
    );
    assert_eq!(text.matches("[layers.").count(), 11);
    let (_, list, _) = r.run(&["layer", "list"]);
    let names: Vec<&str> = list
        .lines()
        .map(|l| l.split("  ").next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
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
            "notes"
        ]
    );
    // A second layer goes to the end, in the declared graph.
    let (code, out, err) = r.run(&["layer", "add", "extra", "--moved-from", "notes"]);
    assert_eq!(code, 0, "{err}");
    assert!(!out.contains("had no [layers] table"), "{out}");
    assert!(
        r.text()
            .contains("[layers.notes]\nmoves-to = [\"extra\"]\nkeep = \"until-finished\"\n"),
        "{}",
        r.text()
    );
    assert!(r.text().ends_with("\n\n[layers.extra]\n"), "{}", r.text());
}

#[test]
fn the_types_list_decides_the_implied_graph_that_layer_add_writes() {
    // `types` is a key of the root table: it comes before the tables.
    let r = Repo::raw(
        "types = [\"rule\", \"intent\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    let (code, out, err) = r.run(&["layer", "add", "notes", "--moved-from", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.text();
    assert_eq!(t.matches("[layers.").count(), 3, "{t}");
    // The edges to the layers that the graph drops are gone.
    assert!(
        t.contains("[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"rules\", \"notes\"]\n"),
        "{t}"
    );
}

#[test]
fn a_custom_layer_by_paths_places_documents_by_their_path() {
    let r = Repo::with_docs("");
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "playbooks",
        "--paths",
        "play/**",
        "--max-docs",
        "5",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.text()
            .ends_with("\n\n[layers.playbooks]\npaths = [\"play/**\"]\nmax-docs = 5\n"),
        "{}",
        r.text()
    );
    // The document `play/cleaning.md` has the type `rule`, and the glob wins.
    let (code, list, err) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{err}");
    let line = list
        .lines()
        .find(|l| l.starts_with("playbooks"))
        .unwrap_or_else(|| panic!("{list}"));
    assert_eq!(
        line,
        "playbooks  level 0  paths play/**  keep forever  context always  max-docs 5  docs 1 (paths 1)"
    );
    let rules = list.lines().find(|l| l.starts_with("rules")).unwrap();
    assert!(rules.ends_with("docs 1 (type 1)"), "{rules}");
    // The JSON lists the rule of each document.
    let (_, json, _) = r.run(&["layer", "list", "--json"]);
    assert!(
        json.contains(
            "{\"path\":\"docs/play/cleaning.md\",\"id\":\"play/cleaning\",\"type\":\"rule\",\"layer\":\"playbooks\",\"rule\":\"paths\"}"
        ),
        "{json}"
    );
    assert!(
        json.contains(
            "{\"path\":\"docs/terms.md\",\"id\":\"terms\",\"type\":\"terms\",\"layer\":\"foundation\",\"rule\":\"files\"}"
        ),
        "{json}"
    );
    assert!(
        json.contains("\"path\":\"docs/rules/valve-pressure.md\",\"id\":\"rules/valve-pressure\",\"type\":\"rule\",\"layer\":\"rules\",\"rule\":\"type\""),
        "{json}"
    );
}

#[test]
fn layer_add_appends_its_types_to_the_types_list_and_prints_the_next_steps() {
    // `types` is a key of the root table: it comes before the tables. The
    // project has the agent skill, which lists the types of the project.
    let body = "# The types of the project.\ntypes = [\"requirements\", \"decision\"]  # keep me\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";
    let r = Repo::raw(body);
    let skill = r.dir.join(".claude/skills/spec-dna/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(&skill, "skill\n").unwrap();
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "research",
        "--types",
        "research",
        "--moves-to",
        "decisions",
        "--linked-from",
        "decisions",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.text();
    assert!(
        t.contains("# The types of the project.\ntypes = [\"requirements\", \"decision\", \"research\"]  # keep me\n"),
        "{t}"
    );
    assert!(
        out.contains("added the types `research` to `types`\n"),
        "{out}"
    );
    // The skill has no generated block, so it stays and `init` updates it (D90).
    assert!(
        out.contains("next: commit spec-dna.toml and the files that init writes in a pull request of their own, and merge it (SD503)\n"),
        "{out}"
    );
    assert!(
        out.contains("next: run spec-dna init to update the agent skill, which lists the types of the project\n"),
        "{out}"
    );
    assert!(
        out.ends_with(
            "next: run spec-dna new research <id> to write the first document of the layer\n"
        ),
        "{out}"
    );
    assert_eq!(out.matches("next: ").count(), 3, "{out}");
    // The type is now a type of the project.
    let (code, out, err) = r.run(&["new", "research", "r"]);
    assert_eq!(code, 0, "{out}{err}");

    // With no skill file, the second step is absent. A type that the list
    // has already is not added twice. `--json` has both fields.
    let r = Repo::raw(body);
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "notes",
        "--types",
        "decision,research",
        "--json",
    ]);
    // `decision` has a layer already, so the command stops.
    assert_eq!(code, 2, "{out}{err}");
    let (code, out, err) = r.run(&["layer", "add", "notes", "--types", "research", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"types_added\":[\"research\"],\"skill_updated\":false,\"next\":[\"commit spec-dna.toml as a change of its own (SD503)\",\"run spec-dna new research <id> to write the first document of the layer\"]"),
        "{out}"
    );

    // A list on many lines cannot grow in place: exit code 2, and the file
    // keeps its bytes.
    let r = Repo::raw(
        "types = [\n  \"requirements\",\n  \"decision\",\n]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    let head = r.config();
    let (code, out, err) = r.run(&["layer", "add", "notes", "--types", "research"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("add `research` to it by hand"), "{err}");
    assert_eq!(r.config(), head);

    // A file with no `types` key stays as it is: each type is valid.
    let r = Repo::with(SMALL);
    let (code, out, err) = r.run(&["layer", "add", "interfaces", "--types", "interface"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.text().contains("types = [\"interface\"]\n\n[layers"));
    assert!(!out.contains("added the types"), "{out}");
    assert!(
        out.contains("next: commit spec-dna.toml as a change of its own (SD503)\n"),
        "{out}"
    );
}

#[test]
fn layer_add_json_and_the_help() {
    let r = Repo::with(SMALL);
    let (code, out, err) = r.run(&[
        "layer",
        "add",
        "interfaces",
        "--types",
        "interface",
        "--moved-from",
        "intent",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out.trim(),
        "{\"layer\":\"interfaces\",\"level\":0,\"path\":\"spec-dna.toml\",\"wrote_implied_graph\":false,\"moved_from\":[\"intent\"],\"types_added\":[],\"skill_updated\":false,\"next\":[\"commit spec-dna.toml as a change of its own (SD503)\",\"run spec-dna new interface <id> to write the first document of the layer\"]}"
    );
    // `layer` without a command, and an unknown command, are usage errors.
    for args in [vec!["layer"], vec!["layer", "remove", "x"]] {
        let (code, _, err) = r.run(&args);
        assert_eq!(code, 2, "{err}");
    }
    let (code, out, _) = r.run(&["layer", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("usage: spec-dna layer add <name>") && out.contains("layer list"),
        "{out}"
    );
}

#[test]
fn layer_add_summaries_adds_the_layer_of_the_type_summary() {
    // Design 0.8.0, D50. The implied graph has no layer `summaries`, so the
    // file gets the tables of the implied graph first, then the new layer,
    // which has the type `summary` with no `--types`.
    let r = Repo::with("");
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.text();
    assert_eq!(text.matches("[layers.").count(), 11, "{text}");
    assert!(
        text.ends_with("\n\n[layers.summaries]\ntypes = [\"summary\"]\n"),
        "{text}"
    );
    assert!(
        out.contains("next: run spec-dna new summary <id> to write the first document"),
        "{out}"
    );
    let (_, list, _) = r.run(&["layer", "list"]);
    assert!(
        list.lines()
            .last()
            .unwrap()
            .starts_with("summaries  level 0"),
        "{list}"
    );
    // The layer works: a summary of an area is a file of the layer.
    let doc = r.dir.join("docs/requirements/valves/closing.md");
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    fs::write(
        doc,
        "---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close-on-timeout**: When the sensor stops, the valve SHALL close after 30 seconds.\n",
    )
    .unwrap();
    let (code, out, err) = r.run(&["new", "summary", "valves"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join("docs/summaries/valves.md").is_file());
    // A second run does not replace the layer.
    let before = r.text();
    let (code, _, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 2, "{err}");
    assert_eq!(r.text(), before);
}

#[test]
fn layer_add_summaries_with_its_own_types_keeps_the_types_of_the_user() {
    // The default type applies only when no option names the content of the
    // layer.
    let r = Repo::with("");
    let (code, out, err) = r.run(&["layer", "add", "summaries", "--types", "research"]);
    assert_eq!(code, 2, "{out}{err}");
    let r = Repo::with("");
    let (code, out, err) = r.run(&["layer", "add", "summaries", "--paths", "docs/summaries/**"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.text().contains("types = [\"summary\"]"), "{}", r.text());
}

const SKILL: &str = ".claude/skills/spec-dna/SKILL.md";

/// A project that `init` wrote: it has the generated agent skill, which lists
/// the types of the project.
fn initialized() -> Repo {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("spec-dna-layer-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let r = Repo { dir };
    let answers = format!("{}-answers.toml", r.dir.display());
    fs::write(&answers, "profile = \"standard\"\n").unwrap();
    let (code, out, err) = r.run(&["init", "--answers", &answers]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(SKILL).is_file());
    // `check` reads the files that git tracks.
    git(&r.dir, &["add", "-A"]);
    r
}

#[test]
fn layer_add_updates_the_adapter_skill() {
    // D90: the skill lists the types of the project, so the same write gives
    // the new skill and `check` has no SD502 for it.
    let r = initialized();
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, checked, _) = r.run(&["check", "--all"]);
    assert!(!checked.contains("SD502"), "{checked}");
    assert!(
        out.contains(&format!(
            "updated {SKILL}: it lists the types of the project\n"
        )),
        "{out}"
    );
    assert!(
        out.contains(&format!("next: commit spec-dna.toml and {SKILL} in a pull request of their own, and merge it (SD503)\n")),
        "{out}"
    );
    assert!(
        out.ends_with("next: after the merge, run spec-dna new summary <id> to write the first document of the layer\n"),
        "{out}"
    );
    assert!(!out.contains("run spec-dna init"), "{out}");
    // The new skill lists the type.
    assert!(r.dir.join(SKILL).is_file());
    // `--json` has the key after `types_added`.
    let r = initialized();
    let (code, out, err) = r.run(&["layer", "add", "summaries", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"types_added\":[],\"skill_updated\":true,\"next\":["),
        "{out}"
    );
}

#[test]
fn a_hand_edited_skill_stays_and_the_init_line_comes_first() {
    let r = initialized();
    let text = fs::read_to_string(r.dir.join(SKILL)).unwrap();
    let edited = text.replace(
        "<!-- spec-dna:end skill -->",
        "A line of the project.\n<!-- spec-dna:end skill -->",
    );
    assert_ne!(text, edited);
    fs::write(r.dir.join(SKILL), &edited).unwrap();
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 0, "{out}{err}");
    // The file keeps its bytes.
    assert_eq!(fs::read_to_string(r.dir.join(SKILL)).unwrap(), edited);
    assert!(!out.contains("updated "), "{out}");
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("next: ")).collect();
    assert_eq!(
        lines[0],
        "next: run spec-dna init to update the agent skill, which lists the types of the project",
        "{out}"
    );
    assert_eq!(
        lines[1],
        "next: commit spec-dna.toml and the files that init writes in a pull request of their own, and merge it (SD503)",
        "{out}"
    );
    let r2 = initialized();
    fs::write(r2.dir.join(SKILL), &edited).unwrap();
    let (_, json, _) = r2.run(&["layer", "add", "summaries", "--json"]);
    assert!(json.contains("\"skill_updated\":false"), "{json}");
}

#[test]
fn dry_run_writes_nothing_and_says_what_it_would_do() {
    let r = initialized();
    let toml = fs::read(r.dir.join("spec-dna.toml")).unwrap();
    let skill = fs::read(r.dir.join(SKILL)).unwrap();
    let (code, out, err) = r.run(&["layer", "add", "summaries", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        fs::read(r.dir.join("spec-dna.toml")).unwrap(),
        toml,
        "{out}"
    );
    assert_eq!(fs::read(r.dir.join(SKILL)).unwrap(), skill, "{out}");
    assert!(out.contains("would add layer `summaries`"), "{out}");
    assert!(
        out.contains(&format!(
            "would update {SKILL}: it lists the types of the project"
        )),
        "{out}"
    );
    assert!(
        out.ends_with("spec-dna layer add: dry run; nothing written\n"),
        "{out}"
    );
    // The real run still works: the dry run left no layer.
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_unwritable_skill_folder_leaves_the_configuration_as_it_was() {
    use std::os::unix::fs::PermissionsExt;
    let r = initialized();
    let toml = fs::read(r.dir.join("spec-dna.toml")).unwrap();
    let dir = r.dir.join(".claude/skills/spec-dna");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    let still = fs::read(r.dir.join("spec-dna.toml")).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    // A process of the super user can write the folder: nothing to test.
    if code == 0 {
        return;
    }
    assert_eq!(code, 4, "{out}{err}");
    assert_eq!(still, toml, "the file of the configuration changed");
    // The second run does not refuse with "layer exists".
    let (code, out, err) = r.run(&["layer", "add", "summaries"]);
    assert_eq!(code, 0, "{out}{err}");
}
