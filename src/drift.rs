//! Drift between documents and code (ADR 0001, amendment "code citations").
//! `check` compares the base tree with the head tree. If the change removes or
//! renames a path or an identifier that a document of the head tree still
//! cites, `check` gives a warning for that document: SD606 for a path, SD607
//! for an identifier. Both are heuristic, so they are warnings.
//!
//! The work starts from the change, so it costs little when the change is
//! small. First, the snapshots give the removed paths and the changed files.
//! Then only the spans that can name a removed path or a removed identifier
//! get a full resolution.

use crate::cite::{Target, Tree, idents, source_text, strip_location, tokens};
use crate::diag::{Code, Diag};
use crate::model::{Corpus, Doc, DocType, is_doc_path};
use crate::repo::{Fatal, Repo, Snapshot};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Returns true if drift in this document is a problem. A superseded,
/// retired or rejected document, a completed change and a note describe the
/// past or give information only, so they can cite code that is gone.
fn current(d: &Doc) -> bool {
    !d.is_inactive()
        && d.ty != DocType::Note
        && !(d.ty == DocType::Change && d.status_str() == "done")
}

/// The last path segment of a cited path, without a location suffix.
fn last_segment(raw: &str) -> &str {
    let s = strip_location(raw).trim_end_matches('/');
    s.rsplit('/').next().unwrap_or(s)
}

/// Finds the drift of the head tree against the base tree. `untracked` is
/// true when the head tree includes untracked files (`check` without
/// `--base`).
pub fn check(
    repo: &Repo,
    base: (&Corpus, &Snapshot),
    head: (&Corpus, &Snapshot),
    untracked: bool,
) -> Result<Vec<Diag>, Fatal> {
    let (base, base_snap) = base;
    let (head, head_snap) = head;

    // Removed paths, and a rename target for a file with the same content.
    let gone_files: HashSet<&str> = base_snap
        .files
        .iter()
        .map(|e| e.path.as_str())
        .filter(|p| !head.files.contains(*p))
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
    // not in its head text. Documents are not code. The head snapshot knows
    // the changes of the working tree only under the document roots, so git
    // lists the other changed files.
    let unstaged = if head_snap.worktree {
        repo.git.unstaged().map_err(Fatal::Io)?
    } else {
        HashSet::new()
    };
    let changed: Vec<(&str, &str)> = base_snap
        .files
        .iter()
        .filter(|e| !is_doc_path(&e.path))
        .filter_map(|e| {
            let base_blob = e.blob.as_deref()?;
            match head_snap.blob_of(&e.path) {
                Some(h) if h.blob.as_deref() == Some(base_blob) && !unstaged.contains(&e.path) => {
                    None
                }
                _ => Some((e.path.as_str(), base_blob)),
            }
        })
        .collect();
    let blob_ids: Vec<String> = changed.iter().map(|c| c.1.to_string()).collect();
    let blobs = repo.git.read_blobs(&blob_ids).map_err(Fatal::Io)?;
    // For each removed identifier: the file, and the only new distinctive
    // identifier of that file, if there is exactly one (a rename candidate).
    let mut removed: HashMap<String, (String, Option<String>)> = HashMap::new();
    for (path, blob) in &changed {
        let Some(old) = blobs.get(*blob).and_then(|b| source_text(b)) else {
            continue;
        };
        let now = renamed.get(path).copied().unwrap_or(path);
        let new = if head.files.contains(now) {
            std::fs::read(repo.root.join(now))
                .ok()
                .and_then(|b| source_text(&b))
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
    // removed identifier. A key is (document, target); the value is the
    // first line.
    let base_tree = Tree::new(&base.files, &base.dirs);
    let head_tree = Tree::new(&head.files, &head.dirs);
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
                match base_tree.resolve(&d.path, &t.text) {
                    Some(Target::File(p)) if gone_files.contains(p.as_str()) => {
                        if !base.by_path.contains_key(&p) {
                            gone_paths.entry((i, p)).or_insert(t.line);
                        }
                        continue;
                    }
                    Some(Target::Dir(p)) if gone_dirs.contains(p.as_str()) => {
                        gone_paths.entry((i, p)).or_insert(t.line);
                        continue;
                    }
                    _ => {}
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
    for ((i, p), line) in gone_paths {
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
    let still = repo.git.grep_words(&names, untracked).map_err(Fatal::Io)?;
    for ((i, id), line) in cited_idents {
        if still.contains(&id) {
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
