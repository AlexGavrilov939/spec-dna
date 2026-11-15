//! Tests of the fixes of the check of PR 5: the messages of a hand edit, the
//! pin message, the `to commit:` line, and a file in the way of a folder.
//! Each message of a hand edit is followed literally: the test makes the edit
//! that the message names, and the next run must give exit code 0 (F1).

mod common;

use common::*;

/// The 10 types of 0.5.
const TYPES_05: [&str; 10] = [
    "spec",
    "change",
    "decision",
    "guide",
    "reference",
    "note",
    "runbook",
    "incident",
    "contract",
    "invariant",
];

const ROOT: &str = "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"note\"\n";

/// A project of 0.5.0 with the configuration `toml` after the pin line and
/// one root `docs` that holds one note.
fn old_project(toml: &str) -> Repo {
    let repo = Repo::new();
    repo.write("spec-dna.toml", &format!("spec-dna = \"0.5.0\"\n{toml}"));
    repo.write("docs/a.md", "# A\n\nText.\n");
    repo.commit("project");
    repo
}

/// The message of the refusal of a hand edit: exit code 3, no write.
fn refused(repo: &Repo) -> String {
    let before = tree_bytes(&repo.dir);
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("cannot edit this form"), "{err}");
    // The tree is dirty after the edit, and `upgrade` reads `HEAD`.
    assert!(err.contains("commit the change"), "{err}");
    assert_eq!(before, tree_bytes(&repo.dir), "no write");
    err
}

/// The text after the 0.5 reader: no name of 0.6 in the message.
fn assert_no_new_name(msg: &str) {
    for name in [
        "procedure",
        "requirements",
        "intent",
        "research",
        "interface",
    ] {
        assert!(!msg.contains(name), "the message names `{name}`: {msg}");
    }
}

#[test]
fn a_dotted_budget_message_names_the_header_with_the_old_name() {
    let repo = old_project(&format!(
        "{ROOT}[budgets]\nguide.soft = 3000\nguide.hard = 6000\n"
    ));
    let msg = refused(&repo);
    assert!(msg.contains("[budgets.guide]"), "{msg}");
    assert!(msg.contains("`guide` as it is now"), "{msg}");
    assert_no_new_name(&msg);
    // Follow the message: the budget is a table with a header and the old name.
    let text = repo.read("spec-dna.toml").replace(
        "[budgets]\nguide.soft = 3000\nguide.hard = 6000\n",
        "[budgets.guide]\nsoft = 3000\nhard = 6000\n",
    );
    repo.write("spec-dna.toml", &text);
    repo.commit("by hand");
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.read("spec-dna.toml").contains("[budgets.procedure]"),
        "{}",
        repo.read("spec-dna.toml")
    );
}

/// Follows the message of a many-lines `types`: the array goes on one line.
fn follow_types_message(repo: &Repo, msg: &str, names: &[&str]) {
    assert!(msg.contains("an array on one line"), "{msg}");
    assert!(!msg.contains("same number of elements"), "{msg}");
    assert_no_new_name(msg);
    let list: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    let mut text = repo.read("spec-dna.toml");
    let from = text.find("types = [").unwrap();
    let to = from + text[from..].find("]\n").unwrap() + 2;
    text.replace_range(from..to, &format!("types = [{}]\n", list.join(", ")));
    repo.write("spec-dna.toml", &text);
    repo.commit("by hand");
}

fn many_lines(names: &[&str]) -> String {
    let mut s = String::from("types = [\n");
    for n in names {
        s.push_str(&format!("  \"{n}\",\n"));
    }
    s.push_str("]\n");
    s
}

#[test]
fn a_types_list_of_many_lines_that_merges_says_one_line() {
    let repo = old_project(&format!("{}{ROOT}", many_lines(&TYPES_05)));
    let msg = refused(&repo);
    follow_types_message(&repo, &msg, &TYPES_05);
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_types_list_of_many_lines_that_needs_research_says_one_line() {
    let repo = Repo::new();
    repo.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"0.5.0\"\n{}{ROOT}",
            many_lines(&["spec", "change"])
        ),
    );
    repo.write("docs/c.md", "---\ntype: change\nstatus: done\n---\n# C\n");
    repo.commit("project");
    let msg = refused(&repo);
    follow_types_message(&repo, &msg, &["spec", "change"]);
    let (code, out, err) = repo.run(&["upgrade"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.read("spec-dna.toml").contains("research"));
}

// --- F2: the pin message names a place outside the repository ---

/// The path of the pin file in the command of the message of a pin mismatch.
fn pin_path_of(msg: &str) -> String {
    let at = msg
        .find("`spec-dna upgrade --pin-file ")
        .expect("a command");
    let rest = &msg[at + "`spec-dna upgrade --pin-file ".len()..];
    rest[..rest.find('`').unwrap()].to_string()
}

#[test]
fn the_pin_message_gives_a_command_that_works() {
    // The project is a folder of `outer`, so `..` is a place outside of it.
    let outer = Repo::empty();
    let repo = Repo {
        dir: outer.dir.join("project"),
    };
    std::fs::create_dir_all(&repo.dir).unwrap();
    git(&repo.dir, &["init", "-q", "-b", "main"]);
    let table = sums_table("0.5.0", 'a');
    repo.write(
        "spec-dna.toml",
        &format!("spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\n\n{table}"),
    );
    repo.write("docs/a.md", "# A\n\nText.\n");
    repo.commit("project");
    let (code, _, err) = repo.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("outside the repository"), "{err}");
    // Follow the message: the file is at the path of the command.
    let path = pin_path_of(&err);
    assert!(path.starts_with("../"), "{path}");
    let text = format!("spec-dna = \"{VERSION}\"\n\n{}", sums_table(VERSION, 'b'));
    std::fs::write(repo.dir.join(&path), text).unwrap();
    let (code, out, err) = repo.run(&["upgrade", "--pin-file", &path]);
    assert_eq!(code, 0, "{out}{err}");
}

// --- F4: a file is in the way of a folder ---

#[test]
fn a_file_on_the_path_of_a_folder_is_named_and_is_no_link() {
    let repo = Repo::from_fixture("v05", &[]);
    repo.write("docs/intent", "a plain file\n");
    repo.commit("file");
    let before = tree_bytes(&repo.dir);
    for args in [&["upgrade", "--dry-run"][..], &["upgrade"]] {
        let (code, out, err) = repo.run(args);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("`docs/intent` is a file"), "{err}");
        assert!(err.contains("folder"), "{err}");
        assert!(!err.contains("link"), "{err}");
        assert!(!err.contains("os error"), "{err}");
        assert!(err.contains("commit the change"), "{err}");
    }
    assert_eq!(before, tree_bytes(&repo.dir), "no write");
}

// --- F3: the `to commit:` command works from a folder of the repository ---

#[test]
fn the_commit_hint_works_from_a_subfolder() {
    let repo = Repo::from_fixture("v05", &[]);
    let sub = repo.dir.join("docs/guides");
    let out = std::process::Command::new(BIN)
        .args(["upgrade"])
        .current_dir(&sub)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(out.status.code(), Some(0), "{text}");
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("to commit: "))
        .expect("a `to commit:` line");
    let run = std::process::Command::new("sh")
        .args(["-c", line])
        .current_dir(&sub)
        .output()
        .unwrap();
    assert!(run.status.success(), "{line}: {run:?}");
    // Each changed path is staged and has no more change: the first column
    // is a letter, and the second column is a space (no `RM`, no `??`).
    let status = repo.git_out(&["status", "--porcelain"]);
    assert!(!status.is_empty());
    for l in status.lines() {
        let col = l.as_bytes();
        assert!(
            col[0] != b' ' && col[0] != b'?' && col[1] == b' ',
            "{line}\n{status}"
        );
    }
}
