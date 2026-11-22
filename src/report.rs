//! Test reports as evidence (design 0.8.0, D1). This module holds the 2
//! readers of a report: a subset of JUnit XML, and the line format. It also
//! holds the stamp that binds a report to the code tree, and the read of all
//! reports of a project.
//!
//! The XML reader is code of spec-dna with no dependency. It reads UTF-8 only.
//! It never loads an external entity, because a DTD is an error.
//!
//! A report counts as proof only when all these are true: it is readable, the
//! git index does not hold it, it has a stamp, and the stamp names the current
//! code tree and the bytes of the report (`read`).

use crate::config::Config;
use crate::diag::{Code, Diag, Severity};
use crate::glob;
use crate::hash;
use crate::repo::{Fatal, Repo};
use crate::tree;
use std::collections::BTreeSet;
use std::path::Path;

/// The largest report that spec-dna reads.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The first line of the line format.
pub const LINES_HEADER: &str = "spec-dna-report 1";

/// The first line of a stamp.
pub const STAMP_HEADER: &str = "spec-dna-stamp 1";

/// The result of one test case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    Skip,
}

/// One test case of a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The name of the case, as the runner writes it.
    pub name: String,
    /// The `classname` of a JUnit case.
    pub classname: Option<String>,
    /// The path of the test file, if the runner writes it.
    pub file: Option<String>,
    pub outcome: Outcome,
    /// The report file that holds the case (empty after a plain `parse`).
    pub report: String,
}

/// The format of a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Junit,
    Lines,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Junit => "junit",
            Format::Lines => "lines",
        }
    }
}

/// Detects the format from the first bytes, and reads the cases.
pub fn parse(bytes: &[u8]) -> Result<(Format, Vec<Case>), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "the report is not UTF-8 text; spec-dna reads UTF-8 only".to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let head = text.trim_start();
    if head.starts_with(LINES_HEADER) {
        Ok((Format::Lines, parse_lines(text)?))
    } else if head.starts_with('<') {
        Ok((Format::Junit, parse_junit(text)?))
    } else {
        Err(format!(
            "unknown format: a report is JUnit XML (it starts with `<`) or the line format (its first line is `{LINES_HEADER}`)"
        ))
    }
}

/// Reads the line format: the header line, then `<pass|fail|skip>`, a tab, a
/// path or `-`, a tab and the name of the case. A blank line and a line that
/// starts with `#` do not count.
fn parse_lines(text: &str) -> Result<Vec<Case>, String> {
    let mut out = Vec::new();
    let mut first = true;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if first {
            if line.trim() != LINES_HEADER {
                return Err(format!("line 1 must be `{LINES_HEADER}`"));
            }
            first = false;
            continue;
        }
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let (Some(status), Some(path), Some(name)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(format!(
                "line {}: expected `<pass|fail|skip>`, a tab, a path or `-`, a tab and a name",
                i + 1
            ));
        };
        let outcome = match status {
            "pass" => Outcome::Pass,
            "fail" => Outcome::Fail,
            "skip" => Outcome::Skip,
            other => {
                return Err(format!(
                    "line {}: `{other}` is not `pass`, `fail` or `skip`",
                    i + 1
                ));
            }
        };
        if name.is_empty() {
            return Err(format!("line {}: the name of the case is empty", i + 1));
        }
        out.push(Case {
            name: name.to_string(),
            classname: None,
            file: (path != "-" && !path.is_empty()).then(|| path.to_string()),
            outcome,
            report: String::new(),
        });
    }
    if first {
        return Err(format!(
            "the report is empty; line 1 must be `{LINES_HEADER}`"
        ));
    }
    Ok(out)
}

/// An XML attribute list: the pairs of name and decoded value.
type Attrs = Vec<(String, String)>;

fn attr<'a>(attrs: &'a Attrs, name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

/// Decodes the 5 predefined entities and the numeric ones. Any other entity is
/// an error, because the reader has no DTD.
fn decode(raw: &str) -> Result<String, String> {
    if !raw.contains('&') {
        return Ok(raw.to_string());
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        let end = tail
            .find(';')
            .ok_or_else(|| "an `&` with no `;`".to_string())?;
        let name = &tail[..end];
        match name {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let code = if let Some(h) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
                    u32::from_str_radix(h, 16).ok()
                } else if let Some(d) = name.strip_prefix('#') {
                    d.parse::<u32>().ok()
                } else {
                    None
                };
                let c = code
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("the entity `&{name};` is not defined"))?;
                out.push(c);
            }
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// One event of the XML reader.
enum Ev {
    Start(String, Attrs, bool),
    End(String),
}

/// A pull reader of the XML subset. It skips text, comments, CDATA and
/// processing instructions, and it rejects a DTD.
struct Xml<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Xml<'a> {
    fn next(&mut self) -> Result<Option<Ev>, String> {
        loop {
            let Some(lt) = self.s[self.pos..].find('<') else {
                return Ok(None);
            };
            self.pos += lt;
            let rest = &self.s[self.pos..];
            if rest.starts_with("<!--") {
                let end = rest
                    .find("-->")
                    .ok_or("a comment is not closed (the file is cut)")?;
                self.pos += end + 3;
            } else if rest.starts_with("<![CDATA[") {
                let end = rest
                    .find("]]>")
                    .ok_or("a CDATA section is not closed (the file is cut)")?;
                self.pos += end + 3;
            } else if rest.starts_with("<?") {
                let end = rest
                    .find("?>")
                    .ok_or("a processing instruction is not closed (the file is cut)")?;
                self.pos += end + 2;
            } else if rest
                .as_bytes()
                .get(..9)
                .is_some_and(|b| b.eq_ignore_ascii_case(b"<!DOCTYPE"))
            {
                return Err("the report has a DTD (`<!DOCTYPE`); spec-dna rejects it".to_string());
            } else if rest.starts_with("<!") {
                return Err("an unknown `<!` construct".to_string());
            } else if let Some(tail) = rest.strip_prefix("</") {
                let end = tail
                    .find('>')
                    .ok_or("an end tag is not closed (the file is cut)")?;
                let name = tail[..end].trim().to_string();
                self.pos += 2 + end + 1;
                return Ok(Some(Ev::End(name)));
            } else {
                return self.start_tag().map(Some);
            }
        }
    }

    fn start_tag(&mut self) -> Result<Ev, String> {
        let bytes = self.s.as_bytes();
        let mut i = self.pos + 1;
        let name_start = i;
        while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>') {
            i += 1;
        }
        let name = self.s[name_start..i].to_string();
        if name.is_empty() {
            return Err("a `<` with no element name".to_string());
        }
        let mut attrs: Attrs = Vec::new();
        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let Some(&b) = bytes.get(i) else {
                return Err("a start tag is not closed (the file is cut)".to_string());
            };
            if b == b'>' {
                self.pos = i + 1;
                return Ok(Ev::Start(name, attrs, false));
            }
            if b == b'/' {
                if bytes.get(i + 1) != Some(&b'>') {
                    return Err(format!("a `/` in the start tag of `{name}` has no `>`"));
                }
                self.pos = i + 2;
                return Ok(Ev::Start(name, attrs, true));
            }
            let key_start = i;
            while i < bytes.len()
                && !matches!(bytes[i], b'=' | b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/')
            {
                i += 1;
            }
            let key = self.s[key_start..i].to_string();
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) != Some(&b'=') {
                return Err(format!("the attribute `{key}` of `{name}` has no value"));
            }
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let quote = match bytes.get(i) {
                Some(&q @ (b'"' | b'\'')) => q,
                _ => return Err(format!("the value of `{key}` has no quote")),
            };
            i += 1;
            let val_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i >= bytes.len() {
                return Err("an attribute value is not closed (the file is cut)".to_string());
            }
            let value = decode(&self.s[val_start..i])?;
            i += 1;
            attrs.push((key, value));
        }
    }
}

/// A `testsuite` that is open: its declared count and the counts that the
/// reader has seen.
struct Suite {
    name: String,
    declared: Option<String>,
    direct: usize,
    nested: usize,
}

/// A `testcase` that is open.
struct Open {
    case: Case,
    failed: bool,
    skipped: bool,
}

/// Reads the JUnit subset: the elements `testsuites`, `testsuite` and
/// `testcase`, the attributes `name`, `classname` and `file`, and the children
/// `failure`, `error`, `skipped`, `flakyFailure`, `flakyError`, `rerunFailure`
/// and `rerunError`. A `testsuite` whose `tests` count is not its number of
/// `testcase` elements is a cut file.
fn parse_junit(text: &str) -> Result<Vec<Case>, String> {
    let mut xml = Xml { s: text, pos: 0 };
    let mut stack: Vec<String> = Vec::new();
    let mut suites: Vec<Suite> = Vec::new();
    let mut open: Option<Open> = None;
    let mut cases: Vec<Case> = Vec::new();
    let mut root_seen = false;
    while let Some(ev) = xml.next()? {
        match ev {
            Ev::Start(name, attrs, empty) => {
                if stack.is_empty() {
                    if root_seen {
                        return Err("more than one root element".to_string());
                    }
                    root_seen = true;
                    if name != "testsuites" && name != "testsuite" {
                        return Err(format!(
                            "the root element is `{name}`, not `testsuites` or `testsuite`"
                        ));
                    }
                }
                match name.as_str() {
                    "testsuite" => suites.push(Suite {
                        name: attr(&attrs, "name").unwrap_or("").to_string(),
                        declared: attr(&attrs, "tests").map(str::to_string),
                        direct: 0,
                        nested: 0,
                    }),
                    "testcase" => {
                        if open.is_some() {
                            return Err("a `testcase` inside a `testcase`: the file is not a report of a runner".to_string());
                        }
                        let case_name = attr(&attrs, "name")
                            .filter(|n| !n.is_empty())
                            .ok_or("a `testcase` has no `name`")?;
                        open = Some(Open {
                            case: Case {
                                name: case_name.to_string(),
                                classname: attr(&attrs, "classname")
                                    .filter(|c| !c.is_empty())
                                    .map(str::to_string),
                                file: attr(&attrs, "file")
                                    .filter(|f| !f.is_empty())
                                    .map(str::to_string),
                                outcome: Outcome::Pass,
                                report: String::new(),
                            },
                            failed: false,
                            skipped: false,
                        });
                        if let Some(s) = suites.last_mut() {
                            s.direct += 1;
                        }
                    }
                    "failure" | "error" | "flakyFailure" | "flakyError" | "rerunFailure"
                    | "rerunError" => {
                        if let Some(o) = open.as_mut() {
                            o.failed = true;
                        }
                    }
                    "skipped" => {
                        if let Some(o) = open.as_mut() {
                            o.skipped = true;
                        }
                    }
                    _ => {}
                }
                if empty {
                    close(&name, &mut suites, &mut open, &mut cases)?;
                } else {
                    stack.push(name);
                }
            }
            Ev::End(name) => {
                match stack.pop() {
                    Some(top) if top == name => {}
                    Some(top) => {
                        return Err(format!(
                            "`</{name}>` closes `<{top}>`: the XML is not well formed"
                        ));
                    }
                    None => return Err(format!("`</{name}>` has no start tag")),
                }
                close(&name, &mut suites, &mut open, &mut cases)?;
            }
        }
    }
    if !stack.is_empty() {
        return Err(format!(
            "the element `{}` is not closed (the file is cut)",
            stack.last().map_or("", String::as_str)
        ));
    }
    if !root_seen {
        return Err("the report has no root element".to_string());
    }
    Ok(cases)
}

/// Handles the end of an element: the end of a case or of a suite.
fn close(
    name: &str,
    suites: &mut Vec<Suite>,
    open: &mut Option<Open>,
    cases: &mut Vec<Case>,
) -> Result<(), String> {
    match name {
        "testcase" => {
            if let Some(mut o) = open.take() {
                o.case.outcome = if o.failed {
                    Outcome::Fail
                } else if o.skipped {
                    Outcome::Skip
                } else {
                    Outcome::Pass
                };
                cases.push(o.case);
            }
        }
        "testsuite" => {
            if let Some(s) = suites.pop() {
                if let Some(d) = &s.declared {
                    let ok = d
                        .trim()
                        .parse::<usize>()
                        .is_ok_and(|n| n == s.direct || n == s.direct + s.nested);
                    if !ok {
                        return Err(format!(
                            "the `testsuite` `{}` declares tests=\"{d}\" and has {} `testcase` element(s): the file is cut or edited",
                            s.name,
                            s.direct + s.nested
                        ));
                    }
                }
                let total = s.direct + s.nested;
                if let Some(parent) = suites.last_mut() {
                    parent.nested += total;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// The stamp of a report: the tree hash and the hash of the report bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub tree: String,
    pub report: String,
}

/// The hash of the bytes of a report, as 16 hexadecimal digits.
pub fn report_hash(bytes: &[u8]) -> String {
    hash::hex(&hash::hash_bytes(bytes))
}

impl Stamp {
    pub fn render(&self) -> String {
        format!(
            "{STAMP_HEADER}\ntree {}\nreport {}\n",
            self.tree, self.report
        )
    }

    pub fn parse(text: &str) -> Result<Stamp, String> {
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
        if lines.next() != Some(STAMP_HEADER) {
            return Err(format!("line 1 must be `{STAMP_HEADER}`"));
        }
        let mut tree = None;
        let mut report = None;
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let (key, value) = line
                .split_once(' ')
                .ok_or_else(|| format!("`{line}` is not `<key> <hash>`"))?;
            let ok = value.len() == 16 && value.bytes().all(|b| b.is_ascii_hexdigit());
            if !ok {
                return Err(format!("`{value}` is not a hash of 16 hexadecimal digits"));
            }
            match key {
                "tree" => tree = Some(value.to_string()),
                "report" => report = Some(value.to_string()),
                other => return Err(format!("unknown key `{other}`")),
            }
        }
        match (tree, report) {
            (Some(tree), Some(report)) => Ok(Stamp { tree, report }),
            _ => Err("the stamp needs the lines `tree` and `report`".to_string()),
        }
    }
}

/// The path of the stamp of a report.
pub fn stamp_path(report: &str) -> String {
    format!("{report}{}", tree::STAMP_SUFFIX)
}

/// Finds the files that `[evidence] reports` matches, from the repository
/// root, on the disk (also the files that git ignores). The result is sorted,
/// with no duplicate, and it has no stamp file. A glob walks the folder of its
/// literal prefix; it follows no link. A folder that does not exist has no
/// file. A folder that exists and that cannot be read is an error: it must not
/// look like a project with no report.
pub fn find(root: &Path, globs: &[String]) -> Result<Vec<String>, String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    for g in globs {
        let g = g.trim_start_matches("./").trim_matches('/');
        if g.is_empty() {
            continue;
        }
        let segs: Vec<&str> = g.split('/').collect();
        let lit = segs
            .iter()
            .take_while(|s| !glob::is_glob(s))
            .count()
            .min(segs.len());
        if lit == segs.len() {
            let full = root.join(g);
            match std::fs::symlink_metadata(&full) {
                Ok(m) if m.is_file() => {
                    found.insert(g.to_string());
                }
                Err(e)
                    if !matches!(
                        e.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                    ) =>
                {
                    return Err(format!("cannot read the report path `{g}`: {e}"));
                }
                _ => {}
            }
            continue;
        }
        let prefix = segs[..lit].join("/");
        walk(root, &prefix, g, &mut found)?;
    }
    Ok(found
        .into_iter()
        .filter(|p| !p.ends_with(tree::STAMP_SUFFIX))
        .collect())
}

fn walk(root: &Path, rel: &str, pattern: &str, found: &mut BTreeSet<String>) -> Result<(), String> {
    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    let read = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        // No folder, no file. A file where the folder should be is the same.
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(());
        }
        Err(e) => {
            return Err(format!(
                "cannot read the folder `{}` of `[evidence] reports`: {e}",
                if rel.is_empty() { "." } else { rel }
            ));
        }
    };
    for entry in read {
        let entry = entry
            .map_err(|e| format!("cannot read the folder `{rel}` of `[evidence] reports`: {e}"))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let Ok(ty) = entry.file_type() else {
            continue;
        };
        if ty.is_dir() {
            if name != ".git" {
                walk(root, &path, pattern, found)?;
            }
        } else if ty.is_file() && glob::matches(pattern, &path) {
            found.insert(path);
        }
    }
    Ok(())
}

/// A report that `stamp` or `read` read.
pub struct Loaded {
    pub path: String,
    pub bytes: Vec<u8>,
    pub format: Format,
    pub cases: Vec<Case>,
}

/// Reads one report file: the size limit, then the format.
pub fn load_file(root: &Path, rel: &str) -> Result<Loaded, String> {
    let full = root.join(rel);
    let meta = std::fs::metadata(&full).map_err(|e| format!("cannot read the file: {e}"))?;
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "the file has {} bytes; spec-dna reads at most {MAX_BYTES} bytes (64 MiB)",
            meta.len()
        ));
    }
    let bytes = std::fs::read(&full).map_err(|e| format!("cannot read the file: {e}"))?;
    let (format, mut cases) = parse(&bytes)?;
    for c in &mut cases {
        c.report = rel.to_string();
    }
    Ok(Loaded {
        path: rel.to_string(),
        bytes,
        format,
        cases,
    })
}

/// A finding of the read: the code, the file and the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub code: Code,
    pub path: String,
    pub message: String,
}

impl Problem {
    /// The diagnostic of the problem, with the severity of the configuration.
    /// `None` if the code is off.
    pub fn diag(&self, config: &Config) -> Option<Diag> {
        let severity: Severity = config.severity(self.code, config.root_of(&self.path))?;
        Some(Diag {
            path: self.path.clone(),
            line: 1,
            code: self.code,
            doc: self.path.clone(),
            target: String::new(),
            message: self.message.clone(),
            severity,
        })
    }
}

/// The result of the read of all reports of a project.
#[derive(Default)]
pub struct Reads {
    /// The hash of the code tree now.
    pub tree: String,
    /// The number of files that `[evidence] reports` matches.
    pub files: usize,
    /// The cases of the reports that are readable, untracked, stamped and
    /// fresh. A case is one entry of a report: a case that runs again has one
    /// entry in each report.
    pub cases: Vec<Case>,
    /// The number of reports that are fresh.
    pub fresh: usize,
    /// The test files of the code tree, and the files that git does not track
    /// yet, for the bind by `classname`.
    pub test_files: Vec<String>,
    pub problems: Vec<Problem>,
}

impl Reads {
    /// The diagnostics of the problems, with the severities of the
    /// configuration. A code that is off gives none.
    pub fn diags(&self, config: &Config) -> Vec<Diag> {
        self.problems
            .iter()
            .filter_map(|p| p.diag(config))
            .collect()
    }
}

/// Reads all reports: SD613 (no file, no stamp), SD614 (a stamp of another tree
/// or of other bytes) and SD615 (an unreadable report, or a report that git
/// tracks). With an empty `[evidence] reports`, it reads nothing.
pub fn read(repo: &Repo, config: &Config) -> Result<Reads, Fatal> {
    let mut out = Reads::default();
    if config.evidence_reports.is_empty() {
        return Ok(out);
    }
    let (now, paths) = tree::compute(repo, config).map_err(Fatal::Io)?;
    out.tree = now;
    out.test_files = paths
        .into_iter()
        .filter(|p| config.is_test_file(p))
        .collect();
    // A test file that git does not track yet is not in the code tree, but it
    // is a test file: it must name its `classname` as any other test file does.
    // Else a case of a new file would bind to a tag of any file, and a skipped
    // test could count as proven by a passed case of the same name. The tree
    // hash does not change. More names only narrow a bind. (`--reports` does not
    // combine with `--staged`, so the read always sees the working tree.)
    let usable = config.evidence_tests.iter().all(|g| {
        let g = g.trim_matches('/');
        !g.is_empty() && !g.split('/').any(|s| s == ".." || s == ".")
    });
    // A glob that git does not accept as a pathspec turns the limit off. The
    // filter below keeps only the test files.
    let specs: Vec<String> = if usable {
        config
            .evidence_tests
            .iter()
            .map(|g| format!(":(glob){}", g.trim_matches('/')))
            .collect()
    } else {
        Vec::new()
    };
    let known: std::collections::HashSet<String> = out.test_files.iter().cloned().collect();
    let intent = crate::lifecycle::intent_dirs(config);
    for p in repo.git.list_worktree(&specs).map_err(Fatal::Io)? {
        if !known.contains(&p)
            && config.is_test_file(&p)
            && tree::is_code_path(config, &p, &intent)
            && repo.root.join(&p).is_file()
        {
            out.test_files.push(p);
        }
    }
    let tracked = tree::tracked_reports(repo, config).map_err(Fatal::Io)?;
    for t in &tracked {
        out.problems.push(Problem {
            code: Code::ReportUnreadable,
            path: t.clone(),
            message: format!(
                "git tracks `{t}`, and a committed report or stamp can be a hand-written one; remove it from git (`git rm --cached`) and add it to `.gitignore`"
            ),
        });
    }
    let files = find(&repo.root, &config.evidence_reports).map_err(Fatal::Io)?;
    out.files = files.len();
    if files.is_empty() && tracked.is_empty() {
        out.problems.push(Problem {
            code: Code::ReportMissing,
            path: crate::config::FILE_NAME.to_string(),
            message: format!(
                "no file matches `[evidence] reports` ({}); run the tests so that they write the report, then run `spec-dna evidence stamp`",
                config
                    .evidence_reports
                    .iter()
                    .map(|g| format!("`{g}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    for f in files.iter().filter(|f| !tracked.contains(f)) {
        let loaded = match load_file(&repo.root, f) {
            Ok(l) => l,
            Err(why) => {
                out.problems.push(Problem {
                    code: Code::ReportUnreadable,
                    path: f.clone(),
                    message: format!("the report `{f}` is unreadable: {why}"),
                });
                continue;
            }
        };
        let sp = stamp_path(f);
        if tracked.contains(&sp) {
            continue;
        }
        let stamp = match std::fs::read(repo.root.join(&sp)) {
            Ok(b) => match Stamp::parse(&String::from_utf8_lossy(&b)) {
                Ok(s) => s,
                Err(why) => {
                    out.problems.push(Problem {
                        code: Code::ReportUnreadable,
                        path: sp.clone(),
                        message: format!("the stamp `{sp}` is unreadable: {why}"),
                    });
                    continue;
                }
            },
            Err(_) => {
                out.problems.push(Problem {
                    code: Code::ReportMissing,
                    path: f.clone(),
                    message: format!(
                        "the report `{f}` has no stamp; run `spec-dna evidence stamp` after the tests, in the same job"
                    ),
                });
                continue;
            }
        };
        if stamp.report != report_hash(&loaded.bytes) {
            out.problems.push(Problem {
                code: Code::ReportStale,
                path: f.clone(),
                message: format!(
                    "the bytes of the report `{f}` changed after its stamp; run the tests, then `spec-dna evidence stamp`"
                ),
            });
            continue;
        }
        if !tree::matches(&stamp.tree, &out.tree) {
            out.problems.push(Problem {
                code: Code::ReportStale,
                path: f.clone(),
                message: format!(
                    "the stamp of `{f}` names the code tree {}, and the code tree now is {}; the code changed after the tests. Run the tests, then `spec-dna evidence stamp`",
                    stamp.tree, out.tree
                ),
            });
            continue;
        }
        out.cases.extend(loaded.cases);
        out.fresh += 1;
    }
    Ok(out)
}

/// The counts of a report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub pass: usize,
    pub fail: usize,
    pub skip: usize,
}

impl Counts {
    pub fn of(cases: &[Case]) -> Counts {
        let mut c = Counts::default();
        for case in cases {
            match case.outcome {
                Outcome::Pass => c.pass += 1,
                Outcome::Fail => c.fail += 1,
                Outcome::Skip => c.skip += 1,
            }
        }
        c
    }
}

/// The result of `evidence stamp`.
pub struct Stamped {
    pub tree: String,
    /// Each stamped report with its format and counts.
    pub reports: Vec<(String, Format, Counts)>,
}

/// `spec-dna evidence stamp`: reads each report, and writes `<report>.stamp`
/// with the tree hash and the hash of the report bytes. A report that is
/// unreadable, or a report that git tracks, stops it before any write. The
/// errors are `Fatal::Config` (no `reports` key, exit code 3), `Fatal::Refused`
/// (no file, or an unreadable file, exit code 1) and `Fatal::Io` (exit code 4).
pub fn stamp(repo: &Repo, config: &Config) -> Result<Stamped, Fatal> {
    if config.evidence_reports.is_empty() {
        return Err(Fatal::Config(
            "`evidence stamp` needs `[evidence] reports` in spec-dna.toml: set it to the globs of the report files".to_string(),
        ));
    }
    let files = find(&repo.root, &config.evidence_reports).map_err(Fatal::Io)?;
    if files.is_empty() {
        return Err(Fatal::Refused(format!(
            "no file matches `[evidence] reports` ({}); run the tests first. Nothing got a stamp",
            config
                .evidence_reports
                .iter()
                .map(|g| format!("`{g}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let tracked = tree::tracked_reports(repo, config).map_err(Fatal::Io)?;
    let mut why: Vec<String> = Vec::new();
    for t in &tracked {
        why.push(format!("git tracks `{t}`"));
    }
    let mut loaded: Vec<Loaded> = Vec::new();
    for f in files.iter().filter(|f| !tracked.contains(f)) {
        match load_file(&repo.root, f) {
            Ok(l) => loaded.push(l),
            Err(e) => why.push(format!("`{f}`: {e}")),
        }
    }
    if !why.is_empty() {
        return Err(Fatal::Refused(format!(
            "a report is unreadable, so nothing got a stamp: {}",
            why.join("; ")
        )));
    }
    let (now, _) = tree::compute(repo, config).map_err(Fatal::Io)?;
    let mut reports = Vec::new();
    for l in &loaded {
        let stamp = Stamp {
            tree: now.clone(),
            report: report_hash(&l.bytes),
        };
        std::fs::write(repo.root.join(stamp_path(&l.path)), stamp.render())
            .map_err(|e| Fatal::Io(format!("cannot write the stamp of `{}`: {e}", l.path)))?;
        reports.push((l.path.clone(), l.format, Counts::of(&l.cases)));
    }
    Ok(Stamped { tree: now, reports })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases(text: &str) -> Vec<(String, Outcome)> {
        parse(text.as_bytes())
            .unwrap()
            .1
            .into_iter()
            .map(|c| (c.name, c.outcome))
            .collect()
    }

    #[test]
    fn the_junit_reader_reads_the_outcomes() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- a comment -->
<testsuites>
    <testsuite name="s" tests="6">
        <testcase classname="a.B" name="ok"/>
        <testcase classname="a.B" name="bad"><failure message="x">text</failure></testcase>
        <testcase classname="a.B" name="err"><error/></testcase>
        <testcase classname="a.B" name="skip"><skipped/></testcase>
        <testcase classname="a.B" name="flaky"><flakyFailure message="m"><![CDATA[<x>]]></flakyFailure></testcase>
        <testcase classname="a.B" name="rerun"><rerunError/></testcase>
    </testsuite>
</testsuites>"#;
        assert_eq!(
            cases(xml),
            [
                ("ok".to_string(), Outcome::Pass),
                ("bad".to_string(), Outcome::Fail),
                ("err".to_string(), Outcome::Fail),
                ("skip".to_string(), Outcome::Skip),
                ("flaky".to_string(), Outcome::Fail),
                ("rerun".to_string(), Outcome::Fail),
            ]
        );
    }

    #[test]
    fn a_failure_wins_over_a_skip_in_one_case() {
        let xml = r#"<testsuite tests="1"><testcase name="x"><skipped/><failure/></testcase></testsuite>"#;
        assert_eq!(cases(xml), [("x".to_string(), Outcome::Fail)]);
    }

    #[test]
    fn the_junit_reader_decodes_entities_and_attributes() {
        let xml = r#"<testsuite><testcase name="a &lt;b&gt; &amp; &quot;c&quot; &apos;d&apos; &#65;&#x42;" file='t/x.py' classname="k"/></testsuite>"#;
        let (_, c) = parse(xml.as_bytes()).unwrap();
        assert_eq!(c[0].name, "a <b> & \"c\" 'd' AB");
        assert_eq!(c[0].file.as_deref(), Some("t/x.py"));
        assert_eq!(c[0].classname.as_deref(), Some("k"));
    }

    #[test]
    fn the_junit_reader_rejects_a_bad_file() {
        let bad = |xml: &str| parse(xml.as_bytes()).unwrap_err();
        assert!(bad("<!DOCTYPE x><testsuite/>").contains("DTD"));
        assert!(bad("<!doctype x><testsuite/>").contains("DTD"));
        // A `<!` and text that is not ASCII is an error, not a panic.
        assert!(bad("<!\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}x><testsuite/>").contains("unknown"));
        assert!(bad("<testsuite><testcase name=\"a\"/>").contains("cut"));
        assert!(bad("<testsuite tests=\"2\"><testcase name=\"a\"/></testsuite>").contains("tests"));
        assert!(bad("<testsuite><testcase name=\"a&nbsp;\"/></testsuite>").contains("entity"));
        assert!(bad("<testsuite><testcase/></testsuite>").contains("name"));
        assert!(bad("<html/>").contains("root element"));
        assert!(bad("<testsuite><testcase name=\"a\"></testsuite>").contains("well formed"));
        assert!(
            bad("<testsuite><testcase name=\"a\"/></testsuite><testsuite/>")
                .contains("more than one")
        );
        assert!(bad("<testsuite><testcase name=\"a").contains("cut"));
        assert!(bad("plain text").contains("unknown format"));
        assert!(parse(&[0xff, 0xfe, 0x3c]).unwrap_err().contains("UTF-8"));
    }

    #[test]
    fn a_nested_suite_counts_its_cases() {
        let xml = r#"<testsuites><testsuite name="o" tests="2"><testsuite name="i" tests="1"><testcase name="a"/></testsuite><testcase name="b"/></testsuite></testsuites>"#;
        assert_eq!(cases(xml).len(), 2);
        // The outer count may also be the sum of the nested ones.
        let xml = r#"<testsuite name="o" tests="2"><testsuite name="i" tests="2"><testcase name="a"/><testcase name="b"/></testsuite></testsuite>"#;
        assert_eq!(cases(xml).len(), 2);
    }

    #[test]
    fn the_line_format_reads_a_case_for_each_line() {
        let text = "spec-dna-report 1\n# a comment\npass\ttests/test_valves.py\ttest_closes_on_timeout\n\nfail\t-\tValveTest.flushesFirst\r\nskip\ttests/sensor_test.go\tTestTimeout/no_reply\n";
        let (f, c) = parse(text.as_bytes()).unwrap();
        assert_eq!(f, Format::Lines);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].file.as_deref(), Some("tests/test_valves.py"));
        assert_eq!(c[1].file, None);
        assert_eq!(c[1].name, "ValveTest.flushesFirst");
        assert_eq!(c[2].outcome, Outcome::Skip);
        assert!(parse(b"spec-dna-report 1\nmaybe\tx\ty\n").is_err());
        assert!(parse(b"spec-dna-report 1\npass\tx\n").is_err());
        assert!(parse(b"spec-dna-report 2\n").is_err());
    }

    #[test]
    fn the_stamp_round_trips_and_rejects_bad_text() {
        let s = Stamp {
            tree: "3f2a9c1b7e0d4a11".into(),
            report: "9c1b7e0d4a113f2a".into(),
        };
        assert_eq!(Stamp::parse(&s.render()).unwrap(), s);
        assert!(Stamp::parse("spec-dna-stamp 1\ntree 3f2a9c1b7e0d4a11\n").is_err());
        assert!(Stamp::parse("spec-dna-stamp 1\ntree xyz\nreport 9c1b7e0d4a113f2a\n").is_err());
        assert!(Stamp::parse("wrong\n").is_err());
    }

    #[test]
    fn find_matches_globs_on_the_disk_and_skips_stamps() {
        let dir = std::env::temp_dir().join(format!("spec-dna-report-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("out/deep")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        for f in [
            "out/a.xml",
            "out/a.xml.stamp",
            "out/deep/b.xml",
            "out/c.txt",
            "plain.xml",
            ".git/d.xml",
        ] {
            std::fs::write(dir.join(f), "x").unwrap();
        }
        let globs: Vec<String> = [
            "out/*.xml",
            "out/deep/**",
            "plain.xml",
            "missing.xml",
            "**/*.xml",
        ]
        .map(String::from)
        .to_vec();
        let got = find(&dir, &globs).unwrap();
        assert_eq!(got, ["out/a.xml", "out/deep/b.xml", "plain.xml"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_report_over_64_mib_is_not_read() {
        let dir = std::env::temp_dir().join(format!("spec-dna-report-big-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A sparse file: it holds no byte on the disk.
        let f = std::fs::File::create(dir.join("big.xml")).unwrap();
        f.set_len(MAX_BYTES + 1).unwrap();
        let err = load_file(&dir, "big.xml").err().unwrap();
        assert!(err.contains("64 MiB"), "{err}");
        std::fs::write(dir.join("ok.xml"), "<testsuite/>").unwrap();
        assert!(load_file(&dir, "ok.xml").is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_testcase_in_a_testcase_is_an_error() {
        let xml = r#"<testsuite tests="2"><testcase name="a"><testcase name="b"/></testcase></testsuite>"#;
        let err = parse(xml.as_bytes()).unwrap_err();
        assert!(err.contains("inside"), "{err}");
        // A testcase after a testcase is fine, also with a failure in the first.
        let ok = r#"<testsuite tests="2"><testcase name="a"><failure/></testcase><testcase name="b"/></testsuite>"#;
        assert_eq!(parse(ok.as_bytes()).unwrap().1.len(), 2);
    }
}
