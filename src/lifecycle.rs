//! The base of the change checks (design 0.6.0, "Gates"), the diff against it,
//! and the requirements that the diff changes. Step 3 builds this module for
//! the lifecycle checks of the next slices.
//!
//! The lifecycle base is the merge base of `--base` (or of `origin/HEAD`) with
//! `HEAD`. It is never the tip of `--base`: a branch that was cut from an old
//! commit is not behind the default branch because of that.

use crate::config::{ChangePolicy, Config};
use crate::git::{Git, PathChange};
use crate::hash::Hash;
use crate::model::{Corpus, Doc, Res};
use std::collections::{BTreeMap, HashSet};

/// The note that a command prints when the lifecycle base is unknown.
pub const NO_BASE_NOTE: &str =
    "no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>";

/// Returns the lifecycle base: the merge base of `--base` (or, without it, of
/// `refs/remotes/origin/HEAD`) with `HEAD`. A `Some` value is always a real
/// merge base. It is never a tip that a command used because no merge base
/// exists. Thus a check that needs the age of a commit can trust it.
///
/// It returns `None` if the policy is `none` (the function then runs no git
/// command), if there is no base revision, if `HEAD` has no commit, or if the
/// two commits have no merge base (an unrelated history, or a shallow clone
/// that does not reach it). It returns an error if `--base` is not a commit.
pub fn change_base(
    config: &Config,
    git: &Git,
    base: Option<&str>,
) -> Result<Option<String>, String> {
    if config.change_policy == ChangePolicy::None {
        return Ok(None);
    }
    let tip = match base {
        Some(rev) => Some(git.commit(rev).ok_or_else(|| {
            format!(
                "base `{rev}` is not a commit here; fetch it first (a depth-1 fetch of the base commit is enough)"
            )
        })?),
        None => git.remote_head(),
    };
    let (Some(tip), Some(head)) = (tip, git.commit("HEAD")) else {
        return Ok(None);
    };
    Ok(git.merge_base(&tip, &head))
}

/// The files that a change touches: the diff of the head against the base.
#[allow(dead_code)] // reader: S4
#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub files: Vec<(String, PathChange)>,
}

#[allow(dead_code)] // reader: S4
impl Diff {
    /// Reads the diff of the working tree (or of the git index, for a
    /// `staged` `Git`) against the base. With `untracked`, a new file that git
    /// does not ignore counts as added.
    pub fn read(git: &Git, base: &str, untracked: bool) -> Result<Diff, String> {
        Ok(Diff {
            files: git.diff_paths(base, untracked)?,
        })
    }

    /// The changed paths under a folder (a repository path with no final `/`).
    pub fn under<'a>(&'a self, folder: &'a str) -> impl Iterator<Item = &'a str> {
        let prefix = format!("{}/", folder.trim_matches('/'));
        self.files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(move |p| p.starts_with(&prefix))
    }

    /// The changed paths that a glob of the repository root matches.
    pub fn matching<'a>(&'a self, glob: &'a str) -> impl Iterator<Item = &'a str> {
        self.files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(move |p| crate::glob::matches(glob, p))
    }

    /// The kind of change of a path, or `None` if the diff does not have it.
    pub fn kind(&self, path: &str) -> Option<PathChange> {
        self.files.iter().find(|(p, _)| p == path).map(|(_, k)| *k)
    }
}

/// How a requirement differs between the base and the head.
#[allow(dead_code)] // reader: S4
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReqChangeKind {
    Changed,
    Added,
    Removed,
}

/// A requirement that the diff changed, added or removed.
#[allow(dead_code)] // reader: S4
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReqChange {
    /// The id of the document in the head (the id of the base for a document
    /// that the head does not have).
    pub doc: String,
    /// The path of that document.
    pub path: String,
    /// The line of the requirement.
    pub line: u32,
    pub key: String,
    pub kind: ReqChangeKind,
    /// `Req.hash` in the base and in the head: the hash that `show` prints and
    /// that a delta line copies.
    pub base_hash: Option<Hash>,
    pub head_hash: Option<Hash>,
    /// `Req.frozen` in the base and in the head: the hash without link
    /// destinations. The comparison of the kinds uses it.
    pub base_frozen: Option<Hash>,
    pub head_frozen: Option<Hash>,
}

/// Returns the requirements that differ between the base and the head, for the
/// documents that `wanted` accepts. A requirement of the base is matched with
/// the head by its key through the aliases (`Corpus::resolve`), so a move that
/// keeps the text gives nothing. It is `Changed` if the `frozen` hash differs,
/// `Removed` if the head has no match, and `Added` if a head requirement has no
/// match in the base. The order is the order of the base documents, then of the
/// head documents. This is the comparison of SD921 (design 0.6.0, D19).
#[allow(dead_code)] // reader: S4
pub fn requirement_changes(
    base: &Corpus,
    head: &Corpus,
    wanted: impl Fn(&Doc) -> bool,
) -> Vec<ReqChange> {
    let mut out = Vec::new();
    let mut matched: HashSet<(u32, usize)> = HashSet::new();
    for b in base.docs.iter().filter(|d| wanted(d)) {
        for &ri in &b.reqs {
            let r = &b.parsed.reqs[ri];
            let target = format!("{}#{}", b.id, r.key);
            let hit = match head.resolve(&target) {
                Res::Found(hd, Some(key), true) => {
                    head.docs[hd as usize].req(&key).map(|hri| (hd, hri))
                }
                // Two documents have the name, so the result is unknown.
                Res::Ambiguous(_) => continue,
                _ => None,
            };
            match hit {
                Some((hd, hri)) => {
                    matched.insert((hd, hri));
                    let h = &head.docs[hd as usize];
                    let hr = &h.parsed.reqs[hri];
                    if hr.frozen != r.frozen {
                        out.push(ReqChange {
                            doc: h.id.clone(),
                            path: h.path.clone(),
                            line: hr.line,
                            key: hr.key.clone(),
                            kind: ReqChangeKind::Changed,
                            base_hash: Some(r.hash),
                            head_hash: Some(hr.hash),
                            base_frozen: Some(r.frozen),
                            head_frozen: Some(hr.frozen),
                        });
                    }
                }
                None => {
                    // The document can be in the head, without the key.
                    let owner = match head.resolve(&b.id) {
                        Res::Found(hd, None, _) => &head.docs[hd as usize],
                        _ => b,
                    };
                    out.push(ReqChange {
                        doc: owner.id.clone(),
                        path: owner.path.clone(),
                        line: if std::ptr::eq(owner, b) { r.line } else { 0 },
                        key: r.key.clone(),
                        kind: ReqChangeKind::Removed,
                        base_hash: Some(r.hash),
                        head_hash: None,
                        base_frozen: Some(r.frozen),
                        head_frozen: None,
                    });
                }
            }
        }
    }
    for (hi, h) in head.docs.iter().enumerate().filter(|(_, d)| wanted(d)) {
        for &ri in &h.reqs {
            if matched.contains(&(hi as u32, ri)) {
                continue;
            }
            let r = &h.parsed.reqs[ri];
            // The key can exist in the base in another kind of document, or
            // under an old name of the document or of the key.
            let names = std::iter::once(&h.id).chain(&h.aliases);
            let old = names
                .map(|n| format!("{n}#{}", r.key))
                .chain(r.aliases.iter().cloned())
                .any(|n| matches!(base.resolve(&n), Res::Found(_, _, true) | Res::Ambiguous(_)));
            if !old {
                out.push(ReqChange {
                    doc: h.id.clone(),
                    path: h.path.clone(),
                    line: r.line,
                    key: r.key.clone(),
                    kind: ReqChangeKind::Added,
                    base_hash: None,
                    head_hash: Some(r.hash),
                    base_frozen: None,
                    head_frozen: Some(r.frozen),
                });
            }
        }
    }
    out
}

/// Groups changed requirements by document id, in the order of the ids.
#[allow(dead_code)] // reader: S4
pub fn by_document(changes: &[ReqChange]) -> BTreeMap<&str, Vec<&ReqChange>> {
    let mut m: BTreeMap<&str, Vec<&ReqChange>> = BTreeMap::new();
    for c in changes {
        m.entry(c.doc.as_str()).or_default().push(c);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use std::sync::Arc;

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        Corpus::build(
            &cfg,
            files.iter().map(|(p, _)| p.to_string()).collect(),
            files
                .iter()
                .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
                .collect(),
        )
    }

    fn doc(reqs: &[(&str, &str)]) -> String {
        let mut s = "---\nstatus: active\ntype: requirements\n---\n# Valve\n\n## Requirements\n\n"
            .to_string();
        for (k, t) in reqs {
            s.push_str(&format!("- **{k}**: {t}\n"));
        }
        s
    }

    fn all(_: &Doc) -> bool {
        true
    }

    #[test]
    fn a_changed_added_and_removed_requirement_is_found() {
        let base = corpus(&[(
            "docs/valve.md",
            &doc(&[
                ("close", "The valve SHALL close in 5 seconds."),
                ("open", "The valve SHALL open."),
                ("flush", "The valve SHALL flush."),
            ]),
        )]);
        let head = corpus(&[(
            "docs/valve.md",
            &doc(&[
                ("close", "The valve SHALL close in 4 seconds."),
                ("open", "The valve SHALL open."),
                ("drain", "The valve SHALL drain."),
            ]),
        )]);
        let ch = requirement_changes(&base, &head, all);
        let got: Vec<(&str, ReqChangeKind)> = ch.iter().map(|c| (c.key.as_str(), c.kind)).collect();
        assert_eq!(
            got,
            vec![
                ("close", ReqChangeKind::Changed),
                ("flush", ReqChangeKind::Removed),
                ("drain", ReqChangeKind::Added),
            ]
        );
        let close = &ch[0];
        assert!(close.base_hash.is_some() && close.head_hash.is_some());
        assert_ne!(close.base_hash, close.head_hash);
        assert!(ch[1].head_hash.is_none() && ch[2].base_hash.is_none());
        assert_eq!(
            by_document(&ch).keys().copied().collect::<Vec<_>>(),
            ["valve"]
        );
    }

    #[test]
    fn the_same_text_and_a_filter_give_nothing() {
        let text = doc(&[("close", "The valve SHALL close.")]);
        let base = corpus(&[("docs/valve.md", &text)]);
        let head = corpus(&[("docs/valve.md", &text)]);
        assert!(requirement_changes(&base, &head, all).is_empty());
        let head2 = corpus(&[("docs/valve.md", &doc(&[("close", "The valve SHALL stay.")]))]);
        assert_eq!(requirement_changes(&base, &head2, all).len(), 1);
        assert!(requirement_changes(&base, &head2, |_| false).is_empty());
    }

    #[test]
    fn an_alias_keeps_a_moved_requirement() {
        let base = corpus(&[(
            "docs/valve.md",
            &doc(&[("close", "The valve SHALL close in 5 seconds.")]),
        )]);
        // The key moved to another document, and the alias names the old one.
        let head = corpus(&[
            ("docs/valve.md", &doc(&[])),
            (
                "docs/pump.md",
                "---\nstatus: active\ntype: requirements\n---\n# Pump\n\n## Requirements\n\n- **close**: The valve SHALL close in 5 seconds. <!-- aliases: valve#close -->\n",
            ),
        ]);
        assert!(requirement_changes(&base, &head, all).is_empty());
    }

    #[test]
    fn the_policy_none_reads_no_git() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        // The path is no repository: any git command would fail.
        let git = Git::new(std::path::Path::new("/nonexistent-spec-dna-path"));
        assert_eq!(change_base(&cfg, &git, Some("main")), Ok(None));
        assert_eq!(change_base(&cfg, &git, None), Ok(None));
    }
}
