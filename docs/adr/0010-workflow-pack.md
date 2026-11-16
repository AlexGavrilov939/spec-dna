---
status: proposed
---

# 0010. The workflow pack: own skills bound to the artifacts

## Context and Problem Statement

Since 0.6.0, `next` names a skill for each step of a change, but spec-dna ships only 4 stage skills ([ADR 0008](0008-harness-and-evals.md)). 0.7.0 adds a skill for each stage of the playbook ([The workflow pack](../design/0.7.0.md#3-the-workflow-pack)). 3 tools already give such skills: Superpowers v7.0.2 (2026-11-03), Spec Kit v1.4.1 (2026-11-16) and OpenSpec v2.0.0 (2026-11-11). Each has the MIT licence. Does spec-dna ship its own skills, or adapters over the skills of an installed plugin?

Constraints:

- [ADR 0003](0003-distribution.md): each project pins the exact binary, and the binary embeds the standard. Thus the pin must also fix the text of each skill.
- The core works with any agent. A skill folder is an adapter for Claude Code, and an agent with no skill folder needs the same text.
- Rule zero: the pack names or copies no consumer project.
- The text that spec-dna writes for agents follows the rules of section "Writing" of `CONTRIBUTING.md` (STE).
- Since 0.6.0, `next` names 9 fixed skills: `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify` and `finish`. A test fixes them, and agents read them.
- The description of each skill is in each request. Thus each byte of a description has a cost.

Evidence ([Evidence and sources](../design/0.7.0.md#1-evidence-and-sources); the data stays outside this repository):

- The gate against the base gave the measured gain. In corpus A (mature product, about 2.3k documents), edits of the locked `Decision` text of accepted ADRs stopped when the gate ran. A synthetic project gave 15 of 15 correct runs with the gate and 6 of 15 without it.
- Agents search the repository and seldom follow a tree of links. A reorganization of large documents gave no gain.
- A short `CLAUDE.md` core gave a difference inside the noise. Only the decrease of the first request (about 2,500 tokens) is firm.
- Superpowers claims fewer tokens and faster work in its own release notes. The sources hold no independent benchmark, so this ADR takes no number from it.
- The sources of Superpowers have no rule of "2 to 5 minutes" for a task. Its plans use "one action with a checkable result". Thus the size of a task is a rule of spec-dna, and no skill credits Superpowers for it.
- Third-party reviews of Spec Kit call it verbose and duplicative. These are opinions, not measurements. This ADR uses them only as a reason to keep skill text short.

## Considered Options

1. Adapters over installed plugins: a spec-dna skill calls a skill of Superpowers, Spec Kit or OpenSpec.
2. Own skills, bound to the artifacts and the checks of spec-dna. The binary holds their text.
3. One large skill for all stages.
4. A separate plugin in a marketplace, outside the binary.
5. A copy of the text of one tool, with its notice.

## Decision Outcome

Option 2 ([D1](../design/0.7.0.md#d1-own-skills-bound-to-the-artifacts-of-spec-dna)). Only a skill that spec-dna owns writes the artifact that the gate reads, follows the pin and binds to a check. The design page gives the tables and the detail of D2 to D6. This ADR gives the reasons.

### The 11 skills

A stage skill binds to a step that `next` names. An event skill binds to an event: a failure or an incident. The floor is the deterministic check that holds the result of a skill, if one exists ([D2](../design/0.7.0.md#d2-the-11-skills)).

| Skill | Kind | Artifact | Floor | Default |
| --- | --- | --- | --- | --- |
| `brainstorm` | Stage `plan` | `intent.md` | SD306 | On |
| `specify` | Stage `design` | `spec.md` | SD914, SD920 | On |
| `clarify` | Stage `design` | `Concerns` and `Open questions` of `spec.md` | SD919, SD922 | On |
| `write-plan` | Stage `build` | `plan.md` with small tasks | SD913, SD923, SD924 | On |
| `execute` | Stage `build` | The code, the tests and the checked task | SD913, `protect-tests` | Off |
| `tdd` | Stage `test` | A test with a `covers:` tag, red before the code | SD608, SD916 | Off |
| `review` | Stage `deploy` | The review of the diff, and the answer to review comments | None (the text of `REVIEW.md`) | On |
| `verify` | Stage `deploy` | The output of the `Proof` commands and of `check` | `finish` stops on SD915, SD916 | Off |
| `finish` | Stage `closed` | The result of `spec-dna finish` | `finish` | On |
| `debug` | Event | A reproduction, a regression test, one fix | None | Off |
| `post-mortem` | Event | A new `intent.md` with `from:`, and an eval | SD918 | On |

- Each skill has 5 parts in this order: when to use it, the commands, the artifact, the check, and the action when the check fails. Thus each skill ends at a check, not at a claim of the agent.
- A skill never sets `accepted` or `approved`. The change guide names the approver ([ADR 0006](0006-change-lifecycle.md)).
- `execute` starts one fresh subagent for each task, with 2 review passes after it: compliance, then quality. It uses the roles of 0.6, so it adds no role.
- `write-plan` uses the task form of 0.6.0. The time of 2 to 5 minutes for a task is a goal in its text. SD924 checks a proxy: at most `max-task-files` exact paths ([D40](../design/0.7.0.md#d40-new-keys-commands-and-codes)).
- 7 skills are on by default. `brainstorm` and `review` replace `spec-dna-intent` and `spec-dna-review` of 0.6, so an upgrade keeps them on.
- `execute`, `tdd`, `verify` and `debug` stay off until an eval gives a verdict `on` ([ADR 0016](0016-skill-evals.md), [Q4](../design/0.7.0.md#14-owner-questions)). `init` proposes them only at the level `full`.

### Disciplines

[D3](../design/0.7.0.md#d3-the-disciplines) gives 5 disciplines: test first, cause first, proof before done, review is data, and isolated work. Each rule is one line of 140 bytes or fewer. The adapter skill `spec-dna` holds the 5 lines in one block of 700 bytes or fewer. Each line names the skill that holds the detail. Thus a skill that is off still has a pointer, and `spec-dna skill show` prints its text ([ADR 0011](0011-entry-points.md)).

### Delivery and names

- The folder is `.claude/skills/spec-dna-<name>/SKILL.md`, with the harness marker of 0.5. `next` prints the bare name, as in 0.6 ([D4](../design/0.7.0.md#d4-delivery-and-names)).
- The text of each skill is a constant in the binary. Thus the pin fixes the text, and an agent with no skill folder gets the same text from `spec-dna skill show`.
- `[harness] skills` lists the new names. The 0.6 names `intent`, `spec` and `plan` end with no runtime alias, and `review` stays, as [ADR 0009](0009-upgrade-from-0-5.md) requires. An old name gives exit code 3, and `upgrade` rewrites it.
- `init` removes an unedited file of an old skill folder, and keeps an edited one. `check` reports SD501 for a kept file.

### Another workflow plugin

A project can also have Superpowers. Then 2 skills of the same stage compete ([D5](../design/0.7.0.md#d5-another-workflow-plugin-in-the-same-project)).

- The description of each spec-dna skill starts with "In a spec-dna project".
- The adapter skill states that a spec-dna skill replaces a general skill of the same stage. For `write-plan`, it replaces `writing-plans`.
- `init` reads `.claude/settings.json` of the project. If `enabledPlugins` names a plugin with the word `superpowers`, `init` prints a note with these rules. It never reads a file of the home folder.
- `init` does not move a file that a plugin writes into a root, such as `docs/superpowers/specs/`. The note tells the person to exclude that folder from the roots.

### Budgets

[D6](../design/0.7.0.md#d6-budgets) keeps the limits of 0.6: 160 characters for a description and 2,500 bytes for a body. All descriptions together, as the skill listing shows them, have 1,900 bytes or fewer for the 11 skills, and 1,250 bytes or fewer for the 7 skills that are on. The discipline block has 700 bytes or fewer. The line of `AGENTS.md` has 400 bytes or fewer. A test fixes each limit.

### Credit and notice

The ideas of 4 rules come from Superpowers. They are a fresh subagent for each task with a review after it, and a test that fails before the code. They are also the cause before the fix, and the proof before the word "done".

The names `brainstorm`, `write-plan` and `tdd` come from Superpowers. The names `specify` and `clarify` come from Spec Kit. spec-dna writes its own text, and `docs/research/matrix.md` gives the credit ([Q8](../design/0.7.0.md#14-owner-questions)). If a later build copies text of a tool, it adds the MIT notice of the copyright holder to `docs/research/matrix.md` in the same pull request.

### Rejected

- Option 1, adapters over installed plugins:
  - A plugin skill writes its own files, such as `docs/superpowers/specs/<date>-<topic>-design.md`. Such a file is not an `intent.md`, so the gate and `next` do not see it.
  - A wrapper cannot pin the text. The project pins the binary, but a plugin updates on its own.
  - A check cannot bind to a skill that spec-dna does not own. A check binds to a file and a section.
  - The SessionStart hook of Superpowers injects its entry skill `using-superpowers` into each session. This entry competes with `spec-dna next`, and 2 entries give 2 answers.
  - The plugin of one agent does not serve an agent with no skill folder.
- Option 3, one large skill: its description must name each stage, so each request carries a long description. Each use loads the text of all stages, and that text stays in each later request of the session.
- Option 4, a separate plugin in a marketplace: the text does not follow the pin of ADR 0003. The project gets a second release channel and a second version to align.
- Option 5, a copy of the text of a tool: each copy carries the duty of the MIT notice, and it gives no gain. The text must name the artifacts and the checks of spec-dna in any case.
- All 11 skills on by default: no check and no eval result exists for what the 4 skills that are off add. Each description that is on costs bytes in each request ([ADR 0016](0016-skill-evals.md)).
- The 0.6 names next to the new names: ADR 0009 rejects runtime aliases, because agents copy the old name and the alias stays forever.

### Consequences

- Good: each skill writes an artifact that the gate, `next` and `check` read. Where a floor exists, a check holds the result, not the text of the skill.
- Good: the pin fixes the text of each skill, and each agent gets the same text from the binary.
- Good: tests fix the bytes that the pack adds to each request.
- Bad: 4 skills are off by default. Thus the pack does not replace Superpowers in full until the evals give a verdict (Q4).
- Bad: the descriptions in each request grow. The listing of the 4 skills of 0.6 has 588 bytes, and the listing of the 7 skills that are on can have 1,250 bytes.
- Bad: spec-dna keeps 11 texts, each with a golden file. A change of a command, a code or a section name can change several skills, and each change needs a release.
- Bad: the prefix "In a spec-dna project" and the replace rule are text, not a control. No source shows that Claude Code then selects the spec-dna skill first. The evals of D32 have no arm with Superpowers, so a project with Superpowers can still get 2 answers.
- Bad: the discipline block is in the body of the adapter skill. A skill body loads on demand, and an agent with no skill folder has no adapter skill. Thus the block is not in each request.
- Bad: no check holds the work of `debug`. The check of `tdd` reads the `covers:` tag, not a red test before the code (D2).
- Bad: a 0.6 project must run `upgrade` for the new names. A hand-edited old skill file stays and gives SD501.
- Neutral: the pack does not import or call the 3 tools. A project can keep Superpowers next to the pack, with the note of `init`.
- Neutral: the time of a task is a goal in text. Only the count of paths has a check.

## Amends

ADR 0003 is accepted, and its text is locked. Read its amendment "Adapters" with this change. The one line that `init` writes into a root instruction file names the 5 disciplines of [D3](../design/0.7.0.md#d3-the-disciplines) and the command `spec-dna skill show`. It has 400 bytes or fewer. It is still one line, and it still imports no file.

ADR 0008 is accepted, and its text is locked. Read it with this change:

- "Stage skills": the 4 skills `intent`, `spec`, `plan` and `review` become `brainstorm`, `specify` and `clarify`, `write-plan`, and `review`. The pack has 11 skills, and 7 of them are on by default. The listing of the 4 skills of 0.6 has 588 bytes in Claude Code, and a 0.6 test fixes this size. In 0.7.0, the listing of the 11 skills has 1,900 bytes or fewer. The listing of the 7 skills that are on has 1,250 bytes or fewer (D6).

## Amendments
