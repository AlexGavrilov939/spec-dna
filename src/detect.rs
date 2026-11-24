//! Detection for `spec-dna init` (docs/init.md). It reads the file list and
//! the Markdown files of a repository. It proposes a configuration: the
//! document roots with their types, ids, keys, statuses and exclusions, and
//! the facts that the questions of `init` show as defaults. It only reads, and
//! it never changes a file.
//!
//! The rules are generic. They use the names of directories, the shape of file
//! names, front matter and headings. They also use the usual files of common
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
    ("specs", DocType::Requirements),
    ("spec", DocType::Requirements),
    ("specifications", DocType::Requirements),
    ("requirements", DocType::Requirements),
    ("prd", DocType::Requirements),
    ("prds", DocType::Requirements),
    ("contracts", DocType::Interface),
    ("interfaces", DocType::Interface),
    ("guides", DocType::Procedure),
    ("guidelines", DocType::Procedure),
    ("conventions", DocType::Procedure),
    ("how-to", DocType::Procedure),
    ("howto", DocType::Procedure),
    ("runbooks", DocType::Procedure),
    ("runbook", DocType::Procedure),
    ("playbooks", DocType::Procedure),
    ("oncall", DocType::Procedure),
    ("on-call", DocType::Procedure),
    ("incidents", DocType::Incident),
    ("incident", DocType::Incident),
    ("postmortems", DocType::Incident),
    ("postmortem", DocType::Incident),
    ("post-mortems", DocType::Incident),
    ("rfcs", DocType::Research),
    ("rfc", DocType::Research),
    ("proposals", DocType::Research),
    ("changes", DocType::Research),
    ("tasks", DocType::Research),
    ("plans", DocType::Research),
    ("enhancements", DocType::Research),
    ("invariants", DocType::Rule),
    ("principles", DocType::Rule),
    ("notes", DocType::Research),
    ("research", DocType::Research),
    ("references", DocType::Research),
    ("reference", DocType::Research),
    ("archive", DocType::Research),
    ("meetings", DocType::Research),
    ("meeting-notes", DocType::Research),
];

/// Type words that are also common names of code directories
/// (`src/incident`). They give a type only inside a documentation tree or
/// to a directory that is mostly Markdown.
const CODE_WORDS: &[&str] = &["incident", "postmortem"];

/// Directory names of documentation trees.
const DOC_DIRS: &[&str] = &["docs", "doc", "documentation", "wiki", "handbook"];

/// Directories of agent instructions. Their Markdown files are procedures.
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

/// Name prefixes of vendored sibling directories, such as `vendor-<name>`.
const VENDOR_PREFIXES: &[&str] = &[
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
    /// True for a mixed root that has no `default-type`: the place of a
    /// document gives its type, and a document with no place and no type gives
    /// SD303. `default_type` is not used then.
    pub no_default_type: bool,
    pub status: Option<String>,
    pub legacy_ids: Vec<String>,
    pub legacy_keys: Vec<String>,
    pub status_map: Vec<(String, String)>,
    /// Types whose default required sections the root removes, because most
    /// documents of the root do not have them.
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
            no_default_type: false,
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

/// The default status of a document without `status` in a detected root. The
/// documents of the repository describe the current state, so the status is
/// the "current" status of the type. A decision stays `proposed`, so that its
/// text is not frozen before an author checks it.
fn first_status(ty: DocType) -> Option<String> {
    match ty {
        DocType::Decision => Some("proposed".into()),
        DocType::Intent => Some("accepted".into()),
        t if t.has_status() => Some("active".into()),
        _ => None,
    }
}

/// The Markdown files whose name, in lowercase and without `.md`, is
/// `roadmap`, `questions` or `open-questions`. Vendored and generated
/// directories and a path that the key `shared-paths` would reject are left
/// out. The result is sorted.
fn shared_docs(files: &[String]) -> Vec<String> {
    let mut out: Vec<String> = files
        .iter()
        .filter(|f| !skipped(f) && crate::config::is_valid_glob(f))
        .filter(|f| {
            let name = f.rsplit('/').next().unwrap_or(f).to_ascii_lowercase();
            matches!(
                name.strip_suffix(".md"),
                Some("roadmap" | "questions" | "open-questions")
            )
        })
        .cloned()
        .collect();
    out.sort();
    out
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
    /// The proposed areas of `requirements/`: at most 8 names of one folder
    /// each, or none if `init` finds fewer than 2.
    pub areas: Vec<String>,
    /// The folders that `areas` leaves out because their name differs from
    /// a proposed name only in case: the proposed name and the other name.
    pub areas_dropped: Vec<(String, String)>,
    /// The Markdown files that a roadmap or a list of questions suggests by
    /// their name. `init` writes them in `[change] shared-paths`. Sorted.
    pub shared: Vec<String>,
}

/// One Markdown file.
struct Md {
    path: String,
    parsed: ParsedDoc,
    generated: bool,
    /// The field labels of the file, in lowercase: the first cell of each
    /// table row and each bold label at the start of a line. Record templates
    /// often put their fields (`Root cause`, `Status`) in a table.
    labels: Vec<String>,
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

/// Returns true if a file stem is `<digits>-<word>`, as adr-tools writes it
/// (`0001-record-architecture-decisions`). A date (`2026-01-05-outage`) is
/// not a number of this shape.
fn is_plain_numbered(stem: &str) -> bool {
    let digits = stem.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0
        && stem[digits..]
            .strip_prefix('-')
            .and_then(|r| r.chars().next())
            .is_some_and(char::is_alphabetic)
}

/// Returns true if most file stems are `<digits>-<word>`.
fn plain_numbered(stems: &[&str], min_share: f64) -> bool {
    let n = stems.iter().filter(|s| is_plain_numbered(s)).count();
    n >= 3 && n as f64 >= min_share * stems.len() as f64
}

/// The type that a file-name prefix names, such as `ADR` or `RFC`.
fn prefix_type(prefix: &str) -> Option<DocType> {
    Some(match prefix {
        "ADR" | "MADR" | "AD" | "DR" | "DEC" => DocType::Decision,
        "RFC" | "RFD" | "KEP" | "PEP" | "CR" | "EP" => DocType::Research,
        "INC" | "INCIDENT" | "PM" | "PIR" | "RCA" => DocType::Incident,
        "RB" | "RUNBOOK" => DocType::Procedure,
        "REQ" | "SPEC" | "PRD" | "SRS" => DocType::Requirements,
        "INV" => DocType::Rule,
        _ => return None,
    })
}

/// The type that the content of a file shows, if one type fits.
fn content_type(m: &Md) -> Option<DocType> {
    let heading = |names: &[&str]| {
        m.parsed.chunks.iter().any(|c| {
            c.level > 0
                && names
                    .iter()
                    .any(|n| crate::model::same_heading(&c.heading, n))
        })
    };
    let field = |name: &str| heading(&[name]) || m.labels.iter().any(|l| l == name);
    let decision = heading(&["Decision", "Decision Outcome"])
        || heading(OUTCOME_ALIASES)
        || (heading(&["Context"]) && heading(&["Consequences"]));
    let incident = field("root cause") || (heading(&["Timeline"]) && heading(&["Impact"]));
    let runbook = heading(&["Steps", "Procedure"]);
    let spec = m
        .parsed
        .reqs
        .iter()
        .any(|r| crate::parse::has_bcp14(&r.text));
    let hits: Vec<DocType> = [
        (decision, DocType::Decision),
        (incident, DocType::Incident),
        (runbook, DocType::Procedure),
        (spec, DocType::Requirements),
    ]
    .into_iter()
    .filter(|x| x.0)
    .map(|x| x.1)
    .collect();
    (hits.len() == 1).then(|| hits[0])
}

/// The type of a directory from its own files. The order is: the file-name
/// prefix, the front matter `type` of most files, the content of most files.
/// Returns the type and the reason.
fn own_type(docs: &[&Md], prefixed: Option<&str>) -> Option<(DocType, String)> {
    if let Some(p) = prefixed.and_then(|p| p.strip_suffix("-{n}"))
        && let Some(t) = prefix_type(p)
    {
        return Some((t, format!("the prefix `{p}` names type `{}`", t.name())));
    }
    let majority = |types: Vec<DocType>| -> Option<DocType> {
        let mut counts: BTreeMap<DocType, usize> = BTreeMap::new();
        for t in types {
            *counts.entry(t).or_default() += 1;
        }
        let (t, n) = counts.into_iter().max_by_key(|x| x.1)?;
        (n as f64 >= 0.6 * docs.len() as f64).then_some(t)
    };
    let fm = docs.iter().filter_map(|m| fm_type(&m.parsed)).collect();
    if let Some(t) = majority(fm) {
        return Some((t, format!("most files have `type: {}`", t.name())));
    }
    let shown = docs.iter().filter_map(|m| content_type(m)).collect();
    majority(shown).map(|t| {
        (
            t,
            format!("most files have the sections of type `{}`", t.name()),
        )
    })
}

/// The field labels of a Markdown text (see `Md::labels`).
fn labels(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in text.lines() {
        let l = l.trim_start();
        let label = if let Some(row) = l.strip_prefix('|') {
            row.split('|')
                .next()
                .map(|c| c.trim().trim_matches('*').trim())
        } else {
            let l = l.strip_prefix("- ").unwrap_or(l);
            l.strip_prefix("**")
                .and_then(|r| r.split_once("**"))
                .map(|(b, _)| b.trim().trim_end_matches(':'))
        };
        if let Some(x) = label.filter(|x| !x.is_empty() && x.len() <= 40) {
            out.push(x.to_lowercase());
        }
    }
    out
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
        DocType::Intent => Some(match w {
            "draft" | "open" | "new" | "todo" | "planned" | "pending" | "backlog" => "draft",
            "approved" | "in-progress" | "wip" | "active" | "doing" | "started" => "accepted",
            "complete" | "completed" | "shipped" | "merged" | "closed" | "implemented"
            | "released" | "finished" | "declined" | "abandoned" | "cancelled" | "canceled"
            | "wontfix" => "closed",
            _ => return None,
        }),
        DocType::Spec | DocType::Plan => Some(match w {
            "draft" | "open" | "new" | "todo" | "planned" | "pending" | "wip" => "draft",
            "approved" | "accepted" | "active" => "approved",
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

/// Information from git, not from the files.
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
        // A generated harness file (an agent file) is not project knowledge.
        if crate::harness::has_marker(&text) {
            return None;
        }
        let head: String = text
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // A marker of a generator: "@generated", "auto-generated", or a line
        // that says "generated" and "do not edit". A plain "do not edit" can
        // be an instruction to a reader.
        let generated = head.lines().any(|l| {
            l.contains("auto-generated")
                || l.contains("autogenerated")
                || l.contains("@generated")
                || (l.contains("generated") && l.contains("do not edit"))
        });
        Some(Md {
            path: p.to_string(),
            parsed: crate::parse::parse(&text),
            generated,
            labels: labels(&text),
        })
    })
    .into_iter()
    .flatten()
    .collect();
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
        // A tree whose name is a type word has that type, as a typed
        // directory has (`rfcs/` at the top level).
        let name = d.rsplit('/').next().unwrap_or(d);
        rows.push(match dir_type(name).filter(|_| !d.is_empty()) {
            Some(t) => Row::new(
                d,
                Some(t),
                t,
                format!("directory name `{name}`, {} Markdown files", md_in(d)),
            ),
            None => Row::new(
                if d.is_empty() { "." } else { d },
                None,
                DocType::Procedure,
                format!("{} Markdown files, mostly documentation", md_in(d)),
            ),
        });
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
                Some(DocType::Procedure),
                DocType::Procedure,
                "agent instructions".into(),
            );
            r.why = format!("agent instructions, {} Markdown files", md_in(d));
            rows.push(r);
        } else if let Some(t) = dir_type(name).filter(|_| md_in(d) >= 2)
            && !rows.iter().any(|r| r.path == d)
            && (!CODE_WORDS.contains(&name.to_ascii_lowercase().as_str())
                || enclosing(&rows, d).is_some()
                || md_in(d) * 2 >= all_tree.get(d).copied().unwrap_or(0))
        {
            rows.push(Row::new(
                d,
                Some(t),
                t,
                format!("directory name `{name}`, {} Markdown files", md_in(d)),
            ));
        }
    }
    // 3. Directories of numbered files (`PREFIX-<n>-*.md`, or `<n>-*.md` as
    // adr-tools writes them): legacy ids. The type of such a directory comes
    // from its own files. If the files show no type, it is the type of the
    // root that contains it, if that root has a type. In a mixed root, it is `research`.
    let mut plain_aliases: Vec<(String, &str)> = Vec::new();
    let mut numbered: Vec<(&String, &Vec<usize>)> = md_direct.iter().collect();
    numbered.sort();
    for (d, idx) in numbered {
        let stems: Vec<&str> = idx.iter().map(|&i| stem(&mds[i].path)).collect();
        let prefixed = legacy_id(&stems, 0.6);
        let plain = plain_numbered(&stems, 0.6);
        if prefixed.is_none() && !plain {
            continue;
        }
        let docs: Vec<&Md> = idx.iter().map(|&i| &mds[i]).collect();
        let own = own_type(&docs, prefixed.as_deref());
        // Files without a prefix make a root only for the layouts of
        // decision and change records (adr-tools, RFC directories).
        if prefixed.is_none() && !matches!(own, Some((DocType::Decision, _))) {
            continue;
        }
        let row = match rows.iter().position(|r| r.path == *d) {
            Some(i) => {
                // A mixed tree takes the type of its numbered files only if
                // it has no other documents.
                let r = &mut rows[i];
                if r.ty.is_none()
                    && md_in(d) == idx.len()
                    && let Some((t, why)) = &own
                {
                    *r = Row::new(d, Some(*t), *t, format!("{}; {why}", r.why));
                }
                i
            }
            None => {
                let parent = enclosing(&rows, d).map(|j| rows[j].ty);
                let (t, why) = match (&own, parent) {
                    (Some(o), _) => o.clone(),
                    (None, Some(Some(pt))) => (
                        pt,
                        format!("the type of the enclosing root `{}`", pt.name()),
                    ),
                    (None, Some(None)) => (DocType::Research, "no type in the files".into()),
                    (None, None) => continue,
                };
                let shape = prefixed.clone().unwrap_or_else(|| "{n}-*".into());
                rows.push(Row::new(
                    d,
                    Some(t),
                    t,
                    format!("numbered files `{shape}`; {why}"),
                ));
                rows.len() - 1
            }
        };
        if let Some(p) = prefixed {
            rows[row].legacy_ids.push(p);
        } else if rows[row].ty == Some(DocType::Decision) {
            plain_aliases.push((d.clone(), "ADR-{n}"));
        }
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
            Some(DocType::Research),
            DocType::Research,
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
            && docs
                .iter()
                .any(|m| fm_type(&m.parsed).is_some_and(|x| x != t))
        {
            r.ty = None;
            r.default_type = t;
        }
        // The type of each document: its front matter, or the root default.
        let types: Vec<DocType> = docs
            .iter()
            .map(|m| {
                fm_type(&m.parsed)
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
        // Required sections: decisions without an outcome heading can be other
        // files (a README, a plan). Exclude them by name. If most documents of
        // a type still lack the section, the root removes it.
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
    // An alias for `<n>-*.md` files is safe only if both conditions are true.
    // First, each document of its root that matches `{n}-*` has the shape
    // `<n>-<word>` (not a date) and a number of its own. Second, no other root
    // makes the same alias.
    for (d, a) in &plain_aliases {
        let Some(ri) = rows.iter().position(|r| r.path == *d) else {
            continue;
        };
        let mut seen: HashSet<String> = HashSet::new();
        let safe = mds
            .iter()
            .filter(|m| in_root(&rows, m) == Some(ri))
            .all(|m| {
                let s = stem(&m.path);
                let digits = s.bytes().take_while(u8::is_ascii_digit).count();
                if digits == 0 || !s[digits..].starts_with('-') {
                    return true;
                }
                let n = s[..digits].trim_start_matches('0').to_string();
                is_plain_numbered(s) && seen.insert(n)
            });
        let taken = plain_aliases.iter().filter(|x| x.1 == *a).count() > 1
            || rows
                .iter()
                .any(|r| r.legacy_ids.iter().any(|l| l.split('|').next() == Some(*a)));
        if safe && !taken {
            rows[ri].legacy_ids.push(format!("{a}|{{n}}-*"));
        }
    }
    // Files that are not documents of a root. They are the root files of the
    // repository (README, AGENTS.md and similar files) in a root `.`, and the
    // skill that `init` generates.
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
    (facts.areas, facts.areas_dropped) = areas_of(&facts.rows, &mds, &facts.apps);
    facts.shared = shared_docs(files);

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
/// The most areas that `init` proposes.
const MAX_AREAS: usize = 8;

/// Folder names that are no area of requirements: they hold other kinds of
/// files, or old text.
const NOT_AREAS: &[&str] = &[
    "archive",
    "archived",
    "assets",
    "attachments",
    "drafts",
    "images",
    "img",
    "templates",
];

/// Folder names of an app directory that name a part of an app, not an app.
const NOT_APPS: &[&str] = &[
    "bench", "benches", "doc", "docs", "example", "examples", "lib", "script", "scripts", "src",
    "test", "tests", "tools",
];

/// Returns true if a folder name can be an area: one visible folder name.
fn is_area_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['.', '_'])
        && name.trim() == name
        && !name.contains(['/', '\\'])
        && !NOT_AREAS.contains(&name.to_ascii_lowercase().as_str())
        && !VENDORED.contains(&name)
        && !GENERATED.contains(&name)
        && !FIXTURES.contains(&name)
}

/// The areas of `requirements/`. The first source is the top folders of the
/// roots that hold requirements: the folders with the most documents come
/// first. If they give fewer than 2 areas, the second source is the folders of
/// the apps (the directories of the manifests of the stack). The result has at
/// most 8 names, in the order of the names. A result of fewer than 2 names is
/// empty: one area is the same as no area. Of 2 folders that differ only in
/// case, the result has the first (the most documents, then the name), because
/// the reader of the configuration stops a pair (D30 of 0.7.0). The second
/// result lists each folder that was left out, with the name that was kept.
fn areas_of(rows: &[Row], mds: &[Md], apps: &[String]) -> (Vec<String>, Vec<(String, String)>) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        if r.path == "." || r.ty.unwrap_or(r.default_type) != DocType::Requirements {
            continue;
        }
        let base = format!("{}/", r.path);
        for m in mds {
            if owner(rows, &m.path) != Some(i) {
                continue;
            }
            let rel = &m.path[base.len()..];
            if r.exclude.iter().any(|g| crate::glob::matches(g, rel)) {
                continue;
            }
            if let Some((folder, _)) = rel.split_once('/')
                && is_area_name(folder)
            {
                *counts.entry(folder.to_string()).or_default() += 1;
            }
        }
    }
    let mut named: Vec<(String, usize)> = counts.into_iter().collect();
    // The most documents first, then the name.
    named.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut dropped: Vec<(String, String)> = Vec::new();
    let mut names = one_of_each_case(named.into_iter().map(|n| n.0), &mut dropped);
    if names.len() < 2 {
        dropped.clear();
        let apps = apps
            .iter()
            .map(|a| a.rsplit('/').next().unwrap_or(a))
            .filter(|name| {
                is_area_name(name) && !NOT_APPS.contains(&name.to_ascii_lowercase().as_str())
            });
        names = one_of_each_case(apps.map(str::to_string), &mut dropped);
    }
    if names.len() < 2 {
        return (Vec::new(), Vec::new());
    }
    names.truncate(MAX_AREAS);
    names.sort();
    dropped.retain(|(kept, _)| names.contains(kept));
    (names, dropped)
}

/// The names in order, with no name that differs only in case from an
/// earlier name or is the same as one. Each name that is left out for case
/// goes to `dropped`, with the name that stays.
fn one_of_each_case(
    names: impl Iterator<Item = String>,
    dropped: &mut Vec<(String, String)>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in names {
        match out.iter().find(|o| crate::config::same_area_name(o, &n)) {
            Some(kept) if *kept == n => {}
            Some(kept) => dropped.push((kept.clone(), n)),
            None => out.push(n),
        }
    }
    out
}

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

/// The type that the front matter of a document names. A document that
/// `init` finds can be a document of 0.5. Thus a type name of 0.5 gives its
/// new type, and `spec` gives `requirements`. `init` never proposes `intent`,
/// `spec` or `plan`: they are valid only in an intent folder.
fn fm_type(p: &ParsedDoc) -> Option<DocType> {
    let name = fm_str(p, "type")?;
    let t = match name.as_str() {
        "spec" => DocType::Requirements,
        n => DocType::from_old_name(n).or_else(|| DocType::from_name(n))?,
    };
    (!matches!(t, DocType::Intent | DocType::Spec | DocType::Plan)).then_some(t)
}

fn fm_str(p: &ParsedDoc, key: &str) -> Option<String> {
    match p.front_matter.as_ref()?.get(key)? {
        (crate::frontmatter::Value::Str(s), _) => Some(s.clone()),
        _ => None,
    }
}

/// The files of a project that can hold the setting of the spell checker
/// `typos`, and the pre-commit file that can run it.
pub const TYPOS_FILES: [&str; 5] = [
    ".pre-commit-config.yaml",
    "_typos.toml",
    "typos.toml",
    ".typos.toml",
    "pyproject.toml",
];

/// The pattern that keeps `typos` away from the 16-digit hashes of the
/// markers of spec-dna (docs/init.md, "Spell checkers"). A file that holds it
/// is a file of a project that has set `typos` already.
const TYPOS_GUARD: &str = "[0-9a-f]{16}";

/// True if a `.pre-commit-config.yaml` text runs the hook `typos`. A line scan
/// reads each line, with no YAML parser: a line that is `id: <name>` or
/// `entry: <command>`, after its indent, a leading dash with its space and a
/// comment (a `#` after a space) are removed. The name, or the first word of
/// the command, is `typos` or starts with `typos-`.
pub fn typos_hook(pre_commit: &str) -> bool {
    pre_commit.lines().any(|line| {
        let mut text = line;
        if let Some(at) = text
            .find(" #")
            .or_else(|| text.trim_start().starts_with('#').then_some(0))
        {
            text = &text[..at];
        }
        let mut text = text.trim();
        while let Some(rest) = text.strip_prefix("- ") {
            text = rest.trim_start();
        }
        let Some(value) = text
            .strip_prefix("id:")
            .or_else(|| text.strip_prefix("entry:"))
        else {
            return false;
        };
        let word = value
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(['"', '\'']);
        word == "typos" || word.starts_with("typos-")
    })
}

/// True if the project runs the hook `typos` and has no setting for the hashes
/// of the markers. `read` gives the text of a file of the project, or `None`.
pub fn typos_unguarded(read: &dyn Fn(&str) -> Option<String>) -> bool {
    read(TYPOS_FILES[0]).is_some_and(|t| typos_hook(&t))
        && !TYPOS_FILES
            .iter()
            .any(|f| read(f).is_some_and(|t| t.contains(TYPOS_GUARD)))
}

/// The words that make a pre-commit hook a hook that runs the tests (design
/// 0.8.1, D89). A word is a run of letters and digits of the `id:` or of the
/// `entry:`, in any case.
const TEST_WORDS: [&str; 12] = [
    "test",
    "tests",
    "pytest",
    "nextest",
    "jest",
    "vitest",
    "gotestsum",
    "junit",
    "junitxml",
    "phpunit",
    "rspec",
    "unittest",
];

/// The id of each hook of a `.pre-commit-config.yaml` text that runs the tests
/// and does not run `spec-dna evidence stamp` in its entry, in the order of the
/// file (design 0.8.1, D89). The scan reads lines, as `typos_hook` does: a line
/// `- id: <name>` starts a hook, and the lines `id:` and `entry:` of the hook
/// give its words. A hook whose `entry:` has `evidence stamp` is no finding.
pub fn test_hooks_without_stamp(pre_commit: &str) -> Vec<String> {
    // (id, runs the tests, stamps)
    let mut hooks: Vec<(String, bool, bool)> = Vec::new();
    let mut open = false;
    for line in pre_commit.lines() {
        let mut text = line;
        if let Some(at) = text
            .find(" #")
            .or_else(|| text.trim_start().starts_with('#').then_some(0))
        {
            text = &text[..at];
        }
        let mut text = text.trim();
        let dash = text.starts_with("- ");
        while let Some(rest) = text.strip_prefix("- ") {
            text = rest.trim_start();
        }
        let words = |value: &str| -> bool {
            value
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|w| TEST_WORDS.contains(&w.to_ascii_lowercase().as_str()))
        };
        if let Some(value) = text.strip_prefix("id:") {
            let id = value.trim().trim_matches(['"', '\'']).to_string();
            if dash || !open {
                hooks.push((id.clone(), words(&id), false));
                open = true;
            } else if let Some(h) = hooks.last_mut() {
                h.1 |= words(&id);
            }
        } else if dash {
            // Another list item, such as `- repo:`, ends the hook.
            open = false;
        } else if let Some(value) = text.strip_prefix("entry:")
            && open
            && let Some(h) = hooks.last_mut()
        {
            h.1 |= words(value);
            h.2 |= value.contains("evidence stamp");
        }
    }
    hooks
        .into_iter()
        .filter(|(_, runs, stamps)| *runs && !*stamps)
        .map(|(id, ..)| id)
        .collect()
}

#[cfg(test)]
mod typos_tests {
    use super::*;

    #[test]
    fn the_hook_is_a_line_id_or_entry() {
        for yes in [
            "repos:\n  - repo: https://example.test/typos\n    hooks:\n      - id: typos\n",
            "      - id: typos-src  # the second\n",
            "    - id: \"typos\"\n",
            "    entry: typos --write-changes\n",
            "- id: typos\n",
            "id: typos\r\n",
        ] {
            assert!(typos_hook(yes), "{yes}");
        }
        for no in [
            "      - id: codespell\n",
            "  - repo: https://example.test/crate-ci/typos\n",
            "      # - id: typos\n",
            "      - id: typosaurus\n",
            "    entry: codespell typos\n",
            "    name: typos\n",
            "",
        ] {
            assert!(!typos_hook(no), "{no}");
        }
    }

    #[test]
    fn a_pattern_in_a_file_of_the_list_guards_the_hashes() {
        let hook = "- id: typos\n";
        let files = |pairs: &[(&str, &str)]| {
            let m: HashMap<String, String> = pairs
                .iter()
                .map(|(p, t)| (p.to_string(), t.to_string()))
                .collect();
            move |p: &str| m.get(p).cloned()
        };
        assert!(typos_unguarded(&files(&[(
            ".pre-commit-config.yaml",
            hook
        )])));
        let guard = "[default]\nextend-ignore-re = ['(?:h:|@)[0-9a-f]{16}\\b']\n";
        for name in ["_typos.toml", "typos.toml", ".typos.toml", "pyproject.toml"] {
            assert!(
                !typos_unguarded(&files(&[(".pre-commit-config.yaml", hook), (name, guard)])),
                "{name}"
            );
        }
        let in_hook = format!("{hook}  args: ['--exclude', '[0-9a-f]{{16}}']\n");
        assert!(!typos_unguarded(&files(&[(
            ".pre-commit-config.yaml",
            &in_hook
        )])));
        // No hook, no note.
        assert!(!typos_unguarded(&files(&[("_typos.toml", "")])));
        assert!(!typos_unguarded(&files(&[(
            ".pre-commit-config.yaml",
            "- id: codespell\n"
        )])));
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
                "docs/specs/pumps.md".into(),
                "---\nstatus: shipped\n---\n# Pumps\n\n- **R-1** The controller SHALL start the pump.\n- **R-2** x\n- **R-2a** y\n".into(),
            ),
            (
                "docs/specs/review.md".into(),
                "---\ntype: research\n---\n# Review\n".into(),
            ),
            ("docs/guides/vendor-a/one.md".into(), "# One\n".into()),
            ("docs/guides/vendor-b/two.md".into(), "# Two\n".into()),
            ("docs/guides/style.md".into(), "# Style\n".into()),
            ("docs/defects/DEF-1-a.md".into(), "# A\n".into()),
            ("docs/defects/DEF-2-b.md".into(), "# B\n".into()),
            ("docs/defects/DEF-3-c.md".into(), "# C\n".into()),
        ];
        for i in 1..=3 {
            files.push((
                format!("docs/adr/ADR-{i}-x.md"),
                "# X\n\n## Decisión\n\nYes.\n\n## Enmiendas\n\nMore.\n\n## Σημειώσεις\n\nText.\n"
                    .into(),
            ));
        }
        for i in 0..LARGE_FILES {
            files.push((format!("docs/datasheets/m{i}.md"), "# M\n".into()));
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
        assert_eq!(row("docs").default_type, DocType::Procedure);
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
        assert_eq!(
            (specs.ty, specs.default_type),
            (None, DocType::Requirements)
        );
        assert_eq!(specs.legacy_keys, ["R-{n}*"]);
        assert_eq!(
            specs.status_map,
            [("shipped".to_string(), "active".to_string())]
        );
        assert_eq!(row("docs/guides").exclude, ["vendor-*/**"]);
        // A numbered directory takes its type from its own files. These files
        // show no type, so the type is `research`, not the type of `docs`.
        let def = row("docs/defects");
        assert_eq!(
            (def.ty, def.legacy_ids.clone()),
            (Some(DocType::Research), vec!["DEF-{n}".to_string()])
        );
        assert_eq!(row("docs/datasheets").ty, Some(DocType::Research));
        assert_eq!(f.outcome_aliases, ["Decisión"]);
        assert_eq!(f.amendment_aliases, ["Enmiendas"]);
        assert_eq!(f.stack, ["Rust"]);
        assert_eq!(f.tests, [("**/tests/**".to_string(), 1)]);
        assert_eq!(f.ci, ["GitHub Actions"]);
        assert_eq!(f.hooks, ["pre-commit"]);
        assert!(f.brownfield);
        assert!(f.scripts.iter().any(|s| s.0 == "Greek"));
    }

    fn rows_of(files: &[(String, String)]) -> Vec<(String, Option<DocType>, Vec<String>)> {
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        facts(&refs)
            .rows
            .into_iter()
            .map(|r| (r.path, r.ty, r.legacy_ids))
            .collect()
    }

    const ADR: &str = "# T\n\n## Status\n\nAccepted\n\n## Context\n\nC.\n\n## Decision\n\nD.\n\n## Consequences\n\nE.\n";

    #[test]
    fn detects_adr_tools_layout() {
        let mut files = vec![("README.md".to_string(), "# R\n".to_string())];
        for (i, s) in ["record-architecture-decisions", "use-markdown", "use-rust"]
            .iter()
            .enumerate()
        {
            files.push((format!("doc/adr/000{}-{s}.md", i + 1), ADR.into()));
        }
        assert_eq!(
            rows_of(&files),
            [(
                "doc/adr".to_string(),
                Some(DocType::Decision),
                vec!["ADR-{n}|{n}-*".to_string()]
            )]
        );
        // Two roots of `<n>-*.md` decisions would make the same aliases.
        for i in 1..=3 {
            files.push((format!("services/api/doc/adr/000{i}-x.md"), ADR.into()));
        }
        let rows = rows_of(&files);
        assert!(rows.iter().all(|r| r.2.is_empty()), "{rows:?}");
    }

    #[test]
    fn top_level_type_directories_keep_their_type() {
        let mut files: Vec<(String, String)> = Vec::new();
        for i in 1..=5 {
            files.push((format!("adrs/ADR-000{i}-choice.md"), ADR.into()));
            files.push((
                format!("rfcs/000{i}-feature.md"),
                "# F\n\n## Motivation\n\nM.\n".into(),
            ));
            files.push((
                format!("runbooks/task-{i}.md"),
                "# R\n\n## Steps\n\n1. Do.\n".into(),
            ));
            files.push((
                format!("incidents/2026-01-0{i}-outage.md"),
                "# O\n\n## Summary\n\nS.\n\n## Root cause\n\nR.\n".into(),
            ));
        }
        for d in ["postmortems", "meetings", "research"] {
            for i in 1..=2 {
                files.push((format!("{d}/item-{i}.md"), "# I\n\nText.\n".into()));
            }
        }
        let rows = rows_of(&files);
        let want = [
            ("adrs", DocType::Decision, vec!["ADR-{n}"]),
            ("incidents", DocType::Incident, vec![]),
            ("meetings", DocType::Research, vec![]),
            ("postmortems", DocType::Incident, vec![]),
            ("research", DocType::Research, vec![]),
            ("rfcs", DocType::Research, vec![]),
            ("runbooks", DocType::Procedure, vec![]),
        ];
        let want: Vec<(String, Option<DocType>, Vec<String>)> = want
            .into_iter()
            .map(|(p, t, l)| {
                (
                    p.to_string(),
                    Some(t),
                    l.into_iter().map(str::to_string).collect(),
                )
            })
            .collect();
        assert_eq!(rows, want);
    }

    #[test]
    fn numbered_directories_take_the_type_of_their_files() {
        let mut files: Vec<(String, String)> = vec![
            ("docs/index.md".into(), "# Docs\n".into()),
            ("docs/guide.md".into(), "# Guide\n".into()),
            (
                "docs/specs/vents.md".into(),
                "# Vents\n\n- **open-vent**: The controller SHALL open the vent.\n".into(),
            ),
            (
                "docs/specs/shades.md".into(),
                "# Shades\n\n- **close-shade**: The controller SHALL close the shade.\n".into(),
            ),
        ];
        for i in 1..=3 {
            files.push((format!("docs/architecture/ADR-000{i}-x.md"), ADR.into()));
            files.push((
                format!("docs/records/BUG-{i}-crash.md"),
                "# BUG\n\n| Field | Value |\n|---|---|\n| Symptom | S |\n| Root cause | R |\n"
                    .into(),
            ));
            files.push((
                format!("docs/reviews/REV-{i}-sign-off.md"),
                "# Sign-off\n\nApproved.\n".into(),
            ));
        }
        let rows = rows_of(&files);
        let get = |p: &str| rows.iter().find(|r| r.0 == p).cloned();
        assert_eq!(
            get("docs/architecture"),
            Some((
                "docs/architecture".into(),
                Some(DocType::Decision),
                vec!["ADR-{n}".into()]
            ))
        );
        assert_eq!(
            get("docs/records").map(|r| r.1),
            Some(Some(DocType::Incident))
        );
        assert_eq!(
            get("docs/reviews").map(|r| r.1),
            Some(Some(DocType::Research))
        );
        assert_eq!(
            get("docs/specs").map(|r| r.1),
            Some(Some(DocType::Requirements))
        );
        assert!(!plain_numbered(
            &["2026-01-05-outage", "2026-02-01-x", "2026-03-01-y"],
            0.6
        ));
    }

    #[test]
    fn review_cases_keep_the_earlier_behaviour() {
        let row =
            |files: &[(String, String)], p: &str| rows_of(files).into_iter().find(|r| r.0 == p);
        // Repeated numbers and dates in the root of `<n>-*.md` decisions:
        // no alias, so that `check` finds no duplicate id.
        let mut files: Vec<(String, String)> = Vec::new();
        for i in 1..=3 {
            files.push((format!("doc/adr/000{i}-x.md"), ADR.into()));
        }
        for i in 1..=2 {
            files.push((format!("doc/adr/superseded/000{i}-old.md"), ADR.into()));
        }
        assert_eq!(row(&files, "doc/adr").map(|r| r.2), Some(vec![]));
        files.truncate(3);
        files.push(("doc/adr/2025-03-01-review.md".into(), ADR.into()));
        assert_eq!(row(&files, "doc/adr").map(|r| r.2), Some(vec![]));
        // A tree with numbered chapters and other documents stays mixed.
        let mut files: Vec<(String, String)> = Vec::new();
        for (i, s) in ["overview", "install", "deploy", "upgrade"]
            .iter()
            .enumerate()
        {
            files.push((
                format!("docs/0{}-{s}.md", i + 1),
                "# S\n\n## Steps\n\n1. Do.\n".into(),
            ));
        }
        files.push(("docs/api/sensors.md".into(), "# Sensors\n".into()));
        files.push(("docs/api/orders.md".into(), "# Orders\n".into()));
        assert_eq!(row(&files, "docs").map(|r| r.1), Some(None));
        // Numbered files without a type in a typed root keep its type.
        let mut files: Vec<(String, String)> = vec![
            ("docs/specs/a.md".into(), "# A\n".into()),
            ("docs/specs/b.md".into(), "# B\n".into()),
        ];
        for i in 1..=3 {
            files.push((format!("docs/specs/features/FEAT-{i}-x.md"), "# F\n".into()));
        }
        assert_eq!(
            row(&files, "docs/specs/features").map(|r| r.1),
            Some(Some(DocType::Requirements))
        );
        // A code directory with a type word is not a root.
        let files: Vec<(String, String)> = vec![
            ("src/incident/README.md".into(), "# I\n".into()),
            ("src/incident/DESIGN.md".into(), "# D\n".into()),
            ("src/incident/mod.rs".into(), "\n".into()),
            ("src/incident/store.rs".into(), "\n".into()),
            ("src/incident/api.rs".into(), "\n".into()),
        ];
        assert_eq!(row(&files, "src/incident"), None);
    }

    #[test]
    fn a_new_repository_is_greenfield() {
        let f = facts(&[("README.md", "# New\n"), ("src/main.rs", "fn main() {}\n")]);
        assert!(!f.brownfield);
        assert!(f.rows.is_empty());
        assert_eq!(over_budget(&f, 1.0), 0);
    }

    #[test]
    fn a_generated_harness_file_is_not_a_document() {
        let generated = "---\nname: verifier\n---\n\nText.\n\n<!-- spec-dna harness v0.5.0 h:0123456789abcdef -->\n";
        let own = "---\nname: other\n---\n\nText.\n";
        let with = |verifier: &str| {
            facts(&[
                (".claude/agents/verifier.md", verifier),
                (".claude/agents/other.md", own),
            ])
            .markdown
        };
        assert_eq!(with(generated), 1, "the generated agent file is skipped");
        assert_eq!(with("---\nname: verifier\n---\n\nText.\n"), 2);
    }

    #[test]
    fn maps_status_words() {
        assert_eq!(map_status("Shipped", DocType::Requirements), Some("active"));
        assert_eq!(map_status("approved", DocType::Decision), Some("accepted"));
        assert_eq!(map_status("closed", DocType::Intent), Some("closed"));
        assert_eq!(map_status("open", DocType::Intent), Some("draft"));
        assert_eq!(map_status("reference", DocType::Procedure), None);
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

    #[test]
    fn areas_propose_one_name_of_a_pair_that_differs_only_in_case() {
        let doc = "# Note\n\n## Requirements\n\n- **open**: The controller SHALL open the valve.\n";
        let files: Vec<(String, String)> = [
            "docs/specs/valves/a.md",
            "docs/specs/valves/b.md",
            "docs/specs/Valves/c.md",
            "docs/specs/sensors/d.md",
            "docs/specs/pumps/e.md",
            "docs/specs/Pumps/f.md",
        ]
        .iter()
        .map(|p| (p.to_string(), doc.to_string()))
        .collect();
        let refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let f = facts(&refs);
        // The most documents win, then the name: `valves` has 2 documents, and
        // `Pumps` is before `pumps`.
        assert_eq!(f.areas, ["Pumps", "sensors", "valves"], "{:?}", f.areas);
    }
}

#[cfg(test)]
mod test_hook_tests {
    use super::*;

    #[test]
    fn a_hook_that_runs_the_tests_and_does_not_stamp_is_named() {
        let yaml = "repos:\n  - repo: local\n    hooks:\n      - id: tests\n        entry: pytest -q\n      - id: format\n        entry: rustfmt\n      - id: slow\n        entry: cargo nextest run  # all\n      - id: stamped\n        entry: sh -c 'jest && spec-dna evidence stamp'\n  - repo: https://example.test/x\n    hooks:\n      - id: lint\n";
        assert_eq!(test_hooks_without_stamp(yaml), ["tests", "slow"]);
        // The words come from the id too, and a part of a word is no word.
        let yaml = "- id: run-unit-tests\n  entry: make\n- id: attest\n  entry: make\n- id: contest-docs\n";
        assert_eq!(test_hooks_without_stamp(yaml), ["run-unit-tests"]);
        // A comment does not count, and an empty file has no hook.
        assert!(test_hooks_without_stamp("      # - id: tests\n").is_empty());
        assert!(test_hooks_without_stamp("").is_empty());
        // The entry of the hook before is not the entry of the next hook.
        let yaml = "- id: a\n  entry: sh -c 'pytest && spec-dna evidence stamp'\n- id: tests\n  entry: pytest\n";
        assert_eq!(test_hooks_without_stamp(yaml), ["tests"]);
    }
}
