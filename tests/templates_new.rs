//! Integration tests of the templates of `spec-dna new`: each embedded
//! template, `--template`, the project folder `[templates] dir`, the live
//! section `Status` of a decision, and the rule that the templates folder is
//! never part of a root. The helpers are small copies of the ones in
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

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    /// The codes of `check --all --json` for the documents of the repository.
    fn codes(&self) -> Vec<String> {
        git(&self.dir, &["add", "-A"]);
        let json = self.run(&["check", "--all", "--json"]).1;
        let mut out = Vec::new();
        let mut rest = json.as_str();
        while let Some(at) = rest.find("\"code\":\"") {
            rest = &rest[at + 8..];
            out.push(rest.split('"').next().unwrap().to_string());
        }
        out
    }
}

/// The default and the named templates: the name, the arguments of `new`, the
/// path of the new file and the type.
const CASES: [(&str, &[&str], &str); 18] = [
    ("product", &["new", "product"], "docs/product.md"),
    ("system", &["new", "system"], "docs/system.md"),
    ("terms", &["new", "terms"], "docs/terms.md"),
    ("roadmap", &["new", "roadmap"], "docs/roadmap.md"),
    ("questions", &["new", "questions"], "docs/questions.md"),
    (
        "requirements",
        &["new", "requirements", "valves"],
        "docs/requirements/valves.md",
    ),
    (
        "interface",
        &["new", "interface", "valves"],
        "docs/interfaces/valves.md",
    ),
    ("rule", &["new", "rule", "water"], "docs/rules/water.md"),
    (
        "procedure",
        &["new", "procedure", "flush"],
        "docs/procedures/flush.md",
    ),
    (
        "runbook",
        &["new", "procedure", "flush", "--template", "runbook"],
        "docs/procedures/flush.md",
    ),
    (
        "madr",
        &["new", "decision", "pump"],
        "docs/decisions/pump.md",
    ),
    (
        "madr",
        &["new", "decision", "pump", "--template", "madr"],
        "docs/decisions/pump.md",
    ),
    (
        "post-mortem",
        &["new", "incident", "leak"],
        "docs/incidents/leak.md",
    ),
    (
        "research",
        &["new", "research", "notes"],
        "docs/research/notes.md",
    ),
    (
        "intent",
        &["new", "intent", "drip"],
        "docs/intent/drip/intent.md",
    ),
    (
        "prd",
        &["new", "intent", "drip", "--template", "prd"],
        "docs/intent/drip/intent.md",
    ),
    (
        "rfc",
        &["new", "intent", "drip", "--template", "rfc"],
        "docs/intent/drip/intent.md",
    ),
    (
        "intent",
        &["new", "intent", "drip", "--template", "intent"],
        "docs/intent/drip/intent.md",
    ),
];

/// The codes that a template of a good form never gives: a wrong type, a wrong
/// status and a missing required section.
const FORM_CODES: [&str; 3] = ["SD303", "SD304", "SD305"];

#[test]
fn each_template_is_its_type_at_its_place_with_a_placeholder_under_700_bytes() {
    for (name, args, path) in CASES {
        let r = Repo::mixed();
        let (code, out, err) = r.run(args);
        assert_eq!(code, 0, "{name}: {out}{err}");
        let text = r.read(path);
        assert!(text.len() < 700, "{name}: {} bytes", text.len());
        assert!(!text.contains("type:"), "{name}: the place gives the type");
        let codes = r.codes();
        assert!(codes.iter().any(|c| c == "SD306"), "{name}: {codes:?}");
        for bad in FORM_CODES {
            assert!(
                !codes.iter().any(|c| c == bad),
                "{name}: {bad} in {codes:?}"
            );
        }
    }
}

#[test]
fn spec_and_plan_have_their_required_sections() {
    let r = Repo::mixed();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    for (ty, section) in [("spec", "## Delta"), ("plan", "## Files")] {
        let (code, out, err) = r.run(&["new", ty, "drip"]);
        assert_eq!(code, 0, "{ty}: {out}{err}");
        let text = r.read(&format!("docs/intent/drip/{ty}.md"));
        assert!(text.contains(section), "{text}");
        assert!(text.len() < 700, "{ty}: {} bytes", text.len());
        assert!(
            text.starts_with("---\nstatus: draft\n---\n# {{The title of the change.}}\n"),
            "{text}"
        );
    }
    let codes = r.codes();
    for bad in FORM_CODES {
        assert!(!codes.iter().any(|c| c == bad), "{bad} in {codes:?}");
    }
}

#[test]
fn the_sections_of_prd_rfc_and_runbook_follow_the_design() {
    let sections = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| l.strip_prefix("## ").map(str::to_string))
            .collect()
    };
    let r = Repo::mixed();
    r.run(&["new", "intent", "a", "--template", "prd"]);
    assert_eq!(
        sections(&r.read("docs/intent/a/intent.md")),
        [
            "Problem",
            "Users",
            "Goals",
            "Non-goals",
            "Success measures",
            "Open questions"
        ]
    );
    r.run(&["new", "intent", "b", "--template", "rfc"]);
    assert_eq!(
        sections(&r.read("docs/intent/b/intent.md")),
        [
            "Summary",
            "Motivation",
            "Proposal",
            "Alternatives",
            "Unresolved"
        ]
    );
    r.run(&["new", "procedure", "c", "--template", "runbook"]);
    assert_eq!(
        sections(&r.read("docs/procedures/c.md")),
        ["When to use", "Steps", "Verify", "Rollback"]
    );
    r.run(&["new", "procedure", "d"]);
    assert_eq!(
        sections(&r.read("docs/procedures/d.md")),
        ["Purpose", "Steps"]
    );
    r.run(&["new", "incident", "e"]);
    let incident = r.read("docs/incidents/e.md");
    assert_eq!(
        sections(&incident),
        [
            "Summary",
            "Timeline",
            "Root cause",
            "Links",
            "Follow-ups",
            "Eval"
        ]
    );
    assert!(incident.contains("- [ ] {{Action.}}"), "{incident}");
    r.run(&["new", "decision", "f"]);
    assert_eq!(
        sections(&r.read("docs/decisions/f.md")),
        [
            "Context and Problem Statement",
            "Considered Options",
            "Decision Outcome",
            "Consequences",
            "Status",
            "Amendments"
        ]
    );
    r.run(&["new", "terms"]);
    assert!(
        r.read("docs/terms.md")
            .contains("| Term | Meaning | Forbidden synonyms |"),
    );
}

#[test]
fn a_decision_has_the_first_status_the_title_and_the_status_section_before_amendments() {
    let r = Repo::mixed();
    assert_eq!(r.run(&["new", "decision", "pump-choice"]).0, 0);
    let text = r.read("docs/decisions/pump-choice.md");
    assert!(
        text.starts_with("---\nstatus: proposed\n---\n# Pump choice\n\n"),
        "{text}"
    );
    assert!(text.find("## Status").unwrap() < text.find("## Amendments").unwrap());
}

#[test]
fn a_project_template_replaces_the_embedded_default_and_keeps_its_own_text() {
    let r = Repo::mixed();
    r.write(
        ".spec-dna/templates/madr.md",
        "---\ntype: decision\nwas-type: note\nstatus: accepted\n---\n# Old title\n\n## Decision Outcome\n\n{{Our own outcome.}}\n\n## Status\n\n{{State.}}\n",
    );
    let (code, out, err) = r.run(&["new", "decision", "pump-choice"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read("docs/decisions/pump-choice.md"),
        "---\nstatus: proposed\n---\n# Pump choice\n\n## Decision Outcome\n\n{{Our own outcome.}}\n\n## Status\n\n{{State.}}\n"
    );
    // The same file, by name.
    let (code, _, err) = r.run(&["new", "decision", "valve", "--template", "madr"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        r.read("docs/decisions/valve.md")
            .starts_with("---\nstatus: proposed\n---\n# Valve\n\n## Decision Outcome\n\n{{Our own")
    );
    // The other types keep their embedded default.
    assert_eq!(r.run(&["new", "incident", "leak"]).0, 0);
    assert!(r.read("docs/incidents/leak.md").contains("## Eval"));
}

#[test]
fn a_project_template_of_another_name_selects_with_the_option() {
    let r = Repo::with(
        "[templates]\ndir = \"tpl/\"\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.write(
        "tpl/short.md",
        "---\ntype: \"decision\"\n---\n# Short\n\n## Decision Outcome\n\n{{Outcome.}}\n",
    );
    // The default folder is not read where `dir` names another one.
    r.write(
        ".spec-dna/templates/short.md",
        "---\ntype: rule\n---\n# Wrong\n",
    );
    let (code, out, err) = r.run(&["new", "decision", "pump", "--template", "short"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("wrote docs/decisions/pump.md (short template)"),
        "{out}"
    );
    assert_eq!(
        r.read("docs/decisions/pump.md"),
        "---\nstatus: proposed\n---\n# Pump\n\n## Decision Outcome\n\n{{Outcome.}}\n"
    );
    // A name with the suffix `.md` is the same name.
    assert_eq!(
        r.run(&["new", "decision", "pump2", "--template", "short.md"])
            .0,
        0
    );
}

#[test]
fn a_project_template_loses_type_and_was_type_where_the_place_gives_the_type() {
    let r = Repo::with(
        "",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"research\"\n",
    );
    r.write(
        ".spec-dna/templates/note.md",
        "---\ntype: research\nwas-type: note\n---\n# Note\n\n{{Text.}}\n",
    );
    // The folder `research/` gives the type: no `type:` line, no `was-type`.
    assert_eq!(r.run(&["new", "research", "a", "--template", "note"]).0, 0);
    assert_eq!(r.read("docs/research/a.md"), "# A\n\n{{Text.}}\n");
}

#[test]
fn a_wrong_template_gives_exit_2_and_writes_nothing() {
    let r = Repo::mixed();
    r.write(
        ".spec-dna/templates/rule-form.md",
        "---\ntype: rule\n---\n# R\n",
    );
    r.write(".spec-dna/templates/untyped.md", "# U\n");
    r.write(".spec-dna/templates/old.md", "---\ntype: guide\n---\n# G\n");
    for (args, want) in [
        (
            &["new", "decision", "a", "--template", "nothing"][..],
            "unknown template `nothing`",
        ),
        (
            &["new", "decision", "a", "--template", "prd"][..],
            "is a template of type `intent`, not `decision`",
        ),
        (
            &["new", "decision", "a", "--template", "rule-form"][..],
            "is a template of type `rule`, not `decision`",
        ),
        (
            &["new", "decision", "a", "--template", "untyped"][..],
            "has no `type` in its front matter",
        ),
        (
            &["new", "decision", "a", "--template", "old"][..],
            "is a template of type `guide`, not `decision`",
        ),
        (
            &["new", "procedure", "a", "--template", "../x"][..],
            "is not a template name",
        ),
        (
            &["new", "terms", "--template", "madr"][..],
            "is a template of type `decision`, not `terms`",
        ),
    ] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(err.contains(want), "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}: {out}");
    }
    assert!(!r.exists("docs"));
    // A default template of another type gives exit 2 too.
    r.write(
        ".spec-dna/templates/post-mortem.md",
        "---\ntype: rule\n---\n# R\n",
    );
    let (code, _, err) = r.run(&["new", "incident", "a"]);
    assert_eq!(code, 2, "{err}");
    assert!(!r.exists("docs"));
}

#[test]
fn the_templates_folder_is_never_part_of_a_root() {
    // A root `.` holds the folder of the templates, and a template has
    // placeholders. `check` does not read it, with no `exclude`.
    let r = Repo::with("", "[[roots]]\npath = \".\"\ntype = \"mixed\"\n");
    r.write(
        ".spec-dna/templates/madr.md",
        "---\ntype: decision\n---\n# T\n\n## Decision Outcome\n\n{{Outcome.}}\n",
    );
    r.write("terms.md", "# Terms\n\nA valve closes a pipe.\n");
    let json = {
        git(&r.dir, &["add", "-A"]);
        r.run(&["check", "--all", "--json"])
    };
    assert_eq!(json.0, 0, "{}{}", json.1, json.2);
    assert!(!json.1.contains(".spec-dna"), "{}", json.1);
    assert!(!r.codes().iter().any(|c| c == "SD306"));
    // `new` still reads it.
    let (code, out, err) = r.run(&["new", "decision", "pump"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("decisions/pump.md").contains("{{Outcome.}}"));
}

#[test]
fn the_templates_dir_key_is_checked() {
    for (cfg, want) in [
        ("[templates]\ndir = \"../x\"\n", "templates.dir"),
        ("[templates]\ndir = \"/x\"\n", "templates.dir"),
        ("[templates]\ndir = \".\"\n", "templates.dir"),
        ("[templates]\ndir = \"\"\n", "templates.dir"),
        ("[templates]\ndir = 3\n", "templates.dir"),
        ("[templates]\nfolder = \"x\"\n", "templates.folder"),
        ("templates = \"x\"\n", "templates"),
    ] {
        let r = Repo::with(cfg, "[[roots]]\npath = \"docs\"\n");
        let (code, _, err) = r.run(&["check"]);
        assert_eq!(code, 3, "{cfg}: {err}");
        assert!(err.contains(want), "{cfg}: {err}");
    }
    // A good value in each form.
    for cfg in [
        "[templates]\ndir = \"tpl\"\n",
        "[templates]\ndir = \"./a/b/\"\n",
        "[templates]\n",
    ] {
        let r = Repo::with(cfg, "[[roots]]\npath = \"docs\"\n");
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{cfg}: {out}{err}");
    }
}

/// Writes an accepted decision with the default sections, and commits it.
fn accepted_decision(r: &Repo) {
    assert_eq!(r.run(&["new", "decision", "pump"]).0, 0);
    let path = "docs/decisions/pump.md";
    let mut text = r.read(path).replace("status: proposed", "status: accepted");
    for (from, to) in [
        (
            "{{The problem in 2 or 3 sentences.}}",
            "The pump is too slow.",
        ),
        ("- {{Option.}}\n- {{Option.}}", "- Pump A\n- Pump B"),
        (
            "Chosen option: {{option}}, because {{reason}}.",
            "Chosen option: Pump A, because it is faster.",
        ),
        (
            "- Good: {{effect}}.\n- Bad: {{effect}}.",
            "- Good: faster.\n- Bad: dearer.",
        ),
        (
            "{{The state of the work that the decision starts. This section can change in place.}}",
            "Step 1 is open.",
        ),
    ] {
        assert!(text.contains(from), "{from}");
        text = text.replace(from, to);
    }
    r.write(path, &text);
    r.commit("decision");
}

#[test]
fn the_status_section_of_an_accepted_decision_is_live_by_default() {
    let r = Repo::mixed();
    accepted_decision(&r);
    let path = "docs/decisions/pump.md";
    // A change of the live section: no SD405.
    r.write(
        path,
        &r.read(path).replace("Step 1 is open.", "Step 1 is done."),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD405"), "{out}");
    r.commit("status");
    // A change of the frozen text: SD405.
    r.write(path, &r.read(path).replace("is faster", "is cheaper"));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("SD405 decision-locked"), "{out}");
}

#[test]
fn an_empty_live_sections_list_turns_the_default_off() {
    let r = Repo::with(
        "",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = { decision = [] }\n",
    );
    accepted_decision(&r);
    let path = "docs/decisions/pump.md";
    r.write(
        path,
        &r.read(path).replace("Step 1 is open.", "Step 1 is done."),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("SD405 decision-locked"), "{out}");
}

#[test]
fn an_amendment_goes_after_the_status_section() {
    let r = Repo::mixed();
    accepted_decision(&r);
    let path = "docs/decisions/pump.md";
    r.write(
        path,
        &format!(
            "{}\n2026-01-01: The team chose Pump C instead.\n",
            r.read(path)
        ),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD405"), "{out}");
}

// The title of a document of a change (design 0.7.0, section 15)

#[test]
fn a_change_document_gets_a_title_placeholder_not_a_title_from_the_slug() {
    let r = Repo::mixed();
    for (ty, line) in [("intent", 5), ("spec", 4), ("plan", 4)] {
        let (code, out, err) = r.run(&["new", ty, "v2-api-client"]);
        assert_eq!(code, 0, "{ty}: {out}{err}");
        let path = format!("docs/intent/v2-api-client/{ty}.md");
        let text = r.read(&path);
        assert!(
            text.lines().any(|l| l == "# {{The title of the change.}}"),
            "{ty}: {text}"
        );
        assert!(!text.contains("# V2 api client"), "{ty}: {text}");
        // The H1 is the line `line`, and SD306 reports it.
        assert_eq!(
            text.lines().nth(line - 1),
            Some("# {{The title of the change.}}"),
            "{ty}: {text}"
        );
        git(&r.dir, &["add", "-A"]);
        let (_, out, _) = r.run(&["check", "--all"]);
        assert!(
            out.lines()
                .any(|l| l.starts_with(&format!("{path}:{line}: error SD306 "))),
            "{ty}: {out}"
        );
    }
}

#[test]
fn a_project_template_of_a_change_gets_the_title_placeholder_too() {
    let r = Repo::mixed();
    r.write(
        ".spec-dna/templates/plain.md",
        "---\ntype: intent\n---\n# T\n\n## Intent\n\n{{Text.}}\n",
    );
    assert_eq!(
        r.run(&["new", "intent", "drip", "--template", "plain"]).0,
        0
    );
    assert!(
        r.read("docs/intent/drip/intent.md")
            .contains("\n# {{The title of the change.}}\n")
    );
}

#[test]
fn a_document_that_is_not_of_a_change_keeps_the_title_of_its_id() {
    let r = Repo::mixed();
    assert_eq!(r.run(&["new", "requirements", "water-supply"]).0, 0);
    assert!(
        r.read("docs/requirements/water-supply.md")
            .contains("\n# Water supply\n")
    );
    assert_eq!(r.run(&["new", "decision", "use-a-timer"]).0, 0);
    assert!(
        r.read("docs/decisions/use-a-timer.md")
            .contains("# Use a timer\n")
    );
}

#[test]
fn the_spec_template_writes_a_task_box_in_the_concern() {
    let r = Repo::mixed();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    let text = r.read("docs/intent/drip/spec.md");
    assert!(
        text.contains("## Concerns\n\n- [ ] {{An open concern.}}\n"),
        "{text}"
    );
    assert!(
        text.contains("## Open questions\n\n- [ ] {{A question that has no answer yet.}}\n"),
        "{text}"
    );
}

// The section `Invariants` of the spec template (design 0.8.0, D6).

const KEYED: &str = "[change]\npolicy = \"three-class\"\ninvariants = true\n\n";

fn keyed_repo() -> Repo {
    Repo::with(KEYED, "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n")
}

#[test]
fn new_spec_writes_the_section_invariants_with_none_only_with_the_key() {
    let r = keyed_repo();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    let text = r.read("docs/intent/drip/spec.md");
    assert!(
        text.contains("## Decisions\n\n{{The decisions that this change needs.}}\n\n## Invariants\n\n- none\n\n## Concerns\n"),
        "{text}"
    );
    // The section is valid at once: the item `none` gives no SD936.
    let codes = r.codes();
    assert!(
        !codes.iter().any(|c| c == "SD936" || c == "SD937"),
        "{codes:?}"
    );
    // Without the key, the template has no such section.
    let r = Repo::mixed();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    assert!(!r.read("docs/intent/drip/spec.md").contains("Invariants"));
    // With the key set to false, none either.
    let r = Repo::with(
        "[change]\npolicy = \"three-class\"\ninvariants = false\n\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    assert!(!r.read("docs/intent/drip/spec.md").contains("Invariants"));
}

#[test]
fn the_key_adds_the_section_to_a_project_template_that_lacks_it_and_keeps_one_that_has_it() {
    let r = keyed_repo();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    r.write(
        "docs/templates/spec.md",
        "---\ntype: spec\n---\n# Title\n\n## Delta\n\n{{Lines.}}\n\n## Concerns\n\n- [ ] {{A concern.}}\n",
    );
    r.write("spec-dna.toml", &format!(
        "spec-dna = \"{VERSION}\"\n{KEYED}[templates]\ndir = \"docs/templates\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    ));
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    let text = r.read("docs/intent/drip/spec.md");
    assert!(
        text.contains("## Delta\n\n{{Lines.}}\n\n## Invariants\n\n- none\n\n## Concerns\n"),
        "{text}"
    );
    assert_eq!(text.matches("## Invariants").count(), 1, "{text}");
    // A template that has the section keeps its own text.
    let r = keyed_repo();
    assert_eq!(r.run(&["new", "intent", "drip"]).0, 0);
    r.write(
        "docs/templates/spec.md",
        "---\ntype: spec\n---\n# Title\n\n## Delta\n\n{{Lines.}}\n\n## Invariants\n\n- {{A rule.}}\n",
    );
    r.write("spec-dna.toml", &format!(
        "spec-dna = \"{VERSION}\"\n{KEYED}[templates]\ndir = \"docs/templates\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    ));
    assert_eq!(r.run(&["new", "spec", "drip"]).0, 0);
    let text = r.read("docs/intent/drip/spec.md");
    assert_eq!(text.matches("## Invariants").count(), 1, "{text}");
    assert!(text.contains("- {{A rule.}}"), "{text}");
    assert!(!text.contains("- none"), "{text}");
}
