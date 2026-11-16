<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
  <img alt="The spec-dna logo: a blue strand and a green strand with gray rungs" src="docs/assets/logo.svg" width="64" height="64">
</picture>

# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AlexGavrilov939/spec-dna)](https://github.com/AlexGavrilov939/spec-dna/releases)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Catch the changes that break your specifications, and give each AI agent only the parts that it needs.**

spec-dna is a toolchain for the specifications that your team keeps as Markdown files in git: requirements, decisions, rules and interfaces. It runs in CI and while a coding agent works. Its checks use no model and make no network call. It gives you 3 things:

- A gate on each pull request. It reports only the problems that the change adds. An edit of an accepted decision, or a broken link or id, stops the change. A document that cites a removed file or function, and a requirement with no linked test, give a warning.
- The context of one file. Before an edit, the agent runs `spec-dna context <file>`. It gets the rules and the sections that apply to that file, under a budget.
- An optional process for each change. One folder holds the intent, the plan and the requirement edits that the change proposes. `finish` writes the edits into the requirements and deletes the folder. A later agent then reads only the current requirements.

It is one binary for macOS, Linux and Windows. It works with any agent that can run a shell command.

```console
$ spec-dna check
docs/decisions/storage.md:1: error SD405 decision-locked: the text outside the amendment and live sections changed after the decision was accepted
docs/requirements/orders.md:8: warning SD606 cited-path-removed: `src/orders.py` was removed in this change (renamed to `src/cancel.py`), but this document still cites it
docs/requirements/orders.md:8: warning SD607 cited-symbol-removed: `cancel_order` was removed from `src/cancel.py` in this change, but this document still cites it
SD405 fix: Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, default `Status`) can change in place. As an alternative, write a decision that supersedes this one.
SD606 fix: Update the document: cite the new path, or change the text about the removed file.
SD607 fix: Update the document: cite the new identifier, or change the text about the removed code.
spec-dna check: 2 documents; 1 new error and 2 new warnings against b32abef70b97 (1 error and 2 warnings in total)
```

In this run, an agent renamed a function that a requirement cites. It also rewrote the outcome of an accepted decision. `check` stops the change with exit code 1 and gives the fix of each code. The last line counts the problems that are new against the base, here the last commit. [Example 1](examples/first-project/README.md) shows the full run.

## Try it in 1 minute

Install the binary. Use these commands on macOS with Apple silicon:

```sh
v=0.6.1 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS   # Linux: sha256sum -c --ignore-missing SHA256SUMS
mkdir -p ~/.local/bin && install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
```

For another platform, set `t` to `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` or `x86_64-unknown-linux-musl`. Make sure that `~/.local/bin` is on your `PATH`. With a Rust toolchain, `cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.6.1` builds the same version. [The install section](docs/quickstart.md#install) gives the commands for Windows and the check of the attestations.

Then run the first example. It needs bash, perl and git 2.32 or later, so on Windows use WSL or Git Bash. It makes a scratch repository in a temporary directory and does not change your files:

```sh
git clone --depth 1 https://github.com/AlexGavrilov939/spec-dna
spec-dna/examples/first-project/run.sh
```

## Why spec-dna

When agents write a large part of the code, the specifications are the main input of each task. Then these problems occur:

| Problem | What spec-dna does | Command |
| --- | --- | --- |
| An agent edits an accepted decision so that it agrees with its new code | Locks the text of an accepted decision. A new fact goes into an amendment or a new decision | `check` (SD405) |
| A change renames a file or a function, and a document still cites the old name | Compares each change with its base. Reports a link or an id that the change breaks, and a cited file or name that the change removes from the code | `check` (SD102, SD202, SD606, SD607) |
| An agent reads whole long files to find one rule | Gives the sections that apply to a file, under a budget of characters | `context` |
| Proposals and plans stay in the tree after the work, and agents read them as the current truth | Keeps each change in one folder, applies its delta to the requirements, and deletes the folder | `finish` |

`check` reports only the problems that a change adds. Thus a mature repository can adopt spec-dna before it fixes its old problems ([example 2](examples/existing-repo/README.md)).

## Use it in your repository

Run these commands in the root of your repository:

```sh
spec-dna init         # detects the repository, asks its questions, shows a preview, then writes
spec-dna check --all  # shows each problem once; the gate reports only new ones
```

`init` asks at most 14 questions and shows a preview of each file. In a terminal, it writes nothing before you confirm. It never edits a document. By default, `init` writes more than the checks need:

- `spec-dna.toml`, an instruction line for agents in `AGENTS.md` and `CLAUDE.md`, and a skill for Claude Code
- the change process, with the gate set to `block`
- the agent harness for Claude Code: agent files, skills for the stages of a change, `REVIEW.md`, and the models, hooks and permissions in `.claude/settings.json`
- the starter files of the evals

For the checks and the context only, answer `none` to the questions of the change policy, the harness and the evals. `--non-interactive` uses the defaults, but `--answers <file>` can give these 3 answers:

```toml
change-policy = "none"
harness = "none"
evals = "none"
```

Commit the files of `init`, then add the gate to CI:

```sh
spec-dna check --base origin/main
```

`init` can write this CI job for you. [The quickstart](docs/quickstart.md) gives each step. For a repository that already has documents, read [the adoption guide](docs/guides/existing-repo.md).

## How it works

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/how-it-works-dark.svg">
  <img alt="An agent writes one change: intent.md, spec.md for a higher-risk change only, plan.md, then code and tests. finish applies the delta to the requirements in docs/ and deletes the folder. check gates the pull request of each step. context gives the agent only the sections that its task needs." src="docs/assets/how-it-works.svg">
</picture>

- Your documents stay Markdown files in git. spec-dna reads their front matter, headings, links, code spans and requirement list items. It needs no database and no service.
- The place of a file gives its type: a file under `decisions/` is a decision, and a file under `requirements/` holds requirements. A folder of a mature repository keeps its name. A root in `spec-dna.toml` maps it to a type.
- A requirement is a list item with a bold key, such as `- **cancel-window**: ... SHALL ...`. Its id is `<document id>#<key>`. A test links to it with a `covers:` comment in any language.
- `check` compares the working tree or a pull request with its base, and it reports only the new problems.
- `context <file>` gives the rules and the sections that apply to the file, with the stable layers first, under a budget.
- The change process is optional. A change has a folder `docs/intent/<slug>/` with its intent, an optional spec and a plan. `finish` applies its delta to the requirements and deletes the folder. git keeps the history.

[Concepts](docs/concepts.md) explains the model.

## Works with any agent

The agent follows 2 steps: it runs `spec-dna context <file>` before an edit, and `spec-dna check` before the work is complete. `init` writes these steps into `AGENTS.md`, which Codex, Cursor and other agents read, and into `CLAUDE.md` for Claude Code. The line and the skill description add less than 300 bytes to each request of the agent. The optional harness for Claude Code adds about 2.2 KB.

For Claude Code, an optional harness adds agent roles with their models, skills for the stages of a change, hooks and evals. `spec-dna eval run` starts the agent for an eval, and that agent uses a model and the network. [Agents](docs/agents.md) lists what each agent gets.

## Examples

Each example is a script with a page that shows its real output. A test runs each script and compares the output with the page.

| Example | It shows |
| --- | --- |
| [A first project](examples/first-project/README.md) | A requirement that cites the code, `context`, and a `check` that stops a bad change |
| [An existing repository](examples/existing-repo/README.md) | Folders that keep their names, a baseline of old problems, and a gate for new problems |
| [One change, from the intent to `finish`](examples/one-change/README.md) | The intent, the spec with its delta, the plan, and the code with `finish`, in 4 pull requests |

## Daily use

| When | Command | Result |
| --- | --- | --- |
| Before an edit | `spec-dna context src/orders.py` | The sections that apply to the file |
| Before a commit | `spec-dna check` | Exit code 1 on a new error |
| To start a change | `spec-dna new intent <slug>` | The folder of the change |
| To see the next step | `spec-dna next <slug>` | The checklist to the next gate |
| To end a change | `spec-dna finish <slug>` | The delta in the requirements, and no folder |
| To read a document | `spec-dna show requirements/orders` | The document, with the tests of each requirement |
| To move a document | `spec-dna mv <from> <to>` | The old id stays an alias |

[Commands](docs/commands.md) gives each command and its options. [Troubleshooting](docs/troubleshooting.md) gives the fix of the frequent stops.

## Where it fits

Spec frameworks, such as [Spec Kit](https://github.com/github/spec-kit) and [OpenSpec](https://github.com/Fission-AI/OpenSpec), give an agent prompts, templates and a process to write specifications and code. spec-dna generates no text with a model. On each change, it checks the links, the ids, the code citations, the test links and the accepted decisions of the specifications. It does not check what the code does. Use it alone or with a framework:

| Framework | Root in `spec-dna.toml` | What works |
| --- | --- | --- |
| Spec Kit | `specs`, type `requirements`, `legacy-keys = ["FR-{n}"]` | Each `FR-001` is a requirement that a test can cover. `check` reports a renamed file that `tasks.md` cites. `context` gives the tasks that cite a file |
| OpenSpec | `openspec/specs`, type `requirements` | `check` reports a broken link, and a renamed file that a spec cites. `context` gives the sections of the specs that cite a file. A `covers:` tag cannot name an OpenSpec requirement, because OpenSpec writes a requirement as a heading |

The results come from spec-dna 0.6.1, which ran on 2026-11-16 on a small project from the Spec Kit 1.4 templates and on the OpenSpec repository at 2.0.0. Map only the root of the table. `openspec/changes` is not a good root, because the archive step of OpenSpec moves each file and `check` reports each old id (SD202). [Types](docs/types.md#standard-documents) maps other standard documents, such as an ADR, an RFC or a PRD.

## Status

spec-dna is at version 0.x. A minor version can rename configuration keys and add diagnostic codes. A code number never changes its meaning. [CHANGELOG.md](CHANGELOG.md) lists each change. This repository keeps its own decisions and changes with spec-dna: see [spec-dna.toml](spec-dna.toml) and [the decisions](docs/adr/).

In agent experiments, 2 graders checked each run. With spec-dna, 15 of 15 runs were correct, against 6 of 15 runs without it. Without spec-dna, the agents edited the locked text of accepted decisions 7 times. With spec-dna, the agents also read more tokens: the gain is in correct results, not in tokens. The experiments used snapshots of repositories: no team used spec-dna in daily work over weeks. [Evidence](docs/evidence.md) gives the method, the other results and the limits.

## Documentation

[docs/README.md](docs/README.md) is the map of all pages.

- Start: [Quickstart](docs/quickstart.md), [Examples](examples/README.md), [Troubleshooting](docs/troubleshooting.md)
- Concepts: [Concepts](docs/concepts.md), [Glossary](docs/glossary.md)
- Guides: [One change](docs/guides/change.md), [Existing repository](docs/guides/existing-repo.md), [Upgrade from 0.5](docs/guides/upgrade-0.5.md)
- Reference: [Commands](docs/commands.md), [Configuration](docs/config.md), [Types](docs/types.md), [Diagnostics](docs/diagnostics.md), [Profiles](docs/profiles.md), [init](docs/init.md), [Agents](docs/agents.md), [Harness](docs/harness.md), [Evals](docs/evals.md), [Transcripts](docs/transcript.md)
- Project: [Evidence](docs/evidence.md), [Decisions](docs/adr/), [Changelog](CHANGELOG.md)

## Contributing

The code of spec-dna is in Rust. It has 4 direct dependencies and no network code. It runs `git` to read the repository. [CONTRIBUTING.md](CONTRIBUTING.md) gives the setup, the rules for commits and pull requests, and the release steps. To report a bug or ask for a feature, open an [issue](https://github.com/AlexGavrilov939/spec-dna/issues/new/choose). To report a vulnerability, follow [SECURITY.md](SECURITY.md).

## License

[Apache-2.0](LICENSE)
