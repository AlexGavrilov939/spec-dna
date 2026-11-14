# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AlexGavrilov939/spec-dna)](https://github.com/AlexGavrilov939/spec-dna/releases)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

spec-dna is a command-line tool for the specifications that a team keeps as Markdown files in its git repository: specs, contracts, decisions, guides and runbooks. It checks each change of these documents, as a linter checks code. It also gives an AI coding agent only the parts of the documents that its task needs.

spec-dna works in any repository, with any language and any agent. It uses no network and no language model.

## Why

When AI agents write a large part of the code, the specifications are the main input of each task. Then these problems occur:

1. An agent changes a decision that the team accepted. For example, it edits the text of an accepted decision record so that the text agrees with its new code. A review can miss this edit.
2. References go stale. A change renames a file or a function, but a spec still names the old one. A link points to a heading that no longer exists.
3. Documents grow. Each task adds text, and nobody removes it. An agent reads whole long files to find one rule, and this costs time and tokens on each task.

spec-dna treats the specifications like code. Each document has a type, a stable id and a status. Each requirement has a key. `check` compares a change with its base and reports the new problems, in CI or before a commit. `context` gives an agent the few sections that apply to the files that it will change.

## How it works

- Your documents stay plain Markdown. spec-dna reads their front matter (`type`, `status`), headings, links, code spans and requirement list items.
- A requirement is a list item with a bold key, for example `- **shutoff**: When a tray overflows, the controller SHALL close the valve of that tray.` Its id is `<document id>#<key>`. Other documents and tests refer to it by this id.
- `spec-dna.toml` maps your directories to document types and sets the rules. It also pins the version of spec-dna, so each person and each CI job uses the same rules.
- `check` reports only the problems that a change adds. A mature repository can adopt spec-dna without a fix of all old problems first.
- An accepted decision is frozen. A change goes into a new amendment section, or into a new decision that supersedes the old one.
- `init` writes one line into `AGENTS.md` and `CLAUDE.md`, and one skill. They tell the agent to run `spec-dna context` before it edits files, and `spec-dna check` before it finishes.

## Example

An agent changed the outcome of an accepted decision. `check` stops the change:

```console
$ spec-dna check
docs/decisions/water-source.md:1: error SD405 decision-frozen: the text outside the amendment and live sections changed after the decision was accepted
SD405 fix: Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, such as a progress section) can change in place. As an alternative, write a decision that supersedes this one.
spec-dna check: 2 documents; 1 new error and 0 new warnings against 3bd199ae40b9 (1 error and 1 warning in total)
```

Before an agent edits `src/valves.rs`, `context` gives it the requirement that cites the file:

```console
$ spec-dna context src/valves.rs
<!-- spec-dna context 0.5.0: src/valves.rs; 2 sections in full, 0 listed; 275 of 8000 characters; about 69 tokens (estimated) -->

<!-- valves#requirements h:e27a5916071223ee spec/active cites -->
## Requirements

- **shutoff**: When a tray overflows, the controller SHALL close the valve of that tray. The check is in `src/valves.rs`.

<!-- valves#valves h:678c75294fdea71f spec/active cites -->
# Valves
```

## Install

Each [release](https://github.com/AlexGavrilov939/spec-dna/releases) has binaries for macOS, Linux and Windows. On macOS (Apple silicon):

```sh
v=0.5.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
```

To build from source, you need Rust 1.88 or later:

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.5.0 spec-dna
```

You also need git. [docs/quickstart.md](docs/quickstart.md#install) gives the targets of the other platforms, the commands for Windows, and the verification of the attestations.

## Use it in a project

1. In the root of the repository, run `spec-dna init`. It finds your documents and their layout, asks at most 10 questions, and shows the configuration. After you confirm, it writes `spec-dna.toml`, the agent adapters and, by default, a small agent harness for Claude Code (a main model, an advisor, a reviewer and a search agent). It does not change your documents.
2. Run `spec-dna check --all`. It shows each problem in the documents once. You do not have to fix the old problems: the gate reports only new ones.
3. Commit `spec-dna.toml` and the adapters.
4. Add the gate to CI with `spec-dna check --base origin/<main branch>`, or as a git hook. `init` can write a GitHub Actions job or a pre-commit hook for you.
5. Write new requirements as list items with a bold key, and cite the code that implements them:

    ```md
    ---
    type: spec
    status: active
    ---
    # Valves

    ## Requirements

    - **shutoff**: When a tray overflows, the controller SHALL close the valve of that tray.

    The code is in `src/valves.rs` (`close_valve()`).
    ```

6. Link a test to a requirement with a comment such as `# covers: valves#shutoff`. `spec-dna show valves` lists the tests of each requirement.

[docs/quickstart.md](docs/quickstart.md) gives each step in full, with the upgrade and the uninstall. [docs/demo.md](docs/demo.md) shows the steps on a small example project.

## Daily use

An agent does these steps itself, because the skill tells it to:

```sh
spec-dna context src/valves.rs   # before an edit: the sections that apply to this file
spec-dna check                   # before it finishes: exit code 1 on a new error
```

People use the same commands, and these ones:

```sh
spec-dna show valves                         # a document, with the tests of each requirement
spec-dna new decision pump-choice            # a new document from a template
spec-dna mv valves#shutoff safety            # move a requirement; the old id stays an alias
spec-dna history decisions/water-source      # move the amendments of a long decision into a reference document
spec-dna index docs/incidents --replace      # a generated index of a directory
```

[docs/commands.md](docs/commands.md) gives each command and its options.

## Status

spec-dna is at version 0.x. A minor version can change the configuration keys and the diagnostics. [CHANGELOG.md](CHANGELOG.md) lists each change.

In agent experiments, 2 blind graders checked each run. Agents with the gate kept accepted decisions intact in 15 of 15 runs, against 6 of 15 runs without it. With spec-dna, the agents also read more tokens. [docs/evidence.md](docs/evidence.md) gives the method, the other results and the limits.

## Documentation

- [Quickstart](docs/quickstart.md): install, setup, CI job, upgrade, uninstall
- [Commands](docs/commands.md): each command, its options and the exit codes
- [init](docs/init.md): what `init` finds and asks
- [Agent harness](docs/harness.md): the roles, models and hooks that `init` installs for Claude Code
- [Configuration](docs/config.md): each key of `spec-dna.toml`
- [Document types](docs/types.md): the types, their statuses and templates
- [Diagnostics](docs/diagnostics.md): each diagnostic code and its fix
- [Profiles](docs/profiles.md): the severity of each code in each profile
- [Transcripts](docs/transcript.md): token and read counts from agent sessions
- [Evidence](docs/evidence.md): the experiments and their limits
- [Decisions](docs/adr/): the information model, the language, the distribution and the agent harness

## Contributing

spec-dna is written in Rust. It has 4 direct dependencies, has no network code, and runs `git` to read the repository. [CONTRIBUTING.md](CONTRIBUTING.md) gives the setup, the rules for commits and pull requests, the dependencies and the release steps. To report a vulnerability, follow [SECURITY.md](SECURITY.md).

## License

[Apache-2.0](LICENSE)
