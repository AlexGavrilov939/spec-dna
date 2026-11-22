//! The approval in the branch (ADR 0012, design 0.7.0, D11 to D17). With
//! `[change] approval = "branch"`, SD910 reads the commits of `base..HEAD`
//! from the oldest, and a commit of an approver can approve an artifact
//! before the work that needs it.
//!
//! `Branch::read` walks the commits once. For each artifact (`intent.md`,
//! `spec.md` and `plan.md` of a change) it keeps the list of the commits that
//! changed its `status`, and for each of them whether it is an approval
//! commit (D11). The queries then answer for a commit of the walk: does an
//! approval of the artifact come before it? The order is the order of the
//! ancestry, never the date of a commit.
//!
//! An approval of the branch holds only while the artifact keeps its approval
//! status at the head (ADR 0019, reason 9). A later commit, or the working
//! tree, that sets it back takes the approval away from the work that used it,
//! and from the work between the step back and a new approval commit. An
//! approval of the base tree is not read so: SD401 reports its step back.
//!
//! The rule fails closed (D13). A shallow clone with an incomplete range and
//! a merge commit that is not neutral stop it: the branch then gives no
//! approval, and SD910 reports as in 0.6. A working tree that differs from
//! `HEAD` counts as one more commit, after the last one, with no author.

use crate::change;
use crate::config::Config;
use crate::frontmatter;
use crate::git::{BranchWalk, Git, PathChange};
use crate::lifecycle::intent_dirs;
use crate::model::Corpus;
use crate::parse::{ParsedDoc, parse};
use crate::record;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// The reason of an SD910 finding (D17). Each message names one of the 9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// 1: no approval in the base tree.
    NoBase,
    /// 2: no approval commit before the work.
    NoCommit,
    /// 3: the author of the approval commit is not an approver.
    Author,
    /// 4: the commit changes more than the `status` line.
    MoreThanStatus,
    /// 5: the clone is shallow and the history is not complete.
    Shallow,
    /// 6: a merge commit stops the rule.
    Merge,
    /// 7: the change was added and finished in this branch (D16).
    Born,
    /// 8: the path is not in the `Files` of the plan at the approval commit.
    Scope,
    /// 9: a later commit set the artifact back after its approval.
    TookBack,
}

impl Reason {
    /// The number of the reason, from 1 to 9.
    pub fn number(self) -> u8 {
        match self {
            Reason::NoBase => 1,
            Reason::NoCommit => 2,
            Reason::Author => 3,
            Reason::MoreThanStatus => 4,
            Reason::Shallow => 5,
            Reason::Merge => 6,
            Reason::Born => 7,
            Reason::Scope => 8,
            Reason::TookBack => 9,
        }
    }

    /// The end of a message: the number of the reason, what it means and its
    /// fix. `commit` is the commit that the reason names, if it has one.
    /// `back` is the artifact and the status of reason 9.
    pub fn sentence(self, commit: Option<&str>, back: Option<&(String, String)>) -> String {
        let sha = commit.map_or(String::new(), |c| c[..c.len().min(12)].to_string());
        let n = self.number();
        match self {
            Reason::NoBase => format!(
                "reason {n}: the base tree has no approval; get the approval into the base first"
            ),
            Reason::NoCommit => format!(
                "reason {n}: no approval commit of an approver comes before this work in the branch; commit the approval alone (the `status` line only, by an approver) before the work"
            ),
            Reason::Author => format!(
                "reason {n}: the author of the approval commit {sha} is not in `[change] approvers`; ask an approver to commit the approval"
            ),
            Reason::MoreThanStatus => format!(
                "reason {n}: the commit {sha} changes more than the `status` line, so it is no approval commit; split it, so that the approval commit changes the `status` line alone"
            ),
            Reason::Shallow => format!(
                "reason {n}: shallow clone, so the history of the branch is not complete and no approval commit counts; fetch the full history (`fetch-depth: 0`)"
            ),
            Reason::Merge => format!(
                "reason {n}: the merge commit {sha} merges another branch, so no approval commit of the branch counts; rebase the branch, or merge only the base into it"
            ),
            Reason::Born => format!(
                "reason {n}: the artifacts and their approvals must merge in a pull request first, and `finish` follows in a later one"
            ),
            Reason::Scope => format!(
                "reason {n}: the path is not in the `Files` of the plan at the approval commit {sha}; put it in `Files` before the approval commit, or move the edit to a change of its own"
            ),
            Reason::TookBack => {
                let (path, status) = back.map_or(("the artifact", "its first status"), |(p, s)| {
                    (p.as_str(), s.as_str())
                });
                // The working tree has no commit: it sets the status itself. A
                // hook that runs `check` rejects the commit of the step back, so
                // the message names the way for that commit (design 0.7.1, B6).
                let (who, way) = if commit.is_some() {
                    (format!("the commit {sha}"), "")
                } else {
                    (
                        "the working tree".to_string(),
                        "; if this is the commit of the step back, a hook that runs `spec-dna check` rejects it: skip only that hook for this commit, name the skip in its body, and make the approval commit next",
                    )
                };
                format!(
                    "reason {n}: {who} set `{path}` back to `{status}` after its approval; approve it again with an approval commit before the work, or take the work out of the branch{way}"
                )
            }
        }
    }
}

/// A reason, with the commit that it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fail {
    pub reason: Reason,
    pub commit: Option<String>,
    /// For reason 9: the artifact and the status that it was set back to.
    back: Option<(String, String)>,
}

impl Fail {
    pub fn new(reason: Reason) -> Fail {
        Fail {
            reason,
            commit: None,
            back: None,
        }
    }

    fn at(reason: Reason, commit: &str) -> Fail {
        Fail {
            reason,
            commit: Some(commit.to_string()),
            back: None,
        }
    }

    /// Reason 9: `commit` (or the working tree, with `None`) set `path` back
    /// to `status`.
    fn took_back(commit: Option<&str>, path: &str, status: &str) -> Fail {
        Fail {
            reason: Reason::TookBack,
            commit: commit.map(str::to_string),
            back: Some((path.to_string(), status.to_string())),
        }
    }

    /// The end of a message (`Reason::sentence`).
    pub fn sentence(&self) -> String {
        self.reason
            .sentence(self.commit.as_deref(), self.back.as_ref())
    }
}

/// An approval commit that cleared a gate (`check --json`, `approvals`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approval {
    /// The full id of the commit.
    pub commit: String,
    /// The artifact that it approved.
    pub path: String,
    /// The e-mail of its author, as the commit stores it.
    pub author: String,
    /// The place of the commit in the walk, from 0.
    pub order: usize,
    /// For a plan: its `Files` at the approval commit (D11).
    scope: Vec<String>,
}

/// Where an approval is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// The base tree.
    Base,
    /// An approval commit of the branch.
    Branch(Approval),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Intent,
    Spec,
    Plan,
}

impl Kind {
    /// The status that an approval commit sets.
    fn wanted(self) -> &'static str {
        match self {
            Kind::Intent => "accepted",
            _ => "approved",
        }
    }
}

/// A change of the `status` of an artifact by a commit.
#[derive(Clone, Debug)]
struct Entry {
    /// The index of the commit in the walk. The working tree has the index
    /// after the last commit.
    idx: usize,
    status: String,
    how: How,
}

#[derive(Clone, Debug)]
enum How {
    Approved(Approval),
    Unapproved(Fail),
}

/// The commits of `base..HEAD` and the changes of the status of each artifact.
pub struct Branch {
    /// Set if the branch gives no approval (D13).
    stop: Option<Fail>,
    /// The number of commits. The index `n` stands for the working tree.
    n: usize,
    /// The ids of the commits of the walk, the oldest first.
    commits: Vec<String>,
    entries: HashMap<String, Vec<Entry>>,
    /// For each path, the indexes of the commits that changed it.
    touches: HashMap<String, Vec<usize>>,
    /// The status of each artifact in the base tree.
    base: HashMap<String, String>,
    /// The status of each artifact at the head: the working tree, or the index
    /// with `--staged`. An artifact that the head does not have is not here.
    head: HashMap<String, String>,
}

impl Branch {
    /// Walks `base_rev..HEAD`. `base` is the lifecycle tree, and `head` the
    /// tree of the working tree (or of the index). It never fails: a history
    /// that git cannot read stops the rule (D13).
    pub fn read(
        config: &Config,
        git: &Git,
        base_rev: &str,
        base: &Corpus,
        head: &Corpus,
    ) -> Branch {
        let mut b = Branch {
            stop: None,
            n: 0,
            commits: Vec::new(),
            entries: HashMap::new(),
            touches: HashMap::new(),
            base: statuses(base),
            head: statuses(head),
        };
        let walk = match git.branch_walk(base_rev, "HEAD") {
            Ok(w) => w,
            Err(_) => {
                b.stop = Some(Fail::new(Reason::Shallow));
                return b;
            }
        };
        b.n = walk.commits.len();
        b.commits = walk.commits.iter().map(|c| c.commit.clone()).collect();
        if let Some(f) = walk_stop(git, base_rev, &walk) {
            b.stop = Some(f);
            return b;
        }
        let dirs = intent_dirs(config);
        let approvers: HashSet<String> = config
            .change_approvers
            .iter()
            .map(|a| a.to_lowercase())
            .collect();
        // The texts that the walk needs, read in one batch: each artifact at
        // each commit that changed it, and, for a commit that could be an
        // approval commit, the artifact at its parent.
        let mut specs: Vec<(String, String)> = Vec::new();
        for c in &walk.commits {
            if c.parents.len() > 1 {
                continue;
            }
            for (p, k) in &c.paths {
                if kind_of(&dirs, p).is_some() && *k != PathChange::Deleted {
                    specs.push((c.commit.clone(), p.clone()));
                    if c.paths.len() == 1 && c.parents.len() == 1 && *k == PathChange::Modified {
                        specs.push((c.parents[0].clone(), p.clone()));
                    }
                }
            }
        }
        let texts = match git.read_at(&specs) {
            Ok(t) => t,
            Err(_) => {
                b.stop = Some(Fail::new(Reason::Shallow));
                return b;
            }
        };
        // A text that git cannot give (a partial clone without the blob) makes
        // the walk a guess, so the rule stops.
        if texts.iter().any(Option::is_none) {
            b.stop = Some(Fail::new(Reason::Shallow));
            return b;
        }
        let text_at: HashMap<(&str, &str), &Vec<u8>> = specs
            .iter()
            .zip(&texts)
            .filter_map(|((c, p), t)| t.as_ref().map(|t| ((c.as_str(), p.as_str()), t)))
            .collect();
        for (i, c) in walk.commits.iter().enumerate() {
            // A neutral merge brings the base, or the commits that the walk
            // lists on their own. The paths that differ from each of its
            // parents are the work of the merge itself (a new file, a fix of
            // a conflict): they count as touched at this commit. The status of
            // an artifact in a merge commit sets no approval.
            if c.parents.len() > 1 {
                for (p, _) in &c.paths {
                    b.touches.entry(p.clone()).or_default().push(i);
                }
                continue;
            }
            let author_ok = approvers.contains(&c.author_email.to_lowercase());
            for (p, k) in &c.paths {
                b.touches.entry(p.clone()).or_default().push(i);
                let Some(kind) = kind_of(&dirs, p) else {
                    continue;
                };
                // `finish` deletes the folder. It does not take the approval
                // of the artifact away.
                if *k == PathChange::Deleted {
                    continue;
                }
                let Some(new) = text_at.get(&(c.commit.as_str(), p.as_str())) else {
                    continue;
                };
                let new = String::from_utf8_lossy(new);
                let status = status_of(&new).unwrap_or_default();
                if b.status_of(p, i) == Some(status.as_str()) {
                    continue;
                }
                let status_only = c.paths.len() == 1
                    && c.parents.len() == 1
                    && *k == PathChange::Modified
                    && text_at
                        .get(&(c.parents[0].as_str(), p.as_str()))
                        .is_some_and(|old| {
                            only_status_differs(&String::from_utf8_lossy(old), &new)
                        });
                let how = if status != kind.wanted() {
                    b.step_back(p, i, Some(c.commit.as_str()), &status)
                } else if !author_ok {
                    How::Unapproved(Fail::at(Reason::Author, &c.commit))
                } else if !status_only {
                    How::Unapproved(Fail::at(Reason::MoreThanStatus, &c.commit))
                } else {
                    let scope = if kind == Kind::Plan {
                        plan_files(config, p, &new)
                    } else {
                        Vec::new()
                    };
                    How::Approved(Approval {
                        commit: c.commit.clone(),
                        path: p.clone(),
                        author: c.author_email.clone(),
                        order: i,
                        scope,
                    })
                };
                b.entries.entry(p.clone()).or_default().push(Entry {
                    idx: i,
                    status,
                    how,
                });
            }
        }
        // The working tree is one more commit, after the last. It has no
        // author, so it never approves.
        let n = b.n;
        for (p, k) in git.diff_paths("HEAD", true).unwrap_or_default() {
            b.touches.entry(p.clone()).or_default().push(n);
            let Some(kind) = kind_of(&dirs, &p) else {
                continue;
            };
            if k == PathChange::Deleted {
                continue;
            }
            let Some(doc) = head.docs.iter().find(|d| d.path == p) else {
                continue;
            };
            let status = doc.status_str().to_string();
            if b.status_of(&p, n) != Some(status.as_str()) {
                let how = if status != kind.wanted() {
                    b.step_back(&p, n, None, &status)
                } else {
                    How::Unapproved(Fail::new(Reason::NoCommit))
                };
                b.entries.entry(p).or_default().push(Entry {
                    idx: n,
                    status,
                    how,
                });
            }
        }
        b
    }

    /// The `how` of a change to a status that is not the approval status, at
    /// the index `i` (reason 9). It is a step back when an approval commit of
    /// the branch held the artifact before, or when an earlier step back has
    /// not been approved again. Else no approval commit set the status.
    fn step_back(&self, path: &str, i: usize, commit: Option<&str>, status: &str) -> How {
        match self.entry_before(path, i).map(|e| &e.how) {
            Some(How::Approved(_)) => How::Unapproved(Fail::took_back(commit, path, status)),
            Some(How::Unapproved(f)) if f.reason == Reason::TookBack => How::Unapproved(f.clone()),
            _ => How::Unapproved(Fail::new(Reason::NoCommit)),
        }
    }

    /// The place of a commit in the walk, from 0, or `None` if the walk does not
    /// list it.
    pub fn position(&self, commit: &str) -> Option<usize> {
        self.commits.iter().position(|c| c == commit)
    }

    /// The place in the walk of the newest approval commit of the artifact `path`:
    /// a commit that set the approval status, by an approver, changing the `status`
    /// line alone (D11). `None` if no such commit exists. The stop rule of the
    /// acceptance counts the rejected records that come after it (design 0.8.0,
    /// D74 item 4).
    pub fn last_approval(&self, path: &str) -> Option<usize> {
        self.entries
            .get(path)?
            .iter()
            .filter_map(|e| match &e.how {
                How::Approved(a) => Some(a.order),
                How::Unapproved(_) => None,
            })
            .max()
    }

    /// The reason that stops the rule, or `None`.
    pub fn stopped(&self) -> Option<&Fail> {
        self.stop.as_ref()
    }

    /// The status of an artifact in the base tree.
    pub fn base_status(&self, path: &str) -> Option<&str> {
        self.base.get(path).map(String::as_str)
    }

    /// The status of an artifact after the commits before the index `i`.
    fn status_of(&self, path: &str, i: usize) -> Option<&str> {
        match self.entry_before(path, i) {
            Some(e) => Some(e.status.as_str()),
            None => self.base_status(path),
        }
    }

    /// The last change of the status of an artifact before the index `i`.
    fn entry_before(&self, path: &str, i: usize) -> Option<&Entry> {
        self.entries.get(path)?.iter().rev().find(|e| e.idx < i)
    }

    /// The indexes of the commits that changed a path. A path that no commit
    /// changed has the working tree only, so the rule reads the state at the
    /// end.
    fn touches_of(&self, path: &str) -> Vec<usize> {
        self.touches
            .get(path)
            .cloned()
            .unwrap_or_else(|| vec![self.n])
    }

    /// Is the artifact `need_path` at the status `status` before the commit `i`,
    /// and who approved it?
    fn need(&self, need_path: &str, status: &str, i: usize) -> Result<Source, Fail> {
        match self.entry_before(need_path, i) {
            Some(e) if e.status == status => match &e.how {
                How::Approved(a) => self.at_head(need_path, status, a),
                How::Unapproved(f) => Err(f.clone()),
            },
            // A step back that follows an approval commit of the branch.
            Some(Entry {
                how: How::Unapproved(f),
                ..
            }) if f.reason == Reason::TookBack => Err(f.clone()),
            Some(_) => Err(self.why_not(need_path, status, i)),
            None if self.base_status(need_path) == Some(status) => Ok(Source::Base),
            None => Err(self.why_not(need_path, status, i)),
        }
    }

    /// The head rule: an approval commit holds while the artifact keeps the
    /// status at the head. An artifact that the head does not have (`finish`
    /// deleted its folder) keeps its approval. The reason names the first
    /// change of the status after the approval commit.
    fn at_head(&self, path: &str, status: &str, a: &Approval) -> Result<Source, Fail> {
        match self.head.get(path) {
            Some(now) if now != status => Err(self
                .entries
                .get(path)
                .and_then(|v| v.iter().find(|e| e.idx > a.order))
                .and_then(|e| match &e.how {
                    How::Unapproved(f) if f.reason == Reason::TookBack => Some(f.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| Fail::took_back(None, path, now))),
            _ => Ok(Source::Branch(a.clone())),
        }
    }

    /// The reason that the status is not approved before the commit `i`: the
    /// newest commit up to `i` that set the status and is no approval commit,
    /// or else no approval commit at all.
    fn why_not(&self, path: &str, status: &str, i: usize) -> Fail {
        self.entries
            .get(path)
            .and_then(|v| {
                v.iter().rev().find_map(|e| match &e.how {
                    How::Unapproved(f) if e.idx <= i && e.status == status => Some(f.clone()),
                    _ => None,
                })
            })
            .filter(|f| matches!(f.reason, Reason::Author | Reason::MoreThanStatus))
            .unwrap_or_else(|| Fail::new(Reason::NoCommit))
    }

    /// SD910 for `spec.md` or `plan.md`: at each commit that leaves the
    /// artifact not `draft`, the artifact `need_path` must have the status
    /// `status` (D11 and the draft rule). It returns the approval commits that
    /// the gates used.
    pub fn artifact(
        &self,
        path: &str,
        need_path: &str,
        status: &str,
    ) -> Result<Vec<Approval>, Fail> {
        let mut used = Vec::new();
        for i in self.touches_of(path) {
            if self.status_of(path, i + 1) == Some("draft") {
                continue;
            }
            if let Source::Branch(a) = self.need(need_path, status, i)? {
                used.push(a);
            }
        }
        Ok(used)
    }

    /// SD910 for code: at each commit that changes `path`, the plan
    /// `plan_path` must be `approved`, and `path` must be in its `Files` at the
    /// approval commit. A plan of the base tree has the scope of the head, as
    /// in 0.6.
    pub fn code(&self, plan_path: &str, path: &str) -> Result<Vec<Approval>, Fail> {
        let mut used = Vec::new();
        for i in self.touches_of(path) {
            if let Source::Branch(a) = self.need(plan_path, "approved", i)? {
                if !change::in_scope(&a.scope, path) {
                    return Err(Fail::at(Reason::Scope, &a.commit));
                }
                used.push(a);
            }
        }
        Ok(used)
    }

    /// The rule of the last approval (D11): the artifact has the status
    /// `status`, which differs from the base tree. An approval commit must have
    /// set it. A status that only the working tree (or the index) sets gives
    /// `None`: no commit holds it yet, and the approver has to run `check` (and
    /// the hook of `init`) before the approval commit.
    pub fn last(&self, path: &str, status: &str) -> Result<Option<Approval>, Fail> {
        match self.entries.get(path).and_then(|v| v.last()) {
            Some(e) if e.status == status && e.idx == self.n => Ok(None),
            Some(e) if e.status == status => match &e.how {
                How::Approved(a) => Ok(Some(a.clone())),
                How::Unapproved(f) => Err(f.clone()),
            },
            _ => Err(Fail::new(Reason::NoCommit)),
        }
    }
}

/// The stops of D13 for a walk of the commits of `base_rev..HEAD`, or `None`. The
/// first stop is an incomplete history of a shallow clone. The second is a merge
/// commit that merges another branch: it is neutral if the branch took the base
/// (each parent after the first is in the history of the base), or if its first
/// parent is the base (the test merge of a code host).
fn walk_stop(git: &Git, base_rev: &str, walk: &BranchWalk) -> Option<Fail> {
    if walk.shallow
        && (walk.merge_base.is_none() || walk.commits.iter().any(|c| c.parents.is_empty()))
    {
        return Some(Fail::new(Reason::Shallow));
    }
    walk.commits
        .iter()
        .find(|c| {
            c.parents.len() > 1
                && c.parents[0] != base_rev
                && !c.parents[1..].iter().all(|p| git.is_ancestor(p, base_rev))
        })
        .map(|c| Fail::at(Reason::Merge, &c.commit))
}

// The acknowledgement of the acceptance record (design 0.8.0, D3 item 10).

/// What the walk of the commits finds about the record of one change with
/// `[change] approval = "branch"`. It is independent of the walk of the status
/// of the artifacts ([`Branch`]): the record is no artifact, and it has no status.
#[derive(Clone, Debug, Default)]
pub struct AckWalk {
    /// Set if the walk cannot run (D13). Then no entry counts.
    pub stop: Option<Fail>,
    /// The entries of `acknowledged-by` that count, in lower case. An entry counts
    /// when its commit added it, its author has that e-mail, the commit changes
    /// the `acknowledged-by` line of the record alone, and no later commit changes
    /// the record. The commit of `finish` moves the same bytes to the archive, so
    /// it is no later change. The entries of the record in the base tree count: the
    /// pull request that added them walked them.
    pub counted: Vec<String>,
    /// The commits of the current life of the record that change the record and
    /// another path, each with the first other path. The commit is empty for the
    /// staged changes of `check --staged`. A life ends at a commit that deletes the
    /// record with no move to the archive, so an earlier commit gives no finding (D80
    /// item 1).
    pub mixed: Vec<(String, String)>,
}

/// True if the commit with these `paths` takes away the record of the change in
/// `record` for good: it deletes the record, or the archive file of the change, and
/// moves it nowhere. The commit of `finish` (the record out, the archive in) moves
/// the record, and the revert of that commit moves it back, so each continues the life
/// (D80 item 1).
fn ends_life(paths: &[(String, PathChange)], record: &str) -> bool {
    let slug = record.rsplit('/').nth(1).unwrap_or_default();
    let archive = |p: &str| crate::acceptance::archive_number(p, slug).is_some();
    let has = |k: PathChange, f: &dyn Fn(&str) -> bool| paths.iter().any(|(p, c)| *c == k && f(p));
    let record_out = has(PathChange::Deleted, &|p| p == record);
    let archive_out = has(PathChange::Deleted, &archive);
    let record_in = has(PathChange::Added, &|p| p == record);
    let archive_in = has(PathChange::Added, &|p| {
        crate::acceptance::is_archive_path(p)
    });
    (record_out && !archive_in) || (archive_out && !record_in && !record_out)
}

fn lower(list: Vec<String>) -> Vec<String> {
    list.into_iter().map(|e| e.to_lowercase()).collect()
}

/// One change of the record text by a commit (or by the staged changes). `old` is
/// the text before, `only_record` is true if the commit changes no other path, and
/// `author` is the e-mail of the author, if it is known.
fn ack_step(
    counted: &mut Vec<String>,
    old: Option<&str>,
    new: &str,
    only_record: bool,
    author: Option<&str>,
) {
    let ack_only = only_record
        && old.is_some_and(|o| {
            record::ack_line(o).is_some()
                && record::ack_line(new).is_some()
                && record::without_ack(o) == record::without_ack(new)
        });
    if !ack_only {
        // Another change of the record: no entry outlives the text it acknowledged.
        counted.clear();
        return;
    }
    let old_list = lower(record::ack_entries(old.unwrap_or_default()));
    let new_list = lower(record::ack_entries(new));
    counted.retain(|e| new_list.contains(e));
    let added: Vec<&String> = new_list.iter().filter(|e| !old_list.contains(e)).collect();
    if let [one] = added.as_slice()
        && author.is_none_or(|a| a.eq_ignore_ascii_case(one))
        && !counted.contains(one)
    {
        counted.push((*one).clone());
    }
}

/// Walks the commits of `base_rev..HEAD` for the record at `record` (a path from
/// the repository root). With a `staged` `Git` the staged changes are one more
/// commit after the last, with the author that `git commit` would use.
pub fn ack_walk(git: &Git, base_rev: &str, record: &str) -> Result<AckWalk, String> {
    let mut out = AckWalk::default();
    let walk = match git.branch_walk(base_rev, "HEAD") {
        Ok(w) => w,
        Err(_) => {
            out.stop = Some(Fail::new(Reason::Shallow));
            return Ok(out);
        }
    };
    if let Some(f) = walk_stop(git, base_rev, &walk) {
        out.stop = Some(f);
        return Ok(out);
    }
    let base_text = git
        .read_at(&[(base_rev.to_string(), record.to_string())])?
        .into_iter()
        .next()
        .flatten();
    let mut counted: Vec<String> = base_text
        .map(|b| lower(record::ack_entries(&String::from_utf8_lossy(&b))))
        .unwrap_or_default();
    // The texts of the record, in one batch.
    let mut specs: Vec<(String, String)> = Vec::new();
    for c in walk.commits.iter().filter(|c| c.parents.len() <= 1) {
        if let Some((_, k)) = c.paths.iter().find(|(p, _)| p == record)
            && *k != PathChange::Deleted
        {
            specs.push((c.commit.clone(), record.to_string()));
            if *k == PathChange::Modified && c.parents.len() == 1 {
                specs.push((c.parents[0].clone(), record.to_string()));
            }
        }
    }
    let texts = match git.read_at(&specs) {
        Ok(t) if t.iter().all(Option::is_some) => t,
        _ => {
            out.stop = Some(Fail::new(Reason::Shallow));
            return Ok(out);
        }
    };
    let text_at: HashMap<(&str, &str), String> = specs
        .iter()
        .zip(&texts)
        .filter_map(|((c, p), t)| {
            t.as_ref().map(|t| {
                (
                    (c.as_str(), p.as_str()),
                    String::from_utf8_lossy(t).into_owned(),
                )
            })
        })
        .collect();
    for c in &walk.commits {
        let Some((_, kind)) = c.paths.iter().find(|(p, _)| p == record) else {
            // A commit that takes the archive file away and holds no record ends
            // the life too: the revert of `finish` with the record deleted (D80 item 2).
            if c.parents.len() <= 1 && ends_life(&c.paths, record) {
                out.mixed.clear();
            }
            continue;
        };
        if c.parents.len() > 1 {
            // A merge commit that carries the record itself (a new file, or a fix of
            // a conflict). The paths of a merge are the paths that differ from each
            // parent, so a path that the merge only takes from the base is not here.
            // The record with another path in one commit is SD932, as in any commit.
            if *kind == PathChange::Deleted {
                if ends_life(&c.paths, record) {
                    out.mixed.clear();
                }
            } else if let Some((p, _)) = c.paths.iter().find(|(p, _)| p != record) {
                out.mixed.push((c.commit.clone(), p.clone()));
            }
            counted.clear();
            continue;
        }
        if *kind == PathChange::Deleted {
            // The commit of `finish` deletes the folder and adds the archive file
            // with the same bytes. Any other deletion ends the record, and its life.
            let moved = c
                .paths
                .iter()
                .any(|(p, k)| *k == PathChange::Added && crate::acceptance::is_archive_path(p));
            if !moved {
                counted.clear();
            }
            if ends_life(&c.paths, record) {
                out.mixed.clear();
            }
            continue;
        }
        let other = c
            .paths
            .iter()
            .find(|(p, _)| p != record)
            .map(|(p, _)| p.clone());
        if let Some(p) = &other {
            out.mixed.push((c.commit.clone(), p.clone()));
        }
        let new = text_at.get(&(c.commit.as_str(), record));
        let old = (*kind == PathChange::Modified && c.parents.len() == 1)
            .then(|| text_at.get(&(c.parents[0].as_str(), record)))
            .flatten();
        ack_step(
            &mut counted,
            old.map(String::as_str),
            new.map_or("", String::as_str),
            other.is_none(),
            Some(&c.author_email),
        );
    }
    // The staged changes (the hook `check --staged`) are one more commit, with the
    // rules of a commit of the walk: a deletion of the record is no edit, and the
    // commit of `finish` keeps the entries that count.
    if git.is_staged() {
        let paths = git.diff_paths("HEAD", false)?;
        if !paths.iter().any(|(p, _)| p == record) && ends_life(&paths, record) {
            out.mixed.clear();
        }
        if let Some((_, kind)) = paths.iter().find(|(p, _)| p == record) {
            if *kind == PathChange::Deleted {
                let moved = paths
                    .iter()
                    .any(|(p, k)| *k == PathChange::Added && crate::acceptance::is_archive_path(p));
                if !moved {
                    counted.clear();
                }
                if ends_life(&paths, record) {
                    out.mixed.clear();
                }
            } else if let Some(new) = git.head_file(record)? {
                let other = paths
                    .iter()
                    .find(|(p, _)| p != record)
                    .map(|(p, _)| p.clone());
                if let Some(p) = &other {
                    out.mixed.push((String::new(), p.clone()));
                }
                let old = git
                    .read_at(&[("HEAD".to_string(), record.to_string())])?
                    .into_iter()
                    .next()
                    .flatten()
                    .map(|b| String::from_utf8_lossy(&b).into_owned());
                let author = git.author_email().ok();
                ack_step(
                    &mut counted,
                    old.as_deref(),
                    &String::from_utf8_lossy(&new),
                    other.is_none(),
                    author.as_deref(),
                );
            }
        }
    }
    counted.sort();
    counted.dedup();
    out.counted = counted;
    Ok(out)
}

/// The status of each artifact of a tree, by path.
fn statuses(base: &Corpus) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for ch in change::collect(base) {
        for a in [&ch.intent, &ch.spec, &ch.plan].into_iter().flatten() {
            out.insert(base.docs[a.doc as usize].path.clone(), a.status.clone());
        }
    }
    out
}

/// The kind of an artifact path: `<intent dir>/<slug>/intent.md`, `spec.md` or
/// `plan.md`.
fn kind_of(dirs: &[String], path: &str) -> Option<Kind> {
    dirs.iter().find_map(|d| {
        let rest = path.strip_prefix(d.as_str())?.strip_prefix('/')?;
        let (slug, file) = rest.split_once('/')?;
        if slug.is_empty() {
            return None;
        }
        match file {
            "intent.md" => Some(Kind::Intent),
            "spec.md" => Some(Kind::Spec),
            "plan.md" => Some(Kind::Plan),
            _ => None,
        }
    })
}

/// The `status` of the front matter of a document, if it has one.
fn status_of(text: &str) -> Option<String> {
    let (fm, _) = frontmatter::split(text)?;
    let parsed = frontmatter::parse(fm, 2).ok()?;
    match parsed.get("status") {
        Some((frontmatter::Value::Str(s), _)) => Some(s.clone()),
        _ => None,
    }
}

/// True if the 2 texts differ in one line only, and that line is the `status`
/// line of the front matter (condition 3 of D11).
fn only_status_differs(old: &str, new: &str) -> bool {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.split('\n').collect(), new.split('\n').collect());
    if a.len() != b.len() {
        return false;
    }
    let Some((fm, _)) = frontmatter::split(new) else {
        return false;
    };
    let Ok(parsed) = frontmatter::parse(fm, 2) else {
        return false;
    };
    let Some((_, line)) = parsed.get("status") else {
        return false;
    };
    let differ: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    differ == [line as usize - 1] && a[differ[0]].trim_start().starts_with("status:")
}

/// The patterns of `Files` of a plan text. The path of the plan decides its
/// type and its change.
fn plan_files(config: &Config, path: &str, text: &str) -> Vec<String> {
    let docs: Vec<(String, Arc<ParsedDoc>)> = vec![(path.to_string(), Arc::new(parse(text)))];
    let corpus = Corpus::build(config, vec![path.to_string()], docs);
    change::collect(&corpus)
        .into_iter()
        .find_map(|c| c.plan.is_some().then(|| c.files(&corpus)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_only_change_has_one_line_that_is_the_status() {
        let old = "---\nstatus: draft\nclass: routine\n---\n# T\n";
        let new = "---\nstatus: accepted\nclass: routine\n---\n# T\n";
        assert!(only_status_differs(old, new));
        // The status and a text line.
        assert!(!only_status_differs(old, &format!("{new}x\n")));
        assert!(!only_status_differs(
            old,
            "---\nstatus: accepted\nclass: higher-risk\n---\n# T\n"
        ));
        // A line of the body that has the word `status:` is no status line.
        let old = "---\nstatus: draft\n---\n# T\n\nstatus: draft\n";
        let new = "---\nstatus: draft\n---\n# T\n\nstatus: accepted\n";
        assert!(!only_status_differs(old, new));
        assert!(!only_status_differs("a\n", "b\n"));
    }

    #[test]
    fn the_kind_of_an_artifact_path() {
        let dirs = vec!["docs/intent".to_string()];
        assert_eq!(kind_of(&dirs, "docs/intent/x/plan.md"), Some(Kind::Plan));
        assert_eq!(
            kind_of(&dirs, "docs/intent/x/intent.md"),
            Some(Kind::Intent)
        );
        assert_eq!(kind_of(&dirs, "docs/intent/x/y/plan.md"), None);
        assert_eq!(kind_of(&dirs, "docs/intent/x/notes.md"), None);
        assert_eq!(kind_of(&dirs, "docs/other/x/plan.md"), None);
        assert_eq!(kind_of(&dirs, "docs/intent/plan.md"), None);
    }

    #[test]
    fn the_status_of_a_text() {
        assert_eq!(
            status_of("---\nstatus: approved\n---\n# T\n").as_deref(),
            Some("approved")
        );
        assert_eq!(status_of("---\nclass: routine\n---\n"), None);
        assert_eq!(status_of("# T\n"), None);
    }

    #[test]
    fn reason_9_names_the_commit_or_the_working_tree_and_the_artifact() {
        let f = Fail::took_back(Some("0123456789abcdef"), "docs/a/plan.md", "draft");
        assert_eq!(
            f.sentence(),
            "reason 9: the commit 0123456789ab set `docs/a/plan.md` back to `draft` after its approval; approve it again with an approval commit before the work, or take the work out of the branch"
        );
        let f = Fail::took_back(None, "docs/a/plan.md", "draft");
        assert!(
            f.sentence()
                .starts_with("reason 9: the working tree set `docs/a/plan.md` back to `draft`")
        );
    }

    fn rec_text(ack: &str, extra: &str) -> String {
        format!("format = 2\nacknowledged-by = [{ack}]\n{extra}")
    }

    #[test]
    fn an_entry_counts_only_for_an_approval_commit_of_its_author() {
        let (a, b) = ("\"a@x.test\"", "\"b@x.test\"");
        let mut counted: Vec<String> = Vec::new();
        let old = rec_text("", "");
        // An approval commit of its author.
        ack_step(
            &mut counted,
            Some(&old),
            &rec_text(a, ""),
            true,
            Some("A@X.test"),
        );
        assert_eq!(counted, ["a@x.test"]);
        // A second approver keeps the first entry.
        ack_step(
            &mut counted,
            Some(&rec_text(a, "")),
            &rec_text(&format!("{a}, {b}"), ""),
            true,
            Some("b@x.test"),
        );
        assert_eq!(counted, ["a@x.test", "b@x.test"]);
        // An entry that is not the author does not count, and the others stay.
        let mut c2 = counted.clone();
        ack_step(
            &mut c2,
            Some(&rec_text(&format!("{a}, {b}"), "")),
            &rec_text(&format!("{a}, {b}, \"c@x.test\""), ""),
            true,
            Some("b@x.test"),
        );
        assert_eq!(c2, ["a@x.test", "b@x.test"]);
        // A removal keeps the entries that stay.
        let mut c3 = counted.clone();
        ack_step(
            &mut c3,
            Some(&rec_text(&format!("{a}, {b}"), "")),
            &rec_text(b, ""),
            true,
            Some("b@x.test"),
        );
        assert_eq!(c3, ["b@x.test"]);
        // Another change of the record clears each entry.
        let mut c4 = counted.clone();
        ack_step(
            &mut c4,
            Some(&rec_text(&format!("{a}, {b}"), "")),
            &rec_text(&format!("{a}, {b}"), "x = 1\n"),
            true,
            Some("a@x.test"),
        );
        assert!(c4.is_empty());
        // The commit changes another path too: no approval commit.
        let mut c5: Vec<String> = Vec::new();
        ack_step(
            &mut c5,
            Some(&old),
            &rec_text(a, ""),
            false,
            Some("a@x.test"),
        );
        assert!(c5.is_empty());
        // A new record (no old text) is no approval commit.
        ack_step(&mut c5, None, &rec_text(a, ""), true, Some("a@x.test"));
        assert!(c5.is_empty());
        // The staged changes with no known author: one added entry counts.
        ack_step(&mut c5, Some(&old), &rec_text(a, ""), true, None);
        assert_eq!(c5, ["a@x.test"]);
        // 2 entries added in one commit: none counts.
        let mut c6: Vec<String> = Vec::new();
        ack_step(
            &mut c6,
            Some(&old),
            &rec_text(&format!("{a}, {b}"), ""),
            true,
            Some("a@x.test"),
        );
        assert!(c6.is_empty());
    }

    #[test]
    fn each_reason_has_a_number_and_names_it() {
        let all = [
            Reason::NoBase,
            Reason::NoCommit,
            Reason::Author,
            Reason::MoreThanStatus,
            Reason::Shallow,
            Reason::Merge,
            Reason::Born,
            Reason::Scope,
            Reason::TookBack,
        ];
        for (i, r) in all.iter().enumerate() {
            assert_eq!(r.number() as usize, i + 1);
            assert!(
                r.sentence(Some("0123456789abcdef"), None)
                    .starts_with(&format!("reason {}:", i + 1))
            );
        }
    }
}
