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
    /// It reads only this ref, with no fallback to a local branch, so that the
    /// result is the same for every checkout. `git remote set-head origin
    /// <branch>` sets the ref.
    pub fn remote_head(&self) -> Option<String> {
        self.commit("refs/remotes/origin/HEAD")
    }

    /// Returns true if `a` is `b` or an ancestor of `b`. An error means that
    /// a commit is absent, as in a shallow clone.
    #[allow(dead_code)] // reader: S5
    pub fn is_ancestor(&self, a: &str, b: &str) -> Result<bool, String> {
        let out = self
            .cmd(&["merge-base", "--is-ancestor", a, b])
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("cannot run git: {e}"))?;
        match out.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(format!(
                "git merge-base --is-ancestor failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        }
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

    /// Returns, for each file under `specs` that a commit touched, the newest
    /// committer date (seconds) of such a commit. It walks the history of
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
    /// the tree of `base` nor the tree of `head`. Such a change was born and
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
    /// only the commits of `<since>..HEAD`. A move is not a deletion. The walk
    /// follows every parent and shows no merge commit: a squash, a merge
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

    /// Returns each touch of an `intent.md`, `spec.md` or `plan.md` that is
    /// one level under `dir` (the folder of the `intent` layer, from the
    /// repository root, with no final `/`), in the history of `HEAD`. The
    /// order is the topological order, the newest first: a parent never comes
    /// before its child. It is not the order of the committer dates, which a
    /// history does not keep in order. A commit that touches 2 files gives 2
    /// entries, one after the other. A merge commit gives no entry. A rename of the
    /// folder inside `dir` is a touch of the new path, with the old path in
    /// `from`. A move out of `dir` reads as a deletion.
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
fn glob_pathspecs(specs: &[String]) -> Vec<String> {
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
