//! The stage of a change and the commands `next` and `status` (design 0.6.0,
//! "Stage derivation"). A change has no stage field. This module derives the
//! stage from the artifacts, their statuses, the diff against the lifecycle base
//! and the evidence of the requirements. One function (`Ctx::derive`) gives the
//! stage, the checklist and the next task. `next` and `status` both print its
//! result, so they cannot disagree.
//!
//! The stage `Maintain` is not the stage of a change: an incident with an open
//! item in `Follow-ups` is a follow-up, and `status` lists it.

use crate::change::{self, Change, PlanTask, Verb};
use crate::config::{ChangePolicy, Config};
use crate::evidence::{self, Evidence};
use crate::json::{self, Obj};
use crate::lifecycle::{self, Diff, ReqChangeKind};
use crate::model::{Class, Corpus, DocType, Res, canonical};
use crate::parse::ItemSection;
use crate::repo::{self, Fatal, Repo, Snapshot};
use std::cell::OnceCell;
use std::collections::HashSet;

/// The names of the skills that a step of the lifecycle uses, in this order.
/// The skills themselves come in 0.7.0. The names are fixed: `next` prints
/// them as data, and a later version writes the skills under these names.
pub const SKILLS: [&str; 9] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "execute",
    "tdd",
    "review",
    "verify",
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
    /// A closed change, or a change that is ready to apply its delta.
    Finish,
}

/// The skill of each step. A test checks that each skill of `SKILLS` is here
/// once.
pub const STEP_SKILL: [(Step, &str); 9] = [
    (Step::Plan, "brainstorm"),
    (Step::Specify, "specify"),
    (Step::Clarify, "clarify"),
    (Step::WritePlan, "write-plan"),
    (Step::Execute, "execute"),
    (Step::Test, "tdd"),
    (Step::Review, "review"),
    (Step::Verify, "verify"),
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
}

fn item(done: bool, step: Step, text: impl Into<String>) -> Item {
    Item {
        done,
        text: text.into(),
        skill: skill(step),
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
    /// The tasks with `- [x]`. A duplicate id does not count.
    pub done: usize,
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
        done: all.iter().filter(|t| t.done).count(),
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
        notes.push(lifecycle::NO_BASE_NOTE.to_string());
    }
    Ok(View {
        head,
        base: base_corpus,
        base_rev,
        diff,
        notes,
    })
}

/// The evidence of the requirements that the change adds or changes.
struct Proof {
    /// The number of such requirements.
    total: usize,
    /// The targets `<id>#<key>` that have no evidence.
    missing: Vec<String>,
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
        let missing = targets
            .iter()
            .filter(|(id, key)| !self.has_evidence(id, key))
            .map(|(id, key)| format!("{id}#{key}"))
            .collect();
        Proof {
            total: targets.len(),
            missing,
        }
    }

    /// True if a requirement has evidence: a `covers:` tag, a `verified-by`
    /// marker, or the `verified-by` of its document. A requirement that the
    /// tree does not have yet (an `ADD` line) has evidence if a `covers:` tag
    /// names it.
    fn has_evidence(&self, id: &str, key: &str) -> bool {
        let head = &self.view.head;
        let ev = self.evidence.get_or_init(|| Evidence::build(head).0);
        if let Res::Found(d, None, _) = head.resolve(id) {
            let doc = &head.docs[d as usize];
            if let Some(r) = doc.req(key) {
                return !ev.of(d, doc, r).is_empty();
            }
        }
        let want = same_target(head, &format!("{id}#{key}"));
        head.covers
            .iter()
            .any(|c| same_target(head, &c.target) == want)
    }

    /// The items for the open concerns (SD919) and the open questions (SD922)
    /// of the spec. A change with no spec has none.
    fn clarify_items(head: &Corpus, ch: &Change) -> Vec<Item> {
        let Some(a) = &ch.spec else {
            return Vec::new();
        };
        let dir = &ch.dir;
        let concerns = change::section_items(&head.docs[a.doc as usize], ItemSection::Concerns)
            .filter(|i| i.task == Some(false))
            .count();
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
        if intent != "accepted" {
            let mut items = Self::clarify_items(head, ch);
            items.push(if ch.intent.is_none() {
                item(
                    false,
                    Step::Plan,
                    format!(
                        "write `{dir}/intent.md` with `spec-dna new intent {slug}`, then set `status: accepted` in it"
                    ),
                )
            } else {
                item(
                    false,
                    Step::Plan,
                    format!("set `status: accepted` in {dir}/intent.md"),
                )
            });
            return report(Stage::Plan, items, None);
        }
        let intent_done = item(true, Step::Plan, "the intent is accepted");
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
            items.push(item(
                false,
                Step::Specify,
                format!("set `status: approved` in {dir}/spec.md"),
            ));
            return report(Stage::Design, items, None);
        }
        let mut done = vec![intent_done];
        if higher {
            done.push(item(true, Step::Specify, "the spec is approved"));
        }
        let changed = self.files_changed(ch);
        if plan != "approved" || changed != Some(true) {
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
            }
            // The tasks of the plan, also if the plan is a draft.
            let t = tasks(ch, head);
            if t.total > 0 {
                items.push(item(
                    t.done == t.total,
                    Step::Execute,
                    format!("tasks of `Order`: {} of {} done", t.done, t.total),
                ));
            }
            if let Some(task) = &t.next {
                items.push(item(false, Step::Execute, task_line(task)));
                next_task = Some(task.clone());
            }
            if !approved {
                items.push(item(
                    false,
                    Step::WritePlan,
                    format!("set `status: approved` in {dir}/plan.md"),
                ));
            } else {
                let files = ch.files(head);
                items.push(item(
                    false,
                    Step::Execute,
                    if files.is_empty() {
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
            return report(Stage::Build, items, next_task);
        }
        let mut items = done;
        items.push(item(true, Step::WritePlan, "the plan is approved"));
        items.push(item(
            true,
            Step::Execute,
            "a file of `Files` differs from the base",
        ));
        let proof = self.proof(ch);
        if !proof.missing.is_empty() {
            items.push(item(
                false,
                Step::Test,
                format!(
                    "{} of {} requirement(s) that the change adds or changes have no evidence",
                    proof.missing.len(),
                    proof.total
                ),
            ));
            for target in proof.missing.iter().take(5) {
                items.push(item(
                    false,
                    Step::Test,
                    format!("add `covers: {target}` in a test file"),
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
            Obj::new()
                .bool("done", i.done)
                .str("text", &i.text)
                .str("skill", i.skill)
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .str("slug", &ch.slug)
                .str("class", ch.class.name())
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
        ch.class.name(),
        r.stage.name()
    );
    match &view.base_rev {
        Some(b) => println!("base {}", short(b)),
        None => println!("base none: {}", lifecycle::NO_BASE_NOTE),
    }
    for i in &r.items {
        println!(
            "[{}] {} [skill: {}]",
            if i.done { "x" } else { " " },
            i.text,
            i.skill
        );
    }
    Ok(0)
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
    struct Row<'a> {
        ch: &'a Change,
        report: Report,
        tasks: Tasks,
        areas: Vec<String>,
        stale: bool,
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
                areas,
                stale: stale.contains(&ch.slug),
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
                .num("total", r.tasks.total)
                .raw("next_task", &opt_task_json(r.tasks.next.as_ref()))
                .end();
            Obj::new()
                .str("slug", &r.ch.slug)
                .str("path", &r.ch.dir)
                .str("title", title)
                .str("class", r.ch.class.name())
                .raw("statuses", &statuses)
                .str("stage", r.report.stage.name())
                .str("next", r.report.next())
                .bool("stale", r.stale)
                .strs("areas", &r.areas)
                .raw("tasks", &tasks)
                .end()
        });
        let follow_ups = follow.iter().map(|f| {
            Obj::new()
                .str("incident", &f.incident)
                .str("path", &f.path)
                .num("line", f.line)
                .str("text", &f.text)
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .str("version", crate::adapters::VERSION)
                .str("policy", cfg.change_policy.name())
                .opt_str("base", view.base_rev.as_deref())
                .num("started", started)
                .raw(
                    "max_started",
                    &max_started.map_or_else(|| "null".to_string(), |m| m.to_string())
                )
                .strs("notes", &view.notes)
                .raw("changes", &json::array(changes))
                .raw("follow_ups", &json::array(follow_ups))
                .end()
        );
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
        let st = |a: &Option<crate::change::Artifact>| {
            a.as_ref().map_or("-".to_string(), |a| {
                if a.status.is_empty() {
                    "none".to_string()
                } else {
                    a.status.clone()
                }
            })
        };
        let mut line = format!(
            "{} {} {} intent {} spec {} plan {}",
            r.ch.slug,
            r.ch.class.name(),
            r.report.stage.name(),
            st(&r.ch.intent),
            st(&r.ch.spec),
            st(&r.ch.plan)
        );
        if r.tasks.total > 0 {
            line.push_str(&format!(" tasks {}/{}", r.tasks.done, r.tasks.total));
            if let Some(t) = &r.tasks.next {
                line.push_str(&format!(" next-task {}", t.id));
            }
        }
        if !r.areas.is_empty() {
            line.push_str(&format!(" areas {}", r.areas.join(",")));
        }
        if r.stale {
            line.push_str(" stale");
        }
        println!("{line}");
        println!("  next: {}", r.report.next());
    }
    for f in &follow {
        println!("follow-up {}: {}:{} {}", f.incident, f.path, f.line, f.text);
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
