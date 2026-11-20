//! The stage of a change and the commands `next` and `status` (design 0.6.0,
//! "Stage derivation"). A change has no stage field. This module derives the
//! stage from the artifacts, their statuses, the diff against the lifecycle base
//! and the evidence of the requirements. One function (`Ctx::derive`) gives the
//! stage, the checklist and the next task. `next` and `status` both print its
//! result, so they cannot disagree.
//!
//! The stage `Maintain` is not the stage of a change: an incident with an open
//! item in `Follow-ups` is a follow-up, and `status` lists it.

use crate::acceptance::{self, Need};
use crate::approval::Branch;
use crate::change::{self, Change, PlanTask, Verb};
use crate::config::{Approval, ChangePolicy, Config};
use crate::diag::Code;
use crate::evidence::{self, Evidence};
use crate::json::{self, Obj};
use crate::lifecycle::{self, Diff, ReqChangeKind};
use crate::model::{Class, Corpus, DocType, Res, canonical};
use crate::parse::ItemSection;
use crate::record::{self, Record};
use crate::repo::{self, Fatal, Repo, Snapshot};
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};

/// The names of the skills that a step of the lifecycle uses, in this order.
/// The skills themselves come in 0.7.0. The names are fixed: `next` prints
/// them as data, and a later version writes the skills under these names.
pub const SKILLS: [&str; 10] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "execute",
    "tdd",
    "review",
    "verify",
    "accept",
    "finish",
];

/// A step of the lifecycle that has a skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The Plan stage: the intent.
    Plan,
    /// The Design stage: the spec.
    Specify,
    /// An open concern or an open question.
    Clarify,
    /// The Build stage with no approved plan.
    WritePlan,
    /// The Build stage with an open task, or with no change of the code yet.
    Execute,
    /// The Test stage: a requirement with no evidence.
    Test,
    /// The first item of the Deploy stage: run the proof of the plan.
    Verify,
    /// The review item of the Deploy stage.
    Review,
    /// The item of the Deploy stage that asks for the acceptance record, after
    /// the review and before `finish` (design 0.8.0, D3).
    Accept,
    /// A closed change, or a change that is ready to apply its delta.
    Finish,
}

/// The skill of each step. A test checks that each skill of `SKILLS` is here
/// once.
pub const STEP_SKILL: [(Step, &str); 10] = [
    (Step::Plan, "brainstorm"),
    (Step::Specify, "specify"),
    (Step::Clarify, "clarify"),
    (Step::WritePlan, "write-plan"),
    (Step::Execute, "execute"),
    (Step::Test, "tdd"),
    (Step::Review, "review"),
    (Step::Verify, "verify"),
    (Step::Accept, "accept"),
    (Step::Finish, "finish"),
];

/// The name of the skill of a step.
pub fn skill(step: Step) -> &'static str {
    let name = STEP_SKILL
        .iter()
        .find(|(s, _)| *s == step)
        .map_or("", |(_, n)| *n);
    debug_assert!(SKILLS.contains(&name), "`{name}` is not a skill name");
    name
}

/// The most items that `next` prints. With the 2 lines before them (the change
/// and the base), the output has at most 12 lines.
pub const MAX_ITEMS: usize = 10;

/// The stage of a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Closed,
    Plan,
    Design,
    Build,
    Test,
    Deploy,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Closed => "closed",
            Stage::Plan => "plan",
            Stage::Design => "design",
            Stage::Build => "build",
            Stage::Test => "test",
            Stage::Deploy => "deploy",
        }
    }
}

/// One line of the checklist of `next`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub done: bool,
    pub text: String,
    pub skill: &'static str,
    /// True for an open task of a person, and for the line that waits for it
    /// (design 0.8.0, D64). `next` prints `[person]` for it, and `--json` adds
    /// `"person": true`. An agent does not do the item.
    pub person: bool,
}

fn item(done: bool, step: Step, text: impl Into<String>) -> Item {
    Item {
        done,
        text: text.into(),
        skill: skill(step),
        person: false,
    }
}

/// An item that a person does. The skill stays the skill of the step.
fn person_item(step: Step, text: impl Into<String>) -> Item {
    Item {
        person: true,
        ..item(false, step, text)
    }
}

/// The end of the line of an item in `next`: the skill, or `[person]`.
fn item_tail(i: &Item) -> String {
    if i.person {
        "[person]".to_string()
    } else {
        format!("[skill: {}]", i.skill)
    }
}

/// Keeps at most `MAX_ITEMS` items and the last one. It drops the done items
/// first, from the start, and then the open item before the last one.
pub fn limit(mut items: Vec<Item>) -> Vec<Item> {
    while items.len() > MAX_ITEMS {
        let last = items.len() - 1;
        let at = items[..last]
            .iter()
            .position(|i| i.done)
            .unwrap_or(last - 1);
        items.remove(at);
    }
    items
}

/// The progress of the task list of a plan.
#[derive(Clone, Debug, Default)]
pub struct Tasks {
    /// The tasks with `- [x]` and no reason in `Skip:`. A duplicate id does not
    /// count.
    pub done: usize,
    /// The tasks with a reason in `Skip:`: they are not needed, and they are
    /// closed whatever their box.
    pub skipped: usize,
    /// The open tasks whose text names `spec-dna finish <slug>`: `finish`
    /// closes them (design 0.7.1, B1).
    pub finish: usize,
    /// All tasks. A duplicate id does not count.
    pub total: usize,
    pub next: Option<PlanTask>,
}

/// The progress of the tasks of a change, whatever its stage.
pub fn tasks(ch: &Change, head: &Corpus) -> Tasks {
    let all: Vec<PlanTask> = ch
        .tasks(head)
        .into_iter()
        .filter(|t| !t.duplicate)
        .collect();
    Tasks {
        done: all.iter().filter(|t| t.is_done()).count(),
        skipped: all.iter().filter(|t| t.skipped()).count(),
        finish: all
            .iter()
            .filter(|t| !t.closed() && t.closes_with_finish(&ch.slug))
            .count(),
        total: all.len(),
        next: ch.next_open_task(head),
    }
}

/// The result of the derivation for one change.
#[derive(Clone, Debug)]
pub struct Report {
    pub stage: Stage,
    /// The checklist to the next gate, at most `MAX_ITEMS` items. The last
    /// item is the edit or the command that ends the stage.
    pub items: Vec<Item>,
    /// The next open task of the plan, in the Build stage.
    pub next_task: Option<PlanTask>,
}

impl Report {
    /// The text of the last item: the next gate.
    pub fn next(&self) -> &str {
        self.items.last().map_or("", |i| i.text.as_str())
    }
}

/// The trees and the diff that the commands read.
pub struct View {
    pub head: Corpus,
    /// The tree of the lifecycle base, if git names the base.
    pub base: Option<Corpus>,
    pub base_rev: Option<String>,
    /// The diff of the working tree against the base.
    pub diff: Option<Diff>,
    pub notes: Vec<String>,
    /// True with `[change] approval = "branch"`. Then an approval line also
    /// tells an approver to commit the `status` line alone (design 0.7.0, D7).
    pub branch: bool,
    /// The item `accept` of each change that needs an acceptance record, by
    /// slug (design 0.8.0, D3 item 1). A change that needs none has no entry,
    /// so `next` prints no item for it.
    pub accept: HashMap<String, AcceptItem>,
}

/// The item `accept` of a change that needs a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptItem {
    /// True if the record is valid, fresh and `accepted`.
    pub done: bool,
    pub text: String,
}

/// Reads the working tree, the lifecycle base and the diff. An unknown `--base`
/// is exit code 3, and a git error is exit code 4. A change process that is off
/// (policy `none`) reads no second tree.
pub fn load(repo: &Repo, cfg: &Config, base: Option<&str>, use_cache: bool) -> Result<View, Fatal> {
    let base_rev = lifecycle::change_base(cfg, &repo.git, base).map_err(Fatal::Config)?;
    let head_snap = Snapshot::worktree(repo, &repo::root_paths(cfg), true)?;
    let base_snap = base_rev
        .as_deref()
        .map(|r| Snapshot::at(repo, r))
        .transpose()?;
    let mut snaps: Vec<&Snapshot> = vec![&head_snap];
    snaps.extend(base_snap.as_ref());
    let mut loaded = repo::load(repo, cfg, &snaps, use_cache, false)?;
    let base_corpus = if base_snap.is_some() {
        Some(loaded.corpora.remove(1))
    } else {
        None
    };
    let mut head = loaded.corpora.remove(0);
    head.covers = evidence::load(repo, cfg, None, true)?;
    let diff = base_rev
        .as_deref()
        .map(|r| Diff::read(&repo.git, r, true))
        .transpose()
        .map_err(Fatal::Io)?;
    let mut notes = Vec::new();
    if base_rev.is_none() {
        notes.push(lifecycle::no_base_note(cfg, &repo.git, base));
    }
    let accept = accept_items(
        repo,
        cfg,
        &head,
        base_corpus.as_ref(),
        base_rev.as_deref(),
        diff.as_ref(),
    )?;
    Ok(View {
        head,
        base: base_corpus,
        base_rev,
        diff,
        notes,
        branch: cfg.change_approval == Approval::Branch,
        accept,
    })
}

/// The item `accept` of each open change that has an approved plan and needs a
/// record, by the rule of `acceptance::need_for_change`. The map is empty with
/// the acceptance off (the profile `lite`), with no base, or with a plan of a
/// version before 0.8.0. Then `next` prints the output of 0.7.1.
fn accept_items(
    repo: &Repo,
    cfg: &Config,
    head: &Corpus,
    base: Option<&Corpus>,
    base_rev: Option<&str>,
    diff: Option<&Diff>,
) -> Result<HashMap<String, AcceptItem>, Fatal> {
    let mut out = HashMap::new();
    for ch in change::collect(head) {
        let approved = ch.plan.as_ref().is_some_and(|p| p.status == "approved");
        if !approved || ch.intent_status() != "accepted" {
            continue;
        }
        let need = acceptance::need_for_change(repo, cfg, &ch, head, base, base_rev)?;
        if let (Some(Need::Yes { .. }), Some(base_rev)) = (&need, base_rev) {
            out.insert(
                ch.slug.clone(),
                accept_item(repo, cfg, &ch, head, base_rev, diff)?,
            );
        }
    }
    Ok(out)
}

/// The item `accept` of one change that needs a record. The record is done when
/// the checks of `finish` step 5.2 pass for it (`acceptance.rs`). They pass when
/// the record is valid, its briefs are right, it is fresh and `accepted`, no 2
/// entries are of the same acceptor, and it has the entries that the change
/// needs. A code edit after the
/// acceptance makes it stale, and the item is open again (design 0.8.0, D3 item
/// 9). An open item names the first problem.
fn accept_item(
    repo: &Repo,
    cfg: &Config,
    ch: &Change,
    head: &Corpus,
    base_rev: &str,
    diff: Option<&Diff>,
) -> Result<AcceptItem, Fatal> {
    let slug = &ch.slug;
    let path = format!("{}/{}", ch.dir, record::FILE_NAME);
    let open = |text: String| AcceptItem { done: false, text };
    let on = |c: Code| !cfg.is_off(c);
    // `finish` reads the record of `HEAD`, and its step 0 refuses an uncommitted
    // record. The item is done only for a committed record.
    let committed = repo
        .git
        .read_at(&[("HEAD".to_string(), path.clone())])
        .map_err(Fatal::Io)?
        .into_iter()
        .next()
        .flatten()
        .map(|b| String::from_utf8_lossy(&b).into_owned());
    // A link or a file over the limit is not read: it is no record.
    let on_disk = match crate::git::read_limited(&repo.root.join(&path), acceptance::READ_LIMIT) {
        crate::git::Limited::Bytes(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        crate::git::Limited::Absent => None,
        other => {
            return Ok(open(format!(
                "{}. Then commit the record alone, and run `spec-dna finish {slug}`",
                acceptance::unreadable_message(&path, &other)
            )));
        }
    };
    let text = match (committed, on_disk) {
        (None, None) => {
            return Ok(open(format!(
                "accept the change: run `spec-dna accept {slug} --brief`, give the whole output, unchanged, to an agent `acceptor`, and commit the record that it writes"
            )));
        }
        // CRLF and LF are one record (ADR 0019).
        (Some(c), Some(d)) if record::to_lf(c.as_bytes()) == record::to_lf(d.as_bytes()) => c,
        _ => {
            return Ok(open(format!(
                "the record `{path}` has an edit that `HEAD` does not have: commit the record alone, then run `spec-dna finish {slug}`"
            )));
        }
    };
    let (rec, mut problems): (Option<Record>, Vec<String>) =
        acceptance::read_record(cfg, &text, slug, acceptance::plan_facts(ch, head));
    // The checks of `finish` step 5.2, by the one function that `finish` uses:
    // a check that is off stops neither of them.
    let stale = match &rec {
        Some(r) if on(Code::AcceptanceStale) => {
            let plan = acceptance::PlanScope::of(ch, head);
            acceptance::stale(cfg, &repo.git, r, diff, &plan, Some(base_rev), false)
                .map_err(Fatal::Io)?
        }
        _ => None,
    };
    if on(Code::AcceptanceInvalid)
        && let Some(r) = &rec
    {
        problems.extend(
            acceptance::brief_problems(cfg, &repo.git, base_rev, &ch.dir, r).map_err(Fatal::Io)?,
        );
    }
    let first = problems.first().cloned().unwrap_or_default();
    let sections =
        acceptance::sections_of(cfg, &repo.git, Some(base_rev), ch, head).map_err(Fatal::Io)?;
    let Some((code, messages)) =
        acceptance::first_stop(cfg, ch.class, rec.as_ref(), problems, stale, &sections)
    else {
        // With each check on, the record is valid, fresh and accepted. With a check
        // off, the text says only that the checks that are on passed.
        let all_on = [
            Code::AcceptanceInvalid,
            Code::AcceptanceStale,
            Code::AcceptanceRejected,
            Code::SelfAccepted,
            Code::AcceptorMissing,
            Code::RecordIncomplete,
            Code::DeviationInvalid,
            Code::DeviationUnresolved,
        ]
        .into_iter()
        .all(on);
        return Ok(AcceptItem {
            done: true,
            text: if all_on && rec.is_some() {
                format!("the record `{path}` is valid, fresh and accepted")
            } else {
                format!("the record `{path}` passes the checks that are on")
            },
        });
    };
    Ok(open(match code {
        Code::AcceptanceInvalid if rec.is_none() => {
            format!("the record `{path}` does not read ({first}); the acceptor fixes it")
        }
        Code::AcceptanceInvalid => {
            format!("the record `{path}` is not valid ({first}); the acceptor fixes it")
        }
        Code::AcceptanceStale => format!(
            "the record `{path}` is stale, because {}: delete it, commit, and run `spec-dna accept {slug} --brief` again",
            acceptance::stale_cause(messages.first().map_or("", String::as_str))
        ),
        Code::AcceptanceRejected => format!(
            "the record `{path}` says `rejected`: fix each finding in the code, delete the record, commit, and accept again"
        ),
        Code::SelfAccepted => format!(
            "the record `{path}` is not accepted ({}); run `spec-dna accept {slug}` again with another agent",
            messages.first().map_or("", String::as_str)
        ),
        _ => messages.into_iter().next().unwrap_or_default(),
    }))
}

/// The evidence of the requirements that the change adds or changes.
struct Proof {
    /// The number of such requirements.
    total: usize,
    /// The targets `<id>#<key>` that have no full evidence.
    missing: Vec<String>,
    /// The targets of `missing` that a partial `covers:` tag names, each with
    /// the first tag as `<path>:<line>`.
    partial: Vec<(String, String)>,
}

pub struct Ctx<'a> {
    view: &'a View,
    /// The changes of the working tree.
    pub changes: Vec<Change>,
    evidence: OnceCell<Evidence>,
}

impl<'a> Ctx<'a> {
    pub fn new(view: &'a View) -> Ctx<'a> {
        Ctx {
            view,
            changes: change::collect(&view.head),
            evidence: OnceCell::new(),
        }
    }

    /// True if a file of `Files` differs from the base. `None` if the base is
    /// unknown, so that the diff is unknown. A file of the folder of the change
    /// does not count.
    fn files_changed(&self, ch: &Change) -> Option<bool> {
        let diff = self.view.diff.as_ref()?;
        let files = ch.files(&self.view.head);
        let prefix = format!("{}/", ch.dir);
        Some(
            diff.files
                .iter()
                .any(|(p, _)| !p.starts_with(&prefix) && change::in_scope(&files, p)),
        )
    }

    /// The same rule as the check of the evidence in `finish`: a delta gives the
    /// requirements (an `ADD` or `CHANGE` line). With no delta line, the
    /// requirements are those whose hash differs from the base. If another change
    /// has a changed file in the diff, only the documents of `Files` count.
    fn proof(&self, ch: &Change) -> Proof {
        let head = &self.view.head;
        let lines = ch.delta(head);
        let mut targets: Vec<(String, String)> = Vec::new();
        if !lines.is_empty() {
            for l in &lines {
                if let Ok(d) = &l.parsed
                    && d.verb != Verb::Remove
                {
                    targets.push((d.id.clone(), d.key.clone()));
                }
            }
        } else if let (Some(base), Some(diff)) = (&self.view.base, &self.view.diff) {
            let files = ch.files(head);
            let others = self
                .changes
                .iter()
                .filter(|c| c.dir != ch.dir)
                .any(|c| diff.under(&c.dir).next().is_some());
            for r in lifecycle::requirement_changes(base, head, lifecycle::requirement_doc) {
                if r.kind != ReqChangeKind::Removed
                    && (!others || change::in_scope(&files, &r.path))
                {
                    targets.push((r.doc, r.key));
                }
            }
        }
        let mut missing = Vec::new();
        let mut partial = Vec::new();
        for (id, key) in &targets {
            let (full, part) = self.evidence_of(id, key);
            if full {
                continue;
            }
            let target = format!("{id}#{key}");
            if let Some(at) = part {
                partial.push((target.clone(), at));
            }
            missing.push(target);
        }
        Proof {
            total: targets.len(),
            missing,
            partial,
        }
    }

    /// The number of requirements that the change adds or changes, that have a
    /// partial `covers:` tag, and that have no full evidence. It counts at each
    /// stage.
    pub fn partial(&self, ch: &Change) -> usize {
        self.proof(ch).partial.len()
    }

    /// The evidence of a requirement: true if it has full evidence (a full
    /// `covers:` tag, a `verified-by` marker, or the `verified-by` of its
    /// document), and the first partial `covers:` tag as `<path>:<line>`. A
    /// requirement that the tree does not have yet (an `ADD` line) has full
    /// evidence if a full `covers:` tag names it.
    fn evidence_of(&self, id: &str, key: &str) -> (bool, Option<String>) {
        let head = &self.view.head;
        let ev = self.evidence.get_or_init(|| Evidence::build(head).0);
        if let Res::Found(d, None, _) = head.resolve(id) {
            let doc = &head.docs[d as usize];
            if let Some(r) = doc.req(key) {
                return (
                    !ev.of(d, doc, r).is_empty(),
                    ev.partial_of(d, r).first().cloned(),
                );
            }
        }
        let want = same_target(head, &format!("{id}#{key}"));
        let mut tags = head
            .covers
            .iter()
            .filter(|c| same_target(head, &c.target) == want);
        let full = tags.clone().any(|c| !c.partial);
        let part = tags
            .find(|c| c.partial)
            .map(|c| format!("{}:{}", c.path, c.line));
        (full, part)
    }

    /// The items for the open concerns (SD919) and the open questions (SD922)
    /// of the spec. A change with no spec has none.
    fn clarify_items(head: &Corpus, ch: &Change) -> Vec<Item> {
        let Some(a) = &ch.spec else {
            return Vec::new();
        };
        let dir = &ch.dir;
        let concerns = change::open_concerns(&head.docs[a.doc as usize]).len();
        let questions = ch.open_questions(head).len();
        let mut out = Vec::new();
        if concerns > 0 {
            out.push(item(
                false,
                Step::Clarify,
                format!("resolve {concerns} open concern(s) in `{dir}/spec.md` (SD919)"),
            ));
        }
        if questions > 0 {
            out.push(item(
                false,
                Step::Clarify,
                format!("resolve {questions} open question(s) in `{dir}/spec.md` (SD922)"),
            ));
        }
        out
    }

    /// The end of a line that asks for an approval. With the branch rule, an
    /// approver commits the `status` line alone, so that the commit counts
    /// (design 0.7.0, D7 and D11).
    fn alone(&self) -> &'static str {
        if self.view.branch {
            ": an approver commits the `status` line alone"
        } else {
            ""
        }
    }

    /// Derives the stage of a change and its checklist. The stage is the first
    /// row of the table of the design that matches.
    pub fn derive(&self, ch: &Change) -> Report {
        let head = &self.view.head;
        let slug = &ch.slug;
        let dir = &ch.dir;
        let intent = ch.intent_status();
        let spec = ch.spec.as_ref().map_or("", |a| a.status.as_str());
        let plan = ch.plan.as_ref().map_or("", |a| a.status.as_str());
        let higher = ch.class == Class::HigherRisk;
        let report = |stage, items: Vec<Item>, next_task| Report {
            stage,
            items: limit(items),
            next_task,
        };
        if intent == "closed" {
            return report(
                Stage::Closed,
                vec![
                    item(
                        true,
                        Step::Plan,
                        "the intent is closed: the change does not proceed",
                    ),
                    item(
                        false,
                        Step::Finish,
                        format!("run `spec-dna finish {slug}` to delete the folder `{dir}`"),
                    ),
                ],
                None,
            );
        }
        let alone = self.alone();
        if intent != "accepted" {
            let mut items = Self::clarify_items(head, ch);
            items.push(if ch.intent.is_none() {
                item(
                    false,
                    Step::Plan,
                    format!(
                        "write `{dir}/intent.md` with `spec-dna new intent {slug}`, then set `status: accepted` in it{alone}"
                    ),
                )
            } else if ch.class_unset {
                // The class comes first: an approval commit changes the
                // `status` line alone.
                item(
                    false,
                    Step::Plan,
                    format!(
                        "write `class: routine` or `class: higher-risk` in the front matter of {dir}/intent.md and commit it, then set `status: accepted`{alone}"
                    ),
                )
            } else {
                item(
                    false,
                    Step::Plan,
                    format!("set `status: accepted` in {dir}/intent.md{alone}"),
                )
            });
            return report(Stage::Plan, items, None);
        }
        let intent_done = item(true, Step::Plan, "the intent is accepted");
        // The class decides the next artifact, so an accepted intent with no
        // class stays at the stage `plan`.
        if ch.class_unset {
            return report(
                Stage::Plan,
                vec![
                    intent_done,
                    item(
                        false,
                        Step::Plan,
                        format!(
                            "write `class: routine` or `class: higher-risk` in the front matter of {dir}/intent.md; until then `check` reads the change as `routine`"
                        ),
                    ),
                ],
                None,
            );
        }
        if higher && spec != "approved" {
            let mut items = vec![intent_done];
            if ch.spec.is_none() {
                items.push(item(
                    false,
                    Step::Specify,
                    format!("write `{dir}/spec.md` with `spec-dna new spec {slug}`"),
                ));
            } else {
                let lines = ch.delta(head).len();
                if lines == 0 {
                    items.push(item(
                        false,
                        Step::Specify,
                        "write the lines of the `Delta` section (ADD, CHANGE or REMOVE)",
                    ));
                } else {
                    items.push(item(
                        true,
                        Step::Specify,
                        format!("the `Delta` section has {lines} line(s)"),
                    ));
                }
            }
            items.extend(Self::clarify_items(head, ch));
            // A line to approve a file that does not exist would be false.
            if ch.spec.is_some() {
                items.push(item(
                    false,
                    Step::Specify,
                    format!("set `status: approved` in {dir}/spec.md{alone}"),
                ));
            }
            return report(Stage::Design, items, None);
        }
        let mut done = vec![intent_done];
        if higher {
            done.push(item(true, Step::Specify, "the spec is approved"));
        }
        // A routine change with an approved plan that lists no files edits no
        // file. The build step then has no test of the diff. A higher-risk
        // change has a delta, so it keeps the test. For each plan, the build
        // step is done only when no task of `Order` is open (0.7.1).
        let no_files = !higher && plan == "approved" && ch.no_files(head);
        let changed = self.files_changed(ch);
        let all_tasks_closed = {
            let t = tasks(ch, head);
            t.done + t.skipped + t.finish == t.total
        };
        let build_done = all_tasks_closed && (no_files || changed == Some(true));
        if plan != "approved" || !build_done {
            let mut items = done;
            let mut next_task = None;
            let approved = plan == "approved";
            if ch.plan.is_none() {
                items.push(item(
                    false,
                    Step::WritePlan,
                    format!(
                        "write `{dir}/plan.md` with `spec-dna new plan {slug}`: `Files`, the task list of `Order`, `Risks` and `Proof`"
                    ),
                ));
            } else if approved {
                items.push(item(true, Step::WritePlan, "the plan is approved"));
                // This line is before the tasks, so that an open task or the
                // edit of the stage stays the last line.
                if no_files {
                    items.push(item(true, Step::Execute, NO_FILES_LINE));
                }
            }
            // A paragraph under `Files` lists no path. A draft plan gets the
            // line first, so that the approval stays the last line.
            if !approved && ch.files_paragraph_only(head) && ch.files(head).is_empty() {
                items.push(item(
                    false,
                    Step::WritePlan,
                    crate::change::FILES_PARAGRAPH_LINE,
                ));
            }
            // The tasks of the plan, also if the plan is a draft.
            let t = tasks(ch, head);
            if t.total > 0 {
                let skipped = match t.skipped {
                    0 => String::new(),
                    n => format!(", {n} not needed"),
                };
                let finish = match t.finish {
                    0 => String::new(),
                    n => format!(", {n} for `finish`"),
                };
                items.push(item(
                    t.done + t.skipped + t.finish == t.total,
                    Step::Execute,
                    format!(
                        "tasks of `Order`: {} of {} done{skipped}{finish}",
                        t.done, t.total
                    ),
                ));
            }
            let after_task = items.len() + usize::from(t.next.is_some());
            if let Some(task) = &t.next {
                items.push(if task.by_person() {
                    person_item(Step::Execute, task_line(task))
                } else {
                    item(false, Step::Execute, task_line(task))
                });
                next_task = Some(task.clone());
            }
            if !approved {
                if ch.plan.is_some() {
                    items.push(item(
                        false,
                        Step::WritePlan,
                        format!("set `status: approved` in {dir}/plan.md{alone}"),
                    ));
                }
            } else if !no_files && !(changed == Some(true) && !all_tasks_closed) {
                // A file of `Files` differs and a task is open: the open task
                // is the last line, so this line would be false.
                let files = ch.files(head);
                items.push(item(
                    false,
                    Step::Execute,
                    if files.is_empty() && ch.files_paragraph_only(head) {
                        crate::change::FILES_PARAGRAPH_LINE.to_string()
                    } else if files.is_empty() {
                        match ch.files_not_paths(head).first() {
                            Some(item) => format!(
                                "the items of `Files` in {dir}/plan.md are not paths, so it lists no path (the first is \"{item}\"); write each path or glob in a code span, as `src/valve.py`"
                            ),
                            None => format!(
                                "add the paths of the work to `Files` in {dir}/plan.md: it lists none"
                            ),
                        }
                    } else if changed.is_none() {
                        "change a file of `Files`; the base is unknown, so pass --base <rev>"
                            .to_string()
                    } else {
                        format!(
                            "change a file that `Files` of {dir}/plan.md lists: none differs from the base"
                        )
                    },
                ));
            }
            // Only tasks of a person are open, and no step of an agent is open
            // after the task (for example a file of `Files` to change). Then
            // this line is the last line (design 0.8.0, D64). A step of an
            // agent stays the last line, and so does the approval of a draft
            // plan.
            if approved
                && let Some(task) = next_task.as_ref().filter(|t| t.by_person())
                && only_person_open(ch, head)
                && !items[after_task..].iter().any(|i| !i.done && !i.person)
            {
                items.push(person_item(
                    Step::Execute,
                    format!(
                        "wait for a person: task {} is a task of a person; an agent does not tick it",
                        task.id
                    ),
                ));
            }
            return report(Stage::Build, items, next_task);
        }
        let mut items = done;
        items.push(item(true, Step::WritePlan, "the plan is approved"));
        items.push(item(
            true,
            Step::Execute,
            if no_files {
                NO_FILES_LINE
            } else {
                "a file of `Files` differs from the base"
            },
        ));
        let proof = self.proof(ch);
        if !proof.missing.is_empty() {
            let (m, n, p) = (proof.missing.len(), proof.total, proof.partial.len());
            items.push(item(
                false,
                Step::Test,
                if p == 0 {
                    format!(
                        "{m} of {n} requirement(s) that the change adds or changes have no evidence"
                    )
                } else {
                    format!(
                        "{m} of {n} requirement(s) that the change adds or changes have no full evidence; {p} of them have a partial `covers:` tag only"
                    )
                },
            ));
            for target in proof.missing.iter().take(5) {
                let at = proof.partial.iter().find(|(t, _)| t == target);
                items.push(item(
                    false,
                    Step::Test,
                    match at {
                        Some((_, at)) => format!(
                            "`{target}` has a partial `covers:` tag only ({at}); add a test that covers all of it"
                        ),
                        None => format!("add `covers: {target}` in a test file"),
                    },
                ));
            }
            return report(Stage::Test, items, None);
        }
        items.push(item(
            true,
            Step::Test,
            if proof.total == 0 {
                "the change adds or changes no requirement".to_string()
            } else {
                format!(
                    "each of the {} requirement(s) that the change adds or changes has evidence",
                    proof.total
                )
            },
        ));
        items.push(item(
            false,
            Step::Verify,
            format!("run the proof of {dir}/plan.md: the commands of its `Proof` section"),
        ));
        items.push(item(
            false,
            Step::Review,
            format!("review the diff against {dir}/plan.md"),
        ));
        if let Some(a) = self.view.accept.get(slug) {
            items.push(item(a.done, Step::Accept, a.text.clone()));
        }
        items.push(item(
            false,
            Step::Finish,
            format!(
                "run `spec-dna finish {slug} --dry-run`, read the diff, then run `spec-dna finish {slug}` and commit the result with the code"
            ),
        ));
        report(Stage::Deploy, items, None)
    }
}

/// True if each open task of the plan is a task of a person. A task that
/// closes with `finish`, a duplicate and a closed task are not open.
fn only_person_open(ch: &Change, head: &Corpus) -> bool {
    ch.tasks(head)
        .iter()
        .filter(|t| !t.closed() && !t.duplicate && !t.closes_with_finish(&ch.slug))
        .all(PlanTask::by_person)
}

/// The step of the files of a routine change with a plan that lists no files.
const NO_FILES_LINE: &str = "the plan lists no files: the change edits no file";

/// The text of a task in one line.
fn task_line(t: &PlanTask) -> String {
    let mut s = format!("task {}: {}", t.id, t.text);
    if !t.files.is_empty() {
        let files: Vec<String> = t.files.iter().map(|f| format!("`{f}`")).collect();
        s.push_str(&format!(". Files: {}", files.join(", ")));
    }
    if let Some(v) = &t.verify {
        s.push_str(&format!(". Verify: {v}"));
    }
    s
}

/// A requirement id in a form that two spellings of one document share.
fn same_target(c: &Corpus, target: &str) -> String {
    let Some((id, key)) = target.split_once('#') else {
        return canonical(target);
    };
    match c.resolve(id) {
        Res::Found(d, None, _) => format!("{}#{key}", c.docs[d as usize].id),
        _ => format!("{}#{key}", canonical(id)),
    }
}

/// The area of a delta target: the first folder under the folder of the layer
/// of `requirements`, if the id has one. `requirements/valves/closing` is in
/// the area `valves`, and `requirements/valves` is in no area.
fn target_area(cfg: &Config, id: &str) -> Option<String> {
    let dir = cfg
        .layers
        .layer_of_type(DocType::Requirements)
        .and_then(|i| cfg.layers.layers()[i].dir())
        .unwrap_or("requirements");
    let rest = id.strip_prefix(dir)?.strip_prefix('/')?;
    let (area, tail) = rest.split_once('/')?;
    (!area.is_empty() && !tail.is_empty()).then(|| area.to_string())
}

/// The areas of a change: the `areas` of its intent, then the areas of the
/// targets of its delta. Each area once.
pub fn change_areas(cfg: &Config, ch: &Change, head: &Corpus) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let from_delta = ch
        .delta(head)
        .into_iter()
        .filter_map(|l| l.parsed.ok())
        .filter_map(|d| target_area(cfg, &d.id));
    for a in ch.areas.iter().cloned().chain(from_delta) {
        if !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

/// An open item of `Follow-ups` of an incident.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowUp {
    pub incident: String,
    pub path: String,
    pub line: u32,
    pub text: String,
}

/// The open items of `Follow-ups` (`- [ ]`) of the incidents of a tree (the
/// stage Maintain).
pub fn follow_ups(head: &Corpus) -> Vec<FollowUp> {
    let mut out = Vec::new();
    for d in &head.docs {
        if d.ty != DocType::Incident
            || d.type_failed
            || d.moved_to.is_some()
            || d.generated.is_some()
        {
            continue;
        }
        for i in change::section_items(d, ItemSection::FollowUps) {
            if i.task == Some(false) {
                out.push(FollowUp {
                    incident: d.id.clone(),
                    path: d.path.clone(),
                    line: i.line,
                    text: i.text.split_whitespace().collect::<Vec<_>>().join(" "),
                });
            }
        }
    }
    out
}

/// The skill of an open follow-up of an incident (design 0.7.0, D8). It is
/// not a step of a change, so it is not in `SKILLS`.
const POST_MORTEM: &str = "post-mortem";

const OFF_NOTE: &str = "the change process is off: `[change] policy` is `none`";

/// The short form of a commit.
fn short(rev: &str) -> &str {
    rev.get(..12).unwrap_or(rev)
}

fn task_json(t: &PlanTask) -> String {
    Obj::new()
        .str("id", &t.id)
        .str("text", &t.text)
        .strs("files", &t.files)
        .opt_str("verify", t.verify.as_deref())
        .end()
}

fn opt_task_json(t: Option<&PlanTask>) -> String {
    t.map_or_else(|| "null".to_string(), task_json)
}

/// `spec-dna next <slug>`: the checklist to the next gate of a change.
pub fn next(
    repo: &Repo,
    cfg: &Config,
    slug: &str,
    base: Option<&str>,
    json: bool,
    use_cache: bool,
) -> Result<u8, Fatal> {
    if cfg.change_policy == ChangePolicy::None {
        if json {
            println!(
                "{}",
                Obj::new()
                    .str("slug", slug)
                    .raw("class", "null")
                    .raw("stage", "null")
                    .raw("base", "null")
                    .raw("items", "[]")
                    .raw("next_task", "null")
                    .raw("next", "null")
                    .strs("notes", &[OFF_NOTE.to_string()])
                    .end()
            );
        } else {
            println!("{OFF_NOTE}");
        }
        return Ok(0);
    }
    let view = load(repo, cfg, base, use_cache)?;
    let cx = Ctx::new(&view);
    let ch = cx.changes.iter().find(|c| c.slug == slug).ok_or_else(|| {
        Fatal::NotFound(format!(
            "no folder `<intent folder>/{slug}` for the change `{slug}`"
        ))
    })?;
    let r = cx.derive(ch);
    if json {
        let items = r.items.iter().map(|i| {
            let o = Obj::new()
                .bool("done", i.done)
                .str("text", &i.text)
                .str("skill", i.skill);
            // The field is additive: only an item of a person has it.
            if i.person { o.bool("person", true) } else { o }.end()
        });
        println!(
            "{}",
            Obj::new()
                .str("slug", &ch.slug)
                .opt_str("class", ch.class_shown())
                .str("stage", r.stage.name())
                .opt_str("base", view.base_rev.as_deref())
                .raw("items", &json::array(items))
                .raw("next_task", &opt_task_json(r.next_task.as_ref()))
                .str("next", r.next())
                .strs("notes", &view.notes)
                .end()
        );
        return Ok(0);
    }
    println!(
        "change {} ({}), stage {}",
        ch.slug,
        ch.class_shown().unwrap_or("class not set"),
        r.stage.name()
    );
    match &view.base_rev {
        Some(b) => println!("base {}", short(b)),
        None => println!(
            "base none: {}",
            view.notes
                .first()
                .map_or(lifecycle::NO_BASE_NOTE, String::as_str)
        ),
    }
    for i in &r.items {
        println!(
            "[{}] {} {}",
            if i.done { "x" } else { " " },
            i.text,
            item_tail(i)
        );
    }
    Ok(0)
}

/// The source of the approval of each artifact of a change, in the order
/// intent, spec and plan (design 0.7.0, D8): `base`, or `branch <short sha>`.
/// An artifact whose status is not `accepted` or `approved`, an absent
/// artifact and an approval that no gate counts give `None`. It follows the
/// rule of the last approval of SD910: the base tree first, then the approval
/// commit that set the status.
fn sources_of(view: &View, ch: &Change, walk: Option<&Branch>) -> [Option<String>; 3] {
    let base_changes = view.base.as_ref().map(change::collect);
    let base_ch = base_changes
        .as_ref()
        .and_then(|b| b.iter().find(|b| b.dir == ch.dir));
    let one = |a: &Option<change::Artifact>, base: Option<&change::Artifact>| {
        let a = a.as_ref()?;
        if !matches!(a.status.as_str(), "accepted" | "approved") {
            return None;
        }
        if base.is_some_and(|b| b.status == a.status) {
            return Some("base".to_string());
        }
        let w = walk.filter(|w| w.stopped().is_none())?;
        let path = &view.head.docs[a.doc as usize].path;
        match w.last(path, &a.status) {
            Ok(Some(ap)) => Some(format!("branch {}", short(&ap.commit))),
            _ => None,
        }
    };
    [
        one(&ch.intent, base_ch.and_then(|b| b.intent.as_ref())),
        one(&ch.spec, base_ch.and_then(|b| b.spec.as_ref())),
        one(&ch.plan, base_ch.and_then(|b| b.plan.as_ref())),
    ]
}

/// The object `approved_in` of `status --json`.
fn approved_json(s: &[Option<String>; 3]) -> String {
    Obj::new()
        .opt_str("intent", s[0].as_deref())
        .opt_str("spec", s[1].as_deref())
        .opt_str("plan", s[2].as_deref())
        .end()
}

/// `spec-dna status [--group <name>]`: each change with its class, stage, next
/// gate and task progress, and the open follow-ups of the incidents. `group` is
/// the list of the areas of a group: a change that has none of them is not
/// listed.
pub fn status(
    repo: &Repo,
    cfg: &Config,
    group: Option<&[String]>,
    base: Option<&str>,
    json: bool,
    use_cache: bool,
) -> Result<u8, Fatal> {
    let max_started = cfg
        .layers
        .layer_of_type(DocType::Intent)
        .and_then(|i| cfg.layers.layers()[i].max_started);
    if cfg.change_policy == ChangePolicy::None {
        if json {
            println!(
                "{}",
                Obj::new()
                    .str("version", crate::adapters::VERSION)
                    .str("policy", cfg.change_policy.name())
                    .raw("base", "null")
                    .num("started", 0)
                    .raw("max_started", "null")
                    .strs("notes", &[OFF_NOTE.to_string()])
                    .raw("changes", "[]")
                    .raw("follow_ups", "[]")
                    .end()
            );
        } else {
            println!("{OFF_NOTE}; no change is listed");
        }
        return Ok(0);
    }
    let view = load(repo, cfg, base, use_cache)?;
    let cx = Ctx::new(&view);
    let stale: HashSet<String> = match &view.base_rev {
        Some(b) => lifecycle::stale_changes(cfg, &repo.git, &view.head, b)
            .map_err(Fatal::Io)?
            .into_iter()
            .map(|s| s.slug)
            .collect(),
        None => HashSet::new(),
    };
    let started = lifecycle::started_count(&view.head);
    let follow = follow_ups(&view.head);
    // The proof debts of the working tree (design 0.8.0, D2). `status` reads no
    // report, so it counts the debts only.
    let proof = evidence::ProofCounts::of(
        &view.head,
        &Evidence::build(&view.head).0,
        cfg.evidence_reliable_runs as usize,
    );
    let walk = match (&view.base, &view.base_rev) {
        (Some(base), Some(rev)) if view.branch => {
            Some(Branch::read(cfg, &repo.git, rev, base, &view.head))
        }
        _ => None,
    };
    struct Row<'a> {
        ch: &'a Change,
        report: Report,
        tasks: Tasks,
        partial: usize,
        areas: Vec<String>,
        stale: bool,
        sources: [Option<String>; 3],
    }
    let rows: Vec<Row> = cx
        .changes
        .iter()
        .filter_map(|ch| {
            let areas = change_areas(cfg, ch, &view.head);
            if let Some(g) = group
                && !areas.iter().any(|a| g.contains(a))
            {
                return None;
            }
            Some(Row {
                ch,
                report: cx.derive(ch),
                tasks: tasks(ch, &view.head),
                partial: cx.partial(ch),
                areas,
                stale: stale.contains(&ch.slug),
                sources: sources_of(&view, ch, walk.as_ref()),
            })
        })
        .collect();
    let state = |a: &Option<crate::change::Artifact>| -> Option<String> {
        a.as_ref().map(|a| a.status.clone())
    };
    if json {
        let changes = rows.iter().map(|r| {
            let title =
                r.ch.intent
                    .as_ref()
                    .map(|a| view.head.docs[a.doc as usize].parsed.title.as_str())
                    .filter(|t| !t.is_empty())
                    .unwrap_or(&r.ch.slug);
            let statuses = Obj::new()
                .opt_str("intent", state(&r.ch.intent).as_deref())
                .opt_str("spec", state(&r.ch.spec).as_deref())
                .opt_str("plan", state(&r.ch.plan).as_deref())
                .end();
            let tasks = Obj::new()
                .num("done", r.tasks.done)
                .num("skipped", r.tasks.skipped)
                .num("finish", r.tasks.finish)
                .num("total", r.tasks.total)
                .raw("next_task", &opt_task_json(r.tasks.next.as_ref()))
                .end();
            Obj::new()
                .str("slug", &r.ch.slug)
                .str("path", &r.ch.dir)
                .str("title", title)
                .opt_str("class", r.ch.class_shown())
                .raw("statuses", &statuses)
                .raw("approved_in", &approved_json(&r.sources))
                .str("stage", r.report.stage.name())
                .str("next", r.report.next())
                .bool("stale", r.stale)
                .strs("areas", &r.areas)
                .raw("tasks", &tasks)
                .num("partial", r.partial)
                .end()
        });
        let follow_ups = follow.iter().map(|f| {
            Obj::new()
                .str("incident", &f.incident)
                .str("path", &f.path)
                .num("line", f.line)
                .str("text", &f.text)
                .str("skill", POST_MORTEM)
                .end()
        });
        let mut out = Obj::new()
            .str("version", crate::adapters::VERSION)
            .str("policy", cfg.change_policy.name())
            .opt_str("base", view.base_rev.as_deref())
            .num("started", started)
            .raw(
                "max_started",
                &max_started.map_or_else(|| "null".to_string(), |m| m.to_string()),
            )
            .strs("notes", &view.notes)
            .raw("changes", &json::array(changes))
            .raw("follow_ups", &json::array(follow_ups));
        // The object `proof` is there only when a marker exists, so a project
        // that uses no proof debt gets the JSON of 0.7.1.
        if proof.marked > 0 {
            out = out.raw("proof", &proof.owed_json());
        }
        println!("{}", out.end());
        return Ok(0);
    }
    println!(
        "policy {}, base {}, {} started{}",
        cfg.change_policy.name(),
        view.base_rev.as_deref().map_or("none", short),
        started,
        max_started.map_or(String::new(), |m| format!(" (max {m})"))
    );
    for n in &view.notes {
        println!("note: {n}");
    }
    if rows.is_empty() {
        println!(
            "no change{}",
            if group.is_some() { " in the group" } else { "" }
        );
    }
    for r in &rows {
        // With the branch rule, a status that an approval gives has its source.
        let st = |a: &Option<crate::change::Artifact>, i: usize| {
            a.as_ref().map_or("-".to_string(), |a| {
                if a.status.is_empty() {
                    "none".to_string()
                } else if view.branch
                    && let Some(src) = &r.sources[i]
                {
                    format!("{} ({src})", a.status)
                } else {
                    a.status.clone()
                }
            })
        };
        let mut line = format!(
            "{} {} {} intent {} spec {} plan {}",
            r.ch.slug,
            r.ch.class_shown().unwrap_or("none"),
            r.report.stage.name(),
            st(&r.ch.intent, 0),
            st(&r.ch.spec, 1),
            st(&r.ch.plan, 2)
        );
        if r.tasks.total > 0 {
            line.push_str(&format!(" tasks {}/{}", r.tasks.done, r.tasks.total));
            if r.tasks.skipped > 0 {
                line.push_str(&format!(" skipped {}", r.tasks.skipped));
            }
            if r.tasks.finish > 0 {
                line.push_str(&format!(" finish {}", r.tasks.finish));
            }
            if let Some(t) = &r.tasks.next {
                line.push_str(&format!(" next-task {}", t.id));
                if t.by_person() {
                    line.push_str(" [person]");
                }
            }
        }
        if r.partial > 0 {
            line.push_str(&format!(" partial {}", r.partial));
        }
        if !r.areas.is_empty() {
            line.push_str(&format!(" areas {}", r.areas.join(",")));
        }
        if r.stale {
            line.push_str(" stale");
        }
        println!("{line}");
        let person = r.report.items.last().is_some_and(|i| i.person);
        println!(
            "  next: {}{}",
            r.report.next(),
            if person { " [person]" } else { "" }
        );
    }
    for f in &follow {
        println!(
            "follow-up {}: {}:{} {} [skill: {POST_MORTEM}]",
            f.incident, f.path, f.line, f.text
        );
    }
    if evidence::proof_codes_on(cfg)
        && let Some(line) = proof.owed_line()
    {
        println!("{line}");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skill_names_are_fixed() {
        assert_eq!(
            SKILLS,
            [
                "brainstorm",
                "specify",
                "clarify",
                "write-plan",
                "execute",
                "tdd",
                "review",
                "verify",
                "accept",
                "finish"
            ]
        );
    }

    #[test]
    fn each_step_has_one_skill_of_the_list_and_each_skill_has_a_step() {
        for (step, name) in STEP_SKILL {
            assert!(SKILLS.contains(&name), "{step:?}: `{name}`");
            assert_eq!(skill(step), name);
        }
        for name in SKILLS {
            assert_eq!(
                STEP_SKILL.iter().filter(|(_, n)| *n == name).count(),
                1,
                "`{name}`"
            );
        }
        // The mapping of the design: the stage, then the skill.
        let want = [
            (Step::Plan, "brainstorm"),
            (Step::Specify, "specify"),
            (Step::Clarify, "clarify"),
            (Step::WritePlan, "write-plan"),
            (Step::Execute, "execute"),
            (Step::Test, "tdd"),
            (Step::Verify, "verify"),
            (Step::Review, "review"),
            (Step::Accept, "accept"),
            (Step::Finish, "finish"),
        ];
        for (step, name) in want {
            assert_eq!(skill(step), name, "{step:?}");
        }
    }

    #[test]
    fn the_limit_keeps_the_last_item_and_drops_done_items_first() {
        let mk = |n: usize, done: bool| item(done, Step::Test, format!("line {n}"));
        let mut v: Vec<Item> = (0..4).map(|n| mk(n, true)).collect();
        v.extend((4..14).map(|n| mk(n, false)));
        assert_eq!(v.len(), 14);
        let out = limit(v);
        assert_eq!(out.len(), MAX_ITEMS);
        assert_eq!(out.last().unwrap().text, "line 13");
        assert!(out.iter().all(|i| !i.done));
        // Only open items: the item before the last one goes.
        let v: Vec<Item> = (0..12).map(|n| mk(n, false)).collect();
        let out = limit(v);
        assert_eq!(out.len(), MAX_ITEMS);
        assert_eq!(out.last().unwrap().text, "line 11");
        assert_eq!(out[0].text, "line 0");
    }
}
