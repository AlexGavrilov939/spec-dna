//! The information model of ADR 0001 over one tree: documents with ids,
//! types, statuses and aliases, and the resolution of references.

use crate::config::Config;
use crate::diag::Code;
use crate::frontmatter::Value;
use crate::hash::nfc;
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
}

impl DocType {
    pub const ALL: [DocType; 6] = [
        DocType::Spec,
        DocType::Contract,
        DocType::Guide,
        DocType::Decision,
        DocType::Change,
        DocType::Note,
    ];

    pub fn name(self) -> &'static str {
        ["spec", "contract", "guide", "decision", "change", "note"][self as usize]
    }

    pub fn from_name(s: &str) -> Option<DocType> {
        DocType::ALL.into_iter().find(|t| t.name() == s)
    }

    pub fn statuses(self) -> &'static [&'static str] {
        match self {
            DocType::Spec | DocType::Contract | DocType::Guide => {
                &["draft", "active", "deprecated", "superseded", "retired"]
            }
            DocType::Decision => &["proposed", "accepted", "superseded", "rejected"],
            DocType::Change => &["proposed", "accepted", "done", "rejected"],
            DocType::Note => &[],
        }
    }

    pub fn has_requirements(self) -> bool {
        matches!(self, DocType::Spec | DocType::Contract)
    }

    pub fn is_normative(self) -> bool {
        matches!(self, DocType::Spec | DocType::Contract | DocType::Guide)
    }
}

/// A problem found while reading a document's metadata.
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
    /// For a decision, the chunks whose heading is one of the root's
    /// decision sections.
    pub outcome: Vec<usize>,
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

/// The document id for a path under a root (ADR 0001, "Identifiers").
pub fn doc_id(rel: &str) -> String {
    let stem = rel.rsplit_once('.').map_or(rel, |(s, _)| s);
    if let Some((dir, last)) = stem.rsplit_once('/')
        && (last.eq_ignore_ascii_case("readme") || last.eq_ignore_ascii_case("index"))
    {
        return dir.to_string();
    }
    stem.to_string()
}

/// The id of a document under a root. A `README.md` or `index.md` takes the
/// name of its directory, so one directly in a root takes the root's name.
pub fn root_doc_id(root: &crate::config::Root, path: &str) -> String {
    let rel = root.rel(path);
    let id = doc_id(rel);
    if !id.contains('/')
        && (id.eq_ignore_ascii_case("readme") || id.eq_ignore_ascii_case("index"))
        && let Some(name) = root.path.rsplit('/').next().filter(|n| !n.is_empty())
    {
        return name.to_string();
    }
    id
}

pub fn is_doc_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

/// Canonical lookup form: NFC, and `PREFIX-0012` becomes `PREFIX-12`.
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
    pub by_path: HashMap<String, u32>,
    pub files: HashSet<String>,
    pub dirs: HashSet<String>,
    pub req_aliases: HashMap<String, Vec<(u32, usize)>>,
    pub token_prefixes: Vec<String>,
    pub repos: Vec<String>,
    pub self_urls: Vec<String>,
}

impl Corpus {
    /// `files` is every path in the tree; `docs` the document paths with
    /// their parsed data (already filtered by roots and excludes).
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
        let mut by_path = HashMap::new();
        let mut req_aliases: HashMap<String, Vec<(u32, usize)>> = HashMap::new();
        for (i, d) in docs.iter().enumerate() {
            let i = i as u32;
            by_path.insert(d.path.clone(), i);
            let mut names: Vec<String> = vec![canonical(&d.id)];
            names.extend(
                d.aliases
                    .iter()
                    .chain(&d.legacy_aliases)
                    .map(|a| canonical(a)),
            );
            names.sort();
            names.dedup();
            for n in names {
                lookup.entry(n).or_default().push(i);
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
            by_path,
            files: files.into_iter().collect(),
            dirs,
            req_aliases,
            token_prefixes: config.token_prefixes(),
            repos: config.repos.iter().map(|r| r.0.clone()).collect(),
            self_urls: config.self_urls.clone(),
        }
    }

    /// Resolves an id reference: `id`, `id#fragment`, either with `@hash`.
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
        match self.lookup.get(&canonical(idpart)).map(Vec::as_slice) {
            None | Some([]) => Res::Missing,
            Some([d]) => {
                let ok = frag.is_none_or(|f| self.docs[*d as usize].has_anchor(f));
                Res::Found(*d, frag.map(str::to_string), ok)
            }
            Some(many) => {
                // A document's own id wins over another document's alias; the
                // clash itself is reported as a duplicate id.
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
            // It may name another branch or commit; leave it unchecked.
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

    let mut ty = root.default_type;
    if let Some((t, line)) = fm_type {
        match DocType::from_name(&t) {
            None => issues.push(Issue {
                code: Code::TypeInvalid,
                line,
                target: t.clone(),
                message: format!(
                    "unknown type `{t}`; use one of spec, contract, guide, decision, change, note"
                ),
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
    let status = match fm_status {
        Some((s, line)) => {
            let allowed = |x: &str| ty == DocType::Note || ty.statuses().contains(&x);
            // A legacy word the root maps stands for its status; the file keeps it.
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
    if status.is_none() && ty != DocType::Note {
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
            let valid = is_slug(&r.key) || root.legacy_keys.iter().any(|p| p.is_match(&r.key));
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
    }
    let outcome = if ty == DocType::Decision {
        parsed
            .chunks
            .iter()
            .enumerate()
            .filter(|(_, ch)| {
                root.decision_sections
                    .iter()
                    .any(|s| same_heading(&ch.heading, s))
            })
            .map(|(i, _)| i)
            .collect()
    } else {
        Vec::new()
    };
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
    }
}

/// Headings compare without case (any script) and surrounding space.
pub fn same_heading(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn canonical_ref(r: &str) -> String {
    let r = strip_hash_suffix(r);
    match r.split_once('#') {
        Some((d, k)) => format!("{}#{}", canonical(d), nfc(k)),
        None => canonical(r),
    }
}

/// Removes a trailing `@<hex>` (a hash pin reserved for suspect links).
pub fn strip_hash_suffix(s: &str) -> &str {
    if let Some((head, tail)) = s.rsplit_once('@')
        && (4..=64).contains(&tail.len())
        && tail.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return head;
    }
    s
}

/// The rest of `url` after a `self-urls` prefix, where `*` is one whole path
/// segment (such as a branch name).
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

/// Joins a relative path to a base directory, resolving `.` and `..`.
/// Returns `None` when the path leaves the repository.
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
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nstatus-map = { implemented = \"active\", approved = \"accepted\" }\n",
            &[
                ("docs/a.md", "---\nstatus: implemented\n---\n# A\n"),
                ("docs/b.md", "---\nstatus: approved\n---\n# B\n"),
                (
                    "docs/d.md",
                    "---\ntype: decision\nstatus: approved\n---\n# D\n",
                ),
                ("docs/e.md", "---\nstatus: shipped\n---\n# E\n"),
            ],
        );
        let issues = |i: usize| -> Vec<String> {
            c.docs[i].issues.iter().map(|x| x.message.clone()).collect()
        };
        assert_eq!(c.docs[0].status.as_deref(), Some("active"));
        assert!(issues(0).is_empty(), "{:?}", issues(0));
        // `approved` stands for `accepted`, which a spec does not allow.
        assert_eq!(
            issues(1),
            [
                "`approved` maps to `accepted`, which is not a status of type `spec` (draft, active, deprecated, superseded, retired)"
            ]
        );
        assert_eq!(c.docs[2].status.as_deref(), Some("accepted"));
        assert!(issues(2).is_empty(), "{:?}", issues(2));
        assert_eq!(
            issues(3),
            [
                "`shipped` is not a status of type `spec` (draft, active, deprecated, superseded, retired)"
            ]
        );
    }

    #[test]
    fn decision_sections_per_root() {
        let text = "---\ntype: decision\nstatus: accepted\n---\n# D\n\n## Context\n\nX.\n\n## РЕШЕНИЕ\n\nY.\n";
        let c = corpus_with(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndecision-sections = [\"Решение\"]\n[[roots]]\npath = \"adr\"\n",
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
        // Gone at HEAD, maybe present on the named branch: left unchecked.
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
