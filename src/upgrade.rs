//! The command `spec-dna upgrade` (design 0.6.0, D133 to D154 and D232 on; ADR
//! 0009). The module has 3 parts.
//!
//! - The plan (`plan`) is a pure function. It reads a tree and writes nothing,
//!   so `check` can run it on a base commit (S12). It calls the plans of the
//!   configuration (S5), the documents (S6, S7) and the files below (S9).
//! - The files part: `unmarked_roles` is step 4 of the plan. It names the roles
//!   whose agent file has no marker, so pass 2 of the configuration turns them
//!   off. `plan_files` is step 7. It gives the adapters and the harness files
//!   of a final configuration, as `init` writes them.
//! - The command (`run`) does what the plan must not do: it reads the disk and
//!   git, it checks the tree, it proves the plan, it writes, and it prints.

use crate::adapters::{self, VERSION};
use crate::check;
use crate::config::{self, Config, PinClass};
use crate::diag::{Diag, Severity};
use crate::evidence::{self, Cover};
use crate::finish;
use crate::git::Git;
use crate::harness::{self, Level, Mode, Model, Role};
use crate::init::{self, Adapter, HOOK_FILES};
use crate::json::{self, Obj};
use crate::model::{Corpus, DocType, LinkRes, Res};
use crate::mv;
use crate::parse::{ParsedDoc, RefKind, parse};
use crate::repo::{self, Fatal, Repo};
use crate::upgrade_config::{self, ConfigOpts};
use crate::upgrade_docs::{
    Basis, Classified, DocsPlan, Fate, MoveKind, Tree, TreeData, areas_of, classify, plan_docs,
};
use crate::upgrade_old::OldConfig;
use crate::upgrade_proof::{self, Planned, Proof};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// The adapter and harness files of a plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilesPlan {
    /// The adapters that `init` writes for the project.
    pub adapters: Vec<Adapter>,
    /// The final path of each file to write, with its new text.
    pub writes: BTreeMap<String, String>,
    /// The files to remove, in path order: the unedited files of an ended role.
    pub removes: Vec<String>,
    /// A short description of each write and remove, for the output.
    pub notes: BTreeMap<String, String>,
    /// The lines of the report that start with `kept`: a file that `upgrade`
    /// leaves, and why.
    pub kept: Vec<String>,
    /// The roles of 0.5 that ended, each with its model: (role, model).
    pub ended_roles: Vec<(String, String)>,
    /// The warnings of the configuration of the harness.
    pub warnings: Vec<String>,
    /// The bytes that the harness adds to each request of the main session.
    pub request_bytes: usize,
}

/// A file that `upgrade` must read as text. Another file that is not UTF-8,
/// such as a skill of the project, is not read, and no plan writes it.
fn must_be_text(path: &str) -> bool {
    harness::is_output_path(path) || adapters::PATHS.contains(&path) || HOOK_FILES.contains(&path)
}

/// The text of each file of the tree that the adapters and the harness read.
/// A path that the plan can write and that is not UTF-8 stops the plan: the
/// plan would replace it with no way to read it (D210).
fn read_texts(tree: &Tree) -> Result<HashMap<String, String>, Fatal> {
    let wanted: Vec<String> = tree
        .files
        .iter()
        .filter(|p| {
            harness::wanted(p)
                || adapters::PATHS.contains(&p.as_str())
                || HOOK_FILES.contains(&p.as_str())
        })
        .cloned()
        .collect();
    let mut out = HashMap::new();
    for (path, bytes) in (tree.read)(&wanted)? {
        match String::from_utf8(bytes) {
            Ok(t) => {
                out.insert(path, t);
            }
            Err(_) if must_be_text(&path) => {
                return Err(Fatal::Config(format!(
                    "{path}: the file is not UTF-8, and upgrade cannot read it"
                )));
            }
            Err(_) => {}
        }
    }
    Ok(out)
}

/// The files that the harness reads, in path order: (path, text).
fn harness_files(texts: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = texts
        .iter()
        .filter(|(p, _)| harness::wanted(p))
        .map(|(p, t)| (p.clone(), t.clone()))
        .collect();
    out.sort();
    out
}

/// The roles of the level whose agent file has no marker: (role, path).
fn conflicts(existing: &[(String, String)], level: Level) -> Vec<(Role, String)> {
    harness::name_conflicts(existing)
        .into_iter()
        .filter(|(role, _)| level.includes(*role))
        .collect()
}

/// Step 4: the keys of the roles of the level whose agent file has no marker.
/// Pass 2 of the configuration turns them off (D147), so `upgrade` never
/// replaces such a file.
pub fn unmarked_roles(tree: &Tree, old: &OldConfig) -> Result<BTreeSet<String>, Fatal> {
    let Some(h) = &old.harness else {
        return Ok(BTreeSet::new());
    };
    let level = Level::from_name(&h.level).unwrap_or(Level::None);
    let existing = harness_files(&read_texts(tree)?);
    Ok(conflicts(&existing, level)
        .into_iter()
        .map(|(role, _)| role.key().to_string())
        .collect())
}

/// True if the configuration generates the agent file of the role.
fn generates(h: &harness::Harness, role: Role) -> bool {
    h.on(role)
        && match role {
            Role::Advisor => h.cfg(role).agent != Model::None,
            _ => true,
        }
}

/// A conflict that `init` reports as a usage error stops `upgrade` with exit
/// code 1: the plan is valid, a file of the project is in the way, and nothing
/// changed (D136).
fn refused(e: Fatal) -> Fatal {
    match e {
        Fatal::Usage(m) => Fatal::Refused(m),
        other => other,
    }
}

/// The model in the front matter of an agent file.
fn model_of(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return None;
    }
    lines
        .take_while(|l| l.trim_end() != "---")
        .find_map(|l| l.strip_prefix("model:"))
        .map(|m| m.trim().trim_matches(['"', '\'']).to_string())
}

/// Step 7: the adapters and the harness files of the final configuration, as
/// `init` writes them (D148). `from_old` is true if the source is a 0.5 file:
/// then the plan runs as `init --reconfigure` and it removes an unedited file
/// that the configuration does not generate, such as `checker.md`. A source of
/// 0.6.0 runs as plain `init`, and it removes nothing (D147). The check for a
/// file that git ignores is a step of the command (`init::refuse_ignored`).
pub fn plan_files(tree: &Tree, fin: &Config, from_old: bool) -> Result<FilesPlan, Fatal> {
    let texts = read_texts(tree)?;
    let src = |p: &str| texts.get(p).cloned();
    let (chosen, mut changes) = init::adapters_of(&src, fin).map_err(refused)?;
    let mut out = FilesPlan {
        adapters: chosen,
        ..FilesPlan::default()
    };
    if let Some(h) = fin.harness.as_ref().filter(|h| h.level != Level::None) {
        let existing = harness_files(&texts);
        let mode = if from_old {
            Mode::Reconfigure
        } else {
            Mode::Upgrade
        };
        for (role, path) in conflicts(&existing, h.level) {
            if !generates(h, role) {
                out.kept.push(format!(
                    "kept {path}: no marker, so spec-dna did not generate it; the role `{}` writes no file",
                    role.key()
                ));
            }
        }
        if from_old {
            for path in harness::OLD_AGENT_PATHS {
                if texts.get(path).is_some_and(|t| !harness::has_marker(t)) {
                    out.kept.push(format!(
                        "kept {path}: no marker, so spec-dna did not generate it; upgrade leaves it"
                    ));
                }
            }
            if let Some(t) = texts
                .get(".claude/agents/spec-writer.md")
                .filter(|t| harness::has_marker(t))
            {
                out.ended_roles.push((
                    "spec-writer".to_string(),
                    model_of(t).unwrap_or_else(|| "unknown".to_string()),
                ));
            }
        }
        let run =
            init::harness_run(existing, h, Some(h), &[], mode, &mut changes).map_err(refused)?;
        out.kept.extend(run.plan.kept.iter().cloned());
        out.warnings = run.plan.warnings.clone();
        out.request_bytes = run.plan.request_bytes;
    }
    for c in changes {
        if c.remove {
            out.removes.push(c.path.clone());
        } else {
            out.writes.insert(c.path.clone(), c.text);
        }
        out.notes.insert(c.path, c.note);
    }
    out.removes.sort();
    out.kept.sort();
    Ok(out)
}

// --- the plan ---

/// The source of a plan: the reader that the pin and the text select (D134,
/// D137).
pub enum Source {
    /// A 0.4 or 0.5 file: the 0.5 reader.
    Old(Box<OldConfig>),
    /// A 0.6 file: the 0.6 parser.
    Current(Box<Config>),
}

/// Selects the reader of a `spec-dna.toml` text (D134, D137). An error is
/// exit code 3.
pub fn source_of(text: &str) -> Result<Source, Fatal> {
    let file = config::FILE_NAME;
    // The names of 0.6 in `[harness] skills` give exit code 3 in the parser,
    // so `upgrade` renames them first (D4).
    let rename = upgrade_config::rename_skills(text);
    // A list that the rename cannot splice stays: the person changes it.
    if let Some(by_hand) = rename.by_hand.first() {
        return Err(Fatal::Config(format!("{file}: {by_hand}")));
    }
    let text = rename.text.as_str();
    let Some(pin) = config::scan_pin(text) else {
        let why = match Config::parse(text) {
            Err(e) => e,
            Ok(_) => "`spec-dna` is required".to_string(),
        };
        return Err(Fatal::Config(format!("{file}: {why}")));
    };
    match config::pin_class(&pin, VERSION) {
        PinClass::Invalid | PinClass::BeforeOldest | PinClass::AfterBinary => {
            Err(Fatal::Config(config::pin_mismatch_of_text(&pin, text)))
        }
        PinClass::Model05 => OldConfig::read(text)
            .map(|o| Source::Old(Box::new(o)))
            .map_err(|e| Fatal::Config(format!("{file}: {e}"))),
        // A 0.5 text with a 0.6 pin: the 0.5 docs told a person to set the
        // pin by hand (D137).
        PinClass::Model06 | PinClass::Binary => match Config::parse(text) {
            Ok(c) => Ok(Source::Current(Box::new(c))),
            Err(e) => OldConfig::read(text)
                .map(|o| Source::Old(Box::new(o)))
                .map_err(|_| Fatal::Config(format!("{file}: {e}"))),
        },
    }
}

/// What the caller gives to `plan`.
#[derive(Clone, Debug)]
pub struct PlanOpts {
    /// `--layout`: also move the folders of the other 0.5 namespaces.
    pub layout: bool,
    /// The new pin: the version of the binary.
    pub pin: String,
    /// The sums of `spec-dna-pin.toml` of that version (`--pin-file`).
    pub sums: Option<Vec<(String, String)>>,
    /// `--areas`: the id of each 0.5 document and its area, sorted by id. It
    /// needs `layout` and a 0.5 source (D45, D48).
    pub areas: Vec<(String, String)>,
}

/// The plan of `upgrade` over one tree. It is the result of a pure function.
/// The proof and the report are separate steps (`prove`, `report`), because
/// the base plan of `check` needs neither.
pub struct Plan {
    /// The old pin.
    pub from: String,
    /// The new pin.
    pub to: String,
    /// The new text of `spec-dna.toml`, or `None` if it does not change.
    pub config: Option<String>,
    /// The text of the new `spec-dna.toml`, also if it did not change.
    pub config_text: String,
    pub sums_added: bool,
    /// The configuration of the new text.
    pub fin: Config,
    pub classified: Classified,
    /// The front matter, the moves and the documents to write.
    pub docs: DocsPlan,
    /// The adapters and the harness files.
    pub files: FilesPlan,
    /// The final path of each file to write, with its new text: the
    /// configuration, the documents, the adapters and the harness files.
    pub writes: BTreeMap<String, String>,
    /// The files to remove, in path order.
    pub removes: Vec<String>,
    /// The source is a 0.4 or 0.5 file.
    pub from_old: bool,
    /// The edits of a 0.6 `spec-dna.toml` beyond the pin (D15, D34), as lines
    /// for the report.
    pub edits: Vec<String>,
    /// The edits of a 0.6 `spec-dna.toml` that the plan cannot make: the person
    /// makes them by hand. The report gives them as warnings.
    pub by_hand: Vec<String>,
}

impl Plan {
    /// True if the plan changes nothing.
    pub fn is_empty(&self) -> bool {
        self.config.is_none()
            && self.writes.is_empty()
            && self.docs.moves.is_empty()
            && self.removes.is_empty()
    }

    /// The plan as a change of the snapshot of its tree: the moves, the
    /// removes and the new text of each write. `check` lays it over the
    /// snapshot of the base commit (D155).
    pub fn overlay(&self) -> repo::Overlay {
        repo::Overlay {
            moved: self
                .docs
                .moves
                .iter()
                .map(|m| (m.from.clone(), m.to.clone()))
                .collect(),
            removed: self.removes.clone(),
            texts: self
                .writes
                .iter()
                .map(|(p, t)| (p.clone(), t.clone().into_bytes()))
                .collect(),
        }
    }
}

/// The plan of the base commit of `check` (D154): the tree of `rev`, with no
/// `--layout`, the pin and the sums of the head. `label` is the revision that
/// the person gave, for the messages. A stop of the plan is exit code 3 here,
/// whatever its code in `upgrade`, because `check` can not ask the person to
/// change the base.
pub fn plan_base(git: &Git, rev: &str, label: &str, head: &Config) -> Result<Plan, Fatal> {
    let data = TreeData::from_git(git, rev)?;
    let opts = PlanOpts {
        layout: false,
        pin: head.pin.clone(),
        sums: head.sha256.clone(),
        areas: Vec::new(),
    };
    data.with(|tree| {
        plan(tree, &opts).map_err(|e| match e {
            Fatal::Config(m) | Fatal::Refused(m) => {
                let pin = tree_text(tree, config::FILE_NAME)
                    .ok()
                    .and_then(|t| config::scan_pin(&t))
                    .unwrap_or_default();
                Fatal::Config(format!(
                    "the base `{label}` pins `{pin}`, and upgrade cannot plan it: {m}"
                ))
            }
            other => other,
        })
    })
}

/// True if the tree has the path. The list of files is sorted.
fn in_tree(tree: &Tree, path: &str) -> bool {
    tree.files
        .binary_search_by(|f| f.as_str().cmp(path))
        .is_ok()
}

fn tree_text(tree: &Tree, path: &str) -> Result<String, Fatal> {
    let bytes = (tree.read)(&[path.to_string()])?
        .remove(path)
        .ok_or_else(|| Fatal::Io(format!("{path}: not in the tree")))?;
    String::from_utf8(bytes).map_err(|_| {
        Fatal::Config(format!(
            "{path}: the file is not UTF-8, and upgrade cannot read it"
        ))
    })
}

/// The 0.5 form of a 0.6 file that has only a pin to change: the plan edits the
/// pin and the sums and nothing else.
fn pin_only(cfg: &Config, text: &str) -> OldConfig {
    let table: toml::Table = text.parse().unwrap_or_default();
    OldConfig {
        pin: cfg.pin.clone(),
        has_sums: table.contains_key("spec-dna-sha256"),
        types: None,
        roots: Vec::new(),
        budgets: Vec::new(),
        conventions: Vec::new(),
        harness: None,
    }
}

/// The text of the stop for `--areas` on a 0.6 source (D48).
const AREAS_ON_06: &str = "`--areas` moves documents of a 0.5 project; in a 0.6 project, run `spec-dna mv` in a pull request of its own";

/// The plan of `upgrade` over a tree (design 0.6.0, "The steps of `plan`"). It
/// reads the tree and writes nothing. A stop of the plan is `Fatal::Refused`
/// (exit code 1); a text that the plan cannot edit is `Fatal::Config` (exit
/// code 3).
pub fn plan(tree: &Tree, opts: &PlanOpts) -> Result<Plan, Fatal> {
    let original = tree_text(tree, config::FILE_NAME).map_err(|e| match e {
        Fatal::Io(_) => Fatal::Config(format!(
            "no {} in the tree; upgrade moves a project that has one",
            config::FILE_NAME
        )),
        other => other,
    })?;
    // The skill names of 0.6 change before the first parse (D4).
    let rename = upgrade_config::rename_skills(&original);
    let text = rename.text.clone();
    let copts = ConfigOpts {
        pin: opts.pin.clone(),
        sums: opts.sums.clone(),
    };
    let mut repairs = upgrade_config::Repair::default();
    let (from, final_text, sums_added, classified, from_old) = match source_of(&text)? {
        Source::Old(old) => {
            let p1 = upgrade_config::pass1(&text, &old, &copts)?;
            let cfg1 = Config::parse(&p1.text).map_err(|e| Fatal::Config(e.to_string()))?;
            let classified = classify(
                tree,
                &Basis {
                    cfg: &cfg1,
                    old: Some(&old),
                    layout: opts.layout,
                },
            )?;
            let mut facts = classified.facts(&cfg1);
            facts.unmarked_roles = unmarked_roles(tree, &old)?;
            facts.areas = areas_of(&opts.areas);
            let cp = upgrade_config::pass2(&p1, &old, &facts, &copts)?;
            (old.pin.clone(), cp.text, cp.sums_added, classified, true)
        }
        Source::Current(cfg) => {
            if !opts.areas.is_empty() {
                return Err(Fatal::Usage(AREAS_ON_06.to_string()));
            }
            // The pin changes, or `--pin-file` gives the table that the file
            // lacks (D135, O4). A table that the file has stays, if the pin
            // stays: the sums are the trust of the project.
            let no_table = !text
                .parse::<toml::Table>()
                .unwrap_or_default()
                .contains_key("spec-dna-sha256");
            let (t, added) = if cfg.pin == opts.pin && !(opts.sums.is_some() && no_table) {
                (text.clone(), false)
            } else {
                let p1 = upgrade_config::pass1(&text, &pin_only(&cfg, &text), &copts)?;
                (p1.text, p1.sums_added)
            };
            let fin = Config::parse(&t).map_err(|e| Fatal::Config(e.to_string()))?;
            // The 2 edits of a 0.6 file beyond the pin (D15, D34).
            repairs = upgrade_config::repair_current(&t, &fin)?;
            let t = repairs.text.clone();
            let fin = Config::parse(&t).map_err(|e| Fatal::Config(e.to_string()))?;
            let classified = classify(
                tree,
                &Basis {
                    cfg: &fin,
                    old: None,
                    layout: opts.layout,
                },
            )?;
            (cfg.pin.clone(), t, added, classified, false)
        }
    };
    let fin = Config::parse(&final_text).map_err(|e| Fatal::Config(e.to_string()))?;
    let docs = plan_docs(tree, &classified, &fin, opts.layout, &opts.areas)?;
    let files = plan_files(tree, &fin, from_old)?;
    // An instruction file can be a document too (D231). The 2 plans would
    // write one path, and upgrade does not merge 2 texts, so it stops.
    let moved: BTreeSet<&str> = docs
        .moves
        .iter()
        .flat_map(|m| [m.from.as_str(), m.to.as_str()])
        .collect();
    let mut clash: Vec<&str> = files
        .writes
        .keys()
        .map(String::as_str)
        .chain(files.removes.iter().map(String::as_str))
        .filter(|p| docs.writes.contains_key(*p) || moved.contains(p))
        .collect();
    clash.sort();
    clash.dedup();
    if !clash.is_empty() {
        return Err(Fatal::Refused(format!(
            "{} is a document, and upgrade also changes it as an adapter or a harness file; upgrade does not merge the 2 changes. Move the file out of the roots of spec-dna.toml (or add it to `exclude`), commit the change, then run `spec-dna upgrade` again",
            clash.join(", ")
        )));
    }
    let mut writes = docs.writes.clone();
    for (path, new) in &files.writes {
        let old = tree
            .files
            .iter()
            .any(|f| f == path)
            .then(|| tree_text(tree, path))
            .transpose()?;
        if old.as_deref() != Some(new.as_str()) {
            writes.insert(path.clone(), new.clone());
        }
    }
    let removes: Vec<String> = files
        .removes
        .iter()
        .filter(|p| in_tree(tree, p))
        .cloned()
        .collect();
    let config = (final_text != original).then(|| final_text.clone());
    if let Some(t) = &config {
        writes.insert(config::FILE_NAME.to_string(), t.clone());
    }
    Ok(Plan {
        from,
        to: opts.pin.clone(),
        config,
        config_text: final_text,
        sums_added,
        fin,
        classified,
        docs,
        files,
        writes,
        removes,
        from_old,
        edits: [rename.edits, repairs.edits].concat(),
        by_hand: [rename.by_hand, rename.warnings, repairs.by_hand].concat(),
    })
}

/// The proof of a plan (D149). It does not run in the base plan of `check`.
pub fn prove(tree: &Tree, plan: &Plan) -> Result<Proof, Fatal> {
    upgrade_proof::prove(
        tree,
        &Planned {
            classified: &plan.classified,
            docs: &plan.docs,
            config: &plan.config_text,
        },
    )
}

// --- the report ---

/// The changes of behavior that `upgrade` reports and the CHANGELOG records
/// (design 0.6.0, "Upgrade").
pub const BEHAVIOR: [&str; 5] = [
    "A former `note` is now `research`, with `keep = \"forever\"`. SD202 allows the deletion of a document that has `was-type: note` or `was-type: change`, as 0.5 did. This is the only exemption in a `forever` layer.",
    "`context` listed incidents and notes. It now counts them in one line.",
    "The context order follows the levels of the graph. `foundation` comes before `rules`.",
    "The `--json` field `name` of SD405 and SD611 changed.",
    "0.6.0 renames types, keys and commands, so it is a minor release that breaks compatibility. Semantic Versioning allows this before 1.0.",
];

/// What `upgrade` tells the person besides the files.
#[derive(Debug, Default)]
pub struct Report {
    /// Each `done` or `rejected` change that no document references, as a
    /// proposal to delete: its path in the new tree.
    pub proposals: Vec<String>,
    /// Each decision with no `status` in its front matter: its path.
    pub no_status: Vec<String>,
    /// Each locked decision that cites a migrated change: the path of the
    /// decision, and the path of the change in the new tree.
    pub locked_cites: Vec<(String, String)>,
    /// The harness files that `upgrade` keeps, as lines of text.
    pub kept: Vec<String>,
    /// Each role that ended, with its model.
    pub ended_roles: Vec<(String, String)>,
    /// The count of each error code of `check --all` on the new tree.
    pub errors: Vec<(String, usize)>,
    /// `--pin-file` added the table `[spec-dna-sha256]` (O4).
    pub sums_added: bool,
    /// The notes of the text output: what the person can do beside the files.
    pub notes: Vec<String>,
    /// The links of a file that is no document to a moved file or folder. The
    /// plan does not rewrite them (D207).
    pub outside_links: Vec<(String, String)>,
    /// The links to a folder that changes its documents (D216).
    pub folder_links: Vec<(String, String)>,
    /// The warnings of the configuration of the harness, and the edits of
    /// `spec-dna.toml` that the person makes by hand.
    pub warnings: Vec<String>,
    /// The edits of a 0.6 `spec-dna.toml` beyond the pin (D15, D34).
    pub edits: Vec<String>,
    /// The source is a 0.4 or 0.5 file: the report lists the changes of
    /// behavior (D56).
    pub from_old: bool,
}

/// The tree after the plan: the corpus of the new tree and the diagnostics of
/// `check --all` on it (D153).
struct After {
    corpus: Corpus,
    diags: Vec<Diag>,
}

/// The text of each file of the new tree that `want` selects: the text that the
/// plan writes, else the text of the tree at the old path.
fn texts_after(
    tree: &Tree,
    plan: &Plan,
    files: &[String],
    want: impl Fn(&str) -> bool,
) -> Result<HashMap<String, String>, Fatal> {
    let old_path: HashMap<&str, &str> = plan
        .docs
        .moves
        .iter()
        .map(|m| (m.to.as_str(), m.from.as_str()))
        .collect();
    let mut out = HashMap::new();
    let mut read = Vec::new();
    for p in files.iter().filter(|p| want(p)) {
        match plan.writes.get(p) {
            Some(t) => {
                out.insert(p.clone(), t.clone());
            }
            None => read.push(old_path.get(p.as_str()).copied().unwrap_or(p).to_string()),
        }
    }
    let new_path: HashMap<&str, &str> = plan
        .docs
        .moves
        .iter()
        .map(|m| (m.from.as_str(), m.to.as_str()))
        .collect();
    for (p, bytes) in (tree.read)(&read)? {
        let end = new_path.get(p.as_str()).copied().unwrap_or(&p).to_string();
        out.insert(end, String::from_utf8_lossy(&bytes).into_owned());
    }
    Ok(out)
}

fn after(tree: &Tree, plan: &Plan) -> Result<After, Fatal> {
    let fin = &plan.fin;
    let moved: HashMap<&str, &str> = plan
        .docs
        .moves
        .iter()
        .map(|m| (m.from.as_str(), m.to.as_str()))
        .collect();
    let mut set: BTreeSet<String> = tree
        .files
        .iter()
        .map(|f| moved.get(f.as_str()).map_or(f.clone(), |t| t.to_string()))
        .collect();
    set.extend(plan.writes.keys().cloned());
    for r in &plan.removes {
        set.remove(r);
    }
    let files: Vec<String> = set.into_iter().collect();
    let docs: Vec<(String, Arc<ParsedDoc>)> = plan
        .classified
        .texts
        .iter()
        .map(|(p, t)| {
            let end = moved.get(p.as_str()).copied().unwrap_or(p);
            let text = plan.docs.writes.get(end).unwrap_or(t);
            (end.to_string(), Arc::new(parse(text)))
        })
        .collect();
    let mut corpus = Corpus::build(fin, files.clone(), docs);
    // The `covers:` tags of the test files, as `evidence::load` reads them.
    let tests = texts_after(tree, plan, &files, |p| fin.is_test_file(p))?;
    let mut covers = Vec::new();
    for p in &files {
        let Some(t) = tests.get(p) else { continue };
        for (i, line) in t.lines().enumerate() {
            for (target, partial) in evidence::tags(line) {
                covers.push(Cover {
                    path: p.clone(),
                    line: i as u32 + 1,
                    target,
                    partial,
                });
            }
        }
    }
    corpus.covers = covers;
    let mut diags = check::gate(fin, None, None, &corpus).head;
    let texts = texts_after(tree, plan, &files, |p| {
        adapters::PATHS.contains(&p) || harness::wanted(p)
    })?;
    let ad: Vec<(&str, String)> = adapters::PATHS
        .iter()
        .filter_map(|p| texts.get(*p).map(|t| (*p, t.clone())))
        .collect();
    diags.extend(check::apply_severity(
        fin,
        adapters::check(&ad, &adapters::project_types(&fin.layers, &fin.types)),
    ));
    if let Some(h) = fin.harness.as_ref().filter(|h| h.level != Level::None) {
        let mut hf: Vec<(String, String)> = texts
            .iter()
            .filter(|(p, _)| harness::wanted(p))
            .map(|(p, t)| (p.clone(), t.clone()))
            .collect();
        hf.sort();
        diags.extend(check::apply_severity(fin, harness::check(h, &hf)));
    }
    Ok(After { corpus, diags })
}

/// The report of a plan and its proof (design 0.6.0, "Detail of ADR 0009:
/// exit codes, `--json` and the report").
pub fn report(tree: &Tree, plan: &Plan, proof: &Proof) -> Result<Report, Fatal> {
    let after = after(tree, plan)?;
    let c = &after.corpus;
    let dest: HashMap<&str, &str> = plan
        .docs
        .moves
        .iter()
        .map(|m| (m.from.as_str(), m.to.as_str()))
        .collect();
    // The documents that another document references: a link, `[[id]]`, or
    // an id field, also in locked text.
    let mut referenced: HashSet<u32> = HashSet::new();
    for (i, d) in c.docs.iter().enumerate() {
        let i = i as u32;
        let mut hit = |t: u32| {
            if t != i {
                referenced.insert(t);
            }
        };
        for r in &d.parsed.refs {
            match r.kind {
                RefKind::Wiki => {
                    if let Res::Found(t, _, _) = c.resolve(&r.target) {
                        hit(t);
                    }
                }
                RefKind::Link | RefKind::Image => match c.resolve_link(&d.path, &r.target) {
                    LinkRes::File(_, Some(t), _) | LinkRes::Dir(Some(t)) => hit(t),
                    _ => {}
                },
            }
        }
        let ids = d
            .supersedes
            .iter()
            .map(|(t, _)| t)
            .chain(d.depends_on.iter().map(|(t, _)| t))
            .chain(d.applies_to.iter())
            .chain(d.from.iter().map(|(t, _)| t));
        for t in ids {
            if let Res::Found(t, _, _) = c.resolve(t) {
                hit(t);
            }
        }
    }
    let mut proposals = Vec::new();
    for cl in &plan.classified.docs {
        // A change that is a top file is a document of its root, no proposal.
        let proposal = match cl.fate {
            _ if cl.top.is_some() => false,
            Fate::Done => true,
            Fate::Move => cl.old_status.as_deref() == Some("rejected"),
            Fate::Keep => false,
        };
        if !proposal {
            continue;
        }
        let end = dest.get(cl.path.as_str()).copied().unwrap_or(&cl.path);
        if c.by_path.get(end).is_some_and(|i| !referenced.contains(i)) {
            proposals.push(end.to_string());
        }
    }
    proposals.sort();
    let mut no_status: Vec<String> = c
        .docs
        .iter()
        .filter(|d| d.ty == DocType::Decision && !d.type_failed && d.moved_to.is_none())
        // spec-dna reads the field `status` of the front matter, not a section
        // `Status`. A field with an empty value counts as missing.
        .filter(|d| {
            !d.parsed
                .front_matter
                .as_ref()
                .and_then(|f| f.get("status"))
                .is_some_and(
                    |(v, _)| !matches!(v, crate::frontmatter::Value::Str(s) if s.trim().is_empty()),
                )
        })
        .map(|d| d.path.clone())
        .collect();
    no_status.sort();
    // The locked decisions that cite a migrated change. The rule is the rule
    // of SD611, but it does not read the severity: a project that turns SD611
    // off still gets the item.
    let migrated: HashSet<&str> = plan
        .docs
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Change)
        .map(|m| m.to.as_str())
        .collect();
    let mut locked_cites: Vec<(String, String)> = Vec::new();
    for d in &c.docs {
        let Some(editable) = check::editable_chunks(&plan.fin, d) else {
            continue;
        };
        let mut hit = |t: u32| {
            let target = &c.docs[t as usize];
            if migrated.contains(target.path.as_str()) {
                locked_cites.push((d.path.clone(), target.path.clone()));
            }
        };
        for r in &d.parsed.refs {
            if !check::in_locked_text(d, &editable, r.line) {
                continue;
            }
            match r.kind {
                RefKind::Wiki => {
                    if let Res::Found(t, _, _) = c.resolve(&r.target) {
                        hit(t);
                    }
                }
                RefKind::Link => {
                    if let LinkRes::File(_, Some(t), _) = c.resolve_link(&d.path, &r.target) {
                        hit(t);
                    }
                }
                RefKind::Image => {}
            }
        }
        for (t, _) in &d.depends_on {
            if let Res::Found(t, _, _) = c.resolve(t) {
                hit(t);
            }
        }
    }
    locked_cites.sort();
    locked_cites.dedup();
    let mut errors: BTreeMap<String, usize> = BTreeMap::new();
    for d in after.diags.iter().filter(|d| d.severity == Severity::Error) {
        *errors.entry(d.code.code().to_string()).or_insert(0) += 1;
    }
    Ok(Report {
        proposals,
        no_status,
        locked_cites,
        kept: plan
            .docs
            .kept
            .iter()
            .chain(&plan.files.kept)
            .cloned()
            .collect(),
        ended_roles: plan.files.ended_roles.clone(),
        errors: errors.into_iter().collect(),
        sums_added: plan.sums_added,
        outside_links: proof.notes.outside_links.clone(),
        folder_links: proof.notes.folder_links.clone(),
        warnings: plan
            .files
            .warnings
            .iter()
            .chain(&plan.by_hand)
            .cloned()
            .collect(),
        edits: plan.edits.clone(),
        from_old: plan.from_old,
        notes: init::typos_note(&|p| {
            if in_tree(tree, p) {
                tree_text(tree, p).ok()
            } else {
                None
            }
        })
        .into_iter()
        .collect(),
    })
}

// --- the command ---

/// What the command line gives to `run`.
#[derive(Clone, Debug, Default)]
pub struct RunOpts {
    pub dry_run: bool,
    pub json: bool,
    pub layout: bool,
    /// `--pin-file`: a path from the current folder.
    pub pin_file: Option<String>,
    /// `--areas`: a path from the current folder (D45).
    pub areas: Option<String>,
}

/// Reads `--pin-file` (D135): the key `spec-dna` must be the version of the
/// binary, and the table `[spec-dna-sha256]` must be valid for it. An error is
/// exit code 3.
pub fn read_pin_file(path: &str) -> Result<Vec<(String, String)>, Fatal> {
    let bad = |m: String| Fatal::Config(format!("--pin-file {path}: {m}"));
    let text = std::fs::read_to_string(path).map_err(|e| bad(format!("cannot read it ({e})")))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| bad(e.to_string().trim().to_string()))?;
    match table.get("spec-dna").and_then(toml::Value::as_str) {
        Some(v) if v == VERSION => {}
        Some(v) => {
            return Err(bad(format!(
                "the file pins version {v}, but this binary is {VERSION}; use `spec-dna-pin.toml` of release v{VERSION}"
            )));
        }
        None => return Err(bad("the key `spec-dna` is missing".into())),
    }
    match config::sha256_table(table.get("spec-dna-sha256"), VERSION) {
        Ok(Some(sums)) if !sums.is_empty() => Ok(sums),
        Ok(_) => Err(bad(
            "the table `[spec-dna-sha256]` is missing or empty; use `spec-dna-pin.toml` of the release".into(),
        )),
        Err(e) => Err(bad(e)),
    }
}

/// Reads `--areas` (D45): the table `[areas]`, whose keys are document ids and
/// whose values are areas. The result is sorted by id. An error is exit code 3.
/// The check of the names is a stop of the plan (D46).
pub fn read_areas_file(path: &str) -> Result<Vec<(String, String)>, Fatal> {
    let bad = |m: String| Fatal::Config(format!("--areas {path}: {m}"));
    let text = std::fs::read_to_string(path).map_err(|e| bad(format!("cannot read it ({e})")))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| bad(e.to_string().trim().to_string()))?;
    let Some(areas) = table.get("areas").and_then(toml::Value::as_table) else {
        return Err(bad(
            "the table `[areas]` is missing; each key is a document id and each value is an area"
                .into(),
        ));
    };
    areas
        .iter()
        .map(|(id, v)| match v.as_str() {
            Some(area) => Ok((id.clone(), area.to_string())),
            None => Err(bad(format!("the area of `{id}` is not a string"))),
        })
        .collect()
}

/// The paths that make the tree not clean, as a message (D151).
fn unclean_message(paths: &[String]) -> String {
    let shown: Vec<&str> = paths.iter().take(5).map(String::as_str).collect();
    let more = if paths.len() > shown.len() {
        format!(" and {} more", paths.len() - shown.len())
    } else {
        String::new()
    };
    format!(
        "the working tree is not clean ({} path{}: {}{more}); stash the changes with `git stash -u`, run `spec-dna upgrade`, commit its paths, then run `git stash pop`. upgrade changes nothing",
        paths.len(),
        if paths.len() == 1 { "" } else { "s" },
        shown.join(", ")
    )
}

/// A file in the place of a folder of a path stops the write. A part of the
/// path that is a symbolic link is the case of `link_problem`.
fn file_problem(root: &Path, path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let mut cur = root.to_path_buf();
    for (i, part) in parts.iter().enumerate().take(parts.len().saturating_sub(1)) {
        cur.push(part);
        let meta = std::fs::symlink_metadata(&cur).ok()?;
        if meta.file_type().is_symlink() {
            return None;
        }
        if meta.is_file() {
            return Some(format!(
                "`{}` is a file, and upgrade needs a folder there for `{path}`; move or delete the file, commit the change, then run `spec-dna upgrade` again",
                parts[..=i].join("/")
            ));
        }
    }
    None
}

/// A symbolic link in a path stops the write (D146, D203).
fn link_problem(root: &Path, path: &str) -> Option<String> {
    if let Some(m) = file_problem(root, path) {
        return Some(m);
    }
    finish::path_problem(root, path).map(|m| {
        let cut = m.split("; `finish`").next().unwrap_or(&m).to_string();
        format!(
            "{cut}; upgrade does not write or move through a link. Replace the link, commit the change, then run `spec-dna upgrade` again"
        )
    })
}

/// One line of the list of files: a move, a write, a remove or a kept file.
struct Entry {
    path: String,
    action: &'static str,
    from: Option<String>,
    note: String,
    diff: String,
    /// For a kept file: the whole line of the harness.
    line: Option<String>,
}

impl Entry {
    fn json(&self) -> String {
        let mut o = Obj::new()
            .str("path", &self.path)
            .str("action", self.action)
            .str("note", &self.note)
            .str("diff", &self.diff);
        if let Some(f) = &self.from {
            o = o.str("from", f);
        }
        o.end()
    }
}

/// The path of a `kept` line of the harness: `kept <path>: <reason>` or
/// `kept <path>:<line>: <reason>`.
fn kept_path(line: &str) -> (String, String) {
    let rest = line.strip_prefix("kept ").unwrap_or(line);
    let (head, note) = rest.split_once(": ").unwrap_or((rest, ""));
    let path = match head.rsplit_once(':') {
        Some((p, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => p,
        _ => head,
    };
    (path.to_string(), note.to_string())
}

/// The entries of the list of files, in the order: moves, writes, removes,
/// kept files. The diff of each is the diff against the tree of `HEAD`.
fn entries(tree: &Tree, plan: &Plan) -> Result<Vec<Entry>, Fatal> {
    let mut want: Vec<String> = plan.removes.clone();
    want.extend(plan.writes.keys().filter(|p| in_tree(tree, p)).cloned());
    let old = (tree.read)(&want)?;
    let old_text =
        |p: &str| -> Option<String> { old.get(p).map(|b| String::from_utf8_lossy(b).into_owned()) };
    let mut out = Vec::new();
    let to_set: HashSet<&str> = plan.docs.moves.iter().map(|m| m.to.as_str()).collect();
    for m in &plan.docs.moves {
        let before = plan.classified.texts.get(&m.from);
        let diff = match (before, plan.writes.get(&m.to)) {
            (Some(b), Some(a)) => init::unified_diff(&m.to, Some(b), a, false).replacen(
                &format!("--- a/{}\n", m.to),
                &format!("--- a/{}\n", m.from),
                1,
            ),
            _ => String::new(),
        };
        out.push(Entry {
            path: m.to.clone(),
            action: "move",
            from: Some(m.from.clone()),
            note: match m.kind {
                MoveKind::Layout => "the folder has its 0.6 name".to_string(),
                MoveKind::Change => "the change becomes an intent".to_string(),
                MoveKind::Top => "the document is the top file of its root".to_string(),
                MoveKind::Area => "the document is in its area".to_string(),
            },
            diff,
            line: None,
        });
    }
    for (p, t) in &plan.writes {
        if to_set.contains(p.as_str()) {
            continue;
        }
        let note = plan
            .files
            .notes
            .get(p)
            .cloned()
            .unwrap_or_else(|| match p.as_str() {
                config::FILE_NAME => "the new configuration".to_string(),
                _ => "front matter, links or aliases".to_string(),
            });
        out.push(Entry {
            path: p.clone(),
            action: "write",
            from: None,
            note,
            diff: init::unified_diff(p, old_text(p).as_deref(), t, false),
            line: None,
        });
    }
    for p in &plan.removes {
        out.push(Entry {
            path: p.clone(),
            action: "remove",
            from: None,
            note: plan.files.notes.get(p).cloned().unwrap_or_default(),
            diff: init::unified_diff(p, old_text(p).as_deref(), "", true),
            line: None,
        });
    }
    for line in plan.docs.kept.iter().chain(&plan.files.kept) {
        let (path, note) = kept_path(line);
        out.push(Entry {
            path,
            action: "keep",
            from: None,
            note,
            diff: String::new(),
            line: Some(line.clone()),
        });
    }
    Ok(out)
}

/// A path for a shell word: single quotes only where it needs them.
fn shell_word(p: &str) -> String {
    if !p.is_empty()
        && p.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._/-+@%=,:".contains(&b))
    {
        p.to_string()
    } else {
        format!("'{}'", p.replace('\'', "'\\''"))
    }
}

/// The paths for `git add -A --`: each path that the write changes. A move
/// has only its new path: `git mv` staged the rename, and git knows no pathspec
/// for the old path any more.
fn add_paths(plan: &Plan) -> Vec<String> {
    let mut set: BTreeSet<String> = plan.writes.keys().cloned().collect();
    set.extend(plan.removes.iter().cloned());
    set.extend(plan.docs.moves.iter().map(|m| m.to.clone()));
    set.into_iter().collect()
}

fn proof_json(p: &Proof) -> String {
    Obj::new()
        .bool("ok", p.ok())
        .num("ids", p.ids)
        .num("requirements", p.requirements)
        .num("sections", p.sections)
        .num("types", p.types)
        .strs("failures", &p.failures)
        .end()
}

fn report_json(r: &Report) -> String {
    let pairs = |l: &[(String, String)], a: &str, b: &str| {
        json::array(l.iter().map(|(x, y)| Obj::new().str(a, x).str(b, y).end()))
    };
    Obj::new()
        .strs("proposals", &r.proposals)
        .strs("no_status", &r.no_status)
        .raw(
            "locked_cites",
            &pairs(&r.locked_cites, "decision", "change"),
        )
        .strs("kept", &r.kept)
        .raw("ended_roles", &pairs(&r.ended_roles, "role", "model"))
        .strs("behavior", &BEHAVIOR.map(str::to_string))
        .raw(
            "errors",
            &json::array(
                r.errors
                    .iter()
                    .map(|(c, n)| Obj::new().str("code", c).num("count", n).end()),
            ),
        )
        .bool("sums_added", r.sums_added)
        .num("outside_links", r.outside_links.len())
        .num("folder_links", r.folder_links.len())
        .strs("warnings", &r.warnings)
        .strs("edits", &r.edits)
        .end()
}

fn output_json(plan: &Plan, files: &[Entry], proof: &Proof, report: &Report, dry: bool) -> String {
    Obj::new()
        .str("version", VERSION)
        .str("from", &plan.from)
        .str("to", &plan.to)
        .bool("dry_run", dry)
        .raw("files", &json::array(files.iter().map(Entry::json)))
        .raw("proof", &proof_json(proof))
        .raw("report", &report_json(report))
        .end()
}

/// The lines of the report for a person.
fn report_text(r: &Report) -> String {
    let mut out = String::from("report:\n");
    let list = |out: &mut String, title: &str, items: Vec<String>| {
        if !items.is_empty() {
            out.push_str(&format!("  {title} ({}):\n", items.len()));
            for i in items {
                out.push_str(&format!("    {i}\n"));
            }
        }
    };
    list(
        &mut out,
        "proposals to delete: a done or rejected change that no document references",
        r.proposals.clone(),
    );
    list(
        &mut out,
        "decisions with no `status` in the front matter (spec-dna does not read a `Status` section)",
        r.no_status.clone(),
    );
    list(
        &mut out,
        "locked decisions that cite a migrated change",
        r.locked_cites
            .iter()
            .map(|(d, c)| format!("{d} cites {c}"))
            .collect(),
    );
    list(
        &mut out,
        "ended roles",
        r.ended_roles
            .iter()
            .map(|(role, model)| format!("{role} (model: {model})"))
            .collect(),
    );
    let links = |l: &[(String, String)]| -> Vec<String> {
        let mut v: Vec<String> = l
            .iter()
            .take(5)
            .map(|(a, b)| format!("{a} -> {b}"))
            .collect();
        if l.len() > 5 {
            v.push(format!("and {} more", l.len() - 5));
        }
        v
    };
    list(
        &mut out,
        "links in files that are no documents, to a moved file or folder (upgrade did not rewrite them)",
        links(&r.outside_links),
    );
    list(
        &mut out,
        "links to a folder whose documents changed (upgrade did not rewrite them)",
        links(&r.folder_links),
    );
    list(
        &mut out,
        "edits of spec-dna.toml beyond the pin",
        r.edits.clone(),
    );
    if r.sums_added {
        out.push_str("  the table `[spec-dna-sha256]` is new: --pin-file added it\n");
    }
    for w in &r.warnings {
        out.push_str(&format!("  warning: {w}\n"));
    }
    let errors: Vec<String> = r.errors.iter().map(|(c, n)| format!("{c} x{n}")).collect();
    out.push_str(&format!(
        "  errors of `check --all` on the new tree: {}\n",
        if errors.is_empty() {
            "none".to_string()
        } else {
            errors.join(", ")
        }
    ));
    // The changes of behavior are the changes of 0.5 to 0.6 (D56).
    if r.from_old {
        out.push_str("  changes of behavior:\n");
        for b in BEHAVIOR {
            out.push_str(&format!("    - {b}\n"));
        }
    }
    out
}

fn output_text(plan: &Plan, files: &[Entry], proof: &Proof, report: &Report, dry: bool) -> String {
    let mut out = String::new();
    for e in files {
        let verb = match (e.action, dry) {
            ("move", false) => "moved",
            ("move", true) => "would move",
            ("write", false) => "wrote",
            ("write", true) => "would write",
            ("remove", false) => "removed",
            ("remove", true) => "would remove",
            _ => "",
        };
        match (&e.from, e.action) {
            (Some(from), _) => out.push_str(&format!("{verb} {from} -> {}\n", e.path)),
            (None, "keep") => {
                out.push_str(e.line.as_deref().unwrap_or("kept"));
                out.push('\n');
            }
            (None, _) => out.push_str(&format!("{verb} {}\n", e.path)),
        }
        if dry {
            out.push_str(&e.diff);
        }
    }
    out.push_str(&format!(
        "proof: ok ({} ids, {} requirements, {} sections, {} types)\n",
        proof.ids, proof.requirements, proof.sections, proof.types
    ));
    out.push_str(&report_text(report));
    let paths: Vec<String> = add_paths(plan)
        .iter()
        .map(|p| shell_word(&format!(":/{p}")))
        .collect();
    out.push_str(&format!("to commit: git add -A -- {}\n", paths.join(" ")));
    if let Some(next) = next_line(plan, report) {
        out.push_str(next);
        out.push('\n');
    }
    for n in &report.notes {
        out.push_str(&format!("note: {n}\n"));
    }
    out.push_str(&if dry {
        "spec-dna upgrade: dry run; nothing written\n".to_string()
    } else {
        format!(
            "spec-dna upgrade: {} path{} changed; nothing is committed\n",
            add_paths(plan).len(),
            if add_paths(plan).len() == 1 { "" } else { "s" }
        )
    });
    out
}

/// The line `next:` after `to commit:`. SD503 does not count `spec-dna.toml`
/// when only its pin and its table `[spec-dna-sha256]` change to a later
/// version (ADR 0019), so the pull request can also hold other work. A 0.4 or
/// 0.5 file is rewritten, and a 0.6 file can have edits of the plan, so the
/// commit goes in a pull request of its own. A plan that leaves
/// `spec-dna.toml` as it is has no line.
fn next_line(plan: &Plan, report: &Report) -> Option<&'static str> {
    plan.config.as_ref()?;
    let later = matches!(
        (config::Version::parse(&plan.from), config::Version::parse(&plan.to)),
        (Some(from), Some(to)) if from < to
    );
    let pin_only = later && !plan.from_old && report.edits.is_empty();
    Some(if pin_only {
        "next: commit the paths above in one commit of their own. Only the pin of spec-dna.toml changes, so the pull request can also hold other work (SD503 does not count a pin that moves to a later version)."
    } else {
        "next: commit the paths above in one commit, in a pull request of its own: spec-dna.toml has edits beyond the pin (SD503)."
    })
}

const RESTORE: &str = "to restore the tree, run `git reset --hard` and `git clean -fd`";

/// The write step (D150): each `git mv`, then each write, then each remove. An
/// error after the first `git mv` is exit code 4 with the restore commands.
fn write_all(repo: &Repo, plan: &Plan) -> Result<(), Fatal> {
    let fail = |what: String| Fatal::Io(format!("{what}; the write stopped half way, {RESTORE}"));
    for m in &plan.docs.moves {
        if let Some(dir) = repo.root.join(&m.to).parent() {
            std::fs::create_dir_all(dir).map_err(|e| fail(format!("{}: {e}", dir.display())))?;
        }
        repo.git.mv(&m.from, &m.to).map_err(fail)?;
    }
    for (path, text) in &plan.writes {
        let full = repo.root.join(path);
        if let Some(dir) = full.parent() {
            std::fs::create_dir_all(dir).map_err(|e| fail(format!("{}: {e}", dir.display())))?;
        }
        let mode = std::fs::metadata(&full).map(|m| m.permissions()).ok();
        mv::write(&full, text).map_err(|e| fail(e.message().to_string()))?;
        if let Some(mode) = mode {
            let _ = std::fs::set_permissions(&full, mode);
        }
    }
    for path in &plan.removes {
        match std::fs::remove_file(repo.root.join(path)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(fail(format!("{path}: {e}"))),
        }
        harness::remove_empty_skill_dir(&repo.root, path);
    }
    // Each folder that a move left empty goes (D32). The folders are no part
    // of the tree of git, so no output lists them, and no error stops the
    // command.
    for m in &plan.docs.moves {
        let root = plan
            .fin
            .root_of(&m.from)
            .map_or("", |r| &plan.fin.roots[r].path);
        mv::remove_empty_parents(repo, root, &m.from);
    }
    Ok(())
}

/// The checks of the write that read the disk or git: a symbolic link in a
/// path (D203), and a file that git ignores. The checks run for `--dry-run`
/// too, so its diff is the diff of a write.
fn check_paths(repo: &Repo, tree: &Tree, plan: &Plan) -> Result<(), Fatal> {
    let mut paths: BTreeSet<&str> = plan.writes.keys().map(String::as_str).collect();
    paths.extend(plan.removes.iter().map(String::as_str));
    for m in &plan.docs.moves {
        paths.insert(&m.from);
        paths.insert(&m.to);
    }
    for p in &paths {
        if let Some(m) = link_problem(&repo.root, p) {
            return Err(Fatal::Refused(m));
        }
    }
    if let Some(h) = plan.fin.harness.as_ref().filter(|h| h.level != Level::None) {
        init::refuse_ignored(repo, h).map_err(refused)?;
    }
    for p in &paths {
        if !in_tree(tree, p) && repo.git.is_ignored(p) {
            return Err(Fatal::Refused(format!(
                "git ignores {p}, so the file cannot be committed; remove the ignore rule, commit the change, then run `spec-dna upgrade` again"
            )));
        }
    }
    Ok(())
}

/// `spec-dna upgrade` (D136, D150 to D153). The phases: the arguments, the
/// repository and the configuration, the clean tree, the plan, the proof, the
/// write, the report. It never commits.
pub fn run(repo: &Repo, o: &RunOpts) -> Result<u8, Fatal> {
    // The configuration, as the working tree has it. A text that neither
    // reader accepts, and a pin that `upgrade` cannot take, are exit code 3
    // before the check of the tree.
    let text = std::fs::read(repo.config_path()).map_err(|_| {
        Fatal::Config(format!(
            "no {} at {}; upgrade moves a project that has one",
            config::FILE_NAME,
            repo::shown(&repo.root)
        ))
    })?;
    let text = String::from_utf8(text)
        .map_err(|_| Fatal::Config(format!("{} is not UTF-8", config::FILE_NAME)))?;
    let source = source_of(&text)?;
    // `--areas` moves documents of a 0.5 project (D45, D48). The check of the
    // arguments is before the check of the file and of the tree.
    if o.areas.is_some() && matches!(source, Source::Current(_)) {
        return Err(Fatal::Usage(AREAS_ON_06.to_string()));
    }
    let sums = o.pin_file.as_deref().map(read_pin_file).transpose()?;
    let areas = o
        .areas
        .as_deref()
        .map(read_areas_file)
        .transpose()?
        .unwrap_or_default();
    let unclean = repo.git.unclean_paths().map_err(Fatal::Io)?;
    if !unclean.is_empty() {
        return Err(Fatal::Refused(unclean_message(&unclean)));
    }
    if repo.head_commit().is_none() {
        return Err(Fatal::Refused(
            "the repository has no commit; commit the project, then run `spec-dna upgrade` again"
                .into(),
        ));
    }
    let data = TreeData::from_git(&repo.git, "HEAD")?;
    let popts = PlanOpts {
        layout: o.layout,
        pin: VERSION.to_string(),
        sums,
        areas,
    };
    data.with(|tree| {
        let plan = plan(tree, &popts)?;
        let proof = if plan.is_empty() {
            Proof::default()
        } else {
            prove(tree, &plan)?
        };
        let report = report(tree, &plan, &proof)?;
        let files = if plan.is_empty() {
            Vec::new()
        } else {
            entries(tree, &plan)?
        };
        if !proof.ok() {
            if o.json {
                println!("{}", output_json(&plan, &files, &proof, &report, o.dry_run));
            }
            return Err(Fatal::Refused(format!(
                "the proof failed, so upgrade wrote nothing:\n  {}",
                proof.failures.join("\n  ")
            )));
        }
        if !plan.is_empty() {
            check_paths(repo, tree, &plan)?;
        }
        if !o.dry_run && !plan.is_empty() {
            write_all(repo, &plan)?;
        }
        if o.json {
            println!("{}", output_json(&plan, &files, &proof, &report, o.dry_run));
        } else if plan.is_empty() {
            println!("spec-dna upgrade: nothing to change");
            for w in &plan.by_hand {
                println!("  warning: {w}");
            }
            for n in &report.notes {
                println!("note: {n}");
            }
        } else {
            print!("{}", output_text(&plan, &files, &proof, &report, o.dry_run));
        }
        Ok(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters;
    use crate::diag::Code;
    use crate::harness::{self, Level, Role};
    use crate::upgrade_config;
    use crate::upgrade_docs::{Basis, TreeData, classify, tests::opts};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
    }

    fn walk(dir: &Path, rel: &str, out: &mut HashMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().to_string();
            let path = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), &path, out);
            } else {
                out.insert(path, std::fs::read(e.path()).unwrap());
            }
        }
    }

    /// `v05/` with the overlays on it, in order. A `spec-dna.toml.append` goes
    /// to the end of the configuration (the helper `Repo::from_fixture`).
    fn blobs_of(overlays: &[&str]) -> HashMap<String, Vec<u8>> {
        let mut blobs = HashMap::new();
        walk(&fixtures().join("v05"), "", &mut blobs);
        for o in overlays {
            let mut more = HashMap::new();
            walk(&fixtures().join(o), "", &mut more);
            if let Some(a) = more.remove("spec-dna.toml.append") {
                blobs.get_mut("spec-dna.toml").unwrap().extend(a);
            }
            blobs.extend(more);
        }
        blobs
    }

    fn text(blobs: &HashMap<String, Vec<u8>>, path: &str) -> String {
        String::from_utf8(blobs[path].clone()).unwrap()
    }

    /// The final configuration of a 0.5 tree, as `upgrade` gives it. The facts
    /// have the roles of `unmarked_roles`.
    fn final_config(tree: &TreeData, toml: &str) -> (OldConfig, Config) {
        let old = OldConfig::read(toml).unwrap();
        let p1 = upgrade_config::pass1(toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let classified = tree
            .with(|t| {
                classify(
                    t,
                    &Basis {
                        cfg: &cfg1,
                        old: Some(&old),
                        layout: false,
                    },
                )
            })
            .unwrap();
        let mut facts = classified.facts(&cfg1);
        facts.unmarked_roles = tree.with(|t| unmarked_roles(t, &old)).unwrap();
        let plan = upgrade_config::plan(toml, &old, &facts, &opts()).unwrap();
        (old, Config::parse(&plan.text).unwrap())
    }

    struct Case {
        blobs: HashMap<String, Vec<u8>>,
        tree: TreeData,
        old: OldConfig,
        fin: Config,
    }

    fn case(overlays: &[&str]) -> Case {
        with_files(overlays, &[])
    }

    /// The same, with more files in the tree (a path and its text).
    fn with_files(overlays: &[&str], more: &[(&str, &str)]) -> Case {
        let mut blobs = blobs_of(overlays);
        for (p, t) in more {
            blobs.insert(p.to_string(), t.as_bytes().to_vec());
        }
        let tree = TreeData::from_map(blobs.clone());
        let toml = text(&blobs, "spec-dna.toml");
        let (old, fin) = final_config(&tree, &toml);
        Case {
            blobs,
            tree,
            old,
            fin,
        }
    }

    fn plan_of(c: &Case) -> FilesPlan {
        c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap()
    }

    fn paths(m: &BTreeMap<String, String>) -> Vec<&str> {
        m.keys().map(String::as_str).collect()
    }

    /// The tree after the plan: the blobs with each write and each remove.
    fn applied(c: &Case, p: &FilesPlan) -> HashMap<String, Vec<u8>> {
        let mut blobs = c.blobs.clone();
        for (path, t) in &p.writes {
            blobs.insert(path.clone(), t.clone().into_bytes());
        }
        for path in &p.removes {
            assert!(blobs.remove(path).is_some(), "{path} is not in the tree");
        }
        blobs
    }

    /// The findings of `harness::check` and `adapters::check` on a tree.
    fn findings(blobs: &HashMap<String, Vec<u8>>, fin: &Config) -> Vec<(Code, String)> {
        let files: Vec<(String, String)> = blobs
            .iter()
            .filter(|(p, _)| harness::wanted(p))
            .map(|(p, b)| (p.clone(), String::from_utf8(b.clone()).unwrap()))
            .collect();
        let mut out: Vec<(Code, String)> = Vec::new();
        if let Some(h) = &fin.harness {
            out.extend(
                harness::check(h, &files)
                    .into_iter()
                    .map(|d| (d.code, d.path)),
            );
        }
        let adapter_files: Vec<(&str, String)> = adapters::PATHS
            .iter()
            .filter_map(|p| {
                blobs
                    .get(*p)
                    .map(|b| (*p, String::from_utf8(b.clone()).unwrap()))
            })
            .collect();
        let types = adapters::project_types(&fin.layers, &fin.types);
        out.extend(
            adapters::check(&adapter_files, &types)
                .into_iter()
                .map(|d| (d.code, d.path)),
        );
        out.sort_by(|a, b| a.1.cmp(&b.1));
        out
    }

    const OWN_AGENT: &str = "---\nname: verifier\ndescription: My own.\n---\nMine.\n";

    #[test]
    fn lean_checker_goes_and_verifier_comes() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        let v = &p.writes[".claude/agents/verifier.md"];
        assert!(v.starts_with("---\nname: verifier\n"), "{v}");
        assert!(!c.blobs.contains_key(".claude/agents/verifier.md"));
        assert!(p.kept.is_empty(), "{:?}", p.kept);
        assert!(p.ended_roles.is_empty());
    }

    #[test]
    fn the_files_of_a_lean_upgrade_are_these() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        // The adapters of 0.6 (both instruction files, the skill), `verifier`
        // and the team page, which names it. The marker of each other agent
        // file has the version, so `Explore` and `fallback-advisor` change.
        // `.claude/settings.json` has the hooks of 0.5 already.
        assert_eq!(
            paths(&p.writes),
            [
                ".claude/agents/Explore.md",
                ".claude/agents/fallback-advisor.md",
                ".claude/agents/model-team.md",
                ".claude/agents/verifier.md",
                ".claude/skills/spec-dna/SKILL.md",
                "AGENTS.md",
                "CLAUDE.md",
            ]
        );
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert!(p.request_bytes > 0);
    }

    #[test]
    fn a_full_upgrade_removes_checker_and_spec_writer_and_writes_no_simplifier() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        assert_eq!(
            p.removes,
            [".claude/agents/checker.md", ".claude/agents/spec-writer.md"]
        );
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        // The code simplifier is off (D139), and the new parts are off.
        assert!(!p.writes.contains_key(".claude/agents/code-simplifier.md"));
        assert!(
            !p.writes.keys().any(|k| k.contains("spec-dna-")),
            "{:?}",
            paths(&p.writes)
        );
        assert!(!p.writes.contains_key("REVIEW.md"));
        let settings = p.writes.get(".claude/settings.json");
        assert!(
            settings.is_none_or(|s| !s.contains("protect-tests")),
            "{settings:?}"
        );
        assert!(!c.fin.harness.as_ref().unwrap().on(Role::CodeSimplifier));
    }

    #[test]
    fn the_files_of_a_full_upgrade_are_these() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        // As lean, and `implementer`, whose marker has the version. No
        // `code-simplifier.md`: the role is off.
        assert_eq!(
            paths(&p.writes),
            [
                ".claude/agents/Explore.md",
                ".claude/agents/fallback-advisor.md",
                ".claude/agents/implementer.md",
                ".claude/agents/model-team.md",
                ".claude/agents/verifier.md",
                ".claude/skills/spec-dna/SKILL.md",
                "AGENTS.md",
                "CLAUDE.md",
            ]
        );
    }

    #[test]
    fn an_unedited_agent_file_changes_only_its_marker() {
        // The marker has the version, so the file is written again. The text
        // above the marker is the text of 0.5, except for a blank line between
        // `Answer:` and its list (0.6.1, for markdownlint MD032), and the
        // paragraph `Scratch folder:` (0.7.1). The test ignores blank lines
        // and this paragraph.
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        let above = |t: &str| {
            let head = t.trim_end().rsplit_once('\n').unwrap().0;
            head.lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with("Scratch folder:"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for name in ["Explore", "fallback-advisor", "implementer"] {
            let path = format!(".claude/agents/{name}.md");
            let old = text(&c.blobs, &path);
            assert_ne!(old, p.writes[&path], "{name}");
            assert_eq!(above(&old), above(&p.writes[&path]), "{name}");
        }
    }

    #[test]
    fn the_report_names_the_model_of_the_ended_role() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        let file = text(&c.blobs, ".claude/agents/spec-writer.md");
        let model = file
            .lines()
            .find_map(|l| l.strip_prefix("model: "))
            .unwrap()
            .to_string();
        assert_eq!(model, "sonnet");
        assert_eq!(p.ended_roles, [("spec-writer".to_string(), model)]);
        // A lean project has no such role.
        assert!(plan_of(&case(&["harness-lean"])).ended_roles.is_empty());
    }

    #[test]
    fn the_planned_tree_has_no_finding_of_the_harness_or_the_adapters() {
        for overlays in [&["harness-lean"][..], &["harness-full"][..]] {
            let c = case(overlays);
            let p = plan_of(&c);
            let found = findings(&applied(&c, &p), &c.fin);
            assert!(found.is_empty(), "{overlays:?}: {found:?}");
        }
    }

    #[test]
    fn an_edited_checker_stays_and_gives_sd501_only() {
        let c = case(&["harness-lean", "harness-edited"]);
        let p = plan_of(&c);
        assert!(p.removes.is_empty(), "{:?}", p.removes);
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/checker.md") && k.contains("SD501")),
            "{:?}",
            p.kept
        );
        let found = findings(&applied(&c, &p), &c.fin);
        assert_eq!(
            found,
            [(
                Code::AdapterModified,
                ".claude/agents/checker.md".to_string()
            )]
        );
    }

    #[test]
    fn a_verifier_file_with_no_marker_is_never_replaced() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/verifier.md", OWN_AGENT)],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert_eq!(roles, BTreeSet::from(["verifier".to_string()]));
        let h = c.fin.harness.as_ref().unwrap();
        assert!(!h.on(Role::Verifier));
        let p = plan_of(&c);
        assert!(!p.writes.contains_key(".claude/agents/verifier.md"));
        assert!(
            !p.removes
                .contains(&".claude/agents/verifier.md".to_string())
        );
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/verifier.md")
                    && k.contains("no marker")
                    && k.contains("`verifier`")),
            "{:?}",
            p.kept
        );
        let after = applied(&c, &p);
        assert_eq!(text(&after, ".claude/agents/verifier.md"), OWN_AGENT);
        // The unedited generated `checker.md` still goes: spec-dna wrote it.
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert_eq!(findings(&after, &c.fin), []);
    }

    #[test]
    fn a_file_of_the_project_under_a_role_name_turns_off_only_roles_of_the_level() {
        // At level lean, `implementer` is not a role of the project.
        let own = |name: &str| format!("---\nname: {name}\ndescription: My own.\n---\nMine.\n");
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/implementer.md", &own("implementer"))],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert!(roles.is_empty(), "{roles:?}");
        let p = plan_of(&c);
        assert!(p.kept.is_empty(), "{:?}", p.kept);
        // At level full, the advisor, the researcher and the implementer are.
        let c = with_files(
            &["harness-full"],
            &[
                (".claude/agents/implementer.md", &own("implementer")),
                (
                    ".claude/agents/fallback-advisor.md",
                    &own("fallback-advisor"),
                ),
            ],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert_eq!(
            roles,
            BTreeSet::from(["advisor".to_string(), "implementer".to_string()])
        );
        let p = plan_of(&c);
        assert_eq!(p.kept.len(), 2, "{:?}", p.kept);
        let h = c.fin.harness.as_ref().unwrap();
        assert_eq!(h.cfg(Role::Advisor).agent, harness::Model::None);
        assert!(!h.on(Role::Implementer));
        assert!(!p.writes.contains_key(".claude/agents/implementer.md"));
        assert!(!p.writes.contains_key(".claude/agents/fallback-advisor.md"));
        assert_eq!(findings(&applied(&c, &p), &c.fin), []);
    }

    #[test]
    fn an_old_agent_file_with_no_marker_stays_and_the_report_names_it() {
        let c = with_files(
            &["harness-lean"],
            &[(
                ".claude/agents/checker.md",
                "---\nname: checker\n---\nMine.\n",
            )],
        );
        let p = plan_of(&c);
        assert!(p.removes.is_empty(), "{:?}", p.removes);
        assert!(!p.writes.contains_key(".claude/agents/checker.md"));
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/checker.md") && k.contains("no marker")),
            "{:?}",
            p.kept
        );
    }

    #[test]
    fn a_project_with_no_harness_gets_the_adapters_only() {
        let c = case(&[]);
        let p = plan_of(&c);
        assert_eq!(
            paths(&p.writes),
            [".claude/skills/spec-dna/SKILL.md", "AGENTS.md", "CLAUDE.md"]
        );
        assert!(p.removes.is_empty());
        assert_eq!(p.request_bytes, 0);
        let line = adapters::line();
        assert!(p.writes["AGENTS.md"].contains(&line));
        assert!(p.writes["CLAUDE.md"].contains(&line));
        let skill = &p.writes[".claude/skills/spec-dna/SKILL.md"];
        let types = adapters::project_types(&c.fin.layers, &c.fin.types);
        assert_eq!(*skill, adapters::skill_file(&types));
        assert_eq!(findings(&applied(&c, &p), &c.fin), []);
    }

    #[test]
    fn the_skill_lists_the_types_of_the_new_layers() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        let skill = &p.writes[".claude/skills/spec-dna/SKILL.md"];
        for t in ["intent", "research", "requirements"] {
            assert!(skill.contains(t), "{t}");
        }
        assert!(!skill.contains("Types of this project: spec, "), "{skill}");
    }

    #[test]
    fn a_hook_of_0_4_1_gets_the_staged_flag() {
        let hook = "repos:\n  - repo: local\n    hooks:\n      - id: spec-dna\n        entry: spec-dna check\n";
        let c = with_files(&["harness-lean"], &[(".pre-commit-config.yaml", hook)]);
        let p = plan_of(&c);
        assert!(
            p.writes[".pre-commit-config.yaml"].contains("entry: spec-dna check --staged"),
            "{:?}",
            p.writes.get(".pre-commit-config.yaml")
        );
    }

    #[test]
    fn a_second_plan_on_the_result_changes_nothing() {
        for overlays in [&[][..], &["harness-lean"][..], &["harness-full"][..]] {
            let c = case(overlays);
            let p = plan_of(&c);
            let tree = TreeData::from_map(applied(&c, &p));
            // The source is now a 0.6 file: the final configuration.
            let again = tree.with(|t| plan_files(t, &c.fin, false)).unwrap();
            assert!(
                again.writes.is_empty(),
                "{overlays:?}: {:?}",
                paths(&again.writes)
            );
            assert!(again.removes.is_empty());
            assert!(again.kept.is_empty(), "{overlays:?}: {:?}", again.kept);
        }
    }

    #[test]
    fn a_source_of_0_6_removes_nothing() {
        let c = case(&["harness-lean"]);
        let from_new = c.tree.with(|t| plan_files(t, &c.fin, false)).unwrap();
        assert!(from_new.removes.is_empty());
        assert!(from_new.writes.contains_key(".claude/agents/verifier.md"));
    }

    #[test]
    fn a_source_of_0_6_refuses_a_file_that_the_role_would_replace() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/verifier.md", OWN_AGENT)],
        );
        // The configuration of the 0.5 run has the role off. A 0.6 file with
        // the role on is a conflict, and the plan stops.
        let mut on = c.fin.harness.clone().unwrap();
        on.cfg_mut(Role::Verifier).model = harness::Model::Id("opus".into());
        let mut fin = c.fin.clone();
        fin.harness = Some(on);
        let e = c.tree.with(|t| plan_files(t, &fin, false)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("verifier.md"), "{}", e.message());
    }

    #[test]
    fn a_skill_of_the_project_at_the_adapter_path_stops_the_plan() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/skills/spec-dna/SKILL.md", "My own skill.\n")],
        );
        let e = c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("SKILL.md"), "{}", e.message());
    }

    #[test]
    fn a_settings_file_that_is_not_json_stops_the_plan() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/settings.json", "{ not json")],
        );
        let e = c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("settings.json"), "{}", e.message());
    }

    #[test]
    fn a_file_that_is_not_text_stops_the_plan_with_its_path() {
        let mut c = case(&["harness-lean"]);
        c.blobs.insert("AGENTS.md".into(), vec![0xff, 0xfe, 0x41]);
        let tree = TreeData::from_map(c.blobs.clone());
        let e = tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Config(_)), "{e:?}");
        assert!(e.message().starts_with("AGENTS.md"), "{}", e.message());
    }

    #[test]
    fn level_none_writes_no_harness_file_and_keeps_the_old_ones() {
        let c = with_files(&["harness-lean"], &[]);
        let mut fin = c.fin.clone();
        fin.harness = Some(harness::Harness::new(Level::None));
        let p = c.tree.with(|t| plan_files(t, &fin, true)).unwrap();
        assert!(p.removes.is_empty());
        assert!(
            !p.writes.keys().any(|k| k.starts_with(".claude/agents/")),
            "{:?}",
            paths(&p.writes)
        );
        assert!(!p.writes.contains_key(".claude/settings.json"));
        assert!(p.writes.contains_key("AGENTS.md"));
    }

    #[test]
    fn the_adapters_follow_the_files_of_the_tree() {
        // Both instruction files exist: all 3 adapters.
        let c = case(&[]);
        assert_eq!(
            plan_of(&c).adapters,
            [Adapter::AgentsMd, Adapter::ClaudeMd, Adapter::Skill]
        );
        // `CLAUDE.md` only: `AGENTS.md` is no target, and `upgrade` creates none.
        let mut blobs = c.blobs.clone();
        blobs.remove("AGENTS.md");
        let tree = TreeData::from_map(blobs);
        let p = tree.with(|t| plan_files(t, &c.fin, true)).unwrap();
        assert_eq!(p.adapters, [Adapter::ClaudeMd, Adapter::Skill]);
        assert!(!p.writes.contains_key("AGENTS.md"));
        // No instruction file: all 3 adapters, and both files are new.
        let mut blobs = c.blobs.clone();
        blobs.remove("AGENTS.md");
        blobs.remove("CLAUDE.md");
        let tree = TreeData::from_map(blobs);
        let p = tree.with(|t| plan_files(t, &c.fin, true)).unwrap();
        assert_eq!(
            p.adapters,
            [Adapter::AgentsMd, Adapter::ClaudeMd, Adapter::Skill]
        );
        assert!(p.writes.contains_key("AGENTS.md") && p.writes.contains_key("CLAUDE.md"));
    }
    // --- the plan and the command (S10) ---

    fn popts() -> PlanOpts {
        PlanOpts {
            layout: false,
            pin: VERSION.to_string(),
            sums: None,
            areas: Vec::new(),
        }
    }

    /// The tree after a plan: the moves, the writes and the removes.
    fn after_plan(blobs: &HashMap<String, Vec<u8>>, p: &Plan) -> HashMap<String, Vec<u8>> {
        let mut out = blobs.clone();
        for m in &p.docs.moves {
            let bytes = out.remove(&m.from).expect("a moved file is in the tree");
            out.insert(m.to.clone(), bytes);
        }
        for (path, t) in &p.writes {
            out.insert(path.clone(), t.clone().into_bytes());
        }
        for path in &p.removes {
            out.remove(path);
        }
        out
    }

    #[test]
    fn the_overlay_of_a_plan_gives_the_tree_that_the_plan_writes() {
        let blobs = blobs_of(&["harness-lean"]);
        let first = TreeData::from_map(blobs.clone())
            .with(|t| plan(t, &popts()))
            .unwrap();
        let ov = first.overlay();
        assert!(!ov.moved.is_empty() && !ov.removed.is_empty());
        assert!(ov.moved.iter().all(|(from, to)| {
            first
                .docs
                .moves
                .iter()
                .any(|m| m.from == *from && m.to == *to)
        }));
        assert!(
            ov.removed
                .contains(&".claude/agents/checker.md".to_string())
        );
        assert_eq!(
            ov.texts.keys().collect::<BTreeSet<_>>(),
            first.writes.keys().collect::<BTreeSet<_>>()
        );
        assert!(ov.texts.contains_key(".claude/agents/verifier.md"));
        // The snapshot with the overlay has the paths of the tree after the plan.
        let snap = repo::Snapshot {
            worktree: false,
            files: {
                let mut f: Vec<repo::Entry> = blobs
                    .keys()
                    .map(|p| repo::Entry {
                        path: p.clone(),
                        blob: Some(format!("blob-{p}")),
                    })
                    .collect();
                f.sort_by(|a, b| a.path.cmp(&b.path));
                f
            },
            texts: None,
        }
        .overlaid(&ov);
        let got: BTreeSet<&str> = snap.files.iter().map(|e| e.path.as_str()).collect();
        let want = after_plan(&blobs, &first);
        assert_eq!(got, want.keys().map(String::as_str).collect());
    }

    /// A git repository with the files of a fixture and 1 commit.
    fn commit_of(overlays: &[&str], name: &str) -> PathBuf {
        use std::process::Command;
        let dir = std::env::temp_dir().join(format!(
            "spec-dna-upgrade-base-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, bytes) in blobs_of(overlays) {
            let full = dir.join(&path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, bytes).unwrap();
        }
        let git = |args: &[&str]| {
            let o = Command::new("git")
                .current_dir(&dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .output()
                .unwrap();
            assert!(o.status.success(), "{args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "x"]);
        dir
    }

    fn head_config(extra: &str) -> Config {
        Config::parse(&format!(
            "spec-dna = \"{VERSION}\"\n{extra}[[roots]]\npath = \"docs\"\n"
        ))
        .unwrap()
    }

    #[test]
    fn the_plan_of_a_base_commit_takes_the_pin_and_the_sums_of_the_head() {
        let dir = commit_of(&[], "sums");
        let sums: String = crate::release::assets(VERSION)
            .into_iter()
            .map(|a| format!("\"{a}\" = \"{}\"\n", "c".repeat(64)))
            .collect();
        let head = head_config(&format!("[spec-dna-sha256]\n{sums}\n"));
        assert!(head.sha256.is_some());
        let git = Git::new(&dir);
        let p = plan_base(&git, "HEAD", "main", &head).unwrap();
        assert_eq!((p.from.as_str(), p.to.as_str()), ("0.5.0", VERSION));
        let toml = &p.writes[config::FILE_NAME];
        assert!(
            toml.starts_with(&format!("spec-dna = \"{VERSION}\"")),
            "{toml}"
        );
        assert!(toml.contains(&"c".repeat(64)), "{toml}");
        // The base has no `--layout` move: only the changes move.
        assert!(p.docs.moves.iter().all(|m| m.kind == MoveKind::Change));
        // The tree of the commit is the base, not the work directory.
        std::fs::write(dir.join("spec-dna.toml"), "spec-dna = \"0.4.2\"\n").unwrap();
        let again = plan_base(&git, "HEAD", "main", &head).unwrap();
        assert_eq!(again.from, "0.5.0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plan_error_of_the_base_is_exit_code_3_and_names_the_base_and_the_pin() {
        let dir = commit_of(&[], "stop");
        let mut text = std::fs::read_to_string(dir.join("spec-dna.toml")).unwrap();
        text.push_str("\nbogus = 1\n");
        std::fs::write(dir.join("spec-dna.toml"), text).unwrap();
        let status = std::process::Command::new("git")
            .current_dir(&dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-a",
                "-m",
                "y",
            ])
            .status()
            .unwrap();
        assert!(status.success());
        let got = plan_base(&Git::new(&dir), "HEAD", "main", &head_config(""));
        match got {
            Err(Fatal::Config(m)) => {
                assert!(
                    m.starts_with("the base `main` pins `0.5.0`, and upgrade cannot plan it: "),
                    "{m}"
                );
                assert!(m.contains("unknown key"), "{m}");
            }
            other => panic!("not a configuration error: {:?}", other.err()),
        }
        let _ = std::fs::remove_dir_all(&dir);
        // A folder with no git repository: a git error is no plan error.
        let none =
            std::env::temp_dir().join(format!("spec-dna-upgrade-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&none).unwrap();
        let got = plan_base(&Git::new(&none), "HEAD", "main", &head_config(""));
        assert!(matches!(got, Err(Fatal::Io(_))), "{:?}", got.err());
        let _ = std::fs::remove_dir_all(&none);
    }

    #[test]
    fn the_source_follows_the_pin_and_the_text() {
        let roots = "[[roots]]\npath = \"docs\"\n";
        // A 0.5 pin: the 0.5 reader.
        let old = format!("spec-dna = \"0.5.0\"\n{roots}");
        assert!(matches!(source_of(&old), Ok(Source::Old(_))));
        // The pin of the binary with a 0.6 file: the 0.6 parser.
        let cur = format!("spec-dna = \"{VERSION}\"\n{roots}");
        assert!(matches!(source_of(&cur), Ok(Source::Current(_))));
        // The pin of the binary with a 0.5 key: read as 0.5 (D137).
        let hand = format!("spec-dna = \"{VERSION}\"\n{roots}type = \"spec\"\n");
        assert!(matches!(source_of(&hand), Ok(Source::Old(_))), "D137");
        // Both readers refuse: the message of the 0.6 parser.
        let bad = format!("spec-dna = \"{VERSION}\"\nfuture-key = 1\n{roots}");
        let Err(Fatal::Config(m)) = source_of(&bad) else {
            panic!("exit code 3");
        };
        assert!(m.contains("future-key"), "{m}");
        for pin in ["0.3.3", "0.5", "9.9.9"] {
            let t = format!("spec-dna = \"{pin}\"\n{roots}");
            let Err(Fatal::Config(m)) = source_of(&t) else {
                panic!("{pin}: exit code 3");
            };
            assert!(m.contains(&format!("pins version {pin}")), "{pin}: {m}");
        }
        let Err(Fatal::Config(m)) = source_of(roots) else {
            panic!("no pin: exit code 3");
        };
        assert!(m.contains("spec-dna"), "{m}");
    }

    #[test]
    fn the_pin_file_must_name_the_version_and_valid_sums() {
        let dir = std::env::temp_dir().join(format!("spec-dna-unit-pin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let table = |sum: &str| {
            let asset = crate::release::asset_name(VERSION, crate::release::TARGETS[0]);
            format!("spec-dna = \"{VERSION}\"\n[spec-dna-sha256]\n\"{asset}\" = \"{sum}\"\n")
        };
        let read = |name: &str, text: &str| {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            read_pin_file(path.to_str().unwrap())
        };
        let good = read("good.toml", &table(&"a".repeat(64))).unwrap();
        assert_eq!(good.len(), 1);
        for (name, text, want) in [
            ("short.toml", table(&"a".repeat(63)), "64"),
            ("upper.toml", table(&"A".repeat(64)), "64"),
            (
                "other.toml",
                table(&"a".repeat(64)).replace(VERSION, "0.5.9"),
                "this binary",
            ),
            (
                "none.toml",
                format!("spec-dna = \"{VERSION}\"\n"),
                "missing",
            ),
            (
                "nokey.toml",
                "[spec-dna-sha256]\n".to_string(),
                "`spec-dna`",
            ),
            ("junk.toml", "= =".to_string(), "pin-file"),
        ] {
            let Err(Fatal::Config(m)) = read(name, &text) else {
                panic!("{name}: exit code 3");
            };
            assert!(m.contains(want), "{name}: {m}");
        }
        let missing = dir.join("absent.toml");
        let Err(Fatal::Config(m)) = read_pin_file(missing.to_str().unwrap()) else {
            panic!("a missing file: exit code 3");
        };
        assert!(m.contains("cannot read"), "{m}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_kept_line_gives_its_path_and_its_reason() {
        assert_eq!(
            kept_path("kept .claude/agents/checker.md: edited by hand (SD501); delete it"),
            (
                ".claude/agents/checker.md".to_string(),
                "edited by hand (SD501); delete it".to_string()
            )
        );
        assert_eq!(
            kept_path("kept CLAUDE.md:12: edited by hand (SD501)"),
            (
                "CLAUDE.md".to_string(),
                "edited by hand (SD501)".to_string()
            )
        );
    }

    #[test]
    fn a_path_with_a_special_character_is_quoted_for_the_shell() {
        assert_eq!(shell_word("docs/a-b_c.md"), "docs/a-b_c.md");
        assert_eq!(shell_word("docs/a b.md"), "'docs/a b.md'");
        assert_eq!(shell_word("docs/it's.md"), "'docs/it'\\''s.md'");
    }

    #[test]
    fn the_plan_of_a_result_is_empty() {
        for overlays in [&[][..], &["harness-lean"], &["harness-full"]] {
            let blobs = blobs_of(overlays);
            let tree = TreeData::from_map(blobs.clone());
            let first = tree.with(|t| plan(t, &popts())).unwrap();
            assert!(!first.is_empty(), "{overlays:?}");
            assert_eq!(first.from, "0.5.0");
            assert_eq!(first.to, VERSION);
            let next = TreeData::from_map(after_plan(&blobs, &first));
            let second = next.with(|t| plan(t, &popts())).unwrap();
            assert!(
                second.is_empty(),
                "{overlays:?}: {:?} {:?} {:?}",
                second.writes.keys().collect::<Vec<_>>(),
                second.docs.moves,
                second.removes
            );
            assert_eq!(second.from, VERSION);
        }
    }

    #[test]
    fn a_plan_over_a_0_6_file_changes_only_the_pin() {
        let blobs = blobs_of(&[]);
        let first = TreeData::from_map(blobs.clone())
            .with(|t| plan(t, &popts()))
            .unwrap();
        let done = after_plan(&blobs, &first);
        let tree = TreeData::from_map(done.clone());
        // The same tree, planned for a later version of the binary.
        let later = PlanOpts {
            pin: "0.6.3".to_string(),
            ..popts()
        };
        let p = tree.with(|t| plan(t, &later)).unwrap();
        assert_eq!(p.writes.keys().collect::<Vec<_>>(), ["spec-dna.toml"]);
        let was = String::from_utf8(done["spec-dna.toml"].clone()).unwrap();
        assert_eq!(
            p.writes["spec-dna.toml"],
            was.replacen(&format!("\"{VERSION}\""), "\"0.6.3\"", 1)
        );
    }

    #[test]
    fn a_table_of_sums_with_no_pin_file_stops_a_change_of_the_pin() {
        let mut blobs = blobs_of(&[]);
        let toml = text(&blobs, "spec-dna.toml");
        blobs.insert(
            "spec-dna.toml".into(),
            format!("{toml}\n[spec-dna-sha256]\n\"x\" = \"y\"\n").into_bytes(),
        );
        let tree = TreeData::from_map(blobs);
        let Err(Fatal::Config(m)) = tree.with(|t| plan(t, &popts())) else {
            panic!("exit code 3");
        };
        assert!(m.contains("--pin-file"), "{m}");
    }

    #[test]
    fn an_instruction_file_that_the_documents_also_write_stops_the_plan() {
        // The root is the folder of the project, and `AGENTS.md` is a 0.5
        // contract there: the plan of the documents writes its type.
        let tree = crate::upgrade_docs::tests::mem(&[
            (
                "spec-dna.toml",
                "spec-dna = \"0.5.0\"\n[[roots]]\npath = \".\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n",
            ),
            (
                "AGENTS.md",
                "---\ntype: contract\nstatus: active\n---\n# Agents\n",
            ),
        ]);
        let Err(Fatal::Refused(m)) = tree.with(|t| plan(t, &popts())) else {
            panic!("exit code 1");
        };
        assert!(
            m.contains("AGENTS.md") && m.contains("does not merge"),
            "{m}"
        );
    }

    #[test]
    fn a_tree_with_no_configuration_is_exit_code_3() {
        let tree = crate::upgrade_docs::tests::mem(&[("docs/a.md", "# A\n")]);
        let Err(Fatal::Config(m)) = tree.with(|t| plan(t, &popts())) else {
            panic!("exit code 3");
        };
        assert!(m.contains("spec-dna.toml"), "{m}");
    }

    #[test]
    fn the_report_counts_what_check_all_counts() {
        let blobs = blobs_of(&["harness-full"]);
        let tree = TreeData::from_map(blobs);
        let (p, proof, r) = tree
            .with(|t| {
                let p = plan(t, &popts())?;
                let proof = prove(t, &p)?;
                let r = report(t, &p, &proof)?;
                Ok::<_, Fatal>((p, proof, r))
            })
            .unwrap();
        assert!(proof.ok(), "{:?}", proof.failures);
        assert_eq!(r.proposals, ["docs/changes/tune-pump.md"]);
        assert_eq!(
            r.locked_cites,
            [(
                "docs/decisions/ADR-3-solar-pump.md".to_string(),
                "docs/intent/solar-pump/intent.md".to_string()
            )]
        );
        assert_eq!(r.ended_roles.len(), 1);
        assert_eq!(r.ended_roles[0].0, "spec-writer");
        assert!(
            r.errors.iter().any(|(c, n)| c == "SD201" && *n == 2),
            "{:?}",
            r.errors
        );
        assert!(!r.sums_added);
        assert!(p.config.is_some());
    }
}
