---
status: accepted
---

# 0017. Profiles: lite, standard and strict

## Context and Problem Statement

In 0.6, the key `profile` sets only the severity of each diagnostic code ([docs/profiles.md](../profiles.md)). `init` asks for the workflow in other questions. They cover the change policy, the gate, the harness level and its parts, and the evals. 0.7.0 adds the approval in the branch ([ADR 0012](0012-approval-in-the-branch.md)), a pack of 11 skills ([ADR 0010](0010-workflow-pack.md)) and evals for each skill ([ADR 0016](0016-skill-evals.md)). A new project must then choose many values that depend on each other.

The owner asks for 3 paths: a light path like OpenSpec, the path of the playbook, and a strict path for a mature corpus. How does spec-dna give these paths, and keep each value visible in `spec-dna.toml`?

Constraints:

- The binary reads keys. The same `spec-dna.toml` and the same commits give the same result on each machine ([ADR 0006](0006-change-lifecycle.md)).
- `profile` keeps its meaning of 0.6, so a current configuration keeps each severity.
- An upgraded project gets no new gate and no new workflow ([ADR 0009](0009-upgrade-from-0-5.md), [D41](../design/0.7.0.md#d41-the-migration-from-06)).
- spec-dna claims no part of another tool that the design does not give.

Evidence:

- [Section 4](../design/0.7.0.md#4-the-count-of-pull-requests) of the design: the routine change of 0.6.1 took 4 pull requests (#34, #35, #37, #40). The rules of D11 to D17 give 2 for `lite` and `standard`, and 3 for `strict`. These numbers come from the rules, not from a measurement of the product.
- Corpus A (mature product, about 2.3k documents): the gate against the base tree held the accepted decisions. Edits of their locked `Decision` text stopped when the gate ran. `strict` keeps this gate on the base tree.
- OpenSpec 2.0.0 has profiles. Its core profile has a short loop from `explore` and `propose` to `apply`, `update`, `sync` and `archive`. An expanded set adds `new`, `continue`, `ff`, `verify`, `bulk-archive` and `onboard`. The machine keeps the profile, and `openspec update` applies it to each project (openspec.dev, page "Profiles", 2026-11-16).
- The [AI-native SDLC playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook) has 6 stages: Plan, Design, Build, Test, Deploy and Maintain. ADR 0006 maps a change to them.
- Superpowers has no profile. A hook turns its skills on in each session (design, section 1).

## Considered Options

1. Four or more profiles.
2. A new key `preset`, separate from `profile` (open question [Q7](../design/0.7.0.md#14-owner-questions)).
3. Profiles with runtime effects: the binary reads the name of the profile and changes the rules of the workflow.
4. `lite` with `policy = "none"`.
5. One key `profile`: it keeps the severity table, and `init` expands it into the keys of the workflow.

## Decision Outcome

Option 5. A profile is a preset. The answer to the profile question of `init` sets the defaults of the later questions, and `init` writes each key. The binary reads only the keys.

At run time, the profile sets the severity of each code, as in 0.6. It has one more effect: `profile = "strict"` with `approval = "branch"` is an error, exit code 3 ([D14](../design/0.7.0.md#d14-strict-keeps-the-base), ADR 0012). The design gives the detail in [D37 to D39](../design/0.7.0.md#d37-a-profile-is-a-preset-that-init-expands).

### What `init` writes

| | `lite` (like OpenSpec) | `standard` (the playbook) | `strict` |
| --- | --- | --- | --- |
| `[change] policy` | `two-class` | `three-class` | `three-class` |
| `[change] gate` | `warn` | `block` | `block` |
| `[change] approval` | `branch` | `branch` | `base` |
| Skills on at `init` | `brainstorm`, `write-plan`, `finish` | The 7 skills of D2 | The 7 skills of D2, and each other skill with a verdict `on` |
| `REVIEW.md` and `protect-tests` | Off | On | On |
| Evals | None | `skeleton` | `skeleton` and `pack` |
| Checks that this profile turns on | The codes of 0.6 `lite` | SD923 and SD924 as warnings | SD923 and SD924 as errors, and SD507 as a warning |
| Pull requests of a routine change, with the gate on | 2 | 2 | 3 |

With `approval = "branch"`, `init` also writes `approvers` with the git `user.email` of the person ([D15](../design/0.7.0.md#d15-keys)). The person can change each answer before `init` writes the file.

### The 3 paths

- `lite` is like OpenSpec: a light path with 2 classes, one intent for each change, a gate that only warns, and 3 skills. The likeness is the path, not the commands. `execute`, the part of spec-dna that maps to `apply` of OpenSpec, is off in `lite`.
- `standard` follows the stages of the playbook, as ADR 0006 maps a change to them. It turns on the 7 skills that D2 sets on, 3 classes, the gate `block`, `REVIEW.md` and `protect-tests`. `tdd`, the skill of the stage `test`, stays off until a verdict (D33).
- `strict` is for a mature corpus, as in 0.6: drift, requirements without evidence and each budget code fail `check`. It reads the approvals of the base tree only, so a routine change needs 3 pull requests. It turns on the 7 skills of D2 and each other skill with a verdict `on`. SD507 warns for an enabled skill that is off in D2 and has no verdict `on`.

### What each tool maps to

| Need | Spec Kit | OpenSpec | Superpowers | spec-dna 0.7.0 |
| --- | --- | --- | --- | --- |
| Rules of the project | constitution | `project.md`, `config.yaml` | None | The layer `rules` |
| The current truth | `specs/` | `openspec/specs/` | None | The layer `requirements`, with ids and hashes |
| A change | `specs/NNN-name/` | `changes/<name>/` | A design page and a plan page | `intent/<slug>/` |
| The delta | None | Delta specs | None | The section `Delta`, with hashes |
| Clarify | `clarify` | None | `brainstorming` | `clarify`, SD919, SD922 |
| Tasks | `tasks.md` | `tasks.md` | The plan | The section `Order`, SD923, SD924 |
| Run | `implement` | apply | `subagent-driven-development` | `execute` |
| End | None | archive | `finishing-a-development-branch` | `finish` |
| Corpus control | None | `validate` | None | `check`, the gate, `context` |

[D38](../design/0.7.0.md#d38-what-each-part-of-the-3-tools-maps-to) gives this table. The column of spec-dna names only parts of the design. A match in a row means the same need, not the same behavior.

### Harness level

- `lean` and `full` of 0.6 stay, and the default level at `init` stays `lean`.
- `full` proposes the 4 skills that are off by default (`execute`, `tdd`, `verify` and `debug`). It also adds the roles `implementer` and `code-simplifier`.
- The level is a separate question of `init`. The profile sets the keys of the workflow, and the level sets the roles and the extra skills.

### Upgrade

An upgraded project keeps each key and each skill. It gets the new `[change]` keys only from `init --reconfigure` (D41). The profile adds only the severities of SD923, SD924 and SD507.

In `strict`, SD923 and SD924 are errors in their first release. This is an exception to the rule of [ADR 0003](0003-distribution.md) that a new rule starts as a warning in a minor release. D41 gives the reason: the gate against the base reports only the new findings.

### Rejected

- Option 1, four or more profiles: each profile adds a column to the severity table, a set of defaults and their tests. The owner asks for 3 paths, and no evidence shows a fourth need. A project with another need edits the keys.
- Option 2, a key `preset`: 2 keys then name one choice, and a reader must learn which key sets which value. `init` already writes each key, so a second name adds no capability. If a project needs the `lite` severity with the `standard` workflow, a later release can add the key. No current file then breaks.
- Option 3, runtime effects: the keys in the file no longer predict the result, and 2 files with the same keys give different results. A later release can then change the behavior of a project with no diff in the project. The rule of `strict` is not such an effect. It refuses one combination with exit code 3, and it changes no result with no message.
- Option 4, `lite` with `policy = "none"`: `none` has no change process, and SD905 and SD910 to SD921 are off. A change then has no intent and no record in `log`. OpenSpec keeps a folder for each change, so this `lite` is not like OpenSpec. `two-class` keeps one intent for each change, with a gate that only warns.

### Consequences

- Good: one answer gives a new project a complete workflow, and `spec-dna.toml` shows each value.
- Good: the binary has no hidden mode. A reader of the keys can predict the result of `check`.
- Good: in `lite` and `standard`, a routine change takes 2 pull requests, not 3. This comes from the rules of section 4, not from a measurement.
- Good: an upgrade changes no key and no skill.
- Bad: the name `profile` now has 2 jobs: the severity at run time and the defaults of `init`. A reader can confuse them.
- Bad: a project that wants the `lite` severity with the `standard` workflow edits the keys by hand. Then the name of the profile describes only the severity, and no check reports the difference.
- Bad: in `lite`, the gate only warns, so `check` does not stop work that starts before its approval.
- Bad: `strict` needs 3 pull requests for a routine change, because it reads the approvals of the base tree only.
- Neutral: the harness level stays a separate question, so a profile does not set the roles.

## Amends

ADR 0003, ADR 0006 and ADR 0008 are accepted, and their text is locked. Read them with these changes:

- ADR 0003, the rule that a new rule starts as a warning in a minor release: SD923 and SD924 are errors in `strict` in 0.7.0. The reason is in the section on the exception.
- ADR 0006, "Change classes and policy": `init` writes `policy = "two-class"` for `lite`, and `three-class` for `standard` and `strict`.
- ADR 0008, "Decision Outcome": the profile sets the defaults of `init` for the skills, `REVIEW.md`, `protect-tests` and the evals. For `lite`, `init` proposes 3 skills, and no `REVIEW.md`, no `protect-tests` and no evals.

The build adds the section "What `init` writes for each profile" to [docs/profiles.md](../profiles.md), with the table of D37. The severity table of that page keeps its meaning, and it gets SD923, SD924 and SD507.

## Amendments
