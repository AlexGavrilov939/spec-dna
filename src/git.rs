//! This module accesses git through the CLI plumbing commands (ADR 0002, git
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
    /// returns the changes in all of the tree.
    /// Lists changed files under `paths`. With `untracked`, it also lists the
    /// untracked files that git does not ignore.
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

    pub fn mv(&self, from: &str, to: &str) -> Result<(), String> {
        self.run(&["mv", "--", from, to]).map(|_| ())
    }

    pub fn is_tracked(&self, path: &str) -> bool {
        self.run(&["ls-files", "--error-unmatch", "--", path])
            .is_ok()
    }
}
