//! End-to-end tests of the first pull request after an upgrade (step 5, slice
//! 12, D154 to D156). Each case uses real git and a committed fixture of
//! `tests/fixtures/upgrade/`. The fixture is the base on `main`. The branch
//! `up` holds the result of `spec-dna upgrade`, and `check --base main` is the
//! gate of the pull request. The base pins 0.5.0, so `check` plans the base in
//! memory with the code of `upgrade` (ADR 0009, "First pull request").

mod common;

use common::*;
use std::fs;

/// The fixtures of a project that pins 0.4.2 or 0.5.0: the name, the fixture
/// and its overlays.
const CASES: &[(&str, &str, &[&str])] = &[
    ("v04", "v04", &[]),
    ("v05", "v05", &[]),
    ("brownfield", "brownfield", &[]),
    ("harness-lean", "v05", &["harness-lean"]),
    ("harness-full", "v05", &["harness-full"]),
    ("harness-edited", "v05", &["harness-lean", "harness-edited"]),
    // Top files that `--layout` makes (0.6.1, D40). Documents only.
    ("top-files", "v05", &["top-files"]),
];

const UPGRADE_DATE: &str = "2026-01-16T12:00:00+00:00";
const LATER_DATE: &str = "2026-01-17T12:00:00+00:00";

/// The fixture on `main`, then the branch `up` with the upgrade, committed.
fn upgraded(fixture: &str, overlays: &[&str], args: &[&str]) -> Repo {
    let repo = Repo::from_fixture(fixture, overlays);
    repo.git(&["switch", "-q", "-c", "up"]);
    let mut a = vec!["upgrade"];
    a.extend_from_slice(args);
    let (code, out, err) = repo.run(&a);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit_at("upgrade", UPGRADE_DATE);
    repo
}

/// The gate of the pull request, with `--json`.
fn gate(repo: &Repo, extra: &[&str]) -> (i32, String, String) {
    let mut a = vec!["check", "--base", "main", "--json"];
    a.extend_from_slice(extra);
    repo.run(&a)
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

/// Each diagnostic of a `check --json` output: the code and the severity. A
/// run without `--all` lists the new diagnostics only.
fn listed(json: &str) -> Vec<(String, String)> {
    json.split("{\"code\":\"")
        .skip(1)
        .map(|part| {
            let code = part[..part.find('"').unwrap()].to_string();
            let obj = &part[..part.find("\"fix\"").unwrap_or(part.len())];
            let sev = obj
                .split("\"severity\":\"")
                .nth(1)
                .map(|s| s[..s.find('"').unwrap()].to_string())
                .unwrap_or_default();
            (code, sev)
        })
        .collect()
}

fn assert_clean_gate(repo: &Repo, what: &str, extra: &[&str]) {
    let (code, out, err) = gate(repo, extra);
    assert_eq!(code, 0, "{what}: {out}{err}");
    assert_eq!(number(&out, "new_errors"), 0, "{what}: {out}");
    assert_eq!(number(&out, "new_warnings"), 0, "{what}: {out}");
    // The base has errors, so "no new error" is not "no error".
    assert!(number(&out, "errors") > 0, "{what}: {out}");
    assert!(!out.contains("\"code\":\"SD503\""), "{what}: {out}");
}

// --- case 1 and 2: the upgrade is not new ---

#[test]
fn the_plain_upgrade_has_no_new_finding() {
    for (name, fixture, overlays) in CASES {
        let repo = upgraded(fixture, overlays, &[]);
        assert_clean_gate(&repo, name, &[]);
        // The text output has the same exit code.
        let (code, out, err) = repo.run(&["check", "--base", "main"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
    }
}

#[test]
fn the_upgrade_with_layout_has_no_new_finding() {
    for (name, fixture, overlays) in CASES {
        let repo = upgraded(fixture, overlays, &["--layout"]);
        assert_clean_gate(&repo, name, &[]);
    }
}

// --- case 3: the gate is still active ---

#[test]
fn a_broken_link_after_the_upgrade_is_the_one_new_error() {
    for layout in [&[][..], &["--layout"]] {
        let repo = upgraded("v05", &["harness-lean"], layout);
        let (code, out, _) = gate(&repo, &[]);
        assert_eq!(code, 0, "{out}");
        let before = number(&out, "errors");
        let mut text = repo.read("docs/irrigation/pumps.md");
        text.push_str("\nSee [the missing page](missing-page.md).\n");
        repo.write("docs/irrigation/pumps.md", &text);
        repo.commit_at("break a link", LATER_DATE);
        let (code, out, err) = gate(&repo, &[]);
        assert_eq!(code, 1, "{layout:?}: {out}{err}");
        assert_eq!(number(&out, "new_errors"), 1, "{layout:?}: {out}");
        assert_eq!(number(&out, "errors"), before + 1, "{layout:?}: {out}");
        let new: Vec<_> = listed(&out)
            .into_iter()
            .filter(|(_, sev)| sev == "error")
            .collect();
        assert_eq!(
            new,
            [("SD102".to_string(), "error".to_string())],
            "{layout:?}: {out}"
        );
    }
}

// --- case 4: the gate before the commit ---

#[test]
fn check_staged_before_the_commit_of_the_upgrade_has_no_new_finding() {
    for (name, fixture, overlays) in CASES {
        let repo = Repo::from_fixture(fixture, overlays);
        let (code, out, err) = repo.run(&["upgrade"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        repo.git(&["add", "-A"]);
        // The base is `HEAD`, which still pins 0.5.0.
        let (code, out, err) = repo.run(&["check", "--staged", "--json"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(number(&out, "new_errors"), 0, "{name}: {out}");
        assert!(!out.contains("\"code\":\"SD503\""), "{name}: {out}");
        // The same gate on the working tree, with no `--staged`.
        let (code, out, err) = repo.run(&["check", "--json"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert_eq!(number(&out, "new_errors"), 0, "{name}: {out}");
    }
}

// --- case 5: the upgrade with a policy ---

/// The branch `up` with the upgrade and `init --reconfigure` to the policy
/// `three-class`, committed.
fn upgraded_with_policy() -> Repo {
    let repo = upgraded("v05", &["harness-lean"], &[]);
    let answers = repo.dir.join("..").join(format!(
        "{}-answers.toml",
        repo.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(
        &answers,
        "harness = \"lean\"\nchange-policy = \"three-class\"\ngate = \"block\"\n",
    )
    .unwrap();
    let (code, out, err) = repo.run(&[
        "init",
        "--reconfigure",
        "--answers",
        answers.to_str().unwrap(),
    ]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.read("spec-dna.toml")
            .contains("policy = \"three-class\""),
        "{}",
        repo.read("spec-dna.toml")
    );
    repo.commit_at("policy", LATER_DATE);
    repo
}

/// No finding of the change checks (SD910 to SD921) in a JSON output.
fn assert_no_change_finding(out: &str, what: &str) {
    let codes = listed(out);
    assert!(
        codes
            .iter()
            .all(|(c, _)| !(c.as_str() >= "SD910" && c.as_str() <= "SD921")),
        "{what}: {codes:?}"
    );
}

#[test]
fn the_upgrade_and_a_policy_in_one_pull_request_have_no_change_finding() {
    let repo = upgraded_with_policy();
    let (code, out, err) = gate(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(number(&out, "new_errors"), 0, "{out}");
    assert_eq!(number(&out, "new_warnings"), 0, "{out}");
    // The note says why there is no lifecycle base. It is on stderr, so
    // `--json` stays valid, and `change_base` is null.
    assert!(err.contains("before 0.6.0"), "{err}");
    assert!(err.contains("main"), "{err}");
    assert!(out.contains("\"change_base\":null"), "{out}");
    // The change checks run as if the base had no lifecycle: no SD910 to
    // SD921. `--all` lists each finding of the tree, also the old ones.
    let (_, out, _) = gate(&repo, &["--all"]);
    assert_no_change_finding(&out, "--base main");
    assert!(!out.contains("\"code\":\"SD503\""), "{out}");
}

#[test]
fn a_lifecycle_base_on_another_commit_than_the_gate_base_has_no_change_finding() {
    // The hook of the repository runs `check --staged` or `check` with no
    // `--base` on each later commit of the branch `up`: the gate base is
    // `HEAD`, which pins 0.6.0, and the lifecycle base is the merge base with
    // `origin/HEAD`, which is `main` and pins 0.5.0.
    let repo = upgraded_with_policy();
    origin_head(&repo.dir, "main");
    for args in [&["check", "--json"][..], &["check", "--staged", "--json"]] {
        let (code, out, err) = repo.run(args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(out.contains("\"change_base\":null"), "{args:?}: {out}");
        assert!(err.contains("the merge base"), "{args:?}: {err}");
        assert!(err.contains("before 0.6.0"), "{args:?}: {err}");
    }
    let (_, out, _) = repo.run(&["check", "--all", "--json"]);
    assert_no_change_finding(&out, "no --base");
}

// --- the table of sums ---

/// Appends `[spec-dna-sha256]` for `pin` to the configuration, and commits.
fn with_sums(repo: &Repo, pin: &str) {
    let mut text = repo.read("spec-dna.toml");
    text.push('\n');
    text.push_str(&sums_table(pin, 'a'));
    repo.write("spec-dna.toml", &text);
    repo.commit_at("sums", "2026-01-15T13:00:00+00:00");
}

#[test]
fn a_project_with_sums_has_no_new_finding_after_upgrade_with_a_pin_file() {
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    with_sums(&repo, "0.5.0");
    repo.git(&["switch", "-q", "-c", "up"]);
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'b')));
    let (code, out, err) = repo.run(&["upgrade", "--pin-file", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit_at("upgrade", UPGRADE_DATE);
    assert_clean_gate(&repo, "sums", &[]);
}

#[test]
fn a_head_with_no_sums_over_a_base_with_sums_gives_exit_code_3() {
    // `upgrade` refuses a project with sums and no `--pin-file`, so this head
    // is a hand edit. The plan of the base cannot take the sums of the head.
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    with_sums(&repo, "0.5.0");
    repo.git(&["switch", "-q", "-c", "up"]);
    let (dir, file) = pin_file(VERSION, Some(&sums_table(VERSION, 'b')));
    let (code, out, err) = repo.run(&["upgrade", "--pin-file", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    let mut keep = true;
    let text: String = repo
        .read("spec-dna.toml")
        .lines()
        .filter(|l| {
            if l.starts_with('[') {
                keep = *l != "[spec-dna-sha256]";
            }
            keep
        })
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(!text.contains("spec-dna-sha256"), "{text}");
    repo.write("spec-dna.toml", &text);
    repo.commit_at("upgrade", UPGRADE_DATE);
    let (code, out, err) = gate(&repo, &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("the base `main` pins `0.5.0`, and upgrade cannot plan it"),
        "{err}"
    );
    assert!(err.contains("--pin-file"), "{err}");
}

// --- case 6: a base that no step of upgrade takes ---

#[test]
fn a_base_pin_before_0_4_0_gives_exit_code_3_and_names_0_4_2() {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        "spec-dna = \"0.3.3\"\n[[roots]]\npath = \"docs\"\n",
    );
    repo.write("docs/a.md", "# A\n\nA text.\n");
    repo.commit_at("base", FIXTURE_DATE);
    repo.git(&["switch", "-q", "-c", "up"]);
    repo.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n[[roots]]\npath = \"docs\"\n"),
    );
    repo.commit_at("head", UPGRADE_DATE);
    for extra in [&[][..], &["--all"]] {
        let (code, out, err) = gate(&repo, extra);
        assert_eq!(code, 3, "{out}{err}");
        assert!(err.contains("0.3.3"), "{err}");
        assert!(err.contains("0.4.2"), "{err}");
        assert!(err.contains("main"), "{err}");
    }
}

#[test]
fn a_base_that_upgrade_cannot_plan_gives_exit_code_3_with_the_reason() {
    let repo = Repo::from_fixture("v05", &[]);
    let toml = repo.read("spec-dna.toml");
    repo.write("spec-dna.toml", &format!("{toml}\nbogus-key = 1\n"));
    repo.commit_at("bad base", "2026-01-15T13:00:00+00:00");
    repo.git(&["switch", "-q", "-c", "up"]);
    repo.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n[[roots]]\npath = \"docs\"\n"),
    );
    repo.commit_at("head", UPGRADE_DATE);
    let (code, out, err) = gate(&repo, &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("the base `main` pins `0.5.0`, and upgrade cannot plan it"),
        "{err}"
    );
    assert!(err.contains("unknown key"), "{err}");
}

// --- case 7: a later change keeps SD503 ---

#[test]
fn a_later_config_change_with_a_document_gives_sd503() {
    // The first pull request is merged: the base on `main` pins 0.6.0.
    let repo = Repo::from_fixture("v05", &["harness-lean"]);
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit_at("upgrade", UPGRADE_DATE);
    repo.git(&["switch", "-q", "-c", "later"]);
    let toml = repo.read("spec-dna.toml");
    repo.write("spec-dna.toml", &format!("{toml}\n# Lint by default.\n"));
    // A configuration change alone is fine.
    repo.commit_at("config", LATER_DATE);
    let (code, out, err) = gate(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("\"code\":\"SD503\""), "{out}");
    // A document in the same change gives SD503.
    let mut text = repo.read("docs/irrigation/pumps.md");
    text.push_str("\nA new line.\n");
    repo.write("docs/irrigation/pumps.md", &text);
    repo.commit_at("document", "2026-01-18T12:00:00+00:00");
    let (code, out, err) = gate(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    let found = listed(&out);
    assert!(found.iter().any(|(c, _)| c == "SD503"), "{found:?}");
    // No note about a base before 0.6.0: the base is a 0.6 tree.
    assert!(!err.contains("before 0.6.0"), "{err}");
}

// --- top files and areas (0.6.1): the gate against the 0.5 base ---

#[test]
fn the_upgrade_with_top_files_and_areas_has_no_new_finding() {
    // The base plan runs with no `--layout`: there the roadmap is an intent
    // and the overview is a `research` document. The head has the top files
    // and the area folder, and each id still resolves. The area of the test is
    // `pumps`: `irrigation/sensors` has a link with a missing anchor in
    // `valves.md`. The link text changes with the move of its target, so its
    // old warning SD602 reads as new in the gate.
    let repo = Repo::from_fixture("v05", &["top-files"]);
    repo.git(&["switch", "-q", "-c", "up"]);
    let dir = std::env::temp_dir().join(format!("spec-dna-areas-first-pr-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("areas.toml");
    fs::write(&file, "[areas]\n\"irrigation/pumps\" = \"pumps\"\n").unwrap();
    let (code, out, err) = repo.run(&["upgrade", "--layout", "--areas", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit_at("upgrade", UPGRADE_DATE);
    assert!(repo.dir.join("docs/roadmap.md").is_file());
    assert!(repo.dir.join("docs/requirements/pumps/pumps.md").is_file());
    assert_clean_gate(&repo, "top files and areas", &[]);
}
