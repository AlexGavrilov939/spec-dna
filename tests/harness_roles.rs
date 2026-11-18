//! Tests of the role names of the harness (step 4, S2): `reviewer` is now
//! `verifier`, `spec-writer` ended, and `code-simplifier` is new at level
//! `full`. Each test uses a temporary repository. No test reads or writes a
//! file of this repository.

mod common;

use common::{Repo, md_lint, normalize_marker};
use std::fs;

const LEAN_FILES: [&str; 4] = [
    ".claude/agents/fallback-advisor.md",
    ".claude/agents/verifier.md",
    ".claude/agents/Explore.md",
    ".claude/agents/model-team.md",
];

/// The agent file of spec-dna 0.5.0 for `reviewer`. The hash covers the text
/// above the marker, so this text is an unedited generated file.
const OLD_CHECKER: &str = "---
name: checker
description: Reviews finished work independently. It runs the tests, examines the diff, probes edge cases and lists each defect. It never edits. Use it before you report that a task is complete.
model: opus
effort: high
tools: Read, Bash, Monitor, TaskStop
---

You review work that another agent reports as complete. Do not change files or git state. Put temporary files in a temporary directory, and remove them at the end.

1. Find the change: `git status`, `git diff`, and the files in the brief.
2. Run the tests and checks that cover the change. Use the commands of the project instruction files.
3. Probe the change with bad input, limits, missing files and errors. Run each probe.
4. Compare the change with the brief. Report missing work and extra work.

Report:
- Each defect: the file and line or the command, the expected result, and the actual result.
- Each check that you could not run, with the reason.
- If you found no defect, list the commands that you ran.

<!-- spec-dna harness v0.5.0 h:95b6014e6608de30 -->
";

/// The agent file of spec-dna 0.5.0 for `spec-writer`.
const OLD_SPEC_WRITER: &str = "---
name: spec-writer
description: Writes and updates the specifications of the project with spec-dna. Use it for a new spec, decision or runbook, or to keep the specs true after a code change.
model: sonnet
effort: high
tools: Read, Grep, Glob, Bash, Edit, Write
---

You keep the specifications true. Use spec-dna for each step.

1. Run `spec-dna context <files>` for the files of the change. Read a listed section with `spec-dna show <id>#<anchor>` only when you need it.
2. Run `spec-dna show --conventions <type>` before you edit a document of that type.
3. For a new document, run `spec-dna new <type> <id>`. The folder of a layer gives the type, so write no `type:` line there. A top file has no id (`spec-dna new terms`), and a change has a folder (`spec-dna new intent <slug>`). Then replace each placeholder.
4. Change an accepted decision only in its amendment section, or write a new decision that supersedes it.
5. Run `spec-dna check`. Fix each error.

Report: the documents that you changed, and the output of `spec-dna check`. Do not commit.

<!-- spec-dna harness v0.5.0 h:13263900c4a2a0bc -->
";

/// Runs `init` with an answers file and the other arguments. The answers file
/// is outside the repository.
fn init_with(r: &Repo, answers: &str, extra: &[&str]) -> (i32, String, String) {
    let path = format!("{}-answers.toml", r.dir.display());
    fs::write(&path, answers).unwrap();
    let mut args = vec!["init", "--answers", path.as_str()];
    args.extend_from_slice(extra);
    let out = r.run(&args);
    let _ = fs::remove_file(&path);
    out
}

fn golden(name: &str) -> String {
    let p = format!("{}/tests/golden/harness/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

fn full_repo() -> Repo {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "harness = \"full\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    r
}

fn project_with(harness: &str) -> Repo {
    let r = Repo::with_config(&format!("\n[[roots]]\npath = \"docs\"\n\n{harness}"));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    r
}

#[test]
fn lean_writes_the_verifier_and_no_checker() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in LEAN_FILES {
        assert!(r.dir.join(f).exists(), "{f}");
        assert!(out.contains(&format!("wrote {f}\n")), "{out}");
    }
    for f in [
        "checker.md",
        "spec-writer.md",
        "implementer.md",
        "code-simplifier.md",
    ] {
        assert!(!r.dir.join(".claude/agents").join(f).exists(), "{f}");
    }
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("\nverifier = { claude-code = \"opus\", effort = \"high\" }\n"),
        "{cfg}"
    );
    assert!(!cfg.contains("reviewer"), "{cfg}");
    assert!(!cfg.contains("spec-writer"), "{cfg}");
    assert!(!cfg.contains("code-simplifier"), "{cfg}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn full_writes_the_implementer_and_the_code_simplifier() {
    let r = full_repo();
    for f in LEAN_FILES {
        assert!(r.dir.join(f).exists(), "{f}");
    }
    assert!(r.dir.join(".claude/agents/implementer.md").exists());
    assert!(r.dir.join(".claude/agents/code-simplifier.md").exists());
    assert!(!r.dir.join(".claude/agents/checker.md").exists());
    assert!(!r.dir.join(".claude/agents/spec-writer.md").exists());
    let cfg = r.read("spec-dna.toml");
    // The order of `Role::ALL`.
    let at = |k: &str| {
        cfg.find(&format!("\n{k} = {{"))
            .unwrap_or_else(|| panic!("{k}\n{cfg}"))
    };
    let keys = [
        "orchestrator",
        "advisor",
        "verifier",
        "researcher",
        "implementer",
        "code-simplifier",
        "helper",
    ];
    for w in keys.windows(2) {
        assert!(at(w[0]) < at(w[1]), "{} before {}\n{cfg}", w[0], w[1]);
    }
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_agent_files_match_the_golden_files() {
    let r = full_repo();
    for (path, name) in [
        (".claude/agents/fallback-advisor.md", "fallback-advisor.md"),
        (".claude/agents/verifier.md", "verifier.md"),
        (".claude/agents/Explore.md", "Explore.md"),
        (".claude/agents/implementer.md", "implementer.md"),
        (".claude/agents/code-simplifier.md", "code-simplifier.md"),
        (".claude/agents/model-team.md", "model-team.md"),
    ] {
        assert_eq!(normalize_marker(&r.read(path)), golden(name), "{path}");
        // The pre-commit hook lints the golden file with markdownlint-cli2.
        // This test reads the same file with the helper.
        assert_eq!(md_lint(&golden(name)), [], "{name}");
    }
}

#[test]
fn the_skill_of_the_adapter_matches_its_golden_file() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "adapters = [\"skill\"]\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    // The version of the begin marker is not part of the golden file.
    let text = r.read(".claude/skills/spec-dna/SKILL.md");
    let (head, tail) = text
        .split_once("<!-- spec-dna:begin skill v")
        .expect("begin marker");
    let (_, tail) = tail.split_once(" h:").expect("hash");
    let normalized = format!("{head}<!-- spec-dna:begin skill v{{VERSION}} h:{tail}");
    let want = golden("skills/spec-dna.md");
    assert_eq!(normalized, want);
    assert_eq!(md_lint(&want), [], "skills/spec-dna.md");
}

#[test]
fn the_texts_of_the_new_agents_follow_the_plan() {
    let r = full_repo();
    let v = r.read(".claude/agents/verifier.md");
    assert!(v.contains("\nname: verifier\n"), "{v}");
    assert!(v.contains("\ntools: Read, Bash\n"), "{v}");
    assert!(v.contains("Do not fix anything; report only."), "{v}");
    let c = r.read(".claude/agents/code-simplifier.md");
    assert!(c.contains("\nname: code-simplifier\n"), "{c}");
    assert!(c.contains("\nmodel: sonnet\n"), "{c}");
    assert!(c.contains("\neffort: high\n"), "{c}");
    assert!(c.contains("\ntools: Read, Grep, Glob, Bash, Edit\n"), "{c}");
    for text in [&v, &c] {
        let d = text
            .lines()
            .find_map(|l| l.strip_prefix("description: "))
            .unwrap();
        assert!(d.chars().count() <= 200, "{} characters: {d}", d.len());
    }
}

/// The paragraph of the scratch folder, as each agent file holds it.
const SCRATCH: &str = "Scratch folder: Make your own new subfolder in the scratchpad folder that the session names. If the session names none, make a new folder with `mktemp -d`. Put each temporary file in that subfolder. Put there also each screenshot and each file of a browser tool. Never write them into the repository. Delete only the subfolder that you made. In the scratchpad, never delete or change a file outside it. Never delete the scratchpad folder itself.";

#[test]
fn each_agent_file_ends_with_the_rule_of_the_scratch_folder() {
    let r = full_repo();
    let agents = r.dir.join(".claude/agents");
    let mut seen = 0;
    for e in fs::read_dir(&agents).unwrap() {
        let name = e.unwrap().file_name().into_string().unwrap();
        if name == "model-team.md" {
            continue;
        }
        let t = r.read(&format!(".claude/agents/{name}"));
        // The paragraph is the last paragraph before the marker.
        let end = t.rfind("\n<!-- spec-dna harness v").expect("marker");
        let body = t[..end].trim_end();
        assert!(body.ends_with(&format!("\n\n{SCRATCH}")), "{name}\n{t}");
        assert_eq!(t.matches("Scratch folder:").count(), 1, "{name}");
        seen += 1;
    }
    // The 5 roles of the level `full`.
    assert_eq!(seen, 5);
}

#[test]
fn the_verifier_does_not_tell_to_remove_a_shared_folder() {
    let r = full_repo();
    let v = r.read(".claude/agents/verifier.md");
    // The old sentence made a subagent delete the scratchpad of its parent.
    assert!(!v.contains("temporary directory"), "{v}");
    assert!(!v.contains("remove them at the end"), "{v}");
    assert!(v.contains("your own scratch folder"), "{v}");
}

#[test]
fn the_team_page_and_the_claude_line_name_the_new_roles() {
    let r = full_repo();
    let page = r.read(".claude/agents/model-team.md");
    assert!(
        page.contains("| verifier | `.claude/agents/verifier.md` | opus | high |"),
        "{page}"
    );
    assert!(
        page.contains("| code-simplifier | `.claude/agents/code-simplifier.md` | sonnet | high |"),
        "{page}"
    );
    for old in ["reviewer", "spec-writer", "checker"] {
        assert!(!page.contains(old), "{old}\n{page}");
    }
    let claude = r.read("CLAUDE.md");
    assert!(
        claude.contains("`verifier` (opus) tests finished work. "),
        "{claude}"
    );
    assert!(
        claude.contains("Run `verifier` before you say that a task is done."),
        "{claude}"
    );
    assert!(
        claude.contains(
            "Give a code change to `implementer`. After the checks pass, give the diff to `code-simplifier`. "
        ),
        "{claude}"
    );
    for old in ["checker", "spec-writer"] {
        assert!(!claude.contains(old), "{old}\n{claude}");
    }
    // A lean line has no sentence for a role that level `full` adds.
    let lean = Repo::new();
    let (code, out, err) = lean.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let claude = lean.read("CLAUDE.md");
    assert!(
        claude.contains("`verifier` (opus) tests finished work. "),
        "{claude}"
    );
    assert!(!claude.contains("code-simplifier"), "{claude}");
    assert!(!claude.contains("implementer"), "{claude}");
}

#[test]
fn a_code_simplifier_that_is_off_has_no_sentence_and_no_file() {
    let r = Repo::new();
    let (code, out, err) = init_with(
        &r,
        "harness = \"full\"\nharness-models = [\"code-simplifier=none\"]\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".claude/agents/code-simplifier.md").exists());
    let claude = r.read("CLAUDE.md");
    assert!(
        claude.contains("Give a code change to `implementer`."),
        "{claude}"
    );
    assert!(!claude.contains("code-simplifier"), "{claude}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_old_role_keys_give_exit_code_3_at_each_level() {
    let reviewer = "`harness.roles.reviewer`: the role is now `verifier`; rename the key";
    let writer = "`harness.roles.spec-writer`: the role ended, and the stage skills of `[harness] skills` replace it; remove the key";
    for level in ["none", "lean", "full"] {
        for (key, message) in [("reviewer", reviewer), ("spec-writer", writer)] {
            let r = project_with(&format!(
                "[harness]\nlevel = \"{level}\"\n\n[harness.roles]\n{key} = {{ claude-code = \"opus\" }}\n"
            ));
            for args in [
                &["check"][..],
                &["init", "--reconfigure", "--non-interactive"][..],
            ] {
                let (code, out, err) = r.run(args);
                assert_eq!(code, 3, "{level} {key} {args:?}: {out}{err}");
                assert!(err.contains(message), "{level} {key} {args:?}: {err}");
            }
        }
    }
}

#[test]
fn code_simplifier_in_a_lean_table_is_a_level_error() {
    let r = project_with(
        "[harness]\nlevel = \"lean\"\n\n[harness.roles]\ncode-simplifier = { claude-code = \"sonnet\" }\n",
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("`harness.roles.code-simplifier`: the role is in level full"),
        "{err}"
    );
    let r = project_with(
        "[harness]\nlevel = \"full\"\n\n[harness.roles]\ncode-simplifier = { claude-code = \"haiku\", effort = \"low\" }\n",
    );
    let (code, out, err) = r.run(&["check"]);
    // The files are absent, so `check` reports them, but the table parses.
    assert!(!err.contains("harness.roles"), "{out}{err}");
    assert_ne!(code, 3, "{out}{err}");
}

#[test]
fn an_unknown_role_lists_the_new_names() {
    let r = project_with(
        "[harness]\nlevel = \"lean\"\n\n[harness.roles]\ncoder = { claude-code = \"opus\" }\n",
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("unknown role; use orchestrator, advisor, verifier, researcher, implementer, code-simplifier, helper"),
        "{err}"
    );
}

#[test]
fn the_old_words_of_question_13_give_exit_code_3() {
    for (word, message) in [
        ("reviewer=sonnet", "the role `reviewer` is now `verifier`"),
        ("spec-writer=sonnet", "the role `spec-writer` ended"),
    ] {
        for level in ["lean", "full"] {
            let r = Repo::new();
            let (code, out, err) = init_with(
                &r,
                &format!("harness = \"{level}\"\nharness-models = [\"{word}\"]\n"),
                &[],
            );
            assert_eq!(code, 3, "{level} {word}: {out}{err}");
            assert!(err.contains(message), "{level} {word}: {err}");
            assert!(!r.dir.join("spec-dna.toml").exists(), "{word}");
        }
    }
    // The new words work.
    let r = Repo::new();
    let (code, out, err) = init_with(
        &r,
        "harness = \"full\"\nharness-models = [\"verifier=sonnet:medium\", \"code-simplifier=haiku\"]\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("verifier = { claude-code = \"sonnet\", effort = \"medium\" }"),
        "{cfg}"
    );
    // A level error stays a usage error.
    let r = Repo::new();
    let (code, out, err) = init_with(
        &r,
        "harness = \"lean\"\nharness-models = [\"code-simplifier=sonnet\"]\n",
        &[],
    );
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("role `code-simplifier` is in level full"),
        "{err}"
    );
}

#[test]
fn reconfigure_after_the_rename_writes_the_verifier_and_removes_the_old_files() {
    let r = project_with("[harness]\nlevel = \"full\"\n");
    r.write(".claude/agents/checker.md", OLD_CHECKER);
    r.write(".claude/agents/spec-writer.md", OLD_SPEC_WRITER);
    // Before `--reconfigure`, an unedited old file is a stray file (SD502).
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    for f in ["checker.md", "spec-writer.md"] {
        assert!(
            out.contains(&format!(".claude/agents/{f}")) && out.contains("SD502"),
            "{f}\n{out}"
        );
    }
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(".claude/agents/verifier.md").exists(), "{out}");
    assert!(
        r.dir.join(".claude/agents/code-simplifier.md").exists(),
        "{out}"
    );
    assert!(!r.dir.join(".claude/agents/checker.md").exists(), "{out}");
    assert!(
        !r.dir.join(".claude/agents/spec-writer.md").exists(),
        "{out}"
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn reconfigure_keeps_an_old_file_that_a_person_edited() {
    let r = project_with("[harness]\nlevel = \"lean\"\n");
    r.write(
        ".claude/agents/checker.md",
        &format!("{OLD_CHECKER}Own note.\n"),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(".claude/agents/verifier.md").exists(), "{out}");
    assert!(r.read(".claude/agents/checker.md").contains("Own note."));
    assert!(
        out.contains("kept .claude/agents/checker.md: edited by hand (SD501)"),
        "{out}"
    );
}

#[test]
fn plain_init_never_removes_an_old_file() {
    let r = project_with("[harness]\nlevel = \"lean\"\n");
    r.write(".claude/agents/checker.md", OLD_CHECKER);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(".claude/agents/verifier.md").exists(), "{out}");
    assert!(r.dir.join(".claude/agents/checker.md").exists(), "{out}");
}

#[test]
fn an_agent_of_the_project_named_verifier_turns_the_role_off() {
    let r = Repo::new();
    r.write(
        ".claude/agents/verifier.md",
        "---\nname: verifier\ndescription: Mine\n---\nMy text.\n",
    );
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read(".claude/agents/verifier.md").contains("My text."));
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("verifier = { claude-code = \"none\""), "{cfg}");
    let claude = r.read("CLAUDE.md");
    assert!(!claude.contains("`verifier`"), "{claude}");
}
