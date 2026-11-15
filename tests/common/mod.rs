//! Shared helpers for the tests that need a git repository. A test file uses
//! them with `mod common;`. The helpers read no clock, no user config and no
//! network. The git calls set the author, disable signing and ignore the
//! global and system config. `git_at` sets the dates, so a test can set the
//! age of a commit.

// Each file in tests/ is its own crate and uses only some helpers.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A git command in `dir` with the fixed identity and the isolated config.
fn git_cmd(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        // A test run from a git hook must not use the repository of the hook.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    cmd
}

fn finish(mut cmd: Command, args: &[&str]) -> Output {
    let out = cmd.args(args).output().expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// Runs git in `dir` and asserts success.
pub fn git(dir: &Path, args: &[&str]) {
    finish(git_cmd(dir), args);
}

/// Runs git in `dir`, asserts success and returns stdout without the final
/// newline.
pub fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = finish(git_cmd(dir), args);
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

/// Like `git`, with the author date and the committer date set to `date`
/// (ISO 8601, for example `2024-01-15T12:00:00+00:00`).
pub fn git_at(dir: &Path, args: &[&str], date: &str) {
    let mut cmd = git_cmd(dir);
    cmd.env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date);
    finish(cmd, args);
}

/// Makes `refs/remotes/origin/HEAD` point to the branch `origin/main` at
/// `rev`. The repository has no remote.
pub fn origin_head(dir: &Path, rev: &str) {
    git(dir, &["update-ref", "refs/remotes/origin/main", rev]);
    git(
        dir,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
}

/// A shallow clone of the repository `src` in `dst` with `depth` commits. It
/// uses a `file://` URL. A plain path would make a full local clone. `dst`
/// must not exist or must be empty.
pub fn shallow_clone(src: &Path, dst: &Path, depth: usize) {
    let abs = fs::canonicalize(src).expect("source exists");
    let url = format!("file://{}", abs.display());
    let depth = depth.to_string();
    let dst = dst.to_str().expect("path is UTF-8");
    git(src, &["clone", "-q", "--depth", &depth, &url, dst]);
}

fn write_file(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, text).unwrap();
}

fn commit_all(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", msg]);
}

/// The feature branch of the 3 merge helpers. It starts at `main`, adds 2
/// commits (`feature-a.txt`, `feature-b.txt`) and returns to `main`. Then
/// `main` gets 1 commit of its own (`main-side.txt`), so `main` has moved
/// when the branch comes back. `main` must have at least 1 commit.
fn feature_branch(dir: &Path) {
    git(dir, &["checkout", "-q", "-b", "feature"]);
    write_file(dir, "feature-a.txt", "a\n");
    commit_all(dir, "feature a");
    write_file(dir, "feature-b.txt", "b\n");
    commit_all(dir, "feature b");
    git(dir, &["checkout", "-q", "main"]);
    write_file(dir, "main-side.txt", "m\n");
    commit_all(dir, "main side");
}

/// Merges the feature branch into `main` as 1 new squash commit. The branch
/// commits do not reach `main`.
pub fn merge_squash(dir: &Path) {
    feature_branch(dir);
    git(dir, &["merge", "-q", "--squash", "feature"]);
    git(dir, &["commit", "-q", "-m", "squash feature"]);
}

/// Merges the feature branch into `main` with a merge commit (`--no-ff`).
pub fn merge_commit(dir: &Path) {
    feature_branch(dir);
    git(
        dir,
        &["merge", "-q", "--no-ff", "-m", "merge feature", "feature"],
    );
}

/// Rebases the feature branch on `main`, then fast-forwards `main`. The
/// history stays linear.
pub fn merge_rebase(dir: &Path) {
    feature_branch(dir);
    git(dir, &["checkout", "-q", "feature"]);
    git(dir, &["rebase", "-q", "main"]);
    git(dir, &["checkout", "-q", "main"]);
    git(dir, &["merge", "-q", "--ff-only", "feature"]);
}

/// A temporary directory with a unique name. It is removed on drop.
pub struct Repo {
    pub dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl Repo {
    /// An empty directory, with no git repository (for a clone target).
    pub fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-common-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Repo { dir }
    }

    /// A git repository with branch `main` and no commit.
    pub fn new() -> Repo {
        let repo = Repo::empty();
        git(&repo.dir, &["init", "-q", "-b", "main"]);
        repo
    }

    /// A repository with a `spec-dna.toml`: the version pin, then `toml`.
    pub fn with_config(toml: &str) -> Repo {
        let repo = Repo::new();
        repo.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{VERSION}\"\n{toml}"),
        );
        repo
    }

    /// Writes a file, and its parent folders.
    pub fn write(&self, path: &str, text: &str) {
        write_file(&self.dir, path, text);
    }

    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    /// Runs git in the repository and asserts success.
    pub fn git(&self, args: &[&str]) {
        git(&self.dir, args);
    }

    /// Runs git in the repository and returns stdout.
    pub fn git_out(&self, args: &[&str]) -> String {
        git_out(&self.dir, args)
    }

    /// Stages all files and commits them.
    pub fn commit(&self, msg: &str) {
        commit_all(&self.dir, msg);
    }

    /// Like `commit`, with the author date and the committer date `date`.
    pub fn commit_at(&self, msg: &str, date: &str) {
        git_at(&self.dir, &["add", "-A"], date);
        git_at(&self.dir, &["commit", "-q", "-m", msg], date);
    }

    /// Runs the built binary in the repository: exit code, stdout, stderr.
    pub fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}
