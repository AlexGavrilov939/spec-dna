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
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, RandomState};
use std::rc::Rc;

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
    /// The detail of the bind: the test, the reason for no case, and the
    /// strength. `None` when `proof` is `None`.
    pub bind: Option<Bind>,
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

/// Why a tag binds no case (design 0.8.1, D82 item 3). A closed list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The tag stands above no test.
    NoTest,
    /// The tag stands above a group.
    Group,
    /// The test under the tag has a template name.
    Template,
    /// No case of the fresh reports names the test, in the file of the tag.
    NoCase,
    /// The test file of the tag is not in the code tree (design 0.8.1, D85).
    OutsideTree,
    /// No report is fresh.
    NoReport,
    /// No case of the fresh reports names the test, and a case has a name that
    /// is too long to read (`is_long`): such a case binds by its whole name only.
    LongName,
}

impl Reason {
    /// The name in `--json` output and in the docs.
    pub fn key(self) -> &'static str {
        match self {
            Reason::NoTest => "no-test",
            Reason::Group => "group",
            Reason::Template => "template",
            Reason::NoCase => "no-case",
            Reason::OutsideTree => "outside-tree",
            Reason::NoReport => "no-report",
            Reason::LongName => "long-name",
        }
    }
}

/// What the tag names: the test under it, or the case that `case:` names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum By {
    #[default]
    Definition,
    Case,
}

impl By {
    pub fn key(self) -> &'static str {
        match self {
            By::Definition => "definition",
            By::Case => "case",
        }
    }
}

/// The detail of the bind of one tag (design 0.8.1, D82). The messages of SD616
/// and SD617 use it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bind {
    /// The name of the test under the tag, or the value of `case:`. `None` for a
    /// tag above no test or above a group.
    pub test: Option<String>,
    pub by: By,
    /// The name of the case that decides the result.
    pub case: Option<String>,
    /// The number of bound cases.
    pub cases: usize,
    /// How the bound cases fit the test file. `None` with no bound case.
    pub fit: Option<Fit>,
    /// True for a shared set: the tag proves only when each case passed.
    pub shared: bool,
    /// True for a weak bind (SD621). `why` gives the cause.
    pub by_name: bool,
    /// The cause of a weak bind. `None` for `case:` and for a bind that is not
    /// weak.
    pub why: Option<Why>,
    /// The reason for no bound case. `None` with a bound case.
    pub reason: Option<Reason>,
    /// The line (from 1) of the first line below the tag that is code and no
    /// test: the line of the reason `no-test`.
    pub line: Option<usize>,
    /// For `case:` with no bound case: the names of cases that end with the
    /// value. The fix of a `case:` with a part of a name.
    pub near: Vec<String>,
}

/// Why a bind is weak (SD621). When the weak cases of a tag have more than one
/// cause, the first one of this order is the cause of the bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The scanner cannot build the identity of the test: it cannot read the
    /// groups, the title or the layout of the file.
    NoIdentity,
    /// The identity is not unique: another definition builds it, or its file
    /// field names more than one test file.
    NotUnique,
    /// No full field ties the case to the test: the report or the project does
    /// not hold the field.
    ByName,
}

impl Why {
    /// The name of the cause in the message of SD621 and in the docs.
    pub fn key(self) -> &'static str {
        match self {
            Why::NoIdentity => "no-identity",
            Why::NotUnique => "not-unique",
            Why::ByName => "by-name",
        }
    }
}

/// How the bound cases fit the test file of the tag (design 0.8.1, D82 item 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// By the `file` of the case.
    File,
    /// By the `classname` of the case.
    Classname,
    /// By the name only.
    Name,
}

impl Fit {
    pub fn key(self) -> &'static str {
        match self {
            Fit::File => "file",
            Fit::Classname => "classname",
            Fit::Name => "name",
        }
    }
}

impl Bind {
    /// The bind of a tag with no bound case.
    fn unbound(mut self, reason: Reason) -> Bind {
        self.reason = Some(reason);
        self
    }
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

/// The name after `case:`: a word, or a text in double quotes. A name that is not
/// valid gives the empty name `Some("")`: no name or a space after the colon, a
/// quote that is not closed, an empty quoted name, and a `"` inside the name or
/// in the rest of the line. That tag binds no case, and it never falls back to
/// the test under it.
fn case_name(value: &str) -> Option<String> {
    // No name, or a space after the colon: not valid.
    if value.is_empty() || value.starts_with(char::is_whitespace) {
        return Some(String::new());
    }
    let name = match value.strip_prefix('"') {
        Some(quoted) => {
            let Some((name, after)) = quoted.split_once('"') else {
                return Some(String::new());
            };
            // The name ends at the quote that closes it. Another `"` in the rest
            // of the line, or a letter after the quote, shows a broken name.
            let broken = after
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
                || after.contains('"');
            if broken || name.is_empty() {
                return Some(String::new());
            }
            name.to_string()
        }
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
                bind: None,
            });
        }
    }
    Ok(out)
}

/// The most lines that the scan below a tag reads (design 0.8.1, D82 item 1).
const SCAN_LINES: usize = 50;

/// The most lines above a tag that the search for the start of its `/* ... */`
/// comment reads.
const COMMENT_LINES: usize = 20;

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
    // A file can start with a byte order mark, which is no text of the line.
    let trimmed = trimmed.trim_start_matches('\u{feff}');
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
    let t = lines[tag].trim().trim_start_matches('\u{feff}');
    if t.starts_with("/*") {
        return Some(tag);
    }
    if !t.starts_with('*') {
        return None;
    }
    for k in (tag.saturating_sub(COMMENT_LINES)..tag).rev() {
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

/// The test that a tag stands above, or the reason why it stands above none
/// (design 0.8.1, D82 items 1 to 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Under {
    /// A test, with the name that a case must name.
    Test(TestName),
    /// A test with a template name: one literal gives many names. The data is
    /// the literal, or empty when the line has none.
    Template(String),
    /// A group (`describe`, `class`, `mod`): it binds no test of its own.
    Group,
    /// No test. The data is the line (from 1) of the first line that is code
    /// and no test, or `None` when no code line follows the tag.
    NoTest(Option<usize>),
}

/// The name of a test, and the form that decides how a case names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestName {
    pub name: String,
    /// True for the form call (`it('<literal>', ...)`): the case name ends with
    /// the literal. Else the name is one segment of the head of the case name.
    pub call: bool,
    /// True for a Kotlin name in backticks (`` fun `closes on timeout`() ``). It
    /// has the form call, and the case can add the parameters of the method:
    /// `closes on timeout()` and `closes on timeout(int) [1] 3`.
    pub bt: bool,
    /// The type of the receiver of a Go method (`func (s *ValveSuite) TestX()`).
    /// Only the method of a suite is a test, and the case is `<Test function>/
    /// <method>`, where the function runs the suite (`suite.Run`).
    pub recv: Option<String>,
}

/// The first byte index after the leading identifier of `s`: letters, digits,
/// `_` and `$`.
fn ident_end(s: &str) -> usize {
    s.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(s.len())
}

/// The position after the bracket that closes the one at byte `off` of
/// `lines[line]`, as (line, byte). The count ignores the text in quotes. It
/// reads the lines before `end` only.
fn close_of(lines: &[&str], line: usize, off: usize, end: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut start = off;
    for (l, text) in lines.iter().enumerate().take(end).skip(line) {
        let mut chars = text[start..].char_indices();
        while let Some((i, c)) = chars.next() {
            if let Some(q) = quote {
                if c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
                continue;
            }
            match c {
                '"' | '\'' | '`' => quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((l, start + i + c.len_utf8()));
                    }
                }
                _ => {}
            }
        }
        // A quote does not go on in the next line.
        quote = None;
        start = 0;
    }
    None
}

/// The end of the item (a decorator, an annotation or an attribute) that starts
/// at byte `off` of `lines[line]`, as (line, byte). The item is `@<name>` with
/// an optional group `(...)`, `#[...]`, `#![...]` or `[...]`. It goes on in the
/// next lines while one of its brackets stays open.
fn skip_item(lines: &[&str], line: usize, off: usize, end: usize) -> Option<(usize, usize)> {
    let text = lines[line];
    let off = off + (text[off..].len() - text[off..].trim_start().len());
    let s = &text[off..];
    if let Some(r) = s.strip_prefix('@') {
        let n = r
            .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | ':')))
            .unwrap_or(r.len());
        if n == 0 {
            return None;
        }
        let after = off + 1 + n;
        if text[after..].starts_with('(') {
            return close_of(lines, line, after, end);
        }
        return Some((line, after));
    }
    let open = if s.starts_with("#![") {
        off + 2
    } else if s.starts_with("#[") {
        off + 1
    } else if s.starts_with('[') {
        off
    } else {
        return None;
    };
    close_of(lines, line, open, end)
}

/// The first line below `from` that holds text after the items of its start.
struct Def<'a> {
    /// The index of the line that holds the text.
    line: usize,
    /// The index of the first line that is code. Its indent is the indent of the
    /// definition.
    first: usize,
    /// The text of the definition line, after the items.
    text: &'a str,
    /// True when an item came before the text.
    annotated: bool,
}

/// The definition line at or after `lines[from]`: the first line that is not
/// blank, not a comment, and not only items (design 0.8.1, D82 item 1). It reads
/// at most 50 lines. `annotated` says that an item came before `from`.
fn definition<'a>(lines: &[&'a str], from: usize, mut annotated: bool) -> Option<Def<'a>> {
    let end = (from + SCAN_LINES).min(lines.len());
    let mut first = None;
    let mut i = from;
    while i < end {
        let t = lines[i].trim();
        if t.is_empty() || is_comment(t) {
            i += 1;
            continue;
        }
        let first = *first.get_or_insert(i);
        let (mut line, mut off) = (i, 0);
        while let Some((l, o)) = skip_item(lines, line, off, end) {
            annotated = true;
            (line, off) = (l, o);
        }
        let text = lines[line][off..].trim();
        if text.is_empty() {
            i = line + 1;
            continue;
        }
        return Some(Def {
            line,
            first,
            text,
            annotated,
        });
    }
    None
}

/// The words that can stand before the keyword of a definition.
const MODIFIERS: &[&str] = &[
    "pub",
    "public",
    "private",
    "protected",
    "internal",
    "static",
    "final",
    "abstract",
    "sealed",
    "open",
    "partial",
    "export",
    "default",
    "async",
    "const",
    "unsafe",
    "override",
    "virtual",
    "extern",
    "data",
    "inner",
    "suspend",
    "readonly",
];

/// The text without its leading modifiers.
fn strip_modifiers(text: &str) -> &str {
    let mut s = text.trim_start();
    loop {
        let n = ident_end(s);
        if n == 0 || !MODIFIERS.contains(&&s[..n]) {
            return s;
        }
        let rest = &s[n..];
        if &s[..n] == "pub" && rest.starts_with('(') {
            match rest.find(')') {
                Some(c) => s = rest[c + 1..].trim_start(),
                None => return s,
            }
        } else if rest.starts_with(char::is_whitespace) {
            s = rest.trim_start();
        } else {
            return s;
        }
    }
}

/// A literal in `'`, `"` or a backtick at the start of `s`: its text.
fn literal(s: &str) -> Option<&str> {
    let q = s.chars().next().filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let body = &s[1..];
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == q {
            return Some(&body[..i]);
        }
    }
    None
}

/// The form call: `it`, `test`, `specify`, `example` or `scenario`, with the
/// suffixes `.only`, `.skip`, `.todo`, `.concurrent` and `.failing`, then `(` or
/// a space, then a literal. `rest` starts after the word.
fn call_form(rest: &str) -> Option<Under> {
    let mut s = rest;
    let mut each = false;
    while let Some(r) = s.strip_prefix('.') {
        let n = ident_end(r);
        match &r[..n] {
            "only" | "skip" | "todo" | "concurrent" | "failing" => {}
            "each" => each = true,
            _ => return None,
        }
        s = &r[n..];
    }
    if each {
        // The table comes first: `(rows)` or a template literal. The name is in
        // the next group.
        let table_end = s.strip_prefix('(').and_then(|_| {
            let one = [s];
            close_of(&one, 0, 0, 1).map(|(_, o)| o)
        });
        let after = match table_end {
            Some(o) => &s[o..],
            None => s,
        };
        let name = after
            .trim_start()
            .strip_prefix('(')
            .map(str::trim_start)
            .and_then(literal)
            .unwrap_or("");
        return Some(Under::Template(name.to_string()));
    }
    let s = match s.strip_prefix('(') {
        Some(r) => r,
        None if s.starts_with(char::is_whitespace) => s,
        None => return None,
    };
    let name = literal(s.trim_start())?;
    if name.trim().is_empty() {
        return None;
    }
    if name.contains(['%', '$']) || name.contains("#{") {
        return Some(Under::Template(name.to_string()));
    }
    Some(Under::Test(TestName {
        name: name.to_string(),
        call: true,
        bt: false,
        recv: None,
    }))
}

/// The form method: a word and `(`, where an annotation or an attribute came
/// before, or where the line has `void`, `fun`, `function` or `def` before the
/// word. In Kotlin, `fun`, a text in backticks and `(` name the test by its text.
/// The text can have spaces, so the test has the form call: the case name ends
/// with the text, and the report can add `()`.
fn method_form(body: &str, annotated: bool) -> Option<Under> {
    if let Some(r) = body.strip_prefix("fun ")
        && let Some(q) = r.trim_start().strip_prefix('`')
    {
        let (name, after) = q.split_once('`')?;
        return (after.starts_with('(') && !name.is_empty()).then(|| {
            Under::Test(TestName {
                name: name.to_string(),
                call: true,
                bt: true,
                recv: None,
            })
        });
    }
    let head = body[..body.find('(')?].trim_end();
    let name_len = head.len()
        - head
            .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '$')
            .len();
    if name_len == 0 {
        return None;
    }
    let (before, name) = head.split_at(head.len() - name_len);
    let tokens: Vec<&str> = before.split_whitespace().collect();
    let marker = tokens
        .iter()
        .any(|t| matches!(*t, "void" | "fun" | "function" | "def"));
    (marker || (annotated && !tokens.is_empty())).then(|| {
        Under::Test(TestName {
            name: name.to_string(),
            call: false,
            bt: false,
            recv: None,
        })
    })
}

/// The test of a definition line, as the first form of the closed list that
/// matches (design 0.8.1, D82 items 2 and 3). `None`: the line is no test and
/// no group.
fn classify(text: &str, annotated: bool) -> Option<Under> {
    let body = strip_modifiers(text);
    let n = ident_end(body);
    let (word, rest) = body.split_at(n);
    // `RSpec.describe`, and `test.describe` of Playwright, open a group.
    if body.starts_with("RSpec.describe") || body.starts_with("test.describe") {
        return Some(Under::Group);
    }
    if matches!(word, "describe" | "context" | "suite") && rest.starts_with(['(', ' ', '.']) {
        return Some(Under::Group);
    }
    if matches!(
        word,
        "class" | "object" | "mod" | "impl" | "module" | "namespace"
    ) && rest.starts_with(' ')
    {
        return Some(Under::Group);
    }
    // A shared example of RSpec gives the cases `<group> behaves like <name> ...`.
    if matches!(word, "it_behaves_like" | "it_should_behave_like")
        && let Some(lit) = literal(
            rest.trim_start()
                .strip_prefix('(')
                .map_or(rest.trim_start(), str::trim_start),
        )
    {
        return Some(Under::Template(format!("behaves like {lit}%s")));
    }
    let named = |name: &str, recv: Option<String>| {
        (!name.is_empty()).then(|| {
            Under::Test(TestName {
                name: name.to_string(),
                call: false,
                bt: false,
                recv,
            })
        })
    };
    match word {
        "def" if rest.starts_with(' ') => {
            let r = rest.trim_start();
            let m = ident_end(r);
            let next = r[m..].chars().next();
            if matches!(next, None | Some('(' | ' ')) {
                return named(&r[..m], None);
            }
        }
        "fn" if rest.starts_with(' ') => {
            let r = rest.trim_start();
            let m = ident_end(r);
            if r[m..].starts_with(['(', '<']) {
                return named(&r[..m], None);
            }
        }
        "func" if rest.starts_with(' ') => {
            let mut r = rest.trim_start();
            let mut recv = None;
            if r.starts_with('(') {
                let one = [r];
                let (_, o) = close_of(&one, 0, 0, 1)?;
                // The type is the last word of `(s *ValveSuite)`, without `*` and
                // type arguments.
                let ty = r[1..o - 1].split_whitespace().last().unwrap_or("");
                let ty = ty.trim_start_matches('*');
                let ty = ty.split('[').next().unwrap_or("");
                recv = Some(ty.to_string()).filter(|t| !t.is_empty());
                r = r[o..].trim_start();
            }
            let m = ident_end(r);
            if r[m..].starts_with(['(', '[']) {
                return named(&r[..m], recv);
            }
        }
        "it" | "test" | "specify" | "example" | "scenario" => {
            if let Some(u) = call_form(rest) {
                return Some(u);
            }
        }
        _ => {}
    }
    method_form(body, annotated)
}

/// The text of a line before the tag, without the opening of a comment.
fn before_tag(prefix: &str) -> &str {
    let mut s = prefix.trim_end();
    loop {
        let cut = ["<!--", "/*", "//", "--", "#", "*"]
            .iter()
            .find_map(|c| s.strip_suffix(c));
        match cut {
            Some(r) => s = r.trim_end(),
            None => return s,
        }
    }
}

/// What a tag stands above (design 0.8.1, D82 items 1 to 3): the test on its
/// own line, before `covers:`, or else the first definition line below it. A
/// definition line with less indent than the tag ends the scope of the tag, so
/// a tag in the body of a test stands above no test.
pub fn under_tag(lines: &[&str], tag: usize) -> Under {
    under_tag_line(lines, tag).under
}

/// What the scan below a tag found: with the index of the definition line, the
/// first line of its items and its text after the items.
struct Scan<'a> {
    under: Under,
    line: usize,
    first: usize,
    text: &'a str,
}

impl Scan<'_> {
    fn none(tag: usize) -> Scan<'static> {
        Scan {
            under: Under::NoTest(None),
            line: tag,
            first: tag,
            text: "",
        }
    }
}

/// Like `under_tag`. It also gives the definition line.
fn under_tag_line<'a>(lines: &[&'a str], tag: usize) -> Scan<'a> {
    let tag_indent =
        comment_open(lines, tag).map_or_else(|| indent(lines[tag]), |k| indent(lines[k]));
    let mut annotated = false;
    // The text before `covers:` is a definition only when the line does not
    // start with a comment: `/// covers:` and `// This test covers:` are
    // comments, and the scan goes on below them.
    if let Some(pos) = lines[tag]
        .find(TAG)
        .filter(|&pos| !is_comment(lines[tag][..pos].trim()))
    {
        let line: &'a str = lines[tag];
        let before: &'a str = before_tag(&line[..pos]).trim_start_matches('\u{feff}');
        let (mut off, mut any) = (0, false);
        while let Some((_, o)) = skip_item(&[before], 0, off, 1) {
            (off, any) = (o, true);
        }
        annotated = any;
        let text = before[off..].trim();
        if !text.is_empty() {
            let under = classify(text, annotated).unwrap_or(Under::NoTest(Some(tag + 1)));
            return Scan {
                under,
                line: tag,
                first: tag,
                text,
            };
        }
    }
    let Some(d) = definition(lines, tag + 1, annotated) else {
        return Scan::none(tag);
    };
    let below = Under::NoTest(Some(d.line + 1));
    let under = if indent(lines[d.first]) < tag_indent {
        below
    } else {
        classify(d.text, d.annotated).unwrap_or(below)
    };
    Scan {
        under,
        line: d.line,
        first: d.first,
        text: d.text,
    }
}

/// The kind of a group that holds a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroupKind {
    /// `describe`, `context` or `suite`.
    Describe,
    /// `class` or `object`.
    Class,
    /// `mod` of Rust.
    Module,
    /// `module` or `namespace`.
    Namespace,
}

/// One group around a definition.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Title {
    kind: GroupKind,
    text: String,
    /// A bare constant of Ruby (`describe Valve do`).
    constant: bool,
    /// A `partial` class of C#.
    partial: bool,
}

/// True for a literal that a runner writes as the source does: no escape and no
/// interpolation. `quoted` starts at the quote.
fn is_plain(quoted: &str, text: &str) -> bool {
    !text.contains('\\')
        && !(quoted.starts_with('`') && text.contains("${"))
        && !(quoted.starts_with('"') && text.contains("#{"))
}

/// True if the text after a literal does not add to the title: no `+`, and no
/// second title.
fn ends_title(after: &str) -> bool {
    let a = after.trim_start();
    if let Some(rest) = a.strip_prefix(',') {
        return !rest.trim_start().starts_with(['"', '\'']);
    }
    a.is_empty()
        || a.starts_with([')', '{', '#'])
        || a == "do"
        || a.starts_with("do ")
        || a.starts_with("//")
}

/// True if the literal of a call is the whole title of the test, as the runner
/// writes it. `text` is the definition line.
fn plain_call(text: &str, name: &str) -> bool {
    let quoted = text
        .match_indices(name)
        .find(|(i, _)| text[..*i].ends_with(['"', '\'', '`']));
    quoted.is_some_and(|(i, _)| {
        let from = &text[i - 1..];
        literal(from) == Some(name) && is_plain(from, name) && ends_title(&from[name.len() + 2..])
    })
}

/// The group that a line opens: `None` for a line that is no group, and
/// `Some(None)` for a group with a title that the scanner cannot read.
fn group_title(line: &str) -> Option<Option<Title>> {
    // `#[cfg(test)] mod tests {`: the items come first.
    let one = [line];
    let mut off = 0;
    while let Some((_, o)) = skip_item(&one, 0, off, 1) {
        off = o;
    }
    let after_items = line[off..].trim_start();
    let body = strip_modifiers(after_items);
    let partial = after_items[..after_items.len() - body.len()]
        .split_whitespace()
        .any(|w| w == "partial");
    let body = body.strip_prefix("RSpec.").unwrap_or(body);
    let body = match body.strip_prefix("test.") {
        // Playwright: `test.describe('Valves', ...)`.
        Some(r) if r.starts_with("describe") => r,
        _ => body,
    };
    let n = ident_end(body);
    let (word, rest) = body.split_at(n);
    let title = |kind, text: &str, constant| Title {
        kind,
        text: text.to_string(),
        constant,
        partial,
    };
    let kind = match word {
        "class" | "object" => Some(GroupKind::Class),
        "mod" => Some(GroupKind::Module),
        "module" | "namespace" => Some(GroupKind::Namespace),
        "impl" => return rest.starts_with([' ', '<']).then_some(None),
        "describe" | "context" | "suite" => None,
        _ => return None,
    };
    if let Some(kind) = kind {
        let r = rest.strip_prefix(' ')?.trim_start();
        let end = r
            .find(|c: char| {
                !(c.is_alphanumeric()
                    || c == '_'
                    || c == '$'
                    || (kind == GroupKind::Namespace && matches!(c, '.' | '\\' | ':')))
            })
            .unwrap_or(r.len());
        return Some((end > 0).then(|| title(kind, &r[..end], false)));
    }
    if !(rest.is_empty() || rest.starts_with(['(', ' ', '.'])) {
        return None;
    }
    let mut s = rest;
    while let Some(r) = s.strip_prefix('.') {
        let m = ident_end(r);
        if !matches!(
            &r[..m],
            "only"
                | "skip"
                | "todo"
                | "concurrent"
                | "sequential"
                | "shuffle"
                | "serial"
                | "parallel"
                | "fixme"
        ) {
            // `describe.each` and its relatives make many titles.
            return Some(None);
        }
        s = &r[m..];
    }
    let s = s.trim_start();
    let s = s.strip_prefix('(').unwrap_or(s).trim_start();
    if let Some(text) = literal(s) {
        let whole = is_plain(s, text) && ends_title(&s[text.len() + 2..]);
        return Some(whole.then(|| title(GroupKind::Describe, text, false)));
    }
    // A bare constant of Ruby: `describe Valve do`.
    let m = ident_end(s);
    let after = s[m..].trim_start();
    let bare = s.starts_with(char::is_uppercase)
        && (after == "do"
            || after.starts_with("do ")
            || after.starts_with(')')
            || after
                .strip_prefix(',')
                .is_some_and(|a| !a.trim_start().starts_with(['"', '\''])));
    Some(bare.then(|| title(GroupKind::Describe, &s[..m], true)))
}

/// The groups that hold the definition at `lines[def]`, the outermost first.
/// `None` when the scanner cannot read them: a line with less indent is no
/// group or has a title that is not a literal or a bare constant, or a group at
/// the indent of the definition stays open above it (no indent).
fn ancestors(lines: &[&str], def: usize) -> Option<Vec<Title>> {
    let mut want = indent(lines[def]);
    let mut out = Vec::new();
    // The closers at the indent `want` that wait for their opener.
    let mut closed = 0usize;
    // A line `{` at the indent `want`: the line above it opens the block.
    let mut brace = false;
    for k in (0..def).rev() {
        let t = lines[k].trim().trim_start_matches('\u{feff}');
        if t.is_empty() || is_comment(t) {
            continue;
        }
        let ind = indent(lines[k]);
        if ind > want {
            continue;
        }
        if t == "{" {
            brace |= ind == want;
            continue;
        }
        // The end of a head on more than one line: `):`.
        if t.starts_with(')') {
            continue;
        }
        if ind < want {
            out.push(group_title(t)??);
            want = ind;
            closed = 0;
            brace = false;
            continue;
        }
        let closes = t.starts_with('}') || t == "end" || t.starts_with("end ");
        let opens = std::mem::take(&mut brace) || t.ends_with('{') || t.ends_with(" do");
        if closes && !opens {
            closed += 1;
        } else if opens && !closes {
            if closed > 0 {
                closed -= 1;
            } else if group_title(t).is_some() {
                return None;
            }
        }
    }
    out.reverse();
    Some(out)
}

/// One definition of a test file: a test, or a template.
struct Found<'a> {
    line: usize,
    /// The first line of the items of the definition.
    first: usize,
    text: &'a str,
    test: TestName,
}

/// The definitions of a file. It reads the file once. A template with a literal
/// and no placeholder (`it.each(rows)('closes')`) gives a name too.
fn tests_of<'a>(lines: &[&'a str]) -> Vec<Found<'a>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() || is_comment(t) {
            i += 1;
            continue;
        }
        let Some(d) = definition(lines, i, false) else {
            i += SCAN_LINES;
            continue;
        };
        let test = match classify(d.text, d.annotated) {
            Some(Under::Test(t)) => Some(t),
            Some(Under::Template(name))
                if !name.is_empty() && !name.contains(['%', '$']) && !name.contains("#{") =>
            {
                Some(TestName {
                    name,
                    call: true,
                    bt: false,
                    recv: None,
                })
            }
            _ => None,
        };
        if let Some(test) = test {
            out.push(Found {
                line: d.line,
                first: d.first,
                text: d.text,
                test,
            });
        }
        i = d.line + 1;
    }
    out
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

/// True if 2 dotted names are equal or one ends the other at a `.`: runners
/// write them from their own root.
fn relates(a: &str, b: &str) -> bool {
    ends_in_segments(a, b) || ends_in_segments(b, a)
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

/// A path of a report with forward slashes and no leading `./`.
fn report_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

/// The separators of the segments of a case name. A space inside a name is no
/// separator: `Closes when cold` is one segment.
const SEPS: [&str; 6] = [" > ", " \u{203a} ", "::", "/", ".", "#"];

/// True if `text` has `lit` as a whole segment: at the start or after a
/// separator, and at the end or before a separator.
fn has_whole(text: &str, lit: &str) -> bool {
    !lit.is_empty()
        && text.match_indices(lit).any(|(i, _)| {
            let j = i + lit.len();
            (i == 0 || SEPS.iter().any(|s| text[..i].ends_with(s)))
                && (j == text.len() || SEPS.iter().any(|s| text[j..].starts_with(s)))
        })
}

/// The case name, and the parts of it before each `(` and ` [`: a report adds
/// the parameters of a method after its name (`closes()`, `closes(int) [1] 3`).
fn tail_variants(case: &str) -> Vec<&str> {
    let mut out = vec![case];
    for (i, c) in case.char_indices() {
        if i > 0 && (c == '(' || case[i..].starts_with(" [")) {
            out.push(case[..i].trim_end());
        }
    }
    out
}

/// The head of a case name for an identifier: the part before the parameters
/// that start right after the name (`test_x[1-2]`, `test1(String) [1] a`) and
/// before ` with data set` (PHPUnit). A `(` after a space is part of a display
/// name.
fn cut_head(case: &str) -> &str {
    let mut cut = case.find(" with data set").unwrap_or(case.len());
    for (i, c) in case.char_indices() {
        if i < cut && (c == '(' || c == '[') && i > 0 && !case[..i].ends_with(char::is_whitespace) {
            cut = i;
            break;
        }
    }
    &case[..cut]
}

/// True if the name of the case can name the test (design 0.8.1, D82 item 4).
/// It is wide on purpose: each case that it accepts is read, and the identity of
/// the test then decides what the case is.
///
/// - A Go method of a suite: `<Test function>/<method>`, and more segments for a
///   subtest.
/// - A name in a literal (the form call): the name is a whole segment, or it is
///   the end of the case name after a space, unless the case name has a segment
///   of ` > `. A Kotlin name in backticks has the parameters after it.
/// - An identifier: it is a whole segment of the name before its parameters.
fn name_match(case: &str, test: &TestName) -> bool {
    if test.recv.is_some() {
        // A case of one segment names no method.
        return case.split('/').nth(1) == Some(test.name.as_str());
    }
    if !test.call {
        return has_whole(cut_head(case), &test.name);
    }
    let lit = test.name.as_str();
    let variants = if test.bt {
        tail_variants(case)
    } else {
        vec![case]
    };
    let space = |v: &str| {
        v.strip_suffix(lit).is_some_and(|prefix| {
            prefix.ends_with(' ') && !prefix.contains(" > ") && !prefix.contains(" \u{203a} ")
        })
    };
    variants.iter().any(|v| has_whole(v, lit) || space(v))
}

/// The types that a line `suite.Run(t, ...)` runs: `new(<Type>)` and
/// `&<Type>{...}`.
fn suite_types(call: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (open, close) in [("new(", ')'), ("&", '{')] {
        for (at, _) in call.match_indices(open) {
            let rest = &call[at + open.len()..];
            let n = ident_end(rest);
            if n > 0 && rest[n..].starts_with(close) {
                out.push(rest[..n].to_string());
            }
        }
    }
    out
}

/// The functions that run the Go suite of the type `ty` and that can name its
/// method `method`. A function `func TestX(t *testing.T)` of the files runs a
/// type when its lines have `suite.Run(t, new(<Type>))` or `suite.Run(t,
/// &<Type>{...})`. The result leaves out a function that also runs another type
/// with a method of the same name: the cases of both methods have one name, so
/// they bind to neither. The result is empty when no function runs the type,
/// and then the method binds no case.
pub fn suite_funcs<'a>(
    texts: impl IntoIterator<Item = &'a str>,
    ty: &str,
    method: &str,
) -> Vec<String> {
    let mut runs: Vec<(String, String)> = Vec::new();
    let mut methods: HashSet<(String, String)> = HashSet::new();
    for text in texts {
        let mut current: Option<&str> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("func ") {
                if rest.starts_with('(') {
                    // A method: `func (s *Suite) TestX(`.
                    current = None;
                    if let Some(close) = rest.find(')') {
                        let recv = rest[1..close].split_whitespace().last().unwrap_or("");
                        let recv = recv.trim_start_matches('*');
                        let after = rest[close + 1..].trim_start();
                        let name = &after[..ident_end(after)];
                        if !recv.is_empty() && !name.is_empty() {
                            methods.insert((recv.to_string(), name.to_string()));
                        }
                    }
                } else {
                    let name = &rest[..ident_end(rest)];
                    current = (name.starts_with("Test") && rest[name.len()..].starts_with('('))
                        .then_some(name);
                }
            }
            if let Some(func) = current
                && let Some(at) = line.find("suite.Run(")
            {
                for t in suite_types(&line[at..]) {
                    runs.push((func.to_string(), t));
                }
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    for (func, t) in &runs {
        let clash = runs.iter().any(|(g, other)| {
            g == func && other != ty && methods.contains(&(other.clone(), method.to_string()))
        });
        if t == ty && !clash && !out.contains(func) {
            out.push(func.clone());
        }
    }
    out
}

/// The path of the folder of a file, or an empty text for a file in the root.
fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(f, _)| f)
}

/// The ends of a path at each `/`, the longest first: `a/b/c`, `b/c`, `c`.
fn path_ends(path: &str) -> impl Iterator<Item = &str> {
    std::iter::once(path).chain(path.match_indices('/').map(|(i, _)| &path[i + 1..]))
}

/// The ends of a dotted name at each `.`, the longest first.
fn dotted_ends(name: &str) -> impl Iterator<Item = &str> {
    std::iter::once(name).chain(name.match_indices('.').map(|(i, _)| &name[i + 1..]))
}

/// The family of a test file for the count of its dotted path: the extension,
/// and one family for the languages of the JVM.
fn family(path: &str) -> &str {
    match path.rsplit_once('.').map_or("", |(_, e)| e) {
        "java" | "kt" | "kts" | "scala" | "groovy" => "jvm",
        other => other,
    }
}

/// The part of a Rust crate that a file belongs to, as its path tells.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    /// An integration test binary: `tests/<name>.rs` or `tests/<name>/`.
    Test(String),
    /// `src/lib.rs`.
    Lib,
    /// `src/main.rs`.
    Main,
    /// A binary of `src/bin/`.
    Bin(String),
    /// Another file of `src/`: a module of the library or of a binary.
    Module,
}

/// A Rust test file: the folder of its crate, its part and its module path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    root: String,
    part: Part,
    module: Vec<String>,
}

impl Target {
    /// 2 files with one key can write the same cases.
    fn key(&self) -> String {
        match &self.part {
            Part::Test(name) => format!("test {name}"),
            Part::Lib => "lib".to_string(),
            Part::Main => "main".to_string(),
            Part::Bin(name) => format!("bin {name} {}", self.module.join("::")),
            Part::Module => format!("module {}", self.module.join("::")),
        }
    }
}

/// The target of a Rust file. `None` for a layout that the path does not tell:
/// a file outside `src/` and `tests/`, a path with both, or a file in a folder
/// of `tests/` with no `main.rs` in the path list.
fn rust_target(path: &str, files: &HashSet<&str>) -> Option<Target> {
    let stem_path = path.strip_suffix(".rs")?;
    let segs: Vec<&str> = stem_path.split('/').collect();
    let dirs = &segs[..segs.len() - 1];
    let has = |name: &str| dirs.contains(&name);
    let anchor = match (has("src"), has("tests")) {
        (true, false) => "src",
        (false, true) => "tests",
        _ => return None,
    };
    let at = dirs.iter().rposition(|s| *s == anchor)?;
    let root = segs[..at].join("/");
    let rest = &segs[at + 1..];
    let module = |from: &[&str], drop: &[&str]| -> Vec<String> {
        let mut m: Vec<String> = from.iter().map(|s| s.to_string()).collect();
        if m.last().is_some_and(|l| drop.contains(&l.as_str())) {
            m.pop();
        }
        m
    };
    let (part, module) = if anchor == "tests" {
        match rest {
            [stem] => (Part::Test(stem.to_string()), Vec::new()),
            [dir, tail @ ..] => {
                let main = if root.is_empty() {
                    format!("tests/{dir}/main.rs")
                } else {
                    format!("{root}/tests/{dir}/main.rs")
                };
                if !files.contains(main.as_str()) {
                    return None;
                }
                (Part::Test(dir.to_string()), module(tail, &["main", "mod"]))
            }
            [] => return None,
        }
    } else {
        match rest {
            ["lib"] => (Part::Lib, Vec::new()),
            ["main"] => (Part::Main, Vec::new()),
            ["bin", name] => (Part::Bin(name.to_string()), Vec::new()),
            ["bin", name, tail @ ..] => {
                (Part::Bin(name.to_string()), module(tail, &["main", "mod"]))
            }
            _ => (Part::Module, module(rest, &["mod"])),
        }
    };
    Some(Target { root, part, module })
}

/// What the test files name: their paths, the ends of their paths and of their
/// dotted paths, the folders of Go and the targets of Rust. Each count is the
/// number of the files, for the rule that an identity must be unique.
struct Names<'t> {
    whole: HashSet<&'t str>,
    ends: HashMap<&'t str, usize>,
    /// (family, dotted path) to the number of files with it.
    dotted_whole: HashMap<(&'t str, String), usize>,
    /// (family, end of a dotted path) to the number of files with it.
    dotted: HashMap<(&'t str, String), usize>,
    /// (family) to the number of files.
    families: HashMap<&'t str, usize>,
    /// The stem of a C# file to the number of files with it.
    stems: HashMap<&'t str, usize>,
    go_folders: Vec<&'t str>,
    /// The key of a Rust target to the folders of the crates that have it.
    rust: HashMap<String, HashSet<String>>,
}

impl<'t> Names<'t> {
    fn new(test_files: &'t [String]) -> Names<'t> {
        let mut n = Names {
            whole: test_files.iter().map(String::as_str).collect(),
            ends: HashMap::new(),
            dotted_whole: HashMap::new(),
            dotted: HashMap::new(),
            families: HashMap::new(),
            stems: HashMap::new(),
            go_folders: Vec::new(),
            rust: HashMap::new(),
        };
        let mut folders = HashSet::new();
        let files: Vec<&'t str> = test_files.iter().map(String::as_str).collect();
        for f in files {
            for end in path_ends(f) {
                *n.ends.entry(end).or_default() += 1;
            }
            let fam = family(f);
            *n.families.entry(fam).or_default() += 1;
            let d = dotted(f);
            for end in dotted_ends(&d) {
                *n.dotted.entry((fam, end.to_string())).or_default() += 1;
            }
            *n.dotted_whole.entry((fam, d)).or_default() += 1;
            if fam == "cs" {
                let name = f.rsplit('/').next().unwrap_or(f);
                *n.stems.entry(name.trim_end_matches(".cs")).or_default() += 1;
            }
            if fam == "go" && folders.insert(folder_of(f)) {
                n.go_folders.push(folder_of(f));
            }
            if let Some(t) = rust_target(f, &n.whole) {
                n.rust.entry(t.key()).or_default().insert(t.root);
            }
        }
        n
    }

    /// The number of test files that the path of a case names.
    fn files_named(&self, case_file: &str) -> usize {
        let shorter = path_ends(case_file)
            .skip(1)
            .filter(|e| self.whole.contains(e))
            .count();
        self.ends.get(case_file).copied().unwrap_or(0) + shorter
    }

    /// The number of test files of a family whose dotted path relates to
    /// `name`. An empty name relates to each file of the family.
    fn dotted_named(&self, path: &str, name: &str) -> usize {
        let fam = family(path);
        if name.is_empty() {
            return self.families.get(fam).copied().unwrap_or(0);
        }
        // The key borrows nothing from the maps.
        let get = |map: &HashMap<(&'t str, String), usize>, key: &str| {
            self.families
                .get_key_value(fam)
                .and_then(|(f, _)| map.get(&(*f, key.to_string())))
                .copied()
                .unwrap_or(0)
        };
        let shorter: usize = dotted_ends(name)
            .skip(1)
            .map(|e| get(&self.dotted_whole, e))
            .sum();
        get(&self.dotted, name) + shorter
    }
}

/// The most bytes and separators of a name that has all its keys.
const LONG_BYTES: usize = 256;
const LONG_SEPS: usize = 32;

/// True for a case name that is too long to read by its segments: more than 256
/// bytes, or more than 32 separators (`SEPS`, `(` and `[`).
fn is_long(name: &str) -> bool {
    name.len() > LONG_BYTES
        || SEPS.iter().map(|s| name.matches(s).count()).sum::<usize>()
            + name.matches(['(', '[']).count()
            > LONG_SEPS
}

/// True if a row tail of a runner can start at byte `i` of a name: `[`, a `(`
/// that does not follow a space, ` [` or ` with data set`.
fn tail_at(name: &str, i: usize) -> bool {
    let rest = &name[i..];
    rest.starts_with('[')
        || (rest.starts_with('(') && !name[..i].ends_with(char::is_whitespace))
        || rest.starts_with(" [")
        || rest.starts_with(" with data set")
}

/// A name without a leading `[<project>] ` of Playwright.
fn without_project(name: &str) -> &str {
    if name.starts_with('[') {
        name.split_once("] ").map_or(name, |(_, rest)| rest)
    } else {
        name
    }
}

/// The keys of a case name for the index, each a part of the name, and true for
/// a head with more words than the keys hold. The keys are:
///
/// - the whole name, and the name without a `[<project>] ` prefix;
/// - the prefixes before the first 32 places where a row tail, a `::` or a `/`
///   starts (in the first 256 bytes);
/// - for a head that is not too long: each whole segment of the head, and, when
///   it has no segment of ` > `, each end of it after one of its last 32 spaces.
///
/// A name has at most 98 keys, and one more with a project prefix.
fn case_keys(name: &str) -> (Vec<&str>, bool) {
    let base = without_project(name);
    let mut keys = vec![name, base];
    let mut prefixes = 0;
    let mut head = base.len();
    let mut last = "";
    for (i, _) in base.char_indices().skip(1) {
        let tail = tail_at(base, i);
        if tail && head == base.len() {
            head = i;
        }
        let rest = &base[i..];
        if i <= LONG_BYTES
            && prefixes < LONG_SEPS
            && (tail || rest.starts_with('/') || rest.starts_with("::"))
        {
            let prefix = base[..i].trim_end();
            if prefix != last {
                keys.push(prefix);
                prefixes += 1;
                last = prefix;
            }
        }
        if (i > LONG_BYTES || prefixes >= LONG_SEPS) && head < base.len() {
            break;
        }
    }
    let head = base[..head].trim_end();
    let mut wordy = false;
    if !is_long(head) {
        let mut cuts: Vec<(usize, usize)> = SEPS
            .iter()
            .flat_map(|s| head.match_indices(s).map(|(i, _)| (i, i + s.len())))
            .collect();
        cuts.sort_unstable();
        let mut from = 0;
        for (a, b) in cuts {
            if a >= from {
                keys.push(&head[from..a]);
                from = b;
            }
        }
        keys.push(&head[from..]);
        if !head.contains(" > ") && !head.contains(" \u{203a} ") {
            let mut spaces = head.rmatch_indices(' ');
            keys.extend(spaces.by_ref().take(LONG_SEPS).map(|(p, _)| &head[p + 1..]));
            wordy = spaces.next().is_some();
        }
    }
    keys.retain(|k| !k.is_empty());
    keys.sort_unstable();
    keys.dedup();
    (keys, wordy)
}

/// The cases of one key. Most keys have one case, so an entry of the index is
/// small: a list of more cases is in a table of its own.
#[derive(Clone, Copy)]
enum Posting {
    One(u32),
    Many(u32),
}

/// The index of the keys of the case names. A key is held as its hash: a report
/// can have millions of keys. Two keys can share a hash, so a lookup only finds
/// the cases to read, and each reader checks the name of the case.
#[derive(Default)]
struct Index {
    hasher: RandomState,
    keys: HashMap<u64, Posting>,
    lists: Vec<Vec<u32>>,
}

impl Index {
    fn add(&mut self, key: &str, case: u32) {
        match self.keys.entry(self.hasher.hash_one(key)) {
            Entry::Vacant(slot) => {
                slot.insert(Posting::One(case));
            }
            Entry::Occupied(mut slot) => match *slot.get() {
                Posting::One(first) => {
                    slot.insert(Posting::Many(self.lists.len() as u32));
                    self.lists.push(vec![first, case]);
                }
                Posting::Many(list) => self.lists[list as usize].push(case),
            },
        }
    }

    fn get(&self, key: &str) -> &[u32] {
        match self.keys.get(&self.hasher.hash_one(key)) {
            None => &[],
            Some(Posting::One(case)) => std::slice::from_ref(case),
            Some(Posting::Many(list)) => &self.lists[*list as usize],
        }
    }

    /// The pairs of a key and a case.
    #[cfg(test)]
    fn entries(&self) -> usize {
        let ones = self.keys.values().filter(|p| matches!(p, Posting::One(_)));
        ones.count() + self.lists.iter().map(Vec::len).sum::<usize>()
    }
}

/// The form of a definition (design 0.8.1, D82 item 2). With the shape of a
/// case, it gives the row of the identity rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    Def,
    Fn,
    Func,
    Call,
    /// The form method, and a Kotlin name in backticks.
    Method,
}

fn form_of(test: &TestName, text: &str) -> Form {
    if test.bt {
        return Form::Method;
    }
    if test.call {
        return Form::Call;
    }
    let body = strip_modifiers(text);
    match &body[..ident_end(body)] {
        "def" => Form::Def,
        "fn" => Form::Fn,
        "func" => Form::Func,
        _ => Form::Method,
    }
}

/// A row of the identity rule (docs/test-report.md): the form of the
/// definition and the shape of the case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// A case with no `classname`: the line format.
    Lines,
    /// A pair that no row lists: it has no full field.
    Unlisted,
    PytestXunit2,
    PytestXunit1,
    Nextest,
    Go,
    /// JUnit and xUnit.
    Jvm,
    Phpunit,
    /// jest-junit with `addFileAttribute`, and RSpec.
    FileTitles,
    /// Vitest and Playwright.
    PathTitles,
    JestDefaults,
}

fn row_of(form: Form, case: &Case) -> Row {
    let Some(cn) = case.classname.as_deref() else {
        return Row::Lines;
    };
    if case.file.is_some() {
        return match form {
            Form::Def => Row::PytestXunit1,
            Form::Method => Row::Phpunit,
            Form::Call => Row::FileTitles,
            Form::Fn | Form::Func => Row::Unlisted,
        };
    }
    // jest-junit writes its title as the `classname`, also with a `/` in it.
    if form == Form::Call && cn == case.name {
        return Row::JestDefaults;
    }
    let binary = cn.contains("::");
    let path = !binary && cn.contains(['/', '\\']);
    // A spec file in the root of the runner has no `/`: its extension shows
    // that the `classname` is a path.
    let script = !binary
        && cn.rsplit_once('.').is_some_and(|(_, e)| {
            matches!(
                e,
                "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" | "vue" | "svelte"
            )
        });
    let extension = script || path && cn.rsplit(['/', '\\']).next().unwrap_or(cn).contains('.');
    // The name of a Cargo package has no dot and no space.
    let package = cn
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    match form {
        Form::Fn if binary || package => Row::Nextest,
        Form::Func if !binary => Row::Go,
        Form::Def if !binary && !path => Row::PytestXunit2,
        Form::Method if !binary && !path => Row::Jvm,
        Form::Call if extension => Row::PathTitles,
        Form::Call if !binary && !path => Row::JestDefaults,
        _ => Row::Unlisted,
    }
}

/// One field of a case against the identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Same,
    /// The report and the identity both hold the field in full, and they
    /// differ: the case is another test.
    Differs,
    /// One of them does not hold the field in full.
    Unread,
}

impl Field {
    fn of(same: bool) -> Field {
        if same { Field::Same } else { Field::Differs }
    }

    fn and(self, other: Field) -> Field {
        match (self, other) {
            (Field::Differs, _) | (_, Field::Differs) => Field::Differs,
            (Field::Unread, _) | (_, Field::Unread) => Field::Unread,
            _ => Field::Same,
        }
    }
}

/// The `package` or `namespace` of the test of a JVM or .NET file.
#[derive(Debug, PartialEq, Eq)]
enum Package {
    Line(String),
    Absent,
    /// A `namespace` block that the indent does not show.
    Unread,
}

/// The line `package x.y`, or a file-scoped `namespace x.y;`.
fn package_line(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|l| {
        let t = l.trim();
        let rest = t.strip_prefix("package ").or_else(|| {
            t.strip_prefix("namespace ")
                .filter(|r| r.trim_end().ends_with(';'))
        })?;
        let name = rest.split([';', ' ', '{']).next().unwrap_or("");
        (!name.is_empty()).then(|| name.replace('\\', "."))
    })
}

fn package_of(lines: &[&str], chain: Option<&[Title]>) -> Package {
    let blocks: Vec<&str> = chain
        .unwrap_or_default()
        .iter()
        .filter(|t| t.kind == GroupKind::Namespace)
        .map(|t| t.text.as_str())
        .collect();
    if !blocks.is_empty() {
        return Package::Line(blocks.join("."));
    }
    if let Some(p) = package_line(lines) {
        return Package::Line(p);
    }
    if lines.iter().any(|l| l.trim().starts_with("namespace ")) {
        return Package::Unread;
    }
    Package::Absent
}

/// The `DisplayName` of the items of a definition: `None` when it has none, and
/// `Some(None)` when its value is no literal.
fn display_name(lines: &[&str], first: usize, line: usize) -> Option<Option<String>> {
    let mut from = first;
    while from > 0 {
        let above = lines[from - 1].trim();
        if above.starts_with('@') || above.starts_with('[') || is_comment(above) {
            from -= 1;
        } else {
            break;
        }
    }
    lines.get(from..=line)?.iter().find_map(|l| {
        let (_, rest) = l.split_once("DisplayName")?;
        let value = rest
            .trim_start()
            .trim_start_matches(['(', '='])
            .trim_start();
        Some(literal(value).map(str::to_string))
    })
}

/// A Go module: the folder of its `go.mod` and its module path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GoModule {
    dir: String,
    path: String,
}

impl GoModule {
    /// The folder of a package below the module, empty for its root.
    fn relative<'f>(&self, folder: &'f str) -> &'f str {
        if self.dir.is_empty() {
            folder
        } else {
            folder
                .strip_prefix(&self.dir)
                .map_or(folder, |r| r.trim_start_matches('/'))
        }
    }

    fn import(&self, folder: &str) -> String {
        match self.relative(folder) {
            "" => self.path.clone(),
            rel => format!("{}/{rel}", self.path),
        }
    }
}

/// The `classname` of a Go case against the folder of the tag. With a `go.mod`,
/// it is the import path or the folder below the module (gotestsum
/// `relative`). With none, the folder must end it.
fn go_field(classname: &str, folder: &str, module: Option<&GoModule>) -> Field {
    let short = |of: &str| {
        !classname.contains('/') && of.contains('/') && of.rsplit('/').next() == Some(classname)
    };
    let ends = |long: &str, end: &str| long.strip_suffix(end).is_some_and(|r| r.ends_with('/'));
    match module {
        Some(m) => {
            let rel = m.relative(folder);
            if classname == m.import(folder)
                || (!rel.is_empty() && classname == rel)
                || (rel.is_empty() && classname == ".")
            {
                Field::Same
            } else if short(rel) {
                Field::Unread
            } else {
                Field::Differs
            }
        }
        None if folder.is_empty() => Field::Unread,
        None if classname == folder || ends(classname, folder) => Field::Same,
        None if short(folder) || ends(folder, classname) => Field::Unread,
        None => Field::Differs,
    }
}

/// A nextest binary id: the package, and the part of the crate.
fn binary_id(classname: &str) -> (&str, Option<Part>) {
    let Some((package, rest)) = classname.split_once("::") else {
        return (classname, Some(Part::Lib));
    };
    let part = if let Some(name) = rest.strip_prefix("bin/") {
        Some(Part::Bin(name.to_string()))
    } else {
        let name = rest.strip_prefix("test/").unwrap_or(rest);
        // Another kind (`bench/`, `example/`) is no test binary of a test file.
        (!name.contains('/')).then(|| Part::Test(name.to_string()))
    };
    (package, part)
}

/// The kind of a binary id against the target of a Rust file. `package` is the
/// name in the `Cargo.toml` of the file.
fn part_field(target: &Target, id: Option<&Part>, package: Option<&str>) -> Field {
    match (&target.part, id) {
        (Part::Test(a), Some(Part::Test(b))) => Field::of(a == b),
        (Part::Lib, Some(Part::Lib)) => Field::Same,
        (Part::Bin(a), Some(Part::Bin(b))) => Field::of(a == b),
        // The name of the binary of `src/main.rs` is the package, unless
        // `Cargo.toml` gives it another name.
        (Part::Main, Some(Part::Bin(b))) => {
            if package == Some(b.as_str()) {
                Field::Same
            } else {
                Field::Unread
            }
        }
        // A module belongs to the library, or to a binary: the path does not
        // tell which.
        (Part::Module, Some(Part::Lib)) => Field::Same,
        (Part::Module, Some(Part::Bin(_))) => Field::Unread,
        _ => Field::Differs,
    }
}

/// The identity that the scanner builds from the definition under a tag.
struct Identity<'p> {
    path: &'p str,
    test: TestName,
    form: Form,
    /// The groups around the definition. `None`: the scanner cannot read them.
    chain: Option<Vec<Title>>,
    /// False for a title of a call that is not the whole literal.
    plain: bool,
    /// The functions that run the Go suite of the method.
    funcs: Vec<String>,
    package: Package,
    display: Option<Option<String>>,
    target: Option<Target>,
    cargo: Option<String>,
    go: Option<GoModule>,
    /// The other definitions of the tag file.
    others: Vec<Found<'p>>,
    lines: &'p [&'p str],
    /// The `classname` of pytest: the dotted path and the classes. A tag can
    /// have thousands of cases to read, so it is built once.
    pytest: Option<String>,
    /// True if the file of a test of the form call has a template. A template
    /// can give any title, also the title of the test.
    templates: bool,
}

/// The case that names the test, as the rule judges it.
struct Judged<'a> {
    case: &'a Case,
    /// The name of the case without a project prefix: with the `classname` and
    /// the `file`, the identity of the case.
    name: &'a str,
    /// For an own case, the field that ties it to the tag.
    own: Option<Fit>,
    /// For a weak case, the cause.
    why: Option<Why>,
    /// True when the runner writes the same case more than once in one report
    /// (the projects of Playwright).
    repeats: bool,
}

impl<'a> Judged<'a> {
    fn new(case: &'a Case, name: &'a str, result: Result<Fit, Why>) -> Judged<'a> {
        Judged {
            case,
            name,
            own: result.ok(),
            why: result.err(),
            repeats: false,
        }
    }

    fn key(&self) -> (&'a str, Option<&'a str>, Option<&'a str>) {
        (
            self.name,
            self.case.classname.as_deref(),
            self.case.file.as_deref(),
        )
    }
}

/// What `case:` reads of the tag file.
struct TagFile<'p> {
    path: &'p str,
    classes: HashSet<String>,
    package: Option<String>,
}

/// The titles and the name of a test, joined with `sep`.
fn joined(chain: &[Title], name: &str, sep: &str) -> String {
    let mut out = String::new();
    for t in chain {
        out.push_str(&t.text);
        out.push_str(sep);
    }
    out.push_str(name);
    out
}

/// The 3 full titles of a test of the form call: with a space as RSpec and
/// jest-junit join them, with ` > ` (Vitest) and with ` \u{203a} `
/// (Playwright). RSpec writes no space between a group of a bare constant and a
/// group below it whose title starts with `#`, `.` or `::`.
fn full_titles(chain: &[Title], name: &str) -> [String; 3] {
    let mut space = String::new();
    for (i, t) in chain.iter().enumerate() {
        let no_space = i > 0
            && chain[i - 1].constant
            && (t.text.starts_with(['#', '.']) || t.text.starts_with("::"));
        if i > 0 && !no_space {
            space.push(' ');
        }
        space.push_str(&t.text);
    }
    if !chain.is_empty() {
        space.push(' ');
    }
    space.push_str(name);
    [
        space,
        joined(chain, name, " > "),
        joined(chain, name, " \u{203a} "),
    ]
}

impl Identity<'_> {
    /// The groups, when each of them is of a kind that the form allows.
    fn groups(&self) -> Option<&[Title]> {
        let chain = self.chain.as_deref()?;
        let fits = match self.form {
            Form::Def => chain.iter().all(|t| t.kind == GroupKind::Class),
            Form::Fn => chain.iter().all(|t| t.kind == GroupKind::Module),
            Form::Func => chain.is_empty(),
            Form::Call => chain.iter().all(|t| t.kind == GroupKind::Describe),
            // A method has a class.
            Form::Method => {
                chain.iter().any(|t| t.kind == GroupKind::Class)
                    && chain
                        .iter()
                        .all(|t| matches!(t.kind, GroupKind::Class | GroupKind::Namespace))
            }
        };
        fits.then_some(chain)
    }

    /// The classes of the chain, joined with `.`.
    fn classes(&self) -> Option<String> {
        let names: Vec<&str> = self
            .groups()?
            .iter()
            .filter(|t| t.kind == GroupKind::Class)
            .map(|t| t.text.as_str())
            .collect();
        Some(names.join("."))
    }

    /// The module path of a Rust test: the file, the `mod` blocks and the `fn`.
    fn module_path(&self) -> Option<String> {
        let mut path = self.target.as_ref()?.module.clone();
        path.extend(self.groups()?.iter().map(|t| t.text.clone()));
        path.push(self.test.name.clone());
        Some(path.join("::"))
    }

    /// True when the scanner builds the whole identity.
    fn built(&self, row: Row) -> bool {
        self.groups().is_some()
            && self.plain
            && self.display.is_none()
            && match row {
                Row::Nextest => self.target.is_some(),
                Row::Jvm => self.package != Package::Unread,
                _ => true,
            }
    }

    /// The names that a case of the test has in this row, before its row tail.
    /// Empty when the scanner cannot build the name.
    fn heads(&self, row: Row) -> Vec<String> {
        let name = &self.test.name;
        match row {
            Row::PytestXunit2 | Row::PytestXunit1 => vec![name.clone()],
            Row::Go if self.test.recv.is_some() => {
                self.funcs.iter().map(|f| format!("{f}/{name}")).collect()
            }
            Row::Go => vec![name.clone()],
            Row::Phpunit if self.display.is_none() => vec![name.clone()],
            Row::Jvm if self.display.is_none() => {
                // xUnit writes `M`, `C.M` or `N.C.M` (`MethodFormat`).
                let Some(classes) = self.classes() else {
                    return Vec::new();
                };
                let mut out = vec![name.clone(), format!("{classes}.{name}")];
                if let Package::Line(p) = &self.package {
                    out.push(format!("{p}.{classes}.{name}"));
                }
                out
            }
            Row::Nextest => self.module_path().into_iter().collect(),
            Row::FileTitles | Row::PathTitles | Row::JestDefaults if self.plain => {
                let Some(chain) = self.groups() else {
                    return Vec::new();
                };
                let titles = full_titles(chain, name);
                if row == Row::JestDefaults {
                    titles[..1].to_vec()
                } else {
                    titles.to_vec()
                }
            }
            _ => Vec::new(),
        }
    }

    /// The names to look up in the index, for each row of the form. A case of
    /// the line format has one of them.
    fn lookups(&self) -> Vec<String> {
        let name = &self.test.name;
        let mut out = vec![name.clone()];
        match self.form {
            Form::Def => {}
            Form::Fn => out.extend(self.module_path()),
            Form::Func => out.extend(self.heads(Row::Go)),
            Form::Method => {
                out.extend(self.heads(Row::Jvm));
                out.extend(self.display.clone().flatten());
            }
            Form::Call => {
                if let Some(chain) = self.groups() {
                    out.extend(full_titles(chain, name));
                }
            }
        }
        out
    }

    /// The identity part of the name of a case that has the name of the test in
    /// this row: the name without a project prefix. `None` when it differs.
    fn hit<'c>(&self, row: Row, case: &'c str) -> Option<&'c str> {
        let heads = self.heads(row);
        let tail = |rest: &str| match row {
            Row::PytestXunit2 | Row::PytestXunit1 => rest.starts_with('['),
            Row::Nextest => rest.starts_with("::"),
            Row::Go => rest.starts_with('/'),
            Row::Jvm | Row::Phpunit => {
                rest.starts_with(['(', '[']) || rest.starts_with(" [") || tail_at(rest, 0)
            }
            _ => false,
        };
        let fits = |name: &str, head: &String| {
            name.strip_prefix(head.as_str())
                .is_some_and(|rest| rest.is_empty() || tail(rest))
        };
        if heads.iter().any(|h| fits(case, h)) {
            return Some(case);
        }
        // Playwright can put `[<project>] ` before the title.
        let short = without_project(case);
        (matches!(row, Row::FileTitles | Row::PathTitles)
            && short != case
            && heads.iter().any(|h| {
                short == h
                    && (h.contains(" \u{203a} ") || self.groups().is_some_and(<[_]>::is_empty))
            }))
        .then_some(short)
    }

    /// True if the name of the case can name the test: only such a case is read.
    fn names(&self, case: &str) -> bool {
        if self.test.recv.is_some() {
            // Only a case of a function that runs the suite.
            let first = case.split('/').next().unwrap_or("");
            if !self.funcs.iter().any(|f| f == first) {
                return false;
            }
        }
        name_match(case, &self.test)
            || self.lookups().iter().any(|l| l == without_project(case))
            || [
                Row::PytestXunit2,
                Row::Nextest,
                Row::Go,
                Row::Jvm,
                Row::PathTitles,
            ]
            .iter()
            .any(|row| row_form(*row) == self.form && self.hit(*row, case).is_some())
    }

    /// The `classname` of a case against the one of pytest.
    fn pytest_classname(&self, classname: &str) -> Field {
        match &self.pytest {
            Some(want) => Field::of(relates(classname, want)),
            None => Field::Unread,
        }
    }

    /// The field that names the test file: the `file`, or the whole
    /// `classname`.
    fn file_field(&self, row: Row, case: &Case) -> Field {
        let file = case.file.as_deref().map(report_path);
        // A case with the `file` of another file is another test in each row.
        if file.as_deref().is_some_and(|f| !same_file(f, self.path)) {
            return Field::Differs;
        }
        let cn = case.classname.as_deref().unwrap_or("");
        match row {
            Row::Lines | Row::Unlisted => Field::Unread,
            Row::FileTitles | Row::JestDefaults => Field::Same,
            Row::PytestXunit1 | Row::PytestXunit2 => self.pytest_classname(cn),
            Row::Nextest => {
                let Some(target) = &self.target else {
                    return Field::Unread;
                };
                let (package, part) = binary_id(cn);
                let part = part_field(target, part.as_ref(), self.cargo.as_deref());
                let package = match &self.cargo {
                    Some(name) => Field::of(name == package),
                    // With no `Cargo.toml`, a binary of `tests/` is unique by
                    // its key, and the library is not.
                    None if matches!(target.part, Part::Test(_)) => Field::Same,
                    None => Field::Unread,
                };
                part.and(package)
            }
            Row::Go => go_field(cn, folder_of(self.path), self.go.as_ref()),
            Row::Jvm => {
                let Some(classes) = self.classes() else {
                    return Field::Unread;
                };
                // `$` of the JVM and `+` of .NET join a nested class.
                let cn = cn.replace(['$', '+'], ".");
                match &self.package {
                    Package::Line(p) if cn == format!("{p}.{classes}") => Field::Same,
                    // A newer Surefire writes the outer class for a nested one.
                    Package::Line(p) if self.nested() && cn == self.outer(p) => Field::Unread,
                    Package::Line(_) => Field::Differs,
                    Package::Absent
                        if ends_in_segments(&cn, &classes)
                            && ends_in_segments(&dotted(self.path), &cn) =>
                    {
                        Field::Same
                    }
                    Package::Absent | Package::Unread => Field::Unread,
                }
            }
            Row::Phpunit => {
                let Some(classes) = self.classes() else {
                    return Field::Unread;
                };
                Field::of(ends_in_segments(&cn.replace('\\', "."), &classes))
            }
            Row::PathTitles => Field::of(same_file(&report_path(cn), self.path)),
        }
    }

    /// True for a test of a class inside a class.
    fn nested(&self) -> bool {
        self.groups()
            .is_some_and(|c| c.iter().filter(|t| t.kind == GroupKind::Class).count() > 1)
    }

    /// The package with the outermost class.
    fn outer(&self, package: &str) -> String {
        let top = self
            .groups()
            .and_then(|c| c.iter().find(|t| t.kind == GroupKind::Class))
            .map_or("", |t| t.text.as_str());
        format!("{package}.{top}")
    }

    /// True if another definition of the tag file builds the name of the case.
    fn twin(&self, case: &str, name: &str) -> bool {
        let mine = self.groups();
        self.others.iter().any(|o| {
            if self.form == Form::Call {
                let t = o.test.name.as_str();
                if !o.test.call || !(name.ends_with(t) || case.ends_with(t)) {
                    return false;
                }
                return match ancestors(self.lines, o.line) {
                    // A group that the scanner cannot read can build it.
                    None => t == self.test.name,
                    Some(chain) => full_titles(&chain, t)
                        .iter()
                        .any(|f| f == name || f == case),
                };
            }
            // A display name replaces the name of the method in some runners.
            if self.form == Form::Method
                && display_name(self.lines, o.first, o.line)
                    .flatten()
                    .is_some_and(|d| d == case)
            {
                return true;
            }
            if form_of(&o.test, o.text) != self.form || o.test.recv != self.test.recv {
                return false;
            }
            let theirs = ancestors(self.lines, o.line);
            // The cases of a `mod` with the name of the test look as its rows.
            if self.form == Form::Fn
                && let (Some(path), Some(chain), Some(target)) =
                    (self.module_path(), &theirs, &self.target)
            {
                let mut other = target.module.clone();
                other.extend(chain.iter().map(|t| t.text.clone()));
                other.push(o.test.name.clone());
                let other = other.join("::");
                if case.starts_with(&format!("{path}::")) && case.starts_with(&other) {
                    return other.len() > path.len();
                }
            }
            if o.test.name != self.test.name {
                return false;
            }
            let (Some(mine), Some(theirs)) = (mine, &theirs) else {
                return true;
            };
            let titles = |c: &[Title]| c.iter().map(|t| t.text.clone()).collect::<Vec<_>>();
            let (mine, theirs) = (titles(mine), titles(theirs));
            // A newer Surefire writes the outer class for a nested one.
            mine == theirs || (self.form == Form::Method && theirs.starts_with(&mine))
        })
    }
}

/// The form of the definitions that a row with a row tail or a project prefix
/// reads.
fn row_form(row: Row) -> Form {
    match row {
        Row::Nextest => Form::Fn,
        Row::Go => Form::Func,
        Row::Jvm | Row::Phpunit => Form::Method,
        Row::PathTitles | Row::FileTitles | Row::JestDefaults => Form::Call,
        Row::PytestXunit1 | Row::PytestXunit2 | Row::Lines | Row::Unlisted => Form::Def,
    }
}

/// The index of the cases of the reports, for the bind of many tags. It holds,
/// once for all tags, the cases by the keys of their names, what the test files
/// name, and each manifest that a bind read. The cost of a bind is then the cost
/// of its test, and not the number of cases times the number of test files.
pub struct Binder<'a> {
    cases: &'a [Case],
    /// A key of a name (`case_keys`), to the cases that have it.
    index: Index,
    /// The cases with a head of more words than their keys hold.
    wordy: Vec<u32>,
    /// The cases with a name that is too long to read (`is_long`).
    long: Vec<u32>,
    names: Names<'a>,
    /// A manifest path, to its text: each is read once.
    manifests: RefCell<HashMap<String, Option<Rc<str>>>>,
    /// The folder of a `Cargo.toml`, to its package: each is parsed once.
    packages: RefCell<HashMap<String, Option<String>>>,
    /// A `classname` of Go, to the number of folders that it names.
    go_counts: RefCell<HashMap<(String, bool), usize>>,
}

/// Reads a file of the working tree, by its path from the root of the
/// repository.
type Read<'r> = &'r dyn Fn(&str) -> Option<String>;

impl<'a> Binder<'a> {
    pub fn new(cases: &'a [Case], test_files: &'a [String]) -> Binder<'a> {
        let mut index = Index::default();
        let mut wordy = Vec::new();
        let mut long = Vec::new();
        for (i, c) in cases.iter().enumerate() {
            let i = i as u32;
            let name = c.name.as_str();
            if is_long(name) {
                long.push(i);
            }
            let (keys, more) = case_keys(name);
            if more {
                wordy.push(i);
            }
            for k in keys {
                index.add(k, i);
            }
        }
        Binder {
            cases,
            index,
            wordy,
            long,
            names: Names::new(test_files),
            manifests: RefCell::new(HashMap::new()),
            packages: RefCell::new(HashMap::new()),
            go_counts: RefCell::new(HashMap::new()),
        }
    }

    fn lookup(&self, key: &str) -> &[u32] {
        self.index.get(key)
    }

    /// The nearest file of the name `file` at or above the folder of `path`: its
    /// folder and its text.
    fn nearest(&self, path: &str, file: &str, read: Read) -> Option<(String, Rc<str>)> {
        let mut dir = folder_of(path);
        loop {
            let at = if dir.is_empty() {
                file.to_string()
            } else {
                format!("{dir}/{file}")
            };
            let text = self
                .manifests
                .borrow_mut()
                .entry(at.clone())
                .or_insert_with(|| read(&at).map(Rc::from))
                .clone();
            if let Some(text) = text {
                return Some((dir.to_string(), text));
            }
            if dir.is_empty() {
                return None;
            }
            dir = folder_of(dir);
        }
    }

    /// `[package] name` of the nearest `Cargo.toml`.
    fn cargo_package(&self, path: &str, read: Read) -> Option<String> {
        let (dir, text) = self.nearest(path, "Cargo.toml", read)?;
        self.packages
            .borrow_mut()
            .entry(dir)
            .or_insert_with(|| {
                let table: toml::Table = text.parse().ok()?;
                let name = table.get("package")?.get("name")?.as_str()?;
                Some(name.to_string())
            })
            .clone()
    }

    /// The `module` line of the nearest `go.mod`.
    fn go_module(&self, path: &str, read: Read) -> Option<GoModule> {
        let (dir, text) = self.nearest(path, "go.mod", read)?;
        let module = text.lines().find_map(|l| {
            let rest = l.trim().strip_prefix("module ")?;
            let rest = rest.split("//").next().unwrap_or("").trim();
            Some(rest.trim_matches('"').to_string())
        })?;
        (!module.is_empty()).then_some(GoModule { dir, path: module })
    }

    /// The number of folders of Go test files that a `classname` can name.
    fn go_folders_named(&self, classname: &str, with_module: bool, read: Read) -> usize {
        let key = (classname.to_string(), with_module);
        if let Some(n) = self.go_counts.borrow().get(&key) {
            return *n;
        }
        let n = self
            .names
            .go_folders
            .iter()
            .filter(|folder| {
                let module = if with_module {
                    let path = if folder.is_empty() {
                        "x.go".to_string()
                    } else {
                        format!("{folder}/x.go")
                    };
                    self.go_module(&path, read)
                } else {
                    None
                };
                match &module {
                    // An import path names one folder. A shorter `classname`
                    // can also be the short form of another folder.
                    Some(m) if classname == m.import(folder) => true,
                    Some(m) => {
                        let rel = m.relative(folder);
                        !classname.starts_with(&m.path)
                            && (rel == classname || rel.rsplit('/').next() == Some(classname))
                    }
                    None => go_field(classname, folder, None) != Field::Differs,
                }
            })
            .count();
        self.go_counts.borrow_mut().insert(key, n);
        n
    }

    /// The number of crates whose Rust target can write the binary id of the
    /// tag.
    fn crates_named(&self, target: &Target, package: Option<&str>, read: Read) -> usize {
        let Some(roots) = self.names.rust.get(&target.key()) else {
            return 0;
        };
        let Some(package) = package else {
            return roots.len();
        };
        roots
            .iter()
            .filter(|root| {
                let path = if root.is_empty() {
                    "x.rs".to_string()
                } else {
                    format!("{root}/x.rs")
                };
                self.cargo_package(&path, read).as_deref() == Some(package)
            })
            .count()
    }

    /// True if the field that names the test file names more than one file of
    /// the path list.
    fn ambiguous(&self, id: &Identity, row: Row, case: &Case, read: Read) -> bool {
        let file = case.file.as_deref().map(report_path);
        let cn = case.classname.as_deref().unwrap_or("");
        let n = match row {
            Row::PytestXunit1 | Row::Phpunit | Row::FileTitles => {
                file.map_or(0, |f| self.names.files_named(&f))
            }
            Row::PathTitles => self.names.files_named(&report_path(cn)),
            Row::PytestXunit2 => {
                // The part of the `classname` that names the module.
                let classes = id.classes().unwrap_or_default();
                let stem = if classes.is_empty() {
                    cn
                } else {
                    cn.strip_suffix(&classes)
                        .map_or("", |s| s.trim_end_matches('.'))
                };
                self.names.dotted_named(id.path, stem)
            }
            Row::Nextest => id
                .target
                .as_ref()
                .map_or(0, |t| self.crates_named(t, id.cargo.as_deref(), read)),
            Row::Go => self.go_folders_named(cn, id.go.is_some(), read),
            Row::Jvm => {
                let classes = id.classes().unwrap_or_default();
                let top = classes.split('.').next().unwrap_or("");
                match &id.package {
                    _ if family(id.path) == "cs" => self.names.stems.get(top).copied().unwrap_or(0),
                    Package::Line(p) => self.names.dotted_named(id.path, &format!("{p}.{top}")),
                    _ => self
                        .names
                        .dotted_named(id.path, &cn.replace(['$', '+'], ".")),
                }
            }
            Row::Lines | Row::Unlisted | Row::JestDefaults => 0,
        };
        n > 1
    }

    /// What case `i` is for the identity of the tag: `None` for another test.
    fn judge(&self, i: usize, id: &Identity, read: Read) -> Option<Judged<'a>> {
        let case: &'a Case = &self.cases[i];
        let row = row_of(id.form, case);
        // The file field comes first: it is cheap, and it leaves out most cases
        // of a name that many files have.
        let file = id.file_field(row, case);
        if file == Field::Differs || !id.names(&case.name) {
            return None;
        }
        let heads = id.heads(row);
        let hit = id.hit(row, &case.name);
        let full_name = !heads.is_empty();
        if full_name && hit.is_none() && !matches!(row, Row::Lines | Row::Unlisted) {
            return None;
        }
        let name = hit.unwrap_or_else(|| {
            let short = without_project(&case.name);
            if id.form == Form::Call && id.lookups().iter().any(|l| l == short) {
                short
            } else {
                &case.name
            }
        });
        let result = if matches!(row, Row::Lines | Row::Unlisted) {
            let built = id.groups().is_some() && id.plain && id.display.is_none();
            Err(if built { Why::ByName } else { Why::NoIdentity })
        } else if !id.built(row) {
            Err(Why::NoIdentity)
        } else if file == Field::Unread || !full_name || row == Row::JestDefaults {
            Err(Why::ByName)
        } else if id.groups().is_some_and(|c| c.iter().any(|t| t.partial))
            || id.twin(&case.name, name)
            || self.ambiguous(id, row, case, read)
        {
            Err(Why::NotUnique)
        } else {
            Ok(match row {
                Row::PytestXunit1 | Row::Phpunit | Row::FileTitles => Fit::File,
                _ => Fit::Classname,
            })
        };
        let mut judged = Judged::new(case, name, result);
        // Playwright runs one test once for each of its projects. With no
        // group, only the name of the test shows in its title, as in each other
        // runner, so a template of the file can be the second case.
        judged.repeats = matches!(row, Row::PathTitles | Row::FileTitles)
            && (name.contains(" \u{203a} ")
                || (!id.templates && id.groups().is_some_and(<[_]>::is_empty)));
        Some(judged)
    }

    /// The tie of a case that `case:` names to the tag file: the field that
    /// ties it, or why it is weak.
    fn tie(&self, file: &TagFile, case: &Case, read: Read) -> Result<Fit, Why> {
        let path = file.path;
        let unique = |n: usize, fit: Fit| {
            if n > 1 { Err(Why::NotUnique) } else { Ok(fit) }
        };
        if let Some(of_case) = &case.file {
            return unique(self.names.files_named(&report_path(of_case)), Fit::File);
        }
        let Some(cn) = case.classname.as_deref().filter(|cn| *cn != case.name) else {
            return Err(Why::ByName);
        };
        if path.ends_with(".rs") {
            let target = rust_target(path, &self.names.whole).ok_or(Why::ByName)?;
            let cargo = self.cargo_package(path, read);
            let (package, part) = binary_id(cn);
            let named = match &cargo {
                Some(name) => name == package,
                None => matches!(target.part, Part::Test(_)),
            };
            if !named || part_field(&target, part.as_ref(), cargo.as_deref()) != Field::Same {
                return Err(Why::ByName);
            }
            return unique(
                self.crates_named(&target, cargo.as_deref(), read),
                Fit::Classname,
            );
        }
        if path.ends_with(".go") {
            let module = self.go_module(path, read);
            if go_field(cn, folder_of(path), module.as_ref()) != Field::Same {
                return Err(Why::ByName);
            }
            return unique(
                self.go_folders_named(cn, module.is_some(), read),
                Fit::Classname,
            );
        }
        if cn.contains("::") {
            return Err(Why::ByName);
        }
        if cn.contains('/') {
            let cn = report_path(cn);
            return if same_file(&cn, path) {
                unique(self.names.files_named(&cn), Fit::Classname)
            } else {
                Err(Why::ByName)
            };
        }
        let cn = cn.replace(['$', '+', '\\'], ".");
        let own = dotted(path);
        if ends_in_segments(&own, &cn) {
            return unique(self.names.dotted_named(path, &cn), Fit::Classname);
        }
        // The classes after the module or the package must be classes of the
        // tag file.
        let mut stem = cn.as_str();
        let mut top = None;
        while let Some((head, last)) = stem.rsplit_once('.') {
            if !file.classes.contains(last) {
                break;
            }
            (stem, top) = (head, Some(last));
        }
        let Some(top) = top else {
            return Err(Why::ByName);
        };
        if ends_in_segments(&own, stem) {
            unique(self.names.dotted_named(path, stem), Fit::Classname)
        } else if file.package.as_deref() == Some(stem) {
            unique(
                self.names.dotted_named(path, &format!("{stem}.{top}")),
                Fit::Classname,
            )
        } else {
            Err(Why::ByName)
        }
    }

    /// The cases that `case:` names: the cases with this whole name. A case
    /// with the `file` of another file is another test. Each other case is in
    /// the set, and it is own only when a full field ties it to the tag file.
    fn named(&self, path: &str, lines: &[&str], value: &str, read: Read) -> Vec<Judged<'a>> {
        let file = TagFile {
            path,
            classes: lines
                .iter()
                .filter_map(|l| group_title(l.trim()).flatten())
                .filter(|t| t.kind == GroupKind::Class)
                .map(|t| t.text)
                .collect(),
            package: package_line(lines),
        };
        let mut out: Vec<Judged> = self
            .lookup(value)
            .iter()
            .map(|i| &self.cases[*i as usize])
            .filter(|c| c.name == value)
            .filter(|c| {
                c.file
                    .as_deref()
                    .is_none_or(|f| same_file(&report_path(f), path))
            })
            .map(|c| {
                let mut judged = Judged::new(c, &c.name, self.tie(&file, c, read));
                // The projects of Playwright run one test more than once.
                judged.repeats = judged.own.is_some() && c.name.contains(" \u{203a} ");
                judged
            })
            .collect();
        // The name does not tell 2 own cases of 2 identities apart.
        let own: HashSet<_> = out
            .iter()
            .filter(|j| j.own.is_some())
            .map(Judged::key)
            .collect();
        if own.len() > 1 {
            for j in out.iter_mut().filter(|j| j.own.is_some()) {
                j.own = None;
                j.why = Some(Why::NotUnique);
            }
        }
        out
    }

    /// The cases that can name the test: the cases with its name, or with the
    /// last segment of its name, as a key. The index is wide, and
    /// `Identity::names` decides.
    fn named_by(&self, test: &TestName) -> Vec<usize> {
        let name = test.name.as_str();
        let last = SEPS
            .iter()
            .filter_map(|s| name.rfind(s).map(|i| i + s.len()))
            .max()
            .map_or(name, |i| &name[i..]);
        let mut out: Vec<usize> = self
            .lookup(name)
            .iter()
            .chain(self.lookup(last))
            .map(|i| *i as usize)
            .collect();
        if name.matches(' ').count() >= LONG_SEPS {
            out.extend(self.wordy.iter().map(|i| *i as usize));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The identity of the test that `scan` found under a tag.
    fn identity<'p>(
        &self,
        path: &'p str,
        lines: &'p [&'p str],
        scan: &Scan<'p>,
        test: TestName,
        suites: &dyn Fn(&str, &str) -> Vec<String>,
        read: Read,
    ) -> Identity<'p> {
        let form = form_of(&test, scan.text);
        let chain = ancestors(lines, scan.line);
        let method = form == Form::Method;
        let package = if method {
            package_of(lines, chain.as_deref())
        } else {
            Package::Absent
        };
        let display = if method {
            display_name(lines, scan.first, scan.line)
        } else {
            None
        };
        let funcs = test
            .recv
            .as_ref()
            .map(|ty| suites(ty, &test.name))
            .unwrap_or_default();
        let rust = form == Form::Fn && path.ends_with(".rs");
        let target = rust.then(|| rust_target(path, &self.names.whole)).flatten();
        let cargo = rust.then(|| self.cargo_package(path, read)).flatten();
        let go = (form == Form::Func && path.ends_with(".go"))
            .then(|| self.go_module(path, read))
            .flatten();
        let templates = form == Form::Call
            && lines.iter().any(|l| {
                let t = l.trim();
                !is_comment(t)
                    && matches!(
                        classify(strip_modifiers(t), false),
                        Some(Under::Template(_))
                    )
            });
        let mut identity = Identity {
            path,
            templates,
            pytest: None,
            plain: !test.call || test.bt || plain_call(scan.text, &test.name),
            form,
            chain,
            funcs,
            package,
            display,
            target,
            cargo,
            go,
            others: tests_of(lines)
                .into_iter()
                .filter(|f| f.line != scan.line)
                .collect(),
            lines,
            test,
        };
        identity.pytest = identity.classes().map(|classes| {
            let module = dotted(path);
            if classes.is_empty() {
                module
            } else {
                format!("{module}.{classes}")
            }
        });
        identity
    }

    /// Binds one tag to the cases of the reports. `lines` are the lines of the
    /// test file, and `tag` is the index of the line of the tag. With `case`,
    /// the tag binds to the cases that have this name. Else it binds to the test
    /// under it (design 0.8.1, D82). It also gives the bind: the test, the fit,
    /// the strength and the reason for no case. The suite functions of Go are
    /// those of the file of the tag, and no manifest is read.
    #[cfg(test)]
    pub fn bind(
        &self,
        path: &str,
        lines: &[&str],
        tag: usize,
        case: Option<&str>,
    ) -> (Proof, Bind) {
        // The suite function is in the file of the tag only.
        let suites = |ty: &str, method: &str| suite_funcs([lines.join("\n").as_str()], ty, method);
        self.bind_with(path, lines, tag, case, &suites, &|_| None)
    }

    /// Like `bind`. `suites` gives the functions that run the Go suite of a type
    /// for a method (`suite_funcs` over the test files of the package), and
    /// `read` gives the text of a manifest.
    ///
    /// The rule: a case that names the test is another test only when a field
    /// that the report and the built identity both hold in full differs. It is
    /// an own case when each field is equal and the identity is unique. Each
    /// other case stays in the set as a weak link. No case leaves the set
    /// because another case fits better.
    pub fn bind_with(
        &self,
        path: &str,
        lines: &[&str],
        tag: usize,
        case: Option<&str>,
        suites: &dyn Fn(&str, &str) -> Vec<String>,
        read: &dyn Fn(&str) -> Option<String>,
    ) -> (Proof, Bind) {
        let mut bind = Bind::default();
        let mut ident: Option<Identity> = None;
        let set: Vec<Judged> = match case {
            Some("") => {
                // A broken `case:` binds no case, and not the test under the tag.
                bind.by = By::Case;
                return (Proof::Unbound, bind.unbound(Reason::NoCase));
            }
            Some(name) => {
                bind.by = By::Case;
                bind.test = Some(name.to_string());
                let set = self.named(path, lines, name, read);
                if set.is_empty() {
                    bind.near = self.near(name);
                }
                set
            }
            None => {
                let scan = if tag < lines.len() {
                    under_tag_line(lines, tag)
                } else {
                    Scan::none(tag)
                };
                match scan.under.clone() {
                    Under::NoTest(line) => {
                        bind.line = line;
                        return (Proof::Unbound, bind.unbound(Reason::NoTest));
                    }
                    Under::Group => return (Proof::Unbound, bind.unbound(Reason::Group)),
                    Under::Template(name) => {
                        bind.test = (!name.is_empty()).then_some(name);
                        return (Proof::Unbound, bind.unbound(Reason::Template));
                    }
                    Under::Test(test) => {
                        bind.test = Some(test.name.clone());
                        let id = self.identity(path, lines, &scan, test, suites, read);
                        let mut cand = self.named_by(&id.test);
                        for key in id.lookups() {
                            cand.extend(self.lookup(&key).iter().map(|i| *i as usize));
                        }
                        cand.sort_unstable();
                        cand.dedup();
                        let set = cand
                            .into_iter()
                            .filter_map(|i| self.judge(i, &id, read))
                            .collect();
                        ident = Some(id);
                        set
                    }
                }
            }
        };
        if set.is_empty() {
            // A case with a long name has not each key of its name.
            let long = ident.as_ref().is_some_and(|id| {
                self.long.iter().any(|&i| {
                    let c = &self.cases[i as usize];
                    c.name.contains(id.test.name.as_str())
                        && id.file_field(row_of(id.form, c), c) != Field::Differs
                })
            });
            let reason = if long {
                Reason::LongName
            } else {
                Reason::NoCase
            };
            return (Proof::Unbound, bind.unbound(reason));
        }
        bind.cases = set.len();
        let owned = |fit: Fit| set.iter().any(|j| j.own == Some(fit));
        bind.fit = Some(if owned(Fit::File) {
            Fit::File
        } else if owned(Fit::Classname) {
            Fit::Classname
        } else {
            Fit::Name
        });
        // The worst cause of the weak links: SD621 names it.
        bind.why = [Why::NoIdentity, Why::NotUnique, Why::ByName]
            .into_iter()
            .find(|why| set.iter().any(|j| j.why == Some(*why)));
        bind.by_name = bind.why.is_some();
        let (proof, shared) = decide(&set);
        bind.shared = shared;
        bind.case = match &proof {
            Proof::Passed(n, _) | Proof::Skipped(n) | Proof::Failed(n) => Some(n.clone()),
            Proof::Unbound => None,
        };
        (proof, bind)
    }

    /// The names of the cases that have `value` as a key: the cases that a
    /// `case:` with a part of a name meant. At most 3.
    fn near(&self, value: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .lookup(value)
            .iter()
            .map(|i| &self.cases[*i as usize].name)
            .filter(|name| case_keys(name).0.contains(&value))
            .cloned()
            .collect();
        names.sort();
        names.dedup();
        names.truncate(3);
        names
    }
}

/// The proof of the set, and true when the worst of more than one identity
/// decided. A case that is own and comes more than once, once in each report or
/// once for each project of Playwright, is one test that ran again: it passed
/// when a run passed and none failed. Each other repeat is a case of its own.
/// The worst case then decides: a failed case, a skipped case, a passed case.
fn decide(set: &[Judged]) -> (Proof, bool) {
    let mut order: Vec<Vec<&Judged>> = Vec::new();
    let mut at = HashMap::new();
    for j in set {
        let k = *at.entry(j.key()).or_insert_with(|| {
            order.push(Vec::new());
            order.len() - 1
        });
        order[k].push(j);
    }
    // One result for each identity, or for each of its repeats.
    let mut results: Vec<(Outcome, &str)> = Vec::new();
    for runs in &order {
        let reports: HashSet<&str> = runs.iter().map(|j| j.case.report.as_str()).collect();
        let one = runs.iter().all(|j| j.own.is_some())
            && (reports.len() == runs.len() || runs.iter().all(|j| j.repeats));
        if one {
            let run = [Outcome::Fail, Outcome::Pass, Outcome::Skip]
                .into_iter()
                .find_map(|o| runs.iter().find(|j| j.case.outcome == o));
            results.extend(run.map(|j| (j.case.outcome, j.case.name.as_str())));
        } else {
            results.extend(runs.iter().map(|j| (j.case.outcome, j.case.name.as_str())));
        }
    }
    // 2 reports can each hold the case once: that is one case, also when it is
    // weak.
    let alone = order.len() == 1
        && order[0]
            .iter()
            .map(|j| j.case.report.as_str())
            .collect::<HashSet<_>>()
            .len()
            == order[0].len();
    let shared = results.len() > 1 && !alone;
    let first = |o: Outcome| results.iter().find(|r| r.0 == o).map(|r| r.1.to_string());
    let proof = if let Some(n) = first(Outcome::Fail) {
        Proof::Failed(n)
    } else if let Some(n) = first(Outcome::Skip) {
        Proof::Skipped(n)
    } else if let Some(n) = first(Outcome::Pass) {
        let (name, runs) = least_passes(set).unwrap_or((n, 0));
        Proof::Passed(name, runs)
    } else {
        Proof::Unbound
    };
    (proof, shared)
}

/// The identity of the set that passed least, with its passes: the reports in
/// which it passed (D2). An identity that never passed does not count. `None`
/// when no case passed.
fn least_passes(set: &[Judged]) -> Option<(String, usize)> {
    let mut passes: HashMap<_, (&str, HashSet<&str>)> = HashMap::new();
    for j in set.iter().filter(|j| j.case.outcome == Outcome::Pass) {
        passes
            .entry(j.key())
            .or_insert_with(|| (j.case.name.as_str(), HashSet::new()))
            .1
            .insert(j.case.report.as_str());
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
            cv.bind = Some(Bind {
                by: if cv.case.is_some() {
                    By::Case
                } else {
                    By::Definition
                },
                test: cv.case.clone().filter(|c| !c.is_empty()),
                ..Bind::default().unbound(Reason::NoReport)
            });
        }
        return;
    }
    let binder = Binder::new(&reads.cases, &reads.test_files);
    let mut texts: HashMap<String, Option<String>> = HashMap::new();
    for cv in covers.iter_mut().filter(|c| !c.partial) {
        // The stamp covers the code tree only (design 0.8.1, D85).
        if !reads.in_tree.contains(&cv.path) {
            cv.proof = Some(Proof::Unbound);
            cv.bind = Some(Bind {
                by: if cv.case.is_some() {
                    By::Case
                } else {
                    By::Definition
                },
                test: cv.case.clone().filter(|c| !c.is_empty()),
                ..Bind::default().unbound(Reason::OutsideTree)
            });
            continue;
        }
        let text = texts.entry(cv.path.clone()).or_insert_with(|| {
            std::fs::read(root.join(&cv.path))
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        });
        let lines: Vec<&str> = text
            .as_deref()
            .map(|t| t.lines().collect())
            .unwrap_or_default();
        // The functions that run a Go suite are in the test files of the package.
        let folder = cv.path.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        let suites = |ty: &str, method: &str| -> Vec<String> {
            let texts: Vec<String> = reads
                .test_files
                .iter()
                .filter(|f| {
                    f.ends_with(".go") && f.rsplit_once('/').map_or("", |(d, _)| d) == folder
                })
                .filter_map(|f| std::fs::read(root.join(f)).ok())
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .collect();
            suite_funcs(texts.iter().map(String::as_str), ty, method)
        };
        let read = |p: &str| std::fs::read_to_string(root.join(p)).ok();
        let (proof, bind) = binder.bind_with(
            &cv.path,
            &lines,
            (cv.line as usize).saturating_sub(1),
            cv.case.as_deref(),
            &suites,
            &read,
        );
        cv.proof = Some(proof);
        cv.bind = Some(bind);
    }
}

/// The bind of a tag when no report is read: the test under the tag, or the
/// reason why it has none (design 0.8.1, D82 items 1 to 3). `show` gives it.
pub fn scan_bind(lines: &[&str], tag: usize, case: Option<&str>) -> Bind {
    let mut bind = Bind::default();
    if let Some(name) = case {
        bind.by = By::Case;
        if name.is_empty() {
            bind.reason = Some(Reason::NoCase);
        } else {
            bind.test = Some(name.to_string());
        }
        return bind;
    }
    let under = if tag < lines.len() {
        under_tag(lines, tag)
    } else {
        Under::NoTest(None)
    };
    match under {
        Under::Test(t) => bind.test = Some(t.name),
        Under::Template(name) => {
            bind.test = (!name.is_empty()).then_some(name);
            bind.reason = Some(Reason::Template);
        }
        Under::Group => bind.reason = Some(Reason::Group),
        Under::NoTest(line) => {
            bind.line = line;
            bind.reason = Some(Reason::NoTest);
        }
    }
    bind
}

/// The result of a bind in `--json` output: `not-read` when no report is read.
fn result_key(proof: Option<&Proof>) -> &'static str {
    match proof {
        None => "not-read",
        Some(Proof::Passed(..)) => "passed",
        Some(Proof::Failed(_)) => "failed",
        Some(Proof::Skipped(_)) => "skipped",
        Some(Proof::Unbound) => "unbound",
    }
}

/// The bracket of a tag in the evidence line of `show` (design 0.8.1, D82,
/// "Formats"). Without a report (`proof` is `None`) it gives the test under the
/// tag, or its reason. With a report it gives the result and the case, or the
/// reason why the tag binds no case.
pub fn bracket(bind: &Bind, proof: Option<&Proof>) -> String {
    let name = |n: &str| {
        if bind.by_name {
            format!("{n}, by name")
        } else {
            n.to_string()
        }
    };
    match (proof, bind.reason) {
        (None, Some(r)) => format!("[no test: {}]", r.key()),
        (None, None) => format!("[test {}]", bind.test.as_deref().unwrap_or("")),
        (Some(_), Some(r)) => format!("[unbound: {}]", r.key()),
        (Some(Proof::Passed(n, _)), None) => format!("[passed {}]", name(n)),
        (Some(Proof::Failed(n)), None) => format!("[failed {}]", name(n)),
        (Some(Proof::Skipped(n)), None) => format!("[skipped {}]", name(n)),
        (Some(Proof::Unbound), None) => "[unbound: no-case]".to_string(),
    }
}

/// The object of one tag in the field `binds` of `show --json` and of
/// `check --reports --json` (design 0.8.1, D82, "Formats"). `lead` holds the
/// fields that come first.
pub fn bind_object(
    lead: crate::json::Obj,
    tag: &str,
    bind: &Bind,
    proof: Option<&Proof>,
) -> String {
    lead.str("tag", tag)
        .opt_str("test", bind.test.as_deref())
        .str("by", bind.by.key())
        .opt_str("case", bind.case.as_deref())
        .num("cases", bind.cases)
        .opt_str("fit", bind.fit.map(Fit::key))
        .bool("shared", bind.shared)
        .str("result", result_key(proof))
        .opt_str("reason", bind.reason.map(Reason::key))
        .end()
}

/// The text of the reason why a tag binds no case (design 0.8.1, D82, "Formats").
/// `at` is `<path>:<line>` of the tag.
fn unbound_text(at: &str, bind: &Bind) -> String {
    let name = bind.test.as_deref().unwrap_or("");
    match bind.reason {
        Some(Reason::NoTest) => {
            let place = match bind.line {
                Some(n) => format!("line {n} is not a test"),
                None => "no code line follows it".to_string(),
            };
            format!(
                "the tag at {at} stands above no test ({place}); put the tag on the line above the test, or add `case:\"<name>\"`"
            )
        }
        Some(Reason::Group) => format!(
            "the tag at {at} stands above a group, not a test; put the tag above one test, or add `case:\"<name>\"`"
        ),
        Some(Reason::Template) => format!(
            "the test under the tag at {at} has a template name; add `case:\"<name>\"` with one name of the report"
        ),
        Some(Reason::NoCase) if bind.by == By::Case && bind.test.is_none() => format!(
            "the `case:` of the tag at {at} is not valid: write `case:\"<name>\"` with the whole name of a case between 2 double quotes, and no `\"` inside the name"
        ),
        Some(Reason::NoCase) if bind.by == By::Case => {
            let near = if bind.near.is_empty() {
                String::new()
            } else {
                format!(
                    ", and the report has {}",
                    bind.near
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            format!(
                "no case of a fresh report has the name `{name}`, the `case:` of the tag at {at}; `case:` needs the whole name of a case{near}"
            )
        }
        Some(Reason::NoCase) => format!(
            "no case of a fresh report names the test `{name}` of the tag at {at} in that file; check the name, or add `case:\"<name>\"`"
        ),
        Some(Reason::LongName) => format!(
            "no case of a fresh report names the test `{name}` of the tag at {at}, and a case of the reports has a name that is too long to read (more than 256 characters, or more than 32 separators); spec-dna binds such a case only by its whole name, so add `case:\"<name>\"`"
        ),
        Some(Reason::OutsideTree) => format!(
            "the tag at {at} is in a file that is not in the code tree (git does not track it, or `[evidence] tree-exclude` matches it), so the stamp does not cover it; `git add` the file, then run the tests and `spec-dna evidence stamp` again"
        ),
        _ => format!(
            "no case of a fresh report binds to the tag at {at}: a missing, stale or unreadable report has no case (SD613 to SD615), or the tag is not above its test; put the tag above the test, or add `case:<name>` to the tag"
        ),
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
    /// The detail of each bind of `proofs`, in the same order.
    pub binds: HashMap<(u32, usize), Vec<Bind>>,
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
                            ev.binds
                                .entry((d, r))
                                .or_default()
                                .push(cv.bind.clone().unwrap_or_default());
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
        let binds = self.binds.get(&(di, r));
        // The text of a failed or skipped case. A shared set names its tests.
        let text = |i: usize, n: &str, plain: String, what: &str| -> String {
            match binds.and_then(|b| b.get(i)).filter(|b| b.shared) {
                Some(b) => {
                    let at = &proofs[i].0;
                    let path = at.rsplit_once(':').map_or(at.as_str(), |x| x.0);
                    let name = b.test.as_deref().unwrap_or(n);
                    format!(
                        "the tests of the name `{name}` in {path} share their cases, and the case `{n}` {what}"
                    )
                }
                None => plain,
            }
        };
        if let Some((i, n)) = proofs.iter().enumerate().find_map(|(i, p)| match &p.1 {
            Proof::Failed(n) => Some((i, n)),
            _ => None,
        }) {
            let plain = format!("the bound case `{n}` failed in the report");
            return Some(text(i, n, plain, "failed"));
        }
        if proofs.iter().any(|p| matches!(p.1, Proof::Passed(..))) {
            return None;
        }
        if let Some((i, n)) = proofs.iter().enumerate().find_map(|(i, p)| match &p.1 {
            Proof::Skipped(n) => Some((i, n)),
            _ => None,
        }) {
            let plain = format!(
                "the bound case `{n}` was skipped in the report, and a skipped case is no pass"
            );
            return Some(text(i, n, plain, "was skipped"));
        }
        let at = &proofs[0].0;
        let bind = binds.and_then(|b| b.first()).cloned().unwrap_or_default();
        Some(unbound_text(at, &bind))
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
        Binder::new(cases, &files)
            .bind(path, &lines, tag, case_name)
            .0
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
        // No name, a space after the colon, and a quote that stops before the
        // end of the word are not valid: the name is empty.
        assert_eq!(t("# covers: a#b case:")[0].case.as_deref(), Some(""));
        assert_eq!(t("# covers: a#b case: \"x\"")[0].case.as_deref(), Some(""));
        assert_eq!(
            t("# covers: a#b case:\"a\"-b\"")[0].case.as_deref(),
            Some("")
        );
        assert_eq!(
            t("# covers: a#b case:\"a\" b\"")[0].case.as_deref(),
            Some("")
        );
        assert_eq!(t("# covers: a#b case:\"\"")[0].case.as_deref(), Some(""));
        // A quote that is not closed, or a `"` inside the name, is no name: the
        // tag gets the empty name, which binds no case. It never falls back to
        // the test under the tag.
        assert_eq!(t("# covers: a#b case:\"open")[0].case.as_deref(), Some(""));
        assert_eq!(
            t("# covers: a#b case:\"say \"hi\" now\"")[0]
                .case
                .as_deref(),
            Some("")
        );
        assert_eq!(
            t("# covers: a#b case:\"fine\" */")[0].case.as_deref(),
            Some("fine")
        );
        assert_eq!(
            t("// covers: a#b case:\"fine\", c#d")[0].case.as_deref(),
            Some("fine")
        );
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
        // The `mod tests` is in the fixture now: the module path is a full field.
        let text = "mod tests {\n    // covers: requirements/valves#close\n    #[test]\n    fn closes() {\n        assert!(true);\n    }\n}\n";
        // The shape of a unit test of a library (E6): the `classname` is the
        // crate, and the name has the module path.
        let cases = [case(
            "valves::tests::closes",
            Some("app"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof("src/valves.rs", text, None, &cases, &[]),
            Proof::Passed("valves::tests::closes".into(), 1)
        );
        // The module path is the one of the file: another module binds nothing.
        let cases = [case(
            "pumps::tests::closes",
            Some("app"),
            None,
            Outcome::Pass,
        )];
        assert_eq!(
            proof("src/valves.rs", text, None, &cases, &[]),
            Proof::Unbound
        );
    }

    #[test]
    fn a_jest_string_literal_binds() {
        // The `describe` is in the fixture now: the name of jest-junit is a full
        // field, the titles and the title.
        let text = "describe('Valves', () => {\n  // covers: requirements/valves#close\n  it('closes on timeout', () => {\n    expect(1).toBe(1);\n  });\n});\n";
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

    /// The bind of the first tag of `text` in the file `path`, to the cases.
    fn bind_of(
        path: &str,
        text: &str,
        case_name: Option<&str>,
        cases: &[Case],
        files: &[&str],
    ) -> (Proof, Bind) {
        let lines: Vec<&str> = text.lines().collect();
        let tag = lines.iter().position(|l| l.contains("covers:")).unwrap();
        let files: Vec<String> = files.iter().map(|f| f.to_string()).collect();
        Binder::new(cases, &files).bind(path, &lines, tag, case_name)
    }

    // The fit by name only went: the `classname` of pytest is a full field, so a
    // case with another one is another test.
    #[test]
    fn a_classname_that_names_another_module_is_another_test() {
        let text = "# covers: requirements/valves#close\ndef test_init():\n    pass\n";
        let cases = [case(
            "test_init",
            Some("unrelated.name"),
            None,
            Outcome::Pass,
        )];
        let (proof, bind) = bind_of("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]);
        assert_eq!(proof, Proof::Unbound);
        assert_eq!(bind.reason, Some(Reason::NoCase));
    }

    /// The table of the file rule (design 0.8.1, D82 item 6): the test file of
    /// the tag, the other test files, the `classname` and the `file` of the
    /// case, and how the case fits.
    #[test]
    fn the_file_rule_fits_the_case_of_each_runner_to_its_test_file() {
        let py = "# covers: a#b\ndef test_x():\n    pass\n";
        let rs = "// covers: a#b\n#[test]\nfn test_x() {\n}\n";
        let go = "// covers: a#b\nfunc TestX(t *testing.T) {\n}\n";
        let java = "package com.example;\n\nclass ValveTest {\n    // covers: a#b\n    @Test\n    void testX() {\n    }\n}\n";
        let nested = "package com.example;\n\nclass ValveTest {\n    class Inner {\n        // covers: a#b\n        @Test\n        void testX() {\n        }\n    }\n}\n";
        let dotnet = "public class ValveTest {\n    // covers: a#b\n    [Fact] public void testX() {\n    }\n}\n";
        let py_class = "class TestA:\n    # covers: a#b\n    def test_x(self):\n        pass\n";
        let js = "describe('V', () => {\n  // covers: a#b\n  it('x', () => {\n  });\n});\n";
        type Row<'a> = (
            &'a str,
            &'a str,
            &'a [&'a str],
            &'a str,
            Option<&'a str>,
            Option<&'a str>,
            Option<Fit>,
        );
        // (tag file, text, all test files, name, classname, file, the fit)
        let rows: Vec<Row> = vec![
            // pytest: the dotted path, with a class after the module.
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py", "tests/test_b.py"],
                "test_x",
                Some("tests.test_a"),
                None,
                Some(Fit::Classname),
            ),
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py", "tests/test_b.py"],
                "test_x",
                Some("tests.test_b"),
                None,
                None,
            ),
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py"],
                "test_x",
                Some("tests.test_a.TestA"),
                None,
                // Changed: the class chain is a full field.
                None,
            ),
            (
                "tests/test_a.py",
                py_class,
                &["tests/test_a.py"],
                "test_x",
                Some("tests.test_a.TestA"),
                None,
                Some(Fit::Classname),
            ),
            // pytest xunit1 and PHPUnit: the file.
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py"],
                "test_x",
                Some("tests.test_a"),
                Some("tests/test_a.py"),
                Some(Fit::File),
            ),
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py", "tests/test_b.py"],
                "test_x",
                None,
                Some("/work/app/tests/test_b.py"),
                None,
            ),
            // cargo-nextest: the binary id, and a `main` file in a folder.
            (
                "tests/valves.rs",
                rs,
                &["tests/valves.rs", "tests/pumps.rs"],
                "test_x",
                Some("app::valves"),
                None,
                Some(Fit::Classname),
            ),
            (
                "tests/valves.rs",
                rs,
                &["tests/valves.rs", "tests/pumps.rs"],
                "test_x",
                Some("app::pumps"),
                None,
                None,
            ),
            (
                "tests/valves/main.rs",
                rs,
                &["tests/valves/main.rs"],
                "test_x",
                Some("app::test/valves"),
                None,
                Some(Fit::Classname),
            ),
            // Vitest: the path of the file.
            (
                "tests/valves.test.ts",
                js,
                &["tests/valves.test.ts", "tests/pumps.test.ts"],
                "V > x",
                Some("tests/valves.test.ts"),
                None,
                Some(Fit::Classname),
            ),
            (
                "tests/valves.test.ts",
                js,
                &["tests/valves.test.ts", "tests/pumps.test.ts"],
                "V > x",
                Some("tests/pumps.test.ts"),
                None,
                None,
            ),
            // Go: the longest folder that ends the package path.
            (
                "valves/sensor_test.go",
                go,
                &["valves/sensor_test.go", "pumps/sensor_test.go"],
                "TestX",
                Some("example.org/app/valves"),
                None,
                Some(Fit::Classname),
            ),
            (
                "valves/sensor_test.go",
                go,
                &["valves/sensor_test.go", "app/valves/sensor_test.go"],
                "TestX",
                Some("example.org/app/valves"),
                None,
                // Changed: with no `go.mod`, 2 folders end the `classname`, and
                // the longest one no longer wins.
                Some(Fit::Name),
            ),
            (
                "app/valves/sensor_test.go",
                go,
                &["valves/sensor_test.go", "app/valves/sensor_test.go"],
                "TestX",
                Some("example.org/app/valves"),
                None,
                Some(Fit::Name),
            ),
            // gotestsum, the short form: one segment ends the folder.
            (
                "valves/sensor_test.go",
                go,
                &["valves/sensor_test.go", "pumps/sensor_test.go"],
                "TestX",
                Some("valves"),
                None,
                Some(Fit::Classname),
            ),
            // The package path of a Go module with no folder of a test file.
            (
                "valves/sensor_test.go",
                go,
                &["valves/sensor_test.go"],
                "TestX",
                Some("example.org/app"),
                None,
                // Changed: the folder of the tag does not end the import path.
                None,
            ),
            // Java and Kotlin: the class with its package, and an inner class.
            (
                "src/test/java/com/example/ValveTest.java",
                java,
                &["src/test/java/com/example/ValveTest.java"],
                "testX",
                Some("com.example.ValveTest"),
                None,
                Some(Fit::Classname),
            ),
            (
                "src/test/java/com/example/ValveTest.java",
                java,
                &["src/test/java/com/example/ValveTest.java"],
                "testX",
                Some("com.example.ValveTest$Inner"),
                None,
                // Changed: the nested class is a full field.
                None,
            ),
            (
                "src/test/java/com/example/ValveTest.java",
                nested,
                &["src/test/java/com/example/ValveTest.java"],
                "testX",
                Some("com.example.ValveTest$Inner"),
                None,
                Some(Fit::Classname),
            ),
            // .NET with a namespace that the folders do not follow.
            (
                "tests/ValveTest.cs",
                dotnet,
                &["tests/ValveTest.cs"],
                "testX",
                Some("Valves.Tests.ValveTest"),
                None,
                Some(Fit::Name),
            ),
            // jest-junit with its defaults: the classname is the title.
            (
                "tests/valves.test.js",
                js,
                &["tests/valves.test.js"],
                "V x",
                Some("V x"),
                None,
                Some(Fit::Name),
            ),
            // The line format: no file and no classname.
            (
                "tests/test_a.py",
                py,
                &["tests/test_a.py"],
                "test_x",
                None,
                None,
                Some(Fit::Name),
            ),
        ];
        for (path, text, files, name, classname, file, want) in rows {
            let cases = [case(name, classname, file, Outcome::Pass)];
            let (proof, bind) = bind_of(path, text, None, &cases, files);
            assert_eq!(bind.fit, want, "{path} / {classname:?} / {file:?}");
            assert_eq!(
                proof != Proof::Unbound,
                want.is_some(),
                "{path} / {classname:?} / {file:?}"
            );
            assert_eq!(bind.by_name, want == Some(Fit::Name), "{classname:?}");
        }
    }

    // The drop of the cases that fit by name only went: such a case is a weak
    // link, and it takes part in the worst case.
    #[test]
    fn a_case_that_fits_the_file_does_not_hide_a_case_with_no_full_field() {
        let text = "# covers: a#b\ndef test_init():\n    pass\n";
        let cases = [
            case("test_init", Some("tests.test_a"), None, Outcome::Skip),
            case("test_init", None, None, Outcome::Pass),
        ];
        let (proof, bind) = bind_of("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]);
        assert_eq!(proof, Proof::Skipped("test_init".into()));
        assert_eq!(bind.fit, Some(Fit::Classname));
        assert_eq!(bind.cases, 2);
        assert_eq!(bind.why, Some(Why::ByName));
    }

    // The shared set of 2 tests of one name went: the class chain is a full
    // field, so the case of the other class is another test.
    #[test]
    fn a_test_of_the_same_name_in_another_class_is_another_test() {
        let text = "class A:\n    # covers: a#b\n    def test_init(self):\n        pass\n\nclass B:\n    def test_init(self):\n        pass\n";
        let two = |a, b| {
            [
                case("test_init", Some("tests.test_a.A"), None, a),
                case("test_init", Some("tests.test_a.B"), None, b),
            ]
        };
        let files = ["tests/test_a.py"];
        for (own, other, want) in [
            (
                Outcome::Skip,
                Outcome::Pass,
                Proof::Skipped("test_init".into()),
            ),
            (
                Outcome::Pass,
                Outcome::Fail,
                Proof::Passed("test_init".into(), 1),
            ),
            (
                Outcome::Fail,
                Outcome::Pass,
                Proof::Failed("test_init".into()),
            ),
        ] {
            let (proof, bind) = bind_of("tests/test_a.py", text, None, &two(own, other), &files);
            assert_eq!(proof, want);
            assert!(!bind.shared && !bind.by_name && bind.cases == 1, "{bind:?}");
        }
        // The module-level test is neither of the 2 tests of a class.
        let one = "# covers: a#b\ndef test_init():\n    pass\n";
        let (proof, bind) = bind_of(
            "tests/test_a.py",
            one,
            None,
            &two(Outcome::Pass, Outcome::Pass),
            &files,
        );
        assert_eq!(proof, Proof::Unbound);
        assert_eq!(bind.reason, Some(Reason::NoCase));
    }

    // The tag is below a `describe` now: for a top-level `it`, `V closes` is
    // another test, and so is `W closes`.
    #[test]
    fn the_cases_of_jest_junit_with_its_defaults_are_weak_and_another_title_is_another_test() {
        let text = "describe('V', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\n";
        let files = ["tests/v.test.js"];
        let run = |name: &str, report: &str, outcome| Case {
            report: report.to_string(),
            ..case(name, Some(name), None, outcome)
        };
        // One case in 3 reports: one case.
        let cases = [
            run("V closes", "a", Outcome::Pass),
            run("V closes", "b", Outcome::Pass),
            run("V closes", "c", Outcome::Pass),
        ];
        let (proof, bind) = bind_of("tests/v.test.js", text, None, &cases, &files);
        assert_eq!(proof, Proof::Passed("V closes".into(), 3));
        assert!(bind.by_name && !bind.shared);
        // The same case twice in one report: 2 cases that share their name.
        let cases = [
            run("V closes", "a", Outcome::Skip),
            run("V closes", "a", Outcome::Pass),
        ];
        let (proof, bind) = bind_of("tests/v.test.js", text, None, &cases, &files);
        assert_eq!(proof, Proof::Skipped("V closes".into()));
        assert!(bind.shared);
        // Another title is another test.
        let cases = [
            run("V closes", "a", Outcome::Pass),
            run("W closes", "a", Outcome::Skip),
        ];
        let (proof, bind) = bind_of("tests/v.test.js", text, None, &cases, &files);
        assert_eq!(proof, Proof::Passed("V closes".into(), 1));
        assert!(bind.by_name && bind.cases == 1);
    }

    #[test]
    fn case_is_exact_and_a_unique_name_is_an_exact_bind() {
        let text = "# covers: a#b case:closes\nVALUE = 1\n";
        let cases = [case("V closes", None, None, Outcome::Pass)];
        let (proof, bind) = bind_of("t/test_a.py", text, Some("closes"), &cases, &[]);
        assert_eq!(proof, Proof::Unbound);
        assert_eq!(bind.reason, Some(Reason::NoCase));
        assert_eq!(bind.near, ["V closes"]);
        let (proof, bind) = bind_of("t/test_a.py", text, Some("V closes"), &cases, &[]);
        assert_eq!(proof, Proof::Passed("V closes".into(), 1));
        assert_eq!(bind.by, By::Case);
        assert_eq!(bind.fit, Some(Fit::Name));
        // Changed: one case of the name with no full field is weak too, because
        // the case of the tag can be absent.
        assert!(bind.by_name && !bind.shared);
        // 2 cases of the name in 2 files: a bind by name, and a shared set.
        let cases = [
            case("V closes", Some("V closes"), None, Outcome::Pass),
            case("V closes", Some("V closes"), None, Outcome::Skip),
        ];
        let (proof, bind) = bind_of("t/test_a.py", text, Some("V closes"), &cases, &[]);
        assert_eq!(proof, Proof::Skipped("V closes".into()));
        assert!(bind.by_name && bind.shared);
        // A case with a `classname` of another test file stays as a weak link.
        // Changed: the case of the file no longer hides it.
        let cases = [
            case("T", Some("tests.test_b"), None, Outcome::Pass),
            case("T", Some("tests.test_a"), None, Outcome::Skip),
        ];
        let files = ["tests/test_a.py", "tests/test_b.py"];
        let (proof, bind) = bind_of("tests/test_a.py", text, Some("T"), &cases, &files);
        assert_eq!(proof, Proof::Skipped("T".into()));
        assert_eq!(bind.fit, Some(Fit::Classname));
        assert!(bind.by_name && bind.cases == 2);
        // A case with a file binds only when the file is the file of the tag.
        let cases = [case("T", None, Some("tests/test_b.py"), Outcome::Pass)];
        let (proof, _) = bind_of("tests/test_a.py", text, Some("T"), &cases, &files);
        assert_eq!(proof, Proof::Unbound);
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

    /// The table of the name forms (design 0.8.1, D82 item 4). Each row is the
    /// text of a test, the name of a case and the result: true when the case
    /// names the test under the tag.
    #[test]
    fn the_name_forms_table_names_each_test_and_not_its_neighbor() {
        let def = |n: &str| format!("# covers: a#b\ndef {n}():\n    pass\n");
        let func = |n: &str| format!("// covers: a#b\nfunc {n}(t *testing.T) {{\n}}\n");
        let rust = |n: &str| format!("// covers: a#b\n#[test]\nfn {n}() {{\n}}\n");
        let java = |n: &str| format!("// covers: a#b\n@Test\nvoid {n}() {{\n}}\n");
        let call = |n: &str| format!("// covers: a#b\nit('{n}', () => {{\n}});\n");
        let kotlin = |n: &str| format!("// covers: a#b\n@Test fun `{n}`() {{\n}}\n");
        let method = |n: &str| {
            format!(
                "func TestValveSuite(t *testing.T) {{\n\tsuite.Run(t, new(ValveSuite))\n}}\n\n// covers: a#b\nfunc (s *ValveSuite) {n}() {{\n}}\n"
            )
        };
        // No suite function runs the type: the method binds no case.
        let lone = |n: &str| format!("// covers: a#b\nfunc (s *ValveSuite) {n}() {{\n}}\n");
        let rows: Vec<(String, &str, bool)> = vec![
            // A Kotlin name with spaces, with and without the `()` of the report.
            (kotlin("closes on timeout"), "closes on timeout()", true),
            (kotlin("closes on timeout"), "closes on timeout", true),
            (
                kotlin("closes on timeout"),
                "ValveTest closes on timeout()",
                true,
            ),
            (kotlin("closes on timeout"), "closes on timeouts()", false),
            // A Go method of a suite: the case is `<Test function>/<method>`.
            (method("TestCloses"), "TestValveSuite/TestCloses", true),
            (
                method("TestCloses"),
                "TestValveSuite/TestCloses/case_1",
                true,
            ),
            // The suite function must be the one that runs the type.
            (method("TestCloses"), "TestBSuite/TestCloses", false),
            (method("TestCloses"), "ValveSuite/TestCloses", false),
            (method("TestCloses"), "TestCloses", false),
            (lone("TestCloses"), "TestValveSuite/TestCloses", false),
            // A parameterized case of a Kotlin name in backticks.
            (kotlin("closes"), "closes(int) [1] 3", true),
            (
                kotlin("closes on timeout"),
                "closes on timeout(int) [1] 3",
                true,
            ),
            (kotlin("closes"), "closed(int) [1] 3", false),
            // The `()` of a report is no part of a JavaScript name.
            (call("flushes"), "Valves > flushes", true),
            (call("flushes"), "Valves > flushes (slow)", false),
            (call("flushes"), "Valves > flushes()", false),
            (method("TestCloses"), "TestValveSuite/TestOther", false),
            (method("TestCloses"), "TestValveSuite", false),
            (method("TestCloses"), "TestValveSuite/TestClosesLate", false),
            // Parameters, subtests, module paths, data sets and class paths.
            (def("test_closes"), "test_closes[1-2]", true),
            (def("test_closes"), "test_closes", true),
            (rust("closes"), "tests::closes", true),
            (rust("closes"), "closes::case_1", true),
            (func("TestTimeout"), "TestTimeout/no_reply", true),
            (java("test1"), "test1(String) [1] a", true),
            (java("testOne"), "testOne with data set #0", true),
            (java("ClosesOnTimeout"), "ValveTest.ClosesOnTimeout", true),
            // A name that only starts or ends like the test is not the test.
            (def("test_x"), "test_x_slow", false),
            (def("test_x"), "slow_test_x", false),
            (rust("closes"), "tests::closes_late", false),
            (rust("closes"), "Valves closes", false),
            // The form call: the literal, after a space, `>` or `\u{203a}`.
            (call("closes on timeout"), "closes on timeout", true),
            (call("closes on timeout"), "Valves closes on timeout", true),
            (
                call("closes on timeout"),
                "Valves > closes on timeout",
                true,
            ),
            (
                call("closes on timeout"),
                "Valves \u{203a} closes on timeout",
                true,
            ),
            (call("closes"), "Valves not closes", true),
            (call("closes on timeout"), "closes on timeouts", false),
            (call("closes on timeout"), "recloseson timeout", false),
        ];
        for (text, name, bound) in rows {
            let cases = [case(name, None, None, Outcome::Pass)];
            let got = proof("tests/x_test.src", &text, None, &cases, &[]);
            let want = if bound {
                Proof::Passed(name.to_string(), 1)
            } else {
                Proof::Unbound
            };
            assert_eq!(got, want, "{text:?} / {name}");
        }
    }

    fn test_name(name: &str, call: bool) -> TestName {
        TestName {
            name: name.to_string(),
            call,
            bt: false,
            recv: None,
        }
    }

    /// What the first tag of a text stands above.
    fn under(text: &str) -> Under {
        let lines: Vec<&str> = text.lines().collect();
        let tag = lines.iter().position(|l| l.contains("covers:")).unwrap();
        under_tag(&lines, tag)
    }

    #[test]
    fn the_scan_reads_the_test_under_a_tag_in_each_form() {
        let def = |n: &str| Under::Test(test_name(n, false));
        let call = |n: &str| Under::Test(test_name(n, true));
        let rows: Vec<(&str, Under)> = vec![
            // The form def.
            ("# covers: a#b\ndef test_x():\n    pass\n", def("test_x")),
            ("# covers: a#b\nasync def test_x(a):\n", def("test_x")),
            // A multi-line decorator, and 2 decorators.
            (
                "# covers: a#b\n@pytest.mark.parametrize(\n    \"a, b\",\n    [(1, 2)],\n)\n@pytest.mark.slow\ndef test_x(a, b):\n",
                def("test_x"),
            ),
            // The form fn, with attributes and modifiers.
            (
                "// covers: a#b\n#[test]\n#[should_panic(expected = \"x\")]\npub(crate) async fn closes() {\n",
                def("closes"),
            ),
            // The form func, with a receiver.
            (
                "// covers: a#b\nfunc TestTimeout(t *testing.T) {\n",
                def("TestTimeout"),
            ),
            (
                "// covers: a#b\nfunc (s *Suite) TestTimeout() {\n",
                Under::Test(TestName {
                    name: "TestTimeout".into(),
                    call: false,
                    bt: false,
                    recv: Some("Suite".into()),
                }),
            ),
            // The form call, with its suffixes.
            ("// covers: a#b\nit('closes', () => {\n", call("closes")),
            (
                "// covers: a#b\ntest.only(\"closes on timeout\", async () => {\n",
                call("closes on timeout"),
            ),
            ("# covers: a#b\nit \"closes\" do\n", call("closes")),
            (
                "// covers: a#b\nit.skip(`closes`, () => {\n",
                call("closes"),
            ),
            // The form method.
            (
                "// covers: a#b\n@Test\n@DisplayName(\"closes\")\nvoid flushesFirst() {\n",
                def("flushesFirst"),
            ),
            ("// covers: a#b\n@Test fun closes() {\n", def("closes")),
            (
                "// covers: a#b\n@Test fun `closes on timeout`() {\n",
                Under::Test(TestName {
                    name: "closes on timeout".into(),
                    call: true,
                    bt: true,
                    recv: None,
                }),
            ),
            (
                "// covers: a#b\n[Fact] public void ClosesOnTimeout()\n",
                def("ClosesOnTimeout"),
            ),
            (
                "// covers: a#b\n[Theory]\n[InlineData(1)]\npublic async Task Closes(int a)\n",
                def("Closes"),
            ),
            (
                "// covers: a#b\npublic function testClosesOnTimeout()\n",
                def("testClosesOnTimeout"),
            ),
            // A definition on the line of the tag, before the text.
            ("def test_x():  # covers: a#b\n    pass\n", def("test_x")),
            ("@Test fun closes() { // covers: a#b\n}\n", def("closes")),
            // A doc comment, and a comment with words before the tag, are no
            // definition: the scan goes on below.
            ("/// covers: a#b\n#[test]\nfn closes() {\n", def("closes")),
            ("// This test covers: a#b\ndef test_x():\n", def("test_x")),
            ("/* The test covers: a#b */\n@Test\nvoid x() {\n", def("x")),
            ("# a note, covers: a#b\ndef test_x():\n", def("test_x")),
            // A comment or blank lines between the tag and the test.
            (
                "# covers: a#b\n# more text\n\n# covers: c#d\ndef test_x():\n",
                def("test_x"),
            ),
            (
                "/**\n * covers: a#b\n */\n@Test\nvoid closes() {\n",
                def("closes"),
            ),
            // A byte order mark before the tag of line 1.
            ("\u{feff}// covers: a#b\ndef test_x():\n", def("test_x")),
            ("\u{feff}# covers: a#b\ndef test_x():\n", def("test_x")),
            ("\u{feff}/* covers: a#b */\n@Test\nvoid x() {\n", def("x")),
            // A group.
            ("// covers: a#b\ndescribe('Valves', () => {\n", Under::Group),
            ("# covers: a#b\nclass TestValves:\n", Under::Group),
            ("// covers: a#b\nmod tests {\n", Under::Group),
            ("// covers: a#b\npublic class ValveTest {\n", Under::Group),
            ("# covers: a#b\ncontext \"valves\" do\n", Under::Group),
            // A template.
            (
                "// covers: a#b\nit.each(rows)('closes %s', (r) => {\n",
                Under::Template("closes %s".into()),
            ),
            (
                "// covers: a#b\ntest.each`\n",
                Under::Template(String::new()),
            ),
            (
                "// covers: a#b\nit('closes ${name}', () => {\n",
                Under::Template("closes ${name}".into()),
            ),
            (
                "# covers: a#b\nit \"closes #{name}\" do\n",
                Under::Template("closes #{name}".into()),
            ),
            // No test.
            (
                "// covers: a#b\nimport { describe, it } from 'vitest'\n",
                Under::NoTest(Some(2)),
            ),
            ("# covers: a#b\nx = 1\n", Under::NoTest(Some(2))),
            ("# covers: a#b\n", Under::NoTest(None)),
            (
                "def test_a():\n    # covers: a#b\n    assert True\n\ndef test_b():\n",
                Under::NoTest(Some(3)),
            ),
            // The body of a test, then a test with less indent.
            (
                "def test_a():\n    # covers: a#b\n\ndef test_b():\n",
                Under::NoTest(Some(4)),
            ),
            ("# covers: a#b\n@Test\n", Under::NoTest(None)),
        ];
        for (text, want) in rows {
            assert_eq!(under(text), want, "{text:?}");
        }
    }

    #[test]
    fn the_scan_reads_at_most_50_lines() {
        let far = format!("# covers: a#b\n{}def test_x():\n", "# text\n".repeat(60));
        assert_eq!(under(&far), Under::NoTest(None));
        let near = format!("# covers: a#b\n{}def test_x():\n", "# text\n".repeat(40));
        assert_eq!(under(&near), Under::Test(test_name("test_x", false)));
    }

    #[test]
    fn the_tests_of_a_file_are_the_definition_lines() {
        let text = "import it\n\ndescribe('Valves', () => {\n  it('closes', () => {\n    expect(1);\n  });\n\n  it('never closes', () => {\n  });\n});\n";
        let lines: Vec<&str> = text.lines().collect();
        let names: Vec<TestName> = tests_of(&lines).into_iter().map(|f| f.test).collect();
        assert_eq!(
            names,
            [test_name("closes", true), test_name("never closes", true)]
        );
    }

    // The strength of a name match went: the identity decides what a case is,
    // and `name_match` only says which cases to read.
    #[test]
    fn the_name_of_a_test_is_a_whole_segment_of_the_name_of_the_case() {
        let call = test_name("closes", true);
        let def = test_name("closes", false);
        // The form call: the whole name, a whole segment after a separator, or
        // the end after a space.
        for name in [
            "closes",
            "Valves > closes",
            "Valves \u{203a} closes",
            "Valves > closes > more",
            "Valves closes",
        ] {
            assert!(name_match(name, &call), "{name}");
        }
        // A name that has a segment of ` > ` never matches by its last word.
        for name in [
            "Valves > fast closes",
            "Valves reopens",
            "Valves closes now",
            "closes (slow)",
            "closes()",
        ] {
            assert!(!name_match(name, &call), "{name}");
        }
        // An identifier: a whole segment before its parameters. A space is no
        // separator, and a tail starts right after the name.
        for name in [
            "closes",
            "tests::closes",
            "closes::case_1",
            "closes[1-2]",
            "closes/no_reply",
            "closes(int) [1] a",
            "closes with data set #0",
        ] {
            assert!(name_match(name, &def), "{name}");
        }
        assert!(!name_match("Valves closes", &def));
        assert!(!name_match("closes_late", &def));
        let upper = test_name("Closes", false);
        assert!(!name_match("Closes when cold", &upper));
        assert!(!name_match("Closes (cold)", &upper));
        // A Kotlin name in backticks is whole, and the parameters follow it.
        let kt = TestName {
            bt: true,
            ..test_name("stops (fast)", true)
        };
        for name in ["stops (fast)", "stops (fast)()", "stops (fast)(int) [1] 3"] {
            assert!(name_match(name, &kt), "{name}");
        }
        assert!(!name_match("stops()", &kt));
    }

    #[test]
    fn the_index_finds_each_case_that_the_matcher_accepts() {
        // The index is only a filter: a key that is missing gives a silent
        // `no-case`. Each pair that `name_match` accepts must come out of it.
        let pairs: Vec<(TestName, &str)> = vec![
            (test_name("closes", true), "closes"),
            (test_name("closes", true), "Valves > closes"),
            (test_name("closes", true), "Valves \u{203a} closes > more"),
            (test_name("closes", true), "Valves closes"),
            (
                test_name("handles v1.2 input", true),
                "A > handles v1.2 input",
            ),
            (test_name("closes", false), "tests::closes"),
            (test_name("closes", false), "closes::case_1"),
            (test_name("closes", false), "closes[1-2]"),
            (test_name("closes", false), "closes(int) [1] a"),
            (test_name("closes", false), "closes with data set #0"),
            (test_name("closes", false), "Suite/closes"),
            (test_name("closes", false), "ValveTest.closes"),
            (test_name("closes", false), "ValveTest#closes"),
            (
                TestName {
                    bt: true,
                    ..test_name("stops (fast)", true)
                },
                "stops (fast)(int) [1] 3",
            ),
            (
                TestName {
                    bt: true,
                    ..test_name("closes on timeout", true)
                },
                "ValveTest closes on timeout()",
            ),
            (
                TestName {
                    recv: Some("Suite".into()),
                    ..test_name("TestCloses", false)
                },
                "TestSuite/TestCloses/sub",
            ),
        ];
        for (test, name) in pairs {
            assert!(name_match(name, &test), "{name} / {}", test.name);
            let cases = [case(name, None, None, Outcome::Pass)];
            let binder = Binder::new(&cases, &[]);
            assert_eq!(binder.named_by(&test), [0], "{name} / {}", test.name);
        }
    }

    #[test]
    fn a_long_case_name_is_read_in_less_than_2_seconds() {
        // 8,000 dots, and 1,200 times `(a.`: the keys of such a name grew with
        // the square of its separators (73 s and 65 s before the cap).
        let text = "# covers: a#b\ndef test_closes():\n    pass\n";
        let names = [
            format!("test_closes{}", ".".repeat(8_000)),
            format!("test_closes{}", "(a.".repeat(1_200)),
        ];
        for name in names {
            assert!(is_long(&name));
            let cases = [case(&name, Some("tests.test_a"), None, Outcome::Pass)];
            let start = std::time::Instant::now();
            let (proof, bind) =
                bind_of("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]);
            assert!(start.elapsed().as_secs_f64() < 2.0, "{:?}", start.elapsed());
            assert_eq!(proof, Proof::Unbound);
            assert_eq!(bind.reason, Some(Reason::LongName));
            // The whole name still binds by `case:`.
            let (proof, _) = bind_of(
                "tests/test_a.py",
                &format!("# covers: a#b case:\"{name}\"\n"),
                Some(&name),
                &cases,
                &["tests/test_a.py"],
            );
            assert_eq!(proof, Proof::Passed(name.clone(), 1));
        }
        // A long case of another test, or of another file, is no reason: the
        // tag has no case, and the reason stays `no-case`.
        let other = [case(
            &format!("test_other{}", ".".repeat(8_000)),
            Some("tests.test_a"),
            None,
            Outcome::Pass,
        )];
        let (_, bind) = bind_of("tests/test_a.py", text, None, &other, &["tests/test_a.py"]);
        assert_eq!(bind.reason, Some(Reason::NoCase));
        let elsewhere = [case(
            &format!("test_closes{}", ".".repeat(8_000)),
            Some("tests.test_b"),
            None,
            Outcome::Pass,
        )];
        let files = ["tests/test_a.py", "tests/test_b.py"];
        let (_, bind) = bind_of("tests/test_a.py", text, None, &elsewhere, &files);
        assert_eq!(bind.reason, Some(Reason::NoCase));
        // A name of an ordinary test is no long name.
        assert!(!is_long("tests.test_a.TestA.test_closes[1-2]"));
    }

    #[test]
    fn no_case_that_fits_a_test_hides_another_case_of_it() {
        // The tag is on `closes`. The case of the test `never closes` ends with
        // the same word, and it fails: it is a case of the set, and the worst
        // case decides.
        let js = "describe('Valves', () => {\n  // covers: requirements/valves#flush\n  it('closes', () => {\n  });\n\n  it('never closes', () => {\n  });\n});\n";
        let cases = [
            case("Valves closes", None, None, Outcome::Pass),
            case("Valves never closes", None, None, Outcome::Fail),
        ];
        assert_eq!(
            proof("tests/valves.test.js", js, None, &cases, &[]),
            Proof::Failed("Valves never closes".into())
        );
        // With the name of the test in the whole name, the one case proves.
        let cases = [case("Valves closes", None, None, Outcome::Pass)];
        assert_eq!(
            proof("tests/valves.test.js", js, None, &cases, &[]),
            Proof::Passed("Valves closes".into(), 1)
        );
        // A name with a segment of ` > ` is no match for the end of a name.
        let tests = [test_name("closes", true)];
        assert!(!name_match("Valves > never closes", &tests[0]));
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
    fn the_literal_of_the_test_binds_and_a_keyword_of_the_line_does_not() {
        let text = "// covers: requirements/valves#flush\nit('closes on timeout', () => {\n  expect(1).toBe(1);\n});\n";
        let cases = [
            case("Widget renders it", None, None, Outcome::Pass),
            case("closes on timeout", None, None, Outcome::Fail),
        ];
        assert_eq!(
            proof("tests/valves.test.js", text, None, &cases, &[]),
            Proof::Failed("closes on timeout".into())
        );
        // The word `it` is the keyword of the test, not its name.
        let cases = [case("Widget renders it", None, None, Outcome::Pass)];
        assert_eq!(
            proof("tests/valves.test.js", text, None, &cases, &[]),
            Proof::Unbound
        );
        // The name of the function binds, and `func` does not.
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
                bind: None,
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
            in_tree: files.iter().cloned().collect(),
            test_files: files,
            ..Reads::default()
        };
        (dir, covers, reads)
    }

    #[test]
    fn a_tag_in_a_file_outside_the_code_tree_binds_no_case() {
        let (dir, mut covers, mut reads) = scale(2);
        reads.in_tree.remove("tests/test_f1.py");
        bind_covers(&dir, &mut covers, &reads);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(covers[0].proof, Some(Proof::Passed("test_a0".into(), 1)));
        assert_eq!(covers[1].proof, Some(Proof::Unbound));
        let bind = covers[1].bind.clone().unwrap();
        assert_eq!(bind.reason, Some(Reason::OutsideTree));
        assert_eq!(bind.cases, 0);
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
            bind: None,
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

/// The matrix of the bind rule: each runner, each result of the case of the
/// tag, and each neighbor that has the same head or last segment. The names of
/// the reports are written out here, as the runners write them. They are not
/// built by the code under test.
#[cfg(test)]
mod matrix {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Own {
        Passed,
        Failed,
        Skipped,
        Absent,
    }

    /// A report case: the name, the `classname` and the `file`.
    type Shape = (&'static str, Option<&'static str>, Option<&'static str>);

    /// A neighbor: its label, its case, and the other test file that holds it
    /// (`None` for a test of the tag file).
    struct Near {
        label: &'static str,
        case: Shape,
        file: Option<&'static str>,
    }

    struct Runner {
        label: &'static str,
        path: &'static str,
        /// The tag file. It holds the test under the tag and the tests of the
        /// neighbors of the same file.
        text: &'static str,
        own: Shape,
        /// 2 rows of the same test, as the runner writes them.
        rows: Option<[&'static str; 2]>,
        near: Vec<Near>,
        manifests: &'static [(&'static str, &'static str)],
        /// The counter-constraint (A3): the bind is silent, and no neighbor is
        /// in the set.
        silent: bool,
    }

    const CARGO: &[(&str, &str)] = &[("Cargo.toml", "[package]\nname = \"app\"\n")];
    const GO_MOD: &[(&str, &str)] = &[("go.mod", "module example.org/app\n")];

    fn near(label: &'static str, case: Shape, file: Option<&'static str>) -> Near {
        Near { label, case, file }
    }

    fn runners() -> Vec<Runner> {
        let nextest_int = |label, manifests| Runner {
            label,
            path: "tests/valves.rs",
            text: "// covers: a#b\n#[test]\nfn closes() {\n}\n\nmod cold {\n    #[test]\n    fn closes() {\n    }\n}\n",
            own: ("closes", Some("app::valves"), None),
            rows: Some(["closes::case_1", "closes::case_2"]),
            near: vec![
                near(
                    "same-file",
                    ("cold::closes", Some("app::valves"), None),
                    None,
                ),
                near(
                    "other-file",
                    ("closes", Some("app::pumps"), None),
                    Some("tests/pumps.rs"),
                ),
                near(
                    "src-vs-tests",
                    ("valves::tests::closes", Some("app"), None),
                    Some("src/valves.rs"),
                ),
            ],
            manifests,
            silent: true,
        };
        let go = |label, manifests| Runner {
            label,
            path: "valves/sensor_test.go",
            text: "package valves\n\n// covers: a#b\nfunc TestClose(t *testing.T) {\n}\n\nfunc TestTable(t *testing.T) {\n\tt.Run(\"TestClose\", func(t *testing.T) {\n\t})\n}\n",
            own: ("TestClose", Some("example.org/app/valves"), None),
            rows: Some(["TestClose/row_1", "TestClose/row_2"]),
            near: vec![
                near(
                    "same-file",
                    ("TestTable/TestClose", Some("example.org/app/valves"), None),
                    None,
                ),
                near(
                    "other-file",
                    ("TestClose", Some("example.org/app/pumps"), None),
                    Some("pumps/sensor_test.go"),
                ),
            ],
            manifests,
            silent: true,
        };
        vec![
            Runner {
                label: "pytest-xunit2",
                path: "tests/test_valves.py",
                text: "class TestValves:\n    # covers: a#b\n    def test_closes(self):\n        pass\n\nclass TestPumps:\n    def test_closes(self):\n        pass\n\ndef test_closes():\n    pass\n",
                own: ("test_closes", Some("tests.test_valves.TestValves"), None),
                rows: Some(["test_closes[1]", "test_closes[2]"]),
                near: vec![
                    near(
                        "same-file",
                        ("test_closes", Some("tests.test_valves.TestPumps"), None),
                        None,
                    ),
                    near(
                        "module-level",
                        ("test_closes", Some("tests.test_valves"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("test_closes", Some("tests.test_pumps.TestValves"), None),
                        Some("tests/test_pumps.py"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "pytest-xunit1",
                path: "tests/test_valves.py",
                text: "class TestValves:\n    # covers: a#b\n    def test_closes(self):\n        pass\n\nclass TestPumps:\n    def test_closes(self):\n        pass\n",
                own: (
                    "test_closes",
                    Some("tests.test_valves.TestValves"),
                    Some("tests/test_valves.py"),
                ),
                rows: Some(["test_closes[1]", "test_closes[2]"]),
                near: vec![
                    near(
                        "same-file",
                        (
                            "test_closes",
                            Some("tests.test_valves.TestPumps"),
                            Some("tests/test_valves.py"),
                        ),
                        None,
                    ),
                    near(
                        "other-file",
                        (
                            "test_closes",
                            Some("tests.test_pumps.TestValves"),
                            Some("tests/test_pumps.py"),
                        ),
                        Some("tests/test_pumps.py"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            nextest_int("nextest-integration", CARGO),
            nextest_int("nextest-integration-no-manifest", &[]),
            Runner {
                label: "nextest-unit",
                path: "src/valves.rs",
                text: "#[cfg(test)]\nmod tests {\n    // covers: a#b\n    #[test]\n    fn closes() {\n    }\n}\n\nmod hot {\n    #[test]\n    fn closes() {\n    }\n}\n",
                own: ("valves::tests::closes", Some("app"), None),
                rows: Some([
                    "valves::tests::closes::case_1",
                    "valves::tests::closes::case_2",
                ]),
                near: vec![
                    near(
                        "same-file",
                        ("valves::hot::closes", Some("app"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("pumps::tests::closes", Some("app"), None),
                        Some("src/pumps.rs"),
                    ),
                    near(
                        "tests-vs-src",
                        ("closes", Some("app::valves"), None),
                        Some("tests/valves.rs"),
                    ),
                ],
                manifests: CARGO,
                silent: true,
            },
            go("go", GO_MOD),
            go("go-no-manifest", &[]),
            Runner {
                label: "go-testify",
                path: "valves/suite_test.go",
                text: "package valves\n\nfunc TestValveSuite(t *testing.T) {\n\tsuite.Run(t, new(ValveSuite))\n}\n\nfunc TestPumpSuite(t *testing.T) {\n\tsuite.Run(t, new(PumpSuite))\n}\n\n// covers: a#b\nfunc (s *ValveSuite) TestCloses() {\n}\n\nfunc (s *PumpSuite) TestCloses() {\n}\n",
                own: (
                    "TestValveSuite/TestCloses",
                    Some("example.org/app/valves"),
                    None,
                ),
                rows: Some([
                    "TestValveSuite/TestCloses/row_1",
                    "TestValveSuite/TestCloses/row_2",
                ]),
                near: vec![
                    near(
                        "same-file",
                        (
                            "TestPumpSuite/TestCloses",
                            Some("example.org/app/valves"),
                            None,
                        ),
                        None,
                    ),
                    near(
                        "other-file",
                        (
                            "TestValveSuite/TestCloses",
                            Some("example.org/app/pumps"),
                            None,
                        ),
                        Some("pumps/suite_test.go"),
                    ),
                ],
                manifests: GO_MOD,
                silent: true,
            },
            Runner {
                label: "junit-java",
                path: "src/test/java/com/example/ValveTest.java",
                text: "package com.example;\n\nclass ValveTest {\n    // covers: a#b\n    @Test\n    void closes() {\n    }\n}\n\nclass SpareTest {\n    @Test\n    void closes() {\n    }\n}\n",
                own: ("closes", Some("com.example.ValveTest"), None),
                rows: Some(["closes(int)[1]", "closes(int)[2]"]),
                near: vec![
                    near(
                        "same-file",
                        ("closes", Some("com.example.SpareTest"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("closes", Some("com.example.PumpTest"), None),
                        Some("src/test/java/com/example/PumpTest.java"),
                    ),
                    near(
                        "other-package",
                        ("closes", Some("com.other.ValveTest"), None),
                        Some("src/test/java/com/other/ValveTest.java"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "junit-kotlin",
                path: "src/test/kotlin/com/example/ValveTest.kt",
                text: "package com.example\n\nclass ValveTest {\n    // covers: a#b\n    @Test fun `closes on timeout`() {\n    }\n}\n\nclass SpareTest {\n    @Test fun `closes on timeout`() {\n    }\n}\n",
                own: ("closes on timeout()", Some("com.example.ValveTest"), None),
                rows: Some(["closes on timeout(int)[1]", "closes on timeout(int)[2]"]),
                near: vec![
                    near(
                        "same-file",
                        ("closes on timeout()", Some("com.example.SpareTest"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("closes on timeout()", Some("com.example.PumpTest"), None),
                        Some("src/test/kotlin/com/example/PumpTest.kt"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "xunit",
                path: "tests/ValveTest.cs",
                text: "namespace Valves.Tests;\n\npublic class ValveTest\n{\n    // covers: a#b\n    [Fact]\n    public void Closes()\n    {\n    }\n}\n\npublic class SpareTest\n{\n    [Fact]\n    public void Closes()\n    {\n    }\n}\n",
                own: ("Closes", Some("Valves.Tests.ValveTest"), None),
                rows: Some(["Closes(n: 1)", "Closes(n: 2)"]),
                near: vec![
                    near(
                        "same-file",
                        ("Closes", Some("Valves.Tests.SpareTest"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("Closes", Some("Valves.Tests.PumpTest"), None),
                        Some("tests/PumpTest.cs"),
                    ),
                ],
                manifests: &[],
                silent: false,
            },
            Runner {
                label: "phpunit",
                path: "tests/ValveTest.php",
                text: "<?php\nnamespace Tests;\n\nclass ValveTest extends TestCase\n{\n    // covers: a#b\n    public function testCloses()\n    {\n    }\n}\n\nclass SpareTest extends TestCase\n{\n    public function testCloses()\n    {\n    }\n}\n",
                own: (
                    "testCloses",
                    Some("Tests.ValveTest"),
                    Some("/work/app/tests/ValveTest.php"),
                ),
                rows: Some(["testCloses with data set #0", "testCloses with data set #1"]),
                near: vec![
                    near(
                        "same-file",
                        (
                            "testCloses",
                            Some("Tests.SpareTest"),
                            Some("/work/app/tests/ValveTest.php"),
                        ),
                        None,
                    ),
                    near(
                        "other-file",
                        (
                            "testCloses",
                            Some("Tests.PumpTest"),
                            Some("/work/app/tests/PumpTest.php"),
                        ),
                        Some("tests/PumpTest.php"),
                    ),
                ],
                manifests: &[],
                silent: false,
            },
            Runner {
                label: "rspec",
                path: "spec/valve_spec.rb",
                text: "describe Valve do\n  # covers: a#b\n  it 'closes' do\n  end\nend\n\ndescribe Pump do\n  it 'closes' do\n  end\n\n  it 'never closes' do\n  end\nend\n",
                own: (
                    "Valve closes",
                    Some("spec.valve_spec"),
                    Some("./spec/valve_spec.rb"),
                ),
                rows: None,
                near: vec![
                    near(
                        "same-file",
                        (
                            "Pump closes",
                            Some("spec.valve_spec"),
                            Some("./spec/valve_spec.rb"),
                        ),
                        None,
                    ),
                    near(
                        "same-end",
                        (
                            "Pump never closes",
                            Some("spec.valve_spec"),
                            Some("./spec/valve_spec.rb"),
                        ),
                        None,
                    ),
                    near(
                        "other-file",
                        (
                            "Valve closes",
                            Some("spec.pump_spec"),
                            Some("./spec/pump_spec.rb"),
                        ),
                        Some("spec/pump_spec.rb"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "vitest",
                path: "tests/valves.test.ts",
                text: "describe('Valves', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\n\ndescribe('Pumps', () => {\n  it('closes', () => {\n  });\n  it('never closes', () => {\n  });\n});\n",
                own: ("Valves > closes", Some("tests/valves.test.ts"), None),
                rows: None,
                near: vec![
                    near(
                        "same-file",
                        ("Pumps > closes", Some("tests/valves.test.ts"), None),
                        None,
                    ),
                    near(
                        "same-end",
                        ("Pumps > never closes", Some("tests/valves.test.ts"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("Valves > closes", Some("tests/pumps.test.ts"), None),
                        Some("tests/pumps.test.ts"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "playwright",
                path: "e2e/valves.spec.ts",
                text: "test.describe('Valves', () => {\n  // covers: a#b\n  test('closes', async () => {\n  });\n});\n\ntest.describe('Pumps', () => {\n  test('closes', async () => {\n  });\n});\n",
                own: ("Valves \u{203a} closes", Some("valves.spec.ts"), None),
                rows: None,
                near: vec![
                    near(
                        "same-file",
                        ("Pumps \u{203a} closes", Some("valves.spec.ts"), None),
                        None,
                    ),
                    near(
                        "other-file",
                        ("Valves \u{203a} closes", Some("pumps.spec.ts"), None),
                        Some("e2e/pumps.spec.ts"),
                    ),
                ],
                manifests: &[],
                silent: true,
            },
            Runner {
                label: "jest-file",
                path: "tests/valves.test.js",
                text: "describe('Valves', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\n\ndescribe('Pumps', () => {\n  it('closes', () => {\n  });\n});\n",
                own: (
                    "Valves closes",
                    Some("Valves closes"),
                    Some("tests/valves.test.js"),
                ),
                rows: None,
                near: vec![
                    near(
                        "same-file",
                        (
                            "Pumps closes",
                            Some("Pumps closes"),
                            Some("tests/valves.test.js"),
                        ),
                        None,
                    ),
                    near(
                        "other-file",
                        (
                            "Valves closes",
                            Some("Valves closes"),
                            Some("tests/pumps.test.js"),
                        ),
                        Some("tests/pumps.test.js"),
                    ),
                ],
                manifests: &[],
                silent: false,
            },
            Runner {
                label: "jest-defaults",
                path: "tests/valves.test.js",
                text: "describe('Valves', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\n\ndescribe('Pumps', () => {\n  it('closes', () => {\n  });\n});\n",
                own: ("Valves closes", Some("Valves closes"), None),
                rows: None,
                near: vec![
                    near(
                        "other-title",
                        ("Pumps closes", Some("Pumps closes"), None),
                        None,
                    ),
                    near(
                        "same-title",
                        ("Valves closes", Some("Valves closes"), None),
                        Some("tests/pumps.test.js"),
                    ),
                ],
                manifests: &[],
                silent: false,
            },
            Runner {
                label: "lines",
                path: "tests/test_valves.py",
                text: "# covers: a#b\ndef test_closes():\n    pass\n",
                own: ("test_closes", None, None),
                rows: None,
                near: vec![near(
                    "other-file",
                    ("test_closes", None, None),
                    Some("tests/test_pumps.py"),
                )],
                manifests: &[],
                silent: false,
            },
            // A file of the same relative path in a second project: the report
            // cannot tell them apart.
            Runner {
                label: "vitest-monorepo-twin",
                path: "tests/valves.test.ts",
                text: "describe('Valves', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\n",
                own: ("Valves > closes", Some("tests/valves.test.ts"), None),
                rows: None,
                near: vec![near(
                    "suffix-twin",
                    ("Valves > closes", Some("pkg/tests/valves.test.ts"), None),
                    Some("pkg/tests/valves.test.ts"),
                )],
                manifests: &[],
                silent: false,
            },
        ]
    }

    fn case_of(shape: Shape, name: &str, outcome: Outcome) -> Case {
        Case {
            name: name.to_string(),
            classname: shape.1.map(str::to_string),
            file: shape.2.map(str::to_string),
            outcome,
            report: "r.xml".to_string(),
        }
    }

    /// The one call of the code under test.
    fn run(
        path: &str,
        text: &str,
        case_name: Option<&str>,
        cases: &[Case],
        files: &[&str],
        manifests: &[(&str, &str)],
    ) -> (Proof, Bind) {
        let lines: Vec<&str> = text.lines().collect();
        let tag = lines.iter().position(|l| l.contains("covers:")).unwrap();
        let files: Vec<String> = files.iter().map(|f| f.to_string()).collect();
        let suites = |ty: &str, method: &str| suite_funcs([text], ty, method);
        let read = |p: &str| {
            manifests
                .iter()
                .find(|(m, _)| *m == p)
                .map(|(_, t)| t.to_string())
        };
        // ADAPTER
        Binder::new(cases, &files).bind_with(path, &lines, tag, case_name, &suites, &read)
    }

    fn word(proof: &Proof, bind: &Bind) -> String {
        let p = match proof {
            Proof::Passed(..) => "passed",
            Proof::Failed(_) => "failed",
            Proof::Skipped(_) => "skipped",
            Proof::Unbound => "unbound",
        };
        assert_eq!(bind.by_name, bind.why.is_some(), "{bind:?}");
        match bind.why {
            Some(why) => format!("{p}+{}", why.key()),
            None => p.to_string(),
        }
    }

    #[test]
    fn no_cell_of_the_matrix_gives_a_silent_false_proof_or_hides_a_fail() {
        let start = std::time::Instant::now();
        let mut binds = 0;
        let mut red: Vec<String> = Vec::new();
        for r in runners() {
            let rows: &[bool] = if r.rows.is_some() {
                &[false, true]
            } else {
                &[false]
            };
            for &rowed in rows {
                for own in [Own::Passed, Own::Failed, Own::Skipped, Own::Absent] {
                    let mut neighbors: Vec<(Option<&Near>, Outcome)> = vec![(None, Outcome::Pass)];
                    for n in &r.near {
                        neighbors.push((Some(n), Outcome::Pass));
                        neighbors.push((Some(n), Outcome::Fail));
                    }
                    for (n, result) in neighbors {
                        if own == Own::Absent && (rowed || n.is_none()) {
                            continue;
                        }
                        let outcome = match own {
                            Own::Passed => Some(Outcome::Pass),
                            Own::Failed => Some(Outcome::Fail),
                            Own::Skipped => Some(Outcome::Skip),
                            Own::Absent => None,
                        };
                        let mut cases = Vec::new();
                        if let Some(o) = outcome {
                            match r.rows.filter(|_| rowed) {
                                // The first row passes. The second row has the
                                // result of the cell.
                                Some([a, b]) => {
                                    cases.push(case_of(r.own, a, Outcome::Pass));
                                    cases.push(case_of(r.own, b, o));
                                }
                                None => cases.push(case_of(r.own, r.own.0, o)),
                            }
                        }
                        let mut files = vec![r.path];
                        if let Some(n) = n {
                            cases.push(case_of(n.case, n.case.0, result));
                            files.extend(n.file);
                        }
                        let (proof, bind) = run(r.path, r.text, None, &cases, &files, r.manifests);
                        binds += 1;
                        let id = format!(
                            "{}{} own={own:?} near={}/{result:?} -> {}",
                            r.label,
                            if rowed { " rows" } else { "" },
                            n.map_or("none", |n| n.label),
                            word(&proof, &bind),
                        );
                        let passed = matches!(proof, Proof::Passed(..));
                        // A1: a silent pass needs a passed case of the tag.
                        if passed && !bind.by_name && own != Own::Passed {
                            red.push(format!("A1 {id}"));
                        }
                        // A2: a failed or skipped case of the tag never passes.
                        if matches!(own, Own::Failed | Own::Skipped) && passed {
                            red.push(format!("A2 {id}"));
                        }
                        // A3: the counter-constraint.
                        if r.silent && own == Own::Passed && !(passed && !bind.by_name) {
                            red.push(format!("A3 {id}"));
                        }
                        if r.silent
                            && own == Own::Absent
                            && !(proof == Proof::Unbound
                                && bind.reason == Some(Reason::NoCase)
                                && !bind.by_name)
                        {
                            red.push(format!("A3 {id}"));
                        }
                        // A4: jest-junit with its defaults.
                        if r.label == "jest-defaults" && own == Own::Passed {
                            let same = n.is_some_and(|n| n.label == "same-title");
                            let ok = if same && result == Outcome::Fail {
                                matches!(proof, Proof::Failed(_))
                            } else {
                                passed
                            } && bind.why == Some(Why::ByName);
                            if !ok {
                                red.push(format!("A4 {id}"));
                            }
                        }
                    }
                }
            }
        }
        let took = start.elapsed();
        assert!(
            red.is_empty(),
            "{} red of {binds} binds\n{}",
            red.len(),
            red.join("\n")
        );
        assert!(binds >= 450, "{binds}");
        assert!(took.as_secs_f64() < 1.0, "{binds} binds took {took:?}");
    }

    /// A named cell: the tag file, the cases, the other test files, the
    /// manifests, the value of `case:`, and the result that the rule gives.
    struct Named {
        id: &'static str,
        path: &'static str,
        text: &'static str,
        case: Option<&'static str>,
        cases: Vec<Case>,
        files: &'static [&'static str],
        manifests: &'static [(&'static str, &'static str)],
        want: &'static str,
    }

    fn c(name: &str, classname: Option<&str>, file: Option<&str>, outcome: Outcome) -> Case {
        Case {
            name: name.to_string(),
            classname: classname.map(str::to_string),
            file: file.map(str::to_string),
            outcome,
            report: "r.xml".to_string(),
        }
    }

    fn named_cells() -> Vec<Named> {
        use Outcome::{Fail, Pass, Skip};
        const RS: &str = "#[cfg(test)]\nmod tests {\n    // covers: a#b\n    #[test]\n    fn closes() {\n    }\n}\n";
        const RS_B: &str = "// covers: a#b\n#[test]\nfn closes() {\n}\n";
        const HOT: &str = "mod hot {\n    // covers: a#b\n    #[test]\n    fn closes() {\n    }\n}\n\nmod cold {\n    #[test]\n    fn closes() {\n    }\n}\n";
        const CLI: &str = "// covers: a#b\n#[test]\nfn runs() {\n}\n";
        const TWO_CRATES: &[(&str, &str)] = &[
            ("crates/a/Cargo.toml", "[package]\nname = \"a-cli\"\n"),
            ("crates/b/Cargo.toml", "[package]\nname = \"b-cli\"\n"),
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        ];
        const CRATE_FILES: &[&str] = &["crates/a/tests/cli.rs", "crates/b/tests/cli.rs"];
        const GO_FILES: &[&str] = &["server/serve_test.go", "cmd/server/serve_test.go"];
        const GO: &str = "package server\n\n// covers: a#b\nfunc TestServe(t *testing.T) {\n}\n";
        const PY_CLASS: &str = "# covers: a#b\ndef test_closes():\n    pass\n\nclass TestPumps:\n    def test_closes(self):\n        pass\n";
        const NESTED: &str = "package com.example;\n\nclass ValveTest {\n    // covers: a#b\n    @Test\n    void closes() {\n    }\n\n    @Nested\n    class Nested {\n        @Test\n        void closes() {\n        }\n    }\n}\n";
        const OVERLOAD: &str = "package com.example;\n\nclass ValveTest {\n    // covers: a#b\n    @Test\n    void closes() {\n    }\n\n    @ParameterizedTest\n    void closes(int n) {\n    }\n}\n";
        const VT: &str = "describe('Valves', () => {\n  // covers: a#b\n  it('closes', () => {\n  });\n});\ndescribe('Pumps', () => {\n  it('closes', () => {\n  });\n});\n";
        const PW: &str = "test.describe('Valves', () => {\n  // covers: a#b\n  test('closes', async () => {\n  });\n});\n";
        const XU: &str = "namespace Valves.Tests;\n\npublic class ValveTest\n{\n    // covers: a#b\n    [Fact]\n    public void Closes()\n    {\n    }\n\n    [Fact(DisplayName = \"Closes when cold\")]\n    public void Other()\n    {\n    }\n}\n";
        const TWINS: &str = "describe Valve do\n  describe '#open' do\n    # covers: a#b\n    it 'works' do\n    end\n  end\n\n  describe '#shut' do\n    it 'works' do\n    end\n  end\nend\n";
        const KT: &str = "package com.example\n\nclass ValveTest {\n    // covers: a#b\n    @ParameterizedTest\n    fun `closes on timeout`(n: Int) {\n    }\n}\n";
        const PY: &str = "# covers: a#b\ndef test_closes():\n    pass\n";
        const EACH: &str = "describe('Valves', () => {\n  // covers: a#b\n  it.skip('closes', () => {\n  });\n\n  it.each(['a', 'b'])('closes', (x) => {\n  });\n});\n";
        const PW_EACH: &str = "// covers: a#b\ntest.skip('closes', async () => {\n});\n\ntest.each(['a', 'b'])('closes', async () => {\n});\n";
        const RS_STRINGS: &str = "describe 'Valve' do\n  describe '#open' do\n    # covers: a#b\n    it 'works' do\n    end\n  end\nend\n";
        const RS_COLONS: &str = "describe Valve do\n  describe '::open' do\n    # covers: a#b\n    it 'works' do\n    end\n  end\nend\n";
        const RS_EXAMPLE: &str =
            "describe Valve do\n  # covers: a#b\n  it '#opens' do\n  end\nend\n";
        const IN_NESTED: &str = "package com.example;\n\nclass ValveTest {\n    @Nested\n    class Nested {\n        // covers: a#b\n        @Test\n        void closes() {\n        }\n    }\n}\n";
        const KT_PLAIN: &str = "package com.example\n\nclass ValveTest {\n    // covers: a#b\n    @Test fun `closes on timeout`() {\n    }\n}\n";
        const XU_TAGGED: &str = "namespace Valves.Tests;\n\npublic class ValveTest\n{\n    // covers: a#b\n    [Fact(DisplayName = \"closes when cold\")]\n    public void Closes()\n    {\n    }\n}\n";
        const HYPHEN: &[(&str, &str)] = &[("Cargo.toml", "[package]\nname = \"my-crate\"\n")];
        const SPEC: Option<&str> = Some("./spec/valve_spec.rb");
        let rspec = |id, text, name: &str, want| Named {
            id,
            path: "spec/valve_spec.rb",
            text,
            case: None,
            cases: vec![c(name, Some("spec.valve_spec"), SPEC, Pass)],
            files: &["spec/valve_spec.rb"],
            manifests: &[],
            want,
        };
        let nested = |id, classname: &str, want| Named {
            id,
            path: "src/test/java/com/example/ValveTest.java",
            text: IN_NESTED,
            case: None,
            cases: vec![c("closes", Some(classname), None, Pass)],
            files: &["src/test/java/com/example/ValveTest.java"],
            manifests: &[],
            want,
        };
        let mut more = vec![
            rspec(
                "RSpec: 2 string titles keep the space",
                RS_STRINGS,
                "Valve #open works",
                "passed",
            ),
            rspec(
                "RSpec: a constant and `::` have no space",
                RS_COLONS,
                "Valve::open works",
                "passed",
            ),
            rspec(
                "RSpec: an example keeps the space",
                RS_EXAMPLE,
                "Valve #opens",
                "passed",
            ),
            nested(
                "a tag in a nested class: the shape of Surefire 3.2.5",
                "com.example.ValveTest$Nested",
                "passed",
            ),
            nested(
                "a tag in a nested class: a newer Surefire writes the outer class",
                "com.example.ValveTest",
                "passed+by-name",
            ),
            Named {
                id: "a template with the literal of the test: a skipped run is not hidden",
                path: "tests/v.test.ts",
                text: EACH,
                case: None,
                cases: vec![
                    c("Valves > closes", Some("tests/v.test.ts"), None, Skip),
                    c("Valves > closes", Some("tests/v.test.ts"), None, Pass),
                    c("Valves > closes", Some("tests/v.test.ts"), None, Pass),
                ],
                files: &["tests/v.test.ts"],
                manifests: &[],
                want: "skipped+not-unique",
            },
            Named {
                id: "a template twin in the shape of Playwright: a skipped run is not hidden",
                path: "e2e/v.spec.ts",
                text: PW_EACH,
                case: None,
                cases: vec![
                    c("closes", Some("v.spec.ts"), None, Skip),
                    c("closes", Some("v.spec.ts"), None, Pass),
                ],
                files: &["e2e/v.spec.ts"],
                manifests: &[],
                want: "skipped+not-unique",
            },
            Named {
                id: "a top-level test beside a table: a repeat in one report is no run",
                path: "tests/v.test.ts",
                text: "// covers: a#b\nit.skip('closes', () => {\n});\n\nit.each(['closes'])('%s', () => {\n});\n",
                case: None,
                cases: vec![
                    c("closes", Some("tests/v.test.ts"), None, Skip),
                    c("closes", Some("tests/v.test.ts"), None, Pass),
                ],
                files: &["tests/v.test.ts"],
                manifests: &[],
                want: "skipped",
            },
            Named {
                id: "a Kotlin name in backticks with no parentheses in the report",
                path: "src/test/kotlin/com/example/ValveTest.kt",
                text: KT_PLAIN,
                case: None,
                cases: vec![c(
                    "closes on timeout",
                    Some("com.example.ValveTest"),
                    None,
                    Pass,
                )],
                files: &["src/test/kotlin/com/example/ValveTest.kt"],
                manifests: &[],
                want: "passed",
            },
            Named {
                id: "xUnit: a tagged test with a DisplayName is weak",
                path: "tests/ValveTest.cs",
                text: XU_TAGGED,
                case: None,
                cases: vec![c(
                    "closes when cold",
                    Some("Valves.Tests.ValveTest"),
                    None,
                    Pass,
                )],
                files: &["tests/ValveTest.cs"],
                manifests: &[],
                want: "passed+no-identity",
            },
            Named {
                id: "a group with a path constant is unreadable",
                path: "spec/valve_spec.rb",
                text: "describe Valves::Valve do\n  # covers: a#b\n  it 'closes' do\n  end\nend\n",
                case: None,
                cases: vec![c(
                    "Valves::Valve closes",
                    Some("spec.valve_spec"),
                    SPEC,
                    Pass,
                )],
                files: &["spec/valve_spec.rb"],
                manifests: &[],
                want: "passed+no-identity",
            },
            Named {
                id: "a file with no indent is unreadable",
                path: "tests/v.test.ts",
                text: "describe('Valves', () => {\n// covers: a#b\nit('closes', () => {\n});\n});\n",
                case: None,
                cases: vec![c("Valves > closes", Some("tests/v.test.ts"), None, Pass)],
                files: &["tests/v.test.ts"],
                manifests: &[],
                want: "passed+no-identity",
            },
            Named {
                id: "a unit test of Rust with no Cargo.toml",
                path: "src/valves.rs",
                text: RS,
                case: None,
                cases: vec![c("valves::tests::closes", Some("app"), None, Pass)],
                files: &["src/valves.rs"],
                manifests: &[],
                want: "passed+by-name",
            },
            Named {
                id: "nextest keeps the hyphen of the package name",
                path: "tests/valves.rs",
                text: RS_B,
                case: None,
                cases: vec![c("closes", Some("my-crate::valves"), None, Pass)],
                files: &["tests/valves.rs"],
                manifests: HYPHEN,
                want: "passed",
            },
            Named {
                id: "one binary in 2 files and no Cargo.toml is one binary",
                path: "tests/it/valves.rs",
                text: RS_B,
                case: None,
                cases: vec![c("valves::closes", Some("mycrate::it"), None, Pass)],
                files: &["tests/it/main.rs", "tests/it/valves.rs"],
                manifests: &[],
                want: "passed",
            },
        ];
        more.extend(vec![
            Named {
                id: "B1 the failed unit case is not hidden by an integration binary",
                path: "src/valves.rs",
                text: RS,
                case: None,
                cases: vec![
                    c("valves::tests::closes", Some("app"), None, Fail),
                    c("closes", Some("app::valves"), None, Pass),
                ],
                files: &["src/valves.rs", "tests/valves.rs"],
                manifests: CARGO,
                want: "failed",
            },
            Named {
                id: "B2 b::closes against a::closes",
                path: "src/b.rs",
                text: RS_B,
                case: None,
                cases: vec![c("a::closes", Some("app"), None, Pass)],
                files: &["src/a.rs", "src/b.rs"],
                manifests: CARGO,
                want: "unbound",
            },
            Named {
                id: "B2 hot::closes against cold::closes",
                path: "src/lib.rs",
                text: HOT,
                case: None,
                cases: vec![c("cold::closes", Some("app"), None, Pass)],
                files: &["src/lib.rs"],
                manifests: CARGO,
                want: "unbound",
            },
            Named {
                id: "B2 crate a against crate b",
                path: "crates/a/tests/cli.rs",
                text: CLI,
                case: None,
                cases: vec![c("runs", Some("b-cli::cli"), None, Pass)],
                files: CRATE_FILES,
                manifests: TWO_CRATES,
                want: "unbound",
            },
            Named {
                id: "B2 crate a against crate b, no Cargo.toml",
                path: "crates/a/tests/cli.rs",
                text: CLI,
                case: None,
                cases: vec![c("runs", Some("b-cli::cli"), None, Pass)],
                files: CRATE_FILES,
                manifests: &[],
                want: "passed+not-unique",
            },
            Named {
                id: "A3 workspace: tests/cli.rs in 2 crates, silent",
                path: "crates/a/tests/cli.rs",
                text: CLI,
                case: None,
                cases: vec![
                    c("runs", Some("a-cli::cli"), None, Pass),
                    c("runs", Some("b-cli::cli"), None, Fail),
                ],
                files: CRATE_FILES,
                manifests: TWO_CRATES,
                want: "passed",
            },
            Named {
                id: "A3 Go: server/ beside cmd/server/, the tag in server/",
                path: "server/serve_test.go",
                text: GO,
                case: None,
                cases: vec![
                    c("TestServe", Some("example.org/app/server"), None, Pass),
                    c("TestServe", Some("example.org/app/cmd/server"), None, Fail),
                ],
                files: GO_FILES,
                manifests: GO_MOD,
                want: "passed",
            },
            Named {
                id: "A3 Go: server/ beside cmd/server/, the tag in cmd/server/",
                path: "cmd/server/serve_test.go",
                text: GO,
                case: None,
                cases: vec![
                    c("TestServe", Some("example.org/app/server"), None, Fail),
                    c("TestServe", Some("example.org/app/cmd/server"), None, Pass),
                ],
                files: GO_FILES,
                manifests: GO_MOD,
                want: "passed",
            },
            Named {
                id: "B3 the pytest class",
                path: "tests/test_a.py",
                text: PY_CLASS,
                case: None,
                cases: vec![c("test_closes", Some("tests.test_a.TestPumps"), None, Pass)],
                files: &["tests/test_a.py"],
                manifests: &[],
                want: "unbound",
            },
            Named {
                id: "B3 the JUnit nested class",
                path: "src/test/java/com/example/ValveTest.java",
                text: NESTED,
                case: None,
                cases: vec![c("closes", Some("com.example.ValveTest$Nested"), None, Pass)],
                files: &["src/test/java/com/example/ValveTest.java"],
                manifests: &[],
                want: "unbound",
            },
            Named {
                id: "a nested class with the method name: a newer Surefire writes the outer class",
                path: "src/test/java/com/example/ValveTest.java",
                text: NESTED,
                case: None,
                cases: vec![c("closes", Some("com.example.ValveTest"), None, Pass)],
                files: &["src/test/java/com/example/ValveTest.java"],
                manifests: &[],
                want: "passed+not-unique",
            },
            Named {
                id: "B3 an overload is weak",
                path: "src/test/java/com/example/ValveTest.java",
                text: OVERLOAD,
                case: None,
                cases: vec![c("closes", Some("com.example.ValveTest"), None, Pass)],
                files: &["src/test/java/com/example/ValveTest.java"],
                manifests: &[],
                want: "passed+not-unique",
            },
            Named {
                id: "B3 the Vitest title chain",
                path: "tests/v.test.ts",
                text: VT,
                case: None,
                cases: vec![c("Pumps > closes", Some("tests/v.test.ts"), None, Pass)],
                files: &["tests/v.test.ts"],
                manifests: &[],
                want: "unbound",
            },
            Named {
                id: "B3 the Playwright title chain",
                path: "e2e/v.spec.ts",
                text: PW,
                case: None,
                cases: vec![c("Pumps \u{203a} closes", Some("v.spec.ts"), None, Pass)],
                files: &["e2e/v.spec.ts"],
                manifests: &[],
                want: "unbound",
            },
            Named {
                id: "B3 the xUnit display name of another test",
                path: "tests/ValveTest.cs",
                text: XU,
                case: None,
                cases: vec![c("Closes when cold", Some("Valves.Tests.ValveTest"), None, Pass)],
                files: &["tests/ValveTest.cs"],
                manifests: &[],
                want: "unbound",
            },
            Named {
                id: "M1 2 RSpec examples with 2 full titles",
                path: "spec/valve_spec.rb",
                text: TWINS,
                case: None,
                cases: vec![
                    c("Valve#open works", Some("spec.valve_spec"), Some("./spec/valve_spec.rb"), Pass),
                    c("Valve#shut works", Some("spec.valve_spec"), Some("./spec/valve_spec.rb"), Fail),
                ],
                files: &["spec/valve_spec.rb"],
                manifests: &[],
                want: "passed",
            },
            Named {
                id: "M2 the rows of a Kotlin name in backticks",
                path: "src/test/kotlin/com/example/ValveTest.kt",
                text: KT,
                case: None,
                cases: vec![
                    c("closes on timeout(int)[1]", Some("com.example.ValveTest"), None, Pass),
                    c("closes on timeout(int)[2]", Some("com.example.ValveTest"), None, Pass),
                ],
                files: &["src/test/kotlin/com/example/ValveTest.kt"],
                manifests: &[],
                want: "passed",
            },
            Named {
                id: "Playwright: one test in 2 projects",
                path: "e2e/v.spec.ts",
                text: PW,
                case: None,
                cases: vec![
                    c("Valves \u{203a} closes", Some("v.spec.ts"), None, Pass),
                    c("Valves \u{203a} closes", Some("v.spec.ts"), None, Skip),
                ],
                files: &["e2e/v.spec.ts"],
                manifests: &[],
                want: "passed",
            },
            Named {
                id: "Playwright: one test in 2 projects, with the project in the name",
                path: "e2e/v.spec.ts",
                text: PW,
                case: None,
                cases: vec![
                    c("[chromium] Valves \u{203a} closes", Some("v.spec.ts"), None, Pass),
                    c("[webkit] Valves \u{203a} closes", Some("v.spec.ts"), None, Skip),
                ],
                files: &["e2e/v.spec.ts"],
                manifests: &[],
                want: "passed",
            },
            Named {
                id: "a pytest row [2] was skipped",
                path: "tests/test_a.py",
                text: PY,
                case: None,
                cases: vec![
                    c("test_closes[1]", Some("tests.test_a"), None, Pass),
                    c("test_closes[2]", Some("tests.test_a"), None, Skip),
                ],
                files: &["tests/test_a.py"],
                manifests: &[],
                want: "skipped",
            },
            Named {
                id: "a long pytest row that failed is not dropped",
                path: "tests/test_a.py",
                text: PY,
                case: None,
                cases: vec![
                    c("test_closes[1]", Some("tests.test_a"), None, Pass),
                    c(
                        "test_closes[a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a.a]",
                        Some("tests.test_a"),
                        None,
                        Fail,
                    ),
                ],
                files: &["tests/test_a.py"],
                manifests: &[],
                want: "failed",
            },
            Named {
                id: "A5 case: with one case of another file that passed",
                path: "tests/test_a.py",
                text: "# covers: a#b case:test_closes\nVALUE = 1\n",
                case: Some("test_closes"),
                cases: vec![c("test_closes", Some("tests.test_b"), None, Pass)],
                files: &["tests/test_a.py", "tests/test_b.py"],
                manifests: &[],
                want: "passed+by-name",
            },
            Named {
                id: "A5 case: with jest-junit defaults and one case",
                path: "tests/v.test.js",
                text: "// covers: a#b case:\"Valves closes\"\n",
                case: Some("Valves closes"),
                cases: vec![c("Valves closes", Some("Valves closes"), None, Pass)],
                files: &["tests/v.test.js"],
                manifests: &[],
                want: "passed+by-name",
            },
            Named {
                id: "case: with the classname of the tag file is silent",
                path: "tests/test_a.py",
                text: "# covers: a#b case:test_closes\nVALUE = 1\n",
                case: Some("test_closes"),
                cases: vec![c("test_closes", Some("tests.test_a"), None, Pass)],
                files: &["tests/test_a.py", "tests/test_b.py"],
                manifests: &[],
                want: "passed",
            },
        ]);
        more
    }

    #[test]
    fn each_named_cell_of_the_rule_has_its_result() {
        let mut red = Vec::new();
        for n in named_cells() {
            let (proof, bind) = run(n.path, n.text, n.case, &n.cases, n.files, n.manifests);
            let got = word(&proof, &bind);
            if got != n.want {
                red.push(format!("{}: want {}, got {got}", n.id, n.want));
            }
        }
        assert!(red.is_empty(), "{} red\n{}", red.len(), red.join("\n"));
    }
}

/// The deterministic guard of the cost of the index (docs/test-report.md).
#[cfg(test)]
mod index_cost {
    use super::*;

    #[test]
    fn a_name_has_at_most_98_keys() {
        // 256 bytes and 32 separators in each shape, with many spaces.
        for sep in [" > ", "::", "/", ".", "#", "(", "[", " \u{203a} "] {
            for word in ["a ", "ab c d ", "a"] {
                let mut name = String::new();
                for i in 0..32 {
                    name.push_str(&format!("{i}{word}x{sep}"));
                }
                while name.len() < 256 {
                    name.push_str(" y");
                }
                let mut end = 256;
                while !name.is_char_boundary(end) {
                    end -= 1;
                }
                name.truncate(end);
                let keys = case_keys(&name).0.len();
                assert!(keys <= 98, "{keys} keys: {name:?}");
            }
        }
        // Many spaces: the ends after the last 32 spaces only, and the name is
        // in the list of the names with more words.
        let words = "w ".repeat(120);
        let (keys, wordy) = case_keys(words.trim_end());
        assert!(keys.len() <= 98 && wordy, "{}", keys.len());
    }

    #[test]
    fn the_index_of_10000_typical_names_has_at_most_6_keys_for_each_case() {
        let names: Vec<Case> = (0..10_000)
            .map(|i| {
                let name = match i % 6 {
                    0 => format!("test_closes_{i}[1-2]"),
                    1 => format!("valves::tests::closes_{i}"),
                    2 => format!("TestClose{i}/no_reply"),
                    3 => format!("Valves closes on timeout {i}"),
                    4 => format!("Valves > closes on timeout {i}"),
                    _ => format!("closes{i}(int)[1]"),
                };
                Case {
                    name,
                    classname: Some("tests.test_valves".to_string()),
                    file: None,
                    outcome: Outcome::Pass,
                    report: "r.xml".to_string(),
                }
            })
            .collect();
        let binder = Binder::new(&names, &[]);
        let entries = binder.index.entries();
        assert!(entries <= 6 * names.len(), "{entries}");
    }

    /// A test name with more words than a case keeps ends of finds its case.
    #[test]
    fn a_test_with_more_than_32_words_still_finds_a_longer_case() {
        let lit = format!("{}closes", "very ".repeat(40));
        let text = format!("// covers: a#b\nit('{lit}', () => {{\n}});\n");
        let lines: Vec<&str> = text.lines().collect();
        let cases = [Case {
            name: format!("Valves {lit}"),
            classname: None,
            file: None,
            outcome: Outcome::Fail,
            report: "r.xml".to_string(),
        }];
        let (proof, _) = Binder::new(&cases, &[]).bind("t/v.test.js", &lines, 0, None);
        assert_eq!(proof, Proof::Failed(cases[0].name.clone()));
    }
}
