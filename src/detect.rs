//! Detection for `spec-dna init` (docs/init.md). It reads the file list and
//! the Markdown files of a repository, and it proposes a configuration: the
//! document roots with their types, ids, keys, statuses and exclusions, and
//! the facts that the questions of `init` show as defaults. It reads only,
//! and it never changes a file.
//!
//! The rules are generic. They use the names of directories, the shape of
//! file names, front matter and headings, and the usual files of common
//! stacks, CI systems and hook frameworks.

use crate::config::DEFAULT_TEST_GLOBS;
use crate::model::DocType;
use crate::parse::ParsedDoc;
use std::collections::{BTreeMap, HashMap, HashSet};

/// The directory names that give a type to a directory of documents.
const TYPE_DIRS: &[(&str, DocType)] = &[
    ("adr", DocType::Decision),
    ("adrs", DocType::Decision),
    ("decisions", DocType::Decision),
    ("decision-records", DocType::Decision),
    ("architecture-decisions", DocType::Decision),
    ("specs", DocType::Spec),
    ("spec", DocType::Spec),
    ("specifications", DocType::Spec),
    ("requirements", DocType::Spec),
    ("prd", DocType::Spec),
    ("prds", DocType::Spec),
    ("contracts", DocType::Contract),
    ("interfaces", DocType::Contract),
    ("guides", DocType::Guide),
    ("guidelines", DocType::Guide),
    ("conventions", DocType::Guide),
    ("how-to", DocType::Guide),
    ("howto", DocType::Guide),
    ("runbooks", DocType::Runbook),
    ("runbook", DocType::Runbook),
    ("playbooks", DocType::Runbook),
    ("oncall", DocType::Runbook),
    ("on-call", DocType::Runbook),
    ("incidents", DocType::Incident),
    ("postmortems", DocType::Incident),
    ("post-mortems", DocType::Incident),
    ("rfcs", DocType::Change),
    ("rfc", DocType::Change),
    ("proposals", DocType::Change),
    ("changes", DocType::Change),
    ("tasks", DocType::Change),
    ("plans", DocType::Change),
    ("enhancements", DocType::Change),
    ("invariants", DocType::Invariant),
    ("principles", DocType::Invariant),
    ("notes", DocType::Note),
    ("research", DocType::Reference),
    ("references", DocType::Reference),
    ("reference", DocType::Reference),
    ("briefs", DocType::Reference),
    ("archive", DocType::Reference),
    ("meetings", DocType::Reference),
    ("meeting-notes", DocType::Reference),
];

/// Directory names of documentation trees.
const DOC_DIRS: &[&str] = &["docs", "doc", "documentation", "wiki", "handbook"];

/// Directories of agent instructions. Their Markdown files are guides.
const AGENT_DIRS: &[&str] = &[
    ".claude/skills",
    ".claude/agents",
    ".claude/commands",
    ".cursor/rules",
    ".github/instructions",
    ".windsurf/rules",
];

/// Hidden directories that can hold project knowledge for agents.
const KNOWLEDGE_HIDDEN: &[&str] = &[".claude", ".cursor", ".windsurf"];

/// Directory names of vendored and generated content.
const VENDORED: &[&str] = &[
    "node_modules",
    "vendor",
    "third_party",
    "third-party",
    "bower_components",
    ".venv",
    "venv",
    "site-packages",
];
const GENERATED: &[&str] = &[
    "generated",
    "__generated__",
    "dist",
    "build",
    "out",
    "_build",
    "coverage",
    "target",
    ".next",
];

/// Directory names of test fixtures: example documents of tests, often broken
/// on purpose.
const FIXTURES: &[&str] = &[
    "fixtures",
    "__fixtures__",
    "testdata",
    "test-data",
    "__snapshots__",
    "golden",
];

/// Name prefixes of vendored sibling directories, such as `ext-<name>`.
const VENDOR_PREFIXES: &[&str] = &[
    "ext-",
    "vendor-",
    "vendored-",
    "third-party-",
    "thirdparty-",
    "external-",
    "upstream-",
];

/// Root files that are not project documents.
const ROOT_FILES: &[&str] = &[
    "readme",
    "changelog",
    "contributing",
    "license",
    "licence",
    "copying",
    "code_of_conduct",
    "security",
    "support",
    "notice",
    "authors",
    "agents",
    "claude",
    "gemini",
    "history",
    "codeowners",
];

/// Headings of a decision outcome and of amendments in other languages. The
/// defaults (`Decision Outcome`, `Decision`, `Amendments`) are not here.
const OUTCOME_ALIASES: &[&str] = &[
    "Решение",
    "Рішення",
    "Entscheidung",
    "Lösung",
    "Décision",
    "Decisión",
    "Decisão",
    "Decisione",
    "Besluit",
    "Beslut",
    "Decyzja",
    "Rozhodnutí",
    "Karar",
    "決定",
    "决策",
    "决定",
    "결정",
];
const AMENDMENT_ALIASES: &[&str] = &[
    "Поправки",
    "Дополнения",
    "Nachtrag",
    "Nachträge",
    "Änderungen",
    "Amendements",
    "Enmiendas",
    "Emendas",
    "Emendamenti",
    "Wijzigingen",
    "Poprawki",
    "Değişiklikler",
];

/// Words at the end of a file name that mark a file that is not a decision,
/// in a directory of decisions.
const NOT_DECISION_SUFFIXES: &[&str] = &[
    "plan",
    "implementation-plan",
    "analysis",
    "notes",
    "review",
    "research",
    "draft",
    "template",
    "summary",
    "checklist",
    "appendix",
];

/// A directory with this number of Markdown files, or this amount of text,
/// and no typed directory in it, is a large reference directory.
const LARGE_FILES: usize = 100;
const LARGE_CHARS: u64 = 1_000_000;

/// One proposed document root.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: String,
    /// `None` for a mixed root.
    pub ty: Option<DocType>,
    pub default_type: DocType,
    pub status: Option<String>,
    pub legacy_ids: Vec<String>,
    pub legacy_keys: Vec<String>,
    pub status_map: Vec<(String, String)>,
    /// Types whose default required sections the root removes, because most
    /// existing documents do not have them.
    pub required_off: Vec<DocType>,
    pub exclude: Vec<String>,
    /// The number of documents of the root.
    pub docs: usize,
    /// The number of decisions of the root.
    pub decisions: usize,
    /// Keys of a configured root that detection does not make, as TOML lines
    /// (`--reconfigure`): `required-sections`, `amendment-sections`,
    /// `enforce-budgets`, `profile` and `rules`.
    pub extra: Vec<String>,
    /// The reason for the row, for the preview.
    pub why: String,
}

impl Row {
    fn new(path: &str, ty: Option<DocType>, default_type: DocType, why: String) -> Row {
        Row {
            path: path.to_string(),
            ty,
            default_type,
            status: first_status(default_type),
            legacy_ids: Vec::new(),
            legacy_keys: Vec::new(),
            status_map: Vec::new(),
            required_off: Vec::new(),
            exclude: Vec::new(),
            docs: 0,
            decisions: 0,
            extra: Vec::new(),
            why,
        }
    }

    /// The types that a document of the row can have.
    pub fn types(&self) -> Vec<DocType> {
        match self.ty {
            Some(t) => vec![t],
            None => DocType::ALL.to_vec(),
        }
    }
}

/// The default status of a document without `status` in a detected root.
/// Existing documents describe the current state, so the status is the
/// "current" status of the type. A decision stays `proposed`, so that its
/// text is not frozen before an author checks it.
fn first_status(ty: DocType) -> Option<String> {
    match ty {
        DocType::Decision => Some("proposed".into()),
        DocType::Change => Some("accepted".into()),
        t if t.has_status() => Some("active".into()),
        _ => None,
    }
}

/// The facts of a repository.
#[derive(Debug, Default)]
pub struct Facts {
    pub files: usize,
    /// The Markdown files outside vendored and generated directories.
    pub markdown: usize,
    pub brownfield: bool,
    pub rows: Vec<Row>,
    /// Global exclusions, with the reason.
    pub exclude: Vec<(String, String)>,
    /// The front-matter status words, with counts.
    pub statuses: Vec<(String, usize)>,
    /// The scripts of the headings, with counts.
    pub scripts: Vec<(String, usize)>,
    pub outcome_aliases: Vec<String>,
    pub amendment_aliases: Vec<String>,
    /// The type and the body characters of each document in a root.
    pub sizes: Vec<(DocType, u32)>,
    /// The types of the documents in the roots, in type order.
    pub types: Vec<DocType>,
    pub stack: Vec<String>,
    pub apps: Vec<String>,
    /// Test globs that match files, with the number of files.
    pub tests: Vec<(String, usize)>,
    pub agent_files: Vec<String>,
    pub ci: Vec<String>,
    pub hooks: Vec<String>,
    pub base: String,
}

/// One Markdown file.
struct Md {
    path: String,
    parsed: ParsedDoc,
    generated: bool,
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').collect()
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

fn is_md(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.ends_with(".md") || p.ends_with(".markdown")
}

/// Returns true if a path is in a vendored or generated directory, or in a
/// hidden directory that holds no project knowledge.
fn skipped(path: &str) -> bool {
    let segs = segments(path);
    let dirs = &segs[..segs.len().saturating_sub(1)];
    if dirs.iter().any(|s| {
        VENDORED.contains(s) || GENERATED.contains(s) || FIXTURES.contains(s) || *s == ".git"
    }) {
        return true;
    }
    if dirs.iter().any(|s| s.starts_with('.')) {
        let under = |d: &str| path.starts_with(&format!("{d}/"));
        return !(KNOWLEDGE_HIDDEN.iter().any(|k| under(k)) || AGENT_DIRS.iter().any(|a| under(a)));
    }
    false
}

/// Returns true if a Markdown file is a project document candidate.
fn candidate(path: &str) -> bool {
    if !is_md(path) || skipped(path) {
        return false;
    }
    if !path.contains('/') {
        let stem = path
            .rsplit_once('.')
            .map_or(path, |(s, _)| s)
            .to_ascii_lowercase();
        return !ROOT_FILES.contains(&stem.as_str());
    }
    true
}

/// The type of a directory name, if the name is a type word.
fn dir_type(name: &str) -> Option<DocType> {
    let n = name.to_ascii_lowercase().replace('_', "-");
    TYPE_DIRS.iter().find(|(k, _)| *k == n).map(|(_, t)| *t)
}

/// Returns `PREFIX` if a file stem starts with `PREFIX-<digits>`.
fn number_prefix(stem: &str) -> Option<&str> {
    let (p, rest) = stem.split_once('-')?;
    let ok_prefix = !p.is_empty()
        && p.as_bytes()[0].is_ascii_uppercase()
        && p.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (ok_prefix
        && digits > 0
        && rest[digits..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric()))
    .then_some(p)
}

fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(s, _)| s)
}

/// The legacy id pattern of a set of file stems: `PREFIX-{n}` if most stems
/// start with the same `PREFIX-<digits>`.
fn legacy_id(stems: &[&str], min_share: f64) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for s in stems {
        if let Some(p) = number_prefix(s) {
            *counts.entry(p).or_default() += 1;
        }
    }
    let (p, n) = counts.into_iter().max_by_key(|x| x.1)?;
    (n >= 3 && n as f64 >= min_share * stems.len() as f64).then(|| format!("{p}-{{n}}"))
}

/// The script of a heading: the script of its first letter.
fn script(text: &str) -> Option<&'static str> {
    let c = text.chars().find(|c| c.is_alphabetic())?;
    Some(match c as u32 {
        0x0041..=0x024F => "Latin",
        0x0370..=0x03FF => "Greek",
        0x0400..=0x04FF => "Cyrillic",
        0x0590..=0x05FF => "Hebrew",
        0x0600..=0x06FF => "Arabic",
        0x0900..=0x097F => "Devanagari",
        0x0E00..=0x0E7F => "Thai",
        0x3040..=0x30FF => "Japanese kana",
        0x4E00..=0x9FFF => "Han",
        0xAC00..=0xD7AF => "Hangul",
        _ => "other",
    })
}

/// Maps a legacy status word to a status of a type, or `None` if the word is
/// unknown.
fn map_status(word: &str, ty: DocType) -> Option<&'static str> {
    let w = word.to_ascii_lowercase().replace(['_', ' '], "-");
    let w = w.as_str();
    match ty {
        DocType::Decision => Some(match w {
            "approved" | "adopted" | "agreed" | "decided" | "implemented" | "done" | "final"
            | "active" | "accept" => "accepted",
            "draft" | "open" | "review" | "in-review" | "pending" | "wip" | "proposal" => {
                "proposed"
            }
            "replaced" | "deprecated" | "obsolete" => "superseded",
            "declined" | "abandoned" | "withdrawn" | "cancelled" | "canceled" => "rejected",
            _ => return None,
        }),
        DocType::Change => Some(match w {
            "draft" | "open" | "new" | "todo" | "planned" | "pending" | "backlog" => "proposed",
            "approved" | "in-progress" | "wip" | "active" | "doing" | "started" => "accepted",
            "complete" | "completed" | "shipped" | "merged" | "closed" | "implemented"
            | "released" | "finished" => "done",
            "declined" | "abandoned" | "cancelled" | "canceled" | "wontfix" => "rejected",
            _ => return None,
        }),
        t if t.has_status() => Some(match w {
            "shipped" | "released" | "implemented" | "done" | "complete" | "completed" | "live"
            | "current" | "approved" | "accepted" | "in-delivery" | "delivered" | "stable"
            | "final" | "partial" | "published" => "active",
            "wip" | "in-progress" | "planned" | "todo" | "open" | "new" | "proposed" | "review"
            | "in-review" | "pending" => "draft",
            "obsolete" | "legacy" | "sunset" => "deprecated",
            "replaced" => "superseded",
            "archived" | "removed" | "cancelled" | "canceled" | "abandoned" | "withdrawn" => {
                "retired"
            }
            _ => return None,
        }),
        _ => None,
    }
}

/// Information that comes from git, not from the files.
#[derive(Debug, Default)]
pub struct GitInfo {
    pub base: String,
}

/// Detects the facts of a repository from its file list. `read` returns the
/// text of a file of the working tree.
pub fn detect(
    files: &[String],
    read: impl Fn(&str) -> Option<String> + Sync,
    git: GitInfo,
) -> Facts {
    let mut facts = Facts {
        files: files.len(),
        base: git.base,
        ..Facts::default()
    };
    let file_set: HashSet<&str> = files.iter().map(String::as_str).collect();
    let has =
        |p: &str| file_set.contains(p) || files.iter().any(|f| f.starts_with(&format!("{p}/")));

    // Markdown files, parsed.
    let md_paths: Vec<&String> = files.iter().filter(|f| candidate(f)).collect();
    let mds: Vec<Md> = crate::repo::par_map(&md_paths, |p| {
        let text = read(p).unwrap_or_default();
        let head: String = text
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // A marker of a generator: "@generated", "auto-generated", or a
        // line that says "generated" and "do not edit". A plain "do not edit"
        // is often an instruction to a reader.
        let generated = head.lines().any(|l| {
            l.contains("auto-generated")
                || l.contains("autogenerated")
                || l.contains("@generated")
                || (l.contains("generated") && l.contains("do not edit"))
        });
        Md {
            path: p.to_string(),
            parsed: crate::parse::parse(&text),
            generated,
        }
    });
    facts.markdown = mds.len();

    // Counts per directory: Markdown files and all files of the subtree.
    let mut md_tree: HashMap<String, usize> = HashMap::new();
    let mut md_direct: HashMap<String, Vec<usize>> = HashMap::new();
    let mut chars_tree: HashMap<String, u64> = HashMap::new();
    for (i, m) in mds.iter().enumerate() {
        md_direct
            .entry(dir_of(&m.path).to_string())
            .or_default()
            .push(i);
        let mut d = dir_of(&m.path);
        loop {
            *md_tree.entry(d.to_string()).or_default() += 1;
            *chars_tree.entry(d.to_string()).or_default() += u64::from(m.parsed.body_chars);
            if d.is_empty() {
                break;
            }
            d = dir_of(d);
        }
    }
    let mut all_tree: HashMap<String, usize> = HashMap::new();
    for f in files.iter().filter(|f| !skipped(f)) {
        let mut d = dir_of(f);
        loop {
            *all_tree.entry(d.to_string()).or_default() += 1;
            if d.is_empty() {
                break;
            }
            d = dir_of(d);
        }
    }
    let md_in = |d: &str| md_tree.get(d).copied().unwrap_or(0);
    let mut dirs: Vec<&String> = md_tree.keys().collect();
    dirs.sort();

    // 1. Documentation trees: top-most directories that are mostly
    // Markdown, or that have a documentation name.
    let is_tree = |d: &str| {
        let name = d.rsplit('/').next().unwrap_or(d);
        let md = md_in(d);
        let all = all_tree.get(d).copied().unwrap_or(md).max(1);
        (DOC_DIRS.contains(&name.to_ascii_lowercase().as_str()) && md > 0)
            || (md >= 5 && md * 2 >= all)
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut trees: Vec<&str> = Vec::new();
    for d in &dirs {
        let d = d.as_str();
        if !is_tree(d)
            || trees
                .iter()
                .any(|t| t.is_empty() || d.starts_with(&format!("{t}/")))
        {
            continue;
        }
        if d.is_empty() && dirs.len() > 1 && md_direct.get("").is_none_or(|v| v.len() < 3) {
            continue;
        }
        trees.push(d);
        rows.push(Row::new(
            if d.is_empty() { "." } else { d },
            None,
            DocType::Guide,
            format!("{} Markdown files, mostly documentation", md_in(d)),
        ));
    }
    // 2. Typed directories and agent directories.
    for d in &dirs {
        let d = d.as_str();
        if d.is_empty() {
            continue;
        }
        let name = d.rsplit('/').next().unwrap_or(d);
        if AGENT_DIRS.contains(&d) {
            let mut r = Row::new(
                d,
                Some(DocType::Guide),
                DocType::Guide,
                "agent instructions".into(),
            );
            r.why = format!("agent instructions, {} Markdown files", md_in(d));
            rows.push(r);
        } else if let Some(t) = dir_type(name).filter(|_| md_in(d) >= 2) {
            rows.push(Row::new(
                d,
                Some(t),
                t,
                format!("directory name `{name}`, {} Markdown files", md_in(d)),
            ));
        }
    }
    // 3. Directories of numbered files (`PREFIX-<n>-*.md`): legacy ids.
    for (d, idx) in &md_direct {
        let stems: Vec<&str> = idx.iter().map(|&i| stem(&mds[i].path)).collect();
        let Some(pattern) = legacy_id(&stems, 0.6) else {
            continue;
        };
        if let Some(r) = rows.iter_mut().find(|r| r.path == *d) {
            r.legacy_ids.push(pattern);
            continue;
        }
        let parent = enclosing(&rows, d).map(|i| rows[i].default_type);
        if parent.is_none() {
            continue;
        }
        let t = parent.unwrap_or(DocType::Guide);
        let mut r = Row::new(d, Some(t), t, format!("numbered files `{pattern}`"));
        r.legacy_ids.push(pattern);
        rows.push(r);
    }
    // 4. Large reference directories: many documents or much text, and no
    // other root inside.
    let mut large: Vec<String> = Vec::new();
    for d in &dirs {
        let d = d.as_str();
        if d.is_empty() || rows.iter().any(|r| r.path == d) || enclosing(&rows, d).is_none() {
            continue;
        }
        if large.iter().any(|l| d.starts_with(&format!("{l}/"))) {
            continue;
        }
        let big = md_in(d) >= LARGE_FILES || chars_tree.get(d).copied().unwrap_or(0) >= LARGE_CHARS;
        let inner = rows.iter().any(|r| r.path.starts_with(&format!("{d}/")));
        if big && !inner {
            large.push(d.to_string());
        }
    }
    for d in large {
        let why = format!(
            "large reference directory: {} Markdown files, {} characters",
            md_in(&d),
            chars_tree.get(&d).copied().unwrap_or(0)
        );
        rows.push(Row::new(
            &d,
            Some(DocType::Reference),
            DocType::Reference,
            why,
        ));
    }
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    rows.dedup_by(|a, b| a.path == b.path);

    // Assign each document to its deepest root.
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); rows.len()];
    for (i, m) in mds.iter().enumerate() {
        if let Some(r) = owner(&rows, &m.path) {
            members[r].push(i);
        }
    }
    // Vendored sibling directories and generated files inside the roots.
    for (ri, r) in rows.iter_mut().enumerate() {
        let base = if r.path == "." {
            String::new()
        } else {
            format!("{}/", r.path)
        };
        let mut children: BTreeMap<&str, usize> = BTreeMap::new();
        for &i in &members[ri] {
            let rel = &mds[i].path[base.len()..];
            if let Some((child, _)) = rel.split_once('/') {
                for p in VENDOR_PREFIXES {
                    if child.starts_with(p) {
                        *children.entry(p).or_default() += 1;
                    }
                }
            }
        }
        for (p, n) in children {
            if n >= 2 {
                r.exclude.push(format!("{p}*/**"));
                r.why
                    .push_str(&format!("; vendored `{p}*` directories excluded"));
            }
        }
    }
    // A root inside an excluded directory of another root is not a root. A
    // typed directory with the single type of its enclosing root, and nothing
    // of its own, is part of that root.
    let excluded_by = |rows: &[Row], i: usize| -> bool {
        let p = &rows[i].path;
        rows.iter().enumerate().any(|(j, r)| {
            j != i
                && (r.path == "." || p.starts_with(&format!("{}/", r.path)))
                && r.exclude.iter().any(|g| {
                    let rel = if r.path == "." {
                        p.as_str()
                    } else {
                        &p[r.path.len() + 1..]
                    };
                    crate::glob::matches(g, &format!("{rel}/x"))
                })
        })
    };
    let drop: Vec<bool> = (0..rows.len())
        .map(|i| {
            let r = &rows[i];
            let parent = enclosing(&rows, &r.path).map(|j| &rows[j]);
            let same = r.why.starts_with("directory name")
                && r.legacy_ids.is_empty()
                && r.exclude.is_empty()
                && parent.is_some_and(|p| p.ty.is_some() && p.ty == r.ty);
            excluded_by(&rows, i) || same
        })
        .collect();
    let mut keep = drop.iter().map(|d| !d);
    rows.retain(|_| keep.next().unwrap_or(true));
    // The documents of each root, without the paths that its exclusions name.
    let in_root = |rows: &[Row], m: &Md| -> Option<usize> {
        let r = owner(rows, &m.path)?;
        let rel = if rows[r].path == "." {
            m.path.as_str()
        } else {
            &m.path[rows[r].path.len() + 1..]
        };
        (!rows[r].exclude.iter().any(|g| crate::glob::matches(g, rel))).then_some(r)
    };
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); rows.len()];
    for (i, m) in mds.iter().enumerate() {
        if let Some(r) = in_root(&rows, m) {
            members[r].push(i);
        }
    }
    let gen_files: Vec<&str> = mds
        .iter()
        .filter(|m| m.generated && in_root(&rows, m).is_some())
        .map(|m| m.path.as_str())
        .collect();
    for p in &gen_files {
        facts
            .exclude
            .push((p.to_string(), "generated (the file says so)".into()));
    }
    for v in VENDORED.iter().chain(GENERATED) {
        if files
            .iter()
            .any(|f| is_md(f) && segments(f).contains(v) && owner(&rows, f).is_some())
        {
            let why = if VENDORED.contains(v) {
                "vendored"
            } else {
                "generated"
            };
            facts.exclude.push((format!("**/{v}/**"), why.into()));
        }
    }
    let node = files
        .iter()
        .any(|f| f == "package.json" || f.ends_with("/package.json"));
    if node && !facts.exclude.iter().any(|e| e.0 == "**/node_modules/**") {
        facts.exclude.push((
            "**/node_modules/**".into(),
            "vendored (Node.js packages)".into(),
        ));
    }

    // Per root: documents, statuses, required sections, legacy keys.
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    let mut scripts: BTreeMap<String, usize> = BTreeMap::new();
    let mut outcome: Vec<String> = Vec::new();
    let mut amend: Vec<String> = Vec::new();
    for (ri, r) in rows.iter_mut().enumerate() {
        let root_rel = |p: &str| -> String {
            if r.path == "." {
                p.to_string()
            } else {
                p[r.path.len() + 1..].to_string()
            }
        };
        let docs: Vec<&Md> = members[ri]
            .iter()
            .map(|&i| &mds[i])
            .filter(|m| !gen_files.contains(&m.path.as_str()))
            .collect();
        // A typed root whose documents name another type becomes mixed.
        if let Some(t) = r.ty
            && docs.iter().any(|m| {
                fm_str(&m.parsed, "type")
                    .and_then(|x| DocType::from_name(&x))
                    .is_some_and(|x| x != t)
            })
        {
            r.ty = None;
            r.default_type = t;
        }
        // The type of each document: its front matter, or the root default.
        let types: Vec<DocType> = docs
            .iter()
            .map(|m| {
                fm_str(&m.parsed, "type")
                    .and_then(|t| DocType::from_name(&t))
                    .filter(|t| r.ty.is_none_or(|rt| rt == *t))
                    .unwrap_or(r.default_type)
            })
            .collect();
        for m in &docs {
            for ch in &m.parsed.chunks {
                if ch.level >= 1
                    && ch.level <= 3
                    && let Some(s) = script(&ch.heading)
                {
                    *scripts.entry(s.to_string()).or_default() += 1;
                }
            }
        }
        // Decision headings in other languages.
        for (m, t) in docs.iter().zip(&types) {
            if *t != DocType::Decision {
                continue;
            }
            for ch in m.parsed.chunks.iter().filter(|c| c.level >= 2) {
                let h = ch.heading.trim();
                for a in OUTCOME_ALIASES {
                    if h.to_lowercase() == a.to_lowercase() && !outcome.iter().any(|x| x == a) {
                        outcome.push(a.to_string());
                    }
                }
                for a in AMENDMENT_ALIASES {
                    if h.to_lowercase().starts_with(&a.to_lowercase())
                        && !amend.iter().any(|x| x == a)
                    {
                        amend.push(a.to_string());
                    }
                }
            }
        }
        // Statuses of the front matter, and a map for legacy words.
        let mut words: BTreeMap<String, usize> = BTreeMap::new();
        for m in &docs {
            if let Some(s) = fm_str(&m.parsed, "status") {
                *words.entry(s.clone()).or_default() += 1;
                *statuses.entry(s).or_default() += 1;
            }
        }
        let row_types = r.types();
        for w in words.keys() {
            if row_types
                .iter()
                .all(|t| t.statuses().contains(&w.as_str()) || !t.has_status())
            {
                continue;
            }
            if r.ty.is_none() && row_types.iter().all(|t| t.statuses().contains(&w.as_str())) {
                continue;
            }
            if r.ty.is_none() && row_types.iter().any(|t| t.statuses().contains(&w.as_str())) {
                // In a mixed root, a word that one type allows needs no map.
                continue;
            }
            let target = map_status(w, r.default_type)
                .map(str::to_string)
                .or_else(|| r.status.clone());
            if let Some(t) = target {
                r.status_map.push((w.clone(), t));
            }
        }
        // Required sections: decisions without an outcome heading are often
        // other files (a README, a plan): exclude them by name. If most
        // documents of a type still lack the section, the root removes it.
        let has_section = |m: &Md, t: DocType| -> bool {
            let groups = crate::config::default_required_sections();
            groups[t as usize].iter().all(|g| {
                m.parsed.chunks.iter().any(|ch| {
                    ch.level > 0
                        && g.iter()
                            .map(String::as_str)
                            .chain(if t == DocType::Decision {
                                OUTCOME_ALIASES.iter().copied()
                            } else {
                                [].iter().copied()
                            })
                            .any(|s| crate::model::same_heading(&ch.heading, s))
                })
            })
        };
        let mut excluded: HashSet<&str> = HashSet::new();
        if types.contains(&DocType::Decision) {
            let lacking: Vec<&Md> = docs
                .iter()
                .zip(&types)
                .filter(|(m, t)| **t == DocType::Decision && !has_section(m, DocType::Decision))
                .map(|(m, _)| *m)
                .collect();
            let with: Vec<&str> = docs
                .iter()
                .zip(&types)
                .filter(|(m, t)| **t == DocType::Decision && has_section(m, DocType::Decision))
                .map(|(m, _)| stem(&m.path))
                .collect();
            for m in &lacking {
                let rel = root_rel(&m.path);
                let s = stem(&m.path);
                let glob = if s.eq_ignore_ascii_case("readme") || s.eq_ignore_ascii_case("index") {
                    Some(rel.clone())
                } else {
                    let prefix = number_prefix(s);
                    NOT_DECISION_SUFFIXES
                        .iter()
                        .filter(|suf| s.ends_with(&format!("-{suf}")))
                        .max_by_key(|suf| suf.len())
                        .filter(|suf| !with.iter().any(|w| w.ends_with(&format!("-{suf}"))))
                        .map(|suf| {
                            let dir = dir_of(&rel);
                            let dir = if dir.is_empty() {
                                String::new()
                            } else {
                                format!("{dir}/")
                            };
                            match prefix {
                                Some(p) => format!("{dir}{p}-*-{suf}.md"),
                                None => format!("{dir}*-{suf}.md"),
                            }
                        })
                };
                if let Some(g) = glob {
                    if !r.exclude.contains(&g) {
                        r.exclude.push(g);
                    }
                    excluded.insert(m.path.as_str());
                }
            }
        }
        let kept: Vec<(&Md, DocType)> = docs
            .iter()
            .zip(&types)
            .filter(|(m, _)| !excluded.contains(m.path.as_str()))
            .map(|(m, t)| (*m, *t))
            .collect();
        r.docs = kept.len();
        r.decisions = kept.iter().filter(|(_, t)| *t == DocType::Decision).count();
        let mut lacking_note = Vec::new();
        for t in DocType::ALL {
            let of_type: Vec<&Md> = kept
                .iter()
                .filter(|(_, x)| *x == t)
                .map(|(m, _)| *m)
                .collect();
            if of_type.is_empty()
                || crate::config::default_required_sections()[t as usize].is_empty()
            {
                continue;
            }
            let ok = of_type.iter().filter(|m| has_section(m, t)).count();
            if (ok as f64) < 0.8 * of_type.len() as f64 {
                r.required_off.push(t);
                lacking_note.push(format!(
                    "{} of {} {} documents lack the required sections",
                    of_type.len() - ok,
                    of_type.len(),
                    t.name()
                ));
            }
        }
        if !lacking_note.is_empty() {
            r.why.push_str(&format!("; {}", lacking_note.join(", ")));
        }
        // Legacy requirement keys: bold keys such as `R-12` at the start of
        // list items.
        if row_types.iter().any(|t| t.has_requirements()) {
            let mut keys: BTreeMap<&str, (usize, bool)> = BTreeMap::new();
            for (m, t) in &kept {
                if !t.has_requirements() {
                    continue;
                }
                for q in &m.parsed.reqs {
                    let k = q.key.as_str();
                    if let Some((p, rest)) = k.split_once('-') {
                        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
                        let tail = &rest[digits..];
                        if !p.is_empty()
                            && p.bytes().all(|b| b.is_ascii_uppercase())
                            && digits > 0
                            && tail.bytes().all(|b| b.is_ascii_lowercase())
                        {
                            let e = keys.entry(p).or_default();
                            e.0 += 1;
                            e.1 |= !tail.is_empty();
                        }
                    }
                }
            }
            for (p, (n, suffix)) in keys {
                if n >= 3 {
                    r.legacy_keys
                        .push(format!("{p}-{{n}}{}", if suffix { "*" } else { "" }));
                }
            }
        }
        for (m, t) in &kept {
            facts.sizes.push((*t, m.parsed.body_chars));
        }
    }
    rows.retain(|r| r.docs > 0);
    // Files that are not documents of a root: the root files of the
    // repository (README, AGENTS.md, ...) in a root `.`, and the skill that
    // `init` generates.
    let skill_dir = crate::adapters::SKILL
        .rsplit_once('/')
        .map_or("", |(d, _)| d);
    for (i, r) in rows.clone().iter().enumerate() {
        if r.path == "." {
            for f in files.iter().filter(|f| !f.contains('/') && is_md(f)) {
                if !candidate(f) && !rows[i].exclude.contains(f) {
                    rows[i].exclude.push(f.clone());
                }
            }
        }
        if owner(&rows, &format!("{skill_dir}/SKILL.md")) == Some(i) {
            let rel = if r.path == "." {
                skill_dir.to_string()
            } else {
                skill_dir[r.path.len() + 1..].to_string()
            };
            rows[i].exclude.push(format!("{rel}/**"));
        }
    }
    let mut types: Vec<DocType> = facts.sizes.iter().map(|s| s.0).collect();
    for r in &rows {
        types.push(r.ty.unwrap_or(r.default_type));
    }
    types.sort();
    types.dedup();
    facts.types = types;
    facts.rows = rows;
    facts.statuses = statuses.into_iter().collect();
    facts.scripts = scripts.into_iter().collect();
    facts.scripts.sort_by_key(|s| std::cmp::Reverse(s.1));
    facts.outcome_aliases = outcome;
    facts.amendment_aliases = amend;
    facts.brownfield = !facts.rows.is_empty() || files.len() >= 20;

    // Stack and workspace apps.
    const MANIFESTS: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js"),
        ("tsconfig.json", "TypeScript"),
        ("deno.json", "Deno"),
        ("go.mod", "Go"),
        ("pyproject.toml", "Python"),
        ("setup.py", "Python"),
        ("requirements.txt", "Python"),
        ("pom.xml", "Java"),
        ("build.gradle", "JVM (Gradle)"),
        ("build.gradle.kts", "JVM (Gradle)"),
        ("Gemfile", "Ruby"),
        ("composer.json", "PHP"),
        ("mix.exs", "Elixir"),
        ("Package.swift", "Swift"),
        ("pubspec.yaml", "Dart"),
        ("CMakeLists.txt", "C/C++ (CMake)"),
    ];
    let mut stack: Vec<String> = Vec::new();
    let mut apps: Vec<String> = Vec::new();
    for f in files.iter().filter(|f| !skipped(f)) {
        let name = f.rsplit('/').next().unwrap_or(f);
        let depth = f.matches('/').count();
        let hit = MANIFESTS
            .iter()
            .find(|(m, _)| *m == name)
            .map(|x| x.1)
            .or_else(|| name.ends_with(".csproj").then_some(".NET"));
        if let Some(s) = hit
            && depth <= 3
        {
            if !stack.iter().any(|x| x == s) {
                stack.push(s.to_string());
            }
            let d = dir_of(f);
            if !d.is_empty() && !apps.iter().any(|a| a == d) {
                apps.push(d.to_string());
            }
        }
    }
    apps.sort();
    facts.stack = stack;
    facts.apps = apps;

    // Tests.
    for g in DEFAULT_TEST_GLOBS {
        let n = files
            .iter()
            .filter(|f| !skipped(f) && !is_md(f) && crate::glob::matches(g, f))
            .count();
        if n > 0 {
            facts.tests.push((g.to_string(), n));
        }
    }

    // Agent files, CI and hooks.
    for p in [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        ".claude/skills",
        ".claude/agents",
        ".cursor/rules",
        ".cursorrules",
        ".github/copilot-instructions.md",
        ".windsurfrules",
        ".clinerules",
    ] {
        if has(p) {
            facts.agent_files.push(p.to_string());
        }
    }
    for (p, name) in [
        (".github/workflows", "GitHub Actions"),
        (".gitlab-ci.yml", "GitLab CI"),
        (".circleci", "CircleCI"),
        ("Jenkinsfile", "Jenkins"),
        ("azure-pipelines.yml", "Azure Pipelines"),
        ("bitbucket-pipelines.yml", "Bitbucket Pipelines"),
        (".buildkite", "Buildkite"),
        (".drone.yml", "Drone"),
        (".woodpecker.yml", "Woodpecker"),
        (".travis.yml", "Travis CI"),
    ] {
        if has(p) {
            facts.ci.push(name.to_string());
        }
    }
    for (p, name) in [
        (".pre-commit-config.yaml", "pre-commit"),
        (".husky", "husky"),
        ("lefthook.yml", "lefthook"),
        (".lefthook.yml", "lefthook"),
        (".overcommit.yml", "overcommit"),
    ] {
        if has(p) && !facts.hooks.iter().any(|h| h == name) {
            facts.hooks.push(name.to_string());
        }
    }
    facts
}

/// The deepest row that contains a path, as an index.
fn owner(rows: &[Row], path: &str) -> Option<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, r)| r.path == "." || path.starts_with(&format!("{}/", r.path)))
        .max_by_key(|(_, r)| if r.path == "." { 0 } else { r.path.len() })
        .map(|(i, _)| i)
}

/// The deepest row that contains a directory (not the directory itself).
fn enclosing(rows: &[Row], dir: &str) -> Option<usize> {
    // `owner` takes the roots whose path is a prefix of `dir` and a `/`, so
    // the root of `dir` itself is not a candidate.
    owner(rows, dir)
}

fn fm_str(p: &ParsedDoc, key: &str) -> Option<String> {
    match p.front_matter.as_ref()?.get(key)? {
        (crate::frontmatter::Value::Str(s), _) => Some(s.clone()),
        _ => None,
    }
}

/// Counts the documents over the hard budget, with the budgets multiplied by
/// `factor`.
pub fn over_budget(facts: &Facts, factor: f64) -> usize {
    facts
        .sizes
        .iter()
        .filter(|(t, chars)| {
            let hard = crate::config::DEFAULT_BUDGETS[*t as usize].1;
            hard > 0 && f64::from(*chars) > f64::from(hard) * factor
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(files: &[(&str, &str)]) -> Facts {
        let list: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
        let map: HashMap<String, String> = files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect();
        detect(
            &list,
            |p| map.get(p).cloned(),
            GitInfo {
                base: "main".into(),
            },
        )
    }

    #[test]
    fn detects_typed_numbered_vendored_and_reference_roots() {
        let mut files: Vec<(String, String)> = vec![
            ("README.md".into(), "# R\n".into()),
            ("Cargo.toml".into(), "[package]\n".into()),
            ("tests/cli.rs".into(), "\n".into()),
            (".github/workflows/ci.yml".into(), "\n".into()),
            (".pre-commit-config.yaml".into(), "repos:\n".into()),
            ("docs/index.md".into(), "# Docs\n".into()),
            ("docs/setup.md".into(), "---\nstatus: current\n---\n# Setup\n".into()),
            (
                "docs/adr/README.md".into(),
                "# Format\n\nHow we write decisions.\n".into(),
            ),
            (
                "docs/adr/ADR-7-cache-implementation-plan.md".into(),
                "# Plan\n\n## Steps\n".into(),
            ),
            (
                "docs/specs/login.md".into(),
                "---\nstatus: shipped\n---\n# Login\n\n- **R-1** The app SHALL log in.\n- **R-2** x\n- **R-2a** y\n".into(),
            ),
            (
                "docs/specs/review.md".into(),
                "---\ntype: note\n---\n# Review\n".into(),
            ),
            ("docs/guides/ext-a/one.md".into(), "# One\n".into()),
            ("docs/guides/ext-b/two.md".into(), "# Two\n".into()),
            ("docs/guides/style.md".into(), "# Style\n".into()),
            ("docs/defects/DEF-1-a.md".into(), "# A\n".into()),
            ("docs/defects/DEF-2-b.md".into(), "# B\n".into()),
            ("docs/defects/DEF-3-c.md".into(), "# C\n".into()),
        ];
        for i in 1..=3 {
            files.push((
                format!("docs/adr/ADR-{i}-x.md"),
                "# X\n\n## Решение\n\nYes.\n\n## Поправки\n\nMore.\n".into(),
            ));
        }
        for i in 0..LARGE_FILES {
            files.push((format!("docs/market/m{i}.md"), "# M\n".into()));
        }
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let f = facts(&refs);
        let row = |p: &str| {
            f.rows
                .iter()
                .find(|r| r.path == p)
                .unwrap_or_else(|| panic!("{p}: {:?}", f.rows))
        };
        assert_eq!(row("docs").ty, None);
        assert_eq!(row("docs").default_type, DocType::Guide);
        assert_eq!(
            row("docs").status_map,
            [("current".to_string(), "active".to_string())]
        );
        let adr = row("docs/adr");
        assert_eq!(adr.ty, Some(DocType::Decision));
        assert_eq!(adr.legacy_ids, ["ADR-{n}"]);
        assert_eq!(adr.exclude, ["README.md", "ADR-*-implementation-plan.md"]);
        assert_eq!(adr.docs, 3);
        assert!(adr.required_off.is_empty());
        let specs = row("docs/specs");
        assert_eq!((specs.ty, specs.default_type), (None, DocType::Spec));
        assert_eq!(specs.legacy_keys, ["R-{n}*"]);
        assert_eq!(
            specs.status_map,
            [("shipped".to_string(), "active".to_string())]
        );
        assert_eq!(row("docs/guides").exclude, ["ext-*/**"]);
        let def = row("docs/defects");
        assert_eq!(
            (def.ty, def.legacy_ids.clone()),
            (Some(DocType::Guide), vec!["DEF-{n}".to_string()])
        );
        assert_eq!(row("docs/market").ty, Some(DocType::Reference));
        assert_eq!(f.outcome_aliases, ["Решение"]);
        assert_eq!(f.amendment_aliases, ["Поправки"]);
        assert_eq!(f.stack, ["Rust"]);
        assert_eq!(f.tests, [("**/tests/**".to_string(), 1)]);
        assert_eq!(f.ci, ["GitHub Actions"]);
        assert_eq!(f.hooks, ["pre-commit"]);
        assert!(f.brownfield);
        assert!(f.scripts.iter().any(|s| s.0 == "Cyrillic"));
    }

    #[test]
    fn a_new_repository_is_greenfield() {
        let f = facts(&[("README.md", "# New\n"), ("src/main.rs", "fn main() {}\n")]);
        assert!(!f.brownfield);
        assert!(f.rows.is_empty());
        assert_eq!(over_budget(&f, 1.0), 0);
    }

    #[test]
    fn maps_status_words() {
        assert_eq!(map_status("Shipped", DocType::Spec), Some("active"));
        assert_eq!(map_status("approved", DocType::Decision), Some("accepted"));
        assert_eq!(map_status("closed", DocType::Change), Some("done"));
        assert_eq!(map_status("reference", DocType::Guide), None);
        assert_eq!(number_prefix("ADR-012-cache"), Some("ADR"));
        assert_eq!(number_prefix("ADR-x"), None);
        assert_eq!(number_prefix("adr-1"), None);
    }

    #[test]
    fn root_files_skills_and_fixtures() {
        let mut files: Vec<(String, String)> = vec![
            ("README.md".into(), "# R\n".into()),
            ("AGENTS.md".into(), "# A\n".into()),
            (".claude/skills/spec-dna/SKILL.md".into(), "# S\n".into()),
            (".claude/skills/a/SKILL.md".into(), "# A\n".into()),
            (".claude/skills/b/SKILL.md".into(), "# B\n".into()),
        ];
        for i in 0..6 {
            files.push((format!("page-{i}.md"), "# P\n".into()));
            files.push((format!("tests/fixtures/repo/docs/d{i}.md"), "# D\n".into()));
        }
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let f = facts(&refs);
        let root = f.rows.iter().find(|r| r.path == ".").expect("a root `.`");
        assert!(root.exclude.contains(&"README.md".to_string()), "{root:?}");
        assert!(root.exclude.contains(&"AGENTS.md".to_string()), "{root:?}");
        let skills = f.rows.iter().find(|r| r.path == ".claude/skills").unwrap();
        assert_eq!(skills.exclude, ["spec-dna/**"]);
        assert!(
            f.rows.iter().all(|r| !r.path.contains("fixtures")),
            "{:?}",
            f.rows
        );
    }
}
