//! `spec-dna check` applies the controls of ADR 0001 to one tree. It also
//! checks the transitions between the base tree and the head tree, and it
//! applies the gate against the base.

use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::evidence::{Evidence, needs_evidence};
use crate::layer::Keep;
use crate::lint::Phrases;
use crate::model::{Corpus, Doc, DocType, LinkRes, Res, canonical};
use crate::parse::{FLAG_BCP14, FLAG_REQUIREMENT, FLAG_STATUS_WORD, RefKind};
use crate::repo::Snapshot;
use std::collections::{BTreeMap, HashMap, HashSet};

fn diag(
    d: &Doc,
    line: u32,
    code: Code,
    target: impl Into<String>,
    message: impl Into<String>,
) -> Diag {
    Diag {
        path: d.path.clone(),
        line,
        code,
        doc: d.id.clone(),
        target: target.into(),
        message: message.into(),
        severity: code.severity(),
    }
}

/// Sets the severity of each diagnostic from the profile and the rules of
/// its root (`Config::severity`), and removes the diagnostics that are off.
pub fn apply_severity(config: &Config, diags: Vec<Diag>) -> Vec<Diag> {
    diags
        .into_iter()
        .filter_map(|mut d| {
            d.severity = config.severity(d.code, config.root_of(&d.path))?;
            Some(d)
        })
        .collect()
}

/// Returns every diagnostic of one tree, with the severities of the
/// configuration.
pub fn diagnose(config: &Config, c: &Corpus) -> Vec<Diag> {
    let (ev, mut out) = Evidence::build(c);
    let layers = doc_layers(config, c);
    let facts = Facts::build(config, c, &ev, &layers);
    let per_doc = crate::repo::par_map(&c.docs, |d| doc_diags(config, c, d, &facts));
    out.extend(per_doc.into_iter().flatten());

    // Duplicate ids and aliases.
    for (name, docs) in &c.lookup {
        if docs.len() > 1 {
            for &i in docs {
                let d = &c.docs[i as usize];
                let others: Vec<&str> = docs
                    .iter()
                    .filter(|&&j| j != i)
                    .map(|&j| c.docs[j as usize].path.as_str())
                    .collect();
                out.push(diag(
                    d,
                    1,
                    Code::DuplicateId,
                    name.clone(),
                    format!("`{name}` also names {}", others.join(", ")),
                ));
            }
        }
    }
    // Supersede cycles.
    let edges: Vec<Vec<u32>> = c
        .docs
        .iter()
        .map(|d| {
            d.supersedes
                .iter()
                .filter_map(|(s, _)| found(c.resolve(s)))
                .collect()
        })
        .collect();
    for (i, d) in c.docs.iter().enumerate() {
        if reaches(&edges, i as u32, i as u32) {
            out.push(diag(
                d,
                1,
                Code::SupersedeCycle,
                "",
                "the `supersedes` chain from this document returns to it",
            ));
        }
    }
    // The pointers of `conventions` must name a document or a section.
    for (t, ptrs) in config.conventions.iter().enumerate() {
        for p in ptrs {
            if crate::context::resolve_pointer(c, p).is_none() {
                let ty = DocType::ALL[t].name();
                out.push(Diag {
                    path: crate::config::FILE_NAME.to_string(),
                    line: 1,
                    code: Code::ConventionUnresolved,
                    doc: crate::config::FILE_NAME.to_string(),
                    target: p.clone(),
                    message: format!(
                        "`conventions.{ty}` points to `{p}`, which names no document, file or section"
                    ),
                    severity: Code::ConventionUnresolved.severity(),
                });
            }
        }
    }
    let mut out = apply_severity(config, out);
    out.extend(layer_limits(config, c, &layers));
    out.sort();
    out
}

/// The layer of each document, as `layer list` shows it. A document with no
/// valid type is in no layer (D12).
fn doc_layers(config: &Config, c: &Corpus) -> Vec<Option<usize>> {
    c.docs
        .iter()
        .map(|d| {
            if d.has_layer() {
                config.layers.layer_of(rel_in_root(config, d), d.ty)
            } else {
                None
            }
        })
        .collect()
}

/// The path of a document relative to its root. The head configuration gives
/// the root. A root that moved gives the full path.
fn rel_in_root<'a>(config: &'a Config, d: &'a Doc) -> &'a str {
    config
        .roots
        .get(d.root)
        .filter(|r| d.path.starts_with(&format!("{}/", r.path)) || r.path.is_empty())
        .map_or(d.path.as_str(), |r| r.rel(&d.path))
}

/// SD902: a layer with more documents than `max-docs`, or more body
/// characters than `max-chars`. The documents of all roots count. A stub and a
/// generated index do not count. The diagnostic is on `spec-dna.toml`, as
/// SD106 is, so the rules and the profile of a root do not apply (D20).
fn layer_limits(config: &Config, c: &Corpus, layers: &[Option<usize>]) -> Vec<Diag> {
    let Some(severity) = config.severity(Code::LayerLimit, None) else {
        return Vec::new();
    };
    let graph = config.layers.layers();
    if graph
        .iter()
        .all(|l| l.max_docs.is_none() && l.max_chars.is_none())
    {
        return Vec::new();
    }
    let mut count = vec![(0u64, 0u64); graph.len()];
    for (d, l) in c.docs.iter().zip(layers) {
        if let Some(l) = l
            && d.moved_to.is_none()
            && d.generated.is_none()
        {
            count[*l].0 += 1;
            count[*l].1 += u64::from(d.parsed.body_chars);
        }
    }
    let mut out = Vec::new();
    for (l, &(docs, chars)) in graph.iter().zip(&count) {
        for (key, n, max, what) in [
            ("max-docs", docs, l.max_docs, "documents"),
            ("max-chars", chars, l.max_chars, "body characters"),
        ] {
            if let Some(max) = max
                && n > u64::from(max)
            {
                out.push(Diag {
                    path: crate::config::FILE_NAME.to_string(),
                    line: 1,
                    code: Code::LayerLimit,
                    doc: crate::config::FILE_NAME.to_string(),
                    target: format!("{}:{key}", l.name),
                    message: format!(
                        "layer `{}` has {n} {what}, more than `{key} = {max}`",
                        l.name
                    ),
                    severity,
                });
            }
        }
    }
    out
}

/// The data that a check shares between the documents of one corpus.
struct Facts<'a> {
    ev: &'a Evidence,
    /// The layer of each document (`doc_layers`).
    layers: &'a [Option<usize>],
    /// For SD901: `bad[x * n + y]` is true if a document of layer `x` must not
    /// link a document of layer `y`. Empty if SD901 is off.
    bad: Vec<bool>,
    n_layers: usize,
    /// The names of the layers, for the messages.
    names: Vec<String>,
    /// The history phrases of SD903.
    history: Phrases,
    /// The forbidden synonyms of SD904, with their terms. Empty if the
    /// language is not English or SD904 is off.
    terms: Phrases,
    english: bool,
}

impl<'a> Facts<'a> {
    fn layer_name(&self, i: usize) -> &str {
        &self.names[i]
    }

    fn build(config: &Config, c: &Corpus, ev: &'a Evidence, layers: &'a [Option<usize>]) -> Self {
        let g = &config.layers;
        let n = g.layers().len();
        let mut bad = Vec::new();
        if !config.is_off(Code::LinkDirection) {
            bad = vec![false; n * n];
            for x in 0..n {
                for y in 0..n {
                    bad[x * n + y] = x != y
                        && g.more_stable(x, y)
                        && !g.layers()[y].linked_from.contains(&g.layers()[x].name);
                }
            }
        }
        let english = config.language_is_english();
        let mut terms = Phrases::default();
        if english && !config.is_off(Code::TermForbidden) {
            for d in c.docs.iter().filter(|d| !d.is_inactive()) {
                for row in d.term_rows() {
                    for synonym in &row.forbidden {
                        if !synonym.eq_ignore_ascii_case(&row.term) {
                            terms.add(synonym, &row.term);
                        }
                    }
                }
            }
        }
        Facts {
            ev,
            layers,
            bad,
            n_layers: n,
            names: g.layers().iter().map(|l| l.name.clone()).collect(),
            history: Phrases::history(config.history_terms.as_deref()),
            terms,
            english,
        }
    }
}

fn found(r: Res) -> Option<u32> {
    match r {
        Res::Found(d, _, _) => Some(d),
        _ => None,
    }
}

fn reaches(edges: &[Vec<u32>], from: u32, target: u32) -> bool {
    let mut seen = HashSet::new();
    let mut stack: Vec<u32> = edges[from as usize].clone();
    while let Some(n) = stack.pop() {
        if n == target {
            return true;
        }
        if seen.insert(n) {
            stack.extend(&edges[n as usize]);
        }
    }
    false
}

fn doc_diags(config: &Config, c: &Corpus, d: &Doc, facts: &Facts) -> Vec<Diag> {
    let ev = facts.ev;
    // The documents that this one references, for SD901. A stub and a
    // generated index have no such references.
    let track = !facts.bad.is_empty() && d.moved_to.is_none() && d.generated.is_none();
    let mut hits: Vec<(u32, u32)> = Vec::new();
    let mut out: Vec<Diag> = d
        .issues
        .iter()
        .map(|i| diag(d, i.line, i.code, i.target.clone(), i.message.clone()))
        .collect();
    let p = &d.parsed;
    if let Some(i) = c.by_path.get(&d.path)
        && let Some(stale) = crate::registry::stale(c, *i)
    {
        out.push(diag(d, 1, Code::IndexStale, "", stale));
    }

    for r in &p.refs {
        match r.kind {
            RefKind::Wiki => {
                if let Some(t) = id_ref(c, d, &r.target, r.line, &mut out)
                    && track
                {
                    hits.push((t, r.line));
                }
            }
            RefKind::Link | RefKind::Image => match c.resolve_link(&d.path, &r.target) {
                LinkRes::Missing(path) => out.push(diag(
                    d,
                    r.line,
                    Code::UnresolvedLink,
                    r.target.clone(),
                    format!(
                        "link target `{}` is not in the git tree (resolved to `{path}`)",
                        r.target
                    ),
                )),
                LinkRes::File(path, Some(t), Some(frag))
                    if !frag.is_empty() && !c.docs[t as usize].has_anchor(&frag) =>
                {
                    if track && r.kind == RefKind::Link {
                        hits.push((t, r.line));
                    }
                    out.push(diag(
                        d,
                        r.line,
                        Code::AnchorMissing,
                        r.target.clone(),
                        format!("`{path}` has no heading or requirement `{frag}`"),
                    ))
                }
                LinkRes::File(_, Some(t), _) | LinkRes::Dir(Some(t))
                    if track && r.kind == RefKind::Link =>
                {
                    hits.push((t, r.line));
                }
                LinkRes::SelfAnchor(frag) if !d.has_anchor(&frag) => out.push(diag(
                    d,
                    r.line,
                    Code::AnchorMissing,
                    r.target.clone(),
                    format!("this document has no heading or requirement `{frag}`"),
                )),
                _ => {}
            },
        }
    }
    // A stub needs only a `moved-to` that resolves.
    if let Some((t, line)) = &d.moved_to {
        id_ref(c, d, t, *line, &mut out);
        return out;
    }
    for (t, line) in d.supersedes.iter() {
        if let Some(t) = id_ref(c, d, t, *line, &mut out)
            && track
        {
            hits.push((t, *line));
        }
    }
    for (t, line) in d.depends_on.iter() {
        if let Some(i) = id_ref(c, d, t, *line, &mut out)
            && track
        {
            hits.push((i, *line));
        }
        if let Some(i) = found(c.resolve(t)) {
            let target = &c.docs[i as usize];
            match target.status_str() {
                "deprecated" => out.push(diag(
                    d,
                    *line,
                    Code::DependsOnDeprecated,
                    t.clone(),
                    format!("`{t}` is deprecated"),
                )),
                "superseded" | "retired" => out.push(diag(
                    d,
                    *line,
                    Code::DependsOnInactive,
                    t.clone(),
                    format!("`{t}` is {}", target.status_str()),
                )),
                _ => {}
            }
        }
    }
    let exists = |path: &str| {
        let p = crate::cite::strip_location(path).trim_matches('/');
        c.files.contains(p) || c.dirs.contains(p)
    };
    for (path, line) in &d.verified_by {
        if !exists(path) {
            out.push(diag(
                d,
                *line,
                Code::EvidenceMissing,
                path.clone(),
                format!("`verified-by` path `{path}` is not in the git tree"),
            ));
        }
    }
    // Evidence of each requirement: the inline marker must resolve, and an
    // active requirement needs a link to a test (SD608).
    let di = c.by_path.get(&d.path).copied().unwrap_or(0);
    for &ri in &d.reqs {
        let r = &p.reqs[ri];
        for path in &r.verified_by {
            if !exists(path) {
                out.push(diag(
                    d,
                    r.line,
                    Code::EvidenceMissing,
                    path.clone(),
                    format!(
                        "`{}`: `verified-by` path `{path}` is not in the git tree",
                        r.key
                    ),
                ));
            }
        }
        if needs_evidence(d) && ev.of(di, d, ri).is_empty() {
            out.push(diag(
                d,
                r.line,
                Code::RequirementUnverified,
                r.key.clone(),
                format!(
                    "`{}#{}` is active but no test covers it (`covers:` tag or `verified-by`)",
                    d.id, r.key
                ),
            ));
        }
    }
    for t in &p.placeholders {
        out.push(diag(
            d,
            t.line,
            Code::PlaceholderLeft,
            t.text.clone(),
            format!(
                "the template placeholder `{}` is still in the document",
                t.text
            ),
        ));
    }
    if d.ty == DocType::Incident && d.generated.is_none() && !links_knowledge(c, d) {
        out.push(diag(
            d,
            1,
            Code::IncidentUnlinked,
            "",
            "the incident links no requirement, rule, decision or procedure",
        ));
    }
    let locked =
        if d.ty == DocType::Decision && d.status_str() == "accepted" && d.generated.is_none() {
            decision_links_change(config, c, d, &mut out)
        } else {
            Vec::new()
        };
    if track {
        link_direction(c, d, facts, &hits, &locked, &mut out);
    }
    for t in &p.tokens {
        let Some((prefix, _)) = t.text.split_once('-') else {
            continue;
        };
        if c.token_prefixes.iter().any(|x| x == prefix)
            && !c.lookup.contains_key(&canonical(&t.text))
        {
            out.push(diag(
                d,
                t.line,
                Code::TokenUnresolved,
                t.text.clone(),
                format!("`{}` names no document", t.text),
            ));
        }
    }

    // Lifecycle state.
    if d.status_str() == "superseded" {
        let want = if d.ty == DocType::Decision {
            "accepted"
        } else {
            "active"
        };
        let has_successor = c.docs.iter().any(|s| {
            s.status_str() == want
                && s.supersedes
                    .iter()
                    .any(|(t, _)| found(c.resolve(t)) == c.by_path.get(&d.path).copied())
        });
        if !has_successor {
            out.push(diag(
                d,
                1,
                Code::SupersededWithoutSuccessor,
                "",
                format!("no {want} document names `{}` in `supersedes`", d.id),
            ));
        }
    }

    // `hard_budget` compares a document over the hard budget with the base.
    let (soft, hard) = config.budgets[d.ty as usize];
    if soft > 0
        && !d.is_exempt_from_budgets()
        && p.body_chars > soft
        && (hard == 0 || p.body_chars <= hard)
    {
        out.push(diag(
            d,
            1,
            Code::BudgetSoft,
            "",
            format!(
                "{} characters, over the soft budget of {soft}",
                p.body_chars
            ),
        ));
    }

    history_and_terms(config, d, facts, &mut out);

    // The profile and the rules of the root can turn the lint off. The word
    // lists are English, so another `language` turns the lint off.
    let lint_on = facts.english
        && [
            Code::RequirementKeyword,
            Code::EarsPattern,
            Code::SentenceLength,
            Code::VagueTerm,
            Code::PassiveVoice,
        ]
        .iter()
        .any(|&code| config.severity(code, Some(d.root)).is_some());
    if !lint_on {
        return out;
    }
    for &ri in &d.reqs {
        let r = &p.reqs[ri];
        for (code, sub, msg) in crate::lint::lint(&r.text, config.max_words) {
            let target = if sub.is_empty() {
                r.key.clone()
            } else {
                format!("{}:{sub}", r.key)
            };
            out.push(diag(d, r.line, code, target, format!("`{}`: {msg}", r.key)));
        }
    }
    out
}

/// Returns true if a document links a requirement (or a document with
/// requirements), a decision or a procedure.
fn links_knowledge(c: &Corpus, d: &Doc) -> bool {
    let useful = |t: u32| {
        let ty = c.docs[t as usize].ty;
        ty.has_requirements() || matches!(ty, DocType::Decision | DocType::Procedure)
    };
    d.parsed.refs.iter().any(|r| match r.kind {
        RefKind::Wiki => matches!(c.resolve(&r.target), Res::Found(t, _, true) if useful(t)),
        RefKind::Link | RefKind::Image => {
            matches!(c.resolve_link(&d.path, &r.target), LinkRes::File(_, Some(t), _) if useful(t))
        }
    })
}

/// SD611: an accepted decision links a document of the type `intent` in its
/// locked text, or in `depends-on`. You can delete a finished intent (SD202
/// allows it: its layer is `until-finished`), but you cannot remove the link from
/// the locked text. A link in an amendment, history or live section can change,
/// so SD611 does not report it. SD901 reports it.
/// Returns the intents that it reports. SD901 skips these pairs.
fn decision_links_change(config: &Config, c: &Corpus, d: &Doc, out: &mut Vec<Diag>) -> Vec<u32> {
    let p = &d.parsed;
    let live = crate::model::amendment_chunks(
        &p.chunks,
        &config.roots[d.root].live_sections[d.ty as usize],
    );
    let editable = |line: u32| {
        p.chunks
            .iter()
            .rposition(|ch| ch.line <= line)
            .is_some_and(|i| {
                d.amendments.contains(&i) || d.history.contains(&i) || live.contains(&i)
            })
    };
    let mut seen: Vec<u32> = Vec::new();
    let mut hit = |t: u32, line: u32| {
        let target = &c.docs[t as usize];
        if target.ty == DocType::Intent && !seen.contains(&t) {
            seen.push(t);
            out.push(diag(
                d,
                line,
                Code::DecisionLinksChange,
                target.id.clone(),
                format!(
                    "the accepted decision links the intent `{}`; an intent can be deleted, and the locked text of the decision cannot drop the link",
                    target.id
                ),
            ));
        }
    };
    for r in &p.refs {
        if editable(r.line) {
            continue;
        }
        match r.kind {
            RefKind::Wiki => {
                if let Res::Found(t, _, _) = c.resolve(&r.target) {
                    hit(t, r.line);
                }
            }
            RefKind::Link => {
                if let LinkRes::File(_, Some(t), _) = c.resolve_link(&d.path, &r.target) {
                    hit(t, r.line);
                }
            }
            RefKind::Image => {}
        }
    }
    for (t, line) in &d.depends_on {
        if let Some(i) = found(c.resolve(t)) {
            hit(i, *line);
        }
    }
    seen
}

/// SD901: a document of layer X references a document of layer Y, X is more
/// stable than Y, and Y does not list X in `linked-from`. One diagnostic for
/// each (document, target) pair. `hits` has the resolved references of the
/// document. `locked` has the intents that SD611 reports for it.
fn link_direction(
    c: &Corpus,
    d: &Doc,
    facts: &Facts,
    hits: &[(u32, u32)],
    locked: &[u32],
    out: &mut Vec<Diag>,
) {
    let Some(x) = c
        .by_path
        .get(&d.path)
        .and_then(|&i| facts.layers[i as usize])
    else {
        return;
    };
    let mut seen: Vec<u32> = Vec::new();
    for &(t, line) in hits {
        let target = &c.docs[t as usize];
        let Some(y) = facts.layers[t as usize] else {
            continue;
        };
        if target.moved_to.is_some()
            || seen.contains(&t)
            || locked.contains(&t)
            || !facts.bad[x * facts.n_layers + y]
        {
            continue;
        }
        seen.push(t);
        out.push(diag(
            d,
            line,
            Code::LinkDirection,
            target.id.clone(),
            format!(
                "`{}` (layer `{}`) references `{}` (layer `{}`), a less stable layer that does not list `{}` in `linked-from`",
                d.id,
                facts.layer_name(x),
                target.id,
                facts.layer_name(y),
                facts.layer_name(x),
            ),
        ));
    }
}

/// SD903 and SD904: the history and the forbidden synonyms in the
/// requirements of a `rule`, a `requirements` and an `interface` document.
fn history_and_terms(config: &Config, d: &Doc, facts: &Facts, out: &mut Vec<Diag>) {
    if !matches!(
        d.ty,
        DocType::Rule | DocType::Requirements | DocType::Interface
    ) || d.type_failed
        || d.generated.is_some()
    {
        return;
    }
    let history_on = config
        .severity(Code::HistoryInRequirements, Some(d.root))
        .is_some();
    let terms_on =
        !facts.terms.is_empty() && config.severity(Code::TermForbidden, Some(d.root)).is_some();
    if !history_on && !terms_on {
        return;
    }
    for &ri in &d.reqs {
        let r = &d.parsed.reqs[ri];
        let text = crate::lint::prose(&r.text);
        let ws = if (history_on && !facts.history.is_empty()) || terms_on {
            crate::lint::words(&text)
        } else {
            Vec::new()
        };
        if history_on {
            for m in crate::lint::history(&text, &ws, &facts.history) {
                out.push(diag(
                    d,
                    r.line,
                    Code::HistoryInRequirements,
                    format!("{}:{m}", r.key),
                    format!(
                        "`{}`: the requirement has history (`{m}`); write the current rule only",
                        r.key
                    ),
                ));
            }
        }
        if terms_on {
            for (synonym, term) in facts.terms.find(&ws) {
                out.push(diag(
                    d,
                    r.line,
                    Code::TermForbidden,
                    format!("{}:{}", r.key, synonym.to_lowercase()),
                    format!(
                        "`{}`: `{synonym}` is a forbidden synonym; use the term `{term}`",
                        r.key
                    ),
                ));
            }
        }
    }
}

/// Reports a reference that does not resolve. Returns the document that it
/// names, if it names one.
fn id_ref(c: &Corpus, d: &Doc, target: &str, line: u32, out: &mut Vec<Diag>) -> Option<u32> {
    match c.resolve(target) {
        Res::Found(i, _, true) => return Some(i),
        Res::Found(i, Some(frag), false) => {
            out.push(diag(
                d,
                line,
                Code::UnresolvedReference,
                target,
                format!(
                    "`{}` has no section or requirement `{frag}`",
                    c.docs[i as usize].id
                ),
            ));
            return Some(i);
        }
        Res::Found(i, ..) => return Some(i),
        Res::Missing => out.push(diag(
            d,
            line,
            Code::UnresolvedReference,
            target,
            format!("`{target}` names no document"),
        )),
        Res::Ambiguous(v) => out.push(diag(
            d,
            line,
            Code::AmbiguousReference,
            target,
            format!("`{target}` names {} documents", v.len()),
        )),
        Res::CrossRepo(repo) => {
            let known = c.repos.contains(&repo);
            let why = if known {
                "has no pinned index yet"
            } else {
                "is not listed under `[repos]` in spec-dna.toml"
            };
            out.push(diag(
                d,
                line,
                Code::CrossRepoUnpinned,
                target,
                format!("`{target}` points into repository `{repo}`, which {why}"),
            ))
        }
    }
    None
}

/// True if SD202 allows the deletion of a base document. The layer of the
/// document in the base tree decides: a layer with `keep = "until-finished"`
/// allows it. In a `forever` layer, only the field `was-type` (`note` or
/// `change`, written by `upgrade`) allows it. The status gives no exemption.
fn deletion_allowed(config: &Config, b: &Doc) -> bool {
    if matches!(b.was_type.as_deref(), Some("note" | "change")) {
        return true;
    }
    config
        .layers
        .layer_of(rel_in_root(config, b), b.ty)
        .is_some_and(|i| config.layers.layers()[i].keep == Keep::UntilFinished)
}

/// Returns the diagnostics that compare the base tree with the head tree. All
/// of them are new by definition.
pub fn transitions(config: &Config, base: &Corpus, head: &Corpus) -> Vec<Diag> {
    let mut out = Vec::new();
    for b in &base.docs {
        // Find the same document in head: by its id, or by an alias that a move kept.
        let h = match head.lookup.get(&canonical(&b.id)).map(Vec::as_slice) {
            Some([i]) => Some(&head.docs[*i as usize]),
            Some(_) => None,
            None => {
                let deletable = deletion_allowed(config, b);
                if !deletable && !head.files.contains(&b.path) {
                    out.push(Diag {
                        path: b.path.clone(),
                        line: 1,
                        code: Code::IdRemoved,
                        severity: Code::IdRemoved.severity(),
                        doc: b.id.clone(),
                        target: b.id.clone(),
                        message: format!(
                            "`{}` was removed without keeping its id as an alias",
                            b.id
                        ),
                    });
                } else if !deletable {
                    out.push(Diag {
                        path: b.path.clone(),
                        line: 1,
                        code: Code::IdRemoved,
                        severity: Code::IdRemoved.severity(),
                        doc: b.id.clone(),
                        target: b.id.clone(),
                        message: format!("`{}` is no longer a document id", b.id),
                    });
                }
                continue;
            }
        };
        let Some(h) = h else { continue };
        if h.ty == b.ty
            && let (Some(from), Some(to)) = (b.status.as_deref(), h.status.as_deref())
            && !transition_ok(b.ty, from, to)
        {
            out.push(diag(
                h,
                1,
                Code::StatusTransition,
                format!("{from}->{to}"),
                format!("status cannot move from `{from}` to `{to}`"),
            ));
        }
        if b.ty == DocType::Decision
            && b.status_str() == "accepted"
            && h.ty == DocType::Decision
            && h.frozen_hash != b.frozen_hash
        {
            out.push(diag(
                h,
                1,
                Code::DecisionFrozen,
                "",
                "the text outside the amendment and live sections changed after the decision was accepted",
            ));
        }
        let (soft, hard) = config.budgets[h.ty as usize];
        let chars = h.parsed.body_chars;
        if soft > 0
            && !h.is_exempt_from_budgets()
            && chars > soft
            && (hard == 0 || chars <= hard)
            && chars > b.parsed.body_chars
        {
            out.push(diag(
                h,
                1,
                Code::BudgetGrew,
                "",
                format!(
                    "over budget and grew from {} to {} characters",
                    b.parsed.body_chars, h.parsed.body_chars
                ),
            ));
        }
    }
    // Find new amendment text that can change the outcome of a decision.
    for h in head.docs.iter().filter(|d| d.ty == DocType::Decision) {
        let base_hashes: HashSet<[u8; 8]> = match head_to_base(base, h) {
            Some(b) => b
                .amendments
                .iter()
                .map(|&i| b.parsed.chunks[i].hash)
                .collect(),
            None => HashSet::new(),
        };
        for &i in &h.amendments {
            let ch = &h.parsed.chunks[i];
            if !base_hashes.contains(&ch.hash)
                && ch.flags & (FLAG_REQUIREMENT | FLAG_BCP14 | FLAG_STATUS_WORD) != 0
            {
                out.push(diag(
                    h,
                    ch.line,
                    Code::AmendmentOutcome,
                    ch.anchor.clone(),
                    "new amendment text states a requirement, a BCP 14 keyword or a status word",
                ));
            }
        }
    }
    let mut out = apply_severity(config, out);
    out.sort();
    out
}

/// Applies the hard budget against the base (ADR 0001, amendments of
/// 2026-11-08 and 0.3.0).
///
/// A document over the hard budget is an error (SD702) in one of these cases:
///
/// - It is new.
/// - It was under the budget in the base.
/// - It grew by more than `[budgets] growth` characters.
///
/// A document that was over the budget in the base and grew by at most that
/// allowance gives a warning (SD703). Thus a task can add an amendment or a
/// registry row without a split. A document that did not grow gives a warning
/// (SD704). A root with `enforce-budgets = false` gives SD704 in all cases. A
/// stub and a generated index have no budget. Returns each diagnostic with its
/// "new" flag.
fn hard_budget(config: &Config, base: Option<&Corpus>, head: &Corpus) -> Vec<(Diag, bool)> {
    let mut out = Vec::new();
    for h in &head.docs {
        let (_, hard) = config.budgets[h.ty as usize];
        let chars = h.parsed.body_chars;
        if hard == 0 || chars <= hard || h.is_exempt_from_budgets() {
            continue;
        }
        let before = base
            .and_then(|b| head_to_base(b, h))
            .map(|b| b.parsed.body_chars);
        let mut small = false;
        // A document that a split moved out of a document over the hard budget
        // (its section aliases name that base document) brings no new text.
        let origin = base.and_then(|b| split_origin(config, b, h));
        let (why, new) = match before {
            None if origin.is_some() => (
                format!(
                    "split out of `{}`, which was over the hard budget in the base",
                    origin.as_deref().unwrap_or("")
                ),
                true,
            ),
            None => ("a new document".to_string(), true),
            Some(b) if b <= hard => (format!("{b} characters in the base"), true),
            Some(b) if chars > b && chars - b <= config.growth => {
                small = true;
                (
                    format!(
                        "grew by {} characters, at most {} in one change",
                        chars - b,
                        config.growth
                    ),
                    true,
                )
            }
            Some(b) if chars > b => (format!("grew from {b} characters in the base"), true),
            Some(_) => ("no growth against the base".to_string(), false),
        };
        let enforce = config.roots[h.root].enforce_budgets;
        let (code, why) = if !enforce {
            (
                Code::BudgetHardKept,
                format!("{why}; the root does not enforce budgets"),
            )
        } else if before.is_none() && origin.is_some() {
            (Code::BudgetHardKept, why)
        } else if small {
            (Code::BudgetGrew, why)
        } else if new {
            (Code::BudgetHard, why)
        } else {
            (Code::BudgetHardKept, why)
        };
        out.push((
            diag(
                h,
                1,
                code,
                "",
                format!("{chars} characters, over the hard budget of {hard} ({why})"),
            ),
            new,
        ));
    }
    out
}

/// The base document that the first section alias of `h` names: the source
/// of a split.
fn split_source<'a>(base: &'a Corpus, h: &Doc) -> Option<&'a Doc> {
    h.aliases.iter().find_map(|a| {
        let (id, _) = a.split_once('#')?;
        match base.lookup.get(&canonical(id))?.as_slice() {
            [i] => Some(&base.docs[*i as usize]),
            _ => None,
        }
    })
}

/// The base document over its hard budget that a section alias of `h`
/// names: `h` holds text that a split moved out of it.
fn split_origin(config: &Config, base: &Corpus, h: &Doc) -> Option<String> {
    h.aliases.iter().find_map(|a| {
        let (id, _) = a.split_once('#')?;
        let [i] = base.lookup.get(&canonical(id))?.as_slice() else {
            return None;
        };
        let b = &base.docs[*i as usize];
        let (_, hard) = config.budgets[b.ty as usize];
        (hard > 0 && b.parsed.body_chars > hard).then(|| b.id.clone())
    })
}

/// The root of the document of a diagnostic.
fn c_root(c: &Corpus, d: &Diag) -> usize {
    c.by_path
        .get(&d.path)
        .map_or(0, |&i| c.docs[i as usize].root)
}

fn head_to_base<'a>(base: &'a Corpus, h: &Doc) -> Option<&'a Doc> {
    let names = std::iter::once(&h.id).chain(&h.aliases);
    for n in names {
        if let Some([i]) = base.lookup.get(&canonical(n)).map(Vec::as_slice) {
            return Some(&base.docs[*i as usize]);
        }
    }
    None
}

fn transition_ok(ty: DocType, from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    // A rule, an interface and a procedure have the states of a requirements
    // document (ADR 0001, amendment of 0.2.0).
    let spec_like = matches!(
        ty,
        DocType::Requirements | DocType::Interface | DocType::Procedure | DocType::Rule
    );
    let rank = |s: &str| -> Option<u8> {
        Some(match (ty, s) {
            (_, "draft") if spec_like => 0,
            (_, "active") if spec_like => 1,
            (_, "deprecated") if spec_like => 2,
            (_, "superseded" | "retired") if spec_like => 3,
            (DocType::Decision, "proposed")
            | (DocType::Intent | DocType::Spec | DocType::Plan, "draft") => 0,
            (DocType::Decision | DocType::Intent, "accepted")
            | (DocType::Spec | DocType::Plan, "approved") => 1,
            (DocType::Decision, "superseded" | "rejected") | (DocType::Intent, "closed") => 2,
            _ => return None,
        })
    };
    let terminal = |s: &str| matches!(s, "superseded" | "retired" | "rejected" | "closed");
    if ty == DocType::Decision && from == "accepted" && to == "rejected" {
        return false;
    }
    match (rank(from), rank(to)) {
        (Some(a), Some(b)) => !terminal(from) && b > a,
        _ => true, // another check reports an invalid status
    }
}

/// SD921: the diff changes a requirement of a `forever` layer, and it touches
/// no file under the folder of the `intent` layer. A requirement is a list
/// item of a `rule`, `requirements` or `interface` document. It changed if its
/// key has another `frozen` hash, and it was added or removed if its key has
/// no match on the other side. `Corpus::resolve` applies the aliases, so a move
/// that keeps the hashes gives no diagnostic. The check compares the base with
/// the head only. It runs only for `check --base`, because a base that is HEAD gives a
/// false SD921 when the branch added its intent in an earlier commit. All
/// diagnostics are new. Each diagnostic names one document.
pub fn truth_changed(
    config: &Config,
    base: (&Corpus, &Snapshot),
    head: (&Corpus, &Snapshot),
) -> Vec<Diag> {
    if config.is_off(Code::TruthChangedWithoutChange) {
        return Vec::new();
    }
    let (base, base_snap) = base;
    let (head, head_snap) = head;
    if intent_changed(config, base_snap, head_snap) {
        return Vec::new();
    }
    // A requirement that spec-dna reads: of a document of the 3 types in a
    // layer that keeps its documents forever.
    let truth = |d: &Doc| {
        matches!(
            d.ty,
            DocType::Rule | DocType::Requirements | DocType::Interface
        ) && !d.type_failed
            && d.moved_to.is_none()
            && d.generated.is_none()
            && config
                .layers
                .layer_of(rel_in_root(config, d), d.ty)
                .is_some_and(|i| config.layers.layers()[i].keep == Keep::Forever)
    };
    #[derive(Default)]
    struct Change {
        path: String,
        line: u32,
        changed: Vec<String>,
        added: Vec<String>,
        removed: Vec<String>,
    }
    let mut by_doc: BTreeMap<String, Change> = BTreeMap::new();
    let mut matched: HashSet<(u32, usize)> = HashSet::new();
    for b in base.docs.iter().filter(|d| truth(d)) {
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
                        let e = by_doc.entry(h.id.clone()).or_default();
                        e.path.clone_from(&h.path);
                        e.line = if e.line == 0 { hr.line } else { e.line };
                        e.changed.push(hr.key.clone());
                    }
                }
                None => {
                    // The document can be in the head, without the key.
                    let owner = match head.resolve(&b.id) {
                        Res::Found(hd, None, _) => &head.docs[hd as usize],
                        _ => b,
                    };
                    let e = by_doc.entry(owner.id.clone()).or_default();
                    e.path.clone_from(&owner.path);
                    e.line = if e.line == 0 && std::ptr::eq(owner, b) {
                        r.line
                    } else {
                        e.line
                    };
                    e.removed.push(r.key.clone());
                }
            }
        }
    }
    for (hi, h) in head.docs.iter().enumerate().filter(|(_, d)| truth(d)) {
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
                let e = by_doc.entry(h.id.clone()).or_default();
                e.path.clone_from(&h.path);
                e.line = if e.line == 0 { r.line } else { e.line };
                e.added.push(r.key.clone());
            }
        }
    }
    let list = |keys: &[String]| {
        let shown: Vec<String> = keys.iter().take(3).map(|k| format!("`{k}`")).collect();
        let more = keys.len().saturating_sub(3);
        let mut s = shown.join(", ");
        if more > 0 {
            s.push_str(&format!(" and {more} more"));
        }
        s
    };
    let out = by_doc
        .into_iter()
        .map(|(id, e)| {
            let mut parts = Vec::new();
            for (what, keys) in [
                ("changed", &e.changed),
                ("added", &e.added),
                ("removed", &e.removed),
            ] {
                if !keys.is_empty() {
                    parts.push(format!("{what} {}", list(keys)));
                }
            }
            Diag {
                path: e.path,
                line: e.line.max(1),
                code: Code::TruthChangedWithoutChange,
                doc: id,
                target: String::new(),
                message: format!(
                    "requirements {}, and no file of the intent folder changed",
                    parts.join("; ")
                ),
                severity: Code::TruthChangedWithoutChange.severity(),
            }
        })
        .collect();
    apply_severity(config, out)
}

/// True if a file under the folder of the `intent` layer is added, deleted or
/// changed. A head file with no blob id (an edit of the working tree) counts as
/// a change. A project with no `intent` layer has no such folder.
fn intent_changed(config: &Config, base: &Snapshot, head: &Snapshot) -> bool {
    let Some(dir) = config
        .layers
        .layer_of_type(DocType::Intent)
        .and_then(|i| config.layers.layers()[i].dir())
    else {
        return false;
    };
    let prefixes: Vec<String> = config
        .roots
        .iter()
        .map(|r| {
            if r.path.is_empty() {
                format!("{dir}/")
            } else {
                format!("{}/{dir}/", r.path)
            }
        })
        .collect();
    let under = |p: &str| prefixes.iter().any(|x| p.starts_with(x.as_str()));
    let before: HashMap<&str, Option<&str>> = base
        .files
        .iter()
        .filter(|e| under(&e.path))
        .map(|e| (e.path.as_str(), e.blob.as_deref()))
        .collect();
    let mut seen = 0usize;
    for e in head.files.iter().filter(|e| under(&e.path)) {
        match before.get(e.path.as_str()) {
            None => return true,
            Some(b) => {
                seen += 1;
                if e.blob.is_none() || e.blob.as_deref() != *b {
                    return true;
                }
            }
        }
    }
    seen != before.len()
}

/// The result of the gate against the base.
pub struct Outcome {
    pub head: Vec<Diag>,
    /// One flag for each index into `head`: true if the base does not have
    /// that diagnostic.
    pub new: Vec<bool>,
}

pub fn gate(config: &Config, base: Option<&Corpus>, head: &Corpus) -> Outcome {
    let mut head_diags = diagnose(config, head);
    let budget: Vec<(Diag, bool)> = hard_budget(config, base, head)
        .into_iter()
        .filter_map(|(mut d, n)| {
            d.severity = config.severity(d.code, Some(c_root(head, &d)))?;
            Some((d, n))
        })
        .collect();
    let Some(base) = base else {
        let mut new = vec![true; head_diags.len()];
        for (d, n) in budget {
            head_diags.push(d);
            new.push(n);
        }
        return Outcome {
            head: head_diags,
            new,
        };
    };
    let base_diags = diagnose(config, base);
    let mut base_fp: HashMap<(Code, &str, &str), usize> = HashMap::new();
    for d in &base_diags {
        *base_fp.entry(d.fingerprint()).or_default() += 1;
    }
    // A moved document keeps its old id as an alias. Compare it under the old
    // id. A document that a split made has the section aliases of its source.
    // Compare it under the id of the source. Then a diagnostic that moved with
    // the text is not new.
    let renamed: HashMap<&str, &str> = head
        .docs
        .iter()
        .filter(|h| !base.lookup.contains_key(&canonical(&h.id)))
        .filter_map(|h| {
            head_to_base(base, h)
                .or_else(|| split_source(base, h))
                .map(|b| (h.id.as_str(), b.id.as_str()))
        })
        .collect();
    let mut new: Vec<bool> = head_diags
        .iter()
        .map(|d| {
            let doc = renamed
                .get(d.doc.as_str())
                .copied()
                .unwrap_or(d.doc.as_str());
            (d.code, doc, d.target.as_str())
        })
        .map(|fp| match base_fp.get_mut(&fp) {
            Some(n) if *n > 0 => {
                *n -= 1;
                false
            }
            _ => true,
        })
        .collect();
    for (d, n) in budget {
        head_diags.push(d);
        new.push(n);
    }
    let trans = transitions(config, base, head);
    new.extend(std::iter::repeat_n(true, trans.len()));
    head_diags.extend(trans);
    Outcome {
        head: head_diags,
        new,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_move_forward_only() {
        assert!(transition_ok(DocType::Requirements, "draft", "active"));
        assert!(transition_ok(DocType::Requirements, "active", "retired"));
        assert!(!transition_ok(DocType::Requirements, "active", "draft"));
        assert!(!transition_ok(
            DocType::Requirements,
            "retired",
            "superseded"
        ));
        assert!(transition_ok(DocType::Decision, "proposed", "accepted"));
        assert!(!transition_ok(DocType::Decision, "accepted", "rejected"));
        assert!(!transition_ok(DocType::Decision, "accepted", "proposed"));
        assert!(transition_ok(DocType::Intent, "accepted", "closed"));
        assert!(!transition_ok(DocType::Intent, "closed", "accepted"));
        assert!(transition_ok(DocType::Spec, "draft", "approved"));
        assert!(!transition_ok(DocType::Plan, "approved", "draft"));
        assert!(!transition_ok(DocType::Rule, "active", "draft"));
        assert!(!transition_ok(DocType::Procedure, "retired", "active"));
        assert!(transition_ok(DocType::Procedure, "draft", "active"));
    }
}
