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

use crate::harness::Skill;

/// The rule of Q5. Each skill body ends with it, in the same words. The line
/// of `AGENTS.md` and `CONTRIBUTING.md` of the build state the same rule.
pub const APPROVAL_RULE: &str = "Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.";

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
2. Write what the change gives, why now and what it does not do, in the words of the request. Set `class` to `routine` or `higher-risk`. If you are not sure, use `higher-risk`.
3. Run `spec-dna next <slug>`.

Artifact: `intent.md` with its class.

Check: `spec-dna check` gives no SD306 (a placeholder stays).

If the check fails: Replace each placeholder. Ask the person for each fact that the request does not give. Run `spec-dna check` again.";

const SPECIFY_PARTS: &str = r"When to use: The intent is `accepted` and its class is `higher-risk`.

Commands:

1. Run `spec-dna new spec <slug>`.
2. Under `Delta`, write one item for each requirement that the change adds, changes or removes. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>` and copy the hash.
3. Write the interfaces under `Interfaces` and the decisions under `Decisions`.

Artifact: `spec.md` with `Delta`, `Interfaces` and `Decisions`.

Check: `spec-dna check` gives no SD914 (a delta line) and no SD920 (a decision).

If the check fails: Fix each item that the message names. Run `spec-dna check` again.";

const CLARIFY_PARTS: &str = r"When to use: `spec.md` has an open concern or an open question.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Ask the person about each open item. Write the answer in the item, and mark its box `[x]`.
3. Give each item a task box, `- [ ]` or `- [x]`.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: `spec-dna check` gives no SD919, SD922 and SD925.

If the check fails: Ask the person about the item that the message names. Do not mark an item that is not answered.";

const WRITE_PLAN_PARTS: &str = r"When to use: Before the code of a change.

Commands:

1. Run `spec-dna new plan <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Under `Files`, list each path of the scope. Under `Order`, write one small task for each step: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
3. Write `Risks` and `Proof`.

Artifact: `plan.md` with `Files` and small tasks.

Check: `spec-dna check` gives no SD913, SD923 and SD924.

If the check fails: Split a task that lists too many paths. Give each task exact paths in `Files:` and one `Verify:`. Run `spec-dna check` again.";

const EXECUTE_PARTS: &str = r"When to use: The plan is `approved` and a task is open.

Commands:

1. Run `spec-dna next <slug>`. Take the first open task.
2. Start one fresh subagent for the task. It changes only the files of the task.
3. Review the diff in 2 passes: compliance with the task, `spec.md` and the rules, then quality.

Artifact: The code, the tests and the checked task.

Check: The `Verify:` of the task passes. `spec-dna check` gives no SD913.

If the check fails: Fix the cause in the same task. If a file is outside `Files`, ask the person to change the plan.";

const TDD_PARTS: &str = r"When to use: A requirement has no evidence.

Commands:

1. Write a test with the tag `covers: <doc-id>#<key>`.
2. Run the test. It must fail, and for the right reason, before you write the code.
3. Write the code. Run the test again.

Artifact: A test with a `covers:` tag, run red before the code.

Check: `spec-dna check` gives no SD608 and no SD916. They check the tag, not the order.

If the check fails: Add the tag, or write a test that fails for the requirement.";

const REVIEW_PARTS: &str = r"When to use: Before a merge, or when a person asks for a review or answers one.

Commands:

1. Find the base with `git merge-base HEAD origin/HEAD`. Read the diff with `git diff <base>`.
2. Run `spec-dna status`. Read `spec.md` and `plan.md` of the change with `spec-dna show <id>`.
3. Make the 3 passes below. Check each review comment against the code before you act on it.

Artifact: The findings, each with its severity, file and line.

Check: `spec-dna check` runs in CI. Do not report its findings.

If the check fails: If you find nothing, say so, and list what you read.";

const VERIFY_PARTS: &str = r"When to use: Before you say that the work is done.

Commands:

1. Run each command of `Proof` in `plan.md`. Read its whole output.
2. Run `spec-dna check`.

Artifact: The output of the commands.

Check: Each command passes, and `spec-dna check` gives no error. `finish` stops on SD915 and SD916.

If the check fails: Find the cause. Fix it, and run the commands again. Do not say that the work is done.";

const FINISH_PARTS: &str = r"When to use: The change is ready to apply.

Commands:

1. Run `spec-dna finish <slug> --dry-run`.
2. Run `spec-dna finish <slug>`.
3. Commit the result.

Artifact: The result of `spec-dna finish`: the delta applied, and the folder of the change deleted.

Check: `spec-dna finish` ends with exit code 0, and `spec-dna check` gives no error.

If the check fails: `finish` names the step that stopped it. Fix that step, and run `spec-dna finish <slug>` again.";

const DEBUG_PARTS: &str = r"When to use: A test or a command fails, and you do not know the cause.

Commands:

1. Make the failure repeat with one command.
2. Find the cause. Do not change the code yet.
3. Write a regression test for the cause, then make one fix. After 3 failed fixes, stop and ask the person.

Artifact: A reproduction, a regression test and one fix.

Check: The regression test and the earlier tests pass.

If the check fails: Go back to the cause. Do not try a new fix without a new cause.";

const POST_MORTEM_PARTS: &str = r"When to use: An `incident` has an open follow-up.

Commands:

1. Run `spec-dna status`. Find the incident.
2. Run `spec-dna new intent <slug>`. Write `from: <incident id>` in its front matter.
3. Write an eval file for the incident in the evals folder.

Artifact: A new `intent.md` with `from:`, and an eval file.

Check: `spec-dna check` gives no SD918.

If the check fails: Add or fix the eval file. Run `spec-dna check` again.";

/// The `description` of a skill. It is in each request, so it is one short
/// sentence. It starts with the scope of D5. The limits of D6 are 160
/// characters for one description, 1,900 bytes for the listing of the 11
/// skills and 1,250 bytes for the listing of the 7 skills that are on.
pub fn description(skill: Skill) -> &'static str {
    match skill {
        Skill::Brainstorm => {
            "In a spec-dna project, write intent.md with its class when a request starts a change."
        }
        Skill::Specify => {
            "In a spec-dna project, write spec.md with the delta when the intent is accepted and its class is higher-risk."
        }
        Skill::Clarify => {
            "In a spec-dna project, close each open concern and question of spec.md before it is approved."
        }
        Skill::WritePlan => {
            "In a spec-dna project, write plan.md with its files and small tasks before any code."
        }
        Skill::Execute => {
            "In a spec-dna project, run the tasks of an approved plan, one fresh subagent for each task."
        }
        Skill::Tdd => {
            "In a spec-dna project, write a test that fails before the code when a requirement has no evidence."
        }
        Skill::Review => {
            "In a spec-dna project, review the diff in 3 passes before a merge, and check each review comment."
        }
        Skill::Verify => {
            "In a spec-dna project, run the proof and read its output before you say that the work is done."
        }
        Skill::Finish => {
            "In a spec-dna project, run spec-dna finish when a change is ready, to apply its delta."
        }
        Skill::Debug => {
            "In a spec-dna project, find the cause of a failing test or command before you fix it."
        }
        Skill::PostMortem => {
            "In a spec-dna project, turn an incident with an open follow-up into a new intent and an eval."
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
#[allow(dead_code)] // The command `skill` (task t17) reads it.
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
        let on = listing(&Skill::default_on_list());
        assert!(on <= 1250, "the 7 skills on: {on} bytes");
    }

    #[test]
    fn the_skills_that_are_on_by_default_are_the_7_of_d2() {
        let on: Vec<&str> = Skill::default_on_list().iter().map(|s| s.key()).collect();
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

    #[test]
    fn a_skill_never_sets_a_status_that_a_person_gives() {
        for s in Skill::ALL {
            let b = body(s);
            assert!(!b.contains("status: accepted") && !b.contains("status: approved"));
            assert!(b.contains("Never set `accepted` or `approved`."), "{s:?}");
        }
    }
}
