//! The code tree (design 0.8.0, D1, items 1 to 3). One hash covers the code
//! as it was when a test ran. A test report binds to this hash by its stamp.
//!
//! The input is each entry of the git index. A tracked file that differs from
//! the index gives the mode and the object id of the file on disk. A tracked
//! file that is deleted on disk does not count, and an untracked file does not
//! count. On a clean checkout, the entries equal the output of
//! `git ls-tree -r HEAD`.
//!
//! The code paths exclude these paths: each path that `[evidence] tree-exclude`
//! matches, each path in a folder of the `intent` layer, each path under
//! `.spec-dna/`, each tool file (`init::is_tool_file`), and each report with its
//! stamp. Thus a change of documents, of a record or of `finish` never changes
//! the tree.
//!
//! A tracked file that differs from the index gives the mode and the object id
//! of what is on disk. A regular file gives the id of its content. A
//! symbolic link gives the id of its text (the path that it names) with the
//! mode `120000`. A submodule keeps the entry of the git index: the tree does
//! not read the files inside it. A path is bytes, so a name that is not UTF-8 stays whole.

use crate::config::Config;
use crate::git::Git;
use crate::glob;
use crate::hash::{self, Hash};
use crate::repo::Repo;
use std::collections::HashSet;

/// The first line of the hashed text. A change of the form needs a new number.
const HEADER: &str = "spec-dna-tree 1\n";

/// One code entry: the mode, the object id and the path as bytes.
pub type Entry = (String, String, Vec<u8>);

/// The suffix of a stamp file.
pub const STAMP_SUFFIX: &str = ".stamp";

/// True if `path` is a report file or its stamp: it matches a glob of
/// `[evidence] reports`, or it is such a file with the suffix of a stamp.
pub fn is_report_path(config: &Config, path: &str) -> bool {
    let is = |p: &str| config.evidence_reports.iter().any(|g| glob::matches(g, p));
    is(path) || path.strip_suffix(STAMP_SUFFIX).is_some_and(is)
}

/// True if `path` is a path of the code tree.
pub fn is_code_path(config: &Config, path: &str, intent_dirs: &[String]) -> bool {
    if path == ".spec-dna" || path.starts_with(".spec-dna/") {
        return false;
    }
    if intent_dirs.iter().any(|d| {
        path.strip_prefix(d.as_str())
            .is_some_and(|r| r.starts_with('/'))
    }) {
        return false;
    }
    if config
        .evidence_tree_exclude
        .iter()
        .any(|g| glob::matches(g, path))
    {
        return false;
    }
    !(crate::init::is_tool_file(config, path) || is_report_path(config, path))
}

/// Hashes the entries of the code tree. The entries need not be in order: the
/// function sorts them by the bytes of the path. The result is the first 8
/// bytes of SHA-256 as 16 hexadecimal digits, the form of `hash.rs`.
pub fn hash_entries(entries: &[Entry]) -> String {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.2.cmp(&b.2));
    let mut text: Vec<u8> = HEADER.as_bytes().to_vec();
    for (mode, oid, path) in sorted {
        text.extend_from_slice(format!("{mode} {oid}\t").as_bytes());
        text.extend_from_slice(path);
        text.push(b'\n');
    }
    let h: Hash = hash::hash_bytes(&text);
    hash::hex(&h)
}

/// The path of a file on disk from the bytes of its name in the index.
#[cfg(unix)]
fn disk_path(root: &std::path::Path, bytes: &[u8]) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    root.join(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn disk_path(root: &std::path::Path, bytes: &[u8]) -> std::path::PathBuf {
    root.join(String::from_utf8_lossy(bytes).as_ref())
}

/// The text of a symbolic link as bytes.
#[cfg(unix)]
fn link_text(target: &std::path::Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    target.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn link_text(target: &std::path::Path) -> Vec<u8> {
    target.to_string_lossy().replace('\\', "/").into_bytes()
}

/// Returns the entries of the code tree of the working tree.
pub fn entries(repo: &Repo, config: &Config) -> Result<Vec<Entry>, String> {
    entries_of(&repo.git, config)
}

/// Returns the entries of the code tree that a `Git` reads: the working tree, or
/// the git index for a `staged` `Git` (`check --staged`). The index gives the
/// mode and the object id of each code path as they are, so a file on disk that
/// differs from the index does not count.
pub fn entries_of(git: &Git, config: &Config) -> Result<Vec<Entry>, String> {
    let index = git.index_entries()?;
    let intent = crate::lifecycle::intent_dirs(config);
    if git.is_staged() {
        return Ok(index
            .into_iter()
            .filter(|(_, _, path)| is_code_path(config, &String::from_utf8_lossy(path), &intent))
            .collect());
    }
    let root = git.root();
    let (modified, deleted) = git.diff_files()?;
    let mut keep: Vec<Entry> = Vec::new();
    let mut rehash: Vec<usize> = Vec::new();
    let mut links: Vec<(usize, Vec<u8>)> = Vec::new();
    for (mode, oid, path) in index {
        // The diff lists a path as text. A name that is not UTF-8 has the same
        // text in both lists, and no glob matches the replaced bytes by chance.
        let text = String::from_utf8_lossy(&path);
        if !is_code_path(config, &text, &intent) || deleted.contains(text.as_ref()) {
            continue;
        }
        if modified.contains(text.as_ref()) && mode != "160000" {
            let on_disk = disk_path(root, &path);
            // What is on disk now decides, also when its kind changed. A path
            // that is neither a file nor a link (a folder) keeps the entry of
            // the index.
            match std::fs::symlink_metadata(&on_disk) {
                Ok(m) if m.file_type().is_symlink() => {
                    if let Ok(target) = std::fs::read_link(&on_disk) {
                        links.push((keep.len(), link_text(&target)));
                    }
                }
                Ok(m) if m.is_file() => rehash.push(keep.len()),
                _ => {}
            }
        }
        keep.push((mode, oid, path));
    }
    if !rehash.is_empty() {
        let paths: Vec<Vec<u8>> = rehash.iter().map(|&i| keep[i].2.clone()).collect();
        let ids = git.hash_files(&paths)?;
        for (&i, id) in rehash.iter().zip(ids) {
            keep[i].1 = id;
            if let Some(m) = disk_mode(&disk_path(root, &keep[i].2)) {
                keep[i].0 = m.to_string();
            }
        }
    }
    for (i, text) in links {
        keep[i].1 = git.hash_blob(text)?;
        keep[i].0 = "120000".to_string();
    }
    Ok(keep)
}

/// The git mode of a regular file on disk: `100755` with an execute bit,
/// else `100644`. `None` where the platform has no such bit.
#[cfg(unix)]
fn disk_mode(path: &std::path::Path) -> Option<&'static str> {
    use std::os::unix::fs::PermissionsExt;
    let m = std::fs::metadata(path).ok()?;
    Some(if m.permissions().mode() & 0o111 != 0 {
        "100755"
    } else {
        "100644"
    })
}

#[cfg(not(unix))]
fn disk_mode(_path: &std::path::Path) -> Option<&'static str> {
    None
}

/// The hash of the code tree of the working tree, and the paths of the code
/// tree (the tracked test files are among them).
pub fn compute(repo: &Repo, config: &Config) -> Result<(String, Vec<String>), String> {
    let entries = entries(repo, config)?;
    let paths: Vec<String> = entries
        .iter()
        .map(|e| String::from_utf8_lossy(&e.2).into_owned())
        .collect();
    Ok((hash_entries(&entries), paths))
}

/// The hash of the code tree of the commit `rev`: the hash over the code
/// entries of `git ls-tree -r <rev>`, with the rules of [`compute`]. A working
/// tree with no edit of code has the same hash as its `HEAD`.
pub fn hash_at(repo: &Repo, config: &Config, rev: &str) -> Result<String, String> {
    let intent = crate::lifecycle::intent_dirs(config);
    let paths: Vec<String> = repo
        .git
        .ls_tree(rev)?
        .into_iter()
        .map(|e| e.0)
        .filter(|p| is_code_path(config, p, &intent))
        .collect();
    Ok(hash_entries(&repo.git.tree_entries_at(rev, &paths)?))
}

/// The paths that git tracks, and that a report glob matches (a report or a
/// stamp that is in a commit or in the index).
pub fn tracked_reports(repo: &Repo, config: &Config) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    Ok(repo
        .git
        .index_entries()?
        .into_iter()
        .map(|e| String::from_utf8_lossy(&e.2).into_owned())
        .filter(|p| is_report_path(config, p) && seen.insert(p.clone()))
        .collect())
}

/// True if two tree hashes are equal. A hash is 16 hexadecimal digits. The
/// compare ignores the case of the digits. This is the second compare of the
/// pinned hashes (`change::hash_matches` compares a requirement).
pub fn matches(a: &str, b: &str) -> bool {
    a.len() == 16 && a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(mode: &str, oid: &str, path: &str) -> Entry {
        (mode.to_string(), oid.to_string(), path.as_bytes().to_vec())
    }

    #[test]
    fn the_hash_ignores_the_order_and_sees_each_field() {
        let a = vec![e("100644", "aa", "src/a.rs"), e("100755", "bb", "b.sh")];
        let b = vec![a[1].clone(), a[0].clone()];
        assert_eq!(hash_entries(&a), hash_entries(&b));
        assert_eq!(hash_entries(&a).len(), 16);
        let mode = vec![e("100755", "aa", "src/a.rs"), a[1].clone()];
        let oid = vec![e("100644", "ab", "src/a.rs"), a[1].clone()];
        let path = vec![e("100644", "aa", "src/c.rs"), a[1].clone()];
        for other in [mode, oid, path] {
            assert_ne!(hash_entries(&a), hash_entries(&other));
        }
    }

    #[test]
    fn the_order_is_the_order_of_the_bytes_of_the_paths() {
        // `-` (0x2d) sorts before `/` (0x2f) in bytes.
        let x = vec![e("100644", "1", "a-b"), e("100644", "2", "a/b")];
        let text = "spec-dna-tree 1\n100644 1\ta-b\n100644 2\ta/b\n";
        let h = hash::hex(&hash::hash_bytes(text.as_bytes()));
        assert_eq!(hash_entries(&x), h);
    }

    #[test]
    fn matches_compares_16_digits() {
        assert!(matches("3f2a9c1b7e0d4a11", "3F2A9C1B7E0D4A11"));
        assert!(!matches("3f2a9c1b7e0d4a11", "3f2a9c1b7e0d4a12"));
        assert!(!matches("3f2a", "3f2a"));
    }

    fn config(extra: &str) -> Config {
        Config::parse(&format!(
            "spec-dna = \"0.1.0\"\n{extra}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n"
        ))
        .unwrap()
    }

    #[test]
    fn the_code_paths_leave_out_documents_records_tools_and_reports() {
        let c = config("[evidence]\nreports = [\"out/*.xml\"]\n");
        let intent = crate::lifecycle::intent_dirs(&c);
        assert_eq!(intent, ["docs/intent"]);
        let code = |p: &str| is_code_path(&c, p, &intent);
        assert!(code("src/main.rs"));
        assert!(code("tests/test_a.py"));
        assert!(!code("README.md"));
        assert!(!code("docs/guide.markdown"));
        assert!(!code("docs/intent/dose/plan.md"));
        assert!(!code("docs/intent/dose/notes.txt"));
        assert!(code("docs/intentional.txt"));
        assert!(!code(".spec-dna/accepted/x.toml"));
        assert!(!code("spec-dna.toml"));
        assert!(!code("out/junit.xml"));
        assert!(!code("out/junit.xml.stamp"));
        assert!(code("other/junit.xml"));
        assert!(code("out/junit.xml.bak"));
    }

    #[test]
    fn a_list_of_tree_exclude_replaces_the_default() {
        let c = config("[evidence]\ntree-exclude = [\"**/*.txt\"]\n");
        let code = |p: &str| is_code_path(&c, p, &[]);
        assert!(code("README.md"));
        assert!(!code("notes.txt"));
    }
}
