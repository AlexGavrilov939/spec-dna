//! Tests of the shared helpers in tests/common/ (the git helpers and
//! `md_lint`). Each test proves 1 helper, so a later test file can trust it.

mod common;

use common::*;
#[cfg(unix)]
use std::fs;

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

#[test]
fn run_env_sets_and_removes_variables() {
    // `SPEC_DNA_TRACE` makes the binary write `trace` lines to stderr.
    let repo = Repo::with_config("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    repo.write("docs/a.md", "# A\n");
    repo.commit("config");
    let has_trace = |set: &[(&str, &str)], remove: &[&str]| {
        let (_, _, err) = repo.run_env(&["check"], set, remove);
        err.contains("trace ")
    };
    assert!(!has_trace(&[], &[]));
    assert!(has_trace(&[("SPEC_DNA_TRACE", "1")], &[]));
    // A name in both lists is removed.
    assert!(!has_trace(&[("SPEC_DNA_TRACE", "1")], &["SPEC_DNA_TRACE"]));
}

#[test]
fn run_without_env_keeps_the_exit_code_and_output() {
    let repo = Repo::with_config("");
    let (code, out, _) = repo.run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("spec-dna {VERSION}\n"));
}

#[cfg(unix)]
#[test]
fn stub_bin_writes_an_executable_script() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Repo::empty();
    let bin = dir.dir.join("nested").join("bin");
    let path = stub_bin(&bin, "claude", "#!/bin/sh\necho \"$1\"\n");
    assert_eq!(path, bin.join("claude"));
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    let out = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "--version\n");
}

#[cfg(unix)]
#[test]
fn path_with_puts_the_stub_first() {
    let dir = Repo::empty();
    let bin = dir.dir.join("bin");
    stub_bin(&bin, "spec-dna-stub-probe", "#!/bin/sh\necho stub\n");
    let path = path_with(&bin);
    let first = std::env::split_paths(&path).next().unwrap();
    assert_eq!(first, bin);
    let out = std::process::Command::new("spec-dna-stub-probe")
        .env("PATH", &path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "stub\n");
    // The real PATH stays after the stub folder.
    let old = std::env::var_os("PATH").unwrap_or_default();
    assert_eq!(
        std::env::split_paths(&path).skip(1).collect::<Vec<_>>(),
        std::env::split_paths(&old).collect::<Vec<_>>()
    );
}

#[test]
fn normalize_marker_replaces_the_version_and_keeps_the_hash() {
    let text = "# Notes\n\n<!-- spec-dna harness v0.0.1 h:abc123 -->\n";
    assert_eq!(
        normalize_marker(text),
        "# Notes\n\n<!-- spec-dna harness v{VERSION} h:abc123 -->\n"
    );
    // An inline marker (the line of CLAUDE.md) and 2 markers in 1 text.
    let two = "line <!-- spec-dna harness v1.2.3 h:aa -->\nx\n<!-- spec-dna harness v9.9.9-rc.1 h:bb -->\n";
    assert_eq!(
        normalize_marker(two),
        "line <!-- spec-dna harness v{VERSION} h:aa -->\nx\n<!-- spec-dna harness v{VERSION} h:bb -->\n"
    );
    // The current version gives the same result as an old one.
    let now = format!("<!-- spec-dna harness v{VERSION} h:cc -->");
    assert_eq!(
        normalize_marker(&now),
        "<!-- spec-dna harness v{VERSION} h:cc -->"
    );
}

#[test]
fn normalize_marker_leaves_other_text_alone() {
    let prose = "The line ends with `<!-- spec-dna harness v<version> h:<hash> -->`.\n";
    // A text with no marker, or a broken marker, stays as it is. A
    // placeholder in prose has the shape of a marker, so it changes too.
    assert_eq!(normalize_marker("no marker\n"), "no marker\n");
    assert_eq!(normalize_marker(""), "");
    let broken = "<!-- spec-dna harness v0.1.0 -->\n<!-- spec-dna harness v\n";
    assert_eq!(normalize_marker(broken), broken);
    assert_eq!(
        normalize_marker(prose),
        "The line ends with `<!-- spec-dna harness v{VERSION} h:<hash> -->`.\n"
    );
}

#[test]
fn from_fixture_commits_the_tree_with_the_fixed_date() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    assert_eq!(repo.git_out(&["rev-list", "--count", "HEAD"]), "1");
    // Git writes a zero offset as `Z`.
    assert_eq!(FIXTURE_DATE, "2026-01-15T12:00:00+00:00");
    assert_eq!(
        repo.git_out(&["log", "-1", "--format=%cI"]),
        "2026-01-15T12:00:00Z"
    );
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
    // A dotfile of the fixture is in the commit.
    assert!(
        repo.git_out(&["ls-files"])
            .contains(".claude/settings.json")
    );
}

#[test]
fn tree_bytes_is_sorted_and_skips_the_git_folder() {
    let one = Repo::from_fixture("v04", &[]);
    let two = Repo::from_fixture("v04", &[]);
    let list = tree_bytes(&one.dir);
    assert_eq!(list, tree_bytes(&two.dir));
    assert!(list.windows(2).all(|w| w[0].0 < w[1].0));
    assert!(list.iter().all(|(path, _)| !path.starts_with(".git/")));
    assert!(list.iter().any(|(path, _)| path == "spec-dna.toml"));
    two.write("docs/new.md", "# New\n");
    assert_ne!(list, tree_bytes(&two.dir));
}

// --- md_lint ---
// Each case below is a small text. The expected lines are the ones that
// `markdownlint-cli2` v0.24.0 gives for the same text and the rules of
// `.markdownlint-cli2.yaml`.

#[test]
fn md_lint_passes_a_clean_text() {
    let text = "---\nname: x\n---\n\n<!-- begin -->\n# Title\n<!-- note -->\n\nText.  \nMore.\n\n- a\n  - b\n- c\n\n```sh\nx\n```\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n<!-- end -->\n";
    assert_eq!(md_lint(text), []);
    assert_eq!(md_lint(""), []);
}

#[test]
fn md_lint_reports_trailing_spaces_and_hard_tabs() {
    // 1 space and 3 spaces are problems. 2 spaces are a line break.
    assert_eq!(
        md_lint("# T\n\na \nb   \nc  \nd\n"),
        [(3, "MD009"), (4, "MD009")]
    );
    assert_eq!(md_lint("# T\n\na\tb\n"), [(3, "MD010")]);
    // A tab in code is a problem. Trailing spaces in code are not.
    assert_eq!(md_lint("# T\n\n```sh\n\tx   \n```\n"), [(4, "MD010")]);
    // The text of a comment and the front matter have no check.
    assert_eq!(md_lint("---\na:   \n---\n\n<!--\nb  \t\n-->\n"), []);
}

#[test]
fn md_lint_reports_a_second_blank_line() {
    assert_eq!(md_lint("# T\n\n\ntext\n"), [(3, "MD012")]);
    assert_eq!(md_lint("# T\n\ntext\n\n"), [(5, "MD012")]);
    // The blank lines in code and after the front matter are free.
    assert_eq!(md_lint("# T\n\n```sh\na\n\n\nb\n```\n"), []);
    assert_eq!(md_lint("---\na: b\n---\n\n\n# T\n"), []);
    // A comment line is not a blank line for this rule.
    assert_eq!(md_lint("# T\n\n<!-- c -->\n\ntext\n"), []);
}

#[test]
fn md_lint_reports_a_heading_with_no_blank_line() {
    assert_eq!(md_lint("# T\ntext\n"), [(1, "MD022")]);
    assert_eq!(md_lint("# T\n\ntext\n## H\n\ntext\n"), [(4, "MD022")]);
    // A heading at the start of the text or of the last line needs only 1 side.
    assert_eq!(md_lint("# T\n\ntext\n\n## H\n"), []);
    assert_eq!(md_lint("---\na: b\n---\n# T\n\ntext\n"), []);
    // A line with `#` and no space, and a line in code, are not headings.
    assert_eq!(md_lint("# T\n\n#tag\ntext\n\n```sh\n# c\n```\n"), []);
}

#[test]
fn md_lint_reads_a_comment_line_as_a_blank_line() {
    // The marker lines of a generated file are such comments.
    let text = "# T\n\n<!-- spec-dna:begin skill v1 h:00 -->\n# Skill\n\n1. a\n2. b\n<!-- spec-dna:end skill -->\n";
    assert_eq!(md_lint(text), []);
    // A comment on many lines is blank too.
    assert_eq!(md_lint("# T\n\n- a\n<!--\nnote\n-->\n\ntext\n"), []);
    // A line with text and a comment is text.
    assert_eq!(md_lint("# T\n\ntext <!-- c -->\n- a\n"), [(4, "MD032")]);
}

#[test]
fn md_lint_reports_a_fence_with_no_blank_line() {
    assert_eq!(
        md_lint("# T\n\ntext\n```sh\nx\n```\nmore\n"),
        [(4, "MD031"), (6, "MD031")]
    );
    assert_eq!(
        md_lint("# T\n\n```sh\na\n```\n```sh\nb\n```\n"),
        [(5, "MD031"), (6, "MD031")]
    );
    // A fence in a list item is a fence too.
    assert_eq!(
        md_lint("# T\n\n- a\n  ```sh\n  x\n  ```\n- b\n"),
        [(4, "MD031"), (6, "MD031")]
    );
    // A longer fence holds a shorter one. A tilde fence is a fence.
    assert_eq!(md_lint("# T\n\n````md\n```\ninner\n```\n````\n"), []);
}

#[test]
fn md_lint_reports_a_fence_with_no_language() {
    assert_eq!(md_lint("# T\n\n```\nx\n```\n"), [(3, "MD040")]);
    assert_eq!(md_lint("# T\n\n~~~\nx\n~~~\n"), [(3, "MD040")]);
    assert_eq!(md_lint("# T\n\n```text\nx\n```\n"), []);
    // The line that closes a fence has no language, and that is right.
    assert_eq!(md_lint("# T\n\n```sh\n```\n\n```sh\n\n```\n"), []);
}

#[test]
fn md_lint_reports_a_list_with_no_blank_line() {
    // Above: the list starts right after text. The problem is on the first line.
    assert_eq!(
        md_lint("# T\n\nAnswer:\n- a\n- b\n\ntext\n"),
        [(4, "MD032")]
    );
    // Below: another block ends the list. The problem is on the last line.
    assert_eq!(
        md_lint("# T\n\n- a\n- b\n## H\n\ntext\n"),
        [(4, "MD032"), (5, "MD022")]
    );
    assert_eq!(md_lint("# T\n\n1. a\n2. b\n---\n\ntext\n"), [(4, "MD032")]);
    assert_eq!(
        md_lint("# T\n\n- a\n```sh\nx\n```\n"),
        [(3, "MD032"), (4, "MD031")]
    );
    // A text line right after an item is part of the item.
    assert_eq!(md_lint("# T\n\n- a\ntext\n\nmore\n"), []);
    // Another marker at the same level starts another list.
    assert_eq!(
        md_lint("# T\n\n+ a\ntext\n- c\n"),
        [(4, "MD032"), (5, "MD032")]
    );
}

#[test]
fn md_lint_reads_nested_lists_and_item_text() {
    let text = "# T\n\n1. a\n   - n\n   - m\n2. b\n\n   text of b\n\n   - n\n3. c\n\ntext\n";
    assert_eq!(md_lint(text), []);
    // A number other than 1 does not interrupt a paragraph.
    assert_eq!(md_lint("# T\n\ntext\n2. x\n\nmore\n"), []);
    assert_eq!(md_lint("# T\n\ntext\n1. x\n"), [(4, "MD032")]);
    // A `-` line, `*emphasis*` and a rule are not items.
    assert_eq!(md_lint("# T\n\n*emphasis* text\n\n---\n\n- a\n\n***\n"), []);
    // The list inside a fence is not a list.
    assert_eq!(md_lint("# T\n\n```sh\n- a\ntext\n- b\n```\n"), []);
}

#[test]
fn md_lint_reports_a_table_with_no_blank_line() {
    let table = "| a | b |\n| - | - |\n| 1 | 2 |\n";
    assert_eq!(md_lint(&format!("# T\n\ntext\n{table}")), [(4, "MD058")]);
    assert_eq!(
        md_lint(&format!("# T\n\n{table}## H\n\ntext\n")),
        [(5, "MD058"), (6, "MD022")]
    );
    assert_eq!(
        md_lint(&format!("# T\n\n{table}- a\n")),
        [(5, "MD058"), (6, "MD032")]
    );
    assert_eq!(md_lint(&format!("# T\n\n{table}<!-- c -->\n")), []);
    // A line with `|` and no delimiter row is no table.
    assert_eq!(md_lint("# T\n\ntext\na | b\nc | d\n"), []);
}

#[test]
fn md_lint_reports_a_text_that_does_not_end_with_a_newline() {
    assert_eq!(md_lint("# T\n\ntext"), [(3, "MD047")]);
    assert_eq!(md_lint("# T\n\ntext\r\n"), []);
}

#[test]
fn md_lint_counts_the_front_matter_in_the_line_numbers() {
    assert_eq!(
        md_lint("---\na: b\n---\ntext\n# H\n\ntext\n"),
        [(5, "MD022")]
    );
    // The front matter needs a closing line. Else its first line is a rule.
    assert_eq!(md_lint("---\na: b\n\n# H\n\ntext   \n"), [(6, "MD009")]);
}
