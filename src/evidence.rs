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

/// Why a bind is weak (SD621). The first cause that applies decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// More than one test of the reports fits: the cases differ before the end
    /// of the name of the test, or in their `classname` or `file`.
    Many,
    /// A case fits the test file by its name only.
    ByName,
    /// No case has the whole name of the test: a case only ends with it, or it
    /// is a segment of a longer name.
    NotWhole,
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
    under_tag_line(lines, tag).0
}

/// Like `under_tag`. It also gives the index of the definition line.
fn under_tag_line(lines: &[&str], tag: usize) -> (Under, usize) {
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
        let own = [before_tag(&lines[tag][..pos]).trim_start_matches('\u{feff}')];
        let (mut off, mut any) = (0, false);
        while let Some((_, o)) = skip_item(&own, 0, off, 1) {
            (off, any) = (o, true);
        }
        annotated = any;
        let text = own[0][off..].trim();
        if !text.is_empty() {
            let under = classify(text, annotated).unwrap_or(Under::NoTest(Some(tag + 1)));
            return (under, tag);
        }
    }
    match definition(lines, tag + 1, annotated) {
        None => (Under::NoTest(None), tag),
        Some(d) if indent(lines[d.first]) < tag_indent => (Under::NoTest(Some(d.line + 1)), d.line),
        Some(d) => (
            classify(d.text, d.annotated).unwrap_or(Under::NoTest(Some(d.line + 1))),
            d.line,
        ),
    }
}

/// The title of the group that a line opens: `Some(Some(title))` for
/// `describe('Valves'`, `Some(None)` for a class or a module, and `None` for a
/// line that is no group.
fn group_title(body: &str) -> Option<Option<String>> {
    let body = body.strip_prefix("RSpec.").unwrap_or(body);
    let n = ident_end(body);
    let (word, rest) = body.split_at(n);
    if matches!(
        word,
        "class" | "object" | "mod" | "impl" | "module" | "namespace"
    ) && rest.starts_with(' ')
    {
        return Some(None);
    }
    if !matches!(word, "describe" | "context" | "suite") {
        return None;
    }
    let mut s = rest;
    while let Some(r) = s.strip_prefix('.') {
        s = &r[ident_end(r)..];
    }
    let s = s.strip_prefix('(').unwrap_or(s).trim_start();
    if let Some(title) = literal(s) {
        return Some(Some(title.to_string()));
    }
    // `describe Valve do`
    let m = ident_end(s);
    (m > 0).then(|| Some(s[..m].to_string()))
}

/// The titles of the groups that hold the definition at `lines[def]`, the
/// outermost first (`describe('Valves'` gives `Valves`). `None` when a line with
/// less indent is no group: the chain is not readable, and the name of the case
/// then stays a weak match.
fn ancestors(lines: &[&str], def: usize) -> Option<Vec<String>> {
    let mut want = indent(lines[def]);
    let mut out = Vec::new();
    for k in (0..def).rev() {
        if want == 0 {
            break;
        }
        let t = lines[k].trim();
        if t.is_empty() || is_comment(t) {
            continue;
        }
        let ind = indent(lines[k]);
        if ind >= want {
            continue;
        }
        if let Some(title) = group_title(strip_modifiers(t))? {
            out.push(title);
        }
        want = ind;
    }
    out.reverse();
    Some(out)
}

/// The tests of a file, each at its definition line. It reads the file once.
fn tests_of(lines: &[&str]) -> Vec<TestName> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() || is_comment(t) {
            i += 1;
            continue;
        }
        let Some(d) = definition(lines, i, false) else {
            break;
        };
        if let Some(Under::Test(t)) = classify(d.text, d.annotated) {
            out.push(t);
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

/// The separators of the segments of a case name. A space inside a name is no
/// separator: `Closes when cold` is one segment.
const SEPS: [&str; 6] = [" > ", " \u{203a} ", "::", "/", ".", "#"];

/// How well the name of a case names a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NameFit {
    /// The case name only ends with the name after a space (jest-junit and
    /// RSpec join the titles with a space).
    Weak,
    /// The name is a whole segment of the case name.
    Strong,
}

/// The end of the last place where `text` has `lit` as a whole segment: at the
/// start or after a separator, and at the end or before a separator.
fn find_whole(text: &str, lit: &str) -> Option<usize> {
    if lit.is_empty() {
        return None;
    }
    text.match_indices(lit)
        .filter(|(i, _)| {
            let j = i + lit.len();
            (*i == 0 || SEPS.iter().any(|s| text[..*i].ends_with(s)))
                && (j == text.len() || SEPS.iter().any(|s| text[j..].starts_with(s)))
        })
        .map(|(i, _)| i + lit.len())
        .max()
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

/// How the name of the case names the test, or `None` (design 0.8.1, D82 item
/// 4). The rule is that the name of the test is a whole segment of the case
/// name, with the separators of `SEPS` only:
///
/// - a Go method of a suite: `<Test function>/<method>`, and more segments for a
///   subtest;
/// - a name in a literal (the form call): the name is a whole segment, or it is
///   the end of the case name after a space (a weak match, unless the case name
///   has a segment of ` > `). A Kotlin name in backticks has the parameters
///   after it: the whole name first, then each part before a `(` or ` [`;
/// - an identifier: it is a whole segment of the name before its parameters.
///
/// The first part is where the name ends in the case name. The cases that
/// differ only after it (the subtests and the rows of a table) are one test.
fn name_match(case: &str, test: &TestName) -> Option<(usize, NameFit)> {
    if test.recv.is_some() {
        // A case of one segment names no method.
        let mut segs = case.split('/');
        let first = segs.next()?;
        let second = segs.next()?;
        return (second == test.name).then_some((first.len() + 1 + second.len(), NameFit::Strong));
    }
    if test.call {
        let lit = test.name.as_str();
        let variants = if test.bt {
            tail_variants(case)
        } else {
            vec![case]
        };
        if variants.iter().any(|v| find_whole(v, lit).is_some()) {
            return Some((case.len(), NameFit::Strong));
        }
        let space = |v: &str| {
            v.strip_suffix(lit).is_some_and(|prefix| {
                prefix.ends_with(' ') && !prefix.contains(" > ") && !prefix.contains(" \u{203a} ")
            })
        };
        return variants
            .iter()
            .any(|v| space(v))
            .then_some((case.len(), NameFit::Weak));
    }
    find_whole(cut_head(case), &test.name).map(|e| (e, NameFit::Strong))
}

/// True if the case has the whole name of the test: it equals the name, or its
/// last segment does (` > `, `::` and `.` end a segment, `/` does not), or it
/// is the expected name, or it is the method of a Go suite under its function.
/// The subtests and the rows of a table have the name of the test before their
/// tail, and they are no own case.
fn is_own(case: &str, test: &TestName, titles: Option<&[String]>) -> bool {
    let last = |text: &str, seps: &[&str]| -> usize {
        seps.iter()
            .filter_map(|sep| text.rfind(sep).map(|i| i + sep.len()))
            .max()
            .unwrap_or(0)
    };
    if test.recv.is_some() {
        let mut segs = case.split('/');
        return segs.next().is_some()
            && segs.next() == Some(test.name.as_str())
            && segs.next().is_none();
    }
    if test.call {
        let variants = if test.bt {
            tail_variants(case)
        } else {
            vec![case]
        };
        return variants
            .iter()
            .any(|v| v[last(v, &[" > ", " \u{203a} "])..] == test.name)
            || titles.is_some_and(|ts| expected_name(case, test, ts));
    }
    let head = cut_head(case);
    head[last(head, &["::", "."])..] == test.name
}

/// True if the case is the one that the titles of the groups and the name of
/// the test give: the titles and the name, joined with a space, ` > ` or
/// ` \u{203a} `. RSpec joins a title that starts with `#` or `.` to the title
/// before it with no space (`Valve#open closes`). Then the weak match is the
/// expected full name.
fn expected_name(case: &str, test: &TestName, titles: &[String]) -> bool {
    let joined = |sep: &str| {
        let mut want = String::new();
        for (i, t) in titles.iter().enumerate() {
            if i > 0 && !(sep == " " && t.starts_with(['#', '.'])) {
                want.push_str(sep);
            }
            want.push_str(t);
        }
        if !titles.is_empty() {
            want.push_str(sep);
        }
        want.push_str(&test.name);
        want
    };
    [" ", " > ", " \u{203a} "]
        .iter()
        .any(|sep| case == joined(sep))
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

/// The keys of a test file for a binary id of cargo-nextest: the stem of the
/// file, or the name of its folder when the stem is `main`. A file in a folder of
/// `tests/` (`tests/it/valves.rs`) is a module of the binary of that folder, so
/// the first folder under `tests/` is a key too.
fn binary_keys(path: &str) -> Vec<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    let mut keys = vec![if stem == "main" {
        folder_of(path).rsplit('/').next().unwrap_or("")
    } else {
        stem
    }];
    let under = path
        .strip_prefix("tests/")
        .or_else(|| path.split_once("/tests/").map(|(_, r)| r));
    if let Some((dir, _)) = under.and_then(|r| r.split_once('/')) {
        keys.push(dir);
    }
    keys
}

/// The ends of a path at each `/`, the longest first: `a/b/c`, `b/c`, `c`.
fn path_ends(path: &str) -> impl Iterator<Item = &str> {
    std::iter::once(path).chain(path.match_indices('/').map(|(i, _)| &path[i + 1..]))
}

/// How a `classname` names a test file (design 0.8.1, D82 item 6). The first
/// row that applies decides.
enum Class {
    /// The binary id of cargo-nextest, with `::`: the key after the last `::`
    /// and its last `/`.
    Binary(String),
    /// A path with `/` whose last segment has a `.`: the rule of `file`.
    Path(String),
    /// A path with `/` and no extension: the longest folder of a test file that
    /// ends it in whole segments.
    Folder(Option<String>),
    /// Another `classname`, with `$` read as `.`: the keys that the dotted path
    /// of the test file can end with, and the `classname` when it has one
    /// segment (it can also end the folder).
    Dotted(Vec<String>, Option<String>),
}

/// The way a case fits a test file: by its file, by its `classname`, or by its
/// name only.
enum Kind {
    File(String),
    Class(Class, bool),
    Name,
}

/// A Rust test file: an integration test (`tests/<name>.rs`, a binary of its
/// own), or a file of `src/` with unit tests, with its module path.
enum RustFile {
    No,
    Test,
    Src(Vec<String>),
}

/// The kind of a Rust file by its path. The module path of `src/a/b.rs` is
/// `a::b`, and `src/lib.rs` and `src/main.rs` have the empty path.
fn rust_file(path: &str) -> RustFile {
    if !path.ends_with(".rs") {
        return RustFile::No;
    }
    let rest = path
        .strip_prefix("src/")
        .or_else(|| path.rsplit_once("/src/").map(|(_, r)| r));
    let Some(rest) = rest else {
        return RustFile::Test;
    };
    let mut module: Vec<String> = rest
        .trim_end_matches(".rs")
        .split('/')
        .map(str::to_string)
        .collect();
    if matches!(
        module.last().map(String::as_str),
        Some("mod" | "lib" | "main")
    ) {
        module.pop();
    }
    RustFile::Src(module)
}

/// The test file of a tag, with what the rules of fit read.
struct Place<'p> {
    path: &'p str,
    dotted: String,
    keys: Vec<&'p str>,
    folder: &'p str,
    rust: RustFile,
}

impl<'p> Place<'p> {
    fn new(path: &'p str) -> Place<'p> {
        Place {
            path,
            dotted: dotted(path),
            keys: binary_keys(path),
            folder: folder_of(path),
            rust: rust_file(path),
        }
    }
}

/// The keys of a case name for the index: the name, the parts before each
/// parameter list, the parts after a space or a separator, and the segments.
/// The index only finds the cases to look at. `name_match` decides.
fn case_keys(name: &str) -> Vec<&str> {
    // A name that is too long has no key but its whole name: the work for the
    // keys grows with the square of the number of separators.
    if is_long(name) {
        return vec![name];
    }
    let mut variants = vec![name];
    for (i, c) in name.char_indices() {
        if i > 0 && (c == '(' || c == '[') {
            variants.push(name[..i].trim_end());
        }
    }
    if let Some(i) = name.find(" with data set") {
        variants.push(&name[..i]);
    }
    let mut keys = Vec::new();
    for v in variants {
        keys.push(v);
        keys.extend(v.match_indices(' ').map(|(p, _)| &v[p + 1..]));
        // The segments: from the start or after a separator, to a separator or
        // the end.
        let mut starts = vec![0];
        let mut ends = vec![v.len()];
        for sep in SEPS {
            for (p, _) in v.match_indices(sep) {
                starts.push(p + sep.len());
                ends.push(p);
            }
        }
        for &a in &starts {
            for &b in &ends {
                if a < b {
                    keys.push(&v[a..b]);
                }
            }
        }
    }
    keys.retain(|k| !k.is_empty());
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// True for a case name that has no key but its whole name: more than 256
/// bytes, or more than 32 separators (`SEPS`, `(` and `[`).
fn is_long(name: &str) -> bool {
    name.len() > 256
        || SEPS.iter().map(|s| name.matches(s).count()).sum::<usize>()
            + name.matches(['(', '[']).count()
            > 32
}

/// The index of the cases of the reports, for the bind of many tags. It holds,
/// once for all tags, what a bind needs about a case: the cases by the keys of
/// their names, and the test files that each `classname` names. The cost of a
/// bind is then the cost of its test, and not the number of cases times the
/// number of test files.
pub struct Binder<'a> {
    cases: &'a [Case],
    /// A key of a name (`case_keys`), to the cases that have it.
    by_key: HashMap<&'a str, Vec<usize>>,
    /// A whole name, to the cases that have it.
    by_name: HashMap<&'a str, Vec<usize>>,
    /// The second segment of a name with `/`, to the cases that have it: the
    /// method of a Go suite.
    by_second: HashMap<&'a str, Vec<usize>>,
    /// For each case, the way that it fits a test file.
    kind: Vec<Kind>,
    /// The cases with a name that is too long to read (`is_long`).
    long: Vec<usize>,
}

impl<'a> Binder<'a> {
    pub fn new(cases: &'a [Case], test_files: &[String]) -> Binder<'a> {
        let names = Names::new(test_files);
        let mut by_key: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut by_second: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut kind = Vec::with_capacity(cases.len());
        let mut long = Vec::new();
        for (i, c) in cases.iter().enumerate() {
            let name = c.name.as_str();
            if is_long(name) {
                long.push(i);
            }
            for k in case_keys(name) {
                by_key.entry(k).or_default().push(i);
            }
            by_name.entry(name).or_default().push(i);
            if let Some(second) = name.split('/').nth(1) {
                by_second.entry(second).or_default().push(i);
            }
            kind.push(match (&c.file, &c.classname) {
                (Some(f), _) => {
                    let f = f.replace('\\', "/");
                    Kind::File(f.trim_start_matches("./").to_string())
                }
                (None, Some(cn)) => {
                    let (class, named) = names.class(cn);
                    Kind::Class(class, named)
                }
                (None, None) => Kind::Name,
            });
        }
        Binder {
            cases,
            by_key,
            by_name,
            by_second,
            kind,
            long,
        }
    }

    /// How case `i` fits the test file `at`, or `None` when it does not fit
    /// (design 0.8.1, D82 item 6). With `loose`, a case that has no `file` and
    /// whose `classname` names another test file still fits by its name only:
    /// the explicit name of `case:` (item 8).
    fn fit(&self, i: usize, at: &Place, loose: bool) -> Option<Fit> {
        match &self.kind[i] {
            Kind::File(f) => same_file(f, at.path).then_some(Fit::File),
            Kind::Name => Some(Fit::Name),
            Kind::Class(class, named) => {
                // The `classname` of a unit test of Rust is the crate. It fits a
                // file of `src/` by the module path of the name, and no file of
                // `tests/`: that file is another binary (`<crate>::<name>`).
                if let Class::Dotted(..) = class {
                    match &at.rust {
                        RustFile::Src(module) => {
                            let name = &self.cases[i].name;
                            let segs: Vec<&str> = name.split("::").collect();
                            let ok = segs.len() > module.len()
                                && module.iter().zip(&segs).all(|(m, s)| m == s);
                            return ok.then_some(Fit::Name);
                        }
                        RustFile::Test if *named => return None,
                        _ => {}
                    }
                }
                if !named {
                    return Some(Fit::Name);
                }
                let fits = match class {
                    Class::Binary(key) => at.keys.contains(&key.as_str()),
                    Class::Path(p) => same_file(p, at.path),
                    Class::Folder(f) => f.as_deref() == Some(at.folder),
                    Class::Dotted(keys, one) => {
                        keys.iter().any(|k| ends_in_segments(&at.dotted, k))
                            || one.as_deref().is_some_and(|c| {
                                path_ends(at.folder).any(|e| e == c) && !at.folder.is_empty()
                            })
                    }
                };
                if fits {
                    Some(Fit::Classname)
                } else {
                    loose.then_some(Fit::Name)
                }
            }
        }
    }

    /// The cases that can name the test: the cases with its name as a key. The
    /// index is wide, and `name_match` decides.
    fn named_by(&self, test: &TestName) -> Vec<usize> {
        let index = if test.recv.is_some() {
            &self.by_second
        } else {
            &self.by_key
        };
        index.get(test.name.as_str()).cloned().unwrap_or_default()
    }

    /// Binds one tag to the cases of the reports. `lines` are the lines of the
    /// test file, and `tag` is the index of the line of the tag. With `case`,
    /// the tag binds to the cases that have this name. Else it binds to the test
    /// under it (design 0.8.1, D82): the cases that name that test, and no case
    /// that belongs to another test of the file. It also gives the bind: the
    /// test, the fit, the strength and the reason for no case. The suite
    /// functions of Go are those of the file of the tag.
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
        self.bind_with(path, lines, tag, case, &suites)
    }

    /// Like `bind`. `suites` gives the functions that run the Go suite of a type
    /// for a method (`suite_funcs` over the test files of the package).
    pub fn bind_with(
        &self,
        path: &str,
        lines: &[&str],
        tag: usize,
        case: Option<&str>,
        suites: &dyn Fn(&str, &str) -> Vec<String>,
    ) -> (Proof, Bind) {
        let at = Place::new(path);
        let mut bind = Bind::default();
        let mut cand: Vec<usize>;
        let mut tests: Vec<TestName> = Vec::new();
        let mut ours: Option<TestName> = None;
        let mut def_line = tag;
        match case {
            Some("") => {
                // A broken `case:` binds no case, and not the test under the tag.
                bind.by = By::Case;
                return (Proof::Unbound, bind.unbound(Reason::NoCase));
            }
            Some(name) => {
                bind.by = By::Case;
                bind.test = Some(name.to_string());
                cand = self.by_name.get(name).cloned().unwrap_or_default();
                if cand.is_empty() {
                    bind.near = self.near(name);
                }
            }
            None => {
                let (under, line) = if tag < lines.len() {
                    under_tag_line(lines, tag)
                } else {
                    (Under::NoTest(None), tag)
                };
                def_line = line;
                match under {
                    Under::NoTest(line) => {
                        bind.line = line;
                        return (Proof::Unbound, bind.unbound(Reason::NoTest));
                    }
                    Under::Group => return (Proof::Unbound, bind.unbound(Reason::Group)),
                    Under::Template(name) => {
                        bind.test = (!name.is_empty()).then_some(name);
                        return (Proof::Unbound, bind.unbound(Reason::Template));
                    }
                    Under::Test(t) => {
                        bind.test = Some(t.name.clone());
                        cand = self.named_by(&t);
                        ours = Some(t);
                    }
                }
            }
        }
        if let Some(t) = &ours {
            // The index is wide. The name of the test decides.
            cand.retain(|&i| name_match(&self.cases[i].name, t).is_some());
            if let Some(ty) = &t.recv {
                // Only a case of the function that runs the suite binds. With no
                // such function, or when another suite of it has a method of the
                // same name, the tag binds no case.
                let funcs = suites(ty, &t.name);
                cand.retain(|&i| {
                    let first = self.cases[i].name.split('/').next().unwrap_or("");
                    funcs.iter().any(|f| f == first)
                });
            }
        }
        // The titles of the groups of the test: with them, a name that a runner
        // joins with a space is the expected name.
        let titles = ours
            .as_ref()
            .filter(|t| t.call && t.recv.is_none())
            .and_then(|_| ancestors(lines, def_line));
        if ours.is_some() {
            tests = tests_of(lines);
        }
        let loose = ours.is_none();
        let mut fits: Vec<(usize, Fit, bool)> = cand
            .iter()
            .filter_map(|&i| {
                let fit = self.fit(i, &at, loose)?;
                let name = &self.cases[i].name;
                // A name that is a whole segment, or the expected full name.
                let strong = match &ours {
                    None => true,
                    Some(t) => match name_match(name, t) {
                        Some((_, NameFit::Strong)) => true,
                        Some((_, NameFit::Weak)) => titles
                            .as_deref()
                            .is_some_and(|ts| expected_name(name, t, ts)),
                        None => false,
                    },
                };
                Some((i, fit, strong))
            })
            .collect();
        // A case that fits the test file by its `file` or its `classname` hides
        // the cases that fit by their name only (item 6). That is the only fit
        // rule that leaves a case out: no case of the name hides another case
        // of the name (the worst case decides).
        if fits.iter().any(|f| f.1 != Fit::Name) {
            fits.retain(|f| f.1 != Fit::Name);
        }
        let bound: Vec<&Case> = fits.iter().map(|(i, ..)| &self.cases[*i]).collect();
        if bound.is_empty() {
            // A case with a long name has no key but its whole name.
            let long = ours.as_ref().is_some_and(|t| {
                self.long.iter().any(|&i| {
                    self.cases[i].name.contains(t.name.as_str())
                        && self.fit(i, &at, false).is_some()
                })
            });
            let reason = if long {
                Reason::LongName
            } else {
                Reason::NoCase
            };
            return (Proof::Unbound, bind.unbound(reason));
        }
        bind.cases = bound.len();
        let by_name = fits.iter().all(|(_, f, _)| *f == Fit::Name);
        bind.fit = Some(if by_name {
            Fit::Name
        } else if fits.iter().any(|(_, f, _)| *f == Fit::File) {
            Fit::File
        } else {
            Fit::Classname
        });
        // A bind is weak when a case of it fits by its name only, or its name is
        // no whole segment, or more than one case fits. SD621 reports it, unless
        // `case:` names the one case of that name in the reports (item 9).
        // A file with 2 tests of one name has the cases of both: that is no bind
        // by name only.
        let twins = ours
            .as_ref()
            .is_some_and(|t| tests.iter().filter(|x| *x == t).count() > 1);
        let own = ours
            .as_ref()
            .is_none_or(|t| bound.iter().any(|c| is_own(&c.name, t, titles.as_deref())));
        bind.why = if !twins && self.many(&bound, &ours) {
            Some(Why::Many)
        } else if fits.iter().any(|f| f.1 == Fit::Name) {
            Some(Why::ByName)
        } else if !own || fits.iter().any(|f| !f.2) {
            Some(Why::NotWhole)
        } else {
            None
        };
        let weak = bind.why.is_some();
        bind.by_name = weak
            && match case {
                Some(name) => !one_case(
                    &self
                        .by_name
                        .get(name)
                        .map(|v| v.iter().map(|i| &self.cases[*i]).collect::<Vec<_>>())
                        .unwrap_or_default(),
                ),
                None => true,
            };
        // A set that is not one case proves the requirement only when each case
        // passed: the worst case decides, so a skipped or failed case never
        // hides behind a passed one.
        bind.shared = twins || !one_case(&bound);
        let proof = decide(&bound, bind.shared);
        bind.case = match &proof {
            Proof::Passed(n, _) | Proof::Skipped(n) | Proof::Failed(n) => Some(n.clone()),
            Proof::Unbound => None,
        };
        (proof, bind)
    }

    /// True when the cases are more than one test of the reports: they do not
    /// share the name up to the end of the name of the test, the `classname` and
    /// the file, or one name is in one report more than once. The subtests of a
    /// test and the rows of a table (`TestA` and `TestA/row`, `test_a[1]` and
    /// `test_a[2]`) are one test.
    fn many(&self, bound: &[&Case], ours: &Option<TestName>) -> bool {
        let family = |c: &Case| {
            let end = ours
                .as_ref()
                .and_then(|t| name_match(&c.name, t))
                .map_or(c.name.len(), |(e, _)| e);
            (
                c.name[..end].to_string(),
                c.classname.clone(),
                c.file.clone(),
            )
        };
        let Some(first) = bound.first() else {
            return false;
        };
        let one = family(first);
        if bound.iter().any(|c| family(c) != one) {
            return true;
        }
        bound.iter().all(|c| c.name == first.name) && !one_case(bound)
    }

    /// The names of the cases that have `value` as a key: the cases that a
    /// `case:` with a part of a name meant. At most 3.
    fn near(&self, value: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .by_key
            .get(value)
            .into_iter()
            .flatten()
            .map(|i| self.cases[*i].name.clone())
            .collect();
        names.sort();
        names.dedup();
        names.truncate(3);
        names
    }
}

/// What the test files name (design 0.8.1, D82 item 6): the ends of their
/// dotted paths, the ends of their paths, their paths, their folders, the ends
/// of their folders and the keys of their binary ids.
struct Names<'t> {
    ends: HashSet<String>,
    paths: HashSet<&'t str>,
    whole: HashSet<&'t str>,
    folders: HashSet<&'t str>,
    folder_ends: HashSet<&'t str>,
    keys: HashSet<&'t str>,
}

impl<'t> Names<'t> {
    fn new(test_files: &'t [String]) -> Names<'t> {
        let mut n = Names {
            ends: HashSet::new(),
            paths: HashSet::new(),
            whole: HashSet::new(),
            folders: HashSet::new(),
            folder_ends: HashSet::new(),
            keys: HashSet::new(),
        };
        for f in test_files {
            let mut rest = dotted(f);
            loop {
                n.ends.insert(rest.clone());
                match rest.split_once('.') {
                    Some((_, tail)) => rest = tail.to_string(),
                    None => break,
                }
            }
            n.paths.extend(path_ends(f));
            n.whole.insert(f.as_str());
            let folder = folder_of(f);
            if !folder.is_empty() {
                n.folders.insert(folder);
                n.folder_ends.extend(path_ends(folder));
            }
            n.keys.extend(binary_keys(f));
        }
        n
    }

    /// The class of a `classname`, and true if it names a test file.
    fn class(&self, cn: &str) -> (Class, bool) {
        if let Some((_, tail)) = cn.rsplit_once("::") {
            let key = tail.rsplit('/').next().unwrap_or(tail);
            return (Class::Binary(key.to_string()), self.keys.contains(key));
        }
        if cn.contains('/') {
            let norm = cn.replace('\\', "/");
            let norm = norm.trim_start_matches("./");
            let last = norm.rsplit('/').next().unwrap_or(norm);
            if last.contains('.') {
                // The rule of `file`: one path ends the other, or they are equal.
                let named =
                    self.paths.contains(norm) || path_ends(norm).any(|e| self.whole.contains(e));
                return (Class::Path(norm.to_string()), named);
            }
            let longest = path_ends(norm).find(|e| self.folders.contains(e));
            return (
                Class::Folder(longest.map(str::to_string)),
                longest.is_some(),
            );
        }
        // The whole `classname` is a key. A class after the module (pytest) or
        // an inner class (Java) is a trailing segment that starts with an upper
        // case letter: it can go. The part before the last dot of another name
        // is no key (`tests.test_close` names no file `tests.py`).
        let cn = cn.replace('$', ".");
        let mut keys: Vec<String> = vec![cn.clone()];
        let mut whole = cn.as_str();
        while let Some((head, last)) = whole.rsplit_once('.') {
            if !head.is_empty() && last.starts_with(char::is_uppercase) {
                keys.push(head.to_string());
                whole = head;
            } else {
                break;
            }
        }
        let one = (!cn.contains('.')).then(|| cn.clone());
        let named = keys.iter().any(|k| self.ends.contains(k))
            || one.as_deref().is_some_and(|c| self.folder_ends.contains(c));
        (Class::Dotted(keys, one), named)
    }
}

/// True if the cases are one case: one name, one `classname` and one file, and
/// at most once in each report (design 0.8.1, D82 item 7).
fn one_case(cases: &[&Case]) -> bool {
    let Some(first) = cases.first() else {
        return true;
    };
    let same = cases
        .iter()
        .all(|c| c.name == first.name && c.classname == first.classname && c.file == first.file);
    let reports: HashSet<&str> = cases.iter().map(|c| c.report.as_str()).collect();
    same && reports.len() == cases.len()
}

/// The proof of the bound cases. A requirement passed when a bound case passed
/// and no bound case failed. A shared set needs a pass of each case: a skipped
/// case is no pass, so a skipped twin never hides behind a passed twin.
fn decide(bound: &[&Case], shared: bool) -> Proof {
    let named = |o: Outcome| {
        bound
            .iter()
            .find(|c| c.outcome == o)
            .map(|c| c.name.clone())
    };
    if let Some(n) = named(Outcome::Fail) {
        Proof::Failed(n)
    } else if shared && let Some(n) = named(Outcome::Skip) {
        Proof::Skipped(n)
    } else if let Some(n) = named(Outcome::Pass) {
        let (name, runs) = least_passes(bound).unwrap_or((n, 0));
        Proof::Passed(name, runs)
    } else if let Some(n) = named(Outcome::Skip) {
        Proof::Skipped(n)
    } else {
        Proof::Unbound
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
        let (proof, bind) = binder.bind_with(
            &cv.path,
            &lines,
            (cv.line as usize).saturating_sub(1),
            cv.case.as_deref(),
            &suites,
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
        let text =
            "// covers: requirements/valves#close\n#[test]\nfn closes() {\n    assert!(true);\n}\n";
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

    #[test]
    fn a_classname_that_names_no_test_file_binds_by_its_name_only() {
        let text = "# covers: requirements/valves#close\ndef test_init():\n    pass\n";
        let cases = [case(
            "test_init",
            Some("unrelated.name"),
            None,
            Outcome::Pass,
        )];
        let (proof, bind) = bind_of("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]);
        assert_eq!(proof, Proof::Passed("test_init".into(), 1));
        assert_eq!(bind.fit, Some(Fit::Name));
        assert!(bind.by_name && !bind.shared);
    }

    /// The table of the file rule (design 0.8.1, D82 item 6): the test file of
    /// the tag, the other test files, the `classname` and the `file` of the
    /// case, and how the case fits.
    #[test]
    fn the_file_rule_fits_the_case_of_each_runner_to_its_test_file() {
        let py = "# covers: a#b\ndef test_x():\n    pass\n";
        let rs = "// covers: a#b\n#[test]\nfn test_x() {\n}\n";
        let go = "// covers: a#b\nfunc TestX(t *testing.T) {\n}\n";
        let java = "// covers: a#b\n@Test\nvoid testX() {\n}\n";
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
                None,
            ),
            (
                "app/valves/sensor_test.go",
                go,
                &["valves/sensor_test.go", "app/valves/sensor_test.go"],
                "TestX",
                Some("example.org/app/valves"),
                None,
                Some(Fit::Classname),
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
                Some(Fit::Name),
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
                Some(Fit::Classname),
            ),
            // .NET with a namespace that the folders do not follow.
            (
                "tests/ValveTest.cs",
                java,
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

    #[test]
    fn a_case_that_fits_the_file_hides_a_case_that_fits_by_its_name_only() {
        let text = "# covers: a#b\ndef test_init():\n    pass\n";
        let cases = [
            case("test_init", Some("tests.test_a"), None, Outcome::Skip),
            case("test_init", None, None, Outcome::Pass),
        ];
        let (proof, bind) = bind_of("tests/test_a.py", text, None, &cases, &["tests/test_a.py"]);
        assert_eq!(proof, Proof::Skipped("test_init".into()));
        assert_eq!(bind.fit, Some(Fit::Classname));
        assert_eq!(bind.cases, 1);
        assert!(!bind.by_name);
    }

    #[test]
    fn two_tests_of_one_name_make_a_shared_set_that_needs_each_case_to_pass() {
        let text = "class A:\n    # covers: a#b\n    def test_init(self):\n        pass\n\nclass B:\n    def test_init(self):\n        pass\n";
        let two = |a, b| {
            [
                case("test_init", Some("tests.test_a.A"), None, a),
                case("test_init", Some("tests.test_a.B"), None, b),
            ]
        };
        let files = ["tests/test_a.py"];
        let (proof, bind) = bind_of(
            "tests/test_a.py",
            text,
            None,
            &two(Outcome::Skip, Outcome::Pass),
            &files,
        );
        assert_eq!(proof, Proof::Skipped("test_init".into()));
        assert!(bind.shared && !bind.by_name && bind.cases == 2);
        let (proof, _) = bind_of(
            "tests/test_a.py",
            text,
            None,
            &two(Outcome::Pass, Outcome::Pass),
            &files,
        );
        assert_eq!(proof, Proof::Passed("test_init".into(), 1));
        let (proof, _) = bind_of(
            "tests/test_a.py",
            text,
            None,
            &two(Outcome::Fail, Outcome::Pass),
            &files,
        );
        assert_eq!(proof, Proof::Failed("test_init".into()));
        // One test of the name with 2 cases that fit it: the set is shared too,
        // and no case hides the other.
        let one = "# covers: a#b\ndef test_init():\n    pass\n";
        let (proof, bind) = bind_of(
            "tests/test_a.py",
            one,
            None,
            &two(Outcome::Skip, Outcome::Pass),
            &files,
        );
        assert!(bind.shared && bind.by_name);
        assert_eq!(proof, Proof::Skipped("test_init".into()));
    }

    #[test]
    fn cases_that_fit_by_name_only_are_shared_unless_they_are_one_case() {
        let text = "// covers: a#b\nit('closes', () => {\n});\n";
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
        // 2 names that end with the literal.
        let cases = [
            run("V closes", "a", Outcome::Pass),
            run("W closes", "a", Outcome::Skip),
        ];
        let (proof, bind) = bind_of("tests/v.test.js", text, None, &cases, &files);
        assert_eq!(proof, Proof::Skipped("W closes".into()));
        assert!(bind.shared);
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
        // The one case of the name: no SD621.
        assert!(!bind.by_name && !bind.shared);
        // 2 cases of the name in 2 files: a bind by name, and a shared set.
        let cases = [
            case("V closes", Some("V closes"), None, Outcome::Pass),
            case("V closes", Some("V closes"), None, Outcome::Skip),
        ];
        let (proof, bind) = bind_of("t/test_a.py", text, Some("V closes"), &cases, &[]);
        assert_eq!(proof, Proof::Skipped("V closes".into()));
        assert!(bind.by_name && bind.shared);
        // A case with a `classname` of another test file still binds by its
        // name, and a case of the file hides it.
        let cases = [
            case("T", Some("tests.test_b"), None, Outcome::Pass),
            case("T", Some("tests.test_a"), None, Outcome::Skip),
        ];
        let files = ["tests/test_a.py", "tests/test_b.py"];
        let (proof, bind) = bind_of("tests/test_a.py", text, Some("T"), &cases, &files);
        assert_eq!(proof, Proof::Skipped("T".into()));
        assert_eq!(bind.fit, Some(Fit::Classname));
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
        assert_eq!(
            tests_of(&lines),
            [test_name("closes", true), test_name("never closes", true)]
        );
    }

    /// The strength of a match: `Some(true)` for a whole segment, `Some(false)`
    /// for the end after a space, `None` for no match.
    fn strong(case: &str, test: &TestName) -> Option<bool> {
        name_match(case, test).map(|(_, f)| f == NameFit::Strong)
    }

    #[test]
    fn the_name_of_a_test_is_a_whole_segment_of_the_name_of_the_case() {
        let call = test_name("closes", true);
        let def = test_name("closes", false);
        // The form call: the whole name, or a whole segment after a separator.
        assert_eq!(strong("closes", &call), Some(true));
        assert_eq!(strong("Valves > closes", &call), Some(true));
        assert_eq!(strong("Valves \u{203a} closes", &call), Some(true));
        assert_eq!(strong("Valves > closes > more", &call), Some(true));
        // The end after a space is weak. A name that has a segment of ` > ` never
        // matches by its last word.
        assert_eq!(strong("Valves closes", &call), Some(false));
        assert_eq!(strong("Valves > fast closes", &call), None);
        assert_eq!(strong("Valves reopens", &call), None);
        assert_eq!(strong("Valves closes now", &call), None);
        assert_eq!(strong("closes (slow)", &call), None);
        assert_eq!(strong("closes()", &call), None);
        // An identifier: a whole segment before its parameters. A space is no
        // separator, and a tail starts right after the name.
        assert_eq!(strong("closes", &def), Some(true));
        assert_eq!(strong("tests::closes", &def), Some(true));
        assert_eq!(strong("closes::case_1", &def), Some(true));
        assert_eq!(strong("closes[1-2]", &def), Some(true));
        assert_eq!(strong("closes/no_reply", &def), Some(true));
        assert_eq!(strong("closes(int) [1] a", &def), Some(true));
        assert_eq!(strong("closes with data set #0", &def), Some(true));
        assert_eq!(strong("Valves closes", &def), None);
        assert_eq!(
            strong("Closes when cold", &test_name("Closes", false)),
            None
        );
        assert_eq!(strong("Closes (cold)", &test_name("Closes", false)), None);
        assert_eq!(strong("closes_late", &def), None);
        // A Kotlin name in backticks is whole, and the parameters follow it.
        let kt = TestName {
            bt: true,
            ..test_name("stops (fast)", true)
        };
        assert_eq!(strong("stops (fast)", &kt), Some(true));
        assert_eq!(strong("stops (fast)()", &kt), Some(true));
        assert_eq!(strong("stops (fast)(int) [1] 3", &kt), Some(true));
        assert_eq!(strong("stops()", &kt), None);
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
            assert!(name_match(name, &test).is_some(), "{name} / {}", test.name);
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
        assert!(name_match("Valves > never closes", &tests[0]).is_none());
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
