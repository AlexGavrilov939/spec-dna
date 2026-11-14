//! `spec-dna init` (docs/init.md). It detects the facts of the repository
//! (`detect.rs`). It asks at most 10 questions with the detected defaults. It
//! shows a preview (the full `spec-dna.toml` and the files to create or
//! change), and it writes only after a confirmation. It never edits a
//! document. It changes an instruction file (`AGENTS.md`, `CLAUDE.md`) only by
//! its generated lines: the spec-dna line, and the model-team line of the
//! agent harness (ADR 0004).
//!
//! The modes are:
//!
//! - interactive (a terminal)
//! - `--non-interactive` (the detected defaults, also without a terminal)
//! - `--answers <file>`
//! - `--reconfigure` (the current values as defaults)
//! - `--dry-run` (the preview only)
//!
//! If `spec-dna.toml` exists and `--reconfigure` is not set, `init` updates
//! the adapters, and the harness files if the configuration has a `[harness]`
//! table. It never removes a file then.

use crate::adapters::{self, VERSION};
use crate::config::{self, Config, DEFAULT_BUDGETS, Profile, Rules};
use crate::detect::{self, Facts, GitInfo, Row};
use crate::diag::Severity;
use crate::harness::{self, Effort, Harness, Level, Model, Role};
use crate::json::{self, Obj};
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

/// The answers of the 10 questions, with the detected defaults.
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
    /// The agent harness (ADR 0004).
    pub harness: Harness,
    /// The harness of the current configuration, for `--reconfigure`.
    pub harness_old: Option<Harness>,
    /// Why a role is off by default.
    pub harness_notes: Vec<String>,
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
    // Every type by default. A list that omits a type makes each document of
    // that type an error, so a narrower list is the choice of the user.
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
        harness: Harness::new(Level::Lean),
        harness_old: None,
        harness_notes: Vec::new(),
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
    // Every current rule stays. The answers for evidence, budgets and the
    // language replace their rule when `render` writes the file.
    p.rules = cfg.rules.clone();
    if cfg.rules.iter().any(|r| r.0 == "budgets" && r.1.is_none()) {
        p.budgets = Budgets::Off;
    }
    if cfg.evidence_tests.is_empty() {
        p.style = Style::VerifiedBy;
    }
    // The current globs stay. The default list renders as no `[evidence]`.
    p.tests = cfg.evidence_tests.clone();
    p.rows = cfg.roots.iter().map(row_of_root).collect();
    if let Some(h) = &cfg.harness {
        p.harness = h.clone();
        p.harness_old = Some(h.clone());
    }
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
    for key in ["lint", "context", "repos", "conventions", "spec-dna-sha256"] {
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
    // The section keys of 0.3 (no default value): history, companion and
    // live sections.
    for (key, lists) in [
        ("history-sections", &r.history_sections),
        ("companion-sections", &r.companion_sections),
        ("live-sections", &r.live_sections),
    ] {
        let v: Vec<String> = row
            .types()
            .into_iter()
            .filter(|&t| !lists[t as usize].is_empty())
            .map(|t| format!("{} = {}", t.name(), toml_list(&lists[t as usize])))
            .collect();
        if !v.is_empty() {
            row.extra.push(format!("{key} = {{ {} }}", v.join(", ")));
        }
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
    // Rules: the answers that differ from the profile. The answers replace the
    // rule of the same group. The other rules stay.
    let mut rules: Vec<(String, &str)> = plan
        .rules
        .iter()
        .map(|(k, s)| (k.clone(), severity_name(*s)))
        .collect();
    let mut set = |key: &str, value: Option<&'static str>| {
        let at = rules.iter().position(|r| r.0 == key);
        match (at, value) {
            (Some(i), Some(v)) => rules[i].1 = v,
            (None, Some(v)) => rules.push((key.to_string(), v)),
            (Some(i), None) => {
                rules.remove(i);
            }
            (None, None) => {}
        }
    };
    set("evidence", plan.evidence.map(severity_name));
    match plan.budgets {
        Budgets::Off => set("budgets", Some("off")),
        // A budget level replaces a rule that turned the budgets off.
        _ if plan.rules.iter().any(|r| r.0 == "budgets" && r.1.is_none()) => set("budgets", None),
        _ => {}
    }
    if plan.language != "en" {
        set("lint", Some("off"));
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
        _ if !plan.tests.is_empty()
            && plan.tests != config::DEFAULT_TEST_GLOBS.map(str::to_string) =>
        {
            out.push_str(&format!(
                "\n[evidence]\ntests = {}  # test files with `covers: <doc-id>#<key>` tags\n",
                toml_list(&plan.tests)
            ))
        }
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
    out.push('\n');
    out.push_str(&plan.harness.toml_section());
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
    if decisions && !plan.outcome.is_empty() && !r.required_off.contains(&DocType::Decision) {
        let mut h = plan.outcome.clone();
        h.extend(config::DEFAULT_DECISION_SECTIONS.map(str::to_string));
        required.push(format!("decision = {}", toml_list(&h)));
    }
    for t in &r.required_off {
        required.push(format!("{} = []", t.name()));
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
    /// True if `init` deletes the file. Only the harness removes files.
    pub remove: bool,
    pub text: String,
    pub note: String,
}

impl Change {
    fn verb(&self) -> &'static str {
        if self.remove {
            "remove"
        } else if self.created {
            "create"
        } else {
            "change"
        }
    }
}

/// The adapter files for a set of adapters.
fn adapter_changes(
    repo: &Repo,
    chosen: &[Adapter],
    types: &[DocType],
) -> Result<Vec<Change>, Fatal> {
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
                remove: false,
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
        if skill != adapters::skill_file(types) {
            out.push(Change {
                path: adapters::SKILL.into(),
                created: skill.is_empty(),
                remove: false,
                text: adapters::skill_file(types),
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

/// The GitHub Actions job, as in docs/quickstart.md. With the sum of the
/// release asset in `spec-dna.toml` and no `install-source`, the job downloads
/// the binary and verifies its sum. Otherwise it builds the pin from source.
fn github_workflow(base: &str) -> String {
    let download = format!(
        "            curl --fail --silent --show-error --location --output \"$RUNNER_TEMP/$asset\" \"{}/releases/download/v$pin/$asset\"",
        config::DEFAULT_INSTALL_SOURCE
    );
    let build = format!(
        "            cargo install --locked --git \"${{src:-{}}}\" --tag \"v$pin\" spec-dna",
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
        "          asset=\"spec-dna-$pin-x86_64-unknown-linux-gnu\"",
        "          sum=$(sed -n \"s/^\\\"$asset\\\" = \\\"\\(.*\\)\\\"/\\1/p\" spec-dna.toml)",
        "          if [ -z \"$src\" ] && [ -n \"$sum\" ]; then",
        &download,
        "            echo \"$sum  $RUNNER_TEMP/$asset\" | sha256sum --check -",
        "            install -D -m 755 \"$RUNNER_TEMP/$asset\" \"$HOME/.local/bin/spec-dna\"",
        "            echo \"$HOME/.local/bin\" >> \"$GITHUB_PATH\"",
        "          else",
        &build,
        "          fi",
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
        "      entry: spec-dna check --staged",
        "      language: system",
        "      pass_filenames: false",
        "      always_run: true",
    ])
}

/// The hook that 0.4.1 wrote runs `spec-dna check`, which reads the working
/// tree. A commit can pass while the committed tree fails. The hook of this
/// version runs `spec-dna check --staged`, which reads the index. This
/// function changes the old line of a hook file in place. It changes only the
/// exact line that 0.4.1 wrote. Any other text stays the same.
fn hook_upgrade(repo: &Repo, path: &str) -> Option<Change> {
    let old = std::fs::read_to_string(repo.root.join(path)).ok()?;
    let (is_old, new_line): (fn(&str) -> bool, &str) = if path == ".husky/pre-commit" {
        (|l| l == "spec-dna check", "spec-dna check --staged")
    } else {
        (
            |l| l == "entry: spec-dna check",
            "entry: spec-dna check --staged",
        )
    };
    let mut changed = false;
    let text: String = old
        .split_inclusive('\n')
        .map(|l| {
            let body = l.trim_end_matches(['\n', '\r']);
            let t = body.trim_start();
            if is_old(t.trim_end()) {
                changed = true;
                let pad = &body[..body.len() - t.len()];
                format!("{pad}{new_line}{}", &l[body.len()..])
            } else {
                l.to_string()
            }
        })
        .collect();
    changed.then(|| Change {
        path: path.to_string(),
        created: false,
        remove: false,
        text,
        note: "the hook of 0.4.1 reads the working tree; it now runs `spec-dna check --staged`"
            .into(),
    })
}

/// Appends the local hook to `.pre-commit-config.yaml`, if `repos:` is its
/// last top-level key. Otherwise returns `None`, and the preview gives the
/// text to add manually.
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

/// The CI and hook files, and the text to add manually.
fn ci_changes(repo: &Repo, plan: &Plan) -> (Vec<Change>, Vec<String>) {
    let mut out = Vec::new();
    let mut manual = Vec::new();
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).ok();
    match plan.ci.as_str() {
        "github" => {
            let path = ".github/workflows/spec-dna.yml";
            let text = github_workflow(&plan.base);
            match read(path) {
                None => out.push(Change {
                    path: path.into(),
                    created: true,
                    remove: false,
                    text,
                    note: "the CI job: `spec-dna check --base`".into(),
                }),
                // A workflow of the project stays as it is.
                Some(old) if old != text => manual.push(format!(
                    "{path} exists and differs from the job of this version; init does not change it"
                )),
                Some(_) => {}
            }
        }
        "gitlab" => manual.push(format!(
            "Add this job to .gitlab-ci.yml:\nspec-dna:\n  stage: test\n  script:\n    - cargo install --locked --git {} --tag v{VERSION}\n    - git fetch origin \"${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n    - spec-dna check --base \"origin/${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n  rules:\n    - if: $CI_PIPELINE_SOURCE == \"merge_request_event\"",
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
                out.extend(hook_upgrade(repo, path));
            } else if old.is_empty() {
                out.push(Change {
                    path: path.into(),
                    created: true,
                    remove: false,
                    text: format!("repos:\n{}", indent2(&pre_commit_hook())),
                    note: "a local hook: `spec-dna check --staged`".into(),
                });
            } else if let Some(text) = pre_commit_append(&old) {
                out.push(Change {
                    path: path.into(),
                    created: false,
                    remove: false,
                    text,
                    note: "a local hook at the end: `spec-dna check --staged`".into(),
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
            if old.lines().any(|l| l.trim() == "spec-dna check") {
                out.extend(hook_upgrade(repo, path));
            } else if !old.lines().any(|l| l.trim() == "spec-dna check --staged") {
                let mut t = old.clone();
                if !t.is_empty() && !t.ends_with('\n') {
                    t.push('\n');
                }
                t.push_str("spec-dna check --staged\n");
                out.push(Change {
                    path: path.into(),
                    created: old.is_empty(),
                    remove: false,
                    text: t,
                    note: "one line: `spec-dna check --staged`".into(),
                });
            }
        }
        "lefthook" => manual.push(
            "Add this command to lefthook.yml:\npre-commit:\n  commands:\n    spec-dna:\n      run: spec-dna check --staged".into(),
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

fn parse_level(w: &str) -> Result<Level, String> {
    Level::from_name(w).ok_or_else(|| format!("harness level `{w}`: use none, lean or full"))
}

/// The line of question 10: one `role=model[:effort]` word for each role of
/// the level. The advisor word has the model of `fallback-advisor` after a
/// slash, if it is not the default.
fn models_line(h: &Harness) -> String {
    let lean = Role::ALL.into_iter().filter(|r| !r.full_only());
    let full = Role::ALL.into_iter().filter(|r| r.full_only());
    let mut out = Vec::new();
    for r in lean.chain(full).filter(|r| h.level.includes(*r)) {
        let c = h.cfg(r);
        let mut w = format!("{}={}", r.key(), c.model.name());
        if r == Role::Advisor && c.agent != Model::Id("opus".into()) {
            w += &format!("/{}", c.agent.name());
        }
        let shown = match r {
            Role::Helper => false,
            Role::Advisor => c.effort != Effort::XHigh,
            _ => true,
        };
        if shown && c.model != Model::None {
            w += &format!(":{}", c.effort.name());
        }
        out.push(w);
    }
    out.join(" ")
}

/// Applies `role=model[:effort]` words to the harness. The advisor word can
/// be `advisor=<model>/<agent model>[:effort]`. The effort of the advisor
/// belongs to its agent.
fn apply_models(h: &mut Harness, words: &[String]) -> Result<(), String> {
    for w in words {
        let (key, value) = w
            .split_once('=')
            .ok_or_else(|| format!("`{w}`: write role=model[:effort]"))?;
        let role = Role::from_key(key).ok_or_else(|| {
            let keys: Vec<&str> = Role::ALL.iter().map(|r| r.key()).collect();
            format!("unknown role `{key}`; use {}", keys.join(", "))
        })?;
        if h.level != Level::None && !h.level.includes(role) {
            return Err(format!("role `{key}` is in level full"));
        }
        // A suffix with a digit belongs to the model id, for example `-v1:0`.
        // A suffix of letters only, or an empty one, is an effort.
        let (spec, effort) = match value.rsplit_once(':') {
            Some((m, e)) => match Effort::from_name(e) {
                Some(x) => (m, Some(x)),
                None if e.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    return Err(format!(
                        "`{w}`: unknown effort `{e}`; use low, medium, high, xhigh, max or inherit"
                    ));
                }
                None => (value, None),
            },
            None => (value, None),
        };
        let (model, agent) = match spec.split_once('/') {
            Some((m, a)) if role == Role::Advisor => (m, Some(a)),
            _ => (spec, None),
        };
        if model.is_empty() || agent.is_some_and(str::is_empty) {
            return Err(format!("`{w}`: the model is empty"));
        }
        if role == Role::Helper && effort.is_some() {
            return Err(format!("`{w}`: the role `helper` has no effort"));
        }
        let c = h.cfg_mut(role);
        c.model = Model::parse(model);
        if let Some(a) = agent {
            c.agent = Model::parse(a);
        }
        if let Some(e) = effort {
            c.effort = e;
        }
    }
    Ok(())
}

/// Reads the `[harness]` table that the harness renders, with the rules of
/// `spec-dna.toml`. It reports a model or an effort that the file does not
/// accept as a usage error.
fn validate_harness(h: &Harness) -> Result<(), String> {
    let t: toml::Table = h
        .toml_section()
        .parse()
        .map_err(|e: toml::de::Error| e.to_string())?;
    match t.get("harness") {
        Some(v) => Harness::parse(v).map(|_| ()),
        None => Ok(()),
    }
}

/// The harness files that exist: (path, text) of `CLAUDE.md`,
/// `.claude/settings.json` and `.claude/agents/*.md`.
fn harness_existing(repo: &Repo) -> Vec<(String, String)> {
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).ok();
    let mut out = Vec::new();
    for p in [harness::CLAUDE, harness::SETTINGS] {
        if let Some(t) = read(p) {
            out.push((p.to_string(), t));
        }
    }
    if let Ok(dir) = std::fs::read_dir(repo.root.join(harness::AGENTS_DIR)) {
        let mut names: Vec<String> = dir
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        for n in names {
            let p = format!("{}{n}", harness::AGENTS_DIR);
            if harness::wanted(&p)
                && let Some(t) = read(&p)
            {
                out.push((p, t));
            }
        }
    }
    out
}

/// Turns off each role whose agent name belongs to a file that spec-dna did
/// not generate, and tells why. With `outside_level`, only a role that the
/// level does not include changes: the configuration set the other roles, and
/// a conflict with them is an error of `plan`.
fn apply_conflicts(
    h: &mut Harness,
    existing: &[(String, String)],
    outside_level: bool,
) -> Vec<String> {
    let mut notes = Vec::new();
    for (role, path) in harness::name_conflicts(existing) {
        if outside_level && h.level.includes(role) {
            continue;
        }
        let Some(def) = role.agent() else { continue };
        if role == Role::Advisor {
            // The advisor tool needs no file. Only the fallback agent goes.
            h.cfg_mut(role).agent = Model::None;
            notes.push(format!(
                "{path} holds the agent `{}`, and spec-dna did not generate it: the advisor gets no `fallback-advisor` file",
                def.name
            ));
        } else {
            h.cfg_mut(role).model = Model::None;
            notes.push(format!(
                "{path} holds the agent `{}`, and spec-dna did not generate it: the role `{}` is none",
                def.name,
                role.key()
            ));
        }
    }
    notes
}

/// The defaults of the harness for a new configuration: level `lean`, the
/// models of the table, the deny rule for notebooks, the main model and the
/// advisor of an existing `.claude/settings.json`, and no role over a file of
/// the project. Returns the harness and the reasons of the roles that are off.
fn harness_defaults(repo: &Repo, existing: &[(String, String)]) -> (Harness, Vec<String>) {
    let mut h = Harness::new(Level::Lean);
    let notebooks = repo
        .git
        .ls_files()
        .is_ok_and(|f| f.iter().any(|e| e.0.ends_with(".ipynb")));
    if !notebooks {
        h.deny.push("NotebookEdit".into());
    }
    let settings = existing
        .iter()
        .find(|e| e.0 == harness::SETTINGS)
        .and_then(|e| json::parse(&e.1).ok());
    if let Some(v) = settings {
        let pick = |key: &str| {
            v.get(key)
                .as_str()
                .filter(|m| !m.is_empty() && !m.contains(char::is_whitespace))
                .filter(|m| !matches!(*m, "none" | "inherit"))
                .map(str::to_string)
        };
        if let Some(m) = pick("model") {
            h.cfg_mut(Role::Orchestrator).model = Model::Id(m);
        }
        if let Some(m) = pick("advisorModel").filter(|m| !m.contains("haiku")) {
            h.cfg_mut(Role::Advisor).model = Model::Id(m);
        }
    }
    let notes = apply_conflicts(&mut h, existing, false);
    (h, notes)
}

/// Refuses the harness if git ignores a file that it writes: each generated
/// file, `CLAUDE.md` and the settings file. A team could not commit it.
fn refuse_ignored(repo: &Repo, h: &Harness) -> Result<(), Fatal> {
    let mut paths: Vec<String> = harness::generated_files(h)
        .into_iter()
        .map(|g| g.path)
        .collect();
    paths.push(harness::CLAUDE.to_string());
    paths.push(harness::SETTINGS.to_string());
    for p in paths {
        if repo.git.is_ignored(&p) {
            return Err(Fatal::Usage(format!(
                "git ignores {p}, so the harness files cannot be committed; remove the ignore rule, or choose the harness level none"
            )));
        }
    }
    Ok(())
}

/// The harness plan of `init`, merged into the list of changes.
struct HarnessRun {
    level: Level,
    plan: harness::Plan,
    notes: Vec<String>,
}

/// Plans the harness files and merges them into `changes`. The text of
/// `CLAUDE.md` after the adapter line is the input of the plan, and the two
/// changes of `CLAUDE.md` become one. Returns `None` if the harness is off and
/// nothing of it is installed.
fn plan_harness(
    repo: &Repo,
    new: &Harness,
    old: Option<&Harness>,
    notes: &[String],
    mode: harness::Mode,
    changes: &mut Vec<Change>,
) -> Result<Option<HarnessRun>, Fatal> {
    let mut existing = harness_existing(repo);
    let installed = harness::is_installed(&existing);
    if new.level == Level::None && !(mode == harness::Mode::Reconfigure && installed) {
        return Ok(None);
    }
    if new.level != Level::None {
        refuse_ignored(repo, new)?;
    }
    if let Some(c) = changes.iter().find(|c| c.path == harness::CLAUDE) {
        existing.retain(|e| e.0 != harness::CLAUDE);
        existing.push((harness::CLAUDE.to_string(), c.text.clone()));
    }
    let mut plan = harness::plan(&existing, old, new, mode).map_err(Fatal::Usage)?;
    // The adapter step can create `CLAUDE.md`, and then the harness step sees a
    // file. The preview must say that the file is new.
    let claude_is_new = changes
        .iter()
        .any(|c| c.path == harness::CLAUDE && c.created);
    for c in plan
        .changes
        .iter_mut()
        .filter(|c| c.path == harness::CLAUDE)
    {
        if claude_is_new && c.action == harness::Action::Change {
            c.action = harness::Action::Create;
        }
    }
    for c in &plan.changes {
        let remove = c.action == harness::Action::Remove;
        match changes.iter_mut().find(|x| x.path == c.path) {
            Some(x) => {
                x.text = c.text.clone();
                x.remove = remove;
                x.note = format!("{}; {}", x.note, c.note);
            }
            None => changes.push(Change {
                path: c.path.clone(),
                created: c.action == harness::Action::Create,
                remove,
                text: c.text.clone(),
                note: c.note.clone(),
            }),
        }
    }
    Ok(Some(HarnessRun {
        level: new.level,
        plan,
        notes: notes.to_vec(),
    }))
}

/// Prints the harness block of the preview, or its summary after a write.
fn print_harness(h: &HarnessRun, with_files: bool) {
    println!("Agent harness: {} (for Claude Code)", h.level.name());
    for n in &h.notes {
        println!("  note: {n}");
    }
    if with_files {
        for c in &h.plan.changes {
            println!("  {} {} ({})", c.action.name(), c.path, c.note);
        }
        if h.plan.changes.is_empty() {
            println!("  no file changes");
        }
    }
    for k in &h.plan.kept {
        println!("  {k}");
    }
    for w in &h.plan.warnings {
        println!("  warning: {w}");
    }
    if h.level != Level::None {
        println!(
            "  The harness adds about {} bytes to each request of the main session (the CLAUDE.md line and the agent descriptions).",
            h.plan.request_bytes
        );
        println!("  Each teammate needs spec-dna 0.5.0 or later on PATH for the hooks.");
    }
}

/// The `harness` object of `--json`.
fn harness_json(h: Option<&HarnessRun>, level: Level) -> String {
    let Some(h) = h else {
        return Obj::new()
            .str("level", level.name())
            .raw("files", "[]")
            .strs("kept", &[])
            .strs("warnings", &[])
            .strs("notes", &[])
            .num("request_bytes", 0)
            .end();
    };
    let files = h.plan.changes.iter().map(|c| {
        Obj::new()
            .str("path", &c.path)
            .str("action", c.action.name())
            .str("note", &c.note)
            .end()
    });
    Obj::new()
        .str("level", h.level.name())
        .raw("files", &json::array(files))
        .strs("kept", &h.plan.kept)
        .strs("warnings", &h.plan.warnings)
        .strs("notes", &h.notes)
        .num("request_bytes", h.plan.request_bytes)
        .end()
}

const RESTART: &str = "Restart Claude Code. Then check: the header shows the main model, /advisor shows the advisor, and @ lists the agents.";

/// The types of a plan change the roots of a new project. They also change the
/// type of a detected root whose type is not chosen (it becomes a note).
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
            r.extra.retain(|e| {
                !e.starts_with("required-sections") && !e.starts_with("amendment-sections")
            });
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
                retype(r);
            }
            "default-type" => {
                r.default_type =
                    DocType::from_name(v).ok_or_else(|| format!("unknown type `{v}`"))?;
                retype(r);
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

/// When the type of a row changes, the status, the status map and the section
/// keys keep only the values that the new types allow.
fn retype(r: &mut Row) {
    let types = r.types();
    let ok = |s: &str| types.iter().any(|t| t.statuses().contains(&s));
    if r.status
        .as_deref()
        .is_some_and(|s| !r.default_type.statuses().contains(&s))
    {
        r.status = match r.default_type {
            DocType::Decision | DocType::Change => Some("proposed".into()),
            t if t.has_status() => Some("active".into()),
            _ => None,
        };
    }
    if !r.default_type.has_status() {
        r.status = None;
    }
    r.status_map.retain(|(from, to)| {
        ok(to) && !types.iter().all(|t| t.statuses().contains(&from.as_str()))
    });
    r.required_off.retain(|t| types.contains(t));
    r.extra
        .retain(|e| !e.starts_with("required-sections") && !e.starts_with("amendment-sections"));
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

/// The value of the evidence answer, as the question shows it.
fn evidence_value(plan: &Plan) -> String {
    let ev = match plan.evidence {
        None => "profile",
        Some(s) => severity_name(s),
    };
    let st = match plan.style {
        Style::Covers => "covers",
        Style::VerifiedBy => "verified-by",
        Style::Both => "both",
    };
    format!("{ev}, {st}")
}

/// The value of the language answer, as the question shows it.
fn language_value(plan: &Plan) -> String {
    let aliases = |v: &[String]| {
        if v.is_empty() {
            "-".to_string()
        } else {
            list(v)
        }
    };
    format!(
        "{}; outcome: {}; amendments: {}",
        plan.language,
        aliases(&plan.outcome),
        aliases(&plan.amendments)
    )
}

fn budgets_value(b: Budgets) -> &'static str {
    match b {
        Budgets::Default => "default",
        Budgets::Relaxed => "relaxed",
        Budgets::Tight => "tight",
        Budgets::Off => "off",
    }
}

/// Asks the questions on a terminal.
fn ask(plan: &mut Plan, facts: &Facts, root: &str, io: &mut Ask) -> Result<(), String> {
    io.say(&format!(
        "spec-dna init: {} repository with {} files and {} Markdown documents. Press Enter to keep a default.",
        plan.kind, facts.files, facts.markdown
    ));
    io.say("lite: warnings only. standard: broken references and budget growth are errors. strict: drift, evidence and budgets are errors, and lint gives warnings.");
    let p = io.line("1/10 Profile (lite, standard, strict)", plan.profile.name());
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
    let t = io.line("2/10 Knowledge types", &list(&type_names(&plan.types)));
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
        "3/10 Evidence policy (profile, off, warning, error) and link style (covers, verified-by, both)",
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
        "    Heading scripts: {}. The requirement lint reads English only.",
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
    let a = io.line("4/10 Normative language and heading aliases", &default);
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
        "5/10 Mapping: {} roots. For each root, press Enter to accept it, enter `drop` to remove it, or enter `key=value ...` to edit it (path, type, default-type, status, exclude, legacy-ids, legacy-keys).",
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
        "6/10 Budget level (default, relaxed, tight, off)",
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
        "7/10 Agent adapters (agents-md, claude-md, skill, none)",
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
        "8/10 CI step (github, gitlab, none), hook (pre-commit, husky, lefthook, none) and base branch",
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

    io.say("    The agent harness is a team of models for Claude Code. lean writes the main model, the advisor, `checker`, `Explore`, `fallback-advisor`, 2 hooks and a model-team line in CLAUDE.md.");
    io.say("    full also writes `implementer` and `spec-writer`. none writes nothing. The files are for Claude Code only.");
    for n in &plan.harness_notes {
        io.say(&format!("    {n}"));
    }
    let a = io.line(
        "9/10 Agent harness (none, lean, full)",
        plan.harness.level.name(),
    );
    plan.harness.level = parse_level(&a)?;
    if plan.harness.level != Level::None {
        io.say("    The main model sets the cost of each request.");
        io.say("    The advisor reads the whole context at its own price each time that it is consulted. It needs Fable access, or `fallback-advisor` gives the advice.");
        io.say("    `checker` and `Explore` run only when the main session uses them.");
        io.say("    `none` turns a role off. Write advisor=<model>/<model of fallback-advisor> to set both.");
        let a = io.line(
            "10/10 Models (role=model[:effort] ...)",
            &models_line(&plan.harness),
        );
        apply_models(&mut plan.harness, &words(&a))?;
    }
    validate_harness(&plan.harness)?;
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
    const KEYS: [&str; 19] = [
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
        "harness",
        "harness-models",
        "harness-deny",
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
    if let Some(v) = s("harness")? {
        plan.harness.level = parse_level(&v).map_err(|e| format!("answers: {e}"))?;
    }
    if let Some(v) = l("harness-deny")? {
        plan.harness.deny.clear();
        for d in v {
            if !plan.harness.deny.contains(&d) {
                plan.harness.deny.push(d);
            }
        }
    }
    if let Some(v) = l("harness-models")? {
        apply_models(&mut plan.harness, &v).map_err(|e| format!("answers: harness-models: {e}"))?;
    }
    validate_harness(&plan.harness).map_err(|e| format!("answers: {e}"))?;
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
            return adapters_only(repo, o, &cfg);
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
    let existing = harness_existing(repo);
    let was_none = plan
        .harness_old
        .as_ref()
        .is_some_and(|h| h.level == Level::None);
    if plan.harness_old.is_none() || was_none {
        // A configuration of level `none` holds no other key, so a new level
        // takes the fresh defaults. The level `none` stays the default.
        let (h, notes) = harness_defaults(repo, &existing);
        plan.harness = h;
        plan.harness_notes = notes;
        if was_none {
            plan.harness.level = Level::None;
        }
    } else {
        // A new level can add a role whose agent name the project uses.
        let h = &mut plan.harness;
        plan.harness_notes = apply_conflicts(h, &existing, true);
    }
    let before = plan.clone();
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
            remove: false,
            text: text.clone(),
            note: if old.is_some() {
                "replaced; comments of the old file are lost".into()
            } else {
                "the configuration".into()
            },
        });
    }
    changes.extend(adapter_changes(repo, &plan.adapters, &plan.types)?);
    let (ci, manual) = ci_changes(repo, &plan);
    changes.extend(ci);
    // A new configuration and `--reconfigure` both replace the harness.
    let hrun = plan_harness(
        repo,
        &plan.harness,
        plan.harness_old.as_ref(),
        &plan.harness_notes,
        harness::Mode::Reconfigure,
        &mut changes,
    )?;

    let preview = o.dry_run || interactive || !write;
    let resolved = resolved_answers(&plan, &before, &text, &changes, &manual, hrun.as_ref());
    if o.json {
        let answers = resolved.iter().map(|r| {
            Obj::new()
                .num("number", r.number)
                .str("name", r.name)
                .str("value", &r.value)
                .str("default", &r.offered)
                .bool("is_default", r.default)
                .str("writes", &r.writes)
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
                .raw("answers", &crate::json::array(answers))
                .str("config", &text)
                .raw("files", &files_json(repo, &changes))
                .strs("manual", &manual)
                .raw("harness", &harness_json(hrun.as_ref(), plan.harness.level))
                .strs("written", &written)
                .end()
        );
        return Ok(0);
    }
    if preview {
        println!("\nHow the answers were applied:");
        for r in &resolved {
            println!("{}", r.line());
        }
        println!("\n--- spec-dna.toml ---\n{}---", text);
        println!("Files to create or change:");
        if changes.is_empty() {
            println!("  none");
        }
        for c in &changes {
            if hrun.as_ref().is_some_and(|h| {
                c.path != harness::CLAUDE && h.plan.changes.iter().any(|x| x.path == c.path)
            }) {
                continue;
            }
            println!("  {} {} ({})", c.verb(), c.path, c.note);
        }
        for c in changes.iter().filter(|c| c.path != config::FILE_NAME) {
            print!("\n{}", change_diff(repo, c));
        }
        for m in &manual {
            println!("{m}");
        }
        if let Some(h) = &hrun {
            print_harness(h, true);
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
    report(&written, &plan.adapters, &changes);
    for m in &manual {
        println!("{m}");
    }
    report_harness(hrun.as_ref(), &written);
    report_commit(&written);
    Ok(0)
}

/// Writes the changes. Returns the paths.
fn apply(repo: &Repo, changes: &[Change]) -> Result<Vec<String>, Fatal> {
    let mut written = Vec::new();
    for c in changes {
        let p = repo.root.join(&c.path);
        if c.remove {
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(Fatal::Io(format!("{}: {e}", p.display()))),
            }
            written.push(c.path.clone());
            continue;
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| Fatal::Io(e.to_string()))?;
        }
        crate::mv::write(&p, &c.text)?;
        written.push(c.path.clone());
    }
    Ok(written)
}

/// One answer of the 10 questions, as the preview shows it.
struct Resolved {
    number: usize,
    name: &'static str,
    value: String,
    /// The default of the question.
    offered: String,
    /// True if the value equals the default of the question.
    default: bool,
    /// What the answer writes. Empty if it writes nothing.
    writes: String,
}

impl Resolved {
    fn line(&self) -> String {
        let state = if self.default {
            "default".to_string()
        } else {
            format!("changed from {}", self.offered)
        };
        if self.writes.is_empty() {
            format!(
                "  {}/10 {}: {} ({state}, writes nothing)",
                self.number, self.name, self.value
            )
        } else {
            format!(
                "  {}/10 {}: {} ({state}) -> {}",
                self.number, self.name, self.value, self.writes
            )
        }
    }
}

/// How `init` applied each of the 10 answers. `before` is the plan that the
/// questions offered, so its values are the defaults. `text` is the new
/// configuration, and `changes` are the files to write. `hrun` is the harness
/// plan.
fn resolved_answers(
    plan: &Plan,
    before: &Plan,
    text: &str,
    changes: &[Change],
    manual: &[String],
    hrun: Option<&HarnessRun>,
) -> Vec<Resolved> {
    let types = |p: &Plan| {
        if p.types.len() == DocType::COUNT {
            format!("all {}", DocType::COUNT)
        } else {
            list(&type_names(&p.types))
        }
    };
    // The value has the full line of each root, so a change of a root shows in
    // the line.
    let mapping = |p: &Plan| {
        format!(
            "{} root{}: {}",
            p.rows.len(),
            if p.rows.len() == 1 { "" } else { "s" },
            p.rows.iter().map(row_line).collect::<Vec<_>>().join("; ")
        )
    };
    let ci = |p: &Plan| format!("{}, {}, {}", p.ci, p.hook, p.base);
    let adapter_files: Vec<&str> = changes
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| adapters::PATHS.contains(p))
        // The harness writes `CLAUDE.md` too, and question 9 lists it.
        .filter(|p| *p != harness::CLAUDE || plan.adapters.contains(&Adapter::ClaudeMd))
        .collect();
    let ci_files: Vec<&str> = changes
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| *p != config::FILE_NAME && !adapters::PATHS.contains(p))
        .filter(|p| !hrun.is_some_and(|h| h.plan.changes.iter().any(|x| x.path == *p)))
        .collect();
    // The harness files that the answer writes. A removal says so.
    let harness_files: Vec<String> = hrun.map_or_else(Vec::new, |h| {
        h.plan
            .changes
            .iter()
            .map(|c| match c.action {
                harness::Action::Remove => format!("removes {}", c.path),
                _ => c.path.clone(),
            })
            .collect()
    });
    let harness_writes = {
        let mut w = vec![format!(
            "[harness] level = \"{}\"",
            plan.harness.level.name()
        )];
        w.extend(harness_files);
        w.join(", ")
    };
    let models = |p: &Plan| {
        let m = models_line(&p.harness);
        if m.is_empty() { "-".to_string() } else { m }
    };
    let has = |needle: &str| text.contains(needle);
    let mut evidence = Vec::new();
    if plan.evidence.is_some() {
        evidence.push("[rules] evidence");
    }
    if has("\n[evidence]\n") {
        evidence.push("[evidence] tests");
    }
    let mut language = Vec::new();
    if plan.language != "en" {
        language.push("[rules] lint = \"off\"");
    }
    if !plan.outcome.is_empty() && has("required-sections = { decision =") {
        language.push("required-sections");
    }
    if !plan.amendments.is_empty() && has("amendment-sections = { decision =") {
        language.push("amendment-sections");
    }
    let budgets = match plan.budgets {
        Budgets::Default => String::new(),
        Budgets::Off => "[rules] budgets = \"off\"".to_string(),
        _ => "[budgets.<type>] tables".to_string(),
    };
    let mut ci_writes = ci_files.join(", ");
    if ci_writes.is_empty() && !manual.is_empty() {
        ci_writes = "text to add by hand".into();
    }
    let rows = [
        (
            "profile",
            plan.profile.name().to_string(),
            before.profile.name().to_string(),
            format!("profile = \"{}\"", plan.profile.name()),
        ),
        (
            "knowledge types",
            types(plan),
            types(before),
            if plan.types.len() < DocType::COUNT {
                format!("types = {}", toml_list(&type_names(&plan.types)))
            } else {
                String::new()
            },
        ),
        (
            "evidence policy and link style",
            evidence_value(plan),
            evidence_value(before),
            evidence.join(", "),
        ),
        (
            "normative language and heading aliases",
            language_value(plan),
            language_value(before),
            language.join(", "),
        ),
        (
            "mapping",
            mapping(plan),
            mapping(before),
            if plan.rows.is_empty() {
                String::new()
            } else {
                format!("{} [[roots]] table(s)", plan.rows.len())
            },
        ),
        (
            "budget level",
            budgets_value(plan.budgets).to_string(),
            budgets_value(before.budgets).to_string(),
            budgets,
        ),
        (
            "agent adapters",
            list(&adapter_names(&plan.adapters)),
            list(&adapter_names(&before.adapters)),
            adapter_files.join(", "),
        ),
        (
            "CI step, hook and base branch",
            ci(plan),
            ci(before),
            ci_writes,
        ),
        (
            "agent harness",
            plan.harness.level.name().to_string(),
            before.harness.level.name().to_string(),
            harness_writes,
        ),
        (
            "harness models",
            models(plan),
            models(before),
            if plan.harness.level == Level::None {
                String::new()
            } else {
                "[harness.roles] table".to_string()
            },
        ),
    ];
    rows.into_iter()
        .enumerate()
        .map(|(i, (name, value, offered, writes))| {
            let default = value == offered;
            Resolved {
                number: i + 1,
                name,
                value,
                offered,
                default,
                writes,
            }
        })
        .collect()
}

/// One edit of a line diff.
enum Op<'a> {
    Keep(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// The largest table (old lines times new lines) that the diff builds. A
/// larger change is a block of removed lines and a block of added lines.
const DIFF_LIMIT: usize = 4_000_000;

fn edit_script<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Op<'a>> {
    let mut head = 0;
    while head < old.len() && head < new.len() && old[head] == new[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < old.len() - head
        && tail < new.len() - head
        && old[old.len() - 1 - tail] == new[new.len() - 1 - tail]
    {
        tail += 1;
    }
    let (a, b) = (&old[head..old.len() - tail], &new[head..new.len() - tail]);
    let mut ops: Vec<Op> = old[..head].iter().map(|l| Op::Keep(l)).collect();
    if a.len() * b.len() > DIFF_LIMIT {
        ops.extend(a.iter().map(|l| Op::Del(l)));
        ops.extend(b.iter().map(|l| Op::Add(l)));
    } else {
        // lcs[i][j] is the length of the longest common subsequence of a[i..] and
        // b[j..].
        let w = b.len() + 1;
        let mut lcs = vec![0u32; (a.len() + 1) * w];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                lcs[i * w + j] = if a[i] == b[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            if i < a.len() && j < b.len() && a[i] == b[j] {
                ops.push(Op::Keep(a[i]));
                i += 1;
                j += 1;
            } else if j == b.len() || (i < a.len() && lcs[(i + 1) * w + j] >= lcs[i * w + j + 1]) {
                ops.push(Op::Del(a[i]));
                i += 1;
            } else {
                ops.push(Op::Add(b[j]));
                j += 1;
            }
        }
    }
    ops.extend(old[old.len() - tail..].iter().map(|l| Op::Keep(l)));
    ops
}

/// The lines of context around a change in a diff.
const DIFF_CONTEXT: usize = 2;

/// A unified diff of two texts, or an empty string if they are equal. `old`
/// is `None` for a file that does not exist yet. `deleted` is true for a file
/// that `init` removes: `new` is then empty, and the new side is `/dev/null`.
fn unified_diff(path: &str, old: Option<&str>, new: &str, deleted: bool) -> String {
    if old == Some(new) || (deleted && old.is_none()) {
        return String::new();
    }
    // A line keeps its line ending, so a change of CRLF to LF, or of the final
    // newline, is a change of the diff.
    let old_lines: Vec<&str> = old.unwrap_or("").split_inclusive('\n').collect();
    let new_lines: Vec<&str> = new.split_inclusive('\n').collect();
    let ops = edit_script(&old_lines, &new_lines);
    // The position of each op in the old and the new text (1-based).
    let mut at = Vec::with_capacity(ops.len());
    let (mut o, mut n) = (1usize, 1usize);
    for op in &ops {
        at.push((o, n));
        match op {
            Op::Keep(_) => {
                o += 1;
                n += 1;
            }
            Op::Del(_) => o += 1,
            Op::Add(_) => n += 1,
        }
    }
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Keep(_)))
        .map(|(i, _)| i)
        .collect();
    let mut out = format!(
        "--- {}\n+++ {}\n",
        if old.is_some() {
            format!("a/{path}")
        } else {
            "/dev/null".to_string()
        },
        if deleted {
            "/dev/null".to_string()
        } else {
            format!("b/{path}")
        }
    );
    let mut k = 0;
    while k < changed.len() {
        let start = changed[k].saturating_sub(DIFF_CONTEXT);
        let mut end = changed[k];
        while k + 1 < changed.len() && changed[k + 1] <= end + 2 * DIFF_CONTEXT + 1 {
            k += 1;
            end = changed[k];
        }
        let end = (end + DIFF_CONTEXT + 1).min(ops.len());
        let count = |keep: bool, del: bool, add: bool| {
            ops[start..end]
                .iter()
                .filter(|op| match op {
                    Op::Keep(_) => keep,
                    Op::Del(_) => del,
                    Op::Add(_) => add,
                })
                .count()
        };
        let (old_n, new_n) = (count(true, true, false), count(true, false, true));
        let (o, n) = at[start];
        out.push_str(&format!(
            "@@ -{},{old_n} +{},{new_n} @@\n",
            if old_n == 0 { o - 1 } else { o },
            if new_n == 0 { n - 1 } else { n }
        ));
        for op in &ops[start..end] {
            let (mark, l) = match op {
                Op::Keep(l) => (' ', l),
                Op::Del(l) => ('-', l),
                Op::Add(l) => ('+', l),
            };
            out.push(mark);
            out.push_str(l.strip_suffix('\n').unwrap_or(l));
            out.push('\n');
            if !l.ends_with('\n') {
                out.push_str("\\ No newline at end of file\n");
            }
        }
        k += 1;
    }
    out
}

/// The `files` array of `--json`: the path, the action, the note and the diff
/// of each change. The harness files are in the list, in the same form.
fn files_json(repo: &Repo, changes: &[Change]) -> String {
    let files = changes.iter().map(|c| {
        Obj::new()
            .str("path", &c.path)
            .str("action", c.verb())
            .str("note", &c.note)
            .str("diff", &change_diff(repo, c))
            .end()
    });
    json::array(files)
}

/// The diff of one change against the file on disk. A removal is a diff to
/// `/dev/null`.
fn change_diff(repo: &Repo, c: &Change) -> String {
    let old = std::fs::read_to_string(repo.root.join(&c.path)).ok();
    let new = if c.remove { "" } else { &c.text };
    unified_diff(&c.path, old.as_deref(), new, c.remove)
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

fn report(written: &[String], adapters: &[Adapter], changes: &[Change]) {
    if written.is_empty() {
        println!("spec-dna init: everything is up to date");
    }
    for w in written {
        let removed = changes.iter().any(|c| c.remove && c.path == *w);
        println!("{} {w}", if removed { "removed" } else { "wrote" });
    }
    let (line, skill, agents) = prefix(adapters);
    println!(
        "per-request prefix: {} bytes for Claude Code (line {line}, skill description {skill}), {agents} bytes for AGENTS.md",
        line + skill
    );
}

/// Prints the `git add` command for the paths that `init` wrote.
fn report_commit(written: &[String]) {
    if !written.is_empty() {
        println!("to commit: git add {}", written.join(" "));
    }
}

/// Prints the summary of the harness after a write, and the restart line.
fn report_harness(h: Option<&HarnessRun>, written: &[String]) {
    let Some(h) = h else { return };
    print_harness(h, false);
    if h.level != Level::None && h.plan.changes.iter().any(|c| written.contains(&c.path)) {
        println!("{RESTART}");
    }
}

/// `init` with an existing `spec-dna.toml`: the adapters, and the harness
/// files if the configuration has a `[harness]` table. It writes and never
/// removes.
fn adapters_only(repo: &Repo, o: &Opts, cfg: &Config) -> Result<u8, Fatal> {
    let exists = |p: &str| repo.root.join(p).exists();
    let mut chosen = vec![Adapter::ClaudeMd, Adapter::Skill];
    if exists(adapters::AGENTS) || !exists(adapters::CLAUDE) {
        chosen.insert(0, Adapter::AgentsMd);
    }
    let mut changes = adapter_changes(repo, &chosen, &cfg.types)?;
    // A hook of 0.4.1 reads the working tree. The hook of this version reads
    // the index.
    for path in [".pre-commit-config.yaml", ".husky/pre-commit"] {
        changes.extend(hook_upgrade(repo, path));
    }
    let hrun = match &cfg.harness {
        Some(h) => plan_harness(repo, h, Some(h), &[], harness::Mode::Upgrade, &mut changes)?,
        None => None,
    };
    if o.dry_run {
        if o.json {
            println!(
                "{}",
                Obj::new()
                    .str("version", VERSION)
                    .bool("dry_run", true)
                    .raw("files", &files_json(repo, &changes))
                    .raw("harness", &harness_json(hrun.as_ref(), harness_level(cfg)))
                    .end()
            );
            return Ok(0);
        }
        for c in &changes {
            println!("would {} {} ({})", c.verb(), c.path, c.note);
            print!("{}", change_diff(repo, c));
        }
        if let Some(h) = &hrun {
            print_harness(h, false);
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
                .raw("harness", &harness_json(hrun.as_ref(), harness_level(cfg)))
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
        report(&written, &chosen, &changes);
        report_harness(hrun.as_ref(), &written);
        report_commit(&written);
    }
    Ok(0)
}

fn harness_level(cfg: &Config) -> Level {
    cfg.harness.as_ref().map_or(Level::None, |h| h.level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_commit_hook_goes_at_the_end_of_repos() {
        let t =
            "repos:\n  - repo: https://example.org/hooks\n    rev: v1\n    hooks:\n      - id: x\n";
        let out = pre_commit_append(t).unwrap();
        assert!(out.ends_with("  - repo: local\n    hooks:\n      - id: spec-dna\n        name: spec-dna check\n        entry: spec-dna check --staged\n        language: system\n        pass_filenames: false\n        always_run: true\n"), "{out}");
        assert!(pre_commit_append("repos:\n  - repo: a\nci:\n  skip: []\n").is_none());
        assert!(pre_commit_append("default_stages: [commit]\n").is_none());
    }

    #[test]
    fn rows_render_as_a_valid_configuration() {
        let facts = Facts::default();
        let mut plan = defaults(&facts, Some("docs"));
        assert_eq!(plan.kind, "greenfield");
        assert_eq!(plan.rows.len(), 1);
        plan.outcome = vec!["Decisión".into()];
        plan.amendments = vec!["Enmiendas".into()];
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
            [["Decisión", "Decision Outcome", "Decision"]]
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

    #[test]
    fn review_fixes_for_rows_and_rules() {
        // A type edit removes the status and the section keys of the old type.
        let mut r = blank_row("docs/adr", Some(DocType::Decision), DocType::Decision);
        r.status = Some("proposed".into());
        r.extra
            .push("required-sections = { decision = [\"Decisión\"] }".into());
        edit_row(&mut r, "type=spec").unwrap();
        assert_eq!(r.status.as_deref(), Some("active"));
        assert!(r.extra.is_empty());
        // A removed decision section stays removed with aliases.
        let facts = Facts::default();
        let mut plan = defaults(&facts, Some("docs"));
        plan.outcome = vec!["Decisión".into()];
        plan.rows[0].required_off.push(DocType::Decision);
        let text = render(&plan);
        assert!(
            text.contains("required-sections = { decision = [] }"),
            "{text}"
        );
        // The answers replace their rule. The other rules stay.
        plan.rules = vec![
            ("lint".into(), Some(Severity::Warning)),
            ("budgets".into(), Some(Severity::Error)),
            ("evidence".into(), Some(Severity::Error)),
        ];
        plan.evidence = None;
        let text = render(&plan);
        assert!(
            text.contains("lint = \"warning\"\nbudgets = \"error\"\n"),
            "{text}"
        );
        assert!(!text.contains("evidence ="), "{text}");
        plan.evidence = Some(Some(Severity::Warning));
        plan.budgets = Budgets::Off;
        let text = render(&plan);
        assert!(
            text.contains("budgets = \"off\"\nevidence = \"warning\"\n"),
            "{text}"
        );
        Config::parse(&text).unwrap();
        // The default test globs render as no `[evidence]` table.
        plan.tests = config::DEFAULT_TEST_GLOBS.map(str::to_string).to_vec();
        assert!(!render(&plan).contains("[evidence]"));
    }

    #[test]
    fn the_models_line_round_trips_through_its_parser() {
        let mut h = Harness::new(Level::Lean);
        assert_eq!(
            models_line(&h),
            "orchestrator=opus:xhigh advisor=fable reviewer=opus:high researcher=haiku:high helper=haiku"
        );
        h.level = Level::Full;
        assert!(
            models_line(&h)
                .ends_with("helper=haiku implementer=sonnet:high spec-writer=sonnet:high"),
            "{}",
            models_line(&h)
        );
        let edits: Vec<String> = [
            "orchestrator=sonnet:inherit",
            "advisor=opus/sonnet:high",
            "reviewer=none",
            "researcher=inherit",
        ]
        .map(String::from)
        .to_vec();
        apply_models(&mut h, &edits).unwrap();
        let line = models_line(&h);
        assert!(
            line.starts_with(
                "orchestrator=sonnet:inherit advisor=opus/sonnet:high reviewer=none researcher=inherit:high helper=haiku"
            ),
            "{line}"
        );
        let mut again = Harness::new(Level::Full);
        apply_models(&mut again, &words(&line)).unwrap();
        assert_eq!(again, h);
        validate_harness(&h).unwrap();
    }

    #[test]
    fn the_model_words_refuse_what_the_configuration_refuses() {
        let lean = || Harness::new(Level::Lean);
        for (word, want) in [
            ("reviewer", "write role=model[:effort]"),
            ("judge=opus", "unknown role `judge`"),
            ("implementer=opus", "role `implementer` is in level full"),
            ("helper=haiku:high", "the role `helper` has no effort"),
            ("reviewer=", "the model is empty"),
            ("reviewer=opus:ultra", "unknown effort `ultra`"),
            ("reviewer=opus:", "unknown effort ``"),
            ("advisor=opus/haiku:ultra", "unknown effort `ultra`"),
        ] {
            let e = apply_models(&mut lean(), &[word.to_string()]).unwrap_err();
            assert!(e.contains(want), "{word}: {e}");
        }
        // The configuration rules apply to the result.
        for (word, want) in [
            ("advisor=haiku", "cannot be \"haiku\""),
            ("advisor=claude-haiku-4-5", "cannot be \"claude-haiku-4-5\""),
            ("orchestrator=none", "cannot be \"none\""),
            ("orchestrator=opus:max", "cannot be \"max\""),
        ] {
            let mut h = lean();
            apply_models(&mut h, &[word.to_string()]).unwrap();
            let e = validate_harness(&h).unwrap_err();
            assert!(e.contains(want), "{word}: {e}");
        }
        // The fallback agent can be haiku.
        let mut h = lean();
        apply_models(&mut h, &["advisor=opus/haiku".to_string()]).unwrap();
        validate_harness(&h).unwrap();
        // A suffix that is no effort belongs to the model id.
        let mut h = lean();
        apply_models(&mut h, &["reviewer=vendor.model-v1:0".to_string()]).unwrap();
        assert_eq!(
            h.cfg(Role::Reviewer).model,
            Model::Id("vendor.model-v1:0".into())
        );
        // Level none takes any role word.
        let mut h = Harness::new(Level::None);
        apply_models(&mut h, &["implementer=opus".to_string()]).unwrap();
    }

    #[test]
    fn an_agent_name_in_use_turns_off_its_role_only() {
        let own =
            |path: &str, name: &str| (path.to_string(), format!("---\nname: {name}\n---\nown\n"));
        let existing = vec![
            own(".claude/agents/mine.md", "checker"),
            own(".claude/agents/fallback-advisor.md", "fallback-advisor"),
            own(".claude/agents/implementer.md", "implementer"),
        ];
        let mut h = Harness::new(Level::Lean);
        let notes = apply_conflicts(&mut h, &existing, false);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(h.cfg(Role::Reviewer).model, Model::None);
        assert_eq!(h.cfg(Role::Advisor).agent, Model::None);
        assert!(h.cfg(Role::Advisor).model != Model::None);
        assert_eq!(h.cfg(Role::Implementer).model, Model::None);
        // For an existing configuration only the roles outside the level go.
        let mut h = Harness::new(Level::Lean);
        let notes = apply_conflicts(&mut h, &existing, true);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(h.cfg(Role::Reviewer).model, Model::Id("opus".into()));
    }
}
