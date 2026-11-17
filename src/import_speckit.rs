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

use crate::import_docs::{
    Ctx, Lines, Pair, PairKind, Read, SourceFile, State, Target, id_of, indent, is_list_line,
    plain_heading, requirements_path, slug, split_sentence, with_front_matter,
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

/// The text of a task and its path, for a line of `tasks.md` (decision I9):
/// the marks `[P]` and `[US1]` go, and the path is the token after the last
/// ` in ` that has a `/` or a `.`, with no white space. A final group in
/// parentheses is no part of the path: its text goes to the text of the task.
pub fn plan_task(rest: &str) -> (String, Option<String>) {
    let words: Vec<&str> = rest
        .split_whitespace()
        .filter(|w| {
            !(*w == "[P]"
                || w.strip_prefix("[US")
                    .and_then(|d| d.strip_suffix(']'))
                    .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())))
        })
        .collect();
    let rest = words.join(" ");
    if let Some(i) = rest.rfind(" in ") {
        let mut cand = rest[i + 4..].trim();
        // A final group in parentheses is no part of the path. Its text stays
        // in the text of the task.
        let mut group = "";
        if cand.ends_with(')')
            && let Some(p) = cand.rfind(" (")
        {
            group = cand[p + 1..].trim();
            cand = cand[..p].trim_end();
        }
        let cand = cand
            .strip_suffix('.')
            .filter(|c| !c.is_empty())
            .unwrap_or(cand);
        if !cand.is_empty()
            && !cand.contains(char::is_whitespace)
            && !cand.contains('`')
            && cand.contains(['/', '.'])
        {
            let head = rest[..i].trim_end();
            let text = if group.is_empty() {
                head.to_string()
            } else {
                format!("{head} {group}")
            };
            return (text, Some(cand.to_string()));
        }
    }
    (rest, None)
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
    // No `specs/` is no stop: a second run after the sources are gone.
    let specs = ctx.under("specs/");
    let mut read = Read {
        universe: tool.iter().chain(&specs).map(|f| f.to_string()).collect(),
        ..Read::default()
    };
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

/// The stop for a requirement or a principle with no sentence: its text is a
/// list, or empty.
pub fn no_sentence(path: &str, line: usize, key: &str) -> Fatal {
    Fatal::Refused(format!(
        "{path}:{line}: `{key}` has no sentence (its text is empty, or it starts with a list); write the sentence before the list, commit, then run import again. import changes nothing"
    ))
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
    if let Some(p) = found.iter().find(|p| split_sentence(&p.body).0.is_empty()) {
        return Err(no_sentence(CONSTITUTION, p.line + 1, &p.key));
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
            r.key
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
        open_feature(ctx, read, folder, name, &text, tasks_text.as_deref())?;
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

/// A line of `- **FR-nnn**` at the start of a list item.
fn is_fr_item(line: &str) -> bool {
    let t = line.trim_start();
    let Some(t) = t
        .strip_prefix("- **FR-")
        .or_else(|| t.strip_prefix("* **FR-"))
    else {
        return false;
    };
    let d = t.chars().take_while(char::is_ascii_digit).count();
    d > 0 && t[d..].starts_with("**")
}

/// The lines `from..to` of a text for a sub-section of the intent: each
/// heading goes one level down and loses its annotation, and a requirement
/// line `FR-nnn` is not in the text (it is in the delta). A heading with no
/// text under it goes too.
pub fn section_lines(l: &Lines, from: usize, to: usize, title: &str, level: usize) -> Vec<String> {
    let mut blocks: Vec<(usize, Vec<String>)> =
        vec![(level, vec![format!("{} {title}", "#".repeat(level))])];
    let mut fence = false;
    let mut skipping = false;
    for line in &l.lines[from..to] {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fence = !fence;
            blocks.last_mut().unwrap().1.push(line.to_string());
            continue;
        }
        if fence {
            blocks.last_mut().unwrap().1.push(line.to_string());
            continue;
        }
        if is_fr_item(line) {
            skipping = true;
            continue;
        }
        if skipping {
            if line.starts_with("  ") || line.starts_with('\t') || line.trim().is_empty() {
                continue;
            }
            skipping = false;
            // A line that no blank line parts from the list before it would
            // join the last item.
            if let Some(last) = blocks.last_mut().map(|b| &mut b.1)
                && last.last().is_some_and(|x| !x.trim().is_empty())
                && !is_list_line(line)
            {
                last.push(String::new());
            }
        }
        if let Some((lvl, text)) = crate::import_docs::heading_of(line) {
            let lvl = (lvl + 1).min(6);
            blocks.push((
                lvl,
                vec![format!("{} {}", "#".repeat(lvl), plain_heading(&text))],
            ));
        } else {
            blocks.last_mut().unwrap().1.push(line.to_string());
        }
    }
    let blank = |b: &(usize, Vec<String>)| b.1[1..].iter().all(|x| x.trim().is_empty());
    let mut keep = vec![false; blocks.len()];
    for i in (0..blocks.len()).rev() {
        let inner = blocks[i + 1..]
            .iter()
            .zip(&keep[i + 1..])
            .take_while(|(b, _)| b.0 > blocks[i].0)
            .any(|(_, k)| *k);
        keep[i] = !blank(&blocks[i]) || inner;
    }
    blocks
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .flat_map(|(b, _)| b.1)
        .collect()
}

/// Joins blocks of lines with one blank line between them.
pub fn join_blocks(blocks: &[Vec<String>]) -> String {
    let mut out = String::new();
    for b in blocks {
        let t = crate::import_docs::trim_blank(&b.join("\n"));
        if t.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&collapse_blank(&t));
    }
    out.push('\n');
    out
}

fn collapse_blank(t: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for l in t.lines() {
        if l.trim().is_empty() && out.last().is_some_and(|p| p.trim().is_empty()) {
            continue;
        }
        out.push(l);
    }
    out.join("\n")
}

/// The title of a feature: the text after `Feature Specification:` of the H1.
fn title_of(l: &Lines, name: &str) -> String {
    let h1 = l
        .headings
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.text.as_str());
    let t = h1.map_or("", |t| {
        let lower = t.to_lowercase();
        match lower.find("feature specification:") {
            Some(i) => t[i + "feature specification:".len()..].trim(),
            None => t.trim(),
        }
    });
    if t.is_empty() {
        crate::import_docs::humanize(name)
    } else {
        t.to_string()
    }
}

fn open_feature(
    ctx: &Ctx,
    read: &mut Read,
    folder: &str,
    name: &str,
    text: &str,
    tasks: Option<&str>,
) -> Result<(), Fatal> {
    let spec_path = format!("specs/{folder}/spec.md");
    let tasks_path = format!("specs/{folder}/tasks.md");
    let dir = format!("{}/{name}", ctx.places.intent_dir()?);
    let (intent_to, spec_to, plan_to) = (
        format!("{dir}/intent.md"),
        format!("{dir}/spec.md"),
        format!("{dir}/plan.md"),
    );
    let req_to = requirements_path(ctx.places.requirements_dir()?, ctx.area_of(folder), name);
    let req_id = id_of(ctx.cfg, &req_to).unwrap_or_default();
    let linked = (ctx.link)(text, &spec_path, &intent_to);
    let l = Lines::new(&linked);
    let title = title_of(&l, name);

    // The requirements: the pairs read the source, the lines read the text
    // with its links for the new place.
    let src = fr_lines(text);
    let lines = parse(&linked);
    let nested = |key: &str| -> Option<String> {
        let r = lines.reqs.iter().find(|r| r.key == key)?;
        let rest = linked.get(r.sentence.end as usize..r.end as usize)?;
        let rest = crate::mv::dedent(rest.trim_start_matches('\n').trim_end());
        (!rest.trim().is_empty()).then(|| crate::import_docs::trim_blank(&rest))
    };
    let mut delta = Vec::new();
    let mut questions = Vec::new();
    let mut details: Vec<Vec<String>> = Vec::new();
    for (key, line, sentence) in &src {
        let lower = key.to_lowercase();
        let own = lines
            .reqs
            .iter()
            .find(|r| &r.key == key)
            .map_or(sentence.as_str(), |r| r.text.as_str());
        let (clean, question) = crate::import_docs::strip_clarification(own);
        delta.push(format!("- ADD {req_id}#{lower}: {clean}"));
        let (_, q) = crate::import_docs::strip_clarification(sentence);
        if let Some(q) = &q {
            questions.push(format!("- [ ] {lower}: {q}"));
        }
        let _ = question;
        if let Some(n) = nested(key) {
            details.push(vec![format!("### Details of {lower}"), String::new(), n]);
        }
        read.pairs.push(Pair {
            from: format!("{spec_path}:{line}"),
            id: format!("{req_id}#{lower}"),
            delta: Some(name.to_string()),
            target: spec_to.clone(),
            key: lower,
            kind: PairKind::Delta,
            sentence: sentence.clone(),
            question: q,
            verb: Some("ADD"),
        });
    }

    // intent.md
    let mut stories: Option<String> = None;
    let mut sections: Vec<Vec<String>> = Vec::new();
    let first_h2 = l
        .headings
        .iter()
        .find(|h| h.level == 2)
        .map_or(l.lines.len(), |h| h.line);
    let h1_line = l
        .headings
        .iter()
        .find(|h| h.level == 1)
        .map_or(0, |h| h.line + 1);
    let preamble = l.block(h1_line.min(first_h2), first_h2);
    for (i, h) in l.headings.iter().enumerate() {
        if h.level != 2 {
            continue;
        }
        let title2 = plain_heading(&h.text);
        let lower = title2.to_lowercase();
        let end = l.tree_end(i);
        if stories.is_none() && (lower.contains("user scenarios") || lower.contains("user stor")) {
            stories = Some(l.block(h.line + 1, end));
        } else {
            sections.push(section_lines(&l, h.line + 1, end, &title2, 3));
        }
    }
    let mut blocks: Vec<Vec<String>> = vec![
        vec![format!("# {title}")],
        vec!["## Intent".to_string()],
        vec![
            stories
                .unwrap_or_else(|| "The source has no section for the user stories.".to_string()),
        ],
    ];
    blocks.extend(sections);
    blocks.extend(details);
    if !preamble.is_empty() {
        blocks.push(vec![
            "### Metadata of the source".to_string(),
            String::new(),
            preamble,
        ]);
    }
    // The text before the title is no heading, so it stays as plain text.
    if let Some(h1) = l.headings.iter().find(|h| h.level == 1) {
        let before = l.block(0, h1.line);
        if !before.is_empty() {
            blocks.push(vec![
                "### Text before the title of the source".to_string(),
                String::new(),
                before,
            ]);
        }
    }
    let intent = with_front_matter(
        &join_blocks(&blocks),
        "draft",
        &[("class", "higher-risk")],
        &[],
    );

    // spec.md
    let mut sb: Vec<Vec<String>> = vec![
        vec![format!("# {title}")],
        vec!["## Delta".to_string()],
        delta,
        vec!["## Open questions".to_string()],
    ];
    sb.push(if questions.is_empty() {
        vec!["- none.".to_string()]
    } else {
        questions
    });
    sb.push(vec!["## Concerns".to_string()]);
    sb.push(vec!["- none.".to_string()]);
    let spec = with_front_matter(&join_blocks(&sb), "draft", &[], &[]);

    let mut targets = vec![
        (
            intent_to.clone(),
            intent,
            vec![spec_path.clone()],
            Some(spec_path.clone()),
        ),
        (spec_to.clone(), spec, vec![spec_path.clone()], None),
    ];
    let mut spec_state = vec![intent_to, spec_to];

    // plan.md
    if let Some(t) = tasks {
        let linked_tasks = (ctx.link)(t, &tasks_path, &plan_to);
        let mut files: Vec<String> = Vec::new();
        let mut order: Vec<String> = Vec::new();
        for line in linked_tasks.lines() {
            if let Some((lvl, text)) = crate::import_docs::heading_of(line) {
                if lvl >= 2 {
                    order.push(String::new());
                    order.push(format!("**{}**", plain_heading(&text)));
                    order.push(String::new());
                }
                continue;
            }
            let Some(task) = task_line(line) else {
                // A line that is no task stays as it is.
                order.push(line.trim_end().to_string());
                continue;
            };
            let (text, path) = plan_task(&task.rest);
            let text = text.trim_end_matches('.').trim();
            let text = if text.is_empty() {
                task.id.as_str()
            } else {
                text
            };
            let mut item = format!(
                "- [{}] {}: {text}.",
                if task.done { "x" } else { " " },
                task.id.to_lowercase()
            );
            if let Some(p) = path {
                item.push_str(&format!(" Files: `{p}`."));
                if !files.contains(&p) {
                    files.push(p);
                }
            }
            order.push(item);
        }
        let mut pb: Vec<Vec<String>> =
            vec![vec![format!("# {title}")], vec!["## Files".to_string()]];
        pb.push(if files.is_empty() {
            vec!["- none".to_string()]
        } else {
            files.iter().map(|f| format!("- `{f}`")).collect()
        });
        pb.push(vec!["## Order".to_string()]);
        pb.push(order);
        let plan = with_front_matter(&join_blocks(&pb), "draft", &[], &[]);
        targets.push((
            plan_to.clone(),
            plan,
            vec![tasks_path.clone()],
            Some(tasks_path.clone()),
        ));
        read.sources.push(SourceFile {
            path: tasks_path,
            state: State::Written,
            to: vec![plan_to],
            reason: "an open feature".to_string(),
        });
    }
    read.sources.push(SourceFile {
        path: spec_path.clone(),
        state: State::Written,
        to: std::mem::take(&mut spec_state),
        reason: "an open feature".to_string(),
    });
    for (path, text, sources, link_of) in targets {
        read.targets.push(Target {
            path,
            text,
            moved_from: None,
            sources,
            link_of,
            scenarios: None,
        });
    }
    read.removes.push(format!("specs/{folder}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::import_docs::{Places, State, corpus_of};
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
            self.read.as_ref().expect_err("an error")
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
        // No `specs/` is no stop: a second run after the sources are gone.
        assert!(run(&[(".specify/init-options.json", "{}\n")]).read.is_ok());
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

    // --- the open features ---

    const SPEC_OPEN: &str = "# Feature Specification: Album export\n\n**Feature Branch**: `003-album-export`\n**Status**: Draft\n\n## User Scenarios & Testing *(mandatory)*\n\n### User Story 1 - Export an album (Priority: P1)\n\nA person downloads an album as one archive.\n\n## Requirements *(mandatory)*\n\n### Functional Requirements\n\n- **FR-001**: The system MUST export an album as one archive.\n- **FR-002**: The system MUST offer [NEEDS CLARIFICATION: which file formats?] as formats.\n\n### Key Entities\n\n- **Archive**: One file with the photos.\n\n## Success Criteria *(mandatory)*\n\n- **SC-001**: A person exports 100 photos in 1 minute.\n";

    const TASKS_OPEN: &str = "# Tasks: Album export\n\n## Phase 1: Setup\n\n- [x] T001 Create the export module in src/export/__init__.py\n\n## Phase 2: User Story 1\n\n- [ ] T002 [P] [US1] Write the archive builder in src/export/zip.py\n- [ ] T003 Add a progress line to the export screen\n";

    fn open() -> Run {
        run(&tool(&[
            ("specs/003-album-export/spec.md", SPEC_OPEN),
            ("specs/003-album-export/tasks.md", TASKS_OPEN),
            ("specs/003-album-export/plan.md", "# Plan\n"),
        ]))
    }

    #[test]
    fn an_open_feature_gives_a_draft_intent_spec_and_plan_and_its_sources_stay() {
        let r = open();
        for f in ["intent", "spec", "plan"] {
            r.target(&format!("docs/intent/album-export/{f}.md"));
        }
        let s = r.source("specs/003-album-export/spec.md");
        assert_eq!(s.state, State::Written);
        assert_eq!(
            s.to,
            [
                "docs/intent/album-export/intent.md",
                "docs/intent/album-export/spec.md"
            ]
        );
        let t = r.source("specs/003-album-export/tasks.md");
        assert_eq!(
            (t.state, t.to.as_slice()),
            (
                State::Written,
                &["docs/intent/album-export/plan.md".to_string()][..]
            )
        );
        assert_eq!(
            r.source("specs/003-album-export/plan.md").state,
            State::Kept
        );
        assert_eq!(r.ok().removes, ["specs/003-album-export"]);
        let link: Vec<(&str, Option<&str>)> = r
            .ok()
            .targets
            .iter()
            .map(|t| (t.path.as_str(), t.link_of.as_deref()))
            .collect();
        assert!(
            link.contains(&(
                "docs/intent/album-export/intent.md",
                Some("specs/003-album-export/spec.md")
            )),
            "{link:?}"
        );
        assert!(
            link.contains(&(
                "docs/intent/album-export/plan.md",
                Some("specs/003-album-export/tasks.md")
            )),
            "{link:?}"
        );
        assert!(r.ok().targets.iter().all(|t| t.moved_from.is_none()));
    }

    #[test]
    fn the_intent_holds_the_user_stories_and_the_other_sections_but_not_the_requirements() {
        let t = open()
            .target("docs/intent/album-export/intent.md")
            .to_string();
        let want = "---\nstatus: draft\nclass: higher-risk\n---\n# Album export\n\n## Intent\n\n### User Story 1 - Export an album (Priority: P1)\n\nA person downloads an album as one archive.\n\n### Requirements\n\n#### Key Entities\n\n- **Archive**: One file with the photos.\n\n### Success Criteria\n\n- **SC-001**: A person exports 100 photos in 1 minute.\n\n### Metadata of the source\n\n**Feature Branch**: `003-album-export`\n**Status**: Draft\n";
        assert_eq!(t, want);
    }

    #[test]
    fn the_spec_has_a_delta_line_and_an_open_question_for_each_marker() {
        let r = open();
        let t = r.target("docs/intent/album-export/spec.md");
        let want = "---\nstatus: draft\n---\n# Album export\n\n## Delta\n\n- ADD requirements/album-export#fr-001: The system MUST export an album as one archive.\n- ADD requirements/album-export#fr-002: The system MUST offer as formats.\n\n## Open questions\n\n- [ ] fr-002: which file formats?\n\n## Concerns\n\n- none.\n";
        assert_eq!(t, want);
        let pairs: Vec<_> = r
            .ok()
            .pairs
            .iter()
            .map(|p| {
                (
                    p.from.as_str(),
                    p.id.as_str(),
                    p.delta.as_deref(),
                    p.kind,
                    p.verb,
                    p.question.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            pairs,
            [
                (
                    "specs/003-album-export/spec.md:16",
                    "requirements/album-export#fr-001",
                    Some("album-export"),
                    PairKind::Delta,
                    Some("ADD"),
                    None
                ),
                (
                    "specs/003-album-export/spec.md:17",
                    "requirements/album-export#fr-002",
                    Some("album-export"),
                    PairKind::Delta,
                    Some("ADD"),
                    Some("which file formats?")
                ),
            ]
        );
        assert!(r.ok().pairs[1].sentence.contains("[NEEDS CLARIFICATION"));
        assert!(!r.ok().fr_lines, "an open feature needs no legacy key");
    }

    #[test]
    fn the_plan_lists_the_paths_and_the_tasks() {
        let t = open()
            .target("docs/intent/album-export/plan.md")
            .to_string();
        let want = "---\nstatus: draft\n---\n# Album export\n\n## Files\n\n- `src/export/__init__.py`\n- `src/export/zip.py`\n\n## Order\n\n**Phase 1: Setup**\n\n- [x] t001: Create the export module. Files: `src/export/__init__.py`.\n\n**Phase 2: User Story 1**\n\n- [ ] t002: Write the archive builder. Files: `src/export/zip.py`.\n- [ ] t003: Add a progress line to the export screen.\n";
        assert_eq!(t, want);
    }

    #[test]
    fn a_plan_with_no_path_says_none_and_a_feature_with_no_tasks_has_no_plan() {
        let r = run(&tool(&[
            ("specs/003-x/spec.md", SPEC_OPEN),
            ("specs/003-x/tasks.md", "- [ ] T001 Think about it\n"),
        ]));
        let t = r.target("docs/intent/x/plan.md");
        assert!(t.contains("\n## Files\n\n- none\n"), "{t}");
        let r = run(&tool(&[("specs/003-x/spec.md", SPEC_OPEN)]));
        assert!(r.ok().targets.iter().all(|t| !t.path.ends_with("plan.md")));
        r.target("docs/intent/x/spec.md");
    }

    #[test]
    fn a_task_gives_its_text_and_its_path() {
        for (rest, text, path) in [
            (
                "Create the export module in src/export/__init__.py",
                "Create the export module",
                Some("src/export/__init__.py"),
            ),
            (
                "[P] [US1] Write the archive builder in src/export/zip.py",
                "Write the archive builder",
                Some("src/export/zip.py"),
            ),
            (
                "Add a progress line to the export screen",
                "Add a progress line to the export screen",
                None,
            ),
            (
                "Write the endpoint in src/albums/create.py (new file)",
                "Write the endpoint (new file)",
                Some("src/albums/create.py"),
            ),
            (
                "Work in the team repo in progress",
                "Work in the team repo in progress",
                None,
            ),
            (
                "Fix a bug in the parser in src/p.rs.",
                "Fix a bug in the parser",
                Some("src/p.rs"),
            ),
            ("Read in a file", "Read in a file", None),
            ("Edit in `src/a.rs`", "Edit in `src/a.rs`", None),
        ] {
            let (t, p) = plan_task(rest);
            assert_eq!((t.as_str(), p.as_deref()), (text, path), "{rest}");
        }
    }

    #[test]
    fn the_area_of_a_feature_is_in_its_delta_lines() {
        let r = run_with(
            &tool(&[("specs/003-album-export/spec.md", SPEC_OPEN)]),
            false,
            false,
            &[("003-album-export", "media")],
        );
        let t = r.target("docs/intent/album-export/spec.md");
        assert!(
            t.contains("- ADD requirements/media/album-export#fr-001:"),
            "{t}"
        );
    }

    #[test]
    fn the_nested_text_of_a_requirement_line_stays_in_the_intent() {
        let spec = "# Feature Specification: A\n\n## Requirements\n\n- **FR-001**: The system MUST do it.\n  - It does it fast.\n  - It does it twice.\n";
        let r = run(&tool(&[("specs/001-a/spec.md", spec)]));
        let i = r.target("docs/intent/a/intent.md");
        assert!(
            i.contains("### Details of fr-001\n\n- It does it fast.\n- It does it twice.\n"),
            "{i}"
        );
        let s = r.target("docs/intent/a/spec.md");
        assert!(
            s.contains("- ADD requirements/a#fr-001: The system MUST do it.\n"),
            "{s}"
        );
    }

    #[test]
    fn a_feature_with_no_user_story_section_still_has_an_intent_section() {
        let spec = "# Feature Specification: A\n\n## Requirements\n\n- **FR-001**: The system MUST do it.\n";
        let r = run(&tool(&[("specs/001-a/spec.md", spec)]));
        let i = r.target("docs/intent/a/intent.md");
        assert!(
            i.contains("\n## Intent\n\nThe source has no section for the user stories.\n"),
            "{i}"
        );
    }

    #[test]
    fn a_requirement_line_with_no_colon_is_a_delta_line() {
        let spec = "# Feature Specification: A\n\n- **FR-001** The system MUST do it.\n- **FR-002**: The system MUST do more.\n";
        let r = run(&tool(&[("specs/001-a/spec.md", spec)]));
        let ids: Vec<&str> = r.ok().pairs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["requirements/a#fr-001", "requirements/a#fr-002"]);
        let s = r.target("docs/intent/a/spec.md");
        assert!(
            s.contains("- ADD requirements/a#fr-001: The system MUST do it.\n"),
            "{s}"
        );
    }

    #[test]
    fn the_lines_of_tasks_md_that_are_no_task_stay_in_the_plan() {
        let tasks = "# Tasks\n\nPrerequisites: a database.\n\n## Phase 1\n\n- [ ] T001 Do it in src/a.py\n\nCheckpoint: it MUST pass.\n\n## Dependencies\n\n- Phase 2 needs phase 1.\n";
        let r = run(&tool(&[
            ("specs/003-x/spec.md", SPEC_OPEN),
            ("specs/003-x/tasks.md", tasks),
        ]));
        let t = r.target("docs/intent/x/plan.md");
        let want = "---\nstatus: draft\n---\n# Album export\n\n## Files\n\n- `src/a.py`\n\n## Order\n\nPrerequisites: a database.\n\n**Phase 1**\n\n- [ ] t001: Do it. Files: `src/a.py`.\n\nCheckpoint: it MUST pass.\n\n**Dependencies**\n\n- Phase 2 needs phase 1.\n";
        assert_eq!(t, want);
    }

    #[test]
    fn a_principle_with_no_sentence_stops_the_import() {
        let c = "# C\n\n## Core Principles\n\n### I. Small\n\n- one\n- two\n";
        let r = run(&tool(&[(".specify/memory/constitution.md", c)]));
        assert!(matches!(r.err(), Fatal::Refused(_)));
        assert!(
            r.err().message().contains("no sentence"),
            "{}",
            r.err().message()
        );
    }

    #[test]
    fn a_moved_extra_file_that_was_a_document_keeps_its_old_id_as_an_alias() {
        let text = format!(
            "{CFG}\n[[roots]]\npath = \"specs\"\ntype = \"requirements\"\nstatus = \"active\"\n"
        );
        let mut all = vec![("spec-dna.toml", text.as_str())];
        let files = tool(&[
            ("specs/001-a/spec.md", SPEC_DONE),
            ("specs/001-a/tasks.md", TASKS_DONE),
            ("specs/001-a/notes.md", "# Notes\n"),
        ]);
        all.extend_from_slice(&files);
        let tree = TreeData::from_map(
            all.iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        );
        let cfg = Config::parse(&text).unwrap();
        let places = Places::of(&cfg);
        let r = tree
            .with(|t| {
                let (before, _) = corpus_of(t, &cfg)?;
                read(&Ctx {
                    tree: t,
                    cfg: &cfg,
                    places: &places,
                    areas: &[],
                    skip_open: false,
                    keep_research: true,
                    link: &|x, _, _| x.to_string(),
                    before: &before,
                })
            })
            .unwrap();
        let t = r
            .targets
            .iter()
            .find(|t| t.path == "docs/research/imported/001-a/notes.md")
            .unwrap();
        assert!(t.text.contains("aliases:\n  - 001-a/notes\n"), "{}", t.text);
    }

    #[test]
    fn a_task_keeps_the_text_in_parentheses_after_the_path() {
        let (t, p) =
            plan_task("[P] Write the builder in src/export/zip.py (each archive MUST be signed)");
        assert_eq!(t, "Write the builder (each archive MUST be signed)");
        assert_eq!(p.as_deref(), Some("src/export/zip.py"));
    }

    #[test]
    fn the_text_before_the_title_and_a_lazy_line_stay_in_the_intent() {
        let spec = "Review note: each export MUST be logged.\n\n# Feature Specification: A\n\n## Requirements\n\n- Another item.\n- **FR-001**: The system MUST do it.\nNote: a lazy line.\n";
        let r = run(&tool(&[("specs/001-a/spec.md", spec)]));
        let i = r.target("docs/intent/a/intent.md");
        assert!(
            i.contains("Review note: each export MUST be logged."),
            "{i}"
        );
        assert!(i.contains("- Another item.\n\nNote: a lazy line."), "{i}");
    }
}
