//! The file `.claude/agents/Explore.md` replaces the built-in `Explore`
//! subagent of Claude Code (ADR 0008). `init` says so when it writes the file,
//! and says how to remove it. It says nothing when the file is unchanged.

mod common;

use common::Repo;

/// The words that the note must have. The preview diff of the team page has
/// the words "replaces the built-in `Explore` subagent" too, so the test
/// matches the `note:` line.
const OVERRIDE: &str = "note: `.claude/agents/Explore.md` replaces the built-in `Explore` subagent";
const REMOVE: &str = "researcher = { claude-code = \"none\" }";

fn count(out: &str, text: &str) -> usize {
    out.matches(text).count()
}

#[test]
fn init_says_that_explore_replaces_the_built_in_subagent() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(".claude/agents/Explore.md").exists(), "{out}");
    assert_eq!(count(&out, OVERRIDE), 1, "{out}");
    assert!(out.contains(REMOVE), "{out}");
    assert!(out.contains("spec-dna init --reconfigure"), "{out}");
}

#[test]
fn the_preview_has_the_note_too() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(count(&out, OVERRIDE), 1, "{out}");
    assert!(!r.dir.join(".claude/agents/Explore.md").exists());
}

#[test]
fn a_second_run_with_an_unchanged_file_does_not_repeat_the_note() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let before = r.read(".claude/agents/Explore.md");
    for args in [
        vec!["init", "--non-interactive"],
        vec!["init", "--reconfigure", "--non-interactive"],
    ] {
        let (code, out, err) = r.run(&args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(!out.contains(OVERRIDE), "{args:?}: {out}");
        assert_eq!(r.read(".claude/agents/Explore.md"), before);
    }
}

#[test]
fn no_note_without_the_file() {
    let r = Repo::new();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{}\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n\n[harness.roles]\nresearcher = {{ claude-code = \"none\" }}\n",
            common::VERSION
        ),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".claude/agents/Explore.md").exists(), "{out}");
    assert!(!out.contains(OVERRIDE), "{out}");
}

/// The `harness` object of `--json` has the field `notes`, and the note of
/// `Explore.md` is in it.
#[test]
fn the_json_has_the_note_in_harness_notes() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--dry-run", "--non-interactive", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let harness = out.split("\"harness\":{").nth(1).expect("a harness object");
    let order: Vec<usize> = [
        "level",
        "files",
        "kept",
        "warnings",
        "notes",
        "request_bytes",
    ]
    .iter()
    .map(|k| {
        harness
            .find(&format!("\"{k}\":"))
            .unwrap_or_else(|| panic!("{k}: {out}"))
    })
    .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{out}");
    let notes = harness.split("\"notes\":[").nth(1).unwrap();
    let notes = notes.split(']').next().unwrap();
    assert!(
        notes.contains("`.claude/agents/Explore.md` replaces the built-in `Explore` subagent"),
        "{out}"
    );
    assert!(
        notes.contains("researcher = { claude-code = \\\"none\\\" }"),
        "{out}"
    );
    // No file is written by the dry run.
    assert!(!r.dir.join(".claude/agents/Explore.md").exists());
    // A project that disables the role has an empty list.
    let r = Repo::new();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{}\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n\n[harness.roles]\nresearcher = {{ claude-code = \"none\" }}\n",
            common::VERSION
        ),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let harness = out.split("\"harness\":{").nth(1).expect("a harness object");
    assert!(harness.contains("\"notes\":[]"), "{out}");
}
