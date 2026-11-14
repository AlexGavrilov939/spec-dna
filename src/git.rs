//! This module accesses git through the commands of the git CLI (ADR 0002, git
//! access decision). Every command runs with `GIT_OPTIONAL_LOCKS=0`, so that a
//! read never writes the index of the repository. No command uses the network.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Git {
    root: PathBuf,
}

/// A tree entry: path, object id, and whether it is a submodule.
pub type Entry = (String, String, bool);

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
        }
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new("git");
        c.arg("-C").arg(&self.root).args(args);
        c.env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null());
        c
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
        let out = self.run(&[
            "diff",
            "--name-status",
            "-z",
            "--no-renames",
            "--ignore-submodules=all",
            rev,
            "--",
        ])?;
        let mut v = Vec::new();
        let mut recs = out.split(|b| *b == 0).filter(|r| !r.is_empty());
        while let (Some(status), Some(path)) = (recs.next(), recs.next()) {
            v.push((
                String::from_utf8_lossy(path).into_owned(),
                status.first() == Some(&b'A'),
            ));
        }
        if untracked {
            let out = self.run(&["ls-files", "--others", "--exclude-standard", "-z"])?;
            for rec in out.split(|b| *b == 0).filter(|r| !r.is_empty()) {
                v.push((String::from_utf8_lossy(rec).into_owned(), true));
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
        if untracked {
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
        if untracked && rev.is_none() {
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
