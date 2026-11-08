//! Code citations (ADR 0001, amendment "code citations"). A document cites
//! code with a link to a file, a path in an inline code span or in prose, a
//! `path:line`, or an identifier in an inline code span. This module resolves
//! these citations against a tree. `context <code path>` uses them to find the
//! sections that cite a file, and `check` uses them to find drift.

use crate::model::{Corpus, DocType, LinkRes, join};
use crate::parse::RefKind;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// One citation: the document, its section (chunk) and the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cite {
    pub doc: u32,
    pub chunk: usize,
    pub line: u32,
}

/// The citations of all documents of a tree, by cited file, directory and
/// identifier. A citation of a document is a link between documents, not a
/// code citation, so it is not here.
#[derive(Default)]
pub struct Citations {
    pub files: HashMap<String, Vec<Cite>>,
    pub dirs: HashMap<String, Vec<Cite>>,
    pub idents: HashMap<String, Vec<Cite>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    File(String),
    Dir(String),
}

/// The paths of a tree, with an index of path tails. A tail is the end of a
/// path after a `/`, such as `loans.py` or `library/loans.py`.
pub struct Tree<'a> {
    pub files: &'a HashSet<String>,
    pub dirs: &'a HashSet<String>,
    /// The file of each tail, or `None` if several files have the tail.
    tails: HashMap<&'a str, Option<&'a str>>,
}

impl<'a> Tree<'a> {
    pub fn new(files: &'a HashSet<String>, dirs: &'a HashSet<String>) -> Tree<'a> {
        let mut tails: HashMap<&str, Option<&str>> = HashMap::new();
        for f in files {
            let mut rest = f.as_str();
            while let Some((_, t)) = rest.split_once('/') {
                tails
                    .entry(t)
                    .and_modify(|v| *v = None)
                    .or_insert(Some(f.as_str()));
                rest = t;
            }
        }
        Tree { files, dirs, tails }
    }

    /// Resolves a cited path. The order is: from the repository root, from
    /// the directory of the citing document, and then a file name or a path
    /// tail that only one file of the tree has.
    pub fn resolve(&self, doc_path: &str, raw: &str) -> Option<Target> {
        let s = strip_location(raw);
        if s.is_empty() || s.contains("://") || s.starts_with("--") {
            return None;
        }
        let rel = s.trim_start_matches("./");
        let mut cands = vec![rel.trim_start_matches('/').to_string()];
        if !s.starts_with('/') {
            let dir = doc_path.rsplit_once('/').map_or("", |(d, _)| d);
            cands.extend(join(dir, rel));
        }
        if let Some(p) = cands.iter().find(|p| self.files.contains(p.as_str())) {
            return Some(Target::File(p.clone()));
        }
        for p in &cands {
            let d = p.trim_end_matches('/');
            if !d.is_empty() && self.dirs.contains(d) {
                return Some(Target::Dir(d.to_string()));
            }
        }
        let last = rel.rsplit('/').next().unwrap_or("");
        if last.contains('.')
            && !rel.ends_with('/')
            && let Some(Some(f)) = self.tails.get(rel.trim_start_matches('/'))
        {
            return Some(Target::File(f.to_string()));
        }
        None
    }
}

/// Removes a location suffix from a path: `:12`, `:12-20`, `:12:5`, `#L12` or
/// `#L12-L20`.
pub fn strip_location(s: &str) -> &str {
    let mut s = s;
    if let Some(i) = s.rfind("#L")
        && i + 2 < s.len()
        && s[i + 2..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'L')
    {
        s = &s[..i];
    }
    while let Some(i) = s.rfind(':') {
        if i > 0 && i + 1 < s.len() && s[i + 1..].bytes().all(|b| b.is_ascii_digit() || b == b'-') {
            s = &s[..i];
        } else {
            break;
        }
    }
    s
}

/// Returns the distinctive identifiers of an inline code span. The span is a
/// name, a qualified name (`a::b`, `a.b`, `a->b`, `a#b`) or a call (`f()`,
/// `f(x, y)`). An identifier is distinctive if it has 3 characters or more,
/// and one of these:
///
/// - an inner `_` (`max_loans`, `MAX_LOANS`);
/// - a lowercase letter before an uppercase letter (`loanBook`, `LoanBook`);
/// - it is the called name of a call, with 4 characters or more (`renew()`).
///
/// Thus words that are common in prose, such as `status` or `HTTP`, are not
/// identifier citations.
pub fn idents(span: &str) -> Vec<String> {
    let called = span.contains('(');
    let head = span.split('(').next().unwrap_or("");
    let head = head.split('<').next().unwrap_or("");
    let head = head.trim_end_matches(['!', '?', ';']);
    if head.is_empty() || head.contains(['/', ' ', '\t']) {
        return Vec::new();
    }
    let segs: Vec<&str> = head
        .split("::")
        .flat_map(|s| s.split("->"))
        .flat_map(|s| s.split(['.', '#']))
        .collect();
    if !segs.iter().all(|s| is_ident(s)) {
        return Vec::new();
    }
    let last = segs.len() - 1;
    segs.iter()
        .enumerate()
        .filter(|(i, s)| distinctive(s, called && *i == last))
        .map(|(_, s)| s.to_string())
        .collect()
}

fn is_ident(s: &str) -> bool {
    let mut b = s.bytes();
    matches!(b.next(), Some(c) if c.is_ascii_alphabetic() || c == b'_' || c == b'$')
        && b.all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
}

/// Names that look distinctive but occur in prose and in code of every kind:
/// common call names, and product names in mixed case.
const COMMON: &[&str] = &[
    "append",
    "apply",
    "assert",
    "build",
    "call",
    "catch",
    "check",
    "clear",
    "close",
    "connect",
    "create",
    "debug",
    "delete",
    "emit",
    "error",
    "exec",
    "execute",
    "fetch",
    "filter",
    "find",
    "flush",
    "format",
    "handle",
    "info",
    "init",
    "insert",
    "join",
    "length",
    "list",
    "load",
    "main",
    "next",
    "open",
    "parse",
    "post",
    "print",
    "push",
    "read",
    "reduce",
    "remove",
    "render",
    "reset",
    "resolve",
    "save",
    "send",
    "setup",
    "size",
    "sleep",
    "sort",
    "spawn",
    "split",
    "start",
    "stop",
    "test",
    "then",
    "update",
    "wait",
    "warn",
    "write",
    "toString",
    "valueOf",
    "ChatGPT",
    "DevOps",
    "GitHub",
    "GitLab",
    "GraphQL",
    "IntelliJ",
    "JavaScript",
    "JetBrains",
    "LinkedIn",
    "MongoDB",
    "MySQL",
    "OAuth",
    "OpenAI",
    "OpenAPI",
    "OpenID",
    "PayPal",
    "PostgreSQL",
    "PowerShell",
    "TypeScript",
    "VSCode",
    "WebAssembly",
    "WebSocket",
    "WordPress",
    "YouTube",
    "iOS",
    "iPad",
    "iPadOS",
    "iPhone",
    "macOS",
    "watchOS",
];

fn distinctive(s: &str, called: bool) -> bool {
    let b = s.as_bytes();
    if b.len() < 3 || !b.iter().any(u8::is_ascii_alphabetic) || COMMON.contains(&s) {
        return false;
    }
    let inner_underscore = s.trim_matches('_').contains('_');
    let hump = b
        .windows(2)
        .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase());
    inner_underscore || hump || (called && b.len() >= 4)
}

/// Returns the identifier tokens of a source text: runs of ASCII letters,
/// digits, `_` and `$`.
pub fn tokens(text: &str) -> HashSet<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .filter(|t| t.len() >= 3)
        .collect()
}

/// The largest source file that spec-dna reads for identifiers: 4 MiB.
pub const MAX_SOURCE: u64 = 4 << 20;

/// Reads a source file of the working tree for identifier matches. Returns
/// `None` for a file that is missing, larger than `MAX_SOURCE`, or binary (a
/// NUL byte in the first 8 KiB).
pub fn read_source(root: &Path, path: &str) -> Option<String> {
    let p = root.join(path);
    let meta = std::fs::metadata(&p).ok()?;
    if !meta.is_file() || meta.len() > MAX_SOURCE {
        return None;
    }
    let bytes = std::fs::read(p).ok()?;
    source_text(&bytes)
}

/// Returns the text of source bytes, or `None` for binary content.
pub fn source_text(bytes: &[u8]) -> Option<String> {
    if bytes.len() as u64 > MAX_SOURCE || bytes[..bytes.len().min(8192)].contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Returns the chunk that contains a line: the last chunk that starts on or
/// before the line.
fn chunk_of(c: &Corpus, doc: u32, line: u32) -> usize {
    let chunks = &c.docs[doc as usize].parsed.chunks;
    chunks.iter().rposition(|ch| ch.line <= line).unwrap_or(0)
}

impl Citations {
    /// Collects the code citations of every document, resolved against the
    /// tree of the corpus.
    pub fn build(c: &Corpus) -> Citations {
        let tree = Tree::new(&c.files, &c.dirs);
        let mut out = Citations::default();
        for (i, d) in c.docs.iter().enumerate() {
            let i = i as u32;
            let p = &d.parsed;
            if p.chunks.is_empty() {
                continue;
            }
            let mut add = |kind: u8, key: String, line: u32| {
                let map = match kind {
                    0 => &mut out.files,
                    1 => &mut out.dirs,
                    _ => &mut out.idents,
                };
                map.entry(key).or_default().push(Cite {
                    doc: i,
                    chunk: chunk_of(c, i, line),
                    line,
                });
            };
            for t in &p.code_spans {
                match tree.resolve(&d.path, &t.text) {
                    Some(Target::File(f)) if !c.by_path.contains_key(&f) => add(0, f, t.line),
                    Some(Target::File(_)) => {}
                    Some(Target::Dir(dir)) => add(1, dir, t.line),
                    None => {
                        for id in idents(&t.text) {
                            add(2, id, t.line);
                        }
                    }
                }
            }
            for t in &p.path_words {
                match tree.resolve(&d.path, &t.text) {
                    Some(Target::File(f)) if !c.by_path.contains_key(&f) => add(0, f, t.line),
                    Some(Target::Dir(dir)) => add(1, dir, t.line),
                    _ => {}
                }
            }
            for r in &p.refs {
                if r.kind != RefKind::Wiki
                    && let LinkRes::File(f, None, _) = c.resolve_link(&d.path, &r.target)
                {
                    add(0, f, r.line);
                }
            }
        }
        out
    }
}

/// The strength of a citation, best first.
pub const BY_FILE: u8 = 0;
pub const BY_IDENT: u8 = 1;
pub const BY_DIR: u8 = 2;

/// Returns the sections that cite the given paths, best first, grouped by
/// document, with the best class of each document (`BY_FILE`, `BY_IDENT` or
/// `BY_DIR`). A section scores 1000 for each citation of the file itself, 10
/// for each distinct cited identifier that occurs in the file, and 1 plus the
/// depth for each citation of a directory that contains the file. The order
/// of the documents is:
///
/// - documents that cite a file, then an identifier, then a directory;
/// - specs and contracts, guides, decisions, changes, notes;
/// - the higher total score;
/// - the document id.
pub fn citing(
    c: &Corpus,
    cits: &Citations,
    root: &Path,
    paths: &[String],
) -> Vec<(u32, u8, Vec<usize>)> {
    #[derive(Default)]
    struct Score {
        file: u32,
        dir: u32,
        idents: HashSet<String>,
    }
    let mut scores: HashMap<(u32, usize), Score> = HashMap::new();
    for p in paths {
        let p = p.trim_start_matches("./");
        for ct in cits.files.get(p).into_iter().flatten() {
            scores.entry((ct.doc, ct.chunk)).or_default().file += 1000;
        }
        let mut a = p;
        while let Some((d, _)) = a.rsplit_once('/') {
            for ct in cits.dirs.get(d).into_iter().flatten() {
                scores.entry((ct.doc, ct.chunk)).or_default().dir +=
                    1 + d.matches('/').count() as u32;
            }
            a = d;
        }
        if let Some(text) = read_source(root, p) {
            let toks = tokens(&text);
            for (id, v) in &cits.idents {
                if toks.contains(id.as_str()) {
                    for ct in v {
                        scores
                            .entry((ct.doc, ct.chunk))
                            .or_default()
                            .idents
                            .insert(id.clone());
                    }
                }
            }
        }
    }
    let class = |s: &Score| -> u8 {
        if s.file > 0 {
            BY_FILE
        } else if !s.idents.is_empty() {
            BY_IDENT
        } else {
            BY_DIR
        }
    };
    let points = |s: &Score| s.file + 10 * s.idents.len() as u32 + s.dir;
    let mut by_doc: HashMap<u32, Vec<(usize, u8, u32)>> = HashMap::new();
    for ((d, ch), s) in &scores {
        by_doc
            .entry(*d)
            .or_default()
            .push((*ch, class(s), points(s)));
    }
    let type_rank = |t: DocType| match t {
        DocType::Spec | DocType::Contract => 0,
        DocType::Guide => 1,
        DocType::Decision => 2,
        DocType::Change => 3,
        DocType::Note => 4,
    };
    let mut docs: Vec<(u32, u8, u32, Vec<usize>)> = by_doc
        .into_iter()
        .map(|(d, mut v)| {
            v.sort_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
            let best = v.iter().map(|x| x.1).min().unwrap_or(BY_DIR);
            let total = v.iter().map(|x| x.2).sum();
            (d, best, total, v.into_iter().map(|x| x.0).collect())
        })
        .collect();
    docs.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then(type_rank(c.docs[a.0 as usize].ty).cmp(&type_rank(c.docs[b.0 as usize].ty)))
            .then(b.2.cmp(&a.2))
            .then(c.docs[a.0 as usize].id.cmp(&c.docs[b.0 as usize].id))
    });
    docs.into_iter().map(|(d, c, _, v)| (d, c, v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_locations() {
        assert_eq!(strip_location("src/a.rs:12"), "src/a.rs");
        assert_eq!(strip_location("src/a.rs:12-20"), "src/a.rs");
        assert_eq!(strip_location("src/a.rs:12:5"), "src/a.rs");
        assert_eq!(strip_location("src/a.rs#L3-L9"), "src/a.rs");
        assert_eq!(strip_location("a::b"), "a::b");
        assert_eq!(strip_location("x:"), "x:");
    }

    #[test]
    fn distinctive_identifiers_only() {
        assert_eq!(idents("checkout_book"), ["checkout_book"]);
        assert_eq!(idents("LoanService::renew()"), ["LoanService", "renew"]);
        assert_eq!(idents("loanBook(member, book)"), ["loanBook"]);
        assert_eq!(idents("self.max_loans"), ["max_loans"]);
        assert_eq!(idents("MAX_LOANS"), ["MAX_LOANS"]);
        assert_eq!(idents("Vec<String>"), Vec::<String>::new());
        for plain in [
            "status", "HTTP", "Loan", "get()", "--budget", "a/b", "v0.1.0", "x y", "read()",
            "GitHub", "macOS",
        ] {
            assert!(idents(plain).is_empty(), "{plain}");
        }
    }

    #[test]
    fn resolves_paths_from_root_document_and_unique_tail() {
        let files: HashSet<String> = ["src/lib/loans.py", "src/lib/books.py", "a/x.py", "b/x.py"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let dirs: HashSet<String> = ["src", "src/lib", "a", "b"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let t = Tree::new(&files, &dirs);
        let file = |p: &str| Some(Target::File(p.to_string()));
        assert_eq!(
            t.resolve("docs/a.md", "src/lib/loans.py:12"),
            file("src/lib/loans.py")
        );
        assert_eq!(
            t.resolve("docs/a.md", "./src/lib/loans.py"),
            file("src/lib/loans.py")
        );
        assert_eq!(
            t.resolve("src/lib/a.md", "books.py"),
            file("src/lib/books.py")
        );
        assert_eq!(
            t.resolve("docs/a.md", "lib/loans.py"),
            file("src/lib/loans.py")
        );
        assert_eq!(t.resolve("docs/a.md", "loans.py"), file("src/lib/loans.py"));
        assert_eq!(
            t.resolve("docs/a.md", "x.py"),
            None,
            "2 files have this name"
        );
        assert_eq!(
            t.resolve("docs/a.md", "src/lib/"),
            Some(Target::Dir("src/lib".into()))
        );
        assert_eq!(t.resolve("docs/a.md", "https://x.org/src/lib"), None);
    }
}
