//! Drift between documents and code (ADR 0001, amendment "code citations").
//! `check` compares the base tree with the head tree. If the change removes or
//! renames a path or an identifier that a document of the head tree still
//! cites, `check` gives a warning for that document: SD606 for a path, SD607
//! for an identifier. Both are heuristic, so they are warnings.
//!
//! The frozen text of an accepted decision cannot change (SD405). Thus an
//! amendment section of the decision that names the new code clears the drift
//! of that decision (see `Amends`).
//!
//! The work starts from the change, so it costs little when the change is
//! small. First, the snapshots give the removed paths and the changed files.
//! Then only the spans that can name a removed path or a removed identifier
//! get a full resolution.

use crate::cite::{
    MAX_SOURCE, Target, Tree, idents, read_source, source_text, strip_location, tokens,
};
use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::model::{Corpus, Doc, DocType, is_doc_path};
use crate::repo::{Fatal, Repo, Snapshot};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Returns true if drift in this document is a problem. A superseded, retired
/// or rejected document, a completed change, a note, an incident and a
/// reference describe the past or give information only. Thus they can cite
/// code that does not exist.
fn current(d: &Doc) -> bool {
    !d.is_inactive()
        && !matches!(d.ty, DocType::Note | DocType::Incident | DocType::Reference)
        && !(d.ty == DocType::Change && d.status_str() == "done")
}

/// The last path segment of a cited path, without a location suffix.
fn last_segment(raw: &str) -> &str {
    let s = strip_location(raw).trim_end_matches('/');
    s.rsplit('/').next().unwrap_or(s)
}

/// Finds the drift of the head tree against the base tree. `untracked` is
/// true when the head tree includes untracked files (`check` without
/// `--base`). The profile and the rules of the root of each document set the
/// severity of its diagnostics, or remove them.
pub fn check(
    repo: &Repo,
    config: &Config,
    base: (&Corpus, &Snapshot),
    head: (&Corpus, &Snapshot),
    untracked: bool,
) -> Result<Vec<Diag>, Fatal> {
    if config.is_off(Code::CitedPathRemoved) && config.is_off(Code::CitedSymbolRemoved) {
        return Ok(Vec::new());
    }
    Ok(crate::check::apply_severity(
        config,
        find(repo, base, head, untracked)?,
    ))
}

fn find(
    repo: &Repo,
    base: (&Corpus, &Snapshot),
    head: (&Corpus, &Snapshot),
    untracked: bool,
) -> Result<Vec<Diag>, Fatal> {
    let (base, base_snap) = base;
    let (head, head_snap) = head;

    // The head snapshot has the changes of the working tree only under the
    // document roots. Git lists the changed and deleted files of the whole
    // tree.
    let (modified, deleted) = if head_snap.worktree {
        repo.git.diff_files().map_err(Fatal::Io)?
    } else {
        Default::default()
    };
    let without_deleted: Option<HashSet<String>> = (!deleted.is_empty()).then(|| {
        head.files
            .iter()
            .filter(|f| !deleted.contains(*f))
            .cloned()
            .collect()
    });
    let head_files = without_deleted.as_ref().unwrap_or(&head.files);

    // Removed paths, and a rename target for a file with the same content.
    let gone_files: HashSet<&str> = base_snap
        .files
        .iter()
        .map(|e| e.path.as_str())
        .filter(|p| !head_files.contains(*p))
        .collect();
    let gone_dirs: HashSet<&str> = base
        .dirs
        .iter()
        .map(String::as_str)
        .filter(|d| !head.dirs.contains(*d))
        .collect();
    let mut renamed: HashMap<&str, &str> = HashMap::new();
    if !gone_files.is_empty() {
        let mut by_blob: HashMap<&str, &str> = HashMap::new();
        for e in &head_snap.files {
            if let Some(b) = &e.blob
                && !base.files.contains(&e.path)
                && head_files.contains(&e.path)
            {
                by_blob.insert(b.as_str(), e.path.as_str());
            }
        }
        for e in &base_snap.files {
            if gone_files.contains(e.path.as_str())
                && let Some(to) = e.blob.as_deref().and_then(|b| by_blob.get(b))
            {
                renamed.insert(e.path.as_str(), to);
            }
        }
    }
    let gone_names: HashSet<&str> = gone_files
        .iter()
        .chain(&gone_dirs)
        .map(|p| p.rsplit('/').next().unwrap_or(p))
        .collect();

    // Removed identifiers: they occur in the base text of a changed file and
    // not in its head text. Documents are not code.
    let changed: Vec<(&str, &str)> = base_snap
        .files
        .iter()
        .filter(|e| !is_doc_path(&e.path))
        .filter_map(|e| {
            let base_blob = e.blob.as_deref()?;
            match head_snap.blob_of(&e.path) {
                Some(h)
                    if h.blob.as_deref() == Some(base_blob)
                        && !modified.contains(&e.path)
                        && !deleted.contains(&e.path) =>
                {
                    None
                }
                _ => Some((e.path.as_str(), base_blob)),
            }
        })
        .collect();
    let blob_ids: Vec<String> = changed.iter().map(|c| c.1.to_string()).collect();
    let blobs = repo
        .git
        .read_small_blobs(&blob_ids, MAX_SOURCE)
        .map_err(Fatal::Io)?;
    // For `check --staged`, the head text of a file is its blob in the index.
    let head_blob_of = |path: &str| -> Option<String> {
        if head_snap.worktree {
            return None;
        }
        head_snap.blob_of(path).and_then(|e| e.blob.clone())
    };
    let head_ids: Vec<String> = changed
        .iter()
        .filter_map(|(path, _)| {
            let now = renamed.get(path).copied().unwrap_or(path);
            head_files
                .contains(now)
                .then(|| head_blob_of(now))
                .flatten()
        })
        .collect();
    let head_blobs = repo
        .git
        .read_small_blobs(&head_ids, MAX_SOURCE)
        .map_err(Fatal::Io)?;
    // For each removed identifier: the file, and the only new distinctive
    // identifier of that file, if there is exactly one (a rename candidate).
    let mut removed: HashMap<String, (String, Option<String>)> = HashMap::new();
    for (path, blob) in &changed {
        let Some(old) = blobs.get(*blob).and_then(|b| source_text(b)) else {
            continue;
        };
        let now = renamed.get(path).copied().unwrap_or(path);
        let new = if head_files.contains(now) {
            if head_snap.worktree {
                read_source(&repo.root, now)
            } else {
                head_blob_of(now).and_then(|b| head_blobs.get(&b).and_then(|t| source_text(t)))
            }
            .unwrap_or_default()
        } else {
            String::new()
        };
        let (old_t, new_t) = (tokens(&old), tokens(&new));
        let gone: Vec<&str> = old_t.difference(&new_t).copied().collect();
        if gone.is_empty() {
            continue;
        }
        let distinct = |a: &HashSet<&str>, b: &HashSet<&str>| -> Vec<String> {
            a.difference(b)
                .filter(|t| !idents(t).is_empty())
                .map(|t| t.to_string())
                .collect()
        };
        let added = distinct(&new_t, &old_t);
        let hint =
            (added.len() == 1 && distinct(&old_t, &new_t).len() == 1).then(|| added[0].clone());
        for g in gone {
            removed
                .entry(g.to_string())
                .or_insert_with(|| (now.to_string(), hint.clone()));
        }
    }
    if gone_names.is_empty() && removed.is_empty() {
        return Ok(Vec::new());
    }

    // The citations of current documents that can name a removed path or a
    // removed identifier. A key is (document, target). The value is the first
    // line.
    let base_tree = Tree::new(&base.files, &base.dirs);
    let head_tree = Tree::new(head_files, &head.dirs);
    let mut gone_paths: BTreeMap<(u32, String), u32> = BTreeMap::new();
    let mut cited_idents: BTreeMap<(u32, String), u32> = BTreeMap::new();
    for (i, d) in head.docs.iter().enumerate() {
        if !current(d) {
            continue;
        }
        let i = i as u32;
        let spans = d.parsed.code_spans.iter().map(|t| (t, true));
        let words = d.parsed.path_words.iter().map(|t| (t, false));
        for (t, is_span) in spans.chain(words) {
            if gone_names.contains(last_segment(&t.text)) {
                // A citation that still resolves in the head tree, for example
                // a file name after a move, is not drift.
                let gone = match base_tree.resolve(&d.path, &t.text) {
                    Some(Target::File(p)) if gone_files.contains(p.as_str()) => {
                        (!base.by_path.contains_key(&p)).then_some(p)
                    }
                    Some(Target::Dir(p)) if gone_dirs.contains(p.as_str()) => Some(p),
                    _ => None,
                };
                if let Some(p) = gone {
                    if head_tree.resolve(&d.path, &t.text).is_none() {
                        gone_paths.entry((i, p)).or_insert(t.line);
                    }
                    continue;
                }
            }
            if !is_span || removed.is_empty() {
                continue;
            }
            let ids: Vec<String> = idents(&t.text)
                .into_iter()
                .filter(|id| removed.contains_key(id))
                .collect();
            if !ids.is_empty()
                && base_tree.resolve(&d.path, &t.text).is_none()
                && head_tree.resolve(&d.path, &t.text).is_none()
            {
                for id in ids {
                    cited_idents.entry((i, id)).or_insert(t.line);
                }
            }
        }
    }

    let mut out = Vec::new();
    let amends = Amends::new(base, head, &head_tree);
    for ((i, p), line) in gone_paths {
        if amends.clears_path(i, line, renamed.get(p.as_str()).copied()) {
            continue;
        }
        let to = renamed
            .get(p.as_str())
            .map(|n| format!(" (renamed to `{n}`)"))
            .unwrap_or_default();
        out.push(diag(
            &head.docs[i as usize],
            line,
            Code::CitedPathRemoved,
            &p,
            format!("`{p}` was removed in this change{to}, but this document still cites it"),
        ));
    }
    if cited_idents.is_empty() {
        return Ok(out);
    }
    // An identifier that still occurs in another file is not removed.
    let mut names: Vec<String> = cited_idents.keys().map(|k| k.1.clone()).collect();
    names.sort();
    names.dedup();
    let mut words = names.clone();
    words.extend(amends.idents());
    let still = repo.git.grep_words(&words, untracked).map_err(Fatal::Io)?;
    for ((i, id), line) in cited_idents {
        if still.contains(&id) || amends.clears_ident(i, line, &still) {
            continue;
        }
        let (file, hint) = &removed[&id];
        let to = hint
            .as_ref()
            .map(|h| format!(" (renamed to `{h}`?)"))
            .unwrap_or_default();
        out.push(diag(
            &head.docs[i as usize],
            line,
            Code::CitedSymbolRemoved,
            &id,
            format!(
                "`{id}` was removed from `{file}` in this change{to}, but this document still cites it"
            ),
        ));
    }
    Ok(out)
}

/// The new code that the amendment sections of accepted decisions name. A
/// citation is new if the base text of the decision does not have it.
struct Amends<'a> {
    head: &'a Corpus,
    /// For each document of the head tree: the new paths that resolve in the
    /// head tree, and the new identifiers.
    docs: HashMap<u32, (HashSet<String>, HashSet<String>)>,
}

impl<'a> Amends<'a> {
    fn new(base: &Corpus, head: &'a Corpus, tree: &Tree) -> Amends<'a> {
        let mut docs = HashMap::new();
        for (i, d) in head.docs.iter().enumerate() {
            if d.ty != DocType::Decision || d.status_str() != "accepted" || d.amendments.is_empty()
            {
                continue;
            }
            let old: HashSet<&str> = base
                .by_path
                .get(&d.path)
                .map(|&b| {
                    let p = &base.docs[b as usize].parsed;
                    p.code_spans
                        .iter()
                        .chain(&p.path_words)
                        .map(|t| t.text.as_str())
                        .collect()
                })
                .unwrap_or_default();
            let (mut paths, mut idents_new) = (HashSet::new(), HashSet::new());
            let spans = d.parsed.code_spans.iter().map(|t| (t, true));
            let words = d.parsed.path_words.iter().map(|t| (t, false));
            for (t, is_span) in spans.chain(words) {
                if old.contains(t.text.as_str()) || !in_amendment(d, t.line) {
                    continue;
                }
                match tree.resolve(&d.path, &t.text) {
                    Some(Target::File(p) | Target::Dir(p)) => {
                        paths.insert(p);
                    }
                    None if is_span => idents_new.extend(idents(&t.text)),
                    None => {}
                }
            }
            if !paths.is_empty() || !idents_new.is_empty() {
                docs.insert(i as u32, (paths, idents_new));
            }
        }
        Amends { head, docs }
    }

    /// The new identifiers of all amendments, to search for in the code.
    fn idents(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .docs
            .values()
            .flat_map(|d| d.1.iter().cloned())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// True if the citation at `line` is in the frozen text of the decision.
    fn frozen(&self, doc: u32, line: u32) -> bool {
        !in_amendment(&self.head.docs[doc as usize], line)
    }

    /// An amendment clears a removed path if it names a new path (the rename
    /// target, if git found one).
    fn clears_path(&self, doc: u32, line: u32, renamed: Option<&str>) -> bool {
        let Some((paths, _)) = self.docs.get(&doc) else {
            return false;
        };
        self.frozen(doc, line)
            && match renamed {
                Some(to) => paths.contains(to),
                None => !paths.is_empty(),
            }
    }

    /// An amendment clears a removed identifier if it names a new identifier
    /// that the code has.
    fn clears_ident(&self, doc: u32, line: u32, in_code: &HashSet<String>) -> bool {
        let Some((_, ids)) = self.docs.get(&doc) else {
            return false;
        };
        self.frozen(doc, line) && ids.iter().any(|i| in_code.contains(i))
    }
}

/// True if `line` is in an amendment section of the document.
fn in_amendment(d: &Doc, line: u32) -> bool {
    let chunks = &d.parsed.chunks;
    let at = chunks.iter().rposition(|c| c.line <= line);
    at.is_some_and(|i| d.amendments.contains(&i))
}

fn diag(d: &Doc, line: u32, code: Code, target: &str, message: String) -> Diag {
    Diag {
        path: d.path.clone(),
        line,
        code,
        doc: d.id.clone(),
        target: target.to_string(),
        message,
        severity: code.severity(),
    }
}
