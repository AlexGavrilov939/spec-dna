---
status: proposed
---

# 0011. Entry points: next, status and skill

## Context and Problem Statement

The pack of [ADR 0010](0010-workflow-pack.md) has 11 skills. An agent needs one entry that names the skill of the next step. A person needs one view of each change and its approvals. An agent with no skill folder also needs the text of a skill. Which entry points does 0.7.0 give, and how does such an agent get the text?

Constraints:

- Since 0.6.0, `next` names 9 fixed skills, and a test fixes them. Agents and scripts read `next --json`.
- spec-dna makes no network call. It starts no agent, except `eval run` ([ADR 0008](0008-harness-and-evals.md), "Evals").
- The core works with any agent. Each byte in each request has a cost.
- The binary holds the text of each skill, so the pin fixes it (ADR 0010).

Evidence ([Evidence and sources](../design/0.7.0.md#1-evidence-and-sources)):

- A short `CLAUDE.md` core gave a difference inside the noise. Only the decrease of the first request is firm. Thus text in each request needs a reason.
- Agents search the repository and seldom follow a tree of links. Thus an entry gives a command that prints the text, not a link to it.

## Considered Options

1. `next` and `status` stay the entries. A read-only command `spec-dna skill` prints the text of a skill.
2. A command `spec-dna run` that starts an agent for the next step.
3. A second entry, such as `spec-dna workflow`.
4. The text of the skills in `AGENTS.md`.

## Decision Outcome

Option 1. The design gives the detail in [The entry points](../design/0.7.0.md#5-the-entry-points) and [D4](../design/0.7.0.md#d4-delivery-and-names).

- `next` stays the one entry of an agent (D7). The 9 names of 0.6.0, and the step of each name, do not change. Each line has `[skill: <name>]`, and each item of `--json` has the field `skill`. A test fixes the list.
- `status` stays the entry of a person (D8). It also names the skill `post-mortem` for an incident with an open follow-up. For each change, it shows the source of each approval: `base` or `branch <short sha>` ([ADR 0012](0012-approval-in-the-branch.md)).
- `debug` is an event skill, and it has no stage (D9). `next` does not name it. Its description starts it, and the eval of `debug` measures this start ([ADR 0016](0016-skill-evals.md)).
- `spec-dna skill list` prints the 11 names, their kind and their state (D10). `spec-dna skill show <name>` prints the body of one skill. An unknown name gives exit code 2. Both commands take `--json`.
- `skill` reads no document and never writes. Its text is a constant in the binary.
- An agent with no skill folder runs `spec-dna skill show <name>` for the step that `next` names. `AGENTS.md` carries this rule in one line (D4).
- The skill folder of a name is `spec-dna-<name>`. The documentation gives this prefix rule, so `next` prints one name only.

### Rejected

- Option 2, `spec-dna run`: spec-dna makes no network call and starts no agent, except `eval run`. A command that starts an agent binds the core to one agent, its model and its permissions.
- Option 3, a second entry: 2 entries give 2 answers. `next` already derives the step from the artifacts and git.
- Option 4, the text of the skills in `AGENTS.md`: each request carries it. The evidence of the short core shows no firm gain from more text in each request.
- A field `invoke` with the adapter name in `next --json`: the name depends on the adapter of one agent. The prefix rule gives the same result with no new field.
- `next` names `debug`: a failure has no stage. `next` derives the stage from the artifacts, and a failure is not an artifact.

### Consequences

- Good: an agent has one entry, and a person has one view. Each agent gets the text of a skill with one command.
- Good: the 9 names, the text form and the `--json` shape of `next` do not change.
- Bad: an agent with no skill folder runs one more command for each step. Each command costs a tool call.
- Bad: only its description starts `debug`, and `debug` is off by default ([ADR 0010](0010-workflow-pack.md)). A project with no folder of `debug` has no description to start it. Only the discipline "cause first" points to it.
- Bad: `next` also names `execute`, `tdd` and `verify` when they are off. The agent then gets the text from `spec-dna skill show`.
- Bad: the line of `AGENTS.md` adds bytes to each request, and D6 gives this line 400 bytes.
- Bad: `skill` is a new contract. Its names, its exit codes and its `--json` shape need golden files.
- Neutral: the design fixes no field of `skill --json`. The build proposes the fields as a contract change.
- Neutral: `post-mortem` is an event skill, as `debug` is. `status` names it, and `next` does not.

## Amends

ADR 0006 and ADR 0008 are accepted, and their text is locked. Read them with these changes:

- ADR 0006, "Stages": no change. The stage has no field, and `debug` has no stage.
- ADR 0008: the command `spec-dna skill` is new. It reads no document, writes nothing and starts no program. Thus `eval run` stays the only command that starts a program that uses the network (section "Evals").

## Amendments
