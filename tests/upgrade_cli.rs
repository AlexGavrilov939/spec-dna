//! End-to-end tests of `spec-dna upgrade` (step 5, slice 10, D136 and D150 to
//! D153). Each case uses real git and a committed fixture of
//! `tests/fixtures/upgrade/`. `upgrade` reads `HEAD`, so a case that runs it
//! twice commits between the runs.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::fs;

/// The fixtures with a project that pins 0.4.2 or 0.5.0, and their overlays.
const CASES: &[(&str, &str, &[&str])] = &[
    ("v04", "v04", &[]),
    ("v05", "v05", &[]),
    ("brownfield", "brownfield", &[]),
    ("harness-lean", "v05", &["harness-lean"]),
    ("harness-full", "v05", &["harness-full"]),
    ("harness-edited", "v05", &["harness-lean", "harness-edited"]),
];

/// The errors of a `check --all --json` output: the code of each error and
/// the number of times that it occurs.
fn errors_of_check(json: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for part in json.split("{\"code\":\"").skip(1) {
        let code = &part[..part.find('"').unwrap()];
        let obj = &part[..part.find("\"fix\"").unwrap_or(part.len())];
        if obj.contains("\"severity\":\"error\"") {
            *out.entry(code.to_string()).or_insert(0) += 1;
        }
    }
    out
}

/// The `errors` of the report of `upgrade --json`.
fn errors_of_report(json: &str) -> BTreeMap<String, usize> {
    let at = json.find("\"errors\":[").expect("the report has errors") + "\"errors\":[".len();
    let list = &json[at..at + json[at..].find(']').unwrap()];
    let mut out = BTreeMap::new();
    for part in list.split("{\"code\":\"").skip(1) {
        let code = &part[..part.find('"').unwrap()];
        let n = part
            .split("\"count\":")
            .nth(1)
            .unwrap()
            .split(['}', ','])
            .next()
            .unwrap();
        out.insert(code.to_string(), n.parse().unwrap());
    }
    out
}

/// The number after `"key":` in a JSON text.
fn number(json: &str, key: &str) -> usize {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no {key}: {json}"))
        + key.len()
        + 3;
    json[at..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

fn upgrade(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["upgrade"];
    a.extend_from_slice(args);
    repo.run(&a)
}

fn status(repo: &Repo) -> String {
    repo.git_out(&["status", "--porcelain"])
}

fn with_sums(repo: &Repo, pin: &str) {
    let mut text = repo.read("spec-dna.toml");
    text.push('\n');
    text.push_str(&sums_table(pin, 'a'));
    repo.write("spec-dna.toml", &text);
    repo.commit("sums");
}

// --- exit codes (D136) ---

#[test]
fn a_usage_error_gives_exit_code_2() {
    let repo = Repo::from_fixture("v05", &[]);
    for args in [
        &["upgrade", "extra"][..],
        &["upgrade", "--pin-file"],
        &["upgrade", "--nope"],
        &["check", "--layout"],
        &["check", "--pin-file", "x.toml"],
    ] {
        let (code, out, err) = repo.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
    }
    assert_eq!(status(&repo), "");
}

#[test]
fn a_missing_repository_or_configuration_gives_exit_code_3() {
    let empty = Repo::empty();
    let (code, _, err) = empty.run(&["upgrade"]);
    assert_eq!(code, 3, "{err}");
    let repo = Repo::new();
    repo.write("docs/a.md", "# A\n");
    repo.commit("a");
    let (code, _, err) = upgrade(&repo, &[]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("spec-dna.toml"), "{err}");
}

#[test]
fn a_pin_that_upgrade_cannot_take_gives_exit_code_3() {
    for (pin, want) in [
        ("0.3.3", "First move the project to 0.4.2"),
        ("0.5", "pins version 0.5"),
        ("9.9.9", "pins version 9.9.9"),
    ] {
        let repo = Repo::new();
        repo.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{pin}\"\n[[roots]]\npath = \"docs\"\n"),
        );
        repo.write("docs/a.md", "# A\n");
        repo.commit("pin");
        let before = tree_bytes(&repo.dir);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 3, "{pin}: {out}{err}");
        assert!(err.contains(want), "{pin}: {err}");
        assert_eq!(before, tree_bytes(&repo.dir), "{pin}");
    }
}

#[test]
fn a_file_that_neither_reader_accepts_gives_exit_code_3() {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        "spec-dna = \"0.5.0\"\nfuture-key = true\n[[roots]]\npath = \"docs\"\n",
    );
    repo.write("docs/a.md", "# A\n");
    repo.commit("pin");
    let (code, _, err) = upgrade(&repo, &[]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("future-key"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn a_dirty_tree_gives_exit_code_1_also_for_a_dry_run() {
    type Dirty = fn(&Repo);
    let cases: [(&str, Dirty); 3] = [
        ("unstaged", |r| {
            r.write("docs/irrigation/pumps.md", "# changed\n")
        }),
        ("staged", |r| {
            r.write("docs/irrigation/pumps.md", "# changed\n");
            r.git(&["add", "docs/irrigation/pumps.md"]);
        }),
        ("untracked", |r| r.write("docs/new.md", "# New\n")),
    ];
    for (what, dirty) in cases {
        for args in [&[][..], &["--dry-run"], &["--json"]] {
            let repo = Repo::from_fixture("v05", &[]);
            dirty(&repo);
            let before = tree_bytes(&repo.dir);
            let (code, out, err) = upgrade(&repo, args);
            assert_eq!(code, 1, "{what} {args:?}: {out}{err}");
            assert!(
                err.contains("not clean") || err.contains("uncommitted"),
                "{what} {args:?}: {err}"
            );
            assert_eq!(out, "", "{what} {args:?}");
            assert_eq!(before, tree_bytes(&repo.dir), "{what} {args:?}");
        }
    }
}

#[test]
fn the_message_of_a_dirty_tree_names_5_paths_and_the_count() {
    let repo = Repo::from_fixture("v05", &[]);
    for i in 0..8 {
        repo.write(&format!("docs/new-{i}.md"), "# New\n");
    }
    let (code, _, err) = upgrade(&repo, &[]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("8 "), "{err}");
    assert_eq!(err.matches("docs/new-").count(), 5, "{err}");
}

#[test]
fn an_ignored_untracked_file_is_not_a_dirty_tree() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(".gitignore", "scratch/\n");
    repo.commit("ignore");
    repo.write("scratch/a.md", "# scratch\n");
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
}

// --- dry run ---

#[test]
fn a_dry_run_leaves_the_tree_byte_identical() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        let before = tree_bytes(&repo.dir);
        let (code, out, err) = upgrade(&repo, &["--dry-run"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(before, tree_bytes(&repo.dir), "{name}");
        assert_eq!(status(&repo), "", "{name}");
        assert!(out.contains("would write spec-dna.toml"), "{name}: {out}");
        assert!(out.contains("--- a/spec-dna.toml\n"), "{name}: {out}");
        assert!(out.contains("proof: ok"), "{name}: {out}");
        assert!(out.contains("\nto commit: git add -A -- "), "{name}: {out}");
        assert!(!out.contains("\nmoved "), "{name}: {out}");
    }
}

#[test]
fn a_dry_run_names_each_move_with_the_header_of_a_rename() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "would move docs/changes/rework-pumps.md -> docs/intent/rework-pumps/intent.md\n"
        ),
        "{out}"
    );
    assert!(
        out.contains("--- a/docs/changes/rework-pumps.md\n"),
        "{out}"
    );
    assert!(
        out.contains("+++ b/docs/intent/rework-pumps/intent.md\n"),
        "{out}"
    );
    // A `done` change stays in place.
    assert!(!out.contains("tune-pump.md ->"), "{out}");
    assert!(
        out.contains("would write docs/changes/tune-pump.md"),
        "{out}"
    );
}

// --- the write ---

#[test]
fn the_write_gives_a_tree_that_check_loads_and_never_commits() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        let head = repo.git_out(&["rev-parse", "HEAD"]);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(head, repo.git_out(&["rev-parse", "HEAD"]), "{name}");
        assert!(out.contains("wrote spec-dna.toml"), "{name}: {out}");
        assert!(out.contains("proof: ok"), "{name}: {out}");
        assert!(!out.contains("\n--- "), "{name}: no diff without --dry-run");
        assert!(
            repo.read("spec-dna.toml")
                .contains(&format!("spec-dna = \"{VERSION}\""))
        );
        repo.commit("upgrade");
        let (code, out, err) = repo.run(&["check", "--all"]);
        // The corpus has errors of its own. None is in the configuration.
        assert_ne!(code, 3, "{name}: {out}{err}");
        assert!(!err.contains("spec-dna.toml"), "{name}: {err}");
    }
}

#[test]
fn the_write_adds_no_error_that_the_old_tree_did_not_have() {
    // The recorded errors of the 0.5.0 binary, by code, for each fixture.
    for (name, base, overlays) in CASES {
        let recorded = fs::read_to_string(fixture_dir("recorded").join(format!("{name}.txt")))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut old: BTreeMap<String, usize> = BTreeMap::new();
        for line in recorded.lines() {
            let f: Vec<&str> = line.split(' ').collect();
            if f.len() == 4 && f[0] == "finding" && f[2] == "error" {
                *old.entry(f[1].to_string()).or_insert(0) += 1;
            }
        }
        let repo = Repo::from_fixture(base, overlays);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        repo.commit("upgrade");
        let (_, out, _) = repo.run(&["check", "--all", "--json"]);
        let new = errors_of_check(&out);
        for (code, n) in &new {
            assert!(
                old.get(code).is_some_and(|o| o >= n),
                "{name}: {code} x{n} is new; the old tree had {old:?}"
            );
        }
    }
}

#[test]
fn each_old_id_resolves_after_the_write() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    for id in [
        "changes/rework-pumps",
        "changes/add-drip-lines",
        "changes/new-sensors",
        "changes/planning/add-pump-timer",
        "changes/solar-pump",
        "changes/tune-pump",
        "ADR-1-valve-controller",
        "ADR-3",
        "DR-4",
        "contracts/valve-api",
        "irrigation/valves",
        "irrigation/valves#shutoff",
    ] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
    }
    // The moved change is at its new place, and the old file is gone.
    assert!(repo.dir.join("docs/intent/rework-pumps/intent.md").exists());
    assert!(!repo.dir.join("docs/changes/rework-pumps.md").exists());
    assert!(repo.dir.join("docs/changes/tune-pump.md").exists());
    let moved = repo.read("docs/intent/rework-pumps/intent.md");
    assert!(moved.contains("type: intent"), "{moved}");
    assert!(moved.contains("  - changes/rework-pumps"), "{moved}");
    // A `done` change becomes `research` and keeps its 0.5 type.
    let done = repo.read("docs/changes/tune-pump.md");
    assert!(done.contains("was-type: change"), "{done}");
    // The staged part of the move: `git mv` recorded the renames.
    let s = status(&repo);
    assert_eq!(s, "", "the commit holds everything: {s}");
}

#[test]
fn each_old_id_of_each_fixture_resolves_after_the_write() {
    // The ids that the 0.5.0 binary recorded (`doc <path> <id> <type> <status>`).
    // An id that 2 documents share gave SD201 in 0.5, so it is not an id to
    // resolve: `show` finds 2 documents.
    for (name, base, overlays) in CASES {
        let recorded = fs::read_to_string(fixture_dir("recorded").join(format!("{name}.txt")))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let clashes: Vec<&str> = recorded
            .lines()
            .filter(|l| l.starts_with("finding SD201 "))
            .filter_map(|l| l.rsplit(' ').next())
            .collect();
        let repo = Repo::from_fixture(base, overlays);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        repo.commit("upgrade");
        let mut n = 0;
        for line in recorded.lines().filter(|l| l.starts_with("doc ")) {
            let f: Vec<&str> = line.split(' ').collect();
            if clashes.contains(&f[1]) {
                continue;
            }
            let (code, out, err) = repo.run(&["show", f[2]]);
            assert_eq!(code, 0, "{name}: {}: {out}{err}", f[2]);
            n += 1;
        }
        assert!(n >= 8, "{name}: {n} ids");
    }
}

#[test]
fn the_move_of_a_change_uses_git_mv() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let s = status(&repo);
    assert!(
        s.lines()
            .any(|l| l.starts_with("R") && l.contains("docs/changes/rework-pumps.md")),
        "{s}"
    );
    // The write itself is not staged: the person commits with the hint.
    assert!(s.contains(" M spec-dna.toml"), "{s}");
}

#[test]
fn the_hint_stages_the_whole_result() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let line = out
        .lines()
        .find_map(|l| l.strip_prefix("to commit: "))
        .unwrap_or_else(|| panic!("no hint: {out}"));
    let paths: Vec<&str> = line
        .strip_prefix("git add -A -- ")
        .unwrap()
        .split(' ')
        .collect();
    assert!(
        paths.contains(&":/spec-dna.toml") && paths.contains(&":/.claude/agents/checker.md"),
        "{line}"
    );
    let mut cmd = vec!["add", "-A", "--"];
    cmd.extend(paths.iter().map(|p| p.trim_matches('\'')));
    repo.git(&cmd);
    assert_eq!(
        repo.git_out(&["status", "--porcelain"])
            .lines()
            .filter(|l| !l.starts_with(&['A', 'M', 'R', 'D'][..]))
            .count(),
        0,
        "every change is staged: {}",
        status(&repo)
    );
}

#[test]
fn a_second_run_after_the_commit_changes_nothing() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        repo.commit("upgrade");
        let before = tree_bytes(&repo.dir);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(out, "spec-dna upgrade: nothing to change\n", "{name}");
        assert_eq!(before, tree_bytes(&repo.dir), "{name}");
        assert_eq!(status(&repo), "", "{name}");
        let (code, out, _) = upgrade(&repo, &["--dry-run", "--json"]);
        assert_eq!(code, 0, "{name}: {out}");
        assert!(out.contains("\"files\":[]"), "{name}: {out}");
    }
}

#[test]
fn a_second_run_after_the_layout_commit_changes_nothing() {
    let repo = Repo::from_fixture("v05", &["harness-full"]);
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn a_run_on_an_uncommitted_result_gives_exit_code_1() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, _, _) = upgrade(&repo, &[]);
    assert_eq!(code, 0);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
}

// --- layout and stops ---

#[test]
fn layout_moves_the_folders_and_keeps_the_old_ids() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("moved docs/contracts/valve-api.md -> docs/interfaces/valve-api.md\n"),
        "{out}"
    );
    repo.commit("upgrade");
    assert!(repo.dir.join("docs/interfaces/valve-api.md").exists());
    assert!(repo.dir.join("docs/rules/water-limits.md").exists());
    assert!(repo.dir.join("docs/procedures/restart-pump.md").exists());
    let (code, out, err) = repo.run(&["show", "contracts/valve-api"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, _) = repo.run(&["check", "--all", "--json"]);
    assert_ne!(code, 3, "{out}");
    assert!(!out.contains("SD303"), "{out}");
}

#[test]
fn without_layout_the_other_folders_stay() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/contracts/valve-api.md").exists());
    assert!(!repo.dir.join("docs/interfaces").exists());
}

#[test]
fn two_changes_with_one_slug_stop_the_plan_with_exit_code_1() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/changes/planning/solar-pump.md",
        "---\ntype: change\nstatus: proposed\n---\n# Solar pump again\n",
    );
    repo.commit("collision");
    let before = tree_bytes(&repo.dir);
    for args in [&[][..], &["--dry-run"], &["--json"]] {
        let (code, out, err) = upgrade(&repo, args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(err.contains("docs/changes/planning/solar-pump.md"), "{err}");
        assert!(err.contains("docs/changes/solar-pump.md"), "{err}");
        assert_eq!(before, tree_bytes(&repo.dir), "{args:?}");
        assert_eq!(status(&repo), "", "{args:?}");
    }
}

#[test]
fn a_link_in_a_file_outside_the_roots_is_a_count_and_not_a_stop() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "NOTES.md",
        "See [the change](docs/changes/rework-pumps.md).\n",
    );
    repo.commit("notes");
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(number(&out, "outside_links") >= 1, "{out}");
    let (code, out, _) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0);
    assert!(out.contains("NOTES.md"), "{out}");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_in_a_target_path_stops_before_the_first_write() {
    let repo = Repo::from_fixture("v05", &[]);
    fs::create_dir_all(repo.dir.join("elsewhere")).unwrap();
    fs::write(repo.dir.join("elsewhere/keep.md"), "# keep\n").unwrap();
    std::os::unix::fs::symlink("../elsewhere", repo.dir.join("docs/intent")).unwrap();
    repo.commit("link");
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("docs/intent"), "{err}");
    assert!(err.contains("symbolic link"), "{err}");
    assert_eq!(before, tree_bytes(&repo.dir));
    assert_eq!(status(&repo), "");
    // The same for a dry run: the stop is a part of the plan.
    let (code, _, _) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 1);
}

#[cfg(unix)]
#[test]
fn a_git_error_after_the_first_move_gives_exit_code_4_and_the_restore_commands() {
    let repo = Repo::from_fixture("v05", &[]);
    let bin = std::env::temp_dir().join(format!("spec-dna-stubgit-{}", std::process::id()));
    let real = String::from_utf8(
        std::process::Command::new("which")
            .arg("git")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    // The stub runs the real git, but the second `git mv` fails.
    let count = bin.join("count");
    stub_bin(
        &bin,
        "git",
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$a\" = mv ]; then\n    n=$(cat '{c}' 2>/dev/null || echo 0)\n    n=$((n+1))\n    echo $n > '{c}'\n    if [ $n -ge 2 ]; then echo 'fatal: stub' >&2; exit 1; fi\n  fi\ndone\nexec {real} \"$@\"\n",
            c = count.display(),
            real = real.trim()
        ),
    );
    let (code, out, err) = repo.run_env(&["upgrade"], &[("PATH", &path_with(&bin))], &[]);
    let _ = fs::remove_dir_all(&bin);
    assert_eq!(code, 4, "{out}{err}");
    assert!(err.contains("git reset --hard"), "{err}");
    assert!(err.contains("git clean -fd"), "{err}");
}

// --- pin file (D135) ---

#[test]
fn a_valid_pin_file_replaces_the_sums() {
    let repo = Repo::from_fixture("v05", &[]);
    with_sums(&repo, "0.5.0");
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'b')));
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    let toml = repo.read("spec-dna.toml");
    assert!(toml.contains(&format!("spec-dna-{VERSION}-")), "{toml}");
    assert!(!toml.contains("spec-dna-0.5.0-"), "{toml}");
    assert!(toml.contains(&"b".repeat(64)), "{toml}");
    repo.commit("upgrade");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_ne!(code, 3, "{out}{err}");
}

#[test]
fn a_pin_file_with_no_table_in_the_project_adds_the_table() {
    let repo = Repo::from_fixture("v05", &[]);
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'c')));
    let (code, out, err) = upgrade(
        &repo,
        &["--dry-run", "--json", "--pin-file", file.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"sums_added\":true"), "{out}");
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("[spec-dna-sha256]"), "{out}");
    assert!(repo.read("spec-dna.toml").contains("[spec-dna-sha256]"));
}

#[test]
fn a_pin_file_gives_the_table_to_a_project_that_pins_the_binary_and_has_none() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'c')));
    let file = file.to_str().unwrap().to_string();
    let (code, out, err) = upgrade(&repo, &["--pin-file", &file]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("wrote spec-dna.toml\n"), "{out}");
    assert_eq!(
        out.matches("\nwrote ").count() + 1,
        1,
        "only the file: {out}"
    );
    assert!(repo.read("spec-dna.toml").contains(&"c".repeat(64)));
    repo.commit("sums");
    // With the table in the file, the pin file changes nothing.
    let (code, out, err) = upgrade(&repo, &["--pin-file", &file]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
}

// --- json ---

#[test]
fn json_has_the_fields_of_d152() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with('{') && out.ends_with("}\n"), "{out}");
    assert_eq!(out.matches('\n').count(), 1, "one line: {out}");
    for key in [
        "\"version\":\"",
        "\"from\":\"0.5.0\"",
        &format!("\"to\":\"{VERSION}\""),
        "\"dry_run\":true",
        "\"files\":[",
        "\"proof\":{\"ok\":true",
        "\"report\":{",
        "\"proposals\":[",
        "\"no_status\":[",
        "\"locked_cites\":[",
        "\"kept\":[",
        "\"ended_roles\":[",
        "\"behavior\":[",
        "\"errors\":[",
        "\"sums_added\":false",
        "\"outside_links\":",
        "\"folder_links\":",
    ] {
        assert!(out.contains(key), "{key}: {out}");
    }
    assert!(
        out.contains("\"action\":\"move\",\"note\""),
        "a move has an action: {out}"
    );
    assert!(out.contains("\"from\":\"docs/changes/rework-pumps.md\""));
    assert!(out.contains("\"path\":\"docs/intent/rework-pumps/intent.md\""));
    assert!(out.contains("\"action\":\"write\""), "{out}");
    assert!(out.contains("\"action\":\"remove\""), "{out}");
    assert!(
        out.contains("\"path\":\".claude/agents/verifier.md\""),
        "{out}"
    );
    assert!(out.contains("\"diff\":\"--- "), "{out}");
    assert!(number(&out, "ids") > 20, "{out}");
    assert!(number(&out, "requirements") > 0, "{out}");
    // Without --dry-run, the same object says dry_run false.
    let (code, out, _) = upgrade(&repo, &["--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"dry_run\":false"), "{out}");
}

#[test]
fn the_report_gives_the_items_of_the_plan() {
    let repo = Repo::from_fixture("v05", &["harness-full"]);
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    // A `done` change that no document cites, and the `rejected` change that
    // is cited only by the locked text of a decision.
    let at = out.find("\"proposals\":[").unwrap();
    let proposals = &out[at..out[at..].find(']').map(|e| at + e).unwrap()];
    assert!(proposals.contains("docs/changes/tune-pump.md"), "{out}");
    // The decision with no Status section.
    assert!(out.contains("\"no_status\":[\""), "{out}");
    assert!(out.contains("4-drain-timing.md"), "{out}");
    // The locked decision that cites a migrated change.
    let at = out.find("\"locked_cites\":[").unwrap();
    let cites = &out[at..out[at..].find("],").map(|e| at + e).unwrap()];
    assert!(cites.contains("ADR-3-solar-pump"), "{cites}");
    assert!(cites.contains("\"decision\":"), "{cites}");
    assert!(cites.contains("\"change\":"), "{cites}");
    // The ended role with its model.
    assert!(
        out.contains("\"role\":\"spec-writer\",\"model\":\""),
        "{out}"
    );
    // The 12 lines of behavior are the lines of the design.
    assert!(out.contains("former `note` is now `research`"), "{out}");
}

#[test]
fn the_locked_cites_do_not_depend_on_the_severity_of_sd611() {
    let repo = Repo::from_fixture("v05", &[]);
    let toml = repo.read("spec-dna.toml").replace(
        "lint = \"warning\"\n",
        "lint = \"warning\"\nSD611 = \"off\"\n",
    );
    assert!(toml.contains("SD611"));
    repo.write("spec-dna.toml", &toml);
    repo.commit("SD611 off");
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"locked_cites\":[{\"decision\":\"docs/decisions/ADR-3-solar-pump.md\""),
        "{out}"
    );
}

#[test]
fn the_counts_of_the_report_equal_check_all_after_the_write() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        let (code, plan, err) = upgrade(&repo, &["--dry-run", "--json"]);
        assert_eq!(code, 0, "{name}: {plan}{err}");
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        repo.commit("upgrade");
        let (_, check, _) = repo.run(&["check", "--all", "--json"]);
        assert_eq!(
            errors_of_report(&plan),
            errors_of_check(&check),
            "{name}: the report and check --all give the same errors"
        );
    }
}

// --- the harness (D147) ---

#[test]
fn a_lean_harness_gets_the_verifier_and_loses_the_unedited_checker() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("would remove .claude/agents/checker.md"),
        "{out}"
    );
    assert!(
        out.contains("would write .claude/agents/verifier.md"),
        "{out}"
    );
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join(".claude/agents/checker.md").exists());
    assert!(repo.dir.join(".claude/agents/verifier.md").exists());
    let toml = repo.read("spec-dna.toml");
    assert!(toml.contains("skills = []"), "{toml}");
    assert!(toml.contains("protect-tests = \"off\""), "{toml}");
    repo.commit("upgrade");
    let (_, out, _) = repo.run(&["check", "--all", "--json"]);
    assert!(
        !out.contains("\"code\":\"SD50"),
        "no harness finding: {out}"
    );
}

#[test]
fn an_edited_agent_file_stays_and_gives_sd501_before_and_after() {
    let repo = Repo::from_fixture("v05", &["harness-lean", "harness-edited"]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("kept .claude/agents/checker.md"), "{out}");
    assert!(repo.dir.join(".claude/agents/checker.md").exists());
    repo.commit("upgrade");
    let (_, out, _) = repo.run(&["check", "--all", "--json"]);
    assert!(out.contains("\"code\":\"SD501\""), "{out}");
    assert!(out.contains(".claude/agents/checker.md"), "{out}");
}

#[test]
fn an_agent_file_with_no_marker_turns_its_role_off() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    repo.write(
        ".claude/agents/verifier.md",
        "---\nname: verifier\n---\nMy own verifier.\n",
    );
    repo.commit("own verifier");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        repo.read(".claude/agents/verifier.md"),
        "---\nname: verifier\n---\nMy own verifier.\n"
    );
    assert!(out.contains("kept .claude/agents/verifier.md"), "{out}");
    let toml = repo.read("spec-dna.toml");
    assert!(
        toml.contains("verifier = { claude-code = \"none\""),
        "{toml}"
    );
    repo.commit("upgrade");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_ne!(code, 3, "{out}{err}");
}

#[test]
fn a_git_ignored_harness_file_stops_the_upgrade() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    repo.write(".gitignore", ".claude/agents/verifier.md\n");
    repo.commit("ignore");
    let before = tree_bytes(&repo.dir);
    for args in [&[][..], &["--dry-run"]] {
        let (code, out, err) = upgrade(&repo, args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(err.contains("verifier.md"), "{err}");
        assert_eq!(before, tree_bytes(&repo.dir));
    }
}

// --- init after the upgrade ---

#[test]
fn init_reconfigure_runs_on_the_result_of_a_real_upgrade() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    let (code, out, err) = repo.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("pins version"), "{err}");
    let (code, out, err) = repo.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // The upgrade wrote the adapters, so plain `init` has no change to make.
    assert!(!out.contains("verifier.md"), "{out}");
    // The real run: it writes, and the tree has no finding of the harness.
    let (code, out, err) = repo.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("reconfigure");
    let (_, out, _) = repo.run(&["check", "--all", "--json"]);
    assert!(!out.contains("\"code\":\"SD50"), "{out}");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
}

#[test]
fn init_and_check_name_upgrade_until_the_upgrade_runs() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, _, err) = repo.run(&["init"]);
    assert_eq!(code, 3);
    assert!(err.contains("spec-dna upgrade"), "{err}");
    let (code, _, _) = upgrade(&repo, &[]);
    assert_eq!(code, 0);
    repo.commit("upgrade");
    let (code, _, err) = repo.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{err}");
}

// --- a second upgrade after new 0.5 documents (D137, D141) ---

#[test]
fn a_branch_that_brings_0_5_documents_after_the_upgrade_runs_upgrade_again() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    // A 0.5 document that a branch brings: its type is a 0.5 name.
    repo.write(
        "docs/irrigation/fans.md",
        "---\ntype: contract\nstatus: active\n---\n# Fans\n\n- **F-1**: The fan shall start in 2 seconds.\n",
    );
    repo.commit("a 0.5 document");
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // The configuration and the other documents do not change.
    assert!(
        out.contains("would write docs/irrigation/fans.md\n"),
        "{out}"
    );
    assert!(!out.contains("would write spec-dna.toml"), "{out}");
    assert_eq!(out.matches("would write ").count(), 1, "{out}");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.read("docs/irrigation/fans.md")
            .contains("type: interface")
    );
    repo.commit("second upgrade");
    let (code, out, _) = upgrade(&repo, &[]);
    assert_eq!(code, 0);
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
}

// --- an instruction file that is a document too (D231) ---

#[test]
fn an_instruction_file_that_is_a_document_stops_the_plan() {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        "spec-dna = \"0.5.0\"\n[[roots]]\npath = \".\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\n",
    );
    repo.write(
        "AGENTS.md",
        "---\ntype: contract\nstatus: active\n---\n# Agents\n\nOur rules.\n",
    );
    repo.commit("project");
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("AGENTS.md"), "{err}");
    assert!(err.contains("does not merge"), "{err}");
    assert_eq!(before, tree_bytes(&repo.dir));
}

// --- a failed proof (D149, D217) ---

#[test]
fn a_failed_proof_stops_with_exit_code_1_and_writes_nothing() {
    // A pointer of `[conventions]` that names a document by its path is not
    // edited, so `--layout` makes it a broken pointer (SD106). Item 5 of the
    // proof refuses the plan.
    let repo = Repo::from_fixture("v05", &[]);
    let mut toml = repo.read("spec-dna.toml");
    toml.push_str("\n[conventions]\ncontract = [\"docs/contracts/valve-api.md\"]\n");
    repo.write("spec-dna.toml", &toml);
    repo.commit("a pointer by path");
    // Without --layout the file stays, and the plan passes.
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let before = tree_bytes(&repo.dir);
    for args in [&["--layout"][..], &["--layout", "--dry-run"]] {
        let (code, out, err) = upgrade(&repo, args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert_eq!(out, "", "{args:?}");
        assert!(err.contains("the proof failed"), "{args:?}: {err}");
        assert!(err.contains("SD106"), "{args:?}: {err}");
        assert_eq!(before, tree_bytes(&repo.dir), "{args:?}");
        assert_eq!(status(&repo), "", "{args:?}");
    }
    // With --json the object is on stdout, and the proof says that it failed.
    let (code, out, err) = upgrade(&repo, &["--layout", "--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"proof\":{\"ok\":false"), "{out}");
    assert!(
        out.contains("\"failures\":[\"") && out.contains("SD106"),
        "{out}"
    );
    assert_eq!(before, tree_bytes(&repo.dir));
}

// --- the repairs of a 0.6 file, the empty folders and the notes (0.6.1) ---

/// A committed 0.6 project of the small graph, with no harness. `types` is a
/// line for the key `types`, or `None` for a file with no such key. The file
/// pins the version of the binary, so a pin change does not hide a repair.
fn project_06(types: Option<&str>) -> Repo {
    let repo = Repo::new();
    let answers = repo.dir.with_extension("answers.toml");
    fs::write(&answers, "harness = \"none\"\n").unwrap();
    let a = answers.to_str().unwrap();
    let (code, out, err) = repo.run(&["init", "--answers", a]);
    assert_eq!(code, 0, "{out}{err}");
    if let Some(line) = types {
        let text = repo.read("spec-dna.toml");
        let at = text.find("profile = ").unwrap();
        repo.write(
            "spec-dna.toml",
            &format!("{}{line}\n{}", &text[..at], &text[at..]),
        );
        // `init` writes the adapters again, so they list the new types.
        let (code, out, err) = repo.run(&["init", "--reconfigure", "--answers", a]);
        assert_eq!(code, 0, "{out}{err}");
    }
    let _ = fs::remove_file(&answers);
    repo.commit("adopt spec-dna");
    repo
}

/// The text of `spec-dna.toml` with the `linked-from` of `[layers.pending]`
/// set to `value`.
fn pending_links(repo: &Repo, value: &str) {
    let text = repo.read("spec-dna.toml");
    let table = text.find("[layers.pending]").unwrap();
    let key = table + text[table..].find("linked-from = ").unwrap();
    let end = key + text[key..].find('\n').unwrap();
    let edited = format!("{}linked-from = {value}{}", &text[..key], &text[end..]);
    repo.write("spec-dna.toml", &edited);
    // A value that is already there changes nothing.
    if !status(repo).is_empty() {
        repo.commit("edit the pending layer");
    }
}

#[test]
fn upgrade_of_a_06_file_with_a_policy_adds_spec_and_plan_to_types() {
    let repo = project_06(Some("types = [\"requirements\", \"intent\"]"));
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("would write spec-dna.toml"), "{out}");
    assert!(
        out.contains("`spec` and `plan`"),
        "the report names it: {out}"
    );
    assert_eq!(before, tree_bytes(&repo.dir), "a dry run writes nothing");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let toml = repo.read("spec-dna.toml");
    assert!(
        toml.contains("types = [\"requirements\", \"intent\", \"spec\", \"plan\"]\n"),
        "{toml}"
    );
    // The skill lists the types of the project, so it changes with the list.
    // No other file changes.
    assert_eq!(
        status(&repo),
        " M .claude/skills/spec-dna/SKILL.md\n M spec-dna.toml"
    );
    repo.commit("upgrade");
    // A second run has nothing to change, and the adapters fit the new list.
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    for ty in ["intent", "spec", "plan"] {
        let (code, out, err) = repo.run(&["new", ty, "x"]);
        assert_eq!(code, 0, "new {ty}: {out}{err}");
    }
}

#[test]
fn upgrade_of_a_06_file_with_the_policy_none_keeps_the_types() {
    let repo = project_06(Some("types = [\"requirements\", \"intent\"]"));
    let toml = repo
        .read("spec-dna.toml")
        .replace("policy = \"three-class\"", "policy = \"none\"");
    repo.write("spec-dna.toml", &toml);
    repo.commit("no policy");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
    // A file with no key `types` allows each type, so it has nothing to add.
    let repo = project_06(None);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
}

#[test]
fn a_types_list_of_many_lines_is_named_and_not_edited() {
    // `init` writes the list in one line, so the test makes the lines.
    let repo = project_06(Some("types = [\"requirements\", \"intent\"]"));
    let toml = repo.read("spec-dna.toml").replace(
        "types = [\"requirements\", \"intent\"]",
        "types = [\n  \"requirements\", # kept\n  \"intent\",\n]",
    );
    repo.write("spec-dna.toml", &toml);
    repo.commit("many lines");
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
    assert!(
        out.contains("`types` lacks `spec` and `plan`"),
        "the output names the hand edit: {out}"
    );
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("`types` lacks `spec` and `plan`"), "{out}");
}

#[test]
fn upgrade_of_a_06_file_widens_the_pending_links() {
    let repo = project_06(None);
    pending_links(&repo, "[\"intent\"]");
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("would write spec-dna.toml"), "{out}");
    assert!(
        out.contains("`[layers.pending]`"),
        "the report names it: {out}"
    );
    assert_eq!(before, tree_bytes(&repo.dir), "a dry run writes nothing");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let toml = repo.read("spec-dna.toml");
    assert!(
        toml.contains("linked-from = [\"intent\", \"requirements\", \"decisions\"]\n"),
        "{toml}"
    );
    assert_eq!(
        status(&repo),
        " M spec-dna.toml",
        "only the file of the project"
    );
    repo.commit("upgrade");
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
}

#[test]
fn upgrade_keeps_an_empty_types_list_of_a_06_file() {
    // An empty list means that every type is valid, so it has nothing to add.
    let repo = project_06(Some("types = []"));
    let (code, out, err) = upgrade(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
    assert!(!out.contains("`types`"), "{out}");
}

#[test]
fn upgrade_json_lists_the_edits_of_a_06_file() {
    let repo = project_06(Some("types = [\"requirements\", \"intent\"]"));
    pending_links(&repo, "[\"intent\"]");
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let at = out.find("\"edits\":[").unwrap_or_else(|| panic!("{out}"));
    let list = &out[at..at + out[at..].find("\"]").unwrap()];
    assert!(list.contains("`types` got `spec` and `plan`"), "{list}");
    assert!(
        list.contains("`linked-from` of `[layers.pending]`"),
        "{list}"
    );
    // A project with nothing to edit has an empty list.
    let repo = project_06(None);
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"edits\":[]"), "{out}");
}

#[test]
fn upgrade_keeps_a_pending_links_value_that_is_not_the_old_default() {
    for value in [
        "[\"intent\", \"decisions\"]",
        "[\"requirements\"]",
        "[]",
        "[\"intent\", \"requirements\", \"decisions\"]",
    ] {
        let repo = project_06(None);
        pending_links(&repo, value);
        let (code, out, err) = upgrade(&repo, &[]);
        assert_eq!(code, 0, "{value}: {out}{err}");
        assert!(out.contains("nothing to change"), "{value}: {out}");
    }
}

#[test]
fn the_behavior_notes_show_only_for_a_05_source() {
    // A 0.6 file with no sums table and `--pin-file`: the plan is not empty.
    let repo = project_06(None);
    assert!(!repo.read("spec-dna.toml").contains("[spec-dna-sha256]"));
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'a')));
    let pin = file.to_str().unwrap();
    let (code, out, err) = upgrade(&repo, &["--dry-run", "--pin-file", pin]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("report:"), "{out}");
    assert!(!out.contains("changes of behavior"), "{out}");
    assert!(!out.contains("A former `note`"), "{out}");
    // A 0.5 source keeps the 5 lines.
    let old = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&old, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("changes of behavior:"), "{out}");
    assert_eq!(out.matches("\n    - ").count(), 5, "{out}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn upgrade_removes_the_folder_of_the_moved_changes() {
    let repo = Repo::from_fixture("v05", &[]);
    assert!(repo.dir.join("docs/changes/planning").is_dir());
    // A dry run removes nothing.
    let (code, out, err) = upgrade(&repo, &["--layout", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/changes/planning").is_dir());
    assert!(repo.dir.join("docs/contracts").is_dir());
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.dir
            .join("docs/intent/add-pump-timer/intent.md")
            .is_file(),
        "{out}"
    );
    // The folder that held only moved changes is gone.
    assert!(!repo.dir.join("docs/changes/planning").exists());
    assert!(!repo.dir.join("docs/contracts").exists());
    // A folder that still holds a file, and the root, stay.
    assert!(repo.dir.join("docs/changes/tune-pump.md").is_file());
    assert!(repo.dir.join("docs/changes").is_dir());
    assert!(repo.dir.join("docs").is_dir());
    // The tree has no empty folder that the write made.
    repo.commit("upgrade");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("nothing to change"), "{out}");
}

// --- top files and areas (0.6.1, D40 to D48) ---

/// The recorded errors of the 0.5.0 binary on `v05`, by code. The overlay
/// `top-files` adds documents that the 0.5.0 binary reads with no error.
fn old_errors_of_v05() -> BTreeMap<String, usize> {
    let recorded = fs::read_to_string(fixture_dir("recorded").join("v05.txt")).unwrap();
    let mut old = BTreeMap::new();
    for line in recorded.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        if f.len() == 4 && f[0] == "finding" && f[2] == "error" {
            *old.entry(f[1].to_string()).or_insert(0) += 1;
        }
    }
    old
}

/// No error of `check --all` is new against the errors of the 0.5.0 binary.
fn assert_no_new_error(repo: &Repo, what: &str) {
    let (_, out, _) = repo.run(&["check", "--all", "--json"]);
    let old = old_errors_of_v05();
    for (code, n) in errors_of_check(&out) {
        assert!(
            old.get(&code).is_some_and(|o| *o >= n),
            "{what}: {code} x{n} is new; the old tree had {old:?}\n{out}"
        );
    }
}

static AREA_FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Writes a file for `--areas` outside the repository (a file inside would
/// make the tree not clean) and returns its folder and its path.
fn areas_file(text: &str) -> (std::path::PathBuf, String) {
    let n = AREA_FILES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("spec-dna-areas-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("areas.toml");
    fs::write(&path, text).unwrap();
    (dir, path.to_str().unwrap().to_string())
}

/// `upgrade --layout --areas <file>` with the text of the file.
fn upgrade_areas(repo: &Repo, text: &str, extra: &[&str]) -> (i32, String, String) {
    let (dir, file) = areas_file(text);
    let mut args = vec!["--layout", "--areas", file.as_str()];
    args.extend_from_slice(extra);
    let r = upgrade(repo, &args);
    let _ = fs::remove_dir_all(dir);
    r
}

const TOP_MOVES: [(&str, &str); 5] = [
    ("docs/changes/roadmap.md", "docs/roadmap.md"),
    ("docs/notes/product-overview.md", "docs/product.md"),
    ("docs/reference/open-questions.md", "docs/questions.md"),
    ("docs/notes/glossary.md", "docs/terms.md"),
    ("docs/lab/reference/architecture.md", "docs/lab/system.md"),
];

#[test]
fn layout_moves_a_roadmap_change_and_overview_notes_to_top_files() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    let (code, out, err) = upgrade(&repo, &["--layout", "--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"proof\":{\"ok\":true"), "{out}");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("proof: ok"), "{out}");
    for (from, to) in TOP_MOVES {
        assert!(out.contains(&format!("moved {from} -> {to}\n")), "{out}");
    }
    repo.commit("upgrade");
    for (from, to) in TOP_MOVES {
        assert!(repo.dir.join(to).is_file(), "{to}");
        assert!(!repo.dir.join(from).exists(), "{from}");
        // The place gives the type, so the file has no type, no status and no
        // marker. The old id is an alias.
        let text = repo.read(to);
        for line in ["type:", "status:", "was-type:"] {
            assert!(!text.contains(line), "{to} has {line}: {text}");
        }
        assert!(text.contains("aliases:\n  - "), "{to}: {text}");
    }
    // A change named `roadmap` is a top file, not an intent.
    assert!(!repo.dir.join("docs/intent/roadmap").exists());
    assert!(
        repo.read("docs/roadmap.md")
            .contains("  - changes/roadmap\n")
    );
    // Each old id resolves, and the new id names the top file.
    for (id, path) in [
        ("changes/roadmap", "docs/roadmap.md"),
        ("roadmap", "docs/roadmap.md"),
        ("notes/product-overview", "docs/product.md"),
        ("reference/open-questions", "docs/questions.md"),
        ("notes/glossary", "docs/terms.md"),
        ("reference/architecture", "docs/lab/system.md"),
    ] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
        assert!(out.contains(path), "{id}: {out}");
    }
    // The links follow the moves.
    let roadmap = repo.read("docs/roadmap.md");
    assert!(roadmap.contains("(irrigation/pumps.md)"), "{roadmap}");
    let product = repo.read("docs/product.md");
    assert!(product.contains("(questions.md)"), "{product}");
    // The layers of the top files are in the graph.
    let toml = repo.read("spec-dna.toml");
    assert!(toml.contains("[layers.pending]"), "{toml}");
    assert!(toml.contains("[layers.foundation]"), "{toml}");
    assert_no_new_error(&repo, "top-files");
    // The result is stable: a second run changes nothing.
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, "spec-dna upgrade: nothing to change\n");
}

#[test]
fn without_layout_a_roadmap_change_is_an_intent_and_the_notes_stay() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/intent/roadmap/intent.md").is_file());
    assert!(repo.dir.join("docs/notes/product-overview.md").is_file());
    assert!(!repo.dir.join("docs/product.md").exists());
    assert!(!repo.dir.join("docs/roadmap.md").exists());
}

#[test]
fn two_files_for_one_top_file_stay_and_the_report_has_one_kept_line() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    repo.write(
        "docs/reference/overview.md",
        "---\ntype: reference\n---\n# Overview\n\nA second overview.\n",
    );
    repo.commit("a second overview");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join("docs/product.md").exists(), "{out}");
    let kept: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("kept ") && l.contains("docs/product.md"))
        .collect();
    assert_eq!(kept.len(), 1, "{out}");
    assert!(
        kept[0].contains("docs/notes/product-overview.md")
            && kept[0].contains("docs/reference/overview.md"),
        "{out}"
    );
    // The other top files still move.
    assert!(repo.dir.join("docs/roadmap.md").is_file(), "{out}");
    assert!(repo.dir.join("docs/terms.md").is_file(), "{out}");
    repo.commit("upgrade");
    // Both files keep their ids.
    for id in ["notes/product-overview", "reference/overview"] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
    }
    assert_no_new_error(&repo, "two files");
}

#[test]
fn a_top_file_that_exists_keeps_the_document_and_the_report_has_a_kept_line() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    repo.write(
        "docs/roadmap.md",
        "---\ntype: research\n---\n# Old roadmap\n\nThe file exists already.\n",
    );
    repo.commit("a roadmap");
    let (code, out, err) = upgrade(&repo, &["--layout", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"kept\":[\"kept docs/changes/roadmap.md"),
        "{out}"
    );
    // The change is an intent, as without the top files.
    assert!(repo.dir.join("docs/intent/roadmap/intent.md").is_file());
    assert!(repo.read("docs/roadmap.md").contains("Old roadmap"));
    // The other documents move to their top files.
    assert!(repo.dir.join("docs/product.md").is_file());
}

#[test]
fn a_document_that_does_not_meet_d41_is_not_a_top_file() {
    let repo = Repo::from_fixture("v05", &[]);
    for (path, text) in [
        // A requirement.
        (
            "docs/notes/glossary.md",
            "---\ntype: note\n---\n# Glossary\n\n- **valve-open**: The valve MUST open.\n",
        ),
        // Not directly in a folder of the list.
        (
            "docs/irrigation/roadmap.md",
            "---\ntype: note\n---\n# Roadmap\n\nA note.\n",
        ),
        // Another type.
        (
            "docs/notes/overview.md",
            "---\ntype: spec\n---\n# Overview\n\nA spec.\n",
        ),
        // A change that is done.
        (
            "docs/changes/questions.md",
            "---\ntype: change\nstatus: done\n---\n# Questions\n\nDone.\n",
        ),
        // A name that is not in the list.
        (
            "docs/notes/pump-glossary.md",
            "---\ntype: note\n---\n# Pump glossary\n\nA note.\n",
        ),
    ] {
        repo.write(path, text);
    }
    repo.commit("documents");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    for top in ["terms", "roadmap", "product", "questions", "system"] {
        assert!(!repo.dir.join(format!("docs/{top}.md")).exists(), "{top}");
    }
    assert!(!out.contains("kept docs/"), "{out}");
    assert!(repo.dir.join("docs/irrigation/roadmap.md").is_file());
    assert!(repo.dir.join("docs/notes/overview.md").is_file());
    assert!(
        repo.read("docs/changes/questions.md")
            .contains("was-type: change")
    );
}

#[test]
fn a_top_file_of_a_06_project_does_not_move() {
    // D44: a 0.6 source gets no move to a top file.
    let repo = project_06(None);
    repo.write(
        "docs/notes/roadmap.md",
        "---\ntype: research\n---\n# Roadmap\n",
    );
    repo.commit("a note");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    // The folder `notes/` of 0.5 has its 0.6 name, as before 0.6.1, and the
    // file is no top file.
    assert!(!repo.dir.join("docs/roadmap.md").exists(), "{out}");
    assert!(repo.dir.join("docs/research/roadmap.md").is_file(), "{out}");
    assert!(!out.contains("kept "), "{out}");
}

#[test]
fn areas_moves_each_mapped_requirements_document_into_its_area() {
    let repo = Repo::from_fixture("v05", &[]);
    let map = "[areas]\n\"irrigation/pumps\" = \"pumps\"\n\"irrigation/sensors\" = \"sensors\"\n";
    let (code, out, err) = upgrade_areas(&repo, map, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("would move docs/irrigation/pumps.md -> docs/requirements/pumps/pumps.md\n"),
        "{out}"
    );
    assert_eq!(status(&repo), "");
    let (code, out, err) = upgrade_areas(&repo, map, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("proof: ok"), "{out}");
    repo.commit("upgrade");
    assert!(repo.dir.join("docs/requirements/pumps/pumps.md").is_file());
    assert!(
        repo.dir
            .join("docs/requirements/sensors/sensors.md")
            .is_file()
    );
    assert!(!repo.dir.join("docs/irrigation/pumps.md").exists());
    // The old id is an alias, and a requirement id still resolves.
    assert!(
        repo.read("docs/requirements/pumps/pumps.md")
            .contains("  - irrigation/pumps\n")
    );
    for id in [
        "irrigation/pumps",
        "irrigation/pumps#fill-window",
        "irrigation/sensors#max-age",
    ] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
        assert!(out.contains("docs/requirements/"), "{id}: {out}");
    }
    // The key `areas` is sorted and is a key of the top level.
    let toml = repo.read("spec-dna.toml");
    assert!(
        toml.contains("\nareas = [\"pumps\", \"sensors\"]\n"),
        "{toml}"
    );
    assert!(
        toml.find("areas = ").unwrap() < toml.find("[[roots]]").unwrap(),
        "{toml}"
    );
    // The links follow the moves, in both directions.
    let pumps = repo.read("docs/requirements/pumps/pumps.md");
    assert!(
        pumps.contains("(../../intent/rework-pumps/intent.md)"),
        "{pumps}"
    );
    let rework = repo.read("docs/intent/rework-pumps/intent.md");
    assert!(
        rework.contains("(../../requirements/pumps/pumps.md)"),
        "{rework}"
    );
    assert_no_new_error(&repo, "areas");
}

#[test]
fn areas_and_top_files_run_in_one_plan() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    let map = "[areas]\n\"irrigation/pumps\" = \"pumps\"\n";
    let (code, out, err) = upgrade_areas(&repo, map, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("upgrade");
    assert!(repo.dir.join("docs/requirements/pumps/pumps.md").is_file());
    assert!(repo.dir.join("docs/roadmap.md").is_file());
    // The roadmap links the pumps at their new place.
    assert!(
        repo.read("docs/roadmap.md")
            .contains("(requirements/pumps/pumps.md)")
    );
    assert_no_new_error(&repo, "areas and top files");
}

#[test]
fn a_stop_of_areas_gives_exit_code_1_and_writes_nothing() {
    let repo = Repo::from_fixture("v05", &["top-files"]);
    let before = tree_bytes(&repo.dir);
    for (map, want) in [
        ("[areas]\n\"nope/pumps\" = \"pumps\"\n", "nope/pumps"),
        (
            "[areas]\n\"contracts/valve-api\" = \"valves\"\n",
            "contracts/valve-api",
        ),
        (
            "[areas]\n\"notes/glossary\" = \"terms\"\n",
            "notes/glossary",
        ),
        ("[areas]\n\"irrigation/pumps\" = \"a/b\"\n", "a/b"),
        ("[areas]\n\"irrigation/pumps\" = \"..\"\n", ".."),
        ("[areas]\n\"irrigation/pumps\" = \"\"\n", "area"),
        (
            "[areas]\n\"irrigation/pumps\" = \"two words\"\n",
            "two words",
        ),
    ] {
        for extra in [&[][..], &["--dry-run"], &["--json"]] {
            let (code, out, err) = upgrade_areas(&repo, map, extra);
            assert_eq!(code, 1, "{map} {extra:?}: {out}{err}");
            assert!(err.contains(want), "{map}: {err}");
            assert_eq!(before, tree_bytes(&repo.dir), "{map} {extra:?}");
            assert_eq!(status(&repo), "", "{map} {extra:?}");
        }
    }
}

#[test]
fn two_documents_for_one_area_file_stop_the_plan() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/harvest/pumps.md",
        "---\ntype: spec\n---\n# Pumps of the harvest\n\n## Requirements\n\n- **harvest-pump**: The pump SHALL stop at the end of the harvest.\n",
    );
    repo.commit("a second pumps");
    let before = tree_bytes(&repo.dir);
    let map = "[areas]\n\"irrigation/pumps\" = \"pumps\"\n\"harvest/pumps\" = \"pumps\"\n";
    let (code, out, err) = upgrade_areas(&repo, map, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("docs/requirements/pumps/pumps.md"), "{err}");
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn an_area_target_that_exists_stops_the_plan() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/requirements/pumps/pumps.md",
        "---\ntype: requirements\n---\n# Pumps\n",
    );
    repo.commit("a target");
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade_areas(&repo, "[areas]\n\"irrigation/pumps\" = \"pumps\"\n", &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("exists"), "{err}");
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn areas_without_layout_is_a_usage_error() {
    let repo = Repo::from_fixture("v05", &[]);
    let (dir, file) = areas_file("[areas]\n\"irrigation/pumps\" = \"pumps\"\n");
    let (code, out, err) = upgrade(&repo, &["--areas", &file]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("--layout"), "{err}");
    // Another command gives a usage error too.
    let (code, out, err) = repo.run(&["check", "--areas", &file]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("option of `upgrade`"), "{err}");
    let (code, out, err) = upgrade(&repo, &["--layout", "--areas"]);
    assert_eq!(code, 2, "{out}{err}");
    let _ = fs::remove_dir_all(dir);
    assert_eq!(status(&repo), "");
}

#[test]
fn a_bad_areas_file_gives_exit_code_3() {
    let repo = Repo::from_fixture("v05", &[]);
    let (code, out, err) = upgrade(&repo, &["--layout", "--areas", "/no/such/areas.toml"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("--areas"), "{err}");
    for text in [
        "not toml [",
        "[other]\nx = \"y\"\n",
        "[areas]\n\"irrigation/pumps\" = 3\n",
        "areas = \"pumps\"\n",
    ] {
        let (code, out, err) = upgrade_areas(&repo, text, &[]);
        assert_eq!(code, 3, "{text}: {out}{err}");
        assert!(err.contains("--areas"), "{text}: {err}");
    }
    assert_eq!(status(&repo), "");
}

#[test]
fn areas_on_a_06_source_is_a_usage_error_with_the_text_of_d48() {
    let repo = project_06(None);
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = upgrade_areas(&repo, "[areas]\n\"a\" = \"b\"\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains(
            "`--areas` moves documents of a 0.5 project; in a 0.6 project, run `spec-dna mv` in a pull request of its own"
        ),
        "{err}"
    );
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn a_document_over_the_hard_budget_of_its_top_type_is_no_top_file() {
    // A `reference` has no budget, and `system` has a hard budget. The move
    // would give a new error SD702.
    let repo = Repo::from_fixture("v05", &["top-files"]);
    let long: String = (0..400)
        .map(|i| format!("Line {i} of the long text that fills the budget.\n"))
        .collect();
    repo.write(
        "docs/reference/system.md",
        &format!("---\ntype: reference\n---\n# System\n\n{long}"),
    );
    repo.commit("a long system");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join("docs/system.md").exists(), "{out}");
    let kept: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("kept docs/reference/system.md"))
        .collect();
    assert_eq!(kept.len(), 1, "{out}");
    assert!(kept[0].contains("over the hard budget of 16000"), "{out}");
    assert!(!out.contains("SD702"), "{out}");
    // The other documents are top files still.
    assert!(repo.dir.join("docs/terms.md").is_file());
    repo.commit("upgrade");
    assert_no_new_error(&repo, "a long system");
}

#[test]
fn a_top_file_with_the_id_of_another_document_is_kept() {
    // `docs/lab` is a root of its own with no namespace, so `docs/lab/terms.md`
    // and `docs/terms.md` would have the id `terms`. A document of a root of one
    // type has the id `questions`. The proof would refuse each of them (SD201),
    // so the plan keeps the documents where their other rules put them.
    let repo = Repo::from_fixture("v05", &["top-files"]);
    repo.write(
        "docs/lab/reference/terms.md",
        "---\ntype: reference\n---\n# Lab terms\n\nThe terms of the lab.\n",
    );
    repo.write(
        "docs/field-notes/questions.md",
        "# Questions of the field\n",
    );
    repo.commit("documents with the ids of top files");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join("docs/terms.md").exists(), "{out}");
    assert!(!repo.dir.join("docs/lab/terms.md").exists(), "{out}");
    assert!(!repo.dir.join("docs/questions.md").exists(), "{out}");
    let kept: Vec<&str> = out.lines().filter(|l| l.starts_with("kept ")).collect();
    assert_eq!(kept.len(), 2, "{out}");
    assert!(
        kept.iter()
            .any(|l| l.contains("docs/lab/reference/terms.md")
                && l.contains("docs/notes/glossary.md")
                && l.contains("`terms`")),
        "{out}"
    );
    assert!(
        kept.iter()
            .any(|l| l.contains("docs/reference/open-questions.md") && l.contains("`questions`")),
        "{out}"
    );
    // The other top files move.
    assert!(repo.dir.join("docs/roadmap.md").is_file());
    assert!(repo.dir.join("docs/product.md").is_file());
    repo.commit("upgrade");
    for id in [
        "notes/glossary",
        "reference/open-questions",
        "reference/terms",
    ] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
    }
}

#[test]
fn areas_moves_a_document_of_a_root_of_one_type_inside_its_root() {
    // D46 says "in its own root", and a root of one type is a root. `--layout`
    // keeps the paths of such a root, but the file of `--areas` names the
    // document, so the move is the choice of the person.
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        "spec-dna = \"0.5.0\"\n\n[[roots]]\npath = \"docs/specs\"\ntype = \"spec\"\n",
    );
    repo.write(
        "docs/specs/pumps.md",
        "---\nstatus: active\n---\n# Pumps\n\n- **fill-window**: The controller SHALL run the pump at night.\n",
    );
    repo.commit("a root of specs");
    let (code, out, err) = upgrade_areas(&repo, "[areas]\n\"pumps\" = \"pumps\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("proof: ok"), "{out}");
    repo.commit("upgrade");
    assert!(
        repo.dir
            .join("docs/specs/requirements/pumps/pumps.md")
            .is_file()
    );
    for id in ["pumps", "pumps#fill-window"] {
        let (code, out, err) = repo.run(&["show", id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
    }
}

// --- names that differ only in case (0.6.1) ---
//
// A file system can fold the case of a name (macOS, Windows). The plan reads
// each path as lowercase ASCII where it decides that a target is free, so the
// result does not depend on the file system. These tests use names that differ
// only in case, and they give the same result on each file system.

#[test]
fn a_top_file_that_exists_under_another_case_keeps_the_document() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/Roadmap.md",
        "---\ntype: spec\n---\n# Roadmap of the spec\n\nText.\n",
    );
    repo.write(
        "docs/notes/roadmap.md",
        "---\ntype: note\n---\n# Roadmap\n\nText.\n",
    );
    repo.commit("2 names");
    for args in [&["--layout", "--dry-run"][..], &["--layout"]] {
        let (code, out, err) = upgrade(&repo, args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(
            out.lines().any(|l| l
                .starts_with("kept docs/notes/roadmap.md: the top file docs/roadmap.md exists")),
            "{args:?}: {out}"
        );
    }
    // The spec stays, and the note follows its folder.
    assert!(repo.read("docs/Roadmap.md").contains("Roadmap of the spec"));
    assert!(repo.dir.join("docs/research/roadmap.md").is_file());
    assert!(!repo.dir.join("docs/notes/roadmap.md").exists());
}

#[test]
fn two_areas_that_differ_only_in_case_stop_the_plan() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/specs/pumps.md",
        "---\ntype: spec\n---\n# Pumps of the field\n\n- **field-pump**: The pump SHALL stop at night.\n",
    );
    repo.commit("a second pumps");
    let before = tree_bytes(&repo.dir);
    let map = "[areas]\n\"irrigation/pumps\" = \"Pumps\"\n\"specs/pumps\" = \"pumps\"\n";
    for extra in [&["--dry-run"][..], &[]] {
        let (code, out, err) = upgrade_areas(&repo, map, extra);
        assert_eq!(code, 1, "{extra:?}: {out}{err}");
        assert!(err.contains("2 files move to"), "{err}");
        assert_eq!(before, tree_bytes(&repo.dir));
    }
}

#[test]
fn two_files_for_one_target_that_differ_only_in_case_stop_the_plan() {
    // Both want `docs/roadmap.md`, so D42 keeps both. `--layout` then moves
    // both to `docs/research/`, and the 2 targets differ only in case. The rule
    // is the rule for 2 files with one target: a stop with exit code 1.
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/notes/Roadmap.md",
        "---\ntype: note\n---\n# One\n\nText.\n",
    );
    repo.write(
        "docs/reference/roadmap.md",
        "---\ntype: reference\n---\n# Two\n\nText.\n",
    );
    repo.commit("2 names");
    let before = tree_bytes(&repo.dir);
    for args in [&["--layout", "--dry-run"][..], &["--layout"]] {
        let (code, out, err) = upgrade(&repo, args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(err.contains("2 files move to"), "{err}");
        assert!(err.contains("docs/notes/Roadmap.md"), "{err}");
        assert_eq!(before, tree_bytes(&repo.dir));
    }
}

#[test]
fn a_document_whose_id_names_2_documents_is_no_top_file() {
    // The id `overview` is a duplicate already: the spec and the note of
    // another root have it. A move would change what it names, so the upgrade
    // keeps the note, and the proof passes as without `--layout`.
    let repo = Repo::from_fixture("v05", &[]);
    repo.write(
        "docs/overview.md",
        "---\ntype: spec\n---\n# Overview\n\nText.\n",
    );
    repo.write("docs/lab/overview.md", "# Lab overview\n\nText.\n");
    repo.commit("a duplicate id");
    let (code, out, err) = upgrade(&repo, &["--layout"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("proof: ok"), "{out}");
    let kept: Vec<&str> = out.lines().filter(|l| l.starts_with("kept ")).collect();
    assert_eq!(kept.len(), 1, "{out}");
    assert!(
        kept[0].contains("docs/lab/overview.md") && kept[0].contains("docs/overview.md"),
        "{out}"
    );
    assert!(!repo.dir.join("docs/lab/product.md").exists());
}
