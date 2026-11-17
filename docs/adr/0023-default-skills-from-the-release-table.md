---
status: proposed
---

# 0023. The default skills come from the table of the release

## Context and Problem Statement

[ADR 0016](0016-skill-evals.md) gives each skill a verdict: `on`, `off` or `inconclusive`. [D33](../design/0.7.0.md#d33-when-a-skill-is-on-by-default) keeps a skill of D2 on unless its verdict is `off`. The owner review of 0.7.0 part A says that a skill is on by default only with a passing verdict. The 2 rules differ for a skill of D2 with no verdict or with `inconclusive`. The verdicts come from the full eval run, after the build.

[ADR 0017](0017-profiles.md) lets `init` expand a profile into keys. Its table names fixed skill lists, and it writes the eval pack for `strict`. How do `init`, `upgrade`, `skill list` and SD507 get one default from the verdicts, so that the choice of the rule is one small edit after the run?

Constraints:

- The binary reads keys, and a profile has no hidden effect (ADR 0017).
- The table of the release is a constant that a test compares with the committed records ([ADR 0021](0021-eval-records-and-the-cap.md)).
- An upgrade changes no value that stays valid ([ADR 0009](0009-upgrade-from-0-5.md)).
- A change of the table or of the rule changes no test and no golden file.

Evidence:

- The pilot of part A gave `on` to 1 of 5 names (`tdd`) and `inconclusive` to 4. D33 gives `on` only for a large gain.
- `spec-dna eval pack` writes 812 files (3.2 MB) into a new project (lab, 2026-11-17).

## Considered Options

1. D33 as written.
2. The rule of the owner: a skill is on by default only with the verdict `on`.
3. One function of the rule, with a constant that selects rule 1 or rule 2. The function reads the table of the release.
4. A key of `spec-dna.toml` or a variable that selects the rule.

## Decision Outcome

Option 3.

- One function gives the default of a skill from 3 facts: the skill is one of the 7 of D2, its verdict, and the constant `DEFAULT_ON_NEEDS_VERDICT_ON` in `src/verdict.rs`. With `true`, a skill is on only with the verdict `on`. With `false`, a skill of D2 is on unless its verdict is `off`, and another skill is on only with `on` (D33). The verdict comes from the table of the release.
- The constant is `true` until the owner decides after the full run. It is the latest word of the owner. A run that does not end then leaves no unproven skill on, and a skill that is off adds no byte to a request. The change to `false` is one line.
- `init` at the level `lean`: `lite` proposes the skills of `brainstorm`, `write-plan` and `finish` that are on by default. `standard` and `strict` propose each skill that is on by default, also a skill outside D2 with the verdict `on`. The level `full` proposes the 11 skills, as before.
- `upgrade` renames a name of 0.6 to the names of 0.7.0 that are on by default. The report names each name that it leaves out, and the fix. A name that stays valid, such as `review`, stays.
- `skill list` prints this default in its column and in the field `default`.
- SD507 reports an enabled skill whose default by this function is off. The verdict of the project comes before the verdict of the release.
- `init` writes no eval pack for `strict`. The preview prints a note that names `spec-dna eval pack`.
- `--reconfigure` keeps the current values. A part that the file does not set gets the default of its profile.

### Rejected

- Option 1 or option 2 alone: the rule is fixed before the data. A change after the run then edits many places and tests.
- Option 4: one release then gives 2 defaults, and an agent can set a variable.
- The fixed lists of ADR 0017: they ignore the verdicts.
- The pack at `init` for `strict`: 812 files in the first commit of each such project. The binary holds the pack, and `eval pack` writes it before a run.
- An `upgrade` that keeps each new name of a rename: `upgrade` chooses these names, so they are a default.
- The preset of a profile in `upgrade` or `--reconfigure`: it changes the workflow of a project without a question.

### Consequences

- Good: one constant and one table give each default. A test with a fake table proves each case.
- Good: a release with few verdicts `on` adds few bytes to a request.
- Bad: with `true` and the table of the pilot, `init` proposes only `tdd` for `standard` and `strict`, and no skill for `lite`. `next` still names the skill of each step, and `spec-dna skill show` prints its text.
- Bad: with `true`, an upgraded project can lose a renamed skill. The report names it.
- Bad: the default of `skill list` can change with each release.
- Neutral: the profile still sets the other keys of the workflow.

## Amends

ADR 0016 and ADR 0017 are accepted and locked, so this ADR does not edit them. These rules apply in place of the named text:

- ADR 0016 and D33, "When a skill is on by default": the function of this ADR gives the default.
- ADR 0016 and D35, SD507: a skill of D2 is exempt only when the function gives it the default on.
- ADR 0017, "What `init` writes": the row "Skills on at `init`" follows this ADR. The row "Evals" for `strict` is `skeleton`, with a note on `eval pack`.
- ADR 0017, "Upgrade", and D41: a renamed name of 0.6 keeps only the new names that are on by default.

## Amendments
