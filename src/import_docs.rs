//! The documents of `import` (design 0.7.0, D21 to D26; decisions I3 to I6,
//! I8, I15 and I17). The plan is pure: it reads a tree and writes nothing.
//! The readers of the 2 tools (`import_speckit`, `import_openspec`) give a
//! `Read`: the target files, the state of each source file and the pairs of
//! requirements. `plan_docs` takes the `Read`, the final configuration and the
//! tree, and it gives the moves, the writes, the edits of links, and the
//! clashes (2 sources with one target, a target that exists).
//!
//! The module also holds what the readers share: the place of a target
//! (`Places`), the split of a Markdown text in sections, and the front matter
//! of a target.

use crate::config::Config;
use crate::frontmatter;
use crate::hash::nfc;
use crate::model::{Corpus, DocType, is_doc_path, root_doc_id};
use crate::mv::{self, add_aliases, rewrite_links};
use crate::parse::{parse, strip_link_destinations};
use crate::repo::Fatal;
use crate::upgrade_docs::Tree;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// The state of a source file in the report (D22, step 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Moved,
    Written,
    Kept,
    Skipped,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Moved => "moved",
            State::Written => "written",
            State::Kept => "kept",
            State::Skipped => "skipped",
        }
    }
}

/// A source file, or a source folder (`path` ends with `/`), with its state.
/// A folder covers each file below it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub state: State,
    /// The targets: where a moved file goes, or the files that a written file
    /// gives.
    pub to: Vec<String>,
    pub reason: String,
}

impl SourceFile {
    pub fn kept(path: &str, reason: &str) -> SourceFile {
        SourceFile {
            path: path.to_string(),
            state: State::Kept,
            to: Vec::new(),
            reason: reason.to_string(),
        }
    }

    pub fn skipped(path: &str, reason: &str) -> SourceFile {
        SourceFile {
            path: path.to_string(),
            state: State::Skipped,
            to: Vec::new(),
            reason: reason.to_string(),
        }
    }

    /// True if the source path is this file, or a file below this folder.
    pub fn covers(&self, file: &str) -> bool {
        if self.path.ends_with('/') {
            file.starts_with(&self.path)
        } else {
            self.path == file
        }
    }
}

/// A file that the plan writes: a moved source with its new text, or a new
/// file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub path: String,
    pub text: String,
    /// The source that `git mv` moves to this path. `None`: a new file.
    pub moved_from: Option<String>,
    /// The source files that give the text.
    pub sources: Vec<String>,
    /// The source file whose links go to this target (a written file has one
    /// primary target). A moved file uses `moved_from`.
    pub link_of: Option<String>,
    /// The number of scenario headings that the source has for this target
    /// (OpenSpec). `None`: the proof does not count.
    pub scenarios: Option<usize>,
}

/// What a pair compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairKind {
    /// A requirement of the target document, by its key.
    Requirement,
    /// A line `ADD`, `CHANGE` or `REMOVE` of the section `Delta`.
    Delta,
}

/// A requirement of a source and its place in the target (D22, step 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    /// `<path>:<line>` of the source.
    pub from: String,
    /// `<doc id>#<key>` of the target.
    pub id: String,
    /// The slug of the change for a delta line, else `None`.
    pub delta: Option<String>,
    /// The path of the target file.
    pub target: String,
    pub key: String,
    pub kind: PairKind,
    /// The sentence of the source. The proof reads it in its normal form.
    pub sentence: String,
    /// The text of a marker `[NEEDS CLARIFICATION: text]` of the sentence.
    pub question: Option<String>,
    /// The verb of a delta line.
    pub verb: Option<&'static str>,
}

/// What a reader gives.
#[derive(Clone, Debug, Default)]
pub struct Read {
    pub targets: Vec<Target>,
    pub sources: Vec<SourceFile>,
    pub pairs: Vec<Pair>,
    /// The lines of the report.
    pub notes: Vec<String>,
    /// The folders of the sources that stay after a write: the report prints
    /// `git rm` for them.
    pub removes: Vec<String>,
    /// True if a done feature has `FR-nnn` lines.
    pub fr_lines: bool,
    /// The files under the folders of the source tool that the reader read
    /// or classified. The proof checks that each has one state.
    pub universe: Vec<String>,
    /// More links: a written source and the target that its links go to (a
    /// target has one primary source in `Target::link_of`).
    pub links: Vec<(String, String)>,
}

/// The change of `spec-dna.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigChange {
    pub old: String,
    pub new: String,
    pub notes: Vec<String>,
}

impl ConfigChange {
    pub fn changed(&self) -> bool {
        self.old != self.new
    }
}

/// The whole plan of one run.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub config: ConfigChange,
    /// `git mv`, in path order: (from, to).
    pub moves: Vec<(String, String)>,
    /// The new text of each file that the plan writes (after the moves): a
    /// moved target, a new file, and a document with an edited link.
    pub writes: BTreeMap<String, String>,
    pub targets: Vec<Target>,
    pub sources: Vec<SourceFile>,
    pub pairs: Vec<Pair>,
    /// The existing documents whose links changed.
    pub edited: Vec<String>,
    /// The links of the files that are no document to a moved file:
    /// (file, link).
    pub outside_links: Vec<(String, String)>,
    pub notes: Vec<String>,
    /// The paths of `git rm`: files, and a folder only if it holds no kept
    /// file.
    pub removes: Vec<String>,
    /// True if `removes` has a folder (`git rm -r`).
    pub removes_recursive: bool,
    /// A source that is a document of the project, and the target that keeps
    /// its id: the plan writes `moved-to` in the source, so its id is no
    /// duplicate of the alias of the target, and the removal of the source
    /// is no lost id (SD202).
    pub stubs: BTreeMap<String, String>,
    pub universe: Vec<String>,
}

impl Plan {
    /// True if a run has nothing to write.
    pub fn is_empty(&self) -> bool {
        !self.config.changed() && self.moves.is_empty() && self.writes.is_empty()
    }

    /// The paths of a write, in path order: the config, the moved and new
    /// targets and the edited documents.
    pub fn written_paths(&self) -> Vec<String> {
        let mut set: BTreeSet<String> = self.writes.keys().cloned().collect();
        set.extend(self.moves.iter().map(|(_, to)| to.clone()));
        set.into_iter().collect()
    }
}

/// The tree after the plan: its files, and the bytes of a file that the plan
/// writes or moves.
pub struct Planned {
    pub files: Vec<String>,
    /// The text of each file that is not the file of the tree at that path:
    /// the config, a write and a moved file.
    pub texts: HashMap<String, String>,
    /// For a moved file: where its bytes are in the tree.
    pub origin: HashMap<String, String>,
}

pub const CONFIG_PATH: &str = "spec-dna.toml";

/// The planned tree: the end state of the plan, with the new configuration and
/// each move and write. The proof reads it also for a first run that writes
/// only `spec-dna.toml`, because the second run then writes the rest.
pub fn apply(tree: &Tree, plan: &Plan) -> Planned {
    let moved: HashMap<&str, &str> = plan
        .moves
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();
    let mut files: Vec<String> = tree
        .files
        .iter()
        .filter(|f| !moved.contains_key(f.as_str()))
        .cloned()
        .collect();
    files.extend(plan.moves.iter().map(|(_, to)| to.clone()));
    files.extend(plan.writes.keys().cloned());
    if !files.iter().any(|f| f == CONFIG_PATH) {
        files.push(CONFIG_PATH.to_string());
    }
    files.sort();
    files.dedup();
    let mut texts: HashMap<String, String> = plan.writes.clone().into_iter().collect();
    texts.insert(CONFIG_PATH.to_string(), plan.config.new.clone());
    let origin = plan
        .moves
        .iter()
        .map(|(f, t)| (t.clone(), f.clone()))
        .collect();
    Planned {
        files,
        texts,
        origin,
    }
}

impl Planned {
    /// The bytes of a path of the planned tree.
    pub fn read(&self, tree: &Tree, paths: &[String]) -> Result<HashMap<String, Vec<u8>>, Fatal> {
        let mut out = HashMap::new();
        let mut want: Vec<String> = Vec::new();
        for p in paths {
            if let Some(t) = self.texts.get(p) {
                out.insert(p.clone(), t.clone().into_bytes());
            } else {
                want.push(self.origin.get(p).unwrap_or(p).clone());
            }
        }
        let got = (tree.read)(&want)?;
        for p in paths {
            if out.contains_key(p) {
                continue;
            }
            let from = self.origin.get(p).unwrap_or(p);
            if let Some(b) = got.get(from) {
                out.insert(p.clone(), b.clone());
            }
        }
        Ok(out)
    }
}

// --- places ---

/// The folder of each layer that holds a type that the import writes (decision
/// I3): the first root that can hold the type, and the folder of the layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Places {
    pub rules: Option<String>,
    pub requirements: Option<String>,
    pub intent: Option<String>,
    pub research: Option<String>,
    /// The index of the root of `requirements`.
    pub requirements_root: Option<usize>,
}

/// The path of the root that the import adds when no root holds a type that a
/// layer has: the first of `docs` and `docs-spec-dna` that no root uses.
pub fn fallback_root(cfg: &Config) -> String {
    let used = |p: &str| cfg.roots.iter().any(|r| r.path == p);
    if used("docs") {
        "docs-spec-dna".to_string()
    } else {
        "docs".to_string()
    }
}

/// The types that a layer holds and that no root can hold: the types of
/// `want` that need a root.
pub fn types_without_root(cfg: &Config, want: &[DocType]) -> Vec<DocType> {
    want.iter()
        .copied()
        .filter(|&t| {
            place_of(cfg, t).is_none()
                && cfg
                    .layers
                    .layer_of_type(t)
                    .and_then(|i| cfg.layers.layers()[i].dir())
                    .is_some()
        })
        .collect()
}

fn join_path(a: &str, b: &str) -> String {
    let a = a.trim_matches('/');
    if a.is_empty() || a == "." {
        b.to_string()
    } else if b.is_empty() {
        a.to_string()
    } else {
        format!("{a}/{b}")
    }
}

/// The folder for a type: a root of that type (it has no layer folder), else
/// the first mixed root, with the folder of the layer that has the type.
fn place_of(cfg: &Config, ty: DocType) -> Option<(usize, String)> {
    if let Some((i, root)) = cfg
        .roots
        .iter()
        .enumerate()
        .find(|(_, r)| r.single_type == Some(ty))
    {
        return Some((i, join_path(&root.path, "")));
    }
    let (i, root) = cfg
        .roots
        .iter()
        .enumerate()
        .find(|(_, r)| r.single_type.is_none())?;
    let layer = cfg.layers.layer_of_type(ty)?;
    let dir = cfg.layers.layers()[layer].dir()?;
    Some((i, join_path(&root.path, dir)))
}

impl Places {
    pub fn of(cfg: &Config) -> Places {
        let req = place_of(cfg, DocType::Requirements);
        Places {
            rules: place_of(cfg, DocType::Rule).map(|p| p.1),
            requirements_root: req.as_ref().map(|p| p.0),
            requirements: req.map(|p| p.1),
            intent: place_of(cfg, DocType::Intent).map(|p| p.1),
            research: place_of(cfg, DocType::Research).map(|p| p.1),
        }
    }

    fn need<'a>(dir: &'a Option<String>, layer: &str, ty: &str) -> Result<&'a str, Fatal> {
        dir.as_deref().ok_or_else(|| {
            Fatal::Usage(format!(
                "no layer of spec-dna.toml holds the type `{ty}`, and import writes a document of this type; run `spec-dna layer add {layer} --types {ty}`, commit the change, then run import again"
            ))
        })
    }

    pub fn rules_dir(&self) -> Result<&str, Fatal> {
        Self::need(&self.rules, "rules", "rule")
    }

    pub fn requirements_dir(&self) -> Result<&str, Fatal> {
        Self::need(&self.requirements, "requirements", "requirements")
    }

    pub fn intent_dir(&self) -> Result<&str, Fatal> {
        Self::need(&self.intent, "intent", "intent")
    }

    pub fn research_dir(&self) -> Result<&str, Fatal> {
        Self::need(&self.research, "research", "research")
    }
}

/// The path of a `requirements` document: `<dir>[/<area>]/<name>.md`.
pub fn requirements_path(dir: &str, area: Option<&str>, name: &str) -> String {
    match area {
        Some(a) => format!("{dir}/{a}/{name}.md"),
        None => format!("{dir}/{name}.md"),
    }
}

/// The id of a document at `path`, as the corpus reads it.
pub fn id_of(cfg: &Config, path: &str) -> Option<String> {
    let root = cfg.root_of(path)?;
    Some(nfc(&root_doc_id(&cfg.roots[root], path)).into_owned())
}

/// True if the file is a document of the project: Markdown, in a root, and
/// not excluded.
pub fn is_document(cfg: &Config, path: &str) -> bool {
    is_doc_path(path) && cfg.root_of(path).is_some_and(|r| !cfg.is_excluded(r, path))
}

// --- Markdown text ---

/// A heading of a Markdown text: the level, the text, and the index of its
/// line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    pub line: usize,
}

/// The lines of a text, and its headings. A heading is a line with 1 to 6 `#`
/// and a space, outside a fenced code block.
pub struct Lines<'a> {
    pub lines: Vec<&'a str>,
    pub headings: Vec<Heading>,
}

/// The start of a code fence in a line: the fence characters.
fn fence_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with("```") || t.starts_with("~~~") {
        let n = t
            .chars()
            .take_while(|c| *c == t.chars().next().unwrap())
            .count();
        Some(&t[..n])
    } else {
        None
    }
}

/// The heading of each line of a text: `None` for a line that is no heading,
/// and for each line of a fenced code block (a `#` there starts a comment).
pub fn headings_by_line(text: &str) -> Vec<Option<(usize, String)>> {
    let mut fence: Option<String> = None;
    text.lines()
        .map(|l| {
            if let Some(f) = &fence {
                if fence_of(l).is_some_and(|g| g.starts_with(f.as_str())) {
                    fence = None;
                }
                return None;
            }
            if let Some(f) = fence_of(l) {
                fence = Some(f.to_string());
                return None;
            }
            heading_of(l)
        })
        .collect()
}

/// The ATX heading of a line: its level and its text, with no closing `#`.
pub fn heading_of(line: &str) -> Option<(usize, String)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let t = &line[indent..];
    let level = t.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &t[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim_end().to_string();
    Some((level, text))
}

impl<'a> Lines<'a> {
    pub fn new(text: &'a str) -> Lines<'a> {
        let lines: Vec<&str> = text.lines().collect();
        let mut headings = Vec::new();
        let mut fence: Option<String> = None;
        for (i, l) in lines.iter().enumerate() {
            if let Some(f) = &fence {
                if fence_of(l).is_some_and(|g| g.starts_with(f.as_str())) {
                    fence = None;
                }
                continue;
            }
            if let Some(f) = fence_of(l) {
                fence = Some(f.to_string());
                continue;
            }
            if let Some((level, text)) = heading_of(l) {
                headings.push(Heading {
                    level,
                    text,
                    line: i,
                });
            }
        }
        Lines { lines, headings }
    }

    /// The end of the section of the heading `h` (an index into `headings`):
    /// the line of the next heading of a level up to its own, or the end. The
    /// sub-sections are inside.
    pub fn tree_end(&self, h: usize) -> usize {
        let level = self.headings[h].level;
        self.headings[h + 1..]
            .iter()
            .find(|x| x.level <= level)
            .map_or(self.lines.len(), |x| x.line)
    }

    /// The end of the text of the heading `h` itself: the line of the next
    /// heading of any level.
    pub fn own_end(&self, h: usize) -> usize {
        self.headings
            .get(h + 1)
            .map_or(self.lines.len(), |x| x.line)
    }

    /// The lines `from..to`, joined with a line end, with no blank line at the
    /// ends.
    pub fn block(&self, from: usize, to: usize) -> String {
        trim_blank(&self.lines[from.min(to)..to].join("\n"))
    }
}

/// The text with no blank line at its ends and no line end after it. The
/// indent of the first line stays.
pub fn trim_blank(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    match (first, last) {
        (Some(f), Some(l)) => lines[f..=l]
            .iter()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The heading text without a closing annotation such as `*(mandatory)*`.
pub fn plain_heading(text: &str) -> String {
    let t = text.trim();
    if let Some(i) = t.rfind("*(")
        && t.ends_with(")*")
    {
        return t[..i].trim_end().to_string();
    }
    t.to_string()
}

/// A key from a name: lowercase letters and digits joined by one hyphen. The
/// result is empty if the name has no ASCII letter or digit.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut hyphen = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if hyphen && !out.is_empty() {
                out.push('-');
            }
            hyphen = false;
            out.push(c.to_ascii_lowercase());
        } else {
            hyphen = true;
        }
    }
    out
}

/// A line of the text that starts a list item at the left margin.
pub fn is_list_line(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") || t == "-" {
        return true;
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && (t[digits..].starts_with(". ") || t[digits..].starts_with(") "))
}

/// Indents each line that has text. A blank line stays empty.
pub fn indent(text: &str, by: usize) -> String {
    let pad = " ".repeat(by);
    text.lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of a principle or a requirement body, split at its first list: the
/// paragraphs before it (the sentence), and the lines from the first list on.
pub fn split_sentence(body: &str) -> (String, String) {
    let mut fence = false;
    let lines: Vec<&str> = body.lines().collect();
    let mut cut = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if fence_of(l).is_some() {
            fence = !fence;
            continue;
        }
        if !fence && is_list_line(l) {
            cut = i;
            break;
        }
    }
    (
        trim_blank(&lines[..cut].join("\n")),
        trim_blank(&lines[cut..].join("\n")),
    )
}

/// The share of the bytes of `a` in lines that `b` keeps, against the larger
/// of the 2 texts, from 0 to 1. Git keeps a rename if the similarity of the 2
/// files is at least one half (it counts bytes of equal chunks), so the report
/// warns about an edit that is larger.
pub fn similarity(a: &str, b: &str) -> f64 {
    let mut left: HashMap<&str, usize> = HashMap::new();
    for l in b.lines() {
        *left.entry(l).or_default() += 1;
    }
    let size = |t: &str| t.lines().map(|l| l.len() + 1).sum::<usize>();
    let total = size(a).max(size(b));
    if total == 0 {
        return 1.0;
    }
    let mut same = 0usize;
    for l in a.lines() {
        if let Some(n) = left.get_mut(l)
            && *n > 0
        {
            *n -= 1;
            same += l.len() + 1;
        }
    }
    same as f64 / total as f64
}

/// The text of a sentence without its markers `[NEEDS CLARIFICATION: text]`
/// (the white space is joined), and the texts of the markers, joined with
/// `; `.
pub fn strip_clarification(text: &str) -> (String, Option<String>) {
    const OPEN: &str = "[NEEDS CLARIFICATION";
    let mut out = String::new();
    let mut texts: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(OPEN) {
        let after = &rest[i + OPEN.len()..];
        let mut depth = 0usize;
        let mut end = None;
        for (j, c) in after.char_indices() {
            if c == '[' {
                depth += 1;
            } else if c == ']' {
                if depth == 0 {
                    end = Some(j);
                    break;
                }
                depth -= 1;
            }
        }
        let Some(j) = end else { break };
        out.push_str(&rest[..i]);
        let inner = after[..j].trim_start_matches(':').trim();
        if !inner.is_empty() {
            texts.push(inner.to_string());
        }
        rest = &after[j + 1..];
        out = out.trim_end().to_string();
        if !rest.starts_with(['.', ',', ';', ':', ')', '!', '?']) {
            out.push(' ');
        }
    }
    out.push_str(rest);
    (
        crate::hash::normalize(&out),
        (!texts.is_empty()).then(|| texts.join("; ")),
    )
}

/// The number of scenario lines of a text, outside code: a heading
/// `#### Scenario:` of a source, and an item `- Scenario:` of a target.
pub fn count_scenarios(text: &str) -> usize {
    let mut fence = false;
    let mut n = 0;
    for l in text.lines() {
        if fence_of(l).is_some() {
            fence = !fence;
            continue;
        }
        if fence {
            continue;
        }
        let t = l.trim_start();
        if t.starts_with("#### Scenario:") || t.starts_with("- Scenario:") {
            n += 1;
        }
    }
    n
}

// --- the front matter of a target ---

/// The text with the front matter of a target: `status`, the lines of
/// `extra` (such as `class`), and the aliases. A text that has a front matter
/// keeps its lines, and it gets only the keys that it lacks.
pub fn with_front_matter(
    text: &str,
    status: &str,
    extra: &[(&str, &str)],
    aliases: &[String],
) -> String {
    let mut pairs: Vec<(&str, &str)> = vec![("status", status)];
    pairs.extend_from_slice(extra);
    let mut out = match frontmatter::split(text) {
        None => {
            let lines: String = pairs.iter().map(|(k, v)| format!("{k}: {v}\n")).collect();
            format!("---\n{lines}---\n{text}")
        }
        Some((fm, _)) => {
            let has = |key: &str| {
                fm.lines()
                    .any(|l| l.starts_with(&format!("{key}:")) && !l.starts_with(' '))
            };
            let add: String = pairs
                .iter()
                .filter(|(k, _)| !has(k))
                .map(|(k, v)| format!("{k}: {v}\n"))
                .collect();
            let open = text.find('\n').map_or(text.len(), |i| i + 1);
            format!("{}{add}{}", &text[..open], &text[open..])
        }
    };
    if !aliases.is_empty() {
        out = add_aliases(&out, aliases);
    }
    crlf_front(text, out)
}

/// The text with its aliases, with the line ending of the text.
pub fn add_aliases_eol(text: &str, aliases: &[String]) -> String {
    crlf_front(text, add_aliases(text, aliases))
}

/// A text of CRLF has a front matter of CRLF: the lines that the plan added
/// have the line ending of the source.
fn crlf_front(source: &str, out: String) -> String {
    if !source.contains("\r\n") {
        return out;
    }
    match frontmatter::split(&out) {
        Some((_, body_at)) => {
            let (front, body) = out.split_at(body_at);
            format!(
                "{}{body}",
                front.replace("\r\n", "\n").replace('\n', "\r\n")
            )
        }
        None => out,
    }
}

/// The text of a source without the line `moved-to` that an earlier run
/// wrote at the start of its front matter, so a second run reads the text of
/// the first run.
pub fn strip_stub(text: &str) -> String {
    for eol in ["\n", "\r\n"] {
        let open = format!("---{eol}moved-to: ");
        if let Some(rest) = text.strip_prefix(&open) {
            let Some(end) = rest.find(eol) else { break };
            let rest = &rest[end + eol.len()..];
            return match rest.strip_prefix(&format!("---{eol}")) {
                Some(body) => body.to_string(),
                None => format!("---{eol}{rest}"),
            };
        }
    }
    text.to_string()
}

/// The text with the line `moved-to` at the start of its front matter.
pub fn with_moved_to(text: &str, id: &str) -> String {
    if strip_stub(text) != text {
        return text.to_string();
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let open = format!("---{eol}");
    match text.strip_prefix(&open) {
        Some(rest) if frontmatter::split(text).is_some() => {
            format!("{open}moved-to: {id}{eol}{rest}")
        }
        _ => format!("{open}moved-to: {id}{eol}---{eol}{text}"),
    }
}

/// The stop for a file that is not valid UTF-8.
pub fn not_utf8(path: &str) -> Fatal {
    Fatal::Refused(format!(
        "`{path}` is not valid UTF-8, and import would change a character of it; convert the file to UTF-8, commit, then run `spec-dna import` again. import changes nothing"
    ))
}

// --- what a reader gets ---

/// What a reader (`import_speckit`, `import_openspec`) gets.
pub struct Ctx<'a> {
    pub tree: &'a Tree<'a>,
    /// The final configuration: it gives the roots, the layers and the ids.
    pub cfg: &'a Config,
    pub places: &'a Places,
    /// `--areas`: the name of a source folder and its area.
    pub areas: &'a [(String, String)],
    pub skip_open: bool,
    pub keep_research: bool,
    /// The text with its links for a file that moves from `from` to `to`
    /// (`rewrite_links` with the moves of the run). The text, `from`, `to`.
    pub link: &'a dyn Fn(&str, &str, &str) -> String,
    /// The corpus of the tree before the import.
    pub before: &'a Corpus,
}

impl Ctx<'_> {
    /// The area of a source folder, if `--areas` names it.
    pub fn area_of(&self, name: &str) -> Option<&str> {
        self.areas
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| a.as_str())
    }

    /// The text of a file of the tree. A file that is not valid UTF-8 stops
    /// the plan (exit code 1): a copy with the replacement character would
    /// change a character with no sign in the proof.
    pub fn text(&self, path: &str) -> Result<String, Fatal> {
        let got = (self.tree.read)(&[path.to_string()])?;
        let bytes = got
            .get(path)
            .ok_or_else(|| Fatal::Io(format!("{path}: not in the tree")))?;
        String::from_utf8(bytes.clone())
            .map(|t| strip_stub(&t))
            .map_err(|_| not_utf8(path))
    }

    /// The files of the tree below a folder (`prefix` ends with `/`), in path
    /// order.
    pub fn under(&self, prefix: &str) -> Vec<&str> {
        self.tree
            .files
            .iter()
            .map(String::as_str)
            .filter(|f| f.starts_with(prefix))
            .collect()
    }

    /// The old id of a source that is a document of the project already: it
    /// stays as an alias of the target (decision I4).
    pub fn old_alias(&self, path: &str, new_id: &str) -> Option<String> {
        if !is_document(self.cfg, path) {
            return None;
        }
        id_of(self.cfg, path).filter(|id| id != new_id)
    }

    /// Reads an extra file of a feature or of a change: it moves to the
    /// research layer with `--keep research`, else it stays (decisions I15).
    /// `folder` is the name of the source folder, and `rel` the path of the
    /// file in it.
    pub fn extra(
        &self,
        read: &mut Read,
        path: &str,
        folder: &str,
        rel: &str,
        keep_reason: &str,
    ) -> Result<(), Fatal> {
        if !is_doc_path(path) {
            read.sources
                .push(SourceFile::kept(path, "not a Markdown file"));
        } else if self.keep_research {
            let to = format!("{}/imported/{folder}/{rel}", self.places.research_dir()?);
            let mut text = (self.link)(&self.text(path)?, path, &to);
            let id = id_of(self.cfg, &to).unwrap_or_default();
            if let Some(alias) = self.old_alias(path, &id) {
                text = add_aliases_eol(&text, &[alias]);
            }
            read.sources.push(SourceFile {
                path: path.to_string(),
                state: State::Moved,
                to: vec![to.clone()],
                reason: "a research document".to_string(),
            });
            read.targets.push(Target {
                path: to,
                text,
                moved_from: Some(path.to_string()),
                sources: vec![path.to_string()],
                link_of: None,
                scenarios: None,
            });
        } else {
            read.sources.push(SourceFile::kept(path, keep_reason));
        }
        Ok(())
    }
}

/// A title from a name: `album-export` is `Album export`.
pub fn humanize(name: &str) -> String {
    let t = name.replace(['-', '_'], " ");
    let mut c = t.trim().chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// --- the plan of the documents ---

/// What `plan_docs` needs besides the tree and the final configuration.
pub struct DocsInput<'a> {
    pub tree: &'a Tree<'a>,
    pub fin: &'a Config,
    pub read: &'a Read,
}

/// A stop of the plan: exit code 1.
fn stop(msg: String) -> Fatal {
    Fatal::Refused(msg)
}

/// The map of the links: each moved file to its new path, and each written
/// source to its primary target.
pub fn link_map(read: &Read) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for t in &read.targets {
        if let Some(from) = t.moved_from.as_ref().or(t.link_of.as_ref()) {
            map.insert(from.clone(), t.path.clone());
        }
    }
    for (from, to) in &read.links {
        map.insert(from.clone(), to.clone());
    }
    map
}

/// The corpus of the tree before the import: the documents of the roots with
/// their texts.
pub fn corpus_of(tree: &Tree, cfg: &Config) -> Result<(Corpus, HashMap<String, String>), Fatal> {
    let docs: Vec<String> = tree
        .files
        .iter()
        .filter(|f| is_document(cfg, f))
        .cloned()
        .collect();
    let bytes = (tree.read)(&docs)?;
    let mut texts = HashMap::new();
    let mut parsed = Vec::new();
    for p in &docs {
        let t = String::from_utf8_lossy(bytes.get(p).map_or(&[][..], |b| b)).into_owned();
        parsed.push((p.clone(), Arc::new(parse(&t))));
        texts.insert(p.clone(), t);
    }
    Ok((Corpus::build(cfg, tree.files.to_vec(), parsed), texts))
}

/// Builds the plan of the documents (D21, D22 step 2, I8 and I17).
pub fn plan_docs(input: &DocsInput, config: ConfigChange) -> Result<Plan, Fatal> {
    let DocsInput { tree, fin, read } = input;
    let existing: HashSet<&str> = tree.files.iter().map(String::as_str).collect();
    // 2 sources with one target, also in another case, stop the plan (I8).
    let mut by_path: HashMap<String, &Target> = HashMap::new();
    for t in &read.targets {
        let key = t.path.to_lowercase();
        if let Some(other) = by_path.insert(key, t) {
            let a = other.sources.join(", ");
            let b = t.sources.join(", ");
            return Err(stop(format!(
                "`{a}` and `{b}` both give the target `{}`{}; use --areas to give them different areas, or rename a folder. import changes nothing",
                t.path,
                if other.path == t.path {
                    String::new()
                } else {
                    format!(" (`{}`, a name that differs only in case)", other.path)
                }
            )));
        }
    }
    // A file that differs from a target only in case is the same file on a
    // file system that ignores case, and `git mv` would stop half way.
    let lower: HashMap<String, &str> = tree
        .files
        .iter()
        .map(|f| (f.to_lowercase(), f.as_str()))
        .collect();
    for t in &read.targets {
        if let Some(other) = lower.get(&t.path.to_lowercase())
            && *other != t.path
            && Some(*other) != t.moved_from.as_deref()
        {
            return Err(stop(format!(
                "`{other}` exists, and the target `{}` differs from it only in case; a file system that ignores case has one file for both, so the write would stop half way. Rename or remove `{other}`, commit, then run import again. import changes nothing",
                t.path
            )));
        }
    }
    // A folder of a target that differs from a tracked folder only in case is
    // the same folder on a file system that ignores case.
    let mut dirs: HashMap<String, String> = HashMap::new();
    for f in tree.files.iter() {
        let mut end = 0;
        while let Some(i) = f[end..].find('/') {
            end += i;
            dirs.entry(f[..end].to_lowercase())
                .or_insert_with(|| f[..end].to_string());
            end += 1;
        }
    }
    for t in &read.targets {
        let mut end = 0;
        while let Some(i) = t.path[end..].find('/') {
            end += i;
            let dir = &t.path[..end];
            if let Some(other) = dirs.get(&dir.to_lowercase())
                && other != dir
            {
                return Err(stop(format!(
                    "the folder `{other}` exists, and the target `{}` has the folder `{dir}`, which differs from it only in case; a file system that ignores case has one folder for both, so the write would mix them. Rename `{other}`, commit, then run import again. import changes nothing",
                    t.path
                )));
            }
            end += 1;
        }
    }
    // A source that is a document of the project keeps its id as an alias of
    // the first target that it gives, and it becomes a stub (`moved-to`).
    let mut targets: Vec<Target> = read.targets.clone();
    let mut stubs: BTreeMap<String, String> = BTreeMap::new();
    for t in targets.iter().filter(|t| t.moved_from.is_none()) {
        for src in &t.sources {
            if existing.contains(src.as_str()) && is_document(fin, src) {
                stubs.entry(src.clone()).or_insert_with(|| t.path.clone());
            }
        }
    }
    for (src, to) in &stubs {
        let Some(id) = id_of(fin, src) else { continue };
        if let Some(t) = targets.iter_mut().find(|t| t.path == *to) {
            t.text = add_aliases_eol(&t.text, &[id]);
        }
    }
    let removes_hint = removes_of(&read.sources, &read.removes, tree.files);
    let (corpus, texts) = corpus_of(tree, fin)?;
    let map = link_map(read);
    let mut moves: Vec<(String, String)> = Vec::new();
    let mut writes: BTreeMap<String, String> = BTreeMap::new();
    let mut sources = read.sources.clone();
    for t in &targets {
        let planned = t.text.clone();
        let exists = existing.contains(t.path.as_str());
        match &t.moved_from {
            Some(from) => {
                if exists {
                    return Err(stop(format!(
                        "`{from}` would move to `{}`, and a file is there already; move or remove one of them, commit the change, then run import again",
                        t.path
                    )));
                }
                moves.push((from.clone(), t.path.clone()));
                writes.insert(t.path.clone(), planned);
            }
            None if exists => {
                let have = (tree.read)(std::slice::from_ref(&t.path))?;
                // The compare has no link destination: a link to a source that
                // moved is another path when the plan runs on the planned tree.
                let same = have.get(&t.path).is_some_and(|b| {
                    strip_link_destinations(&String::from_utf8_lossy(b))
                        == strip_link_destinations(&planned)
                });
                if !same {
                    return Err(stop(format!(
                        "`{}` exists with other text than the plan writes from `{}`. If the import is complete, remove the sources with `git rm -r -- {}` (the report of the first run has the list) and commit; else move or remove `{}`, commit, then run import again",
                        t.path,
                        t.sources.join("`, `"),
                        removes_hint.join(" "),
                        t.path
                    )));
                }
                // The planned text is there already: no write.
            }
            None => {
                writes.insert(t.path.clone(), planned);
            }
        }
    }
    moves.sort();
    // A written source whose targets exist with the planned text is "kept".
    for s in &mut sources {
        if s.state == State::Written
            && !s.to.is_empty()
            && s.to.iter().all(|p| !writes.contains_key(p))
        {
            s.state = State::Kept;
            s.reason = format!("already imported to `{}`", s.to.join("`, `"));
        }
    }
    // The links of the documents to a moved file or to a written source.
    let mut edited = Vec::new();
    let moved_from: HashSet<&str> = moves.iter().map(|(f, _)| f.as_str()).collect();
    for (path, text) in &texts {
        if moved_from.contains(path.as_str()) {
            continue;
        }
        let dir = mv::parent(path);
        let new = rewrite_links(text, dir, dir, &map, &corpus);
        if &new != text {
            // The plan writes this text, so the file must be valid UTF-8.
            let bytes = (tree.read)(std::slice::from_ref(path))?;
            if bytes
                .get(path)
                .is_some_and(|b| std::str::from_utf8(b).is_err())
            {
                return Err(not_utf8(path));
            }
            writes.insert(path.clone(), new);
            edited.push(path.clone());
        }
    }
    edited.sort();
    // The stubs: the source of a target, if it is a document.
    for (src, to) in &stubs {
        let Some(id) = id_of(fin, to) else { continue };
        let current = writes
            .get(src)
            .cloned()
            .or_else(|| texts.get(src).cloned())
            .unwrap_or_default();
        let new = with_moved_to(&current, &id);
        if new != current {
            writes.insert(src.clone(), new);
        }
    }
    // A kept file whose links changed is written, not kept.
    for s in &mut sources {
        if s.state == State::Kept && edited.contains(&s.path) {
            s.state = State::Written;
            s.to = vec![s.path.clone()];
            s.reason = "its links to a moved file changed".to_string();
        }
    }
    let (removes, recursive) = {
        let list = removes_of(&sources, &read.removes, tree.files);
        let rec = list.iter().any(|p| read.removes.contains(p));
        (list, rec)
    };
    // The links of a file that is no document.
    let mut outside = Vec::new();
    let outside_files: Vec<String> = tree
        .files
        .iter()
        .filter(|f| is_doc_path(f) && !texts.contains_key(*f) && !moved_from.contains(f.as_str()))
        .cloned()
        .collect();
    if !outside_files.is_empty() && !map.is_empty() {
        let bytes = (tree.read)(&outside_files)?;
        for f in &outside_files {
            let Some(b) = bytes.get(f) else { continue };
            let t = String::from_utf8_lossy(b);
            let dir = mv::parent(f);
            for r in &parse(&t).refs {
                if r.kind == crate::parse::RefKind::Wiki {
                    continue;
                }
                if let Some(p) = link_target(&corpus, f, &r.target)
                    && map.contains_key(&p)
                {
                    outside.push((f.clone(), r.target.clone()));
                }
                let _ = dir;
            }
        }
    }
    Ok(Plan {
        config,
        moves,
        writes,
        targets,
        sources,
        pairs: read.pairs.clone(),
        edited,
        outside_links: outside,
        notes: read.notes.clone(),
        removes,
        removes_recursive: recursive,
        stubs,
        universe: read.universe.clone(),
    })
}

/// The paths for `git rm`: each file that the plan wrote from a source, or a
/// folder of `hints` if each file in it is written or moved. A folder with a
/// kept file is never a path: the command would remove the only copy of the
/// kept file.
pub fn removes_of(sources: &[SourceFile], hints: &[String], files: &[String]) -> Vec<String> {
    let written: BTreeSet<&str> = sources
        .iter()
        .filter(|s| s.state == State::Written && s.to != [s.path.clone()])
        .map(|s| s.path.as_str())
        .collect();
    let gone: BTreeSet<&str> = sources
        .iter()
        .filter(|s| s.state == State::Moved)
        .map(|s| s.path.as_str())
        .collect();
    let mut out: BTreeSet<String> = BTreeSet::new();
    for f in &written {
        let folder = hints
            .iter()
            .find(|h| f.starts_with(&format!("{h}/")))
            .filter(|h| {
                files
                    .iter()
                    .filter(|x| x.starts_with(&format!("{h}/")))
                    .all(|x| written.contains(x.as_str()) || gone.contains(x.as_str()))
            });
        out.insert(folder.map_or_else(|| f.to_string(), |h| h.to_string()));
    }
    out.into_iter().collect()
}

/// The path that a relative link of `from` names, if it is a file or a folder
/// of the tree.
pub fn link_target(c: &Corpus, from: &str, target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with(['#', '<'])
        || target.starts_with("//")
        || target.contains("://")
        || target.starts_with("mailto:")
    {
        return None;
    }
    let path = &target[..target.find(['#', '?']).unwrap_or(target.len())];
    let resolved = match path.strip_prefix('/') {
        Some(abs) => crate::model::join("", abs),
        None => crate::model::join(mv::parent(from), path),
    }?;
    (c.files.contains(&resolved) || c.dirs.contains(&resolved)).then_some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade_docs::TreeData;

    const CFG: &str = "spec-dna = \"0.6.1\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

    fn cfg() -> Config {
        Config::parse(CFG).unwrap()
    }

    fn tree(files: &[(&str, &str)]) -> TreeData {
        TreeData::from_map(
            files
                .iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        )
    }

    fn target(path: &str, from: Option<&str>, source: &str, text: &str) -> Target {
        Target {
            path: path.into(),
            text: text.into(),
            moved_from: from.map(str::to_string),
            sources: vec![source.into()],
            link_of: None,
            scenarios: None,
        }
    }

    fn plan_of(files: &[(&str, &str)], read: &Read) -> Result<Plan, Fatal> {
        let t = tree(files);
        t.with(|t| {
            plan_docs(
                &DocsInput {
                    tree: t,
                    fin: &cfg(),
                    read,
                },
                ConfigChange::default(),
            )
        })
    }

    #[test]
    fn two_sources_with_one_target_stop_the_plan_and_name_both() {
        let read = Read {
            targets: vec![
                target("docs/requirements/a.md", Some("x/a.md"), "x/a.md", "A\n"),
                target("docs/requirements/a.md", Some("y/a.md"), "y/a.md", "B\n"),
            ],
            ..Read::default()
        };
        let e = plan_of(&[("x/a.md", "A\n"), ("y/a.md", "B\n")], &read).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        let m = e.message();
        assert!(m.contains("x/a.md") && m.contains("y/a.md"), "{m}");
        assert!(
            m.contains("docs/requirements/a.md") && m.contains("--areas"),
            "{m}"
        );
        // A target that differs only in case is the same target.
        let read = Read {
            targets: vec![
                target("docs/requirements/a.md", Some("x/a.md"), "x/a.md", "A\n"),
                target("docs/requirements/A.md", Some("y/a.md"), "y/a.md", "B\n"),
            ],
            ..Read::default()
        };
        let e = plan_of(&[("x/a.md", "A\n"), ("y/a.md", "B\n")], &read).unwrap_err();
        assert!(e.message().contains("only in case"), "{}", e.message());
    }

    #[test]
    fn a_target_that_exists_stops_unless_it_has_the_planned_text() {
        let mut read = Read {
            targets: vec![target(
                "docs/intent/c/intent.md",
                None,
                "src/c.md",
                "planned\n",
            )],
            removes: vec!["src".into()],
            ..Read::default()
        };
        read.sources.push(SourceFile {
            path: "src/c.md".into(),
            state: State::Written,
            to: vec!["docs/intent/c/intent.md".into()],
            reason: String::new(),
        });
        let other = [("src/c.md", "x\n"), ("docs/intent/c/intent.md", "edited\n")];
        let e = plan_of(&other, &read).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        assert!(
            e.message().contains("docs/intent/c/intent.md"),
            "{}",
            e.message()
        );
        assert!(e.message().contains("git rm -r -- src"), "{}", e.message());
        // The planned text is there already: no write, and the source is kept.
        let same = [
            ("src/c.md", "x\n"),
            ("docs/intent/c/intent.md", "planned\n"),
        ];
        let p = plan_of(&same, &read).unwrap();
        assert!(p.writes.is_empty() && p.moves.is_empty());
        assert_eq!(p.sources[0].state, State::Kept);
        assert!(p.sources[0].reason.contains("already imported"));
        assert!(p.is_empty());
        // A new target is a write.
        let p = plan_of(&[("src/c.md", "x\n")], &read).unwrap();
        assert_eq!(p.writes.len(), 1);
        assert_eq!(p.sources[0].state, State::Written);
    }

    #[test]
    fn a_move_to_a_path_that_exists_stops_the_plan() {
        let read = Read {
            targets: vec![target("docs/rules/c.md", Some("x/c.md"), "x/c.md", "A\n")],
            ..Read::default()
        };
        let e = plan_of(&[("x/c.md", "A\n"), ("docs/rules/c.md", "B\n")], &read).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        assert!(
            e.message().contains("a file is there already"),
            "{}",
            e.message()
        );
        let p = plan_of(&[("x/c.md", "A\n")], &read).unwrap();
        assert_eq!(
            p.moves,
            [("x/c.md".to_string(), "docs/rules/c.md".to_string())]
        );
        assert_eq!(p.writes["docs/rules/c.md"], "A\n");
    }

    #[test]
    fn a_document_that_links_to_a_moved_file_gets_the_new_link() {
        let doc = "---\nstatus: active\n---\n# Core\n\nSee [the spec](../../src/c.md).\n";
        let read = Read {
            targets: vec![target(
                "docs/rules/c.md",
                Some("src/c.md"),
                "src/c.md",
                "A\n",
            )],
            ..Read::default()
        };
        let files = [
            ("src/c.md", "A\n"),
            ("docs/requirements/core.md", doc),
            ("README.md", "[spec](src/c.md) and [other](docs/x.md)\n"),
        ];
        let p = plan_of(&files, &read).unwrap();
        assert_eq!(p.edited, ["docs/requirements/core.md"]);
        assert!(
            p.writes["docs/requirements/core.md"].contains("[the spec](../rules/c.md)"),
            "{:?}",
            p.writes
        );
        // A link of a file that is no document stays, and the plan counts it.
        assert_eq!(
            p.outside_links,
            [("README.md".to_string(), "src/c.md".to_string())]
        );
    }

    #[test]
    fn the_link_of_a_written_source_goes_to_its_primary_target() {
        let doc = "---\nstatus: active\n---\n# Core\n\nSee [the proposal](../../src/p.md).\n";
        let mut t = target("docs/intent/c/intent.md", None, "src/p.md", "I\n");
        t.link_of = Some("src/p.md".into());
        let read = Read {
            targets: vec![t],
            ..Read::default()
        };
        let p = plan_of(
            &[("src/p.md", "P\n"), ("docs/requirements/core.md", doc)],
            &read,
        )
        .unwrap();
        assert!(
            p.writes["docs/requirements/core.md"].contains("(../intent/c/intent.md)"),
            "{:?}",
            p.writes
        );
    }

    #[test]
    fn the_place_of_a_target_is_the_folder_of_the_layer_of_its_type() {
        let p = Places::of(&cfg());
        assert_eq!(p.requirements.as_deref(), Some("docs/requirements"));
        assert_eq!(p.rules.as_deref(), Some("docs/rules"));
        assert_eq!(p.intent.as_deref(), Some("docs/intent"));
        assert_eq!(p.research, None);
        let e = p.research_dir().unwrap_err();
        assert!(matches!(e, Fatal::Usage(_)));
        assert!(
            e.message()
                .contains("spec-dna layer add research --types research"),
            "{}",
            e.message()
        );
        assert_eq!(
            requirements_path("docs/requirements", Some("media"), "a"),
            "docs/requirements/media/a.md"
        );
        // A root of one type has no layer folder, and a root before it is skipped.
        let text = "spec-dna = \"0.6.1\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[[roots]]\npath = \"specs\"\ntype = \"requirements\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";
        let p = Places::of(&Config::parse(text).unwrap());
        assert_eq!(p.requirements.as_deref(), Some("specs"));
        assert_eq!(p.requirements_root, Some(0));
    }

    #[test]
    fn the_headings_of_a_text_ignore_a_code_block() {
        let text = "# T\n\n## A\n\ntext\n\n```\n## not a heading\n```\n\n### B\n\n## C\n";
        let l = Lines::new(text);
        let names: Vec<(usize, &str)> = l
            .headings
            .iter()
            .map(|h| (h.level, h.text.as_str()))
            .collect();
        assert_eq!(names, [(1, "T"), (2, "A"), (3, "B"), (2, "C")]);
        // The section of A holds B, and the text of A ends at B.
        assert_eq!(l.tree_end(1), l.headings[3].line);
        assert_eq!(l.own_end(1), l.headings[2].line);
        assert_eq!(l.block(4, 9), "text\n\n```\n## not a heading\n```");
    }

    #[test]
    fn a_body_splits_at_its_first_list() {
        let (s, rest) = split_sentence("One. Two.\n\nThree.\n\n- a\n- b\n");
        assert_eq!(s, "One. Two.\n\nThree.");
        assert_eq!(rest, "- a\n- b");
        let (s, rest) = split_sentence("Only a sentence.\n");
        assert_eq!((s.as_str(), rest.as_str()), ("Only a sentence.", ""));
        let (s, _) = split_sentence("A.\n\n```\n- not a list\n```\n\nB.\n");
        assert_eq!(s, "A.\n\n```\n- not a list\n```\n\nB.");
        assert_eq!(indent("a\n\nb", 2), "  a\n\n  b");
    }

    #[test]
    fn a_key_is_the_slug_of_a_name_and_a_heading_loses_its_annotation() {
        assert_eq!(slug("Library-First"), "library-first");
        assert_eq!(slug("  User   Authentication! "), "user-authentication");
        assert_eq!(slug("I/O 2"), "i-o-2");
        assert_eq!(slug("!!!"), "");
        assert_eq!(
            plain_heading("Success Criteria *(mandatory)*"),
            "Success Criteria"
        );
        assert_eq!(plain_heading("Plain"), "Plain");
    }

    #[test]
    fn the_front_matter_of_a_target_keeps_the_keys_that_the_source_has() {
        let t = with_front_matter("# T\n", "active", &[], &["old-id".to_string()]);
        assert_eq!(
            t,
            "---\naliases:\n  - old-id\nstatus: active\n---\n# T\n".replace(
                "aliases:\n  - old-id\nstatus: active\n",
                "status: active\naliases:\n  - old-id\n"
            )
        );
        let t = with_front_matter(
            "---\nowner: me\n---\n# T\n",
            "draft",
            &[("class", "higher-risk")],
            &[],
        );
        assert_eq!(
            t,
            "---\nstatus: draft\nclass: higher-risk\nowner: me\n---\n# T\n"
        );
        let t = with_front_matter("---\nstatus: proposed\n---\n# T\n", "draft", &[], &[]);
        assert_eq!(t, "---\nstatus: proposed\n---\n# T\n");
    }

    #[test]
    fn a_clarification_marker_is_cut_and_its_text_is_kept() {
        let (s, q) = strip_clarification(
            "The tool MUST offer [NEEDS CLARIFICATION: which formats?] as formats.",
        );
        assert_eq!(s, "The tool MUST offer as formats.");
        assert_eq!(q.as_deref(), Some("which formats?"));
        let (s, q) = strip_clarification("It MUST work [NEEDS CLARIFICATION].");
        assert_eq!((s.as_str(), q), ("It MUST work.", None));
        let (s, q) = strip_clarification("A [NEEDS CLARIFICATION: x] and [NEEDS CLARIFICATION: y]");
        assert_eq!(s, "A and");
        assert_eq!(q.as_deref(), Some("x; y"));
        assert_eq!(strip_clarification("No marker.").0, "No marker.");
    }

    #[test]
    fn the_similarity_is_the_share_of_the_lines_that_stay() {
        assert_eq!(similarity("a\nb\n", "a\nb\n"), 1.0);
        assert_eq!(similarity("a\nb\nc\nd\n", "a\nb\nx\ny\n"), 0.5);
        assert_eq!(similarity("a\na\n", "a\nb\n"), 0.5);
        // The larger text counts, and a long line weighs more than a short one.
        assert_eq!(similarity("aaaa\nb\n", "aaaa\n"), 5.0 / 7.0);
        assert_eq!(similarity("", ""), 1.0);
    }

    #[test]
    fn the_scenarios_of_a_text_count_headings_and_items_outside_code() {
        let t = "#### Scenario: A\n\n- **WHEN** x\n\n```\n#### Scenario: no\n```\n\n  - Scenario: B\n    - Scenario: C\n";
        assert_eq!(count_scenarios(t), 3);
    }

    #[test]
    fn an_existing_file_that_differs_only_in_case_stops_the_plan() {
        let read = Read {
            targets: vec![target("docs/rules/c.md", Some("x/c.md"), "x/c.md", "A\n")],
            ..Read::default()
        };
        let e = plan_of(&[("x/c.md", "A\n"), ("docs/rules/C.md", "B\n")], &read).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        let m = e.message();
        assert!(
            m.contains("docs/rules/C.md") && m.contains("docs/rules/c.md"),
            "{m}"
        );
        assert!(m.contains("only in case"), "{m}");
    }

    #[test]
    fn a_kept_document_with_an_edited_link_is_written() {
        let doc = "---\nstatus: active\n---\n# Plan\n\nSee [the spec](../../src/c.md).\n";
        let mut read = Read {
            targets: vec![target(
                "docs/rules/c.md",
                Some("src/c.md"),
                "src/c.md",
                "A\n",
            )],
            ..Read::default()
        };
        read.sources.push(SourceFile::kept(
            "docs/requirements/plan.md",
            "an extra file",
        ));
        let p = plan_of(
            &[("src/c.md", "A\n"), ("docs/requirements/plan.md", doc)],
            &read,
        )
        .unwrap();
        assert_eq!(p.edited, ["docs/requirements/plan.md"]);
        let s = &p.sources[0];
        assert_eq!(
            (s.state, s.to.as_slice()),
            (
                State::Written,
                &["docs/requirements/plan.md".to_string()][..]
            )
        );
    }

    #[test]
    fn the_sources_to_remove_are_the_files_that_the_plan_wrote() {
        let mut read = Read {
            removes: vec!["specs/f".into()],
            ..Read::default()
        };
        for f in ["specs/f/spec.md", "specs/f/tasks.md", "specs/g/spec.md"] {
            read.sources.push(SourceFile {
                path: f.into(),
                state: State::Written,
                to: vec![format!("docs/{f}")],
                reason: String::new(),
            });
            read.targets
                .push(target(&format!("docs/{f}"), None, f, "T\n"));
        }
        read.removes.push("specs/g".into());
        read.sources
            .push(SourceFile::kept("specs/f/plan.md", "an extra file"));
        let files = [
            ("specs/f/spec.md", "a\n"),
            ("specs/f/tasks.md", "b\n"),
            ("specs/f/plan.md", "c\n"),
            ("specs/g/spec.md", "d\n"),
        ];
        let p = plan_of(&files, &read).unwrap();
        // `specs/f` holds a kept file, so the plan names its files. `specs/g`
        // holds none.
        assert_eq!(
            p.removes,
            ["specs/f/spec.md", "specs/f/tasks.md", "specs/g"]
        );
    }

    #[test]
    fn the_front_matter_has_the_line_ending_of_the_source() {
        let t = with_front_matter("# T\r\n\r\nText.\r\n", "active", &[], &["old".to_string()]);
        assert_eq!(
            t,
            "---\r\nstatus: active\r\naliases:\r\n  - old\r\n---\r\n# T\r\n\r\nText.\r\n"
        );
        let t = with_front_matter("---\r\nowner: me\r\n---\r\n# T\r\n", "draft", &[], &[]);
        assert_eq!(t, "---\r\nstatus: draft\r\nowner: me\r\n---\r\n# T\r\n");
    }

    #[test]
    fn a_source_that_is_not_utf_8_stops_the_read() {
        let t = TreeData::from_map(
            [("a.md".to_string(), b"calf\xe9 MUST\n".to_vec())]
                .into_iter()
                .collect(),
        );
        let e = t
            .with(|t| {
                let (before, _) = corpus_of(t, &cfg())?;
                let places = Places::of(&cfg());
                Ctx {
                    tree: t,
                    cfg: &cfg(),
                    places: &places,
                    areas: &[],
                    skip_open: false,
                    keep_research: false,
                    link: &|x, _, _| x.to_string(),
                    before: &before,
                }
                .text("a.md")
            })
            .unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        assert!(
            e.message().contains("a.md") && e.message().contains("not valid UTF-8"),
            "{}",
            e.message()
        );
    }

    #[test]
    fn a_folder_that_differs_only_in_case_stops_the_plan() {
        let read = Read {
            targets: vec![target("docs/intent/x/intent.md", None, "src/x.md", "A\n")],
            ..Read::default()
        };
        let e = plan_of(
            &[("src/x.md", "A\n"), ("docs/Intent/y/intent.md", "B\n")],
            &read,
        )
        .unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        let m = e.message();
        assert!(
            m.contains("docs/Intent") && m.contains("docs/intent") && m.contains("only in case"),
            "{m}"
        );
    }
}
