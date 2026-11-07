//! This module finds the repository, makes tree snapshots (of the working tree
//! or of a commit), and loads parsed documents through the index.

use crate::config::{self, Config};
use crate::git::Git;
use crate::index::Index;
use crate::model::{Corpus, is_doc_path};
use crate::parse::{ParsedDoc, parse};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A failure that stops a command, with its exit code (README, "Exit codes").
#[derive(Debug)]
pub enum Fatal {
    Usage(String),
    Config(String),
    Io(String),
    NotFound(String),
}

impl Fatal {
    pub fn exit_code(&self) -> i32 {
        match self {
            Fatal::Usage(_) => 2,
            Fatal::Config(_) => 3,
            Fatal::Io(_) => 4,
            Fatal::NotFound(_) => 5,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Fatal::Usage(m) | Fatal::Config(m) | Fatal::Io(m) | Fatal::NotFound(m) => m,
        }
    }
}

pub struct Repo {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub git: Git,
}

impl Repo {
    /// Finds the git working tree that contains `start`.
    pub fn discover(start: &Path) -> Result<Repo, Fatal> {
        let start = start
            .canonicalize()
            .map_err(|e| Fatal::Io(format!("{}: {e}", start.display())))?;
        let mut dir = start.as_path();
        loop {
            let dotgit = dir.join(".git");
            if dotgit.is_dir() {
                return Ok(Repo::new(dir, dotgit));
            }
            if dotgit.is_file() {
                let text =
                    std::fs::read_to_string(&dotgit).map_err(|e| Fatal::Io(e.to_string()))?;
                let gd = text
                    .trim()
                    .strip_prefix("gitdir:")
                    .map(str::trim)
                    .unwrap_or("");
                let gd = dir.join(gd);
                return Ok(Repo::new(dir, gd));
            }
            dir = dir.parent().ok_or_else(|| {
                Fatal::Config(format!(
                    "{} is not inside a git working tree; spec-dna reads documents through git",
                    start.display()
                ))
            })?;
        }
    }

    fn new(root: &Path, git_dir: PathBuf) -> Repo {
        Repo {
            root: root.to_path_buf(),
            git_dir,
            git: Git::new(root),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join(config::FILE_NAME)
    }

    pub fn load_config(&self) -> Result<Config, Fatal> {
        let p = self.config_path();
        let text = std::fs::read_to_string(&p).map_err(|_| {
            Fatal::Config(format!(
                "no {} at {}; run `spec-dna init`",
                config::FILE_NAME,
                self.root.display()
            ))
        })?;
        Config::parse(&text).map_err(|e| Fatal::Config(format!("{}: {e}", config::FILE_NAME)))
    }

    /// Returns the commit that `HEAD` points to. It reads the commit from the
    /// git directory and does not start git. For layouts that it does not read
    /// (reftable, unusual worktrees), it uses `git rev-parse`. Before the first
    /// commit, it returns `None`.
    pub fn head_commit(&self) -> Option<String> {
        let read = |p: PathBuf| {
            std::fs::read_to_string(p)
                .ok()
                .map(|s| s.trim().to_string())
        };
        let common = read(self.git_dir.join("commondir"))
            .map_or(self.git_dir.clone(), |c| self.git_dir.join(c));
        let fallback = || self.git.commit("HEAD");
        if common.join("reftable").exists() {
            return fallback();
        }
        let head = read(self.git_dir.join("HEAD"))?;
        let Some(name) = head.strip_prefix("ref: ") else {
            return if is_oid(&head) {
                Some(head)
            } else {
                fallback()
            };
        };
        for dir in [&self.git_dir, &common] {
            if let Some(oid) = read(dir.join(name)).filter(|o| is_oid(o)) {
                return Some(oid);
            }
        }
        if let Some(packed) = read(common.join("packed-refs")) {
            for line in packed.lines() {
                if let Some((oid, r)) = line.split_once(' ')
                    && r == name
                    && is_oid(oid)
                {
                    return Some(oid.to_string());
                }
            }
            return None;
        }
        if common.join("refs").is_dir() {
            None
        } else {
            fallback()
        }
    }

    pub fn index_path(&self) -> PathBuf {
        self.git_dir.join("spec-dna").join("index-v1.bin")
    }
}

fn is_oid(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Returns the status pathspec: the document roots, or nothing for the whole
/// tree.
pub fn root_paths(config: &Config) -> Vec<String> {
    if config.roots.iter().any(|r| r.path.is_empty()) {
        return Vec::new();
    }
    config
        .roots
        .iter()
        .map(|r| format!(":(literal){}", r.path))
        .collect()
}

/// One file of a tree. `blob` is `None` for a working-tree file whose
/// content differs from the git index, or that git does not track.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    pub blob: Option<String>,
}

#[derive(Default)]
pub struct Snapshot {
    /// True for the working tree. Then spec-dna reads clean files from disk.
    pub worktree: bool,
    pub files: Vec<Entry>,
}

impl Snapshot {
    pub fn empty() -> Snapshot {
        Snapshot::default()
    }

    /// Returns the working tree: tracked files (but not deleted ones) and
    /// untracked files that git does not ignore.
    /// `paths` (the document roots) limits the search for changed and
    /// untracked files. The file list always covers the whole index.
    pub fn worktree(repo: &Repo, paths: &[String]) -> Result<Snapshot, Fatal> {
        let (files, status) = std::thread::scope(|s| {
            let st = s.spawn(|| repo.git.status(paths));
            let files = repo.git.ls_files();
            (
                files,
                st.join()
                    .unwrap_or_else(|_| Err("git status panicked".into())),
            )
        });
        let files = files.map_err(Fatal::Io)?;
        let status = status.map_err(Fatal::Io)?;
        let mut out: Vec<Entry> = Vec::with_capacity(files.len() + status.untracked.len());
        for (path, oid, _) in files {
            if status.deleted.contains(&path) {
                continue;
            }
            let blob = (!status.modified.contains(&path)).then_some(oid);
            out.push(Entry { path, blob });
        }
        out.extend(
            status
                .untracked
                .into_iter()
                .map(|path| Entry { path, blob: None }),
        );
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.dedup_by(|a, b| a.path == b.path);
        Ok(Snapshot {
            worktree: true,
            files: out,
        })
    }

    pub fn at(repo: &Repo, commit: &str) -> Result<Snapshot, Fatal> {
        let files = repo.git.ls_tree(commit).map_err(Fatal::Io)?;
        Ok(Snapshot {
            worktree: false,
            files: files
                .into_iter()
                .map(|(path, oid, _)| Entry {
                    path,
                    blob: Some(oid),
                })
                .collect(),
        })
    }

    pub fn blob_of(&self, path: &str) -> Option<&Entry> {
        self.files
            .binary_search_by(|e| e.path.as_str().cmp(path))
            .ok()
            .map(|i| &self.files[i])
    }
}

/// The source of the text of a document.
enum Source {
    Disk(String),
    Blob(String),
}

/// Loads the parsed documents of several snapshots through the index.
/// Returns the corpus files and documents for each snapshot.
pub struct Loaded {
    pub corpora: Vec<Corpus>,
    pub parsed: usize,
    pub cached: usize,
}

pub fn load(
    repo: &Repo,
    config: &Config,
    snaps: &[&Snapshot],
    use_cache: bool,
    prune: bool,
) -> Result<Loaded, Fatal> {
    let mut t = std::time::Instant::now();
    let mut index = if use_cache {
        Index::load(repo.index_path())
    } else {
        Index::empty(repo.index_path())
    };
    // Documents per snapshot, and the work to do.
    let mut per_snap: Vec<Vec<&Entry>> = Vec::new();
    let mut jobs: Vec<(Source, Option<String>)> = Vec::new();
    let mut queued: HashSet<String> = HashSet::new();
    let mut used: HashSet<String> = HashSet::new();
    for snap in snaps {
        let docs: Vec<&Entry> = snap
            .files
            .iter()
            .filter(|e| is_doc_path(&e.path))
            .filter(|e| {
                config
                    .root_of(&e.path)
                    .is_some_and(|r| !config.is_excluded(r, &e.path))
            })
            .collect();
        for e in &docs {
            match &e.blob {
                Some(b) => {
                    used.insert(b.clone());
                    if !index.entries.contains_key(b) && queued.insert(b.clone()) {
                        let src = if snap.worktree {
                            Source::Disk(e.path.clone())
                        } else {
                            Source::Blob(b.clone())
                        };
                        jobs.push((src, Some(b.clone())));
                    }
                }
                None => jobs.push((Source::Disk(e.path.clone()), None)),
            }
        }
        per_snap.push(docs);
    }

    trace("index load", &mut t);
    let blob_ids: Vec<String> = jobs
        .iter()
        .filter_map(|(s, _)| {
            if let Source::Blob(b) = s {
                Some(b.clone())
            } else {
                None
            }
        })
        .collect();
    let blobs = repo.git.read_blobs(&blob_ids).map_err(Fatal::Io)?;
    let root = &repo.root;
    let results: Vec<Result<ParsedDoc, String>> = par_map(&jobs, |(src, _)| {
        let bytes = match src {
            Source::Disk(p) => std::fs::read(root.join(p)).map_err(|e| format!("{p}: {e}"))?,
            Source::Blob(b) => blobs
                .get(b)
                .cloned()
                .ok_or_else(|| format!("blob {b} is missing"))?,
        };
        Ok(parse(&String::from_utf8_lossy(&bytes)))
    });

    trace("read and parse", &mut t);
    let mut worktree_docs: HashMap<String, Arc<ParsedDoc>> = HashMap::new();
    let parsed = results.len();
    for ((src, blob), res) in jobs.into_iter().zip(results) {
        let doc = Arc::new(res.map_err(Fatal::Io)?);
        match blob {
            Some(b) => {
                index.entries.insert(b, doc);
            }
            None => {
                if let Source::Disk(p) = src {
                    worktree_docs.insert(p, doc);
                }
            }
        }
    }
    let mut changed = parsed > worktree_docs.len();
    if prune {
        let before = index.entries.len();
        index.entries.retain(|k, _| used.contains(k));
        changed |= index.entries.len() != before;
    }
    let cached = used.len().saturating_sub(parsed - worktree_docs.len());

    let mut corpora = Vec::new();
    for (snap, docs) in snaps.iter().zip(per_snap) {
        let files: Vec<String> = snap.files.iter().map(|e| e.path.clone()).collect();
        let docs: Vec<(String, Arc<ParsedDoc>)> = docs
            .into_iter()
            .map(|e| {
                let d = match &e.blob {
                    Some(b) => index.entries[b].clone(),
                    None => worktree_docs[&e.path].clone(),
                };
                (e.path.clone(), d)
            })
            .collect();
        corpora.push(Corpus::build(config, files, docs));
    }
    trace("build corpora", &mut t);
    if use_cache && changed {
        // The index is a cache. If the write fails, it is not an error.
        let _ = index.save();
        trace("index save", &mut t);
    }
    Ok(Loaded {
        corpora,
        parsed,
        cached,
    })
}

/// If `SPEC_DNA_TRACE` is set, writes the time of each phase to stderr (to
/// profile spec-dna).
pub fn trace(phase: &str, since: &mut std::time::Instant) {
    if std::env::var_os("SPEC_DNA_TRACE").is_some() {
        eprintln!(
            "trace {phase:<18} {:7.2} ms",
            since.elapsed().as_secs_f64() * 1000.0
        );
    }
    *since = std::time::Instant::now();
}

/// Maps items on all cores. A small input runs on the current thread.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(items.len() / 8 + 1);
    if threads <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut parts: Vec<Vec<(usize, R)>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= items.len() {
                            break;
                        }
                        local.push((i, f(&items[i])));
                    }
                    local
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("worker panicked"))
            .collect()
    });
    let mut out: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    for part in parts.iter_mut() {
        for (i, r) in part.drain(..) {
            out[i] = Some(r);
        }
    }
    out.into_iter()
        .map(|r| r.expect("every item mapped"))
        .collect()
}
