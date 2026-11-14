//! Tests of the shared git helpers in tests/common/mod.rs. Each test proves 1
//! helper, so a later test file can trust it.

mod common;

use common::*;

/// A repository with 1 commit on `main`.
fn base() -> Repo {
    let repo = Repo::new();
    repo.write("base.txt", "base\n");
    repo.commit("base");
    repo
}

#[test]
fn git_at_sets_the_committer_date() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    git_at(&repo.dir, &["add", "-A"], "2001-02-03T04:05:06+02:00");
    git_at(
        &repo.dir,
        &["commit", "-q", "-m", "old"],
        "2001-02-03T04:05:06+02:00",
    );
    let date = repo.git_out(&["log", "-1", "--format=%cI"]);
    assert_eq!(date, "2001-02-03T04:05:06+02:00");
    let author = repo.git_out(&["log", "-1", "--format=%aI"]);
    assert_eq!(author, "2001-02-03T04:05:06+02:00");
}

#[test]
fn commit_at_sets_the_date() {
    let repo = Repo::new();
    repo.write("a.txt", "a\n");
    repo.commit_at("old", "2005-06-07T08:09:10+02:00");
    assert_eq!(
        repo.git_out(&["log", "-1", "--format=%cI"]),
        "2005-06-07T08:09:10+02:00"
    );
}

#[test]
fn origin_head_resolves_without_a_remote() {
    let repo = base();
    repo.write("b.txt", "b\n");
    repo.commit("second");
    origin_head(&repo.dir, "HEAD~1");
    assert_eq!(
        repo.git_out(&["symbolic-ref", "refs/remotes/origin/HEAD"]),
        "refs/remotes/origin/main"
    );
    assert_eq!(
        repo.git_out(&["rev-parse", "origin/HEAD"]),
        repo.git_out(&["rev-parse", "HEAD~1"])
    );
    assert_eq!(repo.git_out(&["remote"]), "");
}

#[test]
fn shallow_clone_has_less_history() {
    let repo = base();
    for n in 0..3 {
        repo.write("f.txt", &format!("{n}\n"));
        repo.commit(&format!("c{n}"));
    }
    let dst = Repo::empty();
    shallow_clone(&repo.dir, &dst.dir, 1);
    assert_eq!(
        git_out(&dst.dir, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(git_out(&dst.dir, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(repo.git_out(&["rev-list", "--count", "HEAD"]), "4");
    assert_eq!(
        repo.git_out(&["rev-parse", "--is-shallow-repository"]),
        "false"
    );
}

/// The facts of a merge on `main`: the tree, the count of commits, the count
/// of first-parent commits and the count of merge commits.
fn shape(repo: &Repo) -> (String, usize, usize, usize) {
    let count = |args: &[&str]| -> usize {
        let mut full = vec!["rev-list", "--count"];
        full.extend_from_slice(args);
        repo.git_out(&full).parse().unwrap()
    };
    (
        repo.git_out(&["rev-parse", "main^{tree}"]),
        count(&["main"]),
        count(&["--first-parent", "main"]),
        count(&["--merges", "main"]),
    )
}

#[test]
fn merge_squash_adds_1_commit_and_no_merge() {
    let repo = base();
    merge_squash(&repo.dir);
    let (_, all, first, merges) = shape(&repo);
    // base, the commit of main, the squash commit: the 2 feature commits
    // do not reach main.
    assert_eq!((all, first, merges), (3, 3, 0));
    assert_eq!(
        repo.git_out(&["log", "-1", "--format=%s", "main"]),
        "squash feature"
    );
    assert_eq!(repo.git_out(&["branch", "--merged", "main"]), "* main");
}

#[test]
fn merge_commit_adds_1_merge() {
    let repo = base();
    merge_commit(&repo.dir);
    let (_, all, first, merges) = shape(&repo);
    // 1 base, 1 commit of main, 2 feature commits, 1 merge commit. The first
    // parent line skips the feature commits.
    assert_eq!((all, first, merges), (5, 3, 1));
}

#[test]
fn merge_rebase_is_linear() {
    let repo = base();
    merge_rebase(&repo.dir);
    let (_, all, first, merges) = shape(&repo);
    assert_eq!((all, first, merges), (4, 4, 0));
    // The branch has the same number of commits as main.
    assert_eq!(repo.git_out(&["rev-list", "--count", "feature"]), "4");
    assert_eq!(
        repo.git_out(&["rev-parse", "feature"]),
        repo.git_out(&["rev-parse", "main"])
    );
}

#[test]
fn the_3_merges_give_the_same_tree() {
    let trees: Vec<String> = [merge_squash, merge_commit, merge_rebase]
        .iter()
        .map(|merge| {
            let repo = base();
            merge(&repo.dir);
            assert_eq!(repo.git_out(&["branch", "--show-current"]), "main");
            let files = repo.git_out(&["ls-tree", "--name-only", "main"]);
            assert_eq!(
                files,
                "base.txt\nfeature-a.txt\nfeature-b.txt\nmain-side.txt"
            );
            shape(&repo).0
        })
        .collect();
    assert_eq!(trees[0], trees[1]);
    assert_eq!(trees[1], trees[2]);
}

#[test]
fn with_config_writes_the_pin_and_run_reads_it() {
    let repo = Repo::with_config("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    assert!(
        repo.read("spec-dna.toml")
            .starts_with(&format!("spec-dna = \"{VERSION}\"\n"))
    );
    let (code, _, _) = repo.run(&["--version"]);
    assert_eq!(code, 0);
}
