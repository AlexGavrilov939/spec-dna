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
        "acceptor",
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
        // The default of `standard` is `advisor = false` (design 0.8.0, D81).
        (".claude/agents/acceptor.md", "acceptor-no-advisor.md"),
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
const SCRATCH: &str = "Scratch folder: These rules cover temporary files only. They never limit an edit of the repository that your task needs. A temporary file is the one write that a read-only role may make, and only in the folder below. Make your own new subfolder in the scratchpad folder that the session names. If the session names none, make a new folder with `mktemp -d`. Put each temporary file in that subfolder. Put there also each screenshot and each file of a browser tool. Never write them into the repository. Delete only the files that you put in the scratchpad, and the subfolder that you made. Never delete or change another file of the scratchpad. Never delete the scratchpad folder itself.";

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
    // The 6 roles of the level `full`, with the acceptor of the skill `accept`.
    assert_eq!(seen, 6);
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
        err.contains("unknown role; use orchestrator, advisor, verifier, acceptor, researcher, implementer, code-simplifier, helper"),
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

// ---- The agent `acceptor` (design 0.8.0, D3 and D71) ----

const ACCEPTOR: &str = ".claude/agents/acceptor.md";

#[test]
fn lean_in_standard_writes_the_acceptor_with_the_skill_review() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(ACCEPTOR).exists(), "{out}");
    assert!(out.contains(&format!("wrote {ACCEPTOR}\n")), "{out}");
    // The skill `accept` merged into `review` (D71): the agent comes with `review`.
    assert!(
        r.dir
            .join(".claude/skills/spec-dna-review/SKILL.md")
            .exists()
    );
    assert!(!r.dir.join(".claude/skills/spec-dna-accept").exists());
    assert_eq!(
        normalize_marker(&r.read(ACCEPTOR)),
        golden("acceptor-no-advisor.md")
    );
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(
            "\nacceptor = { claude-code = \"opus\", effort = \"high\", advisor = false }\n"
        ),
        "{cfg}"
    );
    let page = r.read(".claude/agents/model-team.md");
    assert!(
        page.contains("| acceptor | `.claude/agents/acceptor.md` | opus | high |"),
        "{page}"
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_acceptor_is_a_role_of_lean_and_full() {
    let r = full_repo();
    assert!(r.dir.join(ACCEPTOR).exists());
    assert_eq!(
        normalize_marker(&r.read(ACCEPTOR)),
        golden("acceptor-no-advisor.md")
    );
}

#[test]
fn lean_in_lite_writes_neither_the_acceptor_nor_the_skill_nor_a_key() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "profile = \"lite\"\nharness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(ACCEPTOR).exists(), "{out}");
    assert!(!r.dir.join(".claude/skills/spec-dna-accept").exists());
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("acceptor"), "{cfg}");
    let page = r.read(".claude/agents/model-team.md");
    assert!(!page.contains("acceptor"), "{page}");
}

/// D76: `lite` writes the agent files of 0.7.1, byte for byte, at each level.
/// `tests/golden/harness/agents-0.7.1/` holds the files of the tag `v0.7.1`. The
/// pass of 0.8.0 that put the body of `code-simplifier` in Simplified Technical
/// English must not reach `lite`. `standard` and `strict` have the new text.
#[test]
fn lite_writes_the_agent_files_of_0_7_1_at_each_level() {
    const LEAN: [&str; 3] = ["fallback-advisor", "verifier", "Explore"];
    const FULL: [&str; 5] = [
        "fallback-advisor",
        "verifier",
        "Explore",
        "implementer",
        "code-simplifier",
    ];
    for (level, want) in [("none", &[][..]), ("lean", &LEAN[..]), ("full", &FULL[..])] {
        let r = Repo::new();
        let answers = format!("profile = \"lite\"\nharness = \"{level}\"\n");
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{level}: {out}{err}");
        for name in FULL {
            let path = format!(".claude/agents/{name}.md");
            if !want.contains(&name) {
                assert!(!r.dir.join(&path).exists(), "{level}: {path}");
                continue;
            }
            assert_eq!(
                normalize_marker(&r.read(&path)),
                golden(&format!("agents-0.7.1/{name}.md")),
                "{level}: {path} is not the file of 0.7.1"
            );
        }
        // The hash of the file of 0.7.1 is in the marker, so the file is the
        // same bytes, not only the same text.
        if level == "full" {
            assert!(
                r.read(".claude/agents/code-simplifier.md")
                    .contains(" h:56a34fb2d62bc033 -->"),
            );
        }
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 0, "{level}: {out}{err}");
    }
    // The other profiles keep the text of this release.
    for profile in ["standard", "strict"] {
        let r = Repo::new();
        let answers = format!("profile = \"{profile}\"\nharness = \"full\"\n");
        let (code, out, err) = init_with(&r, &answers, &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        for name in FULL {
            let path = format!(".claude/agents/{name}.md");
            assert_eq!(
                normalize_marker(&r.read(&path)),
                golden(&format!("{name}.md")),
                "{profile}: {path}"
            );
        }
    }
}

#[test]
fn the_acceptor_goes_with_the_skill_not_with_the_level() {
    // A skill list with no `review` writes no agent file, also at `lean`.
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\"]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(ACCEPTOR).exists(), "{out}");
    // A skill list with `review` writes both.
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(ACCEPTOR).exists(), "{out}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // The level `none` writes neither.
    let r = project_with("[harness]\nlevel = \"none\"\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(ACCEPTOR).exists(), "{out}");
}

#[test]
fn the_acceptor_has_the_tools_of_a_reader_and_one_write_and_no_edit() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(ACCEPTOR);
    assert!(t.starts_with("---\nname: acceptor\n"), "{t}");
    assert!(
        t.contains("\nmodel: opus\neffort: high\ntools: Read, Grep, Glob, Bash, Write\n"),
        "{t}"
    );
    for w in [
        "Tries to break a finished change from the brief of `spec-dna accept --brief`, and writes its acceptance record. It edits no code.",
        "Your prompt is one line: `cd '<root>' && spec-dna accept <slug> --brief`",
        "7. Commit the record alone. This commit is part of your task. It is no approval commit: it changes no `status` line, and it needs no go-ahead of the owner.",
        "Write only `acceptance.toml`",
        "Report each defect as a finding. Do not fix it.",
    ] {
        assert!(t.contains(w), "`{w}`\n{t}");
    }
}

/// D75 items 1 to 4: the agent runs its prompt line first, starts each shell call
/// with the `cd` of that line, keeps its test repositories under its scratch
/// folder, and commits the record alone with a path, so rule D of the hook passes.
#[test]
fn the_acceptor_runs_the_prompt_line_and_commits_the_record_with_its_path() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(ACCEPTOR);
    for part in [
        "Your prompt is one line: `cd '<root>' && spec-dna accept <slug> --brief`, or the same with `--follow-up`. Run it first. Read its output as your brief.",
        "Start each shell call with that `cd '<root>' &&`. If the `cd` fails, stop.",
        "Put each test repository under your scratch folder, never in another checkout.",
        // `standard` writes no hook, so one step names the call (D93, item 2).
        "Make the commit in a call of its own, after the write: `cd '<root>' && git add <path> && git commit -m '<text>' <path>`. Use the literal path of `acceptance.toml` for `<path>`. A new record is not tracked, so `git commit` alone refuses its path.",
    ] {
        assert!(t.contains(part), "{part}\n{t}");
    }
    assert!(!t.contains("Join the parts with `&&` only"), "{t}");
    assert!(!t.contains("Your prompt is the output of"), "{t}");
}

// ---- The key `advisor` of the acceptor (design 0.8.0, D81) ----

/// The line that `acceptor.md` has with `advisor = false` (D81 item 4).
const NO_ADVISOR: &str = "Advisor: do not consult the advisor in this task. This rule has priority over each project instruction that tells you to consult the advisor.";

/// The steps 8 to 10 of the acceptor file of a project with the hook
/// `protect-approvals` (strict).
const HOOK_STEPS: &str = "8. Make the commit in a call of its own, after the write. The hook `protect-approvals` denies one call that writes `accepted` and runs `git commit`.\n9. Make the call in one shape: `cd '<root>' && git add <path> && git commit -m '<text>' <path>`. Use the literal path of `acceptance.toml` for `<path>`. A new record is not tracked, so `git commit` alone refuses its path.\n10. Join the parts with `&&` only. Use no option but `-m` and no other command in that call. Use no `$`, backtick, `;`, `|` or `#` outside the single quotes of the text. The hook denies each other form.";

/// The one step of the file of a project with no such hook (D93, item 2).
const NO_HOOK_STEP: &str = "8. Make the commit in a call of its own, after the write: `cd '<root>' && git add <path> && git commit -m '<text>' <path>`. Use the literal path of `acceptance.toml` for `<path>`. A new record is not tracked, so `git commit` alone refuses its path.";

/// The text above the marker line.
fn above_marker(t: &str) -> String {
    let n = t.lines().count() - 1;
    t.lines().take(n).collect::<Vec<_>>().join("\n")
}

/// A project that `init` wrote in `profile` at the level `lean`. Then the key
/// `advisor` of the entry `acceptor` is set to `key`, and `None` removes it.
fn acceptor_project(profile: &str, key: Option<bool>) -> Repo {
    let r = Repo::new();
    let answers = format!("profile = \"{profile}\"\nharness = \"lean\"\n");
    let (code, out, err) = init_with(&r, &answers, &[]);
    assert_eq!(code, 0, "{profile}: {out}{err}");
    let cfg = r.read("spec-dna.toml");
    let entry = cfg
        .lines()
        .find(|l| l.starts_with("acceptor = "))
        .unwrap_or_else(|| panic!("{profile}: no entry `acceptor`\n{cfg}"))
        .to_string();
    let new = match key {
        None => "acceptor = { claude-code = \"opus\", effort = \"high\" }".to_string(),
        Some(b) => {
            format!("acceptor = {{ claude-code = \"opus\", effort = \"high\", advisor = {b} }}")
        }
    };
    r.write("spec-dna.toml", &cfg.replace(&entry, &new));
    r
}

#[test]
fn the_key_advisor_selects_the_text_of_the_acceptor() {
    // No key: false in `standard` and true in `strict`. A key of the file wins.
    // `standard` writes no hook `protect-approvals`, and `strict` writes it, so
    // the steps 8 to 10 follow the profile (D93, item 2). A case that is not
    // one of the 2 golden files differs from one of them by those steps only.
    for (profile, key, want) in [
        ("standard", None, "acceptor-no-advisor.md"),
        ("standard", Some(true), "acceptor.md"),
        ("standard", Some(false), "acceptor-no-advisor.md"),
        ("strict", None, "acceptor.md"),
        ("strict", Some(true), "acceptor.md"),
        ("strict", Some(false), "acceptor-no-advisor.md"),
    ] {
        let r = acceptor_project(profile, key);
        let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
        assert_eq!(code, 0, "{profile} {key:?}: {out}{err}");
        let actual = normalize_marker(&r.read(ACCEPTOR));
        let exact = (profile == "standard") == (want == "acceptor-no-advisor.md");
        if exact {
            assert_eq!(actual, golden(want), "{profile} {key:?}");
        } else {
            // The same text with the steps of the other profile. The marker
            // hash covers the text, so only the text above it compares.
            let g = golden(want);
            let other = if profile == "standard" {
                g.replace(HOOK_STEPS, NO_HOOK_STEP)
            } else {
                g.replace(NO_HOOK_STEP, HOOK_STEPS)
            };
            assert_ne!(other, g, "{profile} {key:?}");
            assert_eq!(
                above_marker(&actual),
                above_marker(&other),
                "{profile} {key:?}"
            );
        }
        // `init` writes the default of the profile, or keeps the key.
        let cfg = r.read("spec-dna.toml");
        let value = key.unwrap_or(profile == "strict");
        assert!(
            cfg.contains(&format!("effort = \"high\", advisor = {value} }}\n")),
            "{profile} {key:?}\n{cfg}"
        );
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 0, "{profile} {key:?}: {out}{err}");
    }
}

#[test]
fn the_acceptor_names_no_hook_that_the_project_lacks() {
    // `standard` writes no hook `protect-approvals`: its acceptor file has one
    // step for the commit and does not name the hook.
    let r = acceptor_project("standard", None);
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(ACCEPTOR);
    assert!(!t.contains("protect-approvals"), "{t}");
    assert!(t.contains(NO_HOOK_STEP), "{t}");
    assert!(
        !t.contains("9. Make the call") && !t.contains("10. Join"),
        "{t}"
    );
    // `strict` writes the hook, and its file keeps the 3 steps.
    let r = acceptor_project("strict", None);
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(ACCEPTOR);
    assert!(t.contains(HOOK_STEPS), "{t}");
    assert!(!t.contains(NO_HOOK_STEP), "{t}");
}

#[test]
fn a_project_with_no_advisor_key_resolves_to_the_default_of_its_profile() {
    // The files of `init` agree with a configuration that has no key, so
    // `check` finds no difference (SD502) in each profile.
    for profile in ["standard", "strict"] {
        let r = acceptor_project(profile, None);
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        // The file has the line in `standard` only.
        let has = r.read(ACCEPTOR).contains(NO_ADVISOR);
        assert_eq!(has, profile == "standard", "{profile}");
    }
}

#[test]
fn both_golden_files_of_the_acceptor_differ_by_the_line_and_the_steps_of_the_hook_only() {
    let strict = golden("acceptor.md");
    let standard = golden("acceptor-no-advisor.md");
    // The frozen file keeps its hash.
    assert!(strict.ends_with(" h:f541cad357de8926 -->\n"), "{strict}");
    assert!(!strict.contains("Advisor:"), "{strict}");
    assert_eq!(standard.matches(NO_ADVISOR).count(), 1, "{standard}");
    assert!(strict.contains(HOOK_STEPS) && !strict.contains(NO_HOOK_STEP));
    assert!(standard.contains(NO_HOOK_STEP) && !standard.contains("protect-approvals"));
    assert_eq!(
        above_marker(&standard)
            .replace(&format!("{NO_ADVISOR}\n\n"), "")
            .replace(NO_HOOK_STEP, HOOK_STEPS),
        above_marker(&strict)
    );
    assert_eq!(md_lint(&standard), []);
    assert_eq!(md_lint(&strict), []);
}

#[test]
fn a_bad_key_advisor_gives_exit_code_3() {
    let r = project_with(
        "[harness]\nlevel = \"lean\"\n\n[harness.roles]\nacceptor = { advisor = \"no\" }\n",
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("`harness.roles.acceptor.advisor` must be true or false"),
        "{err}"
    );
    // Only the role `acceptor` has the key.
    let r = project_with(
        "[harness]\nlevel = \"lean\"\n\n[harness.roles]\nverifier = { advisor = false }\n",
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("unknown key `harness.roles.verifier.advisor`"),
        "{err}"
    );
}

#[test]
fn a_change_of_the_key_with_no_init_gives_sd502_and_reconfigure_clears_it() {
    let r = acceptor_project("standard", Some(false));
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &cfg.replace("advisor = false", "advisor = true"),
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.lines()
            .any(|l| l.contains("SD502") && l.contains(ACCEPTOR)),
        "{out}"
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    // `standard` has no hook, so the file is `acceptor.md` with the one step.
    assert_eq!(
        above_marker(&normalize_marker(&r.read(ACCEPTOR))),
        above_marker(&golden("acceptor.md").replace(HOOK_STEPS, NO_HOOK_STEP))
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_hand_edit_of_the_line_gives_sd501() {
    let r = acceptor_project("standard", Some(false));
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read(ACCEPTOR);
    r.write(
        ACCEPTOR,
        &text.replace("Advisor: do not consult", "Advisor: please do not consult"),
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.lines()
            .any(|l| l.contains("SD501") && l.contains(ACCEPTOR)),
        "{out}"
    );
}

#[test]
fn the_role_that_is_off_gets_no_key_advisor_and_gets_it_back_with_the_role() {
    // D81 items 2 and 6: `claude-code = "none"` is a case where the role does
    // not exist, so `init` writes no key. The entry stays, as it has a model.
    let r = Repo::new();
    let (code, out, err) = init_with(
        &r,
        "profile = \"standard\"\nharness = \"lean\"\nharness-models = [\"acceptor=none\"]\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("\nacceptor = { claude-code = \"none\", effort = \"high\" }\n"),
        "{cfg}"
    );
    assert!(!r.dir.join(ACCEPTOR).exists(), "{out}");
    // The role returns: `init` writes the default of the profile.
    let (code, out, err) = init_with(
        &r,
        "harness = \"lean\"\nharness-models = [\"acceptor=opus\"]\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(
            "\nacceptor = { claude-code = \"opus\", effort = \"high\", advisor = false }\n"
        ),
        "{cfg}"
    );
    assert!(r.read(ACCEPTOR).contains(NO_ADVISOR));
}

#[test]
fn question_13_keeps_the_key_advisor() {
    let r = acceptor_project("standard", Some(true));
    let (code, out, err) = init_with(
        &r,
        "harness = \"lean\"\nharness-models = [\"acceptor=sonnet:low\"]\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("acceptor = { claude-code = \"sonnet\", effort = \"low\", advisor = true }\n"),
        "{cfg}"
    );
    assert!(!r.read(ACCEPTOR).contains(NO_ADVISOR));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}
