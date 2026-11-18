//! This module accesses git through the commands of the git CLI (ADR 0002, git
//! access decision). Every command runs with `GIT_OPTIONAL_LOCKS=0`, so that a
//! read never writes the index of the repository. No command uses the network.
//!
//! This module uses only `std`. `tests/git_lifecycle.rs` compiles it as a
//! module of its own, because the crate has no library.
//!
//! A shallow clone has fewer commits than the full repository. For a path, the
//! oldest commit that the clone has (the boundary commit) shows each of its
//! files as added. Thus `last_touch` in a shallow clone gives a date that is
//! the same as the date of the full repository or later. A shallow clone can
//! hide a stale change, but it never reports a false one. `deleted_intents`
//! and `added_intents` can miss a commit that the clone does not have. They
//! never give an entry that the full repository does not have.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct Git {
    root: PathBuf,
    /// True for `check --staged`: the head tree is the git index, not the
    /// working tree. The readers of the head tree then use `--cached` and
    /// never read untracked files.
    staged: bool,
}

/// A tree entry: path, object id, and whether it is a submodule.
pub type Entry = (String, String, bool);

/// The kind of a change of one path in a diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathChange {
    Added,
    Modified,
    Deleted,
}

/// A commit that deleted an `intent.md` (`Git::deleted_intents`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletedIntent {
    /// The deleting commit. It has at most 1 parent.
    pub commit: String,
    /// Its parent, or `None` for a root commit.
    pub parent: Option<String>,
    /// The committer date of the commit, in seconds.
    pub time: i64,
    /// The deleted file, from the repository root.
    pub path: String,
}

/// A touch of an `intent.md`, a `spec.md` or a `plan.md` by a commit
/// (`Git::artifact_touches`). A rename is a touch of the new path with the old
/// path in `from`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactTouch {
    pub commit: String,
    /// The parents of the commit. The list has at most 1 item, because a merge
    /// commit gives no touch.
    pub parents: Vec<String>,
    /// The committer date of the commit, in seconds.
    pub time: i64,
    /// The file, from the repository root.
    pub path: String,
    pub kind: PathChange,
    /// The old path, if the commit renamed the file (`kind` is `Modified`).
    pub from: Option<String>,
}

/// A commit of the range `<base>..<head>` (`Git::branch_walk`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchCommit {
    pub commit: String,
    /// The parents of the commit, first parent first. A root commit has none.
    /// A merge commit has 2 or more.
    pub parents: Vec<String>,
    /// The e-mail of the author (`%ae`), as the commit stores it. The mailmap
    /// does not change it, so a `.mailmap` file of the branch makes no
    /// approver.
    pub author_email: String,
    /// The paths that the commit changed against its parent, with the kind of
    /// each change. A root commit adds all its files. A merge commit lists only
    /// the paths that differ from each parent, all with the kind `Modified`.
    /// The diff has no rename detection, so a move is a deletion and an
    /// addition.
    pub paths: Vec<(String, PathChange)>,
}

/// The commits of a branch (`Git::branch_walk`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchWalk {
    /// The commits of `<base>..<head>`, the oldest first. A parent comes
    /// before each of its children. The order is the order of the ancestry,
    /// not of the commit dates.
    pub commits: Vec<BranchCommit>,
    /// The merge base of `base` and `head`, or `None` if the history that
    /// the repository has shows none.
    pub merge_base: Option<String>,
    /// True if the repository is a shallow clone. The history is complete
    /// only if `merge_base` is `Some`: a grafted boundary commit has no
    /// parents, like a root commit. With `shallow` and no `merge_base`, the
    /// walk is not a proof, so a rule that needs the order of the commits
    /// stops (design 0.7.0, D13).
    pub shallow: bool,
}

/// True if `token` is the status of a record of `diff-tree --name-status`:
/// capital letters only, one for each parent. A commit id has digits and small
/// letters, so it is no status.
fn is_status(token: &[u8]) -> bool {
    !token.is_empty() && token.iter().all(|b| b"ACDMRTUXB".contains(b))
}

#[derive(Default)]
pub struct Status {
    /// Tracked files whose working-tree content differs from the index.
    pub modified: HashSet<String>,
    /// Tracked files deleted from the working tree.
    pub deleted: HashSet<String>,
    /// Untracked files that git does not ignore.
    pub untracked: Vec<String>,
}

impl Git {
    pub fn new(root: &Path) -> Git {
        Git {
            root: root.to_path_buf(),
            staged: false,
        }
    }

    /// The same repository, with the git index as its head tree.
    pub fn staged(&self) -> Git {
        Git {
            root: self.root.clone(),
            staged: true,
        }
    }

    /// Returns the paths that the index has as intent-to-add (`git add -N`).
    /// The index lists them with the empty blob, but a commit leaves them out.
    pub fn intent_to_add(&self) -> Result<HashSet<String>, String> {
        let out = self.run(&["diff-files", "--name-status", "--diff-filter=A", "-z"])?;
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        let mut paths = HashSet::new();
        while let (Some(_status), Some(path)) = (recs.next(), recs.next()) {
            paths.insert(String::from_utf8_lossy(path).into_owned());
        }
        Ok(paths)
    }

    /// Returns the first path that has an unmerged entry in the git index, or
    /// `None`.
    pub fn first_unmerged(&self) -> Result<Option<String>, String> {
        let out = self.run(&["ls-files", "-u", "-z"])?;
        Ok(out
            .split(|b| *b == 0)
            .find(|r| !r.is_empty())
            .and_then(|rec| {
                String::from_utf8_lossy(rec)
                    .split_once('\t')
                    .map(|(_, p)| p.to_string())
            }))
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new("git");
        c.arg("-C").arg(&self.root).args(args);
        c.env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null());
        c
    }

    /// Like `run`, and git never fetches an absent object from a remote. A
    /// reader of history that uses rename detection needs this in a partial
    /// clone.
    fn run_offline(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let out = self
            .cmd(args)
            .env("GIT_NO_LAZY_FETCH", "1")
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(out.stdout)
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let out = self
            .cmd(args)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(out.stdout)
    }

    /// Returns the paths that differ between a commit and the working tree
    /// (tracked files), each with `true` if the commit does not have it. With
    /// `untracked`, it also returns the untracked files that git does not
    /// ignore, as new paths.
    pub fn changed_since(&self, rev: &str, untracked: bool) -> Result<Vec<(String, bool)>, String> {
        Ok(self
            .diff_paths(rev, untracked)?
            .into_iter()
            .map(|(p, k)| (p, k == PathChange::Added))
            .collect())
    }

    /// Like `changed_since`, with the kind of each change. A deleted file is
    /// in the result. The diff has no rename detection, so a move is a deletion
    /// and an addition.
    pub fn diff_paths(
        &self,
        rev: &str,
        untracked: bool,
    ) -> Result<Vec<(String, PathChange)>, String> {
        let untracked = untracked && !self.staged;
        let mut args = vec!["diff"];
        if self.staged {
            args.push("--cached");
        }
        args.extend([
            "--name-status",
            "-z",
            "--no-renames",
            "--ignore-submodules=all",
            rev,
            "--",
        ]);
        let out = self.run(&args)?;
        let mut v = Vec::new();
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let (Some(status), Some(path)) = (recs.next(), recs.next()) {
            let kind = match status.first() {
                Some(b'A') => PathChange::Added,
                Some(b'D') => PathChange::Deleted,
                _ => PathChange::Modified,
            };
            v.push((String::from_utf8_lossy(path).into_owned(), kind));
        }
        if untracked {
            let out = self.run(&["ls-files", "--others", "--exclude-standard", "-z"])?;
            for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
                v.push((String::from_utf8_lossy(rec).into_owned(), PathChange::Added));
            }
        }
        Ok(v)
    }

    /// Returns the new paths of the renames between a commit and the working
    /// tree (or the git index, for a `staged` `Git`), with the rename
    /// detection of git. Only a tracked path can be the target of a rename, so
    /// an untracked file is never in the result.
    pub fn renamed_paths(&self, rev: &str) -> Result<Vec<String>, String> {
        let mut args = vec!["diff"];
        if self.staged {
            args.push("--cached");
        }
        args.extend([
            "--name-status",
            "-z",
            "-M",
            "--ignore-submodules=all",
            rev,
            "--",
        ]);
        let out = self.run(&args)?;
        let mut v = Vec::new();
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let (Some(status), Some(_)) = (recs.next(), recs.next()) {
            // A rename or a copy has the old path, then the new path.
            match status.first() {
                Some(b'R') => {
                    if let Some(new) = recs.next() {
                        v.push(String::from_utf8_lossy(new).into_owned());
                    }
                }
                Some(b'C') => {
                    recs.next();
                }
                _ => {}
            }
        }
        Ok(v)
    }

    /// Returns the paths and blob ids of the git index (stage 0, or the first
    /// stage of a conflicted path).
    pub fn ls_files(&self) -> Result<Vec<Entry>, String> {
        let out = self.run(&["ls-files", "-s", "-z"])?;
        let mut v: Vec<Entry> = Vec::new();
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            let Some((meta, path)) = rec.split_once('\t') else {
                continue;
            };
            let mut it = meta.split(' ');
            let (mode, oid) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
            if v.last().is_some_and(|l| l.0 == path) {
                continue;
            }
            v.push((path.to_string(), oid.to_string(), mode == "160000"));
        }
        Ok(v)
    }

    /// Returns the files of the working tree under `specs` (git pathspecs):
    /// the tracked files, and the untracked files that git does not ignore.
    /// The paths are from the repository root, sorted, with no duplicate. A
    /// tracked file that is deleted in the working tree is also listed, so a
    /// reader must skip a path that is absent. `eval run` uses it.
    pub fn list_worktree(&self, specs: &[String]) -> Result<Vec<String>, String> {
        let mut args = vec![
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
        ];
        args.extend(specs.iter().map(String::as_str));
        let out = self.run(&args)?;
        let mut paths: Vec<String> = out
            .split(|b| *b == 0)
            .filter(|r| !r.is_empty())
            .map(|r| String::from_utf8_lossy(r).into_owned())
            .collect();
        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    /// Returns the working-tree changes in `paths`. If `paths` is empty, it
    /// returns the changes in the whole tree. With `untracked`, it also lists
    /// the untracked files that git does not ignore.
    pub fn status(&self, paths: &[String], untracked: bool) -> Result<Status, String> {
        let mut args = vec![
            "status",
            "--porcelain=v2",
            "-z",
            if untracked {
                "--untracked-files=all"
            } else {
                "--untracked-files=no"
            },
            "--no-renames",
            "--ignore-submodules=all",
            "--",
        ];
        args.extend(paths.iter().map(String::as_str));
        let out = self.run(&args)?;
        let mut st = Status::default();
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let Some(rec) = recs.next() {
            let rec = String::from_utf8_lossy(rec);
            let kind = rec.as_bytes()[0];
            match kind {
                b'1' | b'2' | b'u' => {
                    let fields = match kind {
                        b'1' => 8,
                        b'2' => 9,
                        _ => 10,
                    };
                    let mut it = rec.splitn(fields + 1, ' ');
                    let xy = it.nth(1).unwrap_or("..");
                    let path = it.last().unwrap_or("").to_string();
                    if kind == b'2' {
                        recs.next(); // skip the original path of a rename
                    }
                    let y = xy.as_bytes().get(1).copied().unwrap_or(b'.');
                    if kind == b'u' || y == b'M' || y == b'T' || y == b'A' {
                        st.modified.insert(path);
                    } else if y == b'D' {
                        st.deleted.insert(path);
                    }
                }
                b'?' => st.untracked.push(rec[2..].to_string()),
                _ => {}
            }
        }
        Ok(st)
    }

    pub fn ls_tree(&self, rev: &str) -> Result<Vec<Entry>, String> {
        let out = self.run(&["ls-tree", "-r", "-z", "--full-tree", rev])?;
        let mut v = Vec::new();
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            let Some((meta, path)) = rec.split_once('\t') else {
                continue;
            };
            let mut it = meta.split(' ');
            let (_mode, ty, oid) = (it.next(), it.next().unwrap_or(""), it.next().unwrap_or(""));
            v.push((path.to_string(), oid.to_string(), ty == "commit"));
        }
        Ok(v)
    }

    /// Returns the commit id of a revision, or `None` if the revision does not
    /// name a commit.
    pub fn commit(&self, rev: &str) -> Option<String> {
        let spec = format!("{rev}^{{commit}}");
        let out = self
            .run(&["rev-parse", "--verify", "--quiet", &spec])
            .ok()?;
        let s = String::from_utf8_lossy(&out).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    pub fn merge_base(&self, a: &str, b: &str) -> Option<String> {
        let out = self.run(&["merge-base", a, b]).ok()?;
        let s = String::from_utf8_lossy(&out).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// Returns the commit that `refs/remotes/origin/HEAD` names, or `None`.
    /// It reads only this ref, with no guess of a branch, so that the result is
    /// the same for every checkout. `git remote set-head origin <branch>` sets
    /// the ref. For a repository with no remote, `[change] base` names a local
    /// branch (`local_branch`).
    pub fn remote_head(&self) -> Option<String> {
        self.commit("refs/remotes/origin/HEAD")
    }

    /// Returns the commit of the local branch `name`, or `None` if the
    /// repository has no such branch. A tag, a remote branch or a commit id
    /// of the same name is not a local branch. The lookup is for the exact
    /// ref, so a revision form such as `main@{1}` (the reflog, which differs
    /// in each clone) names no branch.
    pub fn local_branch(&self, name: &str) -> Option<String> {
        let out = self
            .run(&[
                "show-ref",
                "--verify",
                "--hash",
                &format!("refs/heads/{name}"),
            ])
            .ok()?;
        let s = String::from_utf8_lossy(&out).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// Returns the committer date of a commit, in seconds. It never reads the
    /// clock.
    pub fn commit_time(&self, rev: &str) -> Result<i64, String> {
        let out = self.run(&["log", "-1", "--format=%ct", rev, "--"])?;
        String::from_utf8_lossy(&out)
            .trim()
            .parse()
            .map_err(|_| format!("`{rev}` has no committer date"))
    }

    /// Returns true if the repository is a shallow clone.
    pub fn is_shallow(&self) -> bool {
        self.run(&["rev-parse", "--is-shallow-repository"])
            .is_ok_and(|o| String::from_utf8_lossy(&o).trim() == "true")
    }

    /// Returns the value of `user.email` of the git configuration of the
    /// repository (the local file, then the global and system files), or
    /// `None` if the key is not set or its value is empty. It never builds a
    /// name from the user and the host.
    pub fn user_email(&self) -> Option<String> {
        let out = self.run(&["config", "--get", "user.email"]).ok()?;
        let s = String::from_utf8_lossy(&out).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// True if `path` is tracked, HEAD has it, and its text is the same in
    /// HEAD, in the index and in the working tree. A repository with no
    /// commit gives `false`.
    pub fn is_clean_in_head(&self, path: &str) -> Result<bool, String> {
        let in_head = self
            .cmd(&["cat-file", "-e", &format!("HEAD:{path}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("cannot run git: {e}"))?
            .success();
        if !in_head {
            return Ok(false);
        }
        let spec = format!(":(literal){path}");
        let out = self.run(&[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=all",
            "--",
            &spec,
        ])?;
        Ok(out.is_empty())
    }

    /// The e-mail of the author of the next commit (`git var
    /// GIT_AUTHOR_IDENT`): the environment variables `GIT_AUTHOR_*` count.
    pub fn author_email(&self) -> Result<String, String> {
        let out = self.run(&["var", "GIT_AUTHOR_IDENT"])?;
        let ident = String::from_utf8_lossy(&out);
        let start = ident.rfind('<').ok_or("git gave no author e-mail")?;
        let end = ident[start..]
            .find('>')
            .ok_or("git gave no author e-mail")?;
        Ok(ident[start + 1..start + end].to_string())
    }

    /// Commits the file `path` alone (`git commit --only`). Other staged
    /// changes stay staged and out of the commit. The command passes no
    /// `--no-verify` and no `--no-gpg-sign`, so the hooks and the signing of
    /// the person apply. It returns the id of the new commit, or the output of
    /// git.
    pub fn commit_only(&self, message: &str, path: &str) -> Result<String, String> {
        let spec = format!(":(literal){path}");
        let out = self
            .cmd(&["commit", "--only", "-m", message, "--", &spec])
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        if !out.status.success() {
            let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&out.stderr));
            return Err(text.trim().to_string());
        }
        let id = self.run(&["rev-parse", "HEAD"])?;
        Ok(String::from_utf8_lossy(&id).trim().to_string())
    }

    /// Returns true if `ancestor` is `descendant` or one of its ancestors. An
    /// unknown revision gives false.
    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        self.cmd(&["merge-base", "--is-ancestor", ancestor, descendant])
            .env("GIT_NO_LAZY_FETCH", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Walks the commits of `<base>..<head>` from the oldest (design 0.7.0,
    /// D11 and D13). Each commit has its parents, the e-mail of its author and
    /// the paths that it changed. A merge commit is in the list, so the caller
    /// can apply the stops of D13. The command reads only the history that the
    /// repository has: it never fetches. `base` and `head` are revisions, such
    /// as a commit id.
    ///
    /// The walk runs a fixed number of git processes, whatever the number of
    /// commits (ADR 0019): `log` gives the commits with their parents and
    /// e-mails, and one `diff-tree --stdin` gives the paths of all commits.
    /// `merge-base` and `rev-parse --is-shallow-repository` are the other 2.
    ///
    /// The walk never parses a path and a commit header from one stream
    /// without a rule. A file name can hold any byte but NUL, so a name can
    /// look like a header. `log -z` gives 3 NUL-separated fields for each
    /// commit: the id, the parents and the e-mail. `diff-tree --stdin -z` gets
    /// the ids from the walk, one for each line, and gives for each commit its
    /// id, then the records of its paths: a status, then a path. A record is
    /// read by its place. A token that is in the place of a status is a status
    /// (letters only) or a commit id of the walk, and the token after a status
    /// is always a path, so no name is read as a header. A commit with no
    /// path, such as an empty commit, has no header in the output. For a merge
    /// commit the paths are the ones that differ from each parent: the work
    /// that the merge itself does (a conflict fix, or a new file), not the
    /// changes that it takes from a parent.
    pub fn branch_walk(&self, base: &str, head: &str) -> Result<BranchWalk, String> {
        let range = format!("{base}..{head}");
        let listed = self.run_offline(&[
            "log",
            "--reverse",
            "--topo-order",
            "-z",
            "--format=%H%x00%P%x00%ae",
            &range,
            "--",
        ])?;
        let mut commits: Vec<BranchCommit> = Vec::new();
        let mut fields = listed.split(|b| *b == 0);
        // The email of the author is as the commit stores it. `-z` ends each
        // record with NUL, so the last field is empty.
        while let (Some(id), Some(parents), Some(email)) =
            (fields.next(), fields.next(), fields.next())
        {
            if id.is_empty() {
                return Err("git gave an empty commit id".to_string());
            }
            commits.push(BranchCommit {
                commit: String::from_utf8_lossy(id).into_owned(),
                parents: String::from_utf8_lossy(parents)
                    .split(' ')
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect(),
                author_email: String::from_utf8_lossy(email).into_owned(),
                paths: Vec::new(),
            });
        }
        self.batch_paths(&mut commits)?;
        Ok(BranchWalk {
            commits,
            merge_base: self.merge_base(base, head),
            shallow: self.is_shallow(),
        })
    }

    /// Fills the paths of the commits with one `git diff-tree --stdin`. A
    /// single commit is compared with its parent (a root commit with the empty
    /// tree), so the kinds have the direction of the commit. A merge commit
    /// lists the paths that differ from each parent (`-c`), all with the kind
    /// `Modified`. The diff has no rename detection.
    fn batch_paths(&self, commits: &mut [BranchCommit]) -> Result<(), String> {
        if commits.is_empty() {
            return Ok(());
        }
        let ids: String = commits.iter().map(|c| format!("{}\n", c.commit)).collect();
        let out = self.run_with_input(
            &[
                "diff-tree",
                "--stdin",
                "-r",
                "-z",
                "--root",
                "--no-renames",
                "--name-status",
                "-c",
            ],
            ids,
        )?;
        let index: HashMap<&str, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.commit.as_str(), i))
            .collect();
        let mut found: Vec<Vec<(String, PathChange)>> = vec![Vec::new(); commits.len()];
        let mut at: Option<usize> = None;
        let mut tokens = out.split(|b| *b == 0).filter(|t| !t.is_empty());
        while let Some(token) = tokens.next() {
            if is_status(token) {
                // The token after a status is a path, whatever its text.
                let (Some(i), Some(path)) = (at, tokens.next()) else {
                    return Err("git gave a path with no commit".to_string());
                };
                let kind = match token.first() {
                    _ if commits[i].parents.len() > 1 => PathChange::Modified,
                    Some(b'A') => PathChange::Added,
                    Some(b'D') => PathChange::Deleted,
                    _ => PathChange::Modified,
                };
                found[i].push((String::from_utf8_lossy(path).into_owned(), kind));
            } else if let Some(i) = std::str::from_utf8(token).ok().and_then(|t| index.get(t)) {
                at = Some(*i);
            } else {
                return Err("git gave a record that is no commit and no status".to_string());
            }
        }
        for (c, paths) in commits.iter_mut().zip(found) {
            c.paths = paths;
        }
        Ok(())
    }

    /// The paths that a commit changed, from one process for the commit. The
    /// batched walk must give the same result (test oracle).
    #[cfg(test)]
    fn commit_paths_each(&self, c: &BranchCommit) -> Result<Vec<(String, PathChange)>, String> {
        // The `--` ends the revisions. The form of 0.7.0 part A had none, so
        // a file named like a commit id of the branch stopped the walk.
        let mut args: Vec<&str> = vec!["diff-tree", "-r", "-z", "--no-commit-id"];
        match c.parents.as_slice() {
            [] => args.extend(["--root", "--name-status", "--no-renames", &c.commit]),
            [parent] => args.extend(["--name-status", "--no-renames", parent, &c.commit]),
            _ => args.extend(["-c", "--name-only", &c.commit]),
        }
        args.push("--");
        let out = self.run_offline(&args)?;
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        let mut paths = Vec::new();
        if c.parents.len() > 1 {
            for path in recs {
                paths.push((
                    String::from_utf8_lossy(path).into_owned(),
                    PathChange::Modified,
                ));
            }
            return Ok(paths);
        }
        while let (Some(status), Some(path)) = (recs.next(), recs.next()) {
            let kind = match status.first() {
                Some(b'A') => PathChange::Added,
                Some(b'D') => PathChange::Deleted,
                _ => PathChange::Modified,
            };
            paths.push((String::from_utf8_lossy(path).into_owned(), kind));
        }
        Ok(paths)
    }

    /// Returns, for each file under `specs` that a commit touched, the newest
    /// committer date (seconds) of such a commit. It reads the history of
    /// `rev` once. A spec is a path or a glob from the repository root. A
    /// folder covers all files under it. A touch is an addition, a change, a
    /// deletion or a rename of the file. Without a spec it returns an empty
    /// map and runs no git command.
    ///
    /// The maximum of the dates is the answer, not the first line: the
    /// committer dates of a history are not always in order. In a shallow
    /// clone, the boundary commit shows its files as added, so each date is
    /// the same as in the full repository or later (see the top of this file).
    pub fn last_touch(&self, rev: &str, specs: &[String]) -> Result<HashMap<String, i64>, String> {
        let mut touched = HashMap::new();
        let pathspecs = glob_pathspecs(specs);
        if pathspecs.is_empty() {
            return Ok(touched);
        }
        let mut args = vec![
            "-c".to_string(),
            "log.showRoot=true".to_string(),
            "log".to_string(),
            "--no-renames".to_string(),
            "--format=%x01%ct".to_string(),
            "--name-only".to_string(),
            "-z".to_string(),
            rev.to_string(),
            "--".to_string(),
        ];
        args.extend(pathspecs);
        let out = self.run(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
        // The output is `\x01<seconds> NUL LF <path> NUL <path> NUL ...` for
        // each commit.
        let mut when: Option<i64> = None;
        let mut first = false;
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            if let Some(ts) = rec.strip_prefix('\x01') {
                when = ts.trim().parse().ok();
                first = true;
                continue;
            }
            let path = if first {
                rec.strip_prefix('\n').unwrap_or(&rec)
            } else {
                &rec
            };
            first = false;
            if let Some(t) = when
                && !path.is_empty()
            {
                let e = touched.entry(path.to_string()).or_insert(t);
                *e = (*e).max(t);
            }
        }
        Ok(touched)
    }

    /// Returns the committer date of the oldest commit in the history of
    /// `rev` that added each of the literal paths. The order is the order of
    /// the ancestry, not of the dates (`--topo-order`). A path that no commit
    /// added is absent from the map. A path can be added again after a
    /// deletion, and then the oldest add counts. A shallow clone shows the
    /// files of its boundary commit as added.
    pub fn first_added(&self, rev: &str, paths: &[String]) -> Result<HashMap<String, i64>, String> {
        let mut added: HashMap<String, i64> = HashMap::new();
        for group in paths.chunks(500) {
            let specs: Vec<String> = group.iter().map(|p| format!(":(literal){p}")).collect();
            let mut args = vec![
                "-c",
                "log.showRoot=true",
                "log",
                "--topo-order",
                "--no-renames",
                "--diff-filter=A",
                "--format=%x01%ct",
                "--name-only",
                "-z",
                rev,
                "--",
            ];
            args.extend(specs.iter().map(String::as_str));
            let out = self.run(&args)?;
            // The output is `\x01<seconds> NUL LF <path> NUL <path> NUL ...`
            // for each commit, the newest first. A later line is an older
            // commit, so it overwrites the earlier one.
            let mut when: Option<i64> = None;
            let mut first = false;
            for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
                let rec = String::from_utf8_lossy(rec);
                if let Some(ts) = rec.strip_prefix('\x01') {
                    when = ts.trim().parse().ok();
                    first = true;
                    continue;
                }
                let path = if first {
                    rec.strip_prefix('\n').unwrap_or(&rec)
                } else {
                    &rec
                };
                first = false;
                if let Some(t) = when
                    && !path.is_empty()
                {
                    added.insert(path.to_string(), t);
                }
            }
        }
        Ok(added)
    }

    /// Returns the newest commit of `HEAD` that added the literal path, or
    /// `None` if no commit did (a path that is not committed). A shallow clone
    /// gives `None`: its boundary commit shows each file as added, and the
    /// commit is not the true add. The lookup is one `git log`.
    pub fn added_by(&self, path: &str) -> Result<Option<String>, String> {
        if self.is_shallow() {
            return Ok(None);
        }
        let spec = format!(":(literal){path}");
        let out = self.run(&[
            "-c",
            "log.showRoot=true",
            "log",
            "-1",
            "--no-renames",
            "--diff-filter=A",
            "--format=%H",
            "HEAD",
            "--",
            &spec,
        ])?;
        let id = String::from_utf8_lossy(&out).trim().to_string();
        Ok((!id.is_empty()).then_some(id))
    }

    /// Returns the commits (id and committer date) that touched a path, the
    /// newest first. A touch is an addition, a change or a deletion of the
    /// file. The path is a literal path from the repository root.
    pub fn file_log(&self, rev: &str, path: &str) -> Result<Vec<(String, i64)>, String> {
        let spec = format!(":(literal){path}");
        let out = self.run(&[
            "-c",
            "log.showRoot=true",
            "log",
            "--no-renames",
            "--format=%H %ct",
            rev,
            "--",
            &spec,
        ])?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|l| {
                let (id, t) = l.split_once(' ')?;
                Some((id.to_string(), t.parse().ok()?))
            })
            .collect())
    }

    /// Returns the folders (without `/intent.md`) under `dir` whose
    /// `intent.md` a commit of `<base>..<head>` adds, and that are in neither
    /// the tree of `base` nor the tree of `head`. Such a change was added and
    /// finished (or removed) in the branch. `dir` is the folder of the
    /// `intent` layer, from the repository root, with no final `/`. The
    /// result is sorted.
    pub fn added_intents(&self, base: &str, head: &str, dir: &str) -> Result<Vec<String>, String> {
        let range = format!("{base}..{head}");
        let spec = intent_glob(dir);
        let out = self.run(&[
            "log",
            "--no-renames",
            "--diff-filter=A",
            "--name-only",
            "--format=",
            "-z",
            &range,
            "--",
            &spec,
        ])?;
        let mut folders: Vec<String> = String::from_utf8_lossy(&out)
            .split('\0')
            .map(|p| p.trim_start_matches('\n'))
            .filter_map(|p| p.strip_suffix("/intent.md"))
            .map(str::to_string)
            .collect();
        folders.sort();
        folders.dedup();
        if folders.is_empty() {
            return Ok(folders);
        }
        let mut present: HashSet<String> = HashSet::new();
        for rev in [base, head] {
            present.extend(self.files_under(rev, dir)?);
        }
        folders.retain(|f| !present.contains(&format!("{f}/intent.md")));
        Ok(folders)
    }

    /// Lists the files under `dir` in the tree of a commit.
    fn files_under(&self, rev: &str, dir: &str) -> Result<Vec<String>, String> {
        let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", rev];
        if !dir.is_empty() {
            args.push("--");
            args.push(dir);
        }
        let out = self.run(&args)?;
        let mut v = Vec::new();
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            if let Some((_, path)) = rec.split_once('\t') {
                v.push(path.to_string());
            }
        }
        Ok(v)
    }

    /// Returns the commits of the history of `HEAD` that deleted an
    /// `intent.md` under `dir` (the folder of the `intent` layer, from the
    /// repository root, with no final `/`), the newest first. With `since`,
    /// only the commits of `<since>..HEAD`. A move inside `dir` is not a
    /// deletion. A move out of `dir` reads as one (see `moved_out`). The
    /// command `git log` follows every parent and shows no merge commit: a squash, a merge
    /// commit and a rebase merge each give one entry for a deleted file. An
    /// entry is the non-merge commit that deleted the file. Its parent gives
    /// the status of the intent before the deletion.
    pub fn deleted_intents(
        &self,
        dir: &str,
        since: Option<&str>,
    ) -> Result<Vec<DeletedIntent>, String> {
        self.walk_deleted(dir, since, false)
    }

    /// Like `deleted_intents`, with the first-parent line of `HEAD` only. A
    /// merge commit shows its diff against its first parent, so the entry of a
    /// merge commit or of a squash commit is the commit that reached the
    /// default branch. A rebase merge has one entry, the commit that deleted
    /// the file. The `parent` of an entry is its first parent.
    pub fn deleted_intents_first_parent(&self, dir: &str) -> Result<Vec<DeletedIntent>, String> {
        self.walk_deleted(dir, None, true)
    }

    fn walk_deleted(
        &self,
        dir: &str,
        since: Option<&str>,
        first_parent: bool,
    ) -> Result<Vec<DeletedIntent>, String> {
        let range = since.map_or("HEAD".to_string(), |s| format!("{s}..HEAD"));
        let spec = intent_glob(dir);
        let mut args = vec![
            "log",
            "-M",
            "--diff-filter=D",
            "--name-only",
            "--format=%x01%H %P|%ct",
            "-z",
        ];
        if first_parent {
            args.extend(["-m", "--first-parent"]);
        }
        args.extend([range.as_str(), "--", spec.as_str()]);
        let out = self.run(&args)?;
        let mut found = Vec::new();
        let mut head: Option<(String, Vec<String>, i64)> = None;
        let mut first = false;
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            if let Some(h) = rec.strip_prefix('\x01') {
                head = parse_commit_header(h);
                first = true;
                continue;
            }
            let path = if first {
                rec.strip_prefix('\n').unwrap_or(&rec)
            } else {
                &rec
            };
            first = false;
            let Some((commit, parents, time)) = &head else {
                continue;
            };
            // A merge commit has no diff here, and the check keeps a
            // guard against a git that shows one. The first-parent walk
            // wants the merge commits.
            if (!first_parent && parents.len() > 1) || !path.ends_with("/intent.md") {
                continue;
            }
            found.push(DeletedIntent {
                commit: commit.clone(),
                parent: parents.first().cloned(),
                time: *time,
                path: path.to_string(),
            });
        }
        Ok(found)
    }

    /// Tells for each entry of a walk of deleted intents if the commit moved
    /// the file instead. `dir` is the folder of the `intent` layer, as for
    /// `deleted_intents`. The result has one item for each entry, in order.
    ///
    /// An entry is a move out only if all 3 conditions are true (design 0.7.0,
    /// D31):
    ///
    /// 1. The deleted file is the source of a rename in the diff of the
    ///    commit against its (first) parent. The diff has no pathspec, so the
    ///    new path can be anywhere.
    /// 2. The rename is `R100`: the new file is the same text. Else the front
    ///    matter `aliases` of the new file, in the tree of the commit, holds
    ///    the old id of the intent. `spec-dna mv` and `upgrade` write it.
    ///    `finish` never does. Git can pair a deletion with a new file of the
    ///    same commit that copies much of its text, so condition 1 alone is not
    ///    enough (design 0.6.1, D10).
    /// 3. The same diff has no other record that deletes or renames a file of
    ///    the change folder (the folder of the `intent.md`). A commit of
    ///    `finish` removes the other files of the folder, and a `git mv` of the
    ///    whole folder renames them: in both cases the folder leaves the
    ///    layer, so the change ended.
    ///
    /// The code reads the records of the one `git diff-tree` that it runs for
    /// each entry. The old id is the path with no `.md`, from a root. A root
    /// is the folder `dir` or one of its parents, so the code tries each of
    /// them.
    pub fn moved_out(&self, dir: &str, entries: &[DeletedIntent]) -> Result<Vec<bool>, String> {
        let mut result = vec![false; entries.len()];
        let mut wanted: Vec<(usize, (String, String))> = Vec::new();
        for (i, e) in entries.iter().enumerate() {
            let Some(parent) = &e.parent else { continue };
            let out = self.run_offline(&[
                "diff-tree",
                "-r",
                "-M",
                "--name-status",
                "-z",
                parent,
                &e.commit,
            ])?;
            let records = diff_records(&out);
            let Some(rename) = records
                .iter()
                .find(|r| r.status.starts_with('R') && r.old == e.path)
            else {
                continue;
            };
            if folder_leaves(&records, &e.path) {
                continue;
            }
            let to = rename.new.clone().unwrap_or_default();
            if rename.status == "R100" {
                result[i] = true;
            } else {
                wanted.push((i, (e.commit.clone(), to)));
            }
        }
        let specs: Vec<(String, String)> = wanted.iter().map(|(_, s)| s.clone()).collect();
        let texts = self.read_at(&specs)?;
        for ((i, _), text) in wanted.iter().zip(texts) {
            let Some(text) = text else { continue };
            let ids = old_ids(&entries[*i].path, dir);
            result[*i] = front_matter_aliases(&String::from_utf8_lossy(&text))
                .iter()
                .any(|a| ids.contains(a));
        }
        Ok(result)
    }

    /// Returns each touch of an `intent.md`, `spec.md` or `plan.md` that is
    /// one level under `dir` (the folder of the `intent` layer, from the
    /// repository root, with no final `/`), in the history of `HEAD`. The
    /// order is the topological order, the newest first: a parent never comes
    /// before its child. It is not the order of the committer dates, which a
    /// history does not keep in order. A commit that touches 2 files gives 2
    /// entries, one after the other. A merge commit gives no entry. A rename of the
    /// folder inside `dir` is a touch of the new path, with the old path in
    /// `from`. A move out of `dir` reads as a deletion (see `moved_out`).
    pub fn artifact_touches(&self, dir: &str) -> Result<Vec<ArtifactTouch>, String> {
        let specs: Vec<String> = ["intent.md", "spec.md", "plan.md"]
            .iter()
            .map(|n| artifact_glob(dir, n))
            .collect();
        let mut args = vec![
            "-c",
            "log.showRoot=true",
            "log",
            "--topo-order",
            "-M",
            "--name-status",
            "--format=%x01%H %P|%ct",
            "-z",
            "HEAD",
            "--",
        ];
        args.extend(specs.iter().map(String::as_str));
        let out = self.run_offline(&args)?;
        let mut found = Vec::new();
        let mut head: Option<(String, Vec<String>, i64)> = None;
        // After a header come the records of a status and one path, or of a
        // status of a rename and 2 paths. The first record after a header
        // starts with a line feed.
        let mut status: Option<char> = None;
        let mut old: Option<String> = None;
        for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            if let Some(h) = rec.strip_prefix('\x01') {
                head = parse_commit_header(h);
                status = None;
                old = None;
                continue;
            }
            let rec = rec.strip_prefix('\n').unwrap_or(&rec);
            let Some(st) = status else {
                status = rec.chars().next();
                continue;
            };
            if st == 'R' && old.is_none() {
                old = Some(rec.to_string());
                continue;
            }
            status = None;
            let Some((commit, parents, time)) = &head else {
                continue;
            };
            let kind = match st {
                'A' => PathChange::Added,
                'D' => PathChange::Deleted,
                _ => PathChange::Modified,
            };
            found.push(ArtifactTouch {
                commit: commit.clone(),
                parents: parents.clone(),
                time: *time,
                path: rec.to_string(),
                kind,
                from: old.take(),
            });
        }
        Ok(found)
    }

    /// Returns, for each commit, the paths that it changes against its first
    /// parent. A merge commit shows the whole diff of its branch. A commit
    /// that git does not have is an error. It runs one git command.
    pub fn first_parent_paths(
        &self,
        commits: &[String],
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut out = HashMap::new();
        if commits.is_empty() {
            return Ok(out);
        }
        let input: String = commits.iter().map(|c| format!("{c}\n")).collect();
        let text = self.run_with_input(
            &[
                "-c",
                "log.showRoot=true",
                "log",
                "--no-walk=unsorted",
                "--stdin",
                "-m",
                "--first-parent",
                "--no-renames",
                "--name-only",
                "--format=%x01%H",
                "-z",
            ],
            input,
        )?;
        let mut cur: Option<String> = None;
        let mut first = false;
        for rec in text.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let rec = String::from_utf8_lossy(rec);
            if let Some(h) = rec.strip_prefix('\x01') {
                let id = h.trim().to_string();
                out.entry(id.clone()).or_insert_with(Vec::new);
                cur = Some(id);
                first = true;
                continue;
            }
            let path = if first {
                rec.strip_prefix('\n').unwrap_or(&rec)
            } else {
                &rec
            };
            first = false;
            if let Some(c) = &cur
                && !path.is_empty()
            {
                out.entry(c.clone()).or_default().push(path.to_string());
            }
        }
        Ok(out)
    }

    /// Reads files of commits with one `git cat-file --batch` process. Each
    /// spec is a commit and a path from the repository root. The result has one
    /// item for each spec, in the same order: `None` if the commit has no such
    /// file, or the path is not a file.
    pub fn read_at(&self, specs: &[(String, String)]) -> Result<Vec<Option<Vec<u8>>>, String> {
        let mut out: Vec<Option<Vec<u8>>> = Vec::with_capacity(specs.len());
        if specs.is_empty() {
            return Ok(out);
        }
        // In a partial clone, an absent blob is `None`: no command here
        // fetches it.
        let mut child = self
            .cmd(&["cat-file", "--batch"])
            .env("GIT_NO_LAZY_FETCH", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot run git cat-file: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        // A name with a line feed cannot go on a line. It reads as absent.
        let input: String = specs
            .iter()
            .map(|(rev, path)| {
                if path.contains('\n') {
                    "missing:\n".to_string()
                } else {
                    format!("{rev}:{path}\n")
                }
            })
            .collect();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
        let mut r = BufReader::new(child.stdout.take().unwrap());
        let mut header = String::new();
        for _ in specs {
            header.clear();
            if r.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
                out.push(None);
                continue;
            }
            let line = header.trim_end();
            let parts: Vec<&str> = line.split(' ').collect();
            let body = if line.ends_with(" missing") || parts.len() != 3 {
                None
            } else {
                Some(
                    parts[2]
                        .parse::<usize>()
                        .map_err(|_| "bad cat-file header".to_string())?,
                )
            };
            let Some(size) = body else {
                out.push(None);
                continue;
            };
            let mut buf = vec![0u8; size + 1];
            r.read_exact(&mut buf).map_err(|e| e.to_string())?;
            buf.pop();
            out.push((parts[1] == "blob").then_some(buf));
        }
        let _ = writer.join();
        let _ = child.wait();
        Ok(out)
    }

    /// Reads a file of the head tree: the git index for a `staged` `Git`, else
    /// the file of the working tree. `None` if the file is absent or is not a
    /// file. The path is from the repository root.
    pub fn head_file(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        if self.staged {
            // The spec `:<path>` names the entry of the index (stage 0).
            let got = self.read_at(&[(String::new(), path.to_string())])?;
            return Ok(got.into_iter().next().flatten());
        }
        Ok(std::fs::read(self.root.join(path)).ok())
    }

    /// True if the head entry of `path` is a symbolic link: the file of the
    /// working tree, or the entry of the git index (mode 120000) for a
    /// `staged` `Git`. The path is from the repository root. A path that is
    /// absent gives `false`. The 2 modes give one answer for a link, where
    /// `head_file` gives the target in the working tree and the link text in
    /// the index (ADR 0019).
    pub fn head_is_symlink(&self, path: &str) -> bool {
        if self.staged {
            return self
                .run(&["ls-files", "-s", "-z", "--", path])
                .is_ok_and(|out| out.starts_with(b"120000 "));
        }
        std::fs::symlink_metadata(self.root.join(path)).is_ok_and(|m| m.file_type().is_symlink())
    }

    /// Runs a git command with `input` on its standard input.
    fn run_with_input(&self, args: &[&str], input: String) -> Result<Vec<u8>, String> {
        let mut child = self
            .cmd(args)
            .env("GIT_NO_LAZY_FETCH", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run git: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
        let out = child
            .wait_with_output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        let _ = writer.join();
        if !out.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(out.stdout)
    }

    /// Reads the messages of commits with one `git cat-file --batch` process.
    /// The result has one item for each id, in order. A commit that git cannot
    /// read gives `None`. The message is the text after the first empty line of
    /// the commit object, so a signature or a merge tag in the header does not
    /// matter.
    pub fn commit_messages(&self, ids: &[String]) -> Result<Vec<Option<String>>, String> {
        let objects = self.read_blobs(ids)?;
        Ok(ids
            .iter()
            .map(|id| {
                let raw = objects.get(id)?;
                let at = raw.windows(2).position(|w| w == b"\n\n")?;
                Some(String::from_utf8_lossy(&raw[at + 2..]).into_owned())
            })
            .collect())
    }

    /// Reads blobs with one `git cat-file --batch` process.
    pub fn read_blobs(&self, ids: &[String]) -> Result<HashMap<String, Vec<u8>>, String> {
        let mut out = HashMap::new();
        if ids.is_empty() {
            return Ok(out);
        }
        let mut child = self
            .cmd(&["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot run git cat-file: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let input: String = ids.iter().map(|i| format!("{i}\n")).collect();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
        let mut r = BufReader::new(child.stdout.take().unwrap());
        let mut header = String::new();
        for _ in ids {
            header.clear();
            if r.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
                break;
            }
            let parts: Vec<&str> = header.trim_end().split(' ').collect();
            if parts.len() != 3 {
                continue; // "<id> missing"
            }
            let size: usize = parts[2]
                .parse()
                .map_err(|_| "bad cat-file header".to_string())?;
            let mut buf = vec![0u8; size + 1];
            r.read_exact(&mut buf).map_err(|e| e.to_string())?;
            buf.pop();
            out.insert(parts[0].to_string(), buf);
        }
        let _ = writer.join();
        let _ = child.wait();
        Ok(out)
    }

    /// Returns the tracked files of the whole tree that differ from the git
    /// index in the working tree: (modified, deleted). `git diff-files` does
    /// not refresh the index, so it never writes it. A file with a new
    /// modification time but the same content can be in `modified`.
    pub fn diff_files(&self) -> Result<(HashSet<String>, HashSet<String>), String> {
        let out = self.run(&[
            "diff-files",
            "--name-status",
            "-z",
            "--no-renames",
            "--ignore-submodules=all",
        ])?;
        let (mut modified, mut deleted) = (HashSet::new(), HashSet::new());
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let (Some(status), Some(path)) = (recs.next(), recs.next()) {
            let path = String::from_utf8_lossy(path).into_owned();
            if status.first() == Some(&b'D') {
                deleted.insert(path);
            } else {
                modified.insert(path);
            }
        }
        Ok((modified, deleted))
    }

    /// Reads the blobs of at most `max` bytes. A larger blob is not read. In a
    /// partial clone, git does not fetch an absent blob, so it is not in the
    /// result.
    pub fn read_small_blobs(
        &self,
        ids: &[String],
        max: u64,
    ) -> Result<HashMap<String, Vec<u8>>, String> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let input: String = ids.iter().map(|i| format!("{i}\n")).collect();
        let mut child = self
            .cmd(&["cat-file", "--batch-check"])
            .env("GIT_NO_LAZY_FETCH", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot run git cat-file: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
        let mut text = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        let _ = writer.join();
        let _ = child.wait();
        let small: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let p: Vec<&str> = l.split(' ').collect();
                (p.len() == 3 && p[1] == "blob" && p[2].parse::<u64>().is_ok_and(|n| n <= max))
                    .then(|| p[0].to_string())
            })
            .collect();
        self.read_blobs(&small)
    }

    /// Returns the words that occur as whole words in the working tree, outside
    /// of Markdown files. It searches tracked files, and with `untracked` also
    /// the untracked files that git does not ignore. Binary files do not count.
    pub fn grep_words(&self, words: &[String], untracked: bool) -> Result<HashSet<String>, String> {
        let mut found = HashSet::new();
        if words.is_empty() {
            return Ok(found);
        }
        // The flags override the grep settings of the user's git config.
        let mut args: Vec<&str> = vec![
            "grep",
            "-I",
            "-w",
            "-F",
            "-o",
            "-h",
            "--no-color",
            "--no-line-number",
            "--no-column",
        ];
        if self.staged {
            args.push("--cached");
        } else if untracked {
            args.push("--untracked");
        }
        for w in words {
            args.push("-e");
            args.push(w);
        }
        args.extend([
            "--",
            ".",
            ":(exclude,glob)**/*.md",
            ":(exclude,glob)**/*.markdown",
        ]);
        let out = self
            .cmd(&args)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        // `git grep` exits with 1 when nothing matches.
        match out.status.code() {
            Some(0) | Some(1) => {}
            _ => {
                return Err(format!(
                    "git grep failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            found.insert(line.to_string());
        }
        Ok(found)
    }

    /// Returns each line that contains `text`, as (path, line number, line).
    /// Without `rev`, it searches the working tree: tracked files, and with
    /// `untracked` also the untracked files that git does not ignore. With
    /// `rev`, it searches that commit. Binary files do not count.
    /// `globs` limits the search to the paths that match one of them (git
    /// `glob` pathspecs: `*` stays in one segment, `**` crosses segments).
    pub fn grep_lines(
        &self,
        text: &str,
        rev: Option<&str>,
        untracked: bool,
        globs: &[String],
    ) -> Result<Vec<(String, u32, String)>, String> {
        let mut args: Vec<&str> = vec![
            "grep",
            "-I",
            "-F",
            "-z",
            "-n",
            "--no-color",
            "--no-column",
            "-e",
            text,
        ];
        if self.staged && rev.is_none() {
            args.push("--cached");
        } else if untracked && rev.is_none() {
            args.push("--untracked");
        }
        if let Some(r) = rev {
            args.push(r);
        }
        args.push("--");
        // A glob that git does not accept as a pathspec (with `..`, or empty
        // after the slashes) turns the limit off. The caller filters the
        // paths.
        let specs: Vec<String> = globs
            .iter()
            .map(|g| g.trim_matches('/'))
            .map(|g| format!(":(glob){g}"))
            .collect();
        let limit = globs.iter().all(|g| {
            let g = g.trim_matches('/');
            !g.is_empty() && !g.split('/').any(|s| s == ".." || s == ".")
        });
        if limit {
            args.extend(specs.iter().map(String::as_str));
        }
        let out = self
            .cmd(&args)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        // `git grep` exits with 1 when nothing matches.
        match out.status.code() {
            Some(0) | Some(1) => {}
            _ => {
                return Err(format!(
                    "git grep failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
        }
        // Each record is `path NUL line NUL text LF`. With a revision, the
        // path starts with `<rev>:`.
        let prefix = rev.map(|r| format!("{r}:")).unwrap_or_default();
        let mut found = Vec::new();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for rec in stdout.split_terminator('\n') {
            let mut parts = rec.splitn(3, '\0');
            let (Some(path), Some(line), Some(body)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let path = path.strip_prefix(prefix.as_str()).unwrap_or(path);
            if let Ok(n) = line.parse() {
                found.push((path.to_string(), n, body.to_string()));
            }
        }
        Ok(found)
    }

    /// Returns the default branch: the branch of `origin/HEAD`, or a local
    /// `main` or `master`, or the current branch, or `main`. It reads local
    /// refs only, without the network.
    pub fn default_branch(&self) -> String {
        let out = |args: &[&str]| {
            self.run(args)
                .ok()
                .map(|o| String::from_utf8_lossy(&o).trim().to_string())
                .filter(|s| !s.is_empty())
        };
        if let Some(b) = out(&[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) {
            return b.strip_prefix("origin/").unwrap_or(&b).to_string();
        }
        for b in ["main", "master"] {
            if out(&[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{b}"),
            ])
            .is_some()
            {
                return b.to_string();
            }
        }
        out(&["symbolic-ref", "--quiet", "--short", "HEAD"]).unwrap_or_else(|| "main".to_string())
    }

    /// The paths that make the working tree not clean (D151): a staged change,
    /// an unstaged change, and an untracked file that git does not ignore.
    /// An untracked folder is one path. The list is in the order of git.
    pub fn unclean_paths(&self) -> Result<Vec<String>, String> {
        let out = self.run(&["status", "--porcelain", "-z", "--untracked-files=normal"])?;
        let mut paths = Vec::new();
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let Some(rec) = recs.next() {
            let rec = String::from_utf8_lossy(rec);
            let (xy, path) = rec.split_at(rec.len().min(2));
            paths.push(path.trim_start().to_string());
            // A rename or a copy is followed by its source path.
            if xy.contains(['R', 'C']) {
                recs.next();
            }
        }
        Ok(paths)
    }

    pub fn mv(&self, from: &str, to: &str) -> Result<(), String> {
        self.run(&["mv", "--", from, to]).map(|_| ())
    }

    /// Stages the paths with `git add -A`: a new file, an edit and a deletion.
    /// Each path is a literal path of the repository (no glob). It writes the
    /// git index and it makes no commit.
    pub fn add_paths(&self, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        let specs: Vec<String> = paths.iter().map(|p| format!(":(literal){p}")).collect();
        let mut args = vec!["add", "-A", "--"];
        args.extend(specs.iter().map(String::as_str));
        self.run(&args).map(|_| ())
    }

    /// Returns true if an ignore rule of git matches the path. A tracked
    /// file is never ignored.
    pub fn is_ignored(&self, path: &str) -> bool {
        self.cmd(&["check-ignore", "-q", "--", path])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    pub fn is_tracked(&self, path: &str) -> bool {
        self.run(&["ls-files", "--error-unmatch", "--", path])
            .is_ok()
    }
}

/// One record of `git diff-tree -r -M --name-status -z`.
struct DiffRecord {
    /// The status, such as `D`, `A`, `M` or `R100`.
    status: String,
    /// The path, or the old path of a rename.
    old: String,
    /// The new path of a rename or a copy.
    new: Option<String>,
}

/// Reads the records of the output of `git diff-tree -r -M --name-status -z`.
fn diff_records(out: &[u8]) -> Vec<DiffRecord> {
    let mut recs = out
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .map(String::from_utf8_lossy);
    let mut found = Vec::new();
    while let Some(status) = recs.next() {
        let Some(old) = recs.next() else { break };
        let new = if status.starts_with(['R', 'C']) {
            let Some(n) = recs.next() else { break };
            Some(n.into_owned())
        } else {
            None
        };
        found.push(DiffRecord {
            status: status.into_owned(),
            old: old.into_owned(),
            new,
        });
    }
    found
}

/// True if the diff deletes or renames a file of the folder of `intent`
/// (the path of an `intent.md`) other than `intent` itself.
fn folder_leaves(records: &[DiffRecord], intent: &str) -> bool {
    let folder = intent.rsplit_once('/').map_or("", |(d, _)| d);
    let prefix = format!("{folder}/");
    records.iter().any(|r| {
        r.old != intent
            && (r.status.starts_with('D') || r.status.starts_with('R'))
            && (folder.is_empty() || r.old.starts_with(&prefix))
    })
}

/// The ids that the file `path` had before a move: the path with no `.md`,
/// from the repository root and from each folder that holds `dir`.
fn old_ids(path: &str, dir: &str) -> Vec<String> {
    let stem = path.strip_suffix(".md").unwrap_or(path);
    let mut ids = vec![stem.to_string()];
    for (i, _) in dir.trim_matches('/').match_indices('/') {
        if let Some(rest) = stem.get(i + 1..) {
            ids.push(rest.to_string());
        }
    }
    ids
}

/// Reads the list `aliases` of the front matter of a document. It reads the
/// block list and the inline list of the YAML subset of the front matter. A
/// text with no front matter gives no alias.
fn front_matter_aliases(text: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Vec::new();
    }
    let unquote = |v: &str| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
    let mut found = Vec::new();
    let mut in_list = false;
    for line in lines {
        let line = line.trim_end();
        if line == "---" {
            break;
        }
        if let Some(rest) = line.strip_prefix("aliases:") {
            let rest = rest.trim();
            if let Some(inner) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                found.extend(inner.split(',').map(unquote).filter(|a| !a.is_empty()));
                in_list = false;
            } else {
                in_list = rest.is_empty();
            }
        } else if in_list {
            match line.trim_start().strip_prefix("- ") {
                Some(item) => found.push(unquote(item)),
                None if line.starts_with([' ', '\t']) || line.is_empty() => {}
                None => in_list = false,
            }
        }
    }
    found
}

/// Reads `<id> <parents>|<seconds>` of a commit header.
fn parse_commit_header(h: &str) -> Option<(String, Vec<String>, i64)> {
    let (ids, time) = h.trim().rsplit_once('|')?;
    let mut parts = ids.split(' ').filter(|p| !p.is_empty());
    let commit = parts.next()?.to_string();
    let parents = parts.map(str::to_string).collect();
    Some((commit, parents, time.trim().parse().ok()?))
}

/// Escapes the characters that a git `glob` pathspec reads as magic.
fn glob_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '*' | '?' | '[' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The pathspec of the `intent.md` files one level under `dir`.
fn intent_glob(dir: &str) -> String {
    let dir = dir.trim_matches('/');
    if dir.is_empty() {
        ":(glob)*/intent.md".to_string()
    } else {
        format!(":(glob){}/*/intent.md", glob_escape(dir))
    }
}

/// The pathspec of the files with the name `name` one level under `dir`.
fn artifact_glob(dir: &str, name: &str) -> String {
    let dir = dir.trim_matches('/');
    if dir.is_empty() {
        format!(":(glob)*/{name}")
    } else {
        format!(":(glob){}/*/{name}", glob_escape(dir))
    }
}

/// Turns paths and globs of the repository root into git pathspecs. A value
/// that git does not accept (empty, or with a `.` or `..` segment) is left out.
pub fn glob_pathspecs(specs: &[String]) -> Vec<String> {
    specs
        .iter()
        .map(|g| g.trim_matches('/'))
        .filter(|g| !g.is_empty() && !g.split('/').any(|s| s == ".." || s == "."))
        // `crate::glob`, which reads the globs of the configuration, has only
        // `*`, `**` and `?`. A path with none of them is a literal path (a
        // folder covers the files under it). In a git `glob` pathspec, `[` and
        // `\` are also magic, so they are escaped.
        .map(|g| {
            if !g.contains(['*', '?']) {
                return format!(":(literal){g}");
            }
            let g: String = g
                .chars()
                .flat_map(|c| match c {
                    '[' | '\\' => vec!['\\', c],
                    _ => vec![c],
                })
                .collect();
            format!(":(glob){g}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A throw-away repository on branch `main`. The tests of this module
    /// have no helper of `tests/common/mod.rs`, so they use `std` only.
    struct Tmp {
        dir: PathBuf,
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "commit.gpgsign=false", "-c", "user.name=Test"])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_DATE", "2024-01-01T00:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2024-01-01T00:00:00+00:00")
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    impl Tmp {
        fn new() -> Tmp {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir =
                std::env::temp_dir().join(format!("spec-dna-gitmod-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            git_in(&dir, &["init", "-q", "-b", "main"]);
            Tmp { dir }
        }

        fn git(&self, args: &[&str]) -> String {
            git_in(&self.dir, args)
        }

        fn write(&self, path: &str, text: &str) {
            let p = self.dir.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }

        /// Commits all files with the given author e-mail. The date is the
        /// same for each commit, so only the ancestry gives the order.
        fn commit(&self, email: &str, msg: &str) -> String {
            self.git(&["add", "-A"]);
            self.git(&[
                "-c",
                &format!("user.email={email}"),
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                msg,
            ]);
            self.git(&["rev-parse", "HEAD"])
        }

        fn walk(&self, base: &str) -> BranchWalk {
            Git::new(&self.dir).branch_walk(base, "HEAD").unwrap()
        }
    }

    /// Adds a file with a name that is not UTF-8 to the index (unix only).
    #[cfg(unix)]
    fn add_odd_name(t: &Tmp) {
        use std::ffi::{OsStr, OsString};
        use std::os::unix::ffi::OsStrExt;
        let out = Command::new("git")
            .arg("-C")
            .arg(&t.dir)
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let oid = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let mut spec = OsString::from(format!("100644,{oid},"));
        spec.push(OsStr::from_bytes(b"odd\xffname.txt"));
        let status = Command::new("git")
            .arg("-C")
            .arg(&t.dir)
            .args(["update-index", "--add", "--cacheinfo"])
            .arg(spec)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn the_batched_walk_gives_the_paths_of_each_commit() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        // A branch with its own root commit, so that the range lists it.
        t.git(&["checkout", "-q", "--orphan", "work"]);
        t.git(&["rm", "-q", "-r", "-f", "."]);
        // Names that look like the records of the output of git: an id, a
        // status letter, and a name with a space and a line feed.
        t.write(&base, "x\n");
        t.write("M", "x\n");
        t.write("MM", "x\n");
        t.write("sp ace.txt", "x\n");
        t.write("line\nfeed.txt", "x\n");
        #[cfg(unix)]
        add_odd_name(&t);
        let root = t.commit("root@example.test", "root");
        // A modify, an add, a delete and a rename.
        t.write("M", "y\n");
        t.write("new.txt", "n\n");
        t.git(&["rm", "-q", "MM"]);
        t.git(&["mv", "sp ace.txt", "moved name.txt"]);
        let edit = t.commit("edit@example.test", "edit");
        // An empty commit has no path.
        let empty = t.commit("empty@example.test", "empty");
        // A neutral merge of a branch that adds one file, and a merge that adds
        // a file of its own.
        t.git(&["checkout", "-q", "-b", "side", &base]);
        t.write("side.txt", "s\n");
        t.commit("side@example.test", "side");
        t.git(&["checkout", "-q", "work"]);
        t.git(&[
            "merge",
            "-q",
            "--no-ff",
            "--allow-unrelated-histories",
            "-m",
            "merge side",
            "side",
        ]);
        t.git(&["checkout", "-q", "-b", "side2", &edit]);
        t.write("side2.txt", "s\n");
        t.commit("side2@example.test", "side2");
        t.git(&["checkout", "-q", "work"]);
        t.git(&[
            "merge",
            "-q",
            "--no-ff",
            "--no-commit",
            "-m",
            "merge side2",
            "side2",
        ]);
        t.write("merge-work.txt", "w\n");
        t.git(&["add", "-A"]);
        t.git(&["commit", "-q", "--no-edit"]);
        t.write("last.txt", "l\n");
        t.commit("last@example.test", "last");
        let g = Git::new(&t.dir);
        let w = g.branch_walk(&base, "HEAD").unwrap();
        assert!(w.commits.len() >= 8, "{}", w.commits.len());
        assert!(w.commits.iter().any(|c| c.commit == root));
        assert!(w.commits.iter().any(|c| c.commit == empty));
        assert!(w.commits.iter().any(|c| c.parents.len() > 1));
        // Each commit has the paths of the form with one process for each
        // commit, with the same kinds and the same order.
        for c in &w.commits {
            let old = g.commit_paths_each(c).unwrap();
            assert_eq!(c.paths, old, "commit {} ({})", c.commit, c.author_email);
        }
        let by = |id: &str| w.commits.iter().find(|c| c.commit == id).unwrap();
        assert!(by(&empty).paths.is_empty());
        let edit_paths: Vec<(&str, PathChange)> = by(&edit)
            .paths
            .iter()
            .map(|(p, k)| (p.as_str(), *k))
            .collect();
        assert!(edit_paths.contains(&("MM", PathChange::Deleted)));
        assert!(edit_paths.contains(&("new.txt", PathChange::Added)));
        assert!(edit_paths.contains(&("M", PathChange::Modified)));
        // The rename is a deletion and an addition.
        assert!(edit_paths.contains(&("sp ace.txt", PathChange::Deleted)));
        assert!(edit_paths.contains(&("moved name.txt", PathChange::Added)));
        // The names that look like a header are paths of the root commit.
        let root_paths: Vec<&str> = by(&root).paths.iter().map(|(p, _)| p.as_str()).collect();
        assert!(root_paths.contains(&base.as_str()));
        assert!(root_paths.contains(&"line\nfeed.txt"));
        assert!(by(&root).paths.iter().all(|(_, k)| *k == PathChange::Added));
        // The merge of its own work lists that file, with the kind `Modified`.
        let merge = w
            .commits
            .iter()
            .find(|c| c.paths.iter().any(|(p, _)| p == "merge-work.txt"))
            .unwrap();
        assert!(merge.parents.len() > 1);
        assert!(merge.paths.iter().all(|(_, k)| *k == PathChange::Modified));
    }

    #[test]
    fn the_walk_goes_from_the_oldest_with_email_parents_and_paths() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        t.write("b.txt", "b\n");
        t.write("a.txt", "a2\n");
        let c1 = t.commit("One@Example.test", "one");
        t.git(&["rm", "-q", "b.txt"]);
        let c2 = t.commit("two@example.test", "two");
        let c3 = t.commit("three@example.test", "empty");
        let w = t.walk(&base);
        let ids: Vec<&str> = w.commits.iter().map(|c| c.commit.as_str()).collect();
        assert_eq!(ids, [c1.as_str(), c2.as_str(), c3.as_str()]);
        // The e-mail has the case of the commit.
        assert_eq!(w.commits[0].author_email, "One@Example.test");
        assert_eq!(w.commits[0].parents, std::slice::from_ref(&base));
        assert_eq!(w.commits[1].parents, std::slice::from_ref(&c1));
        assert_eq!(
            w.commits[0].paths,
            [
                ("a.txt".to_string(), PathChange::Modified),
                ("b.txt".to_string(), PathChange::Added)
            ]
        );
        assert_eq!(
            w.commits[1].paths,
            [("b.txt".to_string(), PathChange::Deleted)]
        );
        // A commit with no change has no path, and it does not hide the next.
        assert!(w.commits[2].paths.is_empty());
        assert_eq!(w.merge_base, Some(base));
        assert!(!w.shallow);
    }

    #[test]
    fn the_walk_of_the_base_itself_is_empty() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        let w = t.walk(&base);
        assert!(w.commits.is_empty());
        assert_eq!(w.merge_base, Some(base));
    }

    #[test]
    fn a_root_commit_adds_its_files_and_has_no_parent() {
        // 2 histories that share no commit: the merge base is `None`, and the
        // root commit of the head shows all its files as added.
        let other = Tmp::new();
        other.write("o.txt", "o\n");
        other.commit("other@example.test", "other");
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        t.write("b.txt", "b\n");
        let root = t.commit("root@example.test", "root");
        t.git(&["fetch", "-q", other.dir.to_str().unwrap(), "HEAD:other"]);
        let w = t.walk("other");
        assert_eq!(w.merge_base, None);
        assert_eq!(w.commits.len(), 1);
        assert_eq!(w.commits[0].commit, root);
        assert!(w.commits[0].parents.is_empty());
        assert_eq!(
            w.commits[0].paths,
            [
                ("a.txt".to_string(), PathChange::Added),
                ("b.txt".to_string(), PathChange::Added)
            ]
        );
    }

    #[test]
    fn a_merge_commit_is_listed_with_its_parents_and_the_paths_that_it_changes_itself() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        t.git(&["checkout", "-q", "-b", "side"]);
        t.write("side.txt", "s\n");
        let side = t.commit("side@example.test", "side");
        t.git(&["checkout", "-q", "main"]);
        t.write("main.txt", "m\n");
        let main = t.commit("main@example.test", "main");
        t.git(&["checkout", "-q", "-b", "work", &base]);
        t.write("work.txt", "w\n");
        let work = t.commit("work@example.test", "work");
        t.git(&[
            "-c",
            "user.email=merger@example.test",
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge",
            "side",
        ]);
        let merge = t.git(&["rev-parse", "HEAD"]);
        let w = t.walk(&main);
        // The branch has `work`, `side` and the merge. A parent comes before
        // its child.
        let pos = |id: &str| w.commits.iter().position(|c| c.commit == id).unwrap();
        assert_eq!(w.commits.len(), 3);
        assert!(pos(&work) < pos(&merge) && pos(&side) < pos(&merge));
        assert_eq!(pos(&merge), 2);
        let m = &w.commits[2];
        assert_eq!(m.parents, [work, side]);
        assert_eq!(m.author_email, "merger@example.test");
        // A clean merge changes no path itself: each file of the result is in
        // a parent. The files of `side` are in the walk by their own commit.
        assert!(m.paths.is_empty(), "{:?}", m.paths);
    }

    #[test]
    fn a_merge_commit_lists_a_path_that_differs_from_each_parent() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        t.git(&["checkout", "-q", "-b", "side"]);
        t.write("side.txt", "s\n");
        t.commit("side@example.test", "side");
        t.git(&["checkout", "-q", "-b", "work", &base]);
        t.write("work.txt", "w\n");
        t.commit("work@example.test", "work");
        t.git(&["merge", "-q", "--no-commit", "--no-ff", "side"]);
        // The merge adds a file, and edits a file that both parents have.
        t.write("inside.txt", "new\n");
        t.write("a.txt", "a merged\n");
        t.git(&["add", "-A"]);
        t.git(&[
            "-c",
            "user.email=merger@example.test",
            "commit",
            "-q",
            "-m",
            "merge",
        ]);
        let w = t.walk(&base);
        let m = w.commits.last().unwrap();
        assert_eq!(m.parents.len(), 2);
        let mut paths: Vec<&str> = m.paths.iter().map(|(p, _)| p.as_str()).collect();
        paths.sort_unstable();
        assert_eq!(paths, ["a.txt", "inside.txt"]);
    }

    #[test]
    fn a_path_cannot_forge_a_commit_header() {
        // A file name can hold the bytes `\x01` and `\x02`, and a line feed.
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        t.write("b.txt", "b\n");
        let c1 = t.commit("real@example.test", "one");
        let forged = format!("\u{1}{c1} {base}\u{2}owner@example.test");
        for name in [forged.as_str(), "\u{1}only", "line\nfeed.txt", "\u{2}x"] {
            t.write(name, "x\n");
        }
        let c2 = t.commit("bot@example.test", "two");
        let w = t.walk(&base);
        let ids: Vec<&str> = w.commits.iter().map(|c| c.commit.as_str()).collect();
        assert_eq!(ids, [c1.as_str(), c2.as_str()]);
        assert_eq!(w.commits[0].author_email, "real@example.test");
        assert_eq!(w.commits[1].author_email, "bot@example.test");
        assert_eq!(w.commits[1].parents, std::slice::from_ref(&c1));
        let mut paths: Vec<&str> = w.commits[1].paths.iter().map(|(p, _)| p.as_str()).collect();
        paths.sort_unstable();
        let mut want = vec![forged.as_str(), "\u{1}only", "line\nfeed.txt", "\u{2}x"];
        want.sort_unstable();
        assert_eq!(paths, want);
        assert!(
            w.commits[1]
                .paths
                .iter()
                .all(|(_, k)| *k == PathChange::Added)
        );
        assert_eq!(
            w.commits[0].paths,
            [("b.txt".to_string(), PathChange::Added)]
        );
    }

    #[test]
    fn the_mailmap_does_not_change_the_author_email() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let base = t.commit("base@example.test", "base");
        t.write(
            ".mailmap",
            "Owner <owner@example.test> <stranger@example.test>\n",
        );
        t.commit("stranger@example.test", "map");
        let w = t.walk(&base);
        assert_eq!(w.commits[0].author_email, "stranger@example.test");
    }

    #[test]
    fn a_shallow_clone_reports_it_and_has_no_merge_base_below_the_boundary() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let first = t.commit("base@example.test", "base");
        t.write("b.txt", "b\n");
        t.commit("one@example.test", "one");
        t.write("c.txt", "c\n");
        t.commit("two@example.test", "two");
        t.git(&["checkout", "-q", "-b", "work", &first]);
        t.write("w.txt", "w\n");
        t.commit("work@example.test", "work 1");
        t.write("w2.txt", "w\n");
        t.commit("work@example.test", "work 2");
        // `--depth` needs a URL, not a path.
        let url = format!("file://{}", t.dir.display());
        let clone =
            std::env::temp_dir().join(format!("spec-dna-gitmod-{}-clone", std::process::id()));
        let _ = fs::remove_dir_all(&clone);
        let tmp = Tmp::new();
        git_in(
            &tmp.dir,
            &[
                "clone",
                "-q",
                "--depth",
                "1",
                "--branch",
                "main",
                &url,
                clone.to_str().unwrap(),
            ],
        );
        let _guard = Tmp { dir: clone.clone() };
        git_in(&clone, &["fetch", "-q", "--depth", "1", &url, "work:work"]);
        let g = Git::new(&clone);
        assert!(g.is_shallow());
        let w = g.branch_walk("main", "work").unwrap();
        // Both tips are boundary commits. They share no visible ancestor.
        assert!(w.shallow);
        assert_eq!(w.merge_base, None);
        // The boundary commit of the head has no parent in the listing.
        assert!(w.commits.iter().any(|c| c.parents.is_empty()));
    }

    #[cfg(unix)]
    #[test]
    fn head_is_symlink_gives_one_answer_in_both_modes() {
        let t = Tmp::new();
        t.write("real.txt", "a\n");
        std::os::unix::fs::symlink("real.txt", t.dir.join("link.txt")).unwrap();
        t.commit("a@example.test", "base");
        let g = Git::new(&t.dir);
        for g in [g.staged(), g] {
            assert!(g.head_is_symlink("link.txt"));
            assert!(!g.head_is_symlink("real.txt"));
            assert!(!g.head_is_symlink("absent.txt"));
        }
    }

    #[test]
    fn user_email_reads_the_config_and_trims_it() {
        let t = Tmp::new();
        t.git(&["config", "user.email", "  owner@example.test "]);
        let g = Git::new(&t.dir);
        assert_eq!(g.user_email().as_deref(), Some("owner@example.test"));
        // An empty value is no e-mail.
        t.git(&["config", "user.email", ""]);
        assert_eq!(g.user_email(), None);
    }

    #[test]
    fn is_ancestor_answers_for_a_commit_and_an_unknown_revision() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let a = t.commit("a@example.test", "a");
        let b = t.commit("b@example.test", "b");
        let g = Git::new(&t.dir);
        assert!(g.is_ancestor(&a, &b));
        assert!(g.is_ancestor(&a, &a));
        assert!(!g.is_ancestor(&b, &a));
        assert!(!g.is_ancestor("nope", &a));
    }

    #[test]
    fn commit_messages_come_from_one_process_in_the_order_of_the_ids() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        let one = t.commit("a@example.test", "one\n\nHours: 2\n\nlast paragraph");
        t.write("a.txt", "b\n");
        let two = t.commit("a@example.test", "two");
        let none = "0".repeat(40);
        let got = Git::new(&t.dir)
            .commit_messages(&[two.clone(), none, one.clone(), two])
            .unwrap();
        assert_eq!(got[0].as_deref(), Some("two\n"));
        assert_eq!(got[1], None);
        assert_eq!(
            got[2].as_deref(),
            Some("one\n\nHours: 2\n\nlast paragraph\n")
        );
        assert_eq!(got[3], got[0]);
        assert!(Git::new(&t.dir).commit_messages(&[]).unwrap().is_empty());
    }

    #[test]
    fn a_commit_message_follows_a_multi_line_header() {
        let t = Tmp::new();
        t.write("a.txt", "a\n");
        t.commit("a@example.test", "base");
        // A merge commit has 2 parents and a header of 4 lines.
        t.git(&["checkout", "-q", "-b", "side"]);
        t.write("b.txt", "b\n");
        t.commit("a@example.test", "side");
        t.git(&["checkout", "-q", "main"]);
        t.write("c.txt", "c\n");
        t.commit("a@example.test", "main");
        t.git(&[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "merge side\n\nHours: 1",
            "side",
        ]);
        let merge = t.git(&["rev-parse", "HEAD"]);
        let got = Git::new(&t.dir).commit_messages(&[merge]).unwrap();
        assert_eq!(got[0].as_deref(), Some("merge side\n\nHours: 1\n"));
    }
}
