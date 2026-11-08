//! `spec-dna context` and `spec-dna show` serve sections with ids and hashes
//! (ADR 0001, "Document types").
//!
//! The assembly uses this order:
//!
//! - the entry documents (or the requested sections);
//! - the documents whose `applies-to` matches an entry or a given path;
//! - the `depends-on` targets of the entries (depth 1);
//! - linked accepted decisions, as summaries.
//!
//! For a spec or contract with requirements, the assembly serves the
//! requirement sections first, then the title section, in full. It serves a
//! document without requirements in heading order. It lists the other sections
//! of these documents, and other documents that the entries link to, as stubs
//! with id, hash and size. Thus one `show` call can read each of them. If a
//! section does not fit the character budget, the assembly lists it as a stub
//! too.
//!
//! A superseded, retired or rejected document that no target names is only
//! listed, with its status and successor, unless `--include-inactive` is set.
//!
//! With `--query`, BM25 ranks the planned sections, and every section of the
//! linked documents, against the words of the query (`rank.rs`). The order is:
//!
//! - the sections that a target names;
//! - the sections that match, best first;
//! - the rest of the plan, in order.
//!
//! The assembly serves a listed-only section (`more`, `linked`) in full only
//! if it matches.

use crate::hash::hex;
use crate::json::{self, Obj};
use crate::model::{Corpus, Doc, DocType, LinkRes, Res};
use crate::parse::RefKind;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Entry,
    Concern,
    Dependency,
    Decision,
    Rest,
    Related,
}

impl Tier {
    fn name(self) -> &'static str {
        match self {
            Tier::Entry => "entry",
            Tier::Concern => "applies",
            Tier::Dependency => "depends",
            Tier::Decision => "decision",
            Tier::Rest => "more",
            Tier::Related => "linked",
        }
    }
}

pub struct Unit {
    pub doc: u32,
    pub chunk: usize,
    pub tier: Tier,
    pub full: bool,
}

pub struct Assembly {
    pub entries: Vec<String>,
    pub units: Vec<Unit>,
    pub omitted: usize,
    pub budget: u32,
    pub used: u32,
    /// The query terms that the ranking used.
    pub query: Option<Vec<String>>,
}

/// A query for the ranking: its terms, and the working tree that spec-dna
/// reads the sections from.
pub struct Query<'a> {
    pub root: &'a Path,
    pub terms: Vec<String>,
}

pub fn resolve_target(c: &Corpus, t: &str) -> Option<(u32, Option<String>)> {
    if let Some(&i) = c.by_path.get(t.trim_start_matches("./")) {
        return Some((i, None));
    }
    match c.resolve(t) {
        Res::Found(d, frag, true) => Some((d, frag)),
        _ => None,
    }
}

pub fn section_id(d: &Doc, chunk: usize) -> String {
    let a = &d.parsed.chunks[chunk].anchor;
    if a.is_empty() {
        d.id.clone()
    } else {
        format!("{}#{a}", d.id)
    }
}

fn header(d: &Doc, chunk: usize, tier: &str) -> String {
    let ch = &d.parsed.chunks[chunk];
    let status = d
        .status
        .as_deref()
        .map(|s| format!("/{s}"))
        .unwrap_or_default();
    format!(
        "<!-- {} h:{} {}{status} {tier} -->\n",
        section_id(d, chunk),
        hex(&ch.hash),
        d.ty.name()
    )
}

fn stub_line(c: &Corpus, d: &Doc, chunk: usize, tier: Tier) -> String {
    let ch = &d.parsed.chunks[chunk];
    let title = if ch.heading.is_empty() {
        d.parsed.title.as_str()
    } else {
        ch.heading.as_str()
    };
    format!(
        "- `{}` h:{} {} chars ({}): {title}{}\n",
        section_id(d, chunk),
        hex(&ch.hash),
        ch.chars,
        tier.name(),
        inactive_note(c, d)
    )
}

/// For an inactive document: its status and its successors, for a stub line.
fn inactive_note(c: &Corpus, d: &Doc) -> String {
    if !d.is_inactive() {
        return String::new();
    }
    let next: Vec<String> = successors(c, d)
        .iter()
        .map(|&s| format!("`{}`", c.docs[s as usize].id))
        .collect();
    if next.is_empty() {
        format!(" [{}]", d.status_str())
    } else {
        format!(" [{}; successor: {}]", d.status_str(), next.join(", "))
    }
}

/// Returns the documents whose `supersedes` names `d`.
fn successors(c: &Corpus, d: &Doc) -> Vec<u32> {
    let me = c.by_path.get(&d.path).copied();
    c.docs
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.supersedes
                .iter()
                .any(|(t, _)| matches!(c.resolve(t), Res::Found(i, _, _) if Some(i) == me))
        })
        .map(|(i, _)| i as u32)
        .collect()
}

pub fn assemble(
    c: &Corpus,
    targets: &[(u32, Option<String>)],
    paths: &[String],
    budget: u32,
    query: Option<&Query>,
    include_inactive: bool,
) -> Assembly {
    let mut plan: Vec<(u32, usize, Tier)> = Vec::new();
    let mut seen: HashSet<(u32, usize)> = HashSet::new();
    let mut add = |plan: &mut Vec<(u32, usize, Tier)>, d: u32, ch: usize, t: Tier| {
        if seen.insert((d, ch)) {
            plan.push((d, ch, t));
        }
    };
    let entry_docs: Vec<u32> = {
        let mut v: Vec<u32> = targets.iter().map(|t| t.0).collect();
        v.dedup();
        v
    };
    let mut rest: Vec<(u32, usize)> = Vec::new();
    // Sections that a target names explicitly. Under a query, they come first.
    let mut pinned: HashSet<(u32, usize)> = HashSet::new();

    // 1. Entries.
    for (d, frag) in targets {
        let doc = &c.docs[*d as usize];
        let chunks = &doc.parsed.chunks;
        match frag {
            Some(f) => {
                let start = chunks
                    .iter()
                    .position(|ch| &ch.anchor == f)
                    .or_else(|| doc.req(f).map(|r| doc.parsed.reqs[r].chunk as usize));
                if let Some(s) = start {
                    add(&mut plan, *d, s, Tier::Entry);
                    pinned.insert((*d, s));
                    let lvl = chunks[s].level;
                    for (j, ch) in chunks.iter().enumerate().skip(s + 1) {
                        if ch.level <= lvl {
                            break;
                        }
                        add(&mut plan, *d, j, Tier::Entry);
                        pinned.insert((*d, j));
                    }
                }
                rest.extend((0..chunks.len()).map(|j| (*d, j)));
            }
            None => normative_units(c, *d, Tier::Entry, &mut plan, &mut rest, &mut add),
        }
    }

    // 2. Documents whose `applies-to` matches an entry or a path.
    let scopes: Vec<&str> = entry_docs
        .iter()
        .flat_map(|&d| {
            [
                c.docs[d as usize].id.as_str(),
                c.docs[d as usize].path.as_str(),
            ]
        })
        .chain(paths.iter().map(String::as_str))
        .collect();
    let mut concerns: Vec<u32> = c
        .docs
        .iter()
        .enumerate()
        .filter(|(i, d)| {
            d.ty.is_normative()
                && !d.is_inactive()
                && !entry_docs.contains(&(*i as u32))
                && d.applies_to
                    .iter()
                    .any(|p| scopes.iter().any(|s| crate::glob::matches_scope(p, s)))
        })
        .map(|(i, _)| i as u32)
        .collect();
    concerns.sort_by(|a, b| c.docs[*a as usize].id.cmp(&c.docs[*b as usize].id));
    for d in concerns {
        normative_units(c, d, Tier::Concern, &mut plan, &mut rest, &mut add);
    }

    // 3. Dependencies, depth 1. A superseded, retired or rejected target is
    // only listed, with its status and successor (ADR 0001, "Lifecycle").
    for &e in &entry_docs {
        for (t, _) in &c.docs[e as usize].depends_on {
            if let Res::Found(d, frag, true) = c.resolve(t) {
                if !include_inactive && c.docs[d as usize].is_inactive() {
                    if !c.docs[d as usize].parsed.chunks.is_empty() {
                        add(&mut plan, d, 0, Tier::Dependency);
                    }
                    continue;
                }
                match frag.and_then(|f| {
                    let doc = &c.docs[d as usize];
                    doc.parsed
                        .chunks
                        .iter()
                        .position(|ch| ch.anchor == f)
                        .or_else(|| doc.req(&f).map(|r| doc.parsed.reqs[r].chunk as usize))
                }) {
                    Some(ch) => {
                        add(&mut plan, d, ch, Tier::Dependency);
                        rest.extend((0..c.docs[d as usize].parsed.chunks.len()).map(|j| (d, j)));
                    }
                    None => normative_units(c, d, Tier::Dependency, &mut plan, &mut rest, &mut add),
                }
            }
        }
    }

    // 4. Accepted decisions linked with an entry, in either direction.
    let mut decisions: Vec<u32> = Vec::new();
    for &e in &entry_docs {
        for d in linked_docs(c, e) {
            if is_accepted_decision(&c.docs[d as usize]) {
                decisions.push(d);
            }
        }
    }
    for (i, d) in c.docs.iter().enumerate() {
        if is_accepted_decision(d)
            && linked_docs(c, i as u32)
                .iter()
                .any(|t| entry_docs.contains(t))
        {
            decisions.push(i as u32);
        }
    }
    decisions.sort_by(|a, b| c.docs[*a as usize].id.cmp(&c.docs[*b as usize].id));
    decisions.dedup();
    for d in decisions {
        let chunks = &c.docs[d as usize].parsed.chunks;
        for (j, ch) in chunks.iter().enumerate() {
            // The summary of a decision: its title, its outcome and its
            // amendments, in document order.
            let doc = &c.docs[d as usize];
            let summary = ch.level <= 1 || doc.outcome.contains(&j) || doc.amendments.contains(&j);
            if summary {
                add(&mut plan, d, j, Tier::Decision);
            } else {
                rest.push((d, j));
            }
        }
    }

    // 5. The rest of the documents above.
    for (d, j) in rest {
        add(&mut plan, d, j, Tier::Rest);
    }

    // 6. Other documents that the entries link to, as stubs.
    let mut related = 0;
    for &e in &entry_docs {
        for d in linked_docs(c, e) {
            if related >= 20 {
                break;
            }
            let n = c.docs[d as usize].parsed.chunks.len();
            if n > 0 && !plan.iter().any(|p| p.0 == d) {
                // Under a query, every section is a candidate. Without a query,
                // only the title is a candidate.
                let take = if query.is_some() { n } else { 1 };
                for j in 0..take {
                    add(&mut plan, d, j, Tier::Related);
                }
                related += 1;
            }
        }
    }

    // The packing order: the order of the plan, or the ranking under a query.
    let terms = query.map(|q| q.terms.clone()).filter(|t| !t.is_empty());
    let (order, mut eligible) = match (query, &terms) {
        (Some(q), Some(terms)) => rank_plan(c, q.root, terms, &plan, &pinned),
        _ => (
            (0..plan.len()).collect(),
            plan.iter()
                .map(|p| !matches!(p.2, Tier::Rest | Tier::Related))
                .collect::<Vec<bool>>(),
        ),
    };
    // An inactive document that no target names is only listed.
    if !include_inactive {
        for (i, &(d, _, _)) in plan.iter().enumerate() {
            if c.docs[d as usize].is_inactive() && !entry_docs.contains(&d) {
                eligible[i] = false;
            }
        }
    }

    // Packing has 2 passes:
    // - First the map. List every planned section as a stub, in priority
    //   order, within half of the budget.
    // - Then the full text, in the same order, with the remaining budget. A
    //   section in full replaces its stub.
    // Thus one `show` call can read each listed section, for documents of
    // any length.
    let costs: Vec<(u32, u32)> = plan
        .iter()
        .map(|&(d, ch, tier)| {
            let doc = &c.docs[d as usize];
            let full =
                doc.parsed.chunks[ch].chars + header(doc, ch, tier.name()).chars().count() as u32;
            (full, stub_line(c, doc, ch, tier).chars().count() as u32)
        })
        .collect();
    let mut listed = vec![false; plan.len()];
    let mut used = 0u32;
    for &i in &order {
        let stub = costs[i].1;
        if used + stub <= budget / 2 {
            used += stub;
            listed[i] = true;
        }
    }
    let mut full = vec![false; plan.len()];
    for &i in &order {
        if !eligible[i] {
            continue;
        }
        let (f, stub) = costs[i];
        let extra = if listed[i] { f.saturating_sub(stub) } else { f };
        if used + extra <= budget {
            used += extra;
            full[i] = true;
        }
    }
    let mut units = Vec::new();
    let mut omitted = 0;
    for (i, &(d, ch, tier)) in plan.iter().enumerate() {
        if full[i] || listed[i] {
            units.push(Unit {
                doc: d,
                chunk: ch,
                tier,
                full: full[i],
            });
        } else {
            omitted += 1;
        }
    }
    Assembly {
        entries: entry_docs
            .iter()
            .map(|&d| c.docs[d as usize].id.clone())
            .collect(),
        units,
        omitted,
        budget,
        used,
        query: terms,
    }
}

/// Returns the packing order under a query, and the sections that the
/// assembly can serve in full. The order is:
///
/// - the sections that a target names;
/// - the sections that match, best first;
/// - the other sections of the plan, in plan order.
///
/// Ties keep the plan order, so the result is deterministic.
fn rank_plan(
    c: &Corpus,
    root: &Path,
    terms: &[String],
    plan: &[(u32, usize, Tier)],
    pinned: &HashSet<(u32, usize)>,
) -> (Vec<usize>, Vec<bool>) {
    let mut texts: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    for &(d, _, _) in plan {
        texts.entry(d).or_insert_with(|| read_doc(c, root, d));
    }
    let sections: Vec<(&str, &str)> = plan
        .iter()
        .map(|&(d, ch, _)| {
            let doc = &c.docs[d as usize];
            let heading = if doc.parsed.chunks[ch].heading.is_empty() {
                doc.parsed.title.as_str()
            } else {
                doc.parsed.chunks[ch].heading.as_str()
            };
            (heading, chunk_text(&texts[&d], doc, ch))
        })
        .collect();
    let scores = crate::rank::bm25(terms, &sections);
    let class = |i: usize| -> u8 {
        let (d, ch, tier) = plan[i];
        if pinned.contains(&(d, ch)) {
            0
        } else if scores[i] > 0.0 {
            1
        } else if matches!(tier, Tier::Rest | Tier::Related) {
            3
        } else {
            2
        }
    };
    let mut order: Vec<usize> = (0..plan.len()).collect();
    order.sort_by(|&a, &b| {
        class(a)
            .cmp(&class(b))
            .then_with(|| {
                if class(a) == 1 {
                    scores[b].total_cmp(&scores[a])
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then(a.cmp(&b))
    });
    let eligible = (0..plan.len()).map(|i| class(i) != 3).collect();
    (order, eligible)
}

fn read_doc(c: &Corpus, root: &Path, d: u32) -> String {
    std::fs::read(root.join(&c.docs[d as usize].path))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

fn is_accepted_decision(d: &Doc) -> bool {
    d.ty == DocType::Decision && d.status_str() == "accepted"
}

/// Plans the sections of a document. For a document with requirements, the
/// sections that hold valid requirements come first, then the title section.
/// The other sections are narrative: they go to `rest`. A document without
/// requirements keeps the heading order.
fn normative_units(
    c: &Corpus,
    d: u32,
    tier: Tier,
    plan: &mut Vec<(u32, usize, Tier)>,
    rest: &mut Vec<(u32, usize)>,
    add: &mut impl FnMut(&mut Vec<(u32, usize, Tier)>, u32, usize, Tier),
) {
    let doc = &c.docs[d as usize];
    let chunks = &doc.parsed.chunks;
    if doc.reqs.is_empty() {
        for j in 0..chunks.len() {
            add(plan, d, j, tier);
        }
        return;
    }
    let (with_reqs, narrative): (Vec<usize>, Vec<usize>) =
        (0..chunks.len()).partition(|&j| doc.has_req_in(j));
    for j in with_reqs {
        add(plan, d, j, tier);
    }
    for j in narrative {
        if chunks[j].level <= 1 {
            add(plan, d, j, tier);
        } else {
            rest.push((d, j));
        }
    }
}

/// Returns the documents that a document references explicitly (links, `[[ ]]`,
/// front matter).
pub fn linked_docs(c: &Corpus, d: u32) -> Vec<u32> {
    let doc = &c.docs[d as usize];
    let mut out = Vec::new();
    for r in &doc.parsed.refs {
        let hit = match r.kind {
            RefKind::Wiki => match c.resolve(&r.target) {
                Res::Found(t, _, _) => Some(t),
                _ => None,
            },
            _ => match c.resolve_link(&doc.path, &r.target) {
                LinkRes::File(_, Some(t), _) | LinkRes::Dir(Some(t)) => Some(t),
                _ => None,
            },
        };
        out.extend(hit);
    }
    for (t, _) in doc.depends_on.iter().chain(&doc.supersedes) {
        if let Res::Found(t, _, _) = c.resolve(t) {
            out.push(t);
        }
    }
    out.retain(|&t| t != d);
    let mut seen = HashSet::new();
    out.retain(|t| seen.insert(*t));
    out
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

fn chunk_text<'a>(text: &'a str, d: &Doc, chunk: usize) -> &'a str {
    let ch = &d.parsed.chunks[chunk];
    text.get(ch.start as usize..ch.end as usize)
        .unwrap_or("")
        .trim_end()
}

pub fn render(c: &Corpus, root: &Path, a: &Assembly, as_json: bool) -> String {
    let mut texts: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    for u in a.units.iter().filter(|u| u.full) {
        texts
            .entry(u.doc)
            .or_insert_with(|| read_doc(c, root, u.doc));
    }
    let full = a.units.iter().filter(|u| u.full).count();
    let stubs = a.units.len() - full;
    if as_json {
        let units = a.units.iter().map(|u| {
            let d = &c.docs[u.doc as usize];
            let ch = &d.parsed.chunks[u.chunk];
            let mut o = Obj::new()
                .str("id", &section_id(d, u.chunk))
                .str("doc", &d.id)
                .str("path", &d.path)
                .str("type", d.ty.name())
                .opt_str("status", d.status.as_deref())
                .str("anchor", &ch.anchor)
                .str("heading", &ch.heading)
                .num("line", ch.line)
                .str("tier", u.tier.name())
                .str("hash", &hex(&ch.hash))
                .num("chars", ch.chars)
                .str("mode", if u.full { "full" } else { "stub" });
            if d.is_inactive() {
                let next: Vec<String> = successors(c, d)
                    .iter()
                    .map(|&s| c.docs[s as usize].id.clone())
                    .collect();
                o = o.strs("successors", &next);
            }
            if u.full {
                o = o.str("text", chunk_text(&texts[&u.doc], d, u.chunk));
            }
            o.end()
        });
        let mut body = Obj::new()
            .str("version", crate::adapters::VERSION)
            .strs("entries", &a.entries);
        if let Some(q) = &a.query {
            body = body.strs("query", q);
        }
        let body = body
            .num("budget", a.budget)
            .num("used", a.used)
            .num("full", full)
            .num("stubs", stubs)
            .num("omitted", a.omitted)
            .raw("units", &json::array(units))
            .end();
        let tokens = estimate_tokens(&body);
        return format!(
            "{}\n",
            body.replacen('{', &format!("{{\"estimated_tokens\":{tokens},"), 1)
        );
    }
    let mut out = String::new();
    for u in a.units.iter().filter(|u| u.full) {
        let d = &c.docs[u.doc as usize];
        out.push_str(&header(d, u.chunk, u.tier.name()));
        out.push_str(chunk_text(&texts[&u.doc], d, u.chunk));
        out.push_str("\n\n");
    }
    if stubs > 0 {
        out.push_str("<!-- not loaded; read one with `spec-dna show <id>` -->\n");
        for u in a.units.iter().filter(|u| !u.full) {
            out.push_str(&stub_line(c, &c.docs[u.doc as usize], u.chunk, u.tier));
        }
    }
    if a.omitted > 0 {
        out.push_str(&format!(
            "<!-- {} more sections did not fit; raise --budget to list them -->\n",
            a.omitted
        ));
    }
    let query = a
        .query
        .as_ref()
        .map_or(String::new(), |q| format!(" (query: {})", q.join(" ")));
    let head = format!(
        "<!-- spec-dna context {}: {}{query}; {full} sections in full, {stubs} listed; {} of {} characters; about {} tokens (estimated) -->\n\n",
        crate::adapters::VERSION,
        a.entries.join(", "),
        a.used,
        a.budget,
        estimate_tokens(&out)
    );
    format!("{head}{}\n", out.trim_end())
}

/// `spec-dna show` gives a document, a section with its subsections, or one
/// requirement.
pub fn show(c: &Corpus, root: &Path, d: u32, frag: Option<&str>, as_json: bool) -> String {
    let doc = &c.docs[d as usize];
    let text = std::fs::read(root.join(&doc.path))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let chunks = &doc.parsed.chunks;
    let (kind, items): (&str, Vec<(String, String, u32, String)>) = match frag {
        Some(f) if doc.req(f).is_some() && !chunks.iter().any(|ch| ch.anchor == f) => {
            let r = &doc.parsed.reqs[doc.req(f).unwrap()];
            let body = text
                .get(r.start as usize..r.end as usize)
                .unwrap_or("")
                .trim_end()
                .to_string();
            (
                "requirement",
                vec![(format!("{}#{}", doc.id, r.key), hex(&r.hash), r.line, body)],
            )
        }
        _ => {
            let range: Vec<usize> =
                match frag.and_then(|f| chunks.iter().position(|ch| ch.anchor == f)) {
                    Some(s) => {
                        let lvl = chunks[s].level;
                        std::iter::once(s)
                            .chain((s + 1..chunks.len()).take_while(|&j| chunks[j].level > lvl))
                            .collect()
                    }
                    None => (0..chunks.len()).collect(),
                };
            let items = range
                .into_iter()
                .map(|j| {
                    (
                        section_id(doc, j),
                        hex(&chunks[j].hash),
                        chunks[j].line,
                        chunk_text(&text, doc, j).to_string(),
                    )
                })
                .collect();
            (
                if frag.is_some() {
                    "section"
                } else {
                    "document"
                },
                items,
            )
        }
    };
    // The recognized requirements. They show a reader whether spec-dna found
    // each requirement of a legacy syntax.
    let keys: Vec<String> = doc
        .reqs
        .iter()
        .map(|&r| doc.parsed.reqs[r].key.clone())
        .collect();
    if as_json {
        let units = items.iter().map(|(id, h, line, t)| {
            Obj::new()
                .str("id", id)
                .str("hash", h)
                .num("line", line)
                .str("text", t)
                .end()
        });
        return format!(
            "{}\n",
            Obj::new()
                .str("doc", &doc.id)
                .str("path", &doc.path)
                .str("type", doc.ty.name())
                .opt_str("status", doc.status.as_deref())
                .opt_str("owner", doc.owner.as_deref())
                .str("kind", kind)
                .strs("requirements", &keys)
                .raw("units", &json::array(units))
                .end()
        );
    }
    let status = doc
        .status
        .as_deref()
        .map(|s| format!("/{s}"))
        .unwrap_or_default();
    let mut out = String::new();
    if kind == "document" && doc.ty.has_requirements() {
        out.push_str(&format!(
            "<!-- {}: {} requirements -->\n",
            doc.id,
            keys.len()
        ));
    }
    for (id, h, _, t) in items {
        out.push_str(&format!(
            "<!-- {id} h:{h} {}{status} {} -->\n{t}\n\n",
            doc.ty.name(),
            doc.path
        ));
    }
    format!("{}\n", out.trim_end())
}
