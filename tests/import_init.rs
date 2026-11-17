//! Tests of `spec-dna import` on a project that `spec-dna init` configured
//! (the documented path). The tree is the output of `init --non-interactive`
//! for the files of a tool. The import runs until it has nothing to change,
//! with a commit after each run, and `check` then gives no new error against
//! the commit of `init`.

mod common;

use common::import as fx;
use common::*;

/// The result of `import_until_done`: the commit before the last run that
/// wrote, and the output of that run.
struct Done {
    runs: usize,
    before_last: String,
    last_out: String,
}

/// Runs `import <source> <args>` and commits, until the output says that
/// nothing changes.
fn import_until_done(repo: &Repo, source: &str, args: &[&str]) -> Done {
    let mut done = Done {
        runs: 0,
        before_last: String::new(),
        last_out: String::new(),
    };
    for _ in 0..4 {
        let mut a = vec!["import", source];
        a.extend_from_slice(args);
        let before = repo.git_out(&["rev-parse", "HEAD"]);
        let (code, out, err) = repo.run(&a);
        assert_eq!(code, 0, "{out}{err}");
        if out.trim() == "spec-dna import: nothing to change" {
            return done;
        }
        done.runs += 1;
        done.before_last = before;
        done.last_out = out;
        repo.commit("import");
    }
    panic!("the import does not end");
}

/// `check` against a base gives no new error. The configuration is a change
/// of its own, so the base is the commit before the last run.
fn no_new_error(repo: &Repo, base: &str) {
    let (code, out, err) = repo.run(&["check", "--base", base]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("0 new errors"), "{out}");
}

#[test]
fn speckit_after_init_ends_in_2_runs_and_check_gives_no_new_error() {
    let (repo, _) = fx::speckit_after_init();
    let done = import_until_done(&repo, "speckit", &[]);
    // The first run adds the root for the types that no root holds.
    assert_eq!(done.runs, 2);
    assert!(repo.read("spec-dna.toml").contains("path = \"docs\""));
    assert!(repo.dir.join("docs/rules/constitution.md").exists());
    assert!(repo.dir.join("specs/photo-albums.md").exists());
    assert!(repo.dir.join("docs/intent/album-export/spec.md").exists());
    no_new_error(&repo, &done.before_last);
}

#[test]
fn openspec_after_init_ends_and_check_gives_no_new_error() {
    let (repo, init) = fx::openspec_after_init();
    let done = import_until_done(&repo, "openspec", &[]);
    assert_eq!(done.runs, 1);
    // A root of the type takes the spec, and the ids of the old place stay.
    assert!(repo.dir.join("openspec/specs/auth.md").exists());
    assert!(!repo.dir.join("openspec/requirements").exists());
    let (code, out, err) = repo.run(&["show", "auth#password-sign-in"]);
    assert_eq!(code, 0, "{out}{err}");
    no_new_error(&repo, &init);
}

#[test]
fn a_run_after_the_printed_git_rm_changes_nothing() {
    for (source, (repo, _)) in [
        ("speckit", fx::speckit_after_init()),
        ("openspec", fx::openspec_after_init()),
    ] {
        let done = import_until_done(&repo, source, &[]);
        // Run the command that the report prints, as it is.
        let line = done
            .last_out
            .lines()
            .find_map(|l| l.strip_prefix("to remove the sources: "))
            .unwrap_or_else(|| panic!("{source}: no line to remove\n{}", done.last_out));
        let args: Vec<&str> = line.split_whitespace().skip(2).collect();
        let mut git = vec!["rm", "-q"];
        git.extend(args);
        repo.git(&git);
        repo.commit("remove the sources");
        let (code, out, err) = repo.run(&["import", source]);
        assert_eq!(code, 0, "{source}: {out}{err}");
        assert_eq!(out.trim(), "spec-dna import: nothing to change", "{source}");
        // The files that stay are still there.
        let kept = if source == "speckit" {
            "specs/003-album-export/plan.md"
        } else {
            "openspec/config.yaml"
        };
        assert!(repo.dir.join(kept).exists(), "{kept}");
    }
}

#[test]
fn a_root_with_the_path_dot_takes_each_type() {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        &fx::config(false, true).replace("path = \"docs\"", "path = \".\""),
    );
    for (path, text) in fx::speckit_files() {
        if !path.starts_with("docs/") {
            repo.write(path, text);
        }
    }
    repo.commit("project");
    let done = import_until_done(&repo, "speckit", &[]);
    assert!(repo.dir.join("rules/constitution.md").exists());
    assert!(repo.dir.join("requirements/photo-albums.md").exists());
    assert!(repo.dir.join("intent/album-export/plan.md").exists());
    no_new_error(&repo, &done.before_last);
}
