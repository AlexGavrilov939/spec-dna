//! The proof of `upgrade` (design 0.6.0, D149; ADR 0009, "Proof"). It is a
//! pure function over a plan in memory: it reads the tree and writes nothing.
//! It runs on the result of the plan of the configuration (S5), the front
//! matter (S6) and the moves (S7).
//!
//! "Before" is the tree with only the in-place edits of `type` and `status`,
//! read with the final configuration: no move, no alias and no link edit. The
//! 0.5 types with requirements map one to one to `requirements`, `interface`
//! and `rule`, so "before" has the ids and the hashes of 0.5. "After" is built
//! only from what `upgrade` would write: the file list with the moves, and the
//! text of each write over the bytes of the tree. The proof never reads the
//! `edited` text of a document for "after". The 5 items:
//!
//! 1. Each id, alias, legacy id, section id and requirement id of "before"
//!    resolves in "after" to the same object.
//! 2. Each requirement and each section has the same hash (the hash without
//!    link destinations), and the locked text of each decision is the same.
//! 3. Each document has the type that the mapping gives for its 0.5 type. The
//!    0.5 type comes from the `Class` of the document, not from its text
//!    (D211). A document that `--layout` makes a top file has the type of the
//!    top file, `Class.top` (D43).
//! 4. The new configuration parses, and its layer graph has no cycle.
//! 5. No error of the groups `references` and `ids` is new (O5).
//!
//! Each link of "before" must also go to the same file in "after". A failure
//! of the proof stops `upgrade` with exit code 1 before it writes. The notes
//! are for the report: they never fail the proof (D207).

use crate::check;
use crate::config::{Config, GROUPS};
use crate::diag::Code;
use crate::model::{Corpus, Doc, DocType, LinkRes, Res, is_doc_path, join};
use crate::parse::{RefKind, parse};
use crate::repo::Fatal;
use crate::split;
use crate::upgrade_docs::{Class, Classified, DocsPlan, Fate, MoveKind, Tree, link_moves};
use crate::upgrade_old::type_of;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// What the proof reads: the classes and the texts of the documents (S6), the
/// plan of the documents (S7) and the text of the new `spec-dna.toml` (S5).
pub struct Planned<'a> {
    pub classified: &'a Classified,
    pub docs: &'a DocsPlan,
    pub config: &'a str,
}

/// Facts for the report. They never fail the proof.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Notes {
    /// A link of a document to the folder of a change that moves only in part,
    /// or whose folder has another document of the index now. The plan does not
    /// rewrite it. Each entry is the document and the link target.
    pub folder_links: Vec<(String, String)>,
    /// A link of a Markdown file that is no document (it is outside the
    /// roots, or excluded) to a moved file or folder. The plan does not
    /// rewrite it (D207). Each entry is the file and the link target.
    pub outside_links: Vec<(String, String)>,
}

/// The result of the proof. `failures` is empty if the proof passes.
#[derive(Debug, Default)]
pub struct Proof {
    /// The ids that the proof resolved in both trees.
    pub ids: usize,
    /// The requirements whose hash the proof compared.
    pub requirements: usize,
    /// The sections whose hash the proof compared.
    pub sections: usize,
    /// The documents whose type the proof checked.
    pub types: usize,
    pub failures: Vec<String>,
    pub notes: Notes,
}

impl Proof {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// The codes of the groups that item 5 checks (O5).
fn guarded_codes() -> Vec<Code> {
    GROUPS
        .iter()
        .filter(|(name, _)| matches!(*name, "references" | "ids"))
        .flat_map(|(_, codes)| codes.iter().filter_map(|c| Code::from_code(c)))
        .collect()
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn is_document(cfg: &Config, path: &str) -> bool {
    is_doc_path(path) && cfg.root_of(path).is_some_and(|r| !cfg.is_excluded(r, path))
}

fn build(cfg: &Config, files: Vec<String>, texts: &[(String, String)]) -> Corpus {
    Corpus::build(
        cfg,
        files,
        texts
            .iter()
            .map(|(p, t)| (p.clone(), Arc::new(parse(t))))
            .collect(),
    )
}

/// Proves a plan. The result lists each failure. The function reads the tree
/// for the files outside the roots and for a document that the plan did not
/// read, so a read error is an error of the function.
pub fn prove(tree: &Tree, planned: &Planned) -> Result<Proof, Fatal> {
    let mut proof = Proof::default();
    // Item 4: the final configuration. Without it no corpus is possible.
    let fin = match Config::parse(planned.config) {
        Ok(c) => c,
        Err(e) => {
            proof.failures.push(format!(
                "configuration: the new spec-dna.toml does not parse, or its layer graph has a cycle: {e}"
            ));
            return Ok(proof);
        }
    };
    let (classified, plan) = (planned.classified, planned.docs);
    let moved: HashMap<&str, &str> = plan
        .moves
        .iter()
        .map(|m| (m.from.as_str(), m.to.as_str()))
        .collect();
    let rev: HashMap<&str, &str> = plan
        .moves
        .iter()
        .map(|m| (m.to.as_str(), m.from.as_str()))
        .collect();
    let map = |path: &str| -> String { moved.get(path).copied().unwrap_or(path).to_string() };

    // "Before": the old paths, with the edits of the front matter.
    let edited: HashMap<&str, &str> = plan
        .docs
        .iter()
        .map(|d| (d.path.as_str(), d.edited.as_str()))
        .collect();
    let before_texts: Vec<(String, String)> = classified
        .texts
        .iter()
        .filter(|(p, _)| is_document(&fin, p))
        .map(|(p, t)| {
            (
                p.clone(),
                edited.get(p.as_str()).map_or(t.clone(), |e| e.to_string()),
            )
        })
        .collect();
    let before = build(&fin, tree.files.to_vec(), &before_texts);

    // "After": the file list with the moves, and the text of each write.
    let mut files: Vec<String> = tree.files.iter().map(|f| map(f)).collect();
    files.sort();
    for w in files.windows(2) {
        if w[0] == w[1] {
            proof
                .failures
                .push(format!("`{}`: 2 files would be at this path", w[0]));
        }
    }
    files.dedup();
    let after_paths: Vec<String> = files
        .iter()
        .filter(|f| is_document(&fin, f))
        .cloned()
        .collect();
    let unknown: Vec<String> = after_paths
        .iter()
        .filter(|p| !plan.writes.contains_key(*p))
        .filter(|p| {
            !classified
                .texts
                .contains_key(*rev.get(p.as_str()).unwrap_or(&p.as_str()))
        })
        .map(|p| {
            rev.get(p.as_str())
                .copied()
                .unwrap_or(p.as_str())
                .to_string()
        })
        .collect();
    let read = if unknown.is_empty() {
        HashMap::new()
    } else {
        (tree.read)(&unknown)?
    };
    let after_texts: Vec<(String, String)> = after_paths
        .iter()
        .map(|p| {
            let old = rev.get(p.as_str()).copied().unwrap_or(p.as_str());
            let text = match plan.writes.get(p) {
                Some(t) => t.clone(),
                None => classified
                    .texts
                    .get(old)
                    .cloned()
                    .unwrap_or_else(|| read.get(old).map(|b| lossy(b)).unwrap_or_default()),
            };
            (p.clone(), text)
        })
        .collect();
    let after = build(&fin, files, &after_texts);

    let class: HashMap<&str, &Class> = classified
        .docs
        .iter()
        .map(|c| (c.path.as_str(), c))
        .collect();
    // A document of "before" with its partner in "after".
    let mut pairs: Vec<(&Doc, &Doc)> = Vec::new();
    for b in &before.docs {
        match after.by_path.get(&map(&b.path)) {
            Some(&i) => pairs.push((b, &after.docs[i as usize])),
            None => proof.failures.push(format!(
                "`{}`: the document `{}` would no longer be a document (the plan writes it at `{}`)",
                b.path,
                b.id,
                map(&b.path)
            )),
        }
    }
    let partners: HashSet<&str> = pairs.iter().map(|(_, a)| a.path.as_str()).collect();
    for a in &after.docs {
        if !partners.contains(a.path.as_str()) {
            proof
                .failures
                .push(format!("`{}`: a new document would appear", a.path));
        }
    }

    // The requirements of "before": a document that failed its type (a change
    // at its old place) reads the type of its root, and it can read a list
    // item as a requirement. The 0.5 type has no requirement there (D211).
    let reqs_before = |d: &Doc| -> Vec<usize> {
        match class
            .get(d.path.as_str())
            .and_then(|c| c.old_type.as_deref())
        {
            Some(t) if !type_of(t).is_some_and(DocType::has_requirements) => Vec::new(),
            _ => d.reqs.clone(),
        }
    };

    check_types(&mut proof, &pairs, &class);
    check_ids(&mut proof, &before, &after, &pairs, &map, &reqs_before);
    check_hashes(&mut proof, &before, &after, &pairs, &reqs_before);
    let folders = link_moves(tree, &plan.moves);
    check_links(&mut proof, &before, &after, &pairs, &map, plan, &folders);
    check_errors(&mut proof, &fin, &before, &after);
    proof.notes.outside_links = outside_links(tree, classified, &folders, &before)?;
    proof.failures.dedup();
    Ok(proof)
}

/// Item 3: the type of each document in "after" is the mapping of its 0.5 type.
fn check_types(proof: &mut Proof, pairs: &[(&Doc, &Doc)], class: &HashMap<&str, &Class>) {
    for (b, a) in pairs {
        proof.types += 1;
        // Only a note, a reference or a change that moves can become a top
        // file (D41). The 0.5 type is the one of the front matter and the
        // configuration, so this check does not trust the choice of `classify`.
        if let Some(c) = class.get(b.path.as_str())
            && c.top.is_some()
            && !matches!(
                (c.old_type.as_deref(), c.fate),
                (Some("note" | "reference"), _) | (Some("change"), Fate::Move)
            )
        {
            proof.failures.push(format!(
                "`{}`: the document has the 0.5 type `{}`, and only a `note`, a `reference` or a change that is not done can become a top file",
                b.path,
                c.old_type.as_deref().unwrap_or("none")
            ));
            continue;
        }
        let want = match class.get(b.path.as_str()) {
            // A top file has the type of its place (D43).
            Some(c) if c.top.is_some() => c.top,
            Some(c) => match (&c.old_type, c.fate) {
                (Some(_), Fate::Done) => Some(DocType::Research),
                (Some(old), _) => type_of(old),
                (None, _) => None,
            },
            None => None,
        };
        let Some(want) = want else {
            // A stub, a name of 0.6, or a name that no version knows: the plan
            // leaves the document as it is, so the type and the state of its
            // type are the same as before (D196).
            if (a.ty, a.type_failed) != (b.ty, b.type_failed) {
                proof.failures.push(format!(
                    "`{}`: the plan leaves the document as it is, and its type would change from `{}` to `{}`",
                    a.path,
                    b.ty.name(),
                    a.ty.name()
                ));
            }
            continue;
        };
        let old = class
            .get(b.path.as_str())
            .and_then(|c| c.old_type.as_deref())
            .map_or(String::new(), |t| format!(" of the 0.5 type `{t}`"));
        if a.type_failed {
            proof.failures.push(format!(
                "`{}`: the type `{}`{old} is not valid at this place after the plan; the mapping gives `{}`",
                a.path,
                a.ty.name(),
                want.name()
            ));
        } else if a.ty != want {
            proof.failures.push(format!(
                "`{}`: the type would be `{}`, and the mapping{old} gives `{}`",
                a.path,
                a.ty.name(),
                want.name()
            ));
        }
    }
}

/// The ids of a document of "before" that must stay: its id, its aliases, its
/// legacy aliases, the id of each requirement and each section.
fn ids_of(d: &Doc, reqs: &[usize]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if d.moved_to.is_some() {
        // A stub has no name: its moved document keeps the old id.
        return ids;
    }
    ids.insert(d.id.clone());
    ids.extend(d.aliases.iter().cloned());
    ids.extend(d.legacy_aliases.iter().cloned());
    for &r in reqs {
        let req = &d.parsed.reqs[r];
        ids.insert(format!("{}#{}", d.id, req.key));
        ids.extend(req.aliases.iter().cloned());
    }
    for c in &d.parsed.chunks {
        if c.level > 0 && !c.anchor.is_empty() {
            ids.insert(format!("{}#{}", d.id, c.anchor));
        }
    }
    ids
}

/// The paths that a result of `resolve` names, sorted.
fn paths_of(c: &Corpus, res: &Res) -> Vec<String> {
    let mut v: Vec<String> = match res {
        Res::Found(d, ..) => vec![c.docs[*d as usize].path.clone()],
        Res::Ambiguous(ds) => ds
            .iter()
            .map(|&d| c.docs[d as usize].path.clone())
            .collect(),
        _ => Vec::new(),
    };
    v.sort();
    v
}

/// Item 1 (and the hash of the object that an id names): each id of "before"
/// resolves in "after" to the same object.
fn check_ids(
    proof: &mut Proof,
    before: &Corpus,
    after: &Corpus,
    pairs: &[(&Doc, &Doc)],
    map: &dyn Fn(&str) -> String,
    reqs_before: &dyn Fn(&Doc) -> Vec<usize>,
) {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (b, _) in pairs {
        for id in ids_of(b, &reqs_before(b)) {
            if !seen.insert(id.clone()) {
                continue;
            }
            let (rb, ra) = (before.resolve(&id), after.resolve(&id));
            let was = match &rb {
                Res::Found(_, _, true) | Res::Ambiguous(_) => paths_of(before, &rb),
                // An id that named nothing before has nothing to keep.
                _ => continue,
            };
            proof.ids += 1;
            let want: Vec<String> = {
                let mut v: Vec<String> = was.iter().map(|p| map(p)).collect();
                v.sort();
                v
            };
            let now = paths_of(after, &ra);
            let good = matches!(
                (&rb, &ra),
                (Res::Found(_, fb, true), Res::Found(_, fa, true)) if fb == fa
            ) || matches!((&rb, &ra), (Res::Ambiguous(_), Res::Ambiguous(_)));
            if !good || now != want {
                let after_text = match &ra {
                    Res::Missing => "does not resolve".to_string(),
                    Res::Found(_, _, false) => format!(
                        "names a section that `{}` does not have",
                        now.first().map_or("?", String::as_str)
                    ),
                    Res::Ambiguous(_) => format!("names {} documents", now.len()),
                    Res::CrossRepo(_) => "does not resolve".to_string(),
                    Res::Found(..) => format!("resolves to `{}`", now.join("`, `")),
                };
                proof.failures.push(format!(
                    "`{id}` resolves to `{}` before the plan; after the plan it {after_text}, and `{}` is the document that keeps it",
                    was.join("`, `"),
                    want.join("`, `"),
                ));
                continue;
            }
            // The object has the same text: a requirement or a section.
            if let (Res::Found(bi, Some(f), true), Res::Found(ai, ..)) = (&rb, &ra) {
                let (bd, ad) = (&before.docs[*bi as usize], &after.docs[*ai as usize]);
                let same = match (bd.req(f), ad.req(f)) {
                    (Some(x), Some(y)) if reqs_before(bd).contains(&x) => {
                        Some(bd.parsed.reqs[x].frozen == ad.parsed.reqs[y].frozen)
                    }
                    (Some(_), None) => None,
                    _ => {
                        let chunk = |d: &Doc| {
                            d.parsed
                                .chunks
                                .iter()
                                .find(|c| c.anchor == *f)
                                .map(|c| c.frozen)
                        };
                        match (chunk(bd), chunk(ad)) {
                            (Some(x), Some(y)) => Some(x == y),
                            (None, _) => Some(true),
                            _ => None,
                        }
                    }
                };
                if same != Some(true) {
                    proof
                        .failures
                        .push(format!("`{id}`: it would no longer name the same text"));
                }
            }
        }
    }
}

/// Item 2: the hash of each requirement and each section, and the locked text
/// of each decision, are the same.
fn check_hashes(
    proof: &mut Proof,
    before: &Corpus,
    after: &Corpus,
    pairs: &[(&Doc, &Doc)],
    reqs_before: &dyn Fn(&Doc) -> Vec<usize>,
) {
    let hashes = |c: &Corpus, f: &dyn Fn(&Doc) -> Vec<usize>| {
        let mut v: Vec<_> = c
            .docs
            .iter()
            .flat_map(|d| f(d).into_iter().map(|r| d.parsed.reqs[r].frozen))
            .collect();
        v.sort_unstable();
        v
    };
    let (hb, ha) = (
        hashes(before, reqs_before),
        hashes(after, &|d| d.reqs.clone()),
    );
    proof.requirements = hb.len();
    if hb != ha {
        proof
            .failures
            .push(split::hash_changes_of(before, after, reqs_before));
    }
    for (b, a) in pairs {
        let (cb, ca) = (&b.parsed.chunks, &a.parsed.chunks);
        proof.sections += cb.len();
        if cb.len() != ca.len() {
            proof.failures.push(format!(
                "`{}`: the document would have {} sections, and it has {} now",
                b.id,
                ca.len(),
                cb.len()
            ));
        } else {
            for (x, y) in cb.iter().zip(ca) {
                if x.anchor != y.anchor || x.frozen != y.frozen {
                    let name = if x.level == 0 {
                        format!("`{}`: the text before the first heading", b.id)
                    } else {
                        format!("`{}#{}`: the section", b.id, x.anchor)
                    };
                    proof.failures.push(format!("{name} would change"));
                }
            }
        }
        if b.ty == DocType::Decision
            && a.ty == DocType::Decision
            && !b.type_failed
            && !a.type_failed
            && b.frozen_hash != a.frozen_hash
        {
            proof.failures.push(format!(
                "`{}`: the locked text of the decision would change",
                b.id
            ));
        }
    }
}

/// The path that a relative link names, as `rewrite_links` reads it: a file or
/// a folder of the tree. `None` for an external link, an anchor or a path that
/// does not exist.
fn link_path(c: &Corpus, from: &str, target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with(['#', '<'])
        || target.starts_with("//")
        || target.contains("://")
        || target.starts_with("mailto:")
    {
        return None;
    }
    let path = &target[..target.find(['#', '?']).unwrap_or(target.len())];
    let resolved = match path.strip_prefix('/') {
        Some(abs) => join("", abs),
        None => join(from.rsplit_once('/').map_or("", |(d, _)| d), path),
    }?;
    (c.files.contains(&resolved) || c.dirs.contains(&resolved)).then_some(resolved)
}

/// Each link of "before" goes to the same file in "after". A link to a folder
/// whose index document changes is a note, not a failure (D207).
fn check_links(
    proof: &mut Proof,
    before: &Corpus,
    after: &Corpus,
    pairs: &[(&Doc, &Doc)],
    map: &dyn Fn(&str) -> String,
    plan: &DocsPlan,
    folders: &HashMap<String, String>,
) {
    // The folders that hold a change that moves: a link to such a folder keeps
    // naming it, and the plan does not rewrite it.
    let change_dirs: HashSet<&str> = plan
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Change)
        .filter_map(|m| m.from.rsplit_once('/').map(|(d, _)| d))
        .collect();
    for (b, a) in pairs {
        if b.parsed.refs.len() != a.parsed.refs.len() {
            proof.failures.push(format!(
                "`{}`: the plan would change the number of links ({} before, {} after)",
                a.path,
                b.parsed.refs.len(),
                a.parsed.refs.len()
            ));
            continue;
        }
        for (x, y) in b.parsed.refs.iter().zip(&a.parsed.refs) {
            if x.kind == RefKind::Wiki {
                continue;
            }
            let fail = |why: String| {
                format!(
                    "`{}`: the link `{}` {why} (it is `{}` after the plan)",
                    b.path, x.target, y.target
                )
            };
            match (
                before.resolve_link(&b.path, &x.target),
                after.resolve_link(&a.path, &y.target),
            ) {
                (LinkRes::File(f, _, fb), LinkRes::File(g, _, fa)) => {
                    if map(&f) != g || fb != fa {
                        proof
                            .failures
                            .push(fail(format!("goes to `{f}`, and it would go to `{g}`")));
                    }
                }
                (LinkRes::Dir(ib), LinkRes::Dir(ia)) => {
                    let moved_index = ib.map(|i| map(&before.docs[i as usize].path));
                    let now = ia.map(|i| after.docs[i as usize].path.clone());
                    let folder = link_path(before, &b.path, &x.target);
                    // A folder that moves whole has a rewritten link.
                    let partial = folder
                        .as_deref()
                        .is_some_and(|f| change_dirs.contains(f) && !folders.contains_key(f));
                    if moved_index != now || partial {
                        proof
                            .notes
                            .folder_links
                            .push((b.path.clone(), x.target.clone()));
                    }
                }
                (LinkRes::Missing(_), LinkRes::Missing(_))
                | (LinkRes::External, LinkRes::External) => {}
                (LinkRes::SelfAnchor(p), LinkRes::SelfAnchor(q)) if p == q => {}
                (LinkRes::File(f, ..), LinkRes::Missing(_)) => proof.failures.push(fail(format!(
                    "goes to `{f}`, and it would be a broken link"
                ))),
                (LinkRes::Dir(_), LinkRes::Missing(_)) => proof.failures.push(fail(
                    "goes to a folder, and it would be a broken link".to_string(),
                )),
                (x2, y2) => proof.failures.push(fail(format!(
                    "would change what it names ({x2:?} before, {y2:?} after)"
                ))),
            }
        }
    }
}

/// Item 5: no error of the groups `references` and `ids` is new. SD202 needs 2
/// trees, so the proof asks for it as the gate does (`check::transitions`).
fn check_errors(proof: &mut Proof, fin: &Config, before: &Corpus, after: &Corpus) {
    let codes = guarded_codes();
    for d in split::new_error_diags(fin, before, after, Some(&codes)) {
        proof.failures.push(format!("a new error: {}", d.text()));
    }
    for d in check::transitions(fin, before, after) {
        if d.code == Code::IdRemoved && d.severity == crate::diag::Severity::Error {
            proof.failures.push(format!("a new error: {}", d.text()));
        }
    }
}

/// The notes of the report (D207): the links of the Markdown files that are no
/// document to a moved file or folder. The plan does not rewrite them.
fn outside_links(
    tree: &Tree,
    classified: &Classified,
    map: &HashMap<String, String>,
    before: &Corpus,
) -> Result<Vec<(String, String)>, Fatal> {
    let mut found = Vec::new();
    let outside: Vec<String> = tree
        .files
        .iter()
        .filter(|f| is_doc_path(f) && !classified.texts.contains_key(*f))
        .cloned()
        .collect();
    if outside.is_empty() || map.is_empty() {
        return Ok(found);
    }
    let bytes = (tree.read)(&outside)?;
    for path in &outside {
        let Some(blob) = bytes.get(path) else {
            continue;
        };
        for r in &parse(&lossy(blob)).refs {
            if r.kind == RefKind::Wiki {
                continue;
            }
            if link_path(before, path, &r.target).is_some_and(|p| map.contains_key(&p)) {
                found.push((path.clone(), r.target.clone()));
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::{Corpus, DocType};
    use crate::parse::parse;
    use crate::upgrade_docs::TreeData;
    use crate::upgrade_docs::tests::Run;
    use crate::upgrade_docs::tests::{CASES, MIXED, after_tree, fixture_case, go, mem, mv};
    use std::sync::Arc;

    fn prove_of(tree: &TreeData, r: &Run, p: &DocsPlan) -> Proof {
        tree.with(|t| {
            prove(
                t,
                &Planned {
                    classified: &r.classified,
                    docs: p,
                    config: &r.plan.text,
                },
            )
        })
        .unwrap()
    }

    fn failed(pr: &Proof) -> String {
        pr.failures.join("\n")
    }

    /// Changes the final text of a document in the plan, as a broken plan
    /// would write it.
    fn tamper(r: &Run, p: &mut DocsPlan, path: &str, f: impl Fn(&str) -> String) {
        let from = p
            .moves
            .iter()
            .find(|m| m.to == path)
            .map_or(path, |m| m.from.as_str());
        let cur = p
            .writes
            .get(path)
            .cloned()
            .unwrap_or_else(|| r.classified.texts[from].clone());
        let new = f(&cur);
        assert_ne!(new, cur, "the change of the test changes nothing in {path}");
        p.writes.insert(path.to_string(), new);
    }

    fn v05(layout: bool) -> (TreeData, Run, DocsPlan) {
        let (tree, toml) = fixture_case("v05");
        let (r, p) = go(&tree, &toml, layout);
        (tree, r, p)
    }

    fn has(pr: &Proof, parts: &[&str]) {
        let all = failed(pr);
        assert!(
            pr.failures
                .iter()
                .any(|f| parts.iter().all(|p| f.contains(p))),
            "no failure with {parts:?} in:\n{all}"
        );
    }

    #[test]
    fn the_plans_of_the_fixtures_pass_the_proof_and_the_proof_counts_what_it_compares() {
        for case in CASES {
            for layout in [false, true] {
                let (tree, toml) = fixture_case(case);
                let (r, p) = go(&tree, &toml, layout);
                let pr = prove_of(&tree, &r, &p);
                assert!(
                    pr.failures.is_empty(),
                    "{case} layout={layout}:\n{}",
                    failed(&pr)
                );
                let docs = r.classified.docs.len();
                assert!(docs > 0);
                assert_eq!(pr.types, docs, "{case}: one type check for each document");
                assert!(
                    pr.ids >= docs,
                    "{case}: {} ids for {docs} documents",
                    pr.ids
                );
                assert!(pr.sections > 0, "{case}");
                if case == "v05" {
                    assert!(pr.requirements >= 6, "{case}: {}", pr.requirements);
                }
            }
        }
    }

    #[test]
    fn a_plan_that_drops_an_alias_fails_the_proof() {
        let (tree, r, mut p) = v05(false);
        let m = mv(&p, "docs/changes/add-drip-lines.md").clone();
        tamper(&r, &mut p, &m.to, |t| {
            t.replace(&format!("  - {}\n", m.old_id), "")
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &[&format!("`{}`", m.old_id), "does not resolve"]);
        // The gate would find the same defect as SD202.
        has(&pr, &["SD202"]);
    }

    #[test]
    fn a_plan_that_loses_a_write_fails_the_proof() {
        let (tree, r, mut p) = v05(false);
        let m = mv(&p, "docs/changes/rework-pumps.md").clone();
        p.writes.remove(&m.to);
        let pr = prove_of(&tree, &r, &p);
        assert!(!pr.failures.is_empty());
        has(&pr, &[&format!("`{}`", m.old_id)]);
        has(&pr, &[&format!("`{}`", m.to), "type"]);
    }

    #[test]
    fn a_changed_requirement_fails_item_2_and_names_the_requirement() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/irrigation/valves.md", |t| {
            t.replace("SHALL close the valve", "SHALL open the valve")
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["irrigation/valves#shutoff", "would change"]);
    }

    #[test]
    fn a_changed_section_fails_item_2_and_names_the_section() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/irrigation/valves.md", |t| {
            t.replace("floods the bed", "waters the bed")
        });
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &["irrigation/valves#rationale", "section", "would change"],
        );
    }

    #[test]
    fn a_changed_text_of_a_decision_fails_the_locked_hash() {
        let (tree, r, mut p) = v05(false);
        tamper(
            &r,
            &mut p,
            "docs/decisions/ADR-1-valve-controller.md",
            |t| {
                t.replace(
                    "one controller for each row",
                    "two controllers for each row",
                )
            },
        );
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["locked", "ADR-1-valve-controller"]);
    }

    #[test]
    fn a_wrong_type_fails_item_3_and_names_the_document() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/contracts/valve-api.md", |t| {
            t.replace("type: interface", "type: rule")
        });
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &["docs/contracts/valve-api.md", "`rule`", "`interface`"],
        );
    }

    #[test]
    fn a_moved_change_with_the_wrong_type_fails_item_3() {
        let (tree, r, mut p) = v05(false);
        let m = mv(&p, "docs/changes/rework-pumps.md").clone();
        tamper(&r, &mut p, &m.to, |t| {
            t.replace("type: intent", "type: research")
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &[&m.to, "`research`", "`intent`"]);
    }

    #[test]
    fn a_configuration_with_a_cycle_fails_item_4() {
        let (tree, mut r, p) = v05(false);
        r.plan.text.push_str(
            "\n[layers.aa]\ntypes = [\"guide\"]\nmoves-to = [\"bb\"]\n[layers.bb]\ntypes = [\"guide\"]\nmoves-to = [\"aa\"]\n",
        );
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["configuration", "cycle"]);
        assert_eq!(
            pr.ids, 0,
            "the proof stops: no corpus without a configuration"
        );
    }

    #[test]
    fn a_configuration_that_does_not_parse_fails_item_4() {
        let (tree, mut r, p) = v05(false);
        r.plan.text = "spec-dna = [".into();
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["configuration", "does not parse"]);
    }

    #[test]
    fn requirements_that_swap_their_keys_keep_the_multiset_but_fail_the_id() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/irrigation/valves.md", |t| {
            t.replace("**shutoff**", "**tmp**")
                .replace("**flush**", "**shutoff**")
                .replace("**tmp**", "**flush**")
        });
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &[
                "`irrigation/valves#shutoff`",
                "no longer name the same text",
            ],
        );
        // The multiset of the hashes is the same, so item 2 names no requirement.
        assert!(!failed(&pr).contains("the text of"), "{}", failed(&pr));
    }

    #[test]
    fn a_new_duplicate_id_fails_item_5() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/irrigation/valves.md", |t| {
            t.replacen("---\n", "---\naliases: [irrigation/sensors]\n", 1)
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["a new error", "SD201", "irrigation/sensors"]);
    }

    #[test]
    fn a_new_broken_link_fails_item_5_and_the_links() {
        // R6: the batch rewrite does not change a destination in angle brackets.
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            ("docs/a.md", "---\ntype: spec\n---\n# A\n"),
            (
                "docs/c.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n\nSee [the spec](<a.md>).\n",
            ),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["a new error", "SD102"]);
        has(&pr, &["docs/c.md", "link", "a.md"]);
    }

    #[test]
    fn a_link_that_goes_to_another_file_fails_the_links() {
        let (tree, r, mut p) = v05(false);
        tamper(&r, &mut p, "docs/irrigation/valves.md", |t| {
            t.replace("(sensors.md#requirements)", "(pumps.md#requirements)")
        });
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &["docs/irrigation/valves.md", "sensors.md", "pumps.md"],
        );
    }

    #[test]
    fn a_document_that_leaves_the_corpus_fails_the_proof() {
        let (tree, r, mut p) = v05(false);
        // A move into an excluded folder removes the document from the corpus.
        p.moves.push(crate::upgrade_docs::Move {
            from: "docs/irrigation/valves.md".into(),
            to: "docs/vendor/valves.md".into(),
            old_id: String::new(),
            new_id: String::new(),
            aliases: Vec::new(),
            kind: MoveKind::Layout,
        });
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &["docs/irrigation/valves.md", "no longer be a document"],
        );
    }

    #[test]
    fn a_change_with_a_requirement_item_passes_because_the_old_type_has_no_requirement() {
        // In "before", a change at its old place fails its type and reads the
        // type of the root. A list item with a bold key is then a requirement
        // there. A 0.5 change has none, so the proof must not count it.
        let change = "---\ntype: change\nstatus: proposed\n---\n# C\n\n- **r-1**: The pump SHALL stop at once.\n";
        let tree = mem(&[("spec-dna.toml", MIXED), ("docs/c.md", change)]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        assert_eq!(pr.requirements, 0);
    }

    #[test]
    fn a_document_that_was_as_broken_before_passes_item_3() {
        // A type that no version knows has no 0.5 type, so the plan leaves the
        // document as it is (D196). It fails its type before and after.
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            (
                "docs/c.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n",
            ),
            ("docs/b.md", "---\ntype: banana\n---\n# B\n"),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        assert_eq!(pr.types, 2);
    }

    #[test]
    fn a_document_that_the_plan_leaves_as_it_is_must_keep_its_type() {
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            ("docs/b.md", "---\ntype: banana\n---\n# B\n"),
        ]);
        let (r, mut p) = go(&tree, MIXED, false);
        tamper(&r, &mut p, "docs/b.md", |t| t.replace("banana", "research"));
        let pr = prove_of(&tree, &r, &p);
        has(
            &pr,
            &["docs/b.md", "leaves the document as it is", "`research`"],
        );
    }

    #[test]
    fn a_link_to_a_folder_that_a_change_leaves_in_part_is_a_note() {
        // The done change stays in the folder, so the link still names a folder
        // with a document. The moved change is gone from it, and the plan does
        // not rewrite the link.
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            (
                "docs/a.md",
                "---\ntype: spec\n---\n# A\n\nSee [the changes](changes/).\n",
            ),
            (
                "docs/changes/c.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n",
            ),
            (
                "docs/changes/d.md",
                "---\ntype: change\nstatus: done\n---\n# D\n",
            ),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        assert_eq!(
            pr.notes.folder_links,
            vec![("docs/a.md".to_string(), "changes/".to_string())]
        );
    }

    #[test]
    fn a_pointer_of_the_conventions_to_a_document_that_layout_moves_fails_item_5() {
        // Pass 1 renames only the keys of `[conventions]`, so a pointer by path
        // is not edited. The first error of the 0.5 tree is not this one: the
        // pointer resolves before the plan and not after it (O5).
        let toml = format!("{MIXED}[conventions]\nguide = [\"docs/guides/log.md\"]\n");
        let tree = mem(&[
            ("spec-dna.toml", &toml),
            ("docs/guides/log.md", "---\ntype: guide\n---\n# Log\n"),
        ]);
        let (r, p) = go(&tree, &toml, true);
        assert!(p.moves.iter().any(|m| m.from == "docs/guides/log.md"));
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["a new error", "SD106", "docs/guides/log.md"]);
        // Without `--layout` the pointer stays valid.
        let (r, p) = go(&tree, &toml, false);
        assert!(prove_of(&tree, &r, &p).failures.is_empty());
    }

    #[test]
    fn the_report_counts_a_folder_link_that_a_partial_move_leaves() {
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            (
                "docs/a.md",
                "---\ntype: spec\n---\n# A\n\nSee [the change](changes/x/).\n",
            ),
            (
                "docs/changes/x/index.md",
                "---\ntype: change\nstatus: proposed\n---\n# X\n",
            ),
            ("docs/changes/x/pic.png", "png"),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        assert_eq!(
            pr.notes.folder_links,
            vec![("docs/a.md".to_string(), "changes/x/".to_string())]
        );
        // The proof does not rewrite the link.
        assert!(p.writes["docs/a.md"].contains("(changes/x/)"));
    }

    #[test]
    fn a_folder_that_moves_whole_has_a_rewritten_link_and_a_note_for_its_lost_index() {
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            (
                "docs/a.md",
                "---\ntype: spec\n---\n# A\n\nSee [the change](changes/x/).\n",
            ),
            (
                "docs/changes/x/index.md",
                "---\ntype: change\nstatus: proposed\n---\n# X\n",
            ),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        // The plan rewrites the link, but `intent.md` is not an index file, so
        // the link names a folder with no document now.
        assert_eq!(pr.notes.folder_links.len(), 1, "{:?}", pr.notes);
        assert!(
            p.writes["docs/a.md"].contains("(intent/x/)"),
            "{}",
            p.writes["docs/a.md"]
        );
    }

    #[test]
    fn the_report_counts_a_link_from_a_file_outside_the_roots_to_a_moved_file() {
        let tree = mem(&[
            ("spec-dna.toml", MIXED),
            (
                "docs/c.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n",
            ),
            ("docs/a.md", "---\ntype: spec\n---\n# A\n"),
            (
                "README.md",
                "# Project\n\n[change](docs/c.md) and [spec](docs/a.md).\n",
            ),
            ("tools/NOTES.md", "[change](../docs/c.md#top)\n"),
        ]);
        let (r, p) = go(&tree, MIXED, false);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        assert_eq!(
            pr.notes.outside_links,
            vec![
                ("README.md".to_string(), "docs/c.md".to_string()),
                ("tools/NOTES.md".to_string(), "../docs/c.md#top".to_string()),
            ]
        );
        assert!(pr.notes.folder_links.is_empty());
    }

    /// A corpus of a tree in memory, with the configuration of its own.
    fn corpus_of(tree: &TreeData, cfg: &Config) -> Corpus {
        tree.with(|t| {
            let paths: Vec<String> = t
                .files
                .iter()
                .filter(|f| crate::model::is_doc_path(f))
                .filter(|f| cfg.root_of(f).is_some_and(|i| !cfg.is_excluded(i, f)))
                .cloned()
                .collect();
            let bytes = (t.read)(&paths).unwrap();
            Corpus::build(
                cfg,
                t.files.to_vec(),
                paths
                    .iter()
                    .map(|p| {
                        (
                            p.clone(),
                            Arc::new(parse(std::str::from_utf8(&bytes[p]).unwrap())),
                        )
                    })
                    .collect(),
            )
        })
    }

    #[test]
    fn the_hand_written_corpus_gives_the_same_ids_and_hashes_as_the_plan_of_v05() {
        // `tests/fixtures/corpus` is the 0.6 form of `v05/` that a person wrote.
        // It does not come from the code under test, so it is a partial check.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus");
        let toml = std::fs::read_to_string(dir.join("spec-dna.toml")).unwrap();
        let hand_cfg = Config::parse(&toml).unwrap();
        let mut blobs = std::collections::HashMap::new();
        fn walk(
            d: &std::path::Path,
            rel: &str,
            out: &mut std::collections::HashMap<String, Vec<u8>>,
        ) {
            for e in std::fs::read_dir(d).unwrap() {
                let e = e.unwrap();
                let name = e.file_name().to_string_lossy().to_string();
                let path = if rel.is_empty() {
                    name
                } else {
                    format!("{rel}/{name}")
                };
                if e.file_type().unwrap().is_dir() {
                    walk(&e.path(), &path, out);
                } else {
                    out.insert(path, std::fs::read(e.path()).unwrap());
                }
            }
        }
        walk(&dir, "", &mut blobs);
        let hand = corpus_of(&TreeData::from_map(blobs), &hand_cfg);

        let (tree, r, p) = v05(false);
        let after = corpus_of(&after_tree(&tree, &p), &r.fin);
        let mut compared = 0;
        for h in &hand.docs {
            // The moved change of the hand-written corpus has other text.
            let Some(&i) = after
                .by_path
                .get(&h.path)
                .filter(|_| h.ty != DocType::Intent)
            else {
                continue;
            };
            let a = &after.docs[i as usize];
            assert_eq!(h.id, a.id, "{}", h.path);
            let reqs = |d: &crate::model::Doc| -> Vec<_> {
                d.reqs
                    .iter()
                    .map(|&r| (d.parsed.reqs[r].key.clone(), d.parsed.reqs[r].frozen))
                    .collect()
            };
            assert_eq!(reqs(h), reqs(a), "{}", h.path);
            let sections = |d: &crate::model::Doc| -> Vec<_> {
                d.parsed
                    .chunks
                    .iter()
                    .map(|c| (c.anchor.clone(), c.frozen))
                    .collect()
            };
            assert_eq!(sections(h), sections(a), "{}", h.path);
            compared += 1;
        }
        assert!(compared >= 6, "only {compared} documents are in both trees");
    }

    // --- the top files and the areas (0.6.1, D40 to D48) ---

    /// `v05/` with the overlay `top-files/`, planned with `--layout` and the
    /// pairs of `--areas`.
    fn v05_top(areas: &[(&str, &str)]) -> (TreeData, Run, DocsPlan) {
        use crate::upgrade_docs::tests::{fixture_tree, full_areas};
        let (base, toml) = fixture_case("v05");
        let mut blobs = base.with(|t| (t.read)(t.files)).unwrap();
        blobs.extend(
            fixture_tree("top-files")
                .with(|t| (t.read)(t.files))
                .unwrap(),
        );
        let tree = TreeData::from_map(blobs);
        let pairs: Vec<(String, String)> = areas
            .iter()
            .map(|(i, a)| (i.to_string(), a.to_string()))
            .collect();
        let (r, p) = full_areas(&tree, &toml, true, &pairs).unwrap_or_else(|e| panic!("{e:?}"));
        (tree, r, p)
    }

    #[test]
    fn the_plan_with_top_files_and_areas_passes_the_proof() {
        let (tree, r, p) = v05_top(&[("irrigation/pumps", "pumps")]);
        let kinds: Vec<MoveKind> = p.moves.iter().map(|m| m.kind).collect();
        assert_eq!(kinds.iter().filter(|k| **k == MoveKind::Top).count(), 5);
        assert_eq!(kinds.iter().filter(|k| **k == MoveKind::Area).count(), 1);
        let pr = prove_of(&tree, &r, &p);
        assert!(pr.failures.is_empty(), "{}", failed(&pr));
        // The proof checked the type of each document, also of a top file.
        assert_eq!(pr.types, r.classified.docs.len());
        assert!(pr.requirements >= 6, "{}", pr.requirements);
    }

    #[test]
    fn a_top_file_that_loses_its_alias_fails_the_proof() {
        let (tree, r, mut p) = v05_top(&[]);
        tamper(&r, &mut p, "docs/terms.md", |t| {
            t.replace("aliases:\n  - notes/glossary\n", "")
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["`notes/glossary`", "does not resolve"]);
    }

    #[test]
    fn a_top_file_with_a_type_line_of_another_type_fails_the_proof() {
        let (tree, r, mut p) = v05_top(&[]);
        tamper(&r, &mut p, "docs/terms.md", |t| {
            t.replacen("---\n", "---\ntype: research\n", 1)
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["docs/terms.md", "`research`", "gives `terms`"]);
    }

    #[test]
    fn a_requirement_text_that_changes_in_an_area_document_fails_the_proof() {
        let (tree, r, mut p) = v05_top(&[("irrigation/pumps", "pumps")]);
        tamper(&r, &mut p, "docs/requirements/pumps/pumps.md", |t| {
            t.replace("between 02:00 and 04:00", "between 01:00 and 04:00")
        });
        let pr = prove_of(&tree, &r, &p);
        assert!(!pr.failures.is_empty());
        has(&pr, &["fill-window"]);
    }

    #[test]
    fn an_area_document_that_loses_its_alias_fails_the_proof() {
        let (tree, r, mut p) = v05_top(&[("irrigation/pumps", "pumps")]);
        tamper(&r, &mut p, "docs/requirements/pumps/pumps.md", |t| {
            t.replace("  - irrigation/pumps\n", "")
        });
        let pr = prove_of(&tree, &r, &p);
        has(&pr, &["`irrigation/pumps#fill-window`", "does not resolve"]);
    }

    #[test]
    fn a_spec_that_is_made_a_top_file_fails_the_proof_on_its_requirement() {
        // A broken `classify` that gives `top` to a document with requirements:
        // the type of the top file has none, so the hashes differ.
        let (tree, mut r, _) = v05_top(&[]);
        r.classified
            .docs
            .iter_mut()
            .find(|c| c.path == "docs/irrigation/pumps.md")
            .unwrap()
            .top = Some(DocType::System);
        let p = tree
            .with(|t| crate::upgrade_docs::plan_docs(t, &r.classified, &r.fin, true, &[]))
            .unwrap();
        assert!(p.moves.iter().any(|m| m.to == "docs/system.md"));
        let pr = prove_of(&tree, &r, &p);
        assert!(!pr.failures.is_empty());
        has(&pr, &["fill-window"]);
    }

    #[test]
    fn a_document_of_another_type_that_is_made_a_top_file_fails_the_proof() {
        // A broken `classify` that gives `top` to a guide, and to an accepted
        // decision. The text and the ids stay, but the type would change, and
        // the locked text of a decision would not be checked any more.
        for (path, doc) in [
            ("docs/guides/logging.md", None),
            (
                "docs/d.md",
                Some("---\ntype: decision\nstatus: accepted\n---\n# D\n\n## Status\n\nAccepted.\n"),
            ),
        ] {
            let (tree, toml) = match doc {
                None => fixture_case("v05"),
                Some(text) => (mem(&[("docs/d.md", text)]), MIXED.to_string()),
            };
            let mut r = crate::upgrade_docs::tests::pipeline_layout(&tree, &toml, true);
            r.classified
                .docs
                .iter_mut()
                .find(|c| c.path == path)
                .unwrap_or_else(|| panic!("no {path}"))
                .top = Some(DocType::System);
            let p = tree
                .with(|t| crate::upgrade_docs::plan_docs(t, &r.classified, &r.fin, true, &[]))
                .unwrap();
            assert!(
                p.moves.iter().any(|m| m.to.ends_with("system.md")),
                "{path}"
            );
            let pr = prove_of(&tree, &r, &p);
            has(&pr, &[path, "can become a top file"]);
        }
    }
}
