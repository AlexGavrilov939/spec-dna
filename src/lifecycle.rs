//! The base of the change checks (design 0.6.0, "Gates"), the diff against it,
//! the requirements that the diff changes, the checks of the change lifecycle
//! that need one tree only (`tree_checks`), and the checks of the diff against
//! the base (`base_checks`: SD905, SD910, SD913 and SD917).
//!
//! The lifecycle base is the merge base of `--base` (or of `origin/HEAD`) with
//! `HEAD`. It is never the tip of `--base`: a branch that was cut from an old
//! commit is not behind the default branch because of that.

use crate::change::{self, Change, Delta, Verb};
use crate::config::{ChangePolicy, Config};
use crate::diag::{Code, Diag};
use crate::git::{Git, PathChange};
use crate::hash::{Hash, hex};
use crate::model::{Class, Corpus, Doc, DocType, Res};
use crate::parse::{ItemSection, ListItem, ParsedDoc, parse};
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

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

    /// The kind of change of a path, or `None` if the diff does not have it.
    pub fn kind(&self, path: &str) -> Option<PathChange> {
        self.files.iter().find(|(p, _)| p == path).map(|(_, k)| *k)
    }
}

/// How a requirement differs between the base and the head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReqChangeKind {
    Changed,
    Added,
    Removed,
}

/// A requirement that the diff changed, added or removed.
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

/// The folders of the `intent` layer from the repository root, one for each
/// root, with no final `/`. A project with no `intent` layer has none.
pub fn intent_dirs(config: &Config) -> Vec<String> {
    let Some(dir) = config
        .layers
        .layer_of_type(DocType::Intent)
        .and_then(|i| config.layers.layers()[i].dir())
    else {
        return Vec::new();
    };
    let mut out: Vec<String> = config
        .roots
        .iter()
        .map(|r| {
            if r.path.is_empty() {
                dir.to_string()
            } else {
                format!("{}/{dir}", r.path)
            }
        })
        .collect();
    out.dedup();
    out
}

/// True for a document whose requirements spec-dna reads: a `rule`,
/// `requirements` or `interface` document with a valid type, which is not a
/// stub and not a generated index.
pub fn requirement_doc(d: &Doc) -> bool {
    d.ty.has_requirements() && !d.type_failed && d.moved_to.is_none() && d.generated.is_none()
}

/// The number of changes with an `accepted` intent (SD912).
pub fn started_count(c: &Corpus) -> usize {
    change::collect(c)
        .iter()
        .filter(|ch| ch.intent_status() == "accepted")
        .count()
}

/// The result of `base_checks`.
#[derive(Default)]
pub struct BaseReport {
    /// The diagnostics of the diff. All of them are new.
    pub diags: Vec<Diag>,
    /// The `Files` patterns of the changes whose plan covers a document of the
    /// diff. A document that one of them matches gives no SD921 (D19).
    pub exempt: Vec<String>,
    /// True if a commit of the branch added a change and the branch finished
    /// it (B5). Such a change is a change of the diff, so SD921 stays silent.
    pub born: bool,
}

/// How a change came into the diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// The diff touches a file of its folder.
    Touched,
    /// A commit of the branch added its folder, and neither tree has it (B5).
    Born,
    /// Its `Files` match a path of the diff, and its folder is not in the diff.
    FilesMatch,
}

/// The tree that holds the artifacts of a change of the diff.
enum Side {
    Head,
    /// The diff deletes the folder, so the checks read the base tree.
    Base,
    /// The newest artifacts of a branch-born change, read from history.
    Born(Box<Corpus>),
}

/// A change of the diff (design K2), with the lists that the checks use.
struct DiffChange {
    change: Change,
    origin: Origin,
    side: Side,
    /// The `Files` patterns of the plan.
    files: Vec<String>,
    /// The scope of SD913: the folder, the `Files` patterns and the documents
    /// that the delta and `Interfaces` name.
    scope: Vec<String>,
    /// The number of items in `Delta`.
    delta_count: usize,
    /// The ids that the valid delta lines name.
    targets: Vec<String>,
}

/// The paths of the documents of both trees, by kind.
#[derive(Default)]
struct DocPaths {
    /// A document that a layer owns.
    owned: HashSet<String>,
    /// A generated index.
    generated: HashSet<String>,
    /// A document that has requirements.
    requirements: HashSet<String>,
}

struct Ctx<'a> {
    config: &'a Config,
    git: &'a Git,
    head: &'a Corpus,
    base: &'a Corpus,
    base_rev: &'a str,
    diff: &'a Diff,
    head_layers: Vec<Option<usize>>,
    base_layers: Vec<Option<usize>>,
    paths: OnceCell<DocPaths>,
}

impl<'a> Ctx<'a> {
    fn corpus<'s>(&'s self, ch: &'s DiffChange) -> &'s Corpus {
        match &ch.side {
            Side::Head => self.head,
            Side::Base => self.base,
            Side::Born(c) => c,
        }
    }

    /// The tree and the layers that the targets of the delta of a change
    /// resolve in.
    fn target_tree(&self, ch: &DiffChange) -> (&Corpus, &[Option<usize>]) {
        match ch.side {
            Side::Base => (self.base, &self.base_layers),
            _ => (self.head, &self.head_layers),
        }
    }

    /// The paths of the documents that an id names: the documents of the trees,
    /// or the places that a new document with that id can have.
    fn named_paths(&self, id: &str) -> Vec<String> {
        let mut v = Vec::new();
        for c in [self.head, self.base] {
            if let Res::Found(i, None, _) = c.resolve(id) {
                v.push(c.docs[i as usize].path.clone());
            }
        }
        if v.is_empty() {
            for r in &self.config.roots {
                v.push(if r.path.is_empty() {
                    format!("{id}.md")
                } else {
                    format!("{}/{id}.md", r.path)
                });
            }
        }
        v
    }

    fn paths(&self) -> &DocPaths {
        self.paths.get_or_init(|| {
            let mut out = DocPaths::default();
            for (c, layers) in [
                (self.head, &self.head_layers),
                (self.base, &self.base_layers),
            ] {
                for (d, layer) in c.docs.iter().zip(layers) {
                    if layer.is_some() {
                        out.owned.insert(d.path.clone());
                    }
                    if d.generated.is_some() {
                        out.generated.insert(d.path.clone());
                    }
                    if requirement_doc(d) {
                        out.requirements.insert(d.path.clone());
                    }
                }
            }
            out
        })
    }

    /// The layer of the document that a delta line names. A new document has
    /// the layer of its place.
    fn target_layer(&self, ch: &DiffChange, id: &str) -> Option<usize> {
        let (c, layers) = self.target_tree(ch);
        match c.resolve(id) {
            Res::Found(i, None, _) => layers[i as usize],
            Res::Missing => {
                let rel = format!("{id}.md");
                let ty = self
                    .config
                    .layers
                    .place_type(&rel)
                    .unwrap_or(DocType::Requirements);
                self.config.layers.layer_of(&rel, ty)
            }
            _ => None,
        }
    }

    fn layer_name(&self, layer: Option<usize>) -> Option<&str> {
        layer.map(|l| self.config.layers.layers()[l].name.as_str())
    }

    fn is_high_risk_layer(&self, layer: Option<usize>) -> bool {
        self.layer_name(layer)
            .is_some_and(|n| self.config.high_risk_layers.iter().any(|h| h == n))
    }
}

/// Runs the checks of the diff against the lifecycle base: SD905 (the part for
/// a change with no delta), SD910, SD913 and SD917. It also finds the data for
/// SD921. `base` is the lifecycle tree, `base_rev` its commit, and `diff` the
/// diff of the head against it. The checks read the artifacts of a change from
/// the head tree, and from the base tree if the diff deletes the folder. They
/// read approvals from the base tree only, so an approval in the same pull
/// request does not count.
pub fn base_checks(
    config: &Config,
    git: &Git,
    head: &Corpus,
    base: &Corpus,
    base_rev: &str,
    diff: &Diff,
) -> Result<BaseReport, String> {
    let on = |k: Code| !config.is_off(k);
    if config.change_policy == ChangePolicy::None
        || ![
            Code::FanOut,
            Code::GateApprovalMissing,
            Code::OutsidePlan,
            Code::ClassTooLow,
            Code::TruthChangedWithoutChange,
        ]
        .iter()
        .any(|&k| on(k))
    {
        return Ok(BaseReport::default());
    }
    let ctx = Ctx {
        config,
        git,
        head,
        base,
        base_rev,
        diff,
        head_layers: crate::check::doc_layers(config, head),
        base_layers: crate::check::doc_layers(config, base),
        paths: OnceCell::new(),
    };
    let changes = diff_changes(&ctx)?;
    let mut out = Vec::new();
    if on(Code::GateApprovalMissing) {
        out.extend(approval_missing(&ctx, &changes));
    }
    if on(Code::OutsidePlan) {
        out.extend(outside_plan(&ctx, &changes));
    }
    if on(Code::ClassTooLow) {
        out.extend(class_too_low(&ctx, &changes));
    }
    if on(Code::FanOut) {
        out.extend(routine_fan_out(&ctx, &changes));
    }
    // A plan that covers a document: the plan of a change in the diff, and the
    // approved plan of a started change in the base.
    let mut exempt: Vec<String> = changes
        .iter()
        .filter(|c| c.origin != Origin::FilesMatch)
        .flat_map(|c| c.files.iter().cloned())
        .collect();
    for b in change::collect(base) {
        if b.intent_status() == "accepted" && plan_status(&b) == "approved" {
            exempt.extend(b.files(base));
        }
    }
    Ok(BaseReport {
        diags: crate::check::apply_severity(config, out),
        exempt,
        born: changes.iter().any(|c| c.origin == Origin::Born),
    })
}

fn plan_status(c: &Change) -> &str {
    c.plan.as_ref().map_or("", |a| &a.status)
}

fn spec_status(c: &Change) -> &str {
    c.spec.as_ref().map_or("", |a| &a.status)
}

/// A diagnostic on a path that is not a document.
fn path_diag(path: &str, code: Code, target: &str, message: String) -> Diag {
    Diag {
        path: path.to_string(),
        line: 1,
        code,
        doc: path.to_string(),
        target: target.to_string(),
        message,
        severity: code.severity(),
    }
}

/// The changes of the diff (design K2): the folders that the diff touches, the
/// changes that a commit of the branch added and finished (B5), and the changes
/// whose `Files` match a path of the diff. A diff with none is a small fix.
fn diff_changes(ctx: &Ctx) -> Result<Vec<DiffChange>, String> {
    let head_changes = change::collect(ctx.head);
    let base_changes = change::collect(ctx.base);
    let mut out: Vec<DiffChange> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (list, base_side) in [(&head_changes, false), (&base_changes, true)] {
        for ch in list {
            if !seen.contains(&ch.dir) && ctx.diff.under(&ch.dir).next().is_some() {
                seen.insert(ch.dir.clone());
                let side = if base_side { Side::Base } else { Side::Head };
                out.push(make(ctx, ch.clone(), Origin::Touched, side));
            }
        }
    }
    for dir in intent_dirs(ctx.config) {
        for folder in ctx.git.added_intents(ctx.base_rev, "HEAD", &dir)? {
            if seen.insert(folder.clone()) {
                let (change, corpus) = born_change(ctx, &folder);
                out.push(make(ctx, change, Origin::Born, Side::Born(corpus)));
            }
        }
    }
    for ch in &head_changes {
        if seen.contains(&ch.dir) {
            continue;
        }
        let files = ch.files(ctx.head);
        if ctx
            .diff
            .files
            .iter()
            .any(|(p, _)| change::in_scope(&files, p))
        {
            seen.insert(ch.dir.clone());
            out.push(make(ctx, ch.clone(), Origin::FilesMatch, Side::Head));
        }
    }
    Ok(out)
}

fn make(ctx: &Ctx, change: Change, origin: Origin, side: Side) -> DiffChange {
    let corpus: &Corpus = match &side {
        Side::Head => ctx.head,
        Side::Base => ctx.base,
        Side::Born(c) => c,
    };
    let files = change.files(corpus);
    let mut scope = vec![format!("{}/", change.dir)];
    scope.extend(files.iter().cloned());
    let lines = change.delta(corpus);
    let mut targets = Vec::new();
    for l in &lines {
        if let Ok(d) = &l.parsed {
            scope.extend(ctx.named_paths(&d.id));
            targets.push(d.id.clone());
        }
    }
    if let Some(a) = &change.spec {
        let doc = &corpus.docs[a.doc as usize];
        for item in change::section_items(doc, ItemSection::Interfaces) {
            if let Some(id) = interface_id(item) {
                scope.extend(ctx.named_paths(id));
            }
        }
    }
    DiffChange {
        delta_count: lines.len(),
        change,
        origin,
        side,
        files,
        scope,
        targets,
    }
}

/// The newest artifacts of a change that a commit of the branch added and
/// deleted: the tree before its last commit. If git cannot give them (a shallow
/// clone, a merge), the change has no artifact, and the checks that need one
/// report the change by its folder.
fn born_change(ctx: &Ctx, folder: &str) -> (Change, Box<Corpus>) {
    let corpus = born_corpus(ctx, folder)
        .unwrap_or_else(|| Corpus::build(ctx.config, Vec::new(), Vec::new()));
    let change = change::collect(&corpus)
        .into_iter()
        .find(|c| c.dir == folder)
        .unwrap_or_else(|| Change {
            slug: folder.rsplit('/').next().unwrap_or(folder).to_string(),
            dir: folder.to_string(),
            intent: None,
            spec: None,
            plan: None,
            class: Class::Routine,
            areas: Vec::new(),
            from: None,
        });
    (change, Box::new(corpus))
}

fn born_corpus(ctx: &Ctx, folder: &str) -> Option<Corpus> {
    let range = format!("{}..HEAD", ctx.base_rev);
    let log = ctx
        .git
        .file_log(&range, &format!("{folder}/intent.md"))
        .ok()?;
    let (last, _) = log.first()?;
    let tree = ctx.git.ls_tree(&format!("{last}^")).ok()?;
    let prefix = format!("{folder}/");
    let wanted: Vec<(String, String)> = tree
        .into_iter()
        .filter(|(p, _, sub)| {
            !sub && p
                .strip_prefix(&prefix)
                .is_some_and(|n| matches!(n, "intent.md" | "spec.md" | "plan.md"))
        })
        .map(|(p, oid, _)| (p, oid))
        .collect();
    let ids: Vec<String> = wanted.iter().map(|w| w.1.clone()).collect();
    let blobs = ctx.git.read_blobs(&ids).ok()?;
    let docs: Vec<(String, Arc<ParsedDoc>)> = wanted
        .iter()
        .filter_map(|(p, oid)| {
            let bytes = blobs.get(oid)?;
            Some((p.clone(), Arc::new(parse(&String::from_utf8_lossy(bytes)))))
        })
        .collect();
    if docs.is_empty() {
        return None;
    }
    Some(Corpus::build(
        ctx.config,
        docs.iter().map(|d| d.0.clone()).collect(),
        docs,
    ))
}

/// SD910. A change adds or edits an artifact of a stage, and the base tree does
/// not show the approval of the artifact before it. Also: code under the `Files`
/// of a change, and no such change has an `approved` plan in the base tree.
fn approval_missing(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    let base_changes = change::collect(ctx.base);
    let base_of = |dir: &str| base_changes.iter().find(|b| b.dir == dir);
    let mut out = Vec::new();
    for ch in changes {
        let corpus = ctx.corpus(ch);
        let c = &ch.change;
        if ch.origin == Origin::Born {
            // The branch added the change and finished it, so the base tree
            // has no approval of any artifact.
            let message = format!(
                "the change `{}` was added and finished in this branch, and the base tree has no approval of it; get the approval of each artifact into the base first",
                c.slug
            );
            let path = format!("{}/intent.md", c.dir);
            out.push(match &c.intent {
                Some(a) => crate::check::diag(
                    &corpus.docs[a.doc as usize],
                    1,
                    Code::GateApprovalMissing,
                    "",
                    message,
                ),
                None => path_diag(&path, Code::GateApprovalMissing, "", message),
            });
            continue;
        }
        if matches!(ch.side, Side::Base) {
            continue;
        }
        let base_ch = base_of(&c.dir);
        for (name, art) in [("spec.md", &c.spec), ("plan.md", &c.plan)] {
            let Some(a) = art else { continue };
            let d = &corpus.docs[a.doc as usize];
            if !matches!(
                ctx.diff.kind(&d.path),
                Some(PathChange::Added | PathChange::Modified)
            ) {
                continue;
            }
            let (need, ok, status) = if name == "spec.md" || c.class != Class::HigherRisk {
                (
                    "intent.md",
                    base_ch.is_some_and(|b| b.intent_status() == "accepted"),
                    "accepted",
                )
            } else {
                (
                    "spec.md",
                    base_ch.is_some_and(|b| spec_status(b) == "approved"),
                    "approved",
                )
            };
            if !ok {
                out.push(crate::check::diag(
                    d,
                    1,
                    Code::GateApprovalMissing,
                    need,
                    format!(
                        "`{name}` changes, and the base tree has no `{status}` `{need}` for the change `{}`; get the approval of the earlier artifact into the base first",
                        c.slug
                    ),
                ));
            }
        }
    }
    // Code under the `Files` of a plan that the base tree does not approve.
    let approved = |ch: &&DiffChange| {
        ch.origin != Origin::Born
            && base_of(&ch.change.dir).is_some_and(|b| plan_status(b) == "approved")
    };
    for (p, _) in &ctx.diff.files {
        if changes
            .iter()
            .any(|c| p.starts_with(&format!("{}/", c.change.dir)))
        {
            continue;
        }
        let matching: Vec<&DiffChange> = changes
            .iter()
            .filter(|c| change::in_scope(&c.files, p))
            .collect();
        if matching.is_empty()
            || matching.iter().any(approved)
            || matching.iter().all(|c| c.origin == Origin::Born)
            || ctx.paths().owned.contains(p)
        {
            continue;
        }
        let first = matching
            .iter()
            .find(|c| c.origin != Origin::Born)
            .unwrap_or(&matching[0]);
        let doc = match &first.change.plan {
            Some(a) => ctx.corpus(first).docs[a.doc as usize].id.clone(),
            None => first.change.slug.clone(),
        };
        out.push(Diag {
            doc,
            ..path_diag(
                p,
                Code::GateApprovalMissing,
                p,
                format!(
                    "`{p}` is under `Files` of the change `{}`, and the base tree has no `approved` plan for it; get the approval of the plan into the base first",
                    first.change.slug
                ),
            )
        });
    }
    out
}

/// SD913: a path of the diff is outside the scope of each change of the diff.
fn outside_plan(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    if changes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (p, _) in &ctx.diff.files {
        if changes.iter().any(|c| change::in_scope(&c.scope, p))
            || ctx.paths().generated.contains(p)
        {
            continue;
        }
        out.push(path_diag(
            p,
            Code::OutsidePlan,
            "",
            format!(
                "`{p}` is in the diff, and no `Files` item of a change in the diff matches it; add it to `Files` of the plan, or move the edit to a change of its own"
            ),
        ));
    }
    out
}

/// True if a glob of `high-risk-paths` matches the path.
fn high_risk_path(config: &Config, path: &str) -> Option<String> {
    let path = path.trim_start_matches("./").trim_start_matches('/');
    config
        .high_risk_paths
        .iter()
        .find(|g| crate::glob::matches(g, path))
        .cloned()
}

/// SD917: the class of an intent is lower than the minimum that the diff gives.
/// A diff with no change has no intent. It gives SD917 only for a path of
/// `high-risk-paths` and a requirement of a layer of `high-risk-layers`.
fn class_too_low(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    let config = ctx.config;
    // The requirements that the diff changes in a layer of `high-risk-layers`.
    let risky = if config.high_risk_layers.is_empty() {
        Vec::new()
    } else {
        let wanted = |d: &Doc| {
            requirement_doc(d) && ctx.is_high_risk_layer(crate::check::doc_layer(config, d))
        };
        requirement_changes(ctx.base, ctx.head, wanted)
    };
    let layer_of_path = |path: &str| -> String {
        ctx.head
            .docs
            .iter()
            .chain(&ctx.base.docs)
            .find(|d| d.path == path)
            .and_then(|d| ctx.layer_name(crate::check::doc_layer(config, d)))
            .unwrap_or("")
            .to_string()
    };
    let mut out = Vec::new();
    if changes.is_empty() {
        for (id, list) in by_document(&risky) {
            let first = list[0];
            out.push(Diag {
                path: first.path.clone(),
                line: first.line.max(1),
                code: Code::ClassTooLow,
                doc: id.to_string(),
                target: String::new(),
                message: format!(
                    "the diff changes {} requirement(s) of `{id}` in the layer `{}` of `high-risk-layers`, and it has no intent; write an intent with `class: higher-risk`",
                    list.len(),
                    layer_of_path(&first.path)
                ),
                severity: Code::ClassTooLow.severity(),
            });
        }
        for (p, _) in &ctx.diff.files {
            if let Some(g) = high_risk_path(config, p) {
                out.push(path_diag(
                    p,
                    Code::ClassTooLow,
                    &g,
                    format!(
                        "`{p}` matches `{g}` of `high-risk-paths`, and the diff has no intent; write an intent with `class: higher-risk`"
                    ),
                ));
            }
        }
        return out;
    }
    let sole = changes.len() == 1;
    for ch in changes {
        let Some(intent) = &ch.change.intent else {
            continue;
        };
        if ch.change.class != Class::Routine {
            continue;
        }
        let mine = |path: &str| sole || change::in_scope(&ch.files, path);
        let mut reason: Option<(String, String)> = None;
        for id in &ch.targets {
            let layer = ctx.target_layer(ch, id);
            if ctx.is_high_risk_layer(layer) {
                reason = Some((
                    id.clone(),
                    format!(
                        "the delta targets `{id}` in the layer `{}` of `high-risk-layers`",
                        ctx.layer_name(layer).unwrap_or("")
                    ),
                ));
                break;
            }
        }
        if reason.is_none()
            && ch.delta_count == 0
            && let Some(r) = risky.iter().find(|r| mine(&r.path))
        {
            reason = Some((
                r.doc.clone(),
                format!(
                    "the diff changes the requirement `{}#{}` in the layer `{}` of `high-risk-layers`",
                    r.doc,
                    r.key,
                    layer_of_path(&r.path)
                ),
            ));
        }
        if reason.is_none() {
            let from_diff = ctx
                .diff
                .files
                .iter()
                .map(|(p, _)| p.as_str())
                .filter(|p| mine(p))
                .find_map(|p| high_risk_path(config, p).map(|g| (p.to_string(), g)));
            let from_plan = ch
                .files
                .iter()
                .find_map(|f| high_risk_path(config, f).map(|g| (f.clone(), g)));
            if let Some((p, g)) = from_diff.or(from_plan) {
                reason = Some((
                    g.clone(),
                    format!("`{p}` matches `{g}` of `high-risk-paths`"),
                ));
            }
        }
        if let Some((target, why)) = reason {
            out.push(crate::check::diag(
                &ctx.corpus(ch).docs[intent.doc as usize],
                1,
                Code::ClassTooLow,
                target,
                format!(
                    "the class of the change `{}` is `routine`, and {why}; set `class: higher-risk`",
                    ch.change.slug
                ),
            ));
        }
    }
    out
}

/// SD905, the part for a change with no delta: the diff of the change touches
/// more requirement documents than `max-fanout`.
fn routine_fan_out(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    let limit = ctx.config.max_fanout as usize;
    let sole = changes.len() == 1;
    let mut out = Vec::new();
    for ch in changes.iter().filter(|c| c.delta_count == 0) {
        let corpus = ctx.corpus(ch);
        let c = &ch.change;
        let Some(anchor) = c.intent.as_ref().or(c.plan.as_ref()).or(c.spec.as_ref()) else {
            continue;
        };
        let docs: HashSet<&str> = ctx
            .diff
            .files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(|p| ctx.paths().requirements.contains(*p))
            .filter(|p| sole || change::in_scope(&ch.files, p))
            .collect();
        if docs.len() > limit {
            out.push(crate::check::diag(
                &corpus.docs[anchor.doc as usize],
                1,
                Code::FanOut,
                "",
                format!(
                    "the diff of the change touches {} requirement documents, more than `max-fanout = {limit}`",
                    docs.len()
                ),
            ));
        }
    }
    out
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
    fn the_checks_of_the_diff_read_nothing_under_the_policy_none() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        let git = Git::new(std::path::Path::new("/nonexistent-spec-dna-path"));
        let c = corpus(&[]);
        let diff = Diff {
            files: vec![("src/a.rs".to_string(), PathChange::Added)],
        };
        let r = base_checks(&cfg, &git, &c, &c, "0000", &diff).unwrap();
        assert!(r.diags.is_empty() && r.exempt.is_empty() && !r.born);
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
