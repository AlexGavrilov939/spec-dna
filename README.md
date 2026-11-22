<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
  <img alt="The spec-dna logo: a blue strand and a green strand with gray rungs" src="docs/assets/logo.svg" width="64" height="64">
</picture>

# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AlexGavrilov939/spec-dna)](https://github.com/AlexGavrilov939/spec-dna/releases)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Catch the changes that break your specifications, and give each AI agent only the parts that it needs.**

spec-dna checks the Markdown specifications in your git repository: requirements, decisions and rules. It runs in CI and while a coding agent works. It is one binary. Its checks use no model and no network.

```console
$ spec-dna check
docs/decisions/storage.md:1: error SD405 decision-locked: the text outside the amendment and live sections changed after the decision was accepted
docs/requirements/orders.md:8: warning SD606 cited-path-removed: `src/orders.py` was removed in this change (renamed to `src/cancel.py`), but this document still cites it
docs/requirements/orders.md:8: warning SD607 cited-symbol-removed: `cancel_order` was removed from `src/cancel.py` in this change, but this document still cites it
...
spec-dna check: 2 documents; 1 new error and 2 new warnings against b32abef70b97 (1 error and 2 warnings in total)
```

Here an agent renamed a function that a requirement cites, and it rewrote an accepted decision. `check` stops the change. [See the full run](examples/first-project/README.md).

## What it catches

| When a change... | spec-dna... |
| --- | --- |
| rewrites an accepted decision | stops it (SD405) |
| breaks a link or an id | stops it (SD102, SD202) |
| renames a file or a function that a document cites | warns (SD606, SD607) |
| adds a requirement with no test | warns (SD608) |

Only the problems that a change adds count. An old repository can start today and fix its old problems later.

Before an edit, an agent runs `spec-dna context <file>`. It gets the rules and the sections for that file, not whole documents.

## Try it in 1 minute

Install the binary. On macOS with Apple silicon:

```sh
v=0.8.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS   # Linux: sha256sum -c --ignore-missing SHA256SUMS
mkdir -p ~/.local/bin && install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
```

Other targets: `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`. For Windows, see [Install](docs/quickstart.md#install). With Rust: `cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.8.0`.

Run the first example in a temporary folder. It needs bash, perl and git 2.32 or later:

```sh
git clone --depth 1 https://github.com/AlexGavrilov939/spec-dna
spec-dna/examples/first-project/run.sh
```

## Use it in your repository

```sh
spec-dna init                      # asks, shows a preview, then writes
spec-dna check --all               # lists each old problem once
spec-dna check --base origin/main  # the gate for CI; init can write the job
```

> [!NOTE]
> By default, `init` also adds a change process, a harness for Claude Code (models, hooks, settings) and eval files. The first question asks for a profile (`lite`, `standard` or `strict`). It sets the defaults of these parts and the severity of the checks ([Profiles](docs/profiles.md)). For the checks only, give it this file with `spec-dna init --answers <file>`:
>
> ```toml
> change-policy = "none"
> harness = "none"
> evals = "none"
> ```

A repository with documents? Read [the adoption guide](docs/guides/existing-repo.md).

## How it works

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/how-it-works-dark.svg">
  <img alt="An agent writes one change: intent.md, spec.md for a higher-risk change only, plan.md, then code and tests. finish applies the delta to the requirements in docs/ and deletes the folder. check tests the pull request of each step. context gives the agent only the sections that its task needs." src="docs/assets/how-it-works.svg">
</picture>

- Your documents stay Markdown files in git. The folder gives the type: `decisions/`, `requirements/`, `rules/`.
- A requirement is a list item with a bold key: `- **cancel-window**: ... SHALL ...`. Its id is `<document>#<key>`.
- A test links to a requirement with a `covers:` comment, in any language.
- A change can have its own folder: the intent, the plan and the requirement edits. `finish` applies the edits and deletes the folder. This process is optional.
- `init` adds one line to the agent files (`AGENTS.md`, `CLAUDE.md`): run `context` before an edit, and `check` before you finish. Each agent that runs shell commands can follow it.

[Concepts](docs/concepts.md) explains the model.

## Examples

Each example runs in a temporary folder. A test compares each page with the real output.

| Example | Shows |
| --- | --- |
| [A first project](examples/first-project/README.md) | A requirement that cites code, `context`, and a blocked edit of a decision |
| [An existing repository](examples/existing-repo/README.md) | Old problems that do not block, and a new problem that does |
| [One change](examples/one-change/README.md) | Intent, spec, plan and `finish`, in 4 pull requests |

## Commands

| Command | Use it to |
| --- | --- |
| `spec-dna context <file>` | get the sections for a file, before an edit |
| `spec-dna check` | find new problems, before a commit |
| `spec-dna new intent <slug>` | start a change |
| `spec-dna next <slug>` | see the next step of a change |
| `spec-dna finish <slug>` | apply a change and delete its folder |
| `spec-dna show <id>` | read a document and its tests |
| `spec-dna mv <from> <to>` | move a document and keep its old id |

[Commands](docs/commands.md) gives each option. [Troubleshooting](docs/troubleshooting.md) gives the fixes for common errors.

## Where it fits

Spec Kit and OpenSpec help an agent write specifications. spec-dna checks the result on each change. It checks links, ids, code citations, test links and accepted decisions. It does not check what the code does.

To keep the files of the tool, map its folder as a root:

| Framework | Root in `spec-dna.toml` | What works |
| --- | --- | --- |
| [Spec Kit](https://github.com/github/spec-kit) | `specs` as `requirements`, with `legacy-keys = ["FR-{n}"]` | Test links to `FR-001`, renamed files in `tasks.md`, `context` |
| [OpenSpec](https://github.com/Fission-AI/OpenSpec) | `openspec/specs` as `requirements` | Links, renamed files in a spec, `context`. A `covers:` tag cannot name an OpenSpec requirement, because it is a heading |

We tested the table with both tools on 2026-11-16. Do not map `openspec/changes`: its archive step moves files, and `check` reports each old id (SD202).

To move the project into the layout of spec-dna, run `spec-dna import speckit` or `spec-dna import openspec` after `init`. The import proves that each requirement sentence and each id stay, and it deletes no file. It reads the layouts of Spec Kit 1.4.1 and OpenSpec 2.0.0. On 2026-11-17, the notes of the newer releases (Spec Kit 1.4.2 and OpenSpec 2.1.0) showed no change of these layouts, but no test ran the import on them. [Commands](docs/commands.md#import) gives the steps.

## Status

- spec-dna is at 0.x. A minor version can rename configuration keys. A diagnostic code keeps its meaning. See the [changelog](CHANGELOG.md).
- In agent experiments, 15 of 15 runs were correct with spec-dna, and 6 of 15 without it. The agents read more tokens with it. [Evidence](docs/evidence.md) gives the method and the limits.
- The experiments used snapshots. So far, no team uses spec-dna in daily work.

## Learn more

- Start: [Quickstart](docs/quickstart.md), [Examples](examples/README.md), [Troubleshooting](docs/troubleshooting.md)
- Understand: [Concepts](docs/concepts.md), [Glossary](docs/glossary.md), [Agents](docs/agents.md)
- Reference: [Commands](docs/commands.md), [Configuration](docs/config.md), [Diagnostics](docs/diagnostics.md), [all pages](docs/README.md)
- Project: [Decisions](docs/adr/), [Changelog](CHANGELOG.md)

## Contributing

spec-dna is a Rust binary with 4 dependencies and no network code. [CONTRIBUTING.md](CONTRIBUTING.md) gives the setup and the rules. Report a bug in [issues](https://github.com/AlexGavrilov939/spec-dna/issues/new/choose), and a vulnerability through [SECURITY.md](SECURITY.md).

## License

[Apache-2.0](LICENSE)
