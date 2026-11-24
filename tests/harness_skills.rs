//! Tests of the 11 skills of the workflow pack and of `REVIEW.md` (ADR 0010,
//! 0.7.0 slice 4; design 0.8.0, D71 and D76). The configuration keys `skills` and `review-md` select the
//! files. Each test uses a temporary repository. No test reads or writes a
//! file of this repository.

mod common;

use common::{Repo, VERSION, normalize_marker};
use std::fs;
use std::path::Path;

/// The 11 names, in the order of D2. The skill `accept` merged into `review`
/// (design 0.8.0, D71).
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
const APPROVAL_RULE: &str = "Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.";

/// The rule that ends the skill `review` of `standard` and `strict` (D71 item 7):
/// the first sentence is for the `status` line of an artifact, because the
/// acceptor writes `result = "accepted"`.
const ACCEPT_RULE: &str = "Never set the `status` line of an artifact to `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.";

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
        s.contains(&format!("\n8. {}", step_back_sentence("spec"))),
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
        w.contains("7. Write no task for `spec-dna finish`: it runs last. Write no task for the review or the acceptance: `next` names them. If the owner reviews the code first, end the plan with `- [ ] tN: Review by the owner. Files: none. Verify: the owner reviewed the code. By: person. Hours: <n>.` Only the owner ends this task: an agent never ticks it or skips it. After the review, a person ticks it with `Actual: <n>`. Then run `spec-dna finish <slug>`: its result goes in a commit of the same pull request."),
        "{w}"
    );
    assert!(!w.contains("`finish` stops while a task is open."), "{w}");
}

#[test]
fn execute_asks_the_person_and_then_adds_the_path_to_files() {
    let e = shown("execute");
    assert!(
        e.contains("If a file is outside `Files`, stop and ask the person. With a yes, add the path to `Files` of `plan.md` in the commit of the edit. A plan with an approval commit in this branch needs a new approval commit first (SD910, reason 8)."),
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

/// D54 and S8b: the skill `finish` tells an agent what to do with a due
/// summary that `next` names. The edit comes in the build, before `finish`
/// runs, because `finish` deletes the folder and re-pins no later edit. A
/// higher summary that pins the edited one gets `refresh`, or `refresh
/// --holds` when its text did not change. A stale summary goes to the owner.
#[test]
fn finish_names_the_due_summary_the_holds_option_and_the_stale_report() {
    let f = shown("finish");
    for w in [
        "1. If `next` names a due summary, edit its text before `finish` when its meaning changes.",
        "run `spec-dna refresh <id>` on a higher summary that pins it, with `--holds` if its meaning holds",
        "Name each other due summary with `--holds <id>`, one option for each id.",
        "`roll-up: stale <id>`",
        "tell the owner. That summary needs an edit and `spec-dna refresh <id>` in a new commit, or `spec-dna refresh <id> --holds` if its text holds.",
    ] {
        assert!(f.contains(w), "`{w}`\n{f}");
    }
    // The edit sits before the run of `finish`, and no sentence after it.
    let edit = f.find("edit its text").expect("edit");
    let run = f.find("2. Run `spec-dna finish <slug>`.").expect("step 2");
    assert!(edit < run, "{f}");
    assert!(!f.contains("in the same commit"), "{f}");
    let file = written().read(&path("finish"));
    assert!(
        body_of(&file).len() <= 2500,
        "{} bytes",
        body_of(&file).len()
    );
}

/// D54: a due summary that `next` names is no file outside `Files`, so
/// `execute` does not stop and ask for it.
#[test]
fn execute_does_not_stop_for_a_due_summary() {
    let e = shown("execute");
    assert!(
        e.contains("A due summary that `next` names is no such file: edit it with no stop."),
        "{e}"
    );
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
        // The skill `review` limits the first sentence to the `status` line: its
        // acceptor sets `result = "accepted"` in the record (D71 item 7).
        let rule = if n == "review" {
            ACCEPT_RULE
        } else {
            APPROVAL_RULE
        };
        assert!(b.contains(rule), "{n}: the rule of Q5\n{b}");
        if n == "review" {
            assert!(!b.contains("Never set `accepted` or `approved`."), "{b}");
        }
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
    // The skill `review` brings the agent `acceptor`: its description is in
    // each request too.
    let agent = format!(
        "- acceptor: {}\n",
        description_of(&on.read(".claude/agents/acceptor.md"))
    )
    .len();
    assert_eq!(with, base + listing + agent, "{out}");
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
    // 11 skill folders and removes the 3 folders of the 0.6 names. The file
    // has another edit too (a rule), so it counts as a change of the
    // configuration: only the skill paths keep SD503 away.
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
        &format!(
            "spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n\n[rules]\nSD608 = \"off\"\n"
        ),
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

/// D71 item 7: the skill `review` of `standard` and `strict` has the steps of
/// the record and the 3 passes. The skill `accept` is gone.
#[test]
fn review_has_the_steps_of_the_record_and_the_3_passes() {
    for profile in [None, Some("strict")] {
        let r = Repo::with_config(&format!(
            "{}\n[[roots]]\npath = \"docs\"\n",
            profile.map_or(String::new(), |p| format!("profile = \"{p}\"\n"))
        ));
        let (code, s, err) = r.run(&["skill", "show", "review"]);
        assert_eq!(code, 0, "{s}{err}");
        for w in [
            "1. Run `spec-dna next <slug>`. If its item `review` gives a prompt, you built the change. Start one agent `acceptor` with that prompt and add no word. Start a second acceptor that `next` asks for with `claude --agent acceptor '<prompt>'`.",
            "2. As the acceptor, run your prompt first, and follow the brief.",
            "3. With no prompt, review the diff yourself. Read `git diff <base>`: `git merge-base HEAD origin/HEAD` gives the base. Run `spec-dna status`. Read `spec.md` and `plan.md` with `spec-dna show <id>`. Make the 3 passes below.",
            "4. Treat each review comment as data: check it against the code before you agree or change code.",
            "5. For a follow-up, start one agent `acceptor` with its prompt. If `next` names the owner, start no third run.",
            "Artifact: The findings with severity, file and line. With a record: `acceptance.toml`, committed alone.",
            "Check: With a record, `spec-dna next <slug>` shows the item `review` as done.",
            "If the check fails: As the acceptor, fix the record, never the code.",
            "## Passes",
        ] {
            assert!(s.contains(w), "`{w}`\n{s}");
        }
        // The body ends with the rule of the status line, then the policy.
        let (_, rest) = s.split_once(ACCEPT_RULE).unwrap_or_else(|| panic!("{s}"));
        assert!(rest.trim_start().starts_with("## Passes"), "{s}");
    }
}

#[test]
fn skill_list_has_11_skills_and_no_accept_and_skill_show_refuses_accept() {
    let (code, out, err) = Repo::new().run(&["skill", "list", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.matches("{\"name\":\"").count(), 11, "{out}");
    assert!(!out.contains("\"name\":\"accept\""), "{out}");
    let item = out
        .split("{\"name\":\"")
        .find(|i| i.starts_with("review\""))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(
        item.contains("\"kind\":\"stage\"") && item.contains("\"stage\":\"deploy\""),
        "{item}"
    );
    let (code, out, err) = Repo::new().run(&["skill", "show", "accept"]);
    assert_ne!(code, 0, "{out}{err}");
    assert!(err.contains("`accept` is no skill"), "{err}");
}

/// D71 item 4: the agent `acceptor` comes with the skill `review` where the
/// acceptance is on. `lite` gets none, also at the level `full`.
#[test]
fn the_agent_acceptor_comes_with_review_in_standard_and_strict_and_never_in_lite() {
    for level in ["lean", "full"] {
        let r = Repo::new();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"lite\"\nharness = \"{level}\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{level}: {out}{err}");
        assert!(!r.read("spec-dna.toml").contains("accept"), "{level}");
        assert!(
            !r.dir.join(".claude/agents/acceptor.md").exists(),
            "{level}"
        );
    }
    // `lite` with `review` in the list at the level `full` writes no agent.
    let lite = Repo::with_config(&format!(
        "profile = \"lite\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"full\"\nskills = {}\n",
        list(&V071)
    ));
    let (code, out, err) = lite.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!lite.dir.join(".claude/agents/acceptor.md").exists());
    assert!(lite.dir.join(path("review")).exists());
    // `standard` and `strict` write it at the level `lean`.
    for profile in ["standard", "strict"] {
        let r = Repo::new();
        let (code, out, err) = init_with(
            &r,
            &format!("profile = \"{profile}\"\nharness = \"lean\"\n"),
            &[],
        );
        assert_eq!(code, 0, "{profile}: {out}{err}");
        assert!(!r.dir.join(path("accept")).exists(), "{profile}: {out}");
        assert!(r.dir.join(path("review")).exists(), "{profile}: {out}");
        assert!(
            r.dir.join(".claude/agents/acceptor.md").exists(),
            "{profile}: {out}"
        );
        // D81: `standard` gets the line of `advisor = false`, and `strict` gets
        // the file of the freeze, with the hash `f541cad357de8926`.
        let want = if profile == "standard" {
            "acceptor-no-advisor.md"
        } else {
            "acceptor.md"
        };
        assert_eq!(
            normalize_marker(&r.read(".claude/agents/acceptor.md")),
            golden(want),
            "{profile}"
        );
    }
    // No `review`, no agent.
    let r = project_with("[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\"]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".claude/agents/acceptor.md").exists());
}

/// D76 and D71: the golden file of each skill that is not `lite` differs from
/// the file of 0.7.1. This table names each change, line by line: the lines of
/// 0.7.1 that go, and the lines that come. The line of the rule of Q5 is in each
/// skill, and a test of its own checks it.
#[test]
fn the_golden_files_of_standard_differ_from_0_7_1_in_the_named_lines_only() {
    const NAMED: [(&str, &[&str], &[&str]); 11] = [
        ("brainstorm", &[], &[]),
        (
            "specify",
            &[
                r#"7. To change the spec after its approval, set `status: draft` in the commit of the edit. A hook that runs `spec-dna check` rejects that commit (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on."#,
                r#"Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD914 (an invalid delta or interface item). SD920 (a decision that is absent or not accepted) reports only after the spec is `approved`. Before the approval, read the status of each linked decision. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`."#,
            ],
            &[
                r#"7. If the spec has the section `Invariants`, list each rule that the change must keep true, or keep `- none`. Write each item as `- <doc-id>#<key>@<hash>`, with the hash from `spec-dna show`. A rule that the delta adds or removes is no invariant."#,
                r#"8. To change the spec after its approval, set `status: draft` in the commit of the edit. A hook that runs `spec-dna check` rejects that commit (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on."#,
                r#"Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD914 (an invalid delta or interface item) and no SD936 (an invalid invariant). SD920 (a decision that is absent or not accepted) reports only after the spec is `approved`. Before the approval, read the status of each linked decision. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`."#,
            ],
        ),
        ("clarify", &[], &[]),
        (
            "write-plan",
            &[
                r#"7. Write no task for `spec-dna finish`: it runs after the last task. If the owner reviews the code before `finish`, end the plan with `- [ ] tN: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: <n>.` Only the owner ends this task: an agent never ticks it and never writes `Skip:` for it. After the review, a person ticks it with `Actual: <n>`. Then `spec-dna finish <slug>` runs, and its result goes in a commit of the same pull request."#,
                r#"Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). SD923 (a task with no exact `Files:` or no `Verify:`) and SD924 (too many paths) report only after the plan is `approved`. Before the approval, count the paths. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`."#,
            ],
            &[
                r#"7. Write no task for `spec-dna finish`: it runs last. Write no task for the review or the acceptance: `next` names them. If the owner reviews the code first, end the plan with `- [ ] tN: Review by the owner. Files: none. Verify: the owner reviewed the code. By: person. Hours: <n>.` Only the owner ends this task: an agent never ticks it or skips it. After the review, a person ticks it with `Actual: <n>`. Then run `spec-dna finish <slug>`: its result goes in a commit of the same pull request."#,
                r#"Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). SD923 (a task with no exact `Files:` or no `Verify:`) and SD924 (too many paths) report only after the plan is `approved`. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`."#,
            ],
        ),
        (
            "execute",
            &[
                r#"2. Take the first open task. Only the review of the owner ends the task `Review by the owner`: never tick it and never write `Skip:` for it. Start one fresh subagent for it, and give it the task and `spec-dna context <file>...` for its files. It changes only the files of the task."#,
                r#"If the check fails: Fix the cause in the same task. If a file is outside `Files`, stop and ask the person. With a yes, add the path to `Files` of `plan.md` in the commit of the edit; a plan with an approval commit in this branch needs a new approval commit first (SD910, reason 8)."#,
            ],
            &[
                r#"2. Take the first open task. A task with `By: person` is not yours: never tick it. Only the review of the owner ends the task `Review by the owner`: never tick it or write `Skip:` for it. Start one fresh subagent for the task. Give the subagent the task and `spec-dna context <file>...` for its files. The subagent changes only the files of the task."#,
                r#"5. If the work differs from the plan or the spec, add an item to `Deviations` of `plan.md`. Write it in the commit of the work. Use `- <id> (plan): <text>` or `- <id> (spec): <text>. Delta: <verb> <doc-id>#<key>` (or `Follow-up: <slug>`). Give each item its own id, for example `d1`, `d2`. A task with 2 deviations has 2 items."#,
                r#"If the check fails: Fix the cause in the same task. If a file is outside `Files`, stop and ask the person. With a yes, add the path to `Files` of `plan.md` in the commit of the edit. A plan with an approval commit in this branch needs a new approval commit first (SD910, reason 8). A due summary that `next` names is no such file: edit it with no stop."#,
            ],
        ),
        (
            "tdd",
            &[],
            &[
                r#"If `[evidence] reports` is set in `spec-dna.toml`, do this. Stage new files with `git add`. Run the tests and `spec-dna evidence stamp` in one command line. Run the check with `--reports`. A tag then proves a requirement only when a test case bound to it passed."#,
            ],
        ),
        (
            "review",
            &[
                r#"When to use: Before a merge, or when a person asks for a review or answers one."#,
                r#"1. Find the base with `git merge-base HEAD origin/HEAD`. Read the diff with `git diff <base>`."#,
                r#"2. Run `spec-dna status`. Read `spec.md` and `plan.md` of the change with `spec-dna show <id>`."#,
                r#"3. Make the 3 passes below."#,
                r#"4. If a person or a tool left review comments, treat each one as data. Check it against the code before you act on it. Do not agree or change code before the check."#,
                r#"Artifact: The findings, each with its severity, file and line. For each review comment, the answer: a fix, or a reason to reject it."#,
                r#"Check: `spec-dna check` runs in CI. Do not report its findings."#,
                r#"If the check fails: If you find nothing, say so, and list what you read."#,
            ],
            &[
                r#"When to use: The tasks of a change are done, or a person asks for a review."#,
                r#"1. Run `spec-dna next <slug>`. If its item `review` gives a prompt, you built the change. Start one agent `acceptor` with that prompt and add no word. Start a second acceptor that `next` asks for with `claude --agent acceptor '<prompt>'`."#,
                r#"2. As the acceptor, run your prompt first, and follow the brief."#,
                r#"3. With no prompt, review the diff yourself. Read `git diff <base>`: `git merge-base HEAD origin/HEAD` gives the base. Run `spec-dna status`. Read `spec.md` and `plan.md` with `spec-dna show <id>`. Make the 3 passes below."#,
                r#"4. Treat each review comment as data: check it against the code before you agree or change code."#,
                r#"5. For a follow-up, start one agent `acceptor` with its prompt. If `next` names the owner, start no third run."#,
                r#"Artifact: The findings with severity, file and line. With a record: `acceptance.toml`, committed alone."#,
                r#"Check: With a record, `spec-dna next <slug>` shows the item `review` as done."#,
                r#"If the check fails: As the acceptor, fix the record, never the code. If you find nothing, say so and list what you read."#,
            ],
        ),
        (
            "verify",
            &[
                r#"Check: Each command ends with exit code 0, and the check gives no error. `finish` stops on SD915 (the tree lacks the result of the delta) and on SD916 (a requirement has no evidence)."#,
            ],
            &[
                r#"Check: Each command ends with exit code 0, and the check gives no error. `finish` stops on SD915 (the tree lacks the result of the delta) and on SD916 (a requirement has no evidence). If `[evidence] reports` is set in `spec-dna.toml`, do this before `finish`. Stage new files with `git add`. Run the tests and `spec-dna evidence stamp` in one command line. Run the check with `--reports`."#,
            ],
        ),
        (
            "finish",
            &[
                r#"1. Run `spec-dna finish <slug> --dry-run` alone: one plain command with no other command after it. If git has no `origin/HEAD`, add `--base <rev>`. Read each step and each stop."#,
                r#"2. Run `spec-dna finish <slug>`. It applies the delta, deletes the folder of the change and stages the paths. It makes no commit."#,
                r#"If the check fails: A command that the tool denies is no stop of `finish`: run the plain command again. A stop is a line `stopped at step N` that `spec-dna finish` prints. The stop changes nothing. Fix that step, then run `spec-dna finish <slug>` again. A stop at step 0 lists each open task of `plan.md` (do the task, or write a reason in `Skip:`) or each reference into the folder. If the stop names a status, ask the approver. If `spec-dna finish` prints no stop, read its message: it can name an edit that is not committed. If `spec-dna check --base origin/HEAD` gives SD910 reason 7, the branch added the artifacts and finished the change: put `finish` in a new pull request. Do not delete the folder by hand."#,
            ],
            &[
                r#"1. If `next` names a due summary, edit its text before `finish` when its meaning changes. Then run `spec-dna refresh <id>` on a higher summary that pins it, with `--holds` if its meaning holds. Run `spec-dna finish <slug> --dry-run` alone: one plain command with no other command after it. If git has no `origin/HEAD`, add `--base <rev>`. Read each step and each stop."#,
                r#"2. Run `spec-dna finish <slug>`. It applies the delta, deletes the folder of the change and stages the paths. It makes no commit. Name each other due summary with `--holds <id>`, one option for each id. If it prints `Deviations: <n>` and one line for each, tell the owner: the merge acknowledges them. With `approval = branch`, an approver acknowledges them with `spec-dna approve <slug> acceptance`."#,
                r#"If the check fails: A command that the tool denies is no stop of `finish`: run the plain command again. A stop is a line `stopped at step N` that `spec-dna finish` prints. The stop changes nothing. Fix that step, then run `spec-dna finish <slug>` again. A stop at step 0 lists each open task of `plan.md` (do it, or write a reason in `Skip:`) or each reference into the folder. If the stop names a status, ask the approver."#,
                r#"If `spec-dna finish` prints no stop, read its message: it can name an uncommitted edit. If `spec-dna check --base origin/HEAD` gives SD910 reason 7, the branch added the artifacts and finished the change. Put `finish` in a new pull request. Do not delete the folder by hand. If `spec-dna finish` prints `roll-up: stale <id>`, tell the owner. That summary needs an edit and `spec-dna refresh <id>` in a new commit, or `spec-dna refresh <id> --holds` if its text holds."#,
            ],
        ),
        ("debug", &[], &[]),
        ("post-mortem", &[], &[]),
    ];

    // The rule of 0.7.1 has no sentence on `acknowledged-by`.
    const RULE_OLD: &str = "Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.";
    for (n, gone, came) in NAMED {
        let old = golden(&format!("skills-0.7.1/spec-dna-{n}.md"));
        let new = golden(&format!("skills/spec-dna-{n}.md"));
        let rule = |t: &str| {
            t.lines()
                .find(|l| l.starts_with("Never set "))
                .unwrap_or_else(|| panic!("{n}: no rule"))
                .to_string()
        };
        assert_eq!(rule(&old), RULE_OLD, "{n}: the rule of 0.7.1");
        let want = if n == "review" {
            ACCEPT_RULE
        } else {
            APPROVAL_RULE
        };
        assert_eq!(rule(&new), want, "{n}: the rule of 0.8.0");
        let other = |l: &&str| {
            !l.is_empty() && !l.starts_with("<!-- spec-dna harness") && !l.starts_with("Never set ")
        };
        let removed: Vec<&str> = old
            .lines()
            .filter(other)
            .filter(|l| !new.lines().any(|m| m == *l))
            .collect();
        let added: Vec<&str> = new
            .lines()
            .filter(other)
            .filter(|l| !old.lines().any(|m| m == *l))
            .collect();
        assert_eq!(removed, gone, "{n}: lines of 0.7.1 that go");
        assert_eq!(added, came, "{n}: lines that come");
    }
}

/// The 11 skills of 0.7.1, in the order of D2.
const V071: [&str; 11] = [
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

/// A committed project of the profile `lite` with the 11 skills of 0.7.1.
fn lite_project() -> Repo {
    let r = Repo::with_config(&format!(
        "profile = \"lite\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\nskills = {}\nreview-md = true\n",
        list(&V071)
    ));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    r
}

/// D76: in `lite`, the text of each skill is the text of 0.7.1, byte for byte.
#[test]
fn lite_writes_each_skill_with_the_bytes_of_0_7_1() {
    let r = lite_project();
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    for n in V071 {
        assert_eq!(
            normalize_marker(&r.read(&path(n))),
            golden(&format!("skills-0.7.1/spec-dna-{n}.md")),
            "{n}"
        );
    }
    // `REVIEW.md` is the same in each profile.
    assert_eq!(
        normalize_marker(&r.read("REVIEW.md")),
        golden("REVIEW.md"),
        "REVIEW.md"
    );
    // The generated files are in sync with the profile.
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

/// D76 item 4: the team page of `lite` is the page of 0.7.1, byte for byte. It
/// has no row for `acceptor` and no skill `accept`. The page of 0.7.1 is the page
/// of the level `full` with the 11 skills and `REVIEW.md`.
#[test]
fn lite_writes_the_team_page_of_0_7_1() {
    let r = Repo::with_config(&format!(
        "profile = \"lite\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"full\"\nskills = {}\nreview-md = true\nprotect-tests = \"fix-branches\"\n",
        list(&V071)
    ));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let page = normalize_marker(&r.read(".claude/agents/model-team.md"));
    assert_eq!(page, golden("model-team-0.7.1.md"));
    assert!(!page.contains("acceptor"), "{page}");
    assert!(!r.dir.join(".claude/agents/acceptor.md").exists());
    // The same project in `standard` has the row of `acceptor`.
    let s = Repo::with_config(&format!(
        "profile = \"standard\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"full\"\nskills = {}\nreview-md = true\nprotect-tests = \"fix-branches\"\n",
        list(&V071)
    ));
    s.write("docs/a.md", "# A\n\nText.\n");
    s.commit("start");
    let (code, out, err) = s.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        s.read(".claude/agents/model-team.md")
            .contains("| acceptor | `.claude/agents/acceptor.md` | opus | high |")
    );
}

/// D76: a `lite` configuration at the level `none` that `init --reconfigure` moves
/// to `full` gets the text of 0.7.1, no key `acceptor` and no agent file. The
/// reconfigure replaces the harness with its defaults, so the profile must follow.
#[test]
fn reconfigure_of_a_lite_configuration_from_level_none_writes_lite_text_and_no_acceptor() {
    let r = Repo::with_config(
        "profile = \"lite\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"none\"\n",
    );
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let answers = format!("harness = \"full\"\nharness-skills = {}\n", list(&V071));
    let (code, out, err) = init_with(&r, &answers, &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("acceptor"), "{cfg}");
    assert!(!r.dir.join(".claude/agents/acceptor.md").exists(), "{out}");
    for n in V071 {
        assert_eq!(
            normalize_marker(&r.read(&path(n))),
            golden(&format!("skills-0.7.1/spec-dna-{n}.md")),
            "{n}"
        );
    }
    // A second `init` changes nothing.
    r.commit("init");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("everything is up to date"), "{out}");
    assert_eq!(r.git_out(&["status", "--porcelain"]), "", "{out}");
}

/// D76: `skill show` in a project of the profile `lite` prints the body of 0.7.1.
#[test]
fn skill_show_in_a_lite_project_prints_the_body_of_0_7_1() {
    let r = lite_project();
    for n in V071 {
        let (code, out, err) = r.run(&["skill", "show", n]);
        assert_eq!(code, 0, "{n}: {out}{err}");
        assert_eq!(
            out.trim_matches('\n'),
            body_of(&golden(&format!("skills-0.7.1/spec-dna-{n}.md"))),
            "{n}"
        );
    }
}

/// D76 item 6: a change of the profile from `lite` to `standard` changes the
/// text of each skill, so SD502 reports each skill file.
#[test]
fn a_change_of_profile_from_lite_to_standard_gives_sd502_for_each_skill_file() {
    let r = lite_project();
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write(
        "spec-dna.toml",
        &r.read("spec-dna.toml")
            .replace("profile = \"lite\"", "profile = \"standard\""),
    );
    let (_, out, err) = r.run(&["check"]);
    for n in V071 {
        assert!(
            out.lines()
                .any(|l| l.contains("SD502") && l.contains(&path(n))),
            "{n}\n{out}{err}"
        );
    }
}

/// D75 item 6: the skill `write-plan` of `standard` and `strict` says that the plan
/// has no task for the review or the acceptance. `lite` keeps the text of 0.7.1.
#[test]
fn write_plan_says_that_next_names_the_review_and_the_acceptance_except_in_lite() {
    const SENTENCE: &str = "Write no task for the review or the acceptance: `next` names them.";
    let golden_text = golden("skills/spec-dna-write-plan.md");
    assert!(golden_text.contains(SENTENCE), "standard");
    let old = golden("skills-0.7.1/spec-dna-write-plan.md");
    assert!(!old.contains(SENTENCE), "0.7.1");
}

/// D76 item 6: a change of the profile changes the generated files, and the
/// message of SD502 names the profile, so a reader sees why a file that nobody
/// edited is outdated.
#[test]
fn sd502_names_the_profile_after_a_change_of_the_profile() {
    let r = Repo::with_config(&format!(
        "profile = \"lite\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"full\"\nskills = {}\nreview-md = true\nprotect-tests = \"fix-branches\"\n",
        list(&V071)
    ));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r
        .read("spec-dna.toml")
        .replace("profile = \"lite\"", "profile = \"standard\"");
    r.write("spec-dna.toml", &cfg);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    let review = out
        .lines()
        .find(|l| l.contains("spec-dna-review/SKILL.md") && l.contains("SD502"))
        .unwrap_or_else(|| panic!("no SD502 for the review skill:\n{out}"));
    assert!(review.contains("profile `standard`"), "{review}");
    // Each SD502 of a generated skill file names it.
    for l in out
        .lines()
        .filter(|l| l.contains("SD502") && l.contains("/SKILL.md"))
    {
        assert!(l.contains("profile `standard`"), "{l}");
    }
}
