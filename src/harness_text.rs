//! The text of the 11 skills of the workflow pack and of `REVIEW.md` (ADR
//! 0008, step 4 S3; ADR 0010). The text is static: no value of the
//! configuration goes into it, so its hash is the same in each project.
//!
//! The layout of this file:
//!
//! - `description(skill)`: one short sentence, in each request.
//! - One constant for each skill, `<NAME>_PARTS`. It holds the 5 parts of D2,
//!   each at the start of a line and in this order: `When to use:`,
//!   `Commands:` (a blank line, then a numbered list), `Artifact:`, `Check:` and
//!   `If the check fails:`.
//! - `body(skill)`: the 5 parts, then `APPROVAL_RULE` (the rule of Q5, the
//!   same words in each skill), then, for `review` only, `REVIEW_POLICY`.
//! - `skill_body(name)`: `body` for a bare name (`write-plan`), for the
//!   command `spec-dna skill show`.
//!
//! The golden file of a skill is `tests/golden/harness/skills/spec-dna-<name>.md`.

use crate::adapters::approval_sentence;
use crate::harness::Skill;

/// The rule of Q5. Each skill body ends with it, in the same words. The line
/// of `AGENTS.md` and `CONTRIBUTING.md` of the build state the same rule.
pub const APPROVAL_RULE: &str = concat!(
    "Never set `accepted` or `approved`. ",
    approval_sentence!(),
    " An approval commit changes only the `status` line."
);

/// The review policy. `REVIEW.md` and the skill `spec-dna-review` both hold it.
pub const REVIEW_POLICY: &str = r"## Passes

Review the diff in 3 passes. For each finding, give the file and the line.

1. Bugs: a logic error, a missing check, a wrong edge case, and a test that does not prove the change.
2. Security: a leaked secret, an unsafe input, and an access that the change does not need.
3. Compliance: compare the diff with `spec.md` and `plan.md` of the change, and with the documents of the layer `rules`. Report a broken requirement, a delta line that the code does not meet, and a file that the `Files` of the plan do not list.

## Severity

- Important: a finding that breaks behavior, leaks data or breaches a policy.
- Nit: each other finding. Report at most 5 Nits in one review.

## Do not report

- A finding of `spec-dna check`. CI runs it.
- The text of a file that spec-dna generates: each file with a `spec-dna harness` marker, `.claude/skills/spec-dna/SKILL.md`, the spec-dna lines of `CLAUDE.md` and `AGENTS.md`, and the entries of `.claude/settings.json` for `model`, `modelSettings`, `fallbackModel`, `advisorModel`, `permissions.deny`, `env.CLAUDE_CODE_SUBAGENT_MODEL` and each `spec-dna hook` command.";

const BRAINSTORM_PARTS: &str = r"When to use: A request starts a change.

Commands:

1. Run `spec-dna new intent <slug>`. Add `--template prd` for a product requirement, or `--template rfc` for a design proposal.
2. Replace each placeholder, also in the title. Write the intent, why now and what is out of scope, in the words of the request.
3. Set `class` in the front matter to `routine` or `higher-risk`. If you are not sure, use `higher-risk`. Set it before the approval commit, because that commit changes the `status` line only.
4. Run `spec-dna next <slug>`. It names the next step.

Artifact: `intent.md` with the intent, why now, out of scope and its class.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD306 (a placeholder stays). Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Replace each placeholder that the message names. Ask the person for each fact that the request does not give. Run the check again.";

const SPECIFY_PARTS: &str = r"When to use: The intent is `accepted` and its class is `higher-risk`.

Commands:

1. Run `spec-dna new spec <slug>`.
2. Under `Delta`, write one list item for each requirement that the change adds, changes or removes: `- ADD <id>#<key>: <sentence>`, `- CHANGE <id>#<key>@<hash>: <sentence>` or `- REMOVE <id>#<key>@<hash>`. `check` and `finish` ignore a paragraph there.
3. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>` and copy the hash.
4. Under `Interfaces`, write `- none` or one paragraph if no interface changes. Else write one item for each interface: `CHANGE <id>: <text>` for an interface that exists, `ADD <id>: <text>` for a new one.
5. Under `Decisions`, link each decision that the change needs: `[[decisions/<slug>]]`.
6. Write each item of `Concerns` and `Open questions` with a box: `- [ ]` is open. If none applies, write one item `- none`.

Artifact: `spec.md` with `Delta`, `Interfaces`, `Decisions`, `Concerns` and `Open questions`.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD914 (an invalid delta or interface item). SD920 (a decision that is absent or not accepted) reports only after the spec is `approved`. Before the approval, read the status of each linked decision. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Fix each item that the message names. For a new hash, run `spec-dna show <id>#<key>` again. Run the check again.";

const CLARIFY_PARTS: &str = r"When to use: `spec.md` has an open concern or an open question.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Give each item a box: `- [ ]` is open, and `- [x]` is resolved. If no item applies, write one item `- none`.
3. Ask the person about each open item. Write the answer in the item, then change its box to `- [x]`.
4. Run the check below.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: Before the approval, confirm that no `- [ ]` item stays in `Concerns` and `Open questions`. SD919 (an open concern) and SD922 (an open question) report it only after the spec is `approved`. Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD925 (an item with no box). Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Ask the person about the item that the message names. Do not mark an item that has no answer. Add a box to an item that has none.";

const WRITE_PLAN_PARTS: &str = r"When to use: Before the code of a change.

Commands:

1. Run `spec-dna next <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Run `spec-dna new plan <slug>`.
3. Under `Files`, list each path of the scope. Write each path in a code span.
4. Under `Order`, write one task for each step of 2 to 5 minutes. Give it a stable id, at most `max-task-files` exact paths (default 3) and one `Verify:`. Use this form: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
5. Write `Risks`. Write `Proof`: the commands that show the result, and the `covers:` ids of the tests.
6. A task can end with `Hours: <n>` or `Hours: <a>-<b>`. `spec-dna finish` adds them and prints the line `Hours: <total>`. For a spike, write one task for each branch. After the spike, add `Skip: <reason>` to each task that is not needed, and tick it.

Artifact: `plan.md` with `Files`, `Order`, `Risks` and `Proof`.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). SD923 (a task with no exact `Files:` or no `Verify:`) and SD924 (a task with too many paths) report only after the plan is `approved`. Before the approval, count the paths and the `Verify:` of each task. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Split a task that lists more than `max-task-files` paths. Give each task exact paths and one `Verify:`. Run the check again.";

const EXECUTE_PARTS: &str = r"When to use: The plan is `approved` and a task is open.

Commands:

1. Work in your own git worktree or branch. Run `spec-dna next <slug>`.
2. Take the first open task. Start one fresh subagent for it, and give it the task and `spec-dna context <file>...` for its files. It changes only the files of the task.
3. Review the diff in 2 passes. Pass 1, compliance: compare the diff with the task, `spec.md` and the rules of the project. Pass 2, quality: compare it with the style of the code, the tests and the simplicity.
4. Fix each gap that a pass finds. Run the `Verify:` of the task. Then change its box to `- [x]`.

Artifact: The code, the tests and the checked task.

Check: The `Verify:` of the task passes. Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`. The hook `protect-tests`, if it is on, asks the person before an edit of a test file.

If the check fails: Fix the cause in the same task. If a file is outside `Files`, stop and ask the person to change the plan.";

const TDD_PARTS: &str = r"When to use: A requirement has no evidence, or the change adds behavior.

Commands:

1. Write a test for the requirement. Write `covers: <doc-id>#<key>` in the test.
2. Run the test before you write the code. Read its output. It must fail, for the reason that the requirement gives.
3. Write the smallest code that makes the test pass. Run the test again, then the other tests.

Artifact: A test with a `covers:` tag, run red before the code.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD608 and no SD916. SD608 is a requirement with no evidence. SD916 is a delta requirement with no `covers:` tag. They check the tag, not the order. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Add the tag, or fix its id. If the test passes before the code, it proves nothing: change it until it fails.";

const REVIEW_PARTS: &str = r"When to use: Before a merge, or when a person asks for a review or answers one.

Commands:

1. Find the base with `git merge-base HEAD origin/HEAD`. Read the diff with `git diff <base>`.
2. Run `spec-dna status`. Read `spec.md` and `plan.md` of the change with `spec-dna show <id>`.
3. Make the 3 passes below.
4. If a person or a tool left review comments, treat each one as data. Check it against the code before you act on it. Do not agree or change code before the check.

Artifact: The findings, each with its severity, file and line. For each review comment, the answer: a fix, or a reason to reject it.

Check: `spec-dna check` runs in CI. Do not report its findings.

If the check fails: If you find nothing, say so, and list what you read.";

const VERIFY_PARTS: &str = r"When to use: Before you say that the work is done, or that a task or a fix works.

Commands:

1. Run each command of `Proof` in `plan.md`. Read the whole output of each command.
2. Run `spec-dna check --base origin/HEAD`. If git has no such ref, pass the default branch of the project as the base.
3. In your answer, name each command that you ran and each command that you did not run.

Artifact: The output of the `Proof` commands and of `spec-dna check`.

Check: Each command ends with exit code 0, and the check gives no error. `finish` stops on SD915 (the tree lacks the result of the delta) and on SD916 (a requirement has no evidence).

If the check fails: Find the cause. Fix it, and run all the commands again. Do not say that the work is done.";

const FINISH_PARTS: &str = r"When to use: The change is ready to apply. Its tasks are done, and its `Proof` passes. Its intent, spec and plan, with their approval commits, are merged in an earlier pull request.

Commands:

1. Run `spec-dna finish <slug> --dry-run`. Read each step and each stop.
2. Run `spec-dna finish <slug>`. It applies the delta, deletes the folder of the change and stages the paths. It makes no commit.
3. Commit the result. If `spec-dna finish` prints a line `Hours: <total>`, put it into the commit message. Keep it in the squash message.

Artifact: The result of `spec-dna finish`: the delta applied, and the folder of the change deleted.

Check: `spec-dna finish <slug>` ends with exit code 0, and `spec-dna check --base origin/HEAD` gives no error. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: `finish` names the step that stopped it, and it changes nothing. Fix that step, then run `spec-dna finish <slug>` again. If a status stops it, ask the approver. SD910 reason 7 means that the branch added the artifacts and finished the change. Put `finish` in a new pull request. Do not delete the folder by hand.";

const DEBUG_PARTS: &str = r"When to use: A test or a command fails, and you do not know the cause.

Commands:

1. Make the failure repeat with one command. Keep the command and its output.
2. Find the cause. Read the code and the output. Do not change the behavior yet.
3. Write a regression test that fails for this cause. Run it.
4. Make one fix. Run the regression test and the earlier tests.

Artifact: A reproduction, a regression test and one fix.

Check: The regression test passes, and the earlier tests pass.

If the check fails: Go back to step 2. Make no new fix without a new cause. After 3 failed fixes, stop and ask the person.";

const POST_MORTEM_PARTS: &str = r"When to use: An `incident` has an open follow-up.

Commands:

1. Run `spec-dna status`. It lists each open follow-up of an incident.
2. Read the incident with `spec-dna show <id>`.
3. Run `spec-dna new intent <slug>`. Add the line `from: <incident id>` to its front matter, and set `class`.
4. Write the intent: what the change gives, why now and what is out of scope.
5. Add an eval for the incident: the file `<id>.json` directly in the evals folder, `[evals] dir`.
6. In the incident, change the open follow-up to `- [x]`, and name the new intent in it.

Artifact: A new `intent.md` with `from:`, and an eval file for the incident.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD101 (a `from:` that names no document), no SD306 and no SD506 (an invalid eval file). SD918 (an incident with no eval) reports only when the diff adds the incident, so check by hand that the eval file exists. Run `spec-dna status`: it lists no open follow-up for the incident. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Add or fix the eval file, or fix the id in `from:`. Run the check again.";

/// The `description` of a skill. It is in each request, so it is one short
/// sentence. It starts with the scope of D5 and then the trigger of D2. The limits of D6 are 160
/// characters for one description, 1,900 bytes for the listing of the 11
/// skills and 1,250 bytes for the listing of the 7 skills of D2.
pub fn description(skill: Skill) -> &'static str {
    match skill {
        Skill::Brainstorm => {
            "In a spec-dna project, when a request starts a change, write intent.md with its class."
        }
        Skill::Specify => {
            "In a spec-dna project, when the intent is accepted and its class is higher-risk, write spec.md with the delta."
        }
        Skill::Clarify => {
            "In a spec-dna project, when spec.md has an open concern or question, close each one before the spec is approved."
        }
        Skill::WritePlan => {
            "In a spec-dna project, before any code, write plan.md with its files and small tasks."
        }
        Skill::Execute => {
            "In a spec-dna project, when the plan is approved and a task is open, run the task with a fresh subagent."
        }
        Skill::Tdd => {
            "In a spec-dna project, when a requirement has no evidence, write a test that fails before the code."
        }
        Skill::Review => {
            "In a spec-dna project, before a merge, review the diff in 3 passes and check each review comment."
        }
        Skill::Verify => {
            "In a spec-dna project, before you say that the work is done, run the proof and read its output."
        }
        Skill::Finish => {
            "In a spec-dna project, when a change is ready, run spec-dna finish to apply its delta."
        }
        Skill::Debug => {
            "In a spec-dna project, when a test or command fails and the cause is not known, find the cause before you fix it."
        }
        Skill::PostMortem => {
            "In a spec-dna project, when an incident has an open follow-up, write a new intent and an eval for it."
        }
    }
}

/// The 5 parts of a skill, with no rule of Q5 and no policy.
fn parts(skill: Skill) -> &'static str {
    match skill {
        Skill::Brainstorm => BRAINSTORM_PARTS,
        Skill::Specify => SPECIFY_PARTS,
        Skill::Clarify => CLARIFY_PARTS,
        Skill::WritePlan => WRITE_PLAN_PARTS,
        Skill::Execute => EXECUTE_PARTS,
        Skill::Tdd => TDD_PARTS,
        Skill::Review => REVIEW_PARTS,
        Skill::Verify => VERIFY_PARTS,
        Skill::Finish => FINISH_PARTS,
        Skill::Debug => DEBUG_PARTS,
        Skill::PostMortem => POST_MORTEM_PARTS,
    }
}

/// The body of a skill: the text between the front matter and the marker. It
/// is the text that `spec-dna skill show` prints.
pub fn body(skill: Skill) -> String {
    let mut s = format!("{}\n\n{APPROVAL_RULE}", parts(skill));
    if skill == Skill::Review {
        s += &format!("\n\n{REVIEW_POLICY}");
    }
    s
}

/// The body of the skill with the bare name `name` (`write-plan`), or `None`
/// for another name. The adapter skill `spec-dna` and the names of 0.6 give
/// `None`.
pub fn skill_body(name: &str) -> Option<String> {
    Skill::from_key(name).map(body)
}

/// The text of `REVIEW.md` above the marker.
pub fn review_md() -> String {
    format!(
        "# Review policy\n\n{REVIEW_POLICY}\n\nspec-dna generates this file. Change `[harness]` in `spec-dna.toml` and run `spec-dna init --reconfigure`.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size of the listing that Claude Code adds to each request: one line
    /// for each skill.
    fn listing(skills: &[Skill]) -> usize {
        skills
            .iter()
            .map(|s| format!("- {}: {}\n", s.name(), description(*s)).len())
            .sum()
    }

    const PARTS: [&str; 5] = [
        "When to use:",
        "Commands:",
        "Artifact:",
        "Check:",
        "If the check fails:",
    ];

    #[test]
    fn each_description_is_short_and_safe_in_yaml() {
        for s in Skill::ALL {
            let d = description(s);
            assert!(d.starts_with("In a spec-dna project"), "{s:?}: {d}");
            assert!(d.len() <= 160, "{s:?}: {} bytes", d.len());
            assert!(d.is_ascii(), "{s:?}");
            assert!(d.ends_with('.') && !d.contains(". "), "{s:?}: one sentence");
            // An unquoted YAML value: no `: `, no ` #`, no leading mark.
            assert!(!d.contains(": ") && !d.contains(" #"), "{s:?}: {d}");
            assert!(!d.starts_with(['`', '#', '-', '&', '*', '!', '|', '>', '"', '\'']));
        }
    }

    #[test]
    fn the_listings_keep_the_budgets_of_d6() {
        let all = listing(&Skill::ALL);
        assert!(all <= 1900, "the 11 skills: {all} bytes");
        // D6 is a budget for the 7 skills of D2.
        let d2: Vec<Skill> = Skill::ALL.into_iter().filter(|s| s.in_d2()).collect();
        let on = listing(&d2);
        assert!(on <= 1250, "the 7 skills of D2: {on} bytes");
    }

    #[test]
    fn the_skills_of_d2_are_the_7_that_the_design_turns_on() {
        let on: Vec<&str> = Skill::ALL
            .iter()
            .filter(|s| s.in_d2())
            .map(|s| s.key())
            .collect();
        assert_eq!(
            on,
            [
                "brainstorm",
                "specify",
                "clarify",
                "write-plan",
                "review",
                "finish",
                "post-mortem"
            ]
        );
    }

    #[test]
    fn each_body_follows_the_text_rules() {
        for s in Skill::ALL {
            let b = body(s);
            assert!(b.len() <= 2500, "{s:?}: {} bytes", b.len());
            assert!(b.is_ascii(), "{s:?}");
            assert!(!b.contains("{{"), "{s:?}");
            assert!(!b.contains("docs/"), "{s:?}");
            assert!(!b.contains("small-fix"), "{s:?}");
            assert!(!b.contains('\r') && !b.ends_with('\n'), "{s:?}");
            assert!(!b.contains("<!-- spec-dna"), "{s:?}: no marker text");
            // The 5 parts, each at the start of a line, in this order.
            let mut at = 0;
            for p in PARTS {
                let found = b[at..]
                    .lines()
                    .scan(0, |off, l| {
                        let here = *off;
                        *off += l.len() + 1;
                        Some((here, l))
                    })
                    .find(|(_, l)| l.starts_with(p));
                let (off, _) = found.unwrap_or_else(|| panic!("{s:?}: `{p}` in order"));
                at += off + p.len();
            }
            assert!(b.contains("\nCommands:\n\n1. "), "{s:?}: numbered commands");
            // The rule of Q5 is in each body, once.
            assert_eq!(b.matches(APPROVAL_RULE).count(), 1, "{s:?}");
        }
    }

    #[test]
    fn the_review_policy_is_one_text_in_two_places() {
        assert!(body(Skill::Review).contains(REVIEW_POLICY));
        assert!(review_md().contains(REVIEW_POLICY));
        for s in Skill::ALL.iter().filter(|s| **s != Skill::Review) {
            assert!(!body(*s).contains(REVIEW_POLICY), "{s:?}");
        }
        for p in [
            "## Passes",
            "Bugs",
            "Security",
            "Compliance",
            "`spec.md` and `plan.md`",
            "the layer `rules`",
            "## Severity",
            "Important",
            "Nit",
            "at most 5 Nits",
            "## Do not report",
            "`spec-dna check`",
            ".claude/skills/spec-dna/SKILL.md",
            "`CLAUDE.md` and `AGENTS.md`",
        ] {
            assert!(REVIEW_POLICY.contains(p), "{p}");
        }
        assert!(REVIEW_POLICY.len() <= 1800, "{} bytes", REVIEW_POLICY.len());
    }

    #[test]
    fn review_md_is_a_title_the_policy_and_the_regeneration_line() {
        let t = review_md();
        assert!(t.len() <= 2500, "{} bytes", t.len());
        assert!(t.starts_with("# Review policy\n\n## Passes\n"), "{t}");
        assert!(t.ends_with(
            "spec-dna generates this file. Change `[harness]` in `spec-dna.toml` and run `spec-dna init --reconfigure`.\n"
        ));
        assert!(!t.contains("{{") && t.is_ascii());
    }

    #[test]
    fn skill_body_takes_a_bare_name_of_the_pack_only() {
        for s in Skill::ALL {
            assert_eq!(skill_body(s.key()), Some(body(s)), "{s:?}");
        }
        for name in [
            "spec-dna",
            "spec-dna-review",
            "intent",
            "spec",
            "plan",
            "nope",
            "",
        ] {
            assert_eq!(skill_body(name), None, "{name}");
        }
    }

    #[test]
    fn each_skill_runs_its_spec_dna_commands() {
        let has = |s: Skill, parts: &[&str]| {
            let b = body(s);
            for p in parts {
                assert!(b.contains(p), "{s:?}: {p}");
            }
        };
        has(
            Skill::Brainstorm,
            &[
                "`spec-dna new intent <slug>`",
                "--template prd",
                "--template rfc",
                "class",
                "SD306",
                "`spec-dna next <slug>`",
            ],
        );
        has(
            Skill::Specify,
            &[
                "`spec-dna new spec <slug>`",
                "`spec-dna show <id>#<key>`",
                "Delta",
                "Interfaces",
                "Decisions",
                "SD914",
                "SD920",
            ],
        );
        has(
            Skill::Clarify,
            &["Concerns", "Open questions", "SD919", "SD922", "SD925"],
        );
        has(
            Skill::WritePlan,
            &[
                "`spec-dna new plan <slug>`",
                "Files",
                "Order",
                "Risks",
                "Proof",
                "- [ ] <id>: <text>. Files: <paths>. Verify: <check>.",
                "SD913",
                "SD923",
                "SD924",
            ],
        );
        has(Skill::Execute, &["fresh subagent", "2 passes", "SD913"]);
        has(Skill::Tdd, &["covers:", "SD608", "SD916"]);
        has(
            Skill::Review,
            &[
                "`spec-dna status`",
                "`spec-dna show <id>`",
                "git diff",
                "3 passes",
            ],
        );
        has(
            Skill::Verify,
            &["`Proof`", "`spec-dna check`", "SD915", "SD916"],
        );
        has(Skill::Finish, &["`spec-dna finish <slug>`", "--dry-run"]);
        has(Skill::Debug, &["regression test", "3 failed fixes"]);
        has(Skill::PostMortem, &["from:", "SD918"]);
    }

    /// The text of one part of a body, from its heading to the next one.
    fn part(skill: Skill, name: &str) -> String {
        let b = body(skill);
        let at = b.find(&format!("\n{name}")).map_or(0, |i| i + 1);
        let rest = &b[at..];
        let end = PARTS
            .iter()
            .filter(|p| **p != name)
            .filter_map(|p| rest[1..].find(&format!("\n{p}")).map(|i| i + 1))
            .min()
            .unwrap_or(rest.len());
        rest[..end].to_string()
    }

    #[test]
    fn a_check_that_reads_the_base_stages_the_new_files_and_names_the_fallback() {
        // `check --base` reads tracked files only, and it needs a base.
        for s in Skill::ALL {
            let b = body(s);
            if !b.contains("check --base origin/HEAD") || s == Skill::Verify {
                continue;
            }
            let c = part(s, "Check:");
            assert!(c.contains("pass the default branch"), "{s:?}: {c}");
            // A skill that never writes a new file still needs no staging:
            // `finish` stages its own paths.
            if s != Skill::Finish {
                assert!(
                    c.contains("`git add`") && c.contains("tracked files only"),
                    "{s:?}: {c}"
                );
            }
        }
        assert!(body(Skill::Verify).contains("pass the default branch"));
    }

    #[test]
    fn finish_needs_the_artifacts_in_an_earlier_pull_request() {
        assert!(part(Skill::Finish, "When to use:").contains("earlier pull request"));
        let f = part(Skill::Finish, "If the check fails:");
        assert!(
            f.contains("SD910 reason 7") && f.contains("new pull request"),
            "{f}"
        );
    }

    #[test]
    fn the_delta_forms_are_list_items() {
        let b = body(Skill::Specify);
        for p in [
            "`- ADD <id>#<key>: <sentence>`",
            "`- CHANGE <id>#<key>@<hash>: <sentence>`",
            "`- REMOVE <id>#<key>@<hash>`",
        ] {
            assert!(b.contains(p), "{p}");
        }
    }

    #[test]
    fn write_plan_names_the_task_labels_hours_and_skip() {
        let b = body(Skill::WritePlan);
        assert!(b.contains("at most `max-task-files` exact paths (default 3)"));
        assert!(!b.contains("at most 3") && !b.contains("more than 3 paths"));
        assert!(b.contains(
            "6. A task can end with `Hours: <n>` or `Hours: <a>-<b>`. `spec-dna finish` adds them and prints the line `Hours: <total>`. For a spike, write one task for each branch. After the spike, add `Skip: <reason>` to each task that is not needed, and tick it.\n"
        ));
        assert!(!b.contains("## Hours") && !b.contains("number of hours"));
    }

    #[test]
    fn a_floor_that_cannot_fail_yet_is_not_claimed() {
        // SD919 and SD922 report an `approved` spec only.
        let c = part(Skill::Clarify, "Check:");
        assert!(c.contains("no `- [ ]` item stays"), "{c}");
        assert!(c.contains("SD919") && c.contains("only after the spec is `approved`"));
        // SD920 reports an `approved` spec only; SD914 reports a draft.
        let c = part(Skill::Specify, "Check:");
        assert!(c.contains("SD920") && c.contains("only after the spec is `approved`"));
        // SD923 and SD924 report an `approved` plan only.
        let c = part(Skill::WritePlan, "Check:");
        assert!(c.contains("SD924") && c.contains("only after the plan is `approved`"));
        // SD918 reports only when the diff adds the incident.
        let c = part(Skill::PostMortem, "Check:");
        assert!(!c.contains("gives no SD918"), "{c}");
        assert!(c.contains("SD918") && c.contains("only when the diff adds the incident"));
        assert!(c.contains("`spec-dna status`"), "{c}");
    }

    #[test]
    fn a_skill_never_sets_a_status_that_a_person_gives() {
        for s in Skill::ALL {
            let b = body(s);
            assert!(!b.contains("status: accepted") && !b.contains("status: approved"));
            assert!(b.contains("Never set `accepted` or `approved`."), "{s:?}");
        }
    }
}
