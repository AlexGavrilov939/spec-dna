# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AlexGavrilov939/spec-dna)](https://github.com/AlexGavrilov939/spec-dna/releases)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Engineering specifications that stay small, linked and checked, for AI coding agents.**

spec-dna is a command-line tool for the specifications that a team keeps as Markdown files in its git repository. It checks each change of these documents, as a linter checks code. It gives a coding agent only the parts of the documents that its task needs. It works in any repository, with any language and with any agent that can run a shell command. spec-dna itself makes no network call and uses no language model. The one exception is `spec-dna eval run`, which starts an agent.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/how-it-works-dark.svg">
  <img alt="An agent writes one change: intent.md, spec.md for a higher-risk change only, plan.md, then code and tests. finish applies the delta to the requirements in docs/ and deletes the folder. check gates the pull request of each step. context gives the agent only the sections that its task needs." src="docs/assets/how-it-works.svg">
</picture>

## See it in action

One task, from the request to the merge. The agent runs spec-dna itself, because `init` wrote the steps into the instruction file of the agent. The output is real. Blank lines are removed, and `...` marks a cut.

```text
You:    When a moisture sensor times out, close the valve of its bed.

Agent:  $ spec-dna context src/valves.rs
        <!-- spec-dna context 0.6.0: src/valves.rs; 4 sections in full, 0 listed; 557 of 8000 characters; about 140 tokens (estimated) -->
        <!-- rules/safety#rules h:890c76437219354e rule/active rule -->
        ## Rules
        - **fail-closed**: A valve SHALL close when the controller loses power.
        ...
        <!-- requirements/valves/closing#requirements h:c6fb31aaba81ab80 requirements/active cites -->
        ## Requirements
        - **overflow**: When a tray overflows, the controller SHALL close the valve of that tray. The check is in `src/valves.rs`.
        ...
        The task adds a requirement, so I start a routine change.
        $ spec-dna new intent sensor-timeout
        wrote docs/intent/sensor-timeout/intent.md (intent template); replace each {{...}} placeholder
        $ spec-dna check
        docs/intent/sensor-timeout/intent.md:9: error SD306 placeholder-left: the template placeholder `{{What this change gives, and for whom.}}` is still in the document
        ...
        spec-dna check: 3 documents; 3 new errors and 0 new warnings against bfce45d2e9c2 (3 errors and 2 warnings in total)
        I replaced the placeholders. Please accept the intent.

You:    Accepted and merged.

Agent:  $ spec-dna next sensor-timeout
        change sensor-timeout (routine), stage build
        base b8a446c30e6a
        [x] the intent is accepted [skill: brainstorm]
        [ ] write `docs/intent/sensor-timeout/plan.md` with `spec-dna new plan sensor-timeout`: `Files`, the task list of `Order`, `Risks` and `Proof` [skill: write-plan]
        [ ] set `status: approved` in docs/intent/sensor-timeout/plan.md [skill: write-plan]
        $ spec-dna new plan sensor-timeout
        wrote docs/intent/sensor-timeout/plan.md (plan template); replace each {{...}} placeholder
        I wrote the files, the tasks, the risks and the proof, and I approved the plan.

You:    Merged.

Agent:  I added the requirement sensor-timeout and wrote the code.
        $ spec-dna finish sensor-timeout
        docs/requirements/valves/closing.md:9: error SD916 delta-evidence-missing: the change `sensor-timeout` adds or changes the requirement `requirements/valves/closing#sensor-timeout`, and it has no `covers:` tag and no `verified-by`
        ...
        spec-dna: finish `sensor-timeout` stopped at step 5: a requirement that the change adds or changes has no evidence (the lines are lines of the result of `finish`); nothing changed
        I added a test with `covers: requirements/valves/closing#sensor-timeout` and committed it.
        $ spec-dna finish sensor-timeout
        removed docs/intent/sensor-timeout/intent.md
        removed docs/intent/sensor-timeout/plan.md
        spec-dna finish `sensor-timeout`: applied 0 ADD, 0 CHANGE and 0 REMOVE delta lines, wrote 0 files and deleted the folder `docs/intent/sensor-timeout` (2 files). `finish` staged the paths with `git add`. Read the diff, then commit it with the code
        $ spec-dna check --base origin/main
        spec-dna check: 2 documents; 0 new errors and 0 new warnings against d7af8531cffe (0 errors and 2 warnings in total)
```

When an agent edits an accepted decision, `check` stops the change:

```console
$ spec-dna check
docs/decisions/water-source.md:1: error SD405 decision-locked: the text outside the amendment and live sections changed after the decision was accepted
SD405 fix: Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, default `Status`) can change in place. As an alternative, write a decision that supersedes this one.
spec-dna check: 2 documents; 1 new error and 0 new warnings against 3bd199ae40b9 (1 error and 1 warning in total)
```

## Why

When AI agents write a large part of the code, the specifications are the main input of each task. Then these problems occur:

| Problem | What spec-dna does | Command |
| --- | --- | --- |
| An agent edits an accepted decision so that it agrees with its new code | Locks the text of an accepted decision. You write a new fact in an amendment or in a new decision | `check` (SD405) |
| A change renames a file or a function, and a spec still names the old one | Checks the links, ids, anchors and code citations of each change against its base | `check` |
| Documents grow, and an agent reads whole files to find one rule | Gives the sections that apply to a file, under a budget. Limits the size of documents and layers | `context`, `check` |
| Proposals and plans stay in the tree after the work | Keeps each change in one folder, applies its delta to the requirements and deletes the folder | `finish` |

## How it works

- Your documents stay Markdown files. spec-dna reads their front matter, headings, links, code spans and requirement list items.
- The place of a file gives its type: a file under `rules/` is a rule, and a file under `decisions/` is a decision. The layers form a graph, from the stable layers to the work that is not complete.
- A requirement is a list item with a bold key, such as `- **shutoff**: ... SHALL ...`. Its id is `<document id>#<key>`. Tests and other documents refer to it by this id.
- A change has its folder `docs/intent/<slug>/`, with its intent, spec and plan. `finish` applies its delta to the requirements and deletes the folder. Git keeps the history.
- `check` compares a change with its base and reports only the new problems. A mature repository can adopt spec-dna before it fixes the old problems.
- `context` gives an agent the sections that apply to the files of its task, with the stable layers first.
- `init` writes the steps for the agent into its instruction file: run `context` before an edit, and `check` before the work is complete.

[docs/concepts.md](docs/concepts.md) explains the model.

## Where it fits

Spec frameworks such as [OpenSpec](https://github.com/Fission-AI/OpenSpec), [Spec Kit](https://github.com/github/spec-kit) and [BMAD](https://github.com/bmad-code-org/BMAD-METHOD) give an agent a process to write specifications. spec-dna checks the specifications and the code that follows from them, with no model. It can work next to such a framework. The change folder of a framework maps to an intent folder, and a constitution maps to `rule` documents. [docs/types.md](docs/types.md#standard-documents) maps the standard documents, such as an ADR, an RFC or a PRD.

## Works with any agent

spec-dna is a command-line tool, so each agent that can run a shell command can use it. The agent follows 2 steps: it runs `spec-dna context <file>` before an edit, and `spec-dna check` before the work is complete. `init` writes these steps into `AGENTS.md`, which Codex, Cursor and other agents read, and into `CLAUDE.md` for Claude Code.

For Claude Code, more adapters add agent roles with their models, skills for the stages of a change, hooks and evals. [docs/agents.md](docs/agents.md) lists each agent and what it gets.

## Install

Each [release](https://github.com/AlexGavrilov939/spec-dna/releases) has binaries for macOS, Linux and Windows. On macOS (Apple silicon):

```sh
v=0.6.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
```

To build from source, you need Rust 1.88 or later:

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.6.0
```

You also need git. [docs/quickstart.md](docs/quickstart.md#install) gives the targets of the other platforms, the commands for Windows, and the verification of the attestations.

## Quick start

Run these commands in the root of your repository:

```sh
spec-dna init         # detect, ask, show a preview, then write spec-dna.toml and the agent files
spec-dna check --all  # show each problem once: the gate reports only new ones
```

Commit the files that `init` wrote. Then add `spec-dna check --base origin/<main branch>` to CI, or let `init` write the job.

[docs/quickstart.md](docs/quickstart.md) gives each step. For a repository that already has documents, read [docs/guides/existing-repo.md](docs/guides/existing-repo.md).

## Daily use

An agent runs `context` and `check` itself, because its instruction file tells it to. People use the same commands:

| When | Command | Result |
| --- | --- | --- |
| Before an edit | `spec-dna context src/valves.rs` | The sections that apply to the file |
| Before a commit | `spec-dna check` | Exit code 1 on a new error |
| To start a change | `spec-dna new intent <slug>` | The folder of the change |
| To see the next step | `spec-dna next <slug>` | The checklist to the next gate |
| To end a change | `spec-dna finish <slug>` | The delta in the requirements, and no folder |
| To read a document | `spec-dna show requirements/valves/closing` | The document, with the tests of each requirement |
| To move a document | `spec-dna mv <from> <to>` | The old id stays an alias |

[docs/commands.md](docs/commands.md) gives each command and its options.

## Status

spec-dna is at version 0.x. A minor version can rename configuration keys and add diagnostic codes. A code number never changes its meaning. [CHANGELOG.md](CHANGELOG.md) lists each change.

In agent experiments, 2 blind graders checked each run. With the gate, 15 of 15 runs were correct, against 6 of 15 runs without it. Without the gate, the agents edited the locked text of accepted decisions 7 times. With spec-dna, the agents also read more tokens. [docs/evidence.md](docs/evidence.md) gives the method, the other results and the limits.

## Documentation

[docs/README.md](docs/README.md) is the map of all pages.

- Start: [Quickstart](docs/quickstart.md), [Demo](docs/demo.md)
- Concepts: [Concepts](docs/concepts.md), [Glossary](docs/glossary.md)
- Guides: [One change](docs/guides/change.md), [Existing repository](docs/guides/existing-repo.md), [Upgrade from 0.5](docs/guides/upgrade-0.5.md)
- Reference: [Commands](docs/commands.md), [Configuration](docs/config.md), [Types](docs/types.md), [Diagnostics](docs/diagnostics.md), [Profiles](docs/profiles.md), [init](docs/init.md), [Agents](docs/agents.md), [Harness](docs/harness.md), [Evals](docs/evals.md), [Transcripts](docs/transcript.md)
- Project: [Evidence](docs/evidence.md), [Decisions](docs/adr/), [Changelog](CHANGELOG.md)

## Contributing

spec-dna is written in Rust. It has 4 direct dependencies and no network code. It runs `git` to read the repository. [CONTRIBUTING.md](CONTRIBUTING.md) gives the setup, the rules for commits and pull requests, the dependencies and the release steps. To report a vulnerability, follow [SECURITY.md](SECURITY.md).

## License

[Apache-2.0](LICENSE)
