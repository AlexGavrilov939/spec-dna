//! The change model (design 0.6.0, "Change lifecycle"). A change is one folder
//! `<intent layer dir>/<slug>/` with up to 3 artifacts: `intent.md`, `spec.md`
//! and `plan.md`. This module reads the artifacts of a tree: their statuses,
//! the `class`, the typed lines of the delta, and the `Files` scope. It runs no
//! check and reads no configuration key. The checks and the commands of the
//! lifecycle use it.
//!
//! The parsed data comes from `ParsedDoc::items` and `Req::sentence`. A line
//! that is not in a valid form is data here, not an error: this module keeps it
//! with its reason, and a check reports it later (SD914).

use crate::glob;
use crate::hash::{Hash, hash_normalized, hex};
use crate::model::{Class, Corpus, Doc, DocType, Res, is_slug};
use crate::parse::{ItemSection, ListItem, Req, strip_link_destinations, strip_markers};
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

/// One artifact of a change: the document in the corpus and its status.
#[derive(Clone, Debug, PartialEq)]
pub struct Artifact {
    /// The index in `Corpus::docs`.
    pub doc: u32,
    /// The status, or an empty string if the document has none.
    pub status: String,
}

/// One change: the folder of its artifacts.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// The name of the folder, which is the name of the change.
    pub slug: String,
    /// The path of the folder from the repository root, with no final `/`.
    pub dir: String,
    pub intent: Option<Artifact>,
    pub spec: Option<Artifact>,
    pub plan: Option<Artifact>,
    /// The `class` of the intent. A change with no intent, and an intent with
    /// no `class` or with an invalid `class`, is `routine`.
    pub class: Class,
    /// The `areas` of the intent.
    pub areas: Vec<String>,
    /// The `from` of the intent: the id of an incident.
    pub from: Option<String>,
}

/// Collects the changes of a tree, in the order of their folders. A document
/// is an artifact if its type is `intent`, `spec` or `plan` and it is at the
/// place of that type. A folder with one such document is a change, also
/// without an intent.
pub fn collect(corpus: &Corpus) -> Vec<Change> {
    let mut by_dir: BTreeMap<&str, Change> = BTreeMap::new();
    for (i, d) in corpus.docs.iter().enumerate() {
        if !d.ty.is_intent_type() || d.type_failed || d.moved_to.is_some() || d.generated.is_some()
        {
            continue;
        }
        let Some((dir, _)) = d.path.rsplit_once('/') else {
            continue;
        };
        let change = by_dir.entry(dir).or_insert_with(|| Change {
            slug: dir.rsplit('/').next().unwrap_or(dir).to_string(),
            dir: dir.to_string(),
            intent: None,
            spec: None,
            plan: None,
            class: Class::Routine,
            areas: Vec::new(),
            from: None,
        });
        let artifact = Some(Artifact {
            doc: i as u32,
            status: d.status_str().to_string(),
        });
        match d.ty {
            DocType::Intent => {
                change.intent = artifact;
                change.class = d.class;
                change.areas = d.areas.clone();
                change.from = d.from.as_ref().map(|(f, _)| f.clone());
            }
            DocType::Spec => change.spec = artifact,
            _ => change.plan = artifact,
        }
    }
    by_dir.into_values().collect()
}

impl Change {
    /// The status of the intent, or an empty string if the change has no
    /// intent.
    #[allow(dead_code)] // reader: S7
    pub fn intent_status(&self) -> &str {
        self.intent.as_ref().map_or("", |a| &a.status)
    }

    /// The typed lines of the delta of the spec, with the line errors. A
    /// change with no spec has no delta line.
    pub fn delta(&self, corpus: &Corpus) -> Vec<DeltaLine> {
        match &self.spec {
            Some(a) => delta_lines(&corpus.docs[a.doc as usize]),
            None => Vec::new(),
        }
    }

    /// The patterns of the `Files` section of the plan, in the order of the
    /// file. A `Files` item is its first code span (`Files` list of the
    /// design, T8). An item with no code span counts for nothing.
    pub fn files(&self, corpus: &Corpus) -> Vec<String> {
        let Some(a) = &self.plan else {
            return Vec::new();
        };
        section_items(&corpus.docs[a.doc as usize], ItemSection::Files)
            .filter_map(|i| i.code.clone())
            .collect()
    }
}

/// The top-level list items of a section of a document.
pub fn section_items(doc: &Doc, section: ItemSection) -> impl Iterator<Item = &ListItem> {
    doc.parsed
        .items
        .iter()
        .filter(move |i| i.section == section)
}

/// True if the path is in the scope of one pattern of `Files`. A pattern is a
/// glob from the repository root (a leading `./` or `/` does not count). A
/// pattern with a final `/` matches the folder and each file below it. Any
/// other pattern matches the whole path.
pub fn in_scope(patterns: &[String], path: &str) -> bool {
    patterns.iter().any(|p| {
        let p = p.trim();
        let p = p.strip_prefix("./").unwrap_or(p).trim_start_matches('/');
        if p.is_empty() {
            return false;
        }
        match p.strip_suffix('/') {
            Some(folder) => !folder.is_empty() && glob::matches(&format!("{folder}/**"), path),
            None => glob::matches(p, path),
        }
    })
}

/// The verb of a delta line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Add,
    Change,
    Remove,
}

/// A delta line in a valid form:
///
/// - `ADD <id>#<key>: <sentence>`
/// - `CHANGE <id>#<key>@<16 hex digits>: <sentence>`
/// - `REMOVE <id>#<key>@<16 hex digits>`
#[derive(Clone, Debug, PartialEq)]
pub struct Delta {
    pub verb: Verb,
    /// The id of the target document, as the line writes it.
    pub id: String,
    /// The key of the target requirement. It is a slug.
    pub key: String,
    /// The hash of the target text that the author read: 16 lowercase
    /// hexadecimal digits. `ADD` has none.
    pub hash: Option<String>,
    /// The new text, as the line writes it. `REMOVE` has none.
    pub sentence: Option<String>,
}

impl Delta {
    /// The target as `<id>#<key>`.
    pub fn target(&self) -> String {
        format!("{}#{}", self.id, self.key)
    }

    /// The hash `frozen` of the sentence, as a requirement has it: the hash of
    /// the text without the markers and without link destinations. A
    /// comparison with the text of a requirement uses it.
    pub fn frozen(&self) -> Option<Hash> {
        let (clean, _, _) = strip_markers(self.sentence.as_deref()?);
        Some(hash_normalized(&strip_link_destinations(&clean)))
    }
}

/// One list item of the `Delta` section, with the result of its parse.
#[derive(Clone, Debug, PartialEq)]
pub struct DeltaLine {
    pub line: u32,
    /// The byte range of the item in the source of the spec.
    pub range: Range<u32>,
    /// The line, or the reason why it is invalid (a short sentence).
    pub parsed: Result<Delta, String>,
}

/// Parses the items of the `Delta` section of a spec. A second line for a
/// target that an earlier line has is invalid.
pub fn delta_lines(spec: &Doc) -> Vec<DeltaLine> {
    let mut seen: HashSet<String> = HashSet::new();
    section_items(spec, ItemSection::Delta)
        .map(|item| {
            let mut parsed = parse_delta(item);
            if let Ok(d) = &parsed
                && !seen.insert(d.target())
            {
                parsed = Err(format!(
                    "a line for `{}` is there already: one line for each key",
                    d.target()
                ));
            }
            DeltaLine {
                line: item.line,
                range: item.range.clone(),
                parsed,
            }
        })
        .collect()
}

/// Parses one item of the `Delta` section. An item is valid if it is one
/// paragraph with no nested list and has one of the 3 forms. The parser does
/// not stop at an invalid item: the result has the reason.
pub fn parse_delta(item: &ListItem) -> Result<Delta, String> {
    if item.nested {
        return Err("a delta line has no nested list".to_string());
    }
    if item.text.lines().any(|l| l.trim().is_empty()) {
        return Err("a delta line is one paragraph".to_string());
    }
    let words: Vec<&str> = item.text.split_whitespace().collect();
    let text = words.join(" ");
    let (verb, rest) = match words.first().copied() {
        Some("ADD") => (Verb::Add, text[3..].trim_start()),
        Some("CHANGE") => (Verb::Change, text[6..].trim_start()),
        Some("REMOVE") => (Verb::Remove, text[6..].trim_start()),
        _ => return Err("a delta line starts with ADD, CHANGE or REMOVE".to_string()),
    };
    let (target, sentence) = match verb {
        Verb::Remove => {
            if rest.contains([' ', ':']) {
                return Err("a REMOVE line has a target and a hash, and no text".to_string());
            }
            (rest, None)
        }
        _ => {
            let Some((target, sentence)) = rest.split_once(':') else {
                return Err("the target must have `:` and the new text after it".to_string());
            };
            let sentence = sentence.trim();
            if sentence.is_empty() {
                return Err("the line has no new text after `:`".to_string());
            }
            (target, Some(sentence.to_string()))
        }
    };
    if target.is_empty() || target.contains(char::is_whitespace) {
        return Err("the target must be `<id>#<key>`, with no space".to_string());
    }
    let (id_key, hash) = match target.split_once('@') {
        Some((a, h)) => (a, Some(h)),
        None => (target, None),
    };
    let Some((id, key)) = id_key.split_once('#') else {
        return Err("the target must be `<id>#<key>`".to_string());
    };
    if id.is_empty() || id.contains(['@', '#']) {
        return Err("the target must be `<id>#<key>`, with one document id".to_string());
    }
    if !is_slug(key) {
        return Err(format!(
            "`{key}` is not a key: use lowercase letters and digits joined by hyphens"
        ));
    }
    let hash = match (verb, hash) {
        (Verb::Add, None) => None,
        (Verb::Add, Some(_)) => return Err("an ADD line has no hash".to_string()),
        (_, None) => return Err("the target must have `@` and the 16 digit hash".to_string()),
        (_, Some(h)) if h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Some(h.to_ascii_lowercase())
        }
        (_, Some(_)) => return Err("the hash must be 16 hexadecimal digits".to_string()),
    };
    Ok(Delta {
        verb,
        id: id.to_string(),
        key: key.to_string(),
        hash,
        sentence,
    })
}

/// True if a hash of a delta line names the text of the requirement: it is
/// the hash that `show` prints, or the hash `frozen` (the same text with no
/// link destinations, so a `mv` that rewrote a link keeps the line).
pub fn hash_matches(req: &Req, hash: &str) -> bool {
    hash.eq_ignore_ascii_case(&hex(&req.hash)) || hash.eq_ignore_ascii_case(&hex(&req.frozen))
}

/// The valid requirement of a document id and a key in a tree.
pub fn target_req<'a>(corpus: &'a Corpus, id: &str, key: &str) -> Option<&'a Req> {
    let Res::Found(d, None, _) = corpus.resolve(id) else {
        return None;
    };
    let doc = &corpus.docs[d as usize];
    doc.req(key).map(|i| &doc.parsed.reqs[i])
}

/// The result of `applied`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    Yes,
    No,
    /// The line has no valid form, or it is a `REMOVE` of a key that neither
    /// tree has.
    Invalid,
}

/// Tells if the head tree has the result of a delta line (design, T3). `base`
/// is the tree before the change, and `head` is the tree now.
///
/// - `ADD` is applied if the base has no such requirement, and the head has
///   the text of the line.
/// - `CHANGE` is applied if the head has the text of the line.
/// - `REMOVE` is applied if the base has the requirement, and the head has
///   not. A `REMOVE` of a key that neither tree has is invalid.
///
/// A requirement has the text of a line if its hash `frozen` is the hash
/// `frozen` of the sentence of the line.
#[allow(dead_code)] // reader: S5
pub fn applied(line: &DeltaLine, base: &Corpus, head: &Corpus) -> Applied {
    let Ok(d) = &line.parsed else {
        return Applied::Invalid;
    };
    let in_base = target_req(base, &d.id, &d.key);
    let in_head = target_req(head, &d.id, &d.key);
    let has_text = |r: Option<&Req>| matches!((r, d.frozen()), (Some(r), Some(f)) if r.frozen == f);
    let yes = match d.verb {
        Verb::Add => in_base.is_none() && has_text(in_head),
        Verb::Change => has_text(in_head),
        Verb::Remove => {
            if in_base.is_none() && in_head.is_none() {
                return Applied::Invalid;
            }
            in_base.is_some() && in_head.is_none()
        }
    };
    if yes { Applied::Yes } else { Applied::No }
}

/// The targets `<id>#<key>` of the valid `ADD` lines of the changes with an
/// `accepted` intent, if the tree has no such requirement yet. A `covers:` tag
/// can name such a requirement before `finish` writes it (SD105). The id is as
/// the line writes it.
pub fn pending_adds(corpus: &Corpus) -> HashSet<String> {
    let mut out = HashSet::new();
    for c in collect(corpus) {
        if c.intent_status() != "accepted" {
            continue;
        }
        for line in c.delta(corpus) {
            if let Ok(d) = line.parsed
                && d.verb == Verb::Add
                && target_req(corpus, &d.id, &d.key).is_none()
            {
                out.insert(d.target());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::parse::parse;
    use std::sync::Arc;

    fn cfg() -> Config {
        Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap()
    }

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        Corpus::build(
            &cfg(),
            files.iter().map(|(p, _)| p.to_string()).collect(),
            files
                .iter()
                .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
                .collect(),
        )
    }

    fn item(text: &str) -> ListItem {
        let src = format!("# S\n\n## Delta\n\n{text}\n");
        let d = parse(&src);
        assert_eq!(d.items.len(), 1, "{text}");
        d.items[0].clone()
    }

    const VALVE: &str = "---\nstatus: active\n---\n# Valve\n\n## Requirements\n\n- **close**: The valve SHALL close in 5 seconds.\n- **open**: The valve SHALL open in 5 seconds. <!-- aliases: x#y -->\n";

    fn valve_hash(key: &str) -> (String, String) {
        let d = parse(VALVE);
        let r = d.reqs.iter().find(|r| r.key == key).unwrap();
        (hex(&r.hash), hex(&r.frozen))
    }

    #[test]
    fn a_delta_line_has_one_of_3_forms() {
        let (h, _) = valve_hash("close");
        let ok = |t: &str| parse_delta(&item(&format!("- {t}"))).unwrap();
        let add = ok("ADD requirements/valve#flush: The valve SHALL flush.");
        assert_eq!(add.verb, Verb::Add);
        assert_eq!(add.target(), "requirements/valve#flush");
        assert_eq!(add.hash, None);
        assert_eq!(add.sentence.as_deref(), Some("The valve SHALL flush."));
        let change = ok(&format!("CHANGE requirements/valve#close@{h}: New text."));
        assert_eq!(change.verb, Verb::Change);
        assert_eq!(change.hash.as_deref(), Some(h.as_str()));
        let remove = ok(&format!("REMOVE requirements/valve#close@{h}"));
        assert_eq!(remove.verb, Verb::Remove);
        assert_eq!(remove.sentence, None);
        // The hash can have capital letters. It is stored in lowercase. A
        // line that wraps is one paragraph.
        let wrapped = ok("CHANGE a/b#c-1@0123456789ABCDEF: First\n  second.");
        assert_eq!(wrapped.hash.as_deref(), Some("0123456789abcdef"));
        assert_eq!(wrapped.sentence.as_deref(), Some("First second."));
        // The text can have a colon, a link and a marker.
        let text = ok("ADD a#b: Use [x](../x.md#y): it SHALL work. <!-- aliases: a#c -->");
        assert_eq!(
            text.sentence.as_deref(),
            Some("Use [x](../x.md#y): it SHALL work. <!-- aliases: a#c -->")
        );
    }

    #[test]
    fn grammar_table_of_invalid_lines() {
        let bad = [
            ("- add a#b: lower case verb", "starts with"),
            ("- ADDS a#b: another word", "starts with"),
            ("- Update a#b: not a verb", "starts with"),
            ("- **k**: A requirement SHALL not be here.", "starts with"),
            ("- ADD", "target"),
            ("- ADD a#b", "`:`"),
            ("- ADD a#b:", "no new text"),
            ("- ADD a#b:   ", "no new text"),
            ("- ADD a: text", "<id>#<key>"),
            ("- ADD #b: text", "<id>#<key>"),
            ("- ADD a#: text", "not a key"),
            ("- ADD a#Bad_Key: text", "not a key"),
            ("- ADD a#b c: text", "no space"),
            ("- ADD a #b: text", "no space"),
            ("- ADD a#b#c: text", "not a key"),
            ("- ADD a@1#b: text", "<id>#<key>"),
            ("- ADD a#b@0123456789abcdef: text", "no hash"),
            ("- CHANGE a#b: text", "16 digit hash"),
            ("- CHANGE a#b@abc: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdeg: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdef01: text", "16 hexadecimal"),
            ("- CHANGE a#b@0123456789abcdef:", "no new text"),
            ("- CHANGE a#b@0123456789abcdef@1: text", "16 hexadecimal"),
            ("- REMOVE a#b", "16 digit hash"),
            ("- REMOVE a#b@0123456789abcdef: text", "no text"),
            ("- REMOVE a#b@0123456789abcdef text", "no text"),
            ("- ADD a#b: one\n\n  second paragraph", "one paragraph"),
            ("- ADD a#b: one\n  - nested", "nested"),
            ("-", "starts with"),
        ];
        for (text, reason) in bad {
            let err = parse_delta(&item(text)).unwrap_err();
            assert!(err.contains(reason), "{text:?}: {err}");
        }
    }

    #[test]
    fn a_spec_with_bad_lines_does_not_stop_the_parse() {
        let spec = "---\nstatus: draft\n---\n# S\n\n## Delta\n\n- ADD a#b: One.\n- garbage\n- ADD a#b: Two.\n- REMOVE a#c@0123456789abcdef\n- [ ] ADD a#d: task box\n\n  second\n";
        let c = corpus(&[("docs/intent/x/spec.md", spec)]);
        let lines = delta_lines(&c.docs[0]);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].parsed.is_ok());
        assert!(lines[1].parsed.is_err());
        // The second line for the key `a#b` is invalid, and so the first
        // line stays the only valid one for it.
        assert!(lines[2].parsed.as_ref().unwrap_err().contains("already"));
        assert!(lines[3].parsed.is_ok());
        assert!(lines[4].parsed.is_err());
        assert_eq!(lines[1].line, 9);
    }

    #[test]
    fn hash_matches_accepts_the_hash_and_the_frozen_hash() {
        let with_link = "- **k**: Use [a](../a.md#x) here.\n";
        let moved = "- **k**: Use [a](../b/a.md#x) here.\n";
        let a = parse(with_link);
        let b = parse(moved);
        let (ha, hb) = (hex(&a.reqs[0].hash), hex(&b.reqs[0].hash));
        assert_ne!(ha, hb);
        // The line has the hash of the old text. The new text has another
        // hash, but the same hash `frozen`.
        assert!(hash_matches(&a.reqs[0], &ha));
        assert!(!hash_matches(&b.reqs[0], &ha));
        assert!(hash_matches(&b.reqs[0], &hex(&a.reqs[0].frozen)));
        assert!(hash_matches(&b.reqs[0], &hb.to_uppercase()));
    }

    #[test]
    fn changes_are_collected_by_folder() {
        let intent = "---\nstatus: accepted\nclass: higher-risk\nareas: [valves, sensors]\nfrom: incidents/flood\n---\n# I\n\n## Intent\n\nText.\n";
        let c = corpus(&[
            (
                "docs/intent/b-change/spec.md",
                "---\nstatus: draft\n---\n# S\n",
            ),
            (
                "docs/intent/a-change/plan.md",
                "---\nstatus: approved\n---\n# P\n",
            ),
            ("docs/intent/a-change/intent.md", intent),
            (
                "docs/intent/a-change/spec.md",
                "---\nstatus: approved\n---\n# S\n",
            ),
            ("docs/requirements/valve.md", VALVE),
            // Not a place of an intent type: no change.
            (
                "docs/intent/loose.md",
                "---\ntype: intent\nstatus: draft\n---\n# L\n",
            ),
        ]);
        let changes = collect(&c);
        assert_eq!(changes.len(), 2);
        let a = &changes[0];
        assert_eq!(
            (a.slug.as_str(), a.dir.as_str()),
            ("a-change", "docs/intent/a-change")
        );
        assert_eq!(a.intent_status(), "accepted");
        assert_eq!(a.spec.as_ref().unwrap().status, "approved");
        assert_eq!(a.plan.as_ref().unwrap().status, "approved");
        assert_eq!(a.class, Class::HigherRisk);
        assert_eq!(a.areas, ["valves", "sensors"]);
        assert_eq!(a.from.as_deref(), Some("incidents/flood"));
        let b = &changes[1];
        assert_eq!(b.slug, "b-change");
        assert!(b.intent.is_none() && b.plan.is_none());
        assert_eq!((b.class, b.intent_status()), (Class::Routine, ""));
    }

    #[test]
    fn files_are_the_first_code_span_of_each_item() {
        let plan = "---\nstatus: draft\n---\n# P\n\n## Files\n\n- `src/valve.py` and `src/other.py`\n- `docs/`: the folder\n- no code here\n- [`tests/*.py`](x.md)\n  - `nested.py`\n\n## Order\n\n- `not-a-file.py`\n";
        let c = corpus(&[("docs/intent/x/plan.md", plan)]);
        let ch = &collect(&c)[0];
        // A code span in a link text is a code span.
        assert_eq!(ch.files(&c), ["src/valve.py", "docs/", "tests/*.py"]);
    }

    #[test]
    fn scope_matches_globs_and_folders() {
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let pats = p(&[
            "src/valve.py",
            "docs/",
            "tests/*.py",
            "./lib/**/x.rs",
            "/top.md",
        ]);
        for path in [
            "src/valve.py",
            "docs/a.md",
            "docs/a/b/c.md",
            "tests/a.py",
            "lib/x.rs",
            "lib/a/b/x.rs",
            "top.md",
        ] {
            assert!(in_scope(&pats, path), "{path}");
        }
        for path in [
            "src/valve.py.bak",
            "src/other.py",
            "documents/a.md",
            "tests/a/b.py",
            "lib/x.rs.in",
        ] {
            assert!(!in_scope(&pats, path), "{path}");
        }
        // A plain path with no final `/` does not match the files below it.
        assert!(!in_scope(&p(&["src"]), "src/a.rs"));
        assert!(in_scope(&p(&["src/"]), "src/a.rs"));
        assert!(!in_scope(&p(&["", "/", "./"]), "a"));
        assert!(!in_scope(&[], "a"));
    }

    fn base_and_head() -> (Corpus, Corpus) {
        let base = corpus(&[("docs/requirements/valve.md", VALVE)]);
        let head_text = VALVE
            .replace("close in 5 seconds", "close in 3 seconds")
            .replace(
                "- **open**: The valve SHALL open in 5 seconds. <!-- aliases: x#y -->\n",
                "",
            )
            + "- **flush**: The valve SHALL flush the line.\n";
        let head = corpus(&[("docs/requirements/valve.md", &head_text)]);
        (base, head)
    }

    fn line(text: &str) -> DeltaLine {
        let spec = format!("---\nstatus: draft\n---\n# S\n\n## Delta\n\n- {text}\n");
        let c = corpus(&[("docs/intent/x/spec.md", &spec)]);
        delta_lines(&c.docs[0]).remove(0)
    }

    #[test]
    fn applied_follows_the_rules_of_t3() {
        let (base, head) = base_and_head();
        let (hc, _) = valve_hash("close");
        let (ho, _) = valve_hash("open");
        let t = "requirements/valve";
        let check = |text: String, want: Applied| {
            assert_eq!(applied(&line(&text), &base, &head), want, "{text}");
        };
        // ADD: the base has no key, and the head has the text.
        check(
            format!("ADD {t}#flush: The valve SHALL flush the line."),
            Applied::Yes,
        );
        check(format!("ADD {t}#flush: Other text."), Applied::No);
        check(
            format!("ADD {t}#absent: The valve SHALL flush the line."),
            Applied::No,
        );
        // The key is in the base: not an ADD that applies, also if the text is there.
        check(
            format!("ADD {t}#close: The valve SHALL close in 3 seconds."),
            Applied::No,
        );
        // CHANGE: the head has the text.
        check(
            format!("CHANGE {t}#close@{hc}: The valve SHALL close in 3 seconds."),
            Applied::Yes,
        );
        check(
            format!("CHANGE {t}#close@{hc}: The valve SHALL close in 4 seconds."),
            Applied::No,
        );
        check(format!("CHANGE {t}#absent@{hc}: Text."), Applied::No);
        // REMOVE: the base has the key and the head has not.
        check(format!("REMOVE {t}#open@{ho}"), Applied::Yes);
        check(format!("REMOVE {t}#close@{hc}"), Applied::No);
        // A key in neither tree, and a line with no valid form.
        check(format!("REMOVE {t}#nothing@{hc}"), Applied::Invalid);
        check(
            format!("REMOVE nothing/here#nothing@{hc}"),
            Applied::Invalid,
        );
        check("garbage".to_string(), Applied::Invalid);
        // A marker and the layout of the line do not change the text.
        check(
            format!(
                "ADD {t}#flush:  The valve   SHALL flush\n  the line. <!-- verified-by: tests/a.rs -->"
            ),
            Applied::Yes,
        );
    }

    #[test]
    fn pending_adds_come_from_accepted_changes() {
        let spec = "---\nstatus: draft\n---\n# S\n\n## Delta\n\n- ADD requirements/valve#flush: Text.\n- ADD requirements/valve#close: Text.\n- CHANGE requirements/valve#open@0123456789abcdef: Text.\n- ADD requirements/new#first: Text.\n";
        let intent = |st: &str| format!("---\nstatus: {st}\n---\n# I\n\n## Intent\n\nText.\n");
        let c = corpus(&[
            ("docs/requirements/valve.md", VALVE),
            ("docs/intent/a/intent.md", &intent("accepted")),
            ("docs/intent/a/spec.md", spec),
            ("docs/intent/b/intent.md", &intent("draft")),
            ("docs/intent/b/spec.md", spec),
            ("docs/intent/c/intent.md", &intent("closed")),
            ("docs/intent/c/spec.md", spec),
        ]);
        let got = pending_adds(&c);
        let mut got: Vec<&str> = got.iter().map(String::as_str).collect();
        got.sort();
        // `valve#close` is in the tree already.
        assert_eq!(got, ["requirements/new#first", "requirements/valve#flush"]);
    }
}
