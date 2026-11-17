//! The proof of `import` (design 0.7.0, D22; ADR 0014, "Proof"). It is a pure
//! function over a plan in memory: it reads the tree and writes nothing. The
//! 5 steps:
//!
//! 1. Each requirement of a source has one target: a requirement of the
//!    target document, or a delta line. The proof parses the planned text of
//!    the target and compares the normal form of the sentences. It never reads
//!    a sentence that the plan carries as the target. A marker
//!    `[NEEDS CLARIFICATION: text]` is no part of the normal form, and the
//!    proof checks that the open question holds its text.
//! 2. Each source file has exactly one state, and a kept or skipped file is
//!    not moved and not written.
//! 3. No id of the tree before the import stops to resolve to the same
//!    document.
//! 4. No error of the codes SD101 to SD106, SD201 to SD204, SD301 to SD306 and
//!    SD401 to SD405 is new in the tree after the import. The proof ignores the
//!    other codes (evidence, budgets, changes), and the report counts them.
//! 5. The plan on the planned tree is empty: a second run writes nothing.
//!
//! 6. No line of a source with a word of BCP 14 (and no other line, but the
//!    headings and the task lines that the reader rewrites) vanishes: each is
//!    in a target of its source. A line with a BCP 14 word that no target holds
//!    stops the write. Another line that no target holds is listed as
//!    `dropped` in the report.
//!
//! The hash of a requirement covers its sentence only, so the proof also
//! counts the scenarios of each target that has a count.
//!
//! The normal form removes the markers of the aliases, the link destinations
//! (the plan edits the relative links of a moved file, as `mv` does), the
//! clarification marker and the white space.

use crate::change::{self, Verb};
use crate::check;
use crate::config::Config;
use crate::diag::{Code, Severity};
use crate::hash::normalize;
use crate::import_docs::{
    PairKind, Plan, State, apply, corpus_of, count_scenarios, is_document, strip_clarification,
};
use crate::model::{Corpus, Doc, Res};
use crate::parse::{ItemSection, has_bcp14, parse, strip_link_destinations, strip_markers};
use crate::repo::Fatal;
use crate::upgrade_docs::Tree;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// The result of the proof. `failures` is empty if the proof passes.
#[derive(Debug, Default)]
pub struct Proof {
    /// The pairs that the proof compared.
    pub requirements: usize,
    /// The scenarios of the targets that have a count.
    pub scenarios: usize,
    /// The ids that the proof resolved in both trees.
    pub ids: usize,
    pub failures: Vec<String>,
    /// The lines of a source with no word of BCP 14 that no target holds, as
    /// `<path>:<line>: <text>`. The report lists them.
    pub dropped: Vec<String>,
    /// The anchors of headings of a moved document that its target has no
    /// more, because the heading became a list item or a title.
    pub removed_anchors: usize,
    /// The new diagnostics of the codes that the proof ignores, as (code,
    /// count). The report lists them.
    pub ignored: Vec<(String, usize)>,
}

impl Proof {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// The normal form of a sentence for the compare.
pub fn norm(sentence: &str) -> String {
    let (clean, _) = strip_clarification(sentence);
    let (clean, _, _) = strip_markers(&clean);
    normalize(&strip_link_destinations(&clean))
}

/// The codes that step 4 guards: references, ids, front matter and statuses.
const GUARDED: [&str; 21] = [
    "SD101", "SD102", "SD103", "SD104", "SD105", "SD106", "SD201", "SD202", "SD203", "SD204",
    "SD301", "SD302", "SD303", "SD304", "SD305", "SD306", "SD401", "SD402", "SD403", "SD404",
    "SD405",
];

fn guarded(code: Code) -> bool {
    GUARDED.contains(&code.code())
}

/// Proves a plan. `replan` runs the plan on a tree (step 5).
pub fn prove(
    tree: &Tree,
    plan: &Plan,
    fin: &Config,
    replan: &dyn Fn(&Tree) -> Result<Plan, Fatal>,
) -> Result<Proof, Fatal> {
    let mut proof = Proof::default();
    let planned = apply(tree, plan);
    let (before, _) = corpus_of(tree, fin)?;
    // The tree after the plan: each document of the roots, parsed again from
    // the planned text.
    let paths: Vec<String> = planned
        .files
        .iter()
        .filter(|f| is_document(fin, f))
        .cloned()
        .collect();
    let bytes = planned.read(tree, &paths)?;
    let mut texts: HashMap<String, String> = HashMap::new();
    let mut docs = Vec::new();
    for p in &paths {
        let t = String::from_utf8_lossy(bytes.get(p).map_or(&[][..], |b| b)).into_owned();
        docs.push((p.clone(), Arc::new(parse(&t))));
        texts.insert(p.clone(), t);
    }
    let after = Corpus::build(fin, planned.files.clone(), docs);

    pairs(&mut proof, plan, &after, &texts);
    scenarios(&mut proof, plan, &texts);
    states(&mut proof, tree, plan);
    text_loss(&mut proof, tree, plan, &texts)?;
    ids(&mut proof, plan, &before, &after);
    errors(&mut proof, fin, &before, &after, &texts);
    // Step 5: the plan on the planned tree is empty.
    let read = |want: &[String]| planned.read(tree, want);
    let tree2 = Tree {
        files: &planned.files,
        read: &read,
    };
    let again = match replan(&tree2) {
        Ok(p) => p,
        // A stop of the plan on the planned tree is a failure of step 5.
        Err(Fatal::Refused(m)) => {
            proof.failures.push(format!(
                "a second run would stop, so the plan is not stable: {m}"
            ));
            return Ok(proof);
        }
        // The plan moved the last file out of `.specify/` or `openspec/`: no
        // layout is left to read, so nothing is left to change.
        Err(Fatal::Config(_)) => return Ok(proof),
        Err(e) => return Err(e),
    };
    if !again.is_empty() {
        let mut paths = again.written_paths();
        if again.config.changed() {
            paths.insert(0, crate::import_docs::CONFIG_PATH.to_string());
        }
        proof.failures.push(format!(
            "a second run would still change {} file(s), the first is `{}`; the plan is not stable",
            paths.len(),
            paths.first().map_or("", String::as_str)
        ));
    }
    proof.failures.dedup();
    Ok(proof)
}

/// Step 1: the sentence of each pair is in the target.
fn pairs(proof: &mut Proof, plan: &Plan, after: &Corpus, texts: &HashMap<String, String>) {
    for pair in &plan.pairs {
        proof.requirements += 1;
        let fail = |why: String| format!("{} ({}): {why}", pair.from, pair.id);
        let Some(&di) = after.by_path.get(&pair.target) else {
            proof
                .failures
                .push(fail(format!("`{}` is not a document", pair.target)));
            continue;
        };
        let doc: &Doc = &after.docs[di as usize];
        let want = norm(&pair.sentence);
        let got = match pair.kind {
            PairKind::Requirement => match doc.req(&pair.key) {
                Some(i) => doc.parsed.reqs[i].text.clone(),
                None => {
                    proof.failures.push(fail(format!(
                        "the requirement `{}` is not in `{}`",
                        pair.key, pair.target
                    )));
                    continue;
                }
            },
            PairKind::Delta => {
                let target = format!("{}#{}", pair.id.split('#').next().unwrap_or(""), pair.key);
                let line = change::delta_lines(doc).into_iter().find_map(|l| {
                    l.parsed.ok().filter(|d| {
                        d.target() == target
                            && pair.verb.is_none_or(|v| {
                                matches!(
                                    (v, d.verb),
                                    ("ADD", Verb::Add)
                                        | ("CHANGE", Verb::Change)
                                        | ("REMOVE", Verb::Remove)
                                )
                            })
                    })
                });
                match line {
                    Some(d) => d.sentence.unwrap_or_default(),
                    None => {
                        proof.failures.push(fail(format!(
                            "no delta line for `{target}` in `{}`",
                            pair.target
                        )));
                        continue;
                    }
                }
            }
        };
        // A `REMOVE` line has no sentence: the compare is for the lines that
        // carry text.
        if pair.verb == Some("REMOVE") {
            continue;
        }
        if norm(&got) != want {
            proof.failures.push(fail(format!(
                "the sentence would change; the source has `{want}` and the target `{}` has `{}`",
                pair.target,
                norm(&got)
            )));
            continue;
        }
        if let Some(q) = &pair.question {
            let spec = texts.get(&pair.target).map(String::as_str).unwrap_or("");
            let doc = parse(spec);
            let held = doc.items.iter().any(|i| {
                i.section == ItemSection::OpenQuestions
                    && i.text.contains(&pair.key)
                    && norm(&i.text).contains(&norm(q))
            });
            if !held {
                proof.failures.push(fail(format!(
                    "the open question of `{}` must hold the text of the marker (`{q}`)",
                    pair.key
                )));
            }
        }
    }
}

/// The scenarios of each target that has a count (D22, last paragraph).
fn scenarios(proof: &mut Proof, plan: &Plan, texts: &HashMap<String, String>) {
    for t in &plan.targets {
        let Some(want) = t.scenarios else { continue };
        let got = texts.get(&t.path).map_or(0, |x| count_scenarios(x));
        proof.scenarios += got;
        if got != want {
            proof.failures.push(format!(
                "`{}`: the source has {want} scenarios, and the target has {got}",
                t.path
            ));
        }
    }
}

/// Step 2: one state for each source file.
fn states(proof: &mut Proof, tree: &Tree, plan: &Plan) {
    let in_tree: std::collections::HashSet<&str> = tree.files.iter().map(String::as_str).collect();
    let mut files: Vec<&String> = plan.universe.iter().collect();
    files.sort();
    files.dedup();
    for f in files {
        let n = plan.sources.iter().filter(|s| s.covers(f)).count();
        if n == 0 {
            proof
                .failures
                .push(format!("`{f}`: the source file has no state in the plan"));
        } else if n > 1 {
            proof.failures.push(format!(
                "`{f}`: the source file has more than one state in the plan"
            ));
        }
    }
    let moved: std::collections::HashSet<&str> =
        plan.moves.iter().map(|(f, _)| f.as_str()).collect();
    for s in &plan.sources {
        match s.state {
            State::Moved => {
                if !moved.contains(s.path.as_str()) {
                    proof.failures.push(format!(
                        "`{}`: the state is moved, and the plan has no move for it",
                        s.path
                    ));
                }
            }
            State::Kept | State::Skipped => {
                let touched = moved.iter().any(|m| s.covers(m))
                    || plan.writes.keys().any(|w| {
                        s.covers(w) && in_tree.contains(w.as_str()) && !plan.edited.contains(w)
                    });
                if touched {
                    proof.failures.push(format!(
                        "`{}`: the state is {}, and the plan moves or writes it",
                        s.path,
                        s.state.name()
                    ));
                }
            }
            State::Written => {}
        }
    }
    for (from, _) in &plan.moves {
        let named = plan
            .sources
            .iter()
            .any(|s| s.state == State::Moved && s.path == *from);
        let in_universe = plan.universe.iter().any(|u| u == from);
        if !named && in_universe {
            proof.failures.push(format!(
                "`{from}`: the plan moves it, and its state is not moved"
            ));
        }
    }
}

/// The line without its list marker, its task box and its bold key.
fn line_body(line: &str) -> &str {
    let mut t = line.trim();
    if let Some(r) = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))
    {
        t = r.trim_start();
    } else {
        let d = t.chars().take_while(char::is_ascii_digit).count();
        if d > 0 && (t[d..].starts_with(". ") || t[d..].starts_with(") ")) {
            t = t[d + 2..].trim_start();
        }
    }
    if let Some(r) = t
        .strip_prefix("[ ] ")
        .or_else(|| t.strip_prefix("[x] "))
        .or_else(|| t.strip_prefix("[X] "))
    {
        t = r.trim_start();
    }
    if let Some(r) = t.strip_prefix("**")
        && let Some(i) = r.find("**")
        && !r[..i].contains(char::is_whitespace)
    {
        t = r[i + 2..].trim_start_matches(':').trim_start();
    }
    t
}

/// A task line of a source: a box, then an id or a number.
fn is_task_line(line: &str) -> bool {
    let t = line.trim_start();
    (t.starts_with("- [ ] ") || t.starts_with("- [x] ") || t.starts_with("- [X] "))
        && t[6..]
            .chars()
            .next()
            .is_some_and(|c| c == 'T' || c.is_ascii_digit())
}

/// The normal form of a line for the compare of step 6: no link destination,
/// no bold marker, no backtick, and the white space joined. It is the form of
/// step 1 without the markers of aliases and the clarification.
fn line_form(s: &str) -> String {
    let t = strip_link_destinations(s)
        .replace("**", "")
        .replace('`', "");
    normalize(&t)
}

/// The units of a target: each line, and each paragraph (the lines up to a
/// blank line, joined with a space), in the line form. A source line must be
/// in one unit, so the words of 2 lines are no copy.
fn units(text: &str) -> String {
    let mut out = String::new();
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut String| {
        if para.len() > 1 {
            out.push_str(&para.join(" "));
            out.push('\u{1}');
        }
        para.clear();
    };
    let heads = crate::import_docs::headings_by_line(text);
    for (n, l) in text.lines().enumerate() {
        if l.trim().is_empty() {
            flush(&mut para, &mut out);
            continue;
        }
        // A heading is compared on its text, with no `#`.
        let f = match &heads[n] {
            Some((_, t)) => line_form(t),
            None => line_form(line_body(l)),
        };
        out.push_str(&f);
        out.push('\u{1}');
        para.push(f);
    }
    flush(&mut para, &mut out);
    out
}

/// The part of a task line that must stay: its text, with no box, no id, no
/// mark and no path (the reader moves them).
fn task_text(line: &str) -> String {
    let t = line.trim_start();
    let t = t
        .strip_prefix("- [ ] ")
        .or_else(|| t.strip_prefix("- [x] "))
        .or_else(|| t.strip_prefix("- [X] "))
        .unwrap_or(t)
        .trim();
    let word = t.split_whitespace().next().unwrap_or("");
    let rest = t[word.len()..].trim();
    crate::import_speckit::plan_task(rest).0
}

/// Step 6: no line of a source vanishes. A heading is rewritten by the
/// readers, so the proof needs no copy of it (a heading with a BCP 14 word
/// still needs one). A line of a source must be in one line or one paragraph
/// of a target, in the line form, or in the sentence of a pair. The task
/// lines are compared on their text.
fn text_loss(
    proof: &mut Proof,
    tree: &Tree,
    plan: &Plan,
    texts: &HashMap<String, String>,
) -> Result<(), Fatal> {
    let sources: Vec<&crate::import_docs::SourceFile> = plan
        .sources
        .iter()
        .filter(|s| {
            matches!(s.state, State::Moved | State::Written)
                && !s.path.ends_with('/')
                && s.to != [s.path.clone()]
        })
        .collect();
    let paths: Vec<String> = sources.iter().map(|s| s.path.clone()).collect();
    let bytes = (tree.read)(&paths)?;
    for s in sources {
        let Some(b) = bytes.get(&s.path) else {
            continue;
        };
        let source = String::from_utf8_lossy(b);
        let mut hay = String::new();
        for t in plan.targets.iter().filter(|t| t.sources.contains(&s.path)) {
            hay.push_str(&units(
                texts.get(&t.path).map_or(t.text.as_str(), String::as_str),
            ));
        }
        let prefix = format!("{}:", s.path);
        for p in plan.pairs.iter().filter(|p| p.from.starts_with(&prefix)) {
            hay.push_str(&line_form(&p.sentence));
            hay.push('\u{1}');
        }
        let keys: Vec<String> = plan
            .pairs
            .iter()
            .filter(|p| p.from.starts_with(&prefix))
            .map(|p| p.key.to_lowercase())
            .filter(|k| k.len() >= 3)
            .collect();
        let heads = crate::import_docs::headings_by_line(&source);
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let bcp = has_bcp14(trimmed);
            let heading = heads[i].as_ref().map(|(_, t)| t.as_str());
            if heading.is_some() && !bcp {
                continue;
            }
            let form = if let Some(t) = heading {
                // A heading of a section that the reader keeps as text is held
                // as a title, a bold line or a list item. A heading of a
                // requirement or a principle is the key of its pair.
                let form = line_form(t);
                let slug = crate::import_docs::slug(t);
                if keys.iter().any(|k| slug.contains(k.as_str())) {
                    continue;
                }
                form
            } else if is_task_line(line) {
                line_form(task_text(line).trim_end_matches('.'))
            } else {
                line_form(line_body(line))
            };
            if form.is_empty() || hay.contains(&form) {
                continue;
            }
            if bcp {
                proof.failures.push(format!(
                    "{}:{}: the line `{trimmed}` has a word of BCP 14, and no line or paragraph of a target holds it, so the write would lose a requirement",
                    s.path,
                    i + 1
                ));
            } else {
                proof
                    .dropped
                    .push(format!("{}:{}: {trimmed}", s.path, i + 1));
            }
        }
    }
    Ok(())
}

/// The ids of a document of the tree before: its id, its aliases, the id of
/// each requirement and each section.
fn ids_of(d: &Doc) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if d.moved_to.is_some() {
        return ids;
    }
    ids.insert(d.id.clone());
    ids.extend(d.aliases.iter().cloned());
    ids.extend(d.legacy_aliases.iter().cloned());
    for &r in &d.reqs {
        let req = &d.parsed.reqs[r];
        ids.insert(format!("{}#{}", d.id, req.key));
        ids.extend(req.aliases.iter().cloned());
    }
    for c in &d.parsed.chunks {
        if c.level > 0 && !c.anchor.is_empty() {
            ids.insert(format!("{}#{}", d.id, c.anchor));
        }
    }
    ids
}

/// Step 3: each id of the tree before resolves to the same document.
fn ids(proof: &mut Proof, plan: &Plan, before: &Corpus, after: &Corpus) {
    let moved: HashMap<&str, &str> = plan
        .moves
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();
    let mut seen = BTreeSet::new();
    for d in &before.docs {
        for id in ids_of(d) {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Res::Found(bi, _, true) = before.resolve(&id) else {
                continue;
            };
            let was = &before.docs[bi as usize].path;
            // A source that became a stub: its id is an alias of the target. The
            // ids of its requirements go: the delta of the target adds them.
            let stub = plan.stubs.get(was.as_str());
            if stub.is_some() && id.contains('#') {
                continue;
            }
            let want = moved
                .get(was.as_str())
                .copied()
                .or(stub.map(String::as_str))
                .unwrap_or(was.as_str());
            proof.ids += 1;
            // A heading of a moved document can become a list item or a
            // title: its anchor is gone on purpose. A requirement key stays.
            let heading_only = matches!(
                &before.resolve(&id),
                Res::Found(bi, Some(f), true) if before.docs[*bi as usize].req(f).is_none()
            );
            match after.resolve(&id) {
                Res::Found(ai, Some(_), false)
                    if heading_only
                        && moved.contains_key(was.as_str())
                        && after.docs[ai as usize].path == want =>
                {
                    proof.removed_anchors += 1;
                }
                Res::Found(ai, _, true) if after.docs[ai as usize].path == want => {}
                Res::Found(ai, _, true) => proof.failures.push(format!(
                    "`{id}` resolves to `{was}` before the plan, and to `{}` after it; `{want}` must keep it",
                    after.docs[ai as usize].path
                )),
                _ => proof.failures.push(format!(
                    "`{id}` resolves to `{was}` before the plan, and it does not resolve after it; add the old id to the `aliases` of `{want}`"
                )),
            }
        }
    }
}

/// Step 4: no new error of a guarded code. The other new errors and warnings
/// are for the report.
fn errors(
    proof: &mut Proof,
    fin: &Config,
    before: &Corpus,
    after: &Corpus,
    texts: &HashMap<String, String>,
) {
    let mut seen: HashMap<(Code, String), usize> = HashMap::new();
    for d in check::diagnose(fin, before) {
        *seen.entry((d.code, d.target)).or_default() += 1;
    }
    let mut ignored: BTreeMap<String, usize> = BTreeMap::new();
    for d in check::diagnose(fin, after) {
        match seen.get_mut(&(d.code, d.target.clone())) {
            Some(n) if *n > 0 => {
                *n -= 1;
                continue;
            }
            _ => {}
        }
        if guarded(d.code) {
            if d.severity != Severity::Error {
                continue;
            }
            let mut msg = format!("a new error: {}", d.text());
            if let Some(line) = texts
                .get(&d.path)
                .and_then(|t| t.lines().nth((d.line as usize).saturating_sub(1)))
            {
                msg.push_str(&format!(" (the line is `{}`)", line.trim()));
            }
            if d.code == Code::KeyFormat {
                msg.push_str(
                    "; import changes no sentence, so either add the pattern to `legacy-keys` in spec-dna.toml, or remove the bold key in the source and commit, then run import again",
                );
            }
            proof.failures.push(msg);
        } else {
            *ignored.entry(d.code.code().to_string()).or_default() += 1;
        }
    }
    proof.ignored = ignored.into_iter().collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_docs::{Pair, SourceFile, Target};
    use crate::upgrade_docs::TreeData;

    const CFG: &str = "spec-dna = \"0.6.1\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

    const SOURCE: &str = "# Feature\n\n- **door-opens**: The door MUST open on the first push.\n  - Scenario: Push\n";

    fn fin() -> Config {
        Config::parse(CFG).unwrap()
    }

    fn tree(files: &[(&str, &str)]) -> TreeData {
        let mut all: Vec<(&str, &str)> = vec![("spec-dna.toml", CFG)];
        all.extend_from_slice(files);
        TreeData::from_map(
            all.iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        )
    }

    /// A plan that moves `src/feature.md` to a requirements document.
    fn moved_plan(text: &str) -> Plan {
        let to = "docs/requirements/feature.md";
        let mut plan = Plan::default();
        plan.config.old = CFG.to_string();
        plan.config.new = CFG.to_string();
        plan.moves = vec![("src/feature.md".into(), to.into())];
        plan.writes.insert(to.into(), text.into());
        plan.targets = vec![Target {
            path: to.into(),
            text: text.into(),
            moved_from: Some("src/feature.md".into()),
            sources: vec!["src/feature.md".into()],
            link_of: None,
            scenarios: None,
        }];
        plan.sources = vec![SourceFile {
            path: "src/feature.md".into(),
            state: State::Moved,
            to: vec![to.into()],
            reason: String::new(),
        }];
        plan.pairs = vec![Pair {
            from: "src/feature.md:3".into(),
            id: "requirements/feature#door-opens".into(),
            delta: None,
            target: to.into(),
            key: "door-opens".into(),
            kind: PairKind::Requirement,
            sentence: "The door MUST open on the first push.".into(),
            question: None,
            verb: None,
        }];
        plan.universe = vec!["src/feature.md".into()];
        plan
    }

    fn planned_text() -> String {
        format!("---\nstatus: active\n---\n{SOURCE}")
    }

    fn run(plan: &Plan, files: &[(&str, &str)]) -> Proof {
        let t = tree(files);
        t.with(|t| prove(t, plan, &fin(), &|_| Ok(Plan::default())))
            .unwrap()
    }

    #[test]
    fn a_good_plan_passes_and_counts_the_requirement() {
        let plan = moved_plan(&planned_text());
        let p = run(&plan, &[("src/feature.md", SOURCE)]);
        assert!(p.ok(), "{:?}", p.failures);
        assert_eq!(p.requirements, 1);
    }

    #[test]
    fn a_changed_sentence_stops_the_write_and_names_the_pair() {
        // The mutation: one word of the planned text changes.
        let text = planned_text().replace("first push", "second push");
        let p = run(&moved_plan(&text), &[("src/feature.md", SOURCE)]);
        assert!(!p.ok());
        let f = p.failures.join("\n");
        assert!(f.contains("src/feature.md:3"), "{f}");
        assert!(f.contains("requirements/feature#door-opens"), "{f}");
        assert!(f.contains("first push") && f.contains("second push"), "{f}");
    }

    #[test]
    fn a_sentence_that_the_target_lost_stops_the_write() {
        let text = "---\nstatus: active\n---\n# Feature\n\nNothing.\n";
        let p = run(&moved_plan(text), &[("src/feature.md", SOURCE)]);
        assert!(!p.ok());
        assert!(
            p.failures.join("\n").contains("door-opens"),
            "{:?}",
            p.failures
        );
    }

    #[test]
    fn the_normal_form_ignores_white_space_link_destinations_and_a_marker() {
        let a = "The door [opens](./a.md)  on\nthe push [NEEDS CLARIFICATION: how fast?].";
        let b = "The door [opens](../b/a.md) on the push.";
        assert_eq!(norm(a), norm(b));
        assert_ne!(norm(a), norm("The door opens on the pull."));
    }

    #[test]
    fn a_changed_scenario_count_stops_the_write() {
        let mut plan = moved_plan(&planned_text());
        plan.targets[0].scenarios = Some(2);
        let p = run(&plan, &[("src/feature.md", SOURCE)]);
        assert!(!p.ok());
        let f = p.failures.join("\n");
        assert!(f.contains("docs/requirements/feature.md"), "{f}");
        assert!(f.contains("2 scenarios") && f.contains("1"), "{f}");
        plan.targets[0].scenarios = Some(1);
        let p = run(&plan, &[("src/feature.md", SOURCE)]);
        assert!(p.ok(), "{:?}", p.failures);
        assert_eq!(p.scenarios, 1);
    }

    #[test]
    fn a_source_file_has_exactly_one_state() {
        let mut plan = moved_plan(&planned_text());
        plan.universe.push("src/other.md".into());
        let p = run(
            &plan,
            &[("src/feature.md", SOURCE), ("src/other.md", "x\n")],
        );
        let f = p.failures.join("\n");
        assert!(f.contains("src/other.md") && f.contains("no state"), "{f}");
        plan.sources
            .push(SourceFile::kept("src/other.md", "a test"));
        plan.sources.push(SourceFile::skipped("src/", "a test"));
        let p = run(
            &plan,
            &[("src/feature.md", SOURCE), ("src/other.md", "x\n")],
        );
        let f = p.failures.join("\n");
        assert!(
            f.contains("src/other.md") && f.contains("more than one state"),
            "{f}"
        );
    }

    #[test]
    fn a_kept_file_that_the_plan_moves_stops_the_write() {
        let mut plan = moved_plan(&planned_text());
        plan.sources[0].state = State::Kept;
        let p = run(&plan, &[("src/feature.md", SOURCE)]);
        assert!(
            p.failures.join("\n").contains("src/feature.md"),
            "{:?}",
            p.failures
        );
    }

    #[test]
    fn a_new_error_of_a_guarded_code_stops_the_write_with_the_fixes() {
        let src = "# Feature\n\n- **SC-001**: The door MUST open.\n";
        let text = format!("---\nstatus: active\n---\n{src}");
        let mut plan = moved_plan(&text);
        plan.pairs.clear();
        let p = run(&plan, &[("src/feature.md", src)]);
        let f = p.failures.join("\n");
        assert!(f.contains("SD203"), "{f}");
        assert!(f.contains("SC-001"), "{f}");
        assert!(f.contains("legacy-keys"), "{f}");
        assert!(f.contains("remove the bold key"), "{f}");
        assert!(f.contains("docs/requirements/feature.md:6"), "{f}");
    }

    #[test]
    fn an_id_that_would_not_resolve_any_more_stops_the_write() {
        let old = "---\nstatus: active\n---\n# Old\n\n- **a-key**: The door MUST open.\n";
        let mut plan = moved_plan(&planned_text());
        plan.moves.push((
            "docs/requirements/old.md".into(),
            "docs/requirements/new-name.md".into(),
        ));
        plan.writes
            .insert("docs/requirements/new-name.md".into(), old.into());
        let files = [
            ("src/feature.md", SOURCE),
            ("docs/requirements/old.md", old),
        ];
        let p = run(&plan, &files);
        let f = p.failures.join("\n");
        assert!(f.contains("requirements/old"), "{f}");
        // The same move with the old id as an alias passes.
        let aliased = old.replace(
            "status: active\n",
            "status: active\naliases:\n  - requirements/old\n",
        );
        plan.writes
            .insert("docs/requirements/new-name.md".into(), aliased);
        let p = run(&plan, &files);
        assert!(p.ok(), "{:?}", p.failures);
        assert!(p.ids >= 2, "{}", p.ids);
    }

    #[test]
    fn a_plan_that_a_second_run_would_change_again_stops_the_write() {
        let plan = moved_plan(&planned_text());
        let t = tree(&[("src/feature.md", SOURCE)]);
        let again = moved_plan(&planned_text());
        let p = t
            .with(|t| prove(t, &plan, &fin(), &|_| Ok(again.clone())))
            .unwrap();
        let f = p.failures.join("\n");
        assert!(f.contains("second run"), "{f}");
        assert!(f.contains("docs/requirements/feature.md"), "{f}");
    }

    #[test]
    fn the_open_question_holds_the_text_of_the_marker() {
        let spec = |q: &str| {
            format!(
                "---\nstatus: draft\n---\n# Feature\n\n## Delta\n\n- ADD requirements/feature#fr-1: The door MUST offer formats.\n\n## Open questions\n\n{q}\n"
            )
        };
        let to = "docs/intent/feature/spec.md";
        let mk = |q: &str| {
            let mut plan = moved_plan("");
            plan.moves.clear();
            plan.writes.clear();
            plan.writes.insert(to.into(), spec(q));
            plan.targets[0] = Target {
                path: to.into(),
                text: spec(q),
                moved_from: None,
                sources: vec!["src/feature.md".into()],
                link_of: None,
                scenarios: None,
            };
            plan.sources[0].state = State::Written;
            plan.pairs = vec![Pair {
                from: "src/feature.md:3".into(),
                id: "requirements/feature#fr-1".into(),
                delta: Some("feature".into()),
                target: to.into(),
                key: "fr-1".into(),
                kind: PairKind::Delta,
                sentence: "The door MUST offer [NEEDS CLARIFICATION: which formats?] formats."
                    .into(),
                question: Some("which formats?".into()),
                verb: Some("ADD"),
            }];
            plan
        };
        let files = [(
            "src/feature.md",
            "# Feature\n\nThe door MUST offer [NEEDS CLARIFICATION: which formats?] formats.\n",
        )];
        let good = run(&mk("- [ ] fr-1: which formats?"), &files);
        assert!(good.ok(), "{:?}", good.failures);
        let bad = run(&mk("- none."), &files);
        let f = bad.failures.join("\n");
        assert!(f.contains("which formats?"), "{f}");
    }

    #[test]
    fn a_stop_of_the_plan_on_the_planned_tree_is_a_failure_of_step_5() {
        let plan = moved_plan(&planned_text());
        let t = tree(&[("src/feature.md", SOURCE)]);
        let p = t
            .with(|t| {
                prove(t, &plan, &fin(), &|_| {
                    Err(Fatal::Refused("a target exists with other text".into()))
                })
            })
            .unwrap();
        let f = p.failures.join("\n");
        assert!(
            f.contains("second run would stop") && f.contains("other text"),
            "{f}"
        );
    }

    fn with_source(text: &str, source: &str) -> Proof {
        run(&moved_plan(text), &[("src/feature.md", source)])
    }

    #[test]
    fn a_dropped_line_with_a_bcp_14_word_stops_the_write_and_names_the_line() {
        let source = format!("{SOURCE}\nThe lock MUST not vanish.\n");
        let p = with_source(&planned_text(), &source);
        let f = p.failures.join("\n");
        assert!(f.contains("src/feature.md:6"), "{f}");
        assert!(f.contains("The lock MUST not vanish."), "{f}");
        // The same line in the target is no loss, also in another layout.
        let kept = format!("{}\n  The lock MUST not vanish.\n", planned_text());
        let p = with_source(&kept, &source);
        assert!(p.ok(), "{:?}", p.failures);
        assert!(p.dropped.is_empty(), "{:?}", p.dropped);
    }

    #[test]
    fn a_dropped_line_with_no_bcp_14_word_is_listed_and_no_failure() {
        let source = format!("{SOURCE}\nA note for the reader.\n");
        let p = with_source(&planned_text(), &source);
        assert!(p.ok(), "{:?}", p.failures);
        assert_eq!(p.dropped, ["src/feature.md:6: A note for the reader."]);
    }

    #[test]
    fn a_heading_and_a_line_of_a_pair_are_no_dropped_lines() {
        // The heading `# Feature` is in the target; a heading that the reader
        // rewrote and the line of a requirement need no copy.
        let source = format!("{SOURCE}\n## A heading that goes\n");
        let p = with_source(&planned_text(), &source);
        assert!(p.ok(), "{:?}", p.failures);
        assert!(p.dropped.is_empty(), "{:?}", p.dropped);
    }

    #[test]
    fn a_heading_that_became_a_list_item_is_no_lost_id() {
        let old = "---\nstatus: active\n---\n# Old\n\n## Old heading\n\n- **a-key**: The door MUST open.\n";
        let new = "---\nstatus: active\naliases:\n  - requirements/old\n---\n# Old\n\n- **a-key**: The door MUST open.\n";
        let mut plan = moved_plan(&planned_text());
        plan.moves.push((
            "docs/requirements/old.md".into(),
            "docs/requirements/new-name.md".into(),
        ));
        plan.writes
            .insert("docs/requirements/new-name.md".into(), new.into());
        let files = [
            ("src/feature.md", SOURCE),
            ("docs/requirements/old.md", old),
        ];
        let p = run(&plan, &files);
        assert!(p.ok(), "{:?}", p.failures);
        // The key of the requirement must stay.
        let lost = new.replace("a-key", "b-key");
        plan.writes
            .insert("docs/requirements/new-name.md".into(), lost);
        let p = run(&plan, &files);
        assert!(
            p.failures.join("\n").contains("requirements/old#a-key"),
            "{:?}",
            p.failures
        );
    }

    #[test]
    fn a_kept_document_with_an_edited_link_is_no_failure_of_step_2() {
        let doc = "---\nstatus: active\n---\n# Plan\n\nSee [it](../../src/feature.md).\n";
        let mut plan = moved_plan(&planned_text());
        plan.universe.push("docs/requirements/plan.md".into());
        plan.sources
            .push(SourceFile::kept("docs/requirements/plan.md", "a file"));
        plan.writes.insert(
            "docs/requirements/plan.md".into(),
            doc.replace("../../src/feature.md", "feature.md"),
        );
        let files = [
            ("src/feature.md", SOURCE),
            ("docs/requirements/plan.md", doc),
        ];
        let p = run(&plan, &files);
        assert!(!p.ok());
        plan.edited.push("docs/requirements/plan.md".into());
        let p = run(&plan, &files);
        assert!(p.ok(), "{:?}", p.failures);
    }

    #[test]
    fn words_in_order_over_2_lines_of_a_target_are_no_copy_of_a_line() {
        // `alpha` ends a paragraph and `beta MUST gamma` starts the next one.
        let source = format!("{SOURCE}\nalpha beta MUST gamma\n");
        let text = format!("{}\nalpha\n\nbeta MUST gamma\n", planned_text());
        let p = with_source(&text, &source);
        assert!(
            p.failures.join("\n").contains("src/feature.md:6"),
            "{:?}",
            p.failures
        );
        // The same words in 1 line are a copy.
        let text = format!("{}\nalpha beta MUST gamma\n", planned_text());
        assert!(with_source(&text, &source).ok());
    }

    #[test]
    fn the_compare_of_a_line_ignores_link_destinations_and_markup() {
        let source = format!("{SOURCE}\nThe page MUST follow [the **plan**](./plan.md).\n");
        let text = format!(
            "{}\nThe page MUST follow [the plan](../../spec-dna.toml).\n",
            planned_text()
        );
        let p = with_source(&text, &source);
        assert!(p.ok(), "{:?}", p.failures);
        assert!(p.dropped.is_empty(), "{:?}", p.dropped);
    }

    #[test]
    fn a_task_line_is_compared_on_its_text() {
        let source = "# T\n\n- [ ] T002 [P] [US1] Write the builder in src/zip.py (each archive MUST be signed)\n- [ ] T004 Document why each export MUST be logged\n";
        let target = "---\nstatus: draft\n---\n# T\n\n- [ ] t002: Write the builder (each archive MUST be signed). Files: `src/zip.py`.\n- [ ] t004: Document why each export MUST be logged.\n";
        let p = run(&moved_plan_of(target), &[("src/feature.md", source)]);
        assert!(p.ok(), "{:?}", p.failures);
        assert!(p.dropped.is_empty(), "{:?}", p.dropped);
        // A task whose text lost the part in parentheses stops the write.
        let lost = target.replace(" (each archive MUST be signed)", "");
        let p = run(&moved_plan_of(&lost), &[("src/feature.md", source)]);
        assert!(
            p.failures.join("\n").contains("src/feature.md:3"),
            "{:?}",
            p.failures
        );
    }

    /// A plan with no pair, for the text of a source.
    fn moved_plan_of(text: &str) -> Plan {
        let mut plan = moved_plan(text);
        plan.pairs.clear();
        plan
    }

    #[test]
    fn a_heading_with_a_bcp_14_word_is_compared_on_its_text() {
        let heading = "### Tests for User Story 1 (OPTIONAL - only if tests requested)";
        let source = format!("{SOURCE}\n{heading}\n");
        // A bold line, a list item and a plain line hold the text.
        for held in [
            "**Tests for User Story 1 (OPTIONAL - only if tests requested)**",
            "- Tests for User Story 1 (OPTIONAL - only if tests requested)",
            "Tests for User Story 1 (OPTIONAL - only if tests requested)",
            "## Tests for User Story 1 (OPTIONAL - only if tests requested)",
        ] {
            let text = format!("{}\n{held}\n", planned_text());
            let p = with_source(&text, &source);
            assert!(p.ok(), "{held}: {:?}", p.failures);
        }
        // No target holds it.
        let p = with_source(&planned_text(), &source);
        assert!(
            p.failures.join("\n").contains("src/feature.md:6"),
            "{:?}",
            p.failures
        );
        // A heading with other words is no copy.
        let text = format!(
            "{}\n## Tests for User Story 2 (OPTIONAL - only if tests requested)\n",
            planned_text()
        );
        assert!(!with_source(&text, &source).ok());
    }

    #[test]
    fn a_scenario_heading_becomes_a_list_item_and_a_key_heading_becomes_a_key() {
        let source = format!(
            "{SOURCE}\n#### Scenario: Admin MAY end a session\n\n### Requirement: door-opens MAY be tested\n"
        );
        let text = format!("{}\n- Scenario: Admin MAY end a session\n", planned_text());
        let plan = moved_plan(&text);
        let p = run(&plan, &[("src/feature.md", &source)]);
        assert!(
            !p.failures.join("\n").contains("src/feature.md:7"),
            "{:?}",
            p.failures
        );
        assert!(p.ok(), "{:?}", p.failures);
    }

    #[test]
    fn a_line_in_a_code_block_is_no_heading() {
        // `# comment MUST x` is a line of code. It must be in a target.
        let source = format!("{SOURCE}\n```bash\n# comment MUST x\n```\n");
        let p = with_source(&planned_text(), &source);
        assert!(
            p.failures.join("\n").contains("src/feature.md:7"),
            "{:?}",
            p.failures
        );
        let text = format!("{}\n```bash\n# comment MUST x\n```\n", planned_text());
        assert!(with_source(&text, &source).ok());
    }

    #[test]
    fn a_layout_that_is_gone_on_the_planned_tree_is_no_failure_of_step_5() {
        let plan = moved_plan(&planned_text());
        let t = tree(&[("src/feature.md", SOURCE)]);
        let p = t
            .with(|t| {
                prove(t, &plan, &fin(), &|_| {
                    Err(Fatal::Config("`.specify/` is not in the tree".into()))
                })
            })
            .unwrap();
        assert!(p.ok(), "{:?}", p.failures);
    }
}
