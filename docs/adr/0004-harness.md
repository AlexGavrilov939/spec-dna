---
status: accepted
---

# 0004. Agent harness

## Context and Problem Statement

A coding agent can use more than one model. A strong model can build, a stronger model can advise, and a small model can search. Each project that wants this setup must write the same agent files, settings and rules by hand. The files change over time, and nobody checks them.

spec-dna already writes agent adapters and checks them ([ADR 0003](0003-distribution.md), amendment of 2026-11-08). Can `init` also install a multi-model agent harness, so that every project gets the same tested setup, and `check` reports a generated file that differs from the configuration?

Constraints:

- The harness must work in any repository, greenfield or brownfield, with any stack.
- It must use generic roles. It must not copy the agents of one project.
- The owner of the project chooses the model of each role.
- Hooks must not need a second runtime.
- spec-dna must not store prompt, command or tool text.
- Hand edits outside the generated files must stay.

Evidence:

- The Claude Code documentation (code.claude.com, Claude Code 2.1.325): subagent files, settings, hooks, the advisor tool.
- Probes of Claude Code 2.1.325 on a scratch repository (2026-11-14):
  - A deny rule removes the tool from the tool list of each request.
  - A project agent file named `Explore` replaces the built-in Explore subagent. The built-in Explore runs on the model of the main session.
  - The first request of the replacement (Haiku, 3 tools, no `CLAUDE.md`) had 3,683 tokens of context. The built-in Explore on Sonnet had 18,575.
  - A subagent writes its own transcript at `<session>/subagents/agent-<id>.jsonl` beside the main transcript.
- Measurements on an anonymized mature corpus. The project owner keeps the data. The numbers stay private:
  - Agent runs with many subagents caused most of the subagent cost.
  - Requests with a large context caused most of the cost.
  - Prose rules about context size and agent counts did not change agent behavior. Hooks that run when the problem occurs did change it.
- A general risk: an event log that stores prompt or command text can store secrets.

## Considered Options

1. No harness. Each project writes its own agent setup.
2. Copy the agent set of a mature project: many domain agents, an ownership matrix, a process skill and script hooks.
3. A Claude Code plugin with the agents and the hooks.
4. Generic roles in `spec-dna.toml`, generated agent files with a version and a hash, a merge into `.claude/settings.json`, and hooks as `spec-dna hook` subcommands.

| Option | Setup per project | Drift check | Cost per request | Fits any repository | Runtime |
| --- | --- | --- | --- | --- | --- |
| 1 | High | None | Varies | Yes | Varies |
| 2 | Low | None | High: many agent descriptions and a large process text | No: domain text | A script runtime |
| 3 | Medium: a marketplace | None: the files are not in the repository | Low | Yes | Plugin agents ignore `hooks` and `mcpServers`; `model` support is not documented |
| 4 | Low: one `init` | `check` | Low | Yes | The spec-dna binary |

## Decision Outcome

Option 4.

### Roles

spec-dna embeds a table of generic roles. A role has a purpose, a tier (the reason for its default model), an effort, a tool set and the levels that include it.

| Role | Purpose | Default model, effort | Claude Code file | Levels |
| --- | --- | --- | --- | --- |
| orchestrator | The main session. It builds, delegates and runs checks | `opus`, `xhigh`; fallback `opus`, `sonnet` | `.claude/settings.json` | lean, full |
| advisor | Advice at a decision point. It never writes code | `fable`; fallback agent `opus`, `xhigh` | `advisorModel`, `fallback-advisor.md` | lean, full |
| reviewer | Checks finished work and looks for defects. It never edits | `opus`, `high` | `checker.md` | lean, full |
| researcher | Read-only search of the repository and the web | `haiku`, `high` | `Explore.md` | lean, full |
| implementer | Code and tests for one delegated task | `sonnet`, `high` | `implementer.md` | full |
| spec-writer | Specs with spec-dna | `sonnet`, `high` | `spec-writer.md` | full |
| helper | A subagent without a model of its own | `haiku` | `CLAUDE_CODE_SUBAGENT_MODEL` | lean, full |

The defaults are the setup that the owner of spec-dna uses (owner ruling, 2026-11-14). The model of a role is a default: an Agent tool call with a `model` parameter overrides it.

### Configuration

`[harness]` in `spec-dna.toml` has the level (`none`, `lean`, `full`), the denied tools, the hook values and `[harness.roles]`. Each role has a per-adapter model key (`claude-code = "opus"`) and an effort. A later adapter adds its own key, for example `codex`. `docs/harness.md` gives the keys.

### Generated files

- spec-dna generates each agent file as a whole. Its last line is a marker with the spec-dna version and a hash of the other text: `<!-- spec-dna harness v<version> h:<hash> -->`.
- `.claude/agents/model-team.md` describes the team for people. It has the same marker.
- `CLAUDE.md` gets a second generated line: the model-team rule, with the same marker form. This rule amends the "one line" rule of the ADR 0003 amendment of 2026-11-08 for a project with a harness.
- `.claude/settings.json` is a merge. spec-dna sets its keys and keeps all other keys. It never deletes a key or a deny entry of the owner. It finds its hook handlers by the command prefix `spec-dna hook <name>`, and it changes or removes only these handlers. `check` compares each expected entry with the file.

### Hooks

- `spec-dna hook context-meter` (PostToolUse): tells an agent its context size once at each level.
- `spec-dna hook fanout-cap` (PreToolUse on `Workflow`): asks the person before a workflow whose agent count can exceed the cap.

A hook reads one JSON object from stdin. It reads no configuration: the generated command contains the values. It never fails: any error gives no output and exit code 0. The generated command ends with `2>/dev/null || true`, so a teammate without spec-dna 0.5.0 or later gets no error.

No hook stores prompt, command or tool text. The context meter stores only the levels that it reported, in the temporary directory. `spec-dna transcript` reads transcripts on demand and prints counts.

### `init` and `check`

- `init` asks for the level and the models. Default level: `lean` (owner ruling: every project). A configuration without `[harness]` (an upgrade from 0.4) stays `none` until `init --reconfigure`.
- A role whose agent name exists in a file that spec-dna did not generate defaults to `none`. spec-dna never replaces such a file.
- `init` with a configuration that exists (the upgrade path) regenerates only unmodified generated files and the settings entries that differ. `init` removes files only with `--reconfigure`, after the preview.
- `check` reports these findings:
  - A missing generated file or line (SD504).
  - A hand edit (SD501).
  - An old or different generated text or settings entry (SD502).
  - A settings file that is not valid JSON (SD505).
- These rules are errors from the start, because they apply only to `[harness]`, which no older configuration has.

### Rejected

- Option 2: it has domain text, many agents with no measured value and a large process text in every request.
- Option 3: the files are outside the repository, so a pull request does not show them and `check` cannot read them.
- Script hooks (Python, shell): they need a second runtime and start a process in each tool call.
- An observer that records events: it can store secrets. `spec-dna transcript` gives the same numbers on demand.
- A gate that stops work until the advisor reads the changes: no data on its outcome exists yet. If spec-dna adds such a gate later, the gate is opt-in through an environment variable at launch. It triggers on kinds of action, not on counts.
- Generated deny rules for large interactive tools: they remove features that some owners use. `docs/harness.md` lists them, and the owner can add them to `deny`.
- Tiers with no concrete model: the owner must be able to name the model of each role.

### Consequences

- Good: one command installs a tested setup, and `check` keeps it true.
- Good: the role table and the hooks do not depend on one agent. An adapter maps them to the files of an agent.
- Bad: the default advisor (Fable) is an experimental feature. It needs the Anthropic API or Claude Platform on AWS, Fable access and, on some plans, a consent to bill usage credits. Each consult sends the whole context at the advisor price. Without access, Claude Code shows a notice, and `fallback-advisor` gives the advice.
- Bad: the default main model (Opus at `xhigh`) costs more than Sonnet. The owner can change it at `init`.
- Bad: the hooks need spec-dna 0.5.0 or later on the PATH of each teammate. Without it, the hooks do nothing.
- Bad: the model id table (alias to id, for `modelSettings`) belongs to a release. When Anthropic changes the model of an alias, `check` of the new spec-dna reports SD502 until `init` writes the new id.

## Amendments
