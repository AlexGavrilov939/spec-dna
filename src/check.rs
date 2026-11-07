//! `spec-dna check` applies the controls of ADR 0001 to one tree. It also
//! checks the transitions between the base tree and the head tree, and it does
//! base-relative gating.

use crate::config::Config;
use crate::diag::{Code, Diag};
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
    }
}

/// Returns every diagnostic of one tree.
pub fn diagnose(config: &Config, c: &Corpus) -> Vec<Diag> {
    let per_doc = crate::repo::par_map(&c.docs, |d| doc_diags(config, c, d));
    let mut out: Vec<Diag> = per_doc.into_iter().flatten().collect();

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

fn doc_diags(config: &Config, c: &Corpus, d: &Doc) -> Vec<Diag> {
    let mut out: Vec<Diag> = d
        .issues
        .iter()
        .map(|i| diag(d, i.line, i.code, i.target.clone(), i.message.clone()))
        .collect();
    let p = &d.parsed;

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
    for (path, line) in &d.verified_by {
        let p = path.trim_matches('/');
        if !c.files.contains(p) && !c.dirs.contains(p) {
            out.push(diag(
                d,
                *line,
                Code::EvidenceMissing,
                path.clone(),
                format!("`verified-by` path `{path}` is not in the git tree"),
            ));
        }
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
    if d.ty == DocType::Decision && d.outcome.is_empty() {
        let names: Vec<String> = config.roots[d.root]
            .decision_sections
            .iter()
            .map(|s| format!("`{s}`"))
            .collect();
        out.push(diag(
            d,
            1,
            Code::SectionMissing,
            "decision-outcome",
            format!("a decision needs a section headed {}", names.join(" or ")),
        ));
    }

    let (soft, hard) = config.budgets[d.ty as usize];
    if hard > 0 && p.body_chars > hard {
        out.push(diag(
            d,
            1,
            Code::BudgetHard,
            "",
            format!(
                "{} characters, over the hard budget of {hard}",
                p.body_chars
            ),
        ));
    } else if soft > 0 && p.body_chars > soft {
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
    let _ = config;
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
            && h.parsed.frozen_hash != b.parsed.frozen_hash
        {
            out.push(diag(
                h,
                1,
                Code::DecisionFrozen,
                "",
                "the text above `## Amendments` changed after the decision was accepted",
            ));
        }
        let (soft, _) = config.budgets[h.ty as usize];
        if soft > 0 && h.parsed.body_chars > soft && h.parsed.body_chars > b.parsed.body_chars {
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
            Some(b) => amendment_chunks(b)
                .map(|i| b.parsed.chunks[i].hash)
                .collect(),
            None => HashSet::new(),
        };
        for i in amendment_chunks(h) {
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
    out.sort();
    out
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

/// Returns the chunk indices of the `## Amendments` section.
fn amendment_chunks(d: &Doc) -> impl Iterator<Item = usize> + '_ {
    let start = d.parsed.amendments.map_or(usize::MAX, |a| a as usize);
    let chunks = &d.parsed.chunks;
    (start..chunks.len()).take_while(move |&i| i == start || chunks[i].level > 2)
}

fn transition_ok(ty: DocType, from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    let rank = |s: &str| -> Option<u8> {
        Some(match (ty, s) {
            (DocType::Spec | DocType::Contract | DocType::Guide, "draft") => 0,
            (DocType::Spec | DocType::Contract | DocType::Guide, "active") => 1,
            (DocType::Spec | DocType::Contract | DocType::Guide, "deprecated") => 2,
            (DocType::Spec | DocType::Contract | DocType::Guide, "superseded" | "retired") => 3,
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
    let Some(base) = base else {
        let n = head_diags.len();
        return Outcome {
            head: head_diags,
            new: vec![true; n],
        };
    };
    let base_diags = diagnose(config, base);
    let mut base_fp: HashMap<(Code, &str, &str), usize> = HashMap::new();
    for d in &base_diags {
        *base_fp.entry(d.fingerprint()).or_default() += 1;
    }
    // A moved document keeps its old id as an alias. Compare it under the old id.
    let renamed: HashMap<&str, &str> = head
        .docs
        .iter()
        .filter(|h| !base.lookup.contains_key(&canonical(&h.id)))
        .filter_map(|h| head_to_base(base, h).map(|b| (h.id.as_str(), b.id.as_str())))
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
    }
}
