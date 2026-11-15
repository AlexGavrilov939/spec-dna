//! The documents of `upgrade` (design 0.6.0, D141 to D146). The plan is pure:
//! it reads a tree and writes nothing. It reads each file through the `Tree`
//! of the caller (the tree of `HEAD`, or a base commit for `check`), never from
//! the work directory.
//!
//! The work has 2 phases, because the front matter depends on the final
//! configuration, and the final configuration depends on the documents.
//!
//! 1. `classify` gives the 0.5 type and status of each document, and the
//!    `Facts` of pass 2 of `upgrade_config`.
//! 2. `plan_docs` takes the final configuration. It plans the moves
//!    (`plan_moves`: the changes, and the folders of `--layout`), then
//!    `edit_docs` rewrites the front matter at the path where each document
//!    ends (`type:` where the place gives another type). Then it writes the
//!    old ids as aliases and rewrites the relative links, in one pass against
//!    the tree before the moves, as `mv` does for one move.
//!
//! The result has no side effect. S8 (the proof), S10 (the command) and S11
//! (the base plan of `check`) use the same data.

use crate::config::Config;
use crate::finish;
use crate::frontmatter::{self, Value};
use crate::git::Git;
use crate::hash::nfc;
use crate::model::{Corpus, DocType, is_doc_path, root_doc_id};
use crate::mv::{self, add_aliases, rewrite_links};
use crate::parse::parse;
use crate::repo::Fatal;
use crate::upgrade_config::Facts;
use crate::upgrade_old::{self as names, Kind, OldConfig, OldRoot, change_status, type_of};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// The reader of a tree: the bytes of each path that it gets.
pub type ReadFn<'a> = dyn Fn(&[String]) -> Result<HashMap<String, Vec<u8>>, Fatal> + 'a;

/// One tree: `HEAD` for `upgrade`, a base commit for `check`. `files` are the
/// paths of the tree, sorted. `read` gives the bytes of the paths that it gets.
pub struct Tree<'a> {
    pub files: &'a [String],
    pub read: &'a ReadFn<'a>,
}

type Reader = Box<ReadFn<'static>>;

/// A tree that owns its list of files and its reader.
pub struct TreeData {
    files: Vec<String>,
    reader: Reader,
}

impl TreeData {
    /// A tree in memory: each path with its bytes.
    #[cfg(test)]
    pub fn from_map(blobs: HashMap<String, Vec<u8>>) -> TreeData {
        let mut files: Vec<String> = blobs.keys().cloned().collect();
        files.sort();
        TreeData {
            files,
            reader: Box::new(move |paths| {
                paths
                    .iter()
                    .map(|p| {
                        blobs
                            .get(p)
                            .map(|b| (p.clone(), b.clone()))
                            .ok_or_else(|| Fatal::Io(format!("{p}: not in the tree")))
                    })
                    .collect()
            }),
        }
    }

    /// The tree of a commit (`HEAD` for `upgrade`, a base for `check`): the
    /// files and the blobs of git, and never the work directory. A submodule
    /// is not a file of the tree.
    pub fn from_git(git: &Git, rev: &str) -> Result<TreeData, Fatal> {
        let entries = git.ls_tree(rev).map_err(Fatal::Io)?;
        let mut ids: HashMap<String, String> = HashMap::new();
        for (path, oid, submodule) in entries {
            if !submodule {
                ids.insert(path, oid);
            }
        }
        let mut files: Vec<String> = ids.keys().cloned().collect();
        files.sort();
        let git = git.clone();
        Ok(TreeData {
            files,
            reader: Box::new(move |paths| {
                let mut want = Vec::with_capacity(paths.len());
                for p in paths {
                    let id = ids
                        .get(p)
                        .ok_or_else(|| Fatal::Io(format!("{p}: not in the tree")))?;
                    want.push(id.clone());
                }
                let blobs = git.read_blobs(&want).map_err(Fatal::Io)?;
                paths
                    .iter()
                    .zip(&want)
                    .map(|(p, id)| {
                        blobs
                            .get(id)
                            .map(|b| (p.clone(), b.clone()))
                            .ok_or_else(|| Fatal::Io(format!("{p}: the blob {id} is missing")))
                    })
                    .collect()
            }),
        })
    }

    /// Runs `f` on a view of the tree.
    pub fn with<R>(&self, f: impl FnOnce(&Tree) -> R) -> R {
        f(&Tree {
            files: &self.files,
            read: &*self.reader,
        })
    }
}

/// The documents and the facts of the tree, by the 0.5 rules or the 0.6 rules.
pub struct Basis<'a> {
    /// The configuration after pass 1: it gives the roots, which are the 0.5
    /// roots.
    pub cfg: &'a Config,
    /// The 0.5 file, if the pin is before 0.6.0 (D137). With `None`, the 0.6
    /// rule applies, and only a 0.5 name changes (D141).
    pub old: Option<&'a OldConfig>,
}

/// What `upgrade` does to a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Nothing that moves it.
    Keep,
    /// A change that becomes an `intent` and moves (D144).
    Move,
    /// A `done` change: it stays as a `research` document (D142).
    Done,
}

/// The class of one document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    pub path: String,
    /// The index of the root, in the configuration of `Basis`.
    pub root: usize,
    /// The 0.5 type. `None`: the front matter has a name that is not a 0.5
    /// type, so the document stays as it is.
    pub old_type: Option<String>,
    /// For a change: the 0.5 status, after `status-map` and the default of
    /// the root. `None`: the document has no status.
    pub old_status: Option<String>,
    pub fate: Fate,
    pub body_chars: u32,
    /// A stub that `mv --stub` left: its front matter has `moved-to`. It is
    /// not a document of its 0.5 type, so nothing changes in it, and nothing
    /// moves it.
    pub stub: bool,
}

/// The documents of a tree, with their classes.
#[derive(Debug, Default)]
pub struct Classified {
    pub docs: Vec<Class>,
    /// The text of each document.
    pub texts: BTreeMap<String, String>,
    /// For each root: true if the root was mixed in the source. `--layout`
    /// moves only these roots (D145): a root of one 0.5 type keeps its paths,
    /// also if the new file makes it a mixed root (D140).
    pub mixed: Vec<bool>,
    /// The documents whose bytes are not UTF-8. The text above is lossy for
    /// them, so an edit of such a document stops the plan.
    pub lossy: std::collections::BTreeSet<String>,
}

impl Classified {
    /// The facts that pass 2 of the configuration needs. `unmarked_roles` is
    /// for the harness (S9), so it stays empty. `cfg` is the configuration of
    /// the `Basis`: it gives the paths of the roots and the budget of an
    /// `intent`.
    pub fn facts(&self, cfg: &Config) -> Facts {
        let hard = cfg.budgets[DocType::Intent as usize].1;
        let mut f = Facts::default();
        for c in &self.docs {
            if let Some(t) = &c.old_type {
                f.old_types.insert(t.clone());
            }
            match c.fate {
                Fate::Move => {
                    f.change_roots.insert(cfg.roots[c.root].path.clone());
                    if hard != 0 && c.body_chars > hard {
                        f.change_over_budget = true;
                    }
                }
                Fate::Done => f.done_changes = true,
                Fate::Keep => {}
            }
        }
        f
    }
}

/// A document whose front matter changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocChange {
    /// The path in the tree, before any move.
    pub path: String,
    pub old_type: String,
    pub new_type: DocType,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub was_type: Option<&'static str>,
    /// The text with the edits of the front matter only: no alias and no link
    /// edit. The proof reads it as the text "before" (D149).
    pub edited: String,
}

/// The 0.5 type of a document in a 0.5 file (D141): the type of a root of
/// one type, else the front matter, else the `default-type` of the root, else
/// `note`. A name that is not a 0.5 type leaves the document as it is.
fn old_type_of(root: &OldRoot, fm_type: Option<&str>) -> Option<String> {
    match root.kind() {
        Kind::Single(t) => Some(t.to_string()),
        Kind::Mixed => match fm_type {
            Some(w) => type_of(w).map(|_| w.to_string()),
            None => Some(root.default_old_type().to_string()),
        },
    }
}

/// The status of a 0.5 change: the word of the front matter, after
/// `status-map`, else the status of the root where the type is the default
/// type of the root (D142).
fn old_status_of(
    status: Option<&str>,
    map: &[(String, String)],
    root_status: Option<&str>,
) -> Option<String> {
    match status {
        Some(s) if change_status(s).is_some() => Some(s.to_string()),
        Some(s) => Some(
            map.iter()
                .find(|(from, _)| from == s)
                .map_or_else(|| s.to_string(), |(_, to)| to.clone()),
        ),
        None => root_status.map(str::to_string),
    }
}

/// The class of each document of a tree (D141, D142). A document is a
/// Markdown file in a root that no `exclude` removes, as the corpus reads it.
pub fn classify(tree: &Tree, basis: &Basis) -> Result<Classified, Fatal> {
    let cfg = basis.cfg;
    if let Some(old) = basis.old
        && old.roots.len() != cfg.roots.len()
    {
        return Err(Fatal::Config(
            "upgrade: the 0.5 file and the new file have not the same roots".into(),
        ));
    }
    let paths: Vec<String> = tree
        .files
        .iter()
        .filter(|p| is_doc_path(p))
        .filter(|p| cfg.root_of(p).is_some_and(|r| !cfg.is_excluded(r, p)))
        .cloned()
        .collect();
    let mut bytes = (tree.read)(&paths)?;
    let mut out = Classified::default();
    for path in paths {
        let blob = bytes
            .remove(&path)
            .ok_or_else(|| Fatal::Io(format!("{path}: not in the tree")))?;
        if std::str::from_utf8(&blob).is_err() {
            out.lossy.insert(path.clone());
        }
        let text = String::from_utf8_lossy(&blob).into_owned();
        let parsed = parse(&text);
        let fm = parsed.front_matter.as_ref();
        let field = |key: &str| match fm.and_then(|f| f.get(key)) {
            Some((Value::Str(s), _)) if !s.is_empty() => Some(s.clone()),
            _ => None,
        };
        let (fm_type, fm_status) = (field("type"), field("status"));
        let stub = field("moved-to").is_some();
        let root = cfg.root_of(&path).expect("a document is in a root");
        let (old_type, old_status) = match basis.old {
            _ if stub => (None, None),
            Some(old) => {
                let or = &old.roots[root];
                let ty = old_type_of(or, fm_type.as_deref());
                let status = (ty.as_deref() == Some("change"))
                    .then(|| {
                        let default = ty.as_deref() == Some(or.default_old_type());
                        old_status_of(
                            fm_status.as_deref(),
                            &or.status_map,
                            or.status.as_deref().filter(|_| default),
                        )
                    })
                    .flatten();
                (ty, status)
            }
            None => {
                let r = &cfg.roots[root];
                let ty = fm_type.as_deref().and_then(|w| {
                    let spec_of_0_5 =
                        w == "spec" && !cfg.layers.is_intent_place(DocType::Spec, r.rel(&path));
                    (names::renamed(w).is_some() || spec_of_0_5).then(|| w.to_string())
                });
                let status = (ty.as_deref() == Some("change"))
                    .then(|| old_status_of(fm_status.as_deref(), &r.status_map, None))
                    .flatten();
                (ty, status)
            }
        };
        let fate = match (old_type.as_deref(), old_status.as_deref()) {
            (Some("change"), Some(w)) if change_status(w) == Some(None) => Fate::Done,
            (Some("change"), _) => Fate::Move,
            _ => Fate::Keep,
        };
        out.docs.push(Class {
            path: path.clone(),
            root,
            old_type,
            old_status,
            fate,
            body_chars: parsed.body_chars,
            stub,
        });
        out.texts.insert(path, text);
    }
    out.mixed = (0..cfg.roots.len())
        .map(|i| match basis.old {
            Some(old) => old.roots[i].kind() == Kind::Mixed,
            None => cfg.roots[i].single_type.is_none(),
        })
        .collect();
    Ok(out)
}

fn stop(path: &str, why: &str) -> Fatal {
    Fatal::Config(format!(
        "{path}: {why}; upgrade does not guess. Write the change by hand, commit the change, then run `spec-dna upgrade` again"
    ))
}

/// The edits of the front matter of each document (D143). `fin` is the final
/// configuration. `dest` maps the path of a document that moves to its new
/// path, so the place rule reads the place where the document ends. A
/// document whose front matter does not parse, or has a form that the edits
/// cannot change, stops the plan with exit code 3, if it needs an edit.
pub fn edit_docs(
    classified: &Classified,
    fin: &Config,
    dest: &HashMap<String, String>,
) -> Result<Vec<DocChange>, Fatal> {
    let mut out = Vec::new();
    for c in &classified.docs {
        let Some(old) = &c.old_type else {
            continue;
        };
        let text = &classified.texts[&c.path];
        let fm = frontmatter::split(text).map(|(f, _)| frontmatter::parse(f, 2));
        let (fm, broken) = match fm {
            Some(Ok(f)) => (Some(f), None),
            Some(Err((line, msg))) => (None, Some(format!("line {line}: {msg}"))),
            None => (None, None),
        };
        let has = |key: &str| fm.as_ref().is_some_and(|f| f.get(key).is_some());
        // A key with a value that is not a string is a form that the edits
        // cannot change: the plan stops only if the document needs an edit.
        let broken = broken.or_else(|| {
            ["type", "status", "was-type"].into_iter().find_map(|key| {
                match fm.as_ref().and_then(|f| f.get(key)) {
                    Some((Value::Str(_), _)) | None => None,
                    Some(_) => Some(format!("`{key}` is not a single string")),
                }
            })
        });
        let value = |key: &str| -> Option<String> {
            match fm.as_ref().and_then(|f| f.get(key)) {
                Some((Value::Str(s), _)) => Some(s.clone()),
                _ => None,
            }
        };
        let new_type = match c.fate {
            Fate::Done => DocType::Research,
            _ => type_of(old).expect("a 0.5 type"),
        };
        let was_type = match (old.as_str(), c.fate) {
            ("note", _) => Some("note"),
            ("change", Fate::Done) => Some("change"),
            _ => None,
        };
        let cannot = |key: &str| {
            stop(
                &c.path,
                &format!("the line `{key}` has a form that upgrade cannot edit"),
            )
        };
        let mut t = text.clone();
        // The type: rewrite the line, or write it where the place gives another.
        match value("type") {
            Some(w) => {
                let to = if w == *old {
                    Some(new_type)
                } else {
                    type_of(&w)
                };
                if let Some(to) = to
                    && to.name() != w
                {
                    t = frontmatter::set_field(&t, "type", to.name())
                        .ok_or_else(|| cannot("type"))?;
                }
            }
            None => {
                let root = &fin.roots[c.root];
                if root.single_type.is_none() && !has("type") {
                    let path = dest.get(&c.path).unwrap_or(&c.path);
                    let want = fin.layers.place_type(root.rel(path)).or(root.default_type);
                    if want != Some(new_type) {
                        t = frontmatter::insert_field(&t, "type", new_type.name(), None);
                    }
                }
            }
        }
        // The status.
        let mut new_status = None;
        match c.fate {
            Fate::Done => {
                if has("status") {
                    t = frontmatter::remove_field(&t, "status").ok_or_else(|| cannot("status"))?;
                }
            }
            Fate::Move => {
                let word = c.old_status.as_deref();
                let to = word
                    .and_then(change_status)
                    .flatten()
                    .map(str::to_string)
                    .or_else(|| word.map(str::to_string));
                if let Some(to) = &to {
                    match value("status") {
                        Some(raw) if raw != *to => {
                            t = frontmatter::set_field(&t, "status", to)
                                .ok_or_else(|| cannot("status"))?;
                        }
                        Some(_) => {}
                        None => t = frontmatter::insert_field(&t, "status", to, Some("type")),
                    }
                }
                new_status = to;
            }
            Fate::Keep => {}
        }
        if let Some(v) = was_type
            && !has("was-type")
        {
            t = frontmatter::insert_field(&t, "was-type", v, Some("type"));
        }
        if t == *text {
            continue;
        }
        if let Some(why) = broken {
            return Err(stop(
                &c.path,
                &format!(
                    "the front matter has a form that upgrade cannot edit ({why}), and the document needs an edit"
                ),
            ));
        }
        if classified.lossy.contains(&c.path) {
            return Err(stop(
                &c.path,
                "the file is not UTF-8, and the document needs an edit",
            ));
        }
        if text.starts_with('\u{feff}') {
            return Err(stop(
                &c.path,
                "the file starts with a byte order mark, so its front matter is not read, and the document needs an edit",
            ));
        }
        out.push(DocChange {
            path: c.path.clone(),
            old_type: old.clone(),
            new_type,
            old_status: c.old_status.clone(),
            new_status,
            was_type,
            edited: t,
        });
    }
    Ok(out)
}

/// A move of a file (D144, D145). `old_id` and `new_id` are the ids of a
/// document, and `aliases` are the aliases that the moved file gets. A file
/// that is not a document has none of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: String,
    pub to: String,
    pub old_id: String,
    pub new_id: String,
    pub aliases: Vec<String>,
    /// True for a move of `--layout`, false for the move of a change.
    pub layout: bool,
}

/// The plan of the documents: the front matter, the moves and the new text of
/// each document that changes.
#[derive(Debug, Default)]
pub struct DocsPlan {
    pub docs: Vec<DocChange>,
    /// In path order.
    pub moves: Vec<Move>,
    /// The final path of each document that changes, with its new text.
    pub writes: BTreeMap<String, String>,
}

/// The folders of the 0.5 namespaces that `--layout` moves, and the 0.6
/// folders that take their files (D145).
const LAYOUT: [(&str, &str); 6] = [
    ("contracts", "interfaces"),
    ("invariants", "rules"),
    ("guides", "procedures"),
    ("runbooks", "procedures"),
    ("reference", "research"),
    ("notes", "research"),
];

const AGAIN: &str = "Then commit the change and run `spec-dna upgrade` again.";

/// `a/b` and `c` give `a/b/c`; an empty `a` gives `c`.
fn under(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

fn slug_of(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

/// The id of a document under a root, as the corpus reads it.
fn id_of(fin: &Config, root: usize, path: &str) -> String {
    nfc(&root_doc_id(&fin.roots[root], path)).into_owned()
}

/// The moves of the files (D144, D145), in path order. A stop is
/// `Fatal::Refused`, exit code 1, and it names each path. The function reads
/// only the list of the paths of the tree, so a target that exists is a path
/// of the list.
pub fn plan_moves(
    tree: &Tree,
    classified: &Classified,
    fin: &Config,
    layout: bool,
) -> Result<Vec<Move>, Fatal> {
    let class: HashMap<&str, &Class> = classified
        .docs
        .iter()
        .map(|c| (c.path.as_str(), c))
        .collect();
    let mut problems: Vec<String> = Vec::new();
    // (from, to, layout)
    let mut want: Vec<(String, String, bool)> = Vec::new();

    // The changes: each one goes to `<root>/<intent folder>/<slug>/intent.md`
    // in its own root, so its id keeps the root and the namespace (D144).
    let changes: Vec<&Class> = classified
        .docs
        .iter()
        .filter(|c| c.fate == Fate::Move)
        .collect();
    if !changes.is_empty() {
        let dir = fin
            .layers
            .layer_of_type(DocType::Intent)
            .and_then(|i| fin.layers.layers()[i].dir())
            .ok_or_else(|| {
                Fatal::Config(
                    "upgrade: no layer of the new spec-dna.toml has the type `intent`, so the changes cannot move"
                        .into(),
                )
            })?;
        for c in changes {
            let id = id_of(fin, c.root, &c.path);
            let slug = slug_of(&id);
            if !finish::valid_slug(slug) {
                problems.push(format!(
                    "the change `{}` has the slug `{slug}`, which cannot be the name of a folder. Rename the file. {AGAIN}",
                    c.path
                ));
                continue;
            }
            let to = under(
                &fin.roots[c.root].path,
                &format!("{dir}/{slug}/{}.md", DocType::Intent.name()),
            );
            if to != c.path {
                want.push((c.path.clone(), to, false));
            }
        }
    }

    // `--layout`: the folders of the 0.5 namespaces in a mixed root. A change
    // moves by its own rule, a `done` change and a 0.5 spec stay.
    if layout {
        for (ri, root) in fin.roots.iter().enumerate() {
            // A root of one 0.5 type keeps its paths, also if it is mixed now.
            let was_mixed = classified.mixed.get(ri).copied().unwrap_or(false);
            if root.single_type.is_some() || !was_mixed {
                continue;
            }
            for f in tree.files {
                if fin.root_of(f) != Some(ri) {
                    continue;
                }
                let Some((top, rest)) = root.rel(f).split_once('/') else {
                    continue;
                };
                let Some((_, new_top)) = LAYOUT.iter().find(|(old, _)| *old == top) else {
                    continue;
                };
                if let Some(c) = class.get(f.as_str())
                    && (c.fate != Fate::Keep || c.stub || c.old_type.as_deref() == Some("spec"))
                {
                    continue;
                }
                let to = under(&root.path, &format!("{new_top}/{rest}"));
                if is_doc_path(f) && (fin.is_excluded(ri, f) || fin.is_excluded(ri, &to)) {
                    problems.push(format!(
                        "an `exclude` glob matches `{f}` or its new path `{to}`, and the move would add or remove a document. Change the glob. {AGAIN}"
                    ));
                    continue;
                }
                want.push((f.clone(), to, true));
            }
        }
    }

    // The stops: one target for 2 files, a target that exists, a target in
    // another root.
    let have: std::collections::HashSet<&str> = tree.files.iter().map(String::as_str).collect();
    let mut by_target: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to, _) in &want {
        by_target.entry(to).or_default().push(from);
    }
    for (to, sources) in &by_target {
        if sources.len() > 1 {
            let list: Vec<String> = sources.iter().map(|f| format!("`{f}`")).collect();
            problems.push(format!(
                "{} files move to `{to}`: {}. Rename one of them. {AGAIN}",
                sources.len(),
                list.join(", ")
            ));
        }
    }
    for (from, to, _) in &want {
        if have.contains(to.as_str()) {
            problems.push(format!(
                "the target `{to}` of `{from}` exists. Move or rename the file that is there. {AGAIN}"
            ));
        }
        let root = class
            .get(from.as_str())
            .map(|c| c.root)
            .or_else(|| fin.root_of(from));
        if root != fin.root_of(to) {
            problems.push(format!(
                "the target `{to}` of `{from}` is in another root. Change the roots of spec-dna.toml, or move the file by hand. {AGAIN}"
            ));
        }
    }
    if !problems.is_empty() {
        problems.sort();
        problems.dedup();
        return Err(Fatal::Refused(format!(
            "upgrade stops, and nothing changed:\n- {}",
            problems.join("\n- ")
        )));
    }

    want.sort();
    Ok(want
        .into_iter()
        .map(|(from, to, layout)| {
            let (old_id, new_id, aliases) = match class.get(from.as_str()) {
                Some(c) => {
                    let root = &fin.roots[c.root];
                    let (old_id, new_id) = (id_of(fin, c.root, &from), id_of(fin, c.root, &to));
                    // The old id, and the legacy aliases that the new file name
                    // does not give again.
                    let regained: Vec<String> = root
                        .legacy_ids
                        .iter()
                        .filter_map(|l| l.alias_for(slug_of(&new_id)))
                        .collect();
                    let mut aliases = vec![old_id.clone()];
                    for a in root
                        .legacy_ids
                        .iter()
                        .filter_map(|l| l.alias_for(slug_of(&old_id)))
                    {
                        if !regained.contains(&a) && !aliases.contains(&a) {
                            aliases.push(a);
                        }
                    }
                    (old_id, new_id, aliases)
                }
                None => (String::new(), String::new(), Vec::new()),
            };
            Move {
                from,
                to,
                old_id,
                new_id,
                aliases,
                layout,
            }
        })
        .collect())
}

/// The ancestors of a path, from the shortest: `a/b/c.md` gives `a` and `a/b`.
fn ancestors(path: &str) -> Vec<&str> {
    path.match_indices('/').map(|(i, _)| &path[..i]).collect()
}

/// The map of the links: each moved file, and each folder that a link can name
/// and that moves whole. A folder moves whole if each tracked file in it moves
/// to the same place under the new folder (D146).
pub(crate) fn link_moves(tree: &Tree, moves: &[Move]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = moves
        .iter()
        .map(|m| (m.from.clone(), m.to.clone()))
        .collect();
    // A folder, its new place, and the files of it that move.
    let mut dirs: BTreeMap<&str, (Option<&str>, usize)> = BTreeMap::new();
    for m in moves {
        let from_dirs = ancestors(&m.from);
        let to_dirs = ancestors(&m.to);
        if m.layout {
            for (d, td) in from_dirs.iter().zip(&to_dirs) {
                let e = dirs.entry(d).or_insert((Some(td), 0));
                if e.0 != Some(td) {
                    e.0 = None;
                }
                e.1 += 1;
            }
        } else if let (Some(d), Some(td)) = (from_dirs.last(), to_dirs.last()) {
            // The folder of an `index.md` or `README.md` change is its name.
            let stem = m.from.rsplit('/').next().unwrap_or("");
            let index =
                stem.eq_ignore_ascii_case("index.md") || stem.eq_ignore_ascii_case("readme.md");
            if index {
                let e = dirs.entry(d).or_insert((Some(td), 0));
                if e.0 != Some(td) {
                    e.0 = None;
                }
                e.1 += 1;
            }
        }
    }
    let mut total: HashMap<&str, usize> = HashMap::new();
    for f in tree.files {
        for d in ancestors(f) {
            if dirs.contains_key(d) {
                *total.entry(d).or_default() += 1;
            }
        }
    }
    for (d, (td, n)) in dirs {
        if let Some(td) = td
            && total.get(d) == Some(&n)
        {
            map.insert(d.to_string(), td.to_string());
        }
    }
    map
}

/// A document that gets an alias needs a front matter that `add_aliases`
/// reads: with a front matter that does not parse, or an `aliases` value that is
/// not a list of strings, it would lose the old text. A byte order mark hides
/// the front matter.
fn alias_stop(path: &str, text: &str) -> Option<Fatal> {
    if text.starts_with('\u{feff}') {
        return Some(stop(
            path,
            "the file starts with a byte order mark, so its front matter is not read, and the move writes aliases",
        ));
    }
    let (fm, _) = frontmatter::split(text)?;
    match frontmatter::parse(fm, 2) {
        Err((line, msg)) => Some(stop(
            path,
            &format!(
                "the front matter does not parse (line {line}: {msg}), and the move writes aliases"
            ),
        )),
        Ok(f) if matches!(f.get("aliases"), Some((Value::Unsupported(_), _))) => Some(stop(
            path,
            "`aliases` is not a list of strings, and the move writes aliases",
        )),
        Ok(_) => None,
    }
}

/// The whole plan of the documents: the moves, the edits of the front matter
/// at the place where each document ends, the aliases, and the relative links
/// that the moves change. A stop is `Fatal::Refused`; a form of front matter
/// that the edits cannot change is `Fatal::Config`.
pub fn plan_docs(
    tree: &Tree,
    classified: &Classified,
    fin: &Config,
    layout: bool,
) -> Result<DocsPlan, Fatal> {
    let moves = plan_moves(tree, classified, fin, layout)?;
    let dest: HashMap<String, String> = moves
        .iter()
        .map(|m| (m.from.clone(), m.to.clone()))
        .collect();
    let docs = edit_docs(classified, fin, &dest)?;
    // The texts of step 3: the documents with the edits of the front matter.
    let edited: HashMap<&str, &str> = docs
        .iter()
        .map(|d| (d.path.as_str(), d.edited.as_str()))
        .collect();
    let texts: Vec<(&String, &str)> = classified
        .texts
        .iter()
        .map(|(p, t)| (p, edited.get(p.as_str()).copied().unwrap_or(t.as_str())))
        .collect();
    // The corpus of the tree before the moves: it says which links resolve.
    let corpus = Corpus::build(
        fin,
        tree.files.to_vec(),
        texts
            .iter()
            .map(|(p, t)| ((*p).clone(), Arc::new(parse(t))))
            .collect(),
    );
    let map = link_moves(tree, &moves);
    let by_from: HashMap<&str, &Move> = moves.iter().map(|m| (m.from.as_str(), m)).collect();
    let mut writes = BTreeMap::new();
    for (path, text) in texts {
        let moved = by_from.get(path.as_str());
        let end = moved.map_or(path.as_str(), |m| m.to.as_str());
        let mut t = rewrite_links(text, mv::parent(path), mv::parent(end), &map, &corpus);
        if let Some(m) = moved {
            if let Some(e) = alias_stop(path, &t) {
                return Err(e);
            }
            t = add_aliases(&t, &m.aliases);
        }
        if t != classified.texts[path] {
            // The text is lossy for a file that is not UTF-8.
            if classified.lossy.contains(path.as_str()) {
                return Err(stop(
                    path,
                    "the file is not UTF-8, and the plan writes it (it moves, or a link in it changes)",
                ));
            }
            writes.insert(end.to_string(), t);
        }
    }
    Ok(DocsPlan {
        docs,
        moves,
        writes,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::upgrade_config::{self, ConfigOpts, ConfigPlan};
    use std::path::{Path, PathBuf};

    pub(crate) fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
    }

    /// Each file of a fixture folder, as a tree in memory.
    pub(crate) fn fixture_tree(case: &str) -> TreeData {
        fn walk(dir: &Path, rel: &str, out: &mut HashMap<String, Vec<u8>>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let e = e.unwrap();
                let name = e.file_name().to_string_lossy().to_string();
                let path = if rel.is_empty() {
                    name.clone()
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
        let mut blobs = HashMap::new();
        walk(&fixtures().join(case), "", &mut blobs);
        TreeData::from_map(blobs)
    }

    pub(crate) fn mem(files: &[(&str, &str)]) -> TreeData {
        TreeData::from_map(
            files
                .iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        )
    }

    pub(crate) fn opts() -> ConfigOpts {
        ConfigOpts {
            pin: "0.6.0".into(),
            sums: None,
        }
    }

    /// What the pipeline of `plan` gives for a tree and a 0.5 file, up to the
    /// edits of the front matter.
    pub(crate) struct Run {
        pub(crate) classified: Classified,
        pub(crate) facts: Facts,
        pub(crate) plan: ConfigPlan,
        pub(crate) fin: Config,
        pub(crate) changes: Vec<DocChange>,
    }

    /// The pipeline up to the final configuration. `changes` is empty.
    pub(crate) fn pipeline(tree: &TreeData, toml: &str) -> Run {
        let old = OldConfig::read(toml).unwrap_or_else(|e| panic!("{e}\n{toml}"));
        let p1 = upgrade_config::pass1(toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let classified = tree
            .with(|t| {
                classify(
                    t,
                    &Basis {
                        cfg: &cfg1,
                        old: Some(&old),
                    },
                )
            })
            .unwrap();
        let facts = classified.facts(&cfg1);
        let plan = upgrade_config::plan(toml, &old, &facts, &opts()).unwrap();
        let fin = Config::parse(&plan.text).unwrap();
        Run {
            classified,
            facts,
            plan,
            fin,
            changes: Vec::new(),
        }
    }

    pub(crate) fn run_with(tree: &TreeData, toml: &str, dest: &HashMap<String, String>) -> Run {
        let mut r = pipeline(tree, toml);
        r.changes = edit_docs(&r.classified, &r.fin, dest).unwrap();
        r
    }

    pub(crate) fn run(tree: &TreeData, toml: &str) -> Run {
        run_with(tree, toml, &HashMap::new())
    }

    pub(crate) fn edited<'a>(r: &'a Run, path: &str) -> &'a str {
        r.changes
            .iter()
            .find(|c| c.path == path)
            .map(|c| c.edited.as_str())
            .unwrap_or_else(|| panic!("no change for {path}"))
    }

    pub(crate) fn message(e: Fatal) -> String {
        match e {
            Fatal::Config(m) | Fatal::Refused(m) => m,
            other => panic!("unexpected error: {other:?}"),
        }
    }

    /// A mixed root with no default type: each document says its type.
    pub(crate) const MIXED: &str = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";

    // --- one case for each row of the mapping ---

    #[test]
    fn each_type_of_the_mapping_gets_its_new_name() {
        let doc = |ty: &str| format!("---\ntype: {ty}\nstatus: active\n---\n# T\n");
        let files = [
            ("docs/a-spec.md", doc("spec")),
            ("docs/a-contract.md", doc("contract")),
            ("docs/a-invariant.md", doc("invariant")),
            ("docs/a-guide.md", doc("guide")),
            ("docs/a-runbook.md", doc("runbook")),
            ("docs/a-reference.md", doc("reference")),
        ];
        let tree = mem(&files
            .iter()
            .map(|(p, t)| (*p, t.as_str()))
            .collect::<Vec<_>>());
        let r = run(&tree, MIXED);
        for (path, new) in [
            ("docs/a-spec.md", "requirements"),
            ("docs/a-contract.md", "interface"),
            ("docs/a-invariant.md", "rule"),
            ("docs/a-guide.md", "procedure"),
            ("docs/a-runbook.md", "procedure"),
            ("docs/a-reference.md", "research"),
        ] {
            assert_eq!(
                edited(&r, path),
                format!("---\ntype: {new}\nstatus: active\n---\n# T\n"),
                "{path}"
            );
        }
        // A reference has no `was-type`: only a note gets the marker.
        let c = r
            .changes
            .iter()
            .find(|c| c.path == "docs/a-reference.md")
            .unwrap();
        assert_eq!(
            (c.old_type.as_str(), c.new_type, c.was_type),
            ("reference", DocType::Research, None)
        );
    }

    #[test]
    fn a_decision_and_an_incident_keep_their_text() {
        let tree = mem(&[
            (
                "docs/d.md",
                "---\ntype: decision\nstatus: accepted\n---\n# D\n",
            ),
            ("docs/i.md", "---\ntype: incident\n---\n# I\n"),
        ]);
        let r = run(&tree, MIXED);
        assert!(r.changes.is_empty(), "{:?}", r.changes);
        // They are still in the facts.
        assert!(r.facts.old_types.contains("decision") && r.facts.old_types.contains("incident"));
    }

    #[test]
    fn a_proposed_accepted_or_rejected_change_becomes_an_intent_with_the_new_status() {
        let tree = mem(&[
            (
                "docs/c1.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n",
            ),
            (
                "docs/c2.md",
                "---\ntype: change\nstatus: accepted\n---\n# C\n",
            ),
            (
                "docs/c3.md",
                "---\ntype: change\nstatus: rejected\n---\n# C\n",
            ),
        ]);
        let r = run(&tree, MIXED);
        for (path, status) in [
            ("docs/c1.md", "draft"),
            ("docs/c2.md", "accepted"),
            ("docs/c3.md", "closed"),
        ] {
            assert_eq!(
                edited(&r, path),
                format!("---\ntype: intent\nstatus: {status}\n---\n# C\n")
            );
            let c = r.changes.iter().find(|c| c.path == path).unwrap();
            assert_eq!(c.new_type, DocType::Intent);
            assert_eq!(c.new_status.as_deref(), Some(status));
            assert_eq!(c.was_type, None);
        }
        assert_eq!(r.facts.change_roots.iter().collect::<Vec<_>>(), ["docs"]);
        assert!(!r.facts.done_changes);
    }

    #[test]
    fn a_done_change_becomes_research_with_no_status_and_the_marker() {
        let tree = mem(&[(
            "docs/done.md",
            "---\ntype: change\nstatus: done\nowner: me\n---\n# C\n",
        )]);
        let r = run(&tree, MIXED);
        assert_eq!(
            edited(&r, "docs/done.md"),
            "---\ntype: research\nwas-type: change\nowner: me\n---\n# C\n"
        );
        let c = &r.changes[0];
        assert_eq!(
            (
                c.new_type,
                c.old_status.as_deref(),
                c.new_status.clone(),
                c.was_type
            ),
            (DocType::Research, Some("done"), None, Some("change"))
        );
        assert!(r.facts.done_changes && r.facts.change_roots.is_empty());
    }

    #[test]
    fn a_note_gets_research_and_the_marker_in_each_form_of_front_matter() {
        let tree = mem(&[
            ("docs/n1.md", "---\ntype: note\n---\n# N\n"),
            ("docs/n2.md", "# N\n"),
            ("docs/n3.md", "---\nowner: me\n---\n# N\n"),
        ]);
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/n1.md"),
            "---\ntype: research\nwas-type: note\n---\n# N\n"
        );
        // The default type `note` is the 0.5 default: no `type` line.
        assert_eq!(edited(&r, "docs/n2.md"), "---\nwas-type: note\n---\n# N\n");
        assert_eq!(
            edited(&r, "docs/n3.md"),
            "---\nwas-type: note\nowner: me\n---\n# N\n"
        );
    }

    // --- the status of a change: status-map and the status of the root ---

    #[test]
    fn the_status_map_decides_the_status_of_a_change() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\nstatus-map = { planned = \"proposed\", shipped = \"done\" }\n";
        let tree = mem(&[
            (
                "docs/p.md",
                "---\ntype: change\nstatus: planned\n---\n# P\n",
            ),
            (
                "docs/s.md",
                "---\ntype: change\nstatus: shipped\n---\n# S\n",
            ),
            (
                "docs/w.md",
                "---\ntype: change\nstatus: waiting\n---\n# W\n",
            ),
        ]);
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/p.md"),
            "---\ntype: intent\nstatus: draft\n---\n# P\n"
        );
        assert_eq!(
            edited(&r, "docs/s.md"),
            "---\ntype: research\nwas-type: change\n---\n# S\n"
        );
        // Another word moves as an intent and keeps the word (D142).
        assert_eq!(
            edited(&r, "docs/w.md"),
            "---\ntype: intent\nstatus: waiting\n---\n# W\n"
        );
    }

    #[test]
    fn a_status_does_not_change_on_a_decision_or_a_spec() {
        let tree = mem(&[
            (
                "docs/d.md",
                "---\ntype: decision\nstatus: proposed\n---\n# D\n",
            ),
            ("docs/s.md", "---\ntype: spec\nstatus: draft\n---\n# S\n"),
        ]);
        let r = run(&tree, MIXED);
        assert_eq!(
            edited(&r, "docs/s.md"),
            "---\ntype: requirements\nstatus: draft\n---\n# S\n"
        );
        assert!(r.changes.iter().all(|c| c.path != "docs/d.md"));
    }

    #[test]
    fn a_change_that_took_the_status_of_its_root_gets_a_status_line() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs/rfcs\"\ntype = \"change\"\nstatus = \"proposed\"\n";
        let tree = mem(&[
            ("docs/rfcs/a.md", "# A\n"),
            ("docs/rfcs/b.md", "---\nstatus: accepted\n---\n# B\n"),
        ]);
        // The place of the moved change is the intent folder, so no `type` line.
        let dest: HashMap<String, String> = [
            ("docs/rfcs/a.md", "docs/rfcs/intent/a/intent.md"),
            ("docs/rfcs/b.md", "docs/rfcs/intent/b/intent.md"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        let r = run_with(&tree, toml, &dest);
        assert_eq!(
            edited(&r, "docs/rfcs/a.md"),
            "---\nstatus: draft\n---\n# A\n"
        );
        // The status line has the final word already, and the place gives the
        // type: nothing to write.
        assert!(r.changes.iter().all(|c| c.path != "docs/rfcs/b.md"));
        let a = r
            .changes
            .iter()
            .find(|c| c.path == "docs/rfcs/a.md")
            .unwrap();
        assert_eq!(a.old_status.as_deref(), Some("proposed"));
        assert_eq!(a.new_status.as_deref(), Some("draft"));
    }

    #[test]
    fn a_done_change_that_took_done_from_its_root_has_no_status_line_to_remove() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs/rfcs\"\ntype = \"change\"\nstatus = \"done\"\n";
        let tree = mem(&[("docs/rfcs/a.md", "# A\n")]);
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/rfcs/a.md"),
            "---\nwas-type: change\n---\n# A\n"
        );
    }

    // --- the place gives another type (D143) ---

    #[test]
    fn a_former_note_in_a_layer_folder_gets_its_type_line() {
        // The folder `rules` of a mixed root gives the type `rule`, because
        // the project uses invariants, so the layer `rules` exists.
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let tree = mem(&[
            (
                "docs/inv.md",
                "---\ntype: invariant\nstatus: active\n---\n# I\n",
            ),
            ("docs/rules/n.md", "# N\n"),
            ("docs/rules/m.md", "---\ntype: note\n---\n# M\n"),
            ("docs/plain.md", "# P\n"),
        ]);
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/rules/n.md"),
            "---\ntype: research\nwas-type: note\n---\n# N\n"
        );
        // A `type` line is rewritten, and the place does not matter.
        assert_eq!(
            edited(&r, "docs/rules/m.md"),
            "---\ntype: research\nwas-type: note\n---\n# M\n"
        );
        // Not in a layer folder: the default type gives research.
        assert_eq!(
            edited(&r, "docs/plain.md"),
            "---\nwas-type: note\n---\n# P\n"
        );
    }

    #[test]
    fn a_root_of_one_type_has_no_place_rule() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n[[roots]]\npath = \"docs/rules\"\ntype = \"note\"\n";
        let tree = mem(&[
            ("docs/rules/n.md", "# N\n"),
            (
                "docs/inv.md",
                "---\ntype: invariant\nstatus: active\n---\n# I\n",
            ),
        ]);
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/rules/n.md"),
            "---\nwas-type: note\n---\n# N\n"
        );
    }

    // --- the bytes that stay ---

    #[test]
    fn quotes_comments_and_line_endings_stay() {
        let tree = mem(&[
            (
                "docs/q.md",
                "---\ntype: \"contract\"  # old\nstatus: active\n---\n# Q\n",
            ),
            (
                "docs/c.md",
                "---\r\ntype: change\r\nstatus: rejected\r\n---\r\n# C\r\n",
            ),
            ("docs/n.md", "# N\r\ntext\r\n"),
        ]);
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let r = run(&tree, toml);
        assert_eq!(
            edited(&r, "docs/q.md"),
            "---\ntype: \"interface\"  # old\nstatus: active\n---\n# Q\n"
        );
        assert_eq!(
            edited(&r, "docs/c.md"),
            "---\r\ntype: intent\r\nstatus: closed\r\n---\r\n# C\r\n"
        );
        assert_eq!(
            edited(&r, "docs/n.md"),
            "---\r\nwas-type: note\r\n---\r\n# N\r\ntext\r\n"
        );
    }

    #[test]
    fn a_front_matter_that_does_not_parse_stops_the_plan_only_if_it_needs_an_edit() {
        let broken = "---\ntype: note\nstatus draft\n---\n# B\n";
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";
        // A spec by the default type: nothing to write.
        let tree = mem(&[("docs/b.md", broken)]);
        let r = run(&tree, toml);
        assert!(r.changes.is_empty());
        // A note by the default type needs the marker: stop, with the path.
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let old = OldConfig::read(toml).unwrap();
        let p1 = upgrade_config::pass1(toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let cl = tree
            .with(|t| {
                classify(
                    t,
                    &Basis {
                        cfg: &cfg1,
                        old: Some(&old),
                    },
                )
            })
            .unwrap();
        let facts = cl.facts(&cfg1);
        let plan = upgrade_config::plan(toml, &old, &facts, &opts()).unwrap();
        let fin = Config::parse(&plan.text).unwrap();
        let e = edit_docs(&cl, &fin, &HashMap::new()).unwrap_err();
        assert!(matches!(e, Fatal::Config(_)), "{e:?}");
        assert!(message(e).contains("docs/b.md"));
    }

    // --- the 0.6 rule (a pin from 0.6.0) ---

    #[test]
    fn with_the_0_6_rule_only_a_0_5_name_changes() {
        let cfg = Config::parse(&format!(
            "spec-dna = \"{}\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"research\"\n",
            crate::adapters::VERSION
        ))
        .unwrap();
        let tree = mem(&[
            ("docs/a.md", "---\ntype: spec\nstatus: draft\n---\n# A\n"),
            // A 0.6 `spec` at its place stays.
            (
                "docs/intent/x/spec.md",
                "---\ntype: spec\nstatus: draft\n---\n# S\n",
            ),
            ("docs/b.md", "---\ntype: contract\n---\n# B\n"),
            ("docs/c.md", "---\ntype: requirements\n---\n# C\n"),
            ("docs/d.md", "# D\n"),
        ]);
        let cl = tree
            .with(|t| {
                classify(
                    t,
                    &Basis {
                        cfg: &cfg,
                        old: None,
                    },
                )
            })
            .unwrap();
        let changes = edit_docs(&cl, &cfg, &HashMap::new()).unwrap();
        let paths: Vec<&str> = changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["docs/a.md", "docs/b.md"]);
        assert_eq!(
            changes[0].edited,
            "---\ntype: requirements\nstatus: draft\n---\n# A\n"
        );
        assert_eq!(changes[1].edited, "---\ntype: interface\n---\n# B\n");
    }

    // --- the fixtures: the records of the 0.5.0 binary ---

    /// The facts of the documents of a fixture, from the file that the 0.5.0
    /// binary recorded: the path, the id, the 0.5 type and the 0.5 status.
    pub(crate) fn recorded(case: &str) -> Vec<(String, String, String)> {
        let rec = std::fs::read_to_string(fixtures().join(format!("recorded/{case}.txt"))).unwrap();
        rec.lines()
            .filter_map(|l| l.strip_prefix("doc "))
            .map(|l| {
                let p: Vec<&str> = l.split(' ').collect();
                (p[0].to_string(), p[2].to_string(), p[3].to_string())
            })
            .collect()
    }

    pub(crate) const CASES: [&str; 5] =
        ["v05", "v04", "brownfield", "harness-lean", "harness-full"];

    /// The tree and the 0.5 file of a case. A harness case is `v05/` with the
    /// files that `init` of 0.5.0 wrote, and its `[harness]` text.
    pub(crate) fn fixture_case(case: &str) -> (TreeData, String) {
        let read = |p: &str| String::from_utf8(std::fs::read(fixtures().join(p)).unwrap()).unwrap();
        if let Some(_level) = case.strip_prefix("harness-") {
            let mut blobs = HashMap::new();
            for base in ["v05", case] {
                let dir = fixtures().join(base);
                let mut stack = vec![(dir.clone(), String::new())];
                while let Some((d, rel)) = stack.pop() {
                    for e in std::fs::read_dir(&d).unwrap() {
                        let e = e.unwrap();
                        let name = e.file_name().to_string_lossy().to_string();
                        let path = if rel.is_empty() {
                            name
                        } else {
                            format!("{rel}/{name}")
                        };
                        if e.file_type().unwrap().is_dir() {
                            stack.push((e.path(), path));
                        } else {
                            blobs.insert(path, std::fs::read(e.path()).unwrap());
                        }
                    }
                }
            }
            blobs.remove("spec-dna.toml.append");
            let toml = read("v05/spec-dna.toml") + &read(&format!("{case}/spec-dna.toml.append"));
            blobs.insert("spec-dna.toml".into(), toml.clone().into_bytes());
            return (TreeData::from_map(blobs), toml);
        }
        (fixture_tree(case), read(&format!("{case}/spec-dna.toml")))
    }

    pub(crate) fn fixture_run(case: &str) -> Run {
        let (tree, toml) = fixture_case(case);
        run(&tree, &toml)
    }

    #[test]
    fn the_classes_are_the_types_and_statuses_that_the_0_5_binary_recorded() {
        for case in CASES {
            let r = fixture_run(case);
            let got: Vec<(String, String, String)> = r
                .classified
                .docs
                .iter()
                .map(|c| {
                    (
                        c.path.clone(),
                        c.old_type.clone().unwrap(),
                        // The record has a status for each type. The class has one
                        // for a change only.
                        if c.old_type.as_deref() == Some("change") {
                            c.old_status.clone().unwrap_or_else(|| "-".into())
                        } else {
                            String::new()
                        },
                    )
                })
                .collect();
            let want: Vec<(String, String, String)> = recorded(case)
                .into_iter()
                .map(|(p, t, s)| (p, t.clone(), if t == "change" { s } else { String::new() }))
                .collect();
            assert_eq!(got, want, "{case}");
        }
    }

    #[test]
    fn the_facts_give_the_golden_configuration_of_each_fixture() {
        for case in CASES {
            let r = fixture_run(case);
            let after = std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("tests/golden/upgrade/config/{case}.after.toml")),
            )
            .unwrap();
            assert_eq!(r.plan.text, after, "{case}");
        }
    }

    #[test]
    fn the_fixture_v05_has_5_moving_changes_and_one_done_change() {
        let r = fixture_run("v05");
        let moving: Vec<&str> = r
            .classified
            .docs
            .iter()
            .filter(|c| c.fate == Fate::Move)
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(
            moving,
            [
                "docs/changes/add-drip-lines.md",
                "docs/changes/new-sensors/index.md",
                "docs/changes/planning/add-pump-timer.md",
                "docs/changes/rework-pumps.md",
                "docs/changes/solar-pump.md",
            ]
        );
        let done: Vec<&str> = r
            .classified
            .docs
            .iter()
            .filter(|c| c.fate == Fate::Done)
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(done, ["docs/changes/tune-pump.md"]);
        assert_eq!(
            edited(&r, "docs/changes/rework-pumps.md"),
            "---\ntype: intent\nstatus: accepted\ndepends-on: [irrigation/pumps#fill-window]\n---\n# Rework the pumps\n\nFollows [the pump rules](../irrigation/pumps.md) and [[ADR-2]]. The new pump starts in 2 seconds.\n"
        );
        assert_eq!(
            edited(&r, "docs/changes/solar-pump.md"),
            "---\ntype: intent\nstatus: closed\n---\n# Solar pump\n\nRejected: the greenhouse has too little sun in winter.\n"
        );
        assert_eq!(
            edited(&r, "docs/changes/tune-pump.md"),
            "---\ntype: research\nwas-type: change\n---\n# Tune the pump\n\nThe night run of the pump is shorter. No document cites this change.\n"
        );
        // A former note without a front matter gets one.
        assert_eq!(
            edited(&r, "docs/field-notes/frost.md"),
            "---\nwas-type: note\n---\n# Frost night\n\nThe heater ran for 6 hours. See [the valve rules](../irrigation/valves.md).\n"
        );
        // The decisions stay.
        assert!(r.changes.iter().all(|c| !c.path.contains("decisions/")));
    }

    #[test]
    fn a_moved_change_over_the_budget_is_a_fact() {
        let big = format!(
            "---\ntype: change\nstatus: proposed\n---\n# Big\n\n{}\n",
            "word ".repeat(2600)
        );
        let tree = mem(&[
            ("docs/big.md", big.as_str()),
            (
                "docs/small.md",
                "---\ntype: change\nstatus: proposed\n---\n# S\n",
            ),
        ]);
        let r = run(&tree, MIXED);
        assert!(r.facts.change_over_budget);
        let tree = mem(&[(
            "docs/small.md",
            "---\ntype: change\nstatus: proposed\n---\n# S\n",
        )]);
        assert!(!run(&tree, MIXED).facts.change_over_budget);
    }

    #[test]
    fn a_tree_reader_gets_only_paths_of_the_tree() {
        // The plan reads the documents through the tree, never from the disk:
        // a tree that holds a path that no disk has gives the same classes.
        let tree = mem(&[("no/such/disk/doc.md", "# T\n")]);
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"no/such/disk\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let r = run(&tree, toml);
        assert_eq!(r.classified.docs.len(), 1);
        assert_eq!(r.fin.roots[0].path, "no/such/disk");
    }

    #[test]
    fn a_form_that_the_edits_cannot_change_stops_the_plan_with_the_path() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let old = OldConfig::read(toml).unwrap();
        let p1 = upgrade_config::pass1(toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let go = |tree: &TreeData| {
            let cl = tree
                .with(|t| {
                    classify(
                        t,
                        &Basis {
                            cfg: &cfg1,
                            old: Some(&old),
                        },
                    )
                })
                .unwrap();
            let plan = upgrade_config::plan(toml, &old, &cl.facts(&cfg1), &opts()).unwrap();
            let fin = Config::parse(&plan.text).unwrap();
            edit_docs(&cl, &fin, &HashMap::new())
        };
        // A `type` that is a list, in a note: the marker needs a `type` line.
        let tree = mem(&[("docs/l.md", "---\ntype: [spec, note]\n---\n# L\n")]);
        let m = message(go(&tree).unwrap_err());
        assert!(m.starts_with("docs/l.md: "), "{m}");
        assert!(m.contains("`type` is not a single string"), "{m}");
        // A file that is not UTF-8 and needs an edit.
        let mut blobs: HashMap<String, Vec<u8>> = HashMap::new();
        blobs.insert("docs/b.md".into(), b"# Title \xff\n".to_vec());
        let m = message(go(&TreeData::from_map(blobs)).unwrap_err());
        assert!(
            m.starts_with("docs/b.md: ") && m.contains("not UTF-8"),
            "{m}"
        );
    }

    #[test]
    fn the_second_run_on_the_result_finds_nothing_to_edit() {
        for case in CASES {
            let (tree, toml) = fixture_case(case);
            let r = run(&tree, &toml);
            // The tree after the edits, read with the 0.6 rule and the final file.
            let mut blobs: HashMap<String, Vec<u8>> = tree.with(|t| (t.read)(t.files).unwrap());
            for c in &r.changes {
                blobs.insert(c.path.clone(), c.edited.clone().into_bytes());
            }
            let again = TreeData::from_map(blobs);
            let cl = again
                .with(|t| {
                    classify(
                        t,
                        &Basis {
                            cfg: &r.fin,
                            old: None,
                        },
                    )
                })
                .unwrap();
            let changes = edit_docs(&cl, &r.fin, &HashMap::new()).unwrap();
            assert!(
                changes.is_empty(),
                "{case}: {:?}",
                changes.iter().map(|c| &c.path).collect::<Vec<_>>()
            );
            assert!(cl.docs.iter().all(|c| c.fate == Fate::Keep), "{case}");
        }
    }

    // --- S7: the moves ---

    pub(crate) fn full(
        tree: &TreeData,
        toml: &str,
        layout: bool,
    ) -> Result<(Run, DocsPlan), Fatal> {
        let mut r = pipeline(tree, toml);
        let p = tree.with(|t| plan_docs(t, &r.classified, &r.fin, layout))?;
        r.changes = p.docs.clone();
        Ok((r, p))
    }

    pub(crate) fn go(tree: &TreeData, toml: &str, layout: bool) -> (Run, DocsPlan) {
        full(tree, toml, layout).unwrap_or_else(|e| panic!("{e:?}"))
    }

    pub(crate) fn refused(tree: &TreeData, toml: &str, layout: bool) -> String {
        match full(tree, toml, layout) {
            Err(Fatal::Refused(m)) => m,
            Err(other) => panic!("not a stop with exit code 1: {other:?}"),
            Ok(_) => panic!("the plan must stop"),
        }
    }

    pub(crate) fn mv<'a>(p: &'a DocsPlan, from: &str) -> &'a Move {
        p.moves
            .iter()
            .find(|m| m.from == from)
            .unwrap_or_else(|| panic!("no move of {from}: {:?}", p.moves))
    }

    #[test]
    fn two_changes_with_one_slug_stop_the_plan() {
        let c = "---\ntype: change\nstatus: proposed\n---\n# C\n";
        let tree = mem(&[("docs/changes/x.md", c), ("docs/changes/planning/x.md", c)]);
        let m = refused(&tree, MIXED, false);
        for part in [
            "docs/changes/x.md",
            "docs/changes/planning/x.md",
            "docs/intent/x/intent.md",
        ] {
            assert!(m.contains(part), "{part} is not in: {m}");
        }
    }

    #[test]
    fn a_target_that_exists_stops_the_plan() {
        let tree = mem(&[
            (
                "docs/x.md",
                "---\ntype: change\nstatus: proposed\n---\n# C\n",
            ),
            ("docs/intent/x/intent.md", "# Other\n"),
        ]);
        let m = refused(&tree, MIXED, false);
        assert!(
            m.contains("docs/x.md") && m.contains("docs/intent/x/intent.md"),
            "{m}"
        );
    }

    #[test]
    fn a_slug_that_finish_refuses_stops_the_plan() {
        let tree = mem(&[(
            "docs/.hidden.md",
            "---\ntype: change\nstatus: proposed\n---\n# C\n",
        )]);
        let m = refused(&tree, MIXED, false);
        assert!(m.contains("docs/.hidden.md") && m.contains("slug"), "{m}");
    }

    #[test]
    fn a_change_that_is_not_done_moves_into_its_intent_folder() {
        let (_, p) = go(&fixture_tree("v05"), &fixture_case("v05").1, false);
        let want = [
            (
                "docs/changes/add-drip-lines.md",
                "docs/intent/add-drip-lines/intent.md",
            ),
            (
                "docs/changes/new-sensors/index.md",
                "docs/intent/new-sensors/intent.md",
            ),
            (
                "docs/changes/planning/add-pump-timer.md",
                "docs/intent/add-pump-timer/intent.md",
            ),
            (
                "docs/changes/rework-pumps.md",
                "docs/intent/rework-pumps/intent.md",
            ),
            (
                "docs/changes/solar-pump.md",
                "docs/intent/solar-pump/intent.md",
            ),
        ];
        assert_eq!(
            p.moves
                .iter()
                .map(|m| (m.from.as_str(), m.to.as_str()))
                .collect::<Vec<_>>(),
            want
        );
        for m in &p.moves {
            // The old id is the id that the 0.5.0 binary recorded.
            let rec = std::fs::read_to_string(fixtures().join("recorded/v05.txt")).unwrap();
            let line = rec
                .lines()
                .find(|l| l.starts_with(&format!("doc {} ", m.from)))
                .unwrap();
            assert_eq!(line.split(' ').nth(2).unwrap(), m.old_id, "{}", m.from);
            assert_eq!(m.aliases, std::slice::from_ref(&m.old_id));
            assert_eq!(
                m.new_id,
                format!("intent/{}/intent", m.to.split('/').nth(2).unwrap())
            );
            assert!(!m.layout);
        }
        // The done change stays, and the decisions do not move.
        assert!(p.moves.iter().all(|m| !m.from.contains("tune-pump")));
    }

    #[test]
    fn a_moved_change_gets_its_aliases_and_its_links_follow() {
        let (_, p) = go(&fixture_tree("v05"), &fixture_case("v05").1, false);
        assert_eq!(
            p.writes["docs/intent/rework-pumps/intent.md"],
            "---\ntype: intent\nstatus: accepted\ndepends-on: [irrigation/pumps#fill-window]\naliases:\n  - changes/rework-pumps\n---\n# Rework the pumps\n\nFollows [the pump rules](../../irrigation/pumps.md) and [[ADR-2]]. The new pump starts in 2 seconds.\n"
        );
        // Same depth: the links of the index change keep their text.
        assert!(
            p.writes["docs/intent/new-sensors/intent.md"]
                .contains("[the sensor rules](../../irrigation/sensors.md)")
        );
        // A document that links to a moved change follows it.
        assert!(
            p.writes["docs/irrigation/pumps.md"]
                .contains("[the rework](../intent/rework-pumps/intent.md)")
        );
        // A wiki link keeps its text: the alias resolves it.
        assert!(!p.writes.contains_key("docs/decisions/ADR-3-solar-pump.md"));
        // No file of the old place is written.
        assert_eq!(
            p.writes
                .keys()
                .filter(|k| k.starts_with("docs/changes/"))
                .collect::<Vec<_>>(),
            ["docs/changes/tune-pump.md"]
        );
        // The done change stays in place with its edits.
        assert!(p.writes["docs/changes/tune-pump.md"].contains("was-type: change"));
    }

    /// Reads the tree after the plan: each moved file at its new path, and
    /// each written text.
    pub(crate) fn after_tree(tree: &TreeData, p: &DocsPlan) -> TreeData {
        let mut blobs: HashMap<String, Vec<u8>> = tree.with(|t| (t.read)(t.files).unwrap());
        for m in &p.moves {
            let b = blobs.remove(&m.from).unwrap();
            blobs.insert(m.to.clone(), b);
        }
        for (path, text) in &p.writes {
            blobs.insert(path.clone(), text.clone().into_bytes());
        }
        TreeData::from_map(blobs)
    }

    /// The preservation of ids, requirement hashes and links: the checks that
    /// the proof of S8 makes, here in a small form.
    fn assert_preserved(tree: &TreeData, r: &Run, p: &DocsPlan) {
        use crate::model::{Corpus, LinkRes, Res};
        use crate::parse::RefKind;
        use std::sync::Arc;
        let moved: HashMap<&str, &str> = p
            .moves
            .iter()
            .map(|m| (m.from.as_str(), m.to.as_str()))
            .collect();
        let map = |path: &str| moved.get(path).copied().unwrap_or(path).to_string();
        let before_texts: BTreeMap<String, String> = r
            .classified
            .texts
            .iter()
            .map(|(path, t)| {
                let t = r
                    .changes
                    .iter()
                    .find(|c| &c.path == path)
                    .map_or(t.clone(), |c| c.edited.clone());
                (path.clone(), t)
            })
            .collect();
        let build = |files: Vec<String>, texts: Vec<(String, String)>| {
            Corpus::build(
                &r.fin,
                files,
                texts
                    .into_iter()
                    .map(|(p, t)| (p, Arc::new(parse(&t))))
                    .collect(),
            )
        };
        let before = build(
            tree.with(|t| t.files.to_vec()),
            before_texts
                .iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect(),
        );
        let after_t = after_tree(tree, p);
        let after = after_t.with(|t| {
            let paths: Vec<String> = before_texts.keys().map(|k| map(k)).collect();
            let bytes = (t.read)(&paths).unwrap();
            build(
                t.files.to_vec(),
                paths
                    .iter()
                    .map(|p| (p.clone(), String::from_utf8(bytes[p].clone()).unwrap()))
                    .collect(),
            )
        });
        // Each id resolves to the same document as before. A fixture has an
        // alias that clashes with an id, so `before` gives the answer.
        for d in &before.docs {
            let mut ids = vec![d.id.clone()];
            ids.extend(d.aliases.iter().cloned());
            ids.extend(d.legacy_aliases.iter().cloned());
            for id in ids {
                match (before.resolve(&id), after.resolve(&id)) {
                    (Res::Found(b, None, true), Res::Found(a, None, true)) => {
                        assert_eq!(
                            after.docs[a as usize].path,
                            map(&before.docs[b as usize].path),
                            "{id}"
                        )
                    }
                    (Res::Ambiguous(b), Res::Ambiguous(a)) => {
                        let mut want: Vec<String> = b
                            .iter()
                            .map(|&i| map(&before.docs[i as usize].path))
                            .collect();
                        let mut got: Vec<String> = a
                            .iter()
                            .map(|&i| after.docs[i as usize].path.clone())
                            .collect();
                        want.sort();
                        got.sort();
                        assert_eq!(got, want, "{id}");
                    }
                    (b, a) => panic!("{id} of {}: {b:?}, then {a:?}", d.path),
                }
            }
        }
        // Each moved change is an `intent` of the layer, with a valid type.
        for m in p.moves.iter().filter(|m| !m.layout) {
            let d = &after.docs[after.by_path[&m.to] as usize];
            assert_eq!(d.ty, DocType::Intent, "{}", m.to);
            assert!(!d.type_failed, "{}", m.to);
        }
        // The same requirement hashes.
        let hashes = |c: &Corpus| {
            let mut v: Vec<_> = c
                .docs
                .iter()
                .flat_map(|d| d.reqs.iter().map(|&i| d.parsed.reqs[i].frozen))
                .collect();
            v.sort_unstable();
            v
        };
        assert_eq!(hashes(&before), hashes(&after));
        // The text that a decision locks does not change, and no body does.
        let frozen = |c: &Corpus| {
            let mut v: Vec<_> = c.docs.iter().map(|d| d.frozen_hash).collect();
            v.sort_unstable();
            v
        };
        assert_eq!(frozen(&before), frozen(&after));
        // Each link goes to the same file.
        for d in &before.docs {
            let a = &after.docs[after.by_path[&map(&d.path)] as usize];
            assert_eq!(d.parsed.refs.len(), a.parsed.refs.len(), "{}", d.path);
            for (x, y) in d.parsed.refs.iter().zip(&a.parsed.refs) {
                if x.kind == RefKind::Wiki {
                    continue;
                }
                match (
                    before.resolve_link(&d.path, &x.target),
                    after.resolve_link(&a.path, &y.target),
                ) {
                    (LinkRes::File(f, ..), LinkRes::File(g, ..)) => {
                        assert_eq!(map(&f), g, "{} -> {}", d.path, x.target)
                    }
                    (LinkRes::Dir(_), LinkRes::Dir(_)) => {}
                    (LinkRes::Missing(_), LinkRes::Missing(_)) => {}
                    (x2, y2) => panic!("{}: {} gives {x2:?}, then {y2:?}", d.path, x.target),
                }
            }
        }
    }

    #[test]
    fn the_plan_keeps_each_id_each_hash_and_each_link_of_the_fixtures() {
        for case in CASES {
            for layout in [false, true] {
                let (tree, toml) = fixture_case(case);
                let (r, p) = go(&tree, &toml, layout);
                assert_preserved(&tree, &r, &p);
                let changes = r
                    .classified
                    .docs
                    .iter()
                    .filter(|c| c.fate == Fate::Move)
                    .count();
                assert_eq!(
                    p.moves.iter().filter(|m| !m.layout).count(),
                    changes,
                    "{case}"
                );
            }
        }
    }

    #[test]
    fn a_moved_change_is_an_intent_for_the_corpus_of_the_final_configuration() {
        use crate::model::Corpus;
        use std::sync::Arc;
        let (tree, toml) = fixture_case("v05");
        let (r, p) = go(&tree, &toml, false);
        let after = after_tree(&tree, &p);
        let c = after.with(|t| {
            let paths: Vec<String> = t
                .files
                .iter()
                .filter(|f| is_doc_path(f))
                .filter(|f| r.fin.root_of(f).is_some_and(|i| !r.fin.is_excluded(i, f)))
                .cloned()
                .collect();
            let bytes = (t.read)(&paths).unwrap();
            Corpus::build(
                &r.fin,
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
        });
        let status = |path: &str| {
            let d = &c.docs[c.by_path[path] as usize];
            assert_eq!(d.ty, DocType::Intent, "{path}");
            assert!(!d.type_failed, "{path}");
            d.status.clone().unwrap()
        };
        assert_eq!(status("docs/intent/add-drip-lines/intent.md"), "draft");
        assert_eq!(status("docs/intent/rework-pumps/intent.md"), "accepted");
        assert_eq!(status("docs/intent/solar-pump/intent.md"), "closed");
        let done = &c.docs[c.by_path["docs/changes/tune-pump.md"] as usize];
        assert_eq!(done.ty, DocType::Research);
        assert_eq!(done.was_type.as_deref(), Some("change"));
    }

    #[test]
    fn the_second_run_finds_no_move_and_no_edit() {
        for case in CASES {
            for layout in [false, true] {
                let (tree, toml) = fixture_case(case);
                let (r, p) = go(&tree, &toml, layout);
                let after = after_tree(&tree, &p);
                let (cl, plan2) = after.with(|t| {
                    let cl = classify(
                        t,
                        &Basis {
                            cfg: &r.fin,
                            old: None,
                        },
                    )
                    .unwrap();
                    let plan2 = plan_docs(t, &cl, &r.fin, layout).unwrap();
                    (cl, plan2)
                });
                assert!(cl.docs.iter().all(|c| c.fate == Fate::Keep), "{case}");
                assert!(
                    plan2.docs.is_empty() && plan2.moves.is_empty(),
                    "{case} {layout}: {:?}",
                    plan2.moves
                );
                assert!(plan2.writes.is_empty(), "{case} {layout}");
            }
        }
    }

    #[test]
    fn the_plan_reads_a_tree_once_and_never_the_disk() {
        use std::cell::Cell;
        let (tree, toml) = fixture_case("v05");
        let files = tree.with(|t| t.files.to_vec());
        let blobs = tree.with(|t| (t.read)(t.files).unwrap());
        let reads = Cell::new(0);
        let read = |paths: &[String]| -> Result<HashMap<String, Vec<u8>>, Fatal> {
            reads.set(reads.get() + 1);
            for p in paths {
                assert!(files.contains(p), "{p} is not in the tree");
            }
            Ok(paths
                .iter()
                .map(|p| (p.clone(), blobs[p].clone()))
                .collect())
        };
        let old = OldConfig::read(&toml).unwrap();
        let p1 = upgrade_config::pass1(&toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let t = Tree {
            files: &files,
            read: &read,
        };
        let cl = classify(
            &t,
            &Basis {
                cfg: &cfg1,
                old: Some(&old),
            },
        )
        .unwrap();
        let plan = upgrade_config::plan(&toml, &old, &cl.facts(&cfg1), &opts()).unwrap();
        let fin = Config::parse(&plan.text).unwrap();
        let p = plan_docs(&t, &cl, &fin, true).unwrap();
        assert!(!p.moves.is_empty());
        assert_eq!(reads.get(), 1);
    }

    // --- --layout ---

    #[test]
    fn layout_moves_the_folders_of_a_mixed_root_at_the_same_depth() {
        let (tree, toml) = fixture_case("v05");
        let (r, p) = go(&tree, &toml, true);
        let layout: Vec<(&str, &str, &str, &str)> = p
            .moves
            .iter()
            .filter(|m| m.layout)
            .map(|m| {
                (
                    m.from.as_str(),
                    m.to.as_str(),
                    m.old_id.as_str(),
                    m.new_id.as_str(),
                )
            })
            .collect();
        assert_eq!(
            layout,
            [
                (
                    "docs/contracts/valve-api.md",
                    "docs/interfaces/valve-api.md",
                    "contracts/valve-api",
                    "interfaces/valve-api"
                ),
                (
                    "docs/guides/logging.md",
                    "docs/procedures/logging.md",
                    "guides/logging",
                    "procedures/logging"
                ),
                (
                    "docs/invariants/water-limits.md",
                    "docs/rules/water-limits.md",
                    "invariants/water-limits",
                    "rules/water-limits"
                ),
                (
                    "docs/notes/research.md",
                    "docs/research/research.md",
                    "notes/research",
                    "research/research"
                ),
                (
                    "docs/reference/soil-study.md",
                    "docs/research/soil-study.md",
                    "reference/soil-study",
                    "research/soil-study"
                ),
                (
                    "docs/runbooks/restart-pump.md",
                    "docs/procedures/restart-pump.md",
                    "runbooks/restart-pump",
                    "procedures/restart-pump"
                ),
            ]
        );
        for m in p.moves.iter().filter(|m| m.layout) {
            assert_eq!(m.aliases, std::slice::from_ref(&m.old_id));
        }
        // A link from one moved folder to another: both follow.
        assert!(
            p.writes["docs/procedures/restart-pump.md"].contains(
                "[the water limits](../rules/water-limits.md) and [the logging guide](logging.md)"
            ),
            "{}",
            p.writes["docs/procedures/restart-pump.md"]
        );
        assert!(r.fin.roots.iter().any(|x| x.path == "docs"));
    }

    #[test]
    fn layout_moves_each_file_of_the_folder_and_rewrites_the_links_to_them() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";
        let tree = mem(&[
            (
                "docs/guides/a.md",
                "---\ntype: guide\nstatus: active\n---\n# A\n\n![p](img/p.png) and [d](sub/) and [s](sub/s.md)\n",
            ),
            ("docs/guides/img/p.png", "png"),
            (
                "docs/guides/sub/s.md",
                "---\ntype: guide\nstatus: active\n---\n# S\n\n[up](../a.md)\n",
            ),
            (
                "docs/other.md",
                "# O\n\n[a](guides/a.md), [dir](guides/), [img](guides/img/p.png)\n",
            ),
        ]);
        let (_, p) = go(&tree, toml, true);
        assert_eq!(
            p.moves
                .iter()
                .map(|m| (m.from.as_str(), m.to.as_str()))
                .collect::<Vec<_>>(),
            [
                ("docs/guides/a.md", "docs/procedures/a.md"),
                ("docs/guides/img/p.png", "docs/procedures/img/p.png"),
                ("docs/guides/sub/s.md", "docs/procedures/sub/s.md"),
            ]
        );
        // The image is not a document: no id, no alias, and no text.
        let img = mv(&p, "docs/guides/img/p.png");
        assert!(img.old_id.is_empty() && img.aliases.is_empty());
        assert!(!p.writes.contains_key("docs/procedures/img/p.png"));
        assert_eq!(
            p.writes["docs/other.md"],
            "# O\n\n[a](procedures/a.md), [dir](procedures/), [img](procedures/img/p.png)\n"
        );
        // Links inside the folder keep their text, because the depth is the same.
        assert!(
            p.writes["docs/procedures/a.md"]
                .contains("![p](img/p.png) and [d](sub/) and [s](sub/s.md)")
        );
        assert!(p.writes["docs/procedures/sub/s.md"].contains("[up](../a.md)"));
    }

    #[test]
    fn layout_stops_when_2_files_have_one_target() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";
        let tree = mem(&[
            (
                "docs/guides/x.md",
                "---\ntype: guide\nstatus: active\n---\n# G\n",
            ),
            (
                "docs/runbooks/x.md",
                "---\ntype: runbook\nstatus: active\n---\n# R\n",
            ),
        ]);
        let m = refused(&tree, toml, true);
        for part in [
            "docs/guides/x.md",
            "docs/runbooks/x.md",
            "docs/procedures/x.md",
        ] {
            assert!(m.contains(part), "{part} is not in: {m}");
        }
        // Without `--layout`, nothing moves.
        assert!(go(&tree, toml, false).1.moves.is_empty());
    }

    #[test]
    fn layout_stops_for_a_target_that_exists() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";
        let tree = mem(&[
            (
                "docs/guides/x.md",
                "---\ntype: guide\nstatus: active\n---\n# G\n",
            ),
            (
                "docs/procedures/x.md",
                "---\ntype: procedure\nstatus: active\n---\n# P\n",
            ),
        ]);
        let m = refused(&tree, toml, true);
        assert!(
            m.contains("docs/guides/x.md") && m.contains("docs/procedures/x.md"),
            "{m}"
        );
    }

    #[test]
    fn layout_stops_for_an_exclude_glob_that_matches_a_moved_path() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\nexclude = [\"guides/skip.md\"]\n";
        let tree = mem(&[
            (
                "docs/guides/a.md",
                "---\ntype: guide\nstatus: active\n---\n# G\n",
            ),
            ("docs/guides/skip.md", "# Skip\n"),
        ]);
        let m = refused(&tree, toml, true);
        assert!(
            m.contains("docs/guides/skip.md") && m.contains("exclude"),
            "{m}"
        );
    }

    #[test]
    fn a_done_change_and_a_0_5_spec_stay_in_a_folder_that_moves_but_a_change_moves_first() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";
        let tree = mem(&[
            (
                "docs/guides/g.md",
                "---\ntype: guide\nstatus: active\n---\n# G\n",
            ),
            (
                "docs/guides/done.md",
                "---\ntype: change\nstatus: done\n---\n# D\n",
            ),
            (
                "docs/guides/spec.md",
                "---\ntype: spec\nstatus: active\n---\n# S\n",
            ),
            (
                "docs/guides/open.md",
                "---\ntype: change\nstatus: accepted\n---\n# O\n",
            ),
        ]);
        let (_, p) = go(&tree, toml, true);
        let pairs: Vec<(&str, &str, bool)> = p
            .moves
            .iter()
            .map(|m| (m.from.as_str(), m.to.as_str(), m.layout))
            .collect();
        assert_eq!(
            pairs,
            [
                ("docs/guides/g.md", "docs/procedures/g.md", true),
                ("docs/guides/open.md", "docs/intent/open/intent.md", false),
            ]
        );
    }

    #[test]
    fn a_root_of_one_type_keeps_its_paths_and_a_nested_root_is_not_moved() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n[[roots]]\npath = \"docs/guides/sub\"\ntype = \"guide\"\nstatus = \"active\"\n[[roots]]\npath = \"docs/specs\"\ntype = \"spec\"\n";
        let tree = mem(&[
            (
                "docs/guides/a.md",
                "---\ntype: guide\nstatus: active\n---\n# A\n",
            ),
            ("docs/guides/sub/b.md", "# B\n"),
            ("docs/specs/guides/c.md", "# C\n"),
        ]);
        let (_, p) = go(&tree, toml, true);
        assert_eq!(
            p.moves.iter().map(|m| m.from.as_str()).collect::<Vec<_>>(),
            ["docs/guides/a.md"]
        );
    }

    #[test]
    fn the_old_legacy_alias_goes_into_aliases_too() {
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\nlegacy-ids = [\"RFC-{n}\"]\n";
        let tree = mem(&[(
            "docs/changes/RFC-01-drip.md",
            "---\ntype: change\nstatus: proposed\naliases: [old/name]\n---\n# R\n",
        )]);
        let (_, p) = go(&tree, toml, false);
        let m = mv(&p, "docs/changes/RFC-01-drip.md");
        assert_eq!(m.old_id, "changes/RFC-01-drip");
        // The id of 0.5 has the folder, so the legacy pattern `RFC-{n}` does
        // not match the slug `changes/RFC-01-drip`: the alias comes from the
        // last segment.
        assert_eq!(m.aliases, ["changes/RFC-01-drip", "RFC-1"]);
        assert_eq!(
            p.writes["docs/intent/RFC-01-drip/intent.md"],
            "---\ntype: intent\nstatus: draft\naliases:\n  - old/name\n  - changes/RFC-01-drip\n  - RFC-1\n---\n# R\n"
        );
    }

    #[test]
    fn a_tree_of_a_commit_reads_the_blobs_and_not_the_work_directory() {
        use std::process::Command;
        let dir =
            std::env::temp_dir().join(format!("spec-dna-upgrade-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        let git = |args: &[&str]| {
            let o = Command::new("git")
                .current_dir(&dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .output()
                .unwrap();
            assert!(
                o.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join("docs/a.md"), "# In the commit\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "x"]);
        // The work directory differs from HEAD, and has a file that HEAD has not.
        std::fs::write(dir.join("docs/a.md"), "# Edited\n").unwrap();
        std::fs::write(dir.join("docs/b.md"), "# New\n").unwrap();
        let tree = TreeData::from_git(&Git::new(&dir), "HEAD").unwrap();
        tree.with(|t| {
            assert_eq!(t.files, ["docs/a.md"]);
            let got = (t.read)(&["docs/a.md".to_string()]).unwrap();
            assert_eq!(got["docs/a.md"], b"# In the commit\n");
            assert!((t.read)(&["docs/b.md".to_string()]).is_err());
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- review fixes of S7: the stops of D196 for the writes of S7, roots of
    // one type, stubs and a byte order mark ---

    /// A mixed root whose documents with no type are runbooks: they need no
    /// edit, so a plan edits them only to move them.
    const MIXED_RUNBOOK: &str = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"runbook\"\nlive-sections = {}\n";

    fn config_stop(tree: &TreeData, toml: &str, layout: bool) -> String {
        match full(tree, toml, layout) {
            Err(Fatal::Config(m)) => m,
            Err(other) => panic!("not a stop with exit code 3: {other:?}"),
            Ok(_) => panic!("the plan must stop"),
        }
    }

    #[test]
    fn a_file_that_is_not_utf8_stops_the_plan_if_the_plan_writes_it() {
        let guide = |end: &[u8]| {
            let mut b = b"---\nstatus: active\n---\n# T ".to_vec();
            b.extend_from_slice(end);
            b
        };
        // It moves with its folder.
        let mut blobs: HashMap<String, Vec<u8>> = HashMap::new();
        blobs.insert("docs/guides/a.md".into(), guide(b"\xff\n"));
        let m = config_stop(&TreeData::from_map(blobs), MIXED_RUNBOOK, true);
        assert!(
            m.starts_with("docs/guides/a.md: ") && m.contains("not UTF-8"),
            "{m}"
        );
        // It links to a file that moves: the link changes.
        let mut blobs: HashMap<String, Vec<u8>> = HashMap::new();
        blobs.insert("docs/guides/a.md".into(), guide(b"A\n"));
        blobs.insert(
            "docs/o.md".into(),
            b"---\nstatus: active\n---\n# O \xff\n\n[a](guides/a.md)\n".to_vec(),
        );
        let m = config_stop(&TreeData::from_map(blobs.clone()), MIXED_RUNBOOK, true);
        assert!(
            m.starts_with("docs/o.md: ") && m.contains("not UTF-8"),
            "{m}"
        );
        // Without a write, the file passes.
        blobs.insert(
            "docs/o.md".into(),
            b"---\nstatus: active\n---\n# O \xff\n".to_vec(),
        );
        assert!(full(&TreeData::from_map(blobs), MIXED_RUNBOOK, true).is_ok());
    }

    #[test]
    fn a_moved_document_with_a_front_matter_that_aliases_would_lose_stops_the_plan() {
        // The front matter does not parse.
        let tree = mem(&[("docs/guides/a.md", "---\nstatus draft\n---\n# A\n")]);
        let m = config_stop(&tree, MIXED_RUNBOOK, true);
        assert!(
            m.starts_with("docs/guides/a.md: ") && m.contains("aliases"),
            "{m}"
        );
        // The value of `aliases` is not a list of strings.
        let tree = mem(&[("docs/guides/a.md", "---\naliases:\n  - a: b\n---\n# A\n")]);
        let m = config_stop(&tree, MIXED_RUNBOOK, true);
        assert!(
            m.starts_with("docs/guides/a.md: ") && m.contains("`aliases`"),
            "{m}"
        );
        // A document that does not move keeps its broken front matter.
        let tree = mem(&[("docs/a.md", "---\nstatus draft\n---\n# A\n")]);
        assert!(
            full(&tree, MIXED_RUNBOOK, true)
                .unwrap()
                .1
                .writes
                .is_empty()
        );
    }

    #[test]
    fn a_byte_order_mark_stops_the_plan_if_the_front_matter_must_change() {
        // A note needs the marker.
        let tree = mem(&[("docs/n.md", "\u{feff}# N\n")]);
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n";
        let m = config_stop(&tree, toml, false);
        assert!(
            m.starts_with("docs/n.md: ") && m.contains("byte order mark"),
            "{m}"
        );
        // A moved document needs aliases.
        let tree = mem(&[(
            "docs/guides/a.md",
            "\u{feff}---\nstatus: active\n---\n# A\n",
        )]);
        let m = config_stop(&tree, MIXED_RUNBOOK, true);
        assert!(
            m.starts_with("docs/guides/a.md: ") && m.contains("byte order mark"),
            "{m}"
        );
        // No edit and no move: the file stays.
        let tree = mem(&[("docs/s.md", "\u{feff}# S\n")]);
        assert!(
            full(&tree, MIXED_RUNBOOK, true)
                .unwrap()
                .1
                .writes
                .is_empty()
        );
    }

    #[test]
    fn layout_leaves_a_root_that_had_one_type_in_0_5() {
        // `docs/team` was a root of notes, so it is a mixed root now. Its
        // folders keep their names. `docs/lab` was mixed, so its folders move.
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n[[roots]]\npath = \"docs/team\"\ntype = \"note\"\n[[roots]]\npath = \"docs/lab\"\ntype = \"mixed\"\n";
        let tree = mem(&[
            ("docs/team/guides/a.md", "# A\n"),
            ("docs/team/notes/b.md", "# B\n"),
            (
                "docs/lab/guides/c.md",
                "---\ntype: guide\nstatus: active\n---\n# C\n",
            ),
        ]);
        let (r, p) = go(&tree, toml, true);
        assert!(
            r.fin
                .roots
                .iter()
                .any(|x| x.path == "docs/team" && x.single_type.is_none())
        );
        assert_eq!(
            p.moves
                .iter()
                .map(|m| (m.from.as_str(), m.to.as_str()))
                .collect::<Vec<_>>(),
            [("docs/lab/guides/c.md", "docs/lab/procedures/c.md")]
        );
    }

    #[test]
    fn a_stub_of_mv_is_no_change_no_note_and_stays_in_place() {
        let stub = |link: &str| {
            format!(
                "---\nmoved-to: guides/real\n---\n# Old\n\nThis document moved to [Old]({link}).\n"
            )
        };
        // A root of changes, and a root whose default type is a note.
        let toml = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\nlive-sections = {}\n[[roots]]\npath = \"docs/rfcs\"\ntype = \"change\"\nstatus = \"proposed\"\n";
        let tree = mem(&[
            ("docs/old.md", &stub("guides/real.md")),
            ("docs/rfcs/old.md", &stub("../guides/real.md")),
            ("docs/guides/stub.md", &stub("real.md")),
            (
                "docs/guides/real.md",
                "---\ntype: guide\nstatus: active\n---\n# Real\n",
            ),
        ]);
        let (r, p) = go(&tree, toml, true);
        for path in ["docs/old.md", "docs/rfcs/old.md", "docs/guides/stub.md"] {
            let c = r.classified.docs.iter().find(|c| c.path == path).unwrap();
            assert!(
                c.stub && c.old_type.is_none() && c.fate == Fate::Keep,
                "{path}"
            );
        }
        assert_eq!(
            r.changes
                .iter()
                .map(|c| c.path.as_str())
                .collect::<Vec<_>>(),
            ["docs/guides/real.md"]
        );
        // The folder moves, the stub stays, and its link follows the target.
        assert_eq!(
            p.moves.iter().map(|m| m.from.as_str()).collect::<Vec<_>>(),
            ["docs/guides/real.md"]
        );
        assert!(p.writes["docs/guides/stub.md"].contains("[Old](../procedures/real.md)"));
        assert!(p.writes["docs/old.md"].contains("[Old](procedures/real.md)"));
        assert!(r.facts.change_roots.is_empty());
    }
}
