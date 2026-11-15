//! The commands `log` and `stats --sdlc` (design 0.6.0, "Log and history" and
//! "Indicators"). Both read the history of git and the committed files. They
//! never read the working tree and never read a clock.
//!
//! A finished change leaves no folder, so the history is the only record of
//! it. `log` lists each commit that deleted an `intent.md`. The status of the
//! intent in the parent tree of that commit gives the result: `accepted` is
//! `finished`, and any other status is `closed`. `stats --sdlc` rebuilds the
//! life of each change from the commits that touched its 3 files, and gives the
//! indicators of the playbook that git can give. One function (`Outcome::of`)
//! decides the result for both commands. Both commands use the same rename
//! detection (`-M`), so a rename of a folder inside the intent folder is not a
//! deletion for either command.
//!
//! The 2 Test indicators do not come from the lives of the changes. They read
//! the tree of `HEAD`: the result files in `<[evals] dir>/results`, and the
//! incidents with the evals that they name (`test_rows`).

use crate::change::{self, Verb};
use crate::config::{ChangePolicy, Config, Evals};
use crate::evals;
use crate::frontmatter::{self, Value};
use crate::git::{ArtifactTouch, Git, PathChange};
use crate::json::{self, Obj};
use crate::lifecycle;
use crate::model::{Class, Corpus, DocType};
use crate::parse::{ItemSection, ListItem, parse};
use crate::registry;
use crate::repo::{self, Fatal, Repo, Snapshot};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

const OFF_NOTE: &str = "the change process is off: `[change] policy` is `none`";
const SHALLOW_NOTE: &str = "this is a shallow clone; older changes may be missing";

/// The result of a change that left the default branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Finished,
    Closed,
}

impl Outcome {
    /// The result of a deleted intent, from its status in the parent tree. A
    /// status other than `accepted` is `closed`, also `draft`.
    fn of(parent_status: &str) -> Outcome {
        if parent_status == "accepted" {
            Outcome::Finished
        } else {
            Outcome::Closed
        }
    }

    fn name(self) -> &'static str {
        match self {
            Outcome::Finished => "finished",
            Outcome::Closed => "closed",
        }
    }
}

/// The status in the front matter of a document, or an empty string.
fn status_of(text: &str) -> String {
    frontmatter::split(text)
        .and_then(|(fm, _)| frontmatter::parse(fm, 2).ok())
        .and_then(|f| match f.get("status") {
            Some((Value::Str(s), _)) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The 3 files of a change in one commit.
#[derive(Default)]
struct Tree {
    intent: Option<String>,
    spec: Option<String>,
    plan: Option<String>,
}

/// Reads the folders `(commit, folder)` with one git process.
fn read_trees(git: &Git, at: &[(String, String)]) -> Result<Vec<Tree>, Fatal> {
    let mut specs = Vec::new();
    for (rev, folder) in at {
        for name in ["intent.md", "spec.md", "plan.md"] {
            specs.push((rev.clone(), format!("{folder}/{name}")));
        }
    }
    let mut blobs = git.read_at(&specs).map_err(Fatal::Io)?.into_iter();
    let mut text = || {
        blobs
            .next()
            .flatten()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    Ok(at
        .iter()
        .map(|_| Tree {
            intent: text(),
            spec: text(),
            plan: text(),
        })
        .collect())
}

/// The facts of a change that its last tree gives.
struct Facts {
    slug: String,
    title: String,
    class: Class,
    areas: Vec<String>,
    /// The counts of the valid delta lines: `ADD`, `CHANGE`, `REMOVE`.
    delta: [usize; 3],
    /// The status of the intent, or an empty string.
    status: String,
    /// The `Files` patterns of the plan and the paths of the label `Files:` of
    /// its tasks, or `None` if there is no plan.
    files: Option<Vec<String>>,
    /// The paths that the delta and `Interfaces` name, for each root.
    named: Vec<String>,
}

/// The paths that a document id can have: one for each root.
fn named_paths(cfg: &Config, id: &str) -> Vec<String> {
    cfg.roots
        .iter()
        .map(|r| {
            if r.path.is_empty() {
                format!("{id}.md")
            } else {
                format!("{}/{id}.md", r.path)
            }
        })
        .collect()
}

fn facts(cfg: &Config, folder: &str, tree: &Tree) -> Facts {
    let slug = folder.rsplit('/').next().unwrap_or(folder).to_string();
    let docs: Vec<(String, Arc<crate::parse::ParsedDoc>)> = [
        ("intent.md", &tree.intent),
        ("spec.md", &tree.spec),
        ("plan.md", &tree.plan),
    ]
    .into_iter()
    .filter_map(|(name, text)| {
        text.as_ref()
            .map(|t| (format!("{folder}/{name}"), Arc::new(parse(t))))
    })
    .collect();
    let corpus = Corpus::build(cfg, docs.iter().map(|d| d.0.clone()).collect(), docs);
    let change = change::collect(&corpus)
        .into_iter()
        .find(|c| c.dir == folder);
    let mut out = Facts {
        title: slug.clone(),
        slug,
        class: Class::Routine,
        areas: Vec::new(),
        delta: [0; 3],
        status: tree.intent.as_deref().map(status_of).unwrap_or_default(),
        files: tree.plan.as_ref().map(|_| Vec::new()),
        named: Vec::new(),
    };
    let Some(ch) = change else { return out };
    if let Some(a) = &ch.intent {
        let t = &corpus.docs[a.doc as usize].parsed.title;
        if !t.is_empty() {
            out.title = t.clone();
        }
    }
    out.class = ch.class;
    out.areas = crate::stage::change_areas(cfg, &ch, &corpus);
    for line in ch.delta(&corpus) {
        let Ok(d) = line.parsed else { continue };
        out.delta[match d.verb {
            Verb::Add => 0,
            Verb::Change => 1,
            Verb::Remove => 2,
        }] += 1;
        out.named.extend(named_paths(cfg, &d.id));
    }
    if let Some(a) = &ch.spec {
        let doc = &corpus.docs[a.doc as usize];
        for item in change::section_items(doc, ItemSection::Interfaces) {
            if let Some(id) = interface_id(item) {
                out.named.extend(named_paths(cfg, id));
            }
        }
    }
    if ch.plan.is_some() {
        out.files = Some(ch.files(&corpus));
    }
    out
}

/// The id of an item of `Interfaces`: `CHANGE <interface id>: <text>`. The id
/// can be in `[[ ]]` or in a code span. This is a copy of `interface_id` in
/// `lifecycle.rs`, which is private there. Keep the two the same.
fn interface_id(item: &ListItem) -> Option<&str> {
    let rest = item.text.strip_prefix("CHANGE")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (id, text) = rest.split_once(':')?;
    let id = id.trim().trim_matches(['[', ']', '`']);
    (!id.is_empty() && !id.contains(char::is_whitespace) && !text.trim().is_empty()).then_some(id)
}

/// The date of a time in seconds, as `YYYY-MM-DD` in UTC.
fn utc_date(secs: i64) -> String {
    // "civil_from_days" (Howard Hinnant).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn short(rev: &str) -> &str {
    rev.get(..12).unwrap_or(rev)
}

fn off(json: bool, empty: &str) -> u8 {
    if json {
        println!("{empty}");
    } else {
        println!("{OFF_NOTE}; no change is listed");
    }
    0
}

// log

struct Entry {
    facts: Facts,
    outcome: Outcome,
    commit: String,
    time: i64,
}

/// `spec-dna log [--since <rev>]`: the finished and the closed changes.
pub fn log(repo: &Repo, cfg: &Config, since: Option<&str>, json: bool) -> Result<u8, Fatal> {
    let git = &repo.git;
    let shallow = git.is_shallow();
    if cfg.change_policy == ChangePolicy::None {
        return Ok(off(
            json,
            &Obj::new()
                .raw("changes", "[]")
                .bool("shallow", shallow)
                .end(),
        ));
    }
    if let Some(rev) = since
        && git.commit(rev).is_none()
    {
        return Err(Fatal::Config(format!(
            "`--since {rev}` is not a commit here; fetch it first, or give another revision"
        )));
    }
    let mut raw = Vec::new();
    if repo.head_commit().is_some() {
        for dir in lifecycle::intent_dirs(cfg) {
            raw.extend(git.deleted_intents(&dir, since).map_err(Fatal::Io)?);
        }
    }
    raw.sort_by_key(|e| std::cmp::Reverse(e.time));
    let mut at = Vec::new();
    let mut kept = Vec::new();
    for e in &raw {
        let (Some(parent), Some(folder)) = (&e.parent, e.path.strip_suffix("/intent.md")) else {
            continue;
        };
        at.push((parent.clone(), folder.to_string()));
        kept.push(e);
    }
    let trees = read_trees(git, &at)?;
    let entries: Vec<Entry> = kept
        .iter()
        .zip(&at)
        .zip(&trees)
        .map(|((e, (_, folder)), tree)| {
            let facts = facts(cfg, folder, tree);
            Entry {
                outcome: Outcome::of(&facts.status),
                facts,
                commit: e.commit.clone(),
                time: e.time,
            }
        })
        .collect();
    if json {
        let changes = entries.iter().map(|e| {
            let delta = Obj::new()
                .num("add", e.facts.delta[0])
                .num("change", e.facts.delta[1])
                .num("remove", e.facts.delta[2])
                .end();
            Obj::new()
                .str("slug", &e.facts.slug)
                .str("class", e.facts.class.name())
                .str("title", &e.facts.title)
                .str("result", e.outcome.name())
                .raw("delta", &delta)
                .str("commit", &e.commit)
                .str("date", &utc_date(e.time))
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .raw("changes", &json::array(changes))
                .bool("shallow", shallow)
                .end()
        );
        return Ok(0);
    }
    if entries.is_empty() {
        println!("no finished or closed change in the history");
    }
    for e in &entries {
        println!(
            "{} {} {} {} add {} change {} remove {} {} {}",
            utc_date(e.time),
            e.outcome.name(),
            e.facts.slug,
            e.facts.class.name(),
            e.facts.delta[0],
            e.facts.delta[1],
            e.facts.delta[2],
            short(&e.commit),
            e.facts.title
        );
    }
    if shallow {
        println!("note: {SHALLOW_NOTE}");
    }
    Ok(0)
}

// stats

/// The end of a life: the commit that deleted its `intent.md`.
struct End {
    commit: String,
    parent: Option<String>,
    time: i64,
}

/// One life of a change folder in the history: from its first commit to the
/// commit that deleted its `intent.md`, or to `HEAD`. A folder name that
/// appears again after a deletion is a new life.
#[derive(Default)]
struct Life {
    folder: String,
    /// The time of the first commit that has a file of the life.
    first: Option<i64>,
    intent_first: Option<i64>,
    accepted_at: Option<i64>,
    spec_first: Option<i64>,
    /// The commits of the spec after the first commit of the plan.
    spec_after_plan: usize,
    plan_seen: bool,
    plan_approved_at: Option<i64>,
    last_status: String,
    end: Option<End>,
}

/// The touch, unless it deletes the file.
fn not_deleted(t: Option<&ArtifactTouch>) -> Option<&ArtifactTouch> {
    t.filter(|t| t.kind != PathChange::Deleted)
}

fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// Rebuilds the lives of the change folders under `dirs`. The commits go from
/// the oldest to the newest in the order of the ancestry, not of the dates.
fn lives(git: &Git, dirs: &[String]) -> Result<Vec<Life>, Fatal> {
    let mut touches = Vec::new();
    for d in dirs {
        touches.extend(git.artifact_touches(d).map_err(Fatal::Io)?);
    }
    // The status of each intent and each plan that a commit wrote.
    let wanted: Vec<(String, String)> = touches
        .iter()
        .filter(|t| {
            t.kind != PathChange::Deleted
                && (t.path.ends_with("/intent.md") || t.path.ends_with("/plan.md"))
        })
        .map(|t| (t.commit.clone(), t.path.clone()))
        .collect();
    let texts = git.read_at(&wanted).map_err(Fatal::Io)?;
    let status: HashMap<(String, String), String> = wanted
        .into_iter()
        .zip(texts)
        .map(|(k, t)| {
            (
                k,
                t.map(|b| status_of(&String::from_utf8_lossy(&b)))
                    .unwrap_or_default(),
            )
        })
        .collect();
    // A rename of a folder inside the intent folder keeps the life: a touch
    // of the old folder, older than the rename, belongs to the new folder. The
    // code reads the touches from the newest to the oldest, so it sets the
    // alias before the older touches.
    let mut alias: HashMap<&str, String> = HashMap::new();
    let mut by_folder: BTreeMap<String, Vec<&ArtifactTouch>> = BTreeMap::new();
    for t in &touches {
        let folder = folder_of(&t.path);
        let canon = alias.get(folder).cloned().unwrap_or(folder.to_string());
        if let Some(old) = &t.from {
            alias.insert(folder_of(old), canon.clone());
        }
        by_folder.entry(canon).or_default().push(t);
    }
    for list in by_folder.values_mut() {
        list.reverse();
    }
    let mut out = Vec::new();
    for (folder, ts) in by_folder {
        let new = || Life {
            folder: folder.clone(),
            ..Life::default()
        };
        let mut cur = new();
        let mut i = 0;
        while i < ts.len() {
            let mut j = i;
            while j < ts.len() && ts[j].commit == ts[i].commit {
                j += 1;
            }
            let group = &ts[i..j];
            i = j;
            let time = group[0].time;
            let find = |name: &str| group.iter().find(|t| t.path.ends_with(name)).copied();
            let (intent, spec, plan) = (find("/intent.md"), find("/spec.md"), find("/plan.md"));
            let state = |t: &ArtifactTouch| {
                status
                    .get(&(t.commit.clone(), t.path.clone()))
                    .cloned()
                    .unwrap_or_default()
            };
            if let Some(t) = not_deleted(intent) {
                cur.first.get_or_insert(time);
                cur.intent_first.get_or_insert(time);
                let s = state(t);
                if s == "accepted" {
                    cur.accepted_at.get_or_insert(time);
                }
                cur.last_status = s;
            }
            if not_deleted(spec).is_some() {
                cur.first.get_or_insert(time);
                cur.spec_first.get_or_insert(time);
                if cur.plan_seen {
                    cur.spec_after_plan += 1;
                }
            }
            if let Some(t) = not_deleted(plan) {
                cur.first.get_or_insert(time);
                cur.plan_seen = true;
                if state(t) == "approved" {
                    cur.plan_approved_at.get_or_insert(time);
                }
            }
            if let Some(t) = intent
                && t.kind == PathChange::Deleted
            {
                cur.end = Some(End {
                    commit: t.commit.clone(),
                    parent: t.parents.first().cloned(),
                    time,
                });
                let done = std::mem::replace(&mut cur, new());
                if done.first.is_some() {
                    out.push(done);
                }
            }
        }
        if cur.first.is_some() {
            out.push(cur);
        }
    }
    Ok(out)
}

/// A life, with the facts of its last tree.
struct Measured {
    life: Life,
    facts: Facts,
    outcome: Option<Outcome>,
    /// The commit that reached the default branch (the first-parent commit of
    /// the deletion), its first parent and its time. If there is no such commit, it is the
    /// deleting commit.
    merged: Option<(String, Option<String>, i64)>,
}

/// One row of the indicator table.
struct Indicator {
    play: &'static str,
    kind: &'static str,
    name: &'static str,
    value: Option<f64>,
    unit: Option<&'static str>,
    count: Option<usize>,
    /// The word of the count in the text line, for 1 and for any other number.
    noun: (&'static str, &'static str),
    reason: Option<String>,
}

const CHANGES: (&str, &str) = ("change", "changes");

impl Indicator {
    /// Sets the word that the text line uses for the count.
    fn counting(mut self, noun: (&'static str, &'static str)) -> Self {
        self.noun = noun;
        self
    }
}

fn na(play: &'static str, kind: &'static str, name: &'static str, reason: &str) -> Indicator {
    Indicator {
        play,
        kind,
        name,
        value: None,
        unit: None,
        count: None,
        noun: CHANGES,
        reason: Some(reason.to_string()),
    }
}

/// An indicator that the history gives. With no data, the value is absent and
/// the reason says what is missing.
fn computed(
    (play, kind, name): (&'static str, &'static str, &'static str),
    value: Option<f64>,
    unit: &'static str,
    count: usize,
    note: &str,
    empty: &str,
    shallow: bool,
) -> Indicator {
    let mut reason = if value.is_some() {
        (!note.is_empty()).then(|| note.to_string())
    } else {
        Some(empty.to_string())
    };
    if shallow {
        let r = reason.get_or_insert_with(String::new);
        if !r.is_empty() {
            r.push_str("; ");
        }
        r.push_str(SHALLOW_NOTE);
    }
    Indicator {
        play,
        kind,
        name,
        value,
        unit: Some(unit),
        count: Some(count),
        noun: CHANGES,
        reason,
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

const DAY: f64 = 86_400.0;

/// The days between 2 times. A negative span (a history whose dates are not in
/// order) is not a measure.
fn days(from: i64, to: i64) -> Option<f64> {
    (to >= from).then(|| (to - from) as f64 / DAY)
}

/// `spec-dna stats --sdlc [--group <name>]`.
pub fn stats(
    repo: &Repo,
    cfg: &Config,
    group: Option<(&str, &[String])>,
    json: bool,
) -> Result<u8, Fatal> {
    if cfg.change_policy == ChangePolicy::None {
        return Ok(off(json, &Obj::new().raw("indicators", "[]").end()));
    }
    let git = &repo.git;
    let shallow = git.is_shallow();
    let head = repo.head_commit();
    let dirs = lifecycle::intent_dirs(cfg);
    let all = match &head {
        Some(_) => lives(git, &dirs)?,
        None => Vec::new(),
    };
    // The commit that reached the default branch for each deleted folder.
    let mut first_parent: HashMap<String, Vec<(String, Option<String>, i64)>> = HashMap::new();
    if head.is_some() {
        for d in &dirs {
            for e in git.deleted_intents_first_parent(d).map_err(Fatal::Io)? {
                let folder = e.path.strip_suffix("/intent.md").unwrap_or(&e.path);
                first_parent
                    .entry(folder.to_string())
                    .or_default()
                    .push((e.commit, e.parent, e.time));
            }
        }
    }
    // The last tree of each life: the parent of the deleting commit, or `HEAD`.
    let mut at = Vec::new();
    let mut with_tree = Vec::new();
    for life in all {
        let rev = match (&life.end, &head) {
            (Some(e), _) => e.parent.clone(),
            (None, h) => h.clone(),
        };
        let Some(rev) = rev else { continue };
        at.push((rev, life.folder.clone()));
        with_tree.push(life);
    }
    let trees = read_trees(git, &at)?;
    // Pair the ended lives of a folder, the newest first, with its commits of
    // the first-parent walk.
    let mut next_ended: HashMap<String, usize> = HashMap::new();
    let mut measured: Vec<Measured> = Vec::new();
    for (life, tree) in with_tree.into_iter().zip(&trees).rev() {
        let facts = facts(cfg, &life.folder, tree);
        let (outcome, merged) = match &life.end {
            None => (None, None),
            Some(e) => {
                let status = if tree.intent.is_some() {
                    facts.status.as_str()
                } else {
                    life.last_status.as_str()
                };
                // The pairing counts each ended life, also one that the group
                // filter drops.
                let n = next_ended.entry(life.folder.clone()).or_default();
                let found = first_parent
                    .get(&life.folder)
                    .and_then(|v| v.get(*n))
                    .cloned();
                *n += 1;
                (
                    Some(Outcome::of(status)),
                    Some(found.unwrap_or((e.commit.clone(), e.parent.clone(), e.time))),
                )
            }
        };
        if let Some((_, g)) = group
            && !facts.areas.iter().any(|a| g.contains(a))
        {
            continue;
        }
        measured.push(Measured {
            life,
            facts,
            outcome,
            merged,
        });
    }
    measured.reverse();
    let rows = indicators(repo, cfg, &measured, shallow)?;
    let finished = measured
        .iter()
        .filter(|m| m.outcome == Some(Outcome::Finished))
        .count();
    let closed = measured
        .iter()
        .filter(|m| m.outcome == Some(Outcome::Closed))
        .count();
    if json {
        let list = rows.iter().map(|r| {
            Obj::new()
                .str("play", r.play)
                .str("kind", r.kind)
                .str("name", r.name)
                .raw(
                    "value",
                    &r.value.map_or("null".to_string(), |v| format!("{v:.1}")),
                )
                .opt_str("unit", r.unit)
                .raw(
                    "count",
                    &r.count.map_or("null".to_string(), |c| c.to_string()),
                )
                .opt_str("reason", r.reason.as_deref())
                .end()
        });
        println!("{}", Obj::new().raw("indicators", &json::array(list)).end());
        return Ok(0);
    }
    println!(
        "{} {} in the history{}: {finished} finished, {closed} closed, {} open",
        measured.len(),
        if measured.len() == 1 {
            "change"
        } else {
            "changes"
        },
        group.map_or(String::new(), |(n, _)| format!(" (group {n})")),
        measured.len() - finished - closed
    );
    for r in &rows {
        let mut line = format!("{} {}: {}: ", r.play, r.kind, r.name);
        match r.value {
            Some(v) => {
                let n = r.count.unwrap_or(0);
                line.push_str(&format!(
                    "{v:.1} {} ({n} {})",
                    r.unit.unwrap_or(""),
                    if n == 1 { r.noun.0 } else { r.noun.1 }
                ));
                println!("{line}");
                if let Some(note) = &r.reason {
                    println!("  note: {note}");
                }
            }
            None => {
                line.push_str(&format!("n/a ({})", r.reason.as_deref().unwrap_or("")));
                println!("{line}");
            }
        }
    }
    Ok(0)
}

/// The rows of the indicator table of the design, in its order.
fn indicators(
    repo: &Repo,
    cfg: &Config,
    ms: &[Measured],
    shallow: bool,
) -> Result<Vec<Indicator>, Fatal> {
    let git = &repo.git;
    let finished = |m: &&Measured| m.outcome == Some(Outcome::Finished);
    let mut rows = Vec::new();

    // Plan, leading: the first commit of the folder to the commit that sets
    // `accepted`.
    let v: Vec<f64> = ms
        .iter()
        .filter_map(|m| days(m.life.first?, m.life.accepted_at?))
        .collect();
    rows.push(computed(
        (
            "Plan",
            "leading",
            "Time from first conversation to a committed intent.md",
        ),
        median(v.clone()),
        "days",
        v.len(),
        "the conversation is not in git; the value is the median time from the first commit of the folder to the commit that sets `accepted`",
        "no change has an `accepted` intent in the history yet",
        shallow,
    ));

    // Plan, lagging: accepted rather than closed.
    let (mut yes, mut no) = (0usize, 0usize);
    for m in ms {
        let s = m.facts_status();
        match (m.outcome, s) {
            (Some(Outcome::Finished), _) | (None, "accepted") => yes += 1,
            (Some(Outcome::Closed), _) | (None, "closed") => no += 1,
            _ => {}
        }
    }
    rows.push(computed(
        (
            "Plan",
            "lagging",
            "Survival rate: the share of intent.md files that are accepted rather than closed",
        ),
        (yes + no > 0).then(|| 100.0 * yes as f64 / (yes + no) as f64),
        "percent",
        yes + no,
        "",
        "no intent is `accepted` or `closed` yet",
        shallow,
    ));

    // Design, leading: the `intent.md` commit to the `spec.md` commit.
    let v: Vec<f64> = ms
        .iter()
        .filter_map(|m| days(m.life.intent_first?, m.life.spec_first?))
        .collect();
    rows.push(computed(
        (
            "Design",
            "leading",
            "Elapsed time between the intent.md commit and the spec.md commit",
        ),
        median(v.clone()),
        "days",
        v.len(),
        "the value is the median time from the first commit of the intent to the first commit of the spec",
        "no change has a spec in the history yet",
        shallow,
    ));

    // Design, lagging: spec commits after the first plan commit.
    let v: Vec<f64> = ms
        .iter()
        .filter(|m| m.life.spec_first.is_some() && m.life.plan_seen)
        .map(|m| m.life.spec_after_plan as f64)
        .collect();
    rows.push(computed(
        (
            "Design",
            "lagging",
            "Requirements rework after build starts: spec.md commits after the first plan.md commit",
        ),
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64),
        "commits per change",
        v.len(),
        "the value is the mean for each change that has a spec and a plan",
        "no change has a spec and a plan in the history yet",
        shallow,
    ));

    // Build, leading: plan approval to the merged pull request.
    let v: Vec<f64> = ms
        .iter()
        .filter(finished)
        .filter_map(|m| days(m.life.plan_approved_at?, m.merged.as_ref()?.2))
        .collect();
    rows.push(computed(
        (
            "Build",
            "leading",
            "Time from plan approval to the merged pull request",
        ),
        median(v.clone()),
        "days",
        v.len(),
        "the value is the median time from the commit that sets the plan `approved` to the first-parent commit that deletes the folder; a rebase merge shows only its last commit",
        "no finished change has an approved plan in the history yet",
        shallow,
    ));
    rows.push(na(
        "Build",
        "leading",
        "Share of changes that merge from the first implementation pass",
        "it needs pull request metadata",
    ));

    // Build, lagging: SD913 for the finished change.
    let (clean, total) = plan_match(git, ms)?;
    rows.push(computed(
        (
            "Build",
            "lagging",
            "How often the merged diff still matches the committed plan.md",
        ),
        (total > 0).then(|| 100.0 * clean as f64 / total as f64),
        "percent",
        total,
        "the value is the share of finished changes whose merged diff has no path outside the folder, `Files`, the `Files:` paths of the tasks and the documents of the delta (SD913); the diff is the first-parent commit that deleted the folder, so a rebase merge shows only its last commit",
        "no finished change has a plan and a readable diff in the history yet",
        shallow,
    ));
    rows.push(na(
        "Build",
        "lagging",
        "Rework cycles per change",
        "it needs pull request metadata",
    ));

    rows.extend(test_rows(repo, cfg, shallow)?);
    rows.push(na(
        "Test",
        "lagging",
        "Regressions caught in CI against regressions found in production",
        "it needs the incident tracker",
    ));
    rows.push(na(
        "Maintain",
        "leading",
        "Time from band breach to an intent.md in the triage queue",
        "spec-dna has no bands",
    ));
    rows.push(na(
        "Maintain",
        "lagging",
        "Repeat incidents of the same class",
        "an incident has no class field",
    ));
    rows.push(na(
        "Deploy",
        "both",
        "The other indicators of the playbook",
        "they need CI logs, telemetry or an incident tracker",
    ));
    Ok(rows)
}

const NO_EVALS_NOTE: &str = "the configuration has no [evals] table";
const PASS_RATE: (&str, &str, &str) = ("Test", "leading", "The eval pass rate over time");
const INCIDENT_TIME: (&str, &str, &str) = (
    "Test",
    "leading",
    "How long a production incident takes to become a permanent eval",
);

/// The 2 leading rows of the Test play. They read the tree of `HEAD`: the
/// result files in `<[evals] dir>/results`, and the incidents with the evals
/// that they name. A `--group` filter does not change them, because neither
/// file belongs to a change. Without `[evals]`, both rows are `n/a`.
fn test_rows(repo: &Repo, cfg: &Config, shallow: bool) -> Result<Vec<Indicator>, Fatal> {
    let Some(evals) = &cfg.evals else {
        return Ok(vec![
            na(PASS_RATE.0, PASS_RATE.1, PASS_RATE.2, NO_EVALS_NOTE),
            na(
                INCIDENT_TIME.0,
                INCIDENT_TIME.1,
                INCIDENT_TIME.2,
                NO_EVALS_NOTE,
            ),
        ]);
    };
    let head = repo.head_commit();
    // The files of the tree: path and blob id. A submodule is not a file.
    let tree: Vec<(String, String)> = match &head {
        Some(h) => repo
            .git
            .ls_tree(h)
            .map_err(Fatal::Io)?
            .into_iter()
            .filter(|e| !e.2)
            .map(|e| (e.0, e.1))
            .collect(),
        None => Vec::new(),
    };
    Ok(vec![
        pass_rate_row(&repo.git, evals, &tree)?,
        incident_time_row(repo, cfg, evals, head.as_deref(), &tree, shallow)?,
    ])
}

/// The started time and the pass rate of a complete result file: a JSON object
/// with a string `started`, a number `pass_rate` from 0 to 1 and
/// `"stopped": null`. A run that the cap stopped has no pass rate.
fn complete_result(bytes: &[u8]) -> Option<(String, f64)> {
    let value = json::parse(std::str::from_utf8(bytes).ok()?).ok()?;
    let json::Value::Obj(fields) = &value else {
        return None;
    };
    let key = |k: &str| fields.iter().find(|(n, _)| n == k).map(|(_, v)| v);
    let started = key("started")?.as_str()?;
    let Some(json::Value::Num(rate)) = key("pass_rate") else {
        return None;
    };
    (matches!(key("stopped"), Some(json::Value::Null))
        && !started.is_empty()
        && (0.0..=1.0).contains(rate))
    .then(|| (started.to_string(), *rate))
}

fn files_word(n: usize) -> &'static str {
    if n == 1 { "file" } else { "files" }
}

/// The pass rate of the newest result file by `started`, in percent. The time
/// text is `YYYY-MM-DDTHH:MM:SSZ`, so the order of the text is the order of the
/// times. If 2 files have the same time, the later path wins.
fn pass_rate_row(git: &Git, evals: &Evals, tree: &[(String, String)]) -> Result<Indicator, Fatal> {
    let folder = format!("{}/{}", evals.dir, evals::RESULTS);
    let prefix = format!("{folder}/");
    let files: Vec<&(String, String)> = tree
        .iter()
        .filter(|(p, _)| {
            p.strip_prefix(&prefix).is_some_and(|n| {
                n.len() > ".json".len() && n.ends_with(".json") && !n.contains('/')
            })
        })
        .collect();
    let mut ids: Vec<String> = files.iter().map(|f| f.1.clone()).collect();
    ids.sort();
    ids.dedup();
    let blobs = git.read_blobs(&ids).map_err(Fatal::Io)?;
    let mut valid = 0;
    let mut skipped = 0;
    let mut newest: Option<(String, &str, f64)> = None;
    for (path, id) in files {
        match blobs.get(id).and_then(|b| complete_result(b)) {
            Some((started, rate)) => {
                valid += 1;
                if newest
                    .as_ref()
                    .is_none_or(|(s, p, _)| (&started, path.as_str()) > (s, *p))
                {
                    newest = Some((started, path.as_str(), rate));
                }
            }
            None => skipped += 1,
        }
    }
    let skip = if skipped > 0 {
        format!(
            "; {skipped} {} skipped: not a complete result",
            files_word(skipped)
        )
    } else {
        String::new()
    };
    let note = newest.as_ref().map_or(String::new(), |(started, ..)| {
        format!(
            "the value is the pass rate of the newest result file ({started}); the committed files hold the history{skip}"
        )
    });
    let empty = format!(
        "no result file in {folder}; run `spec-dna eval run --out {folder}` and commit the file{skip}"
    );
    // The row reads the tree of `HEAD`, not the history, so a shallow clone
    // adds no note.
    Ok(computed(
        PASS_RATE,
        newest.map(|(_, _, rate)| 100.0 * rate),
        "percent",
        valid,
        &note,
        &empty,
        false,
    )
    .counting(("result file", "result files")))
}

/// The text of each section `Eval` of a document, with its subsections. The
/// case and the level of the heading do not matter.
fn eval_section(text: &str, doc: &crate::parse::ParsedDoc) -> String {
    let chunks = &doc.chunks;
    let mut out = String::new();
    let mut i = 0;
    while i < chunks.len() {
        let head = &chunks[i];
        i += 1;
        if head.level == 0 || !head.heading.trim().eq_ignore_ascii_case("eval") {
            continue;
        }
        out.push_str(
            text.get(head.start as usize..head.end as usize)
                .unwrap_or(""),
        );
        out.push('\n');
        while i < chunks.len() && chunks[i].level > head.level {
            let c = &chunks[i];
            out.push_str(text.get(c.start as usize..c.end as usize).unwrap_or(""));
            out.push('\n');
            i += 1;
        }
    }
    out
}

/// The first path `<dir>/<id>.json` in the text that is a file of `files`. A
/// leading `/`, `./` or a chain of `../` is part of a link to the path and does
/// not count. Before that, the path must not continue a longer path or word, so
/// `docs/evals/x.json` does not name `evals/x.json`. After `.json`, a letter, a
/// digit or `_` ends the match. An eval id has the characters `a` to `z`, `0`
/// to `9` and `-`.
fn first_eval_path(text: &str, dir: &str, files: &HashSet<String>) -> Option<String> {
    let prefix = format!("{dir}/");
    for (at, _) in text.match_indices(&prefix) {
        let mut before = &text[..at];
        // One `/` for a path from the root, then the dots of a relative link.
        before = before.strip_suffix('/').unwrap_or(before);
        if before.ends_with('.') {
            before = before.trim_end_matches(['.', '/']);
        }
        let left_ok = before
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-')));
        if !left_ok {
            continue;
        }
        let rest = &text[at + prefix.len()..];
        let id_len = rest
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(rest.len());
        let Some(after) = rest[id_len..].strip_prefix(".json") else {
            continue;
        };
        if id_len == 0
            || after
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let path = format!("{prefix}{}.json", &rest[..id_len]);
        if files.contains(&path) {
            return Some(path);
        }
    }
    None
}

/// The median days from the commit that added an incident to the commit that
/// added the eval that its `Eval` section names. A span below 0 does not
/// count.
fn incident_time_row(
    repo: &Repo,
    cfg: &Config,
    evals: &Evals,
    head: Option<&str>,
    tree: &[(String, String)],
    shallow: bool,
) -> Result<Indicator, Fatal> {
    let row = |v: Vec<f64>| {
        computed(
            INCIDENT_TIME,
            median(v.clone()),
            "days",
            v.len(),
            "",
            "no incident names an eval file in its `Eval` section",
            shallow,
        )
        .counting(("incident", "incidents"))
    };
    let Some(head) = head else {
        return Ok(row(Vec::new()));
    };
    // An eval file is a `*.json` file directly in `[evals] dir`.
    let prefix = format!("{}/", evals.dir);
    let files: HashSet<String> = tree
        .iter()
        .filter(|(p, _)| {
            p.strip_prefix(&prefix).is_some_and(|n| {
                n.len() > ".json".len() && n.ends_with(".json") && !n.contains('/')
            })
        })
        .map(|(p, _)| p.clone())
        .collect();
    if files.is_empty() {
        return Ok(row(Vec::new()));
    }
    let snap = Snapshot::at(repo, head)?;
    let corpus = repo::load(repo, cfg, &[&snap], true, false)?
        .corpora
        .remove(0);
    let incidents: Vec<&crate::model::Doc> = corpus
        .docs
        .iter()
        .filter(|d| d.ty == DocType::Incident)
        .collect();
    let specs: Vec<(String, String)> = incidents
        .iter()
        .map(|d| (head.to_string(), d.path.clone()))
        .collect();
    let texts = repo.git.read_at(&specs).map_err(Fatal::Io)?;
    let mut pairs: Vec<(&str, String)> = Vec::new();
    for (d, text) in incidents.iter().zip(&texts) {
        let Some(bytes) = text else { continue };
        let text = String::from_utf8_lossy(bytes);
        let section = eval_section(&text, &d.parsed);
        if let Some(eval) = first_eval_path(&section, &evals.dir, &files) {
            pairs.push((d.path.as_str(), eval));
        }
    }
    let mut paths: Vec<String> = pairs
        .iter()
        .flat_map(|(i, e)| [(*i).to_string(), e.clone()])
        .collect();
    paths.sort();
    paths.dedup();
    let added = repo.git.first_added(head, &paths).map_err(Fatal::Io)?;
    let spans = pairs
        .iter()
        .filter_map(|(i, e)| days(*added.get(*i)?, *added.get(e)?))
        .collect();
    Ok(row(spans))
}

impl Measured {
    /// The status of the intent in the last tree of the life.
    fn facts_status(&self) -> &str {
        if self.facts.status.is_empty() {
            &self.life.last_status
        } else {
            &self.facts.status
        }
    }
}

/// SD913 for each finished change: returns the number of changes whose merged
/// diff is inside the scope of the plan, and the number of changes that it
/// could measure. The scope is the folder, the `Files` patterns of the plan and
/// the documents that the delta and `Interfaces` name. A generated index is in
/// scope. A change with no plan, or with no readable diff, is not measured.
fn plan_match(git: &Git, ms: &[Measured]) -> Result<(usize, usize), Fatal> {
    let list: Vec<&Measured> = ms
        .iter()
        .filter(|m| m.outcome == Some(Outcome::Finished) && m.facts.files.is_some())
        .filter(|m| m.merged.is_some())
        .collect();
    let commits: Vec<String> = list
        .iter()
        .filter_map(|m| m.merged.as_ref().map(|x| x.0.clone()))
        .collect();
    let diffs = git.first_parent_paths(&commits).map_err(Fatal::Io)?;
    // The scope of a commit is the union of the scopes of the changes that it
    // finished, as SD913 does for the changes of one diff. A path that one of
    // them covers is not outside.
    let mut scopes: HashMap<&str, Vec<String>> = HashMap::new();
    for m in &list {
        let Some((commit, _, _)) = &m.merged else {
            continue;
        };
        let scope = scopes.entry(commit.as_str()).or_default();
        scope.push(format!("{}/", m.life.folder));
        scope.extend(m.facts.files.iter().flatten().cloned());
        scope.extend(m.facts.named.iter().cloned());
    }
    // The paths of each diff that the scope does not cover.
    let mut outside: Vec<(&Measured, Vec<String>)> = Vec::new();
    for m in list {
        let Some((commit, _, _)) = &m.merged else {
            continue;
        };
        let Some(paths) = diffs.get(commit) else {
            continue;
        };
        let scope = &scopes[commit.as_str()];
        let out: Vec<String> = paths
            .iter()
            .filter(|p| !change::in_scope(scope, p))
            .cloned()
            .collect();
        outside.push((m, out));
    }
    // A generated index is never outside the scope (SD913). It is a file with
    // the marker of `index` in its front matter, in the merged tree or, for a
    // deleted file, in its parent.
    let mut specs = Vec::new();
    for (m, paths) in &outside {
        if let Some((commit, parent, _)) = &m.merged {
            for p in paths {
                specs.push((commit.clone(), p.clone()));
                specs.push((parent.clone().unwrap_or_else(|| commit.clone()), p.clone()));
            }
        }
    }
    let texts = git.read_at(&specs).map_err(Fatal::Io)?;
    let generated = |i: usize| -> bool {
        texts[i * 2]
            .as_ref()
            .or(texts[i * 2 + 1].as_ref())
            .is_some_and(|b| {
                let text = String::from_utf8_lossy(b);
                frontmatter::split(&text)
                    .and_then(|(fm, _)| frontmatter::parse(fm, 2).ok())
                    .is_some_and(|f| {
                        matches!(f.get("generated"),
                            Some((Value::Str(s), _)) if s.starts_with(registry::MARKER))
                    })
            })
    };
    let (mut clean, mut total) = (0, 0);
    let mut i = 0;
    for (_, paths) in &outside {
        let mut ok = true;
        for _ in paths {
            if !generated(i) {
                ok = false;
            }
            i += 1;
        }
        total += 1;
        if ok {
            clean += 1;
        }
    }
    Ok((clean, total))
}
