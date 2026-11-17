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
//! The rule fails closed (D13). A shallow clone with an incomplete range and
//! a merge commit that is not neutral stop it: the branch then gives no
//! approval, and SD910 reports as in 0.6. A working tree that differs from
//! `HEAD` counts as one more commit, after the last one, with no author.

use crate::change;
use crate::config::Config;
use crate::frontmatter;
use crate::git::{Git, PathChange};
use crate::lifecycle::intent_dirs;
use crate::model::Corpus;
use crate::parse::{ParsedDoc, parse};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// The reason of an SD910 finding (D17). Each message names one of the 8.
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
}

impl Reason {
    /// The number of the reason, from 1 to 8.
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
        }
    }

    /// The end of a message: the number of the reason, what it means and its
    /// fix. `commit` is the commit that the reason names, if it has one.
    pub fn sentence(self, commit: Option<&str>) -> String {
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
        }
    }
}

/// A reason, with the commit that it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fail {
    pub reason: Reason,
    pub commit: Option<String>,
}

impl Fail {
    pub fn new(reason: Reason) -> Fail {
        Fail {
            reason,
            commit: None,
        }
    }

    fn at(reason: Reason, commit: &str) -> Fail {
        Fail {
            reason,
            commit: Some(commit.to_string()),
        }
    }

    /// The end of a message (`Reason::sentence`).
    pub fn sentence(&self) -> String {
        self.reason.sentence(self.commit.as_deref())
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
    entries: HashMap<String, Vec<Entry>>,
    /// For each path, the indexes of the commits that changed it.
    touches: HashMap<String, Vec<usize>>,
    /// The status of each artifact in the base tree.
    base: HashMap<String, String>,
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
            entries: HashMap::new(),
            touches: HashMap::new(),
            base: base_statuses(base),
        };
        let walk = match git.branch_walk(base_rev, "HEAD") {
            Ok(w) => w,
            Err(_) => {
                b.stop = Some(Fail::new(Reason::Shallow));
                return b;
            }
        };
        b.n = walk.commits.len();
        // The first stop of D13: an incomplete history of a shallow clone.
        if walk.shallow
            && (walk.merge_base.is_none() || walk.commits.iter().any(|c| c.parents.is_empty()))
        {
            b.stop = Some(Fail::new(Reason::Shallow));
            return b;
        }
        // The second stop: a merge commit that merges another branch. It is
        // neutral if the branch took the base (each parent after the first is
        // in the history of the base), or if its first parent is the base (the
        // test merge of a code host).
        for c in &walk.commits {
            if c.parents.len() > 1
                && c.parents[0] != base_rev
                && !c.parents[1..].iter().all(|p| git.is_ancestor(p, base_rev))
            {
                b.stop = Some(Fail::at(Reason::Merge, &c.commit));
                return b;
            }
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
        let text_at: HashMap<(&str, &str), &Vec<u8>> = specs
            .iter()
            .zip(&texts)
            .filter_map(|((c, p), t)| t.as_ref().map(|t| ((c.as_str(), p.as_str()), t)))
            .collect();
        for (i, c) in walk.commits.iter().enumerate() {
            // A neutral merge brings the base, or the commits that the walk
            // lists on their own.
            if c.parents.len() > 1 {
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
                    How::Unapproved(Fail::new(Reason::NoCommit))
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
            if kind_of(&dirs, &p).is_none() || k == PathChange::Deleted {
                continue;
            }
            let Some(doc) = head.docs.iter().find(|d| d.path == p) else {
                continue;
            };
            let status = doc.status_str().to_string();
            if b.status_of(&p, n) != Some(status.as_str()) {
                b.entries.entry(p).or_default().push(Entry {
                    idx: n,
                    status,
                    how: How::Unapproved(Fail::new(Reason::NoCommit)),
                });
            }
        }
        b
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
                How::Approved(a) => Ok(Source::Branch(a.clone())),
                How::Unapproved(f) => Err(f.clone()),
            },
            Some(_) => Err(self.why_not(need_path, status, i)),
            None if self.base_status(need_path) == Some(status) => Ok(Source::Base),
            None => Err(self.why_not(need_path, status, i)),
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
    /// set it.
    pub fn last(&self, path: &str, status: &str) -> Result<Approval, Fail> {
        match self.entries.get(path).and_then(|v| v.last()) {
            Some(e) if e.status == status => match &e.how {
                How::Approved(a) => Ok(a.clone()),
                How::Unapproved(f) => Err(f.clone()),
            },
            _ => Err(Fail::new(Reason::NoCommit)),
        }
    }
}

/// The status of each artifact of the base tree, by path.
fn base_statuses(base: &Corpus) -> HashMap<String, String> {
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
        ];
        for (i, r) in all.iter().enumerate() {
            assert_eq!(r.number() as usize, i + 1);
            assert!(
                r.sentence(Some("0123456789abcdef"))
                    .starts_with(&format!("reason {}:", i + 1))
            );
        }
    }
}
