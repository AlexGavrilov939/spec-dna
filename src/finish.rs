//! `spec-dna finish <slug>` (design 0.6.0, "Finish"). It applies the delta of
//! a change to the requirements, checks the evidence, and deletes the folder of
//! the change. It is the one command that deletes documents.
//!
//! The work is a plan: `plan` runs the steps 0 to 5 of the design in memory. It
//! reads the head tree, the lifecycle base tree and the text of the files that it
//! edits. It writes nothing. A stop returns the step and the diagnostics, and the
//! tree stays as it was. If no step stops, the plan lists the files to write and
//! the files to delete. The caller (`write_plan`) then writes all of them, in one
//! pass, deletes the folder, and stages the paths. The same plan gives the diff of
//! `--dry-run`.
//!
//! The stops do not follow the severity of a code, the profile or the policy:
//! `finish` stops on step 0, SD914, SD916, SD919, SD920 and SD922, and on a
//! delta line that the result does not apply (design, K3). Each diagnostic that
//! a stop reports is an error.
//!
//! An edit starts from a new parse of the text on disk. It goes by byte range,
//! the last range first, so the text outside the range stays as it was: CRLF, a
//! BOM, the final newline, the markers and the nested lists.

use crate::change::{self, Applied, Change, Delta, DeltaLine, Verb};
use crate::check;
use crate::config::Config;
use crate::diag::{Code, Diag, Severity};
use crate::lifecycle::{self, Diff};
use crate::model::{Class, Corpus, Doc, DocType, LinkRes, Res, join};
use crate::mv::parent;
use crate::parse::{ItemSection, ParsedDoc, RefKind, parse};
use crate::registry;
use crate::split;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// The input of `plan`.
pub struct Ctx<'a> {
    pub config: &'a Config,
    /// The working tree, with the `covers:` tags of the test files.
    pub head: &'a Corpus,
    /// The lifecycle base tree. `None` if git cannot name the base.
    pub base: Option<&'a Corpus>,
    /// The diff of the working tree against the lifecycle base.
    pub diff: Option<&'a Diff>,
    /// The folder of the change, from the repository root, with no final `/`.
    pub folder: &'a str,
    /// Each tracked file of the folder, from the repository root.
    pub folder_files: &'a [String],
    /// Reads the text of a file of the working tree.
    pub read: &'a dyn Fn(&str) -> Result<String, String>,
    /// True if a path exists on disk, tracked or not.
    pub exists: &'a dyn Fn(&str) -> bool,
    /// The problem of a path that `finish` writes or deletes (a symbolic link
    /// in the path, or a path outside the repository), or `None`. `main` gives
    /// `path_problem` of the repository root.
    pub path_problem: &'a dyn Fn(&str) -> Option<String>,
    /// The read of the test reports (design 0.8.0, D1), with the result of the
    /// bind on the tags of `head`. `None`: the reports are off, so step 5 reads
    /// the tags as 0.7.1 does.
    pub reports: Option<&'a crate::report::Reads>,
    /// The facts of the acceptance record that `main` read with git (design
    /// 0.8.0, D3). `None`: steps 5.2 and 5.5 run no check, as in 0.7.1.
    pub acceptance: Option<&'a crate::acceptance::FinishFacts>,
}

/// A file that `finish` writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Write {
    pub path: String,
    /// The text now, or `None` for a new file.
    pub old: Option<String>,
    pub new: String,
}

/// The result of a plan with no stop.
#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    /// The files to write, in the order of their paths.
    pub writes: Vec<Write>,
    /// The tracked files of the folder to delete, in the order of their paths.
    pub removes: Vec<String>,
    /// The folder of the change. After the deletion, `write_plan` removes the
    /// folders under it that are empty, and the folder itself.
    pub folder: String,
    /// The delta lines that the plan applies: `ADD`, `CHANGE`, `REMOVE`.
    pub applied: (usize, usize, usize),

    /// The hours of the tasks of the plan of the change.
    pub hours: Hours,
    /// The notes that the report prints with the result (design 0.8.0, D3 item
    /// 1: the note for a plan of a version before 0.8.0).
    pub notes: Vec<String>,
    /// The archive file that step 5.5 adds to `writes`, if the change needed a
    /// record.
    pub archive: Option<String>,
    /// Step 5.3 (design 0.8.0, D7): the deviations of the plan, for a change that
    /// needs a record, each with the verdict of the record (`none` if the record
    /// has no entry for it). `finish` prints them. In 0.8.0 the merge of the pull
    /// request acknowledges them.
    pub deviations: Vec<Deviation>,
}

/// True if `finish --json` has the key `deviations` (design 0.8.0, D7): the plan
/// has at least one deviation, or the pin of `spec-dna.toml` is 0.8.0 or later. A
/// project of 0.7.1 with no deviation sees the output of 0.7.1.
pub fn emits_deviations(pin: &str, count: usize) -> bool {
    count > 0
        || crate::config::Version::parse(pin).is_some_and(|v| v >= crate::config::Version(0, 8, 0))
}

/// One deviation as step 5.3 reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct Deviation {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub verdict: String,
}

/// The hours of the tasks of a plan (design 0.7.0, section 15). Each task that
/// is not a duplicate and is not skipped counts. Its label `Hours:` gives a
/// number or a range ([`change::parse_hours`]).
#[derive(Debug, Default, PartialEq)]
pub struct Hours {
    /// The total as (low, high) in hundredths of an hour: the sums of the low
    /// ends and of the high ends. It is `None` if no counted task has the label
    /// `Hours:`, or if a value is invalid. The sum of integers has no rounding
    /// error.
    pub total: Option<(u64, u64)>,
    /// The id and the text of each counted task with an invalid value.
    pub invalid: Vec<(String, String)>,
    /// The ids of the counted tasks that have no `Hours:`. It is empty if no
    /// counted task has one, or if a value is invalid: the notes of those
    /// cases are other notes.
    pub missing: Vec<String>,
    /// The sum of the labels `Actual:` in hundredths of an hour (design 0.7.1,
    /// B7). Each task that is not a duplicate counts, also a skipped task: its
    /// time was spent. It is `None` if no such task has the label, or if a
    /// value is invalid.
    pub actual: Option<u64>,
    /// The id and the text of each task with an invalid `Actual:`.
    pub actual_invalid: Vec<(String, String)>,
    /// The ids of the done tasks (not skipped) that have no `Actual:`. It is
    /// empty if no task has the label, or if a value is invalid.
    pub actual_missing: Vec<String>,
}

impl Hours {
    /// The total as the line `Hours: <total>` writes it: `<low>` if the ends
    /// are equal, else `<low>-<high>`, with no zero after the last digit.
    pub fn text(&self) -> Option<String> {
        self.total.map(|(low, high)| {
            if low == high {
                format_hours(low)
            } else {
                format!("{}-{}", format_hours(low), format_hours(high))
            }
        })
    }

    /// The total as the value of `hours` in `--json`: `{"low", "high"}`, or
    /// `null`.
    pub fn json(&self) -> String {
        match self.total {
            Some((low, high)) => crate::json::Obj::new()
                .raw("low", &format_hours(low))
                .raw("high", &format_hours(high))
                .end(),
            None => "null".to_string(),
        }
    }
}

impl Hours {
    /// The total of the actual hours as a number of hours, or `None`.
    pub fn actual_text(&self) -> Option<String> {
        self.actual.map(format_hours)
    }

    /// The value of `actual` in `--json`: a number, or `null`.
    pub fn actual_json(&self) -> String {
        self.actual_text().unwrap_or_else(|| "null".to_string())
    }

    /// The line `Hours: ...` of the commit message (design 0.7.1, B7):
    /// `<estimate>, actual <n>`, `actual <n>` with no estimate, or
    /// `<estimate>`. `None` if the plan gives neither.
    pub fn line(&self) -> Option<String> {
        match (self.text(), self.actual_text()) {
            (Some(h), Some(a)) => Some(format!("Hours: {h}, actual {a}")),
            (None, Some(a)) => Some(format!("Hours: actual {a}")),
            (Some(h), None) => Some(format!("Hours: {h}")),
            (None, None) => None,
        }
    }
}

/// A number of hundredths as a number of hours, with no zero after the last
/// digit: `350` is `3.5`, and `400` is `4`.
pub fn format_hours(hundredths: u64) -> String {
    let (whole, frac) = (hundredths / 100, hundredths % 100);
    match frac {
        0 => whole.to_string(),
        f if f % 10 == 0 => format!("{whole}.{}", f / 10),
        f => format!("{whole}.{f:02}"),
    }
}

/// Adds the actual hours of the tasks of a plan into `out`.
fn task_actual(tasks: &[change::PlanTask], out: &mut Hours) {
    let mut sum = 0u64;
    let mut any = false;
    let mut missing = Vec::new();
    for t in tasks.iter().filter(|t| !t.duplicate) {
        let Some(value) = &t.actual else {
            if t.is_done() {
                missing.push(t.id.clone());
            }
            continue;
        };
        any = true;
        match change::parse_actual(value) {
            Some(n) => sum += n,
            None => out
                .actual_invalid
                .push((t.id.clone(), lifecycle::short(value))),
        }
    }
    if any && out.actual_invalid.is_empty() {
        out.actual = Some(sum);
        out.actual_missing = missing;
    }
}

/// Adds the hours of the tasks of a plan.
fn task_hours(tasks: &[change::PlanTask]) -> Hours {
    let mut out = Hours::default();
    task_actual(tasks, &mut out);
    let (mut low, mut high) = (0u64, 0u64);
    let mut any = false;
    let mut missing = Vec::new();
    for t in tasks.iter().filter(|t| !t.duplicate && !t.skipped()) {
        let Some(value) = &t.hours else {
            missing.push(t.id.clone());
            continue;
        };
        any = true;
        match change::parse_hours(value) {
            Some((l, h)) => {
                low += l;
                high += h;
            }
            None => out.invalid.push((t.id.clone(), lifecycle::short(value))),
        }
    }
    if any && out.invalid.is_empty() {
        out.total = Some((low, high));
        out.missing = missing;
    }
    out
}

/// A stop: the step of the design and the reason.
#[derive(Debug)]
pub struct Stop {
    pub step: u8,
    pub reason: String,
    pub diags: Vec<Diag>,
}

#[derive(Debug)]
pub enum Failure {
    Stop(Stop),
    /// The change is `accepted`, and git cannot name the lifecycle base.
    NoBase,
    /// A read failed, or a file is not valid UTF-8.
    Io(String),
}

fn stop<T>(step: u8, reason: impl Into<String>, diags: Vec<Diag>) -> Result<T, Failure> {
    Err(Failure::Stop(Stop {
        step,
        reason: reason.into(),
        diags: diags.into_iter().map(as_error).collect(),
    }))
}

/// The most open tasks that a stop names with their text.
const OPEN_TASKS_SHOWN: usize = 10;

/// The reason of the stop for open tasks: the count, the id and the text of
/// the first open tasks (at most `OPEN_TASKS_SHOWN`, the text cut at 60
/// characters), and then the ids of the others, so that no open task is left
/// out.
fn open_tasks_reason(open: &[change::PlanTask], slug: &str) -> String {
    let shown: Vec<String> = open
        .iter()
        .take(OPEN_TASKS_SHOWN)
        .map(|t| {
            let text: String = t.text.chars().take(60).collect();
            format!("{}: {}", t.id, text.trim_end())
        })
        .collect();
    let rest: Vec<&str> = open
        .iter()
        .skip(OPEN_TASKS_SHOWN)
        .map(|t| t.id.as_str())
        .collect();
    let more = if rest.is_empty() {
        String::new()
    } else {
        format!("; also open: {}", rest.join(", "))
    };
    // A task that names `spec-dna finish` with no slug, or with another one,
    // gets a hint: only the real slug closes a task with `finish`.
    let hint = if open.iter().any(|t| t.mentions_finish()) {
        format!("; a task closes with `finish` only when its text names `spec-dna finish {slug}`")
    } else {
        String::new()
    };
    format!(
        "{} open task(s) in `Order` of `plan.md` ({}{more}); do each task, or close it with a reason in `Skip:`{hint}",
        open.len(),
        shown.join("; ")
    )
}

fn as_error(mut d: Diag) -> Diag {
    d.severity = Severity::Error;
    d
}

/// True if `slug` is one path segment, so the folder is a child of a folder of
/// the `intent` layer.
pub fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug != "."
        && slug != ".."
        && !slug.contains(['/', '\\', '\0'])
        && !slug.starts_with('.')
}

fn in_folder(folder: &str, path: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Runs the steps 0 to 5 in memory (see the module comment).
pub fn plan(cx: &Ctx) -> Result<Plan, Failure> {
    let head = cx.head;
    let changes = change::collect(head);

    // Step 0: no task of the plan is open. A closed change is abandoned, so
    // its tasks do not count.
    if let Some(ch) = changes.iter().find(|c| c.dir == cx.folder)
        && ch.intent_status() != "closed"
    {
        // A task that names `spec-dna finish <slug>` closes with `finish`.
        let open: Vec<change::PlanTask> = ch
            .tasks(head)
            .into_iter()
            .filter(|t| !t.closed() && !t.duplicate && !t.closes_with_finish(&ch.slug))
            .collect();
        if !open.is_empty() {
            return stop(0, open_tasks_reason(&open, &ch.slug), Vec::new());
        }
    }

    // Step 0: no document outside the folder links into it.
    let links = inbound(cx);
    if !links.is_empty() {
        return stop(
            0,
            format!(
                "{} reference(s) into the folder `{}` would break when `finish` deletes it; remove each one, or name the change as plain text",
                links.len(),
                cx.folder
            ),
            links,
        );
    }

    // Step 1: the statuses.
    let Some(ch) = changes.iter().find(|c| c.dir == cx.folder) else {
        return stop(
            1,
            format!(
                "the folder `{}` has no `intent.md`, `spec.md` or `plan.md`",
                cx.folder
            ),
            Vec::new(),
        );
    };
    let removed: HashSet<String> = cx.folder_files.iter().cloned().collect();
    let mut out = Plan {
        removes: sorted(cx.folder_files),
        folder: cx.folder.to_string(),
        hours: task_hours(&ch.tasks(head)),
        ..Plan::default()
    };
    if ch.intent_status() == "closed" {
        // A closed change does not proceed: only the folder goes away.
        let rebuilt = finish_indexes(cx, &removed, &mut out.writes)?;
        paths_problem(cx, &out, 1)?;
        no_new_errors(cx, &rebuilt, None, &HashSet::new())?;
        return Ok(out);
    }
    let why = status_problems(ch, head);
    if !why.is_empty() {
        return stop(1, why.join("; "), Vec::new());
    }

    // Step 2: concerns, open questions and decisions.
    let mut diags: Vec<Diag> = Vec::new();
    if let Some(a) = &ch.spec {
        lifecycle::concerns(&head.docs[a.doc as usize], &mut diags);
    }
    diags.extend(lifecycle::question_diags(head, ch));
    diags.extend(lifecycle::decisions_unaccepted(head, head, ch));
    if !diags.is_empty() {
        return stop(
            2,
            "the spec has an open concern or question, or a needed decision is absent or not `accepted`",
            diags,
        );
    }

    // Step 3: the delta and the interfaces against the trees.
    let Some(base) = cx.base else {
        return Err(Failure::NoBase);
    };
    let lines = ch.delta(head);
    let spec = ch.spec.as_ref().map(|a| &head.docs[a.doc as usize]);
    let mut diags = approved_delta(cx, ch, base, &lines);
    let layers = check::doc_layers(cx.config, head);
    let allowed = lifecycle::allowed_layers(cx.config);
    let mut todo: Vec<&Delta> = Vec::new();
    if let Some(spec) = spec {
        let items = change::section_items(spec, ItemSection::Delta);
        for (line, item) in lines.iter().zip(items) {
            let d = match &line.parsed {
                Ok(d) => d,
                Err(reason) => {
                    diags.push(check::diag(
                        spec,
                        line.line,
                        Code::DeltaInvalid,
                        lifecycle::short(&item.text),
                        format!("the delta line is not valid: {reason}"),
                    ));
                    continue;
                }
            };
            // A line that the tree applies already needs no edit.
            let applied = change::applied(line, base, head);
            if applied == Applied::Yes {
                continue;
            }
            let problem = lifecycle::delta_problem(cx.config, head, &layers, allowed.as_deref(), d)
                .or_else(|| target_in_folder_problem(cx, ch, d))
                .or_else(|| new_document_problem(cx, ch, d))
                .or_else(|| (cx.path_problem)(&target_path(cx, ch, d)))
                .or_else(|| {
                    (applied == Applied::Invalid).then(|| {
                        "the target is in neither the base tree nor the head tree".to_string()
                    })
                });
            match problem {
                Some(p) => diags.push(check::diag(
                    spec,
                    line.line,
                    Code::DeltaInvalid,
                    d.target(),
                    format!("`{}`: {p}", d.target()),
                )),
                None => todo.push(d),
            }
        }
        lifecycle::interfaces(cx.config, head, spec, &mut diags);
        // A new interface needs its document, or a delta line that adds it.
        let added: HashSet<String> = lines
            .iter()
            .filter_map(|l| l.parsed.as_ref().ok())
            .filter(|d| d.verb == Verb::Add)
            .map(|d| d.id.clone())
            .collect();
        diags.extend(lifecycle::interfaces_absent(head, spec, &added));
        let diff = cx.diff;
        let own: HashSet<String> = todo.iter().map(|d| target_path(cx, ch, d)).collect();
        let changed = |p: &str| diff.is_some_and(|d| d.kind(p).is_some()) || own.contains(p);
        diags.extend(lifecycle::interfaces_unchanged(head, spec, &changed));
    }
    if !diags.is_empty() {
        return stop(
            3,
            "the delta or the list of interfaces is not valid against the tree",
            diags,
        );
    }

    // Step 4: apply the delta.
    let mut created: HashSet<String> = HashSet::new();
    let edits = apply_delta(cx, ch, &todo)?;
    for w in &edits {
        if w.old.is_none() {
            created.insert(w.path.clone());
        }
    }
    out.writes = edits;
    out.applied = counts(&todo);
    let rebuilt = finish_indexes(cx, &removed, &mut out.writes)?;
    paths_problem(cx, &out, 4)?;
    dropped_keys(cx, ch, &todo, &removed, &rebuilt)?;
    // The result must give the result of each line, as SD915 will read it.
    let unapplied: Vec<String> = lines
        .iter()
        .filter_map(|l| l.parsed.as_ref().ok().map(|d| (l, d)))
        .filter(|(l, _)| change::applied(l, base, &rebuilt) != Applied::Yes)
        .map(|(_, d)| d.target())
        .collect();
    if !unapplied.is_empty() {
        return stop(
            4,
            format!(
                "the result would not give the delta line for {}; `check` would report SD915",
                unapplied
                    .iter()
                    .map(|t| format!("`{t}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Vec::new(),
        );
    }

    // Step 5.1: the test reports are present, readable and fresh.
    if let Some(reads) = cx.reports {
        let diags = reads.diags(cx.config);
        if !diags.is_empty() {
            return stop(
                5,
                "a test report is missing, stale or unreadable; run the tests, then `spec-dna evidence stamp`, then `finish` again",
                diags,
            );
        }
    }

    // Step 5.2: the acceptance record.
    let record = acceptance(cx, ch, &mut out)?;

    // Step 5.3: the deviations. They come from the plan, with the verdict of the
    // record. A change that needs no record has none (a section of an older plan
    // is plain text).
    if let Some(f) = cx
        .acceptance
        .filter(|f| matches!(f.need, crate::acceptance::Need::Yes { .. }))
    {
        let rec = record
            .as_deref()
            .and_then(|t| crate::record::Record::parse(t).ok());
        // The items are the declared ones: the rule of the pin is applied.
        out.deviations = f
            .sections
            .declared
            .deviations
            .iter()
            .map(|(id, kind, text)| Deviation {
                verdict: rec
                    .as_ref()
                    .and_then(|r| r.deviations.iter().find(|e| e.id == *id))
                    .map_or_else(|| "none".to_string(), |e| e.verdict.clone()),
                id: id.clone(),
                kind: kind.clone(),
                text: text.clone(),
            })
            .collect();
    }

    // Step 5: the evidence, then the check of the whole result.
    let files = ch.files(head);
    let others_touched = changes
        .iter()
        .filter(|c| c.dir != cx.folder)
        .any(|c| cx.diff.is_some_and(|d| d.under(&c.dir).next().is_some()));
    let scope = |p: &str| !others_touched || change::in_scope(&files, p);
    let reliable = crate::evidence::reliable_for(cx.config, ch.class);
    let mut missing = lifecycle::evidence_missing(&rebuilt, base, head, ch, &scope, reliable);
    // SD617 and SD620 are results of the test reports. A project that turned
    // one off keeps the tags as evidence.
    missing.retain(|d| {
        !matches!(d.code, Code::ChangeUnproven | Code::ProofNotReliable)
            || !cx.config.is_off(d.code)
    });
    if !missing.is_empty() {
        let unproven = missing.iter().all(|d| d.code == Code::ChangeUnproven);
        let unreliable = missing.iter().all(|d| d.code == Code::ProofNotReliable);
        return stop(
            5,
            if unproven {
                "a requirement that the change adds or changes has a tag, and no bound test case passed in the reports (the lines are lines of the result of `finish`)"
            } else if unreliable {
                "a requirement that the change adds or changes passed fewer times than `[evidence] reliable-runs` in the reports (the lines are lines of the result of `finish`)"
            } else if missing
                .iter()
                .all(|d| matches!(d.code, Code::ChangeUnproven | Code::ProofNotReliable))
            {
                "a requirement that the change adds or changes has a tag that no bound test case proves (SD617), or its bound case passed fewer times than `[evidence] reliable-runs` in the reports (SD620); the lines are lines of the result of `finish`"
            } else {
                "a requirement that the change adds or changes has no evidence (the lines are lines of the result of `finish`)"
            },
            missing,
        );
    }
    // Step 5.5: the record moves to the archive folder.
    archive(cx, record, &mut out)?;
    no_new_errors(cx, &rebuilt, Some(base), &created)?;
    Ok(out)
}

/// A diagnostic of the record, on the record file.
fn record_diag(path: &str, slug: &str, code: Code, message: String) -> Diag {
    Diag {
        path: path.to_string(),
        line: 1,
        code,
        doc: path.to_string(),
        target: slug.to_string(),
        message,
        severity: Severity::Error,
    }
}

/// Step 5.2 (design 0.8.0, D3 item 9): for a change that needs a record, the
/// record of the head is present, valid, fresh, accepted, and has the acceptors
/// that the change needs, and its sections are valid and answered. The step stops
/// at the first code of SD927 to SD933 and SD936 to SD940 that is not off, and it
/// reports each finding of that code. It returns the text of
/// the record, which step 5.5 moves, or `None` if the folder has no record to move
/// (a record that no rule needs is returned too, with no check). For a
/// plan of a version before 0.8.0 it adds the note of the design to the plan.
fn acceptance(cx: &Ctx, ch: &Change, out: &mut Plan) -> Result<Option<String>, Failure> {
    use crate::acceptance::{self as acc, Need};
    let Some(f) = cx.acceptance else {
        return Ok(None);
    };
    let shallow = match &f.need {
        Need::No(note) => {
            out.notes.extend(note.clone());
            // No rule needs the record, but a record in the folder is archived.
            let path = format!("{}/{}", cx.folder, crate::record::FILE_NAME);
            // A link or a file over the limit is no record: SD928 stops, and
            // nothing is read through the link.
            if let acc::RecordFile::Unreadable(m) = &f.record {
                if !cx.config.is_off(Code::AcceptanceInvalid) {
                    let diags = vec![record_diag(
                        &path,
                        &ch.slug,
                        Code::AcceptanceInvalid,
                        m.clone(),
                    )];
                    return stop(5, "the acceptance record is not valid", diags);
                }
                return Ok(None);
            }
            if let acc::RecordFile::Text(text) = &f.record {
                let text = text.clone();
                // The archive is never edited: a record that is not valid (a
                // skeleton with `pending` values) must not go into it. The
                // structural rules of SD928 run here, as with a need.
                if !cx.config.is_off(Code::AcceptanceInvalid) {
                    let facts = acc::plan_facts(ch, cx.head);
                    let (_, problems) = acc::read_record(cx.config, &text, &ch.slug, facts);
                    if !problems.is_empty() {
                        let diags = problems
                            .into_iter()
                            .map(|m| record_diag(&path, &ch.slug, Code::AcceptanceInvalid, m))
                            .collect();
                        return stop(5, "the acceptance record is not valid", diags);
                    }
                }
                return Ok(Some(text));
            }
            return Ok(None);
        }
        Need::Yes { shallow } => *shallow,
    };
    let cfg = cx.config;
    let on = |c: Code| !cfg.is_off(c);
    let path = format!("{}/{}", cx.folder, crate::record::FILE_NAME);
    let slug = ch.slug.as_str();
    let stop_with = |code: Code, why: &str, messages: Vec<String>| {
        let diags = messages
            .into_iter()
            .map(|m| record_diag(&path, slug, code, m))
            .collect();
        stop(5, why, diags)
    };
    let text = if let acc::RecordFile::Text(text) = &f.record {
        text.clone()
    } else if let acc::RecordFile::Unreadable(m) = &f.record {
        // A link or a file over the limit: the record does not read.
        if on(Code::AcceptanceInvalid) {
            return stop_with(
                Code::AcceptanceInvalid,
                "the acceptance record is not valid",
                vec![m.clone()],
            );
        }
        return Ok(None);
    } else {
        if on(Code::AcceptanceMissing) {
            return stop_with(
                Code::AcceptanceMissing,
                "the change needs an acceptance record, and the folder has none",
                vec![acc::missing_message(
                    slug,
                    &format!("`{path}` is absent"),
                    shallow,
                )],
            );
        }
        return Ok(None);
    };
    let facts = acc::plan_facts(ch, cx.head);
    let (rec, mut problems) = acc::read_record(cfg, &text, slug, facts);
    problems.extend(f.brief_problems.iter().cloned());
    if let Some((code, messages)) = acc::first_stop(
        cfg,
        ch.class,
        rec.as_ref(),
        problems,
        f.stale.clone(),
        &f.sections,
    ) {
        let why = match code {
            Code::AcceptanceInvalid => "the acceptance record is not valid".to_string(),
            // The text follows the reason: the scope of the plan, or the code.
            Code::AcceptanceStale => format!(
                "the acceptance record is stale: {}",
                acc::stale_cause(messages.first().map_or("", String::as_str))
            ),
            Code::AcceptanceRejected => "the acceptance record says `rejected`".to_string(),
            Code::SelfAccepted => {
                "2 acceptor entries have the same session and the same agent".to_string()
            }
            Code::AcceptorMissing => "the acceptance record has too few acceptors".to_string(),
            Code::InvariantInvalid => "an item of `Invariants` is not valid".to_string(),
            Code::InvariantStale => "the hash of an item of `Invariants` is stale".to_string(),
            Code::RecordIncomplete => {
                "the acceptance record does not answer each invariant and each deviation"
                    .to_string()
            }
            Code::DeviationInvalid => "an item of `Deviations` is not valid".to_string(),
            _ => "a `spec` deviation names no line of the delta and no change folder".to_string(),
        };
        return stop_with(code, &why, messages);
    }
    Ok(Some(text))
}

/// Step 5.5 (design 0.8.0, D3 item 3): the record moves to the archive file. The
/// plan adds the file to its writes, and the record goes away with the folder.
/// A path that git ignores, or that has a link, stops the step before any write.
fn archive(cx: &Ctx, record: Option<String>, out: &mut Plan) -> Result<(), Failure> {
    let (Some(text), Some(f)) = (record, cx.acceptance) else {
        return Ok(());
    };
    if let Some(rule) = &f.ignored {
        return stop(
            5,
            format!(
                "git ignores the archive file `{}` by the rule `{rule}`; remove the rule, so the record stays in the repository, then run `finish` again",
                f.archive
            ),
            Vec::new(),
        );
    }
    if let Some(why) = (cx.path_problem)(&f.archive) {
        return stop(5, why, Vec::new());
    }
    out.writes.push(Write {
        path: f.archive.clone(),
        old: None,
        // The archive has LF: a working tree with `core.autocrlf` has CRLF, and
        // git has LF (ADR 0019).
        new: String::from_utf8_lossy(&crate::record::to_lf(text.as_bytes())).into_owned(),
    });
    out.writes.sort_by(|a, b| a.path.cmp(&b.path));
    out.archive = Some(f.archive.clone());
    Ok(())
}

/// The problem of a path that `finish` writes or deletes, if there is one: a
/// symbolic link in the path (also as the last part), or a path that leaves the
/// repository. It reads only the metadata of the path, and it never follows a
/// link. `rel` is a path from the repository root.
pub fn path_problem(root: &std::path::Path, rel: &str) -> Option<String> {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return Some(format!(
            "`{rel}` is not a path below the root of the repository"
        ));
    }
    let mut cur = root.to_path_buf();
    let mut shown = String::new();
    for part in &parts {
        cur.push(part);
        if !shown.is_empty() {
            shown.push('/');
        }
        shown.push_str(part);
        match std::fs::symlink_metadata(&cur) {
            Ok(m) if m.file_type().is_symlink() => {
                return Some(format!(
                    "`{rel}` is, or goes through, the symbolic link `{shown}`; `finish` does not write or delete through a link. Replace the link, or move the requirement to a path with no link"
                ));
            }
            Ok(_) => {}
            // The rest of the path does not exist, so it has no link.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Some(format!("cannot read `{shown}`: {e}")),
        }
    }
    // The deepest folder of the path that exists must be in the repository.
    let canon = root.canonicalize().ok()?;
    let mut dir = root.join(rel);
    dir.pop();
    let real = loop {
        match dir.canonicalize() {
            Ok(p) => break p,
            Err(_) if dir.pop() => {}
            Err(_) => return Some(format!("cannot resolve the folder of `{rel}`")),
        }
    };
    (!real.starts_with(&canon)).then(|| format!("`{rel}` is outside the repository"))
}

/// Stops if a path of the plan (a file to write or to delete) has a symbolic
/// link or leaves the repository. The plan has written nothing yet.
fn paths_problem(cx: &Ctx, plan: &Plan, step: u8) -> Result<(), Failure> {
    let why: Vec<String> = plan
        .writes
        .iter()
        .map(|w| w.path.as_str())
        .chain(plan.removes.iter().map(String::as_str))
        .filter_map(|p| (cx.path_problem)(p))
        .collect();
    if why.is_empty() {
        return Ok(());
    }
    stop(step, why.join("; "), Vec::new())
}

/// After the writes, the only requirements that the head has and the result has
/// not are those of the `REMOVE` lines. Another loss is a defect of the edit,
/// so `finish` stops and writes nothing. The documents of the folder are not
/// part of the check.
fn dropped_keys(
    cx: &Ctx,
    ch: &Change,
    todo: &[&Delta],
    removed: &HashSet<String>,
    rebuilt: &Corpus,
) -> Result<(), Failure> {
    let keys = |c: &Corpus, skip: &HashSet<String>| -> HashSet<(String, String)> {
        c.docs
            .iter()
            .filter(|d| !skip.contains(&d.path))
            .flat_map(|d| {
                d.reqs
                    .iter()
                    .map(|&i| (d.path.clone(), d.parsed.reqs[i].key.clone()))
            })
            .collect()
    };
    let none = HashSet::new();
    let after = keys(rebuilt, &none);
    let named: HashSet<(String, String)> = todo
        .iter()
        .filter(|d| d.verb == Verb::Remove)
        .map(|d| (target_path(cx, ch, d), d.key.clone()))
        .collect();
    let mut lost: Vec<String> = keys(cx.head, removed)
        .into_iter()
        .filter(|k| !after.contains(k) && !named.contains(k))
        .map(|(p, k)| format!("`{p}#{k}`"))
        .collect();
    if lost.is_empty() {
        return Ok(());
    }
    lost.sort();
    stop(
        4,
        format!(
            "the result would drop the requirement {} that no `REMOVE` line names",
            lost.join(", ")
        ),
        Vec::new(),
    )
}

fn sorted(paths: &[String]) -> Vec<String> {
    let mut v = paths.to_vec();
    v.sort();
    v
}

fn counts(todo: &[&Delta]) -> (usize, usize, usize) {
    let n = |v: Verb| todo.iter().filter(|d| d.verb == v).count();
    (n(Verb::Add), n(Verb::Change), n(Verb::Remove))
}

/// The statuses that step 1 needs: the intent is `accepted`, the plan is
/// `approved`, and the spec is `approved` for a `higher-risk` change or for a
/// spec with delta lines. Returns the reason of each failure.
fn status_problems(ch: &Change, head: &Corpus) -> Vec<String> {
    let mut why = Vec::new();
    match ch.intent_status() {
        "accepted" => {}
        "" => why.push("the change has no `intent.md`".to_string()),
        s => why.push(format!(
            "the intent is `{s}`, and `finish` needs `accepted` (or `closed`, to drop the change)"
        )),
    }
    match ch.plan.as_ref().map(|a| a.status.as_str()) {
        Some("approved") => {}
        None => why.push("the change has no `plan.md`".to_string()),
        Some(s) => why.push(format!(
            "the plan is `{}`, and `finish` needs `approved`",
            if s.is_empty() { "without a status" } else { s }
        )),
    }
    let has_lines = ch.spec.as_ref().is_some_and(|a| {
        change::section_items(&head.docs[a.doc as usize], ItemSection::Delta)
            .next()
            .is_some()
    });
    match ch.spec.as_ref().map(|a| a.status.as_str()) {
        Some("approved") => {}
        None if ch.class == Class::HigherRisk => {
            why.push("the change is `higher-risk` and has no `spec.md`".to_string())
        }
        None => {}
        Some(s) if ch.class == Class::HigherRisk || has_lines => why.push(format!(
            "the spec is `{}`, and `finish` needs `approved`",
            if s.is_empty() { "without a status" } else { s }
        )),
        Some(_) => {}
    }
    why
}

/// A delta that the base tree did not approve: the base tree has the change,
/// and its delta differs from the delta of the head. An approved delta must not
/// change in the pull request of `finish` (design, F3). A change that the base
/// tree does not have was added in this branch: nothing was approved, so there
/// is nothing to compare.
fn approved_delta(cx: &Ctx, ch: &Change, base: &Corpus, lines: &[DeltaLine]) -> Vec<Diag> {
    let Some(bch) = change::collect(base).into_iter().find(|c| c.dir == ch.dir) else {
        return Vec::new();
    };
    let texts = |c: &Corpus, ch: &Change| -> Vec<String> {
        ch.spec
            .as_ref()
            .map(|a| {
                change::section_items(&c.docs[a.doc as usize], ItemSection::Delta)
                    .map(|i| i.text.split_whitespace().collect::<Vec<_>>().join(" "))
                    .collect()
            })
            .unwrap_or_default()
    };
    if texts(base, &bch) == texts(cx.head, ch) {
        return Vec::new();
    }
    let anchor = ch
        .spec
        .as_ref()
        .or(ch.intent.as_ref())
        .or(ch.plan.as_ref())
        .map(|a| &cx.head.docs[a.doc as usize]);
    let Some(anchor) = anchor else {
        return Vec::new();
    };
    vec![check::diag(
        anchor,
        lines.first().map_or(1, |l| l.line),
        Code::DeltaInvalid,
        "delta",
        "the delta of the spec differs from the delta that the base tree has (the approved delta); an approved delta must not change. Correct the delta in a pull request of its own, merge it, and run `finish` again",
    )]
}

/// The path of the document that a delta line writes: the document of the
/// tree, or the new path under the root of the folder.
fn target_path(cx: &Ctx, ch: &Change, d: &Delta) -> String {
    match cx.head.resolve(&d.id) {
        Res::Found(i, None, _) => cx.head.docs[i as usize].path.clone(),
        _ => new_path(cx, ch, &d.id).1,
    }
}

/// The root index and the path of a new document of an id. The root is the
/// root of the folder of the change.
fn new_path(cx: &Ctx, ch: &Change, id: &str) -> (usize, String) {
    let anchor = ch
        .intent
        .as_ref()
        .or(ch.spec.as_ref())
        .or(ch.plan.as_ref())
        .map(|a| cx.head.docs[a.doc as usize].root)
        .unwrap_or(0);
    let root = &cx.config.roots[anchor];
    let path = if root.path.is_empty() {
        format!("{id}.md")
    } else {
        format!("{}/{id}.md", root.path)
    };
    (anchor, path)
}

/// The type of a new document at a place, and whether its front matter needs
/// a `type:` line. A root of one type gives the type. In a mixed root, the
/// place or `default-type` gives it, and else `type: requirements` is needed.
fn new_type(cx: &Ctx, root: usize, path: &str) -> (DocType, bool) {
    let root = &cx.config.roots[root];
    if let Some(t) = root.single_type {
        return (t, false);
    }
    let placed = cx
        .config
        .layers
        .place_type(root.rel(path))
        .or(root.default_type);
    match placed {
        Some(t) => (t, false),
        None => (DocType::Requirements, true),
    }
}

/// The problem of a line whose target is a file of the folder that `finish`
/// deletes: the deletion of the folder removes the edit.
fn target_in_folder_problem(cx: &Ctx, ch: &Change, d: &Delta) -> Option<String> {
    let path = target_path(cx, ch, d);
    in_folder(cx.folder, &path).then(|| {
        format!(
            "the target is `{path}`, a file of the folder `{}` that `finish` deletes",
            cx.folder
        )
    })
}

/// The problem of an `ADD` line for a document that the tree does not have.
fn new_document_problem(cx: &Ctx, ch: &Change, d: &Delta) -> Option<String> {
    if d.verb != Verb::Add || !matches!(cx.head.resolve(&d.id), Res::Missing) {
        return None;
    }
    let safe = !d.id.starts_with('/')
        && !d.id.contains('\\')
        && d.id
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..");
    if !safe {
        return Some(format!(
            "`{}` is not a document id that `finish` can write as a file",
            d.id
        ));
    }
    let (root, path) = new_path(cx, ch, &d.id);
    if cx.config.is_excluded(root, &path) || cx.config.root_of(&path) != Some(root) {
        return Some(format!(
            "the new document `{path}` is not in a document root"
        ));
    }
    if cx.head.files.contains(&path) || (cx.exists)(&path) {
        return Some(format!(
            "the file `{path}` exists, and it is not the document `{}`",
            d.id
        ));
    }
    let (ty, _) = new_type(cx, root, &path);
    if !ty.has_requirements() {
        return Some(format!(
            "the new document `{path}` would have the type `{}`, and a delta line needs `requirements`, `interface` or `rule`",
            ty.name()
        ));
    }
    None
}

/// A reference from a document outside the folder of a change to a document or a
/// file in that folder (`refs_into`).
pub(crate) struct RefInto {
    /// The document that has the reference.
    pub doc: u32,
    pub line: u32,
    /// `UnresolvedReference` for an id and a field, `UnresolvedLink` for a
    /// Markdown link or image.
    pub code: Code,
    /// The target as the text writes it.
    pub target: String,
    /// The place of the folder in the list that `refs_into` got.
    pub folder: usize,
    /// The document that the reference names, if it names one.
    pub to: Option<u32>,
}

/// Each reference from a document outside a folder to a document or a file in
/// it: a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` and
/// `from`. A stub and a generated index are not sources. The locked text of an
/// accepted decision is not a source: its text cannot change (SD405), so SD101
/// and SD102 do not report it either. The front matter is not locked. Step 0
/// of `finish` gives one folder, and SD612 (`check`) gives all the folders of
/// the changes. One pass over the references resolves each target once.
pub(crate) fn refs_into(config: &Config, head: &Corpus, folders: &[&str]) -> Vec<RefInto> {
    let mut out = Vec::new();
    // The first folder that holds the path and does not hold the source.
    let folder_of = |source: &str, path: &str| {
        folders
            .iter()
            .position(|f| in_folder(f, path) && !in_folder(f, source))
    };
    for (di, d) in head.docs.iter().enumerate() {
        if d.moved_to.is_some() || d.generated.is_some() {
            continue;
        }
        let editable = check::editable_chunks(config, d);
        let locked = |line: u32| {
            editable
                .as_ref()
                .is_some_and(|e| check::in_locked_text(d, e, line))
        };
        let mut hit = |line: u32, code: Code, target: &str, folder: usize, to: Option<u32>| {
            out.push(RefInto {
                doc: di as u32,
                line,
                code,
                target: target.to_string(),
                folder,
                to,
            });
        };
        // The folder and the document of an id.
        let by_id = |r: Res| match r {
            Res::Found(t, _, _) => {
                folder_of(&d.path, &head.docs[t as usize].path).map(|f| (f, Some(t)))
            }
            Res::Ambiguous(v) => v.iter().find_map(|&t| {
                folder_of(&d.path, &head.docs[t as usize].path).map(|f| (f, Some(t)))
            }),
            _ => None,
        };
        for r in &d.parsed.refs {
            if locked(r.line) {
                continue;
            }
            match r.kind {
                RefKind::Wiki => {
                    if let Some((f, to)) = by_id(head.resolve(&r.target)) {
                        hit(r.line, Code::UnresolvedReference, &r.target, f, to);
                    }
                }
                RefKind::Link | RefKind::Image => {
                    if let Some((f, to)) = link_into(head, &d.path, &r.target, &folder_of) {
                        hit(r.line, Code::UnresolvedLink, &r.target, f, to);
                    }
                }
            }
        }
        let fields = d
            .supersedes
            .iter()
            .chain(&d.depends_on)
            .chain(d.from.iter());
        for (t, line) in fields {
            if let Some((f, to)) = by_id(head.resolve(t)) {
                hit(*line, Code::UnresolvedReference, t, f, to);
            }
        }
    }
    out
}

/// Step 0. Each reference into the folder, as an error of `finish`.
fn inbound(cx: &Ctx) -> Vec<Diag> {
    let folder = cx.folder;
    refs_into(cx.config, cx.head, &[folder])
        .into_iter()
        .map(|r| {
            as_error(check::diag(
                &cx.head.docs[r.doc as usize],
                r.line,
                r.code,
                &r.target,
                format!(
                    "`{}` names the folder `{folder}` of the change, and `finish` deletes the folder",
                    r.target
                ),
            ))
        })
        .collect()
}

/// The folder that a Markdown link names, and the document that it resolves
/// to. A link to the folder itself counts. `folder_of` gives the folder of a
/// path, for the document `from`.
fn link_into(
    c: &Corpus,
    from: &str,
    target: &str,
    folder_of: &dyn Fn(&str, &str) -> Option<usize>,
) -> Option<(usize, Option<u32>)> {
    let to = match c.resolve_link(from, target) {
        LinkRes::External | LinkRes::SelfAnchor(_) | LinkRes::Missing(_) => return None,
        LinkRes::File(_, to, _) | LinkRes::Dir(to) => to,
    };
    let path = target.split(['#', '?']).next().unwrap_or("").trim();
    if path.is_empty() {
        return None;
    }
    let base = if path.starts_with('/') {
        ""
    } else {
        parent(from)
    };
    let joined = join(base, path)?;
    folder_of(from, &joined).map(|f| (f, to))
}

/// One edit of a text: the byte range to replace, and the new text. An insert
/// has an empty range.
#[derive(Clone)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

/// Step 4: the writes of the delta lines that the tree does not apply yet. The
/// lines of one document give one write. A new document gets its own file.
fn apply_delta(cx: &Ctx, ch: &Change, todo: &[&Delta]) -> Result<Vec<Write>, Failure> {
    struct Group<'a> {
        path: String,
        doc: Option<u32>,
        lines: Vec<&'a Delta>,
    }
    let mut groups: Vec<Group> = Vec::new();
    for &d in todo {
        let (path, doc) = match cx.head.resolve(&d.id) {
            Res::Found(i, None, _) => (cx.head.docs[i as usize].path.clone(), Some(i)),
            _ => (new_path(cx, ch, &d.id).1, None),
        };
        match groups.iter_mut().find(|g| g.path == path) {
            Some(g) => g.lines.push(d),
            None => groups.push(Group {
                path,
                doc,
                lines: vec![d],
            }),
        }
    }
    let spec_path = ch
        .spec
        .as_ref()
        .map(|a| cx.head.docs[a.doc as usize].path.clone())
        .unwrap_or_default();
    let mut out = Vec::new();
    for g in groups {
        let sentence = |d: &Delta| rebase(d, &spec_path, &g.path, cx.head);
        let write = match g.doc {
            Some(i) => {
                let doc = &cx.head.docs[i as usize];
                let text = (cx.read)(&doc.path).map_err(Failure::Io)?;
                let new = edit_document(&text, doc, &g.lines, &sentence).map_err(|why| {
                    Failure::Stop(Stop {
                        step: 4,
                        reason: format!("`{}`: {why}", doc.path),
                        diags: Vec::new(),
                    })
                })?;
                Write {
                    path: doc.path.clone(),
                    old: Some(text),
                    new,
                }
            }
            None => {
                let (root, path) = new_path(cx, ch, &g.lines[0].id);
                let (ty, type_line) = new_type(cx, root, &path);
                let id = &g.lines[0].id;
                Write {
                    path,
                    old: None,
                    new: new_document(id, ty, type_line, &g.lines, &sentence),
                }
            }
        };
        out.push(write);
    }
    Ok(out)
}

/// The sentence of a delta line for the document at `to`: the relative links
/// of the spec at `from` point to the same targets (as `mv` moves a text).
fn rebase(d: &Delta, from: &str, to: &str, c: &Corpus) -> String {
    let sentence = d.sentence.as_deref().unwrap_or("");
    let none = HashSet::new();
    split::map_links(sentence, &|dest| {
        split::retarget(dest, from, to, &none, to, c)
    })
}

/// The text of a document with its lines applied.
fn edit_document(
    text: &str,
    doc: &Doc,
    lines: &[&Delta],
    sentence: &dyn Fn(&Delta) -> String,
) -> Result<String, String> {
    let fresh = parse(text);
    let valid: HashSet<&str> = doc
        .reqs
        .iter()
        .map(|&i| doc.parsed.reqs[i].key.as_str())
        .collect();
    let req_of = |key: &str| {
        fresh
            .reqs
            .iter()
            .find(|r| r.key == key && valid.contains(key))
    };
    let mut edits: Vec<Edit> = Vec::new();
    // The removals, each with its item span before the blank line.
    let mut removals: Vec<((usize, usize), Edit)> = Vec::new();
    let mut adds: Vec<String> = Vec::new();
    for d in lines {
        match d.verb {
            Verb::Add => adds.push(format!("**{}**: {}", d.key, sentence(d))),
            Verb::Change | Verb::Remove => {
                let Some(r) = req_of(&d.key) else {
                    return Err(format!("the requirement `{}` is not in the file", d.key));
                };
                if !change::hash_matches(r, d.hash.as_deref().unwrap_or("")) {
                    return Err(format!(
                        "the text of `{}` is not the text that the hash names",
                        d.key
                    ));
                }
                if d.verb == Verb::Change {
                    let (s, e) = (r.sentence.start as usize, r.sentence.end as usize);
                    let mut new = sentence(d);
                    // A line with a marker `proof-owed` replaces the debt: the
                    // old marker goes, so the requirement has one.
                    let new_debt = crate::parse::strip_markers_all(&new).3.is_some();
                    // A marker inside the old span (a `mv` puts the alias on
                    // the first line of a wrapped requirement) stays.
                    for m in inner_markers(&text[s..e]) {
                        let debt = m
                            .trim_start_matches("<!--")
                            .trim()
                            .starts_with("proof-owed:");
                        if !new.contains(m) && !(debt && new_debt) {
                            new.push(' ');
                            new.push_str(m);
                        }
                    }
                    if new_debt {
                        for (from, to) in trailing_markers(text, e, "proof-owed:") {
                            edits.push(Edit {
                                start: from,
                                end: to,
                                text: String::new(),
                            });
                        }
                    }
                    if s == e && !text[..s].ends_with(char::is_whitespace) {
                        new.insert(0, ' ');
                    }
                    edits.push(Edit {
                        start: s,
                        end: e,
                        text: new,
                    });
                } else {
                    let (a, mut b) = split::item_span(text, r.start as usize, r.end as usize);
                    let core = (a, b);
                    // The removal drops each requirement in the span. The
                    // delta must name each of them.
                    let mut open: Vec<(u32, &str)> = fresh
                        .reqs
                        .iter()
                        .filter(|o| (a..b).contains(&(o.start as usize)) && o.start != r.start)
                        .filter(|o| valid.contains(o.key.as_str()))
                        .filter(|o| {
                            !lines
                                .iter()
                                .any(|l| l.verb == Verb::Remove && l.key == o.key)
                        })
                        .map(|o| (o.start, o.key.as_str()))
                        .collect();
                    if !open.is_empty() {
                        open.sort();
                        let keys: Vec<String> =
                            open.iter().map(|(_, k)| format!("`{k}`")).collect();
                        return Err(format!(
                            "the requirement `{}` has the nested requirement {}, and `REMOVE` drops them too; add a `REMOVE` line for each of them to the delta",
                            d.key,
                            keys.join(", ")
                        ));
                    }
                    let before_blank = text[..a].ends_with("\n\n") || text[..a].ends_with("\n\r\n");
                    let after = &text[b..];
                    if before_blank && (after.starts_with('\n') || after.starts_with("\r\n")) {
                        b += if after.starts_with("\r\n") { 2 } else { 1 };
                    }
                    removals.push((
                        core,
                        Edit {
                            start: a,
                            end: b,
                            text: String::new(),
                        },
                    ));
                }
            }
        }
    }
    // A removal inside another removal is part of it: the span of the outer
    // item holds it already.
    let cores: Vec<(usize, usize)> = removals.iter().map(|r| r.0).collect();
    // Each removal that stays: its range, and the end of its item.
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    for (core, e) in removals {
        let inner = cores
            .iter()
            .any(|&c| c != core && c.0 <= core.0 && core.1 <= c.1);
        if !inner {
            spans.push((e.start, e.end, core.1));
            edits.push(e);
        }
    }
    let mut result = apply_edits(text, edits.clone())?;
    if !adds.is_empty() {
        let after = parse(&result);
        let survives = after.reqs.iter().any(|r| valid.contains(r.key.as_str()));
        if survives || spans.is_empty() {
            // The new items go by the list that the edits leave: the place
            // and the number do not depend on a removed item.
            let (pos, ins) = add_items(&result, &after, &valid, doc.ty, &adds, false);
            result.insert_str(pos, &ins);
        } else {
            // The edits remove each requirement item. The new items take the
            // place and the marker of the last of them.
            let (mut pos, mut ins) = add_items(text, &fresh, &valid, doc.ty, &adds, true);
            if let Some(&(_, end, item_end)) = spans.iter().find(|&&(s, e, _)| s < pos && pos <= e)
            {
                // The removal took one blank line after the item: it goes back
                // behind the new items.
                ins.push_str(&text[item_end..end]);
                pos = end;
            }
            edits.push(Edit {
                start: pos,
                end: pos,
                text: ins,
            });
            result = apply_edits(text, edits)?;
        }
    }
    Ok(result)
}

/// The markers that follow `from` at the end of a sentence, with only space
/// between them, whose text starts with `kind` (for example `proof-owed:`). Each
/// item is the byte range of the marker with the space before it.
fn trailing_markers(text: &str, from: usize, kind: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = from;
    loop {
        let rest = &text[at..];
        let ws = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let tail = &rest[ws..];
        if !tail.starts_with("<!--") {
            return out;
        }
        let Some(len) = tail[4..].find("-->") else {
            return out;
        };
        let end = at + ws + 4 + len + 3;
        let inner = text[at + ws + 4..end - 3].trim();
        if !crate::parse::MARKERS.iter().any(|m| inner.starts_with(m)) {
            return out;
        }
        if inner.starts_with(kind) {
            out.push((at, end));
        }
        at = end;
    }
}

/// The markers `<!-- aliases: ... -->`, `<!-- verified-by: ... -->` and
/// `<!-- proof-owed: ... -->` in a text, as they are written, in the order of the text.
fn inner_markers(span: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = span[at..].find("<!--") {
        let start = at + i;
        let Some(len) = span[start + 4..].find("-->") else {
            break;
        };
        let end = start + 4 + len + 3;
        let inner = span[start + 4..end - 3].trim();
        if crate::parse::MARKERS.iter().any(|m| inner.starts_with(m)) {
            out.push(&span[start..end]);
        }
        at = end;
    }
    out
}

/// Applies edits that do not overlap, the last range first.
fn apply_edits(text: &str, mut edits: Vec<Edit>) -> Result<String, String> {
    edits.sort_by_key(|e| std::cmp::Reverse((e.start, e.end)));
    let mut limit = text.len();
    for e in &edits {
        if e.end > limit || e.start > e.end {
            return Err("two delta lines edit the same lines of the file".to_string());
        }
        limit = e.start;
    }
    let mut out = text.to_string();
    for e in edits {
        out.replace_range(e.start..e.end, &e.text);
    }
    Ok(out)
}

/// The line ending of the text around `pos`.
fn eol_at(text: &str, pos: usize) -> &'static str {
    let at = text[pos.min(text.len())..]
        .find('\n')
        .map(|i| pos + i)
        .or_else(|| text[..pos.min(text.len())].rfind('\n'));
    match at {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

/// The insert of the new items of a document: the position and the text. The
/// items (`**key**: sentence`) go after the outermost requirement item that
/// holds the last requirement item, as its siblings. They have its indent and
/// list marker, and the next number in an ordered list (with `restart`, the
/// number of the first item). If the document has no requirement item, they go
/// to the end of the section `Requirements` (`Rules` for a rule). The command
/// adds that section at the end of the file if it is not there.
fn add_items(
    text: &str,
    fresh: &ParsedDoc,
    valid: &HashSet<&str>,
    ty: DocType,
    items: &[String],
    restart: bool,
) -> (usize, String) {
    let last = fresh
        .reqs
        .iter()
        .filter(|r| valid.contains(r.key.as_str()))
        .max_by_key(|r| r.start);
    if let Some(last) = last {
        // The last requirement can be nested. The new items go after the
        // outermost requirement item that holds it, as its siblings.
        let last_end = split::item_span(text, last.start as usize, last.end as usize).1;
        let last = fresh
            .reqs
            .iter()
            .filter(|r| valid.contains(r.key.as_str()) && r.start <= last.start)
            .filter(|r| split::item_span(text, r.start as usize, r.end as usize).1 >= last_end)
            .min_by_key(|r| r.start)
            .unwrap_or(last);
        let (a, b) = split::item_span(text, last.start as usize, last.end as usize);
        let eol = eol_at(text, a);
        let start = last.start as usize;
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let prefix = &text[line_start..start];
        let item = &text[start..];
        let (bullet, mut number) = marker_of(item);
        if restart {
            // The list has no item left: it starts again at its first number.
            let first = fresh
                .reqs
                .iter()
                .filter(|r| valid.contains(r.key.as_str()))
                .min_by_key(|r| r.start);
            if let (Some(f), Some((_, delim))) = (first, number)
                && let (_, Some((n, _))) = marker_of(&text[f.start as usize..])
                && n > 0
            {
                number = Some((n - 1, delim));
            }
        }
        let (mlen, glen) = (marker_len(item), gap_len(item));
        let gap = if glen == 0 {
            " "
        } else {
            &item[mlen..mlen + glen]
        };
        // A blank line between the last item and the item before it: the list
        // is loose, so each new item gets a blank line before it.
        let before = fresh
            .reqs
            .iter()
            .filter(|r| valid.contains(r.key.as_str()) && r.start < last.start)
            .max_by_key(|r| r.start);
        let loose = before.is_some_and(|p| {
            let (_, pb) = split::item_span(text, p.start as usize, p.end as usize);
            pb < a && text[pb..a].trim().is_empty()
        });
        let mut lines: Vec<String> = Vec::new();
        for (k, new) in items.iter().enumerate() {
            if loose {
                lines.push(String::new());
            }
            let m = match &number {
                Some((n, delim)) => format!("{}{delim}", n + 1 + k as u64),
                None => bullet.clone(),
            };
            lines.push(format!("{prefix}{m}{gap}{new}"));
        }
        let ins = if text[..b].ends_with('\n') {
            lines.iter().map(|l| format!("{l}{eol}")).collect()
        } else {
            format!("{eol}{}", lines.join(eol))
        };
        return (b, ins);
    }
    let eol = eol_at(text, 0);
    let section = if ty == DocType::Rule {
        "Rules"
    } else {
        "Requirements"
    };
    let list: String = items
        .iter()
        .map(|i| format!("- {i}"))
        .collect::<Vec<_>>()
        .join(eol);
    let found = fresh
        .chunks
        .iter()
        .find(|c| c.level >= 2 && c.heading.trim().eq_ignore_ascii_case(section));
    match found {
        Some(c) => {
            let pos = text[..c.end as usize].trim_end().len();
            (pos, format!("{eol}{eol}{list}"))
        }
        None => {
            let pos = text.trim_end().len();
            (pos, format!("{eol}{eol}## {section}{eol}{eol}{list}"))
        }
    }
}

/// The list marker at the start of a list item: the marker with its text, and
/// for an ordered list the number and the delimiter. A bullet gives `None`.
fn marker_of(item: &str) -> (String, Option<(u64, char)>) {
    let digits = item.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0
        && let Some(delim) = item[digits..]
            .chars()
            .next()
            .filter(|c| matches!(c, '.' | ')'))
        && let Ok(n) = item[..digits].parse::<u64>()
    {
        return (item[..=digits].to_string(), Some((n, delim)));
    }
    match item.chars().next() {
        Some(c @ ('-' | '*' | '+')) => (c.to_string(), None),
        _ => ("-".to_string(), None),
    }
}

/// The byte length of the marker of `marker_of`.
fn marker_len(item: &str) -> usize {
    let digits = item.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && item[digits..].starts_with(['.', ')']) {
        digits + 1
    } else {
        1
    }
}

/// The length of the white space after the marker (one space if there is none).
fn gap_len(item: &str) -> usize {
    item[marker_len(item)..]
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count()
}

/// The text of a new document: the front matter with the status `active` (and
/// the type, if the place does not give it), the H1 from the id, one section
/// with the items. It has LF line ends and no placeholder.
fn new_document(
    id: &str,
    ty: DocType,
    type_line: bool,
    lines: &[&Delta],
    sentence: &dyn Fn(&Delta) -> String,
) -> String {
    let section = if ty == DocType::Rule {
        "Rules"
    } else {
        "Requirements"
    };
    let mut out = String::from("---\nstatus: active\n");
    if type_line {
        out.push_str(&format!("type: {}\n", ty.name()));
    }
    out.push_str(&format!(
        "---\n# {}\n\n## {section}\n\n",
        split::title_of(id)
    ));
    for d in lines {
        out.push_str(&format!("- **{}**: {}\n", d.key, sentence(d)));
    }
    out
}

/// The corpus after the writes, with the files of the folder removed.
fn rebuild(cx: &Ctx, writes: &[Write], removed: &HashSet<String>) -> Corpus {
    let by_path: HashMap<&str, &Write> = writes.iter().map(|w| (w.path.as_str(), w)).collect();
    let mut docs: Vec<(String, Arc<ParsedDoc>)> = Vec::new();
    for d in &cx.head.docs {
        if removed.contains(&d.path) {
            continue;
        }
        let parsed = match by_path.get(d.path.as_str()) {
            Some(w) => Arc::new(parse(&w.new)),
            None => d.parsed.clone(),
        };
        docs.push((d.path.clone(), parsed));
    }
    let mut files: Vec<String> = cx
        .head
        .files
        .iter()
        .filter(|f| !removed.contains(*f))
        .cloned()
        .collect();
    for w in writes.iter().filter(|w| w.old.is_none()) {
        docs.push((w.path.clone(), Arc::new(parse(&w.new))));
        files.push(w.path.clone());
    }
    let mut c = Corpus::build(cx.config, files, docs);
    c.outside = cx.head.outside.clone();
    c.covers = cx.head.covers.clone();
    c
}

/// Adds to `writes` the text of each generated index whose entries change:
/// an index that lists a document of the folder, and an index that lists a new
/// document. Returns the corpus of the whole result. A plan is the same
/// whether the index is fresh or stale, because `registry::render` writes the
/// whole index.
fn finish_indexes(
    cx: &Ctx,
    removed: &HashSet<String>,
    writes: &mut Vec<Write>,
) -> Result<Corpus, Failure> {
    let mid = rebuild(cx, writes, removed);
    let mut more: Vec<Write> = Vec::new();
    for d in &cx.head.docs {
        if d.generated.is_none() || removed.contains(&d.path) {
            continue;
        }
        let old = registry::entries(cx.head, &d.path);
        let new = registry::entries(&mid, &d.path);
        if registry::hash(&old) == registry::hash(&new) {
            continue;
        }
        let text = (cx.read)(&d.path).map_err(Failure::Io)?;
        let title = Some(d.parsed.title.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| split::title_of(parent(&d.path)));
        // A place that gives no type keeps the front matter as it is.
        let status = registry::index_status(cx.config, &d.path, Some(&text)).unwrap_or(None);
        let new_text = registry::render(&d.path, &new, Some(&text), &title, status);
        more.push(Write {
            path: d.path.clone(),
            old: Some(text),
            new: new_text,
        });
    }
    writes.extend(more);
    writes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(rebuild(cx, writes, removed))
}

/// The rebuilt corpus may add no error. The only exception is SD305 on a new
/// document: the author adds the sections that the root requires.
fn no_new_errors(
    cx: &Ctx,
    rebuilt: &Corpus,
    base: Option<&Corpus>,
    created: &HashSet<String>,
) -> Result<(), Failure> {
    type Key = (Code, String, String);
    // SD937 is an error only for a spec that the diff touches, as `check` reads it
    // ([`lifecycle::lower_untouched`]). SD914 stays an error for each spec, as in
    // 0.7.1. The result of `finish` deletes
    // the folder of this change, so a stale pin that it shows is the pin of another
    // open spec. If this diff touches that spec, `check` of the pull request gives
    // an error, so `finish` stops. If not, it is a warning, and the author of that
    // spec pins the rule again (design 0.8.0, D6).
    let touched = |p: &str| cx.diff.is_some_and(|d| d.files.iter().any(|(f, _)| f == p));
    let errors = |c: &Corpus| -> Vec<Diag> {
        let mut all = check::diagnose_against(cx.config, c, base);
        lifecycle::lower_untouched(&mut all, &lifecycle::FINISH_TOUCHED_CODES, &touched);
        all.into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    };
    let mut before: HashMap<Key, usize> = HashMap::new();
    for d in errors(cx.head) {
        *before
            .entry((d.code, d.doc.clone(), d.target.clone()))
            .or_default() += 1;
    }
    let mut seen: HashMap<Key, usize> = HashMap::new();
    let mut new: Vec<Diag> = Vec::new();
    for d in errors(rebuilt) {
        if d.code == Code::SectionMissing && created.contains(&d.path) {
            continue;
        }
        let key = (d.code, d.doc.clone(), d.target.clone());
        let n = seen.entry(key.clone()).or_default();
        *n += 1;
        if *n > before.get(&key).copied().unwrap_or(0) {
            new.push(d);
        }
    }
    if new.is_empty() {
        return Ok(());
    }
    stop(
        5,
        format!(
            "the result would add {} error(s); nothing changed",
            new.len()
        ),
        new,
    )
}

/// The failure of `write_plan`: what is on disk now.
#[derive(Debug)]
pub struct WriteError {
    pub message: String,
}

/// Writes the plan: all the files, then the deletion of the folder, then
/// `git add -A` of each path. Each file goes to a temporary file first, and a
/// rename puts it in place. If a temporary file cannot be written, nothing has
/// changed. If a rename fails, the files that were renamed get their old text
/// again, and the message lists the files.
pub fn write_plan(
    root: &std::path::Path,
    git: &crate::git::Git,
    plan: &Plan,
) -> Result<(), WriteError> {
    let err = |message: String| WriteError { message };
    let tmp_of = |p: &str| root.join(format!("{p}.spec-dna.tmp"));
    let mut tmps: Vec<std::path::PathBuf> = Vec::new();
    let cleanup = |tmps: &[std::path::PathBuf]| {
        for t in tmps {
            let _ = std::fs::remove_file(t);
        }
    };
    for w in &plan.writes {
        let tmp = tmp_of(&w.path);
        let made = tmp
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&tmp, &w.new));
        if let Err(e) = made {
            cleanup(&tmps);
            return Err(err(format!(
                "cannot write `{}`: {e}; nothing changed. These files were to change: {}",
                w.path,
                list(plan.writes.iter().map(|w| w.path.as_str()))
            )));
        }
        tmps.push(tmp.clone());
        if w.old.is_some()
            && let Ok(meta) = std::fs::metadata(root.join(&w.path))
        {
            let _ = std::fs::set_permissions(&tmp, meta.permissions());
        }
    }
    let mut done: Vec<&Write> = Vec::new();
    for w in &plan.writes {
        if let Err(e) = std::fs::rename(tmp_of(&w.path), root.join(&w.path)) {
            cleanup(&tmps);
            for d in &done {
                match &d.old {
                    Some(old) => {
                        let _ = std::fs::write(root.join(&d.path), old);
                    }
                    None => {
                        let _ = std::fs::remove_file(root.join(&d.path));
                    }
                }
            }
            return Err(err(format!(
                "cannot replace `{}`: {e}. These files were written and restored: {}. These were not written: {}",
                w.path,
                list(done.iter().map(|d| d.path.as_str())),
                list(plan.writes.iter().skip(done.len()).map(|w| w.path.as_str()))
            )));
        }
        done.push(w);
    }
    let mut gone: Vec<&str> = Vec::new();
    for p in &plan.removes {
        if let Err(e) = std::fs::remove_file(root.join(p)) {
            return Err(err(format!(
                "cannot delete `{p}`: {e}. The files were written, and these were deleted: {}. These were not deleted: {}",
                list(gone.iter().copied()),
                list(plan.removes.iter().skip(gone.len()).map(String::as_str))
            )));
        }
        gone.push(p);
    }
    // Remove the folders that are empty now, the deepest first. A folder
    // that holds a file that git does not track (an ignored file) stays. The
    // folders above the folder of the change stay.
    let mut dirs: Vec<&str> = plan
        .removes
        .iter()
        .filter_map(|p| p.rsplit_once('/').map(|(d, _)| d))
        .filter(|d| in_folder(&plan.folder, d))
        .collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    dirs.dedup();
    for d in dirs {
        let mut cur = d;
        while in_folder(&plan.folder, cur) && std::fs::remove_dir(root.join(cur)).is_ok() {
            match cur.rsplit_once('/') {
                Some((up, _)) => cur = up,
                None => break,
            }
        }
    }
    let mut paths: Vec<String> = plan.writes.iter().map(|w| w.path.clone()).collect();
    paths.extend(plan.removes.iter().cloned());
    git.add_paths(&paths).map_err(|e| {
        err(format!(
            "the files are written and deleted, but `git add` failed: {e}"
        ))
    })
}

fn list<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let v: Vec<String> = items.map(|p| format!("`{p}`")).collect();
    if v.is_empty() {
        "none".to_string()
    } else {
        v.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_deviations_is_for_a_deviation_or_a_pin_of_0_8_0() {
        assert!(!emits_deviations("0.7.1", 0));
        assert!(emits_deviations("0.7.1", 1));
        assert!(emits_deviations("0.8.0", 0));
        assert!(emits_deviations("0.9.2", 0));
        assert!(!emits_deviations("not a version", 0));
    }

    fn config(extra: &str) -> Config {
        Config::parse(&format!(
            "spec-dna = \"0.1.0\"\nprofile = \"standard\"\n[change]\npolicy = \"three-class\"\ngate = \"warn\"\n{extra}\n[rules]\nSD608 = \"off\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
        ))
        .unwrap()
    }

    fn corpus(cfg: &Config, files: &[(&str, &str)]) -> Corpus {
        // The test file that the evidence names is a file of the tree.
        let mut paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        paths.push("tests/a.rs".to_string());
        Corpus::build(
            cfg,
            paths,
            files
                .iter()
                .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
                .collect(),
        )
    }

    const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds. <!-- verified-by: tests/a.rs -->\n  - nested note\n";
    const INTENT: &str =
        "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nText.\n";
    const PLAN: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n";
    const VALVES_PATH: &str = "docs/requirements/valves.md";

    fn spec(lines: &[String]) -> String {
        format!(
            "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n{}\n",
            lines.join("\n")
        )
    }

    fn hash_of(cfg: &Config, text: &str, key: &str) -> String {
        let c = corpus(cfg, &[(VALVES_PATH, text)]);
        let r = c.docs[0].parsed.reqs.iter().find(|r| r.key == key).unwrap();
        crate::hash::hex(&r.hash)
    }

    /// The tree files of the change `dose`, with the given requirements file
    /// and spec text.
    fn files<'a>(valves: &'a str, spec_text: &'a str) -> Vec<(&'a str, &'a str)> {
        vec![
            (VALVES_PATH, valves),
            ("docs/intent/dose/intent.md", INTENT),
            ("docs/intent/dose/spec.md", spec_text),
            ("docs/intent/dose/plan.md", PLAN),
        ]
    }

    /// The plan with a head tree, a base tree and the `covers:` targets of the
    /// test files.
    fn go(
        cfg: &Config,
        head_files: &[(&str, &str)],
        base_files: &[(&str, &str)],
        covers: &[&str],
    ) -> Result<Plan, Failure> {
        let mut head = corpus(cfg, head_files);
        head.covers = covers
            .iter()
            .enumerate()
            .map(|(i, t)| crate::evidence::Cover {
                path: "tests/test_valves.py".to_string(),
                line: i as u32 + 1,
                target: t.to_string(),
                partial: false,
                case: None,
                proof: None,
            })
            .collect();
        let base = corpus(cfg, base_files);
        let folder_files: Vec<String> = head_files
            .iter()
            .map(|f| f.0.to_string())
            .filter(|p| p.starts_with("docs/intent/dose/"))
            .collect();
        let texts: HashMap<String, String> = head_files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect();
        let read = |p: &str| texts.get(p).cloned().ok_or_else(|| format!("no file {p}"));
        let exists = |p: &str| texts.contains_key(p);
        plan(&Ctx {
            config: cfg,
            head: &head,
            base: Some(&base),
            diff: None,
            folder: "docs/intent/dose",
            folder_files: &folder_files,
            read: &read,
            exists: &exists,
            path_problem: &|_| None,
            reports: None,
            acceptance: None,
        })
    }

    /// The plan of a tree whose base is the same tree.
    fn run(cfg: &Config, valves: &str, spec_text: &str, covers: &[&str]) -> Result<Plan, Failure> {
        let f = files(valves, spec_text);
        go(cfg, &f, &f, covers)
    }

    fn write_of<'a>(plan: &'a Plan, path: &str) -> &'a Write {
        plan.writes.iter().find(|w| w.path == path).unwrap()
    }

    fn stopped(r: Result<Plan, Failure>) -> Stop {
        match r {
            Err(Failure::Stop(s)) => s,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_slug_is_one_path_segment() {
        for ok in ["dose", "a-b", "x_1", "Dose"] {
            assert!(valid_slug(ok), "{ok}");
        }
        for bad in ["", ".", "..", "a/b", "../x", "a\\b", ".hidden", "a\0b"] {
            assert!(!valid_slug(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_markers_the_nested_list_and_the_key_stay_on_a_change() {
        let cfg = config("");
        let h = hash_of(&cfg, VALVES, "open");
        let s = spec(&[format!(
            "- CHANGE requirements/valves#open@{h}: The controller SHALL open the valve in 3 seconds."
        )]);
        let p = run(&cfg, VALVES, &s, &[]).unwrap();
        let w = write_of(&p, VALVES_PATH);
        assert_eq!(
            w.new,
            VALVES.replace(
                "open the valve in 5 seconds.",
                "open the valve in 3 seconds."
            )
        );
        assert_eq!(p.applied, (0, 1, 0));
        assert_eq!(p.removes.len(), 3);
    }

    #[test]
    fn a_marker_inside_the_old_sentence_stays_on_a_change() {
        let cfg = config("");
        let valves = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **k**: First part <!-- aliases: old/valves#k --> and\n  the second part. <!-- verified-by: tests/a.rs -->\n  - nested\n- **m**: The valve SHALL open <!-- verified-by: tests/a.rs --> now.\n";
        let h = hash_of(&cfg, valves, "k");
        let s = spec(&[
            format!("- CHANGE requirements/valves#k@{h}: The valve SHALL close in 2 seconds."),
            // The marker that the line has already is not written twice.
            format!(
                "- CHANGE requirements/valves#m@{}: The valve SHALL open soon. <!-- verified-by: tests/a.rs -->",
                hash_of(&cfg, valves, "m")
            ),
        ]);
        let p = run(&cfg, valves, &s, &[]).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **k**: The valve SHALL close in 2 seconds. <!-- aliases: old/valves#k --> <!-- verified-by: tests/a.rs -->\n  - nested\n- **m**: The valve SHALL open soon. <!-- verified-by: tests/a.rs -->\n"
        );
    }

    #[test]
    fn a_change_with_a_proof_owed_marker_drops_the_old_one() {
        let cfg = config("");
        let valves = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **k**: The valve SHALL close. <!-- proof-owed: manual -->\n- **m**: The valve SHALL open <!-- proof-owed: manual --> now.\n- **n**: The valve SHALL drain. <!-- proof-owed: manual -->\n";
        let at = |k| hash_of(&cfg, valves, k);
        let s = spec(&[
            format!(
                "- CHANGE requirements/valves#k@{}: The valve SHALL close in 2 seconds. <!-- proof-owed: environment -->",
                at("k")
            ),
            format!(
                "- CHANGE requirements/valves#m@{}: The valve SHALL open soon. <!-- proof-owed: external -->",
                at("m")
            ),
            // No marker in the line: the debt of the requirement stays.
            format!(
                "- CHANGE requirements/valves#n@{}: The valve SHALL drain fast.",
                at("n")
            ),
        ]);
        let covers = ["k", "m", "n"].map(|k| format!("requirements/valves#{k}"));
        let covers: Vec<&str> = covers.iter().map(String::as_str).collect();
        let p = run(&cfg, valves, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **k**: The valve SHALL close in 2 seconds. <!-- proof-owed: environment -->\n- **m**: The valve SHALL open soon. <!-- proof-owed: external -->\n- **n**: The valve SHALL drain fast. <!-- proof-owed: manual -->\n"
        );
    }

    #[test]
    fn add_follows_the_last_item_with_its_marker_and_number() {
        let cfg = config("");
        let s = spec(&[
            "- ADD requirements/valves#flush: The controller SHALL flush the line.".to_string(),
            "- ADD requirements/valves#drain: The controller SHALL drain the line.".to_string(),
        ]);
        let covers = ["requirements/valves#flush", "requirements/valves#drain"];
        // A bulleted list, with a nested item in the last item.
        let p = run(&cfg, VALVES, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            format!(
                "{VALVES}- **flush**: The controller SHALL flush the line.\n- **drain**: The controller SHALL drain the line.\n"
            )
        );
        // An ordered list gets the next numbers.
        let ordered = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n7. **close**: The controller SHALL close the valve.\n8. **open**: The controller SHALL open the valve.\n";
        let p = run(&cfg, ordered, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            format!(
                "{ordered}9. **flush**: The controller SHALL flush the line.\n10. **drain**: The controller SHALL drain the line.\n"
            )
        );
        // A loose list gets a blank line before each new item.
        let loose = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve.\n\n- **open**: The controller SHALL open the valve.\n\n## Notes\n\nText.\n";
        let p = run(&cfg, loose, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            loose.replace(
                "open the valve.\n\n## Notes",
                "open the valve.\n\n- **flush**: The controller SHALL flush the line.\n\n- **drain**: The controller SHALL drain the line.\n\n## Notes"
            )
        );
    }

    #[test]
    fn add_keeps_crlf_a_bom_and_a_missing_final_newline() {
        let cfg = config("");
        let s = spec(&["- ADD requirements/valves#flush: The controller SHALL flush.".to_string()]);
        let covers = ["requirements/valves#flush"];
        let crlf = format!("\u{feff}{}", VALVES.replace('\n', "\r\n"));
        let p = run(&cfg, &crlf, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            format!("{crlf}- **flush**: The controller SHALL flush.\r\n")
        );
        let bare =
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.";
        let p = run(&cfg, bare, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            format!("{bare}\n- **flush**: The controller SHALL flush.")
        );
    }

    #[test]
    fn add_to_a_document_with_no_item_uses_its_section_or_adds_one() {
        let cfg = config("");
        let s = spec(&["- ADD requirements/valves#flush: The controller SHALL flush.".to_string()]);
        let covers = ["requirements/valves#flush"];
        let with = "---\nstatus: active\n---\n# V\n\n## Requirements\n\nNothing yet.\n\n## Notes\n\nText.\n";
        let p = run(&cfg, with, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\nNothing yet.\n\n- **flush**: The controller SHALL flush.\n\n## Notes\n\nText.\n"
        );
        let without = "---\nstatus: active\n---\n# V\n\nText.\n";
        let p = run(&cfg, without, &s, &covers).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\nText.\n\n## Requirements\n\n- **flush**: The controller SHALL flush.\n"
        );
    }

    #[test]
    fn a_new_document_has_the_minimal_form() {
        let cfg = config("");
        let s = spec(&[
            "- ADD requirements/pump/priming#dry-run: The pump SHALL stop when it runs dry."
                .to_string(),
            "- ADD requirements/pump/priming#prime: The pump SHALL prime for 4 seconds."
                .to_string(),
        ]);
        let covers = [
            "requirements/pump/priming#dry-run",
            "requirements/pump/priming#prime",
        ];
        let p = run(&cfg, VALVES, &s, &covers).unwrap();
        let w = write_of(&p, "docs/requirements/pump/priming.md");
        assert_eq!(w.old, None);
        // The layer `requirements` gives the type, so no `type:` line.
        assert_eq!(
            w.new,
            "---\nstatus: active\n---\n# Priming\n\n## Requirements\n\n- **dry-run**: The pump SHALL stop when it runs dry.\n- **prime**: The pump SHALL prime for 4 seconds.\n"
        );
    }

    #[test]
    fn remove_drops_the_lines_of_the_item_and_one_blank_line() {
        let cfg = config("");
        let valves = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.\n\n- **b**: The valve SHALL close.\n  - nested\n\n- **c**: The valve SHALL drain.\n";
        let h = hash_of(&cfg, valves, "b");
        let s = spec(&[format!("- REMOVE requirements/valves#b@{h}")]);
        let p = run(&cfg, valves, &s, &[]).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.\n\n- **c**: The valve SHALL drain.\n"
        );
        assert_eq!(p.applied, (0, 0, 1));
    }

    #[test]
    fn an_add_after_a_removed_last_item_goes_behind_it() {
        let cfg = config("");
        let valves = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.\n- **b**: The valve SHALL close.\n";
        let h = hash_of(&cfg, valves, "b");
        let s = spec(&[
            format!("- REMOVE requirements/valves#b@{h}"),
            "- ADD requirements/valves#c: The valve SHALL drain.".to_string(),
        ]);
        let p = run(&cfg, valves, &s, &["requirements/valves#c"]).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.\n- **c**: The valve SHALL drain.\n"
        );
    }

    #[test]
    fn a_new_item_takes_the_place_of_the_only_item_that_a_remove_drops() {
        let cfg = config("");
        let head = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n";
        let covers = ["requirements/valves#c"];
        let add = "- ADD requirements/valves#c: The valve SHALL drain.".to_string();
        for (old, new) in [
            (
                "- **a**: The valve SHALL open.\n",
                "- **c**: The valve SHALL drain.\n",
            ),
            (
                "7. **a**: The valve SHALL open.\n",
                "7. **c**: The valve SHALL drain.\n",
            ),
            (
                "- **a**: The valve SHALL open.\n\n## Notes\n\nText.\n",
                "- **c**: The valve SHALL drain.\n\n## Notes\n\nText.\n",
            ),
            (
                "1. **a**: The valve SHALL open.\n2. **b**: The valve SHALL close.\n",
                "1. **c**: The valve SHALL drain.\n",
            ),
        ] {
            let valves = format!("{head}{old}");
            let mut lines = vec![format!(
                "- REMOVE requirements/valves#a@{}",
                hash_of(&cfg, &valves, "a")
            )];
            if old.contains("**b**") {
                lines.push(format!(
                    "- REMOVE requirements/valves#b@{}",
                    hash_of(&cfg, &valves, "b")
                ));
            }
            lines.push(add.clone());
            let p = run(&cfg, &valves, &spec(&lines), &covers).unwrap();
            assert_eq!(write_of(&p, VALVES_PATH).new, format!("{head}{new}"));
        }
    }

    #[test]
    fn a_remove_with_nested_requirements_needs_a_line_for_each_of_them() {
        let cfg = config("");
        let valves = "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **a**: The valve SHALL open.\n  - **a1**: The valve SHALL report.\n    - **a2**: The valve SHALL log.\n\n- **b**: The valve SHALL close.\n";
        let line = |k: &str| {
            format!(
                "- REMOVE requirements/valves#{k}@{}",
                hash_of(&cfg, valves, k)
            )
        };
        let st = stopped(run(&cfg, valves, &spec(&[line("a")]), &[]));
        assert_eq!(st.step, 4);
        assert!(st.reason.contains("`a1`, `a2`"), "{}", st.reason);
        // Each key has a line: the nested lines are part of the outer span.
        let all = spec(&[line("a2"), line("a"), line("a1")]);
        let p = run(&cfg, valves, &all, &[]).unwrap();
        assert_eq!(
            write_of(&p, VALVES_PATH).new,
            "---\nstatus: active\n---\n# V\n\n## Requirements\n\n- **b**: The valve SHALL close.\n"
        );
        assert_eq!(p.applied, (0, 0, 3));
    }

    #[cfg(unix)]
    #[test]
    fn a_path_with_a_link_or_outside_the_root_is_a_problem() {
        use std::os::unix::fs::symlink;
        let dir =
            std::env::temp_dir().join(format!("spec-dna-finish-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (root, outside) = (dir.join("repo"), dir.join("outside"));
        std::fs::create_dir_all(root.join("docs/real")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("docs/real/a.md"), "x").unwrap();
        symlink(&outside, root.join("docs/out")).unwrap();
        symlink(root.join("docs/real/a.md"), root.join("docs/link.md")).unwrap();
        // The root can be behind a link, as a temporary folder is on macOS.
        symlink(&root, dir.join("alias")).unwrap();
        for ok in ["docs/real/a.md", "docs/real/new.md", "docs/new/deep/x.md"] {
            for base in [&root, &dir.join("alias")] {
                assert_eq!(path_problem(base, ok), None, "{ok}");
            }
        }
        for bad in [
            "docs/out/x.md",
            "docs/out",
            "docs/link.md",
            "docs/out/deep/x.md",
            "docs/../x.md",
            "/etc/x.md",
            "",
        ] {
            assert!(path_problem(&root, bad).is_some(), "{bad}");
        }
        let why = path_problem(&root, "docs/out/x.md").unwrap();
        assert!(why.contains("symbolic link `docs/out`"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_plan_is_idempotent_a_second_apply_changes_nothing() {
        let cfg = config("");
        let h = hash_of(&cfg, VALVES, "close");
        let s = spec(&[
            format!(
                "- CHANGE requirements/valves#close@{h}: The controller SHALL close the valve in 2 seconds."
            ),
            "- ADD requirements/valves#flush: The controller SHALL flush the line.".to_string(),
            "- ADD requirements/new/pump#prime: The pump SHALL prime.".to_string(),
        ]);
        let covers = [
            "requirements/valves#close",
            "requirements/valves#flush",
            "requirements/new/pump#prime",
        ];
        let first = run(&cfg, VALVES, &s, &covers).unwrap();
        assert_eq!(first.applied, (2, 1, 0));
        // The tree after the first apply: the head has the result, the base
        // has the old text. A second plan on it writes no requirement file.
        let valves = write_of(&first, VALVES_PATH).new.clone();
        let pump = write_of(&first, "docs/requirements/new/pump.md")
            .new
            .clone();
        let mut head_files = files(&valves, &s);
        head_files.push(("docs/requirements/new/pump.md", pump.as_str()));
        let base_files = files(VALVES, &s);
        let second = go(&cfg, &head_files, &base_files, &covers).unwrap();
        assert!(second.writes.is_empty(), "{:?}", second.writes);
        assert_eq!(second.applied, (0, 0, 0));
        assert_eq!(second.removes, first.removes);
    }

    #[test]
    fn a_hash_that_does_not_match_stops_at_step_3() {
        let cfg = config("");
        let s = spec(&[
            "- CHANGE requirements/valves#close@0123456789abcdef: The controller SHALL close."
                .to_string(),
        ]);
        let st = stopped(run(&cfg, VALVES, &s, &[]));
        assert_eq!(st.step, 3);
        assert_eq!(st.diags.len(), 1);
        assert_eq!(st.diags[0].code, Code::DeltaInvalid);
        assert_eq!(st.diags[0].severity, Severity::Error);
    }

    #[test]
    fn an_edit_of_the_approved_delta_stops_at_step_3() {
        let cfg = config("");
        let h = hash_of(&cfg, VALVES, "close");
        let line = |t: &str| {
            format!(
                "- CHANGE requirements/valves#close@{h}: The controller SHALL close in {t} seconds."
            )
        };
        let head_spec = spec(&[line("3")]);
        let base_spec = spec(&[line("4")]);
        let st = stopped(go(
            &cfg,
            &files(VALVES, &head_spec),
            &files(VALVES, &base_spec),
            &["requirements/valves#close"],
        ));
        assert_eq!(st.step, 3);
        assert!(
            st.diags[0].message.contains("approved delta"),
            "{:?}",
            st.diags
        );
    }

    #[test]
    fn a_change_that_the_base_does_not_have_has_no_approved_delta() {
        let cfg = config("");
        let s = spec(&["- ADD requirements/valves#flush: The controller SHALL flush.".to_string()]);
        let base = [(VALVES_PATH, VALVES)];
        let p = go(
            &cfg,
            &files(VALVES, &s),
            &base,
            &["requirements/valves#flush"],
        )
        .unwrap();
        assert_eq!(p.applied, (1, 0, 0));
    }

    #[test]
    fn a_missing_evidence_stops_at_step_5_and_a_closed_change_needs_none() {
        let cfg = config("");
        let s = spec(&["- ADD requirements/valves#flush: The controller SHALL flush.".to_string()]);
        let st = stopped(run(&cfg, VALVES, &s, &[]));
        assert_eq!(st.step, 5);
        assert_eq!(st.diags[0].code, Code::DeltaEvidenceMissing);
        // A closed intent: no step 2 to 5, and only the folder goes away.
        let closed = INTENT.replace("accepted", "closed");
        let f = [
            (VALVES_PATH, VALVES),
            ("docs/intent/dose/intent.md", closed.as_str()),
            ("docs/intent/dose/spec.md", s.as_str()),
        ];
        let p = go(&cfg, &f, &f, &[]).unwrap();
        assert!(p.writes.is_empty());
        assert_eq!(p.removes.len(), 2);
    }

    #[test]
    fn the_statuses_of_step_1_stop_before_the_delta() {
        let cfg = config("");
        let s = spec(&["- ADD requirements/valves#flush: The controller SHALL flush.".to_string()]);
        let draft = INTENT.replace("accepted", "draft");
        let f = [
            (VALVES_PATH, VALVES),
            ("docs/intent/dose/intent.md", draft.as_str()),
            ("docs/intent/dose/spec.md", s.as_str()),
            ("docs/intent/dose/plan.md", PLAN),
        ];
        let st = stopped(go(&cfg, &f, &f, &[]));
        assert_eq!(st.step, 1);
        assert!(st.reason.contains("`draft`"), "{}", st.reason);
        // The plan and the spec of a higher-risk change.
        let draft_spec = s.replace("approved", "draft");
        let draft_plan = PLAN.replace("approved", "draft");
        let f = [
            (VALVES_PATH, VALVES),
            ("docs/intent/dose/intent.md", INTENT),
            ("docs/intent/dose/spec.md", draft_spec.as_str()),
            ("docs/intent/dose/plan.md", draft_plan.as_str()),
        ];
        let st = stopped(go(&cfg, &f, &f, &[]));
        assert_eq!(st.step, 1);
        assert!(
            st.reason.contains("the plan is `draft`") && st.reason.contains("the spec is `draft`"),
            "{}",
            st.reason
        );
    }

    #[test]
    fn the_hours_of_the_tasks_add_up_as_a_range() {
        let tasks = |lines: &[&str]| {
            let src = format!("# P\n\n## Order\n\n{}\n", lines.join("\n"));
            let mut seen = HashSet::new();
            let mut tasks: Vec<_> = parse(&src)
                .items
                .iter()
                .filter(|i| i.section == ItemSection::Order)
                .filter_map(change::parse_task)
                .collect();
            for t in &mut tasks {
                t.duplicate = !seen.insert(t.id.clone());
            }
            tasks
        };
        let h = |lines: &[&str]| task_hours(&tasks(lines));
        let t = h(&["- [x] a: A. Hours: 1-2.", "- [ ] b: B. Hours: 1.5."]);
        assert_eq!(t.total, Some((250, 350)));
        assert_eq!(t.text().as_deref(), Some("2.5-3.5"));
        assert_eq!(t.json(), "{\"low\":2.5,\"high\":3.5}");
        // A skipped task and a duplicate id add nothing.
        let t = h(&[
            "- [x] a: A. Hours: 2.",
            "- [x] b: B. Hours: 9. Skip: no.",
            "- [x] a: C. Hours: 5.",
        ]);
        assert_eq!(t.text().as_deref(), Some("2"));
        assert!(t.missing.is_empty() && t.invalid.is_empty());
        // The notes: a task with no value, and a bad value.
        let t = h(&["- [x] a: A. Hours: 2.", "- [x] b: B.", "- [x] c: C."]);
        assert_eq!(t.missing, ["b", "c"]);
        let t = h(&[
            "- [x] a: A. Hours: 2.",
            "- [x] b: B. Hours: soon.",
            "- [x] c: C.",
        ]);
        assert_eq!(t.total, None);
        assert_eq!(t.invalid, [("b".to_string(), "soon".to_string())]);
        assert!(t.missing.is_empty());
        // No value at all: no total and no note.
        let t = h(&["- [x] a: A.", "- [x] b: B."]);
        assert_eq!(t, Hours::default());
        assert_eq!(t.json(), "null");
    }

    #[test]
    fn the_total_has_no_zero_after_the_last_digit() {
        for (hundredths, want) in [
            (350, "3.5"),
            (400, "4"),
            (30, "0.3"),
            (205, "2.05"),
            (0, "0"),
        ] {
            assert_eq!(format_hours(hundredths), want);
        }
    }
}
