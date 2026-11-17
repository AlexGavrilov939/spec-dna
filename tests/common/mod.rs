//! Shared helpers for the tests that need a git repository. A test file uses
//! them with `mod common;`. The helpers read no clock, no user config and no
//! network. The git calls set the author, disable signing and ignore the
//! global and system config. `git_at` sets the dates, so a test can set the
//! age of a commit. `Repo::run_env` runs the binary with a changed
//! environment, `stub_bin` and `path_with` replace a program on `PATH`, and
//! `normalize_marker` removes the version from a harness marker. `import`
//! writes the fixture projects of `spec-dna import`.
//! `Repo::from_fixture` copies a committed tree of `tests/fixtures/upgrade/`
//! into a new repository with 1 commit, and `tree_bytes` lists a tree for an
//! equal compare. `pin_file` and `sums_table` write the file `spec-dna-pin.toml`
//! that `upgrade --pin-file` reads. `md_lint` (in `md_lint.rs`) checks a
//! Markdown text against the markdownlint rules that the generated files can
//! break.

// Each file in tests/ is its own crate and uses only some helpers.
#![allow(dead_code)]

pub mod import;
mod md_lint;
// Only some test crates use it.
#[allow(unused_imports)]
pub use md_lint::md_lint;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);
/// The date of the first commit of `Repo::from_fixture`.
pub const FIXTURE_DATE: &str = "2026-01-15T12:00:00+00:00";

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

/// Like `git`, with the environment variables `env` set for the command (a
/// name and a value).
pub fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) {
    let mut cmd = git_cmd(dir);
    for (name, value) in env {
        cmd.env(name, value);
    }
    finish(cmd, args);
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

    /// A repository on `main` with 1 commit, dated `FIXTURE_DATE`. The
    /// commit holds the fixture `name` of `tests/fixtures/upgrade/`, then
    /// each folder of `overlays` laid over it. A file of an overlay replaces
    /// the file of the same path. A file `<path>.append` adds its text at the
    /// end of `<path>`, which must exist. The fixture folders have no other
    /// file than the files of the project, so a dotfile is copied too.
    pub fn from_fixture(name: &str, overlays: &[&str]) -> Repo {
        let repo = Repo::new();
        for part in std::iter::once(&name).chain(overlays) {
            overlay(&fixture_dir(part), &repo.dir);
        }
        repo.commit_at("fixture", FIXTURE_DATE);
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

    /// Like `commit`, with the e-mail of the author set to `author`. The
    /// committer keeps the e-mail of the fixture, so the commit has an author
    /// that differs from its committer, as a commit of the web editor of a
    /// code host has.
    pub fn commit_as(&self, msg: &str, author: &str) {
        self.commit_env(msg, &[("GIT_AUTHOR_EMAIL", author)]);
    }

    /// Like `commit`, with the environment variables `env` set for the commit
    /// (for example `GIT_AUTHOR_EMAIL` and `GIT_COMMITTER_EMAIL`).
    pub fn commit_env(&self, msg: &str, env: &[(&str, &str)]) {
        git(&self.dir, &["add", "-A"]);
        git_env(&self.dir, &["commit", "-q", "-m", msg], env);
    }

    /// Like `commit`, with the author date and the committer date `date`.
    pub fn commit_at(&self, msg: &str, date: &str) {
        git_at(&self.dir, &["add", "-A"], date);
        git_at(&self.dir, &["commit", "-q", "-m", msg], date);
    }

    /// Runs the built binary in the repository: exit code, stdout, stderr.
    pub fn run(&self, args: &[&str]) -> (i32, String, String) {
        self.run_env(args, &[], &[])
    }

    /// Like `run`, with the environment variables `set` added (a name and a
    /// value) and the variables `remove` deleted. A name in both lists is
    /// deleted, because the removals apply last.
    pub fn run_env(
        &self,
        args: &[&str],
        set: &[(&str, &str)],
        remove: &[&str],
    ) -> (i32, String, String) {
        let mut cmd = Command::new(BIN);
        cmd.args(args).current_dir(&self.dir);
        for (name, value) in set {
            cmd.env(name, value);
        }
        for name in remove {
            cmd.env_remove(name);
        }
        let out = cmd.output().expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

/// Writes the executable script `dir/name` (mode 755) and returns its path.
/// The script is the whole file, so it needs its own shebang line. The helper
/// creates `dir` if it is absent. A test puts `dir` before the real `PATH`
/// (see `path_with`) to replace a program such as `claude` with a stub.
#[cfg(unix)]
pub fn stub_bin(dir: &Path, name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The value of `PATH` with `dir` before the `PATH` of the test process.
pub fn path_with(dir: &Path) -> String {
    let mut paths = vec![dir.to_path_buf()];
    if let Some(old) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&old));
    }
    std::env::join_paths(paths)
        .expect("PATH has no separator in a folder name")
        .to_string_lossy()
        .into_owned()
}

/// Replaces the version of each harness marker in `text` with `{VERSION}`,
/// so a golden file does not change at a release. The hash of a marker does
/// not cover the version, so it stays as it is. Text that is not a marker
/// stays as it is.
pub fn normalize_marker(text: &str) -> String {
    const OPEN: &str = "<!-- spec-dna harness v";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        let (head, tail) = rest.split_at(at + OPEN.len());
        out.push_str(head);
        let end = tail.find(' ').unwrap_or(tail.len());
        if tail[end..].starts_with(" h:") && end > 0 && !tail[..end].contains('\n') {
            out.push_str("{VERSION}");
            rest = &tail[end..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The folder of the fixture `name` under `tests/fixtures/upgrade/`.
pub fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/upgrade")
        .join(name)
}

/// Copies the files of `from` into `to`. A file `<path>.append` is added to
/// the end of `<path>`.
fn overlay(from: &Path, to: &Path) {
    assert!(from.is_dir(), "no fixture folder {}", from.display());
    for (rel, bytes) in tree_bytes(from) {
        let target = match rel.strip_suffix(".append") {
            Some(base) => {
                let mut text = fs::read(to.join(base)).expect("the file to append to exists");
                text.extend_from_slice(&bytes);
                write_bytes(to, base, &text);
                continue;
            }
            None => rel,
        };
        write_bytes(to, &target, &bytes);
    }
}

fn write_bytes(dir: &Path, path: &str, bytes: &[u8]) {
    let full = dir.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, bytes).unwrap();
}

/// Each file under `dir` with its path (relative, with `/`) and its bytes,
/// sorted by path. The folder `.git` is not in the list. Two equal trees give
/// two equal lists.
pub fn tree_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap();
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

// --- a pin file for `upgrade --pin-file` ---

static PINS: AtomicUsize = AtomicUsize::new(0);

fn pin_dir() -> PathBuf {
    let n = PINS.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("spec-dna-pinfile-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub const TARGETS: [&str; 7] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
    "aarch64-pc-windows-gnullvm",
];

/// A table `[spec-dna-sha256]` of a version, with the sum `digit` repeated.
pub fn sums_table(version: &str, digit: char) -> String {
    let mut t = String::from("[spec-dna-sha256]\n");
    for target in TARGETS {
        let ext = if target.contains("-windows-") {
            ".exe"
        } else {
            ""
        };
        t.push_str(&format!(
            "\"spec-dna-{version}-{target}{ext}\" = \"{}\"\n",
            digit.to_string().repeat(64)
        ));
    }
    t
}

/// Writes a file `spec-dna-pin.toml` outside the repository (a file inside
/// would make the tree dirty) and returns its path.
pub fn pin_file(version: &str, table: Option<&str>) -> (PathBuf, PathBuf) {
    let dir = pin_dir();
    let path = dir.join("spec-dna-pin.toml");
    let mut text = format!("spec-dna = \"{version}\"\n");
    if let Some(t) = table {
        text.push('\n');
        text.push_str(t);
    }
    fs::write(&path, text).unwrap();
    (dir, path)
}
