//! `spec-dna mv <source>... <dest>` for requirements and sections: a split
//! (ADR 0001, amendment "batch split"). One command moves several
//! requirements, or whole sections, into a new or an existing spec or
//! contract. A source is `<doc-id>#<key>` (a requirement) or
//! `<doc-id>#<anchor>` (a section with its subsections). A new destination
//! gets front matter and a title. Each moved requirement whose id is in the
//! base tree (HEAD) keeps that id as an alias. A key that is new in the
//! change gets no alias, because no reference can use its old id yet. Links
//! to a moved key or section follow it.
//!
//! A preservation check runs before any file changes. It compares the
//! requirement text without link destinations, because the move rewrites
//! links:
//!
//! - the requirement hashes of all documents are the same multiset;
//! - every requirement id resolves, to a requirement with the same hash (a
//!   new key at its new id);
//! - the frozen text of every accepted decision is the same (a link
//!   destination is not frozen text);
//! - no error diagnostic is new.
//!
//! If a check fails, `mv` changes no file and exits with code 1.

use crate::config::Config;
use crate::diag::{Code, Severity};
use crate::hash::Hash;
use crate::model::{Corpus, DocType, Res, is_doc_path, join, same_heading};
use crate::mv::{add_req_alias, dedent, line_span, parent, read, relative, write};
use crate::parse::parse;
use crate::repo::{Fatal, Repo};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Returns the whole lines of a requirement list item, without the blank
/// lines after it. The range of a list item can end after a blank line. A
/// cut or an insert at that end would join the next paragraph to an item as
/// a lazy continuation line, and change its text.
fn item_span(text: &str, start: usize, end: usize) -> (usize, usize) {
    let (a, mut b) = line_span(text, start, end);
    while b > a + 1 && text[..b].ends_with("\n\n") {
        b -= 1;
    }
    (a, b)
}

/// The result of a split.
pub struct Split {
    pub to: String,
    pub to_path: String,
    pub created: bool,
    /// The old global ids of the moved requirements.
    pub moved: Vec<String>,
    /// The old ids that stay as aliases: the moved ids of the base tree.
    pub aliases: Vec<String>,
    pub sections: usize,
    pub files: Vec<String>,
    /// Requirement hashes compared, ids that resolve, accepted decisions
    /// compared.
    pub preserved: (usize, usize, usize),
}

enum Part {
    Req { doc: u32, req: usize },
    Section { doc: u32, first: usize, last: usize },
}

/// Moves the requirements and sections that `sources` name to `dest`.
/// `base` is the corpus of the base tree (HEAD), or `None` if there is no
/// commit. Then every moved id gets an alias.
pub fn run(
    repo: &Repo,
    config: &Config,
    c: &Corpus,
    base: Option<&Corpus>,
    sources: &[String],
    dest: &str,
) -> Result<Split, Fatal> {
    let parts = parse_sources(c, sources)?;
    let first_doc = match parts[0] {
        Part::Req { doc, .. } | Part::Section { doc, .. } => doc,
    };
    let (dest_doc, to_path, to_id) = destination(repo, config, c, first_doc, dest)?;
    let created = dest_doc.is_none();

    // The section ranges of each source document, and the requirements that
    // a moved section already contains.
    let mut in_section: HashSet<(u32, usize)> = HashSet::new();
    for p in &parts {
        if let Part::Section { doc, first, last } = *p {
            let d = &c.docs[doc as usize];
            for &r in &d.reqs {
                let ch = d.parsed.reqs[r].chunk as usize;
                if ch >= first && ch <= last {
                    in_section.insert((doc, r));
                }
            }
        }
    }

    // The moved anchors: (source path, key or heading anchor).
    let mut moved_anchors: HashSet<(String, String)> = HashSet::new();
    let mut moved_ids: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut section_anchors: Vec<String> = Vec::new();
    for p in &parts {
        let doc = match *p {
            Part::Req { doc, .. } | Part::Section { doc, .. } => doc,
        };
        let d = &c.docs[doc as usize];
        if Some(doc) == dest_doc {
            return Err(Fatal::Usage(format!("`{}` is already in `{to_id}`", d.id)));
        }
        let reqs: Vec<usize> = match *p {
            Part::Req { req, .. } => vec![req],
            Part::Section { first, last, .. } => {
                for ch in &d.parsed.chunks[first..=last] {
                    moved_anchors.insert((d.path.clone(), ch.anchor.clone()));
                    section_anchors.push(ch.anchor.clone());
                }
                d.reqs
                    .iter()
                    .copied()
                    .filter(|&r| (first..=last).contains(&(d.parsed.reqs[r].chunk as usize)))
                    .collect()
            }
        };
        for r in reqs {
            let key = &d.parsed.reqs[r].key;
            let id = format!("{}#{key}", d.id);
            if moved_ids.contains(&id) {
                continue;
            }
            moved_anchors.insert((d.path.clone(), key.clone()));
            moved_ids.push(id);
            keys.push(key.clone());
        }
    }
    check_names(c, dest_doc, &keys, &section_anchors, &to_id)?;
    // The moved ids that are new in the change: they get no alias.
    let new_ids: HashSet<String> = moved_ids
        .iter()
        .filter(|id| base.is_some_and(|b| !matches!(b.resolve(id), Res::Found(_, Some(_), true))))
        .cloned()
        .collect();
    let alias = |item: &str, id: String| {
        if new_ids.contains(&id) {
            item.to_string()
        } else {
            add_req_alias(item, &id)
        }
    };

    // Cut the parts out of their source documents.
    let mut texts: HashMap<String, String> = HashMap::new();
    let mut cuts: HashMap<u32, Vec<(usize, usize)>> = HashMap::new();
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut sections: Vec<String> = Vec::new();
    for p in &parts {
        let doc = match *p {
            Part::Req { doc, .. } | Part::Section { doc, .. } => doc,
        };
        let d = &c.docs[doc as usize];
        if !texts.contains_key(&d.path) {
            texts.insert(d.path.clone(), read(repo, &d.path)?);
        }
        let text = &texts[&d.path];
        match *p {
            Part::Req { req, .. } => {
                if in_section.contains(&(doc, req)) {
                    continue;
                }
                let r = &d.parsed.reqs[req];
                let (a, b) = item_span(text, r.start as usize, r.end as usize);
                let item = alias(&dedent(&text[a..b]), format!("{}#{}", d.id, r.key));
                let item = map_links(&item, &|l| {
                    retarget(l, &d.path, &to_path, &moved_anchors, &to_path, c)
                });
                let ch = &d.parsed.chunks[r.chunk as usize];
                let heading = if ch.level <= 1 {
                    "Requirements".to_string()
                } else {
                    ch.heading.clone()
                };
                match groups.iter_mut().find(|g| same_heading(&g.0, &heading)) {
                    Some(g) => g.1.push(item),
                    None => groups.push((heading, vec![item])),
                }
                cuts.entry(doc).or_default().push((a, b));
            }
            Part::Section { first, last, .. } => {
                let chunks = &d.parsed.chunks;
                let (a, b) = (chunks[first].start as usize, chunks[last].end as usize);
                let mut body = text[a..b].to_string();
                // Alias markers, from the last requirement to the first, so
                // that the offsets stay valid.
                let mut reqs: Vec<usize> = d
                    .reqs
                    .iter()
                    .copied()
                    .filter(|&r| (first..=last).contains(&(d.parsed.reqs[r].chunk as usize)))
                    .collect();
                reqs.sort_by_key(|&r| std::cmp::Reverse(d.parsed.reqs[r].start));
                for r in reqs {
                    let req = &d.parsed.reqs[r];
                    let (x, y) = item_span(text, req.start as usize, req.end as usize);
                    let item = alias(&text[x..y], format!("{}#{}", d.id, req.key));
                    body.replace_range(x - a..y - a, &item);
                }
                let body = shift_headings(&body, 2 - i32::from(chunks[first].level));
                let body = map_links(&body, &|l| {
                    retarget(l, &d.path, &to_path, &moved_anchors, &to_path, c)
                });
                sections.push(body);
                cuts.entry(doc).or_default().push((a, b));
            }
        }
    }

    // The source documents without the moved parts, with their links updated.
    let mut changed: HashMap<String, String> = HashMap::new();
    for (doc, mut ranges) in cuts {
        let d = &c.docs[doc as usize];
        let mut text = texts[&d.path].clone();
        ranges.sort_by_key(|r| std::cmp::Reverse(r.0));
        for (a, b) in ranges {
            let b = if text[..a].ends_with("\n\n") && text[b..].starts_with('\n') {
                b + 1
            } else {
                b
            };
            text.replace_range(a..b, "");
        }
        let text = drop_empty_headings(&text, &groups);
        let text = map_links(&text, &|l| {
            retarget(l, &d.path, &d.path, &moved_anchors, &to_path, c)
        });
        changed.insert(d.path.clone(), format!("{}\n", text.trim_end()));
    }

    // The destination.
    let new_dest = match dest_doc {
        None => {
            let src = &c.docs[first_doc as usize];
            let root = &config.roots[config.root_of(&to_path).unwrap_or(src.root)];
            let mut fm = String::new();
            if root.single_type.is_none() && src.ty != root.default_type {
                fm.push_str(&format!("type: {}\n", src.ty.name()));
            }
            if let Some(s) = &src.status {
                fm.push_str(&format!("status: {s}\n"));
            }
            let title = title_of(&to_id);
            let fm = if fm.is_empty() {
                String::new()
            } else {
                format!("---\n{fm}---\n")
            };
            let mut text = format!("{fm}# {title}\n");
            append(&mut text, &blocks(&groups, &sections));
            text
        }
        Some(t) => {
            let d = &c.docs[t as usize];
            let text = read(repo, &d.path)?;
            let text = map_links(&text, &|l| {
                retarget(l, &d.path, &d.path, &moved_anchors, &to_path, c)
            });
            insert_into(&text, &parse(&text), groups, &sections)
        }
    };
    changed.insert(to_path.clone(), new_dest);

    // Other documents that link to a moved key or section.
    let sources: HashSet<&str> = moved_anchors.iter().map(|m| m.0.as_str()).collect();
    for o in &c.docs {
        if changed.contains_key(&o.path) {
            continue;
        }
        let links_here = o.parsed.refs.iter().any(|r| {
            r.kind != crate::parse::RefKind::Wiki
                && matches!(
                    c.resolve_link(&o.path, &r.target),
                    crate::model::LinkRes::File(ref p, _, Some(_)) if sources.contains(p.as_str())
                )
        });
        if !links_here {
            continue;
        }
        let t = read(repo, &o.path)?;
        let nt = map_links(&t, &|l| {
            retarget(l, &o.path, &o.path, &moved_anchors, &to_path, c)
        });
        if nt != t {
            changed.insert(o.path.clone(), nt);
        }
    }

    let preserved =
        preserve(config, c, &changed, &to_path, created, &to_id, &new_ids).map_err(|e| {
            Fatal::Refused(format!(
                "the move would not preserve the specifications, so nothing changed: {e}"
            ))
        })?;

    if let Some(dir) = repo.root.join(&to_path).parent() {
        std::fs::create_dir_all(dir).map_err(|e| Fatal::Io(e.to_string()))?;
    }
    let mut files: Vec<String> = changed.keys().cloned().collect();
    files.sort();
    for (p, t) in &changed {
        write(&repo.root.join(p), t)?;
    }
    let aliases = moved_ids
        .iter()
        .filter(|id| !new_ids.contains(*id))
        .cloned()
        .collect();
    Ok(Split {
        to: to_id,
        to_path,
        created,
        moved: moved_ids,
        aliases,
        sections: sections.len(),
        files,
        preserved,
    })
}

fn parse_sources(c: &Corpus, sources: &[String]) -> Result<Vec<Part>, Fatal> {
    let mut parts = Vec::new();
    for s in sources {
        let (d, frag) = crate::context::resolve_target(c, s)
            .ok_or_else(|| Fatal::NotFound(format!("`{s}` names no requirement or section")))?;
        let doc = &c.docs[d as usize];
        let Some(frag) = frag else {
            return Err(Fatal::Usage(format!(
                "`{s}` is a document; name a requirement or a section as `<doc-id>#<key or anchor>`, or move one document alone"
            )));
        };
        if !doc.ty.has_requirements() {
            return Err(Fatal::Usage(format!(
                "`{}` is a {}; `mv` moves requirements and sections out of a spec or contract",
                doc.id,
                doc.ty.name()
            )));
        }
        if let Some(r) = doc.req(&frag) {
            parts.push(Part::Req { doc: d, req: r });
            continue;
        }
        let chunks = &doc.parsed.chunks;
        let Some(first) = chunks.iter().position(|ch| ch.anchor == frag) else {
            return Err(Fatal::NotFound(format!(
                "`{}` has no requirement or section `{frag}`",
                doc.id
            )));
        };
        if chunks[first].level <= 1 {
            return Err(Fatal::Usage(format!(
                "`{s}` is the title section; to move the whole document, run `spec-dna mv {} <new-id>`",
                doc.id
            )));
        }
        let level = chunks[first].level;
        let last = (first + 1..chunks.len())
            .take_while(|&j| chunks[j].level > level)
            .last()
            .unwrap_or(first);
        parts.push(Part::Section {
            doc: d,
            first,
            last,
        });
    }
    Ok(parts)
}

/// Returns the destination: an existing spec or contract, or the path and id
/// of a new spec in the root of the first source.
fn destination(
    repo: &Repo,
    config: &Config,
    c: &Corpus,
    first_doc: u32,
    dest: &str,
) -> Result<(Option<u32>, String, String), Fatal> {
    match crate::context::resolve_target(c, dest) {
        Some((_, Some(_))) => Err(Fatal::Usage(
            "the destination is a document id, without `#`".into(),
        )),
        Some((t, None)) => {
            let d = &c.docs[t as usize];
            if !d.ty.has_requirements() {
                return Err(Fatal::Usage(format!(
                    "`{}` is a {}; requirements live in a spec or contract",
                    d.id,
                    d.ty.name()
                )));
            }
            Ok((Some(t), d.path.clone(), d.id.clone()))
        }
        None => {
            if dest.contains('#') {
                return Err(Fatal::NotFound(format!("`{dest}` names no document")));
            }
            let src = &c.docs[first_doc as usize];
            let root = &config.roots[src.root];
            let path = if is_doc_path(dest) {
                dest.trim_start_matches("./").to_string()
            } else if root.path.is_empty() {
                format!("{dest}.md")
            } else {
                format!("{}/{dest}.md", root.path)
            };
            let r = config
                .root_of(&path)
                .filter(|r| !config.is_excluded(*r, &path))
                .ok_or_else(|| {
                    Fatal::Usage(format!(
                        "`{path}` is not under a document root of spec-dna.toml"
                    ))
                })?;
            if repo.root.join(&path).exists() {
                return Err(Fatal::Usage(format!(
                    "`{path}` exists, but it is not a document of a root"
                )));
            }
            let id = crate::model::root_doc_id(&config.roots[r], &path);
            Ok((None, path, id))
        }
    }
}

/// Refuses a move that gives the destination a key or a section anchor
/// twice.
fn check_names(
    c: &Corpus,
    dest: Option<u32>,
    keys: &[String],
    anchors: &[String],
    to_id: &str,
) -> Result<(), Fatal> {
    let mut seen: HashSet<&str> = HashSet::new();
    for k in keys {
        let taken = dest.is_some_and(|t| c.docs[t as usize].req(k).is_some());
        if taken || !seen.insert(k) {
            return Err(Fatal::Usage(format!(
                "`{to_id}` would have the requirement key `{k}` two times"
            )));
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for a in anchors {
        let taken = dest.is_some_and(|t| {
            c.docs[t as usize]
                .parsed
                .chunks
                .iter()
                .any(|ch| ch.anchor == *a)
        });
        if taken || !seen.insert(a) {
            return Err(Fatal::Usage(format!(
                "`{to_id}` would have the section `{a}` two times"
            )));
        }
    }
    Ok(())
}

/// The title of a new document: its slug with spaces, first letter in
/// uppercase.
fn title_of(id: &str) -> String {
    let slug = id.rsplit('/').next().unwrap_or(id).replace(['-', '_'], " ");
    let mut ch = slug.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().chain(ch).collect(),
        None => String::new(),
    }
}

/// The new text for a destination: one section for each group of
/// requirements, then the moved sections.
fn blocks(groups: &[(String, Vec<String>)], sections: &[String]) -> String {
    let mut parts: Vec<String> = groups
        .iter()
        .map(|(heading, items)| format!("## {heading}\n\n{}", items.concat().trim_end()))
        .collect();
    parts.extend(sections.iter().map(|s| s.trim().to_string()));
    parts.join("\n\n")
}

fn append(text: &mut String, block: &str) {
    if block.trim().is_empty() {
        return;
    }
    let base = text.trim_end().to_string();
    *text = format!("{base}\n\n{}\n", block.trim());
}

/// Puts groups of requirements into an existing destination. A group goes
/// after the last requirement of the section with the same heading, or at
/// the end of that section. The group of title requirements
/// ("Requirements") goes after the last requirement of the document if no
/// section has that heading. Other groups and the moved sections go to the
/// end.
fn insert_into(
    text: &str,
    parsed: &crate::parse::ParsedDoc,
    groups: Vec<(String, Vec<String>)>,
    sections: &[String],
) -> String {
    // (offset, true if the offset is right after a list item).
    let mut inserts: Vec<(usize, bool, String)> = Vec::new();
    let mut tail: Vec<(String, Vec<String>)> = Vec::new();
    let chunks = &parsed.chunks;
    let last_req = |chunk: Option<usize>| {
        parsed
            .reqs
            .iter()
            .filter(|r| r.colon && chunk.is_none_or(|c| r.chunk as usize == c))
            .map(|r| item_span(text, r.start as usize, r.end as usize).1)
            .max()
    };
    for (heading, items) in groups {
        let section = chunks
            .iter()
            .position(|ch| ch.level >= 2 && same_heading(&ch.heading, &heading));
        let at = match section {
            Some(s) => last_req(Some(s))
                .map(|p| (p, true))
                .or_else(|| Some((text[..chunks[s].end as usize].trim_end().len(), false))),
            None if heading == "Requirements" => last_req(None).map(|p| (p, true)),
            None => None,
        };
        match at {
            Some((pos, after_item)) => inserts.push((pos, after_item, items.concat())),
            None => tail.push((heading, items)),
        }
    }
    inserts.sort_by_key(|i| std::cmp::Reverse(i.0));
    let mut out = text.to_string();
    for (pos, after_item, block) in inserts {
        let block = if after_item {
            block
        } else if out[pos..].starts_with('\n') {
            format!("\n\n{}", block.trim_end())
        } else {
            format!("\n\n{block}")
        };
        out.insert_str(pos, &block);
    }
    append(&mut out, &blocks(&tail, sections));
    format!("{}\n", out.trim_end())
}

/// Removes a heading whose section became empty because all its
/// requirements moved.
fn drop_empty_headings(text: &str, groups: &[(String, Vec<String>)]) -> String {
    let parsed = parse(text);
    let chunks = &parsed.chunks;
    let mut out = text.to_string();
    for (i, ch) in chunks.iter().enumerate().rev() {
        let child = chunks.get(i + 1).is_some_and(|n| n.level > ch.level);
        if ch.level < 2 || child || !groups.iter().any(|g| same_heading(&g.0, &ch.heading)) {
            continue;
        }
        let (a, b) = (ch.start as usize, ch.end as usize);
        let body = out[a..b].split_once('\n').map_or("", |x| x.1);
        if body.trim().is_empty() {
            out.replace_range(a..b, "");
        }
    }
    out
}

/// Changes the level of the ATX headings outside code blocks by `delta`.
fn shift_headings(text: &str, delta: i32) -> String {
    if delta == 0 {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            out.push_str(line);
            continue;
        }
        let hashes = line.bytes().take_while(|b| *b == b'#').count();
        if (1..=6).contains(&hashes) && line[hashes..].starts_with([' ', '\t', '\n']) {
            let level = (hashes as i32 + delta).clamp(1, 6) as usize;
            out.push_str(&"#".repeat(level));
            out.push_str(&line[hashes..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Applies `f` to each Markdown link destination outside code blocks: inline
/// links and images (`](dest)`) and reference definitions (`[label]: dest`).
/// `f` returns the new destination, or `None` to keep it.
pub fn map_links(text: &str, f: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(fe) = fence {
            if t.starts_with(fe) {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        if t.starts_with('[')
            && let Some(i) = t.find("]: ")
        {
            let lead = line.len() - t.len() + i + 3;
            let end = line[lead..]
                .find(char::is_whitespace)
                .map_or(line.len(), |j| lead + j);
            out.push_str(&line[..lead]);
            let d = &line[lead..end];
            out.push_str(&f(d).unwrap_or_else(|| d.to_string()));
            rest = &line[end..];
        }
        while let Some(i) = rest.find("](") {
            out.push_str(&rest[..i + 2]);
            rest = &rest[i + 2..];
            let end = rest
                .find(|ch: char| ch == ')' || ch.is_whitespace())
                .unwrap_or(rest.len());
            let d = &rest[..end];
            out.push_str(&f(d).unwrap_or_else(|| d.to_string()));
            rest = &rest[end..];
        }
        out.push_str(rest);
    }
    out
}

/// Returns the new destination of a link in text that was in the document
/// at `orig` and is now in the document at `now`. A link to a moved key or
/// section goes to `target`. Returns `None` if the destination stays.
fn retarget(
    dest: &str,
    orig: &str,
    now: &str,
    moved: &HashSet<(String, String)>,
    target: &str,
    c: &Corpus,
) -> Option<String> {
    if dest.starts_with('<')
        || dest.starts_with("//")
        || dest.contains("://")
        || dest.starts_with("mailto:")
    {
        return None;
    }
    let split = dest.find('#').unwrap_or(dest.len());
    let (path, suffix) = dest.split_at(split);
    let frag = suffix.strip_prefix('#').unwrap_or("");
    let file = if path.is_empty() {
        orig.to_string()
    } else if let Some(abs) = path.strip_prefix('/') {
        join("", abs)?
    } else {
        join(parent(orig), path)?
    };
    if !path.is_empty() && !c.files.contains(&file) && !c.dirs.contains(&file) {
        return None;
    }
    let to = if !frag.is_empty() && moved.contains(&(file.clone(), frag.to_string())) {
        target
    } else {
        file.as_str()
    };
    if to == file && orig == now {
        return None;
    }
    if to == now && !frag.is_empty() {
        return Some(format!("#{frag}"));
    }
    if path.starts_with('/') {
        return Some(format!("/{to}{suffix}"));
    }
    Some(format!("{}{suffix}", relative(parent(now), to)))
}

/// The preservation check. Returns the numbers of compared requirement
/// hashes, resolved ids and accepted decisions. `new_ids` are the moved ids
/// without an alias: each must resolve at its new id in `to_id`.
fn preserve(
    config: &Config,
    c: &Corpus,
    changed: &HashMap<String, String>,
    to_path: &str,
    created: bool,
    to_id: &str,
    new_ids: &HashSet<String>,
) -> Result<(usize, usize, usize), String> {
    let mut docs: Vec<(String, Arc<crate::parse::ParsedDoc>)> = c
        .docs
        .iter()
        .map(|d| {
            let parsed = match changed.get(&d.path) {
                Some(t) => Arc::new(parse(t)),
                None => d.parsed.clone(),
            };
            (d.path.clone(), parsed)
        })
        .collect();
    let mut files: Vec<String> = c.files.iter().cloned().collect();
    if created {
        docs.push((to_path.to_string(), Arc::new(parse(&changed[to_path]))));
        files.push(to_path.to_string());
    }
    let n = Corpus::build(config, files, docs);

    let hashes = |k: &Corpus| {
        let mut v: Vec<Hash> = k
            .docs
            .iter()
            .flat_map(|d| d.reqs.iter().map(|&r| d.parsed.reqs[r].frozen))
            .collect();
        v.sort_unstable();
        v
    };
    let (before, after) = (hashes(c), hashes(&n));
    if before != after {
        return Err(hash_changes(c, &n));
    }
    let mut ids = 0;
    for d in &c.docs {
        for &r in &d.reqs {
            let req = &d.parsed.reqs[r];
            let id = format!("{}#{}", d.id, req.key);
            let at = if new_ids.contains(&id) {
                format!("{to_id}#{}", req.key)
            } else {
                id.clone()
            };
            let same = match n.resolve(&at) {
                Res::Found(hit, Some(k), true) => {
                    let hit = &n.docs[hit as usize];
                    hit.req(&k)
                        .is_some_and(|i| hit.parsed.reqs[i].frozen == req.frozen)
                }
                _ => false,
            };
            if !same {
                return Err(format!(
                    "`{id}` would no longer resolve to the same requirement"
                ));
            }
            ids += 1;
        }
    }
    let mut decisions = 0;
    for d in c
        .docs
        .iter()
        .filter(|d| d.ty == DocType::Decision && d.status_str() == "accepted")
    {
        let kept = n
            .by_path
            .get(&d.path)
            .is_some_and(|&i| n.docs[i as usize].frozen_hash == d.frozen_hash);
        if !kept {
            return Err(format!(
                "the frozen text of the accepted decision `{}` would change",
                d.id
            ));
        }
        decisions += 1;
    }
    let errors = |k: &Corpus| {
        let mut m: HashMap<(Code, String), usize> = HashMap::new();
        for d in crate::check::diagnose(config, k) {
            if d.severity == Severity::Error {
                *m.entry((d.code, d.target)).or_default() += 1;
            }
        }
        m
    };
    let (eb, ea) = (errors(c), errors(&n));
    for ((code, target), count) in &ea {
        if count > eb.get(&(*code, target.clone())).unwrap_or(&0) {
            return Err(format!("a new error {} for `{target}`", code.code()));
        }
    }
    Ok((after.len(), ids, decisions))
}

/// Names each requirement whose text the move would change, with the
/// reason: the changed text, a lost requirement or a new one.
fn hash_changes(before: &Corpus, after: &Corpus) -> String {
    type Items<'a> = HashMap<Hash, Vec<(String, &'a str)>>;
    fn items(k: &Corpus) -> Items<'_> {
        let mut m: Items = HashMap::new();
        for d in &k.docs {
            for &r in &d.reqs {
                let req = &d.parsed.reqs[r];
                m.entry(req.frozen)
                    .or_default()
                    .push((format!("{}#{}", d.id, req.key), req.text.as_str()));
            }
        }
        m
    }
    let (b, a) = (items(before), items(after));
    // The items of `x` that `y` does not have, in a stable order.
    let extra = |x: &Items<'_>, y: &Items<'_>| {
        let mut out: Vec<(String, String)> = Vec::new();
        for (h, list) in x {
            let keep = y.get(h).map_or(0, Vec::len);
            out.extend(
                list.iter()
                    .skip(keep)
                    .map(|(i, t)| (i.clone(), t.to_string())),
            );
        }
        out.sort();
        out
    };
    let (lost, mut gained) = (extra(&b, &a), extra(&a, &b));
    let key = |id: &str| id.rsplit('#').next().unwrap_or(id).to_string();
    let mut reasons = Vec::new();
    for (id, old) in lost {
        match gained.iter().position(|(g, _)| key(g) == key(&id)) {
            Some(i) => {
                let (to, new) = gained.remove(i);
                let (o, n) = excerpt(&old, &new);
                let at = if to == id {
                    String::new()
                } else {
                    format!(" (as `{to}`)")
                };
                reasons.push(format!("`{id}`{at}: the text \"{o}\" would become \"{n}\""));
            }
            None => reasons.push(format!("`{id}`: the requirement would be lost")),
        }
    }
    for (id, _) in gained {
        reasons.push(format!("`{id}`: a new requirement would appear"));
    }
    format!(
        "the text of {} would change: {}",
        if reasons.len() == 1 {
            "1 requirement".to_string()
        } else {
            format!("{} requirements", reasons.len())
        },
        reasons.join("; ")
    )
}

/// The parts of two texts around their first difference.
fn excerpt(a: &str, b: &str) -> (String, String) {
    let (ac, bc): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let p = ac.iter().zip(&bc).take_while(|(x, y)| x == y).count();
    let from = p.saturating_sub(20);
    let cut = |v: &[char]| {
        let to = v.len().min(p + 30);
        let mut s: String = v[from..to].iter().collect();
        if from > 0 {
            s.insert(0, '…');
        }
        if to < v.len() {
            s.push('…');
        }
        s
    };
    (cut(&ac), cut(&bc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_heading_levels() {
        assert_eq!(title_of("circulation/member-accounts"), "Member accounts");
        assert_eq!(
            shift_headings("### A\n\n#### B\n```\n### code\n```\n", -1),
            "## A\n\n### B\n```\n### code\n```\n"
        );
    }

    fn corpus(cfg: &Config, files: &[(&str, &str)]) -> Corpus {
        let docs = files
            .iter()
            .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
            .collect();
        Corpus::build(cfg, files.iter().map(|f| f.0.to_string()).collect(), docs)
    }

    #[test]
    fn a_refusal_names_each_changed_requirement() {
        let cfg = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ntype = \"spec\"\nstatus = \"active\"\n",
        )
        .unwrap();
        let c = corpus(
            &cfg,
            &[
                (
                    "docs/a.md",
                    "# A\n\n- **fee**: The desk SHALL charge 3 percent of [the amount](b.md#amount).\n- **cap**: The fee SHALL NOT exceed 5 euros.\n",
                ),
                (
                    "docs/b.md",
                    "# B\n\n- **amount**: The amount SHALL be the paid price.\n",
                ),
            ],
        );
        // A rewritten link destination keeps the hash.
        let moved = HashMap::from([(
            "docs/a.md".to_string(),
            "# A\n\n- **fee**: The desk SHALL charge 3 percent of [the amount](./b.md#amount).\n- **cap**: The fee SHALL NOT exceed 5 euros.\n".to_string(),
        )]);
        assert!(preserve(&cfg, &c, &moved, "docs/c.md", false, "c", &HashSet::new()).is_ok());
        // A changed sentence and a lost requirement are named.
        let broken = HashMap::from([(
            "docs/a.md".to_string(),
            "# A\n\n- **fee**: The desk SHALL charge 4 percent of [the amount](b.md#amount).\n"
                .to_string(),
        )]);
        let e = preserve(&cfg, &c, &broken, "docs/c.md", false, "c", &HashSet::new()).unwrap_err();
        assert_eq!(
            e,
            "the text of 2 requirements would change: `a#cap`: the requirement would be lost; `a#fee`: the text \"…e desk SHALL charge 3 percent of [the amount](b.md…\" would become \"…e desk SHALL charge 4 percent of [the amount](b.md…\""
        );
    }

    #[test]
    fn links_map_outside_code() {
        let t = "See [a](x.md#k) and ![i](i.png).\n```\n[c](x.md)\n```\n[r]: x.md#k\n";
        let out = map_links(t, &|d| {
            d.starts_with("x.md").then(|| d.replace("x.md", "y.md"))
        });
        assert_eq!(
            out,
            "See [a](y.md#k) and ![i](i.png).\n```\n[c](x.md)\n```\n[r]: y.md#k\n"
        );
    }
}
