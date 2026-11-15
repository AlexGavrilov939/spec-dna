//! The base of the change checks (design 0.6.0, "Gates"), the diff against it,
//! the requirements that the diff changes, and the checks of the change
//! lifecycle that need one tree only (`tree_checks`).
//!
//! The lifecycle base is the merge base of `--base` (or of `origin/HEAD`) with
//! `HEAD`. It is never the tip of `--base`: a branch that was cut from an old
//! commit is not behind the default branch because of that.

use crate::change::{self, Delta, Verb};
use crate::config::{ChangePolicy, Config};
use crate::diag::{Code, Diag};
use crate::git::{Git, PathChange};
use crate::hash::{Hash, hex};
use crate::model::{Corpus, Doc, DocType, Res};
use crate::parse::{ItemSection, ListItem};
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

/// The checks of the change lifecycle that read one tree: SD905 (the delta
/// part), SD912, SD914 (without the check of the diff), SD919 and SD920 (the
/// absent decision). They need no base and no history. `base` is the tree that
/// the gate compares with. A `REMOVE` line uses it to tell that the head already
/// has the result of the line. `layers` is the layer of each document of `c`.
///
/// A change whose intent is `closed` does not proceed, so its spec gives
/// nothing. All the checks return no diagnostic if the policy is `none`.
pub fn tree_checks(
    config: &Config,
    c: &Corpus,
    base: Option<&Corpus>,
    layers: &[Option<usize>],
) -> Vec<Diag> {
    if config.change_policy == ChangePolicy::None
        || [
            Code::FanOut,
            Code::StartedLimit,
            Code::DeltaInvalid,
            Code::ConcernOpen,
            Code::DecisionNeededUnaccepted,
        ]
        .iter()
        .all(|&k| config.is_off(k))
    {
        return Vec::new();
    }
    let changes = change::collect(c);
    let mut out = Vec::new();
    // The layers that the `intent` layer moves into.
    let intent_layer = config.layers.layer_of_type(DocType::Intent);
    let allowed: Option<Vec<&str>> = intent_layer.map(|i| {
        config.layers.layers()[i]
            .moves_to
            .iter()
            .map(String::as_str)
            .collect()
    });
    for ch in &changes {
        if ch.intent_status() == "closed" {
            continue;
        }
        let Some(spec) = &ch.spec else { continue };
        let d = &c.docs[spec.doc as usize];
        if spec.status == "approved" {
            concerns(d, &mut out);
            decisions(c, d, &mut out);
        }
        let mut targets: HashSet<String> = HashSet::new();
        for (line, item) in ch
            .delta(c)
            .into_iter()
            .zip(change::section_items(d, ItemSection::Delta))
        {
            let delta = match &line.parsed {
                Ok(delta) => delta,
                Err(reason) => {
                    out.push(crate::check::diag(
                        d,
                        line.line,
                        Code::DeltaInvalid,
                        short(&item.text),
                        format!("the delta line is not valid: {reason}"),
                    ));
                    continue;
                }
            };
            targets.insert(target_doc(c, &delta.id));
            if already_applied(c, base, delta) {
                continue;
            }
            if let Some(problem) = delta_problem(config, c, layers, allowed.as_deref(), delta) {
                out.push(crate::check::diag(
                    d,
                    line.line,
                    Code::DeltaInvalid,
                    delta.target(),
                    format!("`{}`: {problem}", delta.target()),
                ));
            }
        }
        interfaces(c, d, &mut out);
        let limit = config.max_fanout as usize;
        if targets.len() > limit {
            out.push(crate::check::diag(
                d,
                1,
                Code::FanOut,
                "",
                format!(
                    "the delta touches {} requirement documents, more than `max-fanout = {limit}`",
                    targets.len()
                ),
            ));
        }
    }
    let mut out = crate::check::apply_severity(config, out);
    out.extend(started_limit(config, intent_layer, &changes));
    out
}

/// The text of an item on one line, at most 60 characters. It is the target
/// of a diagnostic about an item that has no id.
fn short(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(60)
        .collect()
}

/// The name of the document that a delta line names: the id of the document in
/// the tree, or the id of the line if the tree has no such document.
fn target_doc(c: &Corpus, id: &str) -> String {
    match c.resolve(id) {
        Res::Found(i, None, _) => c.docs[i as usize].id.clone(),
        _ => id.to_string(),
    }
}

/// True if the tree already has the result of a delta line, so the line gives
/// no diagnostic (design T3). `ADD` and `CHANGE`: the head requirement has the
/// text of the line. `REMOVE`: the head has no such requirement, and the base
/// has it. With no base, a `REMOVE` of an absent requirement is not applied.
fn already_applied(c: &Corpus, base: Option<&Corpus>, d: &Delta) -> bool {
    let head = change::target_req(c, &d.id, &d.key);
    match d.verb {
        Verb::Add | Verb::Change => {
            matches!((head, d.frozen()), (Some(r), Some(f)) if r.frozen == f)
        }
        Verb::Remove => {
            head.is_none() && base.is_some_and(|b| change::target_req(b, &d.id, &d.key).is_some())
        }
    }
}

/// The first problem of a delta line with a valid form, or `None`. The order
/// is: the target document, its type, its layer, its key, its hash.
fn delta_problem(
    config: &Config,
    c: &Corpus,
    layers: &[Option<usize>],
    allowed: Option<&[&str]>,
    d: &Delta,
) -> Option<String> {
    let id = &d.id;
    let (doc, layer) = match c.resolve(id) {
        Res::Ambiguous(v) => return Some(format!("`{id}` names {} documents", v.len())),
        Res::Found(i, None, _) => (Some(&c.docs[i as usize]), layers[i as usize]),
        _ => (None, None),
    };
    let layer_problem = |layer: Option<usize>, what: &str| -> Option<String> {
        let allowed = allowed?;
        let Some(l) = layer else {
            return Some(format!("{what} is in no layer"));
        };
        let name = &config.layers.layers()[l].name;
        (!allowed.contains(&name.as_str())).then(|| {
            format!("{what} is in the layer `{name}`, and the `moves-to` list of the layer `intent` does not have it")
        })
    };
    let not_requirements = |ty: DocType| {
        format!(
            "`{id}` is a `{}` document; a delta line needs a document of the type `requirements`, `interface` or `rule`",
            ty.name()
        )
    };
    let Some(doc) = doc else {
        if d.verb != Verb::Add {
            return Some(format!("`{id}` names no document"));
        }
        // A new document: `finish` writes it at the place of its id.
        let rel = format!("{id}.md");
        let ty = config
            .layers
            .place_type(&rel)
            .unwrap_or(DocType::Requirements);
        if !ty.has_requirements() {
            return Some(not_requirements(ty));
        }
        return layer_problem(
            config.layers.layer_of(&rel, ty),
            &format!("the new document `{id}`"),
        );
    };
    if !doc.has_layer() {
        return Some(format!("`{id}` has no valid type"));
    }
    if !doc.ty.has_requirements() {
        return Some(not_requirements(doc.ty));
    }
    if let Some(p) = layer_problem(layer, &format!("`{id}`")) {
        return Some(p);
    }
    let req = doc.req(&d.key).map(|i| &doc.parsed.reqs[i]);
    match (d.verb, req) {
        (Verb::Add, Some(_)) => Some(format!(
            "`{id}` has the key `{}` already; an ADD line needs a new key",
            d.key
        )),
        (Verb::Add, None) => None,
        (_, None) => Some(format!("`{id}` has no requirement `{}`", d.key)),
        (_, Some(r)) => {
            let hash = d.hash.as_deref().unwrap_or("");
            (!change::hash_matches(r, hash)).then(|| {
                format!(
                    "the hash `{hash}` is not the hash of the requirement now (`{}`); read the requirement again and copy the hash from `spec-dna show`",
                    hex(&r.hash)
                )
            })
        }
    }
}

/// SD914 for the items of `Interfaces`: `CHANGE <interface id>: <text>`. An
/// item with another form, an absent document and a document that is not an
/// interface give SD914.
fn interfaces(c: &Corpus, spec: &Doc, out: &mut Vec<Diag>) {
    for item in change::section_items(spec, ItemSection::Interfaces) {
        let mut push = |target: String, message: String| {
            out.push(crate::check::diag(
                spec,
                item.line,
                Code::DeltaInvalid,
                target,
                message,
            ))
        };
        let Some(id) = interface_id(item) else {
            push(
                short(&item.text),
                "an item of `Interfaces` is `CHANGE <interface id>: <text>`".to_string(),
            );
            continue;
        };
        match c.resolve(id) {
            Res::Found(i, None, _) if c.docs[i as usize].ty == DocType::Interface => {}
            Res::Found(i, None, _) => push(
                id.to_string(),
                format!(
                    "`{id}` is a `{}` document, not an interface",
                    c.docs[i as usize].ty.name()
                ),
            ),
            Res::Ambiguous(v) => push(
                id.to_string(),
                format!("`{id}` names {} documents", v.len()),
            ),
            _ => push(
                id.to_string(),
                format!("`{id}` names no interface document"),
            ),
        }
    }
}

/// The id of an item of `Interfaces`, or `None` if the item is not in the form
/// `CHANGE <interface id>: <text>`. The id can be in `[[ ]]` or in a code span.
fn interface_id(item: &ListItem) -> Option<&str> {
    let rest = item.text.strip_prefix("CHANGE")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (id, text) = rest.split_once(':')?;
    let id = id.trim().trim_matches(['[', ']', '`']);
    (!id.is_empty() && !id.contains(char::is_whitespace) && !text.trim().is_empty()).then_some(id)
}

/// SD919: each open concern (`- [ ]`) of a spec that is `approved`.
fn concerns(spec: &Doc, out: &mut Vec<Diag>) {
    for item in change::section_items(spec, ItemSection::Concerns) {
        if item.task == Some(false) {
            out.push(crate::check::diag(
                spec,
                item.line,
                Code::ConcernOpen,
                short(&item.text),
                format!(
                    "the spec is `approved`, and the concern `{}` is open",
                    short(&item.text)
                ),
            ));
        }
    }
}

/// SD920, the absent part: a decision of `Decisions` that names no document,
/// in a spec that is `approved`. The check that the decision is `accepted`
/// runs when `finish` runs.
fn decisions(c: &Corpus, spec: &Doc, out: &mut Vec<Diag>) {
    for item in change::section_items(spec, ItemSection::Decisions) {
        let mut rest = item.text.as_str();
        while let Some(start) = rest.find("[[") {
            let Some(len) = rest[start + 2..].find("]]") else {
                break;
            };
            let inner = &rest[start + 2..start + 2 + len];
            rest = &rest[start + 2 + len + 2..];
            let id = inner.split(['#', '|']).next().unwrap_or("").trim();
            if !id.is_empty() && matches!(c.resolve(id), Res::Missing) {
                out.push(crate::check::diag(
                    spec,
                    item.line,
                    Code::DecisionNeededUnaccepted,
                    id,
                    format!("the spec is `approved`, and the decision `{id}` names no document"),
                ));
            }
        }
    }
}

/// SD912: more changes than `max-started` of the `intent` layer have an
/// `accepted` intent. The diagnostic is on `spec-dna.toml`, as the one of SD902
/// is, so the rules of a root do not apply.
fn started_limit(config: &Config, layer: Option<usize>, changes: &[change::Change]) -> Vec<Diag> {
    let Some(layer) = layer.map(|i| &config.layers.layers()[i]) else {
        return Vec::new();
    };
    let Some(max) = layer.max_started else {
        return Vec::new();
    };
    let Some(severity) = config.severity(Code::StartedLimit, None) else {
        return Vec::new();
    };
    let started = changes
        .iter()
        .filter(|ch| ch.intent_status() == "accepted")
        .count();
    if started <= max as usize {
        return Vec::new();
    }
    vec![Diag {
        path: crate::config::FILE_NAME.to_string(),
        line: 1,
        code: Code::StartedLimit,
        doc: crate::config::FILE_NAME.to_string(),
        target: format!("{}:max-started", layer.name),
        message: format!(
            "{started} changes have an `accepted` intent, more than `max-started = {max}` of the layer `{}`",
            layer.name
        ),
        severity,
    }]
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
