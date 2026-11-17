//! The reader of a project of Spec Kit 1.4.1 (design 0.7.0, D23; decisions I7
//! to I12 of the plan). It reads the constitution, the done features and the
//! open features, and it gives a `Read`: the targets, the state of each source
//! file and the pairs of requirements. The import changes no sentence: each
//! requirement line stays as the source has it, and the proof compares it.
//!
//! - The constitution `.specify/memory/constitution.md` becomes a `rule`
//!   document. `Core Principles` becomes `Rules`, and each principle becomes a
//!   requirement with the slug of its heading as key.
//! - A feature `specs/<digits>-<name>/` is done if its `tasks.md` has a task
//!   and no open task. Its `spec.md` moves to a `requirements` document, and
//!   `FR-nnn` is a legacy key.
//! - An open feature becomes a draft `intent.md`, `spec.md` and `plan.md`.
//!   Each file stays, because the sources are written, not moved.

#![allow(dead_code)]

use crate::import_docs::{
    Ctx, Lines, Pair, PairKind, Read, SourceFile, State, Target, id_of, indent, plain_heading,
    requirements_path, slug, split_sentence, with_front_matter,
};
use crate::model::is_slug;
use crate::parse::parse;
use crate::repo::Fatal;
use std::collections::BTreeMap;

/// The supported version of the tool, for a message.
pub const VERSION: &str = "Spec Kit 1.4.1";

const CONSTITUTION: &str = ".specify/memory/constitution.md";

/// The key of a principle: the slug of its heading, with no leading numeral
/// (a Roman or an Arabic numeral and its `.` or `)`).
pub fn principle_key(heading: &str) -> String {
    let h = plain_heading(heading);
    let t = h.trim();
    let end = t
        .find(|c: char| !(c.is_ascii_digit() || "IVXLCDM".contains(c)))
        .unwrap_or(t.len());
    let t = if end > 0 && t[end..].starts_with(['.', ')']) {
        &t[end + 1..]
    } else {
        t
    };
    slug(t)
}

/// The number of task lines of a `tasks.md`, and the number of open tasks. A
/// task line is a box (`- [ ]`, `- [x]` or `- [X]`), then `T` and digits.
pub fn tasks_state(text: &str) -> (usize, usize) {
    let mut all = 0;
    let mut open = 0;
    for l in text.lines() {
        let Some(task) = task_line(l) else { continue };
        all += 1;
        if !task.done {
            open += 1;
        }
    }
    (all, open)
}

/// A task line of a `tasks.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskLine {
    pub done: bool,
    /// `T001` as the source has it.
    pub id: String,
    /// The text after the id, with the marks `[P]` and `[US1]` not removed.
    pub rest: String,
}

pub fn task_line(line: &str) -> Option<TaskLine> {
    let t = line.trim_start();
    let t = t.strip_prefix("- [")?;
    let done = match t.chars().next()? {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    let t = t[1..].strip_prefix("] ")?.trim_start();
    let t = t.strip_prefix('T')?;
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || !(t[digits..].is_empty() || t[digits..].starts_with(char::is_whitespace)) {
        return None;
    }
    Some(TaskLine {
        done,
        id: format!("T{}", &t[..digits]),
        rest: t[digits..].trim().to_string(),
    })
}

/// A folder of `specs/`: `<digits>-<name>` with a slug as name.
fn feature_name(folder: &str) -> Option<&str> {
    let (digits, name) = folder.split_once('-')?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && is_slug(name))
        .then_some(name)
}

fn layout_error(why: &str) -> Fatal {
    Fatal::Config(format!(
        "{why}; the import supports {VERSION} (`.specify/` and `specs/<digits>-<name>/`)"
    ))
}

/// Reads a project of Spec Kit.
pub fn read(ctx: &Ctx) -> Result<Read, Fatal> {
    let tool = ctx.under(".specify/");
    if tool.is_empty() {
        let old = ctx.under("memory/").len() + ctx.under("scripts/").len() > 0;
        return Err(layout_error(&format!(
            "`.specify/` is not in the tree{}",
            if old {
                ", and `memory/` and `scripts/` at the top are the layout of an older version"
            } else {
                ""
            }
        )));
    }
    let specs = ctx.under("specs/");
    if specs.is_empty() {
        return Err(layout_error("`specs/` is not in the tree"));
    }
    let mut read = Read::default();
    read.universe = tool.iter().chain(&specs).map(|f| f.to_string()).collect();
    for f in &tool {
        if *f != CONSTITUTION {
            read.sources.push(SourceFile::kept(f, "a file of the tool"));
        }
    }
    if tool.contains(&CONSTITUTION) {
        constitution(ctx, &mut read)?;
    }
    let mut folders: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for f in &specs {
        match f["specs/".len()..].split_once('/') {
            Some((folder, rest)) => folders.entry(folder).or_default().push(rest),
            None => read
                .sources
                .push(SourceFile::skipped(f, "not in a feature folder")),
        }
    }
    for (folder, files) in folders {
        match feature_name(folder) {
            Some(name) => feature(ctx, &mut read, folder, name, &files)?,
            None => read.sources.push(SourceFile::skipped(
                &format!("specs/{folder}/"),
                "not a feature folder (`<digits>-<name>`)",
            )),
        }
    }
    read.sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(read)
}

// --- the constitution ---

/// The first template placeholder of a text: `[PROJECT_NAME]`, or another
/// token of capitals, digits and `_` in brackets.
fn placeholder(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(i) = rest.find('[') {
        let after = &rest[i + 1..];
        if let Some(j) = after.find(']') {
            let inner = &after[..j];
            if inner.len() >= 3
                && inner.starts_with(|c: char| c.is_ascii_uppercase())
                && inner
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                && inner.contains('_')
            {
                return Some(format!("[{inner}]"));
            }
        }
        rest = after;
    }
    None
}

/// A principle: its key, the line of its heading (from 0) and its text.
struct Principle {
    key: String,
    line: usize,
    own_end: usize,
    body: String,
}

/// The principles of a constitution: the level 3 headings under the level 2
/// heading `Core Principles`. `None` if the heading is absent.
fn principles(l: &Lines) -> Option<(usize, Vec<Principle>)> {
    let ci = l.headings.iter().position(|h| {
        h.level == 2 && plain_heading(&h.text).eq_ignore_ascii_case("Core Principles")
    })?;
    let end = l.tree_end(ci);
    let mut out = Vec::new();
    for (j, h) in l.headings.iter().enumerate().skip(ci + 1) {
        if h.line >= end {
            break;
        }
        if h.level == 3 {
            out.push(Principle {
                key: principle_key(&h.text),
                line: h.line,
                own_end: l.own_end(j),
                body: l.block(h.line + 1, l.own_end(j)),
            });
        }
    }
    Some((ci, out))
}

/// The list item of a principle or a requirement: the key, the first
/// paragraphs, and the rest as a nested block.
pub fn item(key: &str, body: &str) -> String {
    let (sentence, rest) = split_sentence(body);
    let mut out = format!("- **{key}**:");
    let mut lines = sentence.lines();
    if let Some(first) = lines.next() {
        out.push(' ');
        out.push_str(first.trim_start());
    }
    let tail: Vec<&str> = lines.collect();
    if !tail.is_empty() {
        out.push('\n');
        out.push_str(&indent(&tail.join("\n"), 2));
    }
    if !rest.is_empty() {
        out.push_str("\n\n");
        out.push_str(&indent(&rest, 2));
    }
    out
}

fn constitution(ctx: &Ctx, read: &mut Read) -> Result<(), Fatal> {
    let skip = |read: &mut Read, why: &str| {
        read.sources.push(SourceFile::skipped(CONSTITUTION, why));
    };
    let orig = ctx.text(CONSTITUTION)?;
    if let Some(p) = placeholder(&orig) {
        skip(
            read,
            &format!("the file still has template placeholders (`{p}`)"),
        );
        return Ok(());
    }
    let ol = Lines::new(&orig);
    let Some((_, found)) = principles(&ol) else {
        skip(read, "no `Core Principles` heading");
        return Ok(());
    };
    if found.is_empty() {
        skip(
            read,
            "`Core Principles` has no principle (a level 3 heading)",
        );
        return Ok(());
    }
    if let Some(p) = found.iter().find(|p| p.key.is_empty()) {
        return Err(Fatal::Refused(format!(
            "{CONSTITUTION}:{}: cannot make a key from this principle heading; rename it, commit, then run import again",
            p.line + 1
        )));
    }
    let dir = ctx.places.rules_dir()?;
    let to = format!("{dir}/constitution.md");
    let linked = (ctx.link)(&orig, CONSTITUTION, &to);
    let l = Lines::new(&linked);
    let (ci, pr) = principles(&l).expect("the same text has the same headings");
    let core_line = l.headings[ci].line;
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < l.lines.len() {
        if i == core_line {
            out.push("## Rules".to_string());
        } else if let Some(p) = pr.iter().find(|p| p.line == i) {
            out.push(item(&p.key, &p.body));
            out.push(String::new());
            i = p.own_end;
            continue;
        } else {
            out.push(l.lines[i].to_string());
        }
        i += 1;
    }
    let mut text = out.join("\n");
    text.push('\n');
    let id = id_of(ctx.cfg, &to).unwrap_or_default();
    let aliases: Vec<String> = ctx.old_alias(CONSTITUTION, &id).into_iter().collect();
    let text = with_front_matter(&text, "active", &[], &aliases);
    for p in &found {
        read.pairs.push(Pair {
            from: format!("{CONSTITUTION}:{}", p.line + 1),
            id: format!("{id}#{}", p.key),
            delta: None,
            target: to.clone(),
            key: p.key.clone(),
            kind: PairKind::Requirement,
            sentence: split_sentence(&p.body).0,
            question: None,
            verb: None,
        });
    }
    read.sources.push(SourceFile {
        path: CONSTITUTION.to_string(),
        state: State::Moved,
        to: vec![to.clone()],
        reason: "the rules of the project".to_string(),
    });
    read.targets.push(Target {
        path: to,
        text,
        moved_from: Some(CONSTITUTION.to_string()),
        sources: vec![CONSTITUTION.to_string()],
        link_of: None,
        scenarios: None,
    });
    Ok(())
}

// --- the features ---

/// The `FR-nnn` requirements of a spec: (key, line, sentence).
fn fr_lines(text: &str) -> Vec<(String, u32, String)> {
    parse(text)
        .reqs
        .iter()
        .filter(|r| {
            r.colon
                && r.key
                    .strip_prefix("FR-")
                    .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|r| (r.key.clone(), r.line, r.text.clone()))
        .collect()
}

fn feature(
    ctx: &Ctx,
    read: &mut Read,
    folder: &str,
    name: &str,
    files: &[&str],
) -> Result<(), Fatal> {
    let path = |f: &str| format!("specs/{folder}/{f}");
    let spec = path("spec.md");
    let tasks = path("tasks.md");
    let has_spec = files.contains(&"spec.md");
    let tasks_text = if files.contains(&"tasks.md") {
        Some(ctx.text(&tasks)?)
    } else {
        None
    };
    let (n, open) = tasks_text.as_deref().map_or((0, 0), tasks_state);
    let done = n > 0 && open == 0;
    if !has_spec {
        for f in files {
            let p = path(f);
            if *f == "tasks.md" {
                read.sources
                    .push(SourceFile::kept(&p, "the folder has no spec.md"));
            } else {
                ctx.extra(read, &p, folder, f, "an extra file of the feature")?;
            }
        }
        return Ok(());
    }
    if !done && ctx.skip_open {
        for f in files {
            read.sources.push(SourceFile::skipped(
                &path(f),
                "an open feature; --skip-open does not read it",
            ));
        }
        return Ok(());
    }
    let text = ctx.text(&spec)?;
    let frs = fr_lines(&text);
    if frs.is_empty() {
        return Err(Fatal::Config(format!(
            "{spec}: the spec file has no requirement (no `FR-nnn` line); the import supports {VERSION}"
        )));
    }
    if done {
        done_feature(ctx, read, folder, name, &text, &frs)?;
    } else {
        open_feature(ctx, read, folder, name, &text, &frs, tasks_text.as_deref())?;
    }
    for f in files {
        let p = path(f);
        match *f {
            "spec.md" => {}
            "tasks.md" if done => read
                .sources
                .push(SourceFile::kept(&p, "the tasks of a done feature")),
            "tasks.md" => {}
            _ => ctx.extra(read, &p, folder, f, "an extra file of the feature")?,
        }
    }
    Ok(())
}

fn done_feature(
    ctx: &Ctx,
    read: &mut Read,
    folder: &str,
    name: &str,
    text: &str,
    frs: &[(String, u32, String)],
) -> Result<(), Fatal> {
    let spec = format!("specs/{folder}/spec.md");
    let to = requirements_path(ctx.places.requirements_dir()?, ctx.area_of(folder), name);
    let id = id_of(ctx.cfg, &to).unwrap_or_default();
    let mut aliases = vec![folder.to_string()];
    aliases.extend(ctx.old_alias(&spec, &id));
    let linked = (ctx.link)(text, &spec, &to);
    let out = with_front_matter(&linked, "active", &[], &aliases);
    for (key, line, sentence) in frs {
        read.pairs.push(Pair {
            from: format!("{spec}:{line}"),
            id: format!("{id}#{key}"),
            delta: None,
            target: to.clone(),
            key: key.clone(),
            kind: PairKind::Requirement,
            sentence: sentence.clone(),
            question: None,
            verb: None,
        });
    }
    read.fr_lines = true;
    read.sources.push(SourceFile {
        path: spec.clone(),
        state: State::Moved,
        to: vec![to.clone()],
        reason: "a done feature".to_string(),
    });
    read.targets.push(Target {
        path: to,
        text: out,
        moved_from: Some(spec.clone()),
        sources: vec![spec],
        link_of: None,
        scenarios: None,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn open_feature(
    ctx: &Ctx,
    read: &mut Read,
    folder: &str,
    name: &str,
    text: &str,
    frs: &[(String, u32, String)],
    tasks: Option<&str>,
) -> Result<(), Fatal> {
    let _ = (ctx, read, folder, name, text, frs, tasks);
    Err(Fatal::Config("an open feature: not built yet".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::import_docs::{Places, State, corpus_of, id_of};
    use crate::upgrade_docs::TreeData;

    const CFG: &str = "spec-dna = \"0.6.1\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[layers.research]\ntypes = [\"research\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

    const CONSTITUTION: &str = "# Albums Constitution\n\n## Core Principles\n\n### I. Library-First\n\nEvery feature starts as a library. A library MUST work on its own.\n\n### 2) Test-First\n\nTests come first.\n\nThe cycle is strict.\n\n- Write the test.\n- See it fail.\n\n## Governance\n\nThe rules are above other practice.\n";

    const SPEC_DONE: &str = "# Feature Specification: Photo albums\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: The system MUST create an album.\n- **FR-002**: The system MUST add a photo. The [plan](./plan.md) says how.\n\n## Success Criteria\n\n- **SC-001**: A person creates an album in 2 minutes.\n";

    const TASKS_DONE: &str = "# Tasks\n\n- [x] T001 Write it in src/a.py\n- [X] T002 [P] Test it\n";

    struct Run {
        read: Result<Read, Fatal>,
    }

    fn run_with(
        files: &[(&str, &str)],
        skip_open: bool,
        keep: bool,
        areas: &[(&str, &str)],
    ) -> Run {
        let mut all: Vec<(&str, &str)> = vec![("spec-dna.toml", CFG)];
        all.extend_from_slice(files);
        let tree = TreeData::from_map(
            all.iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        );
        let cfg = Config::parse(CFG).unwrap();
        let places = Places::of(&cfg);
        let areas: Vec<(String, String)> = areas
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let read = tree.with(|t| {
            let (before, _) = corpus_of(t, &cfg)?;
            read(&Ctx {
                tree: t,
                cfg: &cfg,
                places: &places,
                areas: &areas,
                skip_open,
                keep_research: keep,
                link: &|text, _, _| text.to_string(),
                before: &before,
            })
        });
        Run { read }
    }

    fn run(files: &[(&str, &str)]) -> Run {
        run_with(files, false, false, &[])
    }

    impl Run {
        fn ok(&self) -> &Read {
            self.read
                .as_ref()
                .unwrap_or_else(|e| panic!("{}", e.message()))
        }

        fn target(&self, path: &str) -> &str {
            let r = self.ok();
            r.targets
                .iter()
                .find(|t| t.path == path)
                .map(|t| t.text.as_str())
                .unwrap_or_else(|| {
                    panic!(
                        "no target {path}: {:?}",
                        r.targets.iter().map(|t| &t.path).collect::<Vec<_>>()
                    )
                })
        }

        fn source(&self, path: &str) -> &crate::import_docs::SourceFile {
            self.ok()
                .sources
                .iter()
                .find(|s| s.path == path)
                .unwrap_or_else(|| panic!("no source {path}: {:?}", self.ok().sources))
        }

        fn err(&self) -> &Fatal {
            self.read.as_ref().err().expect("an error")
        }
    }

    fn tool(extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        let mut v = vec![
            (".specify/init-options.json", "{}\n"),
            ("specs/README.md", "x\n"),
        ];
        v.extend_from_slice(extra);
        v
    }

    // --- the constitution ---

    #[test]
    fn the_constitution_becomes_a_rule_document_with_a_requirement_for_each_principle() {
        let r = run(&tool(&[(".specify/memory/constitution.md", CONSTITUTION)]));
        let t = r.target("docs/rules/constitution.md");
        assert!(
            t.starts_with("---\nstatus: active\n---\n# Albums Constitution\n\n## Rules\n"),
            "{t}"
        );
        assert!(
            t.contains("- **library-first**: Every feature starts as a library. A library MUST work on its own.\n"),
            "{t}"
        );
        assert!(t.contains("- **test-first**: Tests come first.\n\n  The cycle is strict.\n\n  - Write the test.\n  - See it fail.\n"), "{t}");
        assert!(
            t.contains("\n## Governance\n\nThe rules are above other practice.\n"),
            "{t}"
        );
        let s = r.source(".specify/memory/constitution.md");
        assert_eq!(s.state, State::Moved);
        assert_eq!(s.to, ["docs/rules/constitution.md"]);
        let tg = &r.ok().targets[0];
        assert_eq!(
            tg.moved_from.as_deref(),
            Some(".specify/memory/constitution.md")
        );
        // One pair for each principle, from the line of its heading.
        let pairs: Vec<(&str, &str)> = r
            .ok()
            .pairs
            .iter()
            .map(|p| (p.from.as_str(), p.id.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                (
                    ".specify/memory/constitution.md:5",
                    "rules/constitution#library-first"
                ),
                (
                    ".specify/memory/constitution.md:9",
                    "rules/constitution#test-first"
                ),
            ]
        );
        assert_eq!(
            r.ok().pairs[1].sentence,
            "Tests come first.\n\nThe cycle is strict."
        );
    }

    #[test]
    fn a_constitution_with_a_placeholder_or_with_no_principles_is_skipped_with_its_reason() {
        let template = "# [PROJECT_NAME] Constitution\n\n## Core Principles\n\n### [PRINCIPLE_1_NAME]\n\n[PRINCIPLE_1_DESCRIPTION]\n";
        let r = run(&tool(&[(".specify/memory/constitution.md", template)]));
        let s = r.source(".specify/memory/constitution.md");
        assert_eq!(s.state, State::Skipped);
        assert!(s.reason.contains("[PROJECT_NAME]"), "{}", s.reason);
        assert!(r.ok().targets.is_empty());
        let r = run(&tool(&[(
            ".specify/memory/constitution.md",
            "# C\n\n## Values\n\nBe kind.\n",
        )]));
        let s = r.source(".specify/memory/constitution.md");
        assert_eq!(s.state, State::Skipped);
        assert_eq!(s.reason, "no `Core Principles` heading");
        let r = run(&tool(&[(
            ".specify/memory/constitution.md",
            "# C\n\n## Core Principles\n\nText only.\n",
        )]));
        assert!(
            r.source(".specify/memory/constitution.md")
                .reason
                .contains("no principle")
        );
    }

    #[test]
    fn the_key_of_a_principle_has_no_leading_numeral() {
        for (heading, key) in [
            ("I. Library-First", "library-first"),
            ("IV. Simple Things", "simple-things"),
            ("2) Test-First", "test-first"),
            ("12. Observability *(required)*", "observability"),
            ("Plain Name", "plain-name"),
            ("Iterative Work", "iterative-work"),
        ] {
            assert_eq!(principle_key(heading), key, "{heading}");
        }
    }

    // --- the layout, D25 ---

    #[test]
    fn a_layout_that_the_import_does_not_know_is_exit_code_3_and_names_the_rule() {
        let r = run(&[("specs/001-a/spec.md", SPEC_DONE)]);
        let e = r.err();
        assert!(matches!(e, Fatal::Config(_)), "{e:?}");
        assert!(
            e.message().contains(".specify/") && e.message().contains(VERSION),
            "{}",
            e.message()
        );
        let r = run(&[(".specify/init-options.json", "{}\n")]);
        assert!(
            r.err().message().contains("specs/"),
            "{}",
            r.err().message()
        );
        // An older layout has `memory/` and `scripts/` at the top.
        let r = run(&[("memory/constitution.md", "# C\n"), ("scripts/a.sh", "x\n")]);
        assert!(
            r.err().message().contains("memory/"),
            "{}",
            r.err().message()
        );
    }

    #[test]
    fn a_spec_file_with_no_requirement_stops_the_import_and_names_the_file() {
        let r = run(&tool(&[
            (
                "specs/001-a/spec.md",
                "# Feature Specification: A\n\nOnly prose.\n",
            ),
            ("specs/001-a/tasks.md", TASKS_DONE),
        ]));
        let m = r.err().message().to_string();
        assert!(matches!(r.err(), Fatal::Config(_)));
        assert!(
            m.contains("specs/001-a/spec.md") && m.contains("no requirement"),
            "{m}"
        );
    }

    // --- the done features ---

    #[test]
    fn a_done_feature_moves_to_a_requirements_document_with_an_alias() {
        let r = run(&tool(&[
            ("specs/001-photo-albums/spec.md", SPEC_DONE),
            ("specs/001-photo-albums/tasks.md", TASKS_DONE),
        ]));
        let t = r.target("docs/requirements/photo-albums.md");
        assert!(t.starts_with("---\nstatus: active\naliases:\n  - 001-photo-albums\n---\n# Feature Specification: Photo albums\n"), "{t}");
        assert!(
            t.contains("- **FR-002**: The system MUST add a photo."),
            "{t}"
        );
        assert!(r.ok().fr_lines);
        let ids: Vec<&str> = r.ok().pairs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "requirements/photo-albums#FR-001",
                "requirements/photo-albums#FR-002"
            ]
        );
        assert_eq!(r.ok().pairs[0].from, "specs/001-photo-albums/spec.md:7");
        assert_eq!(r.ok().pairs[0].sentence, "The system MUST create an album.");
        let s = r.source("specs/001-photo-albums/spec.md");
        assert_eq!(
            (s.state, s.to.as_slice()),
            (
                State::Moved,
                &["docs/requirements/photo-albums.md".to_string()][..]
            )
        );
        let k = r.source("specs/001-photo-albums/tasks.md");
        assert_eq!(
            (k.state, k.reason.as_str()),
            (State::Kept, "the tasks of a done feature")
        );
        assert_eq!(
            r.source(".specify/init-options.json").reason,
            "a file of the tool"
        );
    }

    #[test]
    fn an_area_places_the_done_feature() {
        let r = run_with(
            &tool(&[
                ("specs/001-photo-albums/spec.md", SPEC_DONE),
                ("specs/001-photo-albums/tasks.md", TASKS_DONE),
            ]),
            false,
            false,
            &[("001-photo-albums", "media")],
        );
        r.target("docs/requirements/media/photo-albums.md");
        assert_eq!(r.ok().pairs[0].id, "requirements/media/photo-albums#FR-001");
    }

    #[test]
    fn a_task_line_is_a_box_then_t_and_digits_and_a_feature_with_no_open_task_is_done() {
        use super::tasks_state;
        assert_eq!(tasks_state(TASKS_DONE), (2, 0));
        assert_eq!(tasks_state("- [ ] T001 a\n- [x] T002 b\n"), (2, 1));
        assert_eq!(
            tasks_state("# Tasks\n\nNo tasks.\n- [ ] not a task\n"),
            (0, 0)
        );
        assert_eq!(tasks_state("  - [ ] T10 indented\n"), (1, 1));
    }

    #[test]
    fn the_extra_files_stay_or_move_to_research() {
        let files = tool(&[
            ("specs/001-photo-albums/spec.md", SPEC_DONE),
            ("specs/001-photo-albums/tasks.md", TASKS_DONE),
            ("specs/001-photo-albums/plan.md", "# Plan\n"),
            ("specs/001-photo-albums/contracts/a.yaml", "a: 1\n"),
            ("specs/001-photo-albums/checklists/req.md", "# C\n"),
        ]);
        let r = run(&files);
        assert_eq!(
            r.source("specs/001-photo-albums/plan.md").state,
            State::Kept
        );
        assert_eq!(
            r.source("specs/001-photo-albums/contracts/a.yaml").reason,
            "not a Markdown file"
        );
        let r = run_with(&files, false, true, &[]);
        r.target("docs/research/imported/001-photo-albums/plan.md");
        r.target("docs/research/imported/001-photo-albums/checklists/req.md");
        let s = r.source("specs/001-photo-albums/plan.md");
        assert_eq!(
            (s.state, s.to.as_slice()),
            (
                State::Moved,
                &["docs/research/imported/001-photo-albums/plan.md".to_string()][..]
            )
        );
        assert_eq!(
            r.source("specs/001-photo-albums/contracts/a.yaml").state,
            State::Kept
        );
    }

    #[test]
    fn a_folder_that_is_no_feature_is_skipped() {
        let r = run(&tool(&[
            ("specs/notes/a.md", "x\n"),
            ("specs/notes/b.md", "y\n"),
            ("specs/Bad_Name/spec.md", "x\n"),
            ("specs/12-/spec.md", "x\n"),
        ]));
        let s = r.source("specs/notes/");
        assert_eq!(s.state, State::Skipped);
        assert_eq!(s.reason, "not a feature folder (`<digits>-<name>`)");
        r.source("specs/Bad_Name/");
        r.source("specs/12-/");
        let s = r.source("specs/README.md");
        assert_eq!(s.state, State::Skipped);
    }

    #[test]
    fn skip_open_does_not_read_an_open_feature() {
        // The spec is a template: with no `FR-nnn` line it would stop the import.
        let files = tool(&[
            (
                "specs/003-x/spec.md",
                "# Feature Specification: [FEATURE NAME]\n",
            ),
            ("specs/003-x/tasks.md", "# Tasks\n"),
        ]);
        assert!(run(&files).read.is_err());
        let r = run_with(&files, true, false, &[]);
        let s = r.source("specs/003-x/spec.md");
        assert_eq!(s.state, State::Skipped);
        assert!(s.reason.contains("--skip-open"), "{}", s.reason);
        assert!(r.ok().targets.is_empty());
    }
}
