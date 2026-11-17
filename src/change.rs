//! The change model (design 0.6.0, "Change lifecycle"). A change is one folder
//! `<intent layer dir>/<slug>/` with up to 3 artifacts: `intent.md`, `spec.md`
//! and `plan.md`. This module reads the artifacts of a tree: their statuses,
//! the `class`, the typed lines of the delta, and the `Files` scope. It runs no
//! check and reads no configuration key. The checks and the commands of the
//! lifecycle use it.
//!
//! The parsed data comes from `ParsedDoc::items` and `Req::sentence`. A line
//! that is not in a valid form is data here, not an error: this module keeps it
//! with its reason, and a check reports it later (SD914).

use crate::glob;
use crate::hash::{Hash, hash_normalized, hex};
use crate::model::{Class, Corpus, Doc, DocType, Res, is_slug};
use crate::parse::{ItemSection, ListItem, Req, strip_link_destinations, strip_markers};
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

/// One artifact of a change: the document in the corpus and its status.
#[derive(Clone, Debug, PartialEq)]
pub struct Artifact {
    /// The index in `Corpus::docs`.
    pub doc: u32,
    /// The status, or an empty string if the document has none.
    pub status: String,
}

/// One change: the folder of its artifacts.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// The name of the folder, which is the name of the change.
    pub slug: String,
    /// The path of the folder from the repository root, with no final `/`.
    pub dir: String,
    pub intent: Option<Artifact>,
    pub spec: Option<Artifact>,
    pub plan: Option<Artifact>,
    /// The `class` of the intent. A change with no intent, and an intent with
    /// no `class` or with an invalid `class`, is `routine`.
    pub class: Class,
    /// True if the change has an intent and its `class` is absent, empty or
    /// invalid. `next` and `status` then give no class. `check` keeps
    /// `routine` (design 0.7.0, section 15).
    pub class_unset: bool,
    /// The `areas` of the intent.
    pub areas: Vec<String>,
    /// The `from` of the intent: the id of an incident.
    pub from: Option<String>,
}

/// Collects the changes of a tree, in the order of their folders. A document
/// is an artifact if its type is `intent`, `spec` or `plan` and it is at the
/// place of that type. A folder with one such document is a change, also
/// without an intent.
pub fn collect(corpus: &Corpus) -> Vec<Change> {
    let mut by_dir: BTreeMap<&str, Change> = BTreeMap::new();
    for (i, d) in corpus.docs.iter().enumerate() {
        if !d.ty.is_intent_type() || d.type_failed || d.moved_to.is_some() || d.generated.is_some()
        {
            continue;
        }
        let Some((dir, _)) = d.path.rsplit_once('/') else {
            continue;
        };
        let change = by_dir.entry(dir).or_insert_with(|| Change {
            slug: dir.rsplit('/').next().unwrap_or(dir).to_string(),
            dir: dir.to_string(),
            intent: None,
            spec: None,
            plan: None,
            class: Class::Routine,
            class_unset: false,
            areas: Vec::new(),
            from: None,
        });
        let artifact = Some(Artifact {
            doc: i as u32,
            status: d.status_str().to_string(),
        });
        match d.ty {
            DocType::Intent => {
                change.intent = artifact;
                change.class = d.class;
                change.class_unset = !d.class_set;
                change.areas = d.areas.clone();
                change.from = d.from.as_ref().map(|(f, _)| f.clone());
            }
            DocType::Spec => change.spec = artifact,
            _ => change.plan = artifact,
        }
    }
    by_dir.into_values().collect()
}

impl Change {
    /// The class for `next` and `status`: `None` if the intent has no valid
    /// class (design 0.7.0, section 15).
    pub fn class_shown(&self) -> Option<&'static str> {
        (!self.class_unset).then(|| self.class.name())
    }

    /// The status of the intent, or an empty string if the change has no
    /// intent.
    pub fn intent_status(&self) -> &str {
        self.intent.as_ref().map_or("", |a| &a.status)
    }

    /// The typed lines of the delta of the spec, with the line errors. A
    /// change with no spec has no delta line.
    pub fn delta(&self, corpus: &Corpus) -> Vec<DeltaLine> {
        match &self.spec {
            Some(a) => delta_lines(&corpus.docs[a.doc as usize]),
            None => Vec::new(),
        }
    }

    /// The patterns of the plan: the paths of the `Files` section, then the
    /// paths of the label `Files:` of the tasks of `Order`, in the order of the
    /// file, with no repeat. A `Files` item gives one pattern: its first code
    /// span, or, with no code span, its text if the text is a plain path
    /// ([`plain_path`]). An item that gives no pattern has no effect
    /// ([`Change::files_not_paths`]). Each reader of the scope of a change uses
    /// this list.
    pub fn files(&self, corpus: &Corpus) -> Vec<String> {
        let Some(a) = &self.plan else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        let items = section_items(&corpus.docs[a.doc as usize], ItemSection::Files)
            .filter_map(item_pattern)
            .chain(self.tasks(corpus).into_iter().flat_map(|t| t.files));
        for p in items {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// The items of the `Files` section that give no pattern, in the order of
    /// the file: prose, and a code span that is a template placeholder. Each
    /// is on one line and has at most 80 characters. The one item that states
    /// "no files" ([`Change::no_files`]) is not in the list.
    pub fn files_not_paths(&self, corpus: &Corpus) -> Vec<String> {
        let Some(a) = &self.plan else {
            return Vec::new();
        };
        // The one item that states "no files" is no wrong item.
        if self.has_none_item(corpus) {
            return Vec::new();
        }
        section_items(&corpus.docs[a.doc as usize], ItemSection::Files)
            .filter(|i| item_pattern(i).is_none())
            .map(|i| shown(&i.text))
            .filter(|t| !t.is_empty())
            .collect()
    }
}

impl Change {
    /// True if the plan states that the change edits no file. The `Files`
    /// section has one item only, and the item starts with the word `none` in
    /// any case, such as `- none: research only`. A plan that lists a path in
    /// its tasks does not state it, because the plan then names a file.
    pub fn no_files(&self, corpus: &Corpus) -> bool {
        self.has_none_item(corpus) && self.files(corpus).is_empty()
    }

    /// True if the `Files` section states "no files". With list items, it has
    /// one item only, and the item starts with the word `none` (the rule of
    /// 0.6.1). With no list item, its first paragraph is the form of D28
    /// ([`is_none_text`]), such as `None: this change is research.`.
    fn has_none_item(&self, corpus: &Corpus) -> bool {
        let Some(a) = &self.plan else {
            return false;
        };
        let doc = &corpus.docs[a.doc as usize];
        let mut items = section_items(doc, ItemSection::Files);
        match (items.next(), items.next()) {
            (Some(item), None) => starts_with_none(&item.text),
            (Some(_), Some(_)) => false,
            (None, _) => doc
                .parsed
                .files_paragraph
                .as_ref()
                .is_some_and(|p| is_none_text(&p.text)),
        }
    }

    /// True if the `Files` section of the plan has a paragraph and no list
    /// item, and the paragraph does not state "no files" (design 0.7.0, D28).
    /// The section then lists no path, and the person must write a list.
    pub fn files_paragraph_only(&self, corpus: &Corpus) -> bool {
        let Some(a) = &self.plan else {
            return false;
        };
        let doc = &corpus.docs[a.doc as usize];
        section_items(doc, ItemSection::Files).next().is_none()
            && doc
                .parsed
                .files_paragraph
                .as_ref()
                .is_some_and(|p| !is_none_text(&p.text))
    }
}

/// The line that `next`, `status` and the messages of SD913 and SD915 give for
/// a `Files` section with a paragraph and no list item (design 0.7.0, D28).
pub const FILES_PARAGRAPH_LINE: &str = "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`";

/// True if `text` states "nothing" in the form of D28: its first word is
/// `none`, in any case, and `:`, `.` or the end of the text follows the word.
/// The text is a paragraph under `Files`, or the one item of `Interfaces`,
/// `Concerns` or `Open questions`, without its task box. `None of the old files
/// stay` is not this form. A list item under `Files` keeps the rule of 0.6.1
/// (the first word is `none`).
pub fn is_none_text(text: &str) -> bool {
    let t = text.trim();
    let end = t
        .char_indices()
        .find(|&(_, c)| !c.is_alphanumeric())
        .map_or(t.len(), |(i, _)| i);
    t[..end].eq_ignore_ascii_case("none")
        && matches!(t[end..].chars().next(), None | Some(':' | '.'))
}

/// The open concerns of a spec: the items `- [ ]` of its `Concerns` section. A
/// resolved concern, an item with no task box and the one item that states
/// "nothing" (`- [ ] None.`) are not open.
pub fn open_concerns(spec: &Doc) -> Vec<&ListItem> {
    if states_nothing(spec, ItemSection::Concerns) {
        return Vec::new();
    }
    section_items(spec, ItemSection::Concerns)
        .filter(|i| i.task == Some(false))
        .collect()
}

/// True if a section of a document has one item only, and the item states
/// "nothing" ([`is_none_text`]). The task box of the item does not matter.
pub fn states_nothing(doc: &Doc, section: ItemSection) -> bool {
    let mut items = section_items(doc, section);
    matches!((items.next(), items.next()), (Some(item), None) if is_none_text(&item.text))
}

/// True if the first word of `text` is `none`, in any case. The word ends at
/// the first character that is not a letter or a digit.
fn starts_with_none(text: &str) -> bool {
    let word: String = text
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric())
        .collect();
    word.eq_ignore_ascii_case("none")
}

/// The pattern of a `Files` item. A code span is the first choice. An item
/// with no code span is a pattern only if its whole text is a plain path.
fn item_pattern(item: &ListItem) -> Option<String> {
    match &item.code {
        Some(c) => usable_code(c).then(|| c.clone()),
        None => plain_path(&item.text),
    }
}

/// True if a code span can be a pattern. A template placeholder is not one.
fn usable_code(code: &str) -> bool {
    !code.contains("{{")
}

/// The text on one line, with at most 80 characters, for a message.
pub(crate) fn shown(text: &str) -> String {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > 80 {
        format!("{}...", t.chars().take(77).collect::<String>().trim_end())
    } else {
        t
    }
}

/// Returns the path or the glob that `text` is, or `None` if `text` is not a
/// plain path. A plain path is the text of a `Files` item, or of one entry of
/// the label `Files:` of a task, that has no code span. Rules:
///
/// - The text, with no white space at its ends, without one final `.`, `,`,
///   `:` or `;` (a mark of the sentence), and without the parentheses that
///   wrap it, is one token: it has no white space.
/// - If the text is a link (`[text](destination)`), the token is the link
///   text.
/// - The token has a `/`, a `.` or a `*`, and it has a letter, a digit or a
///   `*`. A token with none of these (`Makefile`, `none`) is a word.
/// - A token with a backtick, `<`, `>`, `"`, `|`, `{{` or `://` is not a path
///   (a URL, a tag or a placeholder).
pub fn plain_path(text: &str) -> Option<String> {
    let mut s = text.trim();
    if let Some(rest) = s.strip_suffix(['.', ',', ':', ';']) {
        s = rest.trim_end();
    }
    s = unwrap_parens(s);
    if let Some(inner) = s
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(')'))
        .and_then(|r| r.find("](").map(|i| &r[..i]))
    {
        s = inner.trim();
    }
    let token = !s.is_empty()
        && !s.contains(char::is_whitespace)
        && !s.contains(['`', '<', '>', '"', '|'])
        && !s.contains("{{")
        && !s.contains("://");
    let looks_like_a_path =
        s.contains(['/', '.', '*']) && s.chars().any(|c| c.is_alphanumeric() || c == '*');
    (token && looks_like_a_path).then(|| s.to_string())
}

/// The top-level list items of a section of a document.
pub fn section_items(doc: &Doc, section: ItemSection) -> impl Iterator<Item = &ListItem> {
    doc.parsed
        .items
        .iter()
        .filter(move |i| i.section == section)
}

/// True if the path is in the scope of one pattern of `Files`. A pattern is a
/// glob from the repository root (a leading `./` or `/` does not count). A
/// pattern with a final `/` matches the folder and each file below it. Any
/// other pattern matches the whole path.
pub fn in_scope(patterns: &[String], path: &str) -> bool {
    patterns.iter().any(|p| {
        let p = p.trim();
        let p = p.strip_prefix("./").unwrap_or(p).trim_start_matches('/');
        if p.is_empty() {
            return false;
        }
        match p.strip_suffix('/') {
            Some(folder) => !folder.is_empty() && glob::matches(&format!("{folder}/**"), path),
            None => glob::matches(p, path),
        }
    })
}

/// The verb of a delta line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Add,
    Change,
    Remove,
}

/// A delta line in a valid form:
///
/// - `ADD <id>#<key>: <sentence>`
/// - `CHANGE <id>#<key>@<16 hex digits>: <sentence>`
/// - `REMOVE <id>#<key>@<16 hex digits>`
#[derive(Clone, Debug, PartialEq)]
pub struct Delta {
    pub verb: Verb,
    /// The id of the target document, as the line writes it.
    pub id: String,
    /// The key of the target requirement. It is a slug.
    pub key: String,
    /// The hash of the target text that the author read: 16 lowercase
    /// hexadecimal digits. `ADD` has none.
    pub hash: Option<String>,
    /// The new text, as the line writes it. `REMOVE` has none.
    pub sentence: Option<String>,
}

impl Delta {
    /// The target as `<id>#<key>`.
    pub fn target(&self) -> String {
        format!("{}#{}", self.id, self.key)
    }

    /// The hash `frozen` of the sentence, as a requirement has it: the hash of
    /// the text without the markers and without link destinations. A
    /// comparison with the text of a requirement uses it.
    pub fn frozen(&self) -> Option<Hash> {
        let (clean, _, _) = strip_markers(self.sentence.as_deref()?);
        Some(hash_normalized(&strip_link_destinations(&clean)))
    }
}

/// One list item of the `Delta` section, with the result of its parse.
#[derive(Clone, Debug, PartialEq)]
pub struct DeltaLine {
    pub line: u32,
    /// The byte range of the item in the source of the spec.
    pub range: Range<u32>,
    /// The line, or the reason why it is invalid (a short sentence).
    pub parsed: Result<Delta, String>,
}

/// Parses the items of the `Delta` section of a spec. A second line for a
/// target that an earlier line has is invalid.
pub fn delta_lines(spec: &Doc) -> Vec<DeltaLine> {
    let mut seen: HashSet<String> = HashSet::new();
    section_items(spec, ItemSection::Delta)
        .map(|item| {
            let mut parsed = parse_delta(item);
            if let Ok(d) = &parsed
                && !seen.insert(d.target())
            {
                parsed = Err(format!(
                    "a line for `{}` is there already: one line for each key",
                    d.target()
                ));
            }
            DeltaLine {
                line: item.line,
                range: item.range.clone(),
                parsed,
            }
        })
        .collect()
}

/// Parses one item of the `Delta` section. An item is valid if it is one
/// paragraph with no nested list and has one of the 3 forms. The parser does
/// not stop at an invalid item: the result has the reason.
pub fn parse_delta(item: &ListItem) -> Result<Delta, String> {
    if item.nested {
        return Err("a delta line has no nested list".to_string());
    }
    if item.text.lines().any(|l| l.trim().is_empty()) {
        return Err("a delta line is one paragraph".to_string());
    }
    let words: Vec<&str> = item.text.split_whitespace().collect();
    let text = words.join(" ");
    let (verb, rest) = match words.first().copied() {
        Some("ADD") => (Verb::Add, text[3..].trim_start()),
        Some("CHANGE") => (Verb::Change, text[6..].trim_start()),
        Some("REMOVE") => (Verb::Remove, text[6..].trim_start()),
        _ => return Err("a delta line starts with ADD, CHANGE or REMOVE".to_string()),
    };
    let (target, sentence) = match verb {
        Verb::Remove => {
            if rest.contains([' ', ':']) {
                return Err("a REMOVE line has a target and a hash, and no text".to_string());
            }
            (rest, None)
        }
        _ => {
            let Some((target, sentence)) = rest.split_once(':') else {
                return Err("the target must have `:` and the new text after it".to_string());
            };
            let sentence = sentence.trim();
            if sentence.is_empty() {
                return Err("the line has no new text after `:`".to_string());
            }
            (target, Some(sentence.to_string()))
        }
    };
    if target.is_empty() || target.contains(char::is_whitespace) {
        return Err("the target must be `<id>#<key>`, with no space".to_string());
    }
    let (id_key, hash) = match target.split_once('@') {
        Some((a, h)) => (a, Some(h)),
        None => (target, None),
    };
    let Some((id, key)) = id_key.split_once('#') else {
        return Err("the target must be `<id>#<key>`".to_string());
    };
    if id.is_empty() || id.contains(['@', '#']) {
        return Err("the target must be `<id>#<key>`, with one document id".to_string());
    }
    if !is_slug(key) {
        return Err(format!(
            "`{key}` is not a key: use lowercase letters and digits joined by hyphens"
        ));
    }
    let hash = match (verb, hash) {
        (Verb::Add, None) => None,
        (Verb::Add, Some(_)) => return Err("an ADD line has no hash".to_string()),
        (_, None) => return Err("the target must have `@` and the 16 digit hash".to_string()),
        (_, Some(h)) if h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Some(h.to_ascii_lowercase())
        }
        (_, Some(_)) => return Err("the hash must be 16 hexadecimal digits".to_string()),
    };
    Ok(Delta {
        verb,
        id: id.to_string(),
        key: key.to_string(),
        hash,
        sentence,
    })
}

/// True if a hash of a delta line names the text of the requirement: it is
/// the hash that `show` prints, or the hash `frozen` (the same text with no
/// link destinations, so a `mv` that rewrote a link keeps the line).
pub fn hash_matches(req: &Req, hash: &str) -> bool {
    hash.eq_ignore_ascii_case(&hex(&req.hash)) || hash.eq_ignore_ascii_case(&hex(&req.frozen))
}

/// The valid requirement of a document id and a key in a tree.
pub fn target_req<'a>(corpus: &'a Corpus, id: &str, key: &str) -> Option<&'a Req> {
    let Res::Found(d, None, _) = corpus.resolve(id) else {
        return None;
    };
    let doc = &corpus.docs[d as usize];
    doc.req(key).map(|i| &doc.parsed.reqs[i])
}

/// The result of `applied`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    Yes,
    No,
    /// The line has no valid form, or it is a `REMOVE` of a key that neither
    /// tree has.
    Invalid,
}

/// Tells if the head tree has the result of a delta line (design, T3). `base`
/// is the tree before the change, and `head` is the tree now.
///
/// - `ADD` is applied if the base has no such requirement, and the head has
///   the text of the line.
/// - `CHANGE` is applied if the head has the text of the line.
/// - `REMOVE` is applied if the base has the requirement, and the head has
///   not. A `REMOVE` of a key that neither tree has is invalid.
///
/// A requirement has the text of a line if its hash `frozen` is the hash
/// `frozen` of the sentence of the line.
pub fn applied(line: &DeltaLine, base: &Corpus, head: &Corpus) -> Applied {
    let Ok(d) = &line.parsed else {
        return Applied::Invalid;
    };
    let in_base = target_req(base, &d.id, &d.key);
    let in_head = target_req(head, &d.id, &d.key);
    let has_text = |r: Option<&Req>| matches!((r, d.frozen()), (Some(r), Some(f)) if r.frozen == f);
    let yes = match d.verb {
        Verb::Add => in_base.is_none() && has_text(in_head),
        Verb::Change => has_text(in_head),
        Verb::Remove => {
            if in_base.is_none() && in_head.is_none() {
                return Applied::Invalid;
            }
            in_base.is_some() && in_head.is_none()
        }
    };
    if yes { Applied::Yes } else { Applied::No }
}

/// One task of the `Order` section of a plan: a list item with a task box and
/// an id. The item form is
/// `- [ ] <id>: <text>. Files: `path`, `glob`. Verify: <text or `command`>.`
/// The id is `[A-Za-z0-9][A-Za-z0-9_-]*` and a colon. A list item of `Order`
/// that has no task box, or has no id of this form, is not a task.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanTask {
    pub id: String,
    /// True for `- [x]` (or `- [X]`), false for `- [ ]`.
    pub done: bool,
    /// The text after the id and the colon, before the first label, on one
    /// line. One final `.` is removed.
    pub text: String,
    /// The paths after the label `Files:`, in the order of the file: the code
    /// spans, and the plain paths of the entries that have no code span
    /// ([`plain_path`]).
    pub files: Vec<String>,
    /// The text after the label `Files:`, on one line, with one final `.`
    /// removed. `None` if the label is absent. A label with no text gives an
    /// empty string. SD923 reads it (design 0.7.0, D40).
    pub files_text: Option<String>,
    /// The text after the label `Verify:`, on one line. One final `.` is
    /// removed. `None` if the label is absent or has no text.
    pub verify: Option<String>,
    /// The text after the label `Hours:`, on one line, with one final `.`
    /// removed. `None` if the label is absent or has no text. [`parse_hours`]
    /// reads the value (design 0.7.0, D28, D41).
    pub hours: Option<String>,
    /// The text after the label `Skip:`, on one line, with one final `.`
    /// removed. `None` if the label is absent. A label with no text gives an
    /// empty string: the task is not skipped, and SD923 reports it.
    pub skip: Option<String>,
    pub line: u32,
    /// The byte range of the item in the source of the plan.
    pub range: Range<u32>,
    /// True if an earlier task of the same plan has this id. The first task
    /// with an id counts. A task with `duplicate` is not the next task.
    pub duplicate: bool,
}

impl PlanTask {
    /// True if the task has the label `Skip:` with a reason. It is not needed,
    /// and its box does not matter (design 0.7.0, D40).
    pub fn skipped(&self) -> bool {
        self.skip.as_deref().is_some_and(|r| !r.is_empty())
    }

    /// True for `- [x]` and no reason in `Skip:`.
    pub fn is_done(&self) -> bool {
        self.done && !self.skipped()
    }

    /// True if the task is done or skipped.
    pub fn closed(&self) -> bool {
        self.done || self.skipped()
    }
}

/// The range of hours of the value of the label `Hours:`, as 2 numbers of
/// hundredths of an hour (low and high), or `None` if the value is invalid.
/// The value is `<n>` or `<a>-<b>` with `a <= b`. A number `<n>` has 1 to 4
/// digits, then 1 or 2 decimals after a `.` (optional), then an `h`
/// (optional). Spaces or tabs can be around the `-`.
pub fn parse_hours(value: &str) -> Option<(u64, u64)> {
    fn number(t: &str) -> Option<u64> {
        let t = t.strip_suffix('h').unwrap_or(t);
        let (whole, frac) = match t.split_once('.') {
            Some((w, f)) => (w, Some(f)),
            None => (t, None),
        };
        let digits = |x: &str, max: usize| {
            (1..=max).contains(&x.len()) && x.bytes().all(|b| b.is_ascii_digit())
        };
        if !digits(whole, 4) || frac.is_some_and(|f| !digits(f, 2)) {
            return None;
        }
        let mut v = whole.parse::<u64>().ok()? * 100;
        if let Some(f) = frac {
            let n: u64 = f.parse().ok()?;
            v += if f.len() == 1 { n * 10 } else { n };
        }
        Some(v)
    }
    match value.split_once('-') {
        None => number(value).map(|n| (n, n)),
        Some((a, b)) => {
            let (a, b) = (
                number(a.trim_matches([' ', '\t']))?,
                number(b.trim_matches([' ', '\t']))?,
            );
            (a <= b).then_some((a, b))
        }
    }
}

/// The labels of a task line.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Label {
    Files,
    Verify,
    Hours,
    Skip,
}

/// Parses one item of the `Order` section as a task. Returns `None` if the
/// item is not a task (`duplicate` is false: `plan_tasks` sets it). The
/// labels `Files:`, `Verify:`, `Hours:` and `Skip:` are case-sensitive, they
/// follow white space or the start of the text, and a code span does not hold
/// a label. Each is
/// optional and ends at the next label.
pub fn parse_task(item: &ListItem) -> Option<PlanTask> {
    let done = item.task?;
    let text = item.text.as_str();
    // The labels outside code spans: (start, end, the label).
    let mut labels: Vec<(usize, usize, Label)> = Vec::new();
    for (start, end) in plain_pieces(text) {
        for (label, kind) in [
            ("Files:", Label::Files),
            ("Verify:", Label::Verify),
            ("Hours:", Label::Hours),
            ("Skip:", Label::Skip),
        ] {
            let mut from = start;
            while let Some(i) = text[from..end].find(label) {
                let pos = from + i;
                if pos == 0 || text[..pos].ends_with(char::is_whitespace) {
                    labels.push((pos, pos + label.len(), kind));
                }
                from = pos + label.len();
            }
        }
    }
    labels.sort();
    let head_end = labels.first().map_or(text.len(), |l| l.0);
    let head = text[..head_end].trim_start();
    let id_len = head
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_alphanumeric() || (i > 0 && (c == '-' || c == '_'))))
        .map_or(head.len(), |(i, _)| i);
    let after = head[id_len..].strip_prefix(':')?;
    if id_len == 0 || !(after.is_empty() || after.starts_with(char::is_whitespace)) {
        return None;
    }
    let mut files = None;
    let mut files_text = None;
    let mut verify = None;
    let mut hours = None;
    let mut skip = None;
    for (n, &(_, end, kind)) in labels.iter().enumerate() {
        let stop = labels.get(n + 1).map_or(text.len(), |l| l.0);
        let seg = &text[end..stop];
        match kind {
            Label::Files => {
                files_text.get_or_insert_with(|| one_line(seg));
                files.get_or_insert_with(|| label_paths(seg));
            }
            Label::Verify if verify.is_none() => {
                let v = one_line(seg);
                verify = Some((!v.is_empty()).then_some(v));
            }
            Label::Hours if hours.is_none() => {
                let v = one_line(seg);
                hours = Some((!v.is_empty()).then_some(v));
            }
            Label::Skip => {
                skip.get_or_insert_with(|| one_line(seg));
            }
            _ => {}
        }
    }
    Some(PlanTask {
        id: head[..id_len].to_string(),
        done,
        text: one_line(after),
        files: files.unwrap_or_default(),
        files_text,
        verify: verify.flatten(),
        hours: hours.flatten(),
        skip,
        line: item.line,
        range: item.range.clone(),
        duplicate: false,
    })
}

/// The text on one line: each run of white space is one space, and one final
/// `.` is removed.
fn one_line(s: &str) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    t.strip_suffix('.').unwrap_or(&t).trim_end().to_string()
}

/// The byte ranges of `text` that are not in a code span. A code span is a run
/// of backticks, and the next run of the same length. A run with no match is
/// plain text.
fn plain_pieces(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut plain = 0;
    for (start, end, _) in spans(text) {
        out.push((plain, start));
        plain = end;
    }
    out.push((plain, text.len()));
    out
}

/// The text without the parentheses that wrap it. They wrap it only if the
/// first `(` closes at the last character: `(a)/(b)` is not wrapped.
fn unwrap_parens(s: &str) -> &str {
    let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return s;
    };
    let mut depth = 0i32;
    for c in inner.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return s,
            ')' => depth -= 1,
            _ => {}
        }
    }
    if depth == 0 { inner.trim() } else { s }
}

/// The paths of the text after the label `Files:`. The text is a list of
/// entries. The mark at its end (`.` or `;`) and the parentheses that wrap it
/// are removed, and then it splits at the commas that are not in a code span
/// and not in a link (`[text](destination)`). An entry gives its code spans,
/// or, with no code span, its text if the text is a plain path.
fn label_paths(seg: &str) -> Vec<String> {
    let mut seg = seg.trim();
    while let Some(rest) = seg.strip_suffix(['.', ';']) {
        seg = rest.trim_end();
    }
    seg = unwrap_parens(seg);
    let mut cuts: Vec<usize> = Vec::new();
    // The depth of `[...]`, and whether the `(...)` of a link is open.
    let mut bracket = 0usize;
    let mut link = false;
    let mut after_close = false;
    for (start, end) in plain_pieces(seg) {
        for (i, c) in seg[start..end].char_indices() {
            let open_paren = after_close && c == '(';
            after_close = false;
            match c {
                '[' if !link => bracket += 1,
                ']' if !link && bracket > 0 => {
                    bracket -= 1;
                    after_close = bracket == 0;
                }
                '(' if open_paren => link = true,
                ')' if link => link = false,
                ',' if bracket == 0 && !link => cuts.push(start + i),
                _ => {}
            }
        }
    }
    let mut entries = Vec::new();
    let mut from = 0;
    for cut in cuts {
        entries.push(&seg[from..cut]);
        from = cut + 1;
    }
    entries.push(&seg[from..]);
    let mut out = Vec::new();
    for entry in entries {
        let codes = code_spans(entry);
        if codes.is_empty() {
            out.extend(plain_path(entry));
        } else {
            out.extend(codes.into_iter().filter(|c| usable_code(c)));
        }
    }
    out
}

/// The text of each code span of `text`, with no space at the ends.
fn code_spans(text: &str) -> Vec<String> {
    spans(text)
        .into_iter()
        .map(|(_, _, c)| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

/// The code spans of `text`: the start, the end (after the last backtick) and
/// the content.
fn spans(text: &str) -> Vec<(usize, usize, &str)> {
    let b = text.as_bytes();
    let run = |i: usize| b[i..].iter().take_while(|&&c| c == b'`').count();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let n = run(i);
        let mut j = i + n;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let m = run(j);
                if m == n {
                    close = Some(j);
                    break;
                }
                j += m;
            } else {
                j += 1;
            }
        }
        match close {
            Some(j) => {
                out.push((i, j + n, &text[i + n..j]));
                i = j + n;
            }
            None => i += n,
        }
    }
    out
}

/// The tasks of the `Order` section of a plan, in the order of the file. A
/// task whose id an earlier task has gets `duplicate = true`. Ids are case
/// sensitive.
pub fn plan_tasks(plan: &Doc) -> Vec<PlanTask> {
    tasks_of(section_items(plan, ItemSection::Order))
}

/// The tasks of the items of an `Order` section (see `plan_tasks`).
fn tasks_of<'a>(items: impl Iterator<Item = &'a ListItem>) -> Vec<PlanTask> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut tasks: Vec<PlanTask> = items.filter_map(parse_task).collect();
    for t in &mut tasks {
        t.duplicate = !seen.insert(t.id.clone());
    }
    tasks
}

/// The first task that is not closed (not done and not skipped) and is not a
/// duplicate.
pub fn next_open_task(tasks: &[PlanTask]) -> Option<&PlanTask> {
    tasks.iter().find(|t| !t.closed() && !t.duplicate)
}

impl Change {
    /// The tasks of the plan, in the order of the file. A change with no plan
    /// has none.
    pub fn tasks(&self, corpus: &Corpus) -> Vec<PlanTask> {
        match &self.plan {
            Some(a) => plan_tasks(&corpus.docs[a.doc as usize]),
            None => Vec::new(),
        }
    }

    /// The first open task of the plan, in the order of the file.
    pub fn next_open_task(&self, corpus: &Corpus) -> Option<PlanTask> {
        next_open_task(&self.tasks(corpus)).cloned()
    }

    /// The open questions of the spec: the items `- [ ]` of its `Open
    /// questions` section. A resolved question (`- [x]`), an item with no task
    /// box and the one item that states "nothing" (`- [ ] None.`) are not
    /// open.
    pub fn open_questions<'a>(&self, corpus: &'a Corpus) -> Vec<&'a ListItem> {
        let Some(a) = &self.spec else {
            return Vec::new();
        };
        let doc = &corpus.docs[a.doc as usize];
        // One item that states "nothing" is no open question (D28).
        if states_nothing(doc, ItemSection::OpenQuestions) {
            return Vec::new();
        }
        section_items(doc, ItemSection::OpenQuestions)
            .filter(|i| i.task == Some(false))
            .collect()
    }
}

/// The targets `<id>#<key>` of the valid `ADD` lines of the changes with an
/// `accepted` intent, if the tree has no such requirement yet. A `covers:` tag
/// can name such a requirement before `finish` writes it (SD105). The id is as
/// the line writes it.
pub fn pending_adds(corpus: &Corpus) -> HashSet<String> {
    let mut out = HashSet::new();
    for c in collect(corpus) {
        if c.intent_status() != "accepted" {
            continue;
        }
        for line in c.delta(corpus) {
            if let Ok(d) = line.parsed
                && d.verb == Verb::Add
                && target_req(corpus, &d.id, &d.key).is_none()
            {
                out.insert(d.target());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::parse::parse;
    use std::sync::Arc;

    fn cfg() -> Config {
        Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap()
    }

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        Corpus::build(
            &cfg(),
            files.iter().map(|(p, _)| p.to_string()).collect(),
            files
                .iter()
                .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
                .collect(),
        )
    }

    fn item(text: &str) -> ListItem {
        let src = format!("# S\n\n## Delta\n\n{text}\n");
        let d = parse(&src);
        assert_eq!(d.items.len(), 1, "{text}");
        d.items[0].clone()
    }

    const VALVE: &str = "---\nstatus: active\n---\n# Valve\n\n## Requirements\n\n- **close**: The valve SHALL close in 5 seconds.\n- **open**: The valve SHALL open in 5 seconds. <!-- aliases: x#y -->\n";

    fn valve_hash(key: &str) -> (String, String) {
        let d = parse(VALVE);
        let r = d.reqs.iter().find(|r| r.key == key).unwrap();
        (hex(&r.hash), hex(&r.frozen))
    }

    #[test]
    fn a_delta_line_has_one_of_3_forms() {
        let (h, _) = valve_hash("close");
        let ok = |t: &str| parse_delta(&item(&format!("- {t}"))).unwrap();
        let add = ok("ADD requirements/valve#flush: The valve SHALL flush.");
        assert_eq!(add.verb, Verb::Add);
        assert_eq!(add.target(), "requirements/valve#flush");
        assert_eq!(add.hash, None);
        assert_eq!(add.sentence.as_deref(), Some("The valve SHALL flush."));
        let change = ok(&format!("CHANGE requirements/valve#close@{h}: New text."));
        assert_eq!(change.verb, Verb::Change);
        assert_eq!(change.hash.as_deref(), Some(h.as_str()));
        let remove = ok(&format!("REMOVE requirements/valve#close@{h}"));
        assert_eq!(remove.verb, Verb::Remove);
        assert_eq!(remove.sentence, None);
        // The hash can have capital letters. It is stored in lowercase. A
        // line that wraps is one paragraph.
        let wrapped = ok("CHANGE a/b#c-1@0123456789ABCDEF: First\n  second.");
        assert_eq!(wrapped.hash.as_deref(), Some("0123456789abcdef"));
        assert_eq!(wrapped.sentence.as_deref(), Some("First second."));
        // The text can have a colon, a link and a marker.
        let text = ok("ADD a#b: Use [x](../x.md#y): it SHALL work. <!-- aliases: a#c -->");
        assert_eq!(
            text.sentence.as_deref(),
            Some("Use [x](../x.md#y): it SHALL work. <!-- aliases: a#c -->")
        );
    }

    #[test]
    fn grammar_table_of_invalid_lines() {
        let bad = [
            ("- add a#b: lower case verb", "starts with"),
            ("- ADDS a#b: another word", "starts with"),
            ("- Update a#b: not a verb", "starts with"),
            ("- **k**: A requirement SHALL not be here.", "starts with"),
            ("- ADD", "target"),
            ("- ADD a#b", "`:`"),
            ("- ADD a#b:", "no new text"),
            ("- ADD a#b:   ", "no new text"),
            ("- ADD a: text", "<id>#<key>"),
            ("- ADD #b: text", "<id>#<key>"),
            ("- ADD a#: text", "not a key"),
            ("- ADD a#Bad_Key: text", "not a key"),
            ("- ADD a#b c: text", "no space"),
            ("- ADD a #b: text", "no space"),
            ("- ADD a#b#c: text", "not a key"),
            ("- ADD a@1#b: text", "<id>#<key>"),
            ("- ADD a#b@0123456789abcdef: text", "no hash"),
            ("- CHANGE a#b: text", "16 digit hash"),
            ("- CHANGE a#b@abc: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdeg: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdef01: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdef:", "no new text"),
            ("- CHANGE a#b@0123456789abcdef@1: text", "16 hexadecimal"),
            ("- REMOVE a#b", "16 digit hash"),
            ("- REMOVE a#b@0123456789abcdef: text", "no text"),
            ("- REMOVE a#b@0123456789abcdef text", "no text"),
            ("- ADD a#b: one\n\n  second paragraph", "one paragraph"),
            ("- ADD a#b: one\n  - nested", "nested"),
            ("-", "starts with"),
        ];
        for (text, reason) in bad {
            let err = parse_delta(&item(text)).unwrap_err();
            assert!(err.contains(reason), "{text:?}: {err}");
        }
    }

    #[test]
    fn a_spec_with_bad_lines_does_not_stop_the_parse() {
        let spec = "---\nstatus: draft\n---\n# S\n\n## Delta\n\n- ADD a#b: One.\n- garbage\n- ADD a#b: Two.\n- REMOVE a#c@0123456789abcdef\n- [ ] ADD a#d: task box\n\n  second\n";
        let c = corpus(&[("docs/intent/x/spec.md", spec)]);
        let lines = delta_lines(&c.docs[0]);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].parsed.is_ok());
        assert!(lines[1].parsed.is_err());
        // The second line for the key `a#b` is invalid, and so the first
        // line stays the only valid one for it.
        assert!(lines[2].parsed.as_ref().unwrap_err().contains("already"));
        assert!(lines[3].parsed.is_ok());
        assert!(lines[4].parsed.is_err());
        assert_eq!(lines[1].line, 9);
    }

    #[test]
    fn hash_matches_accepts_the_hash_and_the_frozen_hash() {
        let with_link = "- **k**: Use [a](../a.md#x) here.\n";
        let moved = "- **k**: Use [a](../b/a.md#x) here.\n";
        let a = parse(with_link);
        let b = parse(moved);
        let (ha, hb) = (hex(&a.reqs[0].hash), hex(&b.reqs[0].hash));
        assert_ne!(ha, hb);
        // The line has the hash of the old text. The new text has another
        // hash, but the same hash `frozen`.
        assert!(hash_matches(&a.reqs[0], &ha));
        assert!(!hash_matches(&b.reqs[0], &ha));
        assert!(hash_matches(&b.reqs[0], &hex(&a.reqs[0].frozen)));
        assert!(hash_matches(&b.reqs[0], &hb.to_uppercase()));
    }

    #[test]
    fn changes_are_collected_by_folder() {
        let intent = "---\nstatus: accepted\nclass: higher-risk\nareas: [valves, sensors]\nfrom: incidents/flood\n---\n# I\n\n## Intent\n\nText.\n";
        let c = corpus(&[
            (
                "docs/intent/b-change/spec.md",
                "---\nstatus: draft\n---\n# S\n",
            ),
            (
                "docs/intent/a-change/plan.md",
                "---\nstatus: approved\n---\n# P\n",
            ),
            ("docs/intent/a-change/intent.md", intent),
            (
                "docs/intent/a-change/spec.md",
                "---\nstatus: approved\n---\n# S\n",
            ),
            ("docs/requirements/valve.md", VALVE),
            // Not a place of an intent type: no change.
            (
                "docs/intent/loose.md",
                "---\ntype: intent\nstatus: draft\n---\n# L\n",
            ),
        ]);
        let changes = collect(&c);
        assert_eq!(changes.len(), 2);
        let a = &changes[0];
        assert_eq!(
            (a.slug.as_str(), a.dir.as_str()),
            ("a-change", "docs/intent/a-change")
        );
        assert_eq!(a.intent_status(), "accepted");
        assert_eq!(a.spec.as_ref().unwrap().status, "approved");
        assert_eq!(a.plan.as_ref().unwrap().status, "approved");
        assert_eq!(a.class, Class::HigherRisk);
        assert_eq!(a.areas, ["valves", "sensors"]);
        assert_eq!(a.from.as_deref(), Some("incidents/flood"));
        let b = &changes[1];
        assert_eq!(b.slug, "b-change");
        assert!(b.intent.is_none() && b.plan.is_none());
        assert_eq!((b.class, b.intent_status()), (Class::Routine, ""));
    }

    #[test]
    fn files_are_the_first_code_span_of_each_item() {
        let plan = "---\nstatus: draft\n---\n# P\n\n## Files\n\n- `src/valve.py` and `src/other.py`\n- `docs/`: the folder\n- no code here\n- [`tests/*.py`](x.md)\n  - `nested.py`\n\n## Order\n\n- `not-a-file.py`\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        // A code span in a link text is a code span.
        assert_eq!(ch.files(&c), ["src/valve.py", "docs/", "tests/*.py"]);
    }

    #[test]
    fn files_accept_plain_paths_and_code_spans() {
        let plan = "---\nstatus: draft\n---\n# P\n\n## Files\n\n- src/valve.py\n- tests/*.py,\n- docs/.\n- `src/code.py` and src/plain.py\n- [src/link.py](src/link.py)\n- [the file](src/other.py)\n- Makefile\n- no code here\n- the code in src/valve.py\n- src/a.py src/b.py\n- https://example.com/a.html\n- ./lib/x.rs\n- src/dose/\n  - nested.py\n\n## Order\n\n- src/not-a-file.py\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        assert_eq!(
            ch.files(&c),
            [
                "src/valve.py",
                "tests/*.py",
                "docs/",
                "src/code.py",
                "src/link.py",
                "./lib/x.rs",
                "src/dose/"
            ]
        );
        // The items that give no pattern, in the order of the file.
        assert_eq!(
            ch.files_not_paths(&c),
            [
                "[the file](src/other.py)",
                "Makefile",
                "no code here",
                "the code in src/valve.py",
                "src/a.py src/b.py",
                "https://example.com/a.html"
            ]
        );
    }

    #[test]
    fn files_of_a_crlf_plan_accept_plain_paths() {
        let plan = "---\r\nstatus: draft\r\n---\r\n# P\r\n\r\n## Files\r\n\r\n- src/valve.py\r\n- `src/code.py`\r\n- two words\r\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        assert_eq!(ch.files(&c), ["src/valve.py", "src/code.py"]);
        assert_eq!(ch.files_not_paths(&c), ["two words"]);
    }

    #[test]
    fn a_plain_path_is_one_token_that_looks_like_a_path() {
        let ok = [
            ("src/a.py", "src/a.py"),
            ("  src/a.py  ", "src/a.py"),
            ("src/a.py.", "src/a.py"),
            ("src/a.py,", "src/a.py"),
            ("src/a.py:", "src/a.py"),
            ("src/", "src/"),
            ("README.md", "README.md"),
            (".github/ci.yml", ".github/ci.yml"),
            ("tests/*.py", "tests/*.py"),
            ("**/*.rs", "**/*.rs"),
            ("*.toml", "*.toml"),
            ("[src/a.py](src/a.py)", "src/a.py"),
            ("[src/a.py](src/a.py).", "src/a.py"),
            ("[src/a.py](../../../src/a.py)", "src/a.py"),
            ("app/[slug]/page.tsx", "app/[slug]/page.tsx"),
            // Known limits: a token with a `/` or a `.` is a path.
            ("n/a", "n/a"),
            ("[[requirements/valves]]", "[[requirements/valves]]"),
            ("**src/a.py**", "**src/a.py**"),
        ];
        for (text, want) in ok {
            assert_eq!(plain_path(text).as_deref(), Some(want), "{text:?}");
        }
        for text in [
            "",
            ".",
            "no code here",
            "None.",
            "Makefile",
            "src/a.py and src/b.py",
            "src/a.py\nsrc/b.py",
            "the code in src/a.py",
            "[the file](src/a.py)",
            "https://example.com/a.html",
            "`src/a.py`",
            "<src/a.py>",
            "{{path}}",
            "{{src/path.py}}",
            "...",
            "[the file](../src/a.py)",
        ] {
            assert_eq!(plain_path(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_placeholder_is_not_a_path_also_in_a_code_span() {
        let plan = "---\nstatus: draft\n---\n# P\n\n## Files\n\n- `{{path}}`: {{Why.}}\n- `src/a.py`\n\n## Order\n\n- [ ] t1: Text. Files: `{{path}}`, `src/b.py`. Verify: ok.\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        assert_eq!(ch.files(&c), ["src/a.py", "src/b.py"]);
        assert_eq!(ch.files_not_paths(&c), ["`{{path}}`: {{Why.}}"]);
        assert_eq!(ch.tasks(&c)[0].files, ["src/b.py"]);
    }

    #[test]
    fn a_long_item_is_cut_for_a_message() {
        let long = "word ".repeat(40);
        let plan = format!("---\nstatus: draft\n---\n# P\n\n## Files\n\n- {long}\n  more\n");
        let c = corpus(&[("docs/intent/x/plan.md", &plan)]);
        let shown = &collect(&c)[0].files_not_paths(&c)[0];
        assert_eq!(shown.chars().count(), 80, "{shown}");
        assert!(shown.ends_with("..."), "{shown}");
        assert!(!shown.contains('\n'));
    }

    #[test]
    fn task_files_accept_plain_paths_and_code_spans() {
        let t = tasks(&[
            "- [ ] t1: Text. Files: src/a.py, tests/*.py. Verify: ok.",
            "- [ ] t2: Text. Files: `src/a.py`, src/b.py. Verify: ok.",
            "- [ ] t3: Text. Files: src/a.py and src/b.py.",
            "- [ ] t4: Text. Files: none.",
            "- [ ] t5: Text. Verify: ok. Files: docs/.",
            "- [ ] t6: Text. Files: [src/a.py](src/a.py), `x`.",
        ]);
        let f = |n: usize| t[n].files.clone();
        assert_eq!(f(0), ["src/a.py", "tests/*.py"]);
        // An entry with a code span gives its code span. An entry with no code
        // span gives its plain path. No entry is dropped.
        assert_eq!(f(1), ["src/a.py", "src/b.py"]);
        // An entry with more than one word is prose.
        assert_eq!(f(2), Vec::<String>::new());
        assert_eq!(f(3), Vec::<String>::new());
        assert_eq!(f(4), ["docs/"]);
        assert_eq!(f(5), ["src/a.py", "x"]);
        assert_eq!(t[0].verify.as_deref(), Some("ok"));
        let t = tasks_of_source(
            "# P\r\n\r\n## Order\r\n\r\n- [ ] t1: One. Files: src/a.py,\r\n  src/b.py. Verify: `go`.\r\n- [ ] t2: Two. Files: src/c.py.\r\n",
        );
        assert_eq!(t[0].files, ["src/a.py", "src/b.py"]);
        assert_eq!(t[1].files, ["src/c.py"]);
    }

    #[test]
    fn the_labels_hours_and_skip_are_read() {
        let t = tasks(&[
            "- [ ] t1: Text. Files: `a`. Verify: ok. Hours: 1.5. Skip: not needed.",
            "- [ ] t2: Text. Skip: reason one. Hours: 1-2. Files: `a`.",
            "- [ ] t3: Text. Hours:  3 \t h",
            "- [ ] t4: Text. Hours: . Skip:",
            "- [x] t5: Text. hours: 2. skip: no.",
            "- [ ] t6: Text with `Hours: 9` and `Skip: x` in code.",
            "- [ ] t7: Text. Hours: 1. Hours: 2. Skip: a. Skip: b.",
            "- [ ] t8: Text.Hours: 1.",
            "- [x] t9: Text.\n  Skip: the second\n  line.",
        ]);
        let h = |n: usize| t[n].hours.as_deref();
        let k = |n: usize| t[n].skip.as_deref();
        assert_eq!((h(0), k(0)), (Some("1.5"), Some("not needed")));
        assert_eq!((h(1), k(1)), (Some("1-2"), Some("reason one")));
        assert_eq!(t[1].files, ["a"]);
        assert_eq!(h(2), Some("3 h"));
        // An empty `Hours:` is no value, and an empty `Skip:` is an empty reason.
        assert_eq!((h(3), k(3)), (None, Some("")));
        // A label is case-sensitive, and a code span does not hold one.
        assert_eq!((h(4), k(4)), (None, None));
        assert_eq!((h(5), k(5)), (None, None));
        // The first label of each kind counts.
        assert_eq!((h(6), k(6)), (Some("1"), Some("a")));
        assert_eq!((h(7), k(7)), (None, None));
        assert_eq!(k(8), Some("the second line"));
        // The state: a non-empty `Skip:` closes the task, whatever its box.
        assert!(t[0].skipped() && t[0].closed() && !t[0].is_done());
        assert!(t[1].skipped() && !t[1].is_done());
        assert!(!t[3].skipped() && !t[3].closed() && !t[3].is_done());
        assert!(t[4].is_done() && t[4].closed() && !t[4].skipped());
        assert!(!t[2].closed());
    }

    #[test]
    fn the_next_open_task_skips_a_skipped_task() {
        let t = tasks(&[
            "- [ ] t1: One. Skip: not needed.",
            "- [ ] t2: Two. Skip:",
            "- [ ] t3: Three.",
        ]);
        assert_eq!(next_open_task(&t).unwrap().id, "t2");
        let t = tasks(&["- [x] t1: One.", "- [ ] t2: Two. Skip: no."]);
        assert!(next_open_task(&t).is_none());
    }

    #[test]
    fn a_label_hours_ends_the_verify_text() {
        let t = tasks(&[
            "- [ ] t1: Text. Verify: `cargo test`. Hours: 2.",
            "- [ ] t2: Text. Hours: 2. Verify: read the log.",
            "- [ ] t3: Text. Files: `a`. Hours: 1. Skip: the branch is gone. Verify: ok.",
        ]);
        assert_eq!(t[0].verify.as_deref(), Some("`cargo test`"));
        assert_eq!(t[0].hours.as_deref(), Some("2"));
        assert_eq!(t[1].verify.as_deref(), Some("read the log"));
        assert_eq!(t[1].text, "Text");
        assert_eq!(t[2].files, ["a"]);
        assert_eq!(t[2].files_text.as_deref(), Some("`a`"));
        assert_eq!(t[2].skip.as_deref(), Some("the branch is gone"));
        assert_eq!(t[2].verify.as_deref(), Some("ok"));
    }

    #[test]
    fn a_value_of_hours_is_a_number_or_a_range() {
        let h = |s: &str| parse_hours(s);
        assert_eq!(h("2"), Some((200, 200)));
        assert_eq!(h("1.5"), Some((150, 150)));
        assert_eq!(h("0.05h"), Some((5, 5)));
        assert_eq!(h("9999.99"), Some((999_999, 999_999)));
        assert_eq!(h("1-2"), Some((100, 200)));
        assert_eq!(h("1.5 - 2.25h"), Some((150, 225)));
        assert_eq!(h("3 \t-\t 3"), Some((300, 300)));
        assert_eq!(h("0-0.5h"), Some((0, 50)));
        for bad in [
            "",
            "soon",
            "2 (estimate)",
            "3-1",
            "1-",
            "-1",
            "1--2",
            "1-2-3",
            "12345",
            "1.",
            ".5",
            "1.234",
            "2 h",
            "2hours",
            "1,5",
            "1 2",
            "1-2x",
            "+1",
            "1e2",
        ] {
            assert_eq!(h(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn task_files_split_only_outside_code_spans_and_links() {
        let t = tasks(&[
            "- [ ] t1: Text. Files: [a, b](src/a.py), src/c.py. Verify: ok.",
            "- [ ] t2: Text. Files: [src/a.py](../a, b), src/b.py;",
            "- [ ] t3: Text. Files: (src/a.py, src/b.py).",
            "- [ ] t4: Text. Files: src/a.py; Verify: ok.",
            "- [ ] t5: Text. Files: [src/a.py](src/a.py), [src/b.py](src/b.py).",
            "- [ ] t6: Text. Files: app/[slug]/page.tsx, `x, y`.",
            "- [ ] t7: Text. Files: (src/a.py).",
        ]);
        let f = |n: usize| t[n].files.clone();
        // The link text `a, b` is prose, and the comma in it is no cut.
        assert_eq!(f(0), ["src/c.py"]);
        assert_eq!(f(1), ["src/a.py", "src/b.py"]);
        assert_eq!(f(2), ["src/a.py", "src/b.py"]);
        assert_eq!(f(3), ["src/a.py"]);
        assert_eq!(f(4), ["src/a.py", "src/b.py"]);
        assert_eq!(f(5), ["app/[slug]/page.tsx", "x, y"]);
        assert_eq!(f(6), ["src/a.py"]);
        assert_eq!(plain_path("src/a.py;").as_deref(), Some("src/a.py"));
        assert_eq!(plain_path("(src/a.py)").as_deref(), Some("src/a.py"));
        // The parentheses do not wrap the whole text.
        assert_eq!(plain_path("(a)/(b)").as_deref(), Some("(a)/(b)"));
    }

    #[test]
    fn files_are_the_section_then_the_task_lines_with_no_repeat() {
        let plan = "---\nstatus: draft\n---\n# P\n\n## Files\n\n- None.\n- `src/a.py`\n\n## Order\n\n- [ ] t1: One. Files: src/b.py, `src/a.py`.\n- [x] t2: Two. Files: tests/*.py.\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        assert_eq!(ch.files(&c), ["src/a.py", "src/b.py", "tests/*.py"]);
        assert_eq!(ch.files_not_paths(&c), ["None."]);
    }

    #[test]
    fn a_files_section_with_one_none_item_states_no_files() {
        let plan = |files: &str, order: &str| {
            format!(
                "---\nstatus: approved\n---\n# P\n\n## Files\n\n{files}\n\n## Order\n\n{order}\n"
            )
        };
        let check = |files: &str, order: &str| {
            let c = corpus(&[("docs/intent/x/plan.md", &plan(files, order))]);
            let ch = &collect(&c)[0];
            (ch.no_files(&c), ch.files_not_paths(&c))
        };
        // The word is `none` in any case, and a mark or a sentence may follow.
        for item in [
            "- None.",
            "- none",
            "- NONE: research only",
            "-   None, no file.",
        ] {
            assert_eq!(check(item, ""), (true, vec![]), "{item}");
        }
        // Another word, a code span, a path with the letters, and a second
        // item are not the form.
        assert!(!check("- nonexistent", "").0);
        assert!(!check("- `none`", "").0);
        assert!(!check("- No files.", "").0);
        assert!(!check("", "").0);
        assert_eq!(
            check("- None.\n- Later.", ""),
            (false, vec!["None.".into(), "Later.".into()])
        );
        // A task that lists a path names a file, so the plan lists files.
        assert!(!check("- None.", "- [ ] t1: Edit. Files: `src/a.py`.").0);
        // A change with no plan lists no file and states nothing.
        let c = corpus(&[(
            "docs/intent/x/intent.md",
            "---\nstatus: accepted\n---\n# I\n",
        )]);
        assert!(!collect(&c)[0].no_files(&c));
    }

    #[test]
    fn the_none_form_of_d28_needs_a_colon_a_full_stop_or_the_end() {
        for t in [
            "none",
            "None",
            "NONE.",
            "none: research only",
            "None. This change is research.",
            "  none  ",
            "none:",
        ] {
            assert!(is_none_text(t), "{t}");
        }
        for t in [
            "",
            "None of the old files stay",
            "none, research only",
            "nonexistent",
            "`none`",
            "no files",
            "none-of",
            "- none",
            "nonexistent.md",
            "none/x",
            "nones.",
        ] {
            assert!(!is_none_text(t), "{t}");
        }
    }

    #[test]
    fn a_paragraph_under_files_states_no_files_or_asks_for_a_list() {
        let plan = |files: &str, order: &str| {
            format!(
                "---\nstatus: approved\n---\n# P\n\n## Files\n\n{files}\n\n## Order\n\n{order}\n"
            )
        };
        // (no_files, paragraph only)
        let check = |files: &str, order: &str| {
            let c = corpus(&[("docs/intent/x/plan.md", &plan(files, order))]);
            let ch = &collect(&c)[0];
            (ch.no_files(&c), ch.files_paragraph_only(&c))
        };
        for para in [
            "None.",
            "none",
            "NONE: this change is research.",
            "None. The change\nedits no file.",
        ] {
            assert_eq!(check(para, ""), (true, false), "{para}");
        }
        for para in [
            "None of the old files stay.",
            "The code of the dose.",
            "nonexistent.md",
            "`none`",
        ] {
            assert_eq!(check(para, ""), (false, true), "{para}");
        }
        // A task that lists a path names a file, so the plan lists files.
        assert_eq!(
            check("None.", "- [ ] t1: Edit. Files: `src/a.py`."),
            (false, false)
        );
        // A list item decides: the paragraph is only text.
        assert_eq!(check("The code.\n\n- `src/a.py`", ""), (false, false));
        assert_eq!(check("None.\n\n- `src/a.py`", ""), (false, false));
        assert_eq!(check("None.\n\n- None.", ""), (true, false));
        // No paragraph and no item: nothing.
        assert_eq!(check("", ""), (false, false));
        // A paragraph gives no `files_not_paths` entry.
        let c = corpus(&[("docs/intent/x/plan.md", &plan("The code.", ""))]);
        assert!(collect(&c)[0].files_not_paths(&c).is_empty());
    }

    #[test]
    fn one_none_item_states_nothing_in_the_sections_of_a_spec() {
        let spec = |body: &str| {
            let c = corpus(&[(
                "docs/intent/x/spec.md",
                &format!("---\nstatus: approved\n---\n# S\n\n{body}"),
            )]);
            let d = &c.docs[0];
            (
                states_nothing(d, ItemSection::Concerns),
                open_concerns(d).len(),
                collect(&c)[0].open_questions(&c).len(),
            )
        };
        assert_eq!(spec("## Concerns\n\n- [ ] None.\n"), (true, 0, 0));
        assert_eq!(spec("## Concerns\n\n- none\n"), (true, 0, 0));
        assert_eq!(spec("## Concerns\n\n- [x] None: no risk.\n"), (true, 0, 0));
        assert_eq!(
            spec("## Concerns\n\n- [ ] None of the risks is open.\n"),
            (false, 1, 0)
        );
        assert_eq!(
            spec("## Concerns\n\n- [ ] None.\n- [ ] A risk.\n"),
            (false, 2, 0)
        );
        assert_eq!(spec("## Open questions\n\n- [ ] None.\n"), (false, 0, 0));
        assert_eq!(
            spec("## Open questions\n\n- [ ] None.\n- [ ] Who?\n"),
            (false, 0, 2)
        );
        assert_eq!(spec("## Open questions\n\n- [ ] Who?\n"), (false, 0, 1));
    }

    #[test]
    fn scope_matches_globs_and_folders() {
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let pats = p(&[
            "src/valve.py",
            "docs/",
            "tests/*.py",
            "./lib/**/x.rs",
            "/top.md",
        ]);
        for path in [
            "src/valve.py",
            "docs/a.md",
            "docs/a/b/c.md",
            "tests/a.py",
            "lib/x.rs",
            "lib/a/b/x.rs",
            "top.md",
        ] {
            assert!(in_scope(&pats, path), "{path}");
        }
        for path in [
            "src/valve.py.bak",
            "src/other.py",
            "documents/a.md",
            "tests/a/b.py",
            "lib/x.rs.in",
        ] {
            assert!(!in_scope(&pats, path), "{path}");
        }
        // A plain path with no final `/` does not match the files below it.
        assert!(!in_scope(&p(&["src"]), "src/a.rs"));
        assert!(in_scope(&p(&["src/"]), "src/a.rs"));
        assert!(!in_scope(&p(&["", "/", "./"]), "a"));
        assert!(!in_scope(&[], "a"));
    }

    fn base_and_head() -> (Corpus, Corpus) {
        let base = corpus(&[("docs/requirements/valve.md", VALVE)]);
        let head_text = VALVE
            .replace("close in 5 seconds", "close in 3 seconds")
            .replace(
                "- **open**: The valve SHALL open in 5 seconds. <!-- aliases: x#y -->\n",
                "",
            )
            + "- **flush**: The valve SHALL flush the line.\n";
        let head = corpus(&[("docs/requirements/valve.md", &head_text)]);
        (base, head)
    }

    fn line(text: &str) -> DeltaLine {
        let spec = format!("---\nstatus: draft\n---\n# S\n\n## Delta\n\n- {text}\n");
        let c = corpus(&[("docs/intent/x/spec.md", &spec)]);
        delta_lines(&c.docs[0]).remove(0)
    }

    #[test]
    fn applied_follows_the_rules_of_t3() {
        let (base, head) = base_and_head();
        let (hc, _) = valve_hash("close");
        let (ho, _) = valve_hash("open");
        let t = "requirements/valve";
        let check = |text: String, want: Applied| {
            assert_eq!(applied(&line(&text), &base, &head), want, "{text}");
        };
        // ADD: the base has no key, and the head has the text.
        check(
            format!("ADD {t}#flush: The valve SHALL flush the line."),
            Applied::Yes,
        );
        check(format!("ADD {t}#flush: Other text."), Applied::No);
        check(
            format!("ADD {t}#absent: The valve SHALL flush the line."),
            Applied::No,
        );
        // The key is in the base: not an ADD that applies, also if the text is there.
        check(
            format!("ADD {t}#close: The valve SHALL close in 3 seconds."),
            Applied::No,
        );
        // CHANGE: the head has the text.
        check(
            format!("CHANGE {t}#close@{hc}: The valve SHALL close in 3 seconds."),
            Applied::Yes,
        );
        check(
            format!("CHANGE {t}#close@{hc}: The valve SHALL close in 4 seconds."),
            Applied::No,
        );
        check(format!("CHANGE {t}#absent@{hc}: Text."), Applied::No);
        // REMOVE: the base has the key and the head has not.
        check(format!("REMOVE {t}#open@{ho}"), Applied::Yes);
        check(format!("REMOVE {t}#close@{hc}"), Applied::No);
        // A key in neither tree, and a line with no valid form.
        check(format!("REMOVE {t}#nothing@{hc}"), Applied::Invalid);
        check(
            format!("REMOVE nothing/here#nothing@{hc}"),
            Applied::Invalid,
        );
        check("garbage".to_string(), Applied::Invalid);
        // A marker and the layout of the line do not change the text.
        check(
            format!(
                "ADD {t}#flush:  The valve   SHALL flush\n  the line. <!-- verified-by: tests/a.rs -->"
            ),
            Applied::Yes,
        );
    }

    #[test]
    fn pending_adds_come_from_accepted_changes() {
        let spec = "---\nstatus: draft\n---\n# S\n\n## Delta\n\n- ADD requirements/valve#flush: Text.\n- ADD requirements/valve#close: Text.\n- CHANGE requirements/valve#open@0123456789abcdef: Text.\n- ADD requirements/new#first: Text.\n";
        let intent = |st: &str| format!("---\nstatus: {st}\n---\n# I\n\n## Intent\n\nText.\n");
        let c = corpus(&[
            ("docs/requirements/valve.md", VALVE),
            ("docs/intent/a/intent.md", &intent("accepted")),
            ("docs/intent/a/spec.md", spec),
            ("docs/intent/b/intent.md", &intent("draft")),
            ("docs/intent/b/spec.md", spec),
            ("docs/intent/c/intent.md", &intent("closed")),
            ("docs/intent/c/spec.md", spec),
        ]);
        let got = pending_adds(&c);
        let mut got: Vec<&str> = got.iter().map(String::as_str).collect();
        got.sort();
        // `valve#close` is in the tree already.
        assert_eq!(got, ["requirements/new#first", "requirements/valve#flush"]);
    }

    /// The tasks of the `Order` section of a plan with the lines `lines`.
    fn tasks(lines: &[&str]) -> Vec<PlanTask> {
        let src = format!("# P\n\n## Order\n\n{}\n", lines.join("\n"));
        tasks_of_source(&src)
    }

    fn tasks_of_source(src: &str) -> Vec<PlanTask> {
        let d = parse(src);
        let items = d.items.iter().filter(|i| i.section == ItemSection::Order);
        tasks_of(items)
    }

    #[test]
    fn task_ids_follow_the_grammar() {
        for id in ["t1", "T-1", "a_b", "0x", "plan-3", "A", "a--b__c", "123"] {
            let t = tasks(&[&format!("- [ ] {id}: Do it.")]);
            assert_eq!(t.len(), 1, "{id}");
            assert_eq!(t[0].id, id);
            assert_eq!(t[0].text, "Do it");
        }
        for line in [
            "- [ ] -t1: x",
            "- [ ] _t1: x",
            "- [ ] t 1: x",
            "- [ ] t1 x",
            "- [ ] t1 : x",
            "- [ ] t.1: x",
            "- [ ] \u{fc}1: x",
            "- [ ] : x",
            "- [ ] t1:x",
            "- [ ] https://example.org/a",
            "- [ ] **t1**: x",
            "- [ ] `t1`: x",
            "- [ ] Write the code",
            "- [ ]",
            // No task box: not a task.
            "- t1: x",
            "1. t1: x",
        ] {
            assert!(tasks(&[line]).is_empty(), "{line}");
        }
        // The colon can end the item.
        assert_eq!(tasks(&["- [ ] t1:"]).len(), 1);
        // Another list marker is fine, and so is a number.
        let t = tasks(&["* [ ] a: x", "+ [x] b: y", "1. [ ] c: z", "2) [X] d: w"]);
        let got: Vec<_> = t.iter().map(|t| (t.id.as_str(), t.done)).collect();
        assert_eq!(got, [("a", false), ("b", true), ("c", false), ("d", true)]);
    }

    #[test]
    fn a_line_that_is_not_a_task_is_ignored() {
        let t = tasks(&[
            "- Prose item.",
            "- [ ] t1: First.",
            "- [ ] Not a task",
            "- [x] t2: Second.",
        ]);
        let ids: Vec<_> = t.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["t1", "t2"]);
        // A task box outside `Order` is not a task of the plan.
        let d = parse("# P\n\n## Risks\n\n- [ ] t1: x\n");
        assert!(tasks_of(d.items.iter().filter(|i| i.section == ItemSection::Order)).is_empty());
    }

    #[test]
    fn files_and_verify_are_labels_after_the_text() {
        let t = tasks(&[
            "- [ ] t1: Add the timer. Files: `src/timer.rs`, `src/*.rs`. Verify: `cargo test timer`.",
            "- [ ] t2: Only text.",
            "- [ ] t3: Text. Verify: read the log. Files: `a/`.",
            "- [x] t4: Text. Files: no code span here. Verify: .",
            "- [ ] t5: Text. files: `x`. Verify : `y`.",
            "- [ ] t6: Text.Files: `x`.",
            "- [ ] t7: Text with `Files: not a label` inside code. Verify: ok",
            "- [ ] t8: Text. Files: ``a `b` c``, ` d `.",
            "- [ ] t9: Text. Files: `a`. Files: `b`. Verify: one. Verify: two.",
        ]);
        let f = |n: usize| (t[n].text.as_str(), t[n].files.clone(), t[n].verify.clone());
        assert_eq!(
            f(0),
            (
                "Add the timer",
                vec!["src/timer.rs".to_string(), "src/*.rs".to_string()],
                Some("`cargo test timer`".to_string())
            )
        );
        assert_eq!(f(1), ("Only text", vec![], None));
        // The order of the labels does not matter.
        assert_eq!(
            f(2),
            (
                "Text",
                vec!["a/".to_string()],
                Some("read the log".to_string())
            )
        );
        // An empty `Verify:` is no text.
        assert_eq!(f(3), ("Text", vec![], None));
        // A label is case-sensitive, and it follows white space.
        assert_eq!(f(4).1, Vec::<String>::new());
        assert_eq!(f(4).0, "Text. files: `x`. Verify : `y`");
        assert_eq!(f(5), ("Text.Files: `x`", vec![], None));
        // A code span does not hold a label.
        assert_eq!(
            f(6),
            (
                "Text with `Files: not a label` inside code",
                vec![],
                Some("ok".to_string())
            )
        );
        // A code span with a longer run of backticks, and the trim of a span.
        assert_eq!(f(7).1, ["a `b` c", "d"]);
        // The first label of each kind counts.
        assert_eq!(f(8).1, ["a"]);
        assert_eq!(f(8).2.as_deref(), Some("one"));
        assert!(t[3].done && !t[0].done);
    }

    #[test]
    fn a_task_can_span_lines_and_hold_a_nested_list() {
        let src = "# P\n\n## Order\n\n- [ ] t1: First line\n  and the second line.\n  Files: `a`.\n  - [ ] t9: a nested box\n  - [x] nested two\n- [x] t2: Next.\n\n  A second paragraph.\n- [ ] t3: Last.\n";
        let t = tasks_of_source(src);
        let ids: Vec<_> = t.iter().map(|t| t.id.as_str()).collect();
        // A nested item is part of its parent, and it is no task.
        assert_eq!(ids, ["t1", "t2", "t3"]);
        assert_eq!(t[0].text, "First line and the second line");
        assert_eq!(t[0].files, ["a"]);
        assert_eq!(t[0].line, 5);
        assert_eq!(t[1].line, 10);
        assert!(src[t[0].range.start as usize..t[0].range.end as usize].ends_with("nested two\n"));
        assert!(t[1].done);
    }

    #[test]
    fn the_byte_range_of_a_task_keeps_crlf() {
        let src = "---\r\nstatus: draft\r\n---\r\n# P\r\n\r\n## Order\r\n\r\n- [ ] t1: One. Files: `a`.\r\n- [x] t2: Two.\r\n  - nested\r\n\r\n- [ ] t3: Three. Verify: `go`.";
        let t = tasks_of_source(src);
        let slice = |n: usize| &src[t[n].range.start as usize..t[n].range.end as usize];
        assert_eq!(slice(0), "- [ ] t1: One. Files: `a`.\r\n");
        assert_eq!(slice(1), "- [x] t2: Two.\r\n  - nested\r\n");
        assert_eq!(slice(2), "- [ ] t3: Three. Verify: `go`.");
        assert_eq!(t[0].text, "One");
        assert_eq!(t[0].files, ["a"]);
        assert_eq!(t[2].verify.as_deref(), Some("`go`"));
        assert_eq!((t[0].line, t[1].line, t[2].line), (8, 9, 12));
    }

    #[test]
    fn the_first_task_with_an_id_counts() {
        let t = tasks(&[
            "- [ ] t1: First.",
            "- [x] t2: Second.",
            "- [ ] t1: Again.",
            "- [ ] T1: Other case.",
            "- [ ] t2: Done again.",
        ]);
        let dup: Vec<_> = t.iter().map(|t| t.duplicate).collect();
        assert_eq!(dup, [false, false, true, false, true]);
        // `next_open_task` skips a duplicate: the open `t1` of line 3 is not it.
        assert_eq!(next_open_task(&t).unwrap().text, "First");
        let t = tasks(&["- [x] t1: First.", "- [ ] t1: Again."]);
        assert!(t[1].duplicate);
        assert!(next_open_task(&t).is_none());
    }

    #[test]
    fn the_next_open_task_is_the_first_open_task_in_file_order() {
        let plan = |lines: &str| {
            format!(
                "---\nstatus: draft\n---\n# P\n\n## Order\n\n{lines}\n\n## Risks\n\n- [ ] r1: A risk.\n"
            )
        };
        let next = |lines: &str| {
            let c = corpus(&[("docs/intent/x/plan.md", &plan(lines))]);
            collect(&c)[0].next_open_task(&c).map(|t| t.id)
        };
        assert_eq!(next("- [ ] b: x\n- [ ] a: y"), Some("b".to_string()));
        assert_eq!(
            next("- [x] b: x\n- [ ] a: y\n- [ ] c: z"),
            Some("a".to_string())
        );
        assert_eq!(next("- [x] b: x\n- [X] a: y"), None);
        assert_eq!(next("1. Step one.\n2. Step two."), None);
        assert_eq!(next(""), None);
        // A change with no plan has no task.
        let c = corpus(&[("docs/intent/x/intent.md", "---\nstatus: draft\n---\n# I\n")]);
        assert!(collect(&c)[0].tasks(&c).is_empty());
        assert_eq!(collect(&c)[0].next_open_task(&c), None);
    }

    #[test]
    fn open_questions_are_the_unchecked_items_of_the_section() {
        let spec = "---\nstatus: draft\n---\n# S\n\n## Concerns\n\n- [ ] A concern.\n\n### open QUESTIONS\n\n- [ ] Who owns it?\n- [x] Which unit? Seconds.\n- A bullet with no box.\n- [ ] Is 5 a limit?\n  - [ ] nested\n\n## Files\n\n- [ ] no\n";
        let c = corpus(&[("docs/intent/x/spec.md", spec)]);
        let ch = &collect(&c)[0];
        let q: Vec<_> = ch
            .open_questions(&c)
            .iter()
            .map(|i| i.text.clone())
            .collect();
        // The heading level and the case do not matter. `Concerns` is not it.
        assert_eq!(q, ["Who owns it?", "Is 5 a limit?"]);
        let c = corpus(&[("docs/intent/x/plan.md", "---\nstatus: draft\n---\n# P\n")]);
        assert!(collect(&c)[0].open_questions(&c).is_empty());
    }
}
