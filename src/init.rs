//! `spec-dna init` (docs/init.md). It detects the facts of the repository
//! (`detect.rs`), asks at most 8 questions with the detected defaults, shows a
//! preview (the full `spec-dna.toml` and the files to create or change), and
//! writes only after a confirmation. It never edits a document. It changes an
//! instruction file (`AGENTS.md`, `CLAUDE.md`) only by its one generated line.
//!
//! Modes: interactive (a terminal), `--non-interactive` (the detected
//! defaults; also without a terminal), `--answers <file>`, `--reconfigure`
//! (the current values as defaults) and `--dry-run` (the preview only). If
//! `spec-dna.toml` exists and `--reconfigure` is not set, `init` updates the
//! adapters only, as in 0.1.

use crate::adapters::{self, VERSION};
use crate::config::{self, Config, DEFAULT_BUDGETS, Profile, Rules};
use crate::detect::{self, Facts, GitInfo, Row};
use crate::diag::Severity;
use crate::json::Obj;
use crate::model::DocType;
use crate::repo::{Fatal, Repo, Snapshot};
use std::io::{BufRead, IsTerminal, Write};

/// The options of `init`.
pub struct Opts {
    pub root: Option<String>,
    pub non_interactive: bool,
    pub answers: Option<String>,
    pub reconfigure: bool,
    pub dry_run: bool,
    pub json: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Covers,
    VerifiedBy,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Budgets {
    Default,
    Relaxed,
    Tight,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adapter {
    AgentsMd,
    ClaudeMd,
    Skill,
}

const ADAPTERS: [(Adapter, &str); 3] = [
    (Adapter::AgentsMd, "agents-md"),
    (Adapter::ClaudeMd, "claude-md"),
    (Adapter::Skill, "skill"),
];

/// The answers of the 8 questions, with the detected defaults.
#[derive(Clone, Debug)]
pub struct Plan {
    pub kind: &'static str,
    pub profile: Profile,
    pub types: Vec<DocType>,
    /// `None`: the profile decides. `Some(s)`: the rule `evidence`.
    pub evidence: Option<Option<Severity>>,
    pub style: Style,
    pub tests: Vec<String>,
    pub language: String,
    pub outcome: Vec<String>,
    pub amendments: Vec<String>,
    pub rows: Vec<Row>,
    /// True if the roots are the layout of a new project.
    pub layout: bool,
    pub budgets: Budgets,
    pub adapters: Vec<Adapter>,
    pub ci: String,
    pub hook: String,
    pub base: String,
    pub exclude: Vec<String>,
    /// For `--reconfigure`: the lines of keys that no question asks for, such
    /// as `install-source` or `[context]`. They stay as they are.
    pub kept: Vec<String>,
    /// For `--reconfigure`: the other rules of `[rules]`.
    pub rules: Rules,
    /// For `--reconfigure`: the `[budgets.<type>]` tables. A new budget level
    /// replaces them.
    pub kept_budgets: Vec<String>,
}

/// The roots of a new project: one mixed root. `spec-dna new` puts each type
/// into its own namespace there (`decisions/<id>`), so that the ids of
/// different types cannot clash.
fn layout_rows(root: &str) -> Vec<Row> {
    let mut main = blank_row(root, None, DocType::Spec);
    main.status = Some("draft".into());
    main.why = "new documents: specs by default; `spec-dna new <type> <id>` uses one namespace for each type".into();
    vec![main]
}

fn blank_row(path: &str, ty: Option<DocType>, default_type: DocType) -> Row {
    Row {
        path: path.to_string(),
        ty,
        default_type,
        status: None,
        legacy_ids: Vec::new(),
        legacy_keys: Vec::new(),
        status_map: Vec::new(),
        required_off: Vec::new(),
        exclude: Vec::new(),
        docs: 0,
        decisions: 0,
        extra: Vec::new(),
        why: String::new(),
    }
}

/// Lists the files of the working tree and reads the facts.
fn detect_repo(repo: &Repo) -> Result<Facts, Fatal> {
    let snap = Snapshot::worktree(repo, &[], true)?;
    let files: Vec<String> = snap.files.iter().map(|e| e.path.clone()).collect();
    let root = repo.root.clone();
    Ok(detect::detect(
        &files,
        |p| std::fs::read_to_string(root.join(p)).ok(),
        GitInfo {
            base: repo.git.default_branch(),
        },
    ))
}

/// The plan of the detected defaults.
fn defaults(facts: &Facts, root: Option<&str>) -> Plan {
    let rows: Vec<Row> = facts.rows.clone();
    let layout = rows.is_empty();
    let docs_root = root.unwrap_or("docs").trim_matches('/');
    let docs_root = if docs_root.is_empty() { "." } else { docs_root };
    // Every type by default: a list that leaves out a type makes each
    // document of that type an error, so a narrower list is the user's choice.
    let types: Vec<DocType> = DocType::ALL.to_vec();
    let mut adapters_default = vec![Adapter::Skill];
    let agents = facts.agent_files.iter().any(|f| f == "AGENTS.md");
    let claude = facts.agent_files.iter().any(|f| f == "CLAUDE.md");
    if agents || !claude {
        adapters_default.insert(0, Adapter::AgentsMd);
    }
    adapters_default.insert(adapters_default.len() - 1, Adapter::ClaudeMd);
    Plan {
        kind: if facts.brownfield {
            "brownfield"
        } else {
            "greenfield"
        },
        profile: Profile::Standard,
        types: types.clone(),
        evidence: None,
        style: Style::Both,
        tests: facts.tests.iter().map(|t| t.0.clone()).collect(),
        language: "en".into(),
        outcome: facts.outcome_aliases.clone(),
        amendments: facts.amendment_aliases.clone(),
        rows: if layout { layout_rows(docs_root) } else { rows },
        layout,
        budgets: Budgets::Default,
        adapters: adapters_default,
        ci: if facts.ci.iter().any(|c| c == "GitHub Actions") {
            "github".into()
        } else if facts.ci.iter().any(|c| c == "GitLab CI") {
            "gitlab".into()
        } else {
            "none".into()
        },
        hook: facts
            .hooks
            .iter()
            .find(|h| ["pre-commit", "husky", "lefthook"].contains(&h.as_str()))
            .cloned()
            .unwrap_or_else(|| "none".into()),
        base: if facts.base.is_empty() {
            "main".into()
        } else {
            facts.base.clone()
        },
        exclude: facts.exclude.iter().map(|e| e.0.clone()).collect(),
        kept: Vec::new(),
        rules: Vec::new(),
        kept_budgets: Vec::new(),
    }
}

/// The plan of the current configuration, for `--reconfigure`.
fn current(cfg: &Config, text: &str, facts: &Facts) -> Plan {
    let mut p = defaults(facts, None);
    p.kind = "reconfigure";
    p.profile = cfg.profile;
    p.layout = false;
    p.exclude = cfg.exclude.clone();
    p.evidence = cfg.rules.iter().find(|r| r.0 == "evidence").map(|r| r.1);
    p.rules = cfg
        .rules
        .iter()
        .filter(|r| !["evidence", "budgets", "lint"].contains(&r.0.as_str()))
        .cloned()
        .collect();
    if cfg.rules.iter().any(|r| r.0 == "budgets" && r.1.is_none()) {
        p.budgets = Budgets::Off;
    }
    if cfg.evidence_tests.is_empty() {
        p.style = Style::VerifiedBy;
    }
    if cfg.evidence_tests != config::DEFAULT_TEST_GLOBS.map(str::to_string) {
        p.tests = cfg.evidence_tests.clone();
    }
    p.rows = cfg.roots.iter().map(row_of_root).collect();
    p.types = if cfg.types.is_empty() {
        DocType::ALL.to_vec()
    } else {
        cfg.types.clone()
    };
    // The outcome and amendment aliases are in the rows already.
    p.outcome = Vec::new();
    p.amendments = Vec::new();
    // Keep the keys that no question asks for, as written.
    let table: toml::Table = text.parse().unwrap_or_default();
    for key in ["install-source", "self-urls"] {
        if let Some(v) = table.get(key) {
            p.kept.push(format!("{key} = {}", toml_value(v)));
        }
    }
    for key in ["lint", "context", "repos"] {
        if let Some(toml::Value::Table(t)) = table.get(key) {
            p.kept.push(format!("\n[{key}]"));
            for (k, v) in t {
                p.kept.push(format!("{} = {}", toml_key(k), toml_value(v)));
            }
        }
    }
    if let Some(toml::Value::Table(t)) = table.get("budgets") {
        for (ty, v) in t {
            if let toml::Value::Table(b) = v {
                p.kept_budgets.push(format!("\n[budgets.{ty}]"));
                for k in ["soft", "hard"] {
                    if let Some(v) = b.get(k) {
                        p.kept_budgets.push(format!("{k} = {}", toml_value(v)));
                    }
                }
            }
        }
    }
    p
}

/// A row of a configured root, for `--reconfigure` and for the roots of an
/// answers file.
fn row_of_root(r: &config::Root) -> Row {
    let mut row = blank_row(
        if r.path.is_empty() { "." } else { &r.path },
        r.single_type,
        r.default_type,
    );
    row.status = r.status.clone();
    row.exclude = r.exclude.clone();
    row.legacy_ids = r
        .legacy_ids
        .iter()
        .map(|l| {
            if l.file.source == format!("{}*", l.alias) {
                l.alias.clone()
            } else {
                format!("{}|{}", l.alias, l.file.source)
            }
        })
        .collect();
    row.legacy_keys = r.legacy_keys.iter().map(|k| k.source.clone()).collect();
    row.status_map = r.status_map.clone();
    // The section keys that differ from the defaults stay as written.
    let defaults = config::default_required_sections();
    let mut required = Vec::new();
    let mut amendments = Vec::new();
    for t in row.types() {
        let groups = &r.required_sections[t as usize];
        if *groups != defaults[t as usize] {
            let g: Vec<String> = groups.iter().map(|g| toml_list(g)).collect();
            required.push(format!("{} = [{}]", t.name(), g.join(", ")));
        }
        let names = &r.amendment_sections[t as usize];
        let default: Vec<String> = if t == DocType::Decision {
            config::DEFAULT_AMENDMENT_SECTIONS
                .map(str::to_string)
                .to_vec()
        } else {
            Vec::new()
        };
        if *names != default {
            amendments.push(format!("{} = {}", t.name(), toml_list(names)));
        }
    }
    if !required.is_empty() {
        row.extra
            .push(format!("required-sections = {{ {} }}", required.join(", ")));
    }
    if !amendments.is_empty() {
        row.extra.push(format!(
            "amendment-sections = {{ {} }}",
            amendments.join(", ")
        ));
    }
    if !r.enforce_budgets {
        row.extra.push("enforce-budgets = false".into());
    }
    if let Some(p) = r.profile {
        row.extra.push(format!("profile = {}", toml_str(p.name())));
    }
    if !r.rules.is_empty() {
        let v: Vec<String> = r
            .rules
            .iter()
            .map(|(k, s)| format!("{} = {}", toml_key(k), toml_str(severity_name(*s))))
            .collect();
        row.extra.push(format!("rules = {{ {} }}", v.join(", ")));
    }
    row.why = String::new();
    row
}

fn toml_key(k: &str) -> String {
    if !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        k.to_string()
    } else {
        toml_str(k)
    }
}

/// A TOML basic string.
fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_list(items: &[String]) -> String {
    let v: Vec<String> = items.iter().map(|s| toml_str(s)).collect();
    format!("[{}]", v.join(", "))
}

fn toml_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => toml_str(s),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(a) => {
            let v: Vec<String> = a.iter().map(toml_value).collect();
            format!("[{}]", v.join(", "))
        }
        toml::Value::Table(t) => {
            let v: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{} = {}", toml_key(k), toml_value(v)))
                .collect();
            format!("{{ {} }}", v.join(", "))
        }
    }
}

fn severity_name(s: Option<Severity>) -> &'static str {
    match s {
        None => "off",
        Some(s) => s.name(),
    }
}

/// Renders the plan as `spec-dna.toml`.
pub fn render(plan: &Plan) -> String {
    let mut out = String::from(
        "# spec-dna configuration (docs/config.md). `spec-dna init` wrote it; edit it freely.\n",
    );
    out.push_str(&format!("spec-dna = {}\n", toml_str(VERSION)));
    out.push_str(&format!(
        "profile = {}  # lite, standard or strict (docs/profiles.md)\n",
        toml_str(plan.profile.name())
    ));
    if plan.types.len() < DocType::COUNT {
        out.push_str(&format!(
            "types = {}  # the document types of this project\n",
            toml_list(&type_names(&plan.types))
        ));
    }
    if !plan.exclude.is_empty() {
        out.push_str(&format!("exclude = {}\n", toml_list(&plan.exclude)));
    }
    if plan.kept.iter().all(|k| !k.starts_with("install-source")) {
        out.push_str(&format!(
            "# install-source = {}  # a local or private clone\n",
            toml_str(config::DEFAULT_INSTALL_SOURCE)
        ));
    }
    let mut kept_top: Vec<&String> = Vec::new();
    let mut kept_tables: Vec<&String> = Vec::new();
    let mut in_table = false;
    for k in &plan.kept {
        in_table |= k.starts_with('\n');
        if in_table {
            kept_tables.push(k);
        } else {
            kept_top.push(k);
        }
    }
    for k in kept_top {
        out.push_str(&format!("{k}\n"));
    }
    // Rules: the answers that differ from the profile.
    let mut rules: Vec<(String, &str)> = plan
        .rules
        .iter()
        .map(|(k, s)| (k.clone(), severity_name(*s)))
        .collect();
    if let Some(e) = plan.evidence {
        rules.push(("evidence".into(), severity_name(e)));
    }
    if plan.budgets == Budgets::Off {
        rules.push(("budgets".into(), "off"));
    }
    if plan.language != "en" {
        rules.push(("lint".into(), "off"));
    }
    if !rules.is_empty() {
        out.push_str("\n[rules]  # overrides of the profile, by rule group or by code\n");
        for (k, v) in rules {
            out.push_str(&format!("{} = {}\n", toml_key(&k), toml_str(v)));
        }
    }
    match plan.style {
        Style::VerifiedBy => {
            out.push_str("\n[evidence]\ntests = []  # evidence by `verified-by` only\n")
        }
        _ if !plan.tests.is_empty() => out.push_str(&format!(
            "\n[evidence]\ntests = {}  # test files with `covers: <doc-id>#<key>` tags\n",
            toml_list(&plan.tests)
        )),
        _ => {}
    }
    let factor = match plan.budgets {
        Budgets::Relaxed => Some(2.0),
        Budgets::Tight => Some(0.5),
        _ => None,
    };
    if let Some(f) = factor {
        for t in &plan.types {
            let (soft, hard) = DEFAULT_BUDGETS[*t as usize];
            if hard > 0 {
                out.push_str(&format!(
                    "\n[budgets.{}]\nsoft = {}\nhard = {}\n",
                    t.name(),
                    (f64::from(soft) * f) as u32,
                    (f64::from(hard) * f) as u32
                ));
            }
        }
    }
    if plan.budgets == Budgets::Default {
        for k in &plan.kept_budgets {
            out.push_str(&format!("{k}\n"));
        }
    }
    for k in kept_tables {
        out.push_str(&format!("{k}\n"));
    }
    for r in &plan.rows {
        out.push('\n');
        if !r.why.is_empty() {
            out.push_str(&format!("# {}\n", r.why));
        }
        out.push_str(&render_row(plan, r));
    }
    out
}

fn render_row(plan: &Plan, r: &Row) -> String {
    let mut out = format!("[[roots]]\npath = {}\n", toml_str(&r.path));
    match r.ty {
        Some(t) => out.push_str(&format!("type = {}\n", toml_str(t.name()))),
        None => {
            out.push_str("type = \"mixed\"\n");
            out.push_str(&format!(
                "default-type = {}\n",
                toml_str(r.default_type.name())
            ));
        }
    }
    if let Some(s) = &r.status {
        out.push_str(&format!("status = {}\n", toml_str(s)));
    }
    if !r.exclude.is_empty() {
        out.push_str(&format!("exclude = {}\n", toml_list(&r.exclude)));
    }
    if !r.legacy_ids.is_empty() {
        let v: Vec<String> = r
            .legacy_ids
            .iter()
            .map(|l| match l.split_once('|') {
                Some((a, f)) => format!("{{ alias = {}, file = {} }}", toml_str(a), toml_str(f)),
                None => toml_str(l),
            })
            .collect();
        out.push_str(&format!("legacy-ids = [{}]\n", v.join(", ")));
    }
    if !r.legacy_keys.is_empty() {
        out.push_str(&format!("legacy-keys = {}\n", toml_list(&r.legacy_keys)));
    }
    if !r.status_map.is_empty() {
        let v: Vec<String> = r
            .status_map
            .iter()
            .map(|(a, b)| format!("{} = {}", toml_key(a), toml_str(b)))
            .collect();
        out.push_str(&format!("status-map = {{ {} }}\n", v.join(", ")));
    }
    // The headings of other languages go to the roots that hold decisions, or
    // that can hold new ones (a new project).
    let decisions = r.ty == Some(DocType::Decision)
        || r.default_type == DocType::Decision
        || (r.ty.is_none() && (r.decisions > 0 || plan.layout));
    let mut required: Vec<String> = Vec::new();
    if decisions && !plan.outcome.is_empty() {
        let mut h = plan.outcome.clone();
        h.extend(config::DEFAULT_DECISION_SECTIONS.map(str::to_string));
        required.push(format!("decision = {}", toml_list(&h)));
    }
    for t in &r.required_off {
        if *t != DocType::Decision || plan.outcome.is_empty() {
            required.push(format!("{} = []", t.name()));
        }
    }
    let has = |k: &str| r.extra.iter().any(|e| e.starts_with(k));
    if !required.is_empty() && !has("required-sections") {
        out.push_str(&format!(
            "required-sections = {{ {} }}\n",
            required.join(", ")
        ));
    }
    if decisions && !plan.amendments.is_empty() && !has("amendment-sections") {
        let mut h = plan.amendments.clone();
        h.extend(config::DEFAULT_AMENDMENT_SECTIONS.map(str::to_string));
        out.push_str(&format!(
            "amendment-sections = {{ decision = {} }}\n",
            toml_list(&h)
        ));
    }
    for e in &r.extra {
        out.push_str(&format!("{e}\n"));
    }
    out
}

/// One file that `init` creates or changes.
pub struct Change {
    pub path: String,
    pub created: bool,
    pub text: String,
    pub note: String,
}

/// The adapter files for a set of adapters.
fn adapter_changes(repo: &Repo, chosen: &[Adapter]) -> Result<Vec<Change>, Fatal> {
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).unwrap_or_default();
    let exists = |p: &str| repo.root.join(p).exists();
    let mut out = Vec::new();
    // An older version wrote a `claude` block that imported AGENTS.md. If
    // AGENTS.md has text of its own, the import stays as a plain line.
    let agents_target = chosen.contains(&Adapter::AgentsMd);
    let claude = adapters::migrate_claude(
        &read(adapters::CLAUDE),
        adapters::has_own_text(&read(adapters::AGENTS)),
    );
    let imports = adapters::imports_agents(&claude);
    // A file that the agent reads through an import keeps no second copy.
    let claude_target = chosen.contains(&Adapter::ClaudeMd) && !(agents_target && imports);
    for (path, target) in [
        (adapters::AGENTS, agents_target),
        (adapters::CLAUDE, claude_target),
    ] {
        let old = read(path);
        let current = if path == adapters::CLAUDE {
            claude.clone()
        } else {
            old.clone()
        };
        let new = if target {
            adapters::upsert_line(&current)
        } else {
            adapters::remove_line(&current)
        };
        if new != old && !(new.is_empty() && !exists(path)) {
            out.push(Change {
                path: path.to_string(),
                created: !exists(path),
                text: new,
                note: if target {
                    "one generated line".into()
                } else {
                    "remove the generated line".into()
                },
            });
        }
    }
    let skill = read(adapters::SKILL);
    if chosen.contains(&Adapter::Skill) {
        if !skill.is_empty() && !skill.contains("<!-- spec-dna:begin skill ") {
            return Err(Fatal::Usage(format!(
                "{} exists and was not generated by spec-dna; move it first",
                adapters::SKILL
            )));
        }
        if skill != adapters::skill_file() {
            out.push(Change {
                path: adapters::SKILL.into(),
                created: skill.is_empty(),
                text: adapters::skill_file(),
                note: "the skill".into(),
            });
        }
    }
    Ok(out)
}

/// Joins lines that keep their YAML indentation (a source file of this
/// repository indents by 4 spaces, so the YAML lines are strings here).
fn lines(items: &[&str]) -> String {
    items.iter().map(|l| format!("{l}\n")).collect()
}

/// The GitHub Actions job, as in docs/quickstart.md.
fn github_workflow(base: &str) -> String {
    let install = format!(
        "          cargo install --locked --git \"${{src:-{}}}\" --tag \"v$pin\" spec-dna",
        config::DEFAULT_INSTALL_SOURCE
    );
    let check =
        format!("      - run: spec-dna check --base \"origin/${{GITHUB_BASE_REF:-{base}}}\"");
    lines(&[
        "name: Specs",
        "on: pull_request",
        "permissions: {}",
        "jobs:",
        "  spec-dna:",
        "    runs-on: ubuntu-24.04",
        "    permissions:",
        "      contents: read",
        "    steps:",
        "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
        "        with:",
        "          fetch-depth: 0",
        "          persist-credentials: false",
        "      - name: Install the pinned spec-dna",
        "        run: |",
        "          pin=$(sed -n 's/^spec-dna = \"\\(.*\\)\"/\\1/p' spec-dna.toml)",
        "          src=$(sed -n 's/^install-source = \"\\(.*\\)\"/\\1/p' spec-dna.toml)",
        &install,
        &check,
    ])
}

/// The local hook of pre-commit.
fn pre_commit_hook() -> String {
    lines(&[
        "- repo: local",
        "  hooks:",
        "    - id: spec-dna",
        "      name: spec-dna check",
        "      entry: spec-dna check",
        "      language: system",
        "      pass_filenames: false",
        "      always_run: true",
    ])
}

/// Appends the local hook to `.pre-commit-config.yaml`, if `repos:` is its
/// last top-level key. Otherwise returns `None`, and the preview gives the
/// text to add by hand.
fn pre_commit_append(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let repos = lines.iter().rposition(|l| l.trim_end() == "repos:")?;
    let after_top = lines[repos + 1..]
        .iter()
        .any(|l| !l.is_empty() && !l.starts_with([' ', '\t', '-', '#']));
    if after_top {
        return None;
    }
    let indent = lines[repos + 1..]
        .iter()
        .find_map(|l| {
            let t = l.trim_start();
            t.starts_with("- repo:").then(|| l.len() - t.len())
        })
        .unwrap_or(2);
    let pad = " ".repeat(indent);
    let block: String = pre_commit_hook()
        .lines()
        .map(|l| format!("{pad}{l}\n"))
        .collect();
    let mut t = text.to_string();
    if !t.ends_with('\n') {
        t.push('\n');
    }
    Some(format!("{t}{block}"))
}

/// The CI and hook files, and the text to add by hand.
fn ci_changes(repo: &Repo, plan: &Plan) -> (Vec<Change>, Vec<String>) {
    let mut out = Vec::new();
    let mut manual = Vec::new();
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).ok();
    match plan.ci.as_str() {
        "github" => {
            let path = ".github/workflows/spec-dna.yml";
            let text = github_workflow(&plan.base);
            if read(path).as_deref() != Some(text.as_str()) {
                out.push(Change {
                    path: path.into(),
                    created: read(path).is_none(),
                    text,
                    note: "the CI job: `spec-dna check --base`".into(),
                });
            }
        }
        "gitlab" => manual.push(format!(
            "Add this job to .gitlab-ci.yml:\nspec-dna:\n  stage: test\n  script:\n    - cargo install --locked --git {} --tag v{VERSION} spec-dna\n    - git fetch origin \"${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n    - spec-dna check --base \"origin/${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n  rules:\n    - if: $CI_PIPELINE_SOURCE == \"merge_request_event\"",
            config::DEFAULT_INSTALL_SOURCE,
            plan.base,
            plan.base
        )),
        _ => {}
    }
    match plan.hook.as_str() {
        "pre-commit" => {
            let path = ".pre-commit-config.yaml";
            let old = read(path).unwrap_or_default();
            if old.contains("entry: spec-dna check") {
            } else if old.is_empty() {
                out.push(Change {
                    path: path.into(),
                    created: true,
                    text: format!("repos:\n{}", indent2(&pre_commit_hook())),
                    note: "a local hook: `spec-dna check`".into(),
                });
            } else if let Some(text) = pre_commit_append(&old) {
                out.push(Change {
                    path: path.into(),
                    created: false,
                    text,
                    note: "a local hook at the end: `spec-dna check`".into(),
                });
            } else {
                manual.push(format!(
                    "Add this hook under `repos:` in .pre-commit-config.yaml:\n{}",
                    indent2(&pre_commit_hook()).trim_end()
                ));
            }
        }
        "husky" => {
            let path = ".husky/pre-commit";
            let old = read(path).unwrap_or_default();
            if !old.lines().any(|l| l.trim() == "spec-dna check") {
                let mut t = old.clone();
                if !t.is_empty() && !t.ends_with('\n') {
                    t.push('\n');
                }
                t.push_str("spec-dna check\n");
                out.push(Change {
                    path: path.into(),
                    created: old.is_empty(),
                    text: t,
                    note: "one line: `spec-dna check`".into(),
                });
            }
        }
        "lefthook" => manual.push(
            "Add this command to lefthook.yml:\npre-commit:\n  commands:\n    spec-dna:\n      run: spec-dna check".into(),
        ),
        _ => {}
    }
    (out, manual)
}

fn indent2(text: &str) -> String {
    text.lines().map(|l| format!("  {l}\n")).collect()
}

/// Reads one answer from the terminal. An empty line or the end of the input
/// keeps the default.
struct Ask<'a> {
    input: &'a mut dyn BufRead,
    out: &'a mut dyn Write,
}

impl Ask<'_> {
    fn line(&mut self, prompt: &str, default: &str) -> String {
        let _ = write!(self.out, "{prompt} [{default}]: ");
        let _ = self.out.flush();
        let mut s = String::new();
        match self.input.read_line(&mut s) {
            Ok(n) if n > 0 && !s.trim().is_empty() => s.trim().to_string(),
            _ => default.to_string(),
        }
    }

    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }
}

fn list(items: &[String]) -> String {
    items.join(", ")
}

fn words(s: &str) -> Vec<String> {
    s.split([',', ' '])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn type_names(types: &[DocType]) -> Vec<String> {
    types.iter().map(|t| t.name().to_string()).collect()
}

fn parse_types(v: &[String]) -> Result<Vec<DocType>, String> {
    let mut out = Vec::new();
    for w in v {
        let t = DocType::from_name(w)
            .ok_or_else(|| format!("unknown type `{w}`; use {}", DocType::names()))?;
        if !out.contains(&t) {
            out.push(t);
        }
    }
    Ok(out)
}

fn parse_evidence(w: &str) -> Result<Option<Option<Severity>>, String> {
    Ok(match w {
        "profile" => None,
        "off" => Some(None),
        "warning" => Some(Some(Severity::Warning)),
        "error" => Some(Some(Severity::Error)),
        _ => {
            return Err(format!(
                "evidence `{w}`: use profile, off, warning or error"
            ));
        }
    })
}

fn parse_style(w: &str) -> Result<Style, String> {
    Ok(match w {
        "covers" => Style::Covers,
        "verified-by" => Style::VerifiedBy,
        "both" => Style::Both,
        _ => return Err(format!("link style `{w}`: use covers, verified-by or both")),
    })
}

fn parse_budgets(w: &str) -> Result<Budgets, String> {
    Ok(match w {
        "default" => Budgets::Default,
        "relaxed" => Budgets::Relaxed,
        "tight" => Budgets::Tight,
        "off" => Budgets::Off,
        _ => {
            return Err(format!(
                "budget level `{w}`: use default, relaxed, tight or off"
            ));
        }
    })
}

fn parse_adapters(v: &[String]) -> Result<Vec<Adapter>, String> {
    let mut out = Vec::new();
    for w in v {
        if w == "none" {
            continue;
        }
        let a = ADAPTERS
            .iter()
            .find(|x| x.1 == w)
            .ok_or_else(|| format!("adapter `{w}`: use agents-md, claude-md, skill or none"))?;
        out.push(a.0);
    }
    Ok(out)
}

fn adapter_names(v: &[Adapter]) -> Vec<String> {
    if v.is_empty() {
        return vec!["none".into()];
    }
    v.iter()
        .map(|a| ADAPTERS.iter().find(|x| x.0 == *a).unwrap().1.to_string())
        .collect()
}

fn check_choice(what: &str, w: &str, allowed: &[&str]) -> Result<String, String> {
    if allowed.contains(&w) {
        Ok(w.to_string())
    } else {
        Err(format!("{what} `{w}`: use {}", allowed.join(", ")))
    }
}

/// The types of a plan change the roots of a new project, and the type of a
/// detected root whose type is not chosen (it becomes a note).
fn apply_types(plan: &mut Plan, root: &str) {
    if plan.layout {
        plan.rows = layout_rows(root);
        return;
    }
    for r in &mut plan.rows {
        let t = r.ty.unwrap_or(r.default_type);
        if !plan.types.contains(&t) && t != DocType::Note {
            r.why = format!("{} (type `{}` not chosen: note)", r.why, t.name());
            if r.ty.is_some() {
                r.ty = Some(DocType::Note);
            }
            r.default_type = DocType::Note;
            r.status = None;
            r.status_map.clear();
            r.required_off.clear();
            r.legacy_keys.clear();
        }
    }
}

/// Edits a row with `key=value` words: `path`, `type` (a type or `mixed`),
/// `default-type`, `status`, `exclude`, `legacy-ids` and `legacy-keys` (lists
/// with commas).
fn edit_row(r: &mut Row, edits: &str) -> Result<(), String> {
    for w in edits.split_whitespace() {
        let (k, v) = w
            .split_once('=')
            .ok_or_else(|| format!("`{w}`: write key=value"))?;
        let items = || -> Vec<String> {
            v.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        };
        match k {
            "path" => r.path = v.trim_matches('/').to_string(),
            "type" if v == "mixed" => r.ty = None,
            "type" => {
                let t = DocType::from_name(v).ok_or_else(|| format!("unknown type `{v}`"))?;
                r.ty = Some(t);
                r.default_type = t;
            }
            "default-type" => {
                r.default_type =
                    DocType::from_name(v).ok_or_else(|| format!("unknown type `{v}`"))?
            }
            "status" => r.status = (!v.is_empty() && v != "none").then(|| v.to_string()),
            "exclude" => r.exclude = items(),
            "legacy-ids" => r.legacy_ids = items(),
            "legacy-keys" => r.legacy_keys = items(),
            _ => return Err(format!("`{k}` is not a key of a row")),
        }
    }
    Ok(())
}

fn row_line(r: &Row) -> String {
    let ty = match r.ty {
        Some(t) => t.name().to_string(),
        None => format!("mixed, default {}", r.default_type.name()),
    };
    let mut extra = Vec::new();
    if let Some(s) = &r.status {
        extra.push(format!("status {s}"));
    }
    if !r.legacy_ids.is_empty() {
        extra.push(format!("ids {}", list(&r.legacy_ids)));
    }
    if !r.legacy_keys.is_empty() {
        extra.push(format!("keys {}", list(&r.legacy_keys)));
    }
    if !r.status_map.is_empty() {
        let m: Vec<String> = r
            .status_map
            .iter()
            .map(|(a, b)| format!("{a}->{b}"))
            .collect();
        extra.push(format!("map {}", m.join(" ")));
    }
    if !r.exclude.is_empty() {
        extra.push(format!("exclude {}", list(&r.exclude)));
    }
    format!(
        "{} ({ty}; {} documents{}{})",
        r.path,
        r.docs,
        if extra.is_empty() { "" } else { "; " },
        extra.join("; ")
    )
}

/// Asks the 8 questions on a terminal.
fn ask(plan: &mut Plan, facts: &Facts, root: &str, io: &mut Ask) -> Result<(), String> {
    io.say(&format!(
        "spec-dna init: {} repository; {} files, {} Markdown documents. Press Enter to keep a default.",
        plan.kind, facts.files, facts.markdown
    ));
    io.say("lite: warnings only. standard: errors for broken references and budget growth. strict: drift, evidence and budgets fail, lint warns.");
    let p = io.line("1/8 Profile (lite, standard, strict)", plan.profile.name());
    plan.profile =
        Profile::from_name(&p).ok_or(format!("profile `{p}`: use lite, standard or strict"))?;

    let found: Vec<String> = type_names(&facts.types);
    io.say(&format!(
        "    Types in use: {}. A shorter list makes each document of another type an error.",
        if found.is_empty() {
            "none yet".to_string()
        } else {
            list(&found)
        }
    ));
    let t = io.line("2/8 Knowledge types", &list(&type_names(&plan.types)));
    plan.types = parse_types(&words(&t))?;
    apply_types(plan, root);

    let found: Vec<String> = facts
        .tests
        .iter()
        .map(|(g, n)| format!("{g} ({n})"))
        .collect();
    io.say(&format!(
        "    Test files: {}.",
        if found.is_empty() {
            "none yet".to_string()
        } else {
            list(&found)
        }
    ));
    let ev = match plan.evidence {
        None => "profile",
        Some(s) => severity_name(s),
    };
    let st = match plan.style {
        Style::Covers => "covers",
        Style::VerifiedBy => "verified-by",
        Style::Both => "both",
    };
    let a = io.line(
        "3/8 Evidence policy (profile, off, warning, error) and link style (covers, verified-by, both)",
        &format!("{ev}, {st}"),
    );
    let w = words(&a);
    plan.evidence = parse_evidence(w.first().map_or(ev, |s| s.as_str()))?;
    plan.style = parse_style(w.get(1).map_or(st, |s| s.as_str()))?;

    let scripts: Vec<String> = facts
        .scripts
        .iter()
        .take(3)
        .map(|(s, n)| format!("{s} {n}"))
        .collect();
    io.say(&format!(
        "    Heading scripts: {}. Requirement lint works for English only.",
        if scripts.is_empty() {
            "none".into()
        } else {
            list(&scripts)
        }
    ));
    let default = format!(
        "{}; outcome: {}; amendments: {}",
        plan.language,
        if plan.outcome.is_empty() {
            "-".into()
        } else {
            list(&plan.outcome)
        },
        if plan.amendments.is_empty() {
            "-".into()
        } else {
            list(&plan.amendments)
        }
    );
    let a = io.line("4/8 Normative language and heading aliases", &default);
    let mut parts = a.split(';');
    plan.language = parts.next().unwrap_or("en").trim().to_string();
    for part in parts {
        let (k, v) = part.split_once(':').unwrap_or((part, ""));
        let v: Vec<String> = v
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "-")
            .map(str::to_string)
            .collect();
        match k.trim() {
            "outcome" => plan.outcome = v,
            "amendments" => plan.amendments = v,
            other => return Err(format!("`{other}`: use outcome: and amendments:")),
        }
    }

    io.say(&format!(
        "5/8 Mapping: {} roots. For each: Enter accepts, `drop` removes, `key=value ...` edits (path, type, default-type, status, exclude, legacy-ids, legacy-keys).",
        plan.rows.len()
    ));
    let mut rows = Vec::new();
    for mut r in std::mem::take(&mut plan.rows) {
        if !r.why.is_empty() {
            io.say(&format!("    # {}", r.why));
        }
        loop {
            let a = io.line(&format!("    {}", row_line(&r)), "accept");
            match a.as_str() {
                "accept" => {
                    rows.push(r);
                    break;
                }
                "drop" => break,
                edits => match edit_row(&mut r, edits) {
                    Ok(()) => continue,
                    Err(e) => io.say(&format!("    {e}")),
                },
            }
        }
    }
    plan.rows = rows;

    io.say(&format!(
        "    Documents over the hard budget: default {}, relaxed (x2) {}, tight (x0.5) {}.",
        detect::over_budget(facts, 1.0),
        detect::over_budget(facts, 2.0),
        detect::over_budget(facts, 0.5)
    ));
    let b = io.line(
        "6/8 Budget level (default, relaxed, tight, off)",
        match plan.budgets {
            Budgets::Default => "default",
            Budgets::Relaxed => "relaxed",
            Budgets::Tight => "tight",
            Budgets::Off => "off",
        },
    );
    plan.budgets = parse_budgets(&b)?;

    io.say(&format!(
        "    Agent files: {}.",
        if facts.agent_files.is_empty() {
            "none".into()
        } else {
            list(&facts.agent_files)
        }
    ));
    let a = io.line(
        "7/8 Agent adapters (agents-md, claude-md, skill, none)",
        &list(&adapter_names(&plan.adapters)),
    );
    plan.adapters = parse_adapters(&words(&a))?;

    io.say(&format!(
        "    CI: {}. Hooks: {}.",
        if facts.ci.is_empty() {
            "none".into()
        } else {
            list(&facts.ci)
        },
        if facts.hooks.is_empty() {
            "none".into()
        } else {
            list(&facts.hooks)
        }
    ));
    let a = io.line(
        "8/8 CI step (github, gitlab, none), hook (pre-commit, husky, lefthook, none) and base branch",
        &format!("{}, {}, {}", plan.ci, plan.hook, plan.base),
    );
    let w = words(&a);
    if let Some(c) = w.first() {
        plan.ci = check_choice("CI step", c, &["github", "gitlab", "none"])?;
    }
    if let Some(h) = w.get(1) {
        plan.hook = check_choice("hook", h, &["pre-commit", "husky", "lefthook", "none"])?;
    }
    if let Some(b) = w.get(2) {
        plan.base = b.clone();
    }
    Ok(())
}

/// Applies an answers file (TOML) to the plan. A key that is not in the file
/// keeps its default.
fn apply_answers(plan: &mut Plan, text: &str, root: &str) -> Result<bool, String> {
    let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let s = |k: &str| -> Result<Option<String>, String> {
        match t.get(k) {
            None => Ok(None),
            Some(toml::Value::String(v)) => Ok(Some(v.clone())),
            Some(_) => Err(format!("answers: `{k}` must be a string")),
        }
    };
    let l = |k: &str| -> Result<Option<Vec<String>>, String> {
        match t.get(k) {
            None => Ok(None),
            Some(toml::Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .map(Some)
                .ok_or_else(|| format!("answers: `{k}` must be an array of strings")),
            Some(_) => Err(format!("answers: `{k}` must be an array of strings")),
        }
    };
    const KEYS: [&str; 16] = [
        "profile",
        "types",
        "evidence",
        "evidence-style",
        "language",
        "outcome-headings",
        "amendment-headings",
        "mapping",
        "roots",
        "drop",
        "budgets",
        "adapters",
        "ci",
        "hook",
        "base",
        "write",
    ];
    if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!(
            "answers: unknown key `{k}`; use {}",
            KEYS.join(", ")
        ));
    }
    if let Some(p) = s("profile")? {
        plan.profile = Profile::from_name(&p).ok_or(format!("answers: profile `{p}`"))?;
    }
    if let Some(v) = l("types")? {
        plan.types = parse_types(&v)?;
        apply_types(plan, root);
    }
    if let Some(e) = s("evidence")? {
        plan.evidence = parse_evidence(&e)?;
    }
    if let Some(e) = s("evidence-style")? {
        plan.style = parse_style(&e)?;
    }
    if let Some(v) = s("language")? {
        plan.language = v;
    }
    if let Some(v) = l("outcome-headings")? {
        plan.outcome = v;
    }
    if let Some(v) = l("amendment-headings")? {
        plan.amendments = v;
    }
    if let Some(m) = s("mapping")?
        && m != "accept"
    {
        return Err(format!(
            "answers: mapping `{m}`: use \"accept\", or give `[[roots]]` tables"
        ));
    }
    if let Some(roots) = t.get("roots") {
        // The roots replace the detected mapping. They use the keys of
        // spec-dna.toml.
        let mut tbl = toml::Table::new();
        tbl.insert("spec-dna".into(), toml::Value::String(VERSION.into()));
        tbl.insert("roots".into(), roots.clone());
        let src = format!(
            "spec-dna = {}\n{}",
            toml_str(VERSION),
            roots_text(roots).ok_or("answers: `roots` must be an array of tables")?
        );
        let cfg = Config::parse(&src).map_err(|e| format!("answers: roots: {e}"))?;
        plan.rows = cfg.roots.iter().map(row_of_root).collect();
        for r in &mut plan.rows {
            r.why = "from the answers file".into();
        }
        plan.layout = false;
    }
    if let Some(v) = l("drop")? {
        plan.rows.retain(|r| !v.contains(&r.path));
    }
    if let Some(b) = s("budgets")? {
        plan.budgets = parse_budgets(&b)?;
    }
    if let Some(v) = l("adapters")? {
        plan.adapters = parse_adapters(&v)?;
    }
    if let Some(c) = s("ci")? {
        plan.ci = check_choice("CI step", &c, &["github", "gitlab", "none"])?;
    }
    if let Some(h) = s("hook")? {
        plan.hook = check_choice("hook", &h, &["pre-commit", "husky", "lefthook", "none"])?;
    }
    if let Some(b) = s("base")? {
        plan.base = b;
    }
    match t.get("write") {
        None => Ok(true),
        Some(toml::Value::Boolean(b)) => Ok(*b),
        Some(_) => Err("answers: `write` must be true or false".into()),
    }
}

/// The `[[roots]]` tables of an answers file as TOML text.
fn roots_text(v: &toml::Value) -> Option<String> {
    let mut out = String::new();
    for item in v.as_array()? {
        out.push_str("\n[[roots]]\n");
        for (k, v) in item.as_table()? {
            out.push_str(&format!("{} = {}\n", toml_key(k), toml_value(v)));
        }
    }
    Some(out)
}

/// Runs `init`.
pub fn run(repo: &Repo, o: &Opts) -> Result<u8, Fatal> {
    let cfg_path = repo.config_path();
    let exists = cfg_path.exists();
    if exists {
        let cfg = repo.load_config()?;
        if cfg.pin != VERSION {
            return Err(crate::pin_mismatch(&cfg));
        }
        if !o.reconfigure {
            return adapters_only(repo, o);
        }
    } else if o.reconfigure {
        return Err(Fatal::Usage(
            "--reconfigure needs a spec-dna.toml; run `spec-dna init` first".into(),
        ));
    }
    let facts = detect_repo(repo)?;
    let root = o.root.as_deref().unwrap_or("docs").to_string();
    let mut plan = if exists {
        let text = std::fs::read_to_string(&cfg_path).map_err(|e| Fatal::Io(e.to_string()))?;
        current(&repo.load_config()?, &text, &facts)
    } else {
        defaults(&facts, Some(&root))
    };
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let interactive = !o.non_interactive && o.answers.is_none() && terminal;
    let mut write = true;
    if let Some(path) = &o.answers {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Fatal::Usage(format!("--answers {path}: {e}")))?;
        write = apply_answers(&mut plan, &text, &root).map_err(Fatal::Usage)?;
    } else if interactive {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut out = std::io::stdout();
        let mut io = Ask {
            input: &mut input,
            out: &mut out,
        };
        ask(&mut plan, &facts, &root, &mut io).map_err(Fatal::Usage)?;
    }
    let text = render(&plan);
    // spec-dna never writes a configuration that it cannot read.
    Config::parse(&text)
        .map_err(|e| Fatal::Config(format!("the new spec-dna.toml is not valid: {e}")))?;
    let mut changes = Vec::new();
    let old = std::fs::read_to_string(&cfg_path).ok();
    if old.as_deref() != Some(text.as_str()) {
        changes.push(Change {
            path: config::FILE_NAME.into(),
            created: old.is_none(),
            text: text.clone(),
            note: if old.is_some() {
                "replaced; comments of the old file are lost".into()
            } else {
                "the configuration".into()
            },
        });
    }
    changes.extend(adapter_changes(repo, &plan.adapters)?);
    let (ci, manual) = ci_changes(repo, &plan);
    changes.extend(ci);

    let preview = o.dry_run || interactive;
    if o.json {
        let files = changes.iter().map(|c| {
            Obj::new()
                .str("path", &c.path)
                .str("action", if c.created { "create" } else { "change" })
                .str("note", &c.note)
                .end()
        });
        let rows = plan.rows.iter().map(|r| {
            Obj::new()
                .str("path", &r.path)
                .str("type", r.ty.map_or("mixed", DocType::name))
                .str("default_type", r.default_type.name())
                .num("documents", r.docs)
                .str("why", &r.why)
                .end()
        });
        let detected = Obj::new()
            .str("kind", plan.kind)
            .num("files", facts.files)
            .num("markdown", facts.markdown)
            .raw("roots", &crate::json::array(rows))
            .strs("stack", &facts.stack)
            .strs("apps", &facts.apps)
            .strs("agent_files", &facts.agent_files)
            .strs("ci", &facts.ci)
            .strs("hooks", &facts.hooks)
            .str("base", &facts.base)
            .num("over_budget", detect::over_budget(&facts, 1.0))
            .end();
        let written: Vec<String> = if o.dry_run || !write {
            Vec::new()
        } else {
            apply(repo, &changes)?
        };
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .bool("dry_run", o.dry_run || !write)
                .raw("detected", &detected)
                .str("config", &text)
                .raw("files", &crate::json::array(files))
                .strs("manual", &manual)
                .strs("written", &written)
                .end()
        );
        return Ok(0);
    }
    if preview {
        println!("\n--- spec-dna.toml ---\n{}---", text);
        println!("Files to create or change:");
        if changes.is_empty() {
            println!("  none");
        }
        for c in &changes {
            println!(
                "  {} {} ({})",
                if c.created { "create" } else { "change" },
                c.path,
                c.note
            );
        }
        for m in &manual {
            println!("{m}");
        }
    }
    if o.dry_run || !write {
        println!("dry run: nothing written");
        return Ok(0);
    }
    if interactive {
        print!("Write these files? [y/N]: ");
        let _ = std::io::stdout().flush();
        let mut s = String::new();
        let _ = std::io::stdin().lock().read_line(&mut s);
        if !matches!(s.trim(), "y" | "Y" | "yes") {
            println!("nothing written");
            return Ok(0);
        }
    } else {
        println!(
            "spec-dna init: {} repository; profile {}; {} root{}{}",
            plan.kind,
            plan.profile.name(),
            plan.rows.len(),
            if plan.rows.len() == 1 { "" } else { "s" },
            if exists { "" } else { " (see spec-dna.toml)" }
        );
    }
    let written = apply(repo, &changes)?;
    report(&written, &plan.adapters);
    for m in &manual {
        println!("{m}");
    }
    Ok(0)
}

/// Writes the changes. Returns the paths.
fn apply(repo: &Repo, changes: &[Change]) -> Result<Vec<String>, Fatal> {
    let mut written = Vec::new();
    for c in changes {
        let p = repo.root.join(&c.path);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| Fatal::Io(e.to_string()))?;
        }
        crate::mv::write(&p, &c.text)?;
        written.push(c.path.clone());
    }
    Ok(written)
}

/// The bytes that spec-dna adds to the prompt prefix of each request.
/// Claude Code reads one copy of the line (in CLAUDE.md, or in AGENTS.md
/// through an import) and the skill descriptions. Other agents read
/// AGENTS.md.
fn prefix(adapters: &[Adapter]) -> (usize, usize, usize) {
    let line = adapters::line().len() + 1;
    let skill = if adapters.contains(&Adapter::Skill) {
        adapters::skill_prefix_bytes()
    } else {
        0
    };
    let agents = if adapters.contains(&Adapter::AgentsMd) {
        line
    } else {
        0
    };
    (line, skill, agents)
}

fn report(written: &[String], adapters: &[Adapter]) {
    if written.is_empty() {
        println!("spec-dna init: everything is up to date");
    }
    for w in written {
        println!("wrote {w}");
    }
    let (line, skill, agents) = prefix(adapters);
    println!(
        "per-request prefix: {} bytes for Claude Code (line {line}, skill description {skill}), {agents} bytes for AGENTS.md",
        line + skill
    );
    if !written.is_empty() {
        println!("to commit: git add {}", written.join(" "));
    }
}

/// `init` with an existing `spec-dna.toml`: only the adapters, as in 0.1.
fn adapters_only(repo: &Repo, o: &Opts) -> Result<u8, Fatal> {
    let exists = |p: &str| repo.root.join(p).exists();
    let mut chosen = vec![Adapter::ClaudeMd, Adapter::Skill];
    if exists(adapters::AGENTS) || !exists(adapters::CLAUDE) {
        chosen.insert(0, Adapter::AgentsMd);
    }
    let changes = adapter_changes(repo, &chosen)?;
    if o.dry_run {
        for c in &changes {
            println!(
                "would {} {} ({})",
                if c.created { "create" } else { "change" },
                c.path,
                c.note
            );
        }
        println!("dry run: nothing written");
        return Ok(0);
    }
    let written = apply(repo, &changes)?;
    if o.json {
        let (line, skill, agents) = prefix(&chosen);
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .strs("written", &written)
                .raw(
                    "prefix_bytes",
                    &Obj::new()
                        .num("claude_code", line + skill)
                        .num("agents_md", agents)
                        .end()
                )
                .end()
        );
    } else {
        report(&written, &chosen);
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_commit_hook_goes_at_the_end_of_repos() {
        let t =
            "repos:\n  - repo: https://example.org/hooks\n    rev: v1\n    hooks:\n      - id: x\n";
        let out = pre_commit_append(t).unwrap();
        assert!(out.ends_with("  - repo: local\n    hooks:\n      - id: spec-dna\n        name: spec-dna check\n        entry: spec-dna check\n        language: system\n        pass_filenames: false\n        always_run: true\n"), "{out}");
        assert!(pre_commit_append("repos:\n  - repo: a\nci:\n  skip: []\n").is_none());
        assert!(pre_commit_append("default_stages: [commit]\n").is_none());
    }

    #[test]
    fn rows_render_as_a_valid_configuration() {
        let facts = Facts::default();
        let mut plan = defaults(&facts, Some("docs"));
        assert_eq!(plan.kind, "greenfield");
        assert_eq!(plan.rows.len(), 1);
        plan.outcome = vec!["Решение".into()];
        plan.amendments = vec!["Поправки".into()];
        plan.evidence = Some(Some(Severity::Error));
        plan.budgets = Budgets::Relaxed;
        plan.language = "de".into();
        let text = render(&plan);
        let cfg = Config::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(cfg.roots.len(), 1);
        assert_eq!(cfg.roots[0].path, "docs");
        assert!(cfg.types.is_empty(), "every type by default");
        assert!(cfg.allows(DocType::Contract));
        assert_eq!(
            cfg.roots[0].required_sections[DocType::Decision as usize],
            [["Решение", "Decision Outcome", "Decision"]]
        );
        assert_eq!(
            cfg.severity(crate::diag::Code::RequirementUnverified, Some(0)),
            Some(Severity::Error)
        );
        assert_eq!(
            cfg.severity(crate::diag::Code::RequirementKeyword, Some(0)),
            None
        );
        assert_eq!(cfg.budgets[DocType::Spec as usize], (20_000, 40_000));
        let mut r = plan.rows[0].clone();
        edit_row(
            &mut r,
            "type=guide status=active exclude=a/**,b.md legacy-ids=DEF-{n}",
        )
        .unwrap();
        assert_eq!(r.ty, Some(DocType::Guide));
        assert_eq!(r.exclude, ["a/**", "b.md"]);
        assert!(edit_row(&mut r, "colour=red").is_err());
    }
}
