# Agents

spec-dna works with any coding agent that can run a shell command. This page gives the steps of each agent and the instruction file of each agent. It also gives the parts that an adapter adds for one agent. Read it when you choose the agent files in `spec-dna init`.

## The steps of an agent

The agent follows these steps in each task:

1. Before it edits code or documents, it runs `spec-dna context <file>...` for the files that it will change. It reads the sections of the output first.
2. Before a commit, it runs `spec-dna check`. Exit code 1 means a new error, and the agent fixes it.

For a change of the requirements, the agent also follows the change lifecycle: `spec-dna new intent`, `spec-dna next` and `spec-dna finish`. [The change guide](guides/change.md) gives the steps.

## Instruction files

`init` writes these steps as one line into the instruction file of the agent. Question 10 of `init` chooses the files: `agents-md`, `claude-md` and `skill`. By default, `init` chooses `claude-md` and `skill`. It adds `agents-md` when `AGENTS.md` exists, or when `CLAUDE.md` does not exist. If `CLAUDE.md` imports `AGENTS.md`, `CLAUDE.md` gets no second copy of the line.

| Agent | Instruction file | Note |
| --- | --- | --- |
| Codex | `AGENTS.md` | Codex reads `AGENTS.md` from the repository root to the current folder |
| Cursor | `AGENTS.md` | Cursor also reads `.cursor/rules` |
| Gemini CLI | `GEMINI.md` | Copy the line from `AGENTS.md` into `GEMINI.md` |
| Claude Code | `CLAUDE.md` | The Claude Code adapter writes this line |
| Another agent | The file that the agent reads | Copy the line from `AGENTS.md` into that file |

The line is short, because each agent request sends it again. `init` prints the bytes that it adds to each request.

## Adapters

An adapter adds parts for one agent. Today, Claude Code has the only adapter. Each other agent uses the steps through its instruction file.

| Part | Each agent | Claude Code adapter |
| --- | --- | --- |
| The instruction line | `AGENTS.md` | `CLAUDE.md` |
| The spec-dna skill, with the full procedure | Not written | `.claude/skills/spec-dna/SKILL.md` |
| Roles with their models, stage skills, hooks and `REVIEW.md` | Not written | The harness at the level `lean` or `full` ([harness.md](harness.md)) |
| Evals of the agent setup | Not written | `spec-dna eval run` starts headless Claude Code ([evals.md](evals.md)) |
| Token and read counts of sessions | Not available | The `claude-code` adapter of `spec-dna transcript` ([transcript.md](transcript.md)) |

`check` compares each generated file of an adapter with the configuration (SD501, SD502). Thus the files of an adapter agree with `spec-dna.toml`.

## Files that other agents also read

Some agents read the files of another agent:

- Cursor also loads the skills in `.claude/skills/`, so it sees the spec-dna skill.
- Cursor can also load the hooks of `.claude/settings.json`. These hooks are for Claude Code. Do not use them in Cursor.

## Add an agent

To use spec-dna with an agent that is not in the table, do these steps:

1. Make sure that the agent can run `spec-dna` in a shell, in the root of the repository.
2. Copy the spec-dna line from `AGENTS.md` into the instruction file of the agent.
3. Give the agent a task that edits a file. Make sure that it runs `spec-dna context` before the edit and `spec-dna check` before the commit.

`spec-dna transcript` can measure the sessions of another agent only through an adapter of its format. [transcript.md](transcript.md#adapters) describes how to add one.
