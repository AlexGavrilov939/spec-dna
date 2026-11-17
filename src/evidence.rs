//! Evidence of requirements (docs/config.md, "Evidence"). A requirement has
//! evidence if one of these links it to a test or a verification record:
//!
//! - `verified-by` in the front matter of its document (every requirement of
//!   the document)
//! - an inline `<!-- verified-by: <path> -->` marker in the requirement
//! - a `covers: <doc-id>#<key>` tag in a test file. The tag is plain text, so
//!   it works in a comment of any language.
//!
//! spec-dna checks that the links resolve. It does not run the tests.

use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::model::{Corpus, Doc, Res, canonical};
use crate::repo::{Fatal, Repo};
use std::collections::{HashMap, HashSet};

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
    /// matter and the inline marker), and the `covers:` tags as
    /// `<path>:<line>`.
    pub fn of(&self, di: u32, d: &Doc, r: usize) -> Vec<String> {
        let mut out: Vec<String> = d.verified_by.iter().map(|v| v.0.clone()).collect();
        out.extend(d.parsed.reqs[r].verified_by.iter().cloned());
        if let Some(v) = self.covers.get(&(di, r)) {
            out.extend(v.iter().cloned());
        }
        out
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
