//! `spec-dna check` applies the controls of ADR 0001 to one tree. It also
//! checks the transitions between the base tree and the head tree, and it does
//! base-relative gating.

use crate::config::Config;
use crate::diag::{Code, Diag};
use crate::evidence::{Evidence, needs_evidence};
use crate::model::{Corpus, Doc, DocType, LinkRes, Res, canonical};
use crate::parse::{FLAG_BCP14, FLAG_REQUIREMENT, FLAG_STATUS_WORD, RefKind};
use std::collections::{HashMap, HashSet};

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
    let per_doc = crate::repo::par_map(&c.docs, |d| doc_diags(config, c, d, &ev));
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
                        "`conventions.{ty}` points to `{p}`, which names no document or section"
                    ),
                    severity: Code::ConventionUnresolved.severity(),
                });
            }
        }
    }
    let mut out = apply_severity(config, out);
    out.sort();
    out
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

fn doc_diags(config: &Config, c: &Corpus, d: &Doc, ev: &Evidence) -> Vec<Diag> {
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
            RefKind::Wiki => id_ref(c, d, &r.target, r.line, &mut out),
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
                    out.push(diag(
                        d,
                        r.line,
                        Code::AnchorMissing,
                        r.target.clone(),
                        format!("`{path}` has no heading or requirement `{frag}`"),
                    ))
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
        id_ref(c, d, t, *line, &mut out);
    }
    for (t, line) in d.depends_on.iter() {
        id_ref(c, d, t, *line, &mut out);
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
            "the incident links no requirement, decision or runbook",
        ));
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

    // The profile and the rules of the root can turn the lint off.
    let lint_on = [
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
/// requirements), a decision or a runbook.
fn links_knowledge(c: &Corpus, d: &Doc) -> bool {
    let useful = |t: u32| {
        let ty = c.docs[t as usize].ty;
        ty.has_requirements() || matches!(ty, DocType::Decision | DocType::Runbook)
    };
    d.parsed.refs.iter().any(|r| match r.kind {
        RefKind::Wiki => matches!(c.resolve(&r.target), Res::Found(t, _, true) if useful(t)),
        RefKind::Link | RefKind::Image => {
            matches!(c.resolve_link(&d.path, &r.target), LinkRes::File(_, Some(t), _) if useful(t))
        }
    })
}

fn id_ref(c: &Corpus, d: &Doc, target: &str, line: u32, out: &mut Vec<Diag>) {
    match c.resolve(target) {
        Res::Found(_, _, true) => {}
        Res::Found(i, Some(frag), false) => out.push(diag(
            d,
            line,
            Code::UnresolvedReference,
            target,
            format!(
                "`{}` has no section or requirement `{frag}`",
                c.docs[i as usize].id
            ),
        )),
        Res::Found(..) => {}
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
                let deletable = matches!(
                    (b.ty, b.status_str()),
                    (DocType::Change, "done" | "rejected") | (DocType::Note, _)
                );
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
/// 2026-11-08 and 0.3.0). A document over the hard budget is an error (SD702)
/// if it is new, if it was under the budget in the base, or if it grew by more
/// than `[budgets] growth` characters. A document that was over the budget in
/// the base and grew by at most that allowance gives a warning (SD703): a
/// task can add an amendment or a registry row without a split. A document
/// that did not grow gives a warning (SD704). A root with
/// `enforce-budgets = false` gives SD704 in all cases. A stub and a generated
/// index have no budget. Returns each diagnostic with its "new" flag.
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
    // An invariant and a runbook have the states of a spec (ADR 0001,
    // amendment of 0.2.0).
    let spec_like = matches!(
        ty,
        DocType::Spec | DocType::Contract | DocType::Guide | DocType::Invariant | DocType::Runbook
    );
    let rank = |s: &str| -> Option<u8> {
        Some(match (ty, s) {
            (_, "draft") if spec_like => 0,
            (_, "active") if spec_like => 1,
            (_, "deprecated") if spec_like => 2,
            (_, "superseded" | "retired") if spec_like => 3,
            (DocType::Decision | DocType::Change, "proposed") => 0,
            (DocType::Decision | DocType::Change, "accepted") => 1,
            (DocType::Decision, "superseded" | "rejected") => 2,
            (DocType::Change, "done" | "rejected") => 2,
            _ => return None,
        })
    };
    let terminal = |s: &str| matches!(s, "superseded" | "retired" | "rejected" | "done");
    if ty == DocType::Decision && from == "accepted" && to == "rejected" {
        return false;
    }
    match (rank(from), rank(to)) {
        (Some(a), Some(b)) => !terminal(from) && b > a,
        _ => true, // another check reports an invalid status
    }
}

/// The result of base-relative gating.
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
    // id. A document that a split made has the section aliases of its source:
    // compare it under the id of the source, so that a diagnostic that moved
    // with the text is not new.
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
        assert!(transition_ok(DocType::Spec, "draft", "active"));
        assert!(transition_ok(DocType::Spec, "active", "retired"));
        assert!(!transition_ok(DocType::Spec, "active", "draft"));
        assert!(!transition_ok(DocType::Spec, "retired", "superseded"));
        assert!(transition_ok(DocType::Decision, "proposed", "accepted"));
        assert!(!transition_ok(DocType::Decision, "accepted", "rejected"));
        assert!(!transition_ok(DocType::Decision, "accepted", "proposed"));
        assert!(transition_ok(DocType::Change, "accepted", "done"));
        assert!(!transition_ok(DocType::Change, "done", "accepted"));
        assert!(!transition_ok(DocType::Invariant, "active", "draft"));
        assert!(!transition_ok(DocType::Runbook, "retired", "active"));
        assert!(transition_ok(DocType::Runbook, "draft", "active"));
    }
}
