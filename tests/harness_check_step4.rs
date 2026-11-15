//! Tests of the drift of the new harness files (step 4, S5). SD501, SD502 and
//! SD504 cover the stage skills, `REVIEW.md` and the new agent files. Each test
//! uses a temporary repository. No test reads or writes a file of this
//! repository.

mod common;

use common::Repo;

const SKILLS: [&str; 4] = [
    ".claude/skills/spec-dna-intent/SKILL.md",
    ".claude/skills/spec-dna-spec/SKILL.md",
    ".claude/skills/spec-dna-plan/SKILL.md",
    ".claude/skills/spec-dna-review/SKILL.md",
];

const IMPLEMENTER: &str = ".claude/agents/implementer.md";
const SIMPLIFIER: &str = ".claude/agents/code-simplifier.md";
const CHECKER: &str = ".claude/agents/checker.md";

const FULL_ALL_ON: &str = "[harness]\nlevel = \"full\"\nskills = [\"intent\", \"spec\", \"plan\", \"review\"]\nreview-md = true\n";
const LEAN_OFF: &str = "[harness]\nlevel = \"lean\"\n";

const SD501_STRAY: &str = "the configuration does not generate this file, and it has an edit; delete the file, or delete its marker line to keep it as a file of the project";
const SD502_STRAY: &str =
    "the configuration does not generate this file; run `spec-dna init --reconfigure` to remove it";

fn project_with(harness: &str) -> Repo {
    let r = Repo::with_config(&format!("\n[[roots]]\npath = \"docs\"\n\n{harness}"));
    r.write("docs/a.md", "---\ntype: research\n---\n# A\n\nText.\n");
    r.commit("start");
    r
}

/// A project that `init` wrote at level `full` with the 4 skills and
/// `REVIEW.md`, and that then drops the level to `lean` with no skill and no
/// `REVIEW.md`. Thus 7 generated files are no longer in the configuration:
/// `implementer.md`, `code-simplifier.md`, the 4 skills and `REVIEW.md`.
fn dropped() -> Repo {
    let r = project_with(FULL_ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("init");
    let cfg = r.read("spec-dna.toml").replace(FULL_ALL_ON, LEAN_OFF);
    assert!(cfg.contains(LEAN_OFF), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    r.commit("drop the files from the configuration");
    r
}

fn append(r: &Repo, path: &str) {
    r.write(path, &format!("{}Own note.\n", r.read(path)));
}

/// The lines of a `check --all` report that name `path`.
fn lines_of<'a>(out: &'a str, path: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.starts_with(path)).collect()
}

fn check_all(r: &Repo) -> (i32, String) {
    let (code, out, err) = r.run(&["check", "--all"]);
    (code, format!("{out}{err}"))
}

#[test]
fn an_unedited_stray_gives_sd502_in_each_folder() {
    let r = dropped();
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    for f in [
        IMPLEMENTER,
        SIMPLIFIER,
        SKILLS[0],
        SKILLS[1],
        SKILLS[2],
        SKILLS[3],
        "REVIEW.md",
    ] {
        let l = lines_of(&out, f);
        assert_eq!(l.len(), 1, "{f}\n{out}");
        assert!(l[0].contains("error SD502"), "{f}: {}", l[0]);
        assert!(l[0].contains(SD502_STRAY), "{f}: {}", l[0]);
    }
}

#[test]
fn an_edited_stray_gives_sd501_in_each_folder() {
    let r = dropped();
    for f in [IMPLEMENTER, SKILLS[2], "REVIEW.md"] {
        append(&r, f);
    }
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    for f in [IMPLEMENTER, SKILLS[2], "REVIEW.md"] {
        let l = lines_of(&out, f);
        assert_eq!(l.len(), 1, "{f}\n{out}");
        assert!(l[0].contains("error SD501"), "{f}: {}", l[0]);
        assert!(l[0].contains(SD501_STRAY), "{f}: {}", l[0]);
    }
    // An unedited stray next to an edited one stays SD502.
    for f in [SIMPLIFIER, SKILLS[0], SKILLS[1], SKILLS[3]] {
        let l = lines_of(&out, f);
        assert_eq!(l.len(), 1, "{f}\n{out}");
        assert!(l[0].contains("error SD502"), "{f}: {}", l[0]);
    }
}

#[test]
fn an_edit_of_the_text_above_the_marker_of_a_stray_gives_sd501() {
    let r = dropped();
    let t = r.read(SKILLS[1]).replacen("spec-dna", "my-tool", 1);
    r.write(SKILLS[1], &t);
    let (_, out) = check_all(&r);
    let l = lines_of(&out, SKILLS[1]);
    assert_eq!(l.len(), 1, "{out}");
    assert!(l[0].contains("error SD501"), "{}", l[0]);
}

#[test]
fn an_old_agent_file_gives_sd501_if_edited_and_sd502_if_not() {
    let r = project_with(LEAN_OFF);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read(".claude/agents/verifier.md");
    r.write(CHECKER, &text);
    r.write(".claude/agents/spec-writer.md", &text);
    append(&r, CHECKER);
    r.commit("init");
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    let l = lines_of(&out, CHECKER);
    assert_eq!(l.len(), 1, "{out}");
    assert!(l[0].contains("error SD501"), "{}", l[0]);
    let l = lines_of(&out, ".claude/agents/spec-writer.md");
    assert_eq!(l.len(), 1, "{out}");
    assert!(l[0].contains("error SD502"), "{}", l[0]);
}

#[test]
fn a_file_with_no_marker_that_the_configuration_does_not_generate_is_ours_to_ignore() {
    let r = dropped();
    r.write(".claude/skills/own/SKILL.md", "# Mine\n");
    r.write(".claude/agents/own.md", "# Mine\n");
    // The files of the dropped parts lose their marker line: the person keeps
    // them as files of the project.
    for f in [IMPLEMENTER, SKILLS[0], "REVIEW.md"] {
        let t = r.read(f);
        let at = t.rfind("<!-- spec-dna harness v").unwrap();
        r.write(f, &t[..at]);
    }
    let (_, out) = check_all(&r);
    for f in [
        ".claude/skills/own/SKILL.md",
        ".claude/agents/own.md",
        IMPLEMENTER,
        SKILLS[0],
        "REVIEW.md",
    ] {
        assert!(lines_of(&out, f).is_empty(), "{f}\n{out}");
    }
}

#[test]
fn the_adapter_skill_is_not_a_stray_file() {
    let r = dropped();
    let adapter = ".claude/skills/spec-dna/SKILL.md";
    assert!(r.dir.join(adapter).exists());
    let (_, out) = check_all(&r);
    assert!(lines_of(&out, adapter).is_empty(), "{out}");
}

#[test]
fn an_untracked_stray_is_read_too() {
    let r = project_with(LEAN_OFF);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("init");
    // A skill and `REVIEW.md` of an older configuration, never committed.
    let full = project_with(FULL_ALL_ON);
    let (code, out, err) = full.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write(SKILLS[3], &full.read(SKILLS[3]));
    r.write("REVIEW.md", &full.read("REVIEW.md"));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    for f in [SKILLS[3], "REVIEW.md"] {
        assert_eq!(lines_of(&out, f).len(), 1, "{f}\n{out}");
    }
}

#[test]
fn a_file_with_no_marker_at_a_generated_path_gives_sd504_with_the_way_out() {
    for (path, config, way) in [
        (
            "REVIEW.md",
            "[harness]\nlevel = \"lean\"\nreview-md = true\n",
            "set `review-md = false` in [harness]",
        ),
        (
            SKILLS[1],
            "[harness]\nlevel = \"lean\"\nskills = [\"spec\"]\n",
            "remove the skill from `skills` in [harness]",
        ),
    ] {
        let r = project_with(config);
        r.write(path, "# Mine\n\nMy text.\n");
        let (code, out) = check_all(&r);
        assert_eq!(code, 1, "{path}: {out}");
        let l = lines_of(&out, path);
        assert_eq!(l.len(), 1, "{path}: {out}");
        assert!(l[0].contains("error SD504"), "{path}: {}", l[0]);
        assert!(l[0].contains("no harness marker"), "{path}: {}", l[0]);
        assert!(l[0].contains(way), "{path}: {}", l[0]);
    }
}

#[test]
fn a_file_with_no_marker_at_a_new_agent_path_gives_sd504_with_the_way_out() {
    let r = project_with("[harness]\nlevel = \"full\"\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write(SIMPLIFIER, "# Mine\n");
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    let l = lines_of(&out, SIMPLIFIER);
    assert_eq!(l.len(), 1, "{out}");
    assert!(l[0].contains("error SD504"), "{}", l[0]);
    assert!(
        l[0].contains("set the role `code-simplifier` to \"none\" in [harness.roles]"),
        "{}",
        l[0]
    );
}

#[test]
fn reconfigure_clears_an_sd502_stray_and_keeps_an_sd501_stray() {
    let r = dropped();
    append(&r, SKILLS[2]);
    append(&r, "REVIEW.md");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in [IMPLEMENTER, SIMPLIFIER, SKILLS[0], SKILLS[1], SKILLS[3]] {
        assert!(!r.dir.join(f).exists(), "{f}\n{out}");
    }
    for f in [SKILLS[2], "REVIEW.md"] {
        assert!(r.dir.join(f).exists(), "{f}");
        assert!(
            out.contains(&format!("kept {f}: edited by hand (SD501)")),
            "{out}"
        );
    }
    // The kept files still give SD501, and nothing else is left.
    let (code, out) = check_all(&r);
    assert_eq!(code, 1, "{out}");
    let rows: Vec<&str> = out.lines().filter(|l| l.contains(": error SD")).collect();
    assert_eq!(rows.len(), 2, "{out}");
    assert!(rows.iter().all(|l| l.contains("error SD501")), "{out}");
    // Deleting the marker line makes each file a file of the project.
    for f in [SKILLS[2], "REVIEW.md"] {
        let t = r.read(f);
        let at = t.rfind("<!-- spec-dna harness v").unwrap();
        r.write(f, &t[..at]);
    }
    let (code, out) = check_all(&r);
    assert_eq!(code, 0, "{out}");
}
