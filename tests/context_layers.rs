//! Integration tests of the order of `context` by layer, and of the layers
//! with `context = "on-request"`. `context` lists the layers by level from 0
//! up, and the layers of one level in the order of the configuration. It
//! serves the documents of an `on-request` layer only for a target that names
//! them, and it counts them in one line. A top file enters the first tier
//! only by its `applies-to`. The helpers are small copies of the ones in
//! tests/new_place.rs.

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
        let dir = std::env::temp_dir().join(format!("spec-dna-ctxl-{}-{n}", std::process::id()));
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

    /// One mixed root `docs`, with the implied (default) graph.
    fn mixed() -> Repo {
        Repo::with("", "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n")
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

    /// The ids of the sections that `context` serves in full, in order.
    fn served(&self, args: &[&str]) -> Vec<String> {
        let (code, out, err) = self.run(args);
        assert_eq!(code, 0, "{out}{err}");
        full_ids(&out)
    }
}

/// The ids in the headers of the sections that the output serves in full.
fn full_ids(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.starts_with("<!-- ") && l.contains(" h:"))
        .map(|l| l.split(' ').nth(1).unwrap().to_string())
        .collect()
}

const VALVE: &str = "def close_valve(zone):\n    return zone\n";

fn rule(applies: &str, key: &str) -> String {
    format!(
        "---\ntype: rule\nstatus: active\n{applies}---\n# Watering\n\n## Rules\n\n- **{key}**: The controller SHALL close each valve after 10 minutes.\n"
    )
}

fn product(applies: &str) -> String {
    format!("---\n{applies}---\n# Irrigation product\n\nThe controller waters garden beds.\n")
}

/// A project of the default graph with a top file, a rule, and one document
/// in each `on-request` layer. Each of them names `src/irrigation/valve.py`.
fn irrigation() -> Repo {
    let r = Repo::mixed();
    r.write("src/irrigation/valve.py", VALVE);
    r.write(
        "docs/product.md",
        &product("applies-to: [\"src/irrigation/**\"]\n"),
    );
    r.write(
        "docs/rules/watering.md",
        &rule("applies-to: [\"src/irrigation/**\"]\n", "valve-timeout"),
    );
    r.write(
        "docs/incidents/flooded-bed.md",
        "---\ntype: incident\n---\n# Flooded bed\n\n## Summary\n\nWater ran over one bed. See `src/irrigation/valve.py`.\n\n## Root cause\n\nA valve without a timeout.\n",
    );
    r.write(
        "docs/research/valves.md",
        "---\ntype: research\n---\n# Valve research\n\nNotes on `src/irrigation/valve.py`.\n",
    );
    r.write(
        "docs/intent/timer/intent.md",
        "---\nstatus: draft\n---\n# Timer\n\n## Intent\n\nChange `src/irrigation/valve.py`.\n",
    );
    r
}

#[test]
fn foundation_comes_before_rules_in_the_default_graph() {
    let r = irrigation();
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    let at = |id: &str| ids.iter().position(|i| i.starts_with(id));
    let top = at("product").unwrap_or_else(|| panic!("{ids:?}"));
    let rules = at("rules/watering").unwrap_or_else(|| panic!("{ids:?}"));
    assert!(
        top < rules,
        "the foundation layer comes before the rules: {ids:?}"
    );
    // Both are in the first tier: nothing else comes before them.
    assert_eq!((top, rules), (0, 1), "{ids:?}");
    // The text and the JSON agree on the tier of each section.
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py"]);
    assert!(out.contains("product applies -->"), "{out}");
    assert!(out.contains("rule/active rule -->"), "{out}");
    let (_, json, _) = r.run(&["context", "src/irrigation/valve.py", "--json"]);
    assert!(
        json.contains("\"type\":\"product\"") && json.contains("\"tier\":\"applies\""),
        "{json}"
    );
    assert!(json.contains("\"tier\":\"rule\""), "{json}");
}

#[test]
fn a_top_file_enters_the_first_tier_only_by_its_applies_to() {
    let r = irrigation();
    // A top file with no `applies-to` is not in the context of a task. A rule
    // with no `applies-to` is: it applies to the whole project.
    r.write("docs/product.md", &product(""));
    r.write(
        "docs/rules/units.md",
        &rule("", "metric-units").replace("Watering", "Units"),
    );
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    assert!(
        !ids.iter().any(|i| i.starts_with("product")),
        "a top file without applies-to stays out: {ids:?}"
    );
    assert!(ids.iter().any(|i| i.starts_with("rules/units")), "{ids:?}");
    // A target that names the top file serves it.
    let ids = r.served(&["context", "product"]);
    assert!(ids.iter().any(|i| i.starts_with("product")), "{ids:?}");
    // An `applies-to` that does not match gives no top file either.
    r.write(
        "docs/product.md",
        &product("applies-to: [\"src/billing/**\"]\n"),
    );
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    assert!(!ids.iter().any(|i| i.starts_with("product")), "{ids:?}");
    // A match puts the top file back, before the rules.
    r.write(
        "docs/product.md",
        &product("applies-to: [\"src/irrigation/valve.py\"]\n"),
    );
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    assert!(ids[0].starts_with("product"), "{ids:?}");
}

#[test]
fn an_on_request_layer_is_counted_in_one_line_and_lists_no_file() {
    let r = irrigation();
    let (code, out, err) = r.run(&["context", "src/irrigation/valve.py"]);
    assert_eq!(code, 0, "{out}{err}");
    // The layers in the order of `context`: research and intent have level 1,
    // incidents has level 2.
    let line = "<!-- on-request, not served: research 1, intent 1, incidents 1; name a document as target to serve it -->";
    assert_eq!(out.matches(line).count(), 1, "{out}");
    for id in ["incidents/flooded-bed", "research/valves", "intent/timer"] {
        assert!(!out.contains(id), "{id} is not listed: {out}");
    }
    // The JSON keeps its fields and lists none of them.
    let (_, json, _) = r.run(&["context", "src/irrigation/valve.py", "--json"]);
    assert!(
        !json.contains("flooded-bed") && !json.contains("valves"),
        "{json}"
    );
    // A request that reaches no such document has no line.
    r.write("src/irrigation/pump.py", "def run_pump():\n    return 1\n");
    let (_, out, _) = r.run(&["context", "src/irrigation/pump.py"]);
    assert!(!out.contains("not served"), "{out}");
}

#[test]
fn a_named_target_serves_an_on_request_document() {
    let r = irrigation();
    let ids = r.served(&["context", "incidents/flooded-bed"]);
    assert!(
        ids.iter().any(|i| i == "incidents/flooded-bed#summary"),
        "{ids:?}"
    );
    let (_, out, _) = r.run(&["context", "incidents/flooded-bed"]);
    assert!(out.contains("Water ran over one bed."), "{out}");
    let ids = r.served(&["context", "research/valves"]);
    assert!(
        ids.iter().any(|i| i.starts_with("research/valves")),
        "{ids:?}"
    );
    let ids = r.served(&["context", "intent/timer/intent"]);
    assert!(
        ids.iter().any(|i| i.starts_with("intent/timer/intent")),
        "{ids:?}"
    );
}

#[test]
fn show_reads_an_on_request_document() {
    let r = irrigation();
    let (code, out, err) = r.run(&["show", "incidents/flooded-bed"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("A valve without a timeout."), "{out}");
    let (code, out, _) = r.run(&["show", "research/valves"]);
    assert_eq!(code, 0);
    assert!(out.contains("Notes on"), "{out}");
}

#[test]
fn an_on_request_setting_of_a_layer_changes_the_plan() {
    // The same project with the layer `incidents` set to `always`: the
    // incident is a section that `context` lists, and the count line has no
    // incident.
    let cfg = |ctx: &str| {
        format!(
            "[layers.rules]\ntypes = [\"rule\"]\n[layers.incidents]\ntypes = [\"incident\"]\ncontext = \"{ctx}\"\n"
        )
    };
    let docs = |r: &Repo| {
        r.write("src/irrigation/valve.py", VALVE);
        r.write(
            "docs/incidents/flooded-bed.md",
            "---\ntype: incident\n---\n# Flooded bed\n\n## Summary\n\nWater ran over one bed. See `src/irrigation/valve.py`.\n",
        );
    };
    let roots = "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";
    let r = Repo::with(&cfg("on-request"), roots);
    docs(&r);
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py"]);
    assert!(
        !out.contains("incidents/flooded-bed")
            && out.contains("on-request, not served: incidents 1;"),
        "{out}"
    );
    let r = Repo::with(&cfg("always"), roots);
    docs(&r);
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py"]);
    assert!(out.contains("incidents/flooded-bed"), "{out}");
    assert!(!out.contains("not served"), "{out}");
}

#[test]
fn a_custom_layer_has_its_place_in_the_order() {
    // `pilot` is a custom layer of level 1: its `paths` put a rule in it
    // although the type belongs to `base`. The file order of the documents
    // would put `pilot/limits` first. The level puts the rule of `base`
    // first. `watch` is an `on-request` custom layer.
    let cfg = "[layers.base]\ntypes = [\"rule\"]\n[layers.pilot]\npaths = [\"pilot/**\"]\nmoves-to = [\"base\"]\n[layers.watch]\npaths = [\"watch/**\"]\ncontext = \"on-request\"\n";
    let r = Repo::with(cfg, "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    r.write("src/irrigation/valve.py", VALVE);
    r.write(
        "docs/pilot/limits.md",
        &rule("", "pilot-limit").replace("Watering", "Pilot limits"),
    );
    r.write(
        "docs/rules/zebra.md",
        &rule("", "zebra-timeout").replace("Watering", "Zebra"),
    );
    r.write(
        "docs/watch/log.md",
        "---\ntype: rule\nstatus: active\n---\n# Watch log\n\n## Rules\n\n- **watch-log**: The controller SHALL keep a log of each run.\n",
    );
    let (code, out, err) = r.run(&["context", "src/irrigation/valve.py"]);
    assert_eq!(code, 0, "{out}{err}");
    let ids = full_ids(&out);
    let rules: Vec<&String> = ids
        .iter()
        .filter(|i| i.starts_with("rules/") || i.starts_with("pilot/"))
        .collect();
    assert_eq!(
        rules.first().map(|i| i.split('#').next().unwrap()),
        Some("rules/zebra"),
        "{ids:?}"
    );
    assert!(ids.iter().any(|i| i.starts_with("pilot/limits")), "{ids:?}");
    assert!(!out.contains("watch/log"), "{out}");
    assert!(out.contains("on-request, not served: watch 1;"), "{out}");
}

#[test]
fn layers_of_one_level_keep_the_order_of_the_file() {
    // Two layers of level 0, in this order: `late` and `early`. A rule of
    // `late` comes first, although its id sorts after the id of the other.
    let cfg = "[layers.late]\npaths = [\"zz/**\"]\n[layers.early]\npaths = [\"aa/**\"]\n";
    let r = Repo::with(cfg, "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    r.write("src/irrigation/valve.py", VALVE);
    let doc = |t: &str| rule("", "a-limit").replace("Watering", t);
    r.write("docs/aa/first.md", &doc("First"));
    r.write("docs/zz/last.md", &doc("Last"));
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    let order: Vec<&str> = ids.iter().map(|i| i.split('#').next().unwrap()).collect();
    assert_eq!(order.first(), Some(&"zz/last"), "{ids:?}");
}

#[test]
fn documents_that_cite_a_file_follow_the_order_of_the_layers() {
    // Both documents cite the file by its path, so they have one citation
    // class. The procedure cites it 2 times and the decision once, and the
    // type order of 0.5 put a procedure before a decision. The layer
    // `decisions` comes before the layer `procedures` in the default graph.
    let r = Repo::mixed();
    r.write("src/irrigation/valve.py", VALVE);
    r.write(
        "docs/procedures/restart.md",
        "---\nstatus: active\n---\n# Restart\n\n## Steps\n\nRun `src/irrigation/valve.py` and see `src/irrigation/valve.py`.\n",
    );
    r.write(
        "docs/decisions/valve-timer.md",
        "---\nstatus: accepted\n---\n# Valve timer\n\n## Decision Outcome\n\nUse the timer in `src/irrigation/valve.py`.\n",
    );
    let ids = r.served(&["context", "src/irrigation/valve.py"]);
    let at = |id: &str| ids.iter().position(|i| i.starts_with(id));
    let decision = at("decisions/valve-timer").unwrap_or_else(|| panic!("{ids:?}"));
    let procedure = at("procedures/restart").unwrap_or_else(|| panic!("{ids:?}"));
    assert!(decision < procedure, "{ids:?}");
}
