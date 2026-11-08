//! Evidence of requirements (docs/config.md, "Evidence"). A requirement has
//! evidence if one of these links it to a test or a verification record:
//!
//! - `verified-by` in the front matter of its document (every requirement of
//!   the document);
//! - an inline `<!-- verified-by: <path> -->` marker in the requirement;
//! - a `covers: <doc-id>#<key>` tag in a test file. The tag is plain text, so
//!   it works in a comment of any language.
//!
//! spec-dna checks that the links resolve. It does not run the tests.

use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::model::{Corpus, Doc, Res};
use crate::repo::{Fatal, Repo};
use std::collections::HashMap;

/// The text that starts a tag in a test file.
pub const TAG: &str = "covers:";

/// One `covers:` tag: the test file, the line and the requirement id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    pub path: String,
    pub line: u32,
    pub target: String,
}

/// Returns the requirement ids of the `covers:` tags of one line. A tag
/// starts at `covers:` after a character that is not part of a word. The ids
/// follow it, separated by spaces or commas, each as `<doc-id>#<key>`. The
/// first word that is not such an id ends the tag, for example `*/` or
/// `-->`.
pub fn tags(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(pos) = rest.find(TAG) {
        let before = rest[..pos].chars().next_back();
        let after = &rest[pos + TAG.len()..];
        rest = after;
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        for word in after.split(|c: char| c.is_whitespace() || c == ',') {
            if word.is_empty() {
                continue;
            }
            let id = word.trim_matches(|c: char| {
                matches!(c, '`' | '"' | '\'' | '[' | ']' | '(' | ')' | ';' | '.')
            });
            match id.split_once('#') {
                Some((d, k)) if !d.is_empty() && !k.is_empty() && !k.contains('#') => {
                    out.push(id.to_string())
                }
                _ => break,
            }
        }
    }
    out
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
        .grep_lines(TAG, rev, untracked)
        .map_err(Fatal::Io)?;
    let mut out = Vec::new();
    for (path, line, text) in lines {
        if !config.is_test_file(&path) {
            continue;
        }
        for target in tags(&text) {
            out.push(Cover {
                path: path.clone(),
                line,
                target,
            });
        }
    }
    Ok(out)
}

/// The evidence of the requirements of a tree.
#[derive(Default)]
pub struct Evidence {
    /// For each (document, index into `parsed.reqs`), the `covers:` tags as
    /// `<path>:<line>`.
    pub covers: HashMap<(u32, usize), Vec<String>>,
}

impl Evidence {
    /// Resolves the `covers:` tags of a tree. Returns the evidence and a
    /// diagnostic (SD105) for each tag that names no requirement.
    pub fn build(c: &Corpus) -> (Evidence, Vec<Diag>) {
        let mut ev = Evidence::default();
        let mut diags = Vec::new();
        for cv in &c.covers {
            let problem = match c.resolve(&cv.target) {
                Res::Found(d, Some(key), true) => match c.docs[d as usize].req(&key) {
                    Some(r) => {
                        ev.covers
                            .entry((d, r))
                            .or_default()
                            .push(format!("{}:{}", cv.path, cv.line));
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

    /// Returns the evidence of one requirement (an index into `parsed.reqs`
    /// of document `di`): the paths of `verified-by` (front matter and the
    /// inline marker), and the `covers:` tags as `<path>:<line>`.
    pub fn of(&self, di: u32, d: &Doc, r: usize) -> Vec<String> {
        let mut out: Vec<String> = d.verified_by.iter().map(|v| v.0.clone()).collect();
        out.extend(d.parsed.reqs[r].verified_by.iter().cloned());
        if let Some(v) = self.covers.get(&(di, r)) {
            out.extend(v.iter().cloned());
        }
        out
    }
}

/// Returns true if a requirement needs evidence: it is a requirement of an
/// active document.
pub fn needs_evidence(d: &Doc) -> bool {
    d.ty.has_requirements() && d.status_str() == "active"
}
