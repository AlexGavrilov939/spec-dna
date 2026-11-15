//! This module finds the repository, makes tree snapshots (of the working tree
//! or of a commit), and loads parsed documents through the index.

use crate::adapters::VERSION;
use crate::config::{self, Config};
use crate::git::Git;
use crate::index::Index;
use crate::model::{Corpus, is_doc_path};
use crate::parse::{ParsedDoc, parse};
use std::collections::{BTreeMap, HashMap, HashSet};
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
    /// A safety check stopped the change, and nothing changed.
    Refused(String),
}

impl Fatal {
    pub fn exit_code(&self) -> i32 {
        match self {
            Fatal::Usage(_) => 2,
            Fatal::Config(_) => 3,
            Fatal::Io(_) => 4,
            Fatal::NotFound(_) => 5,
            Fatal::Refused(_) => 1,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Fatal::Usage(m)
            | Fatal::Config(m)
            | Fatal::Io(m)
            | Fatal::NotFound(m)
            | Fatal::Refused(m) => m,
        }
    }
}

#[derive(Clone)]
pub struct Repo {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub git: Git,
}

/// A path for a message. On Windows, `canonicalize` adds a `\\?\` prefix that
/// a user does not type. This function removes it.
pub fn shown(p: &Path) -> String {
    let s = p.display().to_string();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s
    }
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
                    shown(&start)
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

    /// The same repository, with the git index as the head tree
    /// (`check --staged`).
    pub fn staged(&self) -> Repo {
        Repo {
            root: self.root.clone(),
            git_dir: self.git_dir.clone(),
            git: self.git.staged(),
        }
    }

    pub fn load_config(&self) -> Result<Config, Fatal> {
        let p = self.config_path();
        let text = std::fs::read_to_string(&p).map_err(|_| {
            Fatal::Config(format!(
                "no {} at {}; run `spec-dna init`",
                config::FILE_NAME,
                shown(&self.root)
            ))
        })?;
        Self::parse_config(&text)
    }

    /// Reads `spec-dna.toml` as the git index has it.
    pub fn load_staged_config(&self) -> Result<Config, Fatal> {
        let blob = self
            .git
            .ls_files()
            .map_err(Fatal::Io)?
            .into_iter()
            .find(|e| e.0 == config::FILE_NAME)
            .map(|e| e.1)
            .ok_or_else(|| {
                Fatal::Config(format!(
                    "no {} in the git index; run `git add {}`",
                    config::FILE_NAME,
                    config::FILE_NAME
                ))
            })?;
        let blobs = self
            .git
            .read_blobs(std::slice::from_ref(&blob))
            .map_err(Fatal::Io)?;
        let bytes = blobs
            .get(&blob)
            .ok_or_else(|| Fatal::Io(format!("blob {blob} is missing")))?;
        Self::parse_config(&String::from_utf8_lossy(bytes))
    }

    /// Reads the text of `spec-dna.toml`.
    pub fn parse_config(text: &str) -> Result<Config, Fatal> {
        let text = text.to_string();
        Config::parse(&text).map_err(|e| {
            // The pin comes first: a configuration of another version can
            // have keys that this binary does not know.
            match config::scan_pin(&text) {
                Some(pin) if pin != VERSION => {
                    Fatal::Config(config::pin_mismatch_of_text(&pin, &text))
                }
                _ => Fatal::Config(format!("{}: {e}", config::FILE_NAME)),
            }
        })
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

/// Returns the status pathspec: the document roots and the convention files
/// outside them, or nothing for the whole tree. Suppose that the pathspec does
/// not have a convention file. Then a changed convention file would keep the
/// blob id of the git index, and the parse cache would keep its old text.
pub fn root_paths(config: &Config) -> Vec<String> {
    if config.roots.iter().any(|r| r.path.is_empty()) {
        return Vec::new();
    }
    config
        .roots
        .iter()
        .map(|r| r.path.clone())
        .chain(config.convention_files())
        .map(|p| format!(":(literal){p}"))
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
    /// The overlay of a planned tree (D155): the text of each path that has no
    /// blob in git. A path with a text has priority over the blob of the path.
    pub texts: Option<Arc<HashMap<String, Vec<u8>>>>,
}

/// A change of a snapshot of a commit: the base tree that `upgrade` plans in
/// memory for `check` (D155). It never writes to git.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overlay {
    /// The moves, as (old path, new path). A moved file keeps its blob.
    pub moved: Vec<(String, String)>,
    /// The files that the plan removes.
    pub removed: Vec<String>,
    /// The new text of each path that the plan writes, also a new path.
    pub texts: HashMap<String, Vec<u8>>,
}

impl Snapshot {
    pub fn empty() -> Snapshot {
        Snapshot::default()
    }

    /// Returns the working tree: tracked files (but not deleted ones) and,
    /// with `untracked`, the untracked files that git does not ignore
    /// (ADR 0001, amendments). `paths` (the document roots) limits the search
    /// for changed and untracked files. The file list always covers the whole
    /// index.
    pub fn worktree(repo: &Repo, paths: &[String], untracked: bool) -> Result<Snapshot, Fatal> {
        let (files, status) = std::thread::scope(|s| {
            let st = s.spawn(|| repo.git.status(paths, untracked));
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
            texts: None,
        })
    }

    /// Returns the git index: the tree that the next commit records. A path
    /// with an unmerged entry is an error, because the index has no tree
    /// before the merge ends.
    pub fn index(repo: &Repo) -> Result<Snapshot, Fatal> {
        if let Some(p) = repo.git.first_unmerged().map_err(Fatal::Io)? {
            return Err(Fatal::Io(format!(
                "the git index has unmerged paths (first: `{p}`); resolve them first"
            )));
        }
        // A path that `git add -N` marked is not in the next commit.
        let ita = repo.git.intent_to_add().map_err(Fatal::Io)?;
        let mut files: Vec<Entry> = repo
            .git
            .ls_files()
            .map_err(Fatal::Io)?
            .into_iter()
            .filter(|(path, ..)| !ita.contains(path))
            .map(|(path, oid, _)| Entry {
                path,
                blob: Some(oid),
            })
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files.dedup_by(|a, b| a.path == b.path);
        Ok(Snapshot {
            worktree: false,
            files,
            texts: None,
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
            texts: None,
        })
    }

    /// The snapshot of a commit with an overlay (D155). Each path that the
    /// plan writes has no blob and has its text in memory. A moved file with
    /// no new text keeps its blob. Another file keeps its blob, so the readers
    /// that compare blob ids see only the planned change.
    pub fn overlaid(self, overlay: &Overlay) -> Snapshot {
        let mut files: BTreeMap<String, Option<String>> =
            self.files.into_iter().map(|e| (e.path, e.blob)).collect();
        for (from, to) in &overlay.moved {
            if let Some(blob) = files.remove(from) {
                files.insert(to.clone(), blob);
            }
        }
        for path in &overlay.removed {
            files.remove(path);
        }
        // The text of a path is in memory, so the entry has no blob.
        for path in overlay.texts.keys() {
            files.insert(path.clone(), None);
        }
        Snapshot {
            worktree: false,
            files: files
                .into_iter()
                .map(|(path, blob)| Entry { path, blob })
                .collect(),
            texts: Some(Arc::new(overlay.texts.clone())),
        }
    }

    /// The overlay text of a path, if the snapshot has one.
    fn text_of(&self, path: &str) -> Option<&Vec<u8>> {
        self.texts.as_ref().and_then(|t| t.get(path))
    }

    /// Reads the bytes of `paths` from a snapshot that is not the working
    /// tree: the overlay text of a path, or else its blob. A path that the
    /// snapshot does not have is not in the result.
    pub fn read(&self, repo: &Repo, paths: &[String]) -> Result<HashMap<String, Vec<u8>>, Fatal> {
        let mut out = HashMap::new();
        let mut ids: Vec<(String, String)> = Vec::new();
        for p in paths {
            let Some(entry) = self.blob_of(p) else {
                continue;
            };
            if let Some(text) = self.text_of(p) {
                out.insert(p.clone(), text.clone());
            } else if let Some(b) = entry.blob.clone() {
                ids.push((p.clone(), b));
            }
        }
        let blobs = repo
            .git
            .read_blobs(&ids.iter().map(|i| i.1.clone()).collect::<Vec<_>>())
            .map_err(Fatal::Io)?;
        for (p, b) in ids {
            if let Some(bytes) = blobs.get(&b) {
                out.insert(p, bytes.clone());
            }
        }
        Ok(out)
    }

    pub fn blob_of(&self, path: &str) -> Option<&Entry> {
        self.files
            .binary_search_by(|e| e.path.as_str().cmp(path))
            .ok()
            .map(|i| &self.files[i])
    }
}

/// The source of the text of a document: a file of the working tree, a blob of
/// git, or a text of the overlay of a planned tree (D155). Disk and memory
/// texts have no blob id, so the index does not cache them.
enum Source {
    Disk(String),
    Blob(String),
    Mem(String, Vec<u8>),
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
    // Documents and convention files per snapshot, and the work to do.
    let mut per_snap: Vec<(Vec<&Entry>, Vec<&Entry>)> = Vec::new();
    // Each job has the number of its snapshot: 2 snapshots can have a file at
    // one path with 2 texts, and the blob id does not tell them apart.
    let mut jobs: Vec<(Source, Option<String>, usize)> = Vec::new();
    let mut queued: HashSet<String> = HashSet::new();
    let mut used: HashSet<String> = HashSet::new();
    let convention_files = config.convention_files();
    let root_dir = std::fs::canonicalize(&repo.root).unwrap_or_else(|_| repo.root.clone());
    for (si, snap) in snaps.iter().enumerate() {
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
        // In the working tree, a convention file can be a symlink: spec-dna
        // reads it only if its target is inside the repository.
        let outs: Vec<&Entry> = convention_files
            .iter()
            .filter_map(|p| snap.blob_of(p))
            .filter(|e| !snap.worktree || inside_repo(&root_dir, &repo.root.join(&e.path)))
            .collect();
        // spec-dna reads a convention file of the working tree from disk on
        // every run. It does not cache the file, because the blob id of a
        // symlink stays the same when its target changes. There are few such
        // files.
        let items = docs
            .iter()
            .map(|e| (e, false))
            .chain(outs.iter().map(|e| (e, snap.worktree)));
        for (e, uncached) in items {
            match &e.blob {
                Some(b) if !uncached => {
                    used.insert(b.clone());
                    if !index.entries.contains_key(b) && queued.insert(b.clone()) {
                        let src = if snap.worktree {
                            Source::Disk(e.path.clone())
                        } else {
                            Source::Blob(b.clone())
                        };
                        jobs.push((src, Some(b.clone()), si));
                    }
                }
                _ => {
                    let src = match snap.text_of(&e.path) {
                        Some(t) if !snap.worktree => Source::Mem(e.path.clone(), t.clone()),
                        _ => Source::Disk(e.path.clone()),
                    };
                    jobs.push((src, None, si));
                }
            }
        }
        per_snap.push((docs, outs));
    }

    trace("index load", &mut t);
    let blob_ids: Vec<String> = jobs
        .iter()
        .filter_map(|(s, ..)| {
            if let Source::Blob(b) = s {
                Some(b.clone())
            } else {
                None
            }
        })
        .collect();
    let blobs = repo.git.read_blobs(&blob_ids).map_err(Fatal::Io)?;
    let root = &repo.root;
    let results: Vec<Result<ParsedDoc, String>> = par_map(&jobs, |(src, ..)| {
        let bytes = match src {
            Source::Disk(p) => std::fs::read(root.join(p)).map_err(|e| format!("{p}: {e}"))?,
            Source::Mem(_, t) => t.clone(),
            Source::Blob(b) => blobs
                .get(b)
                .cloned()
                .ok_or_else(|| format!("blob {b} is missing"))?,
        };
        Ok(parse(&String::from_utf8_lossy(&bytes)))
    });

    trace("read and parse", &mut t);
    // The documents with no blob id: the index does not keep them.
    let mut uncached: HashMap<(usize, String), Arc<ParsedDoc>> = HashMap::new();
    let parsed = results.len();
    for ((src, blob, si), res) in jobs.into_iter().zip(results) {
        let doc = Arc::new(res.map_err(Fatal::Io)?);
        match blob {
            Some(b) => {
                index.entries.insert(b, doc);
            }
            None => {
                if let Source::Disk(p) | Source::Mem(p, _) = src {
                    uncached.insert((si, p), doc);
                }
            }
        }
    }
    let mut changed = parsed > uncached.len();
    if prune {
        let before = index.entries.len();
        index.entries.retain(|k, _| used.contains(k));
        changed |= index.entries.len() != before;
    }
    let cached = used.len().saturating_sub(parsed - uncached.len());

    let mut corpora = Vec::new();
    for (si, (snap, (docs, outs))) in snaps.iter().zip(per_snap).enumerate() {
        let files: Vec<String> = snap.files.iter().map(|e| e.path.clone()).collect();
        let parsed_of = |e: &Entry| -> (String, Arc<ParsedDoc>) {
            let d = match &e.blob {
                Some(b) => index.entries[b].clone(),
                None => uncached[&(si, e.path.clone())].clone(),
            };
            (e.path.clone(), d)
        };
        let docs: Vec<(String, Arc<ParsedDoc>)> = docs.into_iter().map(parsed_of).collect();
        let mut corpus = Corpus::build(config, files, docs);
        corpus.outside = outs
            .into_iter()
            .map(|e| {
                if snap.worktree {
                    (e.path.clone(), uncached[&(si, e.path.clone())].clone())
                } else {
                    parsed_of(e)
                }
            })
            .collect();
        corpora.push(corpus);
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

/// True if `path` exists and resolves (through symlinks) to a path inside the
/// canonical repository root `root`.
fn inside_repo(root: &Path, path: &Path) -> bool {
    std::fs::canonicalize(path).is_ok_and(|p| p.starts_with(root))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_drops_the_windows_verbatim_prefix() {
        assert_eq!(shown(Path::new(r"\\?\C:\repo")), r"C:\repo");
        assert_eq!(
            shown(Path::new(r"\\?\UNC\server\share\repo")),
            r"\\server\share\repo"
        );
        assert_eq!(shown(Path::new("/home/u/repo")), "/home/u/repo");
    }

    fn snap(files: &[(&str, Option<&str>)]) -> Snapshot {
        Snapshot {
            worktree: false,
            files: files
                .iter()
                .map(|(p, b)| Entry {
                    path: p.to_string(),
                    blob: b.map(str::to_string),
                })
                .collect(),
            texts: None,
        }
    }

    fn overlay(moved: &[(&str, &str)], removed: &[&str], texts: &[(&str, &str)]) -> Overlay {
        Overlay {
            moved: moved
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            removed: removed.iter().map(|p| p.to_string()).collect(),
            texts: texts
                .iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        }
    }

    fn view(s: &Snapshot) -> Vec<(&str, Option<&str>)> {
        s.files
            .iter()
            .map(|e| (e.path.as_str(), e.blob.as_deref()))
            .collect()
    }

    /// A temporary folder that is no git repository.
    fn scratch(name: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "spec-dna-repo-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_overlay_text_has_priority_over_the_blob_of_its_path() {
        // The folder is no git repository: a read of a blob would fail.
        let dir = scratch("priority");
        let repo = Repo::new(&dir, dir.join(".git"));
        let base = snap(&[("a.md", Some("aaaa")), ("b.md", Some("bbbb"))]).overlaid(&overlay(
            &[],
            &[],
            &[("a.md", "planned a\n")],
        ));
        let got = base.read(&repo, &["a.md".to_string()]).unwrap();
        assert_eq!(got["a.md"], b"planned a\n");
        // A path that the snapshot lacks has no text, also if the overlay has one.
        let got = base.read(&repo, &["zz.md".to_string()]).unwrap();
        assert!(got.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_overlay_moves_removes_and_writes_and_keeps_the_other_blobs() {
        let base = snap(&[
            ("docs/a.md", Some("a1")),
            ("docs/b.md", Some("b1")),
            ("docs/c.md", Some("c1")),
            ("docs/img.png", Some("p1")),
            (".claude/agents/checker.md", Some("k1")),
        ]);
        let ov = overlay(
            &[
                ("docs/a.md", "docs/intent/a/intent.md"),
                ("docs/img.png", "docs/res/img.png"),
            ],
            &[".claude/agents/checker.md"],
            &[
                ("docs/intent/a/intent.md", "moved and edited\n"),
                ("docs/b.md", "edited\n"),
                (".claude/agents/verifier.md", "new\n"),
            ],
        );
        let got = base.overlaid(&ov);
        assert!(!got.worktree);
        assert_eq!(
            view(&got),
            vec![
                (".claude/agents/verifier.md", None),
                ("docs/b.md", None),
                ("docs/c.md", Some("c1")),
                ("docs/intent/a/intent.md", None),
                ("docs/res/img.png", Some("p1")),
            ]
        );
        // The list stays sorted, so `blob_of` finds each path.
        assert!(got.blob_of("docs/res/img.png").is_some());
        assert!(got.blob_of("docs/a.md").is_none());
    }

    #[test]
    fn an_overlay_text_and_a_head_file_at_one_path_do_not_collide() {
        let dir = scratch("collide");
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(dir.join("docs/a.md"), "# Head title\n").unwrap();
        let repo = Repo::new(&dir, dir.join(".git"));
        let cfg = Config::parse(&format!(
            "spec-dna = \"{VERSION}\"\n[[roots]]\npath = \"docs\"\n"
        ))
        .unwrap();
        // The head is the working tree, and its file differs from the index
        // (no blob). The base is a planned tree with a text at the same path.
        let head = Snapshot {
            worktree: true,
            files: vec![Entry {
                path: "docs/a.md".into(),
                blob: None,
            }],
            texts: None,
        };
        let base = snap(&[("docs/a.md", Some("aaaa"))]).overlaid(&overlay(
            &[],
            &[],
            &[("docs/a.md", "# Base title\n")],
        ));
        let loaded = load(&repo, &cfg, &[&head, &base], false, false).unwrap();
        let title = |i: usize| loaded.corpora[i].docs[0].parsed.title.clone();
        assert_eq!(title(0), "Head title");
        assert_eq!(title(1), "Base title");
        assert_eq!(loaded.parsed, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
