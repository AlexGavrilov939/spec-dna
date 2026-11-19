//! Tests of the 11 skills of the workflow pack and of `REVIEW.md` (ADR 0010,
//! 0.7.0 slice 4). The configuration keys `skills` and `review-md` select the
//! files. Each test uses a temporary repository. No test reads or writes a
//! file of this repository.

mod common;

use common::{Repo, VERSION, normalize_marker};
use std::fs;
use std::path::Path;

/// The 11 names, in the order of D2.
const NAMES: [&str; 11] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "execute",
    "tdd",
    "review",
    "verify",
    "finish",
    "debug",
    "post-mortem",
];

/// The 7 skills of D2. The tests that name them pin `skills`. A test of the
/// defaults of `init` reads them with `default_skills`.
const ON: [&str; 7] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "review",
    "finish",
    "post-mortem",
];

/// The 4 skills that D2 leaves off.
const OFF: [&str; 4] = ["execute", "tdd", "verify", "debug"];

/// The 3 names of 0.6 that end, and the paths of their files.
const OLD_NAMES: [&str; 3] = ["intent", "spec", "plan"];
const OLD_PATHS: [&str; 3] = [
    ".claude/skills/spec-dna-intent/SKILL.md",
    ".claude/skills/spec-dna-spec/SKILL.md",
    ".claude/skills/spec-dna-plan/SKILL.md",
];

/// The 5 parts of a body, in order (the contract of the build).
const PARTS: [&str; 5] = [
    "When to use:",
    "Commands:",
    "Artifact:",
    "Check:",
    "If the check fails:",
];

/// The sentence of the rule of Q5, in the same words in the line of
/// `AGENTS.md` and in each skill.
const APPROVAL_SENTENCE: &str =
    "An agent writes an approval commit only when the owner says so explicitly in the chat.";

/// The rule of Q5, in the same words in each skill.
const APPROVAL_RULE: &str = "Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.";

fn path(name: &str) -> String {
    format!(".claude/skills/spec-dna-{name}/SKILL.md")
}

fn paths(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| path(n)).collect()
}

/// The skills that are on by default in this build (ADR 0023), in the order
/// of D2. They come from `skill list --json` in a folder with no
/// `spec-dna.toml`, so a new table of the release changes no test.
fn default_skills() -> Vec<String> {
    let r = Repo::new();
    let (code, out, err) = r.run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    out.split("{\"name\":\"")
        .skip(1)
        .filter(|item| item.contains("\"default\":\"on\""))
        .map(|item| item.split('"').next().unwrap().to_string())
        .collect()
}

/// The `skills` list of a configuration, in the TOML form.
fn list(names: &[&str]) -> String {
    let q: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    format!("[{}]", q.join(", "))
}

/// The `[harness]` table with the 11 skills on.
fn all_on() -> String {
    format!(
        "[harness]\nlevel = \"lean\"\nskills = {}\nreview-md = true\n",
        list(&NAMES)
    )
}

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

/// An answers file next to the repository folder. It is not part of the
/// project.
fn init_with(r: &Repo, answers: &str, args: &[&str]) -> (i32, String, String) {
    let file = r.dir.join("..").join(format!(
        "spec-dna-skills-answers-{}-{}.toml",
        std::process::id(),
        r.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&file, answers).unwrap();
    let mut a = vec!["init", "--answers", file.to_str().unwrap()];
    a.extend_from_slice(args);
    let res = r.run(&a);
    let _ = fs::remove_file(&file);
    res
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

/// The line that Claude Code adds to each request for a skill.
fn listing_line(name: &str, description: &str) -> String {
    format!("- spec-dna-{name}: {description}\n")
}

/// Writes the files of the 11 skills with `init` and returns the repository.
fn written() -> Repo {
    let r = project_with(&all_on());
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r
}

/// The text of an old skill file of 0.6, as `init` 0.6.1 wrote it.
fn old_skill_file(name: &str) -> String {
    golden(&format!("old-skills/spec-dna-{name}.md")).replace("{VERSION}", "0.6.1")
}

#[test]
fn init_writes_the_11_skills_and_review_md() {
    let r = project_with(&all_on());
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let mut files = paths(&NAMES);
    files.push("REVIEW.md".to_string());
    for f in &files {
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
    let lean = project_with(&all_on());
    let full = project_with(&all_on().replace("lean", "full"));
    for r in [&lean, &full] {
        let (code, out, err) = r.run(&["init"]);
        assert_eq!(code, 0, "{out}{err}");
    }
    let mut files = paths(&NAMES);
    files.push("REVIEW.md".to_string());
    for f in &files {
        assert_eq!(lean.read(f), full.read(f), "{f}");
    }
    // The agent files of `full` are there too.
    assert!(full.dir.join(".claude/agents/implementer.md").exists());
}

#[test]
fn skills_follow_the_order_of_the_skill_list_and_write_only_the_listed_ones() {
    // The list is in the order of D2, whatever the order of the file.
    let r = project_with(&format!(
        "[harness]\nlevel = \"lean\"\nskills = {}\n",
        list(&["review", "brainstorm"])
    ));
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join(path("brainstorm")).exists());
    assert!(r.dir.join(path("review")).exists());
    for n in NAMES
        .iter()
        .filter(|n| !["brainstorm", "review"].contains(n))
    {
        assert!(!r.dir.join(path(n)).exists(), "{n}");
    }
    assert!(!r.dir.join("REVIEW.md").exists());
    let a = out
        .find("wrote .claude/skills/spec-dna-brainstorm")
        .unwrap();
    let b = out.find("wrote .claude/skills/spec-dna-review").unwrap();
    assert!(a < b, "{out}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_skill_that_is_off_is_written_only_when_the_project_lists_it() {
    let r = project_with(&format!(
        "[harness]\nlevel = \"lean\"\nskills = {}\n",
        list(&ON)
    ));
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for n in ON {
        assert!(r.dir.join(path(n)).exists(), "{n}");
    }
    for n in OFF {
        assert!(!r.dir.join(path(n)).exists(), "{n}");
    }
}

#[test]
fn the_skill_files_match_the_golden_files() {
    let r = written();
    for n in NAMES {
        assert_eq!(
            normalize_marker(&r.read(&path(n))),
            golden(&format!("skills/spec-dna-{n}.md")),
            "{n}"
        );
    }
    assert_eq!(
        normalize_marker(&r.read("REVIEW.md")),
        golden("REVIEW.md"),
        "REVIEW.md"
    );
}

/// The text of the skill `name`, as `spec-dna skill show` prints it.
fn shown(name: &str) -> String {
    let (code, out, err) = Repo::new().run(&["skill", "show", name]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// The sentence of 0.7.1, B6: the documented way for a step back to `draft`.
fn step_back_sentence(artifact: &str) -> String {
    format!(
        "To change the {artifact} after its approval, set `status: draft` in the commit of the edit. A hook that runs `spec-dna check` rejects that commit (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on."
    )
}

#[test]
fn specify_and_write_plan_give_the_way_of_the_step_back_to_draft() {
    let s = shown("specify");
    assert!(
        s.contains(&format!("\n7. {}", step_back_sentence("spec"))),
        "{s}"
    );
    let w = shown("write-plan");
    assert!(
        w.contains(&format!("\n8. {}", step_back_sentence("plan"))),
        "{w}"
    );
}

#[test]
fn write_plan_separates_the_estimate_the_actual_hours_and_the_task_for_finish() {
    let w = shown("write-plan");
    assert!(
        w.contains("6. A task can end with `Hours: <n>` or `Hours: <a>-<b>`: the estimate. When the task is done, add `Actual: <n>`: the real hours. `spec-dna finish` adds them and prints the line `Hours:`. For a spike, write one task for each branch."),
        "{w}"
    );
    assert!(
        w.contains("7. Write no task for `spec-dna finish`: it runs after the last task. If the owner reviews the code before `finish`, end the plan with `- [ ] tN: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: <n>.` Only the owner ends this task: an agent never ticks it and never writes `Skip:` for it. After the review, a person ticks it with `Actual: <n>`. Then `spec-dna finish <slug>` runs, and its result goes in a commit of the same pull request."),
        "{w}"
    );
    assert!(!w.contains("`finish` stops while a task is open."), "{w}");
}

#[test]
fn execute_asks_the_person_and_then_adds_the_path_to_files() {
    let e = shown("execute");
    assert!(
        e.contains("If a file is outside `Files`, stop and ask the person. With a yes, add the path to `Files` of `plan.md` in the commit of the edit; a plan with an approval commit in this branch needs a new approval commit first (SD910, reason 8)."),
        "{e}"
    );
    assert!(!e.contains("ask the person to change the plan"), "{e}");
}

#[test]
fn finish_names_the_line_hours_without_its_form() {
    let f = shown("finish");
    assert!(
        f.contains("3. Commit the result. If `spec-dna finish` prints a line `Hours:`, put it into the commit message. Keep it in the squash message."),
        "{f}"
    );
    assert!(!f.contains("Hours: <total>"), "{f}");
}

#[test]
fn the_front_matter_and_the_sizes_follow_the_budgets_of_d6() {
    let r = written();
    let (mut all, mut on) = (0, 0);
    for n in NAMES {
        let f = path(n);
        let t = r.read(&f);
        let head: Vec<&str> = t.lines().take(4).collect();
        assert_eq!(head[0], "---", "{f}");
        assert_eq!(head[1], format!("name: spec-dna-{n}"), "{f}");
        assert!(head[2].starts_with("description: "), "{f}");
        assert_eq!(head[3], "---", "{f}: no other key");
        let d = description_of(&t);
        // The scope of D5, one short sentence, and a plain YAML value.
        assert!(d.starts_with("In a spec-dna project"), "{n}: {d}");
        assert!(d.len() <= 160, "{n}: {} bytes", d.len());
        assert!(d.is_ascii() && d.ends_with('.'), "{n}: {d}");
        assert!(!d.contains(". "), "{n}: one sentence: {d}");
        assert!(!d.contains(": ") && !d.contains(" #"), "{n}: {d}");
        let line = listing_line(n, d).len();
        all += line;
        if ON.contains(&n) {
            on += line;
        }
        let b = body_of(&t);
        assert!(b.len() <= 2500, "{n}: {} bytes", b.len());
        assert!(!b.contains("{{"), "{n}");
    }
    assert!(all <= 1900, "the listing of the 11 skills: {all} bytes");
    assert!(on <= 1250, "the listing of the 7 skills on: {on} bytes");
    let review = r.read("REVIEW.md");
    assert!(review.len() <= 2500, "{} bytes", review.len());
}

#[test]
fn each_body_has_the_5_parts_in_order_and_the_rule_of_q5() {
    let r = written();
    for n in NAMES {
        let t = r.read(&path(n));
        let b = body_of(&t);
        let mut at = 0;
        for part in PARTS {
            let found = b[at..]
                .lines()
                .scan(0, |off, l| {
                    let here = *off;
                    *off += l.len() + 1;
                    Some((here, l))
                })
                .find(|(_, l)| l.starts_with(part));
            let (off, _) = found.unwrap_or_else(|| panic!("{n}: part `{part}` in order\n{b}"));
            at += off + part.len();
        }
        assert!(
            b.contains("\nCommands:\n\n1. "),
            "{n}: numbered commands\n{b}"
        );
        assert!(b.contains(APPROVAL_RULE), "{n}: the rule of Q5\n{b}");
        // A skill never sets a status that a person gives.
        assert!(!b.contains("status: accepted") && !b.contains("status: approved"));
        assert!(!b.contains("small-fix") && b.is_ascii(), "{n}");
    }
}

#[test]
fn review_md_and_the_review_skill_hold_the_same_policy() {
    let r = written();
    let review = r.read("REVIEW.md");
    let (_, policy) = review.split_once("# Review policy\n\n").unwrap();
    let (policy, _) = policy
        .split_once("\n\nspec-dna generates this file.")
        .unwrap();
    assert!(policy.starts_with("## Passes\n"), "{policy}");
    let skill = r.read(&path("review"));
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
fn the_skills_run_their_spec_dna_commands_and_name_their_floor() {
    let r = written();
    for (name, words) in [
        (
            "brainstorm",
            &[
                "`spec-dna new intent <slug>`",
                "--template prd",
                "--template rfc",
                "class",
                "SD306",
                "`spec-dna next <slug>`",
            ][..],
        ),
        (
            "specify",
            &[
                "`spec-dna new spec <slug>`",
                "`spec-dna show <id>#<key>`",
                "SD914",
                "SD920",
            ][..],
        ),
        ("clarify", &["SD919", "SD922", "SD925"][..]),
        (
            "write-plan",
            &[
                "`spec-dna new plan <slug>`",
                "SD913",
                "SD923",
                "SD924",
                "- [ ] <id>: <text>. Files: ",
            ][..],
        ),
        ("execute", &["fresh subagent", "SD913"][..]),
        ("tdd", &["covers:", "SD608", "SD916"][..]),
        ("review", &["`spec-dna status`", "git diff"][..]),
        ("verify", &["SD915", "SD916", "`spec-dna check`"][..]),
        ("finish", &["`spec-dna finish <slug>`"][..]),
        ("debug", &["reproduc", "regression test"][..]),
        ("post-mortem", &["SD918", "from:"][..]),
    ] {
        let b = body_of(&r.read(&path(name))).to_string();
        for w in words {
            assert!(b.contains(w), "{name}: `{w}`\n{b}");
        }
    }
}

#[test]
fn the_team_page_lists_the_skills_and_review_md_only_when_they_are_on() {
    let on = project_with(&all_on());
    let (code, out, err) = on.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let page = on.read(".claude/agents/model-team.md");
    assert!(page.contains("\n## Skills\n"), "{page}");
    for n in NAMES {
        assert!(page.contains(&format!("`spec-dna-{n}`")), "{n}\n{page}");
    }
    assert!(page.contains("`REVIEW.md`"), "{page}");
    // With the keys off, the page and the line are those of step 4 S2.
    let off = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = off.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let page = off.read(".claude/agents/model-team.md");
    assert!(!page.contains("## Skills") && !page.contains("REVIEW.md"));
    assert_eq!(off.read("CLAUDE.md"), on.read("CLAUDE.md"));
    for f in paths(&NAMES)
        .iter()
        .map(String::as_str)
        .chain(["REVIEW.md"])
    {
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
    // The 11 skills add the listing bytes.
    let on = project_with(&all_on());
    let (code, out, err) = on.run(&["init", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let with = request_bytes(&out);
    let (code, out, err) = on.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let listing: usize = NAMES
        .iter()
        .map(|n| listing_line(n, description_of(&on.read(&path(n)))).len())
        .sum();
    assert!(listing > 0 && listing <= 1900, "{listing}");
    assert_eq!(with, base + listing, "{out}");
}

#[test]
fn a_new_project_gets_the_skills_that_are_on_by_default_and_their_bytes() {
    let on = default_skills();
    let on_refs: Vec<&str> = on.iter().map(String::as_str).collect();
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(&format!("skills = {}\n", list(&on_refs))),
        "{cfg}"
    );
    for n in NAMES {
        assert_eq!(
            r.dir.join(path(n)).exists(),
            on.iter().any(|o| o == n),
            "{n}"
        );
    }
    let listing: usize = on_refs
        .iter()
        .map(|n| listing_line(n, description_of(&r.read(&path(n)))).len())
        .sum();
    assert!(listing <= 1900, "{listing}");
}

#[test]
fn an_old_name_gives_exit_code_3_and_names_the_new_names_and_upgrade() {
    for old in OLD_NAMES {
        let r = project_with(&format!(
            "[harness]\nlevel = \"lean\"\nskills = [\"{old}\"]\n"
        ));
        for args in [&["check"][..], &["init"][..], &["next", "x"][..]] {
            let (code, out, err) = r.run(args);
            assert_eq!(code, 3, "{old} {args:?}: {out}{err}");
            assert!(err.contains(&format!("`{old}`")), "{old}: {err}");
            for n in ["brainstorm", "specify", "clarify", "write-plan"] {
                assert!(err.contains(n), "{old}: `{n}` in\n{err}");
            }
            assert!(err.contains("spec-dna upgrade"), "{old}: {err}");
        }
    }
    // `review` is no old name: it stays.
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    // An unknown name lists the 11 names.
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"nope\"]\n");
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("unknown skill `nope`"), "{err}");
    for n in NAMES {
        assert!(err.contains(n), "{n}: {err}");
    }
}

#[test]
fn an_old_name_in_the_answers_of_init_gives_exit_code_2() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "harness-skills = [\"intent\"]\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    // `upgrade` never reads an answers file, so the message names the new
    // name and not the command.
    assert!(
        err.contains("use `brainstorm`") && !err.contains("spec-dna upgrade"),
        "{err}"
    );
    assert!(!r.dir.join("spec-dna.toml").exists());
    for (old, new) in [
        ("spec", "use `specify` and `clarify`"),
        ("plan", "use `write-plan`"),
    ] {
        let r = Repo::new();
        let (code, out, err) = init_with(&r, &format!("harness-skills = [\"{old}\"]\n"), &[]);
        assert_eq!(code, 2, "{out}{err}");
        assert!(
            err.contains(new) && !err.contains("spec-dna upgrade"),
            "{err}"
        );
    }
}

#[test]
fn a_file_with_no_marker_at_a_generated_path_stops_init() {
    for (file, config) in [
        (
            "REVIEW.md".to_string(),
            "[harness]\nlevel = \"lean\"\nreview-md = true\n".to_string(),
        ),
        (
            path("specify"),
            "[harness]\nlevel = \"lean\"\nskills = [\"specify\"]\n".to_string(),
        ),
    ] {
        let r = project_with(&config);
        r.write(&file, "# Mine\n\nMy text.\n");
        let (code, out, err) = r.run(&["init"]);
        assert_eq!(code, 2, "{file}: {out}{err}");
        assert!(
            err.contains(&format!("{file} exists, and spec-dna did not generate it")),
            "{file}: {err}"
        );
        assert_eq!(r.read(&file), "# Mine\n\nMy text.\n");
        assert!(
            !r.dir.join(".claude/agents/verifier.md").exists(),
            "init writes nothing"
        );
    }
}

#[test]
fn reconfigure_removes_an_unedited_skill_and_review_md_that_the_configuration_drops() {
    let r = written();
    let cfg = r
        .read("spec-dna.toml")
        .replace(
            &format!("skills = {}", list(&NAMES)),
            "skills = [\"specify\"]",
        )
        .replace("review-md = true", "review-md = false");
    assert!(cfg.contains("skills = [\"specify\"]"), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    // Plain `init` removes no file.
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in paths(&NAMES)
        .iter()
        .map(String::as_str)
        .chain(["REVIEW.md"])
    {
        assert!(r.dir.join(f).exists(), "plain init keeps {f}");
    }
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in paths(&NAMES)
        .iter()
        .filter(|f| **f != path("specify"))
        .map(String::as_str)
        .chain(["REVIEW.md"])
    {
        assert!(!r.dir.join(f).exists(), "{f}\n{out}");
        // `--reconfigure` also removes the folder of a skill file, if empty.
        if let Some(dir) = Path::new(f)
            .parent()
            .filter(|d| d.starts_with(".claude/skills"))
        {
            assert!(!r.dir.join(dir).exists(), "the folder of {f}");
        }
        assert!(
            out.contains(&format!("remove {f}")) || out.contains(f),
            "{out}"
        );
    }
    assert!(r.dir.join(path("specify")).exists(), "{out}");
    // The adapter skill has its own marker and stays.
    assert!(r.dir.join(".claude/skills/spec-dna/SKILL.md").exists());
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn reconfigure_keeps_an_edited_skill_and_names_it() {
    let r = written();
    let f = path("write-plan");
    let text = format!("{}Own note.\n", r.read(&f));
    r.write(&f, &text);
    let edited = r.read("REVIEW.md").replace("Bugs", "Defects");
    r.write("REVIEW.md", &edited);
    let cfg = r
        .read("spec-dna.toml")
        .replace(&format!("skills = {}", list(&NAMES)), "skills = []");
    r.write(
        "spec-dna.toml",
        &cfg.replace("review-md = true", "review-md = false"),
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read(&f).contains("Own note."));
    assert!(r.read("REVIEW.md").contains("Defects"));
    assert!(
        out.contains(&format!("kept {f}: edited by hand (SD501)")),
        "{out}"
    );
    assert!(
        out.contains("kept REVIEW.md: edited by hand (SD501)"),
        "{out}"
    );
    for n in NAMES.iter().filter(|n| **n != "write-plan") {
        assert!(!r.dir.join(path(n)).exists(), "{n}");
    }
}

#[test]
fn level_none_at_reconfigure_removes_the_skills_and_review_md() {
    let r = written();
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in paths(&NAMES)
        .iter()
        .map(String::as_str)
        .chain(["REVIEW.md"])
    {
        assert!(!r.dir.join(f).exists(), "{f}\n{out}");
    }
    assert!(!r.dir.join(".claude/agents/model-team.md").exists());
    assert!(r.dir.join(".claude/skills/spec-dna/SKILL.md").exists());
}

#[test]
fn check_reports_each_absent_generated_skill_and_review_md() {
    let r = project_with(&all_on());
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    for f in paths(&NAMES)
        .iter()
        .map(String::as_str)
        .chain(["REVIEW.md"])
    {
        assert!(
            out.lines().any(|l| l.contains(f) && l.contains("SD504")),
            "{f}\n{out}"
        );
    }
}

#[test]
fn an_edit_of_a_generated_skill_gives_sd501() {
    let r = written();
    let f = path("brainstorm");
    r.write(&f, &r.read(&f).replacen("When to use:", "When you use:", 1));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.lines().any(|l| l.contains(&f) && l.contains("SD501")),
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
        &format!(
            "level = \"lean\"\nskills = {}\nreview-md = true\n",
            list(&ON)
        ),
    );
    assert!(cfg.contains("review-md = true"), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("skills");
    let (code, out, err) = r.run(&["check", "--base", "HEAD~1"]);
    assert!(!out.contains("SD503"), "{code}: {out}{err}");
}

#[test]
fn a_pin_change_with_the_new_and_the_old_skill_paths_gives_no_sd503() {
    // The pull request of the pin of 0.7.0: the pin moves, `init` writes the
    // 11 skill folders and removes the 3 folders of the 0.6 names.
    let r = Repo::new();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.6.0\"\n\n[[roots]]\npath = \"docs\"\n",
    );
    r.write("docs/a.md", "---\ntype: research\n---\n# A\n\nText.\n");
    for f in OLD_PATHS {
        r.write(f, "An old skill.\n");
    }
    r.commit("base");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n"),
    );
    for f in OLD_PATHS {
        fs::remove_file(r.dir.join(f)).unwrap();
    }
    for n in NAMES {
        r.write(&path(n), "A new skill.\n");
    }
    r.git(&["add", "-A"]);
    let (_, out, err) = r.run(&["check", "--base", "HEAD"]);
    assert!(!out.contains("SD503"), "{out}{err}");
    // The test has teeth: a document in the same change gives SD503.
    r.write("docs/b.md", "---\ntype: research\n---\n# B\n\nText.\n");
    r.git(&["add", "-A"]);
    let (_, out, err) = r.run(&["check", "--base", "HEAD"]);
    assert!(out.contains("SD503"), "{out}{err}");
}

// --- The tests of the adapter block, the levels, the old files and the note of
// --- D5.

#[test]
fn the_adapter_skill_holds_the_discipline_block_and_the_replace_rule() {
    let r = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(".claude/skills/spec-dna/SKILL.md");
    // The 5 rules: one line each, 140 bytes or fewer, with the skill that
    // holds the detail. The 5 lines together have 700 bytes or fewer.
    let mut total = 0;
    for (rule, skill) in [
        ("Test first", "`tdd`"),
        ("Cause first", "`debug`"),
        ("Proof before done", "`verify`"),
        ("Review is data", "`review`"),
        ("Isolated work", "`execute`"),
    ] {
        let l = t
            .lines()
            .find(|l| l.contains(rule))
            .unwrap_or_else(|| panic!("{rule}\n{t}"));
        assert!(l.contains(skill), "{rule}: {l}");
        assert!(l.len() <= 140, "{rule}: {} bytes", l.len());
        total += l.len() + 1;
    }
    assert!(total <= 700, "{total} bytes");
    // The words of D5.
    assert!(
        t.contains(
            "A spec-dna skill replaces a general skill of the same stage. For `write-plan` it replaces `writing-plans`."
        ),
        "{t}"
    );
}

#[test]
fn the_line_of_agents_md_names_the_rules_the_skill_command_and_the_approval_rule() {
    let r = project_with("[harness]\nlevel = \"lean\"\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read("AGENTS.md");
    let line = t
        .lines()
        .find(|l| l.contains("<!-- spec-dna v"))
        .unwrap_or_else(|| panic!("{t}"));
    // The whole line, with its marker, has 400 bytes or fewer.
    assert!(line.len() <= 400, "{} bytes: {line}", line.len());
    assert!(line.contains("spec-dna skill show"), "{line}");
    for w in ["tdd", "debug", "verify", "review", "execute"] {
        assert!(line.contains(w), "{w}: {line}");
    }
    // The same sentence as in each skill (question C of the review).
    assert!(line.contains(APPROVAL_SENTENCE), "{line}");
    assert!(APPROVAL_RULE.contains(APPROVAL_SENTENCE));
}

#[test]
fn level_full_proposes_the_11_skills_and_lean_the_skills_that_are_on_by_default() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "harness = \"full\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(&format!("skills = {}\n", list(&NAMES))),
        "{cfg}"
    );
    for n in NAMES {
        assert!(r.dir.join(path(n)).exists(), "{n}");
    }
    // The level `lean` proposes the skills that are on by default.
    let on = default_skills();
    let on_refs: Vec<&str> = on.iter().map(String::as_str).collect();
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "harness = \"lean\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.read("spec-dna.toml")
            .contains(&format!("skills = {}\n", list(&on_refs)))
    );
}

#[test]
fn init_prints_the_note_of_d5_for_a_plugin_with_the_word_superpowers() {
    let r = Repo::new();
    r.write(
        ".claude/settings.json",
        "{\"enabledPlugins\":{\"superpowers@example\":true}}\n",
    );
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("superpowers"), "{out}");
    assert!(
        out.contains("replaces a general skill of the same stage"),
        "{out}"
    );
    assert!(out.contains("docs/superpowers/specs/"), "{out}");
    // A project with no such plugin gets no note.
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("superpowers"), "{out}");
}

#[test]
fn init_removes_an_unedited_old_skill_file_and_keeps_an_edited_one() {
    let r = project_with(&format!(
        "[harness]\nlevel = \"lean\"\nskills = {}\n",
        list(&ON)
    ));
    for n in OLD_NAMES {
        r.write(&path(n), &old_skill_file(n));
    }
    // An edited file stays, and `check` reports SD501.
    r.write(
        &path("plan"),
        &format!("{}Own note.\n", old_skill_file("plan")),
    );
    r.commit("old skills");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for n in ["intent", "spec"] {
        assert!(!r.dir.join(path(n)).exists(), "{n}\n{out}");
        // The empty folder of the skill goes too.
        assert!(
            !r.dir.join(format!(".claude/skills/spec-dna-{n}")).exists(),
            "the folder of {n}"
        );
        assert!(out.contains(&path(n)), "{n}: {out}");
    }
    assert!(r.dir.join(path("plan")).exists());
    // The folder of a kept file stays, and so does a folder with another file.
    assert!(r.dir.join(".claude/skills/spec-dna-plan").is_dir());
    assert!(
        out.contains(&format!("kept {}: edited by hand (SD501)", path("plan"))),
        "{out}"
    );
}
