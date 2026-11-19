//! Tests of the pin classes and of the pin mismatch message (step 5, slice 2,
//! D134). The pin of a project is a release version. The class of the pin
//! against the binary decides the last sentence of the message. Each command
//! gives the same message, and `init` and `init --reconfigure` stop on it with
//! exit code 3 and write nothing. The class "0.6.0 to before the binary" has
//! no case here, because the binary is 0.6.0: a unit test of `src/config.rs`
//! covers it.

mod common;

use common::*;
use std::fs;

/// The fixtures of the upgrade: each pins a version before the binary.
const FIXTURES: &[(&str, &str)] = &[("v04", "0.4.2"), ("v05", "0.5.0"), ("brownfield", "0.5.0")];

/// A repository with a configuration that pins `pin`, and `extra` after the
/// pin line. The configuration has 1 root.
fn pinned(pin: &str, extra: &str) -> Repo {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{pin}\"\n{extra}[[roots]]\npath = \"docs\"\n"),
    );
    repo.write("docs/a.md", "# A\n");
    repo.commit("pin");
    repo
}

/// The sentence that moves a project from a pin of 0.4.0 or later.
fn upgrade_sentence() -> String {
    format!("To move the project to {VERSION}, run `spec-dna upgrade`.")
}

#[test]
fn init_stops_on_an_old_pin_and_names_upgrade() {
    for (name, pin) in FIXTURES {
        for args in [&["init"][..], &["init", "--reconfigure"]] {
            let repo = Repo::from_fixture(name, &[]);
            let before = tree_bytes(&repo.dir);
            let (code, out, err) = repo.run(args);
            assert_eq!(code, 3, "{name} {args:?}: {out}{err}");
            assert!(
                err.contains(&format!(
                    "spec-dna.toml pins version {pin}, but this binary is {VERSION}."
                )),
                "{name} {args:?}: {err}"
            );
            assert!(err.contains(&upgrade_sentence()), "{name} {args:?}: {err}");
            assert!(
                !err.contains("and run `spec-dna init`"),
                "{name} {args:?}: init is not the way to move the pin: {err}"
            );
            assert_eq!(out, "", "{name} {args:?}");
            assert_eq!(before, tree_bytes(&repo.dir), "{name} {args:?}: no write");
            assert_eq!(
                repo.git_out(&["status", "--porcelain"]),
                "",
                "{name} {args:?}"
            );
        }
    }
}

#[test]
fn init_stops_on_an_old_pin_with_the_harness_files_too() {
    let repo = Repo::from_fixture("v05", &["harness-full"]);
    let before = tree_bytes(&repo.dir);
    for args in [
        &["init"][..],
        &["init", "--reconfigure"],
        &["init", "--dry-run"],
    ] {
        let (code, _, err) = repo.run(args);
        assert_eq!(code, 3, "{args:?}: {err}");
        assert!(err.contains(&upgrade_sentence()), "{args:?}: {err}");
    }
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn each_command_gives_the_same_message() {
    let repo = Repo::from_fixture("v05", &[]);
    let (_, _, want) = repo.run(&["check"]);
    assert!(want.contains(&upgrade_sentence()), "{want}");
    let commands: &[&[&str]] = &[
        &["init"],
        &["init", "--reconfigure"],
        &["context", "a"],
        &["show", "a"],
        &["new", "decision", "a"],
        &["layer", "list"],
        &["mv", "a", "b"],
        &["history", "a"],
        &["index", "docs"],
        &["finish", "a"],
        &["next", "a"],
        &["status"],
        &["log"],
        &["stats", "--sdlc"],
        &["eval", "run"],
    ];
    for args in commands {
        let (code, out, err) = repo.run(args);
        assert_eq!(code, 3, "{args:?}: {out}{err}");
        assert_eq!(err, want, "{args:?}");
    }
}

#[test]
fn the_last_sentence_follows_the_class_of_the_pin() {
    // Before 0.4.0: the project moves to 0.4.2 first.
    for pin in ["0.3.3", "0.1.0"] {
        let repo = pinned(pin, "");
        let (code, _, err) = repo.run(&["check"]);
        assert_eq!(code, 3, "{pin}: {err}");
        assert!(
            err.ends_with("  First move the project to 0.4.2 with the 0.4.2 binary and `spec-dna init`, then run `spec-dna upgrade`.\n"),
            "{pin}: {err}"
        );
        let (code, _, err) = repo.run(&["init"]);
        assert_eq!(code, 3, "{pin}: {err}");
        assert!(
            err.contains("First move the project to 0.4.2"),
            "{pin}: {err}"
        );
    }
    // 0.4.0 to before the binary: `upgrade`.
    for pin in ["0.4.0", "0.4.2", "0.5.0", "0.5.9"] {
        let (code, _, err) = pinned(pin, "").run(&["check"]);
        assert_eq!(code, 3, "{pin}: {err}");
        assert!(
            err.ends_with(&format!("  {}\n", upgrade_sentence())),
            "{pin}: {err}"
        );
    }
    // The same, with `[spec-dna-sha256]`: the file of the release.
    let table = "\n[spec-dna-sha256]\n\"x\" = 1\n";
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        &format!("spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\n{table}"),
    );
    let (code, _, err) = repo.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(
        err.ends_with(&format!("  To move the project to {VERSION}, download https://github.com/AlexGavrilov939/spec-dna/releases/download/v{VERSION}/spec-dna-pin.toml to a place outside the repository, for example the folder above it, verify it with `gh attestation verify`, and run `spec-dna upgrade --pin-file ../spec-dna-pin.toml` in the root folder of the repository.\n")),
        "{err}"
    );
    // After the binary: no sentence to move the project.
    let pin = "9.9.9";
    let (code, _, err) = pinned(pin, "").run(&["check"]);
    assert_eq!(code, 3, "{pin}: {err}");
    assert!(err.contains(&format!("pins version {pin}")), "{pin}: {err}");
    assert!(err.contains("To use "), "{pin}: {err}");
    assert!(!err.contains("To move the project"), "{pin}: {err}");
    assert!(!err.contains("spec-dna upgrade"), "{pin}: {err}");
    assert!(
        err.ends_with(&format!("--tag v{pin} spec-dna\n")),
        "{pin}: {err}"
    );
    // Not 3 numbers: the sentence of 0.5. `init` stays the way to set the pin.
    for pin in ["0.5", "0.5.0-rc1", "latest", "5"] {
        let (code, _, err) = pinned(pin, "").run(&["check"]);
        assert_eq!(code, 3, "{pin}: {err}");
        assert!(
            err.ends_with(&format!("  To move the project to {VERSION}, set `spec-dna = \"{VERSION}\"` in spec-dna.toml and run `spec-dna init`.\n")),
            "{pin}: {err}"
        );
    }
}

#[test]
fn the_pin_of_the_binary_has_no_mismatch() {
    let repo = pinned(VERSION, "");
    for args in [&["check"][..], &["init", "--dry-run"]] {
        let (code, out, err) = repo.run(args);
        assert!(!err.contains("pins version"), "{args:?}: {out}{err}");
        assert_ne!(code, 3, "{args:?}: {out}{err}");
    }
}

#[test]
fn a_pin_before_a_parse_error_still_gives_the_message() {
    // A key of another version does not hide the mismatch (ADR 0003).
    let repo = pinned("0.5.0", "future-key = true\n");
    let (code, _, err) = repo.run(&["init"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains(&upgrade_sentence()), "{err}");
    assert!(!err.contains("unknown key"), "{err}");
    // The same for a pin after the binary: the message, with no sentence to
    // move the project, and not the parse error.
    let repo = pinned("9.9.9", "future-key = true\n");
    for args in [&["check"][..], &["init"], &["show", "a"], &["context", "a"]] {
        let (code, _, err) = repo.run(args);
        assert_eq!(code, 3, "{args:?}: {err}");
        assert!(err.contains("pins version 9.9.9"), "{args:?}: {err}");
        assert!(!err.contains("unknown key"), "{args:?}: {err}");
        assert!(!err.contains("To move the project"), "{args:?}: {err}");
    }
    // A pin in single quotes and a file with a byte order mark.
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        "\u{feff}spec-dna = '0.5.0'\r\nfuture-key = true\r\n[[roots]]\r\npath = \"docs\"\r\n",
    );
    let (code, _, err) = repo.run(&["init"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("pins version 0.5.0"), "{err}");
    assert!(err.contains(&upgrade_sentence()), "{err}");
}

// --- `--pin-file` (D135): the cases that waited for the command ---

fn with_sums(repo: &Repo, pin: &str) {
    let mut text = repo.read("spec-dna.toml");
    text.push('\n');
    text.push_str(&sums_table(pin, 'a'));
    repo.write("spec-dna.toml", &text);
    repo.commit("sums");
}

fn upgrade(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["upgrade"];
    a.extend_from_slice(args);
    repo.run(&a)
}

#[test]
fn a_table_of_sums_needs_a_pin_file_with_the_sums_of_the_new_version() {
    let repo = Repo::from_fixture("v05", &[]);
    with_sums(&repo, "0.5.0");
    let before = tree_bytes(&repo.dir);
    // No flag.
    let (code, out, err) = upgrade(&repo, &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("--pin-file"), "{err}");
    // Another version in the file.
    let (dir, file) = pin_file("0.5.9", Some(&sums_table("0.5.9", 'b')));
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains(VERSION), "{err}");
    let _ = fs::remove_dir_all(dir);
    // A bad sum: 63 characters.
    let bad = sums_table(VERSION, 'b').replacen(&"b".repeat(64), &"b".repeat(63), 1);
    let (dir, file) = pin_file(VERSION, Some(&bad));
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("64"), "{err}");
    let _ = fs::remove_dir_all(dir);
    // A file with no table.
    let (dir, file) = pin_file(VERSION, None);
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    assert_eq!(code, 3, "{out}{err}");
    let _ = fs::remove_dir_all(dir);
    // A file that is not there.
    let (code, out, err) = upgrade(&repo, &["--pin-file", "/nonexistent/pin.toml"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(before, tree_bytes(&repo.dir));
}

#[test]
fn a_pin_file_of_another_version_stops_a_project_with_no_table_too() {
    let repo = Repo::from_fixture("v05", &[]);
    let (dir, file) = pin_file("0.5.9", Some(&sums_table("0.5.9", 'b')));
    let (code, out, err) = upgrade(&repo, &["--pin-file", file.to_str().unwrap()]);
    let _ = fs::remove_dir_all(dir);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
}

// --- SD503 and a pin that moves to a later version (0.7.1, B3) ---

/// The text of a configuration that pins `pin`, with `extra` after the roots.
fn config_of(pin: &str, extra: &str) -> String {
    format!("spec-dna = \"{pin}\"\n[[roots]]\npath = \"docs\"\n{extra}")
}

/// A repository whose base `main` pins `base_pin`. The branch `feature` has the
/// configuration `head` and a changed script, in 1 commit. The result of
/// `check --base main --json` is the gate of the pull request.
fn upgraded_branch(base_pin: &str, head: &str) -> (Repo, String) {
    let repo = Repo::new();
    repo.write("spec-dna.toml", &config_of(base_pin, ""));
    repo.write("docs/a.md", "# A\n");
    repo.write("src/a.sh", "echo 1\n");
    repo.commit("base");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("spec-dna.toml", head);
    repo.write("src/a.sh", "echo 2\n");
    repo.commit("upgrade and work");
    let (code, out, err) = repo.run(&["check", "--base", "main", "--json"]);
    assert_ne!(code, 3, "{out}{err}");
    (repo, out)
}

#[test]
fn a_pin_that_moves_to_a_later_version_is_no_sd503_with_other_work() {
    let (_, out) = upgraded_branch("0.6.1", &config_of(VERSION, ""));
    assert!(!out.contains("\"code\":\"SD503\""), "{out}");
}

#[test]
fn a_pin_that_moves_to_a_later_version_with_a_sums_table_is_no_sd503() {
    // The table of the release file: valid for the pin of the binary.
    let table = format!("\n{}", sums_table(VERSION, 'a'));
    let (_, out) = upgraded_branch("0.6.1", &config_of(VERSION, &table));
    assert!(!out.contains("\"code\":\"SD503\""), "{out}");
}

#[test]
fn a_pin_and_another_edit_of_the_configuration_is_sd503() {
    let (_, out) = upgraded_branch("0.6.1", &config_of(VERSION, "[rules]\nSD608 = \"off\"\n"));
    assert!(out.contains("\"code\":\"SD503\""), "{out}");
}

#[test]
fn a_move_to_the_same_pin_with_a_new_table_is_sd503() {
    // The pin does not move: a change of the table alone counts as before.
    let table = format!("\n{}", sums_table(VERSION, 'a'));
    let repo = Repo::new();
    repo.write("spec-dna.toml", &config_of(VERSION, ""));
    repo.write("docs/a.md", "# A\n");
    repo.commit("base");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("spec-dna.toml", &config_of(VERSION, &table));
    repo.write("src/a.sh", "echo 2\n");
    repo.commit("work");
    let (_, out, _) = repo.run(&["check", "--base", "main", "--json"]);
    assert!(out.contains("\"code\":\"SD503\""), "{out}");
}

#[test]
fn a_base_pin_after_the_head_pin_is_sd503() {
    // The head binary is older than the base pin: the move goes back.
    let (_, out) = upgraded_branch("9.9.9", &config_of(VERSION, ""));
    assert!(out.contains("\"code\":\"SD503\""), "{out}");
}
