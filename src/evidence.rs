//! Evidence of requirements (docs/config.md, "Evidence"). A requirement has
//! evidence if one of these links it to a test or a verification record:
//!
//! - `verified-by` in the front matter of its document (every requirement of
//!   the document)
//! - an inline `<!-- verified-by: <path> -->` marker in the requirement
//! - a `covers: <doc-id>#<key>` tag in a test file. The tag is plain text, so
//!   it works in a comment of any language.
//!
//! A tag `covers: partial <doc-id>#<key>` records that a test proves a part of
//! a requirement. It is not evidence: [`Evidence::of`] gives full evidence
//! only, and [`Evidence::partial_of`] gives the partial tags.
//!
//! spec-dna checks that the links resolve. It does not run the tests. With
//! `[evidence] reports`, `check --reports` and `finish` also bind a tag to a
//! test case of a stamped report (design 0.8.0, D1), and a tag proves a
//! requirement only when a bound case passed.

use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::model::{Corpus, Doc, Res, canonical};
use crate::repo::{Fatal, Repo};
use crate::report::{Case, Outcome, Reads};
use std::collections::{HashMap, HashSet};

/// The text that starts a tag in a test file.
pub const TAG: &str = "covers:";

/// One `covers:` tag: the test file, the line and the requirement id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    pub path: String,
    pub line: u32,
    pub target: String,
    /// True for a tag `covers: partial <id>`. It is not evidence.
    pub partial: bool,
    /// The name after `case:` in the tag, if it has one.
    pub case: Option<String>,
    /// The result of the bind of the tag to a test case of the reports.
    /// `None`: the reports are not read, so the tag counts as in 0.7.1.
    pub proof: Option<Proof>,
}

/// The result of the bind of a tag to the test cases of the reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proof {
    /// A bound case passed, and no bound case failed. The data is the bound case
    /// that passed least and its number of passes (the reports in which it
    /// passed). Each bound case counts, and a case counts once for one report.
    Passed(String, usize),
    /// Each bound case was skipped. The name of a case.
    Skipped(String),
    /// A bound case failed, or passed after a rerun. The name of the case.
    Failed(String),
    /// No case of the reports binds to the tag.
    Unbound,
}

/// The proof state of a requirement, from the best to the worst (design 0.8.0,
/// D2). The order of the variants is the order of the ranking, so `Reliable` is
/// the least value. The record of an acceptance lists the state of each
/// requirement, so the type is public.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProofState {
    /// Passed once, and each bound case that passed has at least
    /// `[evidence] reliable-runs` passes in the reports of the tree.
    Reliable,
    /// A bound case passed in a fresh report, and no bound case failed.
    PassedOnce,
    /// A `verified-by` record: a check by a person.
    Verified,
    /// A full tag that no fresh report proves: no `reports` key, no fresh
    /// report, or a case that failed, was skipped or was not found.
    Tagged,
    /// A proof debt (`proof-owed` with a reason of the list), and no evidence.
    Owed,
    /// No evidence and no debt.
    None,
}

impl ProofState {
    /// The 6 states, best first.
    pub const ALL: [ProofState; 6] = [
        ProofState::Reliable,
        ProofState::PassedOnce,
        ProofState::Verified,
        ProofState::Tagged,
        ProofState::Owed,
        ProofState::None,
    ];

    /// The name of the state, as the docs write it.
    pub fn name(self) -> &'static str {
        match self {
            ProofState::Reliable => "reliable",
            ProofState::PassedOnce => "passed-once",
            ProofState::Verified => "verified",
            ProofState::Tagged => "tagged",
            ProofState::Owed => "owed",
            ProofState::None => "none",
        }
    }

    /// The name of the state as a key of JSON output: a hyphen is `_`.
    pub fn key(self) -> &'static str {
        match self {
            ProofState::PassedOnce => "passed_once",
            other => other.name(),
        }
    }
}

/// The reasons of a proof debt. The list is closed: a person must look
/// (`manual`), CI has no such device, account or data (`environment`), the cause
/// is a third party (`external`), or the test is not written (`pending`).
pub const OWED_REASONS: [&str; 4] = ["manual", "environment", "external", "pending"];

/// What SD620 needs for one change (design 0.8.0, D2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reliable {
    /// `[evidence] reliable-runs`: the passes that each bound case needs.
    pub runs: usize,
    /// True if SD617 is on. Then SD617 reports a requirement that no bound case
    /// proves, and SD620 leaves it. With SD617 off, such a requirement has 0
    /// passes, which is fewer than `runs`, so SD620 reports it.
    pub sd617_on: bool,
}

/// The rule of SD620 for a change of class `class`, or `None` when
/// `[evidence] reliable-for` does not cover the change.
pub fn reliable_for(config: &Config, class: crate::model::Class) -> Option<Reliable> {
    use crate::config::ReliableFor;
    let runs = config.evidence_reliable_runs as usize;
    let covered = match config.evidence_reliable_for {
        ReliableFor::All => true,
        ReliableFor::HigherRisk => class == crate::model::Class::HigherRisk,
        ReliableFor::None => false,
    };
    covered.then(|| Reliable {
        runs,
        sd617_on: !config.is_off(Code::ChangeUnproven),
    })
}

/// True if a code of the proof levels and the proof debt (SD618 to SD620) is
/// on. In `lite`, each is off, so `status` and `stats` show no proof line and
/// read no report (design 0.8.0, profiles).
pub fn proof_codes_on(config: &Config) -> bool {
    Code::PROOF_CODES.iter().any(|c| !config.is_off(*c))
}

/// True if `reason` is one of `OWED_REASONS`, with this spelling.
pub fn is_owed_reason(reason: &str) -> bool {
    OWED_REASONS.contains(&reason)
}

/// The proof state from the facts of one requirement. `verified`: it has a
/// `verified-by` record. `tagged`: it has a full tag. `owed`: it has a debt with
/// a valid reason. `proofs`: the result of the bind of each full tag (empty when
/// the reports are not read). `runs`: `[evidence] reliable-runs`.
///
/// A tag that a bound case proves ranks above `verified-by`. A case that failed
/// in a report makes each tag of the requirement unproven, as for SD616.
pub fn state_of(
    verified: bool,
    tagged: bool,
    owed: bool,
    proofs: &[Proof],
    runs: usize,
) -> ProofState {
    let failed = proofs.iter().any(|p| matches!(p, Proof::Failed(_)));
    let passes: Vec<usize> = proofs
        .iter()
        .filter_map(|p| match p {
            Proof::Passed(_, n) => Some(*n),
            _ => None,
        })
        .collect();
    if !failed && !passes.is_empty() {
        return if passes.iter().all(|n| *n >= runs) {
            ProofState::Reliable
        } else {
            ProofState::PassedOnce
        };
    }
    if verified {
        ProofState::Verified
    } else if tagged {
        ProofState::Tagged
    } else if owed {
        ProofState::Owed
    } else {
        ProofState::None
    }
}

/// The characters that a tag word can have around its text.
fn trim_word(word: &str) -> &str {
    word.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '[' | ']' | '(' | ')' | ';' | '.'))
}

/// One tag of a line: a requirement id, the flag of a partial tag, and the
/// name after `case:`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub partial: bool,
    pub case: Option<String>,
}

/// Returns the requirement ids of the `covers:` tags of one line, each with
/// the flag of a partial tag. See `tags_with_case`.
pub fn tags(line: &str) -> Vec<(String, bool)> {
    tags_with_case(line)
        .into_iter()
        .map(|t| (t.id, t.partial))
        .collect()
}

/// Returns the `covers:` tags of one line. A tag starts at `covers:` after a
/// character that is not part of a word. The ids follow it, separated by spaces
/// or commas, each as `<doc-id>#<key>`. The first word that is not such an id
/// ends the tag, for example `*/` or `-->`. The word `partial`, in any ASCII
/// case, is the exception when it is the first word: it makes each id of the
/// tag partial, with or without one colon after it. After an id, the word
/// `partial` ends the tag as any other word.
///
/// The word `case:<name>` or `case:"<name with spaces>"` after the ids names
/// the test case of the tag (design 0.8.0, D1). It ends the tag, as any other
/// word does, so a binary of 0.7.1 reads the same ids. It applies to each id of
/// the tag.
pub fn tags_with_case(line: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find(TAG) {
        let before = rest[..pos].chars().next_back();
        let after = &rest[pos + TAG.len()..];
        rest = after;
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        let mut partial = false;
        let mut first = true;
        let mut found: Vec<(String, bool)> = Vec::new();
        let mut case = None;
        let sep = |c: char| c.is_whitespace() || c == ',';
        let mut i = 0;
        while i < after.len() {
            let tail = &after[i..];
            let skip = tail.len() - tail.trim_start_matches(sep).len();
            i += skip;
            let tail = &after[i..];
            if tail.is_empty() {
                break;
            }
            // The name of a case ends the tag. The test comes before the test
            // of an id, because a name can have a `#`.
            if let Some(value) = tail.strip_prefix("case:") {
                if !found.is_empty() {
                    case = case_name(value);
                }
                break;
            }
            let end = tail.find(sep).unwrap_or(tail.len());
            let word = &tail[..end];
            i += end;
            let id = trim_word(word);
            // One colon after the word is part of the form: `partial: <id>`.
            if first
                && id
                    .strip_suffix(':')
                    .unwrap_or(id)
                    .eq_ignore_ascii_case("partial")
            {
                partial = true;
                first = false;
                continue;
            }
            first = false;
            match id.split_once('#') {
                Some((d, k)) if !d.is_empty() && !k.is_empty() && !k.contains('#') => {
                    found.push((id.to_string(), partial))
                }
                _ => break,
            }
        }
        for (id, partial) in found {
            out.push(Tag {
                id,
                partial,
                case: case.clone(),
            });
        }
    }
    out
}

/// The name after `case:`: a word, or a text in double quotes. `None` for an
/// empty name or a quote that is not closed.
fn case_name(value: &str) -> Option<String> {
    let name = match value.strip_prefix('"') {
        Some(quoted) => quoted.split_once('"')?.0.to_string(),
        None => value
            .split(char::is_whitespace)
            .next()
            .unwrap_or("")
            .trim_end_matches([',', ';'])
            .to_string(),
    };
    (!name.is_empty()).then_some(name)
}

/// Finds the `covers:` tags in the test files of a tree. Without `rev`, it
/// reads the working tree (with `untracked`, also the untracked files that git
/// does not ignore). With `rev`, it reads that commit.
pub fn load(
    repo: &Repo,
    config: &Config,
    rev: Option<&str>,
    untracked: bool,
) -> Result<Vec<Cover>, Fatal> {
    if config.evidence_tests.is_empty() {
        return Ok(Vec::new());
    }
    let lines = repo
        .git
        .grep_lines(TAG, rev, untracked, &config.evidence_tests)
        .map_err(Fatal::Io)?;
    // A file below `[evals] dir` is a fixture or an eval, never a test of the
    // project: the fixtures of the pack hold test files with `covers:` tags.
    let evals_prefix = config.evals.as_ref().map(|e| format!("{}/", e.dir));
    let mut out = Vec::new();
    for (path, line, text) in lines {
        if !config.is_test_file(&path) || evals_prefix.as_ref().is_some_and(|p| path.starts_with(p))
        {
            continue;
        }
        for t in tags_with_case(&text) {
            out.push(Cover {
                path: path.clone(),
                line,
                target: t.id,
                partial: t.partial,
                case: t.case,
                proof: None,
            });
        }
    }
    Ok(out)
}

/// The most lines that the block of a tag has, and the most lines that the
/// search for its start reads.
const BLOCK_LINES: usize = 20;

/// The width of the indent of a line: a space is 1, a tab is 4.
fn indent(line: &str) -> usize {
    line.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// True for a line that is a comment in a common language: `//`, `/*`, `*`,
/// `<!--`, `--`, or `#` that does not start an attribute (`#[` or `#![`).
fn is_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("<!--")
        || trimmed.starts_with("--")
        || (trimmed.starts_with('#') && !trimmed.starts_with("#[") && !trimmed.starts_with("#!["))
}

/// The index of the line that opens the `/* ... */` comment of the tag at
/// `lines[tag]`, if the tag line is in such a comment: the tag line, when it
/// starts with `/*`, or the nearest line above it that starts with `/*`, when
/// the lines between start with `*`.
fn comment_open(lines: &[&str], tag: usize) -> Option<usize> {
    let t = lines[tag].trim();
    if t.starts_with("/*") {
        return Some(tag);
    }
    if !t.starts_with('*') {
        return None;
    }
    for k in (tag.saturating_sub(BLOCK_LINES)..tag).rev() {
        let above = lines[k].trim();
        if above.starts_with("/*") {
            return Some(k);
        }
        if !above.starts_with('*') {
            return None;
        }
    }
    None
}

/// The lines of the block of the tag at `lines[tag]`: the first code block
/// after the tag. The block starts at the first line after the tag that is not
/// blank and not a comment. It ends before the first later line whose indent
/// is not deeper than the indent of the start line, after a deeper line. It
/// also ends before a line with less indent than the tag line, before the next
/// `covers:` line, and after 20 lines. The tag line of a `/* ... */` comment
/// that spans lines has no indent of its own: the line that opens the comment
/// gives it.
fn block<'a>(lines: &[&'a str], tag: usize) -> Vec<&'a str> {
    // A tag in the lines of a doc comment (`/**`, ` * covers: ...`, ` */`) has
    // the indent of the comment lines, and the test below it has the indent of
    // the line that opens the comment. So the opening line gives the scope of
    // the tag. A tag in a comment of one line, or in a comment in the body of
    // a test, keeps the indent of its own line.
    let tag_indent =
        comment_open(lines, tag).map_or_else(|| indent(lines[tag]), |k| indent(lines[k]));
    let mut start = None;
    for (k, line) in lines.iter().enumerate().skip(tag + 1).take(BLOCK_LINES) {
        let t = line.trim();
        if !t.is_empty() && !is_comment(t) {
            start = Some(k);
            break;
        }
    }
    let Some(start) = start else {
        return Vec::new();
    };
    let start_indent = indent(lines[start]);
    if start_indent < tag_indent {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut deeper = false;
    for (n, line) in lines.iter().enumerate().skip(start).take(BLOCK_LINES) {
        if line.trim().is_empty() {
            continue;
        }
        let ind = indent(line);
        if n > start && (ind < tag_indent || line.contains(TAG)) {
            break;
        }
        if n > start && deeper && ind <= start_indent {
            break;
        }
        if ind > start_indent {
            deeper = true;
        }
        out.push(*line);
    }
    out
}

/// The words and the string literals of a block, in order. A word is a run of
/// letters, digits, `_` and `$`. A string literal is a text in `"`, `'` or a
/// backtick, and it is one item, with no words of its own.
fn candidates(block: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for line in block {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if matches!(c, '"' | '\'' | '`') {
                let close = chars[i + 1..].iter().position(|x| *x == c);
                match close {
                    Some(n) => {
                        let lit: String = chars[i + 1..i + 1 + n].iter().collect();
                        if !lit.trim().is_empty() {
                            out.push(lit);
                        }
                        i += n + 2;
                    }
                    None => i += 1,
                }
            } else if c.is_alphanumeric() || c == '_' || c == '$' {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
                {
                    i += 1;
                }
                out.push(chars[start..i].iter().collect());
            } else {
                i += 1;
            }
        }
    }
    out
}

/// The name of a case without a final `[...]` or `(...)`.
fn strip_params(name: &str) -> &str {
    let t = name.trim_end();
    for (open, close) in [('[', ']'), ('(', ')')] {
        if t.ends_with(close)
            && let Some(i) = t.rfind(open)
            && i > 0
        {
            return t[..i].trim_end();
        }
    }
    t
}

/// True if the name of a case matches a word or a string of the block. The
/// name, without a final `[...]` or `(...)`, equals the word, ends with the
/// word after one of `.` `::` `/` `#` space `>` `›`, or starts with the word and
/// `/`. The bind uses [`name_keys`], and the tests compare the two.
#[cfg(test)]
pub fn name_matches(name: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let base = strip_params(name);
    if base == word {
        return true;
    }
    if let Some(prefix) = base.strip_suffix(word)
        && [".", "::", "/", "#", " ", ">", "\u{203a}"]
            .iter()
            .any(|sep| prefix.ends_with(sep))
    {
        return true;
    }
    base.strip_prefix(word)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// The dotted path of a file: the path without its extension, with `.` for `/`.
fn dotted(path: &str) -> String {
    let stem = match path.rsplit_once('.') {
        Some((s, ext)) if !ext.contains('/') && !s.is_empty() && !s.ends_with('/') => s,
        _ => path,
    };
    stem.replace('/', ".")
}

/// True if `dotted_path` ends with `key` in whole segments.
fn ends_in_segments(dotted_path: &str, key: &str) -> bool {
    dotted_path == key
        || dotted_path
            .strip_suffix(key)
            .is_some_and(|p| p.ends_with('.'))
}

/// The classnames that name a test file: the `classname`, and its part before
/// the last `.` (the form of pytest: module, then class).
fn classname_keys(classname: &str) -> Vec<&str> {
    let mut keys = vec![classname];
    if let Some((module, _)) = classname.rsplit_once('.') {
        keys.push(module);
    }
    keys
}

/// True if a file path of a case names the test file of a tag. `case_file` has
/// forward slashes and no leading `./`. A runner that starts in a subfolder
/// writes a shorter path, so the tag path can also end with `/` and the path of
/// the case.
fn same_file(case_file: &str, path: &str) -> bool {
    let ends = |long: &str, short: &str| {
        long.strip_suffix(short)
            .is_some_and(|rest| rest.ends_with('/'))
    };
    case_file == path || ends(case_file, path) || ends(path, case_file)
}

/// Words that start a test in many languages, and that are no name of a test.
/// The bind tries them after each string literal and each other word.
const KEYWORDS: &[&str] = &[
    "it",
    "test",
    "tests",
    "fn",
    "def",
    "func",
    "fun",
    "function",
    "async",
    "await",
    "pub",
    "public",
    "private",
    "protected",
    "static",
    "final",
    "void",
    "class",
    "describe",
    "suite",
    "fact",
    "theory",
];

/// True if a candidate is a keyword that starts a test.
fn is_keyword(word: &str) -> bool {
    KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(word))
}

/// The words that a name can match: the name without a final `[...]` or
/// `(...)`, each part of it after one of the separators of [`name_matches`], and
/// each part of it before a `/`. `name_matches(name, word)` is true exactly when
/// `word` is one of these.
fn name_keys(name: &str) -> Vec<&str> {
    const SEPS: [&str; 7] = [".", "::", "/", "#", " ", ">", "\u{203a}"];
    let base = strip_params(name);
    let mut keys = vec![base];
    for sep in SEPS {
        for (i, _) in base.match_indices(sep) {
            keys.push(&base[i + sep.len()..]);
        }
    }
    for (i, _) in base.match_indices('/') {
        keys.push(&base[..i]);
    }
    keys.retain(|k| !k.is_empty());
    keys
}

/// The index of the cases of the reports, for the bind of many tags. It holds,
/// once for all tags, what a bind needs about a case: the cases by the words
/// that their names match, and the test files that each `classname` names. The
/// cost of a bind is then the cost of its words, and not the number of cases
/// times the number of test files.
pub struct Binder<'a> {
    cases: &'a [Case],
    /// The word of a name, to the cases whose name it matches.
    by_word: HashMap<&'a str, Vec<usize>>,
    /// For each case with a `classname` and no file: the keys of the
    /// `classname`, and true if a test file names it.
    class: Vec<Option<(Vec<&'a str>, bool)>>,
    /// For each case with a file: the path with forward slashes, with no `./`.
    file: Vec<Option<String>>,
}

impl<'a> Binder<'a> {
    pub fn new(cases: &'a [Case], test_files: &[String]) -> Binder<'a> {
        // The dotted path of each test file, and each end of it in whole
        // segments.
        let mut ends: HashSet<String> = HashSet::new();
        for f in test_files {
            let d = dotted(f);
            let mut rest = d.as_str();
            loop {
                ends.insert(rest.to_string());
                match rest.split_once('.') {
                    Some((_, tail)) => rest = tail,
                    None => break,
                }
            }
        }
        let mut by_word: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut class = Vec::with_capacity(cases.len());
        let mut file = Vec::with_capacity(cases.len());
        for (i, c) in cases.iter().enumerate() {
            let mut keys = name_keys(&c.name);
            keys.sort_unstable();
            keys.dedup();
            for k in keys {
                by_word.entry(k).or_default().push(i);
            }
            file.push(c.file.as_ref().map(|f| {
                let f = f.replace('\\', "/");
                f.trim_start_matches("./").to_string()
            }));
            class.push(match (&c.file, &c.classname) {
                (None, Some(cn)) => {
                    let keys = classname_keys(cn);
                    let named = keys.iter().any(|k| ends.contains(*k));
                    Some((keys, named))
                }
                _ => None,
            });
        }
        Binder {
            cases,
            by_word,
            class,
            file,
        }
    }

    /// True if case `i` can bind a tag of the test file `path`. `dotted_path`
    /// is the dotted path of `path`. A case that has a file binds only a tag of
    /// that file. A case with no file and a `classname` binds only a tag of a
    /// test file whose dotted path ends with the `classname` or with its part
    /// before the last `.`. When no test file has such a path, the case binds a
    /// tag of any test file.
    fn fits(&self, i: usize, path: &str, dotted_path: &str) -> bool {
        if let Some(f) = &self.file[i] {
            return same_file(f, path);
        }
        match &self.class[i] {
            None => true,
            Some((keys, named)) => !named || keys.iter().any(|k| ends_in_segments(dotted_path, k)),
        }
    }

    /// The cases that a word names and that fit the test file.
    fn named(&self, word: &str, path: &str, dotted_path: &str) -> Vec<&'a Case> {
        self.by_word.get(word).map_or_else(Vec::new, |v| {
            v.iter()
                .filter(|i| self.fits(**i, path, dotted_path))
                .map(|i| &self.cases[*i])
                .collect()
        })
    }

    /// Binds one tag to the cases of the reports. `lines` are the lines of the
    /// test file, and `tag` is the index of the line of the tag. With `case`,
    /// the tag binds to the cases that this name gives. Else it binds to the
    /// first string literal or word of the block of the tag that names a case.
    /// A keyword like `it` or `def` is tried after all other candidates.
    pub fn bind(&self, path: &str, lines: &[&str], tag: usize, case: Option<&str>) -> Proof {
        let d = dotted(path);
        let bound: Vec<&Case> = match case {
            Some(name) => self.named(name, path, &d),
            None if tag < lines.len() => {
                let words = candidates(&block(lines, tag));
                let (keywords, others): (Vec<String>, Vec<String>) =
                    words.into_iter().partition(|w| is_keyword(w));
                let mut found = Vec::new();
                for word in others.iter().chain(keywords.iter()) {
                    let hit = self.named(word, path, &d);
                    if !hit.is_empty() {
                        found = hit;
                        break;
                    }
                }
                found
            }
            None => Vec::new(),
        };
        let named = |o: Outcome| {
            bound
                .iter()
                .find(|c| c.outcome == o)
                .map(|c| c.name.clone())
        };
        if let Some(n) = named(Outcome::Fail) {
            Proof::Failed(n)
        } else if let Some(n) = named(Outcome::Pass) {
            let (name, runs) = least_passes(&bound).unwrap_or((n, 0));
            Proof::Passed(name, runs)
        } else if let Some(n) = named(Outcome::Skip) {
            Proof::Skipped(n)
        } else {
            Proof::Unbound
        }
    }
}

/// The bound case that passed least, with its passes. A case is a name with its
/// `classname` and its file. Its passes are the reports in which it passed
/// (D2). A case that never passed does not count: a skipped case of one report
/// is not a fail. `None` when no case passed.
fn least_passes(bound: &[&Case]) -> Option<(String, usize)> {
    let mut passes: HashMap<String, (&str, HashSet<&str>)> = HashMap::new();
    for c in bound.iter().filter(|c| c.outcome == Outcome::Pass) {
        // The `\0` cannot be in a name, so 2 cases never share a key.
        let key = format!(
            "{}\0{}\0{}",
            c.name,
            c.classname.as_deref().unwrap_or(""),
            c.file.as_deref().unwrap_or("")
        );
        passes
            .entry(key)
            .or_insert_with(|| (c.name.as_str(), HashSet::new()))
            .1
            .insert(c.report.as_str());
    }
    // The least count first, and the name decides a tie, so the result does
    // not depend on the order of a hash map.
    passes
        .into_values()
        .map(|(name, reports)| (reports.len(), name))
        .min()
        .map(|(n, name)| (name.to_string(), n))
}

/// Sets the result of the bind on each full tag of `covers`. It reads the test
/// files from the working tree. When no report is fresh, each full tag gets
/// `Unbound`. The report problems (SD613 to SD615) say why. A setting that
/// turns them off or makes them a warning must not turn the requirement into a
/// proven one.
pub fn bind_covers(root: &std::path::Path, covers: &mut [Cover], reads: &Reads) {
    if reads.fresh == 0 {
        for cv in covers.iter_mut().filter(|c| !c.partial) {
            cv.proof = Some(Proof::Unbound);
        }
        return;
    }
    let binder = Binder::new(&reads.cases, &reads.test_files);
    let mut texts: HashMap<String, Option<String>> = HashMap::new();
    for cv in covers.iter_mut().filter(|c| !c.partial) {
        let text = texts.entry(cv.path.clone()).or_insert_with(|| {
            std::fs::read(root.join(&cv.path))
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        });
        let lines: Vec<&str> = text
            .as_deref()
            .map(|t| t.lines().collect())
            .unwrap_or_default();
        cv.proof = Some(binder.bind(
            &cv.path,
            &lines,
            (cv.line as usize).saturating_sub(1),
            cv.case.as_deref(),
        ));
    }
}

/// The evidence of the requirements of a tree.
#[derive(Default)]
pub struct Evidence {
    /// For each (document, index into `parsed.reqs`), the `covers:` tags as
    /// `<path>:<line>`.
    pub covers: HashMap<(u32, usize), Vec<String>>,
    /// The partial tags, as `<path>:<line>`, in the same form. They are not
    /// evidence.
    pub partial: HashMap<(u32, usize), Vec<String>>,
    /// For each requirement, the full tags with a result of the bind, as
    /// (`<path>:<line>`, result). Empty when the reports are not read.
    pub proofs: HashMap<(u32, usize), Vec<(String, Proof)>>,
}

impl Evidence {
    /// Resolves the `covers:` tags of a tree. Returns the evidence and a
    /// diagnostic (SD105) for each tag that names no requirement. A tag that
    /// names the requirement of an `ADD` line of an `accepted` change is not
    /// reported: `finish` writes the requirement later.
    pub fn build(c: &Corpus) -> (Evidence, Vec<Diag>) {
        let mut ev = Evidence::default();
        let mut diags = Vec::new();
        // Read the changes only for the first tag that names no requirement.
        let mut pending: Option<HashSet<String>> = None;
        for cv in &c.covers {
            let problem = match c.resolve(&cv.target) {
                Res::Found(d, Some(key), true) => match c.docs[d as usize].req(&key) {
                    Some(r) => {
                        let map = if cv.partial {
                            &mut ev.partial
                        } else {
                            &mut ev.covers
                        };
                        let at = format!("{}:{}", cv.path, cv.line);
                        if let (false, Some(p)) = (cv.partial, &cv.proof) {
                            ev.proofs
                                .entry((d, r))
                                .or_default()
                                .push((at.clone(), p.clone()));
                        }
                        map.entry((d, r)).or_default().push(at);
                        continue;
                    }
                    None => format!(
                        "`{}` names a section of `{}`, not a requirement",
                        cv.target, c.docs[d as usize].id
                    ),
                },
                Res::Found(d, _, _) => format!(
                    "`{}` has no requirement `{}`",
                    c.docs[d as usize].id,
                    cv.target.split_once('#').map_or("", |x| x.1)
                ),
                Res::Ambiguous(v) => format!("`{}` names {} documents", cv.target, v.len()),
                Res::Missing | Res::CrossRepo(_) => {
                    format!("`{}` names no document", cv.target)
                }
            };
            let pending = pending.get_or_insert_with(|| {
                crate::change::pending_adds(c)
                    .iter()
                    .map(|t| same_target(c, t))
                    .collect()
            });
            if pending.contains(&same_target(c, &cv.target)) {
                continue;
            }
            diags.push(Diag {
                path: cv.path.clone(),
                line: cv.line,
                code: Code::CoversUnresolved,
                doc: cv.path.clone(),
                target: cv.target.clone(),
                message: format!("`covers:` tag: {problem}"),
                severity: Code::CoversUnresolved.severity(),
            });
        }
        (ev, diags)
    }

    /// Returns the evidence of one requirement (an index into `parsed.reqs` of
    /// document `di`). The evidence is the paths of `verified-by` (front
    /// matter and the inline marker), and the full `covers:` tags as
    /// `<path>:<line>`. A partial tag is not evidence.
    pub fn of(&self, di: u32, d: &Doc, r: usize) -> Vec<String> {
        let mut out: Vec<String> = d.verified_by.iter().map(|v| v.0.clone()).collect();
        out.extend(d.parsed.reqs[r].verified_by.iter().cloned());
        if let Some(v) = self.covers.get(&(di, r)) {
            out.extend(v.iter().cloned());
        }
        out
    }

    /// The reason why the test reports do not prove a requirement, or `None`.
    /// It is `None` in these cases:
    ///
    /// - The reports are not read.
    /// - The requirement has no full tag with a result.
    /// - The requirement has a `verified-by` (a check by a person, which the
    ///   report does not apply to).
    /// - A bound case passed and no bound case failed.
    ///
    /// The text names the case, so a reviewer can see the bind.
    pub fn unproven(&self, di: u32, d: &Doc, r: usize) -> Option<String> {
        let proofs = self.proofs.get(&(di, r))?;
        if !d.verified_by.is_empty() || !d.parsed.reqs[r].verified_by.is_empty() {
            return None;
        }
        if let Some(Proof::Failed(n)) = proofs
            .iter()
            .map(|p| &p.1)
            .find(|p| matches!(p, Proof::Failed(_)))
        {
            return Some(format!("the bound case `{n}` failed in the report"));
        }
        if proofs.iter().any(|p| matches!(p.1, Proof::Passed(..))) {
            return None;
        }
        if let Some(Proof::Skipped(n)) = proofs
            .iter()
            .map(|p| &p.1)
            .find(|p| matches!(p, Proof::Skipped(_)))
        {
            return Some(format!(
                "the bound case `{n}` was skipped in the report, and a skipped case is no pass"
            ));
        }
        let at = &proofs[0].0;
        Some(format!(
            "no case of a fresh report binds to the tag at {at}: a missing, stale or unreadable report has no case (SD613 to SD615), or the tag is not above its test; put the tag above the test, or add `case:<name>` to the tag"
        ))
    }

    /// The reason of the debt of requirement `r`, if the marker `proof-owed` has
    /// a reason of `OWED_REASONS`.
    pub fn owed_reason(d: &Doc, r: usize) -> Option<&str> {
        d.parsed.reqs[r]
            .proof_owed
            .as_deref()
            .filter(|x| is_owed_reason(x))
    }

    /// The proof state of requirement `r` of document `di`. `runs` is
    /// `[evidence] reliable-runs`. With no report read, a full tag is `Tagged`.
    pub fn state(&self, di: u32, d: &Doc, r: usize, runs: usize) -> ProofState {
        let verified = !d.verified_by.is_empty() || !d.parsed.reqs[r].verified_by.is_empty();
        let proofs: Vec<Proof> = self
            .proofs
            .get(&(di, r))
            .map(|v| v.iter().map(|p| p.1.clone()).collect())
            .unwrap_or_default();
        state_of(
            verified,
            self.covers.contains_key(&(di, r)),
            Evidence::owed_reason(d, r).is_some(),
            &proofs,
            runs,
        )
    }

    /// The reason why requirement `r` is below `reliable` (SD620), or `None`.
    /// It is `Some` in 2 cases. In the first case the state is `PassedOnce`: the
    /// text names the bound case with the fewest passes. In the second case the
    /// requirement has a `verified-by` record and a full tag with a bound case
    /// that failed or was skipped: the record does not replace the passes of a
    /// tag. A requirement with a `verified-by` record and no tag, or a tag that
    /// binds to no case, is not below `reliable`. A requirement with no pass and
    /// no `verified-by` is SD617 or SD916.
    pub fn not_reliable(&self, di: u32, d: &Doc, r: usize, runs: usize) -> Option<String> {
        let proofs = self.proofs.get(&(di, r))?;
        match self.state(di, d, r, runs) {
            ProofState::PassedOnce => {
                let (name, n) = proofs
                    .iter()
                    .filter_map(|p| match &p.1 {
                        Proof::Passed(name, n) => Some((name, *n)),
                        _ => None,
                    })
                    .min_by_key(|(_, n)| *n)?;
                Some(format!(
                    "the bound case `{name}` passed {} in the reports of this code tree, and `reliable-runs` is {runs}; run the tests {runs} times, write one report for each run, then run `spec-dna evidence stamp`",
                    if n == 1 {
                        "1 time".to_string()
                    } else {
                        format!("{n} times")
                    }
                ))
            }
            ProofState::Verified => {
                let (name, what) = proofs.iter().find_map(|p| match &p.1 {
                    Proof::Failed(n) => Some((n, "failed")),
                    Proof::Skipped(n) => Some((n, "was skipped")),
                    _ => None,
                })?;
                Some(format!(
                    "the bound case `{name}` {what} in the reports of this code tree, so it has fewer passes than `reliable-runs` ({runs}); a `verified-by` record does not replace the passes of a tag"
                ))
            }
            _ => None,
        }
    }

    /// The partial `covers:` tags of one requirement, as `<path>:<line>`, in
    /// the order of the tags.
    pub fn partial_of(&self, di: u32, r: usize) -> &[String] {
        self.partial.get(&(di, r)).map_or(&[], Vec::as_slice)
    }
}

/// The count of the requirements of each proof state, and of the debts of each
/// reason, for `status` and `stats`. It counts the requirements of the active
/// documents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProofCounts {
    /// One count for each state of `ProofState::ALL`.
    pub states: [usize; 6],
    /// One count for each reason of `OWED_REASONS`, for the state `Owed`.
    pub reasons: [usize; 4],
    /// The requirements of any document that have a marker `proof-owed`, with
    /// any reason. A project with none uses no proof debt.
    pub marked: usize,
}

impl ProofCounts {
    /// Counts the requirements of `c`. `ev` is `Evidence::build(c).0`.
    pub fn of(c: &Corpus, ev: &Evidence, runs: usize) -> ProofCounts {
        let mut out = ProofCounts::default();
        for (di, d) in c.docs.iter().enumerate() {
            out.marked += d
                .reqs
                .iter()
                .filter(|&&ri| d.parsed.reqs[ri].proof_owed.is_some())
                .count();
            if !needs_evidence(d) {
                continue;
            }
            for &ri in &d.reqs {
                let state = ev.state(di as u32, d, ri, runs);
                out.states[state as usize] += 1;
                if state == ProofState::Owed
                    && let Some(i) = Evidence::owed_reason(d, ri)
                        .and_then(|x| OWED_REASONS.iter().position(|r| *r == x))
                {
                    out.reasons[i] += 1;
                }
            }
        }
        out
    }

    pub fn count(&self, s: ProofState) -> usize {
        self.states[s as usize]
    }

    pub fn owed(&self) -> usize {
        self.count(ProofState::Owed)
    }

    /// The line of `status`: `proof: 3 owed (manual 1, environment 2)`. `None`
    /// when no requirement is owed.
    pub fn owed_line(&self) -> Option<String> {
        if self.owed() == 0 {
            return None;
        }
        let parts: Vec<String> = OWED_REASONS
            .iter()
            .zip(self.reasons)
            .filter(|(_, n)| *n > 0)
            .map(|(r, n)| format!("{r} {n}"))
            .collect();
        Some(format!(
            "proof: {} owed ({})",
            self.owed(),
            parts.join(", ")
        ))
    }

    /// The object `proof` of `status --json`: `owed` and `by_reason`, with one
    /// count for each reason of the list.
    pub fn owed_json(&self) -> String {
        let mut by = crate::json::Obj::new();
        for (r, n) in OWED_REASONS.iter().zip(self.reasons) {
            by = by.num(r, n);
        }
        crate::json::Obj::new()
            .num("owed", self.owed())
            .raw("by_reason", &by.end())
            .end()
    }

    /// The object `proof` of `stats --json`: one count for each state.
    pub fn states_json(&self) -> String {
        let mut o = crate::json::Obj::new();
        for s in ProofState::ALL {
            o = o.num(s.key(), self.count(s));
        }
        o.end()
    }
}

/// A requirement id in a form that two spellings of one document share: the
/// id of the document that the name resolves to, or the canonical name.
fn same_target(c: &Corpus, target: &str) -> String {
    let Some((id, key)) = target.split_once('#') else {
        return canonical(target);
    };
    match c.resolve(id) {
        Res::Found(d, None, _) => format!("{}#{key}", c.docs[d as usize].id),
        _ => format!("{}#{key}", canonical(id)),
    }
}

/// Returns true if a requirement needs evidence: it is a requirement of an
/// active document.
pub fn needs_evidence(d: &Doc) -> bool {
    d.ty.has_requirements() && d.status_str() == "active"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ids of the tags of a line, each with `~` if the tag is partial.
    fn ids(line: &str) -> Vec<String> {
        tags(line)
            .into_iter()
            .map(|(t, p)| if p { format!("~{t}") } else { t })
            .collect()
    }

    #[test]
    fn tags_reads_partial_with_one_trailing_colon() {
        assert_eq!(ids("# covers: partial: a#b"), ["~a#b"]);
        assert_eq!(ids("# covers: Partial: a#b, c#d"), ["~a#b", "~c#d"]);
        // One colon only, and only on the first word.
        assert!(ids("# covers: partial:: a#b").is_empty());
        assert_eq!(ids("# covers: a#b partial: c#d"), ["a#b"]);
    }

    #[test]
    fn tags_reads_the_word_partial_first_only() {
        assert_eq!(ids("# covers: a#b"), ["a#b"]);
        assert_eq!(ids("# covers: partial a#b"), ["~a#b"]);
        assert_eq!(ids("# covers: PARTIAL a#b"), ["~a#b"]);
        // The word applies to each id of the tag.
        assert_eq!(ids("# covers: partial a#b, c#d"), ["~a#b", "~c#d"]);
        // The trim of punctuation is the one of an id.
        assert_eq!(ids("# covers: (partial) a#b"), ["~a#b"]);
        // After an id, the word ends the tag.
        assert_eq!(ids("# covers: a#b partial c#d"), ["a#b"]);
        assert_eq!(ids("# covers: a#b partial"), ["a#b"]);
        // Each tag of a line has its own flag.
        assert_eq!(ids("# covers: a#b # covers: partial c#d"), ["a#b", "~c#d"]);
        // The word alone names no id.
        assert!(ids("# covers: partial").is_empty());
        assert!(ids("# covers: partial text a#b").is_empty());
    }

    fn case(name: &str, classname: Option<&str>, file: Option<&str>, outcome: Outcome) -> Case {
        Case {
            name: name.to_string(),
            classname: classname.map(str::to_string),
            file: file.map(str::to_string),
            outcome,
            report: "r.xml".to_string(),
        }
    }

    fn proof(
        path: &str,
        text: &str,
        case_name: Option<&str>,
        cases: &[Case],
        files: &[&str],
    ) -> Proof {
        let lines: Vec<&str> = text.lines().collect();
        let tag = lines.iter().position(|l| l.contains("covers:")).unwrap();
        let files: Vec<String> = files.iter().map(|f| f.to_string()).collect();
        Binder::new(cases, &files).bind(path, &lines, tag, case_name)
    }

    #[test]
    fn tags_with_case_reads_the_name_after_the_ids() {
        let t = |l: &str| tags_with_case(l);
        let one = t("# covers: a#b case:test_x");
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].case.as_deref(), Some("test_x"));
        let q = t("// covers: a#b, c#d case:\"closes on timeout\" */");
        assert_eq!(q.len(), 2);
        assert!(
            q.iter()
                .all(|x| x.case.as_deref() == Some("closes on timeout"))
        );
        // A name can have a `#`: it is not an id.
        let h = t("# covers: a#b case:ValveTest#flushesFirst");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].case.as_deref(), Some("ValveTest#flushesFirst"));
        // No name, a quote that is not closed, and `case:` before an id.
        assert_eq!(t("# covers: a#b case:")[0].case, None);
        assert_eq!(t("# covers: a#b case:\"open")[0].case, None);
        assert!(t("# covers: case:x a#b").is_empty());
        // The ids of a tag with a name are the ids of 0.7.1.
        assert_eq!(ids("# covers: a#b case:x"), ["a#b"]);
        // A partial tag can have a name too.
        assert!(t("# covers: partial a#b case:x")[0].partial);
    }

    #[test]
    fn a_tag_above_a_pytest_test_binds_to_its_case() {
        let text =
            "# covers: requirements/valves#close\ndef test_closes_on_timeout():\n    assert True\n";
        for (outcome, want) in [
            (
                Outcome::Skip,
                Proof::Skipped("test_closes_on_timeout".into()),
            ),
            (
                Outcome::Pass,
                Proof::Passed("test_closes_on_timeout".into(), 1),
            ),
            (
                Outcome::Fail,
                Proof::Failed("test_closes_on_timeout".into()),
            ),
        ] {
            let cases = [case(
                "test_closes_on_timeout",
                Some("tests.test_valves"),
                None,
                outcome,
            )];
            assert_eq!(
                proof(
                    "tests/test_valves.py",
                    text,
                    None,
                    &cases,
                    &["tests/test_valves.py"]
                ),
                want
            );
        }
    }

    #[test]
    fn a_rust_attribute_line_is_code_and_a_nextest_case_binds() {
        let text =
            "// covers: requirements/valves#close\n#[test]\nfn closes() {\n    assert!(true);\n}\n";
        let cases = [case("tests::closes", Some("valves"), None, Outcome::Pass)];
        assert_eq!(
            proof("src/valves.rs", text, None, &cases, &[]),
            Proof::Passed("tests::closes".into(), 1)
        );
    }

    #[test]
    fn a_jest_string_literal_binds() {
        let text = "// covers: requirements/valves#close\nit('closes on timeout', () => {\n  expect(1).toBe(1);\n});\n";
        let cases = [case(
            "Valves closes on timeout",
            Some("Valves closes on timeout"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof(
                "tests/valves.test.js",
                text,
                None,
                &cases,
                &["tests/valves.test.js"]
            ),
            Proof::Passed("Valves closes on timeout".into(), 1)
        );
    }

    #[test]
    fn a_go_test_binds_to_its_case_and_its_subtests() {
        let text = "// covers: requirements/valves#close\nfunc TestTimeout(t *testing.T) {\n\tt.Run(\"no_reply\", func(t *testing.T) {})\n}\n";
        let cases = [
            case(
                "TestTimeout",
                Some("example.com/valves"),
                None,
                Outcome::Pass,
            ),
            case(
                "TestTimeout/no_reply",
                Some("example.com/valves"),
                None,
                Outcome::Fail,
            ),
        ];
        assert_eq!(
            proof(
                "valves/sensor_test.go",
                text,
                None,
                &cases,
                &["valves/sensor_test.go"]
            ),
            Proof::Failed("TestTimeout/no_reply".into())
        );
    }

    #[test]
    fn a_java_case_binds_by_the_classname_path() {
        let text = "    // covers: requirements/valves#flush\n    @Test\n    void flushesFirst() {\n        assertTrue(true);\n    }\n}\n";
        let cases = [case(
            "flushesFirst",
            Some("com.example.ValveTest"),
            None,
            Outcome::Pass,
        )];
        let file = "src/test/java/com/example/ValveTest.java";
        assert_eq!(
            proof(file, text, None, &cases, &[file]),
            Proof::Passed("flushesFirst".into(), 1)
        );
    }

    #[test]
    fn a_case_name_binds_by_the_tag_name() {
        // The block has no test, so only `case:` binds.
        let text = "# covers: requirements/valves#close\nVALUE = 1\n";
        let cases = [case(
            "test_closes",
            None,
            Some("tests/test_v.py"),
            Outcome::Pass,
        )];
        assert_eq!(
            proof("tests/test_v.py", text, None, &cases, &[]),
            Proof::Unbound
        );
        assert_eq!(
            proof("tests/test_v.py", text, Some("test_closes"), &cases, &[]),
            Proof::Passed("test_closes".into(), 1)
        );
        let cases = [case("closes on timeout", None, None, Outcome::Skip)];
        assert_eq!(
            proof("t/x.js", text, Some("closes on timeout"), &cases, &[]),
            Proof::Skipped("closes on timeout".into())
        );
    }

    #[test]
    fn the_block_of_a_function_that_is_no_test_does_not_reach_the_next_test() {
        let text = "# covers: requirements/valves#close\ndef check_timeout():\n    assert True\n\ndef test_next():\n    assert True\n";
        let cases = [case(
            "test_next",
            Some("tests.test_valves"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof(
                "tests/test_valves.py",
                text,
                None,
                &cases,
                &["tests/test_valves.py"]
            ),
            Proof::Unbound
        );
    }

    #[test]
    fn a_tag_in_the_body_of_a_test_does_not_bind_to_the_next_test() {
        let text = "@pytest.mark.skip\ndef test_a():\n    # covers: requirements/valves#close\n    assert True\n\ndef test_b():\n    assert True\n";
        let cases = [case(
            "test_b",
            Some("tests.test_valves"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof(
                "tests/test_valves.py",
                text,
                None,
                &cases,
                &["tests/test_valves.py"]
            ),
            Proof::Unbound
        );
    }

    #[test]
    fn the_block_ends_at_the_next_tag_and_after_20_lines() {
        let text = "# covers: a#b\nx = 1\n# covers: c#d\ndef test_next():\n    pass\n";
        let cases = [case("test_next", None, None, Outcome::Pass)];
        assert_eq!(
            proof("t/test_x.py", text, None, &cases, &[]),
            Proof::Unbound
        );
        let mut far = String::from("# covers: a#b\ndef helper():\n");
        for _ in 0..25 {
            far.push_str("    pass\n");
        }
        far.push_str("    test_next()\n");
        assert_eq!(
            proof("t/test_x.py", &far, None, &cases, &[]),
            Proof::Unbound
        );
    }

    #[test]
    fn stacked_tags_bind_to_the_same_test() {
        let text = "# covers: a#b\n# covers: c#d\ndef test_x():\n    pass\n";
        let cases = [case("test_x", None, None, Outcome::Pass)];
        assert_eq!(
            proof("t/test_x.py", text, None, &cases, &[]),
            Proof::Passed("test_x".into(), 1)
        );
    }

    #[test]
    fn a_case_with_a_classname_binds_only_the_file_that_the_classname_names() {
        let text = "# covers: requirements/valves#close\ndef test_init():\n    pass\n";
        let cases = [
            case("test_init", Some("tests.test_a"), None, Outcome::Skip),
            case("test_init", Some("tests.test_b"), None, Outcome::Pass),
        ];
        let files = ["tests/test_a.py", "tests/test_b.py"];
        assert_eq!(
            proof("tests/test_a.py", text, None, &cases, &files),
            Proof::Skipped("test_init".into())
        );
        assert_eq!(
            proof("tests/test_b.py", text, None, &cases, &files),
            Proof::Passed("test_init".into(), 1)
        );
    }

    #[test]
    fn a_classname_that_names_no_test_file_binds_a_tag_of_any_test_file() {
        let text = "# covers: requirements/valves#close\ndef test_init():\n    pass\n";
        let cases = [case(
            "test_init",
            Some("unrelated.name"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]),
            Proof::Passed("test_init".into(), 1)
        );
    }

    #[test]
    fn a_case_with_a_file_binds_only_a_tag_of_that_file() {
        let text = "# covers: requirements/valves#close\ndef test_init():\n    pass\n";
        let cases = [
            case(
                "test_init",
                None,
                Some("/work/tests/test_a.py"),
                Outcome::Skip,
            ),
            case("test_init", None, Some("tests/test_b.py"), Outcome::Pass),
        ];
        assert_eq!(
            proof("tests/test_a.py", text, None, &cases, &[]),
            Proof::Skipped("test_init".into())
        );
    }

    #[test]
    fn a_case_name_matches_a_word_by_the_rule_of_the_design() {
        assert!(name_matches("closes", "closes"));
        assert!(name_matches("tests::closes", "closes"));
        assert!(name_matches("a.b.closes", "closes"));
        assert!(name_matches("Valves closes", "closes"));
        assert!(name_matches("Suite > closes", "closes"));
        assert!(name_matches("Suite \u{203a} closes", "closes"));
        assert!(name_matches("test_closes[param-1]", "test_closes"));
        assert!(name_matches("test_closes(int)", "test_closes"));
        assert!(name_matches("TestTimeout/no_reply", "TestTimeout"));
        assert!(!name_matches("test_closes_late", "test_closes"));
        assert!(!name_matches("enclosed", "closed"));
        assert!(!name_matches("x", ""));
    }

    #[test]
    fn a_tag_in_a_doc_comment_binds_to_the_test_below_it() {
        let java = "    /**\n     * covers: requirements/valves#flush\n     */\n    @Test\n    void flushesFirst() {\n        assertTrue(true);\n    }\n";
        let cases = [case(
            "flushesFirst",
            Some("com.example.ValveTest"),
            None,
            Outcome::Fail,
        )];
        let file = "src/test/java/com/example/ValveTest.java";
        assert_eq!(
            proof(file, java, None, &cases, &[file]),
            Proof::Failed("flushesFirst".into())
        );
        let js = "describe('valves', () => {\n  /**\n   * covers: requirements/valves#flush\n   */\n  it('closes on timeout', () => {\n    expect(1).toBe(1);\n  });\n});\n";
        let cases = [case("valves closes on timeout", None, None, Outcome::Skip)];
        assert_eq!(
            proof("tests/valves.test.js", js, None, &cases, &[]),
            Proof::Skipped("valves closes on timeout".into())
        );
        // A block comment in the body of a test is not a doc comment: the tag
        // does not bind to the next test.
        let body_js = "it('a', () => {\n  /* covers: requirements/valves#flush */\n});\nit('b', () => {\n  expect(1);\n});\n";
        let cases = [case("b", None, None, Outcome::Pass)];
        assert_eq!(
            proof("tests/valves.test.js", body_js, None, &cases, &[]),
            Proof::Unbound
        );
        let body_multi = "it('a', () => {\n  /*\n   * covers: requirements/valves#flush\n   */\n});\nit('b', () => {\n  expect(1);\n});\n";
        assert_eq!(
            proof("tests/valves.test.js", body_multi, None, &cases, &[]),
            Proof::Unbound
        );
        let body_java = "    @Test\n    void a() {\n        /* covers: requirements/valves#flush */\n    }\n\n    @Test\n    void b() {\n        assertTrue(true);\n    }\n";
        let cases = [case(
            "b",
            Some("com.example.ValveTest"),
            None,
            Outcome::Pass,
        )];
        let file = "src/test/java/com/example/ValveTest.java";
        assert_eq!(
            proof(file, body_java, None, &cases, &[file]),
            Proof::Unbound
        );
        // A line comment of a body, with a comment line below it, still ends
        // at the test that has less indent.
        let body = "def test_a():\n    # covers: requirements/valves#flush\n    # more text\ndef test_b():\n    pass\n";
        let cases = [case("test_b", None, None, Outcome::Pass)];
        assert_eq!(
            proof("tests/test_x.py", body, None, &cases, &[]),
            Proof::Unbound
        );
    }

    #[test]
    fn a_string_literal_binds_before_a_keyword_like_word() {
        let text = "// covers: requirements/valves#flush\nit('closes on timeout', () => {\n  expect(1).toBe(1);\n});\n";
        let cases = [
            case("Widget renders it", None, None, Outcome::Pass),
            case("closes on timeout", None, None, Outcome::Fail),
        ];
        assert_eq!(
            proof("tests/valves.test.js", text, None, &cases, &[]),
            Proof::Failed("closes on timeout".into())
        );
        // A keyword still binds when no other candidate does.
        let cases = [case("Widget renders it", None, None, Outcome::Pass)];
        assert_eq!(
            proof("tests/valves.test.js", text, None, &cases, &[]),
            Proof::Passed("Widget renders it".into(), 1)
        );
        // An identifier comes before `def`, `fn` and `func`.
        let go = "// covers: requirements/valves#flush\nfunc TestTimeout(t *testing.T) {\n}\n";
        let cases = [
            case("func", None, None, Outcome::Pass),
            case("TestTimeout", None, None, Outcome::Fail),
        ];
        assert_eq!(
            proof("a/x_test.go", go, None, &cases, &[]),
            Proof::Failed("TestTimeout".into())
        );
    }

    #[test]
    fn a_runner_in_a_subfolder_writes_a_shorter_path() {
        let text = "# covers: requirements/valves#flush\ndef test_init():\n    pass\n";
        let cases = [
            case("test_init", None, Some("tests/test_a.py"), Outcome::Skip),
            case("test_init", None, Some("tests/test_b.py"), Outcome::Pass),
        ];
        assert_eq!(
            proof("service/tests/test_a.py", text, None, &cases, &[]),
            Proof::Skipped("test_init".into())
        );
        // The shorter path ends at a `/`, not inside a name.
        assert_eq!(
            proof("service/xtests/test_a.py", text, None, &cases, &[]),
            Proof::Unbound
        );
    }

    #[test]
    fn the_keys_of_a_name_are_the_words_that_name_matches() {
        for name in [
            "closes",
            "tests::closes",
            "a.b.closes",
            "Valves closes on timeout",
            "Suite \u{203a} closes[1]",
            "TestTimeout/no_reply",
            "A#b(int)",
            "x/",
        ] {
            let keys = name_keys(name);
            for word in [
                "closes",
                "closes on timeout",
                "on timeout",
                "timeout",
                "b",
                "Suite",
                "TestTimeout",
                "no_reply",
                "tests",
                "a",
                "x",
                "Valves",
                "int",
                "closes[1]",
            ] {
                assert_eq!(
                    keys.contains(&word),
                    name_matches(name, word),
                    "{name} / {word}"
                );
            }
        }
    }

    /// A temporary folder with `n` pytest files, each with one tag above a
    /// test, and the cases of a pytest report for them.
    fn scale(n: usize) -> (std::path::PathBuf, Vec<Cover>, Reads) {
        let dir = std::env::temp_dir().join(format!("spec-dna-bind-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        let mut covers = Vec::new();
        let mut cases = Vec::new();
        let mut files = Vec::new();
        for i in 0..n {
            let path = format!("tests/test_f{i}.py");
            let text = format!(
                "# covers: requirements/valves#r{i}\ndef test_a{i}():\n    pass\n\ndef test_b{i}():\n    pass\n"
            );
            std::fs::write(dir.join(&path), text).unwrap();
            covers.push(Cover {
                path: path.clone(),
                line: 1,
                target: format!("requirements/valves#r{i}"),
                partial: false,
                case: None,
                proof: None,
            });
            for name in ["a", "b"] {
                cases.push(case(
                    &format!("test_{name}{i}"),
                    Some(&format!("tests.test_f{i}")),
                    None,
                    Outcome::Pass,
                ));
            }
            files.push(path);
        }
        let reads = Reads {
            cases,
            fresh: 1,
            test_files: files,
            ..Reads::default()
        };
        (dir, covers, reads)
    }

    #[test]
    fn the_bind_of_many_tags_does_not_scan_each_case_against_each_file() {
        // The cost of a naive bind is tags x cases x files: for 300 files it
        // is more than 10 s in a debug build. The index makes it a few ms.
        let (dir, mut covers, reads) = scale(300);
        let start = std::time::Instant::now();
        bind_covers(&dir, &mut covers, &reads);
        let took = start.elapsed();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            covers
                .iter()
                .enumerate()
                .all(|(i, c)| c.proof == Some(Proof::Passed(format!("test_a{i}"), 1))),
            "each tag binds to the first test below it"
        );
        assert!(took.as_secs_f64() < 2.0, "the bind took {took:?}");
    }

    /// Run it with `cargo test --release -- --ignored bind_of_5000`.
    #[test]
    #[ignore]
    fn bind_of_5000_files_and_10000_cases_takes_seconds() {
        let (dir, mut covers, reads) = scale(5000);
        let start = std::time::Instant::now();
        bind_covers(&dir, &mut covers, &reads);
        let took = start.elapsed();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            covers
                .iter()
                .all(|c| matches!(c.proof, Some(Proof::Passed(..))))
        );
        assert!(took.as_secs_f64() < 10.0, "the bind took {took:?}");
    }

    #[test]
    fn with_no_fresh_report_each_full_tag_is_unbound_and_a_partial_tag_is_not() {
        let mk = |partial| Cover {
            path: "tests/none.py".into(),
            line: 1,
            target: "a#b".into(),
            partial,
            case: None,
            proof: None,
        };
        let mut covers = [mk(false), mk(true)];
        bind_covers(
            std::path::Path::new("/nonexistent"),
            &mut covers,
            &Reads::default(),
        );
        assert_eq!(covers[0].proof, Some(Proof::Unbound));
        assert_eq!(covers[1].proof, None);
    }

    fn run_case(name: &str, report: &str, outcome: Outcome) -> Case {
        Case {
            name: name.to_string(),
            classname: Some("tests.test_valves".to_string()),
            file: None,
            outcome,
            report: report.to_string(),
        }
    }

    const PY: &str = "# covers: requirements/valves#close\ndef test_x():\n    assert True\n";

    fn runs(cases: &[Case]) -> Proof {
        proof(
            "tests/test_valves.py",
            PY,
            None,
            cases,
            &["tests/test_valves.py"],
        )
    }

    #[test]
    fn the_passes_of_a_case_are_the_reports_in_which_it_passed() {
        let one = |r: &str| run_case("test_x", r, Outcome::Pass);
        assert_eq!(
            runs(&[one("a.xml"), one("b.xml"), one("c.xml")]),
            Proof::Passed("test_x".into(), 3)
        );
        // The same case twice in one report is one pass.
        assert_eq!(
            runs(&[one("a.xml"), one("a.xml")]),
            Proof::Passed("test_x".into(), 1)
        );
        // A skip in a report is not a pass and not a fail.
        assert_eq!(
            runs(&[
                one("a.xml"),
                run_case("test_x", "b.xml", Outcome::Skip),
                one("c.xml")
            ]),
            Proof::Passed("test_x".into(), 2)
        );
        // Each bound case counts: the case with the fewest passes decides.
        let param = |n: &str, r: &str| run_case(&format!("test_x[{n}]"), r, Outcome::Pass);
        assert_eq!(
            runs(&[
                param("1", "a.xml"),
                param("1", "b.xml"),
                param("1", "c.xml"),
                param("2", "a.xml"),
                param("2", "b.xml"),
            ]),
            Proof::Passed("test_x[2]".into(), 2)
        );
        // A fail in one report of three is a fail.
        assert_eq!(
            runs(&[
                one("a.xml"),
                one("b.xml"),
                run_case("test_x", "c.xml", Outcome::Fail)
            ]),
            Proof::Failed("test_x".into())
        );
    }

    #[test]
    fn the_state_is_the_best_state_that_the_facts_give() {
        use ProofState::*;
        let passed = |n| vec![Proof::Passed("t".into(), n)];
        // (verified, tagged, owed, proofs, runs) -> state
        let rows: Vec<(bool, bool, bool, Vec<Proof>, usize, ProofState)> = vec![
            (false, true, false, passed(3), 3, Reliable),
            (false, true, false, passed(5), 3, Reliable),
            (false, true, false, passed(2), 3, PassedOnce),
            (false, true, false, passed(1), 2, PassedOnce),
            // A passed tag is above `verified-by`.
            (true, true, false, passed(2), 3, PassedOnce),
            (true, true, false, passed(3), 3, Reliable),
            // 2 tags: each passed tag must have the passes.
            (
                false,
                true,
                false,
                vec![Proof::Passed("a".into(), 3), Proof::Passed("b".into(), 2)],
                3,
                PassedOnce,
            ),
            // A fail leaves the tag as `tagged`, or `verified` with a record.
            (
                false,
                true,
                false,
                vec![Proof::Passed("a".into(), 3), Proof::Failed("b".into())],
                3,
                Tagged,
            ),
            (
                true,
                true,
                false,
                vec![Proof::Failed("a".into())],
                3,
                Verified,
            ),
            (
                false,
                true,
                false,
                vec![Proof::Skipped("a".into())],
                3,
                Tagged,
            ),
            (false, true, false, vec![Proof::Unbound], 3, Tagged),
            // No report is read: a tag is a claim.
            (false, true, false, vec![], 3, Tagged),
            (true, false, false, vec![], 3, Verified),
            (true, false, true, vec![], 3, Verified),
            // A debt is a state only with no evidence.
            (false, false, true, vec![], 3, Owed),
            (false, true, true, vec![], 3, Tagged),
            (false, false, false, vec![], 3, None),
        ];
        for (verified, tagged, owed, proofs, n, want) in rows {
            assert_eq!(
                state_of(verified, tagged, owed, &proofs, n),
                want,
                "{verified} {tagged} {owed} {proofs:?} {n}"
            );
        }
        // Best first, with the names of the design.
        let names: Vec<&str> = ProofState::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(
            names,
            [
                "reliable",
                "passed-once",
                "verified",
                "tagged",
                "owed",
                "none"
            ]
        );
        assert!(ProofState::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn a_reason_is_valid_only_when_it_is_in_the_closed_list() {
        assert_eq!(
            OWED_REASONS,
            ["manual", "environment", "external", "pending"]
        );
        for ok in OWED_REASONS {
            assert!(is_owed_reason(ok));
        }
        for bad in ["", "later", "Manual", "manual environment", "manual,"] {
            assert!(!is_owed_reason(bad), "{bad:?}");
        }
    }
}
