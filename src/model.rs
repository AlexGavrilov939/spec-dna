//! This module applies the information model of ADR 0001 to one tree. It
//! contains documents with ids, types, statuses and aliases, and it resolves
//! references.

use crate::config::Config;
use crate::diag::Code;
use crate::frontmatter::Value;
use crate::hash::{Hash, nfc};
use crate::parse::ParsedDoc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DocType {
    Spec = 0,
    Contract = 1,
    Guide = 2,
    Decision = 3,
    Change = 4,
    Note = 5,
    Invariant = 6,
    Runbook = 7,
    Incident = 8,
    Reference = 9,
}

impl DocType {
    /// The number of types. Per-type tables of the configuration have this
    /// length, with the index `DocType as usize`.
    pub const COUNT: usize = 10;

    pub const ALL: [DocType; DocType::COUNT] = [
        DocType::Spec,
        DocType::Contract,
        DocType::Guide,
        DocType::Decision,
        DocType::Change,
        DocType::Note,
        DocType::Invariant,
        DocType::Runbook,
        DocType::Incident,
        DocType::Reference,
    ];

    pub fn name(self) -> &'static str {
        [
            "spec",
            "contract",
            "guide",
            "decision",
            "change",
            "note",
            "invariant",
            "runbook",
            "incident",
            "reference",
        ][self as usize]
    }

    pub fn from_name(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|t| t.name() == s)
    }

    /// The names of all types, for messages.
    pub fn names() -> String {
        DocType::ALL.map(DocType::name).join(", ")
    }

    /// The statuses of the type. A type without statuses (note, incident,
    /// reference) needs no `status` and accepts any value.
    pub fn statuses(self) -> &'static [&'static str] {
        match self {
            DocType::Spec
            | DocType::Contract
            | DocType::Guide
            | DocType::Invariant
            | DocType::Runbook => &["draft", "active", "deprecated", "superseded", "retired"],
            DocType::Decision => &["proposed", "accepted", "superseded", "rejected"],
            DocType::Change => &["proposed", "accepted", "done", "rejected"],
            DocType::Note | DocType::Incident | DocType::Reference => &[],
        }
    }

    pub fn has_status(self) -> bool {
        !self.statuses().is_empty()
    }

    pub fn has_requirements(self) -> bool {
        matches!(self, DocType::Spec | DocType::Contract | DocType::Invariant)
    }

    /// Normative types: `context` serves them for a matching `applies-to`.
    pub fn is_normative(self) -> bool {
        matches!(
            self,
            DocType::Spec
                | DocType::Contract
                | DocType::Guide
                | DocType::Invariant
                | DocType::Runbook
        )
    }

    /// Types that describe the past or give information only. They do not
    /// give drift warnings, and `context` lists them without their text.
    pub fn is_informative(self) -> bool {
        matches!(
            self,
            DocType::Change | DocType::Note | DocType::Incident | DocType::Reference
        )
    }
}

/// A problem that spec-dna finds when it reads the metadata of a document.
#[derive(Clone, Debug)]
pub struct Issue {
    pub code: Code,
    pub line: u32,
    pub target: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub path: String,
    pub id: String,
    pub root: usize,
    pub ty: DocType,
    pub status: Option<String>,
    pub parsed: Arc<ParsedDoc>,
    /// Aliases from front matter.
    pub aliases: Vec<String>,
    /// Aliases from the root's legacy id patterns.
    pub legacy_aliases: Vec<String>,
    pub supersedes: Vec<(String, u32)>,
    pub depends_on: Vec<(String, u32)>,
    pub applies_to: Vec<String>,
    pub verified_by: Vec<(String, u32)>,
    pub owner: Option<String>,
    pub issues: Vec<Issue>,
    /// Indices into `parsed.reqs` of the valid requirements.
    pub reqs: Vec<usize>,
    /// For a decision, the chunks whose heading is one of the required
    /// section headings of the root (the outcome).
    pub outcome: Vec<usize>,
    /// The chunks of the amendment sections: each chunk whose heading matches
    /// an `amendment-sections` name of the root, and the chunks below it.
    pub amendments: Vec<usize>,
    /// The chunks of the history sections (`history-sections` of the root),
    /// with the chunks below them.
    pub history: Vec<usize>,
    /// The chunks of the companion sections (`companion-sections` of the
    /// root), with the chunks below them.
    pub companions: Vec<usize>,
    /// Hash of the chunks outside the amendment, history and live sections,
    /// without link destinations. After a decision is accepted, this text is
    /// frozen (SD405).
    pub frozen_hash: Hash,
    /// For a stub that `mv --stub` left at an old path: the id in its
    /// `moved-to` field. A stub has no id of its own and no budget.
    pub moved_to: Option<(String, u32)>,
    /// For an index that `spec-dna index` generated: the hash in its
    /// `generated` field.
    pub generated: Option<String>,
}

impl Doc {
    pub fn has_anchor(&self, frag: &str) -> bool {
        self.parsed.chunks.iter().any(|c| c.anchor == frag) || self.req(frag).is_some()
    }

    pub fn req(&self, key: &str) -> Option<usize> {
        self.reqs
            .iter()
            .copied()
            .find(|&i| self.parsed.reqs[i].key == key)
    }

    pub fn status_str(&self) -> &str {
        self.status.as_deref().unwrap_or("")
    }

    pub fn is_inactive(&self) -> bool {
        matches!(self.status_str(), "superseded" | "retired" | "rejected")
            || self.moved_to.is_some()
    }

    /// A stub or a generated index: no budget applies.
    pub fn is_exempt_from_budgets(&self) -> bool {
        self.moved_to.is_some() || self.generated.is_some()
    }

    /// Returns true if the chunk holds at least one valid requirement.
    pub fn has_req_in(&self, chunk: usize) -> bool {
        self.reqs
            .iter()
            .any(|&r| self.parsed.reqs[r].chunk as usize == chunk)
    }
}

/// A requirement key is a slug: lowercase ASCII letters and digits in
/// hyphen-separated groups.
pub fn is_slug(key: &str) -> bool {
    !key.is_empty()
        && key.split('-').all(|g| {
            !g.is_empty()
                && g.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Returns the document id for a path under a root (ADR 0001, "Identifiers").
pub fn doc_id(rel: &str) -> String {
    let stem = rel.rsplit_once('.').map_or(rel, |(s, _)| s);
    if let Some((dir, last)) = stem.rsplit_once('/')
        && (last.eq_ignore_ascii_case("readme") || last.eq_ignore_ascii_case("index"))
    {
        return dir.to_string();
    }
    stem.to_string()
}

/// Returns the id of a document under a root. A `README.md` or `index.md`
/// takes the name of its directory. Thus such a file directly in a root takes
/// the name of the root.
pub fn root_doc_id(root: &crate::config::Root, path: &str) -> String {
    let id = bare_doc_id(root, path);
    match &root.namespace {
        Some(ns) if root.rel(path).contains('/') || !is_index_name(&doc_id(root.rel(path))) => {
            format!("{ns}/{id}")
        }
        Some(ns) => ns.clone(),
        None => id,
    }
}

fn is_index_name(id: &str) -> bool {
    !id.contains('/') && (id.eq_ignore_ascii_case("readme") || id.eq_ignore_ascii_case("index"))
}

/// The id of a document without the namespace of a reference root: the id of
/// version 0.2. It stays a fallback name (`Corpus::weak`).
pub fn bare_doc_id(root: &crate::config::Root, path: &str) -> String {
    let rel = root.rel(path);
    let id = doc_id(rel);
    if is_index_name(&id)
        && let Some(name) = root.path.rsplit('/').next().filter(|n| !n.is_empty())
    {
        return name.to_string();
    }
    id
}

pub fn is_doc_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

/// Returns the canonical lookup form: NFC, and `PREFIX-0012` becomes `PREFIX-12`.
pub fn canonical(name: &str) -> String {
    let name = nfc(name);
    if let Some((p, n)) = name.rsplit_once('-')
        && !p.is_empty()
        && p.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && p.as_bytes()[0].is_ascii_uppercase()
        && !n.is_empty()
        && n.bytes().all(|b| b.is_ascii_digit())
    {
        let n = n.trim_start_matches('0');
        return format!("{p}-{}", if n.is_empty() { "0" } else { n });
    }
    name.into_owned()
}

#[derive(Debug, PartialEq)]
pub enum Res {
    /// The document, the fragment if any, and whether the fragment exists.
    Found(u32, Option<String>, bool),
    Ambiguous(Vec<u32>),
    Missing,
    CrossRepo(String),
}

#[derive(Debug, PartialEq)]
pub enum LinkRes {
    External,
    /// A tracked file (and the document index if it is one) with a fragment.
    File(String, Option<u32>, Option<String>),
    /// A directory, with the document of its `README.md` or `index.md`.
    Dir(Option<u32>),
    SelfAnchor(String),
    Missing(String),
}

pub struct Corpus {
    pub docs: Vec<Doc>,
    pub lookup: HashMap<String, Vec<u32>>,
    /// The ids without the namespace of a reference root. A name resolves
    /// here only if `lookup` does not have it. These names give no duplicate
    /// id.
    pub weak: HashMap<String, Vec<u32>>,
    /// Section aliases: an `aliases` entry `<old-id>#<anchor>` of the document
    /// that a section moved to.
    pub frag_aliases: HashMap<String, Vec<u32>>,
    pub by_path: HashMap<String, u32>,
    pub files: HashSet<String>,
    pub dirs: HashSet<String>,
    pub req_aliases: HashMap<String, Vec<(u32, usize)>>,
    pub token_prefixes: Vec<String>,
    pub repos: Vec<String>,
    pub self_urls: Vec<String>,
    /// The convention files of the tree (`Config::convention_files`), parsed:
    /// Markdown files that a `conventions` pointer names and that are not
    /// documents. No check, budget, id or rank uses them.
    pub outside: HashMap<String, Arc<ParsedDoc>>,
    /// The `covers:` tags of the test files. `check` and `show` load them
    /// (`evidence::load`). The other commands leave the list empty.
    pub covers: Vec<crate::evidence::Cover>,
}

impl Corpus {
    /// `files` is every path in the tree. `docs` contains the document paths
    /// with their parsed data, after the filter by roots and excludes.
    pub fn build(
        config: &Config,
        files: Vec<String>,
        docs: Vec<(String, Arc<ParsedDoc>)>,
    ) -> Corpus {
        let mut dirs = HashSet::new();
        for f in &files {
            let mut p = f.as_str();
            while let Some((d, _)) = p.rsplit_once('/') {
                if !dirs.insert(d.to_string()) {
                    break;
                }
                p = d;
            }
        }
        let docs: Vec<Doc> = docs
            .into_iter()
            .map(|(path, parsed)| make_doc(config, path, parsed))
            .collect();
        let mut lookup: HashMap<String, Vec<u32>> = HashMap::new();
        let mut weak: HashMap<String, Vec<u32>> = HashMap::new();
        let mut frag_aliases: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_path = HashMap::new();
        let mut req_aliases: HashMap<String, Vec<(u32, usize)>> = HashMap::new();
        for (i, d) in docs.iter().enumerate() {
            let i = i as u32;
            by_path.insert(d.path.clone(), i);
            // A stub has no name: the moved document keeps its old id.
            if d.moved_to.is_some() {
                continue;
            }
            let mut names: Vec<String> = vec![canonical(&d.id)];
            for a in d.aliases.iter().chain(&d.legacy_aliases) {
                if a.contains('#') {
                    frag_aliases.entry(canonical_ref(a)).or_default().push(i);
                } else {
                    names.push(canonical(a));
                }
            }
            names.sort();
            names.dedup();
            for n in names {
                lookup.entry(n).or_default().push(i);
            }
            let root = &config.roots[d.root];
            if root.namespace.is_some() {
                let bare = canonical(&nfc(&bare_doc_id(root, &d.path)));
                weak.entry(bare).or_default().push(i);
            }
            for &r in &d.reqs {
                for a in &d.parsed.reqs[r].aliases {
                    req_aliases
                        .entry(canonical_ref(a))
                        .or_default()
                        .push((i, r));
                }
            }
        }
        Corpus {
            docs,
            lookup,
            weak,
            frag_aliases,
            by_path,
            files: files.into_iter().collect(),
            dirs,
            req_aliases,
            token_prefixes: config.token_prefixes(),
            repos: config.repos.iter().map(|r| r.0.clone()).collect(),
            self_urls: config.self_urls.clone(),
            outside: HashMap::new(),
            covers: Vec::new(),
        }
    }

    /// Resolves an id reference: `id` or `id#fragment`, each with an optional
    /// `@hash`.
    pub fn resolve(&self, target: &str) -> Res {
        let target = strip_hash_suffix(target.trim());
        if let Some((repo, rest)) = target.split_once(':')
            && !repo.is_empty()
            && !repo.contains(['/', '#'])
            && !rest.is_empty()
        {
            return Res::CrossRepo(repo.to_string());
        }
        let (idpart, frag) = match target.split_once('#') {
            Some((a, b)) => (a, Some(b)),
            None => (target, None),
        };
        if let Some(f) = frag
            && let Some(hits) = self
                .req_aliases
                .get(&canonical_ref(&format!("{idpart}#{f}")))
        {
            if hits.len() == 1 {
                let (d, r) = hits[0];
                return Res::Found(
                    d,
                    Some(self.docs[d as usize].parsed.reqs[r].key.clone()),
                    true,
                );
            }
            return Res::Ambiguous(hits.iter().map(|h| h.0).collect());
        }
        let res = self.resolve_name(idpart, frag);
        // A section that moved: the old `<id>#<anchor>` is an alias of the
        // document that holds it now, unless the old document still has it.
        if let Some(f) = frag
            && matches!(res, Res::Missing | Res::Found(_, _, false))
            && let Some(hits) = self
                .frag_aliases
                .get(&canonical_ref(&format!("{idpart}#{f}")))
        {
            return match hits.as_slice() {
                [d] => Res::Found(
                    *d,
                    Some(f.to_string()),
                    self.docs[*d as usize].has_anchor(f),
                ),
                many => Res::Ambiguous(many.to_vec()),
            };
        }
        res
    }

    fn resolve_name(&self, idpart: &str, frag: Option<&str>) -> Res {
        let hits = match self.lookup.get(&canonical(idpart)).map(Vec::as_slice) {
            None | Some([]) => self.weak.get(&canonical(idpart)).map(Vec::as_slice),
            other => other,
        };
        match hits {
            None | Some([]) => Res::Missing,
            Some([d]) => {
                let ok = frag.is_none_or(|f| self.docs[*d as usize].has_anchor(f));
                Res::Found(*d, frag.map(str::to_string), ok)
            }
            Some(many) => {
                // The id of the document itself has priority over the alias of
                // another document. Another check reports the clash as a
                // duplicate id.
                let own: Vec<u32> = many
                    .iter()
                    .copied()
                    .filter(|&d| canonical(&self.docs[d as usize].id) == canonical(idpart))
                    .collect();
                match own.as_slice() {
                    [d] => {
                        let ok = frag.is_none_or(|f| self.docs[*d as usize].has_anchor(f));
                        Res::Found(*d, frag.map(str::to_string), ok)
                    }
                    _ => Res::Ambiguous(many.to_vec()),
                }
            }
        }
    }

    /// Resolves a Markdown link target from the document at `from`.
    pub fn resolve_link(&self, from: &str, target: &str) -> LinkRes {
        let t = target.trim();
        if t.is_empty() {
            return LinkRes::External;
        }
        // A URL under one of `self-urls` is a path from the repository root.
        let (t, absolute) = if t.starts_with("//") || has_scheme(t) {
            match self.self_urls.iter().find_map(|p| strip_url_prefix(p, t)) {
                Some(rest) => (rest, true),
                None => return LinkRes::External,
            }
        } else {
            (t, false)
        };
        let (path_part, frag) = match t.split_once('#') {
            Some((p, f)) => (p, Some(percent_decode(f))),
            None => (t, None),
        };
        let path_part = path_part.split('?').next().unwrap_or("");
        if path_part.is_empty() && !absolute {
            return match frag {
                Some(f) => LinkRes::SelfAnchor(f),
                None => LinkRes::External,
            };
        }
        let decoded = percent_decode(path_part);
        let base = if absolute || decoded.starts_with('/') {
            ""
        } else {
            from.rsplit_once('/').map_or("", |(d, _)| d)
        };
        let Some(joined) = join(base, &decoded) else {
            return LinkRes::Missing(decoded);
        };
        let joined = nfc(&joined).into_owned();
        if self.files.contains(&joined) {
            let doc = self.by_path.get(&joined).copied();
            return LinkRes::File(joined, doc, frag);
        }
        if joined.is_empty() || self.dirs.contains(&joined) {
            let index = ["README.md", "readme.md", "Readme.md", "index.md"]
                .iter()
                .find_map(|n| {
                    let p = if joined.is_empty() {
                        n.to_string()
                    } else {
                        format!("{joined}/{n}")
                    };
                    self.by_path.get(&p).copied()
                });
            return LinkRes::Dir(index);
        }
        if absolute {
            // It can name another branch or commit. Do not check it.
            return LinkRes::External;
        }
        LinkRes::Missing(joined)
    }
}

fn make_doc(config: &Config, path: String, parsed: Arc<ParsedDoc>) -> Doc {
    let root_i = config.root_of(&path).expect("documents are inside a root");
    let root = &config.roots[root_i];
    let id = nfc(&root_doc_id(root, &path)).into_owned();
    let slug = id.rsplit('/').next().unwrap_or(&id).to_string();
    let mut issues = Vec::new();
    if let Some((line, msg)) = &parsed.front_matter_error {
        issues.push(Issue {
            code: Code::FrontMatterSyntax,
            line: *line,
            target: String::new(),
            message: msg.clone(),
        });
    }
    let empty = crate::frontmatter::FrontMatter::default();
    let fm = parsed.front_matter.as_ref().unwrap_or(&empty);

    let list = |key: &str, issues: &mut Vec<Issue>| -> Vec<(String, u32)> {
        match fm.get(key) {
            None => Vec::new(),
            Some((Value::Str(s), l)) => {
                if s.is_empty() {
                    Vec::new()
                } else {
                    vec![(s.clone(), l)]
                }
            }
            Some((Value::List(v), l)) => v.iter().map(|s| (s.clone(), l)).collect(),
            Some((Value::Unsupported(what), l)) => {
                issues.push(Issue {
                    code: Code::FrontMatterField,
                    line: l,
                    target: key.to_string(),
                    message: format!("`{key}` holds {what}; use a string or a list of strings"),
                });
                Vec::new()
            }
        }
    };
    let supersedes = list("supersedes", &mut issues);
    let depends_on = list("depends-on", &mut issues);
    let applies_to = list("applies-to", &mut issues)
        .into_iter()
        .map(|(s, _)| s)
        .collect();
    let verified_by = list("verified-by", &mut issues);
    let aliases: Vec<String> = list("aliases", &mut issues)
        .into_iter()
        .map(|(s, _)| s)
        .collect();
    let scalar = |key: &str, issues: &mut Vec<Issue>| -> Option<(String, u32)> {
        match fm.get(key) {
            None => None,
            Some((Value::Str(s), l)) => Some((s.clone(), l)),
            Some((_, l)) => {
                issues.push(Issue {
                    code: Code::FrontMatterField,
                    line: l,
                    target: key.to_string(),
                    message: format!("`{key}` must be a single string"),
                });
                None
            }
        }
    };
    let owner = scalar("owner", &mut issues).map(|(s, _)| s);
    let fm_type = scalar("type", &mut issues);
    let fm_status = scalar("status", &mut issues);
    let moved_to = scalar("moved-to", &mut issues).filter(|(s, _)| !s.is_empty());
    let generated = scalar("generated", &mut issues).and_then(|(s, _)| {
        s.strip_prefix(crate::registry::MARKER)
            .map(|h| h.trim().to_string())
    });
    if moved_to.is_some() {
        // A stub: no type rules, no status, no requirements, no budget.
        return Doc {
            path,
            id,
            root: root_i,
            ty: root.default_type,
            status: None,
            parsed,
            aliases: Vec::new(),
            legacy_aliases: Vec::new(),
            supersedes: Vec::new(),
            depends_on: Vec::new(),
            applies_to: Vec::new(),
            verified_by: Vec::new(),
            owner,
            issues,
            reqs: Vec::new(),
            outcome: Vec::new(),
            amendments: Vec::new(),
            history: Vec::new(),
            companions: Vec::new(),
            frozen_hash: Hash::default(),
            moved_to,
            generated: None,
        };
    }

    let mut ty = root.default_type;
    if let Some((t, line)) = fm_type {
        match DocType::from_name(&t) {
            None => issues.push(Issue {
                code: Code::TypeInvalid,
                line,
                target: t.clone(),
                message: format!("unknown type `{t}`; use one of {}", DocType::names()),
            }),
            Some(t2) => match root.single_type {
                Some(st) if st != t2 => issues.push(Issue {
                    code: Code::TypeInvalid,
                    line,
                    target: t.clone(),
                    message: format!(
                        "type `{t}` disagrees with root `{}` of type `{}`",
                        root.path,
                        st.name()
                    ),
                }),
                _ => ty = t2,
            },
        }
    }
    if !config.allows(ty) {
        let names: Vec<&str> = config.types.iter().map(|t| t.name()).collect();
        issues.push(Issue {
            code: Code::TypeInvalid,
            line: 1,
            target: ty.name().to_string(),
            message: format!(
                "type `{}` is not a type of this project (`types` in spec-dna.toml: {})",
                ty.name(),
                names.join(", ")
            ),
        });
    }
    let status = match fm_status {
        Some((s, line)) => {
            let allowed = |x: &str| !ty.has_status() || ty.statuses().contains(&x);
            // If the root maps a legacy word, use the mapped status. The file
            // keeps the legacy word.
            let canon = if allowed(&s) {
                s.clone()
            } else {
                root.status_map
                    .iter()
                    .find(|(from, _)| *from == s)
                    .map_or_else(|| s.clone(), |(_, to)| to.clone())
            };
            if !allowed(&canon) {
                let what = if canon == s {
                    format!("`{s}`")
                } else {
                    format!("`{s}` maps to `{canon}`, which")
                };
                issues.push(Issue {
                    code: Code::StatusInvalid,
                    line,
                    target: s.clone(),
                    message: format!(
                        "{what} is not a status of type `{}` ({})",
                        ty.name(),
                        ty.statuses().join(", ")
                    ),
                });
            }
            Some(canon)
        }
        None if ty == root.default_type => root.status.clone(),
        None => None,
    };
    if status.is_none() && ty.has_status() {
        issues.push(Issue {
            code: Code::StatusInvalid,
            line: 1,
            target: String::new(),
            message: format!(
                "a `{}` needs a `status` ({})",
                ty.name(),
                ty.statuses().join(", ")
            ),
        });
    }

    let legacy_aliases: Vec<String> = root
        .legacy_ids
        .iter()
        .filter_map(|l| l.alias_for(&slug))
        .collect();

    let mut reqs = Vec::new();
    if ty.has_requirements() {
        let mut seen: Vec<&str> = Vec::new();
        for (i, r) in parsed.reqs.iter().enumerate() {
            let legacy = root.legacy_keys.iter().any(|p| p.is_match(&r.key));
            // Without a colon, only a legacy key makes a requirement. Other
            // bold labels are prose, unless a slug key comes with a BCP 14
            // keyword. Then the item is a requirement without its colon, and
            // the check gives SD807.
            if !r.colon && !legacy {
                if is_slug(&r.key) && crate::parse::has_bcp14(&r.text) {
                    issues.push(Issue {
                        code: Code::RequirementColonMissing,
                        line: r.line,
                        target: r.key.clone(),
                        message: format!(
                            "`**{}**` starts a list item with a BCP 14 keyword, but no colon follows the key, so it is not a requirement",
                            r.key
                        ),
                    });
                }
                continue;
            }
            let valid = legacy || is_slug(&r.key);
            if !valid {
                if crate::parse::has_bcp14(&r.text) {
                    issues.push(Issue {
                        code: Code::KeyFormat,
                        line: r.line,
                        target: r.key.clone(),
                        message: format!(
                            "requirement key `{}` is not a slug (lowercase letters and digits joined by hyphens) and matches no legacy key pattern",
                            r.key
                        ),
                    });
                }
                continue;
            }
            if seen.contains(&r.key.as_str()) {
                // A repeated legacy key without a colon is a reference, for
                // example in a traceability list. The first item defines it.
                if !r.colon {
                    continue;
                }
                issues.push(Issue {
                    code: Code::KeyDuplicate,
                    line: r.line,
                    target: r.key.clone(),
                    message: format!("requirement key `{}` is used twice in this document", r.key),
                });
                continue;
            }
            seen.push(&r.key);
            reqs.push(i);
        }
        let keys: HashSet<&str> = reqs.iter().map(|&i| parsed.reqs[i].key.as_str()).collect();
        let missed: Vec<&crate::parse::Token> = parsed
            .leads
            .iter()
            .filter(|t| !keys.contains(t.text.as_str()))
            .filter(|t| root.legacy_keys.iter().any(|p| p.is_match(&t.text)))
            .collect();
        if !reqs.is_empty()
            && let Some(first) = missed.first()
        {
            // Some requirements are in a syntax that spec-dna reads, and
            // others are not.
            issues.push(Issue {
                code: Code::RequirementsUnrecognized,
                line: first.line,
                target: String::new(),
                message: format!(
                    "{} list items or table rows start with a `legacy-keys` token but are not recognized requirements (first: `{}`)",
                    missed.len(),
                    first.text
                ),
            });
        }
        if reqs.is_empty() {
            let legacy: Vec<&crate::parse::Token> = parsed
                .tokens
                .iter()
                .filter(|t| root.legacy_keys.iter().any(|p| p.is_match(&t.text)))
                .collect();
            if let Some(first) = legacy.first() {
                issues.push(Issue {
                    code: Code::RequirementsUnrecognized,
                    line: first.line,
                    target: String::new(),
                    message: format!(
                        "{} tokens match `legacy-keys` (first: `{}`), but no requirement is recognized; write each one as a list item that starts with its bold key",
                        legacy.len(),
                        first.text
                    ),
                });
            }
        }
    }
    // Each group of required headings needs one matching section (SD305).
    // The chunks that match the first group are the outcome of a decision.
    let mut matched: Vec<usize> = Vec::new();
    // A generated index has no required sections.
    let required: &[Vec<String>] = if generated.is_some() {
        &[]
    } else {
        &root.required_sections[ty as usize]
    };
    for (g, group) in required.iter().enumerate() {
        let hits: Vec<usize> = parsed
            .chunks
            .iter()
            .enumerate()
            .filter(|(_, ch)| ch.level > 0 && group.iter().any(|s| same_heading(&ch.heading, s)))
            .map(|(i, _)| i)
            .collect();
        if hits.is_empty() {
            let names: Vec<String> = group.iter().map(|s| format!("`{s}`")).collect();
            issues.push(Issue {
                code: Code::SectionMissing,
                line: 1,
                target: if g == 0 {
                    format!("{}-section", ty.name())
                } else {
                    format!("{}-section-{}", ty.name(), g + 1)
                },
                message: format!(
                    "a {} needs a section headed {}",
                    ty.name(),
                    names.join(" or ")
                ),
            });
        }
        if g == 0 {
            matched = hits;
        }
    }
    let outcome = if ty == DocType::Decision {
        matched
    } else {
        Vec::new()
    };
    let amendments = amendment_chunks(&parsed.chunks, &root.amendment_sections[ty as usize]);
    let history = amendment_chunks(&parsed.chunks, &root.history_sections[ty as usize]);
    let companions = amendment_chunks(&parsed.chunks, &root.companion_sections[ty as usize]);
    // The live sections (such as an implementation status) record a current
    // state, so their text can change in place.
    let live = amendment_chunks(&parsed.chunks, &root.live_sections[ty as usize]);
    let mut frozen = Vec::new();
    for (i, ch) in parsed.chunks.iter().enumerate() {
        if !amendments.contains(&i) && !history.contains(&i) && !live.contains(&i) {
            frozen.extend_from_slice(&ch.frozen);
        }
    }
    let frozen_hash = crate::hash::hash_bytes(&frozen);
    Doc {
        path,
        id,
        root: root_i,
        ty,
        status,
        parsed,
        aliases,
        legacy_aliases,
        supersedes,
        depends_on,
        applies_to,
        verified_by,
        owner,
        issues,
        reqs,
        outcome,
        amendments,
        history,
        companions,
        frozen_hash,
        moved_to: None,
        generated,
    }
}

/// Compares headings without case (in any script), and ignores the space at
/// the start and at the end.
pub fn same_heading(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Returns true if a heading is an amendment name. It also returns true if the
/// heading starts with the name and a character that is not a letter or a
/// digit (`Amendment 3: TTL`). The comparison ignores case and a leading
/// section number (`12. Acceptance map`, `§4.2 Amendments`).
fn amendment_heading(heading: &str, name: &str) -> bool {
    let h = without_number(heading.trim()).to_lowercase();
    let n = name.trim().to_lowercase();
    match h.strip_prefix(&n) {
        Some(rest) => rest.chars().next().is_none_or(|c| !c.is_alphanumeric()),
        None => false,
    }
}

/// Removes a leading section number and the space after it: `12. `, `4.2 `,
/// `§3 `, `7) `.
fn without_number(heading: &str) -> &str {
    let rest = heading.strip_prefix('§').unwrap_or(heading).trim_start();
    let digits = rest
        .bytes()
        .take_while(|b| b.is_ascii_digit() || *b == b'.')
        .count();
    if digits == 0 || !rest.as_bytes()[0].is_ascii_digit() {
        return heading;
    }
    let after = rest[digits..].strip_prefix(')').unwrap_or(&rest[digits..]);
    match after.strip_prefix([' ', '\t']) {
        Some(r) => r.trim_start(),
        None => heading,
    }
}

/// Returns the chunks of the amendment sections. An amendment section starts
/// at a heading of level 2 or more that matches a name. It continues to the
/// next heading of the same or a higher level.
pub(crate) fn amendment_chunks(chunks: &[crate::parse::Chunk], names: &[String]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut open: Option<u8> = None;
    for (i, ch) in chunks.iter().enumerate() {
        if open.is_some_and(|level| ch.level <= level) {
            open = None;
        }
        if open.is_none()
            && ch.level >= 2
            && names.iter().any(|n| amendment_heading(&ch.heading, n))
        {
            open = Some(ch.level);
        }
        if open.is_some() {
            out.push(i);
        }
    }
    out
}

pub(crate) fn canonical_ref(r: &str) -> String {
    let r = strip_hash_suffix(r);
    match r.split_once('#') {
        Some((d, k)) => format!("{}#{}", canonical(d), nfc(k)),
        None => canonical(r),
    }
}

/// Removes a `@<hex>` at the end (a hash pin that spec-dna reserves for
/// suspect links).
pub fn strip_hash_suffix(s: &str) -> &str {
    if let Some((head, tail)) = s.rsplit_once('@')
        && (4..=64).contains(&tail.len())
        && tail.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return head;
    }
    s
}

/// Returns the rest of `url` after a `self-urls` prefix. In the prefix, `*` is
/// one whole path segment (such as a branch name).
fn strip_url_prefix<'a>(pattern: &str, url: &'a str) -> Option<&'a str> {
    let mut parts = pattern.split('*');
    let mut rest = url.strip_prefix(parts.next()?)?;
    for lit in parts {
        let end = rest.find(['/', '#', '?']).unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        rest = rest[end..].strip_prefix(lit)?;
    }
    Some(rest)
}

fn has_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(i) if i > 0 => {
            let scheme = &s[..i];
            scheme.as_bytes()[0].is_ascii_alphabetic()
                && scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.')
        }
        _ => false,
    }
}

fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hexval(b[i + 1]), hexval(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hexval(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

/// Joins a relative path to a base directory, and resolves `.` and `..`.
/// Returns `None` if the path is outside the repository.
pub fn join(base: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn corpus_with(config: &str, files: &[(&str, &str)]) -> Corpus {
        let cfg = Config::parse(config).unwrap();
        let mut all: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
        all.push("src/main.rs".into());
        let docs = files
            .iter()
            .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
            .collect();
        Corpus::build(&cfg, all, docs)
    }

    #[test]
    fn legacy_statuses_map_per_root() {
        let c = corpus_with(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nstatus-map = { released = \"active\", signed-off = \"accepted\" }\n",
            &[
                ("docs/a.md", "---\nstatus: released\n---\n# A\n"),
                ("docs/b.md", "---\nstatus: signed-off\n---\n# B\n"),
                (
                    "docs/d.md",
                    "---\ntype: decision\nstatus: signed-off\n---\n# D\n\n## Decision\n\nX.\n",
                ),
                ("docs/e.md", "---\nstatus: frozen\n---\n# E\n"),
            ],
        );
        let issues = |i: usize| -> Vec<String> {
            c.docs[i].issues.iter().map(|x| x.message.clone()).collect()
        };
        assert_eq!(c.docs[0].status.as_deref(), Some("active"));
        assert!(issues(0).is_empty(), "{:?}", issues(0));
        // `signed-off` maps to `accepted`, which a spec does not allow.
        assert_eq!(
            issues(1),
            [
                "`signed-off` maps to `accepted`, which is not a status of type `spec` (draft, active, deprecated, superseded, retired)"
            ]
        );
        assert_eq!(c.docs[2].status.as_deref(), Some("accepted"));
        assert!(issues(2).is_empty(), "{:?}", issues(2));
        assert_eq!(
            issues(3),
            [
                "`frozen` is not a status of type `spec` (draft, active, deprecated, superseded, retired)"
            ]
        );
    }

    #[test]
    fn required_sections_per_root() {
        let text = "---\ntype: decision\nstatus: accepted\n---\n# D\n\n## Context\n\nX.\n\n## LÖSUNG\n\nY.\n";
        let c = corpus_with(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\nrequired-sections = { decision = [\"Lösung\"] }\n[[roots]]\npath = \"adr\"\n",
            &[("docs/d.md", text), ("adr/d.md", text)],
        );
        assert_eq!(c.docs[0].outcome, [2]);
        assert!(c.docs[1].outcome.is_empty());
    }

    #[test]
    fn self_urls_resolve_inside_the_repository() {
        let c = corpus_with(
            "spec-dna = \"0.1.0\"\nself-urls = [\"https://example.org/acme/specs/blob/*/\", \"https://example.org/acme/specs/tree/*/\"]\n[[roots]]\npath = \"docs\"\n",
            &[
                ("docs/x/README.md", "# X\n"),
                ("docs/z.md", "# Z\n\n## Part\n"),
            ],
        );
        assert_eq!(
            c.resolve_link(
                "docs/x/README.md",
                "https://example.org/acme/specs/blob/main/docs/z.md#part"
            ),
            LinkRes::File("docs/z.md".into(), Some(1), Some("part".into()))
        );
        assert_eq!(
            c.resolve_link(
                "docs/z.md",
                "https://example.org/acme/specs/tree/v1.2/docs/x/"
            ),
            LinkRes::Dir(Some(0))
        );
        assert_eq!(
            c.resolve_link("docs/z.md", "../docs/x"),
            LinkRes::Dir(Some(0))
        );
        // The file is not at HEAD, but it can be on the named branch. Do not
        // check it.
        assert_eq!(
            c.resolve_link(
                "docs/z.md",
                "https://example.org/acme/specs/blob/old/docs/gone.md"
            ),
            LinkRes::External
        );
        assert_eq!(
            c.resolve_link(
                "docs/z.md",
                "https://example.org/acme/other/blob/main/docs/z.md"
            ),
            LinkRes::External
        );
        assert_eq!(
            strip_url_prefix("https://h/o/r/blob/*/", "https://h/o/r/blob/main/a/b.md"),
            Some("a/b.md")
        );
        assert_eq!(
            strip_url_prefix("https://h/o/r/blob/*/", "https://h/o/r/blob//a.md"),
            None
        );
        assert_eq!(
            strip_url_prefix("https://h/o/r/blob/*/", "https://h/o/r/blob/main"),
            None
        );
    }

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        let cfg = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-ids = [\"ADR-{n}\"]\nlegacy-keys = [\"R-{n}\"]\n",
        )
        .unwrap();
        let mut all: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
        all.push("src/main.rs".into());
        let docs = files
            .iter()
            .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
            .collect();
        Corpus::build(&cfg, all, docs)
    }

    #[test]
    fn ids_from_paths() {
        assert_eq!(doc_id("auth/session.md"), "auth/session");
        assert_eq!(doc_id("auth/README.md"), "auth");
        assert_eq!(doc_id("README.md"), "README");
        let cfg = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"a/agents\"\n[[roots]]\npath = \".\"\n",
        )
        .unwrap();
        assert_eq!(root_doc_id(&cfg.roots[0], "a/agents/README.md"), "agents");
        assert_eq!(root_doc_id(&cfg.roots[0], "a/agents/x/README.md"), "x");
        assert_eq!(root_doc_id(&cfg.roots[1], "README.md"), "README");
        assert_eq!(canonical("ADR-012"), "ADR-12");
        assert_eq!(canonical("auth/session"), "auth/session");
    }

    #[test]
    fn resolves_ids_aliases_and_fragments() {
        let c = corpus(&[
            (
                "docs/auth/session.md",
                "---\naliases: [auth/old]\n---\n# S\n\n## Limits\n\n- **ttl**: The gateway SHALL expire tokens. <!-- aliases: auth/legacy#expiry -->\n- **R-7**: The client MUST retry.\n",
            ),
            (
                "docs/decisions/ADR-012-cache.md",
                "---\ntype: decision\nstatus: accepted\n---\n# Cache\n\n## Decision Outcome\n\nX.\n",
            ),
        ]);
        assert_eq!(c.resolve("auth/session"), Res::Found(0, None, true));
        assert_eq!(
            c.resolve("auth/old#ttl@3f2a9c1b"),
            Res::Found(0, Some("ttl".into()), true)
        );
        assert_eq!(
            c.resolve("auth/session#limits"),
            Res::Found(0, Some("limits".into()), true)
        );
        assert_eq!(
            c.resolve("auth/session#R-7"),
            Res::Found(0, Some("R-7".into()), true)
        );
        assert_eq!(
            c.resolve("auth/session#nope"),
            Res::Found(0, Some("nope".into()), false)
        );
        assert_eq!(
            c.resolve("auth/legacy#expiry"),
            Res::Found(0, Some("ttl".into()), true)
        );
        assert_eq!(c.resolve("ADR-12"), Res::Found(1, None, true));
        assert_eq!(c.resolve("ADR-0012"), Res::Found(1, None, true));
        assert_eq!(c.resolve("nope/x"), Res::Missing);
        assert_eq!(c.resolve("other:auth/x"), Res::CrossRepo("other".into()));
    }

    #[test]
    fn resolves_links() {
        let c = corpus(&[("docs/a/x.md", "# X\n"), ("docs/b.md", "# B\n")]);
        assert_eq!(
            c.resolve_link("docs/a/x.md", "../b.md#b"),
            LinkRes::File("docs/b.md".into(), Some(1), Some("b".into()))
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "/src/main.rs"),
            LinkRes::File("src/main.rs".into(), None, None)
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "../../src"),
            LinkRes::Dir(None)
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "https://x.org/a"),
            LinkRes::External
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "mailto:a@b.c"),
            LinkRes::External
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "#top"),
            LinkRes::SelfAnchor("top".into())
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "y%20z.md"),
            LinkRes::Missing("docs/a/y z.md".into())
        );
        assert_eq!(
            c.resolve_link("docs/a/x.md", "../../../out.md"),
            LinkRes::Missing("../../../out.md".into())
        );
    }

    #[test]
    fn slug_key_without_colon_warns() {
        let c = corpus(&[(
            "docs/a.md",
            "# A\n\n- **member-registration** [1.1]: The desk SHALL check an identity document.\n- **note** this list item is prose.\n- **ok-key**: The system MUST run.\n",
        )]);
        let d = &c.docs[0];
        assert_eq!(d.reqs.len(), 1);
        let w: Vec<&Issue> = d
            .issues
            .iter()
            .filter(|i| i.code == Code::RequirementColonMissing)
            .collect();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].target, "member-registration");
        assert_eq!(w[0].line, 3);
    }

    #[test]
    fn legacy_keys_need_no_colon() {
        let c = corpus(&[
            (
                "docs/s.md",
                "# S\n\n## Intro\n\n- **Goal** a bold label in prose\n- **Step.** a numbered label in prose\n\n## Rules\n\n- **R-1** The client MUST retry.\n- **R-2:** The server SHALL log.\n- **R-3.** The cache MAY drop entries.\n- **ok-key**: The system MUST run.\n- **other** no colon and not a legacy key\n",
            ),
            (
                "docs/table.md",
                "# T\n\n| Key | Rule |\n| --- | --- |\n| R-1 | The client MUST retry. |\n| R-2 | The server SHALL log. |\n",
            ),
            (
                "docs/notes/n.md",
                "---\ntype: note\n---\n# N\n\nSee R-1 and R-2.\n",
            ),
        ]);
        let s = &c.docs[0];
        let keys: Vec<&str> = s
            .reqs
            .iter()
            .map(|&r| s.parsed.reqs[r].key.as_str())
            .collect();
        assert_eq!(keys, ["R-1", "R-2", "R-3", "ok-key"]);
        // A traceability list repeats legacy keys without a colon: no error.
        let c2 = corpus(&[(
            "docs/t.md",
            "# T\n\n- **R-1** The client MUST retry.\n\n## Tests\n\n- **R-1** covered by test_retry\n- **ok**: The system MUST run.\n- **ok**: The system MUST run again.\n",
        )]);
        let codes: Vec<Code> = c2.docs[0].issues.iter().map(|i| i.code).collect();
        assert_eq!(codes, [Code::KeyDuplicate]);
        assert_eq!(c2.docs[0].reqs.len(), 2);
        assert!(s.issues.is_empty(), "{:?}", s.issues);
        assert!(!s.has_req_in(1) && s.has_req_in(2));
        // A spec with legacy-key tokens and no recognized requirement gets a
        // warning. A note does not.
        let t: Vec<(Code, u32)> = c.docs[1].issues.iter().map(|i| (i.code, i.line)).collect();
        assert_eq!(t, [(Code::RequirementsUnrecognized, 5)]);
        assert!(c.docs[1].issues[0].message.starts_with("2 tokens match"));
        assert!(c.docs[2].issues.is_empty());
    }

    #[test]
    fn partial_recognition_warns() {
        let c = corpus(&[(
            "docs/s.md",
            "# S\n\n- **R-1** The client MUST retry.\n- R-2 The server SHALL log.\n\n| Key | Rule |\n| --- | --- |\n| R-3 | The cache MAY drop entries. |\n\n## Tests\n\n| R-1 | test_retry |\n| --- | --- |\n| R-1 | test_retry_twice |\n",
        )]);
        let d = &c.docs[0];
        assert_eq!(d.reqs.len(), 1);
        let t: Vec<(Code, u32, &str)> = d
            .issues
            .iter()
            .map(|i| (i.code, i.line, i.message.as_str()))
            .collect();
        // A traceability row of a recognized key does not count.
        assert_eq!(
            t,
            [(
                Code::RequirementsUnrecognized,
                4,
                "2 list items or table rows start with a `legacy-keys` token but are not recognized requirements (first: `R-2`)"
            )]
        );
    }

    #[test]
    fn amendment_sections_per_root() {
        let base = "---\ntype: decision\nstatus: accepted\n---\n# D\n\n## Lösung\n\nUse X.\n\n### Nachtrag 2: scope\n\nAlso Y.\n\n## Nachtrag 1: TTL\n\nThe TTL is 5 min.\n\n#### Detail\n\nZ.\n\n## Folgen\n\nW.\n\n## Nachtragsplan\n\nV.\n";
        let cfg = "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\nrequired-sections = { decision = [\"Lösung\"] }\namendment-sections = { decision = [\"Nachtrag\"] }\n[[roots]]\npath = \"adr\"\n";
        let c = corpus_with(cfg, &[("docs/d.md", base), ("adr/d.md", base)]);
        let d = &c.docs[0];
        let headings: Vec<&str> = d
            .amendments
            .iter()
            .map(|&i| d.parsed.chunks[i].heading.as_str())
            .collect();
        // `Nachtragsplan` starts with the name, but a letter follows it.
        assert_eq!(headings, ["Nachtrag 2: scope", "Nachtrag 1: TTL", "Detail"]);
        // A leading section number does not count.
        assert!(amendment_heading(
            "12. Acceptance map (requirement -> test)",
            "Acceptance map"
        ));
        assert!(amendment_heading("§4.2 Amendments", "amendments"));
        assert!(amendment_heading("7) Amendments", "Amendments"));
        assert!(!amendment_heading("12.Acceptance map", "Acceptance map"));
        // The default name is `Amendments`.
        assert!(c.docs[1].amendments.is_empty());
        let frozen = |text: &str| corpus_with(cfg, &[("docs/d.md", text)]).docs[0].frozen_hash;
        // Amendment text can change and grow. The other text is frozen.
        assert_eq!(frozen(base), frozen(&base.replace("5 min", "10 min")));
        assert_eq!(
            frozen(base),
            frozen(&format!("{base}\n## Nachtrag 3\n\nMore.\n"))
        );
        assert_ne!(frozen(base), frozen(&base.replace("W.", "W2.")));
        assert_ne!(frozen(base), frozen(&base.replace("Use X.", "Use X2.")));
    }

    #[test]
    fn live_sections_are_not_frozen() {
        let base = "---\ntype: decision\nstatus: accepted\n---\n# D\n\n## Решение\n\nUse X.\n\n## Статус реализации\n\n- step 1: done\n- step 2: open\n\n### Детали\n\nZ.\n\n## Последствия\n\nW.\n\n## Поправки\n\nA.\n";
        let cfg = "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\nrequired-sections = { decision = [\"Решение\"] }\namendment-sections = { decision = [\"Поправки\"] }\nlive-sections = { decision = [\"Статус реализации\"] }\n";
        let c = corpus_with(cfg, &[("docs/d.md", base)]);
        let names = &Config::parse(cfg).unwrap().roots[0].live_sections[DocType::Decision as usize];
        let live = amendment_chunks(&c.docs[0].parsed.chunks, names);
        let headings: Vec<&str> = live
            .iter()
            .map(|&i| c.docs[0].parsed.chunks[i].heading.as_str())
            .collect();
        assert_eq!(headings, ["Статус реализации", "Детали"]);
        let frozen = |text: &str| corpus_with(cfg, &[("docs/d.md", text)]).docs[0].frozen_hash;
        // Live text can change in place and grow.
        assert_eq!(
            frozen(base),
            frozen(&base.replace("step 2: open", "step 2: done (2026-11-13)"))
        );
        assert_eq!(frozen(base), frozen(&base.replace("Z.", "Z.\n\nMore.")));
        // The outcome and the other sections stay frozen.
        assert_ne!(frozen(base), frozen(&base.replace("Use X.", "Use X2.")));
        assert_ne!(frozen(base), frozen(&base.replace("W.", "W2.")));
        // A renamed live heading is frozen text again.
        assert_ne!(
            frozen(base),
            frozen(&base.replace("## Статус реализации", "## Статус"))
        );
    }

    #[test]
    fn requirement_keys() {
        let c = corpus(&[(
            "docs/s.md",
            "# S\n\n- **Bad_Key**: The system SHALL work.\n- **Goal**: just a label\n- **ok-key**: The system MUST run.\n- **ok-key**: The system MUST run twice.\n",
        )]);
        let d = &c.docs[0];
        let codes: Vec<Code> = d.issues.iter().map(|i| i.code).collect();
        assert_eq!(codes, vec![Code::KeyFormat, Code::KeyDuplicate]);
        assert_eq!(d.reqs.len(), 1);
    }
}
