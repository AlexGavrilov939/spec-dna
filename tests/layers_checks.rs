//! Integration tests of the checks of the layer graph and of the requirement
//! text: SD901 (link direction), SD902 (layer limit), SD903 (history in
//! requirements) and SD904 (forbidden term), and the new names of SD405 and
//! SD611. Each check has a fixture that passes and a fixture that fails, in
//! each profile. The helpers are small copies of the ones in tests/sd202.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

mod common;

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
        let dir = std::env::temp_dir().join(format!("spec-dna-layers-{}-{n}", std::process::id()));
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

/// A `requirements` document with one requirement, `pump-1`.
fn req_doc(text: &str) -> String {
    doc(
        "requirements",
        "active",
        "",
        &format!("## Requirements\n\n- **pump-1**: {text}"),
    )
}

const INTENT: &str = "## Intent\n\nText.";

/// The severity of a code in the output of `check`, or `None`.
fn severity(out: &str, code: &str) -> Option<&'static str> {
    ["error", "warning"]
        .into_iter()
        .find(|s| out.contains(&format!("{s} {code} ")))
}

/// Runs a fixture in each profile. A failing fixture gives nothing in `lite`
/// and a warning in `standard` and `strict`. A passing fixture gives nothing
/// in any profile.
fn in_each_profile(code: &str, fails: bool, make: impl Fn(&str) -> Repo) {
    for (profile, want) in [
        ("lite", None),
        ("standard", Some("warning")),
        ("strict", Some("warning")),
    ] {
        let out = make(profile).check();
        let want = if fails { want } else { None };
        assert_eq!(severity(&out, code), want, "{code} in {profile}: {out}");
    }
}

/// The default graph (no `[layers]` table) with a rule document that has
/// `rule_body` and an intent document that has `intent_body`.
fn rule_and_intent(profile: &str, rule_extra: &str, rule_body: &str, intent_body: &str) -> Repo {
    let r = Repo::new(profile, "", "", "");
    r.write(
        "docs/rules/pump.md",
        &doc("rule", "active", rule_extra, rule_body),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", intent_body),
    );
    r
}

const RULE_PLAIN: &str = "## Rules\n\n- **pump-1**: The controller SHALL stop the pump.";

// SD901

#[test]
fn sd901_a_rule_that_links_an_intent_fails_in_each_profile() {
    in_each_profile("SD901", true, |p| {
        rule_and_intent(
            p,
            "",
            "## Rules\n\n- **pump-1**: The controller SHALL stop the pump. See [the change](../intent/pump/intent.md).",
            INTENT,
        )
    });
}

#[test]
fn sd901_an_intent_that_links_a_rule_passes_in_each_profile() {
    in_each_profile("SD901", false, |p| {
        rule_and_intent(
            p,
            "",
            RULE_PLAIN,
            "## Intent\n\nSee [the rule](../../rules/pump.md).",
        )
    });
}

#[test]
fn sd901_each_reference_form_gives_one_diagnostic_for_the_pair() {
    // A link, a `[[id]]` and `depends-on` name the same intent. There is one
    // diagnostic for the pair (document, target).
    let r = rule_and_intent(
        "standard",
        "depends-on: intent/pump/intent\n",
        "## Rules\n\n- **pump-1**: The controller SHALL stop the pump. See [[intent/pump/intent]] and [the change](../intent/pump/intent.md).",
        INTENT,
    );
    let out = r.check();
    assert_eq!(out.matches(" SD901 link-direction").count(), 1, "{out}");
    assert!(
        out.contains(
            "`rules/pump` (layer `rules`) references `intent/pump/intent` (layer `intent`)"
        ),
        "{out}"
    );
    // Each form gives it alone.
    for (extra, body) in [
        ("depends-on: intent/pump/intent\n", RULE_PLAIN.to_string()),
        (
            "",
            "## Rules\n\n- **pump-1**: The controller SHALL stop the pump. See [[intent/pump/intent]]."
                .to_string(),
        ),
    ] {
        let out = rule_and_intent("standard", extra, &body, INTENT).check();
        assert_eq!(out.matches(" SD901 link-direction").count(), 1, "{out}");
    }
}

#[test]
fn sd901_a_link_in_front_matter_supersedes_gives_it() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/rules/pump.md",
        &doc(
            "rule",
            "active",
            "supersedes: intent/pump/intent\n",
            RULE_PLAIN,
        ),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    assert_eq!(severity(&r.check(), "SD901"), Some("warning"));
}

#[test]
fn sd901_a_layer_in_linked_from_is_allowed() {
    let graph = |linked_from: &str| {
        format!(
            "[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"rules\"]\nkeep = \"until-finished\"\n{linked_from}\n"
        )
    };
    let link = "## Rules\n\n- **pump-1**: The controller SHALL stop the pump. See [the change](../intent/pump/intent.md).";
    for (linked_from, want) in [("", Some("warning")), ("linked-from = [\"rules\"]", None)] {
        let r = Repo::new("standard", "", &graph(linked_from), "");
        r.write("docs/rules/pump.md", &doc("rule", "active", "", link));
        r.write(
            "docs/intent/pump/intent.md",
            &doc("intent", "draft", "", INTENT),
        );
        assert_eq!(severity(&r.check(), "SD901"), want, "{linked_from}");
    }
}

#[test]
fn sd901_a_default_edge_allows_a_requirement_to_link_research() {
    // `research` lists `requirements` in `linked-from`.
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/requirements/pump.md",
        &req_doc("The controller SHALL stop the pump. See [the study](../research/pump.md)."),
    );
    r.write("docs/research/pump.md", &doc("research", "", "", "Text."));
    let out = r.check();
    assert_eq!(severity(&out, "SD901"), None, "{out}");
}

/// The default graph, and the small graph that `init` writes. The small graph
/// is not a file of this test, so the code of `init` gives it.
fn graph_repos() -> Vec<(&'static str, Repo)> {
    let default = Repo::new("standard", "", "", "");
    let small = Repo::new("standard", "", "", "");
    fs::remove_file(small.dir.join("spec-dna.toml")).unwrap();
    let out = Command::new(BIN)
        .args(["init", "--non-interactive"])
        .current_dir(&small.dir)
        .output()
        .expect("binary runs");
    assert!(out.status.success(), "{out:?}");
    let config = fs::read_to_string(small.dir.join("spec-dna.toml")).unwrap();
    assert!(config.contains("[layers.pending]"), "{config}");
    vec![("default", default), ("small", small)]
}

#[test]
fn requirements_and_decisions_can_link_the_pending_layer() {
    // A requirement can name an open question, and a decision can name the
    // roadmap. The `pending` layer lists both layers in `linked-from`.
    for (graph, r) in graph_repos() {
        r.write(
            "docs/requirements/pump.md",
            &req_doc("The controller SHALL stop the pump. See [the question](../questions.md)."),
        );
        r.write(
            "docs/decisions/pump.md",
            &doc(
                "decision",
                "proposed",
                "",
                "## Decision Outcome\n\nUse the pump. See [the plan](../roadmap.md).",
            ),
        );
        r.write("docs/questions.md", "# Questions\n\nWhich pump?\n");
        r.write("docs/roadmap.md", "# Roadmap\n\nThe pump comes next.\n");
        let out = r.check();
        assert_eq!(severity(&out, "SD901"), None, "{graph}: {out}");
        assert_eq!(severity(&out, "SD101"), None, "{graph}: {out}");
    }
}

#[test]
fn sd901_two_layers_with_no_path_and_an_unresolved_target_give_nothing() {
    // `rules` and `requirements` have no edge between them.
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/rules/pump.md",
        &doc(
            "rule",
            "active",
            "",
            "## Rules\n\n- **pump-1**: The controller SHALL stop the pump. See [the spec](../requirements/pump.md) and [[intent/missing/intent]].",
        ),
    );
    r.write("docs/requirements/pump.md", &req_doc("The pump SHALL run."));
    let out = r.check();
    assert_eq!(severity(&out, "SD901"), None, "{out}");
    assert_eq!(severity(&out, "SD101"), Some("error"), "{out}");
}

#[test]
fn sd901_a_document_in_no_layer_gives_nothing() {
    let r = Repo::new("standard", "", "", "");
    // The type is unknown, so the document is in no layer (SD303 only).
    r.write(
        "docs/rules/pump.md",
        &doc(
            "valve",
            "active",
            "",
            "See [the change](../intent/pump/intent.md).",
        ),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    let out = r.check();
    assert_eq!(severity(&out, "SD303"), Some("error"), "{out}");
    assert_eq!(severity(&out, "SD901"), None, "{out}");
}

fn decision(outcome: &str, amendment: &str) -> String {
    doc(
        "decision",
        "accepted",
        "",
        &format!("## Decision Outcome\n\n{outcome}\n\n## Amendments\n\n{amendment}"),
    )
}

#[test]
fn sd901_a_locked_decision_that_links_an_intent_gives_sd611_not_sd901() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/decisions/pump.md",
        &decision(
            "Use the pump. See [the change](../intent/pump/intent.md).",
            "None.",
        ),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    let out = r.check();
    assert_eq!(severity(&out, "SD611"), Some("warning"), "{out}");
    assert_eq!(severity(&out, "SD901"), None, "{out}");
}

#[test]
fn sd901_a_link_in_an_amendment_is_not_an_sd611_pair() {
    // SD611 allows the link in an amendment section. SD901 reports it, and
    // the fix of SD611 tells the author to name the intent as plain text.
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/decisions/pump.md",
        &decision(
            "Use the pump.",
            "See [the change](../intent/pump/intent.md).",
        ),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    let out = r.check();
    assert_eq!(severity(&out, "SD611"), None, "{out}");
    assert_eq!(severity(&out, "SD901"), Some("warning"), "{out}");
}

// SD902

fn limited(profile: &str, limit: &str, root: &str, docs: &[(&str, &str)]) -> Repo {
    let r = Repo::new(
        profile,
        "",
        &format!("[layers.rules]\ntypes = [\"rule\"]\n{limit}\n"),
        root,
    );
    for (path, body) in docs {
        r.write(path, &doc("rule", "active", "", body));
    }
    r
}

#[test]
fn sd902_more_documents_than_max_docs_fails_in_each_profile() {
    in_each_profile("SD902", true, |p| {
        limited(
            p,
            "max-docs = 1",
            "",
            &[
                ("docs/rules/a.md", RULE_PLAIN),
                ("docs/rules/b.md", RULE_PLAIN),
            ],
        )
    });
}

#[test]
fn sd902_a_layer_at_max_docs_passes_in_each_profile() {
    in_each_profile("SD902", false, |p| {
        limited(
            p,
            "max-docs = 2",
            "",
            &[
                ("docs/rules/a.md", RULE_PLAIN),
                ("docs/rules/b.md", RULE_PLAIN),
            ],
        )
    });
}

#[test]
fn sd902_more_characters_than_max_chars_fails_in_each_profile() {
    in_each_profile("SD902", true, |p| {
        limited(p, "max-chars = 40", "", &[("docs/rules/a.md", RULE_PLAIN)])
    });
    in_each_profile("SD902", false, |p| {
        limited(
            p,
            "max-chars = 4000",
            "",
            &[("docs/rules/a.md", RULE_PLAIN)],
        )
    });
}

#[test]
fn sd902_is_on_the_configuration_and_the_rules_of_a_root_do_not_apply() {
    let r = limited(
        "standard",
        "max-docs = 1",
        "rules = { layers = \"off\" }",
        &[
            ("docs/rules/a.md", RULE_PLAIN),
            ("docs/rules/b.md", RULE_PLAIN),
        ],
    );
    let out = r.check();
    assert!(
        out.contains("spec-dna.toml:1: warning SD902 layer-limit: layer `rules` has 2 documents, more than `max-docs = 1`"),
        "{out}"
    );
    // The project rule turns it off.
    let r = Repo::new(
        "standard",
        "",
        "[rules]\nlayers = \"off\"\n\n[layers.rules]\ntypes = [\"rule\"]\nmax-docs = 1\n",
        "",
    );
    r.write("docs/rules/a.md", &doc("rule", "active", "", RULE_PLAIN));
    r.write("docs/rules/b.md", &doc("rule", "active", "", RULE_PLAIN));
    assert_eq!(severity(&r.check(), "SD902"), None);
}

#[test]
fn sd902_a_stub_and_a_generated_index_do_not_count() {
    // The layer `research` has no status, so a generated index is valid there.
    let note = "---\ntype: research\n---\n# Note\n\nText.\n";
    let r = Repo::new(
        "standard",
        "",
        "[layers.research]\ntypes = [\"research\"]\nmax-docs = 1\n",
        "",
    );
    r.write("docs/research/a.md", note);
    r.write(
        "docs/research/old.md",
        "---\nmoved-to: research/a\n---\n# Moved\n\nMoved to `research/a`.\n",
    );
    git(&r.dir, &["add", "-A"]);
    r.run(&["index", "docs/research"]);
    assert!(r.dir.join("docs/research/README.md").exists());
    // The stub and the index are in the layer `research`, and they do not count.
    let layers = r.run(&["layer", "list", "--json"]);
    for path in ["docs/research/old.md", "docs/research/README.md"] {
        assert!(layers.contains(&format!("\"path\":\"{path}\"")), "{layers}");
    }
    assert_eq!(
        layers.matches("\"layer\":\"research\"").count(),
        3,
        "{layers}"
    );
    let out = r.check();
    assert_eq!(severity(&out, "SD902"), None, "{out}");
    // A second document counts.
    r.write("docs/research/b.md", note);
    assert_eq!(severity(&r.check(), "SD902"), Some("warning"));
}

// SD903

#[test]
fn sd903_a_date_in_a_requirement_fails_in_each_profile() {
    in_each_profile("SD903", true, |p| {
        let r = Repo::new(p, "", "", "");
        r.write(
            "docs/requirements/pump.md",
            &req_doc("The pump SHALL stop at 90 percent, as set on 2024-03-05."),
        );
        r
    });
}

#[test]
fn sd903_a_current_rule_passes_in_each_profile() {
    in_each_profile("SD903", false, |p| {
        let r = Repo::new(p, "", "", "");
        r.write(
            "docs/requirements/pump.md",
            &req_doc("The pump SHALL stop at 90 percent and keep logs for 30 days."),
        );
        r
    });
}

#[test]
fn sd903_each_date_form_and_each_default_phrase_gives_a_diagnostic() {
    let cases = [
        ("Set on 2024-03-05.", "2024-03-05"),
        ("Set on 2024/03/05.", "2024/03/05"),
        ("Set on 5.3.2024.", "5.3.2024"),
        ("Set on 5/3/2024.", "5/3/2024"),
        ("Set on 5 March 2024.", "5 march 2024"),
        ("Set on March 5, 2024.", "march 5, 2024"),
        ("Set in March 2024.", "march 2024"),
        ("Fixed in round 3.", "round 3"),
        ("Fixed in Iteration 12.", "iteration 12"),
        ("The pump previously ran dry.", "previously"),
        ("The pump formerly ran dry.", "formerly"),
        ("The pump no longer runs dry.", "no longer"),
        ("The pump used to run dry.", "used to"),
        ("The limit was changed.", "was changed"),
        ("Before this change the pump ran.", "before this change"),
        ("After this change the pump stops.", "after this change"),
    ];
    for (text, want) in cases {
        let r = Repo::new("standard", "", "", "");
        r.write(
            "docs/requirements/pump.md",
            &req_doc(&format!("The pump SHALL stop. {text}")),
        );
        let out = r.check();
        assert!(
            out.contains(&format!("warning SD903 history-in-requirements: `pump-1`: the requirement has history (`{want}`)")),
            "{text}: {out}"
        );
    }
}

#[test]
fn sd903_text_that_is_not_history_gives_nothing() {
    for text in [
        "The key SHALL be used to open the valve.",
        "The log SHALL keep 30 days of rows.",
        "See `2024-03-05` and `previously`.",
        "See [the notes](../research/2024-03-05-notes.md).",
        "See [[decisions/2024-03-05-pump]].",
        "The pump SHALL stop at 2024 ms.",
    ] {
        let r = Repo::new("standard", "", "", "");
        r.write(
            "docs/requirements/pump.md",
            &req_doc(&format!("The pump SHALL stop. {text}")),
        );
        let out = r.check();
        assert_eq!(severity(&out, "SD903"), None, "{text}: {out}");
    }
}

#[test]
fn sd903_reads_the_requirements_of_a_rule_and_an_interface_but_not_a_decision() {
    let r = Repo::new("standard", "", "", "");
    let text = "- **pump-1**: The pump SHALL stop. Set on 2024-03-05.";
    r.write(
        "docs/rules/pump.md",
        &doc("rule", "active", "", &format!("## Rules\n\n{text}")),
    );
    r.write(
        "docs/interfaces/pump.md",
        &doc(
            "interface",
            "active",
            "",
            &format!("## Interface\n\n{text}"),
        ),
    );
    r.write(
        "docs/decisions/pump.md",
        &doc(
            "decision",
            "accepted",
            "",
            &format!("## Decision Outcome\n\n{text}"),
        ),
    );
    let out = r.check();
    assert_eq!(out.matches(" SD903 ").count(), 2, "{out}");
    assert!(out.contains("docs/rules/pump.md:"), "{out}");
    assert!(out.contains("docs/interfaces/pump.md:"), "{out}");
}

#[test]
fn sd903_history_terms_replace_the_default_phrases() {
    let make = |terms: &str, text: &str| {
        let r = Repo::new(
            "standard",
            "",
            &format!("[lint]\nhistory-terms = {terms}\n"),
            "",
        );
        r.write("docs/requirements/pump.md", &req_doc(text));
        r.check()
    };
    let text = "The pump SHALL stop. It was superseded by the valve and previously ran dry.";
    let out = make("[\"superseded by\"]", text);
    assert!(out.contains("(`superseded by`)"), "{out}");
    assert!(!out.contains("(`previously`)"), "{out}");
    // An empty list removes the phrases, not the dates.
    let both = "The pump SHALL stop. It previously ran dry, as set on 2024-03-05.";
    let out = make("[]", both);
    assert!(out.contains("(`2024-03-05`)"), "{out}");
    assert!(!out.contains("(`previously`)"), "{out}");
}

#[test]
fn sd903_a_language_other_than_english_does_not_turn_it_off() {
    let r = Repo::new("standard", "language = \"de\"", "", "");
    r.write(
        "docs/requirements/pump.md",
        &req_doc("Die Pumpe SHALL stoppen. Stand 2024-03-05."),
    );
    assert_eq!(severity(&r.check(), "SD903"), Some("warning"));
}

// SD904

const TERMS: &str = "---\ntype: terms\n---\n# Terms\n\n| Term | Meaning | Forbidden synonyms |\n| --- | --- | --- |\n| valve | The part that closes a line. | tap; shut-off device |\n| tray | The bed of a row. | |\n";

fn terms_repo(profile: &str, top: &str, text: &str) -> Repo {
    let r = Repo::new(profile, top, "", "");
    r.write("docs/terms.md", TERMS);
    r.write("docs/requirements/pump.md", &req_doc(text));
    r
}

#[test]
fn sd904_a_forbidden_synonym_fails_in_each_profile() {
    in_each_profile("SD904", true, |p| {
        terms_repo(p, "", "The controller SHALL close the tap.")
    });
}

#[test]
fn sd904_the_term_passes_in_each_profile() {
    in_each_profile("SD904", false, |p| {
        terms_repo(p, "", "The controller SHALL close the valve and the tray.")
    });
}

#[test]
fn sd904_names_the_synonym_and_the_term() {
    let out = terms_repo(
        "standard",
        "",
        "The controller SHALL close the Shut-off device, then the tap. A tape SHALL stay.",
    )
    .check();
    assert!(
        out.contains("`pump-1`: `shut-off device` is a forbidden synonym; use the term `valve`"),
        "{out}"
    );
    assert_eq!(out.matches(" SD904 ").count(), 2, "{out}");
    let json = terms_repo("standard", "", "The controller SHALL close the tap.")
        .run(&["check", "--all", "--json"]);
    assert!(json.contains("\"target\":\"pump-1:tap\""), "{json}");
}

#[test]
fn sd904_reads_the_columns_of_a_table_in_any_order() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/terms.md",
        "---\ntype: terms\n---\n# Terms\n\n| Forbidden synonyms | Term |\n| --- | --- |\n| tap, spigot | valve |\n",
    );
    r.write(
        "docs/requirements/pump.md",
        &req_doc("The controller SHALL close the spigot."),
    );
    assert_eq!(severity(&r.check(), "SD904"), Some("warning"));
}

#[test]
fn sd904_a_language_other_than_english_turns_it_off_with_the_lint() {
    // A requirement with no BCP 14 keyword gives SD801 in `strict`.
    let text = "The controller closes the tap.";
    for (top, on) in [
        ("", true),
        ("language = \"en-GB\"", true),
        ("language = \"de\"", false),
    ] {
        let r = terms_repo("strict", top, text);
        for args in [&["check", "--all"][..], &["check", "--all", "--lint"][..]] {
            let out = r.run(args);
            let want = on.then_some("warning");
            assert_eq!(severity(&out, "SD904"), want, "{top} {args:?}: {out}");
            assert_eq!(severity(&out, "SD801"), want, "{top} {args:?}: {out}");
        }
    }
    // `--lint` turns the lint on in `standard`, for English only.
    for (top, on) in [("", true), ("language = \"de\"", false)] {
        let out = terms_repo("standard", top, text).run(&["check", "--all", "--lint"]);
        assert_eq!(severity(&out, "SD801").is_some(), on, "{top}: {out}");
    }
}

// The new names of SD405 and SD611

#[test]
fn the_names_of_sd611_and_sd405_changed_and_the_codes_did_not() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/decisions/pump.md",
        &decision(
            "Use the pump. See [the change](../intent/pump/intent.md).",
            "None.",
        ),
    );
    r.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    let json = r.run(&["check", "--all", "--json"]);
    assert!(
        json.contains("\"code\":\"SD611\",\"name\":\"decision-links-intent\""),
        "{json}"
    );
    assert!(
        json.contains("name the intent as plain text, not as a link"),
        "{json}"
    );
    assert!(!json.contains("decision-links-change"), "{json}");
    // `[rules]` still names the code.
    let r2 = Repo::new("standard", "", "[rules]\nSD611 = \"off\"\n", "");
    r2.write(
        "docs/decisions/pump.md",
        &decision(
            "Use the pump. See [the change](../intent/pump/intent.md).",
            "None.",
        ),
    );
    r2.write(
        "docs/intent/pump/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    assert_eq!(severity(&r2.check(), "SD611"), None);

    // SD405: the text of an accepted decision changed against the base.
    let r3 = Repo::new("standard", "", "", "");
    r3.write(
        "docs/decisions/pump.md",
        &decision("Use the pump.", "None."),
    );
    git(&r3.dir, &["add", "-A"]);
    git(&r3.dir, &["commit", "-q", "-m", "base"]);
    r3.write(
        "docs/decisions/pump.md",
        &decision("Use the valve.", "None."),
    );
    let json = r3.run(&["check", "--json"]);
    assert!(
        json.contains("\"code\":\"SD405\",\"name\":\"decision-locked\""),
        "{json}"
    );
    assert!(!json.contains("decision-frozen"), "{json}");
    let text = r3.run(&["check"]);
    assert!(text.contains("error SD405 decision-locked: "), "{text}");
}

// SD612

/// The lines of the output of `check` that carry a code.
fn code_lines<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines()
        .filter(|l| l.contains(&format!(" {code} ")))
        .collect()
}

/// The default graph (no `[change]` table) with the change `alpha`. The change
/// `beta` and a roadmap link it, and so does a rule.
fn links_into_alpha(profile: &str) -> Repo {
    let r = Repo::new(profile, "", "", "");
    r.write(
        "docs/intent/alpha/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    r.write(
        "docs/intent/beta/intent.md",
        &doc(
            "intent",
            "draft",
            "",
            "## Intent\n\nAfter [[intent/alpha/intent]].",
        ),
    );
    r.write(
        "docs/roadmap.md",
        &doc("roadmap", "active", "", "- [Alpha](intent/alpha/intent.md)"),
    );
    r.write(
        "docs/rules/pump.md",
        &doc(
            "rule",
            "active",
            "",
            "## Rules\n\n- See [Alpha](../intent/alpha/intent.md).",
        ),
    );
    r
}

#[test]
fn a_link_into_a_change_folder_gives_sd612_once() {
    let out = links_into_alpha("standard").check();
    let found = code_lines(&out, "SD612");
    assert_eq!(found.len(), 2, "{out}");
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("docs/intent/beta/intent.md:")),
        "{out}"
    );
    assert!(
        found.iter().any(|l| l.starts_with("docs/roadmap.md:")),
        "{out}"
    );
    assert!(
        found.iter().all(|l| l.contains(": warning SD612 link-into-change: ")
            && l.contains("names the folder `docs/intent/alpha` of the change `alpha`, and `finish alpha` stops on it; name the change as plain text")),
        "{out}"
    );
    // The rule gives SD901, and no SD612.
    assert_eq!(code_lines(&out, "SD901").len(), 1, "{out}");
    assert!(
        !found.iter().any(|l| l.starts_with("docs/rules/pump.md:")),
        "{out}"
    );
    // SD901 is off in `lite`, so SD612 reports the rule too.
    let out = links_into_alpha("lite").check();
    assert_eq!(code_lines(&out, "SD612").len(), 3, "{out}");
    assert!(code_lines(&out, "SD901").is_empty(), "{out}");
}

#[test]
fn sd612_is_a_warning_in_each_profile_and_a_rule_changes_it() {
    for profile in ["lite", "standard", "strict"] {
        let out = links_into_alpha(profile).check();
        assert_eq!(severity(&out, "SD612"), Some("warning"), "{profile}: {out}");
    }
    let off = Repo::new("standard", "", "[rules]\nSD612 = \"off\"\n", "");
    let r = links_into_alpha("standard");
    let config = std::fs::read_to_string(off.dir.join("spec-dna.toml")).unwrap();
    std::fs::write(r.dir.join("spec-dna.toml"), config).unwrap();
    assert_eq!(severity(&r.check(), "SD612"), None);
    let strict = Repo::new("standard", "", "[rules]\nSD612 = \"error\"\n", "");
    let config = std::fs::read_to_string(strict.dir.join("spec-dna.toml")).unwrap();
    std::fs::write(r.dir.join("spec-dna.toml"), config).unwrap();
    assert_eq!(severity(&r.check(), "SD612"), Some("error"));
}

#[test]
fn sd612_has_no_report_for_a_link_inside_the_folder_or_a_plain_name() {
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/intent/alpha/intent.md",
        &doc(
            "intent",
            "draft",
            "",
            "## Intent\n\nSee [the spec](spec.md).",
        ),
    );
    r.write(
        "docs/intent/alpha/spec.md",
        &doc("spec", "draft", "", "## Delta\n\n- None."),
    );
    r.write(
        "docs/roadmap.md",
        &doc("roadmap", "active", "", "- The change `alpha` is open."),
    );
    let out = r.check();
    assert!(code_lines(&out, "SD612").is_empty(), "{out}");
}

#[test]
fn sd612_pairs_with_the_exact_line_that_sd611_reports() {
    // SD611 reports the id in the locked text. The link in the amendment is a
    // second reference to the same target, and `finish` stops on it.
    let r = Repo::new("standard", "", "", "");
    r.write(
        "docs/decisions/pump.md",
        &decision(
            "Use [[intent/alpha/intent]].",
            "See [Alpha](../intent/alpha/intent.md).",
        ),
    );
    r.write(
        "docs/intent/alpha/intent.md",
        &doc("intent", "draft", "", INTENT),
    );
    let out = r.check();
    let sd611 = code_lines(&out, "SD611");
    let sd612 = code_lines(&out, "SD612");
    assert_eq!(sd611.len(), 1, "{out}");
    assert_eq!(sd612.len(), 1, "{out}");
    assert!(sd612[0].starts_with("docs/decisions/pump.md:"), "{out}");
    let line = |l: &str| l.split(':').nth(1).unwrap().to_string();
    assert_ne!(line(sd611[0]), line(sd612[0]), "{out}");
}

// The budget of an area: SD705 to SD707 of the group `area-budget` (design
// 0.8.0, D55). A fixture has the layers `requirements` and `summaries`, and
// the key `[budgets.area]`. These cases use `common::Repo`, which commits with
// an isolated git identity.

const AREA_LAYERS: &str = "[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

/// The key of the tests: 1,500 and 2,500 characters, and a growth allowance of
/// 100 characters.
const AREA_KEY: &str = "[budgets]\ngrowth = 100\n\n[budgets.area]\nsoft = 1500\nhard = 2500\n";

fn area_repo(profile: &str, tables: &str) -> common::Repo {
    common::Repo::with_config(&format!("profile = \"{profile}\"\n{tables}\n{AREA_LAYERS}"))
}

/// A requirement document of about `pad + 100` body characters.
fn area_doc(key: &str, status: &str, pad: usize) -> String {
    format!(
        "---\nstatus: {status}\n---\n# {key}\n\n## Requirements\n\n- **{key}**: The valve SHALL close after 30 seconds.\n\n## Notes\n\n{}\n",
        "x".repeat(pad)
    )
}

/// The first `n` documents of `a`, `b` and `c` in the area `valves`.
fn valves(r: &common::Repo, n: usize) {
    for key in ["a", "b", "c"].iter().take(n) {
        r.write(
            &format!("docs/requirements/valves/{key}.md"),
            &area_doc(key, "active", 1000),
        );
    }
}

/// The finding lines of SD705 to SD707, not their fix lines.
fn area_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            ["SD705", "SD706", "SD707"]
                .iter()
                .any(|c| l.contains(&format!("error {c} ")) || l.contains(&format!("warning {c} ")))
        })
        .collect()
}

fn run_check(r: &common::Repo, args: &[&str]) -> (i32, String) {
    let (code, out, err) = r.run(args);
    (code, format!("{out}{err}"))
}

/// The number of characters that a finding line names after `has`.
fn count_in(line: &str) -> u32 {
    let at = line.find(" has ").unwrap() + 5;
    line[at..].split(' ').next().unwrap().parse().unwrap()
}

#[test]
fn sd706_a_new_area_over_the_hard_budget_is_an_error_in_standard_and_strict_and_off_in_lite() {
    for (profile, want) in [("lite", None), ("standard", Some(1)), ("strict", Some(1))] {
        let r = area_repo(profile, AREA_KEY);
        valves(&r, 3);
        // No commit: every area is new.
        let (code, out) = run_check(&r, &["check"]);
        let found = area_lines(&out);
        match want {
            None => {
                assert!(found.is_empty(), "{profile}: {out}");
                assert_eq!(code, 0, "{profile}: {out}");
            }
            Some(exit) => {
                assert_eq!(code, exit, "{profile}: {out}");
                assert!(
                    found.iter().any(|l| l.starts_with(
                        "docs/requirements/valves:1: error SD706 area-budget-hard: area `valves` has "
                    ) && l.ends_with("characters, over the hard budget of 2500 (a new area)")),
                    "{profile}: {out}"
                );
                assert!(out.contains("SD706 fix: "), "{out}");
            }
        }
    }
}

#[test]
fn sd707_an_excess_of_the_base_with_no_growth_is_not_new_and_check_all_shows_it() {
    for (profile, severity) in [("standard", "warning"), ("strict", "error")] {
        // The rule `evidence = "off"` takes the SD608 errors of `strict` away.
        let r = area_repo(
            profile,
            &format!("{AREA_KEY}\n[rules]\nevidence = \"off\"\n"),
        );
        valves(&r, 3);
        r.commit("base");
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(area_lines(&out).is_empty(), "{profile}: not new: {out}");
        // `--all` fails on each error it lists, new or not: so on the error
        // SD707 of `strict`, and not on the warning SD707 of `standard`.
        let (code, out) = run_check(&r, &["check", "--all"]);
        assert_eq!(code, i32::from(profile == "strict"), "{profile}: {out}");
        assert!(
            area_lines(&out).iter().any(|l| l.contains(&format!(
                "{severity} SD707 area-budget-hard-kept: area `valves` has "
            )) && l.ends_with("(no growth against the base)")),
            "{profile}: {out}"
        );
        assert!(!out.contains("SD706 area"), "{out}");
    }
}

#[test]
fn sd707_growth_up_to_the_allowance_is_new_and_more_growth_is_sd706() {
    for (profile, exit_small) in [("standard", 0), ("strict", 1)] {
        let r = area_repo(profile, AREA_KEY);
        valves(&r, 3);
        r.commit("base");
        // Exactly `growth` characters more: SD707, new.
        r.write(
            "docs/requirements/valves/a.md",
            &area_doc("a", "active", 1100),
        );
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, exit_small, "{profile}: {out}");
        assert!(
            area_lines(&out)
                .iter()
                .any(|l| l.contains(" SD707 area-budget-hard-kept: ")
                    && l.ends_with("(grew by 100 characters, at most 100 in one change)")),
            "{profile}: {out}"
        );
        // One character more: SD706, an error in each profile.
        r.write(
            "docs/requirements/valves/a.md",
            &area_doc("a", "active", 1101),
        );
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, 1, "{profile}: {out}");
        assert!(
            area_lines(&out)
                .iter()
                .any(|l| l.contains("error SD706 area-budget-hard: ")
                    && l.contains("(grew from ")
                    && l.ends_with(" characters in the base)")),
            "{profile}: {out}"
        );
        // Less text: no growth, so not new.
        r.write(
            "docs/requirements/valves/a.md",
            &area_doc("a", "active", 900),
        );
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(area_lines(&out).is_empty(), "{profile}: {out}");
    }
}

#[test]
fn sd706_an_area_that_was_under_the_hard_budget_in_the_base_is_an_error() {
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 2);
    r.commit("base");
    valves(&r, 3);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        area_lines(&out)
            .iter()
            .any(|l| l.contains("error SD706 ") && l.contains("characters in the base)")),
        "{out}"
    );
}

#[test]
fn sd705_the_soft_budget_is_a_warning_in_standard_an_error_in_strict_and_new_only_when_crossed() {
    for (profile, severity, exit) in [("standard", "warning", 0), ("strict", "error", 1)] {
        let r = area_repo(profile, AREA_KEY);
        valves(&r, 2);
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, exit, "{profile}: {out}");
        assert!(
            area_lines(&out)
                .iter()
                .any(|l| l.contains(&format!(
                    "{severity} SD705 area-budget-soft: area `valves` has "
                )) && l.ends_with("characters, over the soft budget of 1500")),
            "{profile}: {out}"
        );
        assert!(
            !out.contains("SD706 area") && !out.contains("SD707 area"),
            "{out}"
        );
        // In the base, it is not new.
        r.commit("base");
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, 0, "{profile}: {out}");
        assert!(area_lines(&out).is_empty(), "{profile}: {out}");
        let (_, out) = run_check(&r, &["check", "--all"]);
        assert!(out.contains("SD705 area-budget-soft"), "{profile}: {out}");
    }
    // An area that crosses the soft budget in the change is new.
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 1);
    r.commit("base");
    valves(&r, 2);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("warning SD705 area-budget-soft"), "{out}");
}

#[test]
fn an_area_budget_that_a_project_adds_over_an_existing_excess_gives_sd707_not_sd706() {
    let r = area_repo("standard", "");
    valves(&r, 3);
    r.commit("base");
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(area_lines(&out).is_empty(), "no table, no code: {out}");
    // The table is in the head. The excess is in the base too.
    let text = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &text.replacen("\n[layers.", &format!("\n{AREA_KEY}\n[layers."), 1),
    );
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(area_lines(&out).is_empty(), "{out}");
    let (code, out) = run_check(&r, &["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("warning SD707 area-budget-hard-kept"), "{out}");
    assert!(!out.contains("SD706 area"), "{out}");
}

#[test]
fn a_project_with_no_area_budget_gives_no_code_of_the_group_in_any_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = area_repo(profile, "");
        valves(&r, 3);
        let (_, out) = run_check(&r, &["check", "--all"]);
        assert!(area_lines(&out).is_empty(), "{profile}: {out}");
        let (_, json) = run_check(&r, &["check", "--all", "--json"]);
        assert!(
            !json.contains("SD705") && !json.contains("SD706") && !json.contains("SD707"),
            "{profile}: {json}"
        );
    }
    // An empty table has no budget: 0 turns a budget off.
    let r = area_repo("standard", "[budgets.area]\n");
    valves(&r, 3);
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(area_lines(&out).is_empty(), "{out}");
    // A key with only `hard` has no soft budget.
    let r = area_repo("standard", "[budgets.area]\nhard = 2500\n");
    valves(&r, 2);
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(area_lines(&out).is_empty(), "{out}");
    valves(&r, 3);
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(out.contains("SD706 area-budget-hard"), "{out}");
}

#[test]
fn a_summary_counts_in_place_of_the_documents_of_its_sub_area() {
    let r = area_repo("standard", AREA_KEY);
    r.write(
        "docs/requirements/valves/a.md",
        &area_doc("a", "active", 500),
    );
    for key in ["b", "c"] {
        r.write(
            &format!("docs/requirements/valves/inner/{key}.md"),
            &area_doc(key, "active", 1200),
        );
    }
    // No summary: `valves` counts the documents of `inner`, and so does the
    // root area.
    let (_, out) = run_check(&r, &["check", "--all"]);
    let found = area_lines(&out);
    let targets: Vec<&str> = found
        .iter()
        .filter(|l| l.contains(" SD706 "))
        .map(|l| l.split(':').next().unwrap())
        .collect();
    assert_eq!(
        targets,
        [
            "docs/requirements",
            "docs/requirements/valves",
            "docs/requirements/valves/inner"
        ],
        "{out}"
    );
    let before = count_in(found.iter().find(|l| l.contains("`valves`")).unwrap());
    assert!(before > 2500, "{out}");
    // With a short summary of `inner`, the finding of `inner` is on the
    // summary, and `valves` counts the summary and its own document only.
    r.write(
        "docs/summaries/valves/inner.md",
        "---\nstatus: active\n---\n# Inner\n\nThe inner valves close after 30 seconds.\n",
    );
    let (_, out) = run_check(&r, &["check", "--all"]);
    let found = area_lines(&out);
    assert!(
        found.iter().any(
            |l| l.starts_with("docs/summaries/valves/inner.md:1: error SD706 ")
                && l.contains("area `valves/inner` has ")
        ),
        "{out}"
    );
    assert!(
        !found
            .iter()
            .any(|l| l.contains("area `valves` ") || l.contains("area `.` ")),
        "{out}"
    );
}

#[test]
fn a_retired_document_does_not_count_and_a_draft_does() {
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 2);
    r.write(
        "docs/requirements/valves/c.md",
        &area_doc("c", "retired", 1000),
    );
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(!out.contains("SD706 area"), "{out}");
    r.write(
        "docs/requirements/valves/c.md",
        &area_doc("c", "draft", 1000),
    );
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(out.contains("error SD706 area-budget-hard"), "{out}");
}

#[test]
fn the_rules_of_the_group_area_budget_and_of_each_code_set_the_severity() {
    // `lite` has the codes off, and the group turns them on.
    let r = area_repo(
        "lite",
        &format!("{AREA_KEY}\n[rules]\narea-budget = \"error\"\n"),
    );
    valves(&r, 3);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD706 area-budget-hard"), "{out}");
    // A rule for the group `budgets` does not change the codes.
    let r = area_repo(
        "standard",
        &format!("{AREA_KEY}\n[rules]\nbudgets = \"off\"\n"),
    );
    valves(&r, 3);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD706 area-budget-hard"), "{out}");
    // A rule for the group, and a rule for the code, turn them off.
    for rule in ["area-budget = \"off\"", "SD706 = \"off\""] {
        let r = area_repo("standard", &format!("{AREA_KEY}\n[rules]\n{rule}\n"));
        valves(&r, 3);
        let (code, out) = run_check(&r, &["check"]);
        assert_eq!(code, 0, "{rule}: {out}");
        assert!(!out.contains("SD706 area"), "{rule}: {out}");
    }
}

#[test]
fn the_json_of_a_finding_names_the_folder_and_the_area() {
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 3);
    let (_, json) = run_check(&r, &["check", "--json"]);
    // The root area is over the budget too: take the object of `valves`.
    let obj = json
        .match_indices("\"code\":\"SD706\"")
        .map(|(at, _)| &json[at..at + json[at..].find('}').unwrap()])
        .find(|o| o.contains("\"target\":\"valves\""))
        .expect(&json);
    for part in [
        "\"name\":\"area-budget-hard\"",
        "\"severity\":\"error\"",
        "\"new\":true",
        "\"path\":\"docs/requirements/valves\"",
        "\"target\":\"valves\"",
    ] {
        assert!(obj.contains(part), "{part}: {obj}");
    }
}

#[test]
fn the_root_area_is_the_folder_of_the_layer_and_a_renamed_area_is_a_new_area() {
    let r = area_repo("standard", AREA_KEY);
    for key in ["a", "b", "c"] {
        r.write(
            &format!("docs/requirements/{key}.md"),
            &area_doc(key, "active", 1000),
        );
    }
    let (_, out) = run_check(&r, &["check", "--all"]);
    let found = area_lines(&out);
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("docs/requirements:1: error SD706 ")
                && l.contains("area `.` has ")),
        "{out}"
    );
    // A folder that moves is a new area: no base area has its path.
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 3);
    r.commit("base");
    r.git(&["mv", "docs/requirements/valves", "docs/requirements/pumps"]);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        area_lines(&out)
            .iter()
            .any(|l| l.contains("area `pumps` ") && l.ends_with("(a new area)")),
        "{out}"
    );
}

// P5: a project with no `[layers]` table and no `[budgets.area]` keeps its
// output byte for byte. The expected text is the output of 0.7.1 for the
// fixture: the same code gave it before D55. The area has 14 documents of
// about 9,100 characters: more than 120,000, the hard budget that `init`
// writes for `standard`. So a default budget in place of the missing table
// would show in the output.

const P5_KEYS: [&str; 14] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
];

fn p5_repo(profile: &str) -> common::Repo {
    let r = common::Repo::with_config(&format!(
        "profile = \"{profile}\"\n\n[rules]\nevidence = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
    ));
    for key in P5_KEYS {
        r.write(
            &format!("docs/requirements/valves/{key}.md"),
            &area_doc(key, "active", 9000),
        );
    }
    r
}

#[test]
fn a_project_with_no_layers_table_and_no_area_budget_keeps_its_output_byte_for_byte() {
    let mut outs = Vec::new();
    for profile in ["lite", "standard", "strict"] {
        let r = p5_repo(profile);
        let (code, out) = run_check(&r, &["check", "--all"]);
        outs.push((code, out));
    }
    // The rule `evidence = "off"` takes the SD608 lines out of the text.
    let want = (
        0,
        "spec-dna check: 14 documents; 0 errors and 0 warnings\n".to_string(),
    );
    assert_eq!(outs, [want.clone(), want.clone(), want]);
}

#[test]
fn only_the_highest_summary_of_a_branch_counts() {
    let r = area_repo("standard", AREA_KEY);
    r.write(
        "docs/requirements/valves/a.md",
        &area_doc("a", "active", 400),
    );
    for key in ["x", "y"] {
        r.write(
            &format!("docs/requirements/valves/inner/deep/{key}.md"),
            &area_doc(key, "active", 1200),
        );
    }
    let summary = |title: &str| {
        format!(
            "---\nstatus: active\n---\n# {title}\n\n{}\n",
            "s".repeat(700)
        )
    };
    r.write("docs/summaries/valves/inner.md", &summary("Inner"));
    r.write("docs/summaries/valves/inner/deep.md", &summary("Deep"));
    let (_, out) = run_check(&r, &["check", "--all"]);
    let found = area_lines(&out);
    // `valves` counts its document and the summary of `inner`: about 1,300
    // characters, under the soft budget. The summary of `deep` is not added.
    assert!(!found.iter().any(|l| l.contains("area `valves` ")), "{out}");
    // `inner` counts the summary of `deep` in place of its 2 documents.
    assert!(
        !found.iter().any(|l| l.contains("area `valves/inner` ")),
        "{out}"
    );
    // `deep` counts its 2 documents, and the finding is on its summary.
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("docs/summaries/valves/inner/deep.md:1: error SD706 ")),
        "{out}"
    );
}

#[test]
fn a_stub_and_a_generated_index_do_not_count_in_the_size_of_an_area() {
    let r = area_repo("standard", AREA_KEY);
    valves(&r, 1);
    let pad = "x".repeat(3000);
    r.write(
        "docs/requirements/valves/old.md",
        &format!("---\nmoved-to: requirements/valves/a\n---\n# Moved\n\n{pad}\n"),
    );
    r.write(
        "docs/requirements/valves/README.md",
        &format!(
            "---\ngenerated: spec-dna index 0000000000000000\nstatus: active\n---\n# Index\n\n{pad}\n"
        ),
    );
    let (_, out) = run_check(&r, &["check", "--all"]);
    assert!(area_lines(&out).is_empty(), "{out}");
}

#[test]
fn a_document_of_the_layer_outside_its_folder_is_in_the_root_area() {
    let r = common::Repo::with_config(&format!(
        "profile = \"standard\"\n{AREA_KEY}\n[layers.requirements]\ntypes = [\"requirements\"]\npaths = [\"specs/*.md\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    ));
    for key in ["a", "b", "c"] {
        let text = area_doc(key, "active", 1000).replacen("---\n", "---\ntype: requirements\n", 1);
        r.write(&format!("docs/specs/{key}.md"), &text);
    }
    let (_, out) = run_check(&r, &["check", "--all"]);
    let found = area_lines(&out);
    assert!(
        found
            .iter()
            .any(|l| l.starts_with("docs/requirements:1: error SD706 ")
                && l.contains("area `.` has ")),
        "{out}"
    );
    assert_eq!(
        found.iter().filter(|l| l.contains(" SD706 ")).count(),
        1,
        "{out}"
    );
}

#[test]
fn the_key_works_with_no_layers_table_on_the_default_graph() {
    let r = common::Repo::with_config(&format!(
        "profile = \"standard\"\n{AREA_KEY}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
    ));
    valves(&r, 3);
    let (code, out) = run_check(&r, &["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        area_lines(&out)
            .iter()
            .any(|l| l.starts_with("docs/requirements/valves:1: error SD706 ")),
        "{out}"
    );
}
