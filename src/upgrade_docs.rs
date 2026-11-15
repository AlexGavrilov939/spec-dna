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
//! 2. `edit_docs` rewrites the front matter by the final configuration. It
//!    reads the place of a document (`type:` where the place gives another
//!    type), at the path where the document ends.

// `upgrade` (step 5, slice 10) calls this module. Until then only the tests do.
#![allow(dead_code)]

use crate::config::Config;
use crate::frontmatter::{self, Value};
use crate::model::{DocType, is_doc_path};
use crate::parse::parse;
use crate::repo::Fatal;
use crate::upgrade_config::Facts;
use crate::upgrade_old::{self as names, Kind, OldConfig, OldRoot, change_status, type_of};
use std::collections::{BTreeMap, HashMap};

/// The reader of a tree: the bytes of each path that it gets.
pub type ReadFn = dyn Fn(&[String]) -> Result<HashMap<String, Vec<u8>>, Fatal>;

/// One tree: `HEAD` for `upgrade`, a base commit for `check`. `files` are the
/// paths of the tree, sorted. `read` gives the bytes of the paths that it gets.
pub struct Tree<'a> {
    pub files: &'a [String],
    pub read: &'a ReadFn,
}

type Reader = Box<ReadFn>;

/// A tree that owns its list of files and its reader.
pub struct TreeData {
    files: Vec<String>,
    reader: Reader,
}

impl TreeData {
    /// A tree in memory: each path with its bytes.
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
}

/// The documents of a tree, with their classes.
#[derive(Debug, Default)]
pub struct Classified {
    pub docs: Vec<Class>,
    /// The text of each document.
    pub texts: BTreeMap<String, String>,
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
        let root = cfg.root_of(&path).expect("a document is in a root");
        let (old_type, old_status) = match basis.old {
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
        });
        out.texts.insert(path, text);
    }
    Ok(out)
}

fn stop(path: &str, why: &str) -> Fatal {
    Fatal::Config(format!(
        "{path}: {why}; upgrade does not guess. Write the change by hand, then run `spec-dna upgrade` again"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade_config::{self, ConfigOpts, ConfigPlan};
    use std::path::{Path, PathBuf};

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
    }

    /// Each file of a fixture folder, as a tree in memory.
    fn fixture_tree(case: &str) -> TreeData {
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

    fn mem(files: &[(&str, &str)]) -> TreeData {
        TreeData::from_map(
            files
                .iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        )
    }

    fn opts() -> ConfigOpts {
        ConfigOpts {
            pin: "0.6.0".into(),
            sums: None,
        }
    }

    /// What the pipeline of `plan` gives for a tree and a 0.5 file, up to the
    /// edits of the front matter.
    struct Run {
        classified: Classified,
        facts: Facts,
        plan: ConfigPlan,
        fin: Config,
        changes: Vec<DocChange>,
    }

    fn run_with(tree: &TreeData, toml: &str, dest: &HashMap<String, String>) -> Run {
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
        let changes = edit_docs(&classified, &fin, dest).unwrap();
        Run {
            classified,
            facts,
            plan,
            fin,
            changes,
        }
    }

    fn run(tree: &TreeData, toml: &str) -> Run {
        run_with(tree, toml, &HashMap::new())
    }

    fn edited<'a>(r: &'a Run, path: &str) -> &'a str {
        r.changes
            .iter()
            .find(|c| c.path == path)
            .map(|c| c.edited.as_str())
            .unwrap_or_else(|| panic!("no change for {path}"))
    }

    fn message(e: Fatal) -> String {
        match e {
            Fatal::Config(m) | Fatal::Refused(m) => m,
            other => panic!("unexpected error: {other:?}"),
        }
    }

    /// A mixed root with no default type: each document says its type.
    const MIXED: &str = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n";

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
    fn recorded(case: &str) -> Vec<(String, String, String)> {
        let rec = std::fs::read_to_string(fixtures().join(format!("recorded/{case}.txt"))).unwrap();
        rec.lines()
            .filter_map(|l| l.strip_prefix("doc "))
            .map(|l| {
                let p: Vec<&str> = l.split(' ').collect();
                (p[0].to_string(), p[2].to_string(), p[3].to_string())
            })
            .collect()
    }

    const CASES: [&str; 5] = ["v05", "v04", "brownfield", "harness-lean", "harness-full"];

    /// The tree and the 0.5 file of a case. A harness case is `v05/` with the
    /// files that `init` of 0.5.0 wrote, and its `[harness]` text.
    fn fixture_case(case: &str) -> (TreeData, String) {
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

    fn fixture_run(case: &str) -> Run {
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
}
