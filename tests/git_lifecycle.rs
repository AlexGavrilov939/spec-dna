//! Tests of the git access of the change lifecycle (step 3 of 0.6.0, slice S2):
//! `remote_head`, `commit_time`, `last_touch`, `file_log`,
//! `added_intents`, `deleted_intents` and `is_shallow`.
//!
//! The binary has no library, and `src/git.rs` uses only `std`. Thus this file
//! compiles that file as a module of its own and calls it. The git fixtures
//! come from `tests/common/mod.rs`. Each date is set by the test, so no test
//! reads a clock.

mod common;

#[allow(dead_code)]
#[path = "../src/git.rs"]
mod spec_git;

use common::*;
use spec_git::{DeletedIntent, Git, PathChange};
use std::collections::HashMap;

const INTENT: &str = "docs/intent/slug/intent.md";
const DIR: &str = "docs/intent";

// 2024-01-01, 2024-03-01 and 2024-06-01 at 00:00:00 UTC.
const T1: i64 = 1_704_067_200;
const T2: i64 = 1_709_251_200;
const T3: i64 = 1_717_200_000;
const D1: &str = "2024-01-01T00:00:00+00:00";
const D2: &str = "2024-03-01T00:00:00+00:00";
const D3: &str = "2024-06-01T00:00:00+00:00";

fn git_of(r: &Repo) -> Git {
    Git::new(&r.dir)
}

fn specs(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// A repository whose `main` has an intent folder and a second file.
fn with_intent() -> Repo {
    let r = Repo::new();
    r.write(INTENT, "---\nstatus: accepted\n---\n# Slug\n");
    r.write("docs/intent/slug/plan.md", "# Plan\n");
    r.write("src/code.txt", "code\n");
    r.commit("start");
    r
}

/// Removes the intent file, so that the next commit of the merge helper holds
/// the deletion. The helper stages all files before it commits.
fn delete_intent_on_the_branch(r: &Repo) {
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
}

fn parents(r: &Repo, commit: &str) -> Vec<String> {
    r.git_out(&["rev-list", "--parents", "-n", "1", commit])
        .split(' ')
        .skip(1)
        .map(str::to_string)
        .collect()
}

#[test]
fn remote_head_reads_only_origin_head() {
    let r = with_intent();
    let git = git_of(&r);
    // The repository has a local `main`, and no `origin/HEAD`. There is no
    // fallback to the local branch.
    assert_eq!(git.remote_head(), None);
    let sha = r.git_out(&["rev-parse", "HEAD"]);
    origin_head(&r.dir, &sha);
    assert_eq!(git.remote_head(), Some(sha.clone()));
    // A remote branch with no `HEAD` ref is not enough.
    let r2 = with_intent();
    r2.git(&["update-ref", "refs/remotes/origin/main", &sha_of(&r2)]);
    assert_eq!(git_of(&r2).remote_head(), None);
}

fn sha_of(r: &Repo) -> String {
    r.git_out(&["rev-parse", "HEAD"])
}

#[test]
fn merge_base_and_commit_time() {
    let r = with_intent();
    let first = sha_of(&r);
    r.write("a.txt", "a\n");
    r.commit_at("second", D2);
    let second = sha_of(&r);
    r.git(&["checkout", "-q", "-b", "side", &first]);
    r.write("b.txt", "b\n");
    r.commit_at("side", D3);
    let side = sha_of(&r);
    let git = git_of(&r);
    assert_eq!(git.merge_base(&second, &side), Some(first.clone()));
    assert_eq!(git.commit_time(&second), Ok(T2));
    assert_eq!(git.commit_time(&side), Ok(T3));
    assert!(git.commit_time("no-such-rev").is_err());
    assert!(!git.is_shallow());
}

/// 3 commits: the files of `a/` and `b/` at T1, `a/x.txt` again at T2, and
/// `c.txt` at T3.
fn history() -> Repo {
    let r = Repo::new();
    r.write("a/x.txt", "1\n");
    r.write("a/deep/z.txt", "1\n");
    r.write("b/y.txt", "1\n");
    r.commit_at("one", D1);
    r.write("a/x.txt", "2\n");
    r.commit_at("two", D2);
    r.write("c.txt", "1\n");
    r.commit_at("three", D3);
    r
}

#[test]
fn last_touch_gives_the_newest_date_of_each_file() {
    let r = history();
    let git = git_of(&r);
    let got = git
        .last_touch("HEAD", &specs(&["a", "b/y.txt", "c.txt"]))
        .unwrap();
    let want: HashMap<String, i64> = [
        ("a/x.txt", T2),
        ("a/deep/z.txt", T1),
        ("b/y.txt", T1),
        ("c.txt", T3),
    ]
    .into_iter()
    .map(|(p, t)| (p.to_string(), t))
    .collect();
    assert_eq!(got, want);
    // A glob, and the revision `HEAD~1`.
    let got = git.last_touch("HEAD~1", &specs(&["**/*.txt"])).unwrap();
    assert_eq!(got.get("a/x.txt"), Some(&T2));
    assert_eq!(got.get("c.txt"), None);
    // A folder with a final slash covers the files below it.
    let got = git.last_touch("HEAD", &specs(&["b/"])).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got.get("b/y.txt"), Some(&T1));
    // No spec, an invalid spec and an absent path give nothing.
    assert!(git.last_touch("HEAD", &[]).unwrap().is_empty());
    assert!(
        git.last_touch("HEAD", &specs(&["../up"]))
            .unwrap()
            .is_empty()
    );
    assert!(
        git.last_touch("HEAD", &specs(&["absent"]))
            .unwrap()
            .is_empty()
    );
    // A bad revision is an error.
    assert!(git.last_touch("no-such-rev", &specs(&["a"])).is_err());
    // A deletion counts as a touch.
    r.git(&["rm", "-q", "b/y.txt"]);
    r.commit_at("four", "2024-07-01T00:00:00+00:00");
    let got = git.last_touch("HEAD", &specs(&["b"])).unwrap();
    assert_eq!(got.get("b/y.txt"), Some(&1_719_792_000));
}

#[test]
fn last_touch_reads_brackets_in_a_path_as_plain_characters() {
    // A path of a web framework has brackets. The globs of the configuration
    // have no character class, so `[id]` must not match `i`.
    let r = Repo::new();
    r.write("app/[id]/page.txt", "1\n");
    r.write("app/i/page.txt", "1\n");
    r.commit_at("one", D1);
    r.write("app/[id]/page.txt", "2\n");
    r.commit_at("two", D2);
    let git = git_of(&r);
    let got = git.last_touch("HEAD", &specs(&["app/[id]/**"])).unwrap();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got.get("app/[id]/page.txt"), Some(&T2));
    let got = git.last_touch("HEAD", &specs(&["app/[id]"])).unwrap();
    assert_eq!(got.get("app/[id]/page.txt"), Some(&T2));
    assert_eq!(got.len(), 1, "{got:?}");
}

#[test]
fn last_touch_uses_the_newest_date_not_the_first_commit() {
    // The committer dates of a history are not always in order: a commit can
    // be older than its parent.
    let r = Repo::new();
    r.write("a.txt", "1\n");
    r.commit_at("one", D3);
    r.write("a.txt", "2\n");
    r.commit_at("two", D1);
    let git = git_of(&r);
    let got = git.last_touch("HEAD", &specs(&["a.txt"])).unwrap();
    assert_eq!(got.get("a.txt"), Some(&T3));
}

#[test]
fn last_touch_in_a_shallow_clone_never_gives_an_older_date() {
    let r = history();
    let dst = Repo::empty();
    shallow_clone(&r.dir, &dst.dir.join("clone"), 1);
    let full = git_of(&r);
    let shallow = Git::new(&dst.dir.join("clone"));
    assert!(shallow.is_shallow());
    assert!(!full.is_shallow());
    let list = specs(&["a", "b", "c.txt"]);
    let full_map = full.last_touch("HEAD", &list).unwrap();
    let shallow_map = shallow.last_touch("HEAD", &list).unwrap();
    // The boundary commit shows each file as added, so the clone knows each
    // file and each of its dates is the same or later. It hides the age of
    // `b/y.txt` (T1 in the full clone), but it never makes a file older.
    assert_eq!(full_map.len(), shallow_map.len());
    for (path, t_full) in &full_map {
        let t_shallow = shallow_map[path];
        assert!(t_shallow >= *t_full, "{path}: {t_shallow} < {t_full}");
    }
    assert_eq!(full_map["b/y.txt"], T1);
    assert_eq!(shallow_map["b/y.txt"], T3);
    assert_eq!(full_map["a/x.txt"], T2);
    assert_eq!(shallow_map["a/x.txt"], T3);
    // A clone with 2 commits is nearer to the full history.
    let dst2 = Repo::empty();
    shallow_clone(&r.dir, &dst2.dir.join("clone"), 2);
    let map2 = Git::new(&dst2.dir.join("clone"))
        .last_touch("HEAD", &list)
        .unwrap();
    assert_eq!(map2["a/x.txt"], T2);
    assert_eq!(map2["b/y.txt"], T2);
}

#[test]
fn file_log_lists_the_commits_of_a_file_newest_first() {
    let r = history();
    let git = git_of(&r);
    let log = git.file_log("HEAD", "a/x.txt").unwrap();
    assert_eq!(log.iter().map(|(_, t)| *t).collect::<Vec<_>>(), [T2, T1]);
    assert_eq!(log[0].0, r.git_out(&["rev-parse", "HEAD~1"]));
    assert!(git.file_log("HEAD", "absent.txt").unwrap().is_empty());
}

#[test]
fn diff_paths_gives_the_kind_of_each_change() {
    let r = history();
    let base = sha_of(&r);
    r.write("a/x.txt", "3\n");
    r.git(&["rm", "-q", "b/y.txt"]);
    r.write("new.txt", "n\n");
    r.commit("change");
    let mut got = git_of(&r).diff_paths(&base, false).unwrap();
    got.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        got,
        vec![
            ("a/x.txt".to_string(), PathChange::Modified),
            ("b/y.txt".to_string(), PathChange::Deleted),
            ("new.txt".to_string(), PathChange::Added),
        ]
    );
    // An untracked file is an addition, only if the caller asks for it.
    r.write("loose.txt", "l\n");
    assert_eq!(git_of(&r).diff_paths("HEAD", false).unwrap().len(), 0);
    assert_eq!(
        git_of(&r).diff_paths("HEAD", true).unwrap(),
        vec![("loose.txt".to_string(), PathChange::Added)]
    );
    // `changed_since` keeps its meaning.
    assert_eq!(
        git_of(&r).changed_since("HEAD", true).unwrap(),
        vec![("loose.txt".to_string(), true)]
    );
}

fn check_one_deletion(r: &Repo, entries: &[DeletedIntent]) {
    assert_eq!(entries.len(), 1, "{entries:?}");
    let e = &entries[0];
    assert_eq!(e.path, INTENT);
    // The entry is a commit with 1 parent, and the parent has the file.
    let ps = parents(r, &e.commit);
    assert_eq!(ps.len(), 1, "{ps:?}");
    assert_eq!(e.parent.as_deref(), Some(ps[0].as_str()));
    let status = r.git_out(&["show", &format!("{}:{INTENT}", ps[0])]);
    assert!(status.contains("status: accepted"), "{status}");
    // The commit does not have the file.
    assert!(
        r.git_out(&["ls-tree", "-r", "--name-only", &e.commit])
            .lines()
            .all(|l| l != INTENT)
    );
    let ct: i64 = r
        .git_out(&["log", "-1", "--format=%ct", &e.commit])
        .parse()
        .unwrap();
    assert_eq!(e.time, ct);
}

#[test]
fn deleted_intents_gives_one_entry_for_a_squash_merge() {
    let r = with_intent();
    let before = sha_of(&r);
    delete_intent_on_the_branch(&r);
    merge_squash(&r.dir);
    let git = git_of(&r);
    // The branch `feature` still has its deletion commit, and the walk of
    // `HEAD` must not count it.
    let entries = git.deleted_intents(DIR, None).unwrap();
    check_one_deletion(&r, &entries);
    assert_eq!(parents(&r, &entries[0].commit).len(), 1);
    assert_eq!(
        r.git_out(&["log", "-1", "--format=%s", &entries[0].commit]),
        "squash feature"
    );
    assert_eq!(git.deleted_intents(DIR, Some(&before)).unwrap().len(), 1);
    assert!(git.deleted_intents(DIR, Some("HEAD")).unwrap().is_empty());
}

#[test]
fn deleted_intents_gives_one_entry_for_a_merge_commit() {
    let r = with_intent();
    delete_intent_on_the_branch(&r);
    merge_commit(&r.dir);
    // `HEAD` is a merge commit with 2 parents.
    assert_eq!(parents(&r, "HEAD").len(), 2);
    let entries = git_of(&r).deleted_intents(DIR, None).unwrap();
    check_one_deletion(&r, &entries);
    // The entry is the commit of the branch, not the merge.
    assert_ne!(entries[0].commit, sha_of(&r));
    assert_eq!(
        r.git_out(&["log", "-1", "--format=%s", &entries[0].commit]),
        "feature a"
    );
}

#[test]
fn deleted_intents_gives_one_entry_for_a_rebase_merge() {
    let r = with_intent();
    delete_intent_on_the_branch(&r);
    merge_rebase(&r.dir);
    assert_eq!(parents(&r, "HEAD").len(), 1);
    let entries = git_of(&r).deleted_intents(DIR, None).unwrap();
    check_one_deletion(&r, &entries);
    // The history is linear, and the parent of the entry has the status.
    assert_eq!(
        r.git_out(&["log", "-1", "--format=%s", &entries[0].commit]),
        "feature a"
    );
}

#[test]
fn deleted_intents_skips_a_move_a_deeper_file_and_a_plan() {
    let r = with_intent();
    r.write(
        "docs/intent/slug/sub/intent.md",
        "---\nstatus: draft\n---\n# Sub\n",
    );
    r.commit("a file deeper in the folder");
    let git = git_of(&r);
    assert!(git.deleted_intents(DIR, None).unwrap().is_empty());
    // A move is not a deletion, and the other files of the folder do not
    // count.
    r.git(&["mv", "docs/intent/slug", "docs/intent/other"]);
    r.commit("move");
    assert!(git.deleted_intents(DIR, None).unwrap().is_empty());
    std::fs::remove_file(r.dir.join("docs/intent/other/plan.md")).unwrap();
    std::fs::remove_file(r.dir.join("docs/intent/other/sub/intent.md")).unwrap();
    r.commit("delete a plan and a deeper file");
    assert!(git.deleted_intents(DIR, None).unwrap().is_empty());
    // The `intent.md` of the moved folder is a deletion.
    std::fs::remove_file(r.dir.join("docs/intent/other/intent.md")).unwrap();
    r.commit("finish");
    let entries = git.deleted_intents(DIR, None).unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].path, "docs/intent/other/intent.md");
    // Another folder gives nothing.
    assert!(
        git.deleted_intents("other/intent", None)
            .unwrap()
            .is_empty()
    );
}

const LONG: &str = "# Slug\n\n## Intent\n\nThe controller must hold the valve open for 4 seconds.\nThe operator needs the time to read the pressure.\n\n## Context\n\nThe old controller closed the valve at once.\nThe pumps stopped, and the line lost its pressure.\n";

/// The ids that `moved_out` reads, from a repository whose `main` holds the
/// intent `slug` with a long body, so that git can pair a move.
fn with_long_intent() -> Repo {
    let r = Repo::new();
    r.write(INTENT, &format!("---\nstatus: accepted\n---\n{LONG}"));
    r.commit("start");
    r
}

/// The result of `moved_out` for the deletion that the last commit made.
fn moved_out_of_head(r: &Repo) -> Vec<bool> {
    let git = git_of(r);
    let entries = git.deleted_intents(DIR, None).unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    git.moved_out(DIR, &entries).unwrap()
}

#[test]
fn moved_out_needs_a_rename_and_the_old_id_as_an_alias() {
    // A move of the file with the old id as an alias is a move out.
    let r = with_long_intent();
    r.write(
        "docs/roadmap.md",
        &format!("---\naliases:\n  - intent/slug/intent\n---\n{LONG}"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("move with an alias");
    assert_eq!(moved_out_of_head(&r), [true]);

    // The same move with an inline list and a quoted id.
    let r = with_long_intent();
    r.write(
        "docs/roadmap.md",
        &format!("---\naliases: [other, \"intent/slug/intent\"]\n---\n{LONG}"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("move with an inline alias");
    assert_eq!(moved_out_of_head(&r), [true]);

    // Condition 2 fails: a plain `git mv` writes no alias.
    let r = with_long_intent();
    r.git(&["mv", INTENT, "docs/roadmap.md"]);
    r.commit("plain move");
    assert_eq!(moved_out_of_head(&r), [false]);

    // Condition 2 fails: the alias is another id.
    let r = with_long_intent();
    r.write(
        "docs/roadmap.md",
        &format!("---\naliases:\n  - intent/other/intent\n---\n{LONG}"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("move with another alias");
    assert_eq!(moved_out_of_head(&r), [false]);

    // Condition 1 fails: a plain deletion, and a file elsewhere that names
    // the id but has another text.
    let r = with_long_intent();
    r.write(
        "docs/other.md",
        "---\naliases:\n  - intent/slug/intent\n---\n# Other\n\nA text that has no line of the intent.\n",
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("delete the intent");
    assert_eq!(moved_out_of_head(&r), [false]);

    // D10: `finish` deletes the intent, and the same commit adds a research
    // note with a text like the intent. Git pairs them as a rename. With no
    // alias, the deletion still ends the change.
    let r = with_long_intent();
    r.write(
        "docs/research/note.md",
        &format!("---\nstatus: draft\n---\n{LONG}\nA new line.\n"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("finish with a near copy");
    let out = r.git_out(&["diff-tree", "-r", "-M", "--name-status", "HEAD~1", "HEAD"]);
    assert!(out.starts_with('R'), "git must pair the files: {out}");
    assert_eq!(moved_out_of_head(&r), [false]);

    // D59: a near copy that names the old id as an alias reads as a move.
    let r = with_long_intent();
    r.write(
        "docs/research/note.md",
        &format!("---\naliases:\n  - intent/slug/intent\n---\n{LONG}\nA new line.\n"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("finish with a near copy that has the alias");
    assert_eq!(moved_out_of_head(&r), [true]);
}

#[test]
fn moved_out_uses_the_first_parent_of_a_merge_commit() {
    let r = with_long_intent();
    r.git(&["checkout", "-q", "-b", "side"]);
    r.write(
        "docs/roadmap.md",
        &format!("---\naliases:\n  - intent/slug/intent\n---\n{LONG}"),
    );
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("move with an alias");
    r.git(&["checkout", "-q", "main"]);
    r.write("main-side.txt", "main\n");
    r.commit("main side");
    r.git(&["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    assert_eq!(parents(&r, "HEAD").len(), 2);
    let git = git_of(&r);
    // The first-parent walk gives the merge commit, and its diff against the
    // first parent holds the rename.
    let entries = git.deleted_intents_first_parent(DIR).unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].commit, sha_of(&r));
    assert_eq!(git.moved_out(DIR, &entries).unwrap(), [true]);
}

#[test]
fn deleted_intents_gives_one_entry_for_each_deleted_file_of_a_commit() {
    let r = with_intent();
    r.write(
        "docs/intent/two/intent.md",
        "---\nstatus: closed\n---\n# Two\n",
    );
    r.commit("a second change");
    std::fs::remove_dir_all(r.dir.join("docs/intent")).unwrap();
    r.commit("delete both");
    let entries = git_of(&r).deleted_intents(DIR, None).unwrap();
    let mut paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    paths.sort();
    assert_eq!(
        paths,
        ["docs/intent/slug/intent.md", "docs/intent/two/intent.md"]
    );
    assert_eq!(entries[0].commit, entries[1].commit);
}

#[test]
fn added_intents_gives_the_folders_that_are_in_neither_tree() {
    let r = with_intent();
    let base = sha_of(&r);
    // A change that the branch adds and finishes: in neither tree.
    r.write(
        "docs/intent/born/intent.md",
        "---\nstatus: draft\n---\n# Born\n",
    );
    r.commit("add born");
    // A change that the branch adds and keeps: in the head tree.
    r.write(
        "docs/intent/kept/intent.md",
        "---\nstatus: draft\n---\n# Kept\n",
    );
    r.commit("add kept");
    // A change that the base has, and the branch deletes: in the base tree.
    std::fs::remove_file(r.dir.join(INTENT)).unwrap();
    r.commit("finish slug");
    // A change that the branch adds, deletes and adds again: in the head tree.
    r.write(
        "docs/intent/again/intent.md",
        "---\nstatus: draft\n---\n# Again\n",
    );
    r.commit("add again");
    std::fs::remove_file(r.dir.join("docs/intent/again/intent.md")).unwrap();
    r.commit("delete again");
    r.write(
        "docs/intent/again/intent.md",
        "---\nstatus: draft\n---\n# Again\n",
    );
    r.commit("add again once more");
    std::fs::remove_file(r.dir.join("docs/intent/born/intent.md")).unwrap();
    r.commit("finish born");
    let git = git_of(&r);
    assert_eq!(
        git.added_intents(&base, "HEAD", DIR).unwrap(),
        ["docs/intent/born"]
    );
    // From the tip, there is no commit in the range.
    assert!(git.added_intents("HEAD", "HEAD", DIR).unwrap().is_empty());
    // Another folder gives nothing.
    assert!(
        git.added_intents(&base, "HEAD", "elsewhere")
            .unwrap()
            .is_empty()
    );
    // A bad revision is an error.
    assert!(git.added_intents("no-such-rev", "HEAD", DIR).is_err());
}

#[test]
fn the_readers_do_not_write_the_index() {
    // The commands run with `GIT_OPTIONAL_LOCKS=0`, so a read never
    // refreshes the index.
    let r = with_intent();
    let before = std::fs::metadata(r.dir.join(".git/index"))
        .unwrap()
        .modified()
        .unwrap();
    let git = git_of(&r);
    let _ = git.diff_paths("HEAD", true).unwrap();
    let _ = git.last_touch("HEAD", &specs(&["docs"])).unwrap();
    let _ = git.deleted_intents(DIR, None).unwrap();
    let _ = git.added_intents("HEAD", "HEAD", DIR).unwrap();
    let after = std::fs::metadata(r.dir.join(".git/index"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after);
}

// Readers of slice S8: `deleted_intents_first_parent`, `artifact_touches`,
// `first_parent_paths` and `read_at`.

#[test]
fn the_first_parent_walk_gives_the_merge_commit_of_a_merge() {
    let r = with_intent();
    delete_intent_on_the_branch(&r);
    merge_commit(&r.dir);
    let git = git_of(&r);
    let all = git.deleted_intents(DIR, None).unwrap();
    let first = git.deleted_intents_first_parent(DIR).unwrap();
    assert_eq!(first.len(), 1, "{first:?}");
    // The first-parent entry is the merge commit, and the other walk gives the
    // commit of the branch.
    assert_eq!(first[0].commit, sha_of(&r));
    assert_ne!(first[0].commit, all[0].commit);
    assert_eq!(
        first[0].parent.as_deref(),
        Some(parents(&r, "HEAD")[0].as_str())
    );
    // A squash and a rebase merge give the same commit in both walks.
    for merge in [merge_squash, merge_rebase] {
        let r = with_intent();
        delete_intent_on_the_branch(&r);
        merge(&r.dir);
        let git = git_of(&r);
        let first = git.deleted_intents_first_parent(DIR).unwrap();
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first, git.deleted_intents(DIR, None).unwrap());
    }
}

#[test]
fn artifact_touches_lists_the_3_files_in_the_order_of_the_ancestry() {
    let r = with_intent();
    r.write("docs/intent/slug/spec.md", "# Spec\n");
    r.write("docs/intent/slug/notes.md", "# Notes\n");
    r.commit_at("spec", D2);
    std::fs::remove_file(r.dir.join("docs/intent/slug/plan.md")).unwrap();
    r.commit_at("drop the plan", D3);
    let touches = git_of(&r).artifact_touches(DIR).unwrap();
    let list: Vec<(&str, PathChange)> = touches.iter().map(|t| (t.path.as_str(), t.kind)).collect();
    // The newest first. `notes.md` is not an artifact.
    assert_eq!(list[0], ("docs/intent/slug/plan.md", PathChange::Deleted));
    assert_eq!(list.len(), 4, "{list:?}");
    assert_eq!(touches[0].time, T3);
    assert!(
        touches
            .iter()
            .any(|t| t.path.ends_with("spec.md") && t.kind == PathChange::Added && t.time == T2)
    );
    assert!(touches.iter().all(|t| !t.path.ends_with("notes.md")));
    assert!(touches.iter().all(|t| t.parents.len() <= 1));
    assert!(git_of(&r).artifact_touches("elsewhere").unwrap().is_empty());
}

#[test]
fn first_parent_paths_gives_the_diff_of_a_merge_against_its_first_parent() {
    let r = with_intent();
    delete_intent_on_the_branch(&r);
    merge_commit(&r.dir);
    let merge = sha_of(&r);
    let paths = git_of(&r)
        .first_parent_paths(std::slice::from_ref(&merge))
        .unwrap();
    let mut got = paths[&merge].clone();
    got.sort();
    assert_eq!(
        got,
        [
            "docs/intent/slug/intent.md",
            "feature-a.txt",
            "feature-b.txt"
        ]
    );
    assert_eq!(paths.len(), 1);
    // A commit that git does not have is an error, and no commit is no run.
    assert!(git_of(&r).first_parent_paths(&["0".repeat(40)]).is_err());
    assert!(git_of(&r).first_parent_paths(&[]).unwrap().is_empty());
}

#[test]
fn read_at_returns_one_item_for_each_spec_in_order() {
    let r = with_intent();
    let first = sha_of(&r);
    r.write("src/code.txt", "changed\n");
    r.commit("change");
    let second = sha_of(&r);
    let got = git_of(&r)
        .read_at(&[
            (second.clone(), "src/code.txt".to_string()),
            (first.clone(), "src/code.txt".to_string()),
            (first.clone(), "no/such file.md".to_string()),
            (first.clone(), "docs".to_string()),
            ("0".repeat(40), "src/code.txt".to_string()),
            (first, "docs/intent/slug/plan.md".to_string()),
        ])
        .unwrap();
    assert_eq!(got.len(), 6);
    assert_eq!(got[0].as_deref(), Some(&b"changed\n"[..]));
    assert_eq!(got[1].as_deref(), Some(&b"code\n"[..]));
    // An absent file, a folder and an unknown commit are `None`.
    assert_eq!((&got[2], &got[3], &got[4]), (&None, &None, &None));
    assert_eq!(got[5].as_deref(), Some(&b"# Plan\n"[..]));
    assert!(git_of(&r).read_at(&[]).unwrap().is_empty());
}

#[test]
fn artifact_touches_gives_a_rename_as_a_touch_of_the_new_path() {
    let r = with_intent();
    r.git(&["mv", "docs/intent/slug", "docs/intent/other"]);
    r.commit_at("rename", D2);
    let touches = git_of(&r).artifact_touches(DIR).unwrap();
    let renames: Vec<_> = touches
        .iter()
        .filter(|t| t.from.is_some())
        .map(|t| (t.path.as_str(), t.from.as_deref().unwrap(), t.kind))
        .collect();
    assert_eq!(renames.len(), 2, "{touches:?}");
    assert!(renames.contains(&(
        "docs/intent/other/intent.md",
        "docs/intent/slug/intent.md",
        PathChange::Modified
    )));
    assert!(renames.contains(&(
        "docs/intent/other/plan.md",
        "docs/intent/slug/plan.md",
        PathChange::Modified
    )));
    // No touch is a deletion: a rename is not an end.
    assert!(touches.iter().all(|t| t.kind != PathChange::Deleted));
}
