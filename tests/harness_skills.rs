//! Tests of the stage skills and `REVIEW.md` (step 4, S3). The configuration
//! keys `skills` and `review-md` select the files. Each test uses a temporary
//! repository. No test reads or writes a file of this repository.

mod common;

use common::{Repo, normalize_marker};
use std::fs;

const SKILLS: [&str; 4] = [
    ".claude/skills/spec-dna-intent/SKILL.md",
    ".claude/skills/spec-dna-spec/SKILL.md",
    ".claude/skills/spec-dna-plan/SKILL.md",
    ".claude/skills/spec-dna-review/SKILL.md",
];

const ALL_ON: &str = "[harness]\nlevel = \"lean\"\nskills = [\"intent\", \"spec\", \"plan\", \"review\"]\nreview-md = true\n";

fn golden(name: &str) -> String {
    let p = format!("{}/tests/golden/harness/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// A committed project with one document and the `[harness]` table `harness`.
fn project_with(harness: &str) -> Repo {
    let r = Repo::with_config(&format!("\n[[roots]]\npath = \"docs\"\n\n{harness}"));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    r
}

/// The text of a skill file between its front matter and its marker.
fn body_of(text: &str) -> &str {
    let rest = text.strip_prefix("---\n").expect("front matter");
    let (_, rest) = rest.split_once("\n---\n").expect("front matter end");
    let end = rest.rfind("\n<!-- spec-dna harness v").expect("marker");
    rest[..end].trim_matches('\n')
}

/// Reads the number of the line "adds about N bytes".
fn request_bytes(out: &str) -> usize {
    let l = out
        .lines()
        .find(|l| l.contains("The harness adds about "))
        .unwrap_or_else(|| panic!("no prefix line\n{out}"));
    l.split("adds about ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{l}"))
}

fn description_of(text: &str) -> &str {
    text.lines()
        .find_map(|l| l.strip_prefix("description: "))
        .expect("description")
}

#[test]
fn init_writes_the_4_skills_and_review_md() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert!(r.dir.join(f).exists(), "{f}\n{out}");
        assert!(out.contains(&format!("wrote {f}\n")), "{f}\n{out}");
        // The marker is the last line of the file.
        let t = r.read(f);
        let last = t.lines().last().unwrap();
        assert!(
            last.starts_with("<!-- spec-dna harness v") && last.ends_with(" -->"),
            "{f}: {last}"
        );
        assert!(t.ends_with(" -->\n"), "{f}");
    }
    // The files are untracked, and `check` reads them.
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_files_are_the_same_at_lean_and_full_and_in_each_project() {
    let lean = project_with(ALL_ON);
    let full = project_with(&ALL_ON.replace("lean", "full"));
    for r in [&lean, &full] {
        let (code, out, err) = r.run(&["init"]);
        assert_eq!(code, 0, "{out}{err}");
    }
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert_eq!(lean.read(f), full.read(f), "{f}");
    }
    // The agent files of `full` are there too.
    assert!(full.dir.join(".claude/agents/implementer.md").exists());
}

#[test]
fn skills_follow_the_order_of_the_skill_list_and_write_only_the_listed_ones() {
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"review\", \"intent\"]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(SKILLS[0]).exists());
    assert!(r.dir.join(SKILLS[3]).exists());
    assert!(!r.dir.join(SKILLS[1]).exists());
    assert!(!r.dir.join(SKILLS[2]).exists());
    assert!(!r.dir.join("REVIEW.md").exists());
    let a = out.find("wrote .claude/skills/spec-dna-intent").unwrap();
    let b = out.find("wrote .claude/skills/spec-dna-review").unwrap();
    assert!(a < b, "{out}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_skill_files_match_the_golden_files() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in SKILLS {
        let name = f.split('/').nth(2).unwrap();
        assert_eq!(
            normalize_marker(&r.read(f)),
            golden(&format!("skills/{name}.md")),
            "{f}"
        );
    }
    assert_eq!(
        normalize_marker(&r.read("REVIEW.md")),
        golden("REVIEW.md"),
        "REVIEW.md"
    );
}

#[test]
fn the_front_matter_and_the_sizes_follow_the_plan() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let mut listing = 0;
    for f in SKILLS {
        let t = r.read(f);
        let name = f.split('/').nth(2).unwrap();
        let head: Vec<&str> = t.lines().take(4).collect();
        assert_eq!(head[0], "---", "{f}");
        assert_eq!(head[1], format!("name: {name}"), "{f}");
        assert!(head[2].starts_with("description: "), "{f}");
        assert_eq!(head[3], "---", "{f}: no other key");
        let d = description_of(&t);
        assert!(d.len() <= 160, "{f}: {} bytes", d.len());
        listing += format!("- {name}: {d}\n").len();
        let b = body_of(&t);
        assert!(b.len() <= 2500, "{f}: {} bytes", b.len());
        assert!(!b.contains("{{"), "{f}");
    }
    assert!(listing <= 800, "{listing} bytes");
    let review = r.read("REVIEW.md");
    assert!(review.len() <= 2500, "{} bytes", review.len());
}

#[test]
fn review_md_and_the_review_skill_hold_the_same_policy() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let review = r.read("REVIEW.md");
    let (_, policy) = review.split_once("# Review policy\n\n").unwrap();
    let (policy, _) = policy
        .split_once("\n\nspec-dna generates this file.")
        .unwrap();
    assert!(policy.starts_with("## Passes\n"), "{policy}");
    let skill = r.read(SKILLS[3]);
    assert!(skill.contains(policy), "{skill}");
    // The 3 passes, the severities, the Nit cap and the exclusions.
    for p in [
        "Bugs",
        "Security",
        "Compliance",
        "Important",
        "Nit",
        "at most 5 Nits",
        "Do not report",
    ] {
        assert!(policy.contains(p), "{p}");
    }
    assert!(review.ends_with(" -->\n"));
    assert!(
        review.contains(
            "\nspec-dna generates this file. Change `[harness]` in `spec-dna.toml` and run `spec-dna init --reconfigure`.\n\n<!-- spec-dna harness v"
        ),
        "{review}"
    );
}

#[test]
fn the_stage_skills_run_spec_dna_commands_and_set_no_approval() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let intent = r.read(SKILLS[0]);
    for p in [
        "spec-dna new intent <slug>",
        "--template prd",
        "--template rfc",
        "spec-dna next <slug>",
    ] {
        assert!(intent.contains(p), "{p}");
    }
    assert!(r.read(SKILLS[1]).contains("spec-dna show <id>#<key>"));
    assert!(r.read(SKILLS[1]).contains("spec-dna new spec <slug>"));
    assert!(r.read(SKILLS[2]).contains("spec-dna new plan <slug>"));
    for f in &SKILLS[..3] {
        let t = r.read(f);
        assert!(t.contains("Do not set `status`"), "{f}");
    }
    for f in SKILLS {
        let t = r.read(f);
        assert!(!t.contains("small-fix"), "{f}");
        assert!(!t.contains("{{"), "{f}");
    }
}

#[test]
fn the_team_page_lists_the_skills_and_review_md_only_when_they_are_on() {
    let on = project_with(ALL_ON);
    let (code, out, err) = on.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let page = on.read(".claude/agents/model-team.md");
    assert!(page.contains("\n## Skills\n"), "{page}");
    for n in [
        "spec-dna-intent",
        "spec-dna-spec",
        "spec-dna-plan",
        "spec-dna-review",
    ] {
        assert!(page.contains(&format!("`{n}`")), "{n}\n{page}");
    }
    assert!(page.contains("`REVIEW.md`"), "{page}");
    // The CLAUDE.md line names no skill.
    let claude = on.read("CLAUDE.md");
    assert!(!claude.contains("spec-dna-"), "{claude}");
    // With the keys off, the page and the line are those of step 4 S2.
    let off = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = off.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let page = off.read(".claude/agents/model-team.md");
    assert!(!page.contains("## Skills") && !page.contains("REVIEW.md"));
    assert_eq!(off.read("CLAUDE.md"), claude);
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert!(!off.dir.join(f).exists(), "{f}");
    }
}

#[test]
fn the_request_bytes_grow_by_the_listing_of_the_skills_only() {
    let off = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = off.run(&["init", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let base = request_bytes(&out);
    // `REVIEW.md` adds no bytes.
    let review = project_with("[harness]\nlevel = \"lean\"\nreview-md = true\n");
    let (code, out, err) = review.run(&["init", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(request_bytes(&out), base, "{out}");
    // The 4 skills add the listing bytes.
    let on = project_with(ALL_ON);
    let (code, out, err) = on.run(&["init", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let with = request_bytes(&out);
    let (code, out, err) = on.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let listing: usize = SKILLS
        .iter()
        .map(|f| {
            let name = f.split('/').nth(2).unwrap();
            format!("- {name}: {}\n", description_of(&on.read(f))).len()
        })
        .sum();
    assert!(listing > 0 && listing <= 800, "{listing}");
    assert_eq!(with, base + listing, "{out}");
}

#[test]
fn a_file_with_no_marker_at_a_generated_path_stops_init() {
    for (path, config) in [
        (
            "REVIEW.md",
            "[harness]\nlevel = \"lean\"\nreview-md = true\n",
        ),
        (
            SKILLS[1],
            "[harness]\nlevel = \"lean\"\nskills = [\"spec\"]\n",
        ),
    ] {
        let r = project_with(config);
        r.write(path, "# Mine\n\nMy text.\n");
        let (code, out, err) = r.run(&["init"]);
        assert_eq!(code, 2, "{path}: {out}{err}");
        assert!(
            err.contains(&format!("{path} exists, and spec-dna did not generate it")),
            "{path}: {err}"
        );
        assert_eq!(r.read(path), "# Mine\n\nMy text.\n");
        assert!(
            !r.dir.join(".claude/agents/verifier.md").exists(),
            "init writes nothing"
        );
    }
}

#[test]
fn reconfigure_removes_an_unedited_skill_and_review_md_that_the_configuration_drops() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    let cfg = cfg
        .replace(
            "skills = [\"intent\", \"spec\", \"plan\", \"review\"]",
            "skills = [\"spec\"]",
        )
        .replace("review-md = true", "review-md = false");
    assert!(cfg.contains("skills = [\"spec\"]"), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    // Plain `init` removes no file.
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert!(r.dir.join(f).exists(), "plain init keeps {f}");
    }
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in [SKILLS[0], SKILLS[2], SKILLS[3], "REVIEW.md"] {
        assert!(!r.dir.join(f).exists(), "{f}\n{out}");
        assert!(
            out.contains(&format!("remove {f}")) || out.contains(f),
            "{out}"
        );
    }
    assert!(r.dir.join(SKILLS[1]).exists(), "{out}");
    // The adapter skill has its own marker and stays.
    assert!(r.dir.join(".claude/skills/spec-dna/SKILL.md").exists());
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn reconfigure_keeps_an_edited_skill_and_names_it() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = format!("{}Own note.\n", r.read(SKILLS[2]));
    r.write(SKILLS[2], &text);
    let edited = r.read("REVIEW.md").replace("Bugs", "Defects");
    r.write("REVIEW.md", &edited);
    let cfg = r.read("spec-dna.toml").replace(
        "skills = [\"intent\", \"spec\", \"plan\", \"review\"]",
        "skills = []",
    );
    r.write(
        "spec-dna.toml",
        &cfg.replace("review-md = true", "review-md = false"),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read(SKILLS[2]).contains("Own note."));
    assert!(r.read("REVIEW.md").contains("Defects"));
    assert!(
        out.contains(&format!("kept {}: edited by hand (SD501)", SKILLS[2])),
        "{out}"
    );
    assert!(
        out.contains("kept REVIEW.md: edited by hand (SD501)"),
        "{out}"
    );
    for f in [SKILLS[0], SKILLS[1], SKILLS[3]] {
        assert!(!r.dir.join(f).exists(), "{f}");
    }
}

#[test]
fn level_none_at_reconfigure_removes_the_skills_and_review_md() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let path = format!("{}-answers.toml", r.dir.display());
    fs::write(&path, "harness = \"none\"\n").unwrap();
    let (code, out, err) = r.run(&["init", "--reconfigure", "--answers", &path]);
    let _ = fs::remove_file(&path);
    assert_eq!(code, 0, "{out}{err}");
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert!(!r.dir.join(f).exists(), "{f}\n{out}");
    }
    assert!(!r.dir.join(".claude/agents/model-team.md").exists());
    assert!(r.dir.join(".claude/skills/spec-dna/SKILL.md").exists());
}

#[test]
fn check_reports_each_absent_generated_skill_and_review_md() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    for f in SKILLS.iter().chain(["REVIEW.md"].iter()) {
        assert!(
            out.lines().any(|l| l.contains(f) && l.contains("SD504")),
            "{f}\n{out}"
        );
    }
}

#[test]
fn an_edit_of_a_generated_skill_gives_sd501() {
    let r = project_with(ALL_ON);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write(
        SKILLS[0],
        &r.read(SKILLS[0])
            .replacen("Use the words", "Use my words", 1),
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.lines()
            .any(|l| l.contains(SKILLS[0]) && l.contains("SD501")),
        "{out}"
    );
}

#[test]
fn a_config_change_with_the_skill_files_is_not_alone() {
    // SD503 knows the new output paths: the configuration and its files are
    // one change.
    let r = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("harness");
    let cfg = r.read("spec-dna.toml").replace(
        "level = \"lean\"\n",
        "level = \"lean\"\nskills = [\"intent\", \"spec\", \"plan\", \"review\"]\nreview-md = true\n",
    );
    assert!(cfg.contains("review-md = true"), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("skills");
    let (code, out, err) = r.run(&["check", "--base", "HEAD~1"]);
    assert!(!out.contains("SD503"), "{code}: {out}{err}");
}
