//! `spec-dna check` applies the controls of ADR 0001 to one tree. It also
//! checks the transitions between the base tree and the head tree, and it
//! applies the gate against the base.

use crate::config::{Config, Evals};
use crate::diag::{Code, Diag};
use crate::evals;
use crate::evidence::{Evidence, OWED_REASONS, is_owed_reason, needs_evidence};
use crate::layer::Keep;
use crate::lint::Phrases;
use crate::model::{Corpus, Doc, DocType, LinkRes, Res, canonical};
use crate::parse::{FLAG_BCP14, FLAG_REQUIREMENT, FLAG_STATUS_WORD, RefKind};
use crate::repo::Snapshot;
use std::collections::{HashMap, HashSet};

pub(crate) fn diag(
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
    diagnose_against(config, c, None)
}

/// Like `diagnose`. `base` is the tree that the gate compares with. The
/// lifecycle checks of one tree use it to find a delta line that the head
/// already applies (`lifecycle::tree_checks`).
pub fn diagnose_against(config: &Config, c: &Corpus, base: Option<&Corpus>) -> Vec<Diag> {
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
    out.extend(rollup_diags(config, c, base));
    let mut out = apply_severity(config, out);
    // SD612 skips a reference that SD611 or SD901 reports in this run, so it
    // reads the diagnostics after their severity.
    let into_change = links_into_changes(config, c, &out);
    out.extend(apply_severity(config, into_change));
    out.extend(layer_limits(config, c, &layers));
    out.extend(crate::lifecycle::tree_checks(config, c, base, &layers));
    out.sort();
    out
}

/// The codes of the roll-up (design 0.8.0, D52): SD641 to SD644 for each
/// summary, and each `system` or `product` document with `rests-on`, and SD645
/// against `base`. The tree has the pins, so the gate sees a second stale pin of
/// one document as new: each finding names its pin as its target. The caller
/// applies the severity. A tree with no such document gives nothing.
fn rollup_diags(config: &Config, c: &Corpus, base: Option<&Corpus>) -> Vec<Diag> {
    use crate::rollup::Kind;
    const CODES: [Code; 5] = [
        Code::SummaryStale,
        Code::SummaryIncomplete,
        Code::PinInvalid,
        Code::SummaryNoArea,
        Code::SummaryRepinned,
    ];
    if CODES.iter().all(|code| config.is_off(*code)) {
        return Vec::new();
    }
    let roll = crate::rollup::Rollup::build(config, c);
    let mut out: Vec<Diag> = roll
        .all()
        .into_iter()
        .map(|(i, f)| {
            let code = match f.kind {
                Kind::Stale => Code::SummaryStale,
                Kind::Incomplete => Code::SummaryIncomplete,
                Kind::Invalid => Code::PinInvalid,
                Kind::NoArea => Code::SummaryNoArea,
            };
            diag(&c.docs[i as usize], f.line, code, f.target, f.message)
        })
        .collect();
    if let Some(base) = base {
        for i in crate::rollup::repinned(c, base) {
            let d = &c.docs[i as usize];
            let line = d
                .rests_on
                .as_ref()
                .and_then(|p| p.first())
                .map_or(1, |p| p.1);
            let message = if d.rests_on.is_none() && d.ty != DocType::Summary {
                format!(
                    "`{}` lost its field `rests-on` against the base and its text did not: the document is no roll-up document now, and its pins no longer stand for its text; check that this is right",
                    d.id
                )
            } else {
                format!(
                    "the pins of `{}` changed against the base and its text did not: a claim that the text still holds (`spec-dna refresh {0} --holds`); check that it does",
                    d.id
                )
            };
            out.push(diag(d, line, Code::SummaryRepinned, "", message));
        }
    }
    out
}

/// SD926: each item of `[change] approvers` that is the address of an agent or
/// a bot. The command `check` compares the findings of the head with the
/// findings of the base configuration, so an item is new when the base does
/// not list it. The caller applies the severity (`apply_severity`).
pub fn approver_agents(config: &Config) -> Vec<Diag> {
    config
        .change_approvers
        .iter()
        .filter(|a| crate::config::is_agent_address(a))
        .map(|a| Diag {
            path: crate::config::FILE_NAME.to_string(),
            line: config.change_approvers_line,
            code: Code::ApproverAgent,
            doc: crate::config::FILE_NAME.to_string(),
            target: a.clone(),
            message: format!(
                "`[change] approvers` lists `{a}`, the address of an agent or a bot; a commit of this author can come from an agent, so it proves no approval of a person"
            ),
            severity: Code::ApproverAgent.severity(),
        })
        .collect()
}

/// SD612: a reference from a document outside the folder of a change into the
/// folder, that step 0 of `finish` stops on. The references and the exceptions
/// are those of step 0 (`finish::refs_into`). `reported` has the diagnostics of
/// the run after their severity. A reference that SD611 or SD901 reports (the
/// same document, line and target) gives no SD612, so one reference gives
/// one warning. When SD901 is off, SD612 reports the reference.
fn links_into_changes(config: &Config, c: &Corpus, reported: &[Diag]) -> Vec<Diag> {
    let changes = crate::change::collect(c);
    let folders: Vec<&str> = changes
        .iter()
        .filter(|ch| crate::finish::valid_slug(&ch.slug))
        .map(|ch| ch.dir.as_str())
        .collect();
    if folders.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Diag> = Vec::new();
    for r in crate::finish::refs_into(config, c, &folders) {
        let d = &c.docs[r.doc as usize];
        if let Some(to) = r.to {
            let id = &c.docs[to as usize].id;
            if reported.iter().any(|x| {
                matches!(x.code, Code::DecisionLinksChange | Code::LinkDirection)
                    && x.path == d.path
                    && x.line == r.line
                    && x.target == *id
            }) {
                continue;
            }
        }
        let folder = folders[r.folder];
        let slug = folder.rsplit('/').next().unwrap_or(folder);
        let new = diag(
            d,
            r.line,
            Code::LinkIntoChange,
            r.target.clone(),
            format!(
                "`{}` names the folder `{folder}` of the change `{slug}`, and `finish {slug}` stops on it; name the change as plain text",
                r.target
            ),
        );
        if !out.contains(&new) {
            out.push(new);
        }
    }
    out
}

/// The layer of each document, as `layer list` shows it. A document with no
/// valid type is in no layer (D12).
pub(crate) fn doc_layers(config: &Config, c: &Corpus) -> Vec<Option<usize>> {
    c.docs.iter().map(|d| doc_layer(config, d)).collect()
}

/// The layer of one document (see `doc_layers`).
pub(crate) fn doc_layer(config: &Config, d: &Doc) -> Option<usize> {
    if d.has_layer() {
        config.layers.layer_of(rel_in_root(config, d), d.ty)
    } else {
        None
    }
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

    // An accepted decision cannot change its locked text (the body outside the
    // amendment, history and live sections). A reference there that no longer
    // resolves gives no SD101 or SD102: the author cannot fix it (a finished
    // intent leaves no document). SD611 already warned. `supersedes` and
    // `depends-on` are in the front matter, which the author can change.
    let editable = editable_chunks(config, d);
    let quiet = |line: u32| {
        editable
            .as_ref()
            .is_some_and(|e| in_locked_text(d, e, line))
    };
    for r in &p.refs {
        match r.kind {
            RefKind::Wiki => {
                if let Some(t) = id_ref(c, d, &r.target, r.line, quiet(r.line), &mut out)
                    && track
                {
                    hits.push((t, r.line));
                }
            }
            RefKind::Link | RefKind::Image => match c.resolve_link(&d.path, &r.target) {
                LinkRes::Missing(_) if quiet(r.line) => {}
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
        id_ref(c, d, t, *line, false, &mut out);
        return out;
    }
    for (t, line) in d.supersedes.iter() {
        if let Some(t) = id_ref(c, d, t, *line, false, &mut out)
            && track
        {
            hits.push((t, *line));
        }
    }
    for (t, line) in d.depends_on.iter() {
        if let Some(i) = id_ref(c, d, t, *line, false, &mut out)
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
    // The field `from` of an intent names the incident that the change
    // answers. It is a reference, as `depends-on` is.
    if let Some((t, line)) = &d.from
        && let Some(i) = id_ref(c, d, t, *line, false, &mut out)
        && track
    {
        hits.push((i, *line));
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
        // The marker `proof-owed` (design 0.8.0, D2): SD619 for a reason that
        // is not in the list, and for a debt on a requirement that has
        // evidence.
        if let Some(reason) = r.proof_owed.as_deref() {
            if !is_owed_reason(reason) {
                out.push(diag(
                    d,
                    r.line,
                    Code::ProofOwedInvalid,
                    r.key.clone(),
                    format!(
                        "`{}#{}`: the reason {} of `proof-owed` is not in the list: {}",
                        d.id,
                        r.key,
                        if reason.is_empty() {
                            "(empty)".to_string()
                        } else {
                            format!("`{reason}`")
                        },
                        OWED_REASONS.join(", ")
                    ),
                ));
            } else if needs_evidence(d)
                && let Some(at) = ev.of(di, d, ri).first()
            {
                out.push(diag(
                    d,
                    r.line,
                    Code::ProofOwedInvalid,
                    r.key.clone(),
                    format!(
                        "`{}#{}` has the marker `proof-owed: {reason}` and it has evidence ({at}); remove the marker",
                        d.id, r.key
                    ),
                ));
            }
        }
        // SD618 replaces SD608 only where SD618 is on: a marker must not hide
        // SD608 in a profile or a root that turns the proof debt off.
        if needs_evidence(d)
            && ev.of(di, d, ri).is_empty()
            && config
                .severity(Code::ProofOwed, config.root_of(&d.path))
                .is_some()
            && let Some(reason) = Evidence::owed_reason(d, ri)
        {
            // A debt and no evidence: SD618 replaces SD608 (Q5). A state code.
            out.push(diag(
                d,
                r.line,
                Code::ProofOwed,
                r.key.clone(),
                format!(
                    "`{}#{}` is active, it has no evidence, and its proof is owed (`{reason}`); add a test that covers it, or a `verified-by`, and remove the marker",
                    d.id, r.key
                ),
            ));
        } else if needs_evidence(d) && ev.of(di, d, ri).is_empty() {
            let message = match ev.partial_of(di, ri).first() {
                Some(at) => format!(
                    "`{}#{}` is active, and only a partial `covers:` tag names it ({at}); add a test that covers all of it, or a `verified-by`",
                    d.id, r.key
                ),
                None => format!(
                    "`{}#{}` is active but no test covers it (`covers:` tag or `verified-by`)",
                    d.id, r.key
                ),
            };
            out.push(diag(
                d,
                r.line,
                Code::RequirementUnverified,
                r.key.clone(),
                message,
            ));
        } else if needs_evidence(d)
            && let Some(why) = ev.unproven(di, d, ri)
        {
            // A tag names the requirement, and the test reports do not prove
            // it. A state code: it is never new (design 0.8.0, D1).
            out.push(diag(
                d,
                r.line,
                Code::RequirementUnproven,
                r.key.clone(),
                format!("`{}#{}` is active, and {why}", d.id, r.key),
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

/// The chunks of an accepted decision that can change in place: the amendment,
/// history and live sections. It returns `None` for any other document: its
/// text is not locked.
pub(crate) fn editable_chunks(config: &Config, d: &Doc) -> Option<Vec<usize>> {
    if d.ty != DocType::Decision || d.status_str() != "accepted" || d.generated.is_some() {
        return None;
    }
    let mut v = crate::model::amendment_chunks(
        &d.parsed.chunks,
        &config.roots[d.root].live_sections[d.ty as usize],
    );
    v.extend(&d.amendments);
    v.extend(&d.history);
    Some(v)
}

/// True if a line of an accepted decision is in none of the `editable` chunks.
/// The line of a reference of the body is in the locked text then (SD405). A
/// line of the front matter has no chunk. SD611 treats it as locked, but SD405
/// does not lock it, so SD101 does not use this function for it.
pub(crate) fn in_locked_text(d: &Doc, editable: &[usize], line: u32) -> bool {
    !d.parsed
        .chunks
        .iter()
        .rposition(|ch| ch.line <= line)
        .is_some_and(|i| editable.contains(&i))
}

/// A reference in the locked text of an accepted decision, with a target that
/// no longer exists. `check` gives no SD101 and no SD102 for it, because the
/// author cannot edit the text. `check --all` names it in an info line, and
/// `check --json` has it in `locked_links`. It is not a diagnostic: it has no
/// code, no severity and no effect on the exit code. An amendment section of
/// the same decision that names the target clears the line (`names_target`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedLink {
    pub path: String,
    pub line: u32,
    /// The target, as the text writes it.
    pub target: String,
    /// Where the amendment must point.
    pub pointer: String,
    /// The commit that deleted the folder of the change that the pointer names,
    /// or `None` if the history has no such commit.
    pub commit: Option<String>,
}

impl LockedLink {
    /// The info line of `check --all`.
    pub fn line_text(&self) -> String {
        format!(
            "{}:{}: info: the locked text links `{}`, which no longer exists; add an amendment that names `{}` and points to {}",
            self.path, self.line, self.target, self.target, self.pointer
        )
    }
}

/// The names of a place that is gone, as an amendment can write them: the
/// target as written (with no fragment), the id or the path of the place from
/// the root of the documents, and the path from the root of the repository,
/// each with and without `.md`.
fn place_names(target: &str, gone: &str, full: &str) -> Vec<String> {
    let mut names = vec![target.split('#').next().unwrap_or("").to_string()];
    for base in [gone, full] {
        let bare = base.strip_suffix(".md").unwrap_or(base);
        for name in [bare.to_string(), format!("{bare}.md")] {
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// True if a line of the decision is in an amendment section. The sections of
/// history and the live sections are not amendments.
fn in_amendment(d: &Doc, line: u32) -> bool {
    d.parsed
        .chunks
        .iter()
        .rposition(|ch| ch.line <= line)
        .is_some_and(|i| d.amendments.contains(&i))
}

/// True if an amendment section of the decision names the target that is
/// gone. A code span, or a word of prose with a `/`, on a line of the section
/// must equal one of the `names` (a `#` fragment of the token does not
/// count). For a change folder, the slug in a code span also names it.
fn names_target(d: &Doc, names: &[String], slug: Option<&str>) -> bool {
    let bare = |t: &str| t.split('#').next().unwrap_or("").to_string();
    let p = &d.parsed;
    p.code_spans
        .iter()
        .chain(&p.path_words)
        .filter(|t| in_amendment(d, t.line))
        .any(|t| names.contains(&bare(&t.text)))
        || slug.is_some_and(|s| {
            p.code_spans
                .iter()
                .any(|t| in_amendment(d, t.line) && t.text == s)
        })
}

/// The locked links of a tree: each document or link target of the locked
/// text of an accepted decision that is absent. A reference that names a
/// document that exists, with a section that is absent, is not in the list. A
/// target that an amendment of the decision names is not in the list either.
/// `ended` gets the folders of the changes (from the repository root) that the
/// remaining links name, and it gives the change that ended in each folder. The
/// function calls it once, and only if such a link exists.
pub fn locked_links(
    config: &Config,
    c: &Corpus,
    ended: &dyn Fn(&[String]) -> HashMap<String, crate::sdlc::EndedChange>,
) -> Vec<LockedLink> {
    // A link with the folder of its change, if the target was in one.
    let mut found: Vec<(LockedLink, Option<(String, String)>)> = Vec::new();
    for d in &c.docs {
        let Some(editable) = editable_chunks(config, d) else {
            continue;
        };
        let root = config.roots.get(d.root).map_or("", |r| r.path.as_str());
        let from_root = |rel: &str| {
            if root.is_empty() {
                rel.to_string()
            } else {
                format!("{root}/{rel}")
            }
        };
        for r in &d.parsed.refs {
            if !in_locked_text(d, &editable, r.line) {
                continue;
            }
            // The place of the target that is gone, to find its change.
            let (gone, full) = match r.kind {
                RefKind::Wiki => match c.resolve(&r.target) {
                    Res::Missing => {
                        let id = r.target.split('#').next().unwrap_or("").to_string();
                        let full = from_root(&id);
                        (id, full)
                    }
                    _ => continue,
                },
                RefKind::Link | RefKind::Image => match c.resolve_link(&d.path, &r.target) {
                    LinkRes::Missing(path) => {
                        let rel = match config.roots.get(d.root) {
                            Some(root) if !root.path.is_empty() => path
                                .strip_prefix(&format!("{}/", root.path))
                                .unwrap_or(&path)
                                .to_string(),
                            _ => path.clone(),
                        };
                        (rel, path)
                    }
                    _ => continue,
                },
            };
            let change = intent_slug(config, &gone);
            let slug = change.as_ref().map(|(_, s)| s.as_str());
            if names_target(d, &place_names(&r.target, &gone, &full), slug) {
                continue;
            }
            let link = LockedLink {
                path: d.path.clone(),
                line: r.line,
                target: r.target.clone(),
                pointer: match &change {
                    Some((_, slug)) => {
                        format!("the change `{slug}` (`spec-dna log` names its commit)")
                    }
                    None => "the new place of the text, or the reason for its removal".to_string(),
                },
                commit: None,
            };
            let folder = change.map(|(dir, slug)| (from_root(&format!("{dir}/{slug}")), slug));
            found.push((link, folder));
        }
    }
    let mut folders: Vec<String> = found
        .iter()
        .filter_map(|(_, f)| f.as_ref().map(|(folder, _)| folder.clone()))
        .collect();
    folders.sort();
    folders.dedup();
    let ended = if folders.is_empty() {
        HashMap::new()
    } else {
        ended(&folders)
    };
    let mut out: Vec<LockedLink> = Vec::new();
    for (mut link, folder) in found {
        if let Some((folder, slug)) = folder
            && let Some(e) = ended.get(&folder)
        {
            link.pointer = format!(
                "the change `{slug}` ({} in commit {} on {})",
                if e.finished { "finished" } else { "closed" },
                e.commit.get(..12).unwrap_or(&e.commit),
                e.date
            );
            link.commit = Some(e.commit.clone());
        }
        if !out.contains(&link) {
            out.push(link);
        }
    }
    out.sort();
    out
}

/// The folder of the layer `intent` and the name of the change whose folder
/// holds a path, if the path is in such a folder. `rel` is relative to the
/// root, and it has no extension for an id.
fn intent_slug(config: &Config, rel: &str) -> Option<(String, String)> {
    config
        .layers
        .layers()
        .iter()
        .filter(|l| l.types.contains(&DocType::Intent))
        .filter_map(|l| l.dir())
        .find_map(|dir| {
            let rest = rel.strip_prefix(dir)?.strip_prefix('/')?;
            let slug = rest.split('/').next().filter(|s| !s.is_empty())?;
            Some((dir.to_string(), slug.to_string()))
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
    let editable = editable_chunks(config, d).unwrap_or_default();
    let editable = |line: u32| !in_locked_text(d, &editable, line);
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
/// names, if it names one. With `quiet`, it does not report a document or a
/// section that is absent (SD101): this is the locked text of a decision.
fn id_ref(
    c: &Corpus,
    d: &Doc,
    target: &str,
    line: u32,
    quiet: bool,
    out: &mut Vec<Diag>,
) -> Option<u32> {
    match c.resolve(target) {
        Res::Found(i, _, true) => return Some(i),
        Res::Found(i, Some(_), false) if quiet => return Some(i),
        Res::Missing if quiet => {}
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

/// The size of one area (design 0.8.0, D55).
struct AreaSize {
    /// The folders of the area under the folder of the layer `requirements`.
    /// The root area has none.
    area: Vec<String>,
    /// The sum of the body characters.
    chars: u64,
    /// The root of the first document of the area.
    root: usize,
    /// The summary of the area, if it has one: the finding is on it.
    summary: Option<u32>,
}

/// The folders of a path, without the file name.
fn path_folders(rel: &str) -> Vec<String> {
    let mut parts: Vec<&str> = rel.split('/').collect();
    parts.pop();
    parts.into_iter().map(str::to_string).collect()
}

/// The size of each area that has a document (D55). An area is a folder of any
/// depth under the folder of the layer `requirements`, and that folder is the
/// root area. Its size is the sum of the body characters of the documents of
/// the layer under it that are not inactive. A stub, a generated index and a
/// document with no valid type do not count. For each sub-area that has a
/// summary, the sum counts the summary in place of the documents of the
/// sub-area: only the highest summary of a branch counts. The own summary of
/// an area is no part of its size. The areas come in the order of their
/// folders.
fn area_sizes(config: &Config, c: &Corpus) -> Vec<AreaSize> {
    let Some(layer) = config.layers.layer_of_type(DocType::Requirements) else {
        return Vec::new();
    };
    let dir = config.layers.layers()[layer].dir();
    // A document of the layer that is not under the folder is in the root area.
    let area_of = |d: &Doc| -> Vec<String> {
        let rel = rel_in_root(config, d);
        dir.and_then(|dir| rel.strip_prefix(dir)?.strip_prefix('/'))
            .map_or_else(Vec::new, path_folders)
    };
    // The documents: the area, the characters, the root.
    let mut docs: Vec<(Vec<String>, u64, usize)> = Vec::new();
    // The summaries of an area that has a folder: the document, the area, the
    // characters.
    let mut summaries: Vec<(u32, Vec<String>, u64)> = Vec::new();
    for (i, d) in c.docs.iter().enumerate() {
        if d.is_inactive() || d.is_exempt_from_budgets() {
            continue;
        }
        let chars = u64::from(d.parsed.body_chars);
        if d.ty == DocType::Summary {
            if let Some(a) = crate::rollup::summary_area(config, d)
                && crate::rollup::area_exists(config, c, &a)
            {
                summaries.push((i as u32, a, chars));
            }
        } else if doc_layer(config, d) == Some(layer) {
            docs.push((area_of(d), chars, d.root));
        }
    }
    let mut candidates: Vec<&[String]> = docs
        .iter()
        .flat_map(|(a, _, _)| (0..=a.len()).map(|n| &a[..n]))
        .collect();
    candidates.sort();
    candidates.dedup();
    let mut out = Vec::new();
    for area in candidates {
        // The summaries of the sub-areas that stand for their documents: the
        // highest one of each branch.
        let below: Vec<&(u32, Vec<String>, u64)> = summaries
            .iter()
            .filter(|(_, b, _)| b.len() > area.len() && b.starts_with(area))
            .collect();
        let top: Vec<&&(u32, Vec<String>, u64)> = below
            .iter()
            .filter(|(_, b, _)| {
                !below
                    .iter()
                    .any(|(_, x, _)| x.len() < b.len() && b.starts_with(x))
            })
            .collect();
        let mut chars: u64 = top.iter().map(|s| s.2).sum();
        let mut root = None;
        for (a, n, r) in &docs {
            if a.starts_with(area) && !top.iter().any(|s| a.starts_with(&s.1)) {
                chars += n;
                root.get_or_insert(*r);
            }
        }
        let Some(root) = root.or_else(|| top.first().map(|s| c.docs[s.0 as usize].root)) else {
            continue;
        };
        out.push(AreaSize {
            area: area.to_vec(),
            chars,
            root,
            summary: summaries
                .iter()
                .find(|(_, a, _)| a == area)
                .map(|(i, _, _)| *i),
        });
    }
    out
}

/// The budget of an area against the base (design 0.8.0, D55). It mirrors
/// SD701 to SD704, and it counts an area as `area_sizes` does, with the
/// configuration of the head for both trees. An area of the head is the area of
/// the base that has the same folders, so a folder that moved is a new area.
///
/// An area over the hard budget is SD706 in one of these cases: it is new, it
/// was under the budget in the base, or it grew by more than `[budgets]
/// growth` characters. An area over it in the base that grew by at most that
/// allowance is SD707, and it is new. An area that did not grow is SD707, and
/// it is not new. An area over the soft budget and not over the hard budget is
/// SD705, and it is new if it was not over the soft budget in the base. A
/// table with no `[budgets.area]` gives nothing, and a field that is 0 turns
/// its budget off. The finding is on the summary of the area, or on its folder
/// if it has none. The severity is the one of the root of the area. Returns each
/// diagnostic with its "new" flag.
fn area_budget(config: &Config, base: Option<&Corpus>, head: &Corpus) -> Vec<(Diag, bool)> {
    let Some((soft, hard)) = config.area_budget else {
        return Vec::new();
    };
    if soft == 0 && hard == 0 {
        return Vec::new();
    }
    let before: HashMap<Vec<String>, u64> = base
        .map(|b| {
            area_sizes(config, b)
                .into_iter()
                .map(|s| (s.area, s.chars))
                .collect()
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    for s in area_sizes(config, head) {
        let (soft, hard) = (u64::from(soft), u64::from(hard));
        let before = before.get(&s.area).copied();
        let chars = s.chars;
        let (code, new, text) = if hard > 0 && chars > hard {
            let (code, new, why) = match before {
                None => (Code::AreaBudgetHard, true, "a new area".to_string()),
                Some(b) if b <= hard => (
                    Code::AreaBudgetHard,
                    true,
                    format!("{b} characters in the base"),
                ),
                Some(b) if chars > b && chars - b <= u64::from(config.growth) => (
                    Code::AreaBudgetHardKept,
                    true,
                    format!(
                        "grew by {} characters, at most {} in one change",
                        chars - b,
                        config.growth
                    ),
                ),
                Some(b) if chars > b => (
                    Code::AreaBudgetHard,
                    true,
                    format!("grew from {b} characters in the base"),
                ),
                Some(_) => (
                    Code::AreaBudgetHardKept,
                    false,
                    "no growth against the base".to_string(),
                ),
            };
            (code, new, format!("over the hard budget of {hard} ({why})"))
        } else if soft > 0 && chars > soft {
            (
                Code::AreaBudgetSoft,
                before.is_none_or(|b| b <= soft),
                format!("over the soft budget of {soft}"),
            )
        } else {
            continue;
        };
        let Some(severity) = config.severity(code, Some(s.root)) else {
            continue;
        };
        let target = if s.area.is_empty() {
            ".".to_string()
        } else {
            s.area.join("/")
        };
        let message = format!("area `{target}` has {chars} characters, {text}");
        let (path, doc) = match s.summary {
            Some(i) => {
                let d = &head.docs[i as usize];
                (d.path.clone(), d.id.clone())
            }
            None => {
                let dir = config
                    .layers
                    .layer_of_type(DocType::Requirements)
                    .and_then(|l| config.layers.layers()[l].dir());
                let parts = std::iter::once(config.roots[s.root].path.as_str())
                    .chain(dir)
                    .chain(s.area.iter().map(String::as_str))
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>();
                let folder = if parts.is_empty() {
                    ".".to_string()
                } else {
                    parts.join("/")
                };
                (folder.clone(), folder)
            }
        };
        out.push((
            Diag {
                path,
                line: 1,
                code,
                doc,
                target,
                message,
                severity,
            },
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

pub(crate) fn head_to_base<'a>(base: &'a Corpus, h: &Doc) -> Option<&'a Doc> {
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
/// the head only. The base is the lifecycle base (design 0.6.0, "Gates"): a
/// base that is `HEAD` gives a false SD921 when the branch added its intent in
/// an earlier commit. All diagnostics are new. Each diagnostic names one
/// document.
///
/// A document that `exempt` matches gives nothing (D19). `exempt` has the
/// `Files` patterns of the changes whose plan covers the diff
/// (`lifecycle::BaseReport::exempt`).
pub fn truth_changed(
    config: &Config,
    base: (&Corpus, &Snapshot),
    head: (&Corpus, &Snapshot),
    exempt: &[String],
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
        crate::lifecycle::requirement_doc(d)
            && config
                .layers
                .layer_of(rel_in_root(config, d), d.ty)
                .is_some_and(|i| config.layers.layers()[i].keep == Keep::Forever)
            && !crate::change::in_scope(exempt, &d.path)
    };
    let changes = crate::lifecycle::requirement_changes(base, head, truth);
    let list = |keys: &[&str]| {
        let shown: Vec<String> = keys.iter().take(3).map(|k| format!("`{k}`")).collect();
        let more = keys.len().saturating_sub(3);
        let mut s = shown.join(", ");
        if more > 0 {
            s.push_str(&format!(" and {more} more"));
        }
        s
    };
    let out = crate::lifecycle::by_document(&changes)
        .into_iter()
        .map(|(id, list_of)| {
            let of = |kind: crate::lifecycle::ReqChangeKind| -> Vec<&str> {
                list_of
                    .iter()
                    .filter(|c| c.kind == kind)
                    .map(|c| c.key.as_str())
                    .collect()
            };
            let mut parts = Vec::new();
            for (what, kind) in [
                ("changed", crate::lifecycle::ReqChangeKind::Changed),
                ("added", crate::lifecycle::ReqChangeKind::Added),
                ("removed", crate::lifecycle::ReqChangeKind::Removed),
            ] {
                let keys = of(kind);
                if !keys.is_empty() {
                    parts.push(format!("{what} {}", list(&keys)));
                }
            }
            let line = list_of
                .iter()
                .map(|c| c.line)
                .find(|&l| l != 0)
                .unwrap_or(0);
            Diag {
                path: list_of.last().map(|c| c.path.clone()).unwrap_or_default(),
                line: line.max(1),
                code: Code::TruthChangedWithoutChange,
                doc: id.to_string(),
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
    let prefixes: Vec<String> = crate::lifecycle::intent_dirs(config)
        .iter()
        .map(|d| format!("{d}/"))
        .collect();
    if prefixes.is_empty() {
        return false;
    }
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

/// The files of one tree under `[evals] dir`, for SD506.
#[derive(Default)]
pub struct EvalTree {
    /// Each path of the tree under `<dir>/`, from the repository root, sorted.
    pub paths: Vec<String>,
    /// The bytes of each `*.json` file directly in `<dir>`. A file that the
    /// reader could not read is absent. At most `evals::MAX_BYTES + 1` bytes
    /// of a file are enough for the check.
    pub json: Vec<(String, Vec<u8>)>,
    /// The bytes of each result file `skills-<stamp>.json` of `<dir>/results`,
    /// for SD507 and `eval verdict`.
    pub results: Vec<(String, Vec<u8>)>,
}

/// SD506: each eval file of the tree that `evals::parse_eval` rejects, and the
/// absent file `<dir>/check.sh`. It reads no text of `check.sh`, of a fixture
/// or of a file in a subfolder. The diagnostic is on the file, on line 1, with
/// the key of the broken rule as the target. Its `doc` is the path of the file,
/// as for the harness files, so that two files that break the same rule have
/// two fingerprints (design 0.6.0, D94). The severity is the default of the
/// code. The caller applies the rules.
pub fn evals_invalid(evals: &Evals, tree: &EvalTree) -> Vec<Diag> {
    let dir = &evals.dir;
    let under = |path: &str| {
        let prefix = format!("{path}/");
        tree.paths.iter().any(|p| p.starts_with(&prefix))
    };
    let on_file = |path: &str, key: &str, message: String| Diag {
        path: path.to_string(),
        line: 1,
        code: Code::EvalsInvalid,
        doc: path.to_string(),
        target: key.to_string(),
        message,
        severity: Code::EvalsInvalid.severity(),
    };
    let mut out = Vec::new();
    let prefix = format!("{dir}/");
    for (path, bytes) in &tree.json {
        // The same files as `eval run`: `<id>.json` directly in the folder.
        let Some(stem) = path
            .strip_prefix(&prefix)
            .filter(|n| !n.contains('/'))
            .and_then(|n| n.strip_suffix(".json"))
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        if let Err(e) = evals::parse_eval(stem, bytes, evals, &under) {
            out.push(on_file(path, &e.key, e.message));
        }
    }
    let check = format!("{dir}/{}", evals::CHECK_FILE);
    if !tree.paths.contains(&check) {
        out.push(on_file(
            &check,
            evals::CHECK_FILE,
            "the check file is absent; `eval run` sources it".to_string(),
        ));
    }
    out
}

/// SD507: each skill of `[harness] skills` that is off by default (ADR 0023).
/// The rule is `verdict::on_by_default` with the 7 skills of D2, the verdict
/// and the constant `DEFAULT_ON_NEEDS_VERDICT_ON`. The verdict of the newest
/// result file of the project that holds the skill comes first, and then the
/// table of the release (`verdict::resolve`). `results` holds the files of
/// `<[evals] dir>/results`. The diagnostic is on the line `line` of
/// `spec-dna.toml`. The severity is the default of the code. The caller
/// applies the rules.
pub fn skill_unproven(
    harness: &crate::harness::Harness,
    results: &[(String, Vec<u8>)],
    line: u32,
) -> Vec<Diag> {
    harness
        .skills
        .iter()
        .filter(|s| {
            let verdict = crate::verdict::resolve(s.key(), results).verdict;
            !crate::verdict::on_by_default(
                s.in_d2(),
                verdict,
                crate::verdict::DEFAULT_ON_NEEDS_VERDICT_ON,
            )
        })
        .map(|s| Diag {
            path: "spec-dna.toml".to_string(),
            line,
            code: Code::SkillUnproven,
            doc: "spec-dna.toml".to_string(),
            target: s.key().to_string(),
            message: format!(
                "the skill `{}` is in [harness] skills, and it is off by default (ADR 0023): neither a result file of the project nor the table of the release gives it the verdict `on`",
                s.key()
            ),
            severity: Code::SkillUnproven.severity(),
        })
        .collect()
}

/// The result of the gate against the base.
pub struct Outcome {
    pub head: Vec<Diag>,
    /// One flag for each index into `head`: true if the base does not have
    /// that diagnostic.
    pub new: Vec<bool>,
}

/// Gates the head tree against `base`. `lifecycle` is the lifecycle base tree,
/// if there is one. The checks of the delta that need a tree before the change
/// read it, and `base` is their fallback.
pub fn gate(
    config: &Config,
    base: Option<&Corpus>,
    lifecycle: Option<&Corpus>,
    head: &Corpus,
) -> Outcome {
    let mut head_diags = diagnose_against(config, head, lifecycle.or(base));
    let mut budget: Vec<(Diag, bool)> = hard_budget(config, base, head)
        .into_iter()
        .filter_map(|(mut d, n)| {
            d.severity = config.severity(d.code, Some(c_root(head, &d)))?;
            Some((d, n))
        })
        .collect();
    // The budget of an area has its severity already: its folder is no file.
    budget.extend(area_budget(config, base, head));
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
