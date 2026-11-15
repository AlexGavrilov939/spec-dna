//! The text of the stage skills and of `REVIEW.md` (ADR 0008, step 4 S3).
//! The text is static: no value of the configuration goes into it, so its
//! hash is the same in each project.

use crate::harness::Skill;

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

const INTENT_STEPS: &str = r"You write `intent.md` of a change. Use the words of the person who made the request. Do not add scope.

1. Choose a slug of 2 to 4 words. Run `spec-dna new intent <slug>`. For a product requirement, add `--template prd`. For a design proposal, add `--template rfc`.
2. Replace each placeholder of the file with text from the request. Write what the change gives, why now, and what it does not do.
3. Set `class` in the front matter. Use `higher-risk` for a change of an interface or a rule, or of a path that the project marks as high risk. Use `routine` for each other change. If you are not sure, use `higher-risk`.
4. Ask the person about each fact that the request does not give. Do not guess.
5. Run `spec-dna check`. Fix each error of the new file.
6. Run `spec-dna next <slug>`. Tell the person the next step.

Do not set `status` to `accepted`. A person accepts the intent in a pull request.";

const SPEC_STEPS: &str = r"You write `spec.md` of a higher-risk change. A routine change has no spec.

1. Run `spec-dna next <slug>`. If the intent is not `accepted`, stop and tell the person.
2. Run `spec-dna new spec <slug>`.
3. Under `Delta`, write one list item for each requirement that the change adds, changes or removes. Use the forms below.
4. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>`. The first line shows `h:<hash>`. Copy the 16 digits after `h:` to the item.
5. Under `Interfaces`, write the item `- CHANGE <interface id>: <text>` for each interface that the change touches. Under `Decisions`, write the item `- [[decisions/<slug>]]` for each decision. If the change needs a new decision, run `spec-dna new decision <slug>`.
6. Write each open concern under `Concerns`, and each open question under `Open questions`, as an item `- [ ] <text>`. Ask the person about each of them.
7. Run `spec-dna check`. Fix each error. Run `spec-dna next <slug>`.

The forms of a `Delta` item:

```text
- ADD <id>#<key>: <new sentence>
- CHANGE <id>#<key>@<hash>: <new sentence>
- REMOVE <id>#<key>@<hash>
```

Do not set `status` to `approved`. A person approves the spec in a pull request.";

const PLAN_STEPS: &str = r"You write `plan.md` of a change. Use plan mode if the session has it. Do not change code until a person approves the plan.

1. Run `spec-dna next <slug>`. Read the intent, and the spec if the change has one.
2. Run `spec-dna context <file>...` for the files that you expect to change. Read a listed section with `spec-dna show <id>#<key>` only when you need it.
3. Run `spec-dna new plan <slug>`.
4. Under `Files`, list each path or glob that the change touches. This list is the scope of the change.
5. Under `Order`, write one task for each step, in order. Give each task an id that is not in the file yet.
6. Under `Risks`, write what can go wrong and what limits it. Under `Proof`, write the tests with their `covers:` ids, and the commands that show the result.
7. Run `spec-dna check`. Fix each error. Run `spec-dna next <slug>`.

A task has this form:

```text
- [ ] <id>: <text>. Files: `path`, `glob`. Verify: <text or `command`>.
```

Do not set `status` to `approved`. A person approves the plan in a pull request.";

const REVIEW_STEPS: &str = r"You review the diff of a change against its spec and its plan. Do not edit files.

1. Find the base with `git merge-base HEAD origin/HEAD`. If git cannot find it, ask the person for the base. Read the diff with `git diff <base>`.
2. Find the change in the diff. Its folder has `intent.md`. If the folder is in the tree, run `spec-dna status` for its stage, and read `spec.md` and `plan.md` with `spec-dna show <id>`. The ids are `intent/<slug>/spec` and `intent/<slug>/plan`. A routine change has no `spec.md`.
3. In a `finish` pull request, the folder is deleted. Read `spec.md` and `plan.md` as deleted text in the diff.
4. Run `spec-dna check`. Do not report its findings.
5. Make the 3 passes below. Give each finding its severity, its file and line, and the reason.
6. If you find nothing, say so, and list what you read.";

/// The `description` of a skill. It is in each request, so it is short.
pub fn description(skill: Skill) -> &'static str {
    match skill {
        Skill::Intent => {
            "Writes the intent of a change from a request, with its class, then runs spec-dna next. Use it when a request starts a new change."
        }
        Skill::Spec => {
            "Writes spec.md of a higher-risk change with the delta, interfaces, decisions and concerns. Use it after the intent is accepted."
        }
        Skill::Plan => {
            "Writes plan.md of a change with the files, the ordered tasks, the risks and the proof. Use it after the intent, or after the spec."
        }
        Skill::Review => {
            "Reviews the diff of a change against its spec and plan in 3 passes. Use it before a merge, or when a person asks for a review."
        }
    }
}

/// The body of a skill: the text between the front matter and the marker.
pub fn body(skill: Skill) -> String {
    match skill {
        Skill::Intent => INTENT_STEPS.to_string(),
        Skill::Spec => SPEC_STEPS.to_string(),
        Skill::Plan => PLAN_STEPS.to_string(),
        Skill::Review => format!("{REVIEW_STEPS}\n\n{REVIEW_POLICY}"),
    }
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

    #[test]
    fn each_description_is_short_and_safe_in_yaml() {
        for s in Skill::ALL {
            let d = description(s);
            assert!(!d.is_empty(), "{s:?}");
            assert!(d.len() <= 160, "{s:?}: {} bytes", d.len());
            assert!(d.is_ascii(), "{s:?}");
            // An unquoted YAML value: no `: `, no ` #`, no leading mark.
            assert!(!d.contains(": ") && !d.contains(" #"), "{s:?}: {d}");
            assert!(!d.starts_with(['`', '#', '-', '&', '*', '!', '|', '>', '"', '\'']));
            assert!(d.ends_with('.'), "{s:?}: {d}");
        }
    }

    #[test]
    fn the_listing_of_the_4_skills_has_at_most_800_bytes() {
        let n = listing(&Skill::ALL);
        assert!(n > 0 && n <= 800, "{n} bytes");
    }

    #[test]
    fn each_body_follows_the_text_rules() {
        for s in Skill::ALL {
            let b = body(s);
            assert!(!b.is_empty(), "{s:?}");
            assert!(b.len() <= 2500, "{s:?}: {} bytes", b.len());
            assert!(b.is_ascii(), "{s:?}");
            assert!(!b.contains("{{"), "{s:?}");
            assert!(!b.contains("docs/"), "{s:?}");
            assert!(!b.contains("small-fix"), "{s:?}");
            assert!(!b.contains('\r') && !b.ends_with('\n'), "{s:?}");
            assert!(b.contains("\n1. "), "{s:?}: numbered steps");
            assert!(!b.contains("<!-- spec-dna"), "{s:?}: no marker text");
        }
    }

    #[test]
    fn the_review_policy_is_one_text_in_two_places() {
        assert!(body(Skill::Review).contains(REVIEW_POLICY));
        assert!(review_md().contains(REVIEW_POLICY));
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
    fn each_skill_runs_its_spec_dna_commands() {
        let has = |s: Skill, parts: &[&str]| {
            let b = body(s);
            for p in parts {
                assert!(b.contains(p), "{s:?}: {p}");
            }
        };
        has(
            Skill::Intent,
            &[
                "`spec-dna new intent <slug>`",
                "--template prd",
                "--template rfc",
                "class",
                "`spec-dna check`",
                "`spec-dna next <slug>`",
            ],
        );
        has(
            Skill::Spec,
            &[
                "`spec-dna new spec <slug>`",
                "`spec-dna show <id>#<key>`",
                "- ADD <id>#<key>: <new sentence>",
                "- CHANGE <id>#<key>@<hash>: <new sentence>",
                "- REMOVE <id>#<key>@<hash>",
                "Copy the 16 digits after `h:`",
                "- CHANGE <interface id>: <text>",
                "- [[decisions/<slug>]]",
                "Delta",
                "Interfaces",
                "Decisions",
                "Concerns",
                "Open questions",
                "`spec-dna check`",
                "`spec-dna next <slug>`",
            ],
        );
        has(
            Skill::Plan,
            &[
                "`spec-dna new plan <slug>`",
                "plan mode",
                "Files",
                "Order",
                "Risks",
                "Proof",
                "- [ ] <id>: <text>. Files: `path`, `glob`. Verify: <text or `command`>.",
                "`spec-dna check`",
                "`spec-dna next <slug>`",
            ],
        );
        has(
            Skill::Review,
            &[
                "`spec-dna status`",
                "`spec-dna check`",
                "`spec-dna show <id>`",
                "git diff",
                "`finish` pull request",
                "deleted text",
            ],
        );
    }

    #[test]
    fn a_skill_never_sets_a_status_that_a_person_gives() {
        for s in [Skill::Intent, Skill::Spec, Skill::Plan] {
            let b = body(s);
            assert!(b.contains("Do not set `status`"), "{s:?}");
            assert!(b.contains("A person "), "{s:?}");
            assert!(!b.contains("status: accepted") && !b.contains("status: approved"));
        }
    }
}
