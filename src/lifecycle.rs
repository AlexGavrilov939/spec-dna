//! The base of the change checks (design 0.6.0, "Gates"), the diff against it,
//! the requirements that the diff changes, the checks of the change lifecycle
//! that need one tree only (`tree_checks`), the checks of the diff against the
//! base (`base_checks`: SD905, SD910, SD913, SD915, SD916, SD917, SD918 and
//! SD920), and
//! the staleness of a started change (`stale_changes`, SD911).
//!
//! The lifecycle base is the merge base of `--base` (or of `origin/HEAD`, or of
//! the local branch of `[change] base`) with `HEAD`. It is never the tip of `--base`: a branch that started from an old
//! commit is not behind the default branch because of that. An older `--base`
//! is replaced by the later merge base with `origin/HEAD` (`newer_merge_base`).
//!
//! SD923 and SD924 skip a plan that a commit added under a pin before 0.7.0
//! (`skip_early_plans`).
//!
//! SD910 reads the approvals from the base tree. With `[change] approval =
//! "branch"`, it also reads the approval commits of `base..HEAD`
//! ([`crate::approval`], design 0.7.0, D11 to D17).

use crate::approval::{Approval, Branch, Fail, Reason};
use crate::change::{self, Change, Delta, Verb};
use crate::config::{self, ChangePolicy, Config};
use crate::diag::{Code, Diag};
use crate::evidence::Evidence;
use crate::git::{Git, Limited, PathChange};
use crate::hash::{Hash, hex};
use crate::model::{Class, Corpus, Doc, DocType, Res};
use crate::parse::{ItemSection, ListItem, ParsedDoc, parse};
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// The note that a command prints when the lifecycle base is unknown.
pub const NO_BASE_NOTE: &str = "no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base";

/// Returns the note for an unknown lifecycle base. If `[change] base` names a
/// branch that the repository does not have, and there is no `--base` and no
/// `origin/HEAD`, the note names the branch (design 0.6.1, D37). If the
/// branch exists but has no merge base with `HEAD`, the note says so. Otherwise
/// it is `NO_BASE_NOTE`.
pub fn no_base_note(config: &Config, git: &Git, base: Option<&str>) -> String {
    let note = no_base_reason(config, git, base);
    // With the branch rule, a shallow clone is the likely cause (D13).
    if config.change_approval == config::Approval::Branch && git.is_shallow() {
        return format!(
            "{note}; the clone is shallow, and the branch rule needs the full history, so fetch it (`fetch-depth: 0`)"
        );
    }
    note
}

fn no_base_reason(config: &Config, git: &Git, base: Option<&str>) -> String {
    // A base that the user passed is not a base to ask for.
    if let Some(rev) = base {
        return format!(
            "no base for the change checks: `{rev}` and HEAD have no merge base (a shallow clone, or histories with no common commit); fetch more history of the branch{}",
            if git.is_shallow() {
                " (`fetch-depth: 0`)"
            } else {
                ""
            }
        );
    }
    // The base branch is known, and the clone does not reach the merge base: the
    // fix is the full history.
    if git.is_shallow() && git.remote_head().is_some() {
        return "no base for the change checks: the clone is shallow and has no merge base with the base branch; fetch the full history (`fetch-depth: 0`)".to_string();
    }
    match &config.change_base {
        Some(b)
            if base.is_none() && git.remote_head().is_none() && git.local_branch(b).is_none() =>
        {
            format!(
                "no base for the change checks: no origin/HEAD, and the branch `{b}` of [change] base does not exist; pass --base <rev>"
            )
        }
        // The key is set and the branch exists, but the base is unknown: the
        // branch and `HEAD` have no merge base. The note does not ask for a
        // key that is already set.
        Some(b)
            if base.is_none()
                && git.remote_head().is_none()
                && git.local_branch(b).is_some()
                && git.commit("HEAD").is_some() =>
        {
            format!(
                "no base for the change checks: the branch `{b}` of [change] base has no common history with HEAD; pass --base <rev>"
            )
        }
        _ => NO_BASE_NOTE.to_string(),
    }
}

/// Returns the lifecycle base: the merge base of `--base` (or, without it, of
/// `refs/remotes/origin/HEAD`, or, if that ref is absent, of the local branch
/// of `[change] base`) with `HEAD`. A `Some` value is always a real
/// merge base. It is never a tip that a command used because no merge base
/// exists. Thus a check that needs the age of a commit can trust it.
///
/// It returns `None` if the policy is `none` (the function then runs no git
/// command), if there is no base revision, if `HEAD` has no commit, or if the
/// two commits have no merge base (an unrelated history, or a shallow clone
/// that does not reach it). It returns an error if `--base` is not a commit.
pub fn change_base(
    config: &Config,
    git: &Git,
    base: Option<&str>,
) -> Result<Option<String>, String> {
    if config.change_policy == ChangePolicy::None {
        return Ok(None);
    }
    merge_base_of(git, base, config.change_base.as_deref())
}

/// The base of the change checks for a shallow clone in branch mode, when the
/// base is known and the merge base is not reachable (design 0.7.0, D13): the
/// tip of the base. The branch rule then gives no approval, and SD910 reports
/// the reason 5 for each change that needs one, so less history never gives a
/// pass. It is `None` in each other case, and the note of an unknown base stays.
/// The tip is not a merge base, so SD911 does not use it.
pub fn shallow_tip(config: &Config, git: &Git, base: Option<&str>) -> Option<String> {
    if config.change_policy == ChangePolicy::None
        || config.change_approval != config::Approval::Branch
        || !git.is_shallow()
    {
        return None;
    }
    let tip = match base {
        Some(rev) => git.commit(rev),
        None => git.remote_head().or_else(|| {
            config
                .change_base
                .as_deref()
                .and_then(|b| git.local_branch(b))
        }),
    }?;
    let head = git.commit("HEAD")?;
    git.merge_base(&tip, &head).is_none().then_some(tip)
}

/// The merge base of `--base` (or, without it, of `refs/remotes/origin/HEAD`,
/// or, if that ref is absent, of the local branch `branch`, the value of
/// `[change] base`) with `HEAD`, for any policy. `change_base` returns `None` for the policy
/// `none` before it calls this function. `finish` always needs the base of an
/// `accepted` change, so it calls this function. The result has the same
/// meaning as the result of `change_base`.
pub fn merge_base_of(
    git: &Git,
    base: Option<&str>,
    branch: Option<&str>,
) -> Result<Option<String>, String> {
    let tip = match base {
        Some(rev) => Some(git.commit(rev).ok_or_else(|| {
            format!(
                "base `{rev}` is not a commit here; fetch it first (a depth-1 fetch of the base commit is enough)"
            )
        })?),
        None => git
            .remote_head()
            .or_else(|| branch.and_then(|b| git.local_branch(b))),
    };
    let (Some(tip), Some(head)) = (tip, git.commit("HEAD")) else {
        return Ok(None);
    };
    let found = git.merge_base(&tip, &head);
    // An older `--base` (design 0.7.1, B10): a later merge base with
    // `origin/HEAD` is the base of the change checks.
    if base.is_some()
        && let Some(newer) = found.as_deref().and_then(|m| newer_merge_base(git, m))
    {
        return Ok(Some(newer));
    }
    Ok(found)
}

/// The merge base of `refs/remotes/origin/HEAD` and `HEAD`, if it is a strict
/// descendant of `older` (the merge base of `--base` and `HEAD`). The commits
/// between them are merged pull requests, and a squash merge keeps no approval
/// commit, so the change checks start at this commit (design 0.7.1, B10). The
/// function reads no policy, so SD503 uses it too. Only `origin/HEAD` starts
/// the rule, not `[change] base`: the branch of a repository with no remote
/// has `HEAD` as its merge base, and that would turn the checks off. It costs
/// 2 `merge-base` calls and one ancestor test.
pub fn newer_merge_base(git: &Git, older: &str) -> Option<String> {
    let tip = git.remote_head()?;
    let head = git.commit("HEAD")?;
    let newer = git.merge_base(&tip, &head)?;
    (newer != older && git.is_ancestor(older, &newer)).then_some(newer)
}

/// SD923 and SD924 skip an `approved` plan that a commit added when
/// `spec-dna.toml` pinned a version before 0.7.0 (design 0.7.1, B5). The
/// templates of 0.6 already had `Files:` and `Verify:`, so only the history
/// tells the version. `head` and `new` are the parallel lists of `check`: the
/// function removes the diagnostics of such a plan from both. The lookup runs
/// for the plans that have a diagnostic of the 2 codes: one `git log` and one
/// read of `spec-dna.toml` for each. No commit that added the plan (a plan
/// that is not committed, or a shallow clone), or no pin in that commit, is
/// the current form.
///
/// It returns one note for each such plan whose dropped diagnostic would show:
/// it is new, or `all` is true. The note is not a diagnostic.
pub fn skip_early_plans(
    git: &Git,
    head: &mut Vec<Diag>,
    new: &mut Vec<bool>,
    all: bool,
) -> Result<Vec<String>, String> {
    let mut paths: Vec<&str> = head
        .iter()
        .filter(|d| matches!(d.code, Code::TaskIncomplete | Code::TaskTooLarge))
        .map(|d| d.path.as_str())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    if paths.is_empty() || git.commit("HEAD").is_none() {
        return Ok(Vec::new());
    }
    let first_current = config::Version(0, 7, 0);
    let mut early: Vec<(String, String)> = Vec::new();
    for path in paths {
        // The reader of the pin of an adding commit that the sections of 0.8.0
        // (`section_is_read`) use too.
        if let Some(pin) = crate::acceptance::added_before(git, "HEAD", path, first_current)? {
            early.push((path.to_string(), pin));
        }
    }
    let mut notes = Vec::new();
    for (path, pin) in &early {
        let hit = |d: &Diag| {
            matches!(d.code, Code::TaskIncomplete | Code::TaskTooLarge) && d.path == *path
        };
        let mut tasks: Vec<&str> = head
            .iter()
            .filter(|d| hit(d))
            .map(|d| d.target.as_str())
            .collect();
        tasks.sort_unstable();
        tasks.dedup();
        let shows = head
            .iter()
            .zip(new.iter())
            .any(|(d, n)| hit(d) && (all || *n));
        if shows {
            notes.push(format!(
                "`{path}` was added when spec-dna.toml pinned {pin}, before 0.7.0, so SD923 and SD924 skip its {} task(s) with a finding; to check them, give each task exact paths in `Files:` and one `Verify:`",
                tasks.len()
            ));
        }
    }
    let keep: Vec<bool> = head
        .iter()
        .map(|d| {
            !(matches!(d.code, Code::TaskIncomplete | Code::TaskTooLarge)
                && early.iter().any(|(p, _)| *p == d.path))
        })
        .collect();
    let mut at = keep.iter();
    head.retain(|_| *at.next().unwrap_or(&true));
    let mut at = keep.iter();
    new.retain(|_| *at.next().unwrap_or(&true));
    Ok(notes)
}

/// SD936, SD937, SD939 and SD940 read a section that 0.7.1 did not know. The
/// function removes the diagnostics of a spec or a plan that the base tree has
/// and that a commit added when `spec-dna.toml` pinned a version before 0.8.0
/// ([`crate::acceptance::section_is_read`]): its section keeps the meaning of
/// 0.7.1. `head` and `new` are the parallel lists of `check`. It runs one lookup
/// for each such path. With no base, it reads the history of `HEAD` for each
/// path, as it does for a file that the base tree lacks. A repository with no
/// commit removes nothing.
pub fn skip_early_sections(
    git: &Git,
    base_rev: Option<&str>,
    head: &mut Vec<Diag>,
    new: &mut Vec<bool>,
) -> Result<(), String> {
    let mine = |c: Code| {
        matches!(
            c,
            Code::InvariantInvalid
                | Code::InvariantStale
                | Code::DeviationInvalid
                | Code::DeviationUnresolved
        )
    };
    let paths: Vec<&str> = head
        .iter()
        .filter(|d| mine(d.code))
        .map(|d| d.path.as_str())
        .collect();
    let early = crate::acceptance::sections_skipped(git, base_rev, &paths)?;
    if early.is_empty() {
        return Ok(());
    }
    let keep: Vec<bool> = head
        .iter()
        .map(|d| !(mine(d.code) && early.contains(&d.path)))
        .collect();
    let mut at = keep.iter();
    head.retain(|_| *at.next().unwrap_or(&true));
    let mut at = keep.iter();
    new.retain(|_| *at.next().unwrap_or(&true));
    Ok(())
}

/// The codes that `check` reads as errors only for a spec that the diff touches
/// (design 0.6.0, K3, and 0.8.0, D6): SD914 and SD937.
pub const TOUCHED_CODES: [Code; 2] = [Code::DeltaInvalid, Code::InvariantStale];

/// The codes that `finish` reads so for the result of step 5: SD937 only. SD914
/// in the result of `finish` is an error for each spec, as in 0.7.1.
pub const FINISH_TOUCHED_CODES: [Code; 1] = [Code::InvariantStale];

/// Lowers the severity of the diagnostics of `codes` to a warning for a spec that
/// the diff does not touch. `touched` tells if the diff has a path. This is the
/// one rule, with the list of codes as a parameter: `check` uses
/// [`TOUCHED_CODES`], and `finish` (the result of step 5) uses
/// [`FINISH_TOUCHED_CODES`]. With no lifecycle base, no spec is touched.
pub fn lower_untouched(diags: &mut [Diag], codes: &[Code], touched: &dyn Fn(&str) -> bool) {
    for d in diags {
        if codes.contains(&d.code)
            && d.severity == crate::diag::Severity::Error
            && !touched(&d.path)
        {
            d.severity = crate::diag::Severity::Warning;
        }
    }
}

/// The files that a change touches: the diff of the head against the base.
#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub files: Vec<(String, PathChange)>,
}

impl Diff {
    /// Reads the diff of the working tree (or of the git index, for a
    /// `staged` `Git`) against the base. With `untracked`, a new file that git
    /// does not ignore counts as added.
    pub fn read(git: &Git, base: &str, untracked: bool) -> Result<Diff, String> {
        Ok(Diff {
            files: git.diff_paths(base, untracked)?,
        })
    }

    /// The changed paths under a folder (a repository path with no final `/`).
    pub fn under<'a>(&'a self, folder: &'a str) -> impl Iterator<Item = &'a str> {
        let prefix = format!("{}/", folder.trim_matches('/'));
        self.files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(move |p| p.starts_with(&prefix))
    }

    /// The kind of change of a path, or `None` if the diff does not have it.
    pub fn kind(&self, path: &str) -> Option<PathChange> {
        self.files.iter().find(|(p, _)| p == path).map(|(_, k)| *k)
    }
}

/// How a requirement differs between the base and the head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReqChangeKind {
    Changed,
    Added,
    Removed,
}

/// A requirement that the diff changed, added or removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReqChange {
    /// The id of the document in the head (the id of the base for a document
    /// that the head does not have).
    pub doc: String,
    /// The path of that document.
    pub path: String,
    /// The line of the requirement.
    pub line: u32,
    pub key: String,
    pub kind: ReqChangeKind,
    /// `Req.hash` in the base and in the head: the hash that `show` prints and
    /// that a delta line copies.
    pub base_hash: Option<Hash>,
    pub head_hash: Option<Hash>,
    /// `Req.frozen` in the base and in the head: the hash without link
    /// destinations. The comparison of the kinds uses it.
    pub base_frozen: Option<Hash>,
    pub head_frozen: Option<Hash>,
}

/// Returns the requirements that differ between the base and the head, for the
/// documents that `wanted` accepts. A requirement of the base is matched with
/// the head by its key through the aliases (`Corpus::resolve`), so a move that
/// keeps the text gives nothing. It is `Changed` if the `frozen` hash differs,
/// `Removed` if the head has no match, and `Added` if a head requirement has no
/// match in the base. The order is the order of the base documents, then of the
/// head documents. This is the comparison of SD921 (design 0.6.0, D19).
pub fn requirement_changes(
    base: &Corpus,
    head: &Corpus,
    wanted: impl Fn(&Doc) -> bool,
) -> Vec<ReqChange> {
    let mut out = Vec::new();
    let mut matched: HashSet<(u32, usize)> = HashSet::new();
    for b in base.docs.iter().filter(|d| wanted(d)) {
        for &ri in &b.reqs {
            let r = &b.parsed.reqs[ri];
            let target = format!("{}#{}", b.id, r.key);
            let hit = match head.resolve(&target) {
                Res::Found(hd, Some(key), true) => {
                    head.docs[hd as usize].req(&key).map(|hri| (hd, hri))
                }
                // Two documents have the name, so the result is unknown.
                Res::Ambiguous(_) => continue,
                _ => None,
            };
            match hit {
                Some((hd, hri)) => {
                    matched.insert((hd, hri));
                    let h = &head.docs[hd as usize];
                    let hr = &h.parsed.reqs[hri];
                    if hr.frozen != r.frozen {
                        out.push(ReqChange {
                            doc: h.id.clone(),
                            path: h.path.clone(),
                            line: hr.line,
                            key: hr.key.clone(),
                            kind: ReqChangeKind::Changed,
                            base_hash: Some(r.hash),
                            head_hash: Some(hr.hash),
                            base_frozen: Some(r.frozen),
                            head_frozen: Some(hr.frozen),
                        });
                    }
                }
                None => {
                    // The document can be in the head, without the key.
                    let owner = match head.resolve(&b.id) {
                        Res::Found(hd, None, _) => &head.docs[hd as usize],
                        _ => b,
                    };
                    out.push(ReqChange {
                        doc: owner.id.clone(),
                        path: owner.path.clone(),
                        line: if std::ptr::eq(owner, b) { r.line } else { 0 },
                        key: r.key.clone(),
                        kind: ReqChangeKind::Removed,
                        base_hash: Some(r.hash),
                        head_hash: None,
                        base_frozen: Some(r.frozen),
                        head_frozen: None,
                    });
                }
            }
        }
    }
    for (hi, h) in head.docs.iter().enumerate().filter(|(_, d)| wanted(d)) {
        for &ri in &h.reqs {
            if matched.contains(&(hi as u32, ri)) {
                continue;
            }
            let r = &h.parsed.reqs[ri];
            // The key can exist in the base in another kind of document, or
            // under an old name of the document or of the key.
            let names = std::iter::once(&h.id).chain(&h.aliases);
            let old = names
                .map(|n| format!("{n}#{}", r.key))
                .chain(r.aliases.iter().cloned())
                .any(|n| matches!(base.resolve(&n), Res::Found(_, _, true) | Res::Ambiguous(_)));
            if !old {
                out.push(ReqChange {
                    doc: h.id.clone(),
                    path: h.path.clone(),
                    line: r.line,
                    key: r.key.clone(),
                    kind: ReqChangeKind::Added,
                    base_hash: None,
                    head_hash: Some(r.hash),
                    base_frozen: None,
                    head_frozen: Some(r.frozen),
                });
            }
        }
    }
    out
}

/// Groups changed requirements by document id, in the order of the ids.
pub fn by_document(changes: &[ReqChange]) -> BTreeMap<&str, Vec<&ReqChange>> {
    let mut m: BTreeMap<&str, Vec<&ReqChange>> = BTreeMap::new();
    for c in changes {
        m.entry(c.doc.as_str()).or_default().push(c);
    }
    m
}

/// The checks of the change lifecycle that read one tree: SD905 (the delta
/// part), SD912, SD914 (without the check of the diff), SD919, SD920 (the
/// absent decision) and SD922. They need no base and no history. `base` is the tree that
/// the gate compares with. A `REMOVE` line uses it to tell that the head already
/// has the result of the line. `layers` is the layer of each document of `c`.
///
/// A change whose intent is `closed` does not proceed, so its spec gives
/// nothing. All the checks return no diagnostic if the policy is `none`.
pub fn tree_checks(
    config: &Config,
    c: &Corpus,
    base: Option<&Corpus>,
    layers: &[Option<usize>],
) -> Vec<Diag> {
    if config.change_policy == ChangePolicy::None
        || [
            Code::FanOut,
            Code::StartedLimit,
            Code::DeltaInvalid,
            Code::ConcernOpen,
            Code::QuestionOpen,
            Code::DecisionNeededUnaccepted,
            Code::TaskIncomplete,
            Code::TaskTooLarge,
            Code::CheckboxMissing,
            Code::InvariantInvalid,
            Code::InvariantStale,
            Code::DeviationInvalid,
            Code::DeviationUnresolved,
        ]
        .iter()
        .all(|&k| config.is_off(k))
    {
        return Vec::new();
    }
    let changes = change::collect(c);
    let mut out = Vec::new();
    // The layers that the `intent` layer moves into.
    let intent_layer = config.layers.layer_of_type(DocType::Intent);
    let allowed = allowed_layers(config);
    for ch in &changes {
        if ch.intent_status() == "closed" {
            continue;
        }
        task_diags(config, c, ch, &mut out);
        out.extend(deviation_diags(config, c, ch));
        let Some(spec) = &ch.spec else { continue };
        let d = &c.docs[spec.doc as usize];
        out.extend(question_diags(c, ch));
        boxes_missing(d, &mut out);
        if spec.status == "approved" {
            concerns(d, &mut out);
            decisions(c, d, &mut out);
        }
        let mut targets: HashSet<String> = HashSet::new();
        for (line, item) in ch
            .delta(c)
            .into_iter()
            .zip(change::section_items(d, ItemSection::Delta))
        {
            let delta = match &line.parsed {
                Ok(delta) => delta,
                Err(reason) => {
                    out.push(crate::check::diag(
                        d,
                        line.line,
                        Code::DeltaInvalid,
                        short(&item.text),
                        format!("the delta line is not valid: {reason}"),
                    ));
                    continue;
                }
            };
            targets.insert(target_doc(c, &delta.id));
            if already_applied(c, base, delta) {
                continue;
            }
            if let Some(problem) = delta_problem(config, c, layers, allowed.as_deref(), delta) {
                out.push(crate::check::diag(
                    d,
                    line.line,
                    Code::DeltaInvalid,
                    delta.target(),
                    format!("`{}`: {problem}", delta.target()),
                ));
            }
        }
        interfaces(config, c, d, &mut out);
        out.extend(invariant_diags(config, c, ch));
        let limit = config.max_fanout as usize;
        if targets.len() > limit {
            out.push(crate::check::diag(
                d,
                1,
                Code::FanOut,
                "",
                format!(
                    "the delta touches {} requirement documents, more than `max-fanout = {limit}`",
                    targets.len()
                ),
            ));
        }
    }
    let mut out = crate::check::apply_severity(config, out);
    out.extend(started_limit(config, intent_layer, &changes));
    out
}

/// The names of the layers that the `intent` layer moves into (its `moves-to`
/// list), or `None` if the project has no `intent` layer. A delta line must
/// target a document of one of them.
pub fn allowed_layers(config: &Config) -> Option<Vec<&str>> {
    config.layers.layer_of_type(DocType::Intent).map(|i| {
        config.layers.layers()[i]
            .moves_to
            .iter()
            .map(String::as_str)
            .collect()
    })
}

/// The text of an item on one line, at most 60 characters. It is the target
/// of a diagnostic about an item that has no id.
pub(crate) fn short(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(60)
        .collect()
}

/// The name of the document that a delta line names: the id of the document in
/// the tree, or the id of the line if the tree has no such document.
fn target_doc(c: &Corpus, id: &str) -> String {
    match c.resolve(id) {
        Res::Found(i, None, _) => c.docs[i as usize].id.clone(),
        _ => id.to_string(),
    }
}

/// True if the tree already has the result of a delta line, so the line gives
/// no diagnostic (design T3). `ADD` and `CHANGE`: the head requirement has the
/// text of the line. `REMOVE`: the head has no such requirement, and the base
/// has it. With no base, a `REMOVE` of an absent requirement is not applied.
fn already_applied(c: &Corpus, base: Option<&Corpus>, d: &Delta) -> bool {
    let head = change::target_req(c, &d.id, &d.key);
    match d.verb {
        Verb::Add | Verb::Change => {
            matches!((head, d.frozen()), (Some(r), Some(f)) if r.frozen == f)
        }
        Verb::Remove => {
            head.is_none() && base.is_some_and(|b| change::target_req(b, &d.id, &d.key).is_some())
        }
    }
}

/// The first problem of a delta line with a valid form, or `None`. The order
/// is: the target document, its type, its layer, its key, its hash.
pub(crate) fn delta_problem(
    config: &Config,
    c: &Corpus,
    layers: &[Option<usize>],
    allowed: Option<&[&str]>,
    d: &Delta,
) -> Option<String> {
    let id = &d.id;
    let (doc, layer) = match c.resolve(id) {
        Res::Ambiguous(v) => return Some(format!("`{id}` names {} documents", v.len())),
        Res::Found(i, None, _) => (Some(&c.docs[i as usize]), layers[i as usize]),
        _ => (None, None),
    };
    let layer_problem = |layer: Option<usize>, what: &str| -> Option<String> {
        let allowed = allowed?;
        let Some(l) = layer else {
            return Some(format!("{what} is in no layer"));
        };
        let name = &config.layers.layers()[l].name;
        (!allowed.contains(&name.as_str())).then(|| {
            format!("{what} is in the layer `{name}`, and the `moves-to` list of the layer `intent` does not have it")
        })
    };
    let not_requirements = |ty: DocType| {
        format!(
            "`{id}` is a `{}` document; a delta line needs a document of the type `requirements`, `interface` or `rule`",
            ty.name()
        )
    };
    let Some(doc) = doc else {
        if d.verb != Verb::Add {
            return Some(format!("`{id}` names no document"));
        }
        if id.ends_with(".md") {
            return Some(format!(
                "`{id}` ends in `.md`, and `finish` adds `.md` to the id of a new document; write the id without `.md`"
            ));
        }
        // A new document: `finish` writes it at the place of its id.
        let rel = format!("{id}.md");
        let ty = config
            .layers
            .place_type(&rel)
            .unwrap_or(DocType::Requirements);
        if !ty.has_requirements() {
            return Some(not_requirements(ty));
        }
        return layer_problem(
            config.layers.layer_of(&rel, ty),
            &format!("the new document `{id}`"),
        );
    };
    if !doc.has_layer() {
        return Some(format!("`{id}` has no valid type"));
    }
    if !doc.ty.has_requirements() {
        return Some(not_requirements(doc.ty));
    }
    if let Some(p) = layer_problem(layer, &format!("`{id}`")) {
        return Some(p);
    }
    let req = doc.req(&d.key).map(|i| &doc.parsed.reqs[i]);
    match (d.verb, req) {
        (Verb::Add, Some(_)) => Some(format!(
            "`{id}` has the key `{}` already; an ADD line needs a new key",
            d.key
        )),
        (Verb::Add, None) => None,
        (_, None) => Some(format!("`{id}` has no requirement `{}`", d.key)),
        (_, Some(r)) => {
            let hash = d.hash.as_deref().unwrap_or("");
            (!change::hash_matches(r, hash)).then(|| {
                format!(
                    "the hash `{hash}` is not the hash of the requirement now (`{}`); read the requirement again and copy the hash from `spec-dna show`",
                    hex(&r.hash)
                )
            })
        }
    }
}

/// SD914 for the items of `Interfaces`: `CHANGE <interface id>: <text>` for an
/// interface that exists, `ADD <interface id>: <text>` for a new interface, or
/// one item `none` (design 0.7.0, section 15). An item with another form, an
/// absent document of a `CHANGE` item, a document that is not an interface and
/// a place of an `ADD` item that gives another type give SD914. A section that
/// states "nothing" gives none.
pub(crate) fn interfaces(config: &Config, c: &Corpus, spec: &Doc, out: &mut Vec<Diag>) {
    if change::states_nothing(spec, ItemSection::Interfaces) {
        return;
    }
    for item in change::section_items(spec, ItemSection::Interfaces) {
        let mut push = |target: String, message: String| {
            out.push(crate::check::diag(
                spec,
                item.line,
                Code::DeltaInvalid,
                target,
                message,
            ))
        };
        let Some((verb, id)) = interface_item(item) else {
            push(
                short(&item.text),
                "an item of `Interfaces` is `CHANGE <interface id>: <text>`, `ADD <interface id>: <text>` or `none`".to_string(),
            );
            continue;
        };
        if bad_interface_id(id) {
            push(
                id.to_string(),
                format!(
                    "`{id}` is not a document id (no `*`, `?` or `[`, no `..` and no leading `/`)"
                ),
            );
            continue;
        }
        match c.resolve(id) {
            Res::Found(i, None, _) if c.docs[i as usize].ty == DocType::Interface => {}
            Res::Found(i, None, _) => push(
                id.to_string(),
                format!(
                    "`{id}` is a `{}` document, not an interface",
                    c.docs[i as usize].ty.name()
                ),
            ),
            Res::Ambiguous(v) => push(
                id.to_string(),
                format!("`{id}` names {} documents", v.len()),
            ),
            Res::Missing if verb == Verb::Add => {
                if id.ends_with(".md") {
                    push(
                        id.to_string(),
                        format!(
                            "`{id}` ends in `.md`, and `finish` adds `.md` to the id of a new document; write the id without `.md`"
                        ),
                    );
                    continue;
                }
                // A new document: it is at the place of its id.
                match config.layers.place_type(&format!("{id}.md")) {
                    Some(DocType::Interface) => {}
                    Some(ty) => push(
                        id.to_string(),
                        format!(
                            "the place of `{id}` gives the type `{}`, not `interface`",
                            ty.name()
                        ),
                    ),
                    None => push(
                        id.to_string(),
                        format!("the place of `{id}` gives no type, not `interface`"),
                    ),
                }
            }
            _ => push(
                id.to_string(),
                format!("`{id}` names no interface document"),
            ),
        }
    }
}

/// SD914 for an `ADD` item of `Interfaces` whose document the tree does not
/// have. `finish` and a pull request that deletes the folder of the change need
/// the document: it exists, or a delta line `ADD <id>#<key>` adds it. `head` is
/// the tree that must have the document, and `added` the ids of the valid `ADD`
/// lines of the delta.
pub fn interfaces_absent(head: &Corpus, spec: &Doc, added: &HashSet<String>) -> Vec<Diag> {
    let mut out = Vec::new();
    for item in change::section_items(spec, ItemSection::Interfaces) {
        let Some((Verb::Add, id)) = interface_item(item) else {
            continue;
        };
        if bad_interface_id(id) {
            continue;
        }
        match head.resolve(id) {
            // A document of another type is no interface: it is absent.
            Res::Found(i, None, _) if head.docs[i as usize].ty != DocType::Interface => {
                out.push(crate::check::diag(
                    spec,
                    item.line,
                    Code::DeltaInvalid,
                    id,
                    format!(
                        "the spec adds the interface `{id}`, and `{id}` is a `{}` document, not an interface",
                        head.docs[i as usize].ty.name()
                    ),
                ));
                continue;
            }
            Res::Missing if !added.contains(id) => {}
            _ => continue,
        }
        out.push(crate::check::diag(
            spec,
            item.line,
            Code::DeltaInvalid,
            id,
            format!(
                "the spec adds the interface `{id}`, and the tree has no such document; write it in this pull request, or add its requirements with `ADD {id}#<key>` lines"
            ),
        ));
    }
    out
}

/// The ids of the valid `ADD` lines of the delta of a change, as the lines
/// write them.
pub fn added_ids(ch: &Change, corpus: &Corpus) -> HashSet<String> {
    ch.delta(corpus)
        .into_iter()
        .filter_map(|l| l.parsed.ok())
        .filter(|d| d.verb == Verb::Add)
        .map(|d| d.id)
        .collect()
}

/// SD914 for an interface of `Interfaces` that the diff does not change: the
/// pull request must change the interface document, or one file that it cites.
/// `changed` tells if a path (from the repository root) is in the diff. An item
/// that `interfaces` reports (a bad form, an absent document, a document that
/// is not an interface) gives nothing here. `check` calls this function for a
/// deleted change, with the spec of the base tree and the diff against the
/// lifecycle base. `finish` calls it with the diff and the documents that its
/// own edits change.
pub fn interfaces_unchanged(c: &Corpus, spec: &Doc, changed: &dyn Fn(&str) -> bool) -> Vec<Diag> {
    let mut out = Vec::new();
    let mut cits: Option<crate::cite::Citations> = None;
    for item in change::section_items(spec, ItemSection::Interfaces) {
        let Some(id) = interface_id(item) else {
            continue;
        };
        let Res::Found(i, None, _) = c.resolve(id) else {
            continue;
        };
        let doc = &c.docs[i as usize];
        if doc.ty != DocType::Interface || changed(&doc.path) {
            continue;
        }
        let cits = cits.get_or_insert_with(|| crate::cite::Citations::build(c));
        let cited = cits
            .files
            .iter()
            .any(|(f, v)| v.iter().any(|ct| ct.doc == i) && changed(f));
        if cited {
            continue;
        }
        out.push(crate::check::diag(
            spec,
            item.line,
            Code::DeltaInvalid,
            id,
            format!(
                "the spec lists the interface `{id}`, and the diff changes neither `{}` nor a file that it cites; change the interface document, or a file that it cites, in this pull request",
                doc.path
            ),
        ));
    }
    out
}

/// The verb and the id of an item of `Interfaces`, or `None` if the item is not
/// in the form `CHANGE <interface id>: <text>` or `ADD <interface id>: <text>`.
/// The id can be in `[[ ]]` or in a code span.
fn interface_item(item: &ListItem) -> Option<(Verb, &str)> {
    let (verb, rest) = match item.text.strip_prefix("CHANGE") {
        Some(rest) => (Verb::Change, rest),
        None => (Verb::Add, item.text.strip_prefix("ADD")?),
    };
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (id, text) = rest.split_once(':')?;
    // Strip a pair of backticks, then a pair of `[[ ]]`. A single `[` stays in
    // the id, so a glob `[ab]` is not cut.
    let id = id.trim();
    let id = id
        .strip_prefix('`')
        .and_then(|r| r.strip_suffix('`'))
        .unwrap_or(id);
    let id = id
        .strip_prefix("[[")
        .and_then(|r| r.strip_suffix("]]"))
        .unwrap_or(id);
    (!id.is_empty() && !id.contains(char::is_whitespace) && !text.trim().is_empty())
        .then_some((verb, id))
}

/// True if `id` cannot be a document id: it has a glob character (`*`, `?`,
/// `[`), a `..` segment, or a leading `/`. Such an id would match more than one
/// path, so it never gets into the scope of SD913.
fn bad_interface_id(id: &str) -> bool {
    id.contains(['*', '?', '[']) || id.starts_with('/') || id.split('/').any(|s| s == "..")
}

/// The id of an item of `Interfaces` in the form of [`interface_item`], if the
/// id can be a document id. A bad id is `None`, and `interfaces` reports it.
pub(crate) fn interface_id(item: &ListItem) -> Option<&str> {
    interface_item(item)
        .map(|(_, id)| id)
        .filter(|id| !bad_interface_id(id))
}

/// SD923 and SD924: the tasks of an `approved` plan. A task needs the label
/// `Files:` with exact paths (`none` is a value), and the label `Verify:`
/// (SD923). A `Skip:` with no reason is a problem too (design 0.7.0, D40). It lists at most `max-task-files` paths (SD924). A draft plan is
/// not checked. A task whose id an earlier task has is not checked: the first
/// task with an id counts. One diagnostic of SD923 names every problem of the
/// task. The target of both codes is the id of the task. The function reads
/// one tree. `skip_early_plans` drops the diagnostics of a plan of 0.6 later,
/// with the history.
fn task_diags(config: &Config, c: &Corpus, ch: &Change, out: &mut Vec<Diag>) {
    let Some(plan) = &ch.plan else { return };
    if plan.status != "approved" {
        return;
    }
    let doc = &c.docs[plan.doc as usize];
    let limit = config.max_task_files as usize;
    for t in ch.tasks(c).iter().filter(|t| !t.duplicate) {
        let mut problems: Vec<String> = Vec::new();
        match &t.files_text {
            None => problems.push("it has no `Files:`".to_string()),
            Some(text) => {
                if t.files.is_empty() && !change::is_none_text(text) {
                    problems.push(
                        "its `Files:` lists no path; write each exact path, or `none`".to_string(),
                    );
                }
                for p in &t.files {
                    let dir = p.trim_start_matches("./").trim_end_matches('/');
                    // One definition of a glob: `*` or `?`. `[` and `]` are plain
                    // characters, as in the scope of the change (ADR 0019).
                    if crate::glob::is_glob(p) {
                        problems.push(format!("its `Files:` has the glob `{p}`"));
                    } else if p.ends_with('/') || c.dirs.contains(dir) {
                        problems.push(format!("its `Files:` has the folder `{p}`"));
                    }
                }
            }
        }
        if t.verify.is_none() {
            problems.push("it has no `Verify:`".to_string());
        }
        let labels_problem = !problems.is_empty();
        let skip_problem = t.skip.as_deref() == Some("");
        if skip_problem {
            problems.push("its `Skip:` has no reason".to_string());
        }
        if !problems.is_empty() {
            let mut fixes = Vec::new();
            if labels_problem {
                fixes.push("write the exact paths in `Files:` and the check in `Verify:`");
            }
            if skip_problem {
                fixes.push("write the reason after `Skip:`");
            }
            out.push(crate::check::diag(
                doc,
                t.line,
                Code::TaskIncomplete,
                &t.id,
                format!(
                    "the plan is `approved`, and the task `{}` is incomplete: {}; {}",
                    t.id,
                    problems.join("; "),
                    fixes.join("; ")
                ),
            ));
        }
        let mut paths: Vec<&String> = t.files.iter().collect();
        paths.sort();
        paths.dedup();
        if paths.len() > limit {
            out.push(crate::check::diag(
                doc,
                t.line,
                Code::TaskTooLarge,
                &t.id,
                format!(
                    "the plan is `approved`, and the task `{}` lists {} paths, more than `max-task-files = {limit}`; split the task",
                    t.id,
                    paths.len()
                ),
            ));
        }
    }
}

/// SD925: each item of `Concerns` and `Open questions` of a spec that has no
/// task box. SD919 and SD922 read the box, so they cannot see such an item. A
/// section that states "nothing" (`- none`) gives none. The check reads a spec
/// of each status. The caller skips the spec of a `closed` change.
fn boxes_missing(spec: &Doc, out: &mut Vec<Diag>) {
    for (section, name, code, what) in [
        (ItemSection::Concerns, "Concerns", "SD919", "concern"),
        (
            ItemSection::OpenQuestions,
            "Open questions",
            "SD922",
            "question",
        ),
    ] {
        if change::states_nothing(spec, section) {
            continue;
        }
        for item in change::section_items(spec, section).filter(|i| i.task.is_none()) {
            let text = change::shown(&item.text);
            out.push(crate::check::diag(
                spec,
                item.line,
                Code::CheckboxMissing,
                &text,
                format!(
                    "an item of `{name}` has no task box, so {code} cannot see if it is open; write `- [ ]` for an open {what} or `- [x]` for a resolved one"
                ),
            ));
        }
    }
}

/// SD919: each open concern (`- [ ]`) of a spec that is `approved`. The
/// function does not read the status: the caller does.
pub(crate) fn concerns(spec: &Doc, out: &mut Vec<Diag>) {
    for item in change::open_concerns(spec) {
        out.push(crate::check::diag(
            spec,
            item.line,
            Code::ConcernOpen,
            short(&item.text),
            format!(
                "the spec is `approved`, and the concern `{}` is open",
                short(&item.text)
            ),
        ));
    }
}

/// SD922: each open question (`- [ ]` of `Open questions`) of a spec that is
/// `approved`. `check` and `finish` use this function. It returns nothing for a
/// change with no spec, for a spec with another status, and for an item with
/// no task box or with a checked box. It reads no configuration: the caller
/// applies the severity.
pub fn question_diags(c: &Corpus, ch: &change::Change) -> Vec<Diag> {
    let Some(spec) = &ch.spec else {
        return Vec::new();
    };
    if spec.status != "approved" {
        return Vec::new();
    }
    let doc = &c.docs[spec.doc as usize];
    ch.open_questions(c)
        .into_iter()
        .map(|item| {
            crate::check::diag(
                doc,
                item.line,
                Code::QuestionOpen,
                short(&item.text),
                format!(
                    "the spec is `approved`, and the question `{}` is open",
                    short(&item.text)
                ),
            )
        })
        .collect()
}

/// SD920, the absent part: a decision of `Decisions` that names no document,
/// in a spec that is `approved`. The check that the decision is `accepted`
/// runs when `finish` runs.
fn decisions(c: &Corpus, spec: &Doc, out: &mut Vec<Diag>) {
    for item in change::section_items(spec, ItemSection::Decisions) {
        for id in decision_ids(&item.text) {
            if matches!(c.resolve(id), Res::Missing) {
                out.push(crate::check::diag(
                    spec,
                    item.line,
                    Code::DecisionNeededUnaccepted,
                    id,
                    format!("the spec is `approved`, and the decision `{id}` names no document"),
                ));
            }
        }
    }
}

/// The ids that the `[[ ]]` references of the text of an item name. A fragment
/// and a label are not part of the id.
fn decision_ids(text: &str) -> Vec<&str> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let Some(len) = rest[start + 2..].find("]]") else {
            break;
        };
        let inner = &rest[start + 2..start + 2 + len];
        rest = &rest[start + 2 + len + 2..];
        let id = inner.split(['#', '|']).next().unwrap_or("").trim();
        if !id.is_empty() {
            ids.push(id);
        }
    }
    ids
}

/// SD936 and SD937 for the section `Invariants` of the spec of a change (design
/// 0.8.0, D6). They run only with `[change] invariants` (the severity of the two
/// codes is off without the key). They read the head tree only, so `check`,
/// `finish` step 5.2 and `next` use this one function.
///
/// - SD936: an item has no valid form (backticks around `none` or a pin do
///   not count, and `[[...]]` is no form), names no document, no requirement or a
///   requirement outside a `rule` document, or names a rule that the delta of the
///   same spec adds or removes (Q45). The section has text and no list item, or
///   no text (D78). A spec in any status gives it, so the author sees it before
///   the approval.
/// - SD937: the hash of an item is not the hash of the rule now. If the delta of
///   the same spec has a `CHANGE` line for the rule, the hash of the item must be
///   the hash of that line: the line pins the old text, and after `finish` the
///   rule has the new text by design.
pub fn invariant_diags(config: &Config, c: &Corpus, ch: &Change) -> Vec<Diag> {
    let mut out = Vec::new();
    let Some(spec) = &ch.spec else { return out };
    if config.is_off(Code::InvariantInvalid) && config.is_off(Code::InvariantStale) {
        return out;
    }
    let doc = &c.docs[spec.doc as usize];
    let lines = ch.delta(c);
    let delta_of = |key: &str| -> Option<&Delta> {
        lines
            .iter()
            .filter_map(|l| l.parsed.as_ref().ok())
            .find(|d| change::same_target(c, &d.target()) == key)
    };
    // The section has no list item: a paragraph or no text. Neither declares an
    // invariant, so the author must write an item, or `- none`.
    if let Some(line) = change::invariants_heading_line(doc)
        && change::invariant_lines(doc).is_empty()
    {
        out.push(crate::check::diag(
            doc,
            line,
            Code::InvariantInvalid,
            "Invariants".to_string(),
            "the section `Invariants` has no list item before the next heading: write each item as a list item, or `- none`"
                .to_string(),
        ));
    }
    for l in change::invariant_lines(doc) {
        let pin = match l.parsed {
            Ok(None) => continue,
            Err(reason) => {
                out.push(crate::check::diag(
                    doc,
                    l.line,
                    Code::InvariantInvalid,
                    l.text,
                    format!("the item of `Invariants` is not valid: {reason}"),
                ));
                continue;
            }
            Ok(Some(p)) => p,
        };
        let target = pin.target();
        let same = change::same_target(c, &target);
        let line = delta_of(&same);
        if let Some(d) = line.filter(|d| d.verb != Verb::Change) {
            let (what, why) = if d.verb == Verb::Add {
                ("adds", "an added rule has no hash to pin")
            } else {
                ("removes", "a removed rule cannot stay true")
            };
            out.push(crate::check::diag(
                doc,
                l.line,
                Code::InvariantInvalid,
                pin.text(),
                format!(
                    "the delta of this spec {what} the rule `{target}`, so it is no invariant of this change ({why}); remove the item"
                ),
            ));
            continue;
        }
        let problem = match c.resolve(&pin.id) {
            Res::Found(d, None, _) => {
                let rule = &c.docs[d as usize];
                match rule.req(&pin.key) {
                    None => Err(format!("`{}` has no requirement `{}`", rule.id, pin.key)),
                    Some(_) if rule.ty != DocType::Rule => Err(format!(
                        "`{target}` is a requirement of a `{}` document, not a rule; an invariant names a requirement of a `rule` document",
                        rule.ty.name()
                    )),
                    Some(i) => Ok(&rule.parsed.reqs[i]),
                }
            }
            Res::Ambiguous(_) => Err(format!("`{}` names more than one document", pin.id)),
            _ => Err(format!("`{}` names no document of the tree", pin.id)),
        };
        let req = match problem {
            Ok(r) => r,
            Err(m) => {
                out.push(crate::check::diag(
                    doc,
                    l.line,
                    Code::InvariantInvalid,
                    pin.text(),
                    m,
                ));
                continue;
            }
        };
        if let Some(d) = line {
            // A `CHANGE` line: the item compares with the hash of the line.
            if d.hash.as_deref() != Some(pin.hash.as_str()) {
                out.push(crate::check::diag(
                    doc,
                    l.line,
                    Code::InvariantStale,
                    pin.text(),
                    format!(
                        "the hash `{}` of `{target}` is not the hash `{}` of the CHANGE line of this spec for the rule; the line pins the text that the change replaces, so write the hash of that line in the item",
                        pin.hash,
                        d.hash.as_deref().unwrap_or("")
                    ),
                ));
            }
        } else if !change::hash_matches(req, &pin.hash) {
            out.push(crate::check::diag(
                doc,
                l.line,
                Code::InvariantStale,
                pin.text(),
                format!(
                    "the hash `{}` of `{target}` is not the hash of the rule now (`{}`); run `spec-dna show {target}`, read the rule again, and write the new hash in the item",
                    pin.hash,
                    hex(&req.hash)
                ),
            ));
        }
    }
    out
}

/// SD939 and SD940 for the section `Deviations` of the plan of a change (design
/// 0.8.0, D7). They need no key: the record carries the deviations. `check`,
/// `finish` step 5.2 and `next` use this one function.
///
/// - SD939: an item is not `- <id> (<kind>): <text>`, has a kind outside `plan`
///   and `spec`, or repeats an id.
/// - SD940: a `spec` item names no line of the delta of the spec and no change
///   folder of the tree. The label `Delta: <verb> <doc-id>#<key>` names a line, and
///   the label `Follow-up: <slug>` names a folder. A code span reads as its text.
///   The message names the form that the item has (D78).
pub fn deviation_diags(config: &Config, c: &Corpus, ch: &Change) -> Vec<Diag> {
    let mut out = Vec::new();
    let Some(plan) = &ch.plan else { return out };
    if config.is_off(Code::DeviationInvalid) && config.is_off(Code::DeviationUnresolved) {
        return out;
    }
    let doc = &c.docs[plan.doc as usize];
    let lines = ch.delta(c);
    let folders: Vec<String> = change::collect(c)
        .into_iter()
        .filter(|o| o.dir != ch.dir)
        .map(|o| o.slug)
        .collect();
    for l in change::deviation_lines(doc) {
        let d = match l.parsed {
            Ok(d) => d,
            Err(reason) => {
                out.push(crate::check::diag(
                    doc,
                    l.line,
                    Code::DeviationInvalid,
                    l.text,
                    format!("the item of `Deviations` is not valid: {reason}"),
                ));
                continue;
            }
        };
        if d.kind != "spec" {
            continue;
        }
        let by_delta = d.fix.delta.as_ref().is_some_and(|(verb, target)| {
            let want = change::same_target(c, target);
            lines
                .iter()
                .filter_map(|l| l.parsed.as_ref().ok())
                .any(|x| x.verb == *verb && change::same_target(c, &x.target()) == want)
        });
        let by_folder = d
            .fix
            .follow_up
            .as_ref()
            .is_some_and(|s| folders.contains(s));
        if by_delta || by_folder {
            continue;
        }
        let cause = if let Some((verb, target)) = &d.fix.delta {
            format!(
                "`Delta: {} {target}` names no line of the delta of this change",
                match verb {
                    Verb::Add => "ADD",
                    Verb::Change => "CHANGE",
                    Verb::Remove => "REMOVE",
                }
            )
        } else if d.fix.delta_bad {
            format!(
                "`{}` is not `Delta: <verb> <doc-id>#<key>` with the verb ADD, CHANGE or REMOVE",
                d.fix.delta_found
            )
        } else if let Some(s) = &d.fix.follow_up {
            format!("`Follow-up: {s}` names no change folder of the tree")
        } else if d.fix.follow_up_empty {
            "`Follow-up:` has no slug after it".to_string()
        } else if let Some(w) = &d.fix.stray {
            format!(
                "`{w}` is not a label; write `Delta:` or `Follow-up:` as a word of its own, with a capital first letter and no mark before it"
            )
        } else {
            "it has no `Delta:` and no `Follow-up:`".to_string()
        };
        out.push(crate::check::diag(
            doc,
            l.line,
            Code::DeviationUnresolved,
            d.id.clone(),
            format!(
                "the `spec` deviation `{}` is not resolved: {cause}; write `Delta: <verb> <doc-id>#<key>` with a line that the delta of the spec has, or `Follow-up: <slug>` with a change folder of the tree",
                d.id
            ),
        ));
    }
    out
}

/// SD912: more changes than `max-started` of the `intent` layer have an
/// `accepted` intent. The diagnostic is on `spec-dna.toml`, as the one of SD902
/// is, so the rules of a root do not apply.
fn started_limit(config: &Config, layer: Option<usize>, changes: &[change::Change]) -> Vec<Diag> {
    let Some(layer) = layer.map(|i| &config.layers.layers()[i]) else {
        return Vec::new();
    };
    let Some(max) = layer.max_started else {
        return Vec::new();
    };
    let Some(severity) = config.severity(Code::StartedLimit, None) else {
        return Vec::new();
    };
    let started = changes
        .iter()
        .filter(|ch| ch.intent_status() == "accepted")
        .count();
    if started <= max as usize {
        return Vec::new();
    }
    vec![Diag {
        path: crate::config::FILE_NAME.to_string(),
        line: 1,
        code: Code::StartedLimit,
        doc: crate::config::FILE_NAME.to_string(),
        target: format!("{}:max-started", layer.name),
        message: format!(
            "{started} changes have an `accepted` intent, more than `max-started = {max}` of the layer `{}`",
            layer.name
        ),
        severity,
    }]
}

/// The folders of the `intent` layer from the repository root, one for each
/// root, with no final `/`. A project with no `intent` layer has none.
pub fn intent_dirs(config: &Config) -> Vec<String> {
    let Some(dir) = config
        .layers
        .layer_of_type(DocType::Intent)
        .and_then(|i| config.layers.layers()[i].dir())
    else {
        return Vec::new();
    };
    let mut out: Vec<String> = config
        .roots
        .iter()
        .map(|r| {
            if r.path.is_empty() {
                dir.to_string()
            } else {
                format!("{}/{dir}", r.path)
            }
        })
        .collect();
    out.dedup();
    out
}

/// True for a document whose requirements spec-dna reads: a `rule`,
/// `requirements` or `interface` document with a valid type, which is not a
/// stub and not a generated index.
pub fn requirement_doc(d: &Doc) -> bool {
    d.ty.has_requirements() && !d.type_failed && d.moved_to.is_none() && d.generated.is_none()
}

/// The number of changes with an `accepted` intent (SD912).
pub fn started_count(c: &Corpus) -> usize {
    change::collect(c)
        .iter()
        .filter(|ch| ch.intent_status() == "accepted")
        .count()
}

/// The result of `base_checks`.
#[derive(Default)]
pub struct BaseReport {
    /// The diagnostics of the diff. All of them are new.
    pub diags: Vec<Diag>,
    /// The `Files` patterns of the changes whose plan covers a document of the
    /// diff. A document that one of them matches gives no SD921 (D19).
    pub exempt: Vec<String>,
    /// True if a commit of the branch added a change and the branch finished
    /// it (B5). Such a change is a change of the diff, so SD921 stays silent.
    pub born: bool,
    /// The approval commits that cleared a gate of SD910 (`check --json`,
    /// `approvals`), in the order of the walk. It is empty with
    /// `approval = "base"`.
    pub approvals: Vec<Approval>,
}

/// How a change came into the diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// The diff touches a file of its folder.
    Touched,
    /// A commit of the branch added its folder, and neither tree has it (B5).
    Born,
    /// Its `Files` match a path of the diff, and its folder is not in the diff.
    FilesMatch,
}

/// The tree that holds the artifacts of a change of the diff.
enum Side {
    Head,
    /// The diff deletes the folder, so the checks read the base tree.
    Base,
    /// The newest artifacts of a branch-born change, read from history.
    Born(Box<Corpus>),
}

/// A change of the diff (design K2), with the lists that the checks use.
struct DiffChange {
    change: Change,
    origin: Origin,
    side: Side,
    /// The patterns of the plan (`Change::files`), from the tree of `side`.
    files: Vec<String>,
    /// The items of `Files` that are not paths, as the plan has them.
    not_paths: Vec<String>,
    /// True if `Files` has a paragraph and no list item, and the paragraph
    /// does not state "no files" (D28).
    paragraph: bool,
    /// The scope of SD913 from the tree of `side`: the folder, the patterns of
    /// the plan, and the documents that the delta and `Interfaces` name.
    scope: Vec<String>,
    /// What the newest text of the branch adds to the scope, if the diff deletes
    /// the folder. It is read when SD913 needs it ([`DiffChange::branch`]).
    branch: OnceCell<Option<BranchScope>>,
    /// The number of items in `Delta`.
    delta_count: usize,
    /// The ids that the valid delta lines name.
    targets: Vec<String>,
}

/// The paths of the documents of both trees, by kind.
#[derive(Default)]
struct DocPaths {
    /// A document that a layer owns.
    owned: HashSet<String>,
    /// A generated index.
    generated: HashSet<String>,
    /// A document that has requirements.
    requirements: HashSet<String>,
    /// A roll-up document (design 0.8.0, D54): a summary, and a `system` or
    /// `product` document with `rests-on`. The head decides. A roll-up document
    /// that the diff deletes counts through the base.
    rollup: HashSet<String>,
}

struct Ctx<'a> {
    config: &'a Config,
    git: &'a Git,
    head: &'a Corpus,
    base: &'a Corpus,
    base_rev: &'a str,
    diff: &'a Diff,
    head_layers: Vec<Option<usize>>,
    base_layers: Vec<Option<usize>>,
    paths: OnceCell<DocPaths>,
}

impl<'a> Ctx<'a> {
    fn corpus<'s>(&'s self, ch: &'s DiffChange) -> &'s Corpus {
        match &ch.side {
            Side::Head => self.head,
            Side::Base => self.base,
            Side::Born(c) => c,
        }
    }

    /// The paths of the documents that an id names: the documents of the trees,
    /// or the places that a new document with that id can have.
    fn named_paths(&self, id: &str) -> Vec<String> {
        let mut v = Vec::new();
        for c in [self.head, self.base] {
            if let Res::Found(i, None, _) = c.resolve(id) {
                v.push(c.docs[i as usize].path.clone());
            }
        }
        if v.is_empty() {
            for r in &self.config.roots {
                v.push(if r.path.is_empty() {
                    format!("{id}.md")
                } else {
                    format!("{}/{id}.md", r.path)
                });
            }
        }
        v
    }

    fn paths(&self) -> &DocPaths {
        self.paths.get_or_init(|| {
            let mut out = DocPaths::default();
            for (c, layers) in [
                (self.head, &self.head_layers),
                (self.base, &self.base_layers),
            ] {
                for (d, layer) in c.docs.iter().zip(layers) {
                    if layer.is_some() {
                        out.owned.insert(d.path.clone());
                    }
                    if d.generated.is_some() {
                        out.generated.insert(d.path.clone());
                    }
                    if requirement_doc(d) {
                        out.requirements.insert(d.path.clone());
                    }
                }
            }
            for d in &self.head.docs {
                if crate::rollup::is_rollup(d) {
                    out.rollup.insert(d.path.clone());
                }
            }
            for d in &self.base.docs {
                if crate::rollup::is_rollup(d) && !self.head.by_path.contains_key(&d.path) {
                    out.rollup.insert(d.path.clone());
                }
            }
            out
        })
    }
}

/// Runs the checks of the diff against the lifecycle base: SD905 (the part for
/// a change with no delta), SD910, SD913, SD914 (the interface that the diff
/// does not change), SD915, SD916, SD917, SD918 and SD920 (the decision that is
/// not `accepted`). It also finds the data for SD921. `base` is the lifecycle tree, `base_rev` its commit, and `diff` the
/// diff of the head against it. The checks read the artifacts of a change from
/// the head tree, and from the base tree if the diff deletes the folder. With
/// `approval = "base"` they read approvals from the base tree only, so an
/// approval in the same pull request does not count. With `approval =
/// "branch"`, SD910 also reads the commits of `base_rev..HEAD`
/// ([`crate::approval`]).
pub fn base_checks(
    config: &Config,
    git: &Git,
    head: &Corpus,
    base: &Corpus,
    base_rev: &str,
    diff: &Diff,
) -> Result<BaseReport, String> {
    let on = |k: Code| !config.is_off(k);
    // The codes of the group `acceptance` follow the configuration of the need:
    // the head, or the base when the head turns the acceptance off (D70 item 6).
    // So a branch cannot skip a record with its own `profile`, `[rules]` or
    // `[change]`.
    let nc = crate::acceptance::need_config(config, git, base_rev)?;
    let acceptance_on = crate::acceptance::CODES
        .iter()
        .any(|&k| !nc.gate().is_off(k));
    if !acceptance_on
        && (config.change_policy == ChangePolicy::None
            || ![
                Code::FanOut,
                Code::GateApprovalMissing,
                Code::OutsidePlan,
                Code::ClassTooLow,
                Code::TruthChangedWithoutChange,
                Code::FinishIncomplete,
                Code::DeltaEvidenceMissing,
                Code::DecisionNeededUnaccepted,
                Code::DeltaInvalid,
                Code::IncidentEvalMissing,
            ]
            .iter()
            .any(|&k| on(k)))
    {
        return Ok(BaseReport::default());
    }
    let ctx = Ctx {
        config,
        git,
        head,
        base,
        base_rev,
        diff,
        head_layers: crate::check::doc_layers(config, head),
        base_layers: crate::check::doc_layers(config, base),
        paths: OnceCell::new(),
    };
    let changes = diff_changes(&ctx)?;
    let mut out = Vec::new();
    let mut approvals = Vec::new();
    if on(Code::GateApprovalMissing) {
        let branch = (config.change_approval == config::Approval::Branch)
            .then(|| Branch::read(config, git, base_rev, base, head));
        let (diags, used) = approval_missing(&ctx, &changes, branch.as_ref());
        out.extend(diags);
        approvals = used;
    }
    if on(Code::OutsidePlan) {
        out.extend(outside_plan(&ctx, &changes));
    }
    if on(Code::ClassTooLow) {
        out.extend(class_too_low(&ctx, &changes));
    }
    if on(Code::FanOut) {
        out.extend(routine_fan_out(&ctx, &changes));
    }
    out.extend(finished_checks(&ctx, &changes, &nc)?);
    out.extend(record_checks(&ctx, &changes, &nc)?);
    if on(Code::IncidentEvalMissing) {
        out.extend(incident_eval_missing(&ctx)?);
    }
    // A plan that covers a document: the plan of a change in the diff, and the
    // approved plan of a started change in the base.
    let mut exempt: Vec<String> = changes
        .iter()
        .filter(|c| c.origin != Origin::FilesMatch)
        .flat_map(|c| c.files.iter().cloned())
        .collect();
    for b in change::collect(base) {
        if b.intent_status() == "accepted" && plan_status(&b) == "approved" {
            exempt.extend(b.files(base));
        }
    }
    // The codes of the group `acceptance` take their severity from the
    // configuration of the need; the other codes take it from the head.
    let (acceptance, other): (Vec<Diag>, Vec<Diag>) = out
        .into_iter()
        .partition(|d| config::is_acceptance_code(d.code));
    let mut diags = crate::check::apply_severity(config, other);
    diags.extend(crate::check::apply_severity(nc.gate(), acceptance));
    Ok(BaseReport {
        diags,
        exempt,
        born: changes.iter().any(|c| c.origin == Origin::Born),
        approvals,
    })
}

/// SD918: the diff adds a document of type `incident`, and adds or changes no
/// `*.json` file directly in `[evals] dir`. It runs only for a project with
/// `[evals]`. An add is a path that the diff adds, that git does not detect as
/// the target of a rename, and whose id and aliases the base does not have. So
/// a move is not an add, also a move to a path with another id. A deleted eval does not count
/// (design 0.6.0, D53, D95). One diagnostic for each added incident, on line 1,
/// with its id as the target.
fn incident_eval_missing(ctx: &Ctx) -> Result<Vec<Diag>, String> {
    let Some(evals) = &ctx.config.evals else {
        return Ok(Vec::new());
    };
    let prefix = format!("{}/", evals.dir);
    let has_eval = ctx.diff.files.iter().any(|(p, k)| {
        *k != PathChange::Deleted
            && p.strip_prefix(&prefix)
                .is_some_and(|n| n.ends_with(".json") && !n.contains('/'))
    });
    if has_eval {
        return Ok(Vec::new());
    }
    let added: Vec<&Doc> = ctx
        .head
        .docs
        .iter()
        .filter(|d| d.ty == DocType::Incident)
        .filter(|d| ctx.diff.kind(&d.path) == Some(PathChange::Added))
        .filter(|d| {
            // The id of the document, or an alias that it keeps (`spec-dna mv`
            // writes the old id as an alias), is a document of the base.
            std::iter::once(&d.id)
                .chain(&d.aliases)
                .chain(&d.legacy_aliases)
                .all(|id| matches!(ctx.base.resolve(id), Res::Missing))
        })
        .collect();
    if added.is_empty() {
        return Ok(Vec::new());
    }
    let renamed = ctx.git.renamed_paths(ctx.base_rev)?;
    Ok(added
        .into_iter()
        .filter(|d| !renamed.contains(&d.path))
        .map(|d| Diag {
            path: d.path.clone(),
            line: 1,
            code: Code::IncidentEvalMissing,
            doc: d.id.clone(),
            target: d.id.clone(),
            message: format!(
                "the change adds the incident `{}` and no eval file (`{}<id>.json`); add an eval for the incident in the same change",
                d.id, prefix
            ),
            severity: Code::IncidentEvalMissing.severity(),
        })
        .collect())
}

fn plan_status(c: &Change) -> &str {
    c.plan.as_ref().map_or("", |a| &a.status)
}

fn spec_status(c: &Change) -> &str {
    c.spec.as_ref().map_or("", |a| &a.status)
}

/// A diagnostic on a path that is not a document.
fn path_diag(path: &str, code: Code, target: &str, message: String) -> Diag {
    Diag {
        path: path.to_string(),
        line: 1,
        code,
        doc: path.to_string(),
        target: target.to_string(),
        message,
        severity: code.severity(),
    }
}

/// The changes of the diff (design K2): the folders that the diff touches, the
/// changes that a commit of the branch added and finished (B5), and the changes
/// whose `Files` match a path of the diff. A diff with none is a small fix.
fn diff_changes(ctx: &Ctx) -> Result<Vec<DiffChange>, String> {
    let head_changes = change::collect(ctx.head);
    let base_changes = change::collect(ctx.base);
    let mut out: Vec<DiffChange> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (list, base_side) in [(&head_changes, false), (&base_changes, true)] {
        for ch in list {
            if !seen.contains(&ch.dir) && ctx.diff.under(&ch.dir).next().is_some() {
                seen.insert(ch.dir.clone());
                let side = if base_side { Side::Base } else { Side::Head };
                out.push(make(ctx, ch.clone(), Origin::Touched, side));
            }
        }
    }
    // The tip of the branch: on the test merge of a code host, the second parent
    // of `HEAD`, so the range holds the commits of the branch (`Git::branch_tip`).
    let tip = ctx.git.branch_tip(ctx.base_rev);
    for dir in intent_dirs(ctx.config) {
        for folder in ctx.git.added_intents(ctx.base_rev, &tip, &dir)? {
            if seen.insert(folder.clone()) {
                let (change, corpus) = born_change(ctx, &folder);
                out.push(make(ctx, change, Origin::Born, Side::Born(corpus)));
            }
        }
    }
    for ch in &head_changes {
        if seen.contains(&ch.dir) {
            continue;
        }
        let files = ch.files(ctx.head);
        if ctx
            .diff
            .files
            .iter()
            .any(|(p, _)| change::in_scope(&files, p))
        {
            seen.insert(ch.dir.clone());
            out.push(make(ctx, ch.clone(), Origin::FilesMatch, Side::Head));
        }
    }
    Ok(out)
}

/// What the newest text of a change on the branch adds to the scope of SD913.
struct BranchScope {
    /// True if the newest text of the branch has a `plan.md`.
    has_plan: bool,
    /// The patterns of the plan of the branch.
    files: Vec<String>,
    /// The items of `Files` of the plan of the branch that are not paths.
    not_paths: Vec<String>,
    /// True if `Files` of the plan of the branch has a paragraph and no list
    /// item (D28).
    paragraph: bool,
    /// The documents that its delta and `Interfaces` name.
    scope: Vec<String>,
    /// The facts of the plan of the branch for the rules of a record. `finish`
    /// reads the same plan, before it deletes the folder.
    facts: crate::acceptance::PlanFacts,
    /// The plan of the branch as the stale rule of a record reads it.
    plan: crate::acceptance::PlanScope,
    /// The invariants and the deviations that the newest spec and plan of the
    /// branch declare. The base plan can lack a deviation that the work added, so
    /// SD938 reads these.
    declared: Result<crate::accept::Declared, String>,
    /// The tasks of the plan of the branch. SD915 reads them for a change with
    /// no delta.
    tasks: Vec<change::PlanTask>,
}

impl DiffChange {
    /// The newest text of the branch before the deletion of the folder. It is
    /// read from git on the first call only. A change that the diff does not
    /// delete has none: the head tree is its newest text.
    fn branch(&self, ctx: &Ctx) -> Option<&BranchScope> {
        self.branch
            .get_or_init(|| {
                if !matches!(self.side, Side::Base) {
                    return None;
                }
                let (newest, corpus) = branch_version(ctx, &self.change.dir)?;
                let mut scope = Vec::new();
                scope_paths(ctx, &newest, &corpus, &mut scope, None);
                Some(BranchScope {
                    has_plan: newest.plan.is_some(),
                    files: newest.files(&corpus),
                    not_paths: newest.files_not_paths(&corpus),
                    paragraph: newest.files_paragraph_only(&corpus),
                    scope,
                    facts: crate::acceptance::plan_facts(&newest, &corpus),
                    plan: crate::acceptance::PlanScope::of(&newest, &corpus),
                    declared: crate::acceptance::declared_for(
                        ctx.config,
                        ctx.git,
                        Some(ctx.base_rev),
                        &newest,
                        &corpus,
                    ),
                    tasks: newest.tasks(&corpus),
                })
            })
            .as_ref()
    }

    /// True if the change has a `plan.md` in the tree that the checks read for
    /// it, or in the base tree. A change that `Files` matched has one: its
    /// `Files` come from its plan. A branch that deletes the `plan.md` of the
    /// base does not turn SD913 off. The branch is read only if no tree has a
    /// plan.
    fn has_plan(&self, ctx: &Ctx) -> bool {
        self.origin == Origin::FilesMatch
            || self.change.plan.is_some()
            || ctx
                .base
                .by_path
                .contains_key(&format!("{}/plan.md", self.change.dir))
            || self.branch(ctx).is_some_and(|b| b.has_plan)
    }

    /// True if `path` is in the scope of SD913. The first test reads the tree
    /// of `side` only. The branch is read only if it fails.
    fn covers(&self, ctx: &Ctx, path: &str) -> bool {
        change::in_scope(&self.scope, path)
            || self.branch(ctx).is_some_and(|b| {
                change::in_scope(&b.scope, path) || change::in_scope(&b.files, path)
            })
    }
}

fn make(ctx: &Ctx, change: Change, origin: Origin, side: Side) -> DiffChange {
    let corpus: &Corpus = match &side {
        Side::Head => ctx.head,
        Side::Base => ctx.base,
        Side::Born(c) => c,
    };
    let files = change.files(corpus);
    let delta_count = change.delta(corpus).len();
    let mut scope = vec![format!("{}/", change.dir)];
    let mut targets = Vec::new();
    let not_paths = change.files_not_paths(corpus);
    let paragraph = change.files_paragraph_only(corpus);
    scope_paths(ctx, &change, corpus, &mut scope, Some(&mut targets));
    scope.extend(files.iter().cloned());
    DiffChange {
        delta_count,
        change,
        origin,
        side,
        files,
        not_paths,
        paragraph,
        scope,
        branch: OnceCell::new(),
        targets,
    }
}

/// Adds the documents that the delta and `Interfaces` of a change name to
/// `scope`, and the ids of the valid delta lines to `targets`.
fn scope_paths(
    ctx: &Ctx,
    change: &Change,
    corpus: &Corpus,
    scope: &mut Vec<String>,
    mut targets: Option<&mut Vec<String>>,
) {
    for l in &change.delta(corpus) {
        if let Ok(d) = &l.parsed {
            scope.extend(ctx.named_paths(&d.id));
            if let Some(t) = targets.as_deref_mut() {
                t.push(d.id.clone());
            }
        }
    }
    if let Some(a) = &change.spec {
        let doc = &corpus.docs[a.doc as usize];
        for item in change::section_items(doc, ItemSection::Interfaces) {
            if let Some(id) = interface_id(item) {
                scope.extend(ctx.named_paths(id));
            }
        }
    }
}

/// The newest artifacts of a change whose folder the diff deletes, as the
/// branch has them before the deletion. For each of `intent.md`, `spec.md` and
/// `plan.md`, the last commit of the branch (`<base>..<tip>`,
/// [`Git::branch_tip`]) that touched the file gives the
/// text: the file in that commit, or, if the commit deleted it, in its first
/// parent. `None` if the branch did not touch the folder, or git cannot give
/// the text (a shallow clone).
///
/// It reads committed text of the branch only. A partial clone without the
/// blobs, an edit that no commit holds, and an edit that a side branch made and
/// merged after `finish` give no text. The scope then comes from the base tree
/// alone, and SD913 can fire.
fn branch_version(ctx: &Ctx, folder: &str) -> Option<(Change, Corpus)> {
    let range = format!("{}..{}", ctx.base_rev, ctx.git.branch_tip(ctx.base_rev));
    let mut specs: Vec<(String, String)> = Vec::new();
    for name in ["intent.md", "spec.md", "plan.md"] {
        let path = format!("{folder}/{name}");
        let log = ctx.git.file_log(&range, &path).ok()?;
        if let Some((last, _)) = log.first() {
            specs.push((last.clone(), path.clone()));
            specs.push((format!("{last}^"), path));
        }
    }
    if specs.is_empty() {
        return None;
    }
    let texts = ctx.git.read_at(&specs).ok()?;
    let docs: Vec<(String, Arc<ParsedDoc>)> = specs
        .chunks(2)
        .zip(texts.chunks(2))
        .filter_map(|(s, t)| {
            let bytes = t[0].as_ref().or(t[1].as_ref())?;
            Some((
                s[0].1.clone(),
                Arc::new(parse(&String::from_utf8_lossy(bytes))),
            ))
        })
        .collect();
    if docs.is_empty() {
        return None;
    }
    let corpus = Corpus::build(ctx.config, docs.iter().map(|d| d.0.clone()).collect(), docs);
    let change = change::collect(&corpus)
        .into_iter()
        .find(|c| c.dir == folder)?;
    Some((change, corpus))
}

/// The newest artifacts of a change that a commit of the branch added and
/// deleted: the tree before its last commit. If git cannot give them (a shallow
/// clone, a merge), the change has no artifact, and the checks that need one
/// report the change by its folder.
fn born_change(ctx: &Ctx, folder: &str) -> (Change, Box<Corpus>) {
    let corpus = born_corpus(ctx, folder)
        .unwrap_or_else(|| Corpus::build(ctx.config, Vec::new(), Vec::new()));
    let change = change::collect(&corpus)
        .into_iter()
        .find(|c| c.dir == folder)
        .unwrap_or_else(|| Change {
            slug: folder.rsplit('/').next().unwrap_or(folder).to_string(),
            dir: folder.to_string(),
            intent: None,
            spec: None,
            plan: None,
            class: Class::Routine,
            class_unset: false,
            areas: Vec::new(),
            from: None,
        });
    (change, Box::new(corpus))
}

fn born_corpus(ctx: &Ctx, folder: &str) -> Option<Corpus> {
    let range = format!("{}..{}", ctx.base_rev, ctx.git.branch_tip(ctx.base_rev));
    let log = ctx
        .git
        .file_log(&range, &format!("{folder}/intent.md"))
        .ok()?;
    let (last, _) = log.first()?;
    let tree = ctx.git.ls_tree(&format!("{last}^")).ok()?;
    let prefix = format!("{folder}/");
    let wanted: Vec<(String, String)> = tree
        .into_iter()
        .filter(|(p, _, sub)| {
            !sub && p
                .strip_prefix(&prefix)
                .is_some_and(|n| matches!(n, "intent.md" | "spec.md" | "plan.md"))
        })
        .map(|(p, oid, _)| (p, oid))
        .collect();
    let ids: Vec<String> = wanted.iter().map(|w| w.1.clone()).collect();
    let blobs = ctx.git.read_blobs(&ids).ok()?;
    let docs: Vec<(String, Arc<ParsedDoc>)> = wanted
        .iter()
        .filter_map(|(p, oid)| {
            let bytes = blobs.get(oid)?;
            Some((p.clone(), Arc::new(parse(&String::from_utf8_lossy(bytes)))))
        })
        .collect();
    if docs.is_empty() {
        return None;
    }
    Some(Corpus::build(
        ctx.config,
        docs.iter().map(|d| d.0.clone()).collect(),
        docs,
    ))
}

/// The end of the message of an SD910 finding. With `approval = "base"` it is
/// the text of 0.6 and the reason 1. With the branch rule it is the sentence of
/// the reason.
fn sd910_tail(branch: Option<&Branch>, base_text: &str, fail: &Fail) -> String {
    match branch {
        Some(_) => format!("; {}", fail.sentence()),
        None => format!("; {base_text} (reason {})", Reason::NoBase.number()),
    }
}

/// SD910. A change adds or edits an artifact of a stage that is not `draft`, and
/// the base tree does not show the approval of the artifact before it. Also: code
/// under the `Files` of a change, and no such change has an `approved` plan in
/// the base tree. With `approval = "branch"`, an approval commit of the branch
/// before the work also counts (D11), the statuses that the branch sets need an
/// approval commit (the last approval), and the message names one reason of 9
/// (D17 and ADR 0019). The second value is the approval commits that cleared a gate.
fn approval_missing(
    ctx: &Ctx,
    changes: &[DiffChange],
    branch: Option<&Branch>,
) -> (Vec<Diag>, Vec<Approval>) {
    let base_changes = change::collect(ctx.base);
    let base_of = |dir: &str| base_changes.iter().find(|b| b.dir == dir);
    // The walk of the branch, if it counts approvals. A stopped walk gives the
    // result of 0.6 with the reason of the stop.
    let walk = branch.filter(|b| b.stopped().is_none());
    let stop = branch.and_then(|b| b.stopped());
    let mut out = Vec::new();
    let mut used: Vec<Approval> = Vec::new();
    for ch in changes {
        let corpus = ctx.corpus(ch);
        let c = &ch.change;
        if ch.origin == Origin::Born {
            // The branch added the change and finished it, so the base tree
            // has no approval of any artifact. With the branch rule the
            // approval commits do not help (D16).
            let message = match branch {
                Some(_) => format!(
                    "the change `{}` was added and finished in this branch, and the base tree has no approval of it; {}",
                    c.slug,
                    Reason::Born.sentence(None, None)
                ),
                None => format!(
                    "the change `{}` was added and finished in this branch, and the base tree has no approval of it; get the approval of each artifact into the base first (reason {})",
                    c.slug,
                    Reason::Born.number()
                ),
            };
            let path = format!("{}/intent.md", c.dir);
            out.push(match &c.intent {
                Some(a) => crate::check::diag(
                    &corpus.docs[a.doc as usize],
                    1,
                    Code::GateApprovalMissing,
                    "",
                    message,
                ),
                None => path_diag(&path, Code::GateApprovalMissing, "", message),
            });
            continue;
        }
        if matches!(ch.side, Side::Base) {
            continue;
        }
        let base_ch = base_of(&c.dir);
        for (name, art) in [("spec.md", &c.spec), ("plan.md", &c.plan)] {
            let Some(a) = art else { continue };
            let d = &corpus.docs[a.doc as usize];
            if !matches!(
                ctx.diff.kind(&d.path),
                Some(PathChange::Added | PathChange::Modified)
            ) {
                continue;
            }
            let (need, status) = if name == "spec.md" || c.class != Class::HigherRisk {
                ("intent.md", "accepted")
            } else {
                ("spec.md", "approved")
            };
            let fail = match walk {
                // The walk checks the artifact at each commit after which its
                // status is not `draft`.
                Some(w) => match w.artifact(&d.path, &format!("{}/{need}", c.dir), status) {
                    Ok(v) => {
                        used.extend(v);
                        None
                    }
                    Err(f) => Some(f),
                },
                None => {
                    // A `draft` artifact needs no approval of the artifact
                    // before it (design 0.7.0, section 15, item 1).
                    if a.status == "draft" {
                        continue;
                    }
                    let ok = base_ch.is_some_and(|b| match need {
                        "intent.md" => b.intent_status() == "accepted",
                        _ => spec_status(b) == "approved",
                    });
                    (!ok).then(|| stop.cloned().unwrap_or_else(|| Fail::new(Reason::NoBase)))
                }
            };
            if let Some(f) = fail {
                out.push(crate::check::diag(
                    d,
                    1,
                    Code::GateApprovalMissing,
                    need,
                    format!(
                        "`{name}` changes, and the base tree has no `{status}` `{need}` for the change `{}`{}",
                        c.slug,
                        sd910_tail(
                            branch,
                            "get the approval of the earlier artifact into the base first",
                            &f
                        )
                    ),
                ));
            }
        }
        // The rule of the last approval (D11): each status that the branch
        // sets needs an approval commit that set it.
        let Some(w) = walk else { continue };
        if !matches!(ch.side, Side::Head) {
            continue;
        }
        for (name, art) in [
            ("intent.md", &c.intent),
            ("spec.md", &c.spec),
            ("plan.md", &c.plan),
        ] {
            let Some(a) = art else { continue };
            let d = &corpus.docs[a.doc as usize];
            if !matches!(
                ctx.diff.kind(&d.path),
                Some(PathChange::Added | PathChange::Modified)
            ) || !matches!(a.status.as_str(), "accepted" | "approved")
                || w.base_status(&d.path) == Some(a.status.as_str())
            {
                continue;
            }
            match w.last(&d.path, &a.status) {
                Ok(v) => used.extend(v),
                Err(f) => out.push(crate::check::diag(
                    d,
                    1,
                    Code::GateApprovalMissing,
                    "status",
                    format!(
                        "`{name}` has the status `{}`, which differs from the base tree, and no approval commit set it{}",
                        a.status,
                        sd910_tail(branch, "", &f)
                    ),
                )),
            }
        }
    }
    // Code under the `Files` of a plan that the base tree does not approve.
    let approved = |ch: &&DiffChange| {
        ch.origin != Origin::Born
            && base_of(&ch.change.dir).is_some_and(|b| plan_status(b) == "approved")
    };
    for (p, _) in &ctx.diff.files {
        if changes
            .iter()
            .any(|c| p.starts_with(&format!("{}/", c.change.dir)))
        {
            continue;
        }
        let matching: Vec<&DiffChange> = changes
            .iter()
            .filter(|c| change::in_scope(&c.files, p))
            .collect();
        if matching.is_empty()
            || matching.iter().all(|c| c.origin == Origin::Born)
            || ctx.paths().owned.contains(p)
        {
            continue;
        }
        let mut first_fail: Option<Fail> = None;
        let mut cleared = false;
        match walk {
            Some(w) => {
                for m in matching.iter().filter(|m| m.origin != Origin::Born) {
                    let plan_path = match &m.change.plan {
                        Some(a) => ctx.corpus(m).docs[a.doc as usize].path.clone(),
                        None => format!("{}/plan.md", m.change.dir),
                    };
                    match w.code(&plan_path, p) {
                        Ok(v) => {
                            used.extend(v);
                            cleared = true;
                            break;
                        }
                        Err(f) => {
                            first_fail.get_or_insert(f);
                        }
                    }
                }
            }
            None => cleared = matching.iter().any(approved),
        }
        if cleared {
            continue;
        }
        let first = matching
            .iter()
            .find(|c| c.origin != Origin::Born)
            .unwrap_or(&matching[0]);
        let doc = match &first.change.plan {
            Some(a) => ctx.corpus(first).docs[a.doc as usize].id.clone(),
            None => first.change.slug.clone(),
        };
        let fail = first_fail
            .or_else(|| stop.cloned())
            .unwrap_or_else(|| Fail::new(Reason::NoBase));
        out.push(Diag {
            doc,
            ..path_diag(
                p,
                Code::GateApprovalMissing,
                p,
                format!(
                    "`{p}` is under `Files` of the change `{}`, and the base tree has no `approved` plan for it{}",
                    first.change.slug,
                    sd910_tail(
                        branch,
                        "get the approval of the plan into the base first",
                        &fail
                    )
                ),
            )
        });
    }
    used.sort_by_key(|a| a.order);
    used.dedup_by(|a, b| a.commit == b.commit);
    (out, used)
}

/// SD913: a path of the diff is outside the scope of each change of the diff.
/// It runs only if a change of the diff has a plan and the diff touches its
/// folder or the branch added it (D92): before a plan exists, a change has no
/// `Files`, so it has no scope to check against. A change that only its
/// `Files` bring into the diff does not start the check, but its `Files` stay
/// in the scope. It does not
/// report a file that SD503 does not count (`init::is_tool_file`), a roll-up
/// document (D54: `execute` must not stop and ask for a summary that is due),
/// or a path that a glob of `[change] shared-paths` matches.
fn outside_plan(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    if !changes
        .iter()
        .any(|c| c.origin != Origin::FilesMatch && c.has_plan(ctx))
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    // A plan with items that are not paths, and no path at all, has no scope.
    let cause = OnceCell::new();
    for (p, _) in &ctx.diff.files {
        // The scope of the trees first: git reads the branch only for a path
        // that no tree covers.
        if changes.iter().any(|c| change::in_scope(&c.scope, p))
            || ctx.paths().generated.contains(p)
            || crate::init::is_tool_file(ctx.config, p)
            || crate::acceptance::is_archive_path(p)
            || ctx.paths().rollup.contains(p)
            || shared_path(ctx.config, p)
            || changes.iter().any(|c| c.covers(ctx, p))
        {
            continue;
        }
        let cause: &String = cause.get_or_init(|| {
            changes
                .iter()
                .find_map(|c| not_paths_cause(c, c.branch(ctx)))
                .unwrap_or_default()
        });
        out.push(path_diag(
            p,
            Code::OutsidePlan,
            "",
            format!(
                "`{p}` is in the diff, and no `Files` item of a change in the diff matches it; add it to `Files` of the plan, or move the edit to a pull request of its own{cause}"
            ),
        ));
    }
    out
}

/// The note of a message when the plan of a change has no path at all, and an
/// item of `Files` that is not a path, or a paragraph and no list item (D28). A
/// plan that has a path gives no note: the path that the message is about is
/// not in it for another reason, and an unrelated item would mislead.
fn not_paths_cause(ch: &DiffChange, branch: Option<&BranchScope>) -> Option<String> {
    if !ch.files.is_empty() || branch.is_some_and(|b| !b.files.is_empty()) {
        return None;
    }
    let first = ch
        .not_paths
        .first()
        .or_else(|| branch.and_then(|b| b.not_paths.first()));
    match first {
        Some(first) => Some(not_paths_note(&ch.change.slug, first)),
        None if ch.paragraph || branch.is_some_and(|b| b.paragraph) => Some(format!(
            "; the plan of the change `{}`: {}",
            ch.change.slug,
            change::FILES_PARAGRAPH_LINE
        )),
        None => None,
    }
}

/// The note for the first item of `Files` that is not a path.
fn not_paths_note(slug: &str, item: &str) -> String {
    format!(
        "; the `Files` of the change `{slug}` has an item that is not a path (the first is \"{item}\"): write each path or glob in a code span"
    )
}

/// True if a glob of `shared-paths` matches the path.
fn shared_path(config: &Config, path: &str) -> bool {
    let path = path.trim_start_matches("./").trim_start_matches('/');
    config
        .shared_paths
        .iter()
        .any(|g| crate::glob::matches(g, path))
}

/// True if a glob of `high-risk-paths` matches the path.
fn high_risk_path(config: &Config, path: &str) -> Option<String> {
    first_glob(&config.high_risk_paths, path)
}

/// The first glob of `globs` that matches `path`.
fn first_glob(globs: &[String], path: &str) -> Option<String> {
    let path = path.trim_start_matches("./").trim_start_matches('/');
    globs
        .iter()
        .find(|g| crate::glob::matches(g, path))
        .cloned()
}

/// What the minimum class of a change reads (SD917, design 0.6.0): the globs of
/// `high-risk-paths`, the layers of `high-risk-layers`, the 2 trees and the diff.
/// `class_too_low` builds it with the lists of the head configuration. The
/// acceptance builds it with the lists of the need ([`crate::acceptance::need_config`]),
/// so SD917 and the class of the record share one rule (D70 item 2).
pub struct MinimumFacts<'a> {
    pub config: &'a Config,
    pub high_risk_layers: &'a [String],
    pub high_risk_paths: &'a [String],
    pub head: &'a Corpus,
    pub base: &'a Corpus,
    pub head_layers: &'a [Option<usize>],
    pub base_layers: &'a [Option<usize>],
    pub diff: &'a Diff,
}

impl MinimumFacts<'_> {
    fn layer_name(&self, layer: Option<usize>) -> Option<&str> {
        layer.map(|l| self.config.layers.layers()[l].name.as_str())
    }

    fn is_high_risk_layer(&self, layer: Option<usize>) -> bool {
        self.layer_name(layer)
            .is_some_and(|n| self.high_risk_layers.iter().any(|h| h == n))
    }

    /// The layer of the document that a delta line names. A new document has the
    /// layer of its place. `base_side` reads the base tree (a change that the diff
    /// deletes), else the head tree.
    fn target_layer(&self, base_side: bool, id: &str) -> Option<usize> {
        let (c, layers) = if base_side {
            (self.base, self.base_layers)
        } else {
            (self.head, self.head_layers)
        };
        match c.resolve(id) {
            Res::Found(i, None, _) => layers[i as usize],
            Res::Missing => {
                let rel = format!("{id}.md");
                let ty = self
                    .config
                    .layers
                    .place_type(&rel)
                    .unwrap_or(DocType::Requirements);
                self.config.layers.layer_of(&rel, ty)
            }
            _ => None,
        }
    }

    fn layer_of_path(&self, path: &str) -> String {
        self.head
            .docs
            .iter()
            .chain(&self.base.docs)
            .find(|d| d.path == path)
            .and_then(|d| self.layer_name(crate::check::doc_layer(self.config, d)))
            .unwrap_or("")
            .to_string()
    }

    /// The requirements that the diff changes in a layer of `high-risk-layers`.
    pub fn risky(&self) -> Vec<ReqChange> {
        if self.high_risk_layers.is_empty() {
            return Vec::new();
        }
        let wanted = |d: &Doc| {
            requirement_doc(d) && self.is_high_risk_layer(crate::check::doc_layer(self.config, d))
        };
        requirement_changes(self.base, self.head, wanted)
    }

    /// Why the minimum class of one change is `higher-risk`, or `None`: the target
    /// of the reason and its text. The reasons, in order: a delta line that names a
    /// requirement of a layer of `high-risk-layers`; with no delta, a changed
    /// requirement of such a layer in a path of the change; a path of the diff or
    /// of `files` that matches `high-risk-paths`. `mine` tells if a path of the
    /// diff is a path of this change. `risky` is [`MinimumFacts::risky`].
    fn reason(
        &self,
        risky: &[ReqChange],
        files: &[String],
        targets: &[String],
        delta_count: usize,
        base_side: bool,
        mine: &dyn Fn(&str) -> bool,
    ) -> Option<(String, String)> {
        for id in targets {
            let layer = self.target_layer(base_side, id);
            if self.is_high_risk_layer(layer) {
                return Some((
                    id.clone(),
                    format!(
                        "the delta targets `{id}` in the layer `{}` of `high-risk-layers`",
                        self.layer_name(layer).unwrap_or("")
                    ),
                ));
            }
        }
        if delta_count == 0
            && let Some(r) = risky.iter().find(|r| mine(&r.path))
        {
            return Some((
                r.doc.clone(),
                format!(
                    "the diff changes the requirement `{}#{}` in the layer `{}` of `high-risk-layers`",
                    r.doc,
                    r.key,
                    self.layer_of_path(&r.path)
                ),
            ));
        }
        let from_diff = self
            .diff
            .files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(|p| mine(p))
            .find_map(|p| first_glob(self.high_risk_paths, p).map(|g| (p.to_string(), g)));
        let from_plan = files
            .iter()
            .find_map(|f| first_glob(self.high_risk_paths, f).map(|g| (f.clone(), g)));
        from_diff.or(from_plan).map(|(p, g)| {
            (
                g.clone(),
                format!("`{p}` matches `{g}` of `high-risk-paths`"),
            )
        })
    }
}

/// True if the minimum class of the change `ch` of `corpus` is `higher-risk`
/// (SD917): the rule of [`MinimumFacts`] for a change that the head tree has.
/// `base_side` is true when `corpus` is the base tree (the change that a diff
/// deletes). `sole` is true when no other change touches the diff, so each path
/// of the diff is a path of this change.
pub fn minimum_is_higher_risk(
    f: &MinimumFacts,
    ch: &Change,
    corpus: &Corpus,
    base_side: bool,
    sole: bool,
) -> bool {
    let files = ch.files(corpus);
    let lines = ch.delta(corpus);
    let targets: Vec<String> = lines
        .iter()
        .filter_map(|l| l.parsed.as_ref().ok().map(|d| d.id.clone()))
        .collect();
    let mine = |p: &str| sole || change::in_scope(&files, p);
    let risky = if lines.is_empty() {
        f.risky()
    } else {
        Vec::new()
    };
    f.reason(&risky, &files, &targets, lines.len(), base_side, &mine)
        .is_some()
}

/// SD917: the class of an intent is lower than the minimum that the diff gives.
/// A diff with no change has no intent. It gives SD917 only for a path of
/// `high-risk-paths` and a requirement of a layer of `high-risk-layers`.
fn class_too_low(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    let config = ctx.config;
    let facts = MinimumFacts {
        config,
        high_risk_layers: &config.high_risk_layers,
        high_risk_paths: &config.high_risk_paths,
        head: ctx.head,
        base: ctx.base,
        head_layers: &ctx.head_layers,
        base_layers: &ctx.base_layers,
        diff: ctx.diff,
    };
    // The requirements that the diff changes in a layer of `high-risk-layers`.
    let risky = facts.risky();
    let layer_of_path = |path: &str| -> String { facts.layer_of_path(path) };
    let mut out = Vec::new();
    if changes.is_empty() {
        for (id, list) in by_document(&risky) {
            let first = list[0];
            out.push(Diag {
                path: first.path.clone(),
                line: first.line.max(1),
                code: Code::ClassTooLow,
                doc: id.to_string(),
                target: String::new(),
                message: format!(
                    "the diff changes {} requirement(s) of `{id}` in the layer `{}` of `high-risk-layers`, and it has no intent; write an intent with `class: higher-risk`",
                    list.len(),
                    layer_of_path(&first.path)
                ),
                severity: Code::ClassTooLow.severity(),
            });
        }
        for (p, _) in &ctx.diff.files {
            if let Some(g) = high_risk_path(config, p) {
                out.push(path_diag(
                    p,
                    Code::ClassTooLow,
                    &g,
                    format!(
                        "`{p}` matches `{g}` of `high-risk-paths`, and the diff has no intent; write an intent with `class: higher-risk`"
                    ),
                ));
            }
        }
        return out;
    }
    let sole = changes.len() == 1;
    for ch in changes {
        let Some(intent) = &ch.change.intent else {
            continue;
        };
        if ch.change.class != Class::Routine {
            continue;
        }
        let mine = |path: &str| sole || change::in_scope(&ch.files, path);
        let reason = facts.reason(
            &risky,
            &ch.files,
            &ch.targets,
            ch.delta_count,
            matches!(ch.side, Side::Base),
            &mine,
        );
        if let Some((target, why)) = reason {
            out.push(crate::check::diag(
                &ctx.corpus(ch).docs[intent.doc as usize],
                1,
                Code::ClassTooLow,
                target,
                format!(
                    "the class of the change `{}` is `routine`, and {why}; set `class: higher-risk`",
                    ch.change.slug
                ),
            ));
        }
    }
    out
}

/// SD905, the part for a change with no delta: the diff of the change touches
/// more requirement documents than `max-fanout`.
fn routine_fan_out(ctx: &Ctx, changes: &[DiffChange]) -> Vec<Diag> {
    let limit = ctx.config.max_fanout as usize;
    let sole = changes.len() == 1;
    let mut out = Vec::new();
    for ch in changes.iter().filter(|c| c.delta_count == 0) {
        let corpus = ctx.corpus(ch);
        let c = &ch.change;
        let Some(anchor) = c.intent.as_ref().or(c.plan.as_ref()).or(c.spec.as_ref()) else {
            continue;
        };
        let docs: HashSet<&str> = ctx
            .diff
            .files
            .iter()
            .map(|(p, _)| p.as_str())
            .filter(|p| ctx.paths().requirements.contains(*p))
            .filter(|p| sole || change::in_scope(&ch.files, p))
            .collect();
        if docs.len() > limit {
            out.push(crate::check::diag(
                &corpus.docs[anchor.doc as usize],
                1,
                Code::FanOut,
                "",
                format!(
                    "the diff of the change touches {} requirement documents, more than `max-fanout = {limit}`",
                    docs.len()
                ),
            ));
        }
    }
    out
}

/// The checks of a finished change: SD914 (the interface of `Interfaces` that
/// the diff does not change), SD915, SD916 and SD920 (a decision that is not
/// `accepted`). They apply to a change that the diff deletes: the base tree
/// has its folder, and the head tree has not. A change whose base status is
/// `closed` gives none of them. A branch-born change gives none, because the
/// base tree has no folder. The checks read the spec and the plan from the base
/// tree, so an edit of them in the pull request of `finish` has no effect
/// (design, "Finish"). The scope of SD913 and the tasks that SD915 reads for a
/// change with no delta come from the newest plan of the branch
/// ([`branch_version`]).
fn finished_checks(
    ctx: &Ctx,
    changes: &[DiffChange],
    nc: &crate::acceptance::NeedConfig,
) -> Result<Vec<Diag>, String> {
    let on = |k: Code| !ctx.config.is_off(k);
    let sole = changes.len() == 1;
    let mut out = Vec::new();
    let deleted = changes
        .iter()
        .filter(|c| c.origin == Origin::Touched && matches!(c.side, Side::Base));
    for ch in deleted {
        if ch.change.intent_status() == "closed" {
            continue;
        }
        if on(Code::FinishIncomplete) {
            out.extend(finish_incomplete(ctx, ch, sole, nc.on));
        }
        if on(Code::DeltaEvidenceMissing) || on(Code::ChangeUnproven) || on(Code::ProofNotReliable)
        {
            let mine = |path: &str| sole || change::in_scope(&ch.files, path);
            // SD916, SD617 and SD620 are 3 checks of one function. A code that
            // is off gives no diagnostic, so each stays as the profile sets it.
            // The class is the class of the acceptance (D70 item 3), so a branch
            // cannot lower the class of the base to skip SD620. It is read only when
            // the key `reliable-for` asks for the class.
            let class = if ctx.config.evidence_reliable_for == config::ReliableFor::HigherRisk {
                let minimum = || minimum_for(ctx, changes, ch, nc);
                crate::acceptance::record_class(&crate::acceptance::ClassInput {
                    nc,
                    git: ctx.git,
                    base_rev: ctx.base_rev,
                    folder: &ch.change.dir,
                    head: None,
                    minimum: &minimum,
                })?
                .class
            } else {
                ch.change.class
            };
            let reliable = crate::evidence::reliable_for(ctx.config, class);
            out.extend(
                evidence_missing(ctx.head, ctx.base, ctx.base, &ch.change, &mine, reliable)
                    .into_iter()
                    .filter(|d| on(d.code)),
            );
        }
        if on(Code::DecisionNeededUnaccepted) {
            out.extend(decisions_unaccepted(ctx.head, ctx.base, &ch.change));
        }
        // The spec of the base lists the interfaces. The diff must change each
        // one, or a file that it cites.
        if on(Code::DeltaInvalid)
            && let Some(a) = &ch.change.spec
        {
            let spec = &ctx.base.docs[a.doc as usize];
            let changed = |p: &str| ctx.diff.kind(p).is_some();
            out.extend(interfaces_unchanged(ctx.base, spec, &changed));
            // A new interface needs its document in the head tree.
            let added = added_ids(&ch.change, ctx.base);
            out.extend(interfaces_absent(ctx.head, spec, &added));
        }
    }
    Ok(out)
}

/// True if the minimum class (SD917) of a change that the diff deletes, or that
/// the branch added and deleted, is `higher-risk`. The lists of
/// `high-risk-paths` and `high-risk-layers` are those of the need
/// ([`crate::acceptance::NeedConfig`]).
fn minimum_for(
    ctx: &Ctx,
    changes: &[DiffChange],
    ch: &DiffChange,
    nc: &crate::acceptance::NeedConfig,
) -> bool {
    let facts = MinimumFacts {
        config: ctx.config,
        high_risk_layers: &nc.high_risk_layers,
        high_risk_paths: &nc.high_risk_paths,
        head: ctx.head,
        base: ctx.base,
        head_layers: &ctx.head_layers,
        base_layers: &ctx.base_layers,
        diff: ctx.diff,
    };
    let sole = changes.len() == 1;
    let mine = |path: &str| sole || change::in_scope(&ch.files, path);
    let risky = if ch.delta_count == 0 {
        facts.risky()
    } else {
        Vec::new()
    };
    facts
        .reason(
            &risky,
            &ch.files,
            &ch.targets,
            ch.delta_count,
            matches!(ch.side, Side::Base),
            &mine,
        )
        .is_some()
}

/// The checks of the acceptance record (design 0.8.0, D3 item 9), in 3 places.
///
/// - A record that the diff adds or changes, in the folder of a change or as an
///   archive file: the rules of a valid record, with the compare of the brief of
///   each entry that the branch adds (SD928), and SD931. Thus the first of 2
///   acceptors can commit, and a code fix after a `rejected` record passes.
/// - A change that the diff deletes, and a change that the branch added and
///   deleted (a branch-born change), when the change needs a record (D3 item 1):
///   the presence (SD927), the tree (SD929), the result (SD930) and the count of
///   acceptors (SD933). They read the archive file that the diff adds. A record
///   that stays in the folder of the base does not count, so a folder that a
///   person deletes with no `finish` gives SD927.
/// - A diff that changes or deletes a file of the archive folder (SD935).
fn record_checks(
    ctx: &Ctx,
    changes: &[DiffChange],
    nc: &crate::acceptance::NeedConfig,
) -> Result<Vec<Diag>, String> {
    use crate::acceptance as acc;
    // The head configuration reads the paths and the layers; the codes follow the
    // configuration of the need (D70 item 6).
    let cfg = ctx.config;
    let on = |k: Code| !nc.gate().is_off(k);
    let any_on = acc::CODES.iter().any(|&k| on(k));
    if !any_on {
        return Ok(Vec::new());
    }
    let intent_dirs = intent_dirs(cfg);
    let mut out = Vec::new();
    // The facts of the plan. For a change that the diff deletes, use the plan of
    // the branch before the deletion, as `finish` read it. The base plan can
    // differ, when the branch edited `Files`. For any other change, use its plan
    // in the tree that the checks read.
    let facts_of = |slug: &str| {
        changes
            .iter()
            .find(|c| c.change.slug == slug)
            .map(|c| match c.branch(ctx) {
                Some(b) => b.facts,
                None => acc::plan_facts(&c.change, ctx.corpus(c)),
            })
            .unwrap_or_default()
    };
    // A record that the diff adds or changes.
    if on(Code::AcceptanceInvalid)
        || on(Code::SelfAccepted)
        || on(Code::AcceptanceRerun)
        || on(Code::RecordCommitInvalid)
    {
        for (p, kind) in &ctx.diff.files {
            if *kind == PathChange::Deleted {
                continue;
            }
            let folder = acc::record_folder(p, &intent_dirs);
            if folder.is_none() && !acc::is_archive_path(p) {
                continue;
            }
            // A link, a folder or a file over the limit is not read: it is no
            // record, and SD928 says so.
            let bytes = match ctx.git.head_file_checked(p, acc::READ_LIMIT)? {
                Limited::Bytes(b) => b,
                Limited::Absent => continue,
                other => {
                    if on(Code::AcceptanceInvalid) {
                        let slug = match &folder {
                            Some((_, slug)) => slug.clone(),
                            None => acc::archive_owner(p, ""),
                        };
                        let m = acc::unreadable_message(p, &other);
                        out.push(path_diag(p, Code::AcceptanceInvalid, &slug, m));
                    }
                    continue;
                }
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let slug = match &folder {
                Some((_, slug)) => slug.clone(),
                // An archive file names its change in the field `change`.
                None => acc::archive_owner(p, &text),
            };
            let place = if folder.is_some() {
                acc::Place::Folder
            } else {
                acc::Place::Archive
            };
            let (rec, mut problems) = acc::read_record(cfg, &text, &slug, facts_of(&slug), place);
            if let (Some(rec), Some((dir, _)), true) = (&rec, &folder, on(Code::AcceptanceInvalid))
            {
                problems.extend(acc::brief_problems(cfg, ctx.git, ctx.base_rev, dir, rec)?);
            }
            if on(Code::AcceptanceInvalid) {
                for m in problems {
                    out.push(path_diag(p, Code::AcceptanceInvalid, &slug, m));
                }
            }
            // The acknowledgement with `branch` (D3 item 10): each commit that adds or
            // edits the record changes the record alone, and each entry counts.
            if let Some((dir, _)) = &folder
                && on(Code::RecordCommitInvalid)
                && nc.approval_branch(cfg)
            {
                let entries = crate::record::ack_entries(&text);
                let f =
                    acc::ack_facts(cfg, nc, ctx.git, ctx.base_rev, (dir, &slug), (&entries, 0))?;
                for m in f.invalid {
                    out.push(path_diag(p, Code::RecordCommitInvalid, &slug, m));
                }
            }
            // The stop rule (D74): the record comes after 2 rejected records.
            if let (Some(rec), Some((dir, _)), true) = (&rec, &folder, on(Code::AcceptanceRerun))
                && nc.approval_branch(cfg)
            {
                let sf = acc::stop_facts(
                    cfg,
                    nc,
                    ctx.git,
                    ctx.base_rev,
                    dir,
                    acc::head_state(Some(rec)),
                    (ctx.base, ctx.head),
                )?;
                if let Some(m) = sf.rerun {
                    out.push(path_diag(p, Code::AcceptanceRerun, &slug, m));
                }
            }
            if let Some(rec) = &rec
                && on(Code::SelfAccepted)
            {
                for m in acc::self_accepted(rec) {
                    out.push(path_diag(p, Code::SelfAccepted, &slug, m));
                }
            }
        }
    }
    // A change that the diff deletes, or that the branch added and deleted.
    for ch in changes {
        let deleted = ch.origin == Origin::Touched && matches!(ch.side, Side::Base);
        if !(deleted || ch.origin == Origin::Born) {
            continue;
        }
        let c = &ch.change;
        let Some(plan) = &c.plan else { continue };
        if c.intent_status() == "closed" {
            continue;
        }
        // The rule of D70 for a change that the diff deletes (its plan is in the
        // base tree) and for a change that the branch added and deleted (its plan is
        // new). The head has no intent, so the class reads the base tree, the commits
        // of the branch and SD917 (item 4).
        let plan_path = &ctx.corpus(ch).docs[plan.doc as usize].path;
        let minimum = || minimum_for(ctx, changes, ch, nc);
        // The scope of the plan as `finish` read it: the newest plan of the
        // branch for a deleted folder, else the plan of the tree.
        let plan = match ch.branch(ctx) {
            Some(b) => b.plan.clone(),
            None => acc::PlanScope::of(c, ctx.corpus(ch)),
        };
        // The roles of `[owners]` read the role paths of the change (D70 item 7).
        let roles = if nc.has_owners(cfg) {
            let scope_of = |p: &acc::PlanScope| {
                let mut s = p.patterns.clone();
                s.extend(cfg.shared_paths.iter().cloned());
                s.sort();
                s.dedup();
                s
            };
            let others: Vec<Vec<String>> = changes
                .iter()
                .filter(|o| o.change.dir != c.dir)
                .filter(|o| ctx.diff.under(&o.change.dir).next().is_some())
                .map(|o| {
                    scope_of(&match o.branch(ctx) {
                        Some(b) => b.plan.clone(),
                        None => acc::PlanScope::of(&o.change, ctx.corpus(o)),
                    })
                })
                .collect();
            acc::change_roles(
                nc,
                &acc::RoleInput {
                    cfg,
                    git: ctx.git,
                    diff: ctx.diff,
                    scope: &scope_of(&plan),
                    other_scopes: &others,
                    plan_path,
                },
            )?
        } else {
            Vec::new()
        };
        let decision = acc::decide(
            &acc::ClassInput {
                nc,
                git: ctx.git,
                base_rev: ctx.base_rev,
                folder: &c.dir,
                head: None,
                minimum: &minimum,
            },
            plan_path,
            deleted,
            &roles,
        )?;
        let acc::Need::Yes {
            shallow,
            branch_cut,
        } = decision.need
        else {
            continue;
        };
        let class = decision.class;
        let record_path = format!("{}/{}", c.dir, crate::record::FILE_NAME);
        // The stop rule (D74 item 6) also runs where the diff deletes the folder.
        if on(Code::AcceptanceRerun) && nc.approval_branch(cfg) {
            let sf = acc::stop_facts(
                cfg,
                nc,
                ctx.git,
                ctx.base_rev,
                &c.dir,
                None,
                (ctx.base, ctx.head),
            )?;
            if let Some(m) = sf.rerun {
                out.push(path_diag(&record_path, Code::AcceptanceRerun, &c.slug, m));
            }
        }
        // The archive file that the diff adds for this change: the file whose
        // field `change` is the slug, the newest number of the slug. The name of
        // a file decides nothing: `fix-2.toml` can be the archive of the change
        // `fix-2`, and it can be the file 2 of the change `fix`.
        // Only a path of the archive folder counts: a file elsewhere is no
        // archive file, whatever its name or its text.
        let added: Vec<&str> = ctx
            .diff
            .files
            .iter()
            .filter(|(_, k)| *k == PathChange::Added)
            .map(|(p, _)| p.as_str())
            .collect();
        let in_folder: Vec<&str> = added
            .iter()
            .copied()
            .filter(|p| acc::is_archive_path(p))
            .collect();
        let found = acc::archive_of(&c.slug, &in_folder, &mut |p| {
            ctx.git.head_file_limited(p, acc::READ_LIMIT)
        })?;
        let archive = found.as_ref().map(|(p, _)| p.clone());
        let Some(archive) = archive else {
            // A commit can move the archive file out of its folder: the diff
            // then adds the record of the change at another path.
            if on(Code::RecordEdited)
                && let Some(p) = acc::moved_archive(
                    &c.slug,
                    &added,
                    &|p| acc::record_folder(p, &intent_dirs).is_some(),
                    &mut |p| ctx.git.head_file_limited(p, acc::READ_LIMIT),
                )?
            {
                out.push(path_diag(
                    &p,
                    Code::RecordEdited,
                    &c.slug,
                    format!(
                        "the diff adds the record of the change at `{p}`, outside `{}/`; an archive file is never moved, so put it back at `{}/{}.toml`",
                        acc::ARCHIVE_DIR,
                        acc::ARCHIVE_DIR,
                        c.slug
                    ),
                ));
            }
            out.push(path_diag(
                &record_path,
                Code::AcceptanceMissing,
                &c.slug,
                acc::missing_message(
                    &c.slug,
                    &format!(
                        "the diff adds no archive file `{}/{}.toml` of the change, and a record in the folder of the base, or an older archive file in the head, does not count",
                        acc::ARCHIVE_DIR,
                        c.slug
                    ),
                    &roles,
                    shallow,
                    branch_cut,
                ),
            ));
            continue;
        };
        let Some((_, bytes)) = found else {
            continue;
        };
        // The archive file is the record that `finish` moved: the same bytes as
        // the folder record of the old tree.
        if on(Code::RecordEdited)
            && let Some(m) = acc::archive_differs(ctx.git, ctx.base_rev, &c.dir, &archive, &bytes)?
        {
            out.push(path_diag(&archive, Code::RecordEdited, &c.slug, m));
        }
        let Ok(rec) = crate::record::Record::parse(&String::from_utf8_lossy(&bytes)) else {
            // SD928 reports a record that does not read.
            continue;
        };
        if on(Code::AcceptanceStale)
            && let Some(m) = acc::stale(
                cfg,
                ctx.git,
                &rec,
                Some(ctx.diff),
                &plan,
                Some(ctx.base_rev),
                true,
            )?
        {
            out.push(path_diag(&archive, Code::AcceptanceStale, &c.slug, m));
        }
        if on(Code::AcceptanceRejected)
            && let Some(m) = acc::rejected(&rec)
        {
            out.push(path_diag(&archive, Code::AcceptanceRejected, &c.slug, m));
        }
        if on(Code::AcceptorMissing)
            && let Some(m) = acc::acceptors_missing(class, nc.higher_risk_acceptors, &roles, &rec)
        {
            out.push(path_diag(&archive, Code::AcceptorMissing, &c.slug, m));
        }
        // SD932 and SD934 with `branch`: the walk reads the commits of the folder
        // record, and the commit of `finish` keeps the entries that count.
        if (on(Code::RecordCommitInvalid) || on(Code::AckMissing)) && nc.approval_branch(cfg) {
            let f = acc::ack_facts(
                cfg,
                nc,
                ctx.git,
                ctx.base_rev,
                (&c.dir, &c.slug),
                (&rec.acknowledged_by, rec.deviations.len()),
            )?;
            if on(Code::RecordCommitInvalid) {
                for m in f.invalid {
                    out.push(path_diag(&archive, Code::RecordCommitInvalid, &c.slug, m));
                }
            }
            if on(Code::AckMissing)
                && let Some(m) = f.missing
            {
                out.push(path_diag(&archive, Code::AckMissing, &c.slug, m));
            }
        }
        // SD938: the answers. The items come from the newest spec and plan of the
        // branch for a deleted folder, as `finish` read them, else from the tree.
        if on(Code::RecordIncomplete) {
            let decl = match ch.branch(ctx) {
                Some(b) => b.declared.clone()?,
                None => acc::declared_for(cfg, ctx.git, Some(ctx.base_rev), c, ctx.corpus(ch))?,
            };
            for m in acc::answers_missing(&rec, &decl) {
                out.push(path_diag(&archive, Code::RecordIncomplete, &c.slug, m));
            }
        }
    }
    // A change of the archive folder.
    if on(Code::RecordEdited) {
        for (p, kind) in &ctx.diff.files {
            if acc::is_archive_path(p) && *kind != PathChange::Added {
                out.push(path_diag(
                    p,
                    Code::RecordEdited,
                    "",
                    format!(
                        "the diff {} the archive file `{p}`; a file of `{}/` is the record of a finished change, so revert the edit",
                        if *kind == PathChange::Deleted { "deletes" } else { "changes" },
                        acc::ARCHIVE_DIR
                    ),
                ));
            }
        }
    }
    Ok(out)
}

/// SD915 for one deleted change whose base status is not `closed`. A change
/// with a delta needs each line applied in the head (design, T3). The lines
/// come from the base tree, so a correction in the same pull request does not
/// count. A change with no delta needs a changed `Files` path or a changed
/// requirement hash, or a newest plan with tasks and no open task (D87): the
/// plan says that the work is done, and `finish` passed its step 0 on it. A
/// `REMOVE` line for a key that neither tree has is not reported: the head has
/// no such key, and SD914 reported the line earlier. `accept_on` is the rule of
/// the need for the tasks that name `spec-dna accept`.
fn finish_incomplete(ctx: &Ctx, ch: &DiffChange, sole: bool, accept_on: bool) -> Vec<Diag> {
    let c = &ch.change;
    let status = match c.intent_status() {
        "" => "none",
        s => s,
    };
    let lines = c.delta(ctx.base);
    if let (false, Some(a)) = (lines.is_empty(), &c.spec) {
        let spec = &ctx.base.docs[a.doc as usize];
        let mut out = Vec::new();
        let items = change::section_items(spec, ItemSection::Delta);
        for (line, item) in lines.iter().zip(items) {
            let (target, what) = match (change::applied(line, ctx.base, ctx.head), &line.parsed) {
                (change::Applied::Yes, _) => continue,
                (change::Applied::Invalid, Ok(_)) => continue,
                (change::Applied::Invalid, Err(reason)) => (
                    short(&item.text),
                    format!("the delta line is not valid ({reason})"),
                ),
                (change::Applied::No, Ok(d)) => (
                    d.target(),
                    format!(
                        "the head does not have the result of the delta line for `{}`",
                        d.target()
                    ),
                ),
                (change::Applied::No, Err(_)) => continue,
            };
            out.push(crate::check::diag(
                spec,
                line.line,
                Code::FinishIncomplete,
                target,
                format!(
                    "the diff deletes the change `{}`, its base status is `{status}`, and {what}; apply the delta with `spec-dna finish {}`, or set the intent to `closed` in an earlier pull request",
                    c.slug, c.slug
                ),
            ));
        }
        return out;
    }
    let prefix = format!("{}/", c.dir);
    let files_changed = ctx
        .diff
        .files
        .iter()
        .any(|(p, _)| !p.starts_with(&prefix) && change::in_scope(&ch.files, p));
    if files_changed {
        return Vec::new();
    }
    let hash_changed = requirement_changes(ctx.base, ctx.head, requirement_doc)
        .iter()
        .any(|r| sole || change::in_scope(&ch.files, &r.path));
    if hash_changed {
        return Vec::new();
    }
    let Some(anchor) = c.intent.as_ref().or(c.plan.as_ref()).or(c.spec.as_ref()) else {
        return Vec::new();
    };
    // The newest plan: the last text of the branch, else the plan of the base.
    let tasks = match ch.branch(ctx).filter(|b| b.has_plan) {
        Some(b) => b.tasks.clone(),
        None => c.tasks(ctx.base),
    };
    let open = change::open_tasks(&tasks, &c.slug, accept_on);
    if !tasks.is_empty() && open.is_empty() {
        return Vec::new();
    }
    let anchor_doc = &ctx.base.docs[anchor.doc as usize];
    if !open.is_empty() {
        let ids: Vec<&str> = open.iter().map(|t| t.id.as_str()).collect();
        return vec![crate::check::diag(
            anchor_doc,
            1,
            Code::FinishIncomplete,
            "",
            format!(
                "the diff deletes the change `{}`, its base status is `{status}`, the diff changes no `Files` path and no requirement hash, and its plan has {} open task(s) ({}); do each task, or close it with `Skip: <reason>`, and run `spec-dna finish {}`, or set the intent to `closed` in an earlier pull request",
                c.slug,
                ids.len(),
                ids.join(", "),
                c.slug
            ),
        )];
    }
    // The note on `Files` reads the plan of the base tree only.
    let cause = not_paths_cause(ch, None).unwrap_or_default();
    vec![crate::check::diag(
        anchor_doc,
        1,
        Code::FinishIncomplete,
        "",
        format!(
            "the diff deletes the change `{}`, its base status is `{status}`, and the diff changes no `Files` path and no requirement hash{cause}; do the work first, or set the intent to `closed` in an earlier pull request",
            c.slug
        ),
    )]
}

/// The index of a requirement in a tree: the document and the position in
/// `parsed.reqs`.
fn req_index(c: &Corpus, id: &str, key: &str) -> Option<(u32, usize)> {
    let Res::Found(d, None, _) = c.resolve(id) else {
        return None;
    };
    c.docs[d as usize].req(key).map(|r| (d, r))
}

/// SD916 for a change: each requirement that the change adds or changes has
/// evidence in `head` (a `covers:` tag, a `verified-by` marker or the
/// `verified-by` of the document). SD617 is the second check: a requirement
/// that has a tag and no `verified-by` needs a bound test case that passed in
/// a report. It runs only when the tags carry a result of the bind. SD620 is
/// the third check: with `reliable` (the passes that `[evidence] reliable-for`
/// needs for this change), a requirement that passed fewer times is below the
/// level `reliable`. A debt (`proof-owed`) is no evidence for any of them.
///
/// - With a delta, the requirements are the targets of the `ADD` and `CHANGE`
///   lines that `head` applies. A line that `head` does not apply gives no
///   SD916 here, because SD915 (or `finish`) reports it.
/// - With no delta, the requirements are those whose hash differs from `base`,
///   in a document whose path `scope` accepts (design, "Finish", step 5).
///
/// `source` is the tree that has the spec of the change: the base tree for a
/// deleted folder, the head tree when `finish` runs. `check` and `finish` both
/// call this function. The function reads no git data. `head.covers` must hold
/// the `covers:` tags of the tests.
pub fn evidence_missing(
    head: &Corpus,
    base: &Corpus,
    source: &Corpus,
    ch: &Change,
    scope: &dyn Fn(&str) -> bool,
    reliable: Option<crate::evidence::Reliable>,
) -> Vec<Diag> {
    let lines = ch.delta(source);
    let mut wanted: Vec<(u32, usize)> = Vec::new();
    if lines.is_empty() {
        for r in requirement_changes(base, head, requirement_doc) {
            if r.kind != ReqChangeKind::Removed
                && scope(&r.path)
                && let Some(i) = req_index(head, &r.doc, &r.key)
            {
                wanted.push(i);
            }
        }
    } else {
        for line in &lines {
            if let Ok(d) = &line.parsed
                && d.verb != Verb::Remove
                && change::applied(line, base, head) == change::Applied::Yes
                && let Some(i) = req_index(head, &d.id, &d.key)
            {
                wanted.push(i);
            }
        }
    }
    if wanted.is_empty() {
        return Vec::new();
    }
    let (ev, _) = Evidence::build(head);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (di, ri) in wanted {
        if !seen.insert((di, ri)) {
            continue;
        }
        let doc = &head.docs[di as usize];
        let req = &doc.parsed.reqs[ri];
        if ev.of(di, doc, ri).is_empty() {
            let message = match ev.partial_of(di, ri).first() {
                Some(at) => format!(
                    "the change `{}` adds or changes the requirement `{}#{}`, and only a partial `covers:` tag names it ({at}); add a test that covers all of it",
                    ch.slug, doc.id, req.key
                ),
                None => format!(
                    "the change `{}` adds or changes the requirement `{}#{}`, and it has no `covers:` tag and no `verified-by`",
                    ch.slug, doc.id, req.key
                ),
            };
            out.push(crate::check::diag(
                doc,
                req.line,
                Code::DeltaEvidenceMissing,
                format!("{}#{}", doc.id, req.key),
                message,
            ));
        } else if let Some(why) = ev.unproven(di, doc, ri) {
            // The requirement has a tag, and the test reports do not prove it
            // (design 0.8.0, D1). `unproven` is `None` when no report is read.
            // With SD617 off and `reliable-for` on, it has 0 passes: SD620.
            let code = match reliable {
                Some(r) if !r.sd617_on => Code::ProofNotReliable,
                _ => Code::ChangeUnproven,
            };
            out.push(crate::check::diag(
                doc,
                req.line,
                code,
                format!("{}#{}", doc.id, req.key),
                format!(
                    "the change `{}` adds or changes the requirement `{}#{}`, and {why}",
                    ch.slug, doc.id, req.key
                ),
            ));
        } else if let Some(r) = reliable
            && let Some(why) = ev.not_reliable(di, doc, ri, r.runs)
        {
            out.push(crate::check::diag(
                doc,
                req.line,
                Code::ProofNotReliable,
                format!("{}#{}", doc.id, req.key),
                format!(
                    "the change `{}` adds or changes the requirement `{}#{}`, and {why}",
                    ch.slug, doc.id, req.key
                ),
            ));
        }
    }
    out
}

/// SD920, the part for a decision that is not `accepted`: each `[[id]]` of
/// `Decisions` in the spec of the change must name a document of `head` with
/// the status `accepted`. `source` is the tree that has the spec (see
/// `evidence_missing`).
pub fn decisions_unaccepted(head: &Corpus, source: &Corpus, ch: &Change) -> Vec<Diag> {
    let Some(a) = &ch.spec else {
        return Vec::new();
    };
    let spec = &source.docs[a.doc as usize];
    let mut out = Vec::new();
    for item in change::section_items(spec, ItemSection::Decisions) {
        for id in decision_ids(&item.text) {
            let why = match head.resolve(id) {
                Res::Missing => "names no document".to_string(),
                Res::Found(d, _, _) => match head.docs[d as usize].status_str() {
                    "accepted" => continue,
                    "" => "has no status, and a needed decision must be `accepted`".to_string(),
                    s => format!("is `{s}`, and a needed decision must be `accepted`"),
                },
                _ => continue,
            };
            out.push(crate::check::diag(
                spec,
                item.line,
                Code::DecisionNeededUnaccepted,
                id,
                format!(
                    "the change `{}` needs the decision `{id}`, and it {why}",
                    ch.slug
                ),
            ));
        }
    }
    out
}

/// A started change that has had no commit for more than `stale-days`
/// (SD911).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stale {
    pub slug: String,
    /// The index of its intent in the head corpus.
    pub intent: u32,
    /// The whole days between the newest commit that touched the change and the
    /// base commit.
    pub idle_days: i64,
    pub limit: u32,
}

/// Finds the started changes (an `accepted` intent) of `head` that are stale
/// (design 0.6.0, K4). A change is stale if the committer date of the newest
/// commit that touched its folder or a `Files` path is more than `stale-days`
/// of the `intent` layer older than the committer date of `base_rev`.
///
/// The function never reads the clock. `base_rev` must be a real merge base
/// (`change_base`), not the tip that a command used because no merge base
/// exists. One `git log` pass reads the dates of all changes. A change that no
/// commit touched (a folder that is not committed) is not stale. In a shallow
/// clone, the newest touch is as new as in the full repository or newer, so a
/// shallow clone can hide a stale change and never reports a false one.
pub fn stale_changes(
    config: &Config,
    git: &Git,
    head: &Corpus,
    base_rev: &str,
) -> Result<Vec<Stale>, String> {
    if config.change_policy == ChangePolicy::None || config.is_off(Code::ChangeStale) {
        return Ok(Vec::new());
    }
    let limit = config
        .layers
        .layer_of_type(DocType::Intent)
        .and_then(|i| config.layers.layers()[i].stale_days);
    let Some(limit) = limit else {
        return Ok(Vec::new());
    };
    let started: Vec<Change> = change::collect(head)
        .into_iter()
        .filter(|c| c.intent_status() == "accepted")
        .collect();
    if started.is_empty() {
        return Ok(Vec::new());
    }
    let mut specs: Vec<String> = Vec::new();
    for c in &started {
        specs.push(c.dir.clone());
        specs.extend(c.files(head));
    }
    let touched = git.last_touch("HEAD", &specs)?;
    let base_time = git.commit_time(base_rev)?;
    let mut out = Vec::new();
    for c in started {
        let prefix = format!("{}/", c.dir);
        let files = c.files(head);
        let newest = touched
            .iter()
            .filter(|(p, _)| p.starts_with(&prefix) || change::in_scope(&files, p))
            .map(|(_, t)| *t)
            .max();
        let (Some(newest), Some(intent)) = (newest, &c.intent) else {
            continue;
        };
        let idle = base_time - newest;
        if idle > i64::from(limit) * 86_400 {
            out.push(Stale {
                slug: c.slug.clone(),
                intent: intent.doc,
                idle_days: idle / 86_400,
                limit,
            });
        }
    }
    Ok(out)
}

/// The SD911 diagnostics of a list of stale changes, with the severity of the
/// profile. They are state diagnostics: they are never new (`Code::is_state`).
pub fn stale_diags(config: &Config, head: &Corpus, stale: &[Stale]) -> Vec<Diag> {
    let out = stale
        .iter()
        .map(|s| {
            crate::check::diag(
                &head.docs[s.intent as usize],
                1,
                Code::ChangeStale,
                "",
                format!(
                    "the change `{}` is `accepted`, and its newest commit is {} days older than the base commit, more than `stale-days = {}`",
                    s.slug, s.idle_days, s.limit
                ),
            )
        })
        .collect();
    crate::check::apply_severity(config, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use std::sync::Arc;

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        Corpus::build(
            &cfg,
            files.iter().map(|(p, _)| p.to_string()).collect(),
            files
                .iter()
                .map(|(p, t)| (p.to_string(), Arc::new(parse(t))))
                .collect(),
        )
    }

    fn doc(reqs: &[(&str, &str)]) -> String {
        let mut s = "---\nstatus: active\ntype: requirements\n---\n# Valve\n\n## Requirements\n\n"
            .to_string();
        for (k, t) in reqs {
            s.push_str(&format!("- **{k}**: {t}\n"));
        }
        s
    }

    fn all(_: &Doc) -> bool {
        true
    }

    #[test]
    fn a_changed_added_and_removed_requirement_is_found() {
        let base = corpus(&[(
            "docs/valve.md",
            &doc(&[
                ("close", "The valve SHALL close in 5 seconds."),
                ("open", "The valve SHALL open."),
                ("flush", "The valve SHALL flush."),
            ]),
        )]);
        let head = corpus(&[(
            "docs/valve.md",
            &doc(&[
                ("close", "The valve SHALL close in 4 seconds."),
                ("open", "The valve SHALL open."),
                ("drain", "The valve SHALL drain."),
            ]),
        )]);
        let ch = requirement_changes(&base, &head, all);
        let got: Vec<(&str, ReqChangeKind)> = ch.iter().map(|c| (c.key.as_str(), c.kind)).collect();
        assert_eq!(
            got,
            vec![
                ("close", ReqChangeKind::Changed),
                ("flush", ReqChangeKind::Removed),
                ("drain", ReqChangeKind::Added),
            ]
        );
        let close = &ch[0];
        assert!(close.base_hash.is_some() && close.head_hash.is_some());
        assert_ne!(close.base_hash, close.head_hash);
        assert!(ch[1].head_hash.is_none() && ch[2].base_hash.is_none());
        assert_eq!(
            by_document(&ch).keys().copied().collect::<Vec<_>>(),
            ["valve"]
        );
    }

    #[test]
    fn the_same_text_and_a_filter_give_nothing() {
        let text = doc(&[("close", "The valve SHALL close.")]);
        let base = corpus(&[("docs/valve.md", &text)]);
        let head = corpus(&[("docs/valve.md", &text)]);
        assert!(requirement_changes(&base, &head, all).is_empty());
        let head2 = corpus(&[("docs/valve.md", &doc(&[("close", "The valve SHALL stay.")]))]);
        assert_eq!(requirement_changes(&base, &head2, all).len(), 1);
        assert!(requirement_changes(&base, &head2, |_| false).is_empty());
    }

    #[test]
    fn an_alias_keeps_a_moved_requirement() {
        let base = corpus(&[(
            "docs/valve.md",
            &doc(&[("close", "The valve SHALL close in 5 seconds.")]),
        )]);
        // The key moved to another document, and the alias names the old one.
        let head = corpus(&[
            ("docs/valve.md", &doc(&[])),
            (
                "docs/pump.md",
                "---\nstatus: active\ntype: requirements\n---\n# Pump\n\n## Requirements\n\n- **close**: The valve SHALL close in 5 seconds. <!-- aliases: valve#close -->\n",
            ),
        ]);
        assert!(requirement_changes(&base, &head, all).is_empty());
    }

    #[test]
    fn the_checks_of_the_diff_read_nothing_under_the_policy_none() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        let git = Git::new(std::path::Path::new("/nonexistent-spec-dna-path"));
        let c = corpus(&[]);
        let diff = Diff {
            files: vec![("src/a.rs".to_string(), PathChange::Added)],
        };
        let r = base_checks(&cfg, &git, &c, &c, "0000", &diff).unwrap();
        assert!(r.diags.is_empty() && r.exempt.is_empty() && !r.born);
    }

    #[test]
    fn the_policy_none_reads_no_git() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        // The path is no repository: any git command would fail.
        let git = Git::new(std::path::Path::new("/nonexistent-spec-dna-path"));
        assert_eq!(change_base(&cfg, &git, Some("main")), Ok(None));
        assert_eq!(change_base(&cfg, &git, None), Ok(None));
    }
}
