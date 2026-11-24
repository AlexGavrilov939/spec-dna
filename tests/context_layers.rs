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

// ---------------------------------------------------------------------------
// `context --group`
// ---------------------------------------------------------------------------

/// A project with 2 areas and 2 groups. Each requirements document of an
/// area names `src/irrigation/valve.py`. A rule has no area, and it applies
/// to the whole project.
fn grouped() -> Repo {
    let r = Repo::with(
        "areas = [\"valves\", \"sensors\"]\n\n[groups]\noperations = [\"valves\"]\nreporting = [\"sensors\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nkeep = \"until-finished\"\n\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.write("src/irrigation/valve.py", VALVE);
    let req = |title: &str, key: &str| {
        format!(
            "---\nstatus: active\n---\n# {title}\n\n## Requirements\n\n- **{key}**: The controller SHALL close the valve in `src/irrigation/valve.py` in 5 seconds.\n"
        )
    };
    r.write(
        "docs/requirements/valves/closing.md",
        &req("Closing", "close"),
    );
    r.write(
        "docs/requirements/sensors/timeouts.md",
        &req("Timeouts", "timeout"),
    );
    r.write(
        "docs/rules/watering.md",
        &rule("applies-to: [\"src/irrigation/**\"]\n", "valve-timeout"),
    );
    r
}

#[test]
fn a_group_serves_only_the_areas_of_the_group() {
    let r = grouped();
    // Without a group, both areas are in the plan.
    let all = r.served(&["context", "src/irrigation/valve.py"]);
    assert!(
        all.iter()
            .any(|i| i.starts_with("requirements/valves/closing"))
    );
    assert!(
        all.iter()
            .any(|i| i.starts_with("requirements/sensors/timeouts"))
    );
    // With the group, the other area is not served and not listed.
    let (code, out, err) = r.run(&[
        "context",
        "src/irrigation/valve.py",
        "--group",
        "operations",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let ids = full_ids(&out);
    assert!(
        ids.iter()
            .any(|i| i.starts_with("requirements/valves/closing")),
        "{ids:?}"
    );
    assert!(!out.contains("sensors/timeouts"), "{out}");
    // A rule has no area: the group does not change it.
    assert!(
        ids.iter().any(|i| i.starts_with("rules/watering")),
        "{ids:?}"
    );
    // One line counts what the group holds back, in the text output only.
    let line = "<!-- group operations: 1 document of another area not served; name a document as target to serve it -->";
    assert_eq!(out.matches(line).count(), 1, "{out}");
    // The other group serves the other area. The form `--group=name` works.
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py", "--group=reporting"]);
    assert!(out.contains("requirements/sensors/timeouts"), "{out}");
    assert!(!out.contains("valves/closing"), "{out}");
}

#[test]
fn a_target_serves_a_document_of_another_area() {
    let r = grouped();
    let ids = r.served(&[
        "context",
        "requirements/sensors/timeouts",
        "--group",
        "operations",
    ]);
    assert!(
        ids.iter()
            .any(|i| i.starts_with("requirements/sensors/timeouts")),
        "a named target is served in any group: {ids:?}"
    );
}

#[test]
fn the_group_keeps_the_json_shape_and_changes_nothing_without_it() {
    let r = grouped();
    let (_, plain, _) = r.run(&["context", "src/irrigation/valve.py", "--json"]);
    let (code, json, err) = r.run(&[
        "context",
        "src/irrigation/valve.py",
        "--json",
        "--group",
        "operations",
    ]);
    assert_eq!(code, 0, "{json}{err}");
    // The same keys, and no count.
    let keys = |s: &str| -> Vec<String> {
        s.split("\"units\"")
            .next()
            .unwrap()
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|k| {
                [
                    "estimated_tokens",
                    "version",
                    "entries",
                    "paths",
                    "budget",
                    "used",
                    "full",
                    "stubs",
                    "omitted",
                    "units",
                ]
                .contains(k)
            })
            .map(str::to_string)
            .collect()
    };
    assert_eq!(keys(&plain), keys(&json), "{json}");
    assert!(
        !json.contains("another area") && !json.contains("sensors/timeouts"),
        "{json}"
    );
    assert!(plain.contains("sensors/timeouts"), "{plain}");
    // Without the option, no line about a group.
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py"]);
    assert!(!out.contains("group"), "{out}");
}

#[test]
fn an_unknown_group_is_a_usage_error() {
    let r = grouped();
    let (code, _, err) = r.run(&["context", "src/irrigation/valve.py", "--group", "billing"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("billing") && err.contains("operations"),
        "{err}"
    );
    // A project with no `[groups]` table.
    let r = irrigation();
    let (code, _, err) = r.run(&[
        "context",
        "src/irrigation/valve.py",
        "--group",
        "operations",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("no [groups] table"), "{err}");
}

#[test]
fn a_group_holds_back_a_dependency_and_an_intent_of_another_area() {
    let r = grouped();
    r.write(
        "docs/requirements/valves/closing.md",
        "---\nstatus: active\ndepends-on: [requirements/sensors/timeouts]\n---\n# Closing\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n",
    );
    r.write(
        "docs/intent/report/intent.md",
        "---\nstatus: draft\nareas: [sensors]\n---\n# Report\n\n## Intent\n\nChange `src/irrigation/valve.py`.\n",
    );
    // Without a group, the entry pulls its dependency.
    let ids = r.served(&["context", "requirements/valves/closing"]);
    assert!(
        ids.iter()
            .any(|i| i.starts_with("requirements/sensors/timeouts")),
        "{ids:?}"
    );
    // With the group, the dependency of another area is counted and not served.
    let (code, out, err) = r.run(&[
        "context",
        "requirements/valves/closing",
        "--group",
        "operations",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("sensors/timeouts#"), "{out}");
    assert!(
        out.contains("<!-- group operations: 1 document of another area not served;"),
        "{out}"
    );
    // An intent with `areas` that are not in the group stays out too. The
    // request reaches it and the timeouts document: 2 documents.
    let (_, out, _) = r.run(&["context", "src/irrigation/valve.py"]);
    assert!(out.contains("report/intent"), "{out}");
    let (_, out, _) = r.run(&[
        "context",
        "src/irrigation/valve.py",
        "--group",
        "operations",
    ]);
    assert!(!out.contains("report/intent"), "{out}");
    assert!(
        out.contains("<!-- group operations: 2 documents of another area not served;"),
        "{out}"
    );
}

#[test]
fn a_group_that_holds_back_every_document_says_so_and_does_not_say_none_cites() {
    let r = grouped();
    // The rule goes, and the document of the group `reporting` does not cite
    // the file. The only citing document is of another area.
    std::fs::remove_file(r.dir.join("docs/rules/watering.md")).unwrap();
    r.write(
        "docs/requirements/sensors/timeouts.md",
        "---\nstatus: active\n---\n# Timeouts\n\n## Requirements\n\n- **timeout**: The controller SHALL report a sensor timeout.\n",
    );
    let (code, out, err) = r.run(&["context", "src/irrigation/valve.py", "--group", "reporting"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("<!-- group reporting: 1 document of another area not served;"),
        "{out}"
    );
    assert!(
        !out.contains("no document cites or applies to"),
        "the output must not say that no document cites the file: {out}"
    );
}

// ---------------------------------------------------------------------------
// The tier `summary` (design 0.8.0, D56)
// ---------------------------------------------------------------------------

/// The layers of a project with a roll-up: the layer `summaries` is explicit,
/// because the implied graph has no such layer. `rules/style` is the
/// convention of the requirements documents.
const ROLL_HEAD: &str = "[conventions]\nrequirements = [\"rules/style\"]\n\n[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n";

const ROLL_ROOTS: &str = "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

fn requirements(title: &str, key: &str, text: &str) -> String {
    format!("---\nstatus: active\n---\n# {title}\n\n## Requirements\n\n- **{key}**: {text}\n")
}

/// The ids of the sections of the output, without the anchor, in order and
/// with each document once.
fn docs_of(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in ids {
        let d = i.split('#').next().unwrap().to_string();
        if out.last() != Some(&d) {
            out.push(d);
        }
    }
    out
}

/// The header line of the first section of a document in the output.
fn header_of<'a>(out: &'a str, doc: &str) -> &'a str {
    out.lines()
        .find(|l| l.starts_with(&format!("<!-- {doc}#")) || l.starts_with(&format!("<!-- {doc} ")))
        .unwrap_or_else(|| panic!("no header of {doc}: {out}"))
}

/// The full sections of the output: the header line, and the characters that
/// the section costs (the header with its line end, and the text).
fn sections(out: &str) -> Vec<(String, usize)> {
    let mut found: Vec<(String, String)> = Vec::new();
    // The first line is the head of the output.
    for line in out.lines().skip(1) {
        if line.starts_with("<!-- ") && line.contains(" h:") {
            found.push((line.to_string(), String::new()));
        } else if line.starts_with("<!--") || line.starts_with("- `") {
            // A note or the list of stubs ends the last section.
            break;
        } else if let Some(last) = found.last_mut() {
            last.1.push_str(line);
            last.1.push('\n');
        }
    }
    found
        .into_iter()
        .map(|(h, t)| {
            let cost = h.chars().count() + 1 + t.trim_end().chars().count();
            (h, cost)
        })
        .collect()
}

/// The characters of the full sections of one tier.
fn tier_chars(out: &str, tier: &str) -> usize {
    sections(out)
        .iter()
        .filter(|(h, _)| {
            h.ends_with(&format!(" {tier} -->")) || h.ends_with(&format!(" {tier} stale -->"))
        })
        .map(|(_, n)| n)
        .sum()
}

/// Writes the summary of `area` with `new summary` and gives it `body` under
/// the title, so the pins are the pins of the tree now.
fn summary(r: &Repo, area: &str, body: &str) {
    let (code, out, err) = r.run(&["new", "summary", area]);
    assert_eq!(code, 0, "{out}{err}");
    let path = format!("docs/summaries/{area}.md");
    let text = fs::read_to_string(r.dir.join(&path)).unwrap();
    let at = text.find("\n---\n").expect("front matter") + "\n---\n".len();
    let title = area.rsplit('/').next().unwrap();
    r.write(&path, &format!("{}\n# {title}\n\n{body}\n", &text[..at]));
}

/// A roll-up document at the top: `rests-on: []` and then `refresh --holds`.
/// The plain `refresh` refuses a text that is the text of the base.
fn top(r: &Repo, path: &str, id: &str, title: &str, body: &str) {
    r.write(
        path,
        &format!("---\nrests-on: []\n---\n# {title}\n\n{body}\n"),
    );
    let (code, out, err) = r.run(&["refresh", id, "--holds"]);
    assert_eq!(code, 0, "{out}{err}");
}

/// A valve project with a full chain: `product`, `system`, `summaries/valves`
/// and `summaries/valves/drives`, all with current pins. The areas are
/// `valves`, `valves/drives` and `sensors` (the last one has no summary). A
/// requirements document at the root has no area. 2 rules: `rules/style` is a
/// convention, `rules/watering` applies to the whole project.
fn rollup_with(valves_body: &str) -> Repo {
    rollup_bodies(valves_body, "The product waters garden beds.")
}

/// The same, with the body of `product` too.
fn rollup_bodies(valves_body: &str, product_body: &str) -> Repo {
    let r = Repo::with(ROLL_HEAD, ROLL_ROOTS);
    r.write("src/irrigation/valve.py", VALVE);
    r.write(
        "docs/rules/style.md",
        &rule("", "short-lines").replace("Watering", "Style"),
    );
    r.write("docs/rules/watering.md", &rule("", "valve-timeout"));
    r.write(
        "docs/requirements/valves/closing.md",
        &requirements(
            "Closing",
            "close-on-timeout",
            "The valve SHALL close after 30 seconds. See `src/irrigation/valve.py`.",
        ),
    );
    r.write(
        "docs/requirements/valves/drives/motor.md",
        &requirements(
            "Motor",
            "spin-up",
            "The motor SHALL reach full speed in 2 seconds.",
        ),
    );
    r.write(
        "docs/requirements/sensors/probes.md",
        &requirements(
            "Probes",
            "probe-count",
            "The controller SHALL read 4 probes.",
        ),
    );
    r.write(
        "docs/requirements/zones.md",
        &requirements(
            "Zones",
            "zone-count",
            "The controller SHALL support 8 zones.",
        ),
    );
    summary(&r, "valves/drives", "A drive spins up in 2 seconds.");
    summary(&r, "valves", valves_body);
    top(
        &r,
        "docs/system.md",
        "system",
        "Irrigation system",
        "The system waters beds in zones.",
    );
    top(
        &r,
        "docs/product.md",
        "product",
        "Irrigation product",
        product_body,
    );
    r
}

fn rollup() -> Repo {
    rollup_with("A valve closes after 30 seconds.")
}

/// A chain whose summary of `valves` has a body of `words` times 16
/// characters.
fn large_summary(words: usize) -> Repo {
    rollup_with(&"A valve closes. ".repeat(words))
}

/// Edits the requirement `valves/closing`, so that `summaries/valves` is stale.
fn make_valves_stale(r: &Repo) {
    let text = fs::read_to_string(r.dir.join("docs/requirements/valves/closing.md")).unwrap();
    r.write(
        "docs/requirements/valves/closing.md",
        &text.replace("30 seconds", "20 seconds"),
    );
}

#[test]
fn the_summary_tier_is_after_the_conventions_and_before_the_rules_top_down() {
    let r = rollup();
    let (code, out, err) = r.run(&["context", "requirements/valves/closing"]);
    assert_eq!(code, 0, "{out}{err}");
    let docs = docs_of(&full_ids(&out));
    assert_eq!(
        docs[..6],
        [
            "rules/style",
            "product",
            "system",
            "summaries/valves",
            "rules/watering",
            "requirements/valves/closing"
        ],
        "{docs:?}\n{out}"
    );
    // The area of the entry is `valves`: the deeper summary is not served.
    assert!(!out.contains("summaries/valves/drives"), "{out}");
    // The header keeps the form of 0.7.1: the type, then the tier.
    assert!(
        header_of(&out, "product").ends_with(" product summary -->"),
        "{out}"
    );
    assert!(
        header_of(&out, "system").ends_with(" system summary -->"),
        "{out}"
    );
    assert!(
        header_of(&out, "summaries/valves").ends_with(" summary summary -->"),
        "{out}"
    );
    assert!(
        header_of(&out, "rules/style").ends_with(" convention -->"),
        "{out}"
    );
}

#[test]
fn a_deeper_area_adds_its_summary_after_the_shallow_one() {
    let r = rollup();
    let docs = docs_of(&r.served(&["context", "requirements/valves/drives/motor"]));
    assert_eq!(
        docs[..6],
        [
            "rules/style",
            "product",
            "system",
            "summaries/valves",
            "summaries/valves/drives",
            "rules/watering"
        ],
        "{docs:?}"
    );
}

#[test]
fn a_document_of_the_root_area_reaches_only_the_top_documents() {
    let r = rollup();
    let docs = docs_of(&r.served(&["context", "requirements/zones"]));
    assert_eq!(
        docs[..4],
        ["rules/style", "product", "system", "rules/watering"],
        "{docs:?}"
    );
    assert!(
        !docs.iter().any(|d| d.starts_with("summaries/")),
        "{docs:?}"
    );
}

#[test]
fn an_area_with_no_summary_adds_the_top_documents_only() {
    // `sensors` has no summary, so the chain is `product` and `system`.
    let r = rollup();
    let docs = docs_of(&r.served(&["context", "requirements/sensors/probes"]));
    assert_eq!(
        docs[..4],
        ["rules/style", "product", "system", "rules/watering"],
        "{docs:?}"
    );
    assert!(
        !docs.iter().any(|d| d.starts_with("summaries/")),
        "{docs:?}"
    );
}

#[test]
fn the_planned_documents_reach_the_areas_also_for_a_code_path() {
    // Only `valves/closing` cites the path.
    let r = rollup();
    let docs = docs_of(&r.served(&["context", "src/irrigation/valve.py"]));
    let at = |d: &str| {
        docs.iter()
            .position(|x| x == d)
            .unwrap_or_else(|| panic!("{d}: {docs:?}"))
    };
    assert!(at("system") < at("summaries/valves"), "{docs:?}");
    assert!(
        at("summaries/valves") < at("requirements/valves/closing"),
        "{docs:?}"
    );
}

#[test]
fn a_request_that_reaches_no_requirement_gets_no_summary() {
    // A rule is no requirement of an area.
    let r = rollup();
    let docs = docs_of(&r.served(&["context", "rules/watering"]));
    assert!(
        !docs
            .iter()
            .any(|d| d == "product" || d == "system" || d.starts_with("summaries/")),
        "{docs:?}"
    );
}

#[test]
fn a_top_document_with_no_rests_on_is_not_in_the_chain() {
    // `system` has no field `rests-on`: it is no roll-up document. `product`
    // still is one.
    let r = rollup();
    r.write(
        "docs/system.md",
        "# Irrigation system\n\nThe system waters beds in zones.\n",
    );
    let docs = docs_of(&r.served(&["context", "requirements/valves/closing"]));
    assert_eq!(
        docs[..5],
        [
            "rules/style",
            "product",
            "summaries/valves",
            "rules/watering",
            "requirements/valves/closing"
        ],
        "{docs:?}"
    );
}

#[test]
fn the_title_section_of_a_summary_comes_first() {
    // The text before the title is a section of level 0. It comes after the
    // title section.
    let r = rollup();
    let text = fs::read_to_string(r.dir.join("docs/summaries/valves.md")).unwrap();
    let at = text.find("\n---\n").unwrap() + "\n---\n".len();
    r.write(
        "docs/summaries/valves.md",
        &format!(
            "{}\nA note before the title.\n\n# Valves\n\nA valve closes after 30 seconds.\n\n## Limits\n\nNo more than 80 degrees.\n",
            &text[..at]
        ),
    );
    let (_, out, _) = r.run(&["context", "requirements/valves/closing"]);
    let ids: Vec<String> = full_ids(&out)
        .into_iter()
        .filter(|i| i.starts_with("summaries/valves"))
        .collect();
    assert_eq!(
        ids,
        [
            "summaries/valves#valves",
            "summaries/valves",
            "summaries/valves#limits"
        ],
        "{out}"
    );
}

#[test]
fn a_stale_summary_has_the_word_stale_in_its_header_and_in_json() {
    let r = rollup();
    let (_, out, _) = r.run(&["context", "requirements/valves/closing"]);
    assert!(!out.contains("stale"), "{out}");
    let (_, json, _) = r.run(&["context", "requirements/valves/closing", "--json"]);
    assert!(!json.contains("stale"), "{json}");
    // The requirement changes: `summaries/valves` is stale. `system` pins the
    // text of the summary, which did not change, so it is current.
    make_valves_stale(&r);
    let (code, out, err) = r.run(&["context", "requirements/valves/closing"]);
    assert_eq!(code, 0, "{out}{err}");
    let h = header_of(&out, "summaries/valves");
    assert!(h.ends_with(" summary summary stale -->"), "{h}");
    assert_eq!(out.matches(" stale -->").count(), 1, "{out}");
    let (_, json, _) = r.run(&["context", "requirements/valves/closing", "--json"]);
    let unit = |id: &str| -> String {
        let at = json
            .find(&format!("{{\"id\":\"{id}"))
            .unwrap_or_else(|| panic!("{id}: {json}"));
        let end = json[at + 1..]
            .find("{\"id\":")
            .map_or(json.len(), |e| at + 1 + e);
        json[at..end].to_string()
    };
    let valves = unit("summaries/valves#");
    assert!(
        valves.contains("\"tier\":\"summary\"") && valves.contains("\"stale\":true"),
        "{valves}"
    );
    // A current document has no field.
    assert!(!unit("system#").contains("stale"), "{json}");
    assert_eq!(json.matches("\"stale\"").count(), 1, "{json}");
}

#[test]
fn a_summary_with_a_missing_pin_is_stale_too() {
    // A new requirement of the area has no pin: SD642.
    let r = rollup();
    r.write(
        "docs/requirements/valves/opening.md",
        &requirements(
            "Opening",
            "open-on-start",
            "The valve SHALL open when a zone starts.",
        ),
    );
    let (_, out, _) = r.run(&["context", "requirements/valves/closing"]);
    assert!(
        header_of(&out, "summaries/valves").ends_with(" summary summary stale -->"),
        "{out}"
    );
}

#[test]
fn a_stale_summary_that_is_only_listed_keeps_its_marker() {
    // The summary of `valves` has 4,800 characters. It is over a half of 8,000
    // characters: a stub.
    let r = large_summary(300);
    make_valves_stale(&r);
    let (code, out, err) = r.run(&["context", "requirements/valves/closing"]);
    assert_eq!(code, 0, "{out}{err}");
    let line = out
        .lines()
        .find(|l| l.starts_with("- `summaries/valves"))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(
        line.contains("(summary)") && line.ends_with("[stale]"),
        "{line}"
    );
    // A current document in the list has no marker.
    let (_, out, _) = r.run(&["context", "requirements/valves/closing", "--budget", "900"]);
    let line = out
        .lines()
        .find(|l| l.starts_with("- `system"))
        .unwrap_or("");
    assert!(!line.contains("stale"), "{out}");
}

#[test]
fn the_summary_tier_uses_at_most_a_half_of_the_budget() {
    // The summary of `valves` has 3,200 characters. It fits in 4,000 of 8,000
    // characters (the default), and the tier has at most 4,000 characters.
    let r = large_summary(200);
    let (code, out, err) = r.run(&["context", "requirements/valves/closing", "--budget", "8000"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("A valve closes. A valve"), "{out}");
    assert!(tier_chars(&out, "summary") <= 4000, "{out}");
    // The other tiers keep their room: the entry is in full.
    assert!(
        full_ids(&out)
            .iter()
            .any(|i| i.starts_with("requirements/valves/closing")),
        "{out}"
    );
    // A summary of 4,800 characters does not fit in 4,000: it is a stub, and
    // the tier still has at most 4,000 characters.
    let r = large_summary(300);
    let (_, out, _) = r.run(&["context", "requirements/valves/closing", "--budget", "8000"]);
    assert!(
        out.lines()
            .any(|l| l.starts_with("- `summaries/valves#valves`") && l.contains("(summary)")),
        "the large section is a stub: {out}"
    );
    assert!(!out.contains("A valve closes. A valve"), "{out}");
    assert!(tier_total(&out) <= 4000, "{out}");
    // It fits in a half of 20,000 characters.
    let (_, out, _) = r.run(&[
        "context",
        "requirements/valves/closing",
        "--budget",
        "20000",
    ]);
    assert!(out.contains("A valve closes. A valve"), "{out}");
    assert!(tier_chars(&out, "summary") <= 10000, "{out}");
}

#[test]
fn the_share_is_a_half_of_the_budget_and_not_a_fixed_number() {
    // The summary of `valves` has 1,600 characters. It fits in 4,000 (the
    // half of 8,000) with the 2 top documents, and not in 1,500 (the half of
    // 3,000).
    let r = large_summary(100);
    let (_, out, _) = r.run(&["context", "requirements/valves/closing", "--budget", "8000"]);
    assert!(out.contains("A valve closes. A valve"), "{out}");
    let (_, out, _) = r.run(&["context", "requirements/valves/closing", "--budget", "3000"]);
    assert!(!out.contains("A valve closes. A valve"), "{out}");
    assert!(tier_total(&out) <= 1500, "{out}");
}

#[test]
fn once_a_section_of_the_tier_is_a_stub_each_later_one_is_a_stub() {
    // The fill order is `drives`, `valves`, `system`, `product`. At 8,000
    // characters, the summary of `valves` of 3,200 characters fits in the
    // half of the budget, so each document is in full.
    let args = [
        "context",
        "requirements/valves/drives/motor",
        "--budget",
        "8000",
        "--json",
    ];
    let r = large_summary(200);
    let modes = tier_modes(&r.run(&args).1);
    assert_eq!(
        modes,
        [
            ("product".to_string(), true),
            ("system".to_string(), true),
            ("summaries/valves".to_string(), true),
            ("summaries/valves/drives".to_string(), true)
        ],
        "{modes:?}"
    );
    // The summary of 4,800 characters is a stub, so each later section of the
    // fill order is a stub too, and only `drives` is in full. The output keeps
    // its order from the top down.
    let r = large_summary(300);
    let (_, json, _) = r.run(&args);
    let modes = tier_modes(&json);
    assert_eq!(
        modes,
        [
            ("product".to_string(), false),
            ("system".to_string(), false),
            ("summaries/valves".to_string(), false),
            ("summaries/valves/drives".to_string(), true)
        ],
        "{json}"
    );
    // The same at a small budget: no full section after a stub in the fill
    // order, which is the output order reversed (each document has 1 section).
    let r = rollup();
    for budget in ["400", "600", "1000", "2000"] {
        let (_, json, _) = r.run(&[
            "context",
            "requirements/valves/drives/motor",
            "--budget",
            budget,
            "--json",
        ]);
        let mut modes = tier_modes(&json);
        modes.reverse();
        let first_stub = modes.iter().position(|m| !m.1).unwrap_or(modes.len());
        assert!(
            modes[first_stub..].iter().all(|m| !m.1),
            "{budget}: {modes:?}"
        );
    }
}

#[test]
fn the_tier_is_a_prefix_of_its_fill_order_at_each_budget() {
    // The product has a long text, and the others are short. The fill order
    // is `drives`, `valves`, `system`, `product`. When the room is small, the
    // last sections of the fill order are cut. Thus the sections that the
    // output has are the first ones of the fill order, and in the fill order
    // each full section comes before each stub.
    let r = rollup_bodies(
        "A valve closes after 30 seconds.",
        &"The product waters. ".repeat(30),
    );
    let args = |b: &str| {
        vec![
            "context".to_string(),
            "requirements/valves/drives/motor".to_string(),
            "--budget".to_string(),
            b.to_string(),
            "--json".to_string(),
        ]
    };
    let run = |b: &str| {
        let a = args(b);
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        tier_modes(&r.run(&a).1)
    };
    let all: Vec<String> = run("100000").into_iter().map(|m| m.0).collect();
    assert_eq!(
        all,
        [
            "product",
            "system",
            "summaries/valves",
            "summaries/valves/drives"
        ]
    );
    let fill: Vec<String> = all.iter().rev().cloned().collect();
    // A rule before the tier takes room, and its size moves the point where
    // the room is gone: each size and each budget is a case.
    for rule_chars in [0, 800, 1200, 1600, 2000, 2400] {
        r.write(
            "docs/rules/watering.md",
            &format!(
                "---\ntype: rule\nstatus: active\n---\n# Watering\n\n## Rules\n\n- **valve-timeout**: The controller SHALL close each valve. {}\n",
                "The rule is long. ".repeat(rule_chars / 18)
            ),
        );
        for budget in (600..4600).step_by(100) {
            let mut modes = run(&budget.to_string());
            modes.reverse();
            let got: Vec<String> = modes.iter().map(|m| m.0.clone()).collect();
            assert_eq!(
                got[..],
                fill[..got.len()],
                "{rule_chars} {budget}: {modes:?}"
            );
            let first_stub = modes.iter().position(|m| !m.1).unwrap_or(modes.len());
            assert!(
                modes[first_stub..].iter().all(|m| !m.1),
                "{rule_chars} {budget}: {modes:?}"
            );
        }
    }
}

/// The documents of the tier `summary` in the `--json` output, in order, with
/// true for a section in full. A document with 2 sections gives 2 items.
fn tier_modes(json: &str) -> Vec<(String, bool)> {
    parse_units(json)
        .into_iter()
        .filter(|(_, tier, _)| tier == "summary")
        .map(|(id, _, full)| (id.split('#').next().unwrap().to_string(), full))
        .collect()
}

/// The units of a `--json` output: id, tier and true if in full.
fn parse_units(json: &str) -> Vec<(String, String, bool)> {
    let field = |u: &str, key: &str| -> String {
        let at = u.find(&format!("\"{key}\":\"")).unwrap() + key.len() + 4;
        u[at..].split('"').next().unwrap().to_string()
    };
    json.split("{\"id\":\"")
        .skip(1)
        .map(|u| {
            let u = format!("{{\"id\":\"{u}");
            (
                field(&u, "id"),
                field(&u, "tier"),
                field(&u, "mode") == "full",
            )
        })
        .collect()
}

/// The characters that the tier `summary` takes: its full sections, and its
/// stub lines.
fn tier_total(out: &str) -> usize {
    tier_chars(out, "summary")
        + out
            .lines()
            .filter(|l| l.starts_with("- `") && l.contains(" chars (summary)"))
            .map(|l| l.chars().count() + 1)
            .sum::<usize>()
}

/// The number of characters that the head of the output reports as used.
fn used_of(out: &str) -> usize {
    out.lines()
        .next()
        .unwrap()
        .split("; ")
        .find_map(|p| {
            p.strip_suffix(" characters")
                .and_then(|p| p.split(" of ").next())
        })
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn the_used_characters_count_the_stale_word_of_the_header() {
    // The edit keeps the length of the text, so the only difference between
    // the 2 outputs is the word ` stale` in one header: 6 characters.
    let args = [
        "context",
        "requirements/valves/closing",
        "--budget",
        "100000",
    ];
    let r = rollup();
    let (_, fresh, _) = r.run(&args);
    make_valves_stale(&r);
    let (_, stale, _) = r.run(&args);
    assert!(!stale.contains("not loaded"), "{stale}");
    assert!(stale.contains(" summary summary stale -->"), "{stale}");
    assert_eq!(used_of(&stale), used_of(&fresh) + " stale".len(), "{stale}");
}

#[test]
fn a_named_summary_is_an_entry_and_the_tier_skips_it() {
    // The second target reaches the area `valves`, so the chain holds the
    // summary that the first target names.
    let r = rollup();
    let (_, out, _) = r.run(&["context", "summaries/valves", "requirements/valves/closing"]);
    assert!(
        header_of(&out, "summaries/valves").ends_with(" summary entry -->"),
        "{out}"
    );
    // The document is in one tier only.
    assert_eq!(out.matches("<!-- summaries/valves").count(), 1, "{out}");
    assert!(out.contains(" system summary -->"), "{out}");
}

#[test]
fn a_summary_that_other_tiers_reach_is_in_the_summary_tier_only() {
    // The `applies-to` of `system` matches the entry: it is in the tier
    // `summary`, not in the tier `applies`.
    let r = rollup();
    let text = fs::read_to_string(r.dir.join("docs/system.md")).unwrap();
    r.write(
        "docs/system.md",
        &text.replacen(
            "rests-on:",
            "applies-to: [\"requirements/**\"]\nrests-on:",
            1,
        ),
    );
    let (_, out, _) = r.run(&["context", "requirements/valves/closing"]);
    assert!(
        header_of(&out, "system").ends_with(" system summary -->"),
        "{out}"
    );
    assert!(!out.contains("system applies"), "{out}");
    let ids = full_ids(&out);
    assert_eq!(
        ids.iter().filter(|i| i.starts_with("system")).count(),
        1,
        "{ids:?}"
    );
}

#[test]
fn a_planned_document_that_is_not_active_reaches_no_area() {
    let r = rollup();
    let retired = |title: &str, key: &str, text: &str| {
        requirements(title, key, text).replace("status: active", "status: retired")
    };
    r.write(
        "docs/requirements/valves/closing.md",
        &retired(
            "Closing",
            "close-on-timeout",
            "The valve SHALL close after 30 seconds. See `src/irrigation/valve.py`.",
        ),
    );
    let docs = docs_of(&r.served(&["context", "src/irrigation/valve.py"]));
    assert!(
        !docs
            .iter()
            .any(|d| d == "system" || d.starts_with("summaries/")),
        "{docs:?}"
    );
}

#[test]
fn under_a_query_the_tier_follows_the_entry_and_keeps_its_order() {
    // The rule `watering` has 1,500 characters and the words of the query.
    // The entry matches too. The tier never takes the room of the entry: the
    // sections of the entry are in full, and the tier has at most a half of
    // 4,000 characters.
    let r = rollup();
    let big = "The motor spin full speed rule. ".repeat(45);
    r.write(
        "docs/rules/watering.md",
        &format!(
            "---\ntype: rule\nstatus: active\n---\n# Watering\n\n## Rules\n\n- **valve-timeout**: The controller SHALL close each valve. {big}\n"
        ),
    );
    let (code, out, err) = r.run(&[
        "context",
        "requirements/valves/drives/motor",
        "--query",
        "motor spin full speed",
        "--budget",
        "4000",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let ids = full_ids(&out);
    assert!(
        ids.iter()
            .any(|i| i.starts_with("requirements/valves/drives/motor#requirements")),
        "{out}"
    );
    // The tier has at most a half of the budget, and with the entry in full
    // the whole output stays in the budget.
    assert!(tier_total(&out) <= 2000, "{out}");
    assert!(used_of(&out) <= 4000, "{out}");
    let docs = docs_of(&ids);
    let at = |d: &str| docs.iter().position(|x| x == d);
    assert!(at("product").is_some(), "{docs:?}");
    assert!(
        at("product") < at("system") && at("system") < at("summaries/valves"),
        "{docs:?}"
    );
}

/// A project whose `system` document has 80 short sections and has pins, and
/// whose requirement `pumps/drive` has a section of about 4,900 characters
/// that cites `src/drive.py`.
fn crowded() -> Repo {
    let r = Repo::with(ROLL_HEAD, ROLL_ROOTS);
    r.write("src/drive.py", "def spin():\n    return 1\n");
    r.write(
        "docs/requirements/pumps/drive.md",
        &requirements(
            "Drive",
            "spin",
            &format!(
                "The drive SHALL keep the speed. See `src/drive.py`. {}",
                "The drive SHALL keep the speed. ".repeat(150)
            ),
        ),
    );
    let sections: String = (1..=80)
        .map(|n| format!("## Section {n}\n\nText of section {n}.\n\n"))
        .collect();
    top(
        &r,
        "docs/system.md",
        "system",
        "Irrigation system",
        &sections,
    );
    r.write(
        "docs/product.md",
        "---\nrests-on: []\n---\n# Product\n\n## Goal\n\nThe product waters beds.\n",
    );
    r
}

#[test]
fn the_tier_never_takes_the_room_of_the_entry_or_of_a_citing_section() {
    let r = crowded();
    for args in [
        vec!["context", "requirements/pumps/drive"],
        vec!["context", "src/drive.py"],
    ] {
        for budget in ["8000", "4000", "2000", "1000", "300"] {
            let mut a = args.clone();
            a.extend(["--budget", budget]);
            let (code, out, err) = r.run(&a);
            assert_eq!(code, 0, "{out}{err}");
            let b: usize = budget.parse().unwrap();
            // The long section is in full or listed: the tier does not push
            // it out. At 8,000 characters, it is in full.
            assert!(
                out.contains("requirements/pumps/drive#requirements"),
                "{a:?}: {out}"
            );
            if b == 8000 {
                assert!(
                    full_ids(&out)
                        .iter()
                        .any(|i| i == "requirements/pumps/drive#requirements"),
                    "{a:?}: {out}"
                );
            }
            // The tier, in full and in stubs, has at most a half.
            assert!(
                tier_total(&out) <= b / 2,
                "{a:?} {budget}: {}",
                tier_total(&out)
            );
        }
    }
}

#[test]
fn a_corpus_with_no_summary_keeps_the_old_output() {
    // The same documents with the layer and with no layer for the summary:
    // `system` and `product` have no `rests-on`, and no document is a summary.
    let docs = |r: &Repo| {
        r.write("src/irrigation/valve.py", VALVE);
        r.write(
            "docs/rules/style.md",
            &rule("", "short-lines").replace("Watering", "Style"),
        );
        r.write(
            "docs/rules/watering.md",
            &rule("applies-to: [\"requirements/**\"]\n", "valve-timeout"),
        );
        r.write(
            "docs/requirements/valves/closing.md",
            &requirements(
                "Closing",
                "close-on-timeout",
                "The valve SHALL close. See `src/irrigation/valve.py`.",
            ),
        );
        r.write("docs/system.md", "# System\n\nThe system waters beds.\n");
        r.write(
            "docs/product.md",
            &product("applies-to: [\"requirements/**\"]\n"),
        );
    };
    let with_layer = Repo::with(ROLL_HEAD, ROLL_ROOTS);
    docs(&with_layer);
    let without = Repo::with(
        &ROLL_HEAD.replace("[layers.summaries]\ntypes = [\"summary\"]\n\n", ""),
        ROLL_ROOTS,
    );
    docs(&without);
    for args in [
        vec!["context", "requirements/valves/closing"],
        vec!["context", "requirements/valves/closing", "--json"],
        vec!["context", "src/irrigation/valve.py", "--budget", "900"],
        vec![
            "context",
            "requirements/valves/closing",
            "--query",
            "valve close",
        ],
    ] {
        let (_, a, _) = with_layer.run(&args);
        let (_, b, _) = without.run(&args);
        assert!(!a.contains("summary"), "{args:?}: {a}");
        assert_eq!(a, b, "{args:?}");
    }
}

#[test]
fn a_summary_in_a_project_with_no_layer_for_it_is_not_served() {
    // `lite` has no layer `summaries`. A file of the type `summary` has no
    // place then (SD303), and `context` does not serve it.
    let r = Repo::with(
        &ROLL_HEAD.replace("[layers.summaries]\ntypes = [\"summary\"]\n\n", ""),
        ROLL_ROOTS,
    );
    r.write(
        "docs/requirements/valves/closing.md",
        &requirements("Closing", "close-on-timeout", "The valve SHALL close."),
    );
    r.write(
        "docs/summaries/valves.md",
        "---\ntype: summary\nrests-on: []\n---\n# Valves\n\nA valve closes.\n",
    );
    let (code, out, err) = r.run(&["context", "requirements/valves/closing"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("summaries/valves"), "{out}");
}

#[test]
fn an_on_request_layer_of_summaries_is_counted_and_not_served() {
    let r = Repo::with(
        &ROLL_HEAD.replace(
            "[layers.summaries]\ntypes = [\"summary\"]\n",
            "[layers.summaries]\ntypes = [\"summary\"]\ncontext = \"on-request\"\n",
        ),
        ROLL_ROOTS,
    );
    r.write(
        "docs/requirements/valves/closing.md",
        &requirements("Closing", "close-on-timeout", "The valve SHALL close."),
    );
    summary(&r, "valves", "A valve closes.");
    let (_, out, _) = r.run(&["context", "requirements/valves/closing"]);
    assert!(!out.contains("<!-- summaries/valves"), "{out}");
    assert!(
        out.contains("on-request, not served: summaries 1;"),
        "{out}"
    );
}

/// The layers of the fixture of D83: no convention and no rule, so the output
/// has the tier `summary` and the entry only.
const D83_HEAD: &str = "[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n";

/// The fixture of D83 (design 0.8.1, part A): 6 areas, a real layer graph, and
/// a `product` and a `system` document of about 4,100 bytes each. The summary
/// of `valves` repeats one sentence `valves_repeat` times: 8 gives about 600
/// bytes, and 56 gives about 3,000 bytes. `context src/motor.rs` reaches the
/// chain `product`, `system`, `summaries/valves`, `summaries/valves/drives`.
fn d83(valves_repeat: usize) -> Repo {
    let r = Repo::with(D83_HEAD, ROLL_ROOTS);
    r.write("src/valve.rs", "pub fn close_valve() {}\n");
    r.write("src/motor.rs", "pub fn spin_up() {}\n");
    r.write(
        "docs/requirements/valves/drives/motor.md",
        "---\nstatus: active\n---\n# Motor\n\n## Requirements\n\n- **spin-up**: The motor SHALL reach full speed in 2 seconds. See `src/motor.rs`.\n- **spin-down**: The motor SHALL stop in 1 second.\n",
    );
    r.write(
        "docs/requirements/valves/closing.md",
        "---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close-on-timeout**: The valve SHALL close after 30 seconds. See `src/valve.rs`.\n- **max-angle**: The valve SHALL open at most 80 degrees.\n",
    );
    for a in ["sensors", "pumps", "zones", "schedules"] {
        r.write(
            &format!("docs/requirements/{a}/main.md"),
            &format!(
                "---\nstatus: active\n---\n# {a}\n\n## Requirements\n\n- **first**: The {a} part SHALL start in 2 seconds.\n- **second**: The {a} part SHALL stop in 3 seconds.\n"
            ),
        );
    }
    for a in [
        "valves/drives",
        "valves",
        "sensors",
        "pumps",
        "zones",
        "schedules",
    ] {
        let body = if a == "valves" {
            "A valve closes 30 seconds after its sensor stops. ".repeat(valves_repeat)
        } else {
            format!("The {a} part starts and stops on time. ").repeat(16)
        };
        summary(&r, a, &body);
    }
    for (path, id, title) in [
        ("docs/system.md", "system", "Irrigation system"),
        ("docs/product.md", "product", "Irrigation product"),
    ] {
        let mut body = format!("The {id} waters garden beds in zones. ").repeat(10);
        for s in 1..=8 {
            let text = format!("Part {s} of the {id} has its own text. ").repeat(11);
            body.push_str(&format!("\n\n## Part {s}\n\n{text}"));
        }
        r.write(
            path,
            &format!("---\nrests-on: []\n---\n# {title}\n\n{body}\n"),
        );
    }
    for id in ["system", "product"] {
        let (code, out, err) = r.run(&["refresh", id, "--holds"]);
        assert_eq!(code, 0, "{out}{err}");
    }
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(format!("{out}{err}").contains(" 0 errors"), "{out}{err}");
    r
}

/// What `context` gives for one target at one budget: the bytes of the
/// output, the characters and the tokens of its head, and the units of the
/// tier `summary` with true for a section in full.
struct Measured {
    bytes: usize,
    used: usize,
    tokens: usize,
    tier: Vec<(String, bool)>,
}

impl Measured {
    /// One line for the test log, as a row of the table of the plan.
    fn line(&self, valves_repeat: usize, budget: u32) -> String {
        let tier: Vec<String> = self
            .tier
            .iter()
            .map(|(id, full)| format!("{id}:{}", if *full { "full" } else { "stub" }))
            .collect();
        format!(
            "d83 valves_repeat={valves_repeat} budget={budget} bytes={} used={} tokens={} tier: {}",
            self.bytes,
            self.used,
            self.tokens,
            tier.join(" ")
        )
    }

    /// The documents of the tier, in the order of the output, each once.
    fn docs(&self) -> Vec<String> {
        let mut docs: Vec<String> = Vec::new();
        for (id, _) in &self.tier {
            let d = id.split('#').next().unwrap().to_string();
            if docs.last() != Some(&d) {
                docs.push(d);
            }
        }
        docs
    }

    /// True if the section of the tier with this id is in full.
    fn is_full(&self, id: &str) -> bool {
        self.tier.iter().any(|(i, full)| i == id && *full)
    }

    /// True if the output has the section of the tier with this id, in full
    /// or as a stub.
    fn has(&self, id: &str) -> bool {
        self.tier.iter().any(|(i, _)| i == id)
    }
}

fn measure(r: &Repo, target: &str, budget: u32) -> Measured {
    let b = budget.to_string();
    let (code, out, err) = r.run(&["context", target, "--budget", &b]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, json, err) = r.run(&["context", target, "--budget", &b, "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    let head = out.lines().next().unwrap();
    let tokens = head
        .split("about ")
        .nth(1)
        .and_then(|p| p.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no tokens in the head: {head}"));
    Measured {
        bytes: out.trim_end_matches('\n').len() + 1,
        used: used_of(&out),
        tokens,
        tier: parse_units(&json)
            .into_iter()
            .filter(|(_, tier, _)| tier == "summary")
            .map(|(id, _, full)| (id, full))
            .collect(),
    }
}

/// The output order of the 4 documents of the chain of `src/motor.rs`.
const D83_ORDER: [&str; 4] = [
    "product",
    "system",
    "summaries/valves",
    "summaries/valves/drives",
];

#[test]
fn the_fixture_of_d83_keeps_the_output_order_at_each_budget_and_prints_its_sizes() {
    for valves_repeat in [8, 56] {
        let r = d83(valves_repeat);
        // At a large budget the tier has all 4 documents, so the guard below
        // is not empty.
        let all = measure(&r, "src/motor.rs", 100_000);
        assert_eq!(
            all.docs(),
            D83_ORDER,
            "{}",
            all.line(valves_repeat, 100_000)
        );
        for budget in [4_000, 8_000, 12_000, 20_000] {
            let m = measure(&r, "src/motor.rs", budget);
            // `cargo test -- --nocapture` shows the line of each budget.
            println!("{}", m.line(valves_repeat, budget));
            // The documents that the tier serves keep the order from the top
            // down. A document that the budget cuts leaves a gap, not a
            // change of order.
            let line = m.line(valves_repeat, budget);
            let ranks: Vec<usize> = m
                .docs()
                .iter()
                .map(|d| {
                    D83_ORDER
                        .iter()
                        .position(|x| x == d)
                        .unwrap_or_else(|| panic!("{d}: {line}"))
                })
                .collect();
            assert!(ranks.windows(2).all(|w| w[0] < w[1]), "{line}");
            assert!(m.used <= budget as usize, "{line}");
        }
    }
}

#[test]
fn the_summary_of_600_bytes_is_in_full_at_8000_with_a_real_layer_graph() {
    // Before D83 the tier filled `product` first and its share ended inside
    // `product`: no summary and no line of `system`.
    let r = d83(8);
    let m = measure(&r, "src/motor.rs", 8_000);
    let line = m.line(8, 8_000);
    assert!(m.is_full("summaries/valves/drives#drives"), "{line}");
    assert!(m.is_full("summaries/valves#valves"), "{line}");
}

#[test]
fn the_title_sections_of_d83_come_before_the_other_sections_in_the_fill_order() {
    // The fill order of the fixture: the title of each document from the
    // deepest summary up, then the other sections of `system`, then those of
    // `product`. At each budget the served sections are a prefix of it, and
    // in the fill order each full section comes before each stub.
    let mut fill: Vec<String> = vec![
        "summaries/valves/drives#drives".into(),
        "summaries/valves#valves".into(),
        "system#irrigation-system".into(),
        "product#irrigation-product".into(),
    ];
    for doc in ["system", "product"] {
        fill.extend((1..=8).map(|s| format!("{doc}#part-{s}")));
    }
    for valves_repeat in [8, 56] {
        let r = d83(valves_repeat);
        for budget in (600..=24_000).step_by(300) {
            let b = budget.to_string();
            let (code, json, err) = r.run(&["context", "src/motor.rs", "--budget", &b, "--json"]);
            assert_eq!(code, 0, "{json}{err}");
            let mut served: Vec<(usize, bool)> = parse_units(&json)
                .into_iter()
                .filter(|(_, tier, _)| tier == "summary")
                .map(|(id, _, full)| {
                    let at = fill.iter().position(|f| *f == id);
                    (
                        at.unwrap_or_else(|| panic!("{id} is not in the fill order")),
                        full,
                    )
                })
                .collect();
            served.sort();
            let case = format!("{valves_repeat} {budget}: {served:?}");
            assert!(
                served.iter().enumerate().all(|(n, s)| s.0 == n),
                "not a prefix: {case}"
            );
            let first_stub = served.iter().position(|s| !s.1).unwrap_or(served.len());
            assert!(
                served[first_stub..].iter().all(|s| !s.1),
                "a full section after a stub: {case}"
            );
        }
    }
}

#[test]
fn the_summary_of_3000_bytes_is_in_full_at_8000_and_a_listed_stub_at_4000() {
    // The summary is over a quarter of 8,000 characters, the old share. The
    // new order alone does not serve it in full, and the half share does.
    let r = d83(56);
    let m = measure(&r, "src/motor.rs", 8_000);
    let line = m.line(56, 8_000);
    assert!(m.is_full("summaries/valves/drives#drives"), "{line}");
    assert!(m.is_full("summaries/valves#valves"), "{line}");
    let m = measure(&r, "src/motor.rs", 4_000);
    let line = m.line(56, 4_000);
    assert!(m.has("summaries/valves#valves"), "{line}");
    assert!(!m.is_full("summaries/valves#valves"), "{line}");
}

/// Gives the requirement `valves/drives/motor` a long section `Requirements`
/// (about 5,900 characters) and a section `Links` with 10 links to new active
/// requirement documents. The links give the plan 10 documents of the tiers
/// after the summary tier, and each of them has a stub line in the map pass.
fn crowd_motor(r: &Repo) {
    let long = "The motor SHALL keep a steady speed. ".repeat(160);
    let links: String = (1..=10)
        .map(|n| format!("- [[requirements/links/l{n}]]\n"))
        .collect();
    r.write(
        "docs/requirements/valves/drives/motor.md",
        &format!(
            "---\nstatus: active\n---\n# Motor\n\n## Requirements\n\n- **spin-up**: The motor SHALL reach full speed in 2 seconds. See `src/motor.rs`. {long}\n- **spin-down**: The motor SHALL stop in 1 second.\n\n## Links\n\n{links}"
        ),
    );
    for n in 1..=10 {
        r.write(
            &format!("docs/requirements/links/l{n}.md"),
            &requirements(
                &format!("Link {n}"),
                "only",
                &format!("The link {n} part SHALL work."),
            ),
        );
    }
}

/// The fixture of D83 with the crowded entry, and the same documents in a tree
/// with no summary and no `rests-on`.
fn d83_crowded() -> (Repo, Repo) {
    let with = d83(8);
    crowd_motor(&with);
    for id in [
        "summaries/valves/drives",
        "summaries/valves",
        "system",
        "product",
    ] {
        let (code, out, err) = with.run(&["refresh", id, "--holds"]);
        assert_eq!(code, 0, "{out}{err}");
    }
    let plain = Repo::with(
        &D83_HEAD.replace("[layers.summaries]\ntypes = [\"summary\"]\n\n", ""),
        ROLL_ROOTS,
    );
    plain.write("src/motor.rs", "pub fn spin_up() {}\n");
    crowd_motor(&plain);
    (with, plain)
}

#[test]
fn the_tier_never_pushes_out_the_full_entry_that_a_tree_with_no_summary_serves() {
    // The sections of the tiers `linked` and `more` come after the summary
    // tier in the packing order, but their stub lines are in the map pass, and
    // the map pass comes before the full text of the entry. The room of the
    // tier must leave the room of these stub lines too.
    let (with, plain) = d83_crowded();
    let entry = "requirements/valves/drives/motor#requirements";
    let full_entry = |r: &Repo, budget: u32| -> bool {
        let b = budget.to_string();
        let (code, json, err) = r.run(&[
            "context",
            "requirements/valves/drives/motor",
            "--budget",
            &b,
            "--json",
        ]);
        assert_eq!(code, 0, "{json}{err}");
        parse_units(&json)
            .iter()
            .any(|(id, _, full)| id == entry && *full)
    };
    let mut served_by_plain = 0;
    for budget in (6_000..=10_000).step_by(50) {
        if full_entry(&plain, budget) {
            served_by_plain += 1;
            assert!(
                full_entry(&with, budget),
                "the tree with no summary serves the entry in full at {budget}, the tree with the roll-up does not"
            );
        }
    }
    // The sweep is not empty: the entry is in full for some budgets.
    assert!(served_by_plain > 10, "{served_by_plain}");
}

// The source layers of the roll-up in the chain (design 0.8.1, D84, item 8).

const SOURCE_HEAD: &str = "[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.research]\ntypes = [\"research\"]\ncontext = \"on-request\"\n\n[layers.interfaces]\ntypes = [\"interface\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\nrolls-up = [\"decisions\", \"research\", \"interfaces\"]\n\n";

const STORAGE_DECISION: &str = "---\nstatus: accepted\n---\n# Use SQLite\n\n## Context and Problem Statement\n\nThe service keeps its data in one place.\n\n## Decision Outcome\n\nUse one SQLite file with a write-ahead log.\n";

/// A project with decisions, research and interfaces as source layers and the
/// summary `summaries/storage`, with current pins.
fn sources() -> Repo {
    let r = Repo::with(SOURCE_HEAD, ROLL_ROOTS);
    r.write("docs/decisions/storage/use-sqlite.md", STORAGE_DECISION);
    r.write(
        "docs/research/storage/write-load.md",
        "# Write load\n\nOne writer at a time is enough for 200 writes in a second.\n",
    );
    r.write(
        "docs/interfaces/storage/api.md",
        "---\nstatus: active\n---\n# Storage API\n\n- **put**: The storage API SHALL store a value under a key.\n",
    );
    summary(
        &r,
        "storage",
        "The service keeps its data in one SQLite file.",
    );
    r
}

#[test]
fn the_chain_of_an_accepted_decision_has_the_summary_of_its_area() {
    let r = sources();
    let docs = docs_of(&r.served(&["context", "decisions/storage/use-sqlite"]));
    assert!(
        docs.iter().any(|d| d == "summaries/storage"),
        "the tier has no summary: {docs:?}"
    );
    let at = |d: &str| docs.iter().position(|x| x == d).unwrap();
    assert!(
        at("summaries/storage") < at("decisions/storage/use-sqlite"),
        "{docs:?}"
    );
}

#[test]
fn the_chain_of_a_note_that_a_target_names_and_of_an_interface_has_the_summary() {
    let r = sources();
    for target in ["research/storage/write-load", "interfaces/storage/api"] {
        let docs = docs_of(&r.served(&["context", target]));
        assert!(
            docs.iter().any(|d| d == "summaries/storage"),
            "{target}: {docs:?}"
        );
    }
}

#[test]
fn a_decision_that_is_not_accepted_reaches_no_area() {
    let r = sources();
    r.write(
        "docs/decisions/storage/use-postgres.md",
        &STORAGE_DECISION.replace("accepted", "proposed"),
    );
    let (code, out, err) = r.run(&["context", "decisions/storage/use-postgres"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("summaries/storage"), "{out}");
}

#[test]
fn a_changed_note_does_not_mark_the_summary_stale_in_the_chain() {
    let r = sources();
    r.write(
        "docs/research/storage/write-load.md",
        "# Write load\n\nOne writer at a time is enough for 400 writes in a second.\n",
    );
    let (code, out, err) = r.run(&["context", "decisions/storage/use-sqlite"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("summaries/storage"), "{out}");
    assert!(
        !header_of(&out, "summaries/storage").contains("stale"),
        "{out}"
    );
}

#[test]
fn a_layer_with_decisions_and_notes_and_no_key_serves_the_chain_of_0_8_0() {
    let head = "[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.specs]\ntypes = [\"requirements\", \"decision\", \"research\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n";
    let r = Repo::with(head, ROLL_ROOTS);
    r.write(
        "docs/specs/storage/closing.md",
        "---\ntype: requirements\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close-on-timeout**: The valve SHALL close.\n",
    );
    r.write(
        "docs/specs/storage/use-sqlite.md",
        &STORAGE_DECISION.replace("---\n", "---\ntype: decision\n"),
    );
    r.write(
        "docs/specs/storage/use-files.md",
        &STORAGE_DECISION
            .replace("---\n", "---\ntype: decision\n")
            .replace("accepted", "proposed"),
    );
    r.write(
        "docs/specs/storage/write-load.md",
        "---\ntype: research\n---\n# Write load\n\nOne writer is enough.\n",
    );
    summary(&r, "storage", "The valve closes.");
    top(&r, "docs/system.md", "system", "System", "The system.");
    let text = fs::read_to_string(r.dir.join("docs/summaries/storage.md")).unwrap();
    assert!(
        !text.contains("use-sqlite") && !text.contains("write-load"),
        "{text}"
    );
    // A requirement, an accepted decision and a proposed decision of the layer
    // reach the area, as in 0.8.0. The summary is current, so it is not stale.
    for target in [
        "specs/storage/closing",
        "specs/storage/use-sqlite",
        "specs/storage/use-files",
    ] {
        let (code, out, err) = r.run(&["context", target]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("summaries/storage"), "{target}: {out}");
        assert!(
            !header_of(&out, "summaries/storage").contains("stale"),
            "{target}: {out}"
        );
        assert!(
            !header_of(&out, "system").contains("stale"),
            "{target}: {out}"
        );
    }
}
